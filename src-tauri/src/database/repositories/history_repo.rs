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
    async fn record_session(&self, entry: &PlaybackHistoryRecord, stat_date: &str, meaningful: bool) -> AppResult<()>;
    async fn get_recent_history(&self, limit: u32, user_id: &str) -> AppResult<Vec<HistoryDetail>>;
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
                id, user_id, track_id, started_at, ended_at, seconds_listened,
                percentage_listened, completed, skipped, source, playlist_id,
                recommendation_session_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&entry.id)
        .bind(&entry.user_id)
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

    async fn record_session(&self, entry: &PlaybackHistoryRecord, stat_date: &str, meaningful: bool) -> AppResult<()> {
        let device = crate::cloud::device::get_or_create_device_id(&self.pool).await?;
        let mut tx = self.pool.begin().await.map_err(|e| AppError::Database(e.to_string()))?;
        let inserted = sqlx::query(
            "INSERT INTO playback_history (
                id, user_id, track_id, started_at, ended_at, seconds_listened,
                percentage_listened, completed, skipped, source, playlist_id,
                recommendation_session_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(id) DO NOTHING"
        )
        .bind(&entry.id)
        .bind(&entry.user_id)
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
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::Database(format!("Failed to record playback history: {}", e)))?;

        if inserted.rows_affected() == 0 {
            return Ok(());
        }
        sqlx::query("INSERT INTO track_statistics
            (user_id, track_id, play_count, total_time_listened, completion_count, skip_count, last_played_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(user_id, track_id) DO UPDATE SET
              play_count = play_count + excluded.play_count,
              total_time_listened = total_time_listened + excluded.total_time_listened,
              completion_count = completion_count + excluded.completion_count,
              skip_count = skip_count + excluded.skip_count,
              last_played_at = MAX(COALESCE(last_played_at, 0), excluded.last_played_at)")
            .bind(&entry.user_id).bind(&entry.track_id).bind(i64::from(meaningful))
            .bind(entry.seconds_listened).bind(entry.completed).bind(entry.skipped).bind(entry.ended_at)
            .execute(&mut *tx).await.map_err(|e| AppError::Database(e.to_string()))?;

        // Also upsert into track_device_statistics for per-device multi-device lifetime accounting.
        sqlx::query("INSERT INTO track_device_statistics
            (user_id, device_id, track_id, play_count, total_time_listened, completion_count, skip_count, last_played_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(user_id, device_id, track_id) DO UPDATE SET
              play_count          = play_count + excluded.play_count,
              total_time_listened = total_time_listened + excluded.total_time_listened,
              completion_count    = completion_count + excluded.completion_count,
              skip_count          = skip_count + excluded.skip_count,
              last_played_at      = MAX(COALESCE(last_played_at, 0), excluded.last_played_at),
              updated_at          = excluded.updated_at")
            .bind(&entry.user_id).bind(&device).bind(&entry.track_id)
            .bind(i64::from(meaningful)).bind(entry.seconds_listened)
            .bind(entry.completed).bind(entry.skipped)
            .bind(entry.ended_at).bind(entry.ended_at)
            .execute(&mut *tx).await.map_err(|e| AppError::Database(e.to_string()))?;

        sqlx::query("INSERT INTO daily_user_stats
            (user_id, device_id, stat_date, listening_seconds, play_count, completion_count, skip_count, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(user_id, device_id, stat_date) DO UPDATE SET
              listening_seconds = listening_seconds + excluded.listening_seconds,
              play_count = play_count + excluded.play_count,
              completion_count = completion_count + excluded.completion_count,
              skip_count = skip_count + excluded.skip_count,
              updated_at = MAX(updated_at, excluded.updated_at)")
            .bind(&entry.user_id).bind(&device).bind(stat_date).bind(entry.seconds_listened)
            .bind(i64::from(meaningful)).bind(entry.completed).bind(entry.skipped).bind(entry.ended_at)
            .execute(&mut *tx).await.map_err(|e| AppError::Database(e.to_string()))?;
        if meaningful {
            let metadata: Option<(Option<String>, Option<String>)> = sqlx::query_as(
                "SELECT artist_id, genre_id FROM tracks WHERE id = ?")
                .bind(&entry.track_id).fetch_optional(&mut *tx).await
                .map_err(|e| AppError::Database(e.to_string()))?;
            if let Some((Some(artist), genre)) = metadata {
                for (table, column, id) in [("artist_statistics", "artist_id", Some(artist)),
                                            ("genre_statistics", "genre_id", genre)] {
                    if let Some(id) = id {
                        let sql = format!("INSERT INTO {table}
                            (user_id, {column}, play_count, total_time_listened, last_played_at, affinity_score)
                            VALUES (?, ?, 1, ?, ?, 1.0)
                            ON CONFLICT(user_id, {column}) DO UPDATE SET
                              play_count = play_count + 1,
                              total_time_listened = total_time_listened + excluded.total_time_listened,
                              last_played_at = excluded.last_played_at,
                              affinity_score = affinity_score + 1.0");
                        sqlx::query(&sql).bind(&entry.user_id).bind(id).bind(entry.seconds_listened)
                            .bind(entry.ended_at).execute(&mut *tx).await
                            .map_err(|e| AppError::Database(e.to_string()))?;
                    }
                }
            }
        }
        tx.commit().await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    async fn get_recent_history(&self, limit: u32, user_id: &str) -> AppResult<Vec<HistoryDetail>> {
        let records = sqlx::query_as::<_, HistoryDetail>(
            "SELECT h.id, h.track_id, t.title, a.name as artist_name, al.title as album_title,
                    h.started_at, h.ended_at, h.seconds_listened, h.percentage_listened,
                    h.completed, h.skipped, h.source
             FROM playback_history h
             JOIN tracks t ON h.track_id = t.id
             LEFT JOIN artists a ON t.artist_id = a.id
             LEFT JOIN albums al ON t.album_id = al.id
             WHERE h.user_id = ?
             ORDER BY h.started_at DESC
             LIMIT ?"
        )
        .bind(user_id)
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
