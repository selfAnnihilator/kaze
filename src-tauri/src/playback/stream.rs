use crate::core::error::{AppError, AppResult};
use crate::downloads::DownloadService;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Manages streaming audio resolution, disk-caching, and local playback file preparation.
pub struct StreamPlaybackManager {
    cache_dir: PathBuf,
    pool: Option<SqlitePool>,
    download_service: Option<Arc<DownloadService>>,
    http_client: reqwest::Client,
}

impl StreamPlaybackManager {
    pub fn new(
        cache_dir: PathBuf,
        pool: Option<SqlitePool>,
        download_service: Option<Arc<DownloadService>>,
    ) -> Self {
        let stream_cache_dir = cache_dir.join("stream_cache");
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .connect_timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            cache_dir: stream_cache_dir,
            pool,
            download_service,
            http_client,
        }
    }

    /// Computes the disk cache path for a stream identifier or URL.
    pub fn get_cache_path(&self, key: &str) -> PathBuf {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        self.cache_dir.join(format!("{}.audio", hash))
    }

    /// Resolves an audio track to a playable local file path and duration.
    ///
    /// Checks in order:
    /// 1. Local matched file on disk (from library or downloads).
    /// 2. Stream cache on disk.
    /// 3. Remote full-audio stream resolution (via yt-dlp / composite provider).
    /// 4. Remote preview URL fallback (via iTunes / Deezer / Audius).
    pub async fn resolve_and_prepare_audio(
        &self,
        track_id: &str,
        title: &str,
        artist: &str,
        preview_url: Option<&str>,
    ) -> AppResult<(PathBuf, f64)> {
        info!(track_id, title, artist, "Resolving stream audio for playback");

        // 1. Check if there is an already-matched local track in the database
        if let Some(pool) = &self.pool {
            let local_row: Option<(Option<String>,)> = sqlx::query_as(
                "SELECT matched_local_track_id FROM external_tracks WHERE id = ?",
            )
            .bind(track_id)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

            if let Some((Some(matched_id),)) = local_row {
                if !matched_id.is_empty() && !matched_id.starts_with("online:") && !matched_id.starts_with("itunes:") {
                    let path_row: Option<(String, f64, String)> = sqlx::query_as(
                        "SELECT file_path, duration_secs, format FROM tracks WHERE id = ?",
                    )
                    .bind(&matched_id)
                    .fetch_optional(pool)
                    .await
                    .unwrap_or(None);

                    if let Some((file_path, dur, format)) = path_row {
                        let path = PathBuf::from(&file_path);
                        if format != "online" && path.exists() {
                            info!(%matched_id, path = %file_path, "Using matched local library file for online track");
                            return Ok((path, dur));
                        }
                    }
                }
            }
        }

        // 2. Check if a downloaded audio file exists in the download directory
        if let Some(ref dl) = self.download_service {
            let dl_dir = dl.download_dir();
            let sanitized_title = sanitize_component(title);
            let sanitized_artist = sanitize_component(artist);

            for ext in &["mp3", "flac", "m4a", "ogg", "opus"] {
                let candidate1 = dl_dir.join(format!("{} - {}.{}", sanitized_artist, sanitized_title, ext));
                if candidate1.exists() {
                    info!(path = %candidate1.display(), "Using downloaded file from download folder");
                    return Ok((candidate1, 210.0));
                }
                let candidate2 = dl_dir.join(format!("{}.{}", sanitized_title, ext));
                if candidate2.exists() {
                    info!(path = %candidate2.display(), "Using downloaded file from download folder");
                    return Ok((candidate2, 210.0));
                }
            }
        }

        // 3. Check if cached in stream disk cache
        let cache_key = format!("{}:{}:{}", track_id, artist.trim().to_lowercase(), title.trim().to_lowercase());
        let cache_path = self.get_cache_path(&cache_key);

        if cache_path.exists() {
            if let Ok(meta) = std::fs::metadata(&cache_path) {
                if meta.len() > 8192 {
                    debug!(path = %cache_path.display(), "Found valid cached audio stream");
                    // Read duration from external_tracks or default
                    let dur = self.get_known_duration(track_id).await.unwrap_or(210.0);
                    return Ok((cache_path, dur));
                }
            }
        }

        // 4. Resolve stream URL
        let mut target_stream_url: Option<String> = None;
        let mut target_duration: f64 = 210.0;

        // Try resolving full track audio first if provider is available
        if let Some(ref dl) = self.download_service {
            match dl.resolve_full_track_audio(artist, title).await {
                Ok(Some((stream_url, dur, _))) => {
                    info!(%stream_url, dur, "Resolved full track stream URL");
                    target_stream_url = Some(stream_url);
                    target_duration = dur;
                }
                Ok(None) => {
                    debug!("Full track stream not resolved by provider, falling back");
                }
                Err(e) => {
                    warn!(error = %e, "Error resolving full track stream from download service");
                }
            }
        }

        // Fallback to preview_url if full audio stream wasn't resolved
        if target_stream_url.is_none() {
            if let Some(prev) = preview_url.filter(|p| !p.trim().is_empty()) {
                target_stream_url = Some(prev.to_string());
                target_duration = 30.0;
            } else if let Some(pool) = &self.pool {
                let db_prev: Option<(Option<String>, Option<f64>)> = sqlx::query_as(
                    "SELECT preview_url, duration_secs FROM external_tracks WHERE id = ?",
                )
                .bind(track_id)
                .fetch_optional(pool)
                .await
                .unwrap_or(None);

                if let Some((Some(p), dur)) = db_prev {
                    if !p.trim().is_empty() {
                        target_stream_url = Some(p);
                        target_duration = dur.unwrap_or(30.0);
                    }
                }
            }
        }

        let stream_url = target_stream_url.ok_or_else(|| {
            AppError::Playback(format!(
                "No streamable audio source available for \"{}\" by \"{}\"",
                title, artist
            ))
        })?;

        // 5. Download and cache the stream
        info!(url = %stream_url, target = %cache_path.display(), "Downloading audio stream to cache");
        self.download_and_cache(&stream_url, &cache_path).await?;

        Ok((cache_path, target_duration))
    }

    async fn download_and_cache(&self, url: &str, target_path: &Path) -> AppResult<()> {
        tokio::fs::create_dir_all(&self.cache_dir)
            .await
            .map_err(|e| AppError::Io(e.to_string()))?;

        let part_path = target_path.with_extension("part");

        let response = self
            .http_client
            .get(url)
            .header(
                reqwest::header::USER_AGENT,
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36",
            )
            .send()
            .await
            .map_err(|e| AppError::Playback(format!("Failed to connect to audio stream: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::Playback(format!(
                "Audio stream returned HTTP {}",
                response.status()
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| AppError::Playback(format!("Failed to read stream bytes: {}", e)))?;

        if bytes.is_empty() {
            return Err(AppError::Playback("Audio stream was empty".to_string()));
        }

        tokio::fs::write(&part_path, &bytes)
            .await
            .map_err(|e| AppError::Io(e.to_string()))?;

        tokio::fs::rename(&part_path, target_path)
            .await
            .map_err(|e| AppError::Io(e.to_string()))?;

        info!(bytes = bytes.len(), path = %target_path.display(), "Successfully cached stream");
        Ok(())
    }

    async fn get_known_duration(&self, track_id: &str) -> Option<f64> {
        let pool = self.pool.as_ref()?;
        let row: Option<(Option<f64>,)> = sqlx::query_as(
            "SELECT duration_secs FROM external_tracks WHERE id = ?",
        )
        .bind(track_id)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        row.and_then(|(d,)| d).filter(|&d| d > 0.0)
    }
}

fn sanitize_component(s: &str) -> String {
    s.chars()
        .map(|c| if c == '/' || c == '\\' || c == ':' || c == '*' || c == '?' || c == '"' || c == '<' || c == '>' || c == '|' {
            '_'
        } else {
            c
        })
        .collect()
}
