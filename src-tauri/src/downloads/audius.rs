use super::traits::DownloadProvider;
use super::types::{DownloadProgress, DownloadSearchResult, DownloadStatus};
use crate::core::error::{AppError, AppResult};
use async_trait::async_trait;
use reqwest::{header::CONTENT_TYPE, Client};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use uuid::Uuid;

const API_BASE: &str = "https://api.audius.co/v1";

/// Public Audius tracks whose artists have enabled free downloads.
pub struct AudiusProvider {
    client: Client,
    available_tracks: Arc<RwLock<HashSet<String>>>,
    tasks: Arc<RwLock<HashMap<String, DownloadProgress>>>,
}

impl AudiusProvider {
    pub fn new() -> Self {
        Self {
            client: Client::builder().build().unwrap_or_default(),
            available_tracks: Arc::new(RwLock::new(HashSet::new())),
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn search_music(&self, query: &str) -> AppResult<Vec<DownloadSearchResult>> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let response: Value = self
            .client
            .get(format!("{API_BASE}/tracks/search"))
            .query(&[
                ("query", query),
                ("only_downloadable", "true"),
                ("include_purchaseable", "false"),
                ("limit", "30"),
            ])
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Audius search failed: {e}")))?
            .error_for_status()
            .map_err(|e| AppError::Network(format!("Audius search failed: {e}")))?
            .json()
            .await
            .map_err(|e| AppError::Network(format!("Invalid Audius search response: {e}")))?;

        let results = parse_tracks(&response);
        self.available_tracks
            .write()
            .await
            .extend(results.iter().map(|track| track.id.clone()));
        Ok(results)
    }
}

impl Default for AudiusProvider {
    fn default() -> Self {
        Self::new()
    }
}

fn safe_name(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn ungated(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => true,
        Some(Value::Object(map)) => map.is_empty(),
        _ => false,
    }
}

fn parse_tracks(payload: &Value) -> Vec<DownloadSearchResult> {
    let Some(tracks) = payload.get("data").and_then(Value::as_array) else {
        return Vec::new();
    };
    tracks
        .iter()
        .filter_map(|track| {
            let id = track.get("id")?.as_str()?;
            if id.is_empty() || id.len() > 80 || !id.chars().all(|c| c.is_ascii_alphanumeric()) {
                return None;
            }
            let artist = track.pointer("/user/name")?.as_str()?.trim();
            let title = track.get("title")?.as_str()?.trim();
            if artist.is_empty()
                || title.is_empty()
                || track.get("downloadable")?.as_bool()? != true
            {
                return None;
            }
            // Audius can gate downloads behind a follow, purchase, coin, or premium tier.
            if track.get("is_download_gated").and_then(Value::as_bool) == Some(true)
                || track.get("is_stream_gated").and_then(Value::as_bool) == Some(true)
                || track.get("is_purchaseable").and_then(Value::as_bool) == Some(true)
                || track.get("is_purchasable").and_then(Value::as_bool) == Some(true)
                || track.pointer("/access/download").and_then(Value::as_bool) == Some(false)
                || !ungated(track.get("download_conditions"))
                || !ungated(track.get("stream_conditions"))
            {
                return None;
            }
            let filename = safe_name(&format!("{artist} - {title}.mp3"));
            Some(DownloadSearchResult {
                id: format!("audius_{id}"),
                provider: "audius".to_string(),
                username: artist.to_string(),
                filename,
                file_size: 0,
                bitrate: None,
                sample_rate: None,
                format: "mp3".to_string(),
                slots_free: true,
                speed_bps: 0,
            })
        })
        .collect()
}

#[async_trait]
impl DownloadProvider for AudiusProvider {
    fn name(&self) -> &'static str {
        "audius"
    }

    fn is_available(&self) -> bool {
        true
    }

    async fn search(&self, query: &str) -> AppResult<Vec<DownloadSearchResult>> {
        self.search_music(query).await
    }

    async fn start_download(
        &self,
        result: &DownloadSearchResult,
        destination_dir: &Path,
    ) -> AppResult<String> {
        if !self.available_tracks.read().await.contains(&result.id) {
            return Err(AppError::NotFound(
                "Audius result expired; search again".to_string(),
            ));
        }
        let track_id = result
            .id
            .strip_prefix("audius_")
            .ok_or_else(|| AppError::Validation("Invalid Audius result ID".to_string()))?;
        let task_id = format!("audius_task_{}", Uuid::new_v4());
        let destination = destination_dir.join(&result.filename);
        let temporary = PathBuf::from(format!("{}.{}.part", destination.display(), task_id));
        let progress = DownloadProgress {
            task_id: task_id.clone(),
            bytes_downloaded: 0,
            total_bytes: result.file_size,
            speed_bps: 0,
            status: DownloadStatus::Downloading,
        };
        self.tasks.write().await.insert(task_id.clone(), progress);

        let client = self.client.clone();
        let tasks = self.tasks.clone();
        let task = task_id.clone();
        let url = format!("{API_BASE}/tracks/{track_id}/download");
        tokio::spawn(async move {
            let outcome = async {
                let mut response = client
                    .get(url)
                    .query(&[("original", "false")])
                    .send()
                    .await
                    .map_err(|e| e.to_string())?
                    .error_for_status()
                    .map_err(|e| e.to_string())?;
                if let Some(content_type) = response.headers().get(CONTENT_TYPE) {
                    let content_type = content_type.to_str().unwrap_or("");
                    if content_type.starts_with("text/") || content_type.contains("json") {
                        return Err("Audius returned a page instead of audio".to_string());
                    }
                }
                if let Some(total) = response.content_length() {
                    if total > 250_000_000 {
                        return Err("Audius download exceeds size limit".to_string());
                    }
                    if let Some(progress) = tasks.write().await.get_mut(&task) {
                        progress.total_bytes = total as i64;
                    }
                }
                let mut output = tokio::fs::File::create(&temporary)
                    .await
                    .map_err(|e| e.to_string())?;
                while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
                    if tasks.read().await.get(&task).map(|p| p.status)
                        == Some(DownloadStatus::Cancelled)
                    {
                        return Err("cancelled".to_string());
                    }
                    output.write_all(&chunk).await.map_err(|e| e.to_string())?;
                    if let Some(progress) = tasks.write().await.get_mut(&task) {
                        progress.bytes_downloaded += chunk.len() as i64;
                        if progress.bytes_downloaded > 250_000_000 {
                            return Err("Audius download exceeds size limit".to_string());
                        }
                    }
                }
                output.flush().await.map_err(|e| e.to_string())?;
                if tasks.read().await.get(&task).map(|p| p.status)
                    == Some(DownloadStatus::Cancelled)
                {
                    return Err("cancelled".to_string());
                }
                tokio::fs::hard_link(&temporary, &destination)
                    .await
                    .map_err(|e| e.to_string())?;
                tokio::fs::remove_file(&temporary)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok::<(), String>(())
            }
            .await;
            if outcome.is_err() {
                let _ = tokio::fs::remove_file(&temporary).await;
            }
            if let Some(progress) = tasks.write().await.get_mut(&task) {
                if progress.status != DownloadStatus::Cancelled {
                    progress.status = if outcome.is_ok() {
                        DownloadStatus::Completed
                    } else {
                        DownloadStatus::Failed
                    };
                }
            }
        });
        Ok(task_id)
    }

    async fn get_progress(&self, task_id: &str) -> AppResult<Option<DownloadProgress>> {
        Ok(self.tasks.read().await.get(task_id).cloned())
    }

    async fn cancel(&self, task_id: &str) -> AppResult<()> {
        if let Some(progress) = self.tasks.write().await.get_mut(task_id) {
            progress.status = DownloadStatus::Cancelled;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_public_downloadable_tracks_are_offered() {
        let payload = json!({"data": [
            {"id":"good1","title":"Pokémon Center","user":{"name":"Kato"},"downloadable":true,"duration":180},
            {"id":"bad1","title":"Locked","user":{"name":"Kato"},"downloadable":true,"download_conditions":{"follow_user_id":"123"}},
            {"id":"bad2","title":"Paid","user":{"name":"Kato"},"downloadable":true,"is_purchaseable":true},
            {"id":"bad3","title":"Stream only","user":{"name":"Kato"},"downloadable":false}
        ]});
        let results = parse_tracks(&payload);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].filename, "Kato - Pokémon Center.mp3");
        assert_eq!(results[0].provider, "audius");
    }

    #[test]
    fn rejects_invalid_track_ids_and_names() {
        let payload = json!({"data": [
            {"id":"../bad","title":"Song","user":{"name":"Artist"},"downloadable":true},
            {"id":"good","title":"","user":{"name":"Artist"},"downloadable":true}
        ]});
        assert!(parse_tracks(&payload).is_empty());
    }
}
