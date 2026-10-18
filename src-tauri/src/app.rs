use music_player_backend::config::AppConfig;
use music_player_backend::core::command::{Command, CommandResponse};
use music_player_backend::core::processor::CoreProcessor;
use music_player_backend::core::query::{Query, QueryResponse};
use music_player_backend::database::init_db_pool;
use music_player_backend::logging::init_logging;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;
use tracing::info;

mod lifecycle;
#[cfg(target_os = "linux")]
mod linux_activation;
#[cfg(target_os = "linux")]
mod linux_rendering;
mod tray;
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
    let parsed_command: Command =
        serde_json::from_value(val).map_err(|e| format!("Command parse error: {}", e))?;
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
            if !obj.contains_key("payload") || obj.get("payload") == Some(&serde_json::Value::Null)
            {
                obj.insert("payload".to_string(), serde_json::json!({}));
            }
        }
    }
    let parsed_query: Query =
        serde_json::from_value(val).map_err(|e| format!("Query parse error: {}", e))?;
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
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            tracing::info!("Second instance launch requested; restoring existing Kaze window");
            tray::show_main_window_from_launch(app, &args);
        }))
        .manage(lifecycle::DesktopLifecycle::default())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            let handle = app.handle().clone();

            tauri::async_runtime::block_on(async move {
                let pool = init_db_pool(&db_path)
                    .await
                    .expect("Failed to initialize SQLite pool");
                let pool_for_repair = pool.clone();
                handle.state::<lifecycle::DesktopLifecycle>().spawn(async move {
                    let _ =
                        music_player_backend::recommendations::mixes::repair_legacy_online_tracks(
                            &pool_for_repair,
                        )
                        .await;
                });
                let processor = Arc::new(CoreProcessor::new(pool, config));
                processor.initialize_close_behavior().await.expect("Failed to load window close setting");
                processor.playback_service().initialize_saved_playback().await;

                let mut rx = processor.event_bus().subscribe();
                let emit_handle = handle.clone();
                handle.state::<lifecycle::DesktopLifecycle>().spawn(async move {
                    loop {
                        match rx.recv().await {
                            Ok(event) => {
                                if let Err(e) = emit_handle.emit("backend-event", &event) {
                                    tracing::error!(?e, "Failed to emit backend-event to webview");
                                }
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                                tracing::warn!(
                                    skipped,
                                    "Event bus receiver lagged behind; continuing"
                                );
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                tracing::info!("Event bus channel closed; exiting event loop");
                                break;
                            }
                        }
                    }
                });

                #[cfg(target_os = "linux")]
                {
                    match music_player_backend::mpris::start_mpris_service(
                        processor.clone(),
                        Some(handle.clone()),
                    )
                    .await
                    {
                        Ok(service_handle) => {
                            handle.manage(service_handle);
                        }
                        Err(e) => tracing::warn!("Failed to start Linux MPRIS service: {}", e),
                    }
                }

                handle.manage(processor);

                // Silent background update check — fires 5s after startup
                let update_handle = handle.clone();
                handle.state::<lifecycle::DesktopLifecycle>().spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    if let Ok(updater) = update_handle.updater() {
                        match updater.check().await {
                            Ok(Some(update)) => {
                                let version = update.version.clone();
                                tracing::info!("Update available: v{}", version);
                                // Notify frontend so it can show a banner
                                let _ = update_handle.emit(
                                    "update-available",
                                    serde_json::json!({ "version": version }),
                                );
                                // Download and install; app will restart automatically
                                if let Err(e) = update.download_and_install(|_, _| {}, || {}).await
                                {
                                    tracing::error!("Update install failed: {}", e);
                                }
                            }
                            Ok(None) => tracing::info!("Kaze is up to date"),
                            Err(e) => tracing::warn!("Update check failed: {}", e),
                        }
                    }
                });
            });

            if let Err(error) = tray::setup_tray(app.handle()) {
                tracing::warn!(%error, "System tray unavailable; closing the window will quit Kaze");
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    // Keep the webview alive until either hide succeeds or the
                    // asynchronous quit drain finishes. Never re-hide on Quit.
                    api.prevent_close();
                    lifecycle::close_main_window(window);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![execute_command, execute_query])
        .build(context)
        .expect("error building tauri application")
        .run(|app_handle, event| {
            let lifecycle = app_handle.state::<lifecycle::DesktopLifecycle>();
            match event {
                tauri::RunEvent::ExitRequested { api, code, .. } if !lifecycle.is_complete() => {
                    if let Some(processor) = app_handle.try_state::<Arc<CoreProcessor>>() { processor.begin_quit(); }
                    // Tauri does not allow preventing its special Restart event.
                    if code == Some(tauri::RESTART_EXIT_CODE) {
                        tauri::async_runtime::block_on(lifecycle.shutdown(app_handle));
                    } else {
                        api.prevent_exit();
                        if lifecycle.begin() {
                            let handle = app_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                handle.state::<lifecycle::DesktopLifecycle>().shutdown(&handle).await;
                                handle.exit(code.unwrap_or(0));
                            });
                        }
                    }
                }
                // Fallback for runtime/session Exit without an interceptable
                // request. The same completion gate makes this a normal no-op.
                tauri::RunEvent::Exit => tauri::async_runtime::block_on(lifecycle.shutdown(app_handle)),
                _ => {}
            }
        });
}

fn main() {
    #[cfg(target_os = "linux")]
    {
        linux_rendering::configure();
        linux_activation::preserve_launcher_token();
    }
    run();
}
