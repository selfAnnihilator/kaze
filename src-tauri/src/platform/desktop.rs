//! Desktop shell: tray, updater, single instance, autostart, MPRIS and the
//! window lifecycle around the shared backend.

use super::shared;
use crate::config::AppConfig;
use crate::core::processor::CoreProcessor;
use crate::database::init_db_pool;
use crate::library::artwork;
use crate::logging::init_logging;
use crate::playback::hidden_ticks::HiddenTicks;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;
use tracing::info;

mod lifecycle;
#[cfg(target_os = "linux")]
mod linux_activation;
#[cfg(target_os = "linux")]
mod linux_rendering;
mod startup;
mod tray;
#[cfg(target_os = "linux")]
mod webview_tuning;
#[cfg(target_os = "linux")]
mod window_chrome;

/// Linux environment tweaks that must run before any toolkit initialises.
pub fn preconfigure() {
    #[cfg(target_os = "linux")]
    {
        linux_rendering::configure();
        linux_activation::preserve_launcher_token();
    }
}

pub fn run() {
    init_logging();
    info!("Starting Intelligent Music Player Desktop Shell");
    // A music player's async work is I/O-bound; a worker per core (Tokio's
    // default) only adds idle threads. Heavy work uses spawn_blocking.
    shared::init_async_runtime();

    let config = AppConfig::default_with_dirs();
    let db_path = config.database_path.clone();

    let login_launch = startup::is_login_launch(&std::env::args().collect::<Vec<_>>());
    let mut context = tauri::generate_context!();
    if login_launch {
        // Set before window creation so login never flashes the main window.
        for window in &mut context.config_mut().app.windows {
            if window.label == "main" {
                window.visible = false;
            }
        }
    }
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

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            // A late session-autostart request must not interrupt an open app.
            if startup::is_login_launch(&args) {
                return;
            }
            tracing::info!("Second instance launch requested; restoring existing Kaze window");
            tray::show_main_window_from_launch(app, &args);
        }))
        .manage(lifecycle::DesktopLifecycle::default())
        .manage(startup::StartupSettings::default());
    #[cfg(not(target_os = "linux"))]
    let builder = builder.plugin(
        tauri_plugin_autostart::Builder::new()
            .app_name("Kaze")
            .arg(startup::LOGIN_ARG)
            .build(),
    );
    builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .register_asynchronous_uri_scheme_protocol(artwork::SCHEME, shared::serve_artwork)
        .setup(move |app| {
            let handle = app.handle().clone();

            tauri::async_runtime::block_on(async move {
                let pool = init_db_pool(&db_path)
                    .await
                    .expect("Failed to initialize SQLite pool");
                let pool_for_repair = pool.clone();
                handle.state::<lifecycle::DesktopLifecycle>().spawn(async move {
                    let _ =
                        crate::recommendations::mixes::repair_legacy_online_tracks(
                            &pool_for_repair,
                        )
                        .await;
                });
                #[cfg(feature = "acceptance-fixtures")]
                let fixture_cache = config.cache_dir.clone();
                let processor = Arc::new(CoreProcessor::new(pool, config));
                // Only the explicitly instrumented acceptance build can preload
                // a loopback source; shipping builds have no fixture entrypoint.
                #[cfg(feature = "acceptance-fixtures")]
                if let Ok(url) = std::env::var("KAZE_PREPARATION_FIXTURE_URL") {
                    let parsed = reqwest::Url::parse(&url).expect("fixture URL");
                    assert_eq!(parsed.scheme(), "http");
                    assert_eq!(parsed.host_str(), Some("127.0.0.1"));
                    let manager = Arc::new(crate::playback::stream::StreamPlaybackManager::new(fixture_cache, None, None));
                    manager.resolution_cache().insert("full:v3:jiosaavn:preparation-fixture".into(),
                        crate::playback::resolution_cache::ResolvedEntry::new(url, 2.0, None));
                    processor.playback_service().set_stream_manager(manager);
                }
                processor.initialize_close_behavior().await.expect("Failed to load window close setting");
                processor.playback_service().initialize_saved_playback().await;
                // Only the real app watches the Kaze folder; tests never touch ~/Music.
                processor.download_service().start_watching();

                let mut rx = processor.event_bus().subscribe();
                let emit_handle = handle.clone();
                let hidden_ticks = watch_visibility(&handle);
                #[cfg(target_os = "linux")]
                if let Some(window) = handle.get_webview_window("main") {
                    webview_tuning::apply(&window);
                }
                handle.state::<lifecycle::DesktopLifecycle>().spawn(async move {
                    loop {
                        match rx.recv().await {
                            Ok(event) => {
                                if !hidden_ticks.forward(&event) {
                                    continue;
                                }
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
                    match crate::mpris::start_mpris_service(
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

                processor.prune_artwork_cache();
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
            if login_launch {
                let usable_tray = app.tray_by_id(tray::TRAY_ID).is_some()
                    && tauri::async_runtime::block_on(lifecycle::tray_host_available());
                if !usable_tray {
                    tracing::warn!("Login startup has no usable tray; showing Kaze instead");
                    tray::show_main_window(app.handle());
                }
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
        .invoke_handler(tauri::generate_handler![shared::execute_command, shared::execute_query, startup::get_autostart, startup::set_autostart])
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

/// Tracks whether the main window is mapped, so position ticks are not sent
/// to a hidden webview. GTK map/unmap covers every show and hide path (tray,
/// close-to-tray, MPRIS raise, launcher activation).
fn watch_visibility(handle: &tauri::AppHandle) -> Arc<HiddenTicks> {
    let window = handle.get_webview_window("main");
    let ticks = Arc::new(HiddenTicks::new(
        window.as_ref().and_then(|w| w.is_visible().ok()).unwrap_or(true),
    ));
    #[cfg(target_os = "linux")]
    if let Some(native) = window.and_then(|w| w.gtk_window().ok()) {
        use gtk::prelude::*;
        let (shown, hidden) = (ticks.clone(), ticks.clone());
        let emit_handle = handle.clone();
        native.connect_map(move |_| {
            if let Some(event) = shown.shown() {
                let _ = emit_handle.emit("backend-event", &event);
            }
        });
        native.connect_unmap(move |_| hidden.hidden());
    }
    ticks
}
