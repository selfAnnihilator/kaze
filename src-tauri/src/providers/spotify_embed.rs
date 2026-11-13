//! Metadata-only Spotify playlist reader.
//!
//! Reads the public embed page (`open.spotify.com/embed/playlist/{id}`) and its
//! `__NEXT_DATA__` JSON. No API key, no audio: every track is matched to a
//! JioSaavn recording by the caller (see ADR 0017 addendum).
use serde_json::Value;
use std::time::Duration;

pub const UNREADABLE: &str = "Couldn't read this Spotify playlist. Make sure it's public.";

#[derive(Debug, Clone, PartialEq)]
pub struct SpotifyEmbedTrack {
    pub title: String,
    pub artists: Vec<String>,
    pub duration_secs: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpotifyEmbedPlaylist {
    pub id: String,
    pub title: String,
    pub cover_url: Option<String>,
    pub tracks: Vec<SpotifyEmbedTrack>,
}

/// Extract a playlist id from an `open.spotify.com` URL or `spotify:playlist:` URI.
pub fn playlist_id(reference: &str) -> Option<String> {
    let reference = reference.trim();
    let candidate = if let Some(rest) = reference.strip_prefix("spotify:playlist:") {
        rest.to_string()
    } else {
        let url = reqwest::Url::parse(reference).ok()?;
        if !matches!(url.scheme(), "https" | "http") || url.host_str() != Some("open.spotify.com") {
            return None;
        }
        let segments: Vec<&str> = url.path_segments()?.filter(|s| !s.is_empty()).collect();
        let at = segments.iter().position(|s| *s == "playlist")?;
        // Allowed prefixes: none, `intl-xx`, `embed`.
        if !segments[..at]
            .iter()
            .all(|s| s.starts_with("intl-") || *s == "embed")
        {
            return None;
        }
        segments.get(at + 1)?.to_string()
    };
    (candidate.len() == 22 && candidate.chars().all(|c| c.is_ascii_alphanumeric()))
        .then_some(candidate)
}

/// Parse the embed HTML. Pure, so it is testable without network.
pub fn parse_embed(html: &str) -> Result<SpotifyEmbedPlaylist, String> {
    let marker = html
        .find("id=\"__NEXT_DATA__\"")
        .ok_or_else(|| UNREADABLE.to_string())?;
    let start = marker + html[marker..].find('>').ok_or_else(|| UNREADABLE.to_string())? + 1;
    let end = start
        + html[start..]
            .find("</script>")
            .ok_or_else(|| UNREADABLE.to_string())?;
    let data: Value = serde_json::from_str(&html[start..end]).map_err(|_| UNREADABLE.to_string())?;
    let entity = data
        .pointer("/props/pageProps/state/data/entity")
        .ok_or_else(|| UNREADABLE.to_string())?;
    let str_at = |v: &Value, key: &str| v.get(key).and_then(Value::as_str).map(str::to_string);
    let id = str_at(entity, "id").unwrap_or_default();
    let title = str_at(entity, "name")
        .or_else(|| str_at(entity, "title"))
        .unwrap_or_else(|| "Spotify playlist".into());
    let cover_url = entity
        .pointer("/coverArt/sources/0/url")
        .and_then(Value::as_str)
        .map(str::to_string);
    let list = entity
        .get("trackList")
        .and_then(Value::as_array)
        .ok_or_else(|| UNREADABLE.to_string())?;
    let tracks = list
        .iter()
        .filter(|t| t.get("isPlayable").and_then(Value::as_bool) != Some(false))
        .filter_map(|t| {
            let title = str_at(t, "title")?.trim().to_string();
            if title.is_empty() {
                return None;
            }
            let artists = str_at(t, "subtitle")
                .unwrap_or_default()
                .split(',')
                .map(|a| a.trim_matches(|c: char| c.is_whitespace() || c == '\u{a0}').to_string())
                .filter(|a| !a.is_empty())
                .collect();
            let duration_secs = t.get("duration").and_then(Value::as_f64).unwrap_or(0.0) / 1000.0;
            Some(SpotifyEmbedTrack { title, artists, duration_secs })
        })
        .collect();
    Ok(SpotifyEmbedPlaylist { id, title, cover_url, tracks })
}

pub struct SpotifyEmbedClient {
    client: reqwest::Client,
    base: String,
}

impl Default for SpotifyEmbedClient {
    fn default() -> Self {
        Self::new()
    }
}

impl SpotifyEmbedClient {
    pub fn new() -> Self {
        Self::with_endpoint("https://open.spotify.com")
    }

    pub fn with_endpoint(base: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(6))
                .timeout(Duration::from_secs(15))
                .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko)")
                .build()
                .expect("valid HTTP configuration"),
            base: base.trim_end_matches('/').into(),
        }
    }

    pub async fn playlist(&self, id: &str) -> Result<SpotifyEmbedPlaylist, String> {
        let response = self
            .client
            .get(format!("{}/embed/playlist/{id}", self.base))
            .send()
            .await
            .map_err(|e| format!("Couldn't reach Spotify: {e}"))?;
        if !response.status().is_success() {
            return Err(UNREADABLE.into());
        }
        let html = response.text().await.map_err(|_| UNREADABLE.to_string())?;
        let mut playlist = parse_embed(&html)?;
        if playlist.id.is_empty() {
            playlist.id = id.into();
        }
        Ok(playlist)
    }
}

#[cfg(test)]
pub(crate) fn fixture_html(tracks: &str) -> String {
    format!(
        r#"<html><head></head><body><script id="__NEXT_DATA__" type="application/json">{{"props":{{"pageProps":{{"state":{{"data":{{"entity":{{"id":"37i9dQZF1DXcBWIGoYBM5M","name":"Fixture mix","coverArt":{{"sources":[{{"url":"https://img/c.jpg"}}]}},"trackList":[{tracks}]}}}}}}}}}}}}</script></body></html>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playlist_id_accepts_url_variants() {
        let id = "17YXlpA1vgyURYow0lQtyo";
        for r in [
            format!("https://open.spotify.com/playlist/{id}?si=abc123"),
            format!("https://open.spotify.com/intl-de/playlist/{id}"),
            format!("https://open.spotify.com/embed/playlist/{id}"),
            format!("  spotify:playlist:{id} "),
        ] {
            assert_eq!(playlist_id(&r).as_deref(), Some(id), "{r}");
        }
        for r in [
            "https://open.spotify.com/album/17YXlpA1vgyURYow0lQtyo",
            "https://evil.example/playlist/17YXlpA1vgyURYow0lQtyo",
            "https://open.spotify.com/playlist/short",
            "https://www.jiosaavn.com/featured/x/abc",
            "123",
        ] {
            assert_eq!(playlist_id(r), None, "{r}");
        }
    }

    #[test]
    fn parse_embed_reads_tracks_and_skips_unplayable() {
        let html = fixture_html(
            r#"{"title":"Arz Kiya Hai | Coke Studio Bharat","subtitle":"Anuv Jain, Lost Stories","duration":294500,"isPlayable":true},{"title":"Gone","subtitle":"X","duration":1000,"isPlayable":false}"#,
        );
        let p = parse_embed(&html).unwrap();
        assert_eq!(p.title, "Fixture mix");
        assert_eq!(p.cover_url.as_deref(), Some("https://img/c.jpg"));
        assert_eq!(p.tracks.len(), 1);
        assert_eq!(p.tracks[0].artists, vec!["Anuv Jain", "Lost Stories"]);
        assert!((p.tracks[0].duration_secs - 294.5).abs() < 1e-9);
    }

    #[test]
    fn parse_embed_rejects_unknown_pages() {
        assert_eq!(parse_embed("<html>nope</html>").unwrap_err(), UNREADABLE);
        assert_eq!(
            parse_embed(r#"<script id="__NEXT_DATA__">{"props":{}}</script>"#).unwrap_err(),
            UNREADABLE
        );
    }
}
