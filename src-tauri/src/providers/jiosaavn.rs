//! Unofficial web API adapter. See docs/archive/jiosaavn-experiment.md for provenance.
use crate::core::sync::MutexExt;
use super::types::ExternalTrackMetadata;
use base64::{engine::general_purpose::STANDARD, Engine};
use des::{
    cipher::{BlockDecrypt, KeyInit},
    Des,
};
use serde_json::Value;

mod parse;
pub use parse::parse_track;
use parse::*;
use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
    time::{Duration, Instant},
};

#[derive(Debug, thiserror::Error)]
pub enum JioSaavnError {
    #[error("JioSaavn network request failed: {0}")]
    Network(String),
    #[error("JioSaavn rate limited this request; try again later")]
    RateLimited,
    #[error("JioSaavn content unavailable: {0}")]
    Unavailable(String),
    #[error("JioSaavn response changed or is malformed: {0}")]
    Malformed(String),
}

impl From<JioSaavnError> for crate::core::AppError {
    fn from(e: JioSaavnError) -> Self {
        Self::ExternalApi {
            provider: "jiosaavn".into(),
            message: e.to_string(),
        }
    }
}

type Result<T> = std::result::Result<T, JioSaavnError>;

pub struct JioSaavnProvider {
    client: reqwest::Client,
    endpoint: String,
    cache: Mutex<HashMap<String, (Instant, Value)>>,
}

impl JioSaavnProvider {
    pub fn new() -> Self {
        Self::with_endpoint("https://www.jiosaavn.com/api.php")
    }

    pub fn with_endpoint(endpoint: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(6))
                .timeout(Duration::from_secs(15))
                .user_agent("Kaze/1.0 (desktop music player)")
                .build()
                .expect("valid HTTP configuration"),
            endpoint: endpoint.into(),
            cache: Mutex::new(HashMap::new()),
        }
    }

    async fn request(&self, call: &str, params: &[(&str, &str)], refresh: bool) -> Result<Value> {
        let key = format!("{call}:{params:?}");
        if !refresh {
            if let Some((at, value)) = self.cache.lock_or_recover().get(&key) {
                if at.elapsed() < Duration::from_secs(300) {
                    return Ok(value.clone());
                }
            }
        }
        // Dropping this future cancels both the request and backoff.
        let mut response = None;
        for attempt in 0..2 {
            let request = self
                .client
                .get(&self.endpoint)
                .query(&[
                    ("__call", call),
                    ("_format", "json"),
                    ("_marker", "0"),
                    ("cc", "in"),
                ])
                .query(params);
            // This endpoint reads its editorial language from L, ignoring the query alone.
            let request =
                if let Some((_, language)) = params.iter().find(|(key, _)| *key == "language") {
                    request.header(reqwest::header::COOKIE, format!("L={language}"))
                } else {
                    request
                };
            let r = request
                .send()
                .await
                .map_err(|e| JioSaavnError::Network(e.without_url().to_string()))?;
            if r.status().as_u16() == 429 {
                return Err(JioSaavnError::RateLimited);
            }
            if r.status().is_server_error() && attempt == 0 {
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            }
            if !r.status().is_success() {
                return Err(JioSaavnError::Unavailable(format!("HTTP {}", r.status())));
            }
            response = Some(r);
            break;
        }
        let mut r = response.ok_or_else(|| JioSaavnError::Network("no response".into()))?;
        let mut bytes = Vec::new();
        while let Some(chunk) = r
            .chunk()
            .await
            .map_err(|e| JioSaavnError::Network(e.without_url().to_string()))?
        {
            if bytes.len() + chunk.len() > 4 * 1024 * 1024 {
                return Err(JioSaavnError::Malformed("response exceeds 4 MiB".into()));
            }
            bytes.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|_| JioSaavnError::Malformed("invalid JSON".into()))?;
        let mut cache = self.cache.lock_or_recover();
        cache.retain(|_, (at, _)| at.elapsed() < Duration::from_secs(300));
        if cache.len() >= 128 {
            if let Some(oldest) = cache
                .iter()
                .min_by_key(|(_, (at, _))| *at)
                .map(|(k, _)| k.clone())
            {
                cache.remove(&oldest);
            }
        }
        cache.insert(key, (Instant::now(), value.clone()));
        Ok(value)
    }

    async fn search_items(&self, query: &str, limit: usize) -> Result<Vec<Value>> {
        if query.trim().is_empty() || limit == 0 {
            return Ok(vec![]);
        }
        let n = limit.clamp(1, 50).to_string();
        let value = self
            .request(
                "search.getResults",
                &[("q", query.trim()), ("n", &n), ("p", "1")],
                false,
            )
            .await?;
        let items = value
            .get("results")
            .and_then(Value::as_array)
            .ok_or_else(|| JioSaavnError::Malformed("missing search results".into()))?;
        Ok(items.iter().take(limit.min(50)).cloned().collect())
    }

    /// User-facing results must have the same full-song source that playback accepts.
    /// Availability is present in the search response: no per-song requests or audio downloads.
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<ExternalTrackMetadata>> {
        let mut seen = HashSet::new();
        let mut tracks: Vec<_> = self
            .search_items(query, limit)
            .await?
            .iter()
            .filter(|item| full_track_source(item).is_ok())
            .filter_map(|item| parse_track(item).ok())
            .filter(|track| seen.insert(track.provider_track_id.clone()))
            .collect();
        super::releases::prefer_artist_editions(&mut tracks, query);
        Ok(tracks)
    }

    /// Find an artist-owned edition for automatic recommendations only.
    /// The compilation's saved source is never rewritten or given another cover.
    pub async fn artist_edition(
        &self,
        source: &ExternalTrackMetadata,
    ) -> Result<Option<ExternalTrackMetadata>> {
        let enriched;
        let source = if source.album_id.is_none() {
            enriched = parse_track(&self.detail(&source.provider_track_id, false).await?)?;
            &enriched
        } else {
            source
        };
        let Some(album_id) = source.album_id.as_deref() else {
            return Ok(None);
        };
        let album = self
            .request("content.getAlbumDetails", &[("albumid", album_id)], false)
            .await?;
        let credits = album_credits(&album);
        if !credits
            .iter()
            .any(|artist| super::strict_match::normalized(artist) == "various artists")
        {
            return Ok(None);
        }
        let candidates = self
            .search(&format!("{} {}", source.title, source.artist_name), 50)
            .await?;
        let mut checked = HashSet::new();
        for track in candidates.into_iter().filter(|track| {
            super::strict_match::normalized(&track.title)
                == super::strict_match::normalized(&source.title)
                && super::strict_match::normalized(&track.artist_name)
                    == super::strict_match::normalized(&source.artist_name)
                && track
                    .duration_secs
                    .zip(source.duration_secs)
                    .is_some_and(|(a, b)| (a - b).abs() <= 3.0)
        }) {
            let Some(album_id) = track.album_id.as_deref() else {
                continue;
            };
            if !checked.insert(album_id.to_string()) {
                continue;
            }
            if checked.len() > 4 {
                break;
            }
            let album = self
                .request("content.getAlbumDetails", &[("albumid", album_id)], false)
                .await?;
            let credits = album_credits(&album);
            if credits
                .iter()
                .any(|artist| super::strict_match::normalized(artist) == "various artists")
            {
                continue;
            }
            if credits.iter().any(|artist| {
                super::strict_match::normalized(artist)
                    == super::strict_match::normalized(&source.artist_name)
            }) {
                return Ok(Some(track));
            }
        }
        Ok(None)
    }

    /// Local-file enrichment can use catalog metadata even when streaming is unavailable.
    pub async fn search_metadata(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ExternalTrackMetadata>> {
        self.search_items(query, limit)
            .await?
            .iter()
            .map(parse_track)
            .collect()
    }

    pub async fn detail(&self, id: &str, refresh: bool) -> Result<Value> {
        if id.is_empty()
            || !id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(JioSaavnError::Malformed("invalid song ID".into()));
        }
        let value = self
            .request("song.getDetails", &[("pids", id)], refresh)
            .await?;
        let item = value
            .get(id)
            .or_else(|| {
                value
                    .get("songs")
                    .and_then(Value::as_array)
                    .and_then(|a| a.iter().find(|s| text(s, "id").as_deref() == Some(id)))
            })
            .ok_or_else(|| JioSaavnError::Unavailable(format!("song {id} not returned")))?;
        if text(item, "id").as_deref() != Some(id) {
            return Err(JioSaavnError::Malformed("song ID mismatch".into()));
        }
        Ok(item.clone())
    }

    pub async fn album(&self, id: &str) -> Result<Vec<ExternalTrackMetadata>> {
        let value = self
            .request("content.getAlbumDetails", &[("albumid", id)], false)
            .await?;
        value
            .get("songs")
            .and_then(Value::as_array)
            .ok_or_else(|| JioSaavnError::Malformed("missing album songs".into()))?
            .iter()
            .map(parse_track)
            .collect()
    }

    /// Request international/English programming explicitly, then merge regional charts.
    /// Titles and IDs always come from editorial responses, never invented rankings.
    pub async fn charts(&self, refresh: bool) -> Result<Vec<Value>> {
        let (english, regional, launch) = tokio::join!(
            self.request(
                "content.getCharts",
                &[
                    ("language", "english"),
                    ("languages", "english"),
                    ("ctx", "web6dot0")
                ],
                refresh
            ),
            self.request(
                "content.getCharts",
                &[
                    ("language", "hindi"),
                    ("languages", "hindi,malayalam,tamil,telugu,kannada,punjabi"),
                    ("ctx", "web6dot0")
                ],
                refresh
            ),
            self.request(
                "webapi.getLaunchData",
                &[
                    ("language", "english"),
                    ("languages", "english"),
                    ("ctx", "web6dot0")
                ],
                refresh
            ),
        );
        let mut feeds = Vec::new();
        let mut first_error = None;
        for result in [english, regional] {
            match result {
                Ok(value) => feeds.push(value),
                Err(error) => {
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
            }
        }
        if let Ok(launch) = launch {
            for key in ["charts", "top_playlists"] {
                if let Some(items) = launch.get(key).and_then(Value::as_array) {
                    feeds.push(Value::Array(
                        items
                            .iter()
                            .filter(|item| {
                                key == "charts" || {
                                    let title = text(item, "title")
                                        .or_else(|| text(item, "listname"))
                                        .unwrap_or_default()
                                        .to_lowercase();
                                    ["global", "international", "english", "worldwide"]
                                        .iter()
                                        .any(|word| title.contains(word))
                                        && ["top", "chart", "hits", "trending"]
                                            .iter()
                                            .any(|word| title.contains(word))
                                }
                            })
                            .cloned()
                            .collect(),
                    ));
                }
            }
        }
        let charts = merge_charts(&feeds);
        if charts.is_empty() {
            if let Some(error) = first_error {
                return Err(error);
            }
        }
        Ok(charts)
    }

    pub async fn playlist_reference(
        &self,
        reference: &str,
    ) -> Result<(String, String, Option<String>, Vec<ExternalTrackMetadata>)> {
        let reference = reference.trim();
        let data = if reference.chars().all(|c| c.is_ascii_digit()) && !reference.is_empty() {
            self.request(
                "playlist.getDetails",
                &[("listid", reference), ("n", "50")],
                false,
            )
            .await?
        } else {
            let url = reqwest::Url::parse(reference).map_err(|_| {
                JioSaavnError::Malformed("use a JioSaavn playlist URL or numeric ID".into())
            })?;
            if url.scheme() != "https"
                || !matches!(url.host_str(), Some("www.jiosaavn.com" | "jiosaavn.com"))
            {
                return Err(JioSaavnError::Malformed(
                    "use a JioSaavn playlist URL".into(),
                ));
            }
            let token = url
                .path_segments()
                .and_then(|mut segments| segments.rfind(|s| !s.is_empty()))
                .unwrap_or("");
            if token.is_empty()
                || !token
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "_,-=".contains(c))
            {
                return Err(JioSaavnError::Malformed("invalid playlist URL".into()));
            }
            self.request(
                "webapi.get",
                &[("token", token), ("type", "playlist"), ("n", "50")],
                false,
            )
            .await?
        };
        let id = text(&data, "id")
            .or_else(|| text(&data, "listid"))
            .ok_or_else(|| JioSaavnError::Malformed("missing playlist ID".into()))?;
        let title = text(&data, "title")
            .or_else(|| text(&data, "listname"))
            .unwrap_or_else(|| "JioSaavn playlist".into());
        let tracks = self.playlist(&id, 50, false).await?;
        Ok((id, title, text(&data, "image"), tracks))
    }

    pub async fn playlist(
        &self,
        id: &str,
        limit: usize,
        refresh: bool,
    ) -> Result<Vec<ExternalTrackMetadata>> {
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
            return Err(JioSaavnError::Malformed("invalid playlist ID".into()));
        }
        let n = limit.clamp(1, 50).to_string();
        let data = self
            .request("playlist.getDetails", &[("listid", id), ("n", &n)], refresh)
            .await?;
        let items = data
            .get("list")
            .or_else(|| data.get("songs"))
            .and_then(Value::as_array)
            .ok_or_else(|| JioSaavnError::Malformed("missing playlist songs".into()))?;
        let mut seen = HashSet::new();
        Ok(items
            .iter()
            .filter(|item| full_track_source(item).is_ok())
            .filter_map(|item| parse_track(item).ok())
            .filter(|track| seen.insert(track.provider_track_id.clone()))
            .take(limit.min(50))
            .collect())
    }

    pub async fn resolve(&self, id: &str, refresh: bool) -> Result<(String, f64)> {
        let value = self.detail(id, refresh).await?;
        full_track_source(&value)
    }
}

impl Default for JioSaavnProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod chart_feed_tests;
