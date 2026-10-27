//! Unofficial web API adapter. See docs/jiosaavn-experiment.md for provenance.
use super::types::ExternalTrackMetadata;
use base64::{engine::general_purpose::STANDARD, Engine};
use des::{
    cipher::{BlockDecrypt, KeyInit},
    Des,
};
use serde_json::Value;
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
            if let Some((at, value)) = self.cache.lock().unwrap().get(&key) {
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
        let mut cache = self.cache.lock().unwrap();
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
                .and_then(|segments| segments.filter(|s| !s.is_empty()).last())
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

fn full_track_source(value: &Value) -> Result<(String, f64)> {
    let info = value.get("more_info").unwrap_or(value);
    if text(value, "type").is_some_and(|kind| kind != "song") {
        return Err(JioSaavnError::Unavailable("not a song".into()));
    }
    let metadata = parse_track(value)?;
    let duration = metadata
        .duration_secs
        .filter(|d| *d > 0.0)
        .ok_or_else(|| JioSaavnError::Malformed("missing full-song duration".into()))?;
    // Decode only the ordinary URL envelope from the reference adapter.
    // Never use encrypted_drm_media_url, licenses, keys, or preview substitutions.
    if value
        .get("rights")
        .or_else(|| info.get("rights"))
        .and_then(|r| number(r, "code"))
        .is_some_and(|n| n != 0.0)
    {
        return Err(JioSaavnError::Unavailable(
            "territory or content restriction".into(),
        ));
    }
    let encrypted = text(value, "encrypted_media_url")
        .or_else(|| text(info, "encrypted_media_url"))
        .ok_or_else(|| JioSaavnError::Unavailable("ordinary full-track media absent".into()))?;
    let url = decode_media_url(
        &encrypted,
        truthy(value.get("320kbps").unwrap_or(&info["320kbps"])),
    )?;
    Ok((url, duration))
}

impl Default for JioSaavnProvider {
    fn default() -> Self {
        Self::new()
    }
}

fn album_credits(value: &Value) -> Vec<String> {
    if let Some(artists) =
        text(value, "primary_artists").or_else(|| text(&value["more_info"], "primary_artists"))
    {
        return artists
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .collect();
    }
    value
        .pointer("/more_info/artistMap/primary_artists")
        .and_then(Value::as_array)
        .map(|artists| {
            artists
                .iter()
                .filter_map(|artist| text(artist, "name"))
                .collect()
        })
        .unwrap_or_default()
}

fn merge_charts(feeds: &[Value]) -> Vec<Value> {
    let mut seen = HashSet::new();
    let mut charts: Vec<_> = feeds
        .iter()
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(|item| {
            let id = text(item, "id").or_else(|| text(item, "listid"))?;
            let title = text(item, "title").or_else(|| text(item, "listname"))?;
            if !id.chars().all(|c| c.is_ascii_digit()) || !seen.insert(id.clone()) {
                return None;
            }
            let image = text(item, "image")
                .filter(|image| image.starts_with("https://"))
                .map(|image| {
                    image
                        .replace("150x150", "500x500")
                        .replace("50x50", "500x500")
                });
            Some(serde_json::json!({"id":id,"title":title,"image":image,"perma_url":text(item,"perma_url")}))
        })
        .take(90)
        .collect();
    charts.sort_by_key(|chart| {
        let title = chart["title"].as_str().unwrap_or("").to_lowercase();
        !["global", "english", "international", "worldwide"]
            .iter()
            .any(|word| title.contains(word))
    });
    charts.truncate(30);
    charts
}

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            _ => None,
        })
        .filter(|s| !s.trim().is_empty())
        .map(|s| html_escape::decode_html_entities(&s).trim().to_string())
}
fn number(v: &Value, key: &str) -> Option<f64> {
    v.get(key)
        .and_then(|v| v.as_f64().or_else(|| v.as_str()?.parse().ok()))
        .filter(|n| n.is_finite())
}
fn truthy(v: &Value) -> bool {
    v == true || v == 1 || v == "1" || v == "true"
}

pub fn parse_track(v: &Value) -> Result<ExternalTrackMetadata> {
    let id = text(v, "id").ok_or_else(|| JioSaavnError::Malformed("missing song ID".into()))?;
    let title = text(v, "song")
        .or_else(|| text(v, "title"))
        .ok_or_else(|| JioSaavnError::Malformed("missing title".into()))?;
    let info = v.get("more_info").unwrap_or(v);
    let artist = text(v, "primary_artists")
        .or_else(|| text(info, "primary_artists"))
        .or_else(|| text(v, "singers"))
        .or_else(|| {
            info.pointer("/artistMap/primary_artists")
                .and_then(Value::as_array)
                .map(|artists| {
                    artists
                        .iter()
                        .filter_map(|artist| text(artist, "name"))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .filter(|name| !name.is_empty())
        })
        .unwrap_or_default();
    let cover = text(v, "image")
        .filter(|u| u.starts_with("https://"))
        .map(|u| u.replace("50x50", "500x500").replace("150x150", "500x500"));
    Ok(ExternalTrackMetadata {
        provider: "jiosaavn".into(),
        provider_track_id: id,
        title,
        artist_name: artist,
        artist_id: text(v, "primary_artists_id"),
        album_title: text(v, "album").or_else(|| text(info, "album")),
        album_id: text(v, "albumid").or_else(|| text(info, "album_id")),
        duration_secs: number(v, "duration").or_else(|| number(info, "duration")),
        year: number(v, "year").filter(|n| *n > 0.0).map(|n| n as i64),
        track_number: number(v, "track_number")
            .filter(|n| *n > 0.0)
            .map(|n| n as u32),
        cover_art_url: cover,
        language: text(v, "language").or_else(|| text(info, "language")),
    })
}

fn decode_media_url(encrypted: &str, high_quality: bool) -> Result<String> {
    let mut bytes = STANDARD
        .decode(encrypted.trim())
        .map_err(|_| JioSaavnError::Malformed("invalid media envelope".into()))?;
    if bytes.is_empty() || bytes.len() % 8 != 0 {
        return Err(JioSaavnError::Malformed(
            "invalid media block length".into(),
        ));
    }
    let cipher = Des::new_from_slice(b"38346591").expect("DES key length");
    for chunk in bytes.chunks_exact_mut(8) {
        cipher.decrypt_block(chunk.into());
    }
    let padding = *bytes.last().unwrap() as usize;
    if padding == 0
        || padding > 8
        || !bytes[bytes.len() - padding..]
            .iter()
            .all(|b| *b as usize == padding)
    {
        return Err(JioSaavnError::Malformed("invalid media padding".into()));
    }
    bytes.truncate(bytes.len() - padding);
    let url = String::from_utf8(bytes)
        .map_err(|_| JioSaavnError::Malformed("non-UTF8 media URL".into()))?
        .replace(
            "_96.mp4",
            if high_quality { "_320.mp4" } else { "_160.mp4" },
        );
    let parsed = reqwest::Url::parse(&url)
        .map_err(|_| JioSaavnError::Malformed("invalid media URL".into()))?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("aac.saavncdn.com")
        || parsed.path().contains("_p.")
    {
        return Err(JioSaavnError::Unavailable(
            "not an ordinary full-track Saavn CDN URL".into(),
        ));
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn parses_entities_numbers_versions_and_missing_fields() {
        let t = parse_track(&json!({"id":"abc","song":"Rock &amp; Roll (Live)","primary_artists":"A &#039;B&#039;","duration":"123","year":"2024","image":"https://c.saavncdn.com/a-150x150.jpg"})).unwrap();
        assert_eq!(t.title, "Rock & Roll (Live)");
        assert_eq!(t.artist_name, "A 'B'");
        assert_eq!(t.duration_secs, Some(123.0));
        assert_eq!(t.year, Some(2024));
        assert!(t.cover_art_url.unwrap().ends_with("500x500.jpg"));
        assert_eq!(t.track_number, None);
        assert!(parse_track(&json!({"id":"abc"})).is_err());
        assert!(decode_media_url("invalid", true).is_err());
    }
    #[test]
    fn decodes_reference_envelope_without_using_drm_field() {
        let s = "ID2ieOjCrwfgWvL5sXl4B1ImC5QfbsDyryhkSYK5IH2E7FCO52VR6yhNbcEbes5iCcja4+W8xhE0SwtCJToN4Bw7tS9a8Gtq";
        assert!(decode_media_url(s, true).unwrap().ends_with("_320.mp4"));
        assert!(decode_media_url(s, false).unwrap().ends_with("_160.mp4"));
    }
}

#[cfg(test)]
mod chart_feed_tests {
    use super::*;
    #[test]
    fn merges_international_and_regional_editorial_lists_with_real_artwork() {
        let charts = merge_charts(&[
            serde_json::json!([
                {"id":"12","title":"Global Hits","image":"https://example.test/global-150x150.jpg"},
                {"listid":"13","listname":"English Top 50"}
            ]),
            serde_json::json!([
                {"id":12,"title":"Duplicate"},{"id":"14","title":"India Top 50"},
                {"id":"https://untrusted.test","title":"Invalid"}
            ]),
        ]);
        assert_eq!(charts.len(), 3);
        assert_eq!(charts[0]["title"], "Global Hits");
        assert_eq!(
            charts[0]["image"],
            "https://example.test/global-500x500.jpg"
        );
        assert_eq!(charts[2]["title"], "India Top 50");
    }
}
