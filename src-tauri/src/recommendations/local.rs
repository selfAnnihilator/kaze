use super::scoring::{CandidateTrack, ScoringEngine, TrackRecommendationScore};
use super::taste::{TasteProfile, TasteProfileEngine};
use crate::core::error::{AppError, AppResult};
use crate::database::repositories::{
    NewRecommendation, RecommendationRepository, SqliteRecommendationRepository,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendedTrackDetail {
    pub track_id: String,
    pub title: String,
    pub artist_name: Option<String>,
    pub album_title: Option<String>,
    pub genre_name: Option<String>,
    pub duration_secs: f64,
    pub score: f64,
    pub reasons: Vec<super::scoring::RecommendationReason>,
    pub is_discovery: bool,
}

#[derive(Debug, FromRow)]
struct CandidateRow {
    id: String,
    title: String,
    artist_id: Option<String>,
    artist_name: Option<String>,
    album_title: Option<String>,
    genre_name: Option<String>,
    year: Option<i64>,
    duration_secs: f64,
    play_count: Option<i64>,
    completion_count: Option<i64>,
    skip_count: Option<i64>,
    last_played_at: Option<i64>,
    manual_like: Option<i64>,
}

pub struct LocalRecommender {
    pool: SqlitePool,
    taste_engine: TasteProfileEngine,
    rec_repo: SqliteRecommendationRepository,
}

impl LocalRecommender {
    pub fn new(pool: SqlitePool) -> Self {
        let taste_engine = TasteProfileEngine::new(pool.clone());
        let rec_repo = SqliteRecommendationRepository::new(pool.clone());
        Self { pool, taste_engine, rec_repo }
    }

    /// Fetches all candidates, evaluates scores against user's taste profile,
    /// enforces diversity (max 2 tracks per artist), and logs the session.
    pub async fn recommend(&self, user_id: &str, limit: usize) -> AppResult<Vec<RecommendedTrackDetail>> {
        let profile = self.taste_engine.compute_taste_profile(user_id).await?;
        self.recommend_with_profile(user_id, &profile, limit).await
    }

    pub async fn recommend_with_profile(&self, user_id: &str, profile: &TasteProfile, limit: usize) -> AppResult<Vec<RecommendedTrackDetail>> {
        let candidates = sqlx::query_as::<_, CandidateRow>(
            "SELECT t.id, t.title, t.artist_id, a.name as artist_name, al.title as album_title,
                    g.name as genre_name, t.year, t.duration_secs,
                    ts.play_count, ts.completion_count, ts.skip_count, ts.last_played_at,
                    ts.manual_like
             FROM tracks t
             LEFT JOIN artists a ON a.id = t.artist_id
             LEFT JOIN albums al ON al.id = t.album_id
             LEFT JOIN genres g ON g.id = t.genre_id
             LEFT JOIN track_statistics ts ON ts.track_id = t.id AND ts.user_id = ?"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let now = Utc::now().timestamp();
        let mut scored_tracks: Vec<(CandidateRow, TrackRecommendationScore)> = Vec::new();

        for c in candidates {
            let feedback = match c.manual_like {
                Some(1) => Some("like".to_string()),
                Some(-1) => Some("dislike".to_string()),
                _ => None,
            };

            let candidate_input = CandidateTrack {
                id: c.id.clone(),
                title: c.title.clone(),
                artist_id: c.artist_id.clone(),
                artist_name: c.artist_name.clone(),
                genre_name: c.genre_name.clone(),
                year: c.year,
                play_count: c.play_count.unwrap_or(0),
                completion_count: c.completion_count.unwrap_or(0),
                skip_count: c.skip_count.unwrap_or(0),
                last_played_at: c.last_played_at,
                user_feedback: feedback,
            };

            let score = ScoringEngine::score_track(&candidate_input, profile, now);
            if score.final_score > 0.0 {
                scored_tracks.push((c, score));
            }
        }

        // Sort descending by final score
        scored_tracks.sort_by(|a, b| b.1.final_score.partial_cmp(&a.1.final_score).unwrap_or(std::cmp::Ordering::Equal));

        // Enforce artist diversity: max 2 tracks per artist
        let mut artist_counts: HashMap<String, usize> = HashMap::new();
        let mut selected: Vec<(CandidateRow, TrackRecommendationScore)> = Vec::new();

        for (candidate, score) in scored_tracks {
            let artist_key = candidate.artist_id.clone().unwrap_or_else(|| "unknown".to_string());
            let count = artist_counts.entry(artist_key).or_insert(0);
            if *count < 2 {
                *count += 1;
                selected.push((candidate, score));
                if selected.len() >= limit {
                    break;
                }
            }
        }

        // Prepare records to save in recommendation session
        let session_id = uuid::Uuid::new_v4().to_string();
        let mut new_recs = Vec::new();
        let mut result = Vec::new();

        for (candidate, score) in selected {
            let reasons_json = serde_json::to_string(&score.reasons).unwrap_or_else(|_| "[]".to_string());

            new_recs.push(NewRecommendation {
                id: uuid::Uuid::new_v4().to_string(),
                track_id: Some(candidate.id.clone()),
                external_track_id: None,
                score: score.final_score,
                reasons_json,
                is_discovery: score.is_discovery,
            });

            result.push(RecommendedTrackDetail {
                track_id: candidate.id,
                title: candidate.title,
                artist_name: candidate.artist_name,
                album_title: candidate.album_title,
                genre_name: candidate.genre_name,
                duration_secs: candidate.duration_secs,
                score: score.final_score,
                reasons: score.reasons,
                is_discovery: score.is_discovery,
            });
        }

        // Record session
        let _ = self.rec_repo.record_session(user_id, &session_id, "local_mix", &new_recs).await;

        Ok(result)
    }
}
