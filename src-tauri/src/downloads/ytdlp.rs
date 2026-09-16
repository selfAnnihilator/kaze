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

        let yt_query = format!("ytsearch8:{}", clean_query);
        info!(query = %yt_query, "Executing yt-dlp search");

        let output = tokio::process::Command::new(&self.binary_path)
            .args([
                &yt_query,
                "--dump-json",
                "--flat-playlist",
                "--no-warnings",
                "--no-playlist",
            ])
            .output()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to run yt-dlp search: {}", e)))?;

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
        let video_id = if let Some(vid) = result.id.strip_prefix("ytdlp_flac_") {
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

    async fn resolve_stream_url(&self, query: &str) -> AppResult<Option<(String, f64)>> {
        let clean = query.trim();
        if clean.is_empty() {
            return Ok(None);
        }

        let yt_query = format!("ytsearch1:{}", clean);
        info!(query = %yt_query, "Resolving full song audio stream URL via yt-dlp");

        let resolver_future = tokio::process::Command::new(&self.binary_path)
            .args([
                "--print",
                "%(url)s",
                "--print",
                "%(duration)s",
                "-f",
                "bestaudio[ext=m4a]/bestaudio/best",
                "--no-warnings",
                "--no-playlist",
                &yt_query,
            ])
            .output();

        let output = match tokio::time::timeout(std::time::Duration::from_secs(12), resolver_future).await {
            Ok(res) => res.map_err(|e| AppError::Internal(format!("Failed to run yt-dlp stream resolver: {}", e)))?,
            Err(_) => {
                warn!(query = %yt_query, "yt-dlp stream resolution timed out after 12s");
                return Ok(None);
            }
        };

        if !output.status.success() {
            return Ok(None);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
        if lines.is_empty() {
            return Ok(None);
        }

        let stream_url = lines[0].to_string();
        let duration = if lines.len() > 1 {
            lines[1].parse::<f64>().unwrap_or(240.0)
        } else {
            240.0
        };

        Ok(Some((stream_url, duration)))
    }
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
