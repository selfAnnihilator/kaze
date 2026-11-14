use crate::recommendations::mixes::OnlineTrackInput;
use crate::core::error::{AppError, AppResult};
use crate::recommendations::local::RecommendedTrackDetail;
use crate::recommendations::scoring::{CandidateTrack, ScoringEngine};
use crate::recommendations::session::{SessionSteer, StyleKeys, StyleKind};
use crate::recommendations::taste::{TasteProfile, TasteProfileEngine};
use chrono::Utc;

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

mod candidates;
use candidates::CandidateRequest;
mod home;
mod explain;
mod types;
pub use types::*;

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct FlowCandidateRow {
    id: String,
    title: String,
    artist_id: Option<String>,
    artist_name: Option<String>,
    album_id: Option<String>,
    album_title: Option<String>,
    genre_name: Option<String>,
    year: Option<i64>,
    duration_secs: f64,
    cover_art_url: Option<String>,
    play_count: Option<i64>,
    completion_count: Option<i64>,
    skip_count: Option<i64>,
    last_played_at: Option<i64>,
    manual_like: Option<i64>,
    is_online: bool,
    preview_url: Option<String>,
}

/// Cached home picks per user: (computed_at, history_count_at_compute, tracks).
type HomePickEntry = (i64, i64, Vec<RecommendedTrackDetail>);

pub struct FlowEngine {
    pool: SqlitePool,
    taste_engine: Arc<TasteProfileEngine>,
    home_picks: tokio::sync::Mutex<HashMap<String, HomePickEntry>>,
}

impl FlowEngine {
    pub fn new(pool: SqlitePool) -> Self {
        let taste_engine = Arc::new(TasteProfileEngine::new(pool.clone()));
        Self {
            pool,
            taste_engine,
            home_picks: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Normalized style keys (artist / genre / language) for one played track.
    /// Same joins as candidate generation, for in-session steering.
    pub async fn style_keys(&self, track_id: &str) -> AppResult<StyleKeys> {
        let row: Option<(Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT COALESCE(a.name, e.artist), COALESCE(g.name, e.genre),
                    COALESCE(e.language, canonical.language, json_extract(s.metadata_json, '$.language'))
             FROM (SELECT ? AS id) req
             LEFT JOIN tracks t ON t.id = req.id
             LEFT JOIN artists a ON a.id = t.artist_id
             LEFT JOIN genres g ON g.id = t.genre_id
             LEFT JOIN external_tracks e ON e.id = req.id
             LEFT JOIN online_track_sources s ON s.track_id = req.id
             LEFT JOIN external_tracks canonical
                    ON canonical.provider = 'jiosaavn' AND canonical.provider_id = s.provider_id
             LIMIT 1",
        )
        .bind(track_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
        let (artist, genre, language) = row.unwrap_or((None, None, None));
        Ok(StyleKeys::new(
            artist.as_deref(),
            genre.as_deref(),
            language.as_deref(),
        ))
    }

    /// Generates next queue tracks using deterministic Flow scoring.
    ///
    /// Steering: `familiarity_ratio`:
    ///   1.0 -> High familiarity (top favorites, high affinity)
    ///   0.5 -> Balanced (familiar + forgotten favorites + fresh picks)
    ///   0.0 -> High rediscovery (unplayed gems, tracks unheard for months)
    pub async fn generate_flow_tracks(
        &self,
        user_id: &str,
        familiarity_ratio: f64,
        limit: usize,
        exclude_track_ids: &[String],
    ) -> AppResult<Vec<FlowTrackItem>> {
        self.generate_flow_tracks_after(user_id, familiarity_ratio, limit, exclude_track_ids, &[])
            .await
    }

    pub async fn generate_flow_tracks_after(
        &self,
        user_id: &str,
        familiarity_ratio: f64,
        limit: usize,
        exclude_track_ids: &[String],
        recent_artists: &[String],
    ) -> AppResult<Vec<FlowTrackItem>> {
        self.generate_flow_tracks_with_session(
            user_id,
            familiarity_ratio,
            limit,
            exclude_track_ids,
            recent_artists,
            None,
        )
        .await
    }

    /// Library-only Flow with in-session steering (live skips / full listens).
    pub async fn generate_flow_tracks_with_session(
        &self,
        user_id: &str,
        familiarity_ratio: f64,
        limit: usize,
        exclude_track_ids: &[String],
        recent_artists: &[String],
        session: Option<&SessionSteer>,
    ) -> AppResult<Vec<FlowTrackItem>> {
        self.generate_candidates(CandidateRequest {
            user_id,
            familiarity_ratio,
            limit,
            exclude_track_ids,
            recent_artists,
            include_online: false,
            session,
            transitions: None,
        })
        .await
    }

    /// Runtime Flow includes account history and taste-matched online catalog metadata.
    /// Selection performs no network/audio work; the shared player resolves a song on play.
    pub async fn generate_playback_tracks_after(
        &self,
        user_id: &str,
        familiarity_ratio: f64,
        limit: usize,
        exclude_track_ids: &[String],
        recent_artists: &[String],
    ) -> AppResult<Vec<FlowTrackItem>> {
        self.generate_playback_tracks_with_session(
            user_id,
            familiarity_ratio,
            limit,
            exclude_track_ids,
            recent_artists,
            None,
        )
        .await
    }

    /// Runtime Flow with in-session steering (live skips / full listens).
    pub async fn generate_playback_tracks_with_session(
        &self,
        user_id: &str,
        familiarity_ratio: f64,
        limit: usize,
        exclude_track_ids: &[String],
        recent_artists: &[String],
        session: Option<&SessionSteer>,
    ) -> AppResult<Vec<FlowTrackItem>> {
        self.generate_candidates(CandidateRequest {
            user_id,
            familiarity_ratio,
            limit,
            exclude_track_ids,
            recent_artists,
            include_online: true,
            session,
            transitions: None,
        })
        .await
    }

    /// Runtime Flow whose first pick should sound close to `seed_track_id` (the
    /// track that plays right before the new batch). Ordering then prefers small
    /// tempo/energy/brightness steps where local audio features are known.
    #[allow(clippy::too_many_arguments)]
    pub async fn generate_playback_tracks_seeded(
        &self,
        user_id: &str,
        familiarity_ratio: f64,
        limit: usize,
        exclude_track_ids: &[String],
        recent_artists: &[String],
        session: Option<&SessionSteer>,
        seed_track_id: Option<&str>,
    ) -> AppResult<Vec<FlowTrackItem>> {
        self.generate_candidates(CandidateRequest {
            user_id,
            familiarity_ratio,
            limit,
            exclude_track_ids,
            recent_artists,
            include_online: true,
            session,
            transitions: Some(seed_track_id),
        })
        .await
    }

    /// Retrieves persisted Flow session state.
    pub async fn get_flow_session(&self, user_id: &str) -> AppResult<Option<FlowSessionState>> {
        let key = format!("flow_session:{}", user_id);
        let row: Option<(String,)> =
            sqlx::query_as("SELECT value FROM application_settings WHERE key = ?")
                .bind(&key)
                .fetch_optional(&self.pool)
                .await?;

        if let Some((json_str,)) = row {
            if let Ok(state) = serde_json::from_str::<FlowSessionState>(&json_str) {
                return Ok(Some(state));
            }
        }

        Ok(None)
    }

    /// Persists Flow session state.
    pub async fn save_flow_session(
        &self,
        user_id: &str,
        state: &FlowSessionState,
    ) -> AppResult<()> {
        let key = format!("flow_session:{}", user_id);
        let json_str =
            serde_json::to_string(state).map_err(|e| AppError::Database(e.to_string()))?;
        let now = Utc::now().timestamp();

        sqlx::query(
            "INSERT INTO application_settings (key, value, updated_at) VALUES (?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at"
        )
        .bind(&key)
        .bind(&json_str)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

}
