use music_player_backend::config::AppConfig;
use music_player_backend::core::command::{Command, CommandResponse};
use music_player_backend::core::processor::CoreProcessor;
use music_player_backend::core::query::{Query, QueryResponse};
use music_player_backend::database::init_db_pool;
use music_player_backend::logging::init_logging;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tracing::info;

#[cfg(target_os = "linux")]
mod window_chrome;

#[tauri::command]
async fn execute_command(
    processor: tauri::State<'_, Arc<CoreProcessor>>,
    command: serde_json::Value,
) -> Result<CommandResponse, String> {
    let mut val = command;
    if let Some(obj) = val.as_object_mut() {
        if !obj.contains_key("payload") || obj.get("payload") == Some(&serde_json::Value::Null) {
            if serde_json::from_value::<Command>(serde_json::Value::Object(obj.clone())).is_err() {
                obj.insert("payload".to_string(), serde_json::json!({}));
            }
        }
    }
    let parsed_command: Command = serde_json::from_value(val).map_err(|e| format!("Command parse error: {}", e))?;
    processor
        .dispatch_command(parsed_command)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn execute_query(
    processor: tauri::State<'_, Arc<CoreProcessor>>,
    query: serde_json::Value,
) -> Result<QueryResponse, String> {
    let mut val = query;
    if let Some(obj) = val.as_object_mut() {
        if obj.get("query").and_then(|q| q.as_str()) == Some("GetStatsOverview") {
            if !obj.contains_key("payload") || obj.get("payload") == Some(&serde_json::Value::Null) {
                obj.insert("payload".to_string(), serde_json::json!({}));
            }
        }
    }
    let parsed_query: Query = serde_json::from_value(val).map_err(|e| format!("Query parse error: {}", e))?;
    processor
        .execute_query(parsed_query)
        .await
        .map_err(|e| e.to_string())
}

pub fn run() {
    init_logging();
    info!("Starting Intelligent Music Player Desktop Shell");

    let config = AppConfig::default_with_dirs();
    let db_path = config.database_path.clone();

    let context = tauri::generate_context!();
    #[cfg(target_os = "linux")]
    let context = {
        let mut context = context;
        if window_chrome::is_niri_session(|name| std::env::var(name).ok()) {
            // Apply before window creation to avoid flashing a title bar on Niri.
            for window in &mut context.config_mut().app.windows {
                window.decorations = false;
            }
        }
        context
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            let handle = app.handle().clone();

            tauri::async_runtime::block_on(async move {
                let pool = init_db_pool(&db_path)
                    .await
                    .expect("Failed to initialize SQLite pool");
                let pool_for_repair = pool.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = music_player_backend::recommendations::mixes::repair_legacy_online_tracks(&pool_for_repair).await;
                });
                let processor = Arc::new(CoreProcessor::new(pool, config));

                let mut rx = processor.event_bus().subscribe();
                let emit_handle = handle.clone();
                tauri::async_runtime::spawn(async move {
                    loop {
                        match rx.recv().await {
                            Ok(event) => {
                                if let Err(e) = emit_handle.emit("backend-event", &event) {
                                    tracing::error!(?e, "Failed to emit backend-event to webview");
                                }
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                                tracing::warn!(skipped, "Event bus receiver lagged behind; continuing");
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                tracing::info!("Event bus channel closed; exiting event loop");
                                break;
                            }
                        }
                    }
                });

                handle.manage(processor);

                // Silent background update check — fires 5s after startup
                let update_handle = handle.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    if let Ok(updater) = update_handle.updater() {
                        match updater.check().await {
                            Ok(Some(update)) => {
                                let version = update.version.clone();
                                tracing::info!("Update available: v{}", version);
                                // Notify frontend so it can show a banner
                                let _ = update_handle.emit("update-available", serde_json::json!({ "version": version }));
                                // Download and install; app will restart automatically
                                if let Err(e) = update.download_and_install(|_, _| {}, || {}).await {
                                    tracing::error!("Update install failed: {}", e);
                                }
                            }
                            Ok(None) => tracing::info!("Kaze is up to date"),
                            Err(e) => tracing::warn!("Update check failed: {}", e),
                        }
                    }
                });
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![execute_command, execute_query])
        .run(context)
        .expect("error while running tauri application");
}

fn main() {
    run();
}
