#[cfg(target_os = "android")]
use crate::models::ListenerArgs;
use crate::models::{AudioAccess, MediaCommandEvent, PickedFolder, PlaybackSnapshot, SecretArgs, SecretValue, ShareArgs};
use serde::{de::DeserializeOwned, Serialize};
use tauri::plugin::PluginApi;
use tauri::{AppHandle, Runtime};

#[cfg(target_os = "android")]
const KOTLIN_PACKAGE: &str = "org.intelligentmusicplayer.kazemedia";

/// Handle to the Kotlin plugin, managed in Tauri state.
#[cfg(target_os = "android")]
pub struct KazeMedia<R: Runtime>(tauri::plugin::PluginHandle<R>);

/// Desktop placeholder so the crate type-checks everywhere; never built.
#[cfg(not(target_os = "android"))]
pub struct KazeMedia<R: Runtime>(std::marker::PhantomData<fn() -> R>);

#[cfg(target_os = "android")]
pub fn init<R: Runtime, C: serde::de::DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> Result<KazeMedia<R>, Box<dyn std::error::Error>> {
    let handle = api.register_android_plugin(KOTLIN_PACKAGE, "KazeMediaPlugin")?;
    Ok(KazeMedia(handle))
}

#[cfg(not(target_os = "android"))]
pub fn init<R: Runtime, C: serde::de::DeserializeOwned>(
    _app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> Result<KazeMedia<R>, Box<dyn std::error::Error>> {
    Err("kaze-media is only available on Android".into())
}

impl<R: Runtime> KazeMedia<R> {
    #[cfg(target_os = "android")]
    fn call<T: DeserializeOwned>(&self, cmd: &str, args: impl Serialize) -> Result<T, String> {
        self.0
            .run_mobile_plugin(cmd, args)
            .map_err(|e| e.to_string())
    }

    #[cfg(not(target_os = "android"))]
    fn call<T: DeserializeOwned>(&self, _cmd: &str, _args: impl Serialize) -> Result<T, String> {
        Err("kaze-media is only available on Android".into())
    }

    /// Mirrors Rust playback state into the media session; starts or stops
    /// the foreground service accordingly.
    pub fn update_playback(&self, snapshot: &PlaybackSnapshot) -> Result<(), String> {
        self.call::<()>("updatePlayback", snapshot)
    }

    /// Opens the system folder picker; `uri` is `None` when cancelled.
    pub fn pick_music_folder(&self) -> Result<PickedFolder, String> {
        self.call::<PickedFolder>("pickMusicFolder", ())
    }

    /// Asks for permission to read music in shared storage; `true` when granted.
    pub fn request_audio_access(&self) -> Result<bool, String> {
        self.call::<AudioAccess>("requestAudioAccess", ()).map(|a| a.granted)
    }

    /// Opens the Android share sheet with `text`.
    pub fn share(&self, title: Option<&str>, text: &str) -> Result<(), String> {
        self.call::<()>("share", ShareArgs { title, text })
    }

    pub fn secret_set(&self, key: &str, value: &str) -> Result<(), String> {
        let args = SecretArgs {
            key,
            value: Some(value),
        };
        self.call::<()>("secretSet", args)
    }

    pub fn secret_get(&self, key: &str) -> Option<String> {
        let args = SecretArgs { key, value: None };
        self.call::<SecretValue>("secretGet", args)
            .ok()
            .and_then(|v| v.value)
    }

    pub fn secret_delete(&self, key: &str) -> Result<(), String> {
        let args = SecretArgs { key, value: None };
        self.call::<()>("secretDelete", args)
    }

    /// Delivers media-session transport commands to `handler` (Android only).
    #[cfg(target_os = "android")]
    pub fn on_media_command(
        &self,
        handler: impl Fn(MediaCommandEvent) + Send + Sync + 'static,
    ) -> Result<(), String> {
        let channel = tauri::ipc::Channel::<serde_json::Value>::new(move |body| {
            if let Ok(event) = body.deserialize::<MediaCommandEvent>() {
                handler(event);
            }
            Ok(())
        });
        let args = ListenerArgs {
            event: "mediaCommand",
            handler: channel,
        };
        self.call::<()>("registerListener", args)
    }

    #[cfg(not(target_os = "android"))]
    pub fn on_media_command(
        &self,
        _handler: impl Fn(MediaCommandEvent) + Send + Sync + 'static,
    ) -> Result<(), String> {
        Err("kaze-media is only available on Android".into())
    }
}
