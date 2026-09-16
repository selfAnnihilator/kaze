use super::traits::DownloadProvider;
use super::types::{DownloadProgress, DownloadSearchResult, DownloadStatus};
use crate::core::error::{AppError, AppResult};
use async_trait::async_trait;
use reqwest::{Client, Url};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone)]
struct ArchiveFile {
    url: Url,
}

/// Public, Creative Commons or public-domain audio files from Internet Archive.
pub struct ArchiveProvider {
    client: Client,
    files: Arc<RwLock<HashMap<String, ArchiveFile>>>,
    tasks: Arc<RwLock<HashMap<String, DownloadProgress>>>,
}

impl ArchiveProvider {
    pub fn new() -> Self {
        Self {
            client: Client::builder().build().unwrap_or_default(),
            files: Arc::new(RwLock::new(HashMap::new())),
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn search_music(
        &self,
        artist: &str,
        title: &str,
    ) -> AppResult<Vec<DownloadSearchResult>> {
        let artist = artist.trim();
        let title = title.trim();
        if artist.is_empty() && title.is_empty() {
            return Ok(Vec::new());
        }

        let terms = if artist.is_empty() {
            format!("title:\"{}\"", quote_term(title))
        } else if title.is_empty() {
            format!(
                "creator:\"{}\" OR title:\"{}\"",
                quote_term(artist),
                quote_term(artist)
            )
        } else {
            format!(
                "creator:\"{}\" OR title:\"{}\" OR title:\"{}\"",
                quote_term(artist),
                quote_term(title),
                quote_term(&format!("{} {}", artist, title))
            )
        };
        let query = format!("mediatype:(audio OR etree) AND ({})", terms);
        let search: Value = self
            .client
            .get("https://archive.org/advancedsearch.php")
            .query(&[
                ("q", query.as_str()),
                ("fl[]", "identifier"),
                ("rows", "12"),
                ("output", "json"),
            ])
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Internet Archive search failed: {e}")))?
            .error_for_status()
            .map_err(|e| AppError::Network(format!("Internet Archive search failed: {e}")))?
            .json()
            .await
            .map_err(|e| AppError::Network(format!("Invalid Internet Archive search data: {e}")))?;

        let mut fetches = tokio::task::JoinSet::new();
        if let Some(items) = search.pointer("/response/docs").and_then(Value::as_array) {
            for identifier in items
                .iter()
                .filter_map(|item| item.get("identifier").and_then(Value::as_str))
            {
                if !valid_identifier(identifier) {
                    continue;
                }
                let client = self.client.clone();
                let identifier = identifier.to_string();
                fetches.spawn(async move {
                    let response = client
                        .get(format!("https://archive.org/metadata/{identifier}"))
                        .timeout(Duration::from_secs(8))
                        .send()
                        .await
                        .ok()?
                        .error_for_status()
                        .ok()?;
                    let metadata = response.json::<Value>().await.ok()?;
                    Some((identifier, metadata))
                });
            }
        }

        let mut results = Vec::new();
        let mut file_cache = HashMap::new();
        while let Some(outcome) = fetches.join_next().await {
            let Ok(Some((identifier, metadata))) = outcome else {
                continue;
            };
            for (result, file) in parse_item(&identifier, &metadata) {
                file_cache.insert(result.id.clone(), file);
                results.push(result);
                if results.len() >= 30 {
                    break;
                }
            }
            if results.len() >= 30 {
                break;
            }
        }
        self.files.write().await.extend(file_cache);
        Ok(results)
    }
}

impl Default for ArchiveProvider {
    fn default() -> Self {
        Self::new()
    }
}

fn quote_term(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn valid_identifier(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 100
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

fn metadata_string<'a>(value: &'a Value) -> Option<&'a str> {
    value
        .as_str()
        .or_else(|| value.as_array()?.iter().find_map(Value::as_str))
}

fn allowed_license(metadata: &Value) -> bool {
    let Some(url) = metadata.get("licenseurl").and_then(metadata_string) else {
        return false;
    };
    let Ok(url) = Url::parse(url) else {
        return false;
    };
    url.host_str() == Some("creativecommons.org")
        && (url.path().starts_with("/licenses/") || url.path().starts_with("/publicdomain/"))
}

fn audio_extension(name: &str) -> Option<&'static str> {
    let extension = name.rsplit('.').next()?.to_ascii_lowercase();
    match extension.as_str() {
        "mp3" => Some("mp3"),
        "flac" => Some("flac"),
        "ogg" => Some("ogg"),
        "m4a" => Some("m4a"),
        "wav" => Some("wav"),
        _ => None,
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

fn parse_item(identifier: &str, item: &Value) -> Vec<(DownloadSearchResult, ArchiveFile)> {
    if !valid_identifier(identifier) || !allowed_license(&item["metadata"]) {
        return Vec::new();
    }
    let Some(files) = item.get("files").and_then(Value::as_array) else {
        return Vec::new();
    };
    let creator = item["metadata"]
        .get("creator")
        .and_then(metadata_string)
        .unwrap_or("");
    let item_title = item["metadata"]
        .get("title")
        .and_then(metadata_string)
        .unwrap_or("");
    let audio_count = files
        .iter()
        .filter(|file| file["name"].as_str().and_then(audio_extension).is_some())
        .count();
    let mut results = Vec::new();

    for (index, file) in files.iter().enumerate() {
        let Some(remote_name) = file["name"].as_str() else {
            continue;
        };
        let Some(extension) = audio_extension(remote_name) else {
            continue;
        };
        let size = file["size"]
            .as_str()
            .and_then(|s| s.parse::<i64>().ok())
            .or_else(|| file["size"].as_i64())
            .unwrap_or(0);
        if size <= 0 || size > 2_000_000_000 {
            continue;
        }
        let artist = file
            .get("artist")
            .and_then(metadata_string)
            .unwrap_or(creator);
        let stem = remote_name
            .rsplit('/')
            .next()
            .unwrap_or(remote_name)
            .rsplit_once('.')
            .map(|(s, _)| s)
            .unwrap_or("");
        let title = file
            .get("title")
            .and_then(metadata_string)
            .or_else(|| {
                if audio_count == 1 {
                    Some(item_title)
                } else {
                    None
                }
            })
            .filter(|s| !s.is_empty())
            .unwrap_or(stem);
        if artist.is_empty() || title.is_empty() {
            continue;
        }
        let filename = safe_name(&format!("{} - {}.{}", artist, title, extension));
        if filename.is_empty() {
            continue;
        }

        let mut url = Url::parse("https://archive.org/download/").expect("static Archive URL");
        let mut valid_path = true;
        {
            let mut path = url
                .path_segments_mut()
                .expect("static Archive URL is a base");
            path.push(identifier);
            for segment in remote_name.split('/') {
                if segment.is_empty() || segment == "." || segment == ".." {
                    valid_path = false;
                    break;
                }
                path.push(segment);
            }
        }
        if !valid_path {
            continue;
        }
        let result = DownloadSearchResult {
            id: format!("archive_{identifier}_{index}"),
            provider: "internet-archive".to_string(),
            username: artist.to_string(),
            filename,
            file_size: size,
            bitrate: None,
            sample_rate: None,
            format: extension.to_string(),
            slots_free: true,
            speed_bps: 0,
        };
        results.push((result, ArchiveFile { url }));
        if results.len() >= 8 {
            break;
        }
    }
    results
}

#[async_trait]
impl DownloadProvider for ArchiveProvider {
    fn name(&self) -> &'static str {
        "internet-archive"
    }
    fn is_available(&self) -> bool {
        true
    }

    async fn search(&self, query: &str) -> AppResult<Vec<DownloadSearchResult>> {
        self.search_music("", query).await
    }

    async fn search_track(
        &self,
        artist: &str,
        title: &str,
    ) -> AppResult<Vec<DownloadSearchResult>> {
        self.search_music(artist, title).await
    }

    async fn start_download(
        &self,
        result: &DownloadSearchResult,
        destination_dir: &Path,
    ) -> AppResult<String> {
        let source = self
            .files
            .read()
            .await
            .get(&result.id)
            .cloned()
            .ok_or_else(|| {
                AppError::NotFound("Internet Archive result expired; search again".to_string())
            })?;
        let task_id = format!("archive_task_{}", Uuid::new_v4());
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
        let tasks = self.tasks.clone();
        let client = self.client.clone();
        let task = task_id.clone();
        tokio::spawn(async move {
            let outcome = async {
                let mut response = client
                    .get(source.url)
                    .send()
                    .await
                    .map_err(|e| e.to_string())?
                    .error_for_status()
                    .map_err(|e| e.to_string())?;
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
                    }
                }
                output.flush().await.map_err(|e| e.to_string())?;
                if tasks.read().await.get(&task).map(|p| p.status)
                    == Some(DownloadStatus::Cancelled)
                {
                    return Err("cancelled".to_string());
                }
                // Publishing via a hard link fails if another download already owns the name.
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
    fn only_licensed_audio_files_become_results() {
        let item = json!({"metadata":{"creator":"Kato","title":"Album","licenseurl":"https://creativecommons.org/licenses/by/4.0/"},"files":[
            {"name":"01 - Pokémon Center.mp3","title":"Pokémon Center","size":"1234","source":"original"},
            {"name":"cover.jpg","size":"123"}
        ]});
        let results = parse_item("kato-album", &item);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.filename, "Kato - Pokémon Center.mp3");
        assert_eq!(results[0].0.provider, "internet-archive");
        assert!(results[0]
            .1
            .url
            .as_str()
            .contains("Pok%C3%A9mon%20Center.mp3"));
        let mut unlicensed = item;
        unlicensed["metadata"]
            .as_object_mut()
            .unwrap()
            .remove("licenseurl");
        assert!(parse_item("kato-album", &unlicensed).is_empty());
    }

    #[test]
    fn rejects_unsafe_archive_paths() {
        let item = json!({"metadata":{"creator":"Kato","licenseurl":"https://creativecommons.org/licenses/by/4.0/"},"files":[
            {"name":"../song.mp3","title":"Song","size":"1234"}
        ]});
        assert!(parse_item("safe-id", &item).is_empty());
        assert!(parse_item("../unsafe", &item).is_empty());
    }
}
