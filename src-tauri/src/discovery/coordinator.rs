use super::matcher::{FuzzyTrackMatcher, MatchResult, MatchStatus};
use crate::core::error::AppResult;
use crate::database::models::ExternalTrackRecord;
use crate::database::repositories::{TrackRepository, WishlistRepository};
use crate::providers::{MetadataProvider, ProviderCoordinator};
use crate::recommendations::TasteProfileEngine;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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

    /// Generates external discovery recommendations tailored to user taste affinities,
    /// prioritizing tracks that do not currently exist in the user's local library.
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
            .list_external_tracks(None, (limit * 3).max(50) as u32)
            .await?;

        // 3. If external tracks pool is small and provider coordinator is available,
        // discover candidates based on taste profile
        if ext_tracks.len() < limit {
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

                        for item in res.into_iter().take(5) {
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
                .list_external_tracks(None, (limit * 3).max(50) as u32)
                .await?;
        }

        // 4. Transform candidate external tracks into recommendations
        let mut recs = Vec::new();
        for track in ext_tracks {
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

            let reason = match status {
                MatchStatus::NotFound => format!(
                    "Missing from library - Recommended based on affinity for {}",
                    track.artist
                ),
                MatchStatus::PossibleMatch => {
                    "Possible alternate version of local track - Check if you want this release".to_string()
                }
                MatchStatus::LikelyMatch => "Likely duplicate or remastered local track".to_string(),
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

        // 5. Sort to prioritize unowned music (NotFound > PossibleMatch > LikelyMatch > ExactMatch)
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
