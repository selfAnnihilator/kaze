use crate::config::RankingWeightsConfig;
use crate::core::error::{AppError, AppResult};
use crate::database::models::TrackStatisticsRecord;
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RankedTrackItem {
    pub track_id: String,
    pub title: String,
    pub artist_name: Option<String>,
    pub album_title: Option<String>,
    pub play_count: i64,
    pub total_seconds: f64,
    pub completion_count: i64,
    pub skip_count: i64,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RankedArtistItem {
    pub artist_id: String,
    pub name: String,
    pub play_count: i64,
    pub total_seconds: f64,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RankedAlbumItem {
    pub album_id: String,
    pub title: String,
    pub artist_name: Option<String>,
    pub play_count: i64,
    pub total_seconds: f64,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RankedGenreItem {
    pub genre_id: String,
    pub name: String,
    pub play_count: i64,
    pub total_seconds: f64,
    pub score: f64,
}

#[async_trait]
pub trait StatsRepository: Send + Sync {
    async fn update_track_playback_stats(
        &self,
        track_id: &str,
        listened_secs: f64,
        is_meaningful: bool,
        completed: bool,
        skipped: bool,
    ) -> AppResult<()>;

    async fn set_track_like(&self, track_id: &str, like_status: i64) -> AppResult<()>;
    async fn get_track_stats(&self, track_id: &str) -> AppResult<Option<TrackStatisticsRecord>>;

    async fn get_ranked_tracks(
        &self,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedTrackItem>>;

    async fn get_ranked_artists(
        &self,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedArtistItem>>;

    async fn get_ranked_albums(
        &self,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedAlbumItem>>;

    async fn get_ranked_genres(
        &self,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedGenreItem>>;
}

#[derive(Clone)]
pub struct SqliteStatsRepository {
    pool: SqlitePool,
}

impl SqliteStatsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StatsRepository for SqliteStatsRepository {
    async fn update_track_playback_stats(
        &self,
        track_id: &str,
        listened_secs: f64,
        is_meaningful: bool,
        completed: bool,
        skipped: bool,
    ) -> AppResult<()> {
        let now = Utc::now().timestamp();
        let play_inc: i64 = if is_meaningful { 1 } else { 0 };
        let comp_inc: i64 = if completed { 1 } else { 0 };
        let skip_inc: i64 = if skipped { 1 } else { 0 };

        // 1. Update track statistics
        sqlx::query(
            "INSERT INTO track_statistics (
                track_id, play_count, total_time_listened, completion_count, skip_count, last_played_at
            ) VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(track_id) DO UPDATE SET
                play_count = play_count + excluded.play_count,
                total_time_listened = total_time_listened + excluded.total_time_listened,
                completion_count = completion_count + excluded.completion_count,
                skip_count = skip_count + excluded.skip_count,
                last_played_at = excluded.last_played_at"
        )
        .bind(track_id)
        .bind(play_inc)
        .bind(listened_secs)
        .bind(comp_inc)
        .bind(skip_inc)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to update track statistics: {}", e)))?;

        // 2. Update artist & genre aggregated stats if track has artist_id / genre_id
        if is_meaningful {
            let track_info: Option<(Option<String>, Option<String>)> = sqlx::query_as(
                "SELECT artist_id, genre_id FROM tracks WHERE id = ?"
            )
            .bind(track_id)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten();

            if let Some((Some(artist_id), genre_id)) = track_info {
                let _ = sqlx::query(
                    "INSERT INTO artist_statistics (artist_id, play_count, total_time_listened, last_played_at, affinity_score)
                     VALUES (?, 1, ?, ?, 1.0)
                     ON CONFLICT(artist_id) DO UPDATE SET
                        play_count = play_count + 1,
                        total_time_listened = total_time_listened + excluded.total_time_listened,
                        last_played_at = excluded.last_played_at,
                        affinity_score = affinity_score + 1.0"
                )
                .bind(&artist_id)
                .bind(listened_secs)
                .bind(now)
                .execute(&self.pool)
                .await;

                if let Some(gid) = genre_id {
                    let _ = sqlx::query(
                        "INSERT INTO genre_statistics (genre_id, play_count, total_time_listened, last_played_at, affinity_score)
                         VALUES (?, 1, ?, ?, 1.0)
                         ON CONFLICT(genre_id) DO UPDATE SET
                            play_count = play_count + 1,
                            total_time_listened = total_time_listened + excluded.total_time_listened,
                            last_played_at = excluded.last_played_at,
                            affinity_score = affinity_score + 1.0"
                    )
                    .bind(&gid)
                    .bind(listened_secs)
                    .bind(now)
                    .execute(&self.pool)
                    .await;
                }
            }
        }

        Ok(())
    }

    async fn set_track_like(&self, track_id: &str, like_status: i64) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO track_statistics (track_id, manual_like) VALUES (?, ?)
             ON CONFLICT(track_id) DO UPDATE SET manual_like = excluded.manual_like"
        )
        .bind(track_id)
        .bind(like_status)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to set track like: {}", e)))?;

        Ok(())
    }

    async fn get_track_stats(&self, track_id: &str) -> AppResult<Option<TrackStatisticsRecord>> {
        let stats = sqlx::query_as::<_, TrackStatisticsRecord>(
            "SELECT track_id, play_count, total_time_listened, completion_count, skip_count,
                    last_played_at, manual_like, playlist_addition_count
             FROM track_statistics WHERE track_id = ?"
        )
        .bind(track_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(stats)
    }

    async fn get_ranked_tracks(
        &self,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedTrackItem>> {
        let start_filter = window_start.unwrap_or(0);

        // Compute aggregated metrics from playback_history within the window, joined with track metadata
        let sql = format!("
            SELECT
                t.id as track_id,
                t.title as title,
                a.name as artist_name,
                al.title as album_title,
                COALESCE(COUNT(CASE WHEN h.completed = 1 OR h.seconds_listened >= 30.0 THEN 1 END), 0) as play_count,
                COALESCE(SUM(h.seconds_listened), 0.0) as total_seconds,
                COALESCE(SUM(h.completed), 0) as completion_count,
                COALESCE(SUM(h.skipped), 0) as skip_count,
                (
                    (? * COALESCE(COUNT(CASE WHEN h.completed = 1 OR h.seconds_listened >= 30.0 THEN 1 END), 0)) +
                    (? * (COALESCE(SUM(h.seconds_listened), 0.0) / 60.0)) +
                    (? * COALESCE(SUM(h.completed), 0)) +
                    (? * COALESCE(ts.manual_like, 0)) -
                    (? * COALESCE(SUM(h.skipped), 0))
                ) as score
            FROM tracks t
            LEFT JOIN artists a ON t.artist_id = a.id
            LEFT JOIN albums al ON t.album_id = al.id
            LEFT JOIN track_statistics ts ON ts.track_id = t.id
            LEFT JOIN playback_history h ON h.track_id = t.id AND h.started_at >= ?
            GROUP BY t.id
            HAVING score > 0 OR play_count > 0
            ORDER BY score DESC, play_count DESC
            LIMIT ?
        ");

        let records = sqlx::query_as::<_, RankedTrackItem>(&sql)
            .bind(weights.play_count_weight)
            .bind(weights.listening_duration_weight)
            .bind(weights.completion_weight)
            .bind(weights.user_preference_weight)
            .bind(weights.skip_penalty)
            .bind(start_filter)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Ranked tracks query failed: {}", e)))?;

        Ok(records)
    }

    async fn get_ranked_artists(
        &self,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedArtistItem>> {
        let start_filter = window_start.unwrap_or(0);

        let sql = format!("
            SELECT
                a.id as artist_id,
                a.name as name,
                COALESCE(COUNT(CASE WHEN h.completed = 1 OR h.seconds_listened >= 30.0 THEN 1 END), 0) as play_count,
                COALESCE(SUM(h.seconds_listened), 0.0) as total_seconds,
                (
                    (? * COALESCE(COUNT(CASE WHEN h.completed = 1 OR h.seconds_listened >= 30.0 THEN 1 END), 0)) +
                    (? * (COALESCE(SUM(h.seconds_listened), 0.0) / 60.0)) -
                    (? * COALESCE(SUM(h.skipped), 0))
                ) as score
            FROM artists a
            JOIN tracks t ON t.artist_id = a.id
            JOIN playback_history h ON h.track_id = t.id AND h.started_at >= ?
            GROUP BY a.id
            ORDER BY score DESC, play_count DESC
            LIMIT ?
        ");

        let records = sqlx::query_as::<_, RankedArtistItem>(&sql)
            .bind(weights.play_count_weight)
            .bind(weights.listening_duration_weight)
            .bind(weights.skip_penalty)
            .bind(start_filter)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Ranked artists query failed: {}", e)))?;

        Ok(records)
    }

    async fn get_ranked_albums(
        &self,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedAlbumItem>> {
        let start_filter = window_start.unwrap_or(0);

        let sql = format!("
            SELECT
                al.id as album_id,
                al.title as title,
                a.name as artist_name,
                COALESCE(COUNT(CASE WHEN h.completed = 1 OR h.seconds_listened >= 30.0 THEN 1 END), 0) as play_count,
                COALESCE(SUM(h.seconds_listened), 0.0) as total_seconds,
                (
                    (? * COALESCE(COUNT(CASE WHEN h.completed = 1 OR h.seconds_listened >= 30.0 THEN 1 END), 0)) +
                    (? * (COALESCE(SUM(h.seconds_listened), 0.0) / 60.0)) -
                    (? * COALESCE(SUM(h.skipped), 0))
                ) as score
            FROM albums al
            LEFT JOIN artists a ON al.artist_id = a.id
            JOIN tracks t ON t.album_id = al.id
            JOIN playback_history h ON h.track_id = t.id AND h.started_at >= ?
            GROUP BY al.id
            ORDER BY score DESC, play_count DESC
            LIMIT ?
        ");

        let records = sqlx::query_as::<_, RankedAlbumItem>(&sql)
            .bind(weights.play_count_weight)
            .bind(weights.listening_duration_weight)
            .bind(weights.skip_penalty)
            .bind(start_filter)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Ranked albums query failed: {}", e)))?;

        Ok(records)
    }

    async fn get_ranked_genres(
        &self,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedGenreItem>> {
        let start_filter = window_start.unwrap_or(0);

        let sql = format!("
            SELECT
                g.id as genre_id,
                g.name as name,
                COALESCE(COUNT(CASE WHEN h.completed = 1 OR h.seconds_listened >= 30.0 THEN 1 END), 0) as play_count,
                COALESCE(SUM(h.seconds_listened), 0.0) as total_seconds,
                (
                    (? * COALESCE(COUNT(CASE WHEN h.completed = 1 OR h.seconds_listened >= 30.0 THEN 1 END), 0)) +
                    (? * (COALESCE(SUM(h.seconds_listened), 0.0) / 60.0)) -
                    (? * COALESCE(SUM(h.skipped), 0))
                ) as score
            FROM genres g
            JOIN tracks t ON t.genre_id = g.id
            JOIN playback_history h ON h.track_id = t.id AND h.started_at >= ?
            GROUP BY g.id
            ORDER BY score DESC, play_count DESC
            LIMIT ?
        ");

        let records = sqlx::query_as::<_, RankedGenreItem>(&sql)
            .bind(weights.play_count_weight)
            .bind(weights.listening_duration_weight)
            .bind(weights.skip_penalty)
            .bind(start_filter)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Ranked genres query failed: {}", e)))?;

        Ok(records)
    }
}
