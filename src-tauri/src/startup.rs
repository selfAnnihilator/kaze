//! Login registration belongs to the OS, rather than a second database flag.
use tauri::{AppHandle, Manager};
#[cfg(not(target_os = "linux"))]
use tauri_plugin_autostart::ManagerExt;

#[cfg(target_os = "linux")]
#[path = "startup_linux.rs"]
mod platform;

pub const LOGIN_ARG: &str = "--autostart";

pub fn is_login_launch(args: &[String]) -> bool {
    args.iter().skip(1).any(|arg| arg == LOGIN_ARG)
}

#[derive(Default)]
pub struct StartupSettings(pub tokio::sync::Mutex<()>);

#[tauri::command]
pub async fn get_autostart(app: AppHandle) -> Result<bool, String> {
    let state = app.state::<StartupSettings>();
    let _guard = state.0.lock().await;
    is_enabled(&app)
}

#[tauri::command]
pub async fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let state = app.state::<StartupSettings>();
    let _guard = state.0.lock().await;
    if app
        .state::<crate::lifecycle::DesktopLifecycle>()
        .is_quitting()
    {
        return Err("Kaze is quitting. Reopen it to change login startup.".into());
    }
    #[cfg(target_os = "linux")]
    platform::set_enabled(&app, enabled)?;
    #[cfg(not(target_os = "linux"))]
    {
        let manager = app.autolaunch();
        if enabled {
            manager.enable()
        } else {
            manager.disable()
        }
        .map_err(|e| e.to_string())?;
    }
    is_enabled(&app)
}

fn is_enabled(app: &AppHandle) -> Result<bool, String> {
    #[cfg(target_os = "linux")]
    {
        let _ = app;
        platform::is_enabled()
    }
    #[cfg(not(target_os = "linux"))]
    {
        app.autolaunch().is_enabled().map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_explicit_login_argument_hides_the_window() {
        for args in [
            vec!["kaze"],
            vec!["kaze", "--autostart=false"],
            vec!["--autostart"],
        ] {
            assert!(!is_login_launch(
                &args.into_iter().map(String::from).collect::<Vec<_>>()
            ));
        }
        assert!(is_login_launch(&["kaze".into(), LOGIN_ARG.into()]));
    }
}
