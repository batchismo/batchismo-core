#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use std::sync::Arc;

use tauri::{Emitter, Manager};

use bat_gateway::{config, db::Database, Gateway};

pub struct AppState {
    pub gateway: Arc<Gateway>,
}

fn main() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        tracing_subscriber::EnvFilter::new(
            "info,hyper_util=warn,hyper=warn,reqwest=warn,h2=warn,rustls=warn,tao=error",
        )
    });
    tracing_subscriber::fmt().with_env_filter(filter).init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Load config (creates defaults if missing)
            let cfg = config::load_config()?;

            // Open database
            let db_path = config::db_path();
            if let Some(parent) = db_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let db = Arc::new(Database::open(&db_path)?);

            // Seed path policies from config if the DB has none
            let existing = db.get_path_policies()?;
            if existing.is_empty() {
                for policy in &cfg.paths {
                    db.add_path_policy(policy)?;
                }
            }

            // Create gateway
            let gateway = Arc::new(Gateway::new(cfg, db)?);

            // Subscribe to gateway events and forward to Tauri frontend
            let mut rx = gateway.subscribe_events();
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    match rx.recv().await {
                        Ok(event) => {
                            if let Err(e) = app_handle.emit("bat-event", &event) {
                                tracing::warn!("Failed to emit bat-event: {e}");
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("Event bus lagged by {n} events");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            tracing::info!("Event bus closed — stopping forwarder");
                            break;
                        }
                    }
                }
            });

            // Log gateway startup
            gateway.log_event(
                bat_types::audit::AuditLevel::Info,
                bat_types::audit::AuditCategory::Gateway,
                "gateway_start",
                "Batchismo gateway started",
                None,
                None,
            );

            app.manage(AppState { gateway });

            // Open devtools automatically when RUST_LOG is set (any build)
            if std::env::var("RUST_LOG").is_ok() {
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::send_message,
            commands::get_history,
            commands::get_session,
            commands::get_path_policies,
            commands::add_path_policy,
            commands::delete_path_policy,
            commands::get_tools,
            commands::toggle_tool,
            commands::get_config,
            commands::update_config,
            commands::get_system_prompt,
            commands::get_memory_files,
            commands::get_memory_file,
            commands::update_memory_file,
            commands::get_observations,
            commands::get_observation_summary,
            commands::trigger_consolidation,
            commands::is_onboarding_complete,
            commands::validate_api_key,
            commands::complete_onboarding,
            commands::get_audit_logs,
            commands::get_audit_stats,
            commands::get_subagents,
            commands::list_sessions,
            commands::create_session,
            commands::switch_session,
            commands::delete_session_by_key,
            commands::rename_session,
            commands::get_active_session_key,
            commands::get_usage_stats,
        ])
        .run(tauri::generate_context!())
        .expect("Error while running Batchismo");
}
