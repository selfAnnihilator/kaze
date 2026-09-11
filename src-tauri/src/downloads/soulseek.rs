use super::traits::DownloadProvider;
use super::types::{DownloadProgress, DownloadSearchResult, DownloadStatus};
use crate::core::error::{AppError, AppResult};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;
use tracing::debug;

pub struct SoulseekProvider {
    base_url: String,
    api_key: Option<String>,
    client: Client,
}

impl SoulseekProvider {
    pub fn new(host: &str, port: u16, api_key: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            base_url: format!("http://{}:{}/api/v0", host, port),
            api_key,
            client,
        }
    }

    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(ref key) = self.api_key {
            req.header("X-API-Key", key)
        } else {
            req
        }
    }
}

#[derive(Serialize)]
struct SlskdSearchRequest<'a> {
    #[serde(rename = "searchText")]
    search_text: &'a str,
}

#[derive(Deserialize)]
struct SlskdSearchResponse {
    id: String,
}

#[derive(Deserialize)]
struct SlskdSearchPollResponse {
    #[serde(default)]
    files: Vec<SlskdSearchFile>,
}

#[derive(Deserialize)]
struct SlskdSearchFile {
    username: String,
    filename: String,
    size: i64,
    #[serde(rename = "bitRate")]
    bit_rate: Option<u32>,
    #[serde(rename = "sampleRate")]
    sample_rate: Option<u32>,
    #[serde(rename = "hasFreeUploadSlot")]
    has_free_upload_slot: Option<bool>,
    #[serde(rename = "uploadSpeed")]
    upload_speed: Option<u64>,
}

#[derive(Serialize)]
struct SlskdDownloadItem<'a> {
    filename: &'a str,
    size: i64,
}

#[async_trait]
impl DownloadProvider for SoulseekProvider {
    fn name(&self) -> &'static str {
        "soulseek"
    }

    fn is_available(&self) -> bool {
        // Quick synchronous check or default true when configured
        true
    }

    async fn search(&self, query: &str) -> AppResult<Vec<DownloadSearchResult>> {
        let url = format!("{}/search", self.base_url);
        let req = self.client.post(&url).json(&SlskdSearchRequest {
            search_text: query,
        });

        let resp = match self.apply_auth(req).send().await {
            Ok(r) => r,
            Err(e) => {
                debug!(error = %e, "Slskd daemon not responding or unreachable");
                return Ok(Vec::new());
            }
        };

        if resp.status() != StatusCode::OK {
            return Ok(Vec::new());
        }

        let search_init: SlskdSearchResponse = resp
            .json()
            .await
            .map_err(|e| AppError::Network(format!("Invalid Slskd search response: {}", e)))?;

        // Poll search results for initial batch
        tokio::time::sleep(Duration::from_millis(1500)).await;

        let poll_url = format!("{}/search/{}", self.base_url, search_init.id);
        let poll_req = self.apply_auth(self.client.get(&poll_url));

        let poll_resp = match poll_req.send().await {
            Ok(r) if r.status() == StatusCode::OK => r,
            _ => return Ok(Vec::new()),
        };

        let data: SlskdSearchPollResponse = poll_resp
            .json()
            .await
            .unwrap_or(SlskdSearchPollResponse { files: Vec::new() });

        let results = data
            .files
            .into_iter()
            .map(|f| {
                let fmt = Path::new(&f.filename)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .unwrap_or("mp3")
                    .to_lowercase();

                DownloadSearchResult {
                    id: format!("{}:{}", f.username, f.filename),
                    provider: "soulseek".to_string(),
                    username: f.username,
                    filename: f.filename,
                    file_size: f.size,
                    bitrate: f.bit_rate,
                    sample_rate: f.sample_rate,
                    format: fmt,
                    slots_free: f.has_free_upload_slot.unwrap_or(false),
                    speed_bps: f.upload_speed.unwrap_or(0),
                }
            })
            .collect();

        Ok(results)
    }

    async fn start_download(
        &self,
        result: &DownloadSearchResult,
        _destination_dir: &Path,
    ) -> AppResult<String> {
        let url = format!("{}/transfers/downloads/{}", self.base_url, result.username);
        let items = vec![SlskdDownloadItem {
            filename: &result.filename,
            size: result.file_size,
        }];

        let req = self.apply_auth(self.client.post(&url).json(&items));
        let resp = req.send().await.map_err(|e| {
            AppError::Network(format!("Failed to enqueue Slskd download: {}", e))
        })?;

        if !resp.status().is_success() {
            return Err(AppError::Network(format!(
                "Slskd download enqueue failed with status {}",
                resp.status()
            )));
        }

        Ok(result.id.clone())
    }

    async fn get_progress(&self, provider_task_id: &str) -> AppResult<Option<DownloadProgress>> {
        let url = format!("{}/transfers/downloads", self.base_url);
        let req = self.apply_auth(self.client.get(&url));

        let resp = match req.send().await {
            Ok(r) if r.status() == StatusCode::OK => r,
            _ => return Ok(None),
        };

        let val: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::Network(format!("Failed to parse downloads: {}", e)))?;

        // Parse transfer list looking for provider_task_id
        if let Some(transfers) = val.as_array() {
            for t in transfers {
                let username = t["username"].as_str().unwrap_or("");
                if let Some(dirs) = t["directories"].as_array() {
                    for dir in dirs {
                        if let Some(files) = dir["files"].as_array() {
                            for f in files {
                                let filename = f["filename"].as_str().unwrap_or("");
                                let id = format!("{}:{}", username, filename);
                                if id == provider_task_id {
                                    let bytes = f["bytesTransferred"].as_i64().unwrap_or(0);
                                    let size = f["size"].as_i64().unwrap_or(1);
                                    let speed = f["averageSpeed"].as_u64().unwrap_or(0);
                                    let state = f["state"].as_str().unwrap_or("");

                                    let status = match state.to_lowercase().as_str() {
                                        "completed" | "succeeded" => DownloadStatus::Completed,
                                        "cancelled" | "aborted" => DownloadStatus::Cancelled,
                                        "errored" | "failed" => DownloadStatus::Failed,
                                        "downloading" | "in_progress" => DownloadStatus::Downloading,
                                        _ => DownloadStatus::Queued,
                                    };

                                    return Ok(Some(DownloadProgress {
                                        task_id: provider_task_id.to_string(),
                                        bytes_downloaded: bytes,
                                        total_bytes: size,
                                        speed_bps: speed,
                                        status,
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    async fn cancel(&self, provider_task_id: &str) -> AppResult<()> {
        if let Some((username, _)) = provider_task_id.split_once(':') {
            let url = format!("{}/transfers/downloads/{}", self.base_url, username);
            let req = self.apply_auth(self.client.delete(&url));
            let _ = req.send().await;
        }
        Ok(())
    }
}
