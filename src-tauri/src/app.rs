use music_player_backend::config::AppConfig;
use music_player_backend::core::command::{Command, CommandResponse};
use music_player_backend::core::processor::CoreProcessor;
use music_player_backend::core::query::{Query, QueryResponse};
use music_player_backend::database::init_db_pool;
use music_player_backend::logging::init_logging;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tracing::info;

#[tauri::command]
async fn execute_command(
    processor: tauri::State<'_, Arc<CoreProcessor>>,
    command: Command,
) -> Result<CommandResponse, String> {
    processor
        .dispatch_command(command)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn execute_query(
    processor: tauri::State<'_, Arc<CoreProcessor>>,
    query: Query,
) -> Result<QueryResponse, String> {
    processor
        .execute_query(query)
        .await
        .map_err(|e| e.to_string())
}

pub fn run() {
    init_logging();
    info!("Starting Intelligent Music Player Desktop Shell");

    let config = AppConfig::default_with_dirs();
    let db_path = config.database_path.clone();

    tauri::Builder::default()
        .setup(move |app| {
            let handle = app.handle().clone();

            tauri::async_runtime::block_on(async move {
                let pool = init_db_pool(&db_path)
                    .await
                    .expect("Failed to initialize SQLite pool");
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
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![execute_command, execute_query])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run();
}
