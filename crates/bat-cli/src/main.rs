use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "batchismo", version, about = "Batchismo — your local AI agent platform")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the terminal UI
    Tui,
    /// Show current status
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    match cli.command {
        None => {
            // Default: launch the desktop app
            let exe = std::env::current_exe()?;
            let dir = exe.parent().unwrap();
            let shell = dir.join("bat-shell.exe");
            if shell.exists() {
                std::process::Command::new(&shell)
                    .spawn()
                    .map_err(|e| anyhow::anyhow!("Failed to launch bat-shell: {e}"))?;
                println!("Launched Batchismo desktop app.");
            } else {
                eprintln!("bat-shell.exe not found alongside batchismo.exe — launching TUI instead.");
                bat_tui::run().await?;
            }
            Ok(())
        }
        Some(Commands::Tui) => {
            bat_tui::run().await
        }
        Some(Commands::Status) => {
            let cfg = bat_gateway::config::load_config()?;
            println!("Batchismo v{}", env!("CARGO_PKG_VERSION"));
            println!("Agent: {}", cfg.agent.name);
            println!("Model: {}", cfg.agent.model);
            println!("Thinking: {}", cfg.agent.thinking_level);
            println!("API Key: {}", if cfg.agent.api_key.is_some() { "configured" } else { "not set" });
            println!("Config: {}", bat_gateway::config::config_path().display());
            println!("Database: {}", bat_gateway::config::db_path().display());
            Ok(())
        }
    }
}
