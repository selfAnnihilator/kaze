use crate::core::error::{AppError, AppResult};
use crate::database::models::PlaybackHistoryRecord;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct HistoryDetail {
    pub id: String,
    pub track_id: String,
    pub title: String,
    pub artist_name: Option<String>,
    pub album_title: Option<String>,
    pub started_at: i64,
    pub ended_at: i64,
    pub seconds_listened: f64,
    pub percentage_listened: f64,
    pub completed: i64,
    pub skipped: i64,
    pub source: String,
}

#[async_trait]
pub trait HistoryRepository: Send + Sync {
    async fn record_playback(&self, entry: &PlaybackHistoryRecord) -> AppResult<()>;
    async fn get_recent_history(&self, limit: u32) -> AppResult<Vec<HistoryDetail>>;
    async fn get_track_history_count(&self, track_id: &str) -> AppResult<i64>;
}

#[derive(Clone)]
pub struct SqliteHistoryRepository {
    pool: SqlitePool,
}

impl SqliteHistoryRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl HistoryRepository for SqliteHistoryRepository {
    async fn record_playback(&self, entry: &PlaybackHistoryRecord) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO playback_history (
                id, track_id, started_at, ended_at, seconds_listened,
                percentage_listened, completed, skipped, source, playlist_id,
                recommendation_session_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&entry.id)
        .bind(&entry.track_id)
        .bind(entry.started_at)
        .bind(entry.ended_at)
        .bind(entry.seconds_listened)
        .bind(entry.percentage_listened)
        .bind(entry.completed)
        .bind(entry.skipped)
        .bind(&entry.source)
        .bind(&entry.playlist_id)
        .bind(&entry.recommendation_session_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to record playback history: {}", e)))?;

        Ok(())
    }

    async fn get_recent_history(&self, limit: u32) -> AppResult<Vec<HistoryDetail>> {
        let records = sqlx::query_as::<_, HistoryDetail>(
            "SELECT h.id, h.track_id, t.title, a.name as artist_name, al.title as album_title,
                    h.started_at, h.ended_at, h.seconds_listened, h.percentage_listened,
                    h.completed, h.skipped, h.source
             FROM playback_history h
             JOIN tracks t ON h.track_id = t.id
             LEFT JOIN artists a ON t.artist_id = a.id
             LEFT JOIN albums al ON t.album_id = al.id
             ORDER BY h.started_at DESC
             LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to fetch history: {}", e)))?;

        Ok(records)
    }

    async fn get_track_history_count(&self, track_id: &str) -> AppResult<i64> {
        let row: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM playback_history WHERE track_id = ?"
        )
        .bind(track_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(row.0)
    }
}
