/// IPC server — Windows named pipes with NDJSON protocol.
///
/// The gateway creates a named pipe, spawns bat-agent as a child process,
/// waits for the agent to connect, then communicates via NDJSON messages.

use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, ReadHalf, WriteHalf};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use uuid::Uuid;

use bat_types::ipc::{AgentToGateway, GatewayToAgent};

/// A bidirectional NDJSON channel to a connected agent.
pub struct AgentPipe {
    writer: WriteHalf<NamedPipeServer>,
    reader: BufReader<ReadHalf<NamedPipeServer>>,
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

/// Returns the Windows named pipe name for a given session ID.
pub fn agent_pipe_name(session_id: Uuid) -> String {
    format!(r"\\.\pipe\bat-agent-{}", session_id)
}

/// Create a named pipe server (non-blocking — agent connects after this returns).
///
/// Returns the server handle and the pipe name. The caller must call
/// `spawn_agent()` with the pipe name BEFORE calling `wait_for_agent()`,
/// otherwise the server will wait forever.
pub fn create_pipe_server(session_id: Uuid) -> Result<(NamedPipeServer, String)> {
    let pipe_name = agent_pipe_name(session_id);
    let server = ServerOptions::new()
        .first_pipe_instance(true)
        .create(&pipe_name)
        .with_context(|| format!("Failed to create named pipe: {pipe_name}"))?;
    Ok((server, pipe_name))
}

/// Wait for the agent to connect to the pipe, then return a bidirectional channel.
pub async fn wait_for_agent(server: NamedPipeServer) -> Result<AgentPipe> {
    server
        .connect()
        .await
        .context("Failed while waiting for agent to connect to pipe")?;
    let (read_half, write_half) = tokio::io::split(server);
    Ok(AgentPipe {
        writer: write_half,
        reader: BufReader::new(read_half),
    })
}

/// Spawn the bat-agent child process, pointed at the given pipe.
/// The API key is passed via environment variable.
pub fn spawn_agent(pipe_name: &str, api_key: &str) -> Result<tokio::process::Child> {
    let agent_exe = find_agent_binary()?;
    let mut cmd = tokio::process::Command::new(&agent_exe);
    cmd.arg("--pipe")
        .arg(pipe_name)
        .env("ANTHROPIC_API_KEY", api_key);

    // On Windows, prevent the agent from flashing a console window
    // Capture stderr so we can log agent errors
    cmd.stderr(std::process::Stdio::piped());

    // On Windows, prevent the agent from flashing a console window.
    // tokio::process::Command re-exports the Windows CommandExt trait.
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
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

    // 1. Same directory as bat-shell.exe (cargo build output)
    let candidate = dir.join("bat-agent.exe");
    if candidate.exists() {
        return Ok(candidate);
    }
    let candidate = dir.join("bat-agent");
    if candidate.exists() {
        return Ok(candidate);
    }

    // 2. Tauri resource directory (installed app)
    // MSI installs resources next to the exe; NSIS puts them in a resources/ subfolder
    let candidate = dir.join("resources").join("bat-agent.exe");
    if candidate.exists() {
        return Ok(candidate);
    }

    anyhow::bail!(
        "bat-agent binary not found in {} or its resources/ subdirectory. \
         Build the workspace first with `cargo build`.",
        dir.display()
    )
}
