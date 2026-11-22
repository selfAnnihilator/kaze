pub mod audio_features;
pub mod cloud;
pub mod config;
pub mod core;
pub mod database;
pub mod discovery;
pub mod downloads;
pub mod history;
pub mod library;
pub mod logging;
#[cfg(target_os = "linux")]
pub mod mpris;
pub mod platform;
pub mod playback;
pub mod profile;
pub mod providers;
pub mod ranking;
pub mod recommendations;

pub use config::AppConfig;
pub use core::{
    AppError, AppResult, Command, CommandResponse, CoreProcessor, Event, EventBus, Query,
    QueryResponse,
};

/// Starts the Kaze shell for the current platform.
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub fn run() {
    platform::desktop::run();
}

/// Starts the Kaze shell for the current platform.
#[cfg(target_os = "android")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    platform::android::run();
}
