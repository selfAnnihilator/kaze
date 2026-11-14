use super::matcher::{FuzzyTrackMatcher, LocalTrackCandidate, MatchResult, MatchStatus};
use crate::core::error::AppResult;
use crate::database::models::ExternalTrackRecord;
use crate::database::repositories::{TrackRepository, WishlistRepository};
use crate::providers::ProviderCoordinator;
use crate::recommendations::TasteProfileEngine;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

mod charts;
mod starters;
mod for_you;

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
    pub preview_url: Option<String>,
    pub genre: Option<String>,
    pub match_status: MatchStatus,
    pub matched_local_track_id: Option<String>,
    pub recommendation_reason: String,
    pub in_wishlist: bool,
}

#[derive(Default)]
struct ForYouSelection {
    generation: u64,
    last_shown: HashMap<String, u64>,
    current: HashMap<String, usize>,
    ranked_ids: Vec<String>,
    touched: Option<std::time::Instant>,
}

pub struct DiscoveryCoordinator {
    wishlist_repo: Arc<dyn WishlistRepository>,
    track_repo: Arc<dyn TrackRepository>,
    taste_engine: Arc<TasteProfileEngine>,
    provider_coordinator: Option<Arc<ProviderCoordinator>>,
    artist_edition_cache: tokio::sync::Mutex<HashMap<String, std::time::Instant>>,
    artist_candidate_cache:
        tokio::sync::Mutex<HashMap<String, (std::time::Instant, Vec<ExternalTrackRecord>)>>,
    for_you_selections: tokio::sync::Mutex<HashMap<String, ForYouSelection>>,
    starter_fetch_lock: tokio::sync::Mutex<()>,
    starter_attempts: tokio::sync::Mutex<HashMap<String, (i64, std::time::Instant)>>,
    world_fetch_lock: tokio::sync::Mutex<()>,
    world_chart_cache:
        tokio::sync::RwLock<Option<(std::time::Instant, Vec<DiscoveryRecommendation>)>>,
}

/// Checks if a candidate genre fuzzy matches a user's library genre
pub fn genre_matches(candidate_genre: &str, user_genre: &str) -> bool {
    let cg = candidate_genre.to_lowercase();
    let ug = user_genre.to_lowercase();
    if cg.is_empty() || ug.is_empty() {
        return false;
    }
    if cg == ug || cg.contains(&ug) || ug.contains(&cg) {
        return true;
    }
    let is_rap_hiphop = (cg.contains("hip-hop") || cg.contains("hip hop") || cg.contains("rap"))
        && (ug.contains("hip-hop") || ug.contains("hip hop") || ug.contains("rap"));
    let is_electronic =
        (cg.contains("dance") || cg.contains("electronic") || cg.contains("electro"))
            && (ug.contains("dance") || ug.contains("electronic") || ug.contains("electro"));
    let is_rock_alt = (cg.contains("rock") || cg.contains("alternative"))
        && (ug.contains("rock") || ug.contains("alternative"));
    let is_rnb =
        (cg.contains("r&b") || cg.contains("soul")) && (ug.contains("r&b") || ug.contains("soul"));

    let is_asian = (cg.contains("asian")
        || cg.contains("j-pop")
        || cg.contains("anime")
        || cg.contains("k-pop")
        || cg.contains("japanese"))
        && (ug.contains("asian")
            || ug.contains("j-pop")
            || ug.contains("anime")
            || ug.contains("k-pop")
            || ug.contains("japanese"));
    let is_latin_brazil = (cg.contains("brazil")
        || cg.contains("latin")
        || cg.contains("samba")
        || cg.contains("bossa"))
        && (ug.contains("brazil")
            || ug.contains("latin")
            || ug.contains("samba")
            || ug.contains("bossa"));
    let is_indian = (cg.contains("bollywood")
        || cg.contains("indian")
        || cg.contains("desi")
        || cg.contains("hindi")
        || cg.contains("punjabi"))
        && (ug.contains("bollywood")
            || ug.contains("indian")
            || ug.contains("desi")
            || ug.contains("hindi")
            || ug.contains("punjabi"));
    let is_malayalam =
        (cg.contains("malayalam") || cg.contains("regional indian") || cg.contains("indian"))
            && (ug.contains("malayalam")
                || ug.contains("asian")
                || ug.contains("indian")
                || ug.contains("soundtrack"));

    is_rap_hiphop
        || is_electronic
        || is_rock_alt
        || is_rnb
        || is_asian
        || is_latin_brazil
        || is_indian
        || is_malayalam
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
            artist_edition_cache: tokio::sync::Mutex::new(HashMap::new()),
            artist_candidate_cache: tokio::sync::Mutex::new(HashMap::new()),
            for_you_selections: tokio::sync::Mutex::new(HashMap::new()),
            starter_fetch_lock: tokio::sync::Mutex::new(()),
            starter_attempts: tokio::sync::Mutex::new(HashMap::new()),
            world_fetch_lock: tokio::sync::Mutex::new(()),
            world_chart_cache: tokio::sync::RwLock::new(None),
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
        is_lofi_context: bool,
    ) -> AppResult<MatchResult> {
        let local_tracks = self.track_repo.list_tracks(0, 50000, None, true).await?;

        let candidates = local_tracks
            .iter()
            .filter(|t| {
                t.format != "online"
                    && !t.file_path.starts_with("online://")
                    && !t.id.starts_with("itunes:")
                    && !t.id.starts_with("online:")
            })
            .map(|t| {
                let fp = t.file_path.to_lowercase();
                let genre = t.genre_name.as_deref().unwrap_or("").to_lowercase();
                let album = t.album_title.as_deref().unwrap_or("").to_lowercase();
                let title = t.title.to_lowercase();
                let artist = t.artist_name.as_deref().unwrap_or("").to_lowercase();

                let is_lofi = fp.contains("/lofi/")
                    || fp.contains("/lo-fi/")
                    || fp.contains("lofi")
                    || fp.contains("lo-fi")
                    || genre.contains("lofi")
                    || genre.contains("lo-fi")
                    || genre.contains("chillhop")
                    || album.contains("lofi")
                    || album.contains("lo-fi")
                    || title.contains("lofi")
                    || title.contains("lo-fi")
                    || artist.contains("lofi")
                    || artist.contains("lo-fi");

                LocalTrackCandidate {
                    id: t.id.as_str(),
                    title: t.title.as_str(),
                    artist: t.artist_name.as_deref().unwrap_or(""),
                    duration_secs: t.duration_secs,
                    is_lofi,
                }
            });

        Ok(FuzzyTrackMatcher::find_best_match(
            ext_title,
            ext_artist,
            ext_duration,
            is_lofi_context,
            candidates,
        ))
    }

    /// Ingests or updates an external track candidate, running fuzzy matching
    /// against the local library to determine match status and saving to database.
    pub async fn ingest_external_track(
        &self,
        mut track: ExternalTrackRecord,
    ) -> AppResult<MatchResult> {
        let is_lofi = FuzzyTrackMatcher::is_lofi_indicator(&track.title)
            || FuzzyTrackMatcher::is_lofi_indicator(&track.artist);
        let match_result = self
            .match_against_library(&track.title, &track.artist, track.duration_secs, is_lofi)
            .await?;

        track.match_status = match_result.status.as_str().to_string();
        track.matched_local_track_id = match_result.matched_track_id.clone();

        self.wishlist_repo.upsert_external_track(&track).await?;
        Ok(match_result)
    }

}

fn recommendation(row: ExternalTrackRecord, reason: &str) -> DiscoveryRecommendation {
    DiscoveryRecommendation {
        external_track_id: row.id,
        provider: row.provider,
        provider_id: row.provider_id,
        title: row.title,
        artist: row.artist,
        album: row.album,
        duration_secs: row.duration_secs,
        cover_art_url: row.cover_art_url,
        preview_url: None,
        genre: row.genre,
        match_status: MatchStatus::NotFound,
        matched_local_track_id: None,
        recommendation_reason: reason.into(),
        in_wishlist: false,
    }
}
