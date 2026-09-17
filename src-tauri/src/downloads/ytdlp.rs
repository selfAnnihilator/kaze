use super::traits::DownloadProvider;
use super::types::{DownloadProgress, DownloadSearchResult, DownloadStatus};
use crate::core::error::{AppError, AppResult};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Clone)]
struct ActiveTask {
    progress: DownloadProgress,
}

/// Download provider powered by the local yt-dlp CLI tool and ffmpeg.
pub struct YtDlpProvider {
    tasks: Arc<RwLock<HashMap<String, ActiveTask>>>,
    binary_path: PathBuf,
}

impl YtDlpProvider {
    pub fn new() -> Self {
        let possible = ["/usr/bin/yt-dlp", "/usr/local/bin/yt-dlp"];
        let mut binary = PathBuf::from("yt-dlp");
        for p in possible {
            if Path::new(p).exists() {
                binary = PathBuf::from(p);
                break;
            }
        }

        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            binary_path: binary,
        }
    }
}

impl Default for YtDlpProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DownloadProvider for YtDlpProvider {
    fn name(&self) -> &'static str {
        "yt-dlp"
    }

    fn is_available(&self) -> bool {
        self.binary_path.exists()
            || std::process::Command::new(&self.binary_path)
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
    }

    async fn search(&self, query: &str) -> AppResult<Vec<DownloadSearchResult>> {
        let clean_query = query.trim();
        if clean_query.is_empty() {
            return Ok(Vec::new());
        }

        // The first few hits for niche titles are often videos mentioning the song's words.
        // Give the client-side song matcher a wider pool of actual track uploads.
        let yt_query = format!("ytsearch30:{}", clean_query);
        info!(query = %yt_query, "Executing yt-dlp search");

        let mut command = tokio::process::Command::new(&self.binary_path);
        command.args([
            &yt_query,
            "--dump-json",
            "--flat-playlist",
            "--no-warnings",
            "--no-playlist",
        ]);
        command.kill_on_drop(true);
        let output = match tokio::time::timeout(std::time::Duration::from_secs(20), command.output()).await {
            Ok(output) => output
                .map_err(|e| AppError::Internal(format!("Failed to run yt-dlp search: {}", e)))?,
            Err(_) => {
                warn!(query = %yt_query, "yt-dlp search timed out");
                return Ok(Vec::new());
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(stderr = %stderr, "yt-dlp search returned non-zero status");
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut results = Vec::new();

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Ok(val) = serde_json::from_str::<Value>(line) {
                let vid_id = match val.get("id").and_then(|v| v.as_str()) {
                    Some(id) => id.to_string(),
                    None => continue,
                };

                let raw_title = val
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Title");
                let uploader = val
                    .get("uploader")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Artist");
                let duration_secs = val
                    .get("duration")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(180.0) as i64;

                // Filter out non-song items (vlogs, walking tours, 1hr+ full albums, unboxings, short clips)
                if duration_secs > 720 || duration_secs < 30 {
                    continue;
                }

                let lower_title = raw_title.to_lowercase();
                if lower_title.contains("tour")
                    || lower_title.contains("vlog")
                    || lower_title.contains("unboxing")
                    || lower_title.contains("reaction")
                    || lower_title.contains("podcast")
                    || lower_title.contains("gameplay")
                    || lower_title.contains("figure")
                {
                    continue;
                }

                // 1. Lossless FLAC stream option (Audiophile 24-bit/48kHz)
                let flac_filename =
                    sanitize_filename(&format!("{} - {}.flac", uploader, raw_title));
                results.push(DownloadSearchResult {
                    id: format!("ytdlp_flac_{}", vid_id),
                    provider: "yt-dlp".to_string(),
                    username: uploader.to_string(),
                    filename: flac_filename,
                    file_size: duration_secs * 120_000,
                    bitrate: None,
                    sample_rate: Some(48000),
                    format: "flac".to_string(),
                    slots_free: true,
                    speed_bps: 20_000_000,
                });

                // 2. High-speed 320kbps MP3 option (Standard High Quality)
                let mp3_filename =
                    sanitize_filename(&format!("{} - {}.mp3", uploader, raw_title));
                results.push(DownloadSearchResult {
                    id: format!("ytdlp_mp3_{}", vid_id),
                    provider: "yt-dlp".to_string(),
                    username: uploader.to_string(),
                    filename: mp3_filename,
                    file_size: duration_secs * 40_000,
                    bitrate: Some(320),
                    sample_rate: Some(44100),
                    format: "mp3".to_string(),
                    slots_free: true,
                    speed_bps: 15_000_000,
                });
            }
        }

        info!(count = results.len(), "yt-dlp search completed");
        Ok(results)
    }

    async fn start_download(
        &self,
        result: &DownloadSearchResult,
        destination_dir: &Path,
    ) -> AppResult<String> {
        let is_flac = result.id.starts_with("ytdlp_flac_") || result.format.to_lowercase() == "flac";
        let video_id = if let Some(vid) = result.id.strip_prefix("ytdlp_stream_mp3_") {
            vid.to_string()
        } else if let Some(vid) = result.id.strip_prefix("ytdlp_flac_") {
            vid.to_string()
        } else if let Some(vid) = result.id.strip_prefix("ytdlp_mp3_") {
            vid.to_string()
        } else if let Some(vid) = result.id.strip_prefix("ytdlp_") {
            vid.to_string()
        } else {
            result.id.clone()
        };
        let task_id = Uuid::new_v4().to_string();

        let clean_filename = sanitize_filename(&result.filename);
        let dest_stem = Path::new(&clean_filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("track");

        let out_template = destination_dir.join(format!("{}.%(ext)s", dest_stem));

        let initial_progress = DownloadProgress {
            task_id: task_id.clone(),
            bytes_downloaded: 0,
            total_bytes: result.file_size,
            speed_bps: 0,
            status: DownloadStatus::Downloading,
        };

        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(
                task_id.clone(),
                ActiveTask {
                    progress: initial_progress,
                },
            );
        }

        let tasks_map = self.tasks.clone();
        let binary_path = self.binary_path.clone();
        let task_id_clone = task_id.clone();
        let total_expected = result.file_size;

        tokio::spawn(async move {
            let url = format!("https://www.youtube.com/watch?v={}", video_id);
            let mut cmd = tokio::process::Command::new(&binary_path);
            let mut args = vec![
                url.as_str(),
                "-x",
                "--audio-format",
                if is_flac { "flac" } else { "mp3" },
                "--audio-quality",
                "0",
                "--add-metadata",
                "--no-playlist",
                "--newline",
            ];
            if !is_flac {
                args.push("--embed-thumbnail");
            }
            args.push("-o");
            cmd.args(&args);
            cmd.arg(&out_template);
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());

            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    error!(error = %e, "Failed to spawn yt-dlp download process");
                    let mut tasks = tasks_map.write().await;
                    if let Some(t) = tasks.get_mut(&task_id_clone) {
                        t.progress.status = DownloadStatus::Failed;
                    }
                    return;
                }
            };

            let stdout = child.stdout.take();

            if let Some(stdout) = stdout {
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let line = line.trim();
                    if line.starts_with("[download]") && line.contains('%') {
                        if let Some(pct) = parse_percent(line) {
                            let mut tasks = tasks_map.write().await;
                            if let Some(t) = tasks.get_mut(&task_id_clone) {
                                let downloaded =
                                    ((pct / 100.0) * (total_expected as f64)).round() as i64;
                                t.progress.bytes_downloaded = downloaded.min(total_expected);
                                t.progress.status = DownloadStatus::Downloading;
                            }
                        }
                    }
                }
            }

            let status = child.wait().await;
            let mut tasks = tasks_map.write().await;
            if let Some(t) = tasks.get_mut(&task_id_clone) {
                match status {
                    Ok(s) if s.success() => {
                        t.progress.bytes_downloaded = t.progress.total_bytes;
                        t.progress.status = DownloadStatus::Completed;
                        info!(task_id = %task_id_clone, "yt-dlp download completed successfully");
                    }
                    Ok(s) => {
                        error!(code = ?s.code(), "yt-dlp process finished with error");
                        t.progress.status = DownloadStatus::Failed;
                    }
                    Err(e) => {
                        error!(error = %e, "yt-dlp process wait error");
                        t.progress.status = DownloadStatus::Failed;
                    }
                }
            }
        });

        Ok(task_id)
    }

    async fn get_progress(&self, provider_task_id: &str) -> AppResult<Option<DownloadProgress>> {
        let tasks = self.tasks.read().await;
        Ok(tasks.get(provider_task_id).map(|t| t.progress.clone()))
    }

    async fn cancel(&self, provider_task_id: &str) -> AppResult<()> {
        let mut tasks = self.tasks.write().await;
        if let Some(t) = tasks.get_mut(provider_task_id) {
            t.progress.status = DownloadStatus::Cancelled;
        }
        Ok(())
    }

    async fn resolve_stream_url(&self, query: &str) -> AppResult<Option<(String, f64, Option<DownloadSearchResult>)>> {
        Ok(self.resolve_stream_urls(query).await?.into_iter().next())
    }

    async fn resolve_stream_urls(&self, query: &str) -> AppResult<Vec<(String, f64, Option<DownloadSearchResult>)>> {
        let clean = query.trim();
        if clean.is_empty() {
            return Ok(Vec::new());
        }

        let yt_query = format!("ytsearch5:{}", clean);
        info!(query = %yt_query, "Resolving full song candidates via yt-dlp");

        let mut command = tokio::process::Command::new(&self.binary_path);
        command.kill_on_drop(true);
        let resolver_future = command.args([
                "--print",
                "%(url)s\t%(duration)s\t%(id)s\t%(title)s\t%(uploader)s",
                "-f",
                "bestaudio[ext=m4a]/bestaudio/best",
                "--no-warnings",
                "--no-playlist",
                &yt_query,
            ])
            .output();

        let output = match tokio::time::timeout(std::time::Duration::from_secs(25), resolver_future).await {
            Ok(res) => res.map_err(|e| AppError::Internal(format!("Failed to run yt-dlp stream resolver: {}", e)))?,
            Err(_) => {
                warn!(query = %yt_query, "yt-dlp stream resolution timed out after 25s");
                return Ok(Vec::new());
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let candidates = stdout.lines().filter_map(parse_playback_candidate).take(5).collect();
        if !output.status.success() {
            warn!(status = %output.status, "yt-dlp returned an error while resolving full song candidates");
        }
        Ok(candidates)
    }
}

fn parse_playback_candidate(line: &str) -> Option<(String, f64, Option<DownloadSearchResult>)> {
    let parts: Vec<&str> = line.trim().splitn(5, '\t').collect();
    if parts.len() != 5 { return None; }
    let stream_url = parts[0].trim();
    if !stream_url.starts_with("https://") && !stream_url.starts_with("http://") { return None; }
    let duration = parts[1].parse::<f64>().ok()?;
    if !duration.is_finite() || !(60.0..=900.0).contains(&duration) { return None; }
    let result = playback_download_result(&[stream_url, parts[1], parts[2], parts[3], parts[4]], duration);
    Some((stream_url.to_string(), duration, result))
}

#[cfg(test)]
mod full_candidate_tests {
    use super::parse_playback_candidate;

    #[test]
    fn rejects_short_clips_and_retains_full_song_sources() {
        let short = "https://example.com/short.m4a\t30\tabcdefghijk\tSong\tArtist";
        let full = "https://example.com/full.m4a\t220\tabcdefghijk\tSong\tArtist";
        assert!(parse_playback_candidate(short).is_none());
        let (url, duration, source) = parse_playback_candidate(full).unwrap();
        assert_eq!(url, "https://example.com/full.m4a");
        assert_eq!(duration, 220.0);
        assert_eq!(source.unwrap().id, "ytdlp_stream_mp3_abcdefghijk");
    }
}

fn playback_download_result(lines: &[&str], duration: f64) -> Option<DownloadSearchResult> {
    let id = *lines.get(2)?;
    if id.len() != 11 || !id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_') {
        return None;
    }
    let title = lines.get(3)?.trim();
    let uploader = lines.get(4)?.trim();
    if title.is_empty() || uploader.is_empty() || title == "NA" || uploader == "NA" {
        return None;
    }
    Some(DownloadSearchResult {
        id: format!("ytdlp_stream_mp3_{id}"),
        provider: "yt-dlp".to_string(),
        username: uploader.to_string(),
        filename: sanitize_filename(&format!("{uploader} - {title}.mp3")),
        file_size: (duration.clamp(0.0, 7200.0) * 40_000.0) as i64,
        bitrate: None,
        sample_rate: None,
        format: "mp3".to_string(),
        slots_free: true,
        speed_bps: 0,
    })
}

/// Helper parsing percentage float from yt-dlp stdout line: "[download]  42.4% of ..."
fn parse_percent(line: &str) -> Option<f64> {
    let after_download = line.strip_prefix("[download]")?.trim();
    let pct_str = after_download.split('%').next()?.trim();
    pct_str.parse::<f64>().ok()
}

/// Clean filename of unsafe filesystem characters
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod playback_source_tests {
    use super::playback_download_result;

    #[test]
    fn resolved_video_keeps_its_real_title_and_stable_id() {
        let lines = ["https://audio.example/stream", "180", "aB0_-123456", "Littleroot Town (Piano)", "Kato"];
        let result = playback_download_result(&lines, 180.0).expect("video metadata");
        assert_eq!(result.id, "ytdlp_stream_mp3_aB0_-123456");
        assert_eq!(result.filename, "Kato - Littleroot Town (Piano).mp3");
        assert!(playback_download_result(&lines[..2], 180.0).is_none());
        let invalid = [lines[0], lines[1], "../bad-id", lines[3], lines[4]];
        assert!(playback_download_result(&invalid, 180.0).is_none());
    }
}
