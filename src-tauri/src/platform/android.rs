//! Android shell. Stub: wires the shared IPC and artwork handler and resolves
//! base directories and registers the kaze-media plugin; the processor setup
//! and event bridge follow in later steps.

use super::shared;
use crate::cloud::secret_store;
use crate::config::settings::AppConfig;
use crate::library::artwork;
use tauri::Manager;

/// Builds the config from Tauri's path resolver and points the credential
/// fallback at the app-private data directory.
fn resolve_config(app: &tauri::App) -> Result<AppConfig, tauri::Error> {
    let paths = app.path();
    let data = paths.app_data_dir()?;
    let cache = paths.app_cache_dir()?;
    let music = paths.audio_dir().unwrap_or_else(|_| data.join("Music"));
    std::fs::create_dir_all(&data)?;
    std::fs::create_dir_all(&cache)?;
    secret_store::set_base_dir(data.clone());
    Ok(AppConfig::with_base_dirs(data, cache, music))
}

pub fn run() {
    shared::init_async_runtime();
    tauri::Builder::default()
        .plugin(tauri_plugin_kaze_media::init())
        .setup(|app| {
            let config = resolve_config(app)?;
            app.manage(config);
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol(artwork::SCHEME, shared::serve_artwork)
        .invoke_handler(tauri::generate_handler![
            shared::execute_command,
            shared::execute_query
        ])
        .run(tauri::generate_context!())
        .expect("error while running Kaze");
}
