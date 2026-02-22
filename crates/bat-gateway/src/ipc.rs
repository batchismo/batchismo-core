/// IPC server — platform-specific transport with NDJSON protocol.
///
/// - Windows: named pipes (`\\.\pipe\bat-agent-{session_id}`)
/// - Unix (macOS/Linux): Unix domain sockets (`/tmp/bat-agent-{session_id}.sock`)
///
/// The gateway creates a server, spawns bat-agent as a child process,
/// waits for the agent to connect, then communicates via NDJSON messages.

use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use uuid::Uuid;

use bat_types::ipc::{AgentToGateway, GatewayToAgent};

// ─── Platform-specific transport ──────────────────────────────────────────────

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use tokio::io::{ReadHalf, WriteHalf};
    use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};

    pub type Reader = BufReader<ReadHalf<NamedPipeServer>>;
    pub type Writer = WriteHalf<NamedPipeServer>;

    pub fn pipe_address(session_id: Uuid) -> String {
        format!(r"\\.\pipe\bat-agent-{}", session_id)
    }

    pub struct PipeServer(pub NamedPipeServer);

    pub fn create_server(session_id: Uuid) -> Result<(PipeServer, String)> {
        let addr = pipe_address(session_id);
        let server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(&addr)
            .with_context(|| format!("Failed to create named pipe: {addr}"))?;
        Ok((PipeServer(server), addr))
    }

    pub async fn wait_for_connection(server: PipeServer) -> Result<(Reader, Writer)> {
        server.0
            .connect()
            .await
            .context("Failed while waiting for agent to connect to pipe")?;
        let (read_half, write_half) = tokio::io::split(server.0);
        Ok((BufReader::new(read_half), write_half))
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    use super::*;
    use tokio::io::{ReadHalf, WriteHalf};
    use tokio::net::UnixListener;
    use tokio::net::UnixStream;

    pub type Reader = BufReader<ReadHalf<UnixStream>>;
    pub type Writer = WriteHalf<UnixStream>;

    pub fn pipe_address(session_id: Uuid) -> String {
        format!("/tmp/bat-agent-{}.sock", session_id)
    }

    pub struct PipeServer {
        pub listener: UnixListener,
        pub path: String,
    }

    pub fn create_server(session_id: Uuid) -> Result<(PipeServer, String)> {
        let addr = pipe_address(session_id);
        // Remove stale socket file if it exists
        let _ = std::fs::remove_file(&addr);
        let listener = UnixListener::bind(&addr)
            .with_context(|| format!("Failed to create Unix socket: {addr}"))?;
        Ok((PipeServer { listener, path: addr.clone() }, addr))
    }

    pub async fn wait_for_connection(server: PipeServer) -> Result<(Reader, Writer)> {
        let (stream, _) = server.listener.accept()
            .await
            .context("Failed while waiting for agent to connect to socket")?;
        let (read_half, write_half) = tokio::io::split(stream);
        Ok((BufReader::new(read_half), write_half))
    }
}

// ─── Cross-platform API ───────────────────────────────────────────────────────

/// A bidirectional NDJSON channel to a connected agent.
pub struct AgentPipe {
    writer: platform::Writer,
    reader: platform::Reader,
}

impl AgentPipe {
    /// Send a message to the agent.
    pub async fn send(&mut self, msg: &GatewayToAgent) -> Result<()> {
        let json = serde_json::to_string(msg).context("Failed to serialize gateway message")?;
        self.writer.write_all(json.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        Ok(())
    }

    /// Receive the next message from the agent. Returns None if the pipe closed.
    pub async fn recv(&mut self) -> Result<Option<AgentToGateway>> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line).await?;
        if n == 0 {
            return Ok(None);
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        let msg = serde_json::from_str(trimmed)
            .with_context(|| format!("Failed to parse agent message: {trimmed}"))?;
        Ok(Some(msg))
    }
}

/// Create an IPC server for the given session.
/// Returns the server handle and the address string (pipe name or socket path).
pub fn create_pipe_server(session_id: Uuid) -> Result<(platform::PipeServer, String)> {
    platform::create_server(session_id)
}

/// Wait for the agent to connect, then return a bidirectional channel.
pub async fn wait_for_agent(server: platform::PipeServer) -> Result<AgentPipe> {
    let (reader, writer) = platform::wait_for_connection(server).await?;
    Ok(AgentPipe { writer, reader })
}

/// Spawn the bat-agent child process, pointed at the given pipe/socket.
/// The API key is passed via environment variable.
pub fn spawn_agent(pipe_name: &str, api_key: &str) -> Result<tokio::process::Child> {
    let agent_exe = find_agent_binary()?;
    let mut cmd = tokio::process::Command::new(&agent_exe);
    cmd.arg("--pipe")
        .arg(pipe_name)
        .env("ANTHROPIC_API_KEY", api_key);

    // Capture stderr so we can log agent errors
    cmd.stderr(std::process::Stdio::piped());

    // On Windows, prevent the agent from flashing a console window.
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        // tokio::process::Command re-exports the Windows CommandExt trait
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let child = cmd
        .spawn()
        .with_context(|| format!("Failed to spawn bat-agent at {}", agent_exe.display()))?;
    Ok(child)
}

/// Find the bat-agent binary. Checks:
/// 1. Next to the current executable (dev/release builds)
/// 2. Tauri resource directory (installed via MSI/NSIS)
fn find_agent_binary() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("Cannot determine current exe path")?;
    let dir = exe
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Current exe has no parent directory"))?;

    let names: &[&str] = if cfg!(target_os = "windows") {
        &["bat-agent.exe"]
    } else {
        &["bat-agent"]
    };

    for name in names {
        // 1. Same directory
        let candidate = dir.join(name);
        if candidate.exists() {
            return Ok(candidate);
        }

        // 2. Tauri resource directory
        let candidate = dir.join("resources").join(name);
        if candidate.exists() {
            return Ok(candidate);
        }

        // 3. macOS app bundle Resources
        #[cfg(target_os = "macos")]
        {
            let candidate = dir.join("../Resources").join(name);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    anyhow::bail!(
        "bat-agent binary not found in {} or its resources/ subdirectory. \
         Build the workspace first with `cargo build`.",
        dir.display()
    )
}
