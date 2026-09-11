use super::types::{ExternalAlbumMetadata, ExternalArtistMetadata, ExternalTrackMetadata};
use crate::core::error::{AppError, AppResult};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::warn;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SpotifyAuthResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
}

pub struct SpotifyProvider {
    client: Client,
    client_id: Option<String>,
    client_secret: Option<String>,
    token_cache: Arc<Mutex<Option<(String, i64)>>>, // (token, expires_at_timestamp)
    api_base_url: String,
    auth_url: String,
}

impl SpotifyProvider {
    pub fn new(client_id: Option<String>, client_secret: Option<String>) -> Self {
        Self::with_endpoints(
            client_id,
            client_secret,
            "https://api.spotify.com/v1",
            "https://accounts.spotify.com/api/token",
        )
    }

    pub fn with_endpoints(
        client_id: Option<String>,
        client_secret: Option<String>,
        api_base_url: &str,
        auth_url: &str,
    ) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            client,
            client_id,
            client_secret,
            token_cache: Arc::new(Mutex::new(None)),
            api_base_url: api_base_url.trim_end_matches('/').to_string(),
            auth_url: auth_url.to_string(),
        }
    }

    /// Sets or updates Spotify API credentials at runtime.
    pub async fn set_credentials(&mut self, client_id: Option<String>, client_secret: Option<String>) {
        self.client_id = client_id;
        self.client_secret = client_secret;
        let mut cache = self.token_cache.lock().await;
        *cache = None;
    }

    /// Fetches or returns a valid cached OAuth access token via Client Credentials flow.
    async fn get_token(&self) -> AppResult<String> {
        let now = chrono::Utc::now().timestamp();
        let mut cache = self.token_cache.lock().await;

        if let Some((token, expires_at)) = &*cache {
            if now < (*expires_at - 60) {
                return Ok(token.clone());
            }
        }

        let (cid, csec) = match (&self.client_id, &self.client_secret) {
            (Some(id), Some(sec)) if !id.trim().is_empty() && !sec.trim().is_empty() => (id, sec),
            _ => return Err(AppError::Network("Spotify credentials not configured".into())),
        };

        let params = [("grant_type", "client_credentials")];
        let resp = self
            .client
            .post(&self.auth_url)
            .basic_auth(cid, Some(csec))
            .form(&params)
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Spotify auth failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(AppError::Network(format!(
                "Spotify auth returned status {}",
                resp.status()
            )));
        }

        let auth_data: SpotifyAuthResponse = resp
            .json()
            .await
            .map_err(|e| AppError::Network(e.to_string()))?;

        let expires_at = now + auth_data.expires_in;
        *cache = Some((auth_data.access_token.clone(), expires_at));

        Ok(auth_data.access_token)
    }
}

#[async_trait]
impl super::MetadataProvider for SpotifyProvider {
    fn name(&self) -> &'static str {
        "spotify"
    }

    fn is_available(&self) -> bool {
        self.client_id.is_some() && self.client_secret.is_some()
    }

    async fn search_track(
        &self,
        title: &str,
        artist: &str,
        album: Option<&str>,
    ) -> AppResult<Vec<ExternalTrackMetadata>> {
        if !self.is_available() {
            return Ok(Vec::new());
        }

        let token = match self.get_token().await {
            Ok(t) => t,
            Err(e) => {
                warn!(error = %e, "Skipping Spotify search due to auth error");
                return Ok(Vec::new());
            }
        };

        let query = if let Some(alb) = album {
            format!("track:\"{}\" artist:\"{}\" album:\"{}\"", title, artist, alb)
        } else {
            format!("track:\"{}\" artist:\"{}\"", title, artist)
        };

        let url = format!("{}/search", self.api_base_url);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .query(&[("q", query.as_str()), ("type", "track"), ("limit", "5")])
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Spotify search request failed: {}", e)))?;

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let json: Value = resp.json().await.map_err(|e| AppError::Network(e.to_string()))?;
        let tracks = json["tracks"]["items"].as_array();
        let mut results = Vec::new();

        if let Some(items) = tracks {
            for item in items {
                let id = item["id"].as_str().unwrap_or_default().to_string();
                let track_name = item["name"].as_str().unwrap_or_default().to_string();
                let artist_name = item["artists"][0]["name"].as_str().unwrap_or(artist).to_string();
                let artist_id = item["artists"][0]["id"].as_str().map(|s| s.to_string());
                let album_title = item["album"]["name"].as_str().map(|s| s.to_string());
                let album_id = item["album"]["id"].as_str().map(|s| s.to_string());
                let duration_secs = item["duration_ms"].as_f64().map(|ms| ms / 1000.0);
                let track_number = item["track_number"].as_u64().map(|n| n as u32);
                let cover_url = item["album"]["images"][0]["url"].as_str().map(|s| s.to_string());

                let year = item["album"]["release_date"]
                    .as_str()
                    .and_then(|d| d.split('-').next())
                    .and_then(|y| y.parse::<i64>().ok());

                results.push(ExternalTrackMetadata {
                    provider: "spotify".to_string(),
                    provider_track_id: id,
                    title: track_name,
                    artist_name,
                    artist_id,
                    album_title,
                    album_id,
                    duration_secs,
                    year,
                    track_number,
                    cover_art_url: cover_url,
                });
            }
        }

        Ok(results)
    }

    async fn search_artist(&self, name: &str) -> AppResult<Option<ExternalArtistMetadata>> {
        if !self.is_available() {
            return Ok(None);
        }

        let token = match self.get_token().await {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };

        let url = format!("{}/search", self.api_base_url);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .query(&[("q", format!("artist:\"{}\"", name).as_str()), ("type", "artist"), ("limit", "1")])
            .send()
            .await
            .map_err(|e| AppError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            return Ok(None);
        }

        let json: Value = resp.json().await.map_err(|e| AppError::Network(e.to_string()))?;
        let artist_item = json["artists"]["items"].as_array().and_then(|a| a.first());

        if let Some(item) = artist_item {
            let id = item["id"].as_str().unwrap_or_default().to_string();
            let artist_name = item["name"].as_str().unwrap_or(name).to_string();
            let image_url = item["images"][0]["url"].as_str().map(|s| s.to_string());
            let genres = item["genres"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|g| g.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();

            Ok(Some(ExternalArtistMetadata {
                provider: "spotify".to_string(),
                provider_artist_id: id,
                name: artist_name,
                bio: None,
                image_url,
                genres,
            }))
        } else {
            Ok(None)
        }
    }

    async fn search_album(&self, title: &str, artist: &str) -> AppResult<Option<ExternalAlbumMetadata>> {
        if !self.is_available() {
            return Ok(None);
        }

        let token = match self.get_token().await {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };

        let query = format!("album:\"{}\" artist:\"{}\"", title, artist);
        let url = format!("{}/search", self.api_base_url);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .query(&[("q", query.as_str()), ("type", "album"), ("limit", "1")])
            .send()
            .await
            .map_err(|e| AppError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            return Ok(None);
        }

        let json: Value = resp.json().await.map_err(|e| AppError::Network(e.to_string()))?;
        let album_item = json["albums"]["items"].as_array().and_then(|a| a.first());

        if let Some(item) = album_item {
            let id = item["id"].as_str().unwrap_or_default().to_string();
            let album_name = item["name"].as_str().unwrap_or(title).to_string();
            let artist_name = item["artists"][0]["name"].as_str().unwrap_or(artist).to_string();
            let cover_url = item["images"][0]["url"].as_str().map(|s| s.to_string());
            let total_tracks = item["total_tracks"].as_u64().map(|n| n as u32);
            let year = item["release_date"]
                .as_str()
                .and_then(|d| d.split('-').next())
                .and_then(|y| y.parse::<i64>().ok());

            Ok(Some(ExternalAlbumMetadata {
                provider: "spotify".to_string(),
                provider_album_id: id,
                title: album_name,
                artist_name,
                release_year: year,
                total_tracks,
                cover_art_url: cover_url,
            }))
        } else {
            Ok(None)
        }
    }

    async fn fetch_cover_art(&self, _release_id: &str) -> AppResult<Option<Vec<u8>>> {
        Ok(None)
    }
}
