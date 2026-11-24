//! Android in-app updater: checks GitHub for a newer release on launch (and
//! from Settings), shows the update banner through the same
//! `update-available` event as the PC, then downloads the APK, verifies its
//! SHA-256 and has the kaze-media plugin install it in place. Android closes
//! Kaze to replace it; with "Display over other apps" allowed the new version
//! reopens itself (kaze-media UpdateRelaunch), otherwise the user reopens it.

use super::update_manifest::{is_newer, means_no_release, parse_manifest, AndroidRelease, MANIFEST_URL, UNREACHABLE};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};
use tauri_plugin_kaze_media::KazeMedia;

/// The newest release found by the last check.
#[derive(Default)]
pub struct PendingUpdate(Mutex<Option<AndroidRelease>>);

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(concat!("Kaze/", env!("CARGO_PKG_VERSION"), " (android)"))
        .timeout(Duration::from_secs(600))
        .build()
        .map_err(|e| e.to_string())
}

/// Debug builds can point at a local test manifest (`KAZE_UPDATE_URL` at build time).
fn test_manifest() -> Option<&'static str> {
    if cfg!(debug_assertions) { option_env!("KAZE_UPDATE_URL") } else { None }
}

fn manifest_url() -> &'static str {
    test_manifest().unwrap_or(MANIFEST_URL)
}

/// The latest Android release, or `None` when the latest release has no APK.
async fn fetch_release() -> Result<Option<AndroidRelease>, String> {
    let unreachable = |e: reqwest::Error| {
        tracing::warn!("Update check failed: {}", e);
        UNREACHABLE.to_string()
    };
    let response = client()?.get(manifest_url()).send().await.map_err(unreachable)?;
    if means_no_release(response.status().as_u16()) {
        return Ok(None);
    }
    let body = response.error_for_status().map_err(unreachable)?.text().await.map_err(unreachable)?;
    parse_manifest(&body, test_manifest().is_some()).map(Some)
}

/// Looks for a newer release; shows the banner and remembers it when there is one.
pub async fn check(app: &tauri::AppHandle) -> Result<Option<String>, String> {
    let Some(release) = fetch_release().await? else { return Ok(None) };
    let current = app.package_info().version.to_string();
    if !is_newer(&release.version, &current) {
        return Ok(None);
    }
    let version = release.version.clone();
    *app.state::<PendingUpdate>().0.lock().map_err(|e| e.to_string())? = Some(release);
    let _ = app.emit("update-available", serde_json::json!({ "version": version }));
    Ok(Some(version))
}

/// Relays an install that failed after it started to the UI as `update-failed`.
fn relay_install_failures(app: &tauri::AppHandle) {
    let Some(media) = app.try_state::<KazeMedia<tauri::Wry>>() else { return };
    let handle = app.clone();
    if let Err(error) = media.on_update_failed(move |message| {
        let _ = handle.emit("update-failed", serde_json::json!({ "message": message }));
    }) {
        tracing::warn!(%error, "Could not listen for update failures");
    }
}

/// The silent check a few seconds after launch (like the PC).
pub fn check_on_launch(app: tauri::AppHandle) {
    relay_install_failures(&app);
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        if let Err(error) = check(&app).await {
            tracing::warn!(%error, "Update check failed");
        }
    });
}

/// Streams the APK to `dest`, hashing as it goes and reporting `update-progress`
/// (0–100) about four times a second. Returns the SHA-256 (hex).
async fn download(app: &tauri::AppHandle, release: &AndroidRelease, dest: &std::path::Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    use tokio::io::AsyncWriteExt;
    let mut response = client()?.get(&release.url).send().await.map_err(|e| e.to_string())?
        .error_for_status().map_err(|e| e.to_string())?;
    let total = response.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(dest).await.map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let (mut done, mut last) = (0u64, Instant::now());
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        hasher.update(&chunk);
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        done += chunk.len() as u64;
        if total > 0 && last.elapsed() >= Duration::from_millis(250) {
            last = Instant::now();
            let _ = app.emit("update-progress", serde_json::json!({ "percent": done * 100 / total }));
        }
    }
    file.sync_all().await.map_err(|e| e.to_string())?;
    let _ = app.emit("update-progress", serde_json::json!({ "percent": 100 }));
    Ok(format!("{:x}", hasher.finalize()))
}

/// Settings › Check for updates: the newer version, or `None` when up to date.
#[tauri::command]
pub async fn android_check_update(app: tauri::AppHandle) -> Result<Option<String>, String> {
    check(&app).await
}

/// Downloads, verifies and installs the update found by the last check.
#[tauri::command]
pub async fn android_install_update(app: tauri::AppHandle) -> Result<(), String> {
    let pending = app.state::<PendingUpdate>().0.lock().map_err(|e| e.to_string())?.clone();
    let release = match pending {
        Some(release) => release,
        None => fetch_release().await?.ok_or("Kaze is up to date")?,
    };
    let dir = app.path().app_cache_dir().map_err(|e| e.to_string())?.join("update");
    tokio::fs::create_dir_all(&dir).await.map_err(|e| e.to_string())?;
    let apk = dir.join("Kaze.apk");
    let sha = download(&app, &release, &apk).await?;
    if !sha.eq_ignore_ascii_case(release.sha256.trim()) {
        let _ = tokio::fs::remove_file(&apk).await;
        return Err("The downloaded update is damaged; try again".into());
    }
    let path = apk.to_string_lossy().into_owned();
    tauri::async_runtime::spawn_blocking(move || {
        app.try_state::<KazeMedia<tauri::Wry>>().ok_or("kaze-media not ready")?.install_apk(&path)
    })
    .await
    .map_err(|e| e.to_string())?
}
