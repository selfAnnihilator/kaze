//! Kaze Android media bridge. The plugin never plays audio: Rust's
//! `PlaybackService` stays the only player. Kotlin mirrors its state into a
//! Media3 session (notification, lock screen, media buttons) and forwards
//! transport commands back as `mediaCommand` events.

use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager, Runtime};

mod mobile;
mod models;

pub use mobile::KazeMedia;
pub use models::{PickedFolder, PlaybackSnapshot};

pub const PLUGIN_NAME: &str = "kaze-media";

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new(PLUGIN_NAME)
        .setup(|app, api| {
            let media = mobile::init(app, api)?;
            app.manage(media);
            Ok(())
        })
        .build()
}
