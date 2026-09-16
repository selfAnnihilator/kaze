use crate::core::error::{AppError, AppResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserPreferenceRecord {
    pub user_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub short_term_affinity: f64,
    pub long_term_affinity: f64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecommendationRecord {
    pub id: String,
    pub session_id: String,
    pub track_id: Option<String>,
    pub external_track_id: Option<String>,
    pub score: f64,
    pub reasons_json: String,
    pub is_discovery: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewRecommendation {
    pub id: String,
    pub track_id: Option<String>,
    pub external_track_id: Option<String>,
    pub score: f64,
    pub reasons_json: String,
    pub is_discovery: bool,
}

#[async_trait]
pub trait RecommendationRepository: Send + Sync {
    async fn record_session(
        &self,
        user_id: &str,
        session_id: &str,
        session_type: &str,
        recommendations: &[NewRecommendation],
    ) -> AppResult<()>;

    async fn get_latest_recommendations(
        &self,
        user_id: &str,
        session_type: &str,
        limit: u32,
    ) -> AppResult<Vec<RecommendationRecord>>;
    async fn upsert_preference(
        &self,
        user_id: &str,
        entity_type: &str,
        entity_id: &str,
        short_term: f64,
        long_term: f64,
    ) -> AppResult<()>;
    async fn get_preferences_by_type(
        &self,
        user_id: &str,
        entity_type: &str,
    ) -> AppResult<Vec<UserPreferenceRecord>>;
}

#[derive(Clone)]
pub struct SqliteRecommendationRepository {
    pool: SqlitePool,
}

impl SqliteRecommendationRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RecommendationRepository for SqliteRecommendationRepository {
    async fn record_session(
        &self,
        user_id: &str,
        session_id: &str,
        session_type: &str,
        recommendations: &[NewRecommendation],
    ) -> AppResult<()> {
        let mut tx = self.pool.begin().await.map_err(|e| AppError::Database(e.to_string()))?;
        let now = chrono::Utc::now().timestamp();

        sqlx::query(
            "INSERT INTO recommendation_sessions (id, generated_at, session_type, user_id)
             VALUES (?, ?, ?, ?)"
        )
        .bind(session_id)
        .bind(now)
        .bind(session_type)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        for rec in recommendations {
            sqlx::query(
                "INSERT INTO recommendations (
                    id, session_id, track_id, external_track_id, score, reasons_json, is_discovery
                ) VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&rec.id)
            .bind(session_id)
            .bind(&rec.track_id)
            .bind(&rec.external_track_id)
            .bind(rec.score)
            .bind(&rec.reasons_json)
            .bind(if rec.is_discovery { 1 } else { 0 })
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        }

        tx.commit().await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    async fn get_latest_recommendations(
        &self,
        user_id: &str,
        session_type: &str,
        limit: u32,
    ) -> AppResult<Vec<RecommendationRecord>> {
        let records = sqlx::query_as::<_, RecommendationRecord>(
            "SELECT r.id, r.session_id, r.track_id, r.external_track_id, r.score, r.reasons_json, r.is_discovery
             FROM recommendations r
             JOIN recommendation_sessions s ON s.id = r.session_id
             WHERE s.session_type = ? AND s.user_id = ?
             ORDER BY s.generated_at DESC, r.score DESC
             LIMIT ?"
        )
        .bind(session_type)
        .bind(user_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(records)
    }

    async fn upsert_preference(
        &self,
        user_id: &str,
        entity_type: &str,
        entity_id: &str,
        short_term: f64,
        long_term: f64,
    ) -> AppResult<()> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO user_preferences (user_id, entity_type, entity_id, short_term_affinity, long_term_affinity, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(user_id, entity_type, entity_id) DO UPDATE SET
                short_term_affinity = excluded.short_term_affinity,
                long_term_affinity = excluded.long_term_affinity,
                updated_at = excluded.updated_at"
        )
        .bind(user_id)
        .bind(entity_type)
        .bind(entity_id)
        .bind(short_term)
        .bind(long_term)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_preferences_by_type(
        &self,
        user_id: &str,
        entity_type: &str,
    ) -> AppResult<Vec<UserPreferenceRecord>> {
        let prefs = sqlx::query_as::<_, UserPreferenceRecord>(
            "SELECT user_id, entity_type, entity_id, short_term_affinity, long_term_affinity, updated_at
             FROM user_preferences
             WHERE user_id = ? AND entity_type = ?
             ORDER BY (short_term_affinity * 0.6 + long_term_affinity * 0.4) DESC"
        )
        .bind(user_id)
        .bind(entity_type)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(prefs)
    }
}
