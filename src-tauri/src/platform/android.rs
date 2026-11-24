//! Android shell: builds the shared backend from Tauri's path resolver,
//! registers the kaze-media plugin and bridges playback events and media
//! session commands between the plugin and the `CoreProcessor`.

use super::media_bridge::{command_for, MediaMirror};
use super::shared;
use crate::cloud::secret_store::{self, SecretStore};
use crate::config::settings::AppConfig;
use crate::core::event::Event;
use crate::core::processor::CoreProcessor;
use crate::database::init_db_pool;
use crate::library::artwork;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use tokio::sync::broadcast::error::RecvError;
use tauri_plugin_kaze_media::{KazeMedia, PlaybackSnapshot};

type Media = KazeMedia<tauri::Wry>;

/// Keystore bridge: forwards secret calls to the kaze-media plugin.
struct PluginSecrets(tauri::AppHandle);

impl PluginSecrets {
    fn media(&self) -> Result<tauri::State<'_, Media>, String> {
        self.0.try_state::<Media>().ok_or_else(|| "kaze-media not ready".to_string())
    }
}

impl SecretStore for PluginSecrets {
    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        self.media()?.secret_set(key, value)
    }
    fn get(&self, key: &str) -> Option<String> {
        self.media().ok()?.secret_get(key)
    }
    fn delete(&self, key: &str) {
        if let Ok(media) = self.media() {
            let _ = media.secret_delete(key);
        }
    }
}

/// Opens the system folder picker (SAF) and returns the chosen folder as a
/// file path the scanner can read, after making sure the app may read music.
/// `None` when cancelled. Plugin calls block until the activity result
/// arrives, so they must not run on the main thread.
#[tauri::command]
async fn android_pick_music_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let media = app.try_state::<Media>().ok_or("kaze-media not ready")?;
        let Some(uri) = media.pick_music_folder()?.uri else { return Ok(None) };
        let path = super::saf_path::tree_uri_to_path(&uri)
            .ok_or("This folder isn’t on the phone’s storage. Pick one under Internal storage or the SD card.")?;
        if !media.request_audio_access()? {
            return Err("Kaze needs permission to read your music. Allow Music and audio in Android settings.".into());
        }
        Ok(Some(path))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Makes sure the app may read music in shared storage, asking when needed.
#[tauri::command]
async fn android_request_audio_access(app: tauri::AppHandle) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.try_state::<Media>().ok_or("kaze-media not ready")?.request_audio_access()
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Opens the Android share sheet.
#[tauri::command]
async fn android_share(app: tauri::AppHandle, title: Option<String>, text: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.try_state::<Media>().ok_or("kaze-media not ready")?.share(title.as_deref(), &text)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Downloads go to Internal storage › Music › Kaze, where the Files app and
/// other players can see them and they survive an uninstall. `None` (keep
/// the app-private folder) when Android won't let Kaze write there.
fn shared_download_dir() -> Option<std::path::PathBuf> {
    let dir = std::path::Path::new(crate::library::service::ANDROID_MUSIC_DIR).join("Kaze");
    std::fs::create_dir_all(&dir).ok()?;
    // Shared folders only take audio-named files, like Kaze's own temporaries.
    let probe = dir.join(".kaze-write-check.m4a");
    std::fs::write(&probe, b"").ok()?;
    let _ = std::fs::remove_file(&probe);
    Some(dir)
}

/// Builds the config from Tauri's path resolver and points the credential
/// fallback at the app-private data directory.
fn resolve_config(app: &tauri::App) -> Result<AppConfig, tauri::Error> {
    let paths = app.path();
    let data = paths.app_data_dir()?;
    let cache = paths.app_cache_dir()?;
    let private = paths.audio_dir().unwrap_or_else(|_| data.join("Music"));
    let music = shared_download_dir().unwrap_or(private);
    std::fs::create_dir_all(&data)?;
    std::fs::create_dir_all(&cache)?;
    secret_store::set_base_dir(data.clone());
    Ok(AppConfig::with_base_dirs(data, cache, music))
}

fn snapshot_of(update: super::media_bridge::MirrorUpdate) -> PlaybackSnapshot {
    PlaybackSnapshot {
        playing: update.playing,
        title: update.title,
        artist: update.artist,
        album: None,
        duration_ms: update.duration_ms,
        position_ms: update.position_ms,
    }
}

/// Mirrors a playback event into the foreground media session.
fn mirror_to_session(app: &tauri::AppHandle, mirror: &mut MediaMirror, event: &Event) {
    let Some(update) = mirror.apply(event) else {
        return;
    };
    let Some(media) = app.try_state::<Media>() else {
        return;
    };
    if let Err(error) = media.update_playback(&snapshot_of(update)) {
        tracing::warn!(%error, "Media session update failed");
    }
}

/// Forwards backend events to the webview and mirrors playback state into
/// the foreground media session.
fn spawn_event_bridge(app: tauri::AppHandle, processor: Arc<CoreProcessor>) {
    let mut rx = processor.event_bus().subscribe();
    tauri::async_runtime::spawn(async move {
        let mut mirror = MediaMirror::default();
        loop {
            match rx.recv().await {
                Ok(event) => {
                    mirror_to_session(&app, &mut mirror, &event);
                    let _ = app.emit("backend-event", &event);
                }
                Err(RecvError::Lagged(skipped)) => {
                    tracing::warn!(skipped, "Event bus receiver lagged behind; continuing");
                }
                Err(RecvError::Closed) => break,
            }
        }
    });
}

/// Routes media-session transport commands into the processor.
fn route_media_commands(app: &tauri::AppHandle, processor: Arc<CoreProcessor>) {
    let Some(media) = app.try_state::<Media>() else {
        return;
    };
    let result = media.on_media_command(move |event| {
        let Some(command) = command_for(&event.command, event.position_ms) else {
            return;
        };
        let processor = processor.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = processor.dispatch_command(command).await {
                tracing::warn!(%error, "Media command failed");
            }
        });
    });
    if let Err(error) = result {
        tracing::warn!(%error, "Could not register media command listener");
    }
}

fn start_backend(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let config = resolve_config(app)?;
    let db_path = config.database_path.clone();
    let handle = app.handle().clone();
    app.manage(config.clone());
    tauri::async_runtime::block_on(async move {
        let pool = init_db_pool(&db_path).await?;
        let processor = Arc::new(CoreProcessor::new(pool, config));
        processor.initialize_close_behavior().await?;
        processor.playback_service().initialize_saved_playback().await;
        spawn_event_bridge(handle.clone(), processor.clone());
        route_media_commands(&handle, processor.clone());
        handle.manage(processor);
        Ok::<(), Box<dyn std::error::Error>>(())
    })
}

pub fn run() {
    crate::logging::init_logging();
    shared::init_async_runtime();
    tauri::Builder::default()
        .plugin(tauri_plugin_kaze_media::init())
        .setup(|app| {
            secret_store::set_bridge(Box::new(PluginSecrets(app.handle().clone())));
            app.manage(super::android_update::PendingUpdate::default());
            start_backend(app)?;
            super::android_update::check_on_launch(app.handle().clone());
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol(artwork::SCHEME, shared::serve_artwork)
        .invoke_handler(tauri::generate_handler![
            shared::execute_command,
            shared::execute_query,
            android_pick_music_folder,
            android_request_audio_access,
            android_share,
            super::android_update::android_check_update,
            super::android_update::android_install_update
        ])
        .run(tauri::generate_context!())
        .expect("error while running Kaze");
}
