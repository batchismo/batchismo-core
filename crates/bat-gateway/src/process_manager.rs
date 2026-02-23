//! Async process manager — spawns and manages long-running child processes
//! that persist across agent turns.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use anyhow::{Context, Result};
use chrono::Utc;
use tracing::info;

use bat_types::ipc::ProcessInfo;

/// Maximum output buffer per process (1 MB).
const MAX_BUFFER: usize = 1_048_576;

/// How long to keep finished processes before cleanup (30 min).
const CLEANUP_AFTER_SECS: i64 = 1800;

struct ManagedProcess {
    command: String,
    started_at: String,
    #[allow(dead_code)]
    finished_at: Option<String>,
    stdin: Option<tokio::process::ChildStdin>,
    stdout_buf: Arc<Mutex<Vec<u8>>>,
    stderr_buf: Arc<Mutex<Vec<u8>>>,
    is_running: Arc<Mutex<bool>>,
    exit_code: Arc<Mutex<Option<i32>>>,
    child: Arc<Mutex<Child>>,
}

#[derive(Clone)]
pub struct ProcessManager {
    processes: Arc<Mutex<HashMap<String, ManagedProcess>>>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Generate a short human-readable session ID.
    fn gen_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        format!("{:06x}", nanos & 0xFFFFFF)
    }

    /// Spawn a new managed process.
    pub async fn spawn(
        &self,
        command: &str,
        workdir: Option<&str>,
    ) -> Result<String> {
        let session_id = Self::gen_id();

        let mut cmd = if cfg!(target_os = "windows") {
            let mut c = Command::new("cmd");
            c.args(["/C", command]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", command]);
            c
        };

        if let Some(dir) = workdir {
            cmd.current_dir(dir);
        }

        cmd.stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Windows: hide console window
        #[cfg(target_os = "windows")]
        {
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        let mut child = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn: {command}"))?;

        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let stdout_buf = Arc::new(Mutex::new(Vec::new()));
        let stderr_buf = Arc::new(Mutex::new(Vec::new()));
        let is_running = Arc::new(Mutex::new(true));
        let exit_code: Arc<Mutex<Option<i32>>> = Arc::new(Mutex::new(None));

        // Spawn stdout reader task
        if let Some(mut out) = stdout {
            let buf = stdout_buf.clone();
            tokio::spawn(async move {
                let mut tmp = [0u8; 4096];
                loop {
                    match out.read(&mut tmp).await {
                        Ok(0) => break,
                        Ok(n) => {
                            let mut b = buf.lock().await;
                            if b.len() < MAX_BUFFER {
                                let take = n.min(MAX_BUFFER - b.len());
                                b.extend_from_slice(&tmp[..take]);
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
        }

        // Spawn stderr reader task
        if let Some(mut err) = stderr {
            let buf = stderr_buf.clone();
            tokio::spawn(async move {
                let mut tmp = [0u8; 4096];
                loop {
                    match err.read(&mut tmp).await {
                        Ok(0) => break,
                        Ok(n) => {
                            let mut b = buf.lock().await;
                            if b.len() < MAX_BUFFER {
                                let take = n.min(MAX_BUFFER - b.len());
                                b.extend_from_slice(&tmp[..take]);
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
        }

        // Spawn waiter task to update status when process exits
        let child_arc = Arc::new(Mutex::new(child));
        {
            let running = is_running.clone();
            let code = exit_code.clone();
            let child_ref = child_arc.clone();
            tokio::spawn(async move {
                let status = child_ref.lock().await.wait().await;
                *running.lock().await = false;
                if let Ok(s) = status {
                    *code.lock().await = s.code();
                }
            });
        }

        let proc = ManagedProcess {
            command: command.to_string(),
            started_at: Utc::now().to_rfc3339(),
            finished_at: None,
            stdin,
            stdout_buf,
            stderr_buf,
            is_running,
            exit_code,
            child: child_arc,
        };

        info!("Process spawned: session={session_id}, cmd={command}");
        self.processes.lock().await.insert(session_id.clone(), proc);

        Ok(session_id)
    }

    /// Get output from a managed process.
    pub async fn get_output(&self, session_id: &str) -> Result<(String, String, bool, Option<i32>)> {
        let procs = self.processes.lock().await;
        let proc = procs
            .get(session_id)
            .ok_or_else(|| anyhow::anyhow!("No process with session_id: {session_id}"))?;

        let stdout = String::from_utf8_lossy(&proc.stdout_buf.lock().await).to_string();
        let stderr = String::from_utf8_lossy(&proc.stderr_buf.lock().await).to_string();
        let running = *proc.is_running.lock().await;
        let code = *proc.exit_code.lock().await;

        Ok((stdout, stderr, running, code))
    }

    /// Write to stdin of a running process.
    pub async fn write_stdin(&self, session_id: &str, data: &str) -> Result<()> {
        let mut procs = self.processes.lock().await;
        let proc = procs
            .get_mut(session_id)
            .ok_or_else(|| anyhow::anyhow!("No process with session_id: {session_id}"))?;

        let stdin = proc
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Process stdin not available"))?;

        stdin.write_all(data.as_bytes()).await?;
        stdin.flush().await?;
        Ok(())
    }

    /// Kill a running process.
    pub async fn kill(&self, session_id: &str) -> Result<()> {
        let procs = self.processes.lock().await;
        let proc = procs
            .get(session_id)
            .ok_or_else(|| anyhow::anyhow!("No process with session_id: {session_id}"))?;

        let mut child = proc.child.lock().await;
        child.kill().await.context("Failed to kill process")?;
        info!("Process killed: session={session_id}");
        Ok(())
    }

    /// List all managed processes.
    pub async fn list(&self) -> Vec<ProcessInfo> {
        let procs = self.processes.lock().await;
        let mut result = Vec::new();
        for (id, proc) in procs.iter() {
            let running = *proc.is_running.lock().await;
            let code = *proc.exit_code.lock().await;
            result.push(ProcessInfo {
                session_id: id.clone(),
                command: proc.command.clone(),
                is_running: running,
                exit_code: code,
                started_at: proc.started_at.clone(),
            });
        }
        result
    }

    /// Run a command and wait for it to complete (foreground mode).
    /// Returns (stdout, stderr, exit_code).
    pub async fn run_foreground(
        &self,
        command: &str,
        workdir: Option<&str>,
    ) -> Result<(String, String, Option<i32>)> {
        let sid = self.spawn(command, workdir).await?;

        // Poll until done (with timeout of 60 seconds)
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(60);
        loop {
            let (stdout, stderr, running, code) = self.get_output(&sid).await?;
            if !running {
                // Clean up
                self.processes.lock().await.remove(&sid);
                return Ok((stdout, stderr, code));
            }
            if tokio::time::Instant::now() > deadline {
                let _ = self.kill(&sid).await;
                self.processes.lock().await.remove(&sid);
                anyhow::bail!("Command timed out after 60 seconds");
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    /// Clean up finished processes older than CLEANUP_AFTER_SECS.
    pub async fn cleanup(&self) {
        let now = Utc::now();
        let mut procs = self.processes.lock().await;
        let to_remove: Vec<String> = procs
            .iter()
            .filter(|(_, p)| {
                // Only clean up finished processes
                // We check is_running synchronously via try_lock
                if let Ok(running) = p.is_running.try_lock() {
                    if *running {
                        return false;
                    }
                }
                // Check age
                if let Ok(started) = chrono::DateTime::parse_from_rfc3339(&p.started_at) {
                    let age = now.signed_duration_since(started);
                    age.num_seconds() > CLEANUP_AFTER_SECS
                } else {
                    true
                }
            })
            .map(|(id, _)| id.clone())
            .collect();

        for id in &to_remove {
            procs.remove(id);
        }
        if !to_remove.is_empty() {
            info!("Cleaned up {} finished processes", to_remove.len());
        }
    }
}
