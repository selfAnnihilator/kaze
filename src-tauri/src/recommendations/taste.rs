use crate::core::error::{AppError, AppResult};
use crate::database::repositories::{RecommendationRepository, SqliteRecommendationRepository};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::collections::HashMap;

/// Represents the user's computed musical affinities across artists, genres, and eras.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TasteProfile {
    /// Artist ID -> Blended affinity [0.0, 1.0]
    pub artist_affinities: HashMap<String, f64>,
    /// Genre name -> Blended affinity [0.0, 1.0]
    pub genre_affinities: HashMap<String, f64>,
    /// Era (e.g., "1970s", "1980s", "1990s", "2000s", "2010s", "2020s") -> Affinity [0.0, 1.0]
    pub era_affinities: HashMap<String, f64>,
    /// Top artists with their names and affinities
    pub top_artists: Vec<EntityAffinity>,
    /// Top genres with their names and affinities
    pub top_genres: Vec<EntityAffinity>,
    /// Total sessions analyzed
    pub total_sessions_analyzed: usize,
    /// Timestamp when profile was computed
    pub computed_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityAffinity {
    pub id_or_name: String,
    pub display_name: String,
    pub affinity: f64,
}

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct HistorySessionRow {
    track_id: String,
    artist_id: Option<String>,
    artist_name: Option<String>,
    genre_name: Option<String>,
    year: Option<i64>,
    started_at: i64,
    seconds_listened: f64,
    percentage_listened: f64,
    completed: i64,
    skipped: i64,
}

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct FeedbackRow {
    track_id: String,
    artist_id: Option<String>,
    artist_name: Option<String>,
    genre_name: Option<String>,
    manual_like: i64,
}

pub struct TasteProfileEngine {
    pool: SqlitePool,
    rec_repo: SqliteRecommendationRepository,
}

impl TasteProfileEngine {
    pub fn new(pool: SqlitePool) -> Self {
        let rec_repo = SqliteRecommendationRepository::new(pool.clone());
        Self { pool, rec_repo }
    }

    /// Computes the complete dual-window taste profile from playback history, stats, and feedback.
    pub async fn compute_taste_profile(&self) -> AppResult<TasteProfile> {
        let now = Utc::now().timestamp();
        let short_term_cutoff = now - (14 * 86400); // 14 days
        let long_term_cutoff = now - (90 * 86400);  // 90 days

        // Query listening history joined with track, artist, and genre
        let rows = sqlx::query_as::<_, HistorySessionRow>(
            "SELECT h.track_id, t.artist_id, a.name as artist_name, g.name as genre_name,
                    t.year, h.started_at, h.seconds_listened, h.percentage_listened,
                    h.completed, h.skipped
             FROM playback_history h
             JOIN tracks t ON t.id = h.track_id
             LEFT JOIN artists a ON a.id = t.artist_id
             LEFT JOIN genres g ON g.id = t.genre_id
             WHERE h.started_at >= ?
             ORDER BY h.started_at ASC"
        )
        .bind(long_term_cutoff)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Query user explicit feedback (likes / dislikes) from track_statistics
        let feedback_rows = sqlx::query_as::<_, FeedbackRow>(
            "SELECT ts.track_id, t.artist_id, a.name as artist_name, g.name as genre_name, ts.manual_like
             FROM track_statistics ts
             JOIN tracks t ON t.id = ts.track_id
             LEFT JOIN artists a ON a.id = t.artist_id
             LEFT JOIN genres g ON g.id = t.genre_id
             WHERE ts.manual_like != 0"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let mut artist_short_raw: HashMap<String, f64> = HashMap::new();
        let mut artist_long_raw: HashMap<String, f64> = HashMap::new();
        let mut artist_names: HashMap<String, String> = HashMap::new();

        let mut genre_short_raw: HashMap<String, f64> = HashMap::new();
        let mut genre_long_raw: HashMap<String, f64> = HashMap::new();

        let mut era_raw: HashMap<String, f64> = HashMap::new();

        for row in &rows {
            // Base session score: completion contributes heavily, skips penalize
            let session_weight = if row.completed == 1 {
                1.0
            } else if row.skipped == 1 {
                0.2
            } else {
                (row.percentage_listened / 100.0).clamp(0.1, 1.0)
            };

            // Half-life time decay for recency
            let age_days = ((now - row.started_at) as f64) / 86400.0;
            let decay = (-0.05 * age_days).exp(); // 14-day half life approx
            let decayed_score = session_weight * decay;

            // Artist aggregation
            if let Some(artist_id) = &row.artist_id {
                if let Some(name) = &row.artist_name {
                    artist_names.insert(artist_id.clone(), name.clone());
                }
                *artist_long_raw.entry(artist_id.clone()).or_insert(0.0) += decayed_score;
                if row.started_at >= short_term_cutoff {
                    *artist_short_raw.entry(artist_id.clone()).or_insert(0.0) += decayed_score;
                }
            }

            // Genre aggregation
            if let Some(genre) = &row.genre_name {
                let clean_genre = genre.trim().to_string();
                if !clean_genre.is_empty() {
                    *genre_long_raw.entry(clean_genre.clone()).or_insert(0.0) += decayed_score;
                    if row.started_at >= short_term_cutoff {
                        *genre_short_raw.entry(clean_genre.clone()).or_insert(0.0) += decayed_score;
                    }
                }
            }

            // Era aggregation
            if let Some(year) = row.year {
                if year > 1900 && year <= 2030 {
                    let decade = format!("{}0s", (year / 10));
                    *era_raw.entry(decade).or_insert(0.0) += decayed_score;
                }
            }
        }

        // Incorporate explicit feedback
        for fb in feedback_rows {
            let bonus = if fb.manual_like > 0 { 2.0 } else { -2.0 };
            if let Some(artist_id) = fb.artist_id {
                if let Some(name) = fb.artist_name {
                    artist_names.insert(artist_id.clone(), name);
                }
                *artist_short_raw.entry(artist_id.clone()).or_insert(0.0) += bonus;
                *artist_long_raw.entry(artist_id).or_insert(0.0) += bonus;
            }
            if let Some(genre) = fb.genre_name {
                let clean_genre = genre.trim().to_string();
                if !clean_genre.is_empty() {
                    *genre_short_raw.entry(clean_genre.clone()).or_insert(0.0) += bonus;
                    *genre_long_raw.entry(clean_genre).or_insert(0.0) += bonus;
                }
            }
        }

        // Normalize raw scores into [0.0, 1.0] and blend short-term (60%) + long-term (40%)
        let max_artist_short = artist_short_raw.values().cloned().fold(0.0, f64::max).max(1.0);
        let max_artist_long = artist_long_raw.values().cloned().fold(0.0, f64::max).max(1.0);

        let mut all_artist_keys = artist_long_raw.keys().cloned().collect::<Vec<_>>();
        for k in artist_short_raw.keys() {
            if !all_artist_keys.contains(k) {
                all_artist_keys.push(k.clone());
            }
        }

        let mut artist_affinities = HashMap::new();
        let mut top_artists = Vec::new();

        for artist_id in all_artist_keys {
            let short_norm = (artist_short_raw.get(&artist_id).copied().unwrap_or(0.0) / max_artist_short).clamp(0.0, 1.0);
            let long_norm = (artist_long_raw.get(&artist_id).copied().unwrap_or(0.0) / max_artist_long).clamp(0.0, 1.0);
            let blended = (short_norm * 0.6 + long_norm * 0.4).clamp(0.0, 1.0);
            artist_affinities.insert(artist_id.clone(), blended);

            let display_name = artist_names.get(&artist_id).cloned().unwrap_or_else(|| "Unknown Artist".to_string());
            top_artists.push(EntityAffinity {
                id_or_name: artist_id.clone(),
                display_name,
                affinity: blended,
            });

            // Persist to user_preferences table
            let _ = self.rec_repo.upsert_preference("artist", &artist_id, short_norm, long_norm).await;
        }

        top_artists.sort_by(|a, b| b.affinity.partial_cmp(&a.affinity).unwrap_or(std::cmp::Ordering::Equal));

        // Normalize genre scores
        let max_genre_short = genre_short_raw.values().cloned().fold(0.0, f64::max).max(1.0);
        let max_genre_long = genre_long_raw.values().cloned().fold(0.0, f64::max).max(1.0);

        let mut all_genre_keys = genre_long_raw.keys().cloned().collect::<Vec<_>>();
        for k in genre_short_raw.keys() {
            if !all_genre_keys.contains(k) {
                all_genre_keys.push(k.clone());
            }
        }

        let mut genre_affinities = HashMap::new();
        let mut top_genres = Vec::new();

        for genre in all_genre_keys {
            let short_norm = (genre_short_raw.get(&genre).copied().unwrap_or(0.0) / max_genre_short).clamp(0.0, 1.0);
            let long_norm = (genre_long_raw.get(&genre).copied().unwrap_or(0.0) / max_genre_long).clamp(0.0, 1.0);
            let blended = (short_norm * 0.6 + long_norm * 0.4).clamp(0.0, 1.0);
            genre_affinities.insert(genre.clone(), blended);

            top_genres.push(EntityAffinity {
                id_or_name: genre.clone(),
                display_name: genre.clone(),
                affinity: blended,
            });

            // Persist to user_preferences table
            let _ = self.rec_repo.upsert_preference("genre", &genre, short_norm, long_norm).await;
        }

        top_genres.sort_by(|a, b| b.affinity.partial_cmp(&a.affinity).unwrap_or(std::cmp::Ordering::Equal));

        // Normalize eras
        let max_era = era_raw.values().cloned().fold(0.0, f64::max).max(1.0);
        let mut era_affinities = HashMap::new();
        for (decade, score) in era_raw {
            let norm = (score / max_era).clamp(0.0, 1.0);
            era_affinities.insert(decade.clone(), norm);
            let _ = self.rec_repo.upsert_preference("era", &decade, norm, norm).await;
        }

        Ok(TasteProfile {
            artist_affinities,
            genre_affinities,
            era_affinities,
            top_artists,
            top_genres,
            total_sessions_analyzed: rows.len(),
            computed_at: now,
        })
    }
}
