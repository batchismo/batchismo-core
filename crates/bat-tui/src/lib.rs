pub mod app;
pub mod event;
pub mod ui;

use std::sync::Arc;

use anyhow::{Context, Result};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use tracing::info;

use bat_gateway::{config, db::Database, Gateway};

use app::App;
use event::EventHandler;

/// Run the TUI. Call this from main or from the CLI `batchismo tui` subcommand.
pub async fn run() -> Result<()> {
    // Load config and database (same as bat-shell)
    let cfg = config::load_config().context("Failed to load config")?;
    let db_path = config::db_path();
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let db = Arc::new(Database::open(&db_path).context("Failed to open database")?);

    // Seed path policies from config if DB has none
    let existing = db.get_path_policies()?;
    if existing.is_empty() {
        for policy in &cfg.paths {
            db.add_path_policy(policy)?;
        }
    }

    // Create gateway
    let gateway = Arc::new(Gateway::new(cfg, db).context("Failed to create gateway")?);
    info!("Gateway initialized");

    // Load history for display
    let history = gateway.get_main_history().await.unwrap_or_default();

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(Arc::clone(&gateway), history);

    // Subscribe to gateway events
    let event_handler = EventHandler::new(gateway.subscribe_events());

    // Main loop
    let result = run_loop(&mut terminal, &mut app, &event_handler).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    event_handler: &EventHandler,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;
        event_handler.handle(app).await?;
        if app.should_quit {
            return Ok(());
        }
    }
}
