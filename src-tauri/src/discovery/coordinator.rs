use super::matcher::{FuzzyTrackMatcher, MatchResult, MatchStatus};
use crate::core::error::AppResult;
use crate::database::models::ExternalTrackRecord;
use crate::database::repositories::{TrackRepository, WishlistRepository};
use crate::providers::{MetadataProvider, ProviderCoordinator};
use crate::recommendations::TasteProfileEngine;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryRecommendation {
    pub external_track_id: String,
    pub provider: String,
    pub provider_id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_secs: Option<f64>,
    pub cover_art_url: Option<String>,
    pub match_status: MatchStatus,
    pub matched_local_track_id: Option<String>,
    pub recommendation_reason: String,
    pub in_wishlist: bool,
}

pub struct DiscoveryCoordinator {
    wishlist_repo: Arc<dyn WishlistRepository>,
    track_repo: Arc<dyn TrackRepository>,
    taste_engine: Arc<TasteProfileEngine>,
    provider_coordinator: Option<Arc<ProviderCoordinator>>,
}

impl DiscoveryCoordinator {
    pub fn new(
        wishlist_repo: Arc<dyn WishlistRepository>,
        track_repo: Arc<dyn TrackRepository>,
        taste_engine: Arc<TasteProfileEngine>,
        provider_coordinator: Option<Arc<ProviderCoordinator>>,
    ) -> Self {
        Self {
            wishlist_repo,
            track_repo,
            taste_engine,
            provider_coordinator,
        }
    }

    pub fn wishlist_repo(&self) -> Arc<dyn WishlistRepository> {
        self.wishlist_repo.clone()
    }

    /// Evaluates fuzzy similarity for an external track candidate against all local library tracks.
    pub async fn match_against_library(
        &self,
        ext_title: &str,
        ext_artist: &str,
        ext_duration: Option<f64>,
    ) -> AppResult<MatchResult> {
        let local_tracks = self.track_repo.list_tracks(0, 50000, None, true).await?;

        let candidates = local_tracks.iter().map(|t| {
            (
                t.id.as_str(),
                t.title.as_str(),
                t.artist_name.as_deref().unwrap_or(""),
                t.duration_secs,
            )
        });

        Ok(FuzzyTrackMatcher::find_best_match(
            ext_title,
            ext_artist,
            ext_duration,
            candidates,
        ))
    }

    /// Ingests or updates an external track candidate, running fuzzy matching
    /// against the local library to determine match status and saving to database.
    pub async fn ingest_external_track(
        &self,
        mut track: ExternalTrackRecord,
    ) -> AppResult<MatchResult> {
        let match_result = self
            .match_against_library(&track.title, &track.artist, track.duration_secs)
            .await?;

        track.match_status = match_result.status.as_str().to_string();
        track.matched_local_track_id = match_result.matched_track_id.clone();

        self.wishlist_repo.upsert_external_track(&track).await?;
        Ok(match_result)
    }

    /// Fetches trending hits and popular new music across multiple genres from the public charts.
    pub async fn fetch_trending_and_genre_candidates(&self) -> AppResult<usize> {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(4))
            .build()
        {
            Ok(c) => c,
            Err(_) => return Ok(0),
        };

        let url = "https://itunes.apple.com/us/rss/topsongs/limit=50/json";
        let resp = match client.get(url).send().await {
            Ok(r) if r.status().is_success() => r,
            _ => return Ok(0),
        };

        let json_val: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => return Ok(0),
        };

        let entries = json_val
            .get("feed")
            .and_then(|f| f.get("entry"))
            .and_then(|e| e.as_array());

        let mut ingested_count = 0;
        if let Some(entries) = entries {
            for entry in entries {
                let title = entry
                    .get("im:name")
                    .and_then(|n| n.get("label"))
                    .and_then(|l| l.as_str())
                    .unwrap_or("")
                    .trim();

                let artist = entry
                    .get("im:artist")
                    .and_then(|a| a.get("label"))
                    .and_then(|l| l.as_str())
                    .unwrap_or("")
                    .trim();

                if title.is_empty() || artist.is_empty() {
                    continue;
                }

                let album = entry
                    .get("im:collection")
                    .and_then(|c| c.get("im:name"))
                    .and_then(|n| n.get("label"))
                    .and_then(|l| l.as_str())
                    .map(|s| s.to_string());

                let cover_art_url = entry
                    .get("im:image")
                    .and_then(|imgs| imgs.as_array())
                    .and_then(|arr| arr.last())
                    .and_then(|img| img.get("label"))
                    .and_then(|l| l.as_str())
                    .map(|s| s.to_string());

                let itunes_id = entry
                    .get("id")
                    .and_then(|i| i.get("attributes"))
                    .and_then(|a| a.get("im:id"))
                    .and_then(|id| id.as_str())
                    .unwrap_or("");

                let genre = entry
                    .get("category")
                    .and_then(|c| c.get("attributes"))
                    .and_then(|a| a.get("label"))
                    .and_then(|l| l.as_str())
                    .unwrap_or("Trending");

                let record = ExternalTrackRecord {
                    id: format!("itunes:{}", if itunes_id.is_empty() { format!("{}:{}", artist, title) } else { itunes_id.to_string() }),
                    provider: "itunes".to_string(),
                    provider_id: if itunes_id.is_empty() { title.to_string() } else { itunes_id.to_string() },
                    title: title.to_string(),
                    artist: artist.to_string(),
                    album: album.or_else(|| Some(genre.to_string())),
                    duration_secs: Some(210.0),
                    cover_art_url,
                    match_status: MatchStatus::NotFound.as_str().to_string(),
                    matched_local_track_id: None,
                    created_at: chrono::Utc::now().timestamp(),
                };

                let _ = self.ingest_external_track(record).await;
                ingested_count += 1;
            }
        }

        Ok(ingested_count)
    }

    /// Generates external discovery recommendations tailored to user taste affinities,
    /// genres, trending charts, and similar artists with strict diversity enforcement.
    pub async fn get_discovery_recommendations(
        &self,
        limit: usize,
    ) -> AppResult<Vec<DiscoveryRecommendation>> {
        // 1. Fetch current wishlist to cross-reference
        let wishlist_items = self.wishlist_repo.get_all(None).await?;
        let wishlist_ext_ids: HashSet<String> = wishlist_items
            .iter()
            .filter_map(|w| w.external_track_id.clone())
            .collect();
        let wishlist_keys: HashSet<String> = wishlist_items
            .iter()
            .map(|w| format!("{}:{}", w.artist.to_lowercase(), w.title.to_lowercase()))
            .collect();

        // 2. Fetch candidate external tracks from database
        let mut ext_tracks = self
            .wishlist_repo
            .list_external_tracks(None, 200)
            .await?;

        // 3. Check artist diversity in existing external tracks
        let unique_artists: HashSet<_> = ext_tracks
            .iter()
            .map(|t| t.artist.to_lowercase().trim().to_string())
            .collect();

        // If pool is small or dominated by only 1-2 artists, fetch fresh trending & genre candidates
        if self.provider_coordinator.is_some() && (unique_artists.len() < 5 || ext_tracks.len() < limit) {
            let _ = self.fetch_trending_and_genre_candidates().await;

            // Also check taste profile or provider coordinator if available
            if let Ok(profile) = self.taste_engine.compute_taste_profile().await {
                if let Some(ref pc) = self.provider_coordinator {
                    for artist_aff in profile.top_artists.iter().take(3) {
                        let artist_name = &artist_aff.display_name;
                        let res = if pc.spotify().is_available() {
                            pc.spotify().search_track("", artist_name, None).await.unwrap_or_default()
                        } else if pc.musicbrainz().is_available() {
                            pc.musicbrainz().search_track("", artist_name, None).await.unwrap_or_default()
                        } else {
                            Vec::new()
                        };

                        for item in res.into_iter().take(3) {
                            let record = ExternalTrackRecord {
                                id: format!("{}:{}", item.provider, item.provider_track_id),
                                provider: item.provider,
                                provider_id: item.provider_track_id,
                                title: item.title,
                                artist: item.artist_name,
                                album: item.album_title,
                                duration_secs: item.duration_secs,
                                cover_art_url: item.cover_art_url,
                                match_status: MatchStatus::NotFound.as_str().to_string(),
                                matched_local_track_id: None,
                                created_at: chrono::Utc::now().timestamp(),
                            };
                            let _ = self.ingest_external_track(record).await;
                        }
                    }
                }
            }

            // Re-fetch external tracks after ingestion
            ext_tracks = self
                .wishlist_repo
                .list_external_tracks(None, 200)
                .await?;
        }

        // 4. Compute user top genres and artists from local library + taste profile
        let local_tracks = self.track_repo.list_tracks(0, 50000, None, true).await.unwrap_or_default();
        let mut top_genres: HashSet<String> = HashSet::new();
        let mut library_artists: HashSet<String> = HashSet::new();

        for t in &local_tracks {
            if let Some(ref g) = t.genre_name {
                let clean = g.to_lowercase().trim().to_string();
                if !clean.is_empty() {
                    top_genres.insert(clean);
                }
            }
            if let Some(ref a) = t.artist_name {
                let clean = a.to_lowercase().trim().to_string();
                if !clean.is_empty() {
                    library_artists.insert(clean);
                }
            }
        }

        let mut listened_artists: HashSet<String> = HashSet::new();
        if let Ok(profile) = self.taste_engine.compute_taste_profile().await {
            for a in profile.top_artists {
                listened_artists.insert(a.display_name.to_lowercase().trim().to_string());
            }
            for g in profile.top_genres {
                top_genres.insert(g.display_name.to_lowercase().trim().to_string());
            }
        }

        // 5. Enforce STRICT ARTIST DIVERSITY (maximum 2 tracks per artist)
        let mut artist_counts: HashMap<String, usize> = HashMap::new();
        let mut diverse_tracks = Vec::new();

        for track in ext_tracks {
            let artist_key = track.artist.to_lowercase().trim().to_string();
            let count = artist_counts.entry(artist_key.clone()).or_insert(0);
            if *count >= 2 {
                continue; // Never allow more than 2 tracks from the same artist!
            }
            *count += 1;
            diverse_tracks.push(track);
        }

        // 6. Transform into recommendations with tailored intelligent reasons
        let mut recs = Vec::new();
        for track in diverse_tracks {
            let status = match track.match_status.as_str() {
                "EXACT_MATCH" => MatchStatus::ExactMatch,
                "LIKELY_MATCH" => MatchStatus::LikelyMatch,
                "POSSIBLE_MATCH" => MatchStatus::PossibleMatch,
                _ => MatchStatus::NotFound,
            };

            let in_wishlist = (track.provider_id.as_str() != ""
                && wishlist_ext_ids.contains(&track.id))
                || wishlist_keys.contains(&format!(
                    "{}:{}",
                    track.artist.to_lowercase(),
                    track.title.to_lowercase()
                ));

            let artist_lower = track.artist.to_lowercase().trim().to_string();
            let album_genre_str = track.album.as_deref().unwrap_or("").to_lowercase();

            let is_matched_genre = top_genres.iter().any(|g| {
                !g.is_empty() && (album_genre_str.contains(g) || g.contains(&album_genre_str))
            });

            let reason = match status {
                MatchStatus::NotFound => {
                    if listened_artists.contains(&artist_lower) {
                        format!("✨ Similar to your listening habit for {}", track.artist)
                    } else if library_artists.contains(&artist_lower) {
                        format!("🎵 New release by your library artist {}", track.artist)
                    } else if is_matched_genre {
                        format!("🎧 Popular in {} • Matches your top genre", track.album.as_deref().unwrap_or("Genre"))
                    } else {
                        format!("🔥 Trending Chart Hit • Popular discovery across charts")
                    }
                }
                MatchStatus::PossibleMatch => {
                    "Alternate release / version of local track".to_string()
                }
                MatchStatus::LikelyMatch => "Remastered edition of local track".to_string(),
                MatchStatus::ExactMatch => "Already in local library".to_string(),
            };

            recs.push(DiscoveryRecommendation {
                external_track_id: track.id,
                provider: track.provider,
                provider_id: track.provider_id,
                title: track.title,
                artist: track.artist,
                album: track.album,
                duration_secs: track.duration_secs,
                cover_art_url: track.cover_art_url,
                match_status: status,
                matched_local_track_id: track.matched_local_track_id,
                recommendation_reason: reason,
                in_wishlist,
            });
        }

        // 7. Sort: Unowned first (NotFound > PossibleMatch > LikelyMatch > ExactMatch)
        recs.sort_by(|a, b| {
            let order = |s: MatchStatus| match s {
                MatchStatus::NotFound => 0,
                MatchStatus::PossibleMatch => 1,
                MatchStatus::LikelyMatch => 2,
                MatchStatus::ExactMatch => 3,
            };
            order(a.match_status).cmp(&order(b.match_status))
        });

        recs.truncate(limit);
        Ok(recs)
    }
}
