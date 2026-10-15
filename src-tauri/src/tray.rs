use music_player_backend::core::command::Command;
use music_player_backend::core::processor::CoreProcessor;
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};
use tracing::info;

pub const TRAY_ID: &str = "kaze-tray";

/// Restores and brings the existing main window into focus.
pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    restore_main_window(app, None);
}

pub fn show_main_window_from_launch<R: Runtime>(app: &AppHandle<R>, args: &[String]) {
    #[cfg(target_os = "linux")]
    let token = crate::linux_activation::token_from_args(args);
    #[cfg(not(target_os = "linux"))]
    let token = {
        let _ = args;
        None
    };
    restore_main_window(app, token);
}

fn restore_main_window<R: Runtime>(app: &AppHandle<R>, token: Option<String>) {
    let handle = app.clone();
    // Single-instance callbacks can arrive off the UI thread, even before
    // setup creates the window. Check Quit when this queued work actually runs,
    // together with the window operations, rather than before enqueueing them.
    if let Err(error) = app.run_on_main_thread(move || {
        if handle
            .try_state::<crate::lifecycle::DesktopLifecycle>()
            .is_some_and(|lifecycle| lifecycle.is_quitting())
            || handle
                .try_state::<Arc<CoreProcessor>>()
                .is_some_and(|processor| processor.is_quitting())
        {
            return;
        }
        if let Some(window) = handle.get_webview_window("main") {
            #[cfg(target_os = "linux")]
            {
                if let Err(error) = crate::linux_activation::present(&window, token.as_deref()) {
                    tracing::warn!(%error, "Could not present Kaze");
                }
            }
            #[cfg(not(target_os = "linux"))]
            {
                let _ = token;
                if let Err(error) = window.show() {
                    tracing::warn!(%error, "Could not show Kaze");
                    return;
                }
                if let Err(error) = window.unminimize() {
                    tracing::debug!(%error, "Compositor did not unminimize Kaze");
                }
                if let Err(error) = window.set_focus() {
                    tracing::debug!(%error, "Compositor did not focus Kaze");
                }
            }
        }
    }) {
        tracing::debug!(%error, "Could not schedule opening Kaze");
    }
}

/// Sets up the persistent native system tray icon and menu.
pub fn setup_tray<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<TrayIcon<R>, Box<dyn std::error::Error>> {
    if let Some(existing) = app.tray_by_id(TRAY_ID) {
        return Ok(existing);
    }
    #[cfg(target_os = "linux")]
    {
        // libappindicator-sys panics inside an extern-C wrapper when no library
        // can be loaded, which cannot safely be caught after entering the FFI.
        // SAFETY: only the fixed platform libraries are opened; no symbols are called.
        let available = [
            "libayatana-appindicator3.so.1",
            "libappindicator3.so.1",
            "libayatana-appindicator3.so",
            "libappindicator3.so",
        ]
        .iter()
        .any(|name| unsafe { libloading::Library::new(name) }.is_ok());
        if !available {
            return Err("AppIndicator library unavailable".into());
        }
    }
    let open_item = MenuItem::with_id(app, "open", "Open Kaze", true, None::<&str>)?;
    let prev_item = MenuItem::with_id(app, "previous", "Previous", true, None::<&str>)?;
    let play_pause_item = MenuItem::with_id(app, "play_pause", "Play/Pause", true, None::<&str>)?;
    let next_item = MenuItem::with_id(app, "next", "Next", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit Kaze", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &open_item,
            &prev_item,
            &play_pause_item,
            &next_item,
            &sep,
            &quit_item,
        ],
    )?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip("Kaze")
        .show_menu_on_left_click(cfg!(target_os = "linux"));

    let icon = app
        .default_window_icon()
        .cloned()
        .or_else(|| tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png")).ok());
    if let Some(icon) = icon {
        builder = builder.icon(icon);
    }

    let tray = builder
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            match id {
                "open" => {
                    show_main_window(app);
                }
                "previous" => {
                    if let Some(processor) = app.try_state::<Arc<CoreProcessor>>() {
                        let proc = processor.inner().clone();
                        app.state::<crate::lifecycle::DesktopLifecycle>()
                            .spawn(async move {
                                if let Err(error) =
                                    proc.dispatch_command(Command::PreviousTrack).await
                                {
                                    tracing::warn!(%error, "Tray Previous failed");
                                }
                            });
                    }
                }
                "play_pause" => {
                    if let Some(processor) = app.try_state::<Arc<CoreProcessor>>() {
                        let proc = processor.inner().clone();
                        app.state::<crate::lifecycle::DesktopLifecycle>()
                            .spawn(async move {
                                if let Err(error) =
                                    proc.dispatch_command(Command::TogglePlayPause).await
                                {
                                    tracing::warn!(%error, "Tray Play/Pause failed");
                                }
                            });
                    }
                }
                "next" => {
                    if let Some(processor) = app.try_state::<Arc<CoreProcessor>>() {
                        let proc = processor.inner().clone();
                        app.state::<crate::lifecycle::DesktopLifecycle>()
                            .spawn(async move {
                                if let Err(error) = proc.dispatch_command(Command::NextTrack).await
                                {
                                    tracing::warn!(%error, "Tray Next failed");
                                }
                            });
                    }
                }
                "quit" => {
                    info!("Explicit quit requested from system tray menu");
                    crate::lifecycle::request_quit(app);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(tray)
}
