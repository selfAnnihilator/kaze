//! The org.mpris.MediaPlayer2 root interface.
use super::*;

/// Root interface implementation: org.mpris.MediaPlayer2
pub struct MprisRoot {
    app_handle: Option<tauri::AppHandle>,
}

impl MprisRoot {
    pub fn new(app_handle: Option<tauri::AppHandle>) -> Self {
        Self { app_handle }
    }
}

#[interface(name = "org.mpris.MediaPlayer2")]
impl MprisRoot {
    /// Brings the media player's user interface to the front.
    async fn raise(&self) -> zbus::fdo::Result<()> {
        if let Some(ref handle) = self.app_handle {
            use tauri::Manager;
            if let Some(window) = handle.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        Ok(())
    }

    /// Causes the media player to close immediately.
    async fn quit(&self) -> zbus::fdo::Result<()> {
        if let Some(ref handle) = self.app_handle {
            handle.exit(0);
        }
        Ok(())
    }

    #[zbus(property)]
    fn can_quit(&self) -> bool {
        self.app_handle.is_some()
    }

    #[zbus(property)]
    fn can_raise(&self) -> bool {
        self.app_handle.is_some()
    }

    #[zbus(property)]
    fn has_track_list(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn identity(&self) -> &'static str {
        "Kaze"
    }

    #[zbus(property)]
    fn desktop_entry(&self) -> &'static str {
        "kaze"
    }

    #[zbus(property)]
    fn supported_uri_schemes(&self) -> Vec<String> {
        // OpenUri is unsupported; these properties describe that method, not
        // formats the internal playback engine can decode.
        Vec::new()
    }

    #[zbus(property)]
    fn supported_mime_types(&self) -> Vec<String> {
        Vec::new()
    }
}
