//! Last.fm `artist.getSimilar`: "fans of X also listen to Y", computed from
//! Last.fm's own listener base. Kaze sends artist names only — no user data,
//! no scrobbles, no account.

use serde_json::Value;
use std::time::Duration;

/// Read-only Last.fm API key for public `artist.getSimilar` lookups.
///
/// It is shipped in the app on purpose: it is a public, rate-limited, read-only
/// key (the same model every open-source Last.fm client uses). The shared
/// secret is never used, never stored and never needed, since Kaze performs
/// no authenticated or write calls. Empty disables Last.fm entirely.
pub const LASTFM_API_KEY: &str = "0966a5ce6c172932880e25b599f1b659";

const ENDPOINT: &str = "https://ws.audioscrobbler.com/2.0/";

#[derive(Debug, Clone, PartialEq)]
pub struct SimilarArtist {
    pub name: String,
    pub score: f64,
}

#[derive(Debug)]
pub enum LastFmError {
    /// Last.fm does not know the artist; cache negatively.
    NotFound,
    /// Network, rate limit or server failure; retry later.
    Transient(String),
}

pub struct LastFmClient {
    client: reqwest::Client,
    endpoint: String,
    api_key: String,
}

impl LastFmClient {
    /// `None` when no key is compiled in.
    pub fn from_builtin_key() -> Option<Self> {
        Self::new(ENDPOINT, LASTFM_API_KEY)
    }

    pub fn new(endpoint: &str, api_key: &str) -> Option<Self> {
        let api_key = api_key.trim();
        if api_key.is_empty() {
            return None;
        }
        Some(Self {
            client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(6))
                .timeout(Duration::from_secs(15))
                .user_agent(concat!("Kaze/", env!("CARGO_PKG_VERSION"), " (desktop music player)"))
                .build()
                .expect("valid HTTP configuration"),
            endpoint: endpoint.into(),
            api_key: api_key.into(),
        })
    }

    pub async fn similar_artists(&self, artist: &str) -> Result<Vec<SimilarArtist>, LastFmError> {
        let response = self
            .client
            .get(&self.endpoint)
            .query(&[
                ("method", "artist.getsimilar"),
                ("artist", artist),
                ("autocorrect", "1"),
                ("limit", "50"),
                ("format", "json"),
                ("api_key", self.api_key.as_str()),
            ])
            .send()
            .await
            .map_err(|e| LastFmError::Transient(e.without_url().to_string()))?;
        let status = response.status();
        let body: Value = response
            .json()
            .await
            .map_err(|e| LastFmError::Transient(e.without_url().to_string()))?;
        if !status.is_success() && body.get("error").is_none() {
            return Err(LastFmError::Transient(format!("HTTP {status}")));
        }
        parse_similar(&body)
    }
}

/// Parses an `artist.getSimilar` JSON body. Last.fm encodes `match` as a string.
pub fn parse_similar(body: &Value) -> Result<Vec<SimilarArtist>, LastFmError> {
    if let Some(code) = body.get("error").and_then(Value::as_i64) {
        // 6 = artist not found; everything else (rate limit, bad key, outage) is transient.
        return if code == 6 {
            Err(LastFmError::NotFound)
        } else {
            Err(LastFmError::Transient(format!("Last.fm error {code}")))
        };
    }
    let Some(list) = body
        .get("similarartists")
        .and_then(|s| s.get("artist"))
    else {
        return Ok(Vec::new());
    };
    let items: Vec<&Value> = match list {
        Value::Array(items) => items.iter().collect(),
        Value::Object(_) => vec![list],
        _ => Vec::new(),
    };
    Ok(items
        .into_iter()
        .filter_map(|item| {
            let name = item.get("name")?.as_str()?.trim();
            let score = match item.get("match")? {
                Value::String(s) => s.trim().parse::<f64>().ok()?,
                Value::Number(n) => n.as_f64()?,
                _ => return None,
            };
            (!name.is_empty() && score.is_finite() && score > 0.0).then(|| SimilarArtist {
                name: name.to_string(),
                score: score.clamp(0.0, 1.0),
            })
        })
        .collect())
}
