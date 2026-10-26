//! Unofficial web API adapter. See docs/jiosaavn-experiment.md for provenance.
use super::types::ExternalTrackMetadata;
use base64::{engine::general_purpose::STANDARD, Engine};
use des::{
    cipher::{BlockDecrypt, KeyInit},
    Des,
};
use serde_json::Value;
use std::{
    collections::HashMap,
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
            let r = self
                .client
                .get(&self.endpoint)
                .query(&[
                    ("__call", call),
                    ("_format", "json"),
                    ("_marker", "0"),
                    ("cc", "in"),
                ])
                .query(params)
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

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<ExternalTrackMetadata>> {
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
        items.iter().take(limit.min(50)).map(parse_track).collect()
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

    pub async fn resolve(&self, id: &str, refresh: bool) -> Result<(String, f64)> {
        let value = self.detail(id, refresh).await?;
        let metadata = parse_track(&value)?;
        let duration = metadata
            .duration_secs
            .filter(|d| *d > 0.0)
            .ok_or_else(|| JioSaavnError::Malformed("missing full-song duration".into()))?;
        // Decode only the ordinary URL envelope from the reference adapter.
        // Never use encrypted_drm_media_url, licenses, keys, or preview substitutions.
        if value
            .get("rights")
            .and_then(|r| number(r, "code"))
            .is_some_and(|n| n != 0.0)
        {
            return Err(JioSaavnError::Unavailable(
                "territory or content restriction".into(),
            ));
        }
        let encrypted = text(&value, "encrypted_media_url")
            .ok_or_else(|| JioSaavnError::Unavailable("ordinary full-track media absent".into()))?;
        let url = decode_media_url(&encrypted, truthy(&value["320kbps"]))?;
        Ok((url, duration))
    }
}

impl Default for JioSaavnProvider {
    fn default() -> Self {
        Self::new()
    }
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
