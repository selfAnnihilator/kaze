use super::types::{ExternalAlbumMetadata, ExternalArtistMetadata, ExternalTrackMetadata};
use crate::core::error::{AppError, AppResult};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

pub struct MusicBrainzProvider {
    client: Client,
    last_request_ms: Arc<Mutex<i64>>,
    base_url: String,
}

impl MusicBrainzProvider {
    pub fn new() -> Self {
        Self::with_base_url("https://musicbrainz.org/ws/2")
    }

    pub fn with_base_url(base_url: &str) -> Self {
        let client = Client::builder()
            .user_agent("MusicPlayer/0.1.0 ( https://github.com/example/music-player )")
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            client,
            last_request_ms: Arc::new(Mutex::new(0)),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Enforces MusicBrainz's strict 1 request per second rate-limit.
    async fn throttle(&self) {
        let mut last = self.last_request_ms.lock().await;
        let now = chrono::Utc::now().timestamp_millis();
        let elapsed = now - *last;
        if elapsed < 1000 && *last != 0 {
            let wait_ms = (1000 - elapsed) as u64;
            tokio::time::sleep(Duration::from_millis(wait_ms)).await;
        }
        *last = chrono::Utc::now().timestamp_millis();
    }
}

impl Default for MusicBrainzProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl super::MetadataProvider for MusicBrainzProvider {
    fn name(&self) -> &'static str {
        "musicbrainz"
    }

    fn is_available(&self) -> bool {
        true
    }

    async fn search_track(
        &self,
        title: &str,
        artist: &str,
        album: Option<&str>,
    ) -> AppResult<Vec<ExternalTrackMetadata>> {
        self.throttle().await;

        let query = if let Some(alb) = album {
            format!("recording:\"{}\" AND artist:\"{}\" AND release:\"{}\"", title, artist, alb)
        } else {
            format!("recording:\"{}\" AND artist:\"{}\"", title, artist)
        };

        let url = format!("{}/recording", self.base_url);
        let resp = self
            .client
            .get(&url)
            .query(&[("query", query.as_str()), ("fmt", "json"), ("limit", "5")])
            .send()
            .await
            .map_err(|e| AppError::Network(format!("MusicBrainz query failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(AppError::Network(format!("MusicBrainz returned status {}", resp.status())));
        }

        let json: Value = resp.json().await.map_err(|e| AppError::Network(e.to_string()))?;
        let recordings = json["recordings"].as_array();
        let mut results = Vec::new();

        if let Some(recs) = recordings {
            for r in recs {
                let id = r["id"].as_str().unwrap_or_default().to_string();
                let rec_title = r["title"].as_str().unwrap_or_default().to_string();
                let artist_name = r["artist-credit"][0]["name"].as_str().unwrap_or(artist).to_string();
                let artist_id = r["artist-credit"][0]["artist"]["id"].as_str().map(|s| s.to_string());

                let first_release = r["releases"].as_array().and_then(|a| a.first());
                let album_title = first_release.and_then(|rel| rel["title"].as_str()).map(|s| s.to_string());
                let album_id = first_release.and_then(|rel| rel["id"].as_str()).map(|s| s.to_string());
                let year = first_release
                    .and_then(|rel| rel["date"].as_str())
                    .and_then(|d| d.split('-').next())
                    .and_then(|y| y.parse::<i64>().ok());

                let duration_secs = r["length"].as_f64().map(|ms| ms / 1000.0);

                results.push(ExternalTrackMetadata {
                    provider: "musicbrainz".to_string(),
                    provider_track_id: id,
                    title: rec_title,
                    artist_name,
                    artist_id,
                    album_title,
                    album_id,
                    duration_secs,
                    year,
                    track_number: None,
                    cover_art_url: None,
                });
            }
        }

        Ok(results)
    }

    async fn search_artist(&self, name: &str) -> AppResult<Option<ExternalArtistMetadata>> {
        self.throttle().await;

        let url = format!("{}/artist", self.base_url);
        let resp = self
            .client
            .get(&url)
            .query(&[("query", format!("artist:\"{}\"", name).as_str()), ("fmt", "json"), ("limit", "1")])
            .send()
            .await
            .map_err(|e| AppError::Network(format!("MusicBrainz artist search failed: {}", e)))?;

        if !resp.status().is_success() {
            return Ok(None);
        }

        let json: Value = resp.json().await.map_err(|e| AppError::Network(e.to_string()))?;
        let artist_obj = json["artists"].as_array().and_then(|a| a.first());

        if let Some(art) = artist_obj {
            let id = art["id"].as_str().unwrap_or_default().to_string();
            let artist_name = art["name"].as_str().unwrap_or(name).to_string();
            let bio = art["disambiguation"].as_str().map(|d| d.to_string());

            let mut genres = Vec::new();
            if let Some(tags) = art["tags"].as_array() {
                for t in tags {
                    if let Some(g) = t["name"].as_str() {
                        genres.push(g.to_string());
                    }
                }
            }

            Ok(Some(ExternalArtistMetadata {
                provider: "musicbrainz".to_string(),
                provider_artist_id: id,
                name: artist_name,
                bio,
                image_url: None,
                genres,
            }))
        } else {
            Ok(None)
        }
    }

    async fn search_album(&self, title: &str, artist: &str) -> AppResult<Option<ExternalAlbumMetadata>> {
        self.throttle().await;

        let query = format!("release:\"{}\" AND artist:\"{}\"", title, artist);
        let url = format!("{}/release", self.base_url);
        let resp = self
            .client
            .get(&url)
            .query(&[("query", query.as_str()), ("fmt", "json"), ("limit", "1")])
            .send()
            .await
            .map_err(|e| AppError::Network(format!("MusicBrainz release search failed: {}", e)))?;

        if !resp.status().is_success() {
            return Ok(None);
        }

        let json: Value = resp.json().await.map_err(|e| AppError::Network(e.to_string()))?;
        let rel_obj = json["releases"].as_array().and_then(|a| a.first());

        if let Some(rel) = rel_obj {
            let id = rel["id"].as_str().unwrap_or_default().to_string();
            let album_title = rel["title"].as_str().unwrap_or(title).to_string();
            let artist_name = rel["artist-credit"][0]["name"].as_str().unwrap_or(artist).to_string();
            let year = rel["date"]
                .as_str()
                .and_then(|d| d.split('-').next())
                .and_then(|y| y.parse::<i64>().ok());
            let total_tracks = rel["track-count"].as_u64().map(|c| c as u32);

            Ok(Some(ExternalAlbumMetadata {
                provider: "musicbrainz".to_string(),
                provider_album_id: id,
                title: album_title,
                artist_name,
                release_year: year,
                total_tracks,
                cover_art_url: None,
            }))
        } else {
            Ok(None)
        }
    }

    async fn fetch_cover_art(&self, _release_id: &str) -> AppResult<Option<Vec<u8>>> {
        // MusicBrainz delegates cover art to Cover Art Archive
        Ok(None)
    }
}
