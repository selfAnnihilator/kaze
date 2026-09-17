use crate::config::RankingWeightsConfig;
use crate::core::error::{AppError, AppResult};
use crate::database::models::TrackStatisticsRecord;
use async_trait::async_trait;
use chrono::{Datelike, Duration as ChronoDuration, Utc};
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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TopListeningDay {
    pub day_date: String,
    pub day_name: String,
    pub total_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyListeningPoint {
    pub day: u32,
    pub date: String,
    pub total_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsOverview {
    pub app_start_date: i64,
    pub user_joined_date: i64,
    pub current_year: i32,
    pub selected_year: i32,
    pub available_years: Vec<i32>,
    pub selected_month: u32,
    pub available_months: Vec<u32>,
    pub daily_seconds: f64,
    pub weekly_seconds: f64,
    pub monthly_seconds: f64,
    pub total_year_seconds: f64,
    #[serde(default)]
    pub lifetime_seconds: f64,
    #[serde(default)]
    pub history_started_at: Option<i64>,
    pub monthly_graph: Vec<DailyListeningPoint>,
    pub top_songs: Vec<RankedTrackItem>,
    pub top_artists: Vec<RankedArtistItem>,
    pub top_days: Vec<TopListeningDay>,
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

    async fn update_track_playback_stats_scoped(
        &self,
        user_id: &str,
        track_id: &str,
        listened_secs: f64,
        is_meaningful: bool,
        completed: bool,
        skipped: bool,
    ) -> AppResult<()>;

    async fn set_track_like(&self, track_id: &str, like_status: i64) -> AppResult<()>;
    async fn set_track_like_scoped(&self, user_id: &str, track_id: &str, like_status: i64) -> AppResult<()>;
    async fn get_track_stats(&self, track_id: &str) -> AppResult<Option<TrackStatisticsRecord>>;
    async fn get_track_stats_scoped(&self, user_id: &str, track_id: &str) -> AppResult<Option<TrackStatisticsRecord>>;

    async fn get_ranked_tracks(
        &self,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedTrackItem>>;

    async fn get_ranked_tracks_for_user(
        &self,
        user_id: Option<&str>,
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

    async fn get_ranked_artists_for_user(
        &self,
        user_id: Option<&str>,
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

    async fn get_ranked_albums_for_user(
        &self,
        user_id: Option<&str>,
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

    async fn get_ranked_genres_for_user(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedGenreItem>>;

    async fn get_stats_overview(
        &self,
        user_id: &str,
        user_joined_at: i64,
        app_start_date: i64,
        selected_year: Option<i32>,
        selected_month: Option<u32>,
        weights: &RankingWeightsConfig,
    ) -> AppResult<StatsOverview>;

    async fn get_top_listening_days(
        &self,
        user_id: &str,
        limit: u32,
    ) -> AppResult<Vec<TopListeningDay>>;

    async fn get_ranked_tracks_scoped(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        window_end: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedTrackItem>>;

    async fn get_ranked_artists_scoped(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        window_end: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedArtistItem>>;

    async fn get_all_time_ranked_tracks(
        &self,
        user_id: &str,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedTrackItem>>;

    async fn get_all_time_ranked_artists(
        &self,
        user_id: &str,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedArtistItem>>;

    async fn record_daily_playback(
        &self,
        user_id: &str,
        device_id: Option<&str>,
        stat_date: &str,
        seconds: f64,
        is_meaningful: bool,
        completed: bool,
        skipped: bool,
    ) -> AppResult<()>;

    async fn backfill_daily_stats_from_history(&self, default_device_id: Option<&str>) -> AppResult<usize>;
}

#[derive(Clone)]
pub struct SqliteStatsRepository {
    pool: SqlitePool,
}

impl SqliteStatsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn resolve_device_id(&self, explicit_id: Option<&str>) -> String {
        if let Some(id) = explicit_id {
            if !id.trim().is_empty() {
                return id.to_string();
            }
        }
        crate::cloud::device::get_or_create_device_id(&self.pool)
            .await
            .unwrap_or_else(|_| "legacy_device".to_string())
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
        self.update_track_playback_stats_scoped("default", track_id, listened_secs, is_meaningful, completed, skipped).await
    }

    async fn update_track_playback_stats_scoped(
        &self,
        user_id: &str,
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

        // 1. Update track statistics for this user
        sqlx::query(
            "INSERT INTO track_statistics (
                user_id, track_id, play_count, total_time_listened, completion_count, skip_count, last_played_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(user_id, track_id) DO UPDATE SET
                play_count = play_count + excluded.play_count,
                total_time_listened = total_time_listened + excluded.total_time_listened,
                completion_count = completion_count + excluded.completion_count,
                skip_count = skip_count + excluded.skip_count,
                last_played_at = excluded.last_played_at"
        )
        .bind(user_id)
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
                    "INSERT INTO artist_statistics (user_id, artist_id, play_count, total_time_listened, last_played_at, affinity_score)
                     VALUES (?, ?, 1, ?, ?, 1.0)
                     ON CONFLICT(user_id, artist_id) DO UPDATE SET
                        play_count = play_count + 1,
                        total_time_listened = total_time_listened + excluded.total_time_listened,
                        last_played_at = excluded.last_played_at,
                        affinity_score = affinity_score + 1.0"
                )
                .bind(user_id)
                .bind(&artist_id)
                .bind(listened_secs)
                .bind(now)
                .execute(&self.pool)
                .await;

                if let Some(gid) = genre_id {
                    let _ = sqlx::query(
                        "INSERT INTO genre_statistics (user_id, genre_id, play_count, total_time_listened, last_played_at, affinity_score)
                         VALUES (?, ?, 1, ?, ?, 1.0)
                         ON CONFLICT(user_id, genre_id) DO UPDATE SET
                            play_count = play_count + 1,
                            total_time_listened = total_time_listened + excluded.total_time_listened,
                            last_played_at = excluded.last_played_at,
                            affinity_score = affinity_score + 1.0"
                    )
                    .bind(user_id)
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

    async fn record_daily_playback(
        &self,
        user_id: &str,
        device_id: Option<&str>,
        stat_date: &str,
        seconds: f64,
        is_meaningful: bool,
        completed: bool,
        skipped: bool,
    ) -> AppResult<()> {
        let dev_id = self.resolve_device_id(device_id).await;
        let now = Utc::now().timestamp();
        let play_inc: i64 = if is_meaningful { 1 } else { 0 };
        let comp_inc: i64 = if completed { 1 } else { 0 };
        let skip_inc: i64 = if skipped { 1 } else { 0 };

        sqlx::query(
            "INSERT INTO daily_user_stats (
                user_id, device_id, stat_date, listening_seconds, play_count, completion_count, skip_count, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(user_id, device_id, stat_date) DO UPDATE SET
                 listening_seconds = daily_user_stats.listening_seconds + excluded.listening_seconds,
                 play_count = daily_user_stats.play_count + excluded.play_count,
                 completion_count = daily_user_stats.completion_count + excluded.completion_count,
                 skip_count = daily_user_stats.skip_count + excluded.skip_count,
                 updated_at = excluded.updated_at"
        )
        .bind(user_id)
        .bind(&dev_id)
        .bind(stat_date)
        .bind(seconds)
        .bind(play_inc)
        .bind(comp_inc)
        .bind(skip_inc)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to record daily playback: {}", e)))?;

        Ok(())
    }

    async fn backfill_daily_stats_from_history(&self, default_device_id: Option<&str>) -> AppResult<usize> {
        let dev_id = self.resolve_device_id(default_device_id).await;
        let res = sqlx::query(
            "INSERT OR IGNORE INTO daily_user_stats (
                user_id, device_id, stat_date, listening_seconds, play_count, completion_count, skip_count, updated_at
             )
             SELECT
                 COALESCE(h.user_id, 'default') as user_id,
                 ? as device_id,
                 strftime('%Y-%m-%d', datetime(h.started_at, 'unixepoch', 'localtime')) as stat_date,
                 COALESCE(SUM(h.seconds_listened), 0.0) as listening_seconds,
                 COALESCE(SUM(CASE WHEN h.seconds_listened >= 30.0 OR h.completed = 1 THEN 1 ELSE 0 END), 0) as play_count,
                 COALESCE(SUM(h.completed), 0) as completion_count,
                 COALESCE(SUM(h.skipped), 0) as skip_count,
                 COALESCE(MAX(h.ended_at), MAX(h.started_at), strftime('%s', 'now')) as updated_at
             FROM playback_history h
             GROUP BY user_id, stat_date"
        )
        .bind(&dev_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to backfill daily stats from history: {}", e)))?;

        Ok(res.rows_affected() as usize)
    }

    async fn set_track_like(&self, track_id: &str, like_status: i64) -> AppResult<()> {
        self.set_track_like_scoped("default", track_id, like_status).await
    }

    async fn set_track_like_scoped(&self, user_id: &str, track_id: &str, like_status: i64) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO track_statistics (user_id, track_id, manual_like) VALUES (?, ?, ?)
             ON CONFLICT(user_id, track_id) DO UPDATE SET manual_like = excluded.manual_like"
        )
        .bind(user_id)
        .bind(track_id)
        .bind(like_status)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to set track like: {}", e)))?;

        Ok(())
    }

    async fn get_track_stats(&self, track_id: &str) -> AppResult<Option<TrackStatisticsRecord>> {
        self.get_track_stats_scoped("default", track_id).await
    }

    async fn get_track_stats_scoped(&self, user_id: &str, track_id: &str) -> AppResult<Option<TrackStatisticsRecord>> {
        let stats = sqlx::query_as::<_, TrackStatisticsRecord>(
            "SELECT user_id, track_id, play_count, total_time_listened, completion_count, skip_count,
                    last_played_at, manual_like, playlist_addition_count
             FROM track_statistics WHERE user_id = ? AND track_id = ?"
        )
        .bind(user_id)
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
        self.get_ranked_tracks_for_user(None, window_start, weights, limit).await
    }

    async fn get_ranked_tracks_for_user(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedTrackItem>> {
        let uid = user_id.unwrap_or("default");
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
            LEFT JOIN track_statistics ts ON ts.track_id = t.id AND ts.user_id = ?
            LEFT JOIN playback_history h ON h.track_id = t.id AND h.started_at >= ? AND h.user_id = ?
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
            .bind(uid)
            .bind(start_filter)
            .bind(uid)
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
        self.get_ranked_artists_for_user(None, window_start, weights, limit).await
    }

    async fn get_ranked_artists_for_user(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedArtistItem>> {
        let uid = user_id.unwrap_or("default");
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
            JOIN playback_history h ON h.track_id = t.id AND h.started_at >= ? AND h.user_id = ?
            GROUP BY a.id
            ORDER BY score DESC, play_count DESC
            LIMIT ?
        ");

        let records = sqlx::query_as::<_, RankedArtistItem>(&sql)
            .bind(weights.play_count_weight)
            .bind(weights.listening_duration_weight)
            .bind(weights.skip_penalty)
            .bind(start_filter)
            .bind(uid)
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
        self.get_ranked_albums_for_user(None, window_start, weights, limit).await
    }

    async fn get_ranked_albums_for_user(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedAlbumItem>> {
        let uid = user_id.unwrap_or("default");
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
            JOIN playback_history h ON h.track_id = t.id AND h.started_at >= ? AND h.user_id = ?
            GROUP BY al.id
            ORDER BY score DESC, play_count DESC
            LIMIT ?
        ");

        let records = sqlx::query_as::<_, RankedAlbumItem>(&sql)
            .bind(weights.play_count_weight)
            .bind(weights.listening_duration_weight)
            .bind(weights.skip_penalty)
            .bind(start_filter)
            .bind(uid)
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
        self.get_ranked_genres_for_user(None, window_start, weights, limit).await
    }

    async fn get_ranked_genres_for_user(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedGenreItem>> {
        let uid = user_id.unwrap_or("default");
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
            JOIN playback_history h ON h.track_id = t.id AND h.started_at >= ? AND h.user_id = ?
            GROUP BY g.id
            ORDER BY score DESC, play_count DESC
            LIMIT ?
        ");

        let records = sqlx::query_as::<_, RankedGenreItem>(&sql)
            .bind(weights.play_count_weight)
            .bind(weights.listening_duration_weight)
            .bind(weights.skip_penalty)
            .bind(start_filter)
            .bind(uid)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Ranked genres query failed: {}", e)))?;

        Ok(records)
    }

    async fn get_stats_overview(
        &self,
        user_id: &str,
        user_joined_at: i64,
        app_start_date: i64,
        selected_year: Option<i32>,
        selected_month: Option<u32>,
        weights: &RankingWeightsConfig,
    ) -> AppResult<StatsOverview> {
        let now = chrono::Local::now();
        let current_year = now.date_naive().year();
        let target_year = selected_year.unwrap_or(current_year);
        let target_month = selected_month.unwrap_or_else(|| {
            if target_year == current_year {
                now.month()
            } else {
                12
            }
        });

        // Query lifetime listening seconds from track_statistics (all-time aggregate, includes cloud-restored data)
        let lifetime_seconds: f64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(total_time_listened), 0.0) FROM track_statistics WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0.0);

        // Daily aggregates are the only timeline authority, including restored installs.
        let min_date: Option<String> = sqlx::query_scalar(
            "SELECT MIN(stat_date) FROM daily_user_stats WHERE user_id = ?")
            .bind(user_id).fetch_one(&self.pool).await
            .map_err(|e| AppError::Database(e.to_string()))?;
        let history_started_at = min_date.and_then(|date| {
            use chrono::TimeZone;
            chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok()
                .and_then(|day| day.and_hms_opt(0, 0, 0))
                .and_then(|midnight| chrono::Local.from_local_datetime(&midnight).earliest())
                .map(|dt| dt.timestamp())
        });
        let year_rows: Vec<(Option<i32>,)> = sqlx::query_as(
            "SELECT DISTINCT CAST(substr(stat_date, 1, 4) AS INTEGER) as yr
             FROM daily_user_stats WHERE user_id = ? ORDER BY yr DESC")
            .bind(user_id).fetch_all(&self.pool).await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut available_years: Vec<i32> = year_rows.into_iter().filter_map(|r| r.0).collect();
        if !available_years.contains(&current_year) {
            available_years.push(current_year);
        }
        available_years.sort_by(|a, b| b.cmp(a));

        let available_months: Vec<u32> = (1..=12).collect();

        // Calculate monthly graph and seconds for the selected month
        let days_in_month = match target_month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                let is_leap = (target_year % 4 == 0 && target_year % 100 != 0) || (target_year % 400 == 0);
                if is_leap { 29 } else { 28 }
            }
            _ => 30,
        };

        let start_date_str = format!("{:04}-{:02}-01", target_year, target_month);
        let end_date_str = format!("{:04}-{:02}-{:02}", target_year, target_month, days_in_month);

        let daily_rows: Vec<(String, f64)> = sqlx::query_as(
            "SELECT
                stat_date as day_date,
                COALESCE(SUM(listening_seconds), 0.0) as total_seconds
             FROM daily_user_stats
             WHERE user_id = ? AND stat_date >= ? AND stat_date <= ?
             GROUP BY stat_date"
        )
        .bind(user_id)
        .bind(&start_date_str)
        .bind(&end_date_str)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        let mut day_map = std::collections::HashMap::new();
        for (d, s) in daily_rows {
            day_map.insert(d, s);
        }

        let mut monthly_graph = Vec::with_capacity(days_in_month as usize);
        let mut monthly_seconds = 0.0;
        for day in 1..=days_in_month {
            let date_str = format!("{:04}-{:02}-{:02}", target_year, target_month, day);
            let seconds = day_map.get(&date_str).copied().unwrap_or(0.0);
            monthly_seconds += seconds;
            monthly_graph.push(DailyListeningPoint {
                day,
                date: date_str,
                total_seconds: seconds,
            });
        }

        // 3. Current year stats from daily_user_stats
        let today_str = chrono::Local::now().format("%Y-%m-%d").to_string();
        let week_ago_str = (chrono::Local::now() - ChronoDuration::days(6)).format("%Y-%m-%d").to_string();
        let year_start_str = format!("{:04}-01-01", target_year);
        let year_end_str = format!("{:04}-12-31", target_year);

        let daily_seconds: f64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(listening_seconds), 0.0) FROM daily_user_stats
             WHERE user_id = ? AND stat_date = ?"
        )
        .bind(user_id)
        .bind(&today_str)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0.0);

        let weekly_seconds: f64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(listening_seconds), 0.0) FROM daily_user_stats
             WHERE user_id = ? AND stat_date >= ? AND stat_date <= ?"
        )
        .bind(user_id)
        .bind(&week_ago_str)
        .bind(&today_str)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0.0);

        let total_year_seconds: f64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(listening_seconds), 0.0) FROM daily_user_stats
             WHERE user_id = ? AND stat_date >= ? AND stat_date <= ?"
        )
        .bind(user_id)
        .bind(&year_start_str)
        .bind(&year_end_str)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0.0);

        // For all-time / current year rankings, base on track_statistics to encompass cloud-restored and local playback
        let top_songs = self.get_all_time_ranked_tracks(user_id, weights, 10).await.unwrap_or_default();
        let top_artists = self.get_all_time_ranked_artists(user_id, weights, 10).await.unwrap_or_default();
        let top_days = self.get_top_listening_days(user_id, 5).await.unwrap_or_default();

        Ok(StatsOverview {
            app_start_date,
            user_joined_date: user_joined_at,
            current_year,
            selected_year: target_year,
            available_years,
            selected_month: target_month,
            available_months,
            daily_seconds,
            weekly_seconds,
            monthly_seconds,
            total_year_seconds,
            lifetime_seconds,
            history_started_at,
            monthly_graph,
            top_songs,
            top_artists,
            top_days,
        })
    }

    async fn get_top_listening_days(&self, user_id: &str, limit: u32) -> AppResult<Vec<TopListeningDay>> {
        let rows: Vec<(String, String, f64)> = sqlx::query_as(
            "SELECT
                stat_date as day_date,
                strftime('%w', stat_date) as day_of_week,
                COALESCE(SUM(listening_seconds), 0.0) as total_seconds
             FROM daily_user_stats
             WHERE user_id = ?
             GROUP BY stat_date
             HAVING total_seconds > 0
             ORDER BY total_seconds DESC
             LIMIT ?"
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to fetch top listening days: {}", e)))?;

        let items = rows
            .into_iter()
            .map(|(date, dow, seconds)| {
                let day_name = match dow.as_str() {
                    "0" => "Sunday",
                    "1" => "Monday",
                    "2" => "Tuesday",
                    "3" => "Wednesday",
                    "4" => "Thursday",
                    "5" => "Friday",
                    "6" => "Saturday",
                    _ => "Day",
                }
                .to_string();
                TopListeningDay {
                    day_date: date,
                    day_name,
                    total_seconds: seconds,
                }
            })
            .collect();

        Ok(items)
    }

    async fn get_ranked_tracks_scoped(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        window_end: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedTrackItem>> {
        let uid = user_id.unwrap_or("default");
        if window_end.is_none() {
            return self.get_all_time_ranked_tracks(uid, weights, limit).await;
        }
        let start_filter = window_start.unwrap_or(0);
        let end_filter = window_end.unwrap_or(i64::MAX);

        let sql = format!("
            SELECT
                COALESCE(t.id, h.track_id) as track_id,
                COALESCE(t.title, et.title, h.track_id) as title,
                COALESCE(a.name, et.artist, 'Unknown Artist') as artist_name,
                COALESCE(al.title, et.album) as album_title,
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
            FROM playback_history h
            LEFT JOIN tracks t ON h.track_id = t.id
            LEFT JOIN artists a ON t.artist_id = a.id
            LEFT JOIN albums al ON t.album_id = al.id
            LEFT JOIN external_tracks et ON h.track_id = et.id
            LEFT JOIN track_statistics ts ON ts.track_id = t.id AND ts.user_id = ?
            WHERE h.user_id = ?
              AND h.started_at >= ?
              AND h.started_at <= ?
            GROUP BY h.track_id
            HAVING score > 0 OR play_count > 0 OR total_seconds > 0
            ORDER BY score DESC, total_seconds DESC, play_count DESC
            LIMIT ?
        ");

        let records = sqlx::query_as::<_, RankedTrackItem>(&sql)
            .bind(weights.play_count_weight)
            .bind(weights.listening_duration_weight)
            .bind(weights.completion_weight)
            .bind(weights.user_preference_weight)
            .bind(weights.skip_penalty)
            .bind(uid)
            .bind(uid)
            .bind(start_filter)
            .bind(end_filter)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Ranked tracks query failed: {}", e)))?;

        Ok(records)
    }

    async fn get_ranked_artists_scoped(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        window_end: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedArtistItem>> {
        let uid = user_id.unwrap_or("default");
        if window_end.is_none() {
            return self.get_all_time_ranked_artists(uid, weights, limit).await;
        }
        let start_filter = window_start.unwrap_or(0);
        let end_filter = window_end.unwrap_or(i64::MAX);

        let sql = format!("
            SELECT
                COALESCE(a.id, et.artist, 'artist_' || COALESCE(a.name, et.artist, 'Unknown')) as artist_id,
                COALESCE(a.name, et.artist, 'Unknown Artist') as name,
                COALESCE(COUNT(CASE WHEN h.completed = 1 OR h.seconds_listened >= 30.0 THEN 1 END), 0) as play_count,
                COALESCE(SUM(h.seconds_listened), 0.0) as total_seconds,
                (
                    (? * COALESCE(COUNT(CASE WHEN h.completed = 1 OR h.seconds_listened >= 30.0 THEN 1 END), 0)) +
                    (? * (COALESCE(SUM(h.seconds_listened), 0.0) / 60.0)) -
                    (? * COALESCE(SUM(h.skipped), 0))
                ) as score
            FROM playback_history h
            LEFT JOIN tracks t ON h.track_id = t.id
            LEFT JOIN artists a ON t.artist_id = a.id
            LEFT JOIN external_tracks et ON h.track_id = et.id
            WHERE h.user_id = ?
              AND h.started_at >= ?
              AND h.started_at <= ?
            GROUP BY artist_id
            HAVING score > 0 OR play_count > 0 OR total_seconds > 0
            ORDER BY score DESC, total_seconds DESC, play_count DESC
            LIMIT ?
        ");

        let records = sqlx::query_as::<_, RankedArtistItem>(&sql)
            .bind(weights.play_count_weight)
            .bind(weights.listening_duration_weight)
            .bind(weights.skip_penalty)
            .bind(uid)
            .bind(start_filter)
            .bind(end_filter)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Ranked artists query failed: {}", e)))?;

        Ok(records)
    }

    async fn get_all_time_ranked_tracks(
        &self,
        user_id: &str,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedTrackItem>> {
        let sql = format!("
            SELECT
                COALESCE(t.id, et.id, ts.track_id) as track_id,
                COALESCE(t.title, et.title, ts.track_id) as title,
                COALESCE(a.name, et.artist, 'Unknown Artist') as artist_name,
                COALESCE(al.title, et.album) as album_title,
                CAST(ts.play_count AS INTEGER) as play_count,
                CAST(ts.total_time_listened AS REAL) as total_seconds,
                CAST(ts.completion_count AS INTEGER) as completion_count,
                CAST(ts.skip_count AS INTEGER) as skip_count,
                (
                    (? * ts.play_count) +
                    (? * (ts.total_time_listened / 60.0)) +
                    (? * ts.completion_count) +
                    (? * ts.manual_like) -
                    (? * ts.skip_count)
                ) as score
            FROM track_statistics ts
            LEFT JOIN tracks t ON ts.track_id = t.id
            LEFT JOIN artists a ON t.artist_id = a.id
            LEFT JOIN albums al ON t.album_id = al.id
            LEFT JOIN external_tracks et ON ts.track_id = et.id
            WHERE ts.user_id = ?
              AND (score > 0 OR ts.play_count > 0 OR ts.total_time_listened > 0)
            ORDER BY score DESC, ts.total_time_listened DESC, ts.play_count DESC
            LIMIT ?
        ");

        let records = sqlx::query_as::<_, RankedTrackItem>(&sql)
            .bind(weights.play_count_weight)
            .bind(weights.listening_duration_weight)
            .bind(weights.completion_weight)
            .bind(weights.user_preference_weight)
            .bind(weights.skip_penalty)
            .bind(user_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("All-time ranked tracks query failed: {}", e)))?;

        Ok(records)
    }

    async fn get_all_time_ranked_artists(
        &self,
        user_id: &str,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedArtistItem>> {
        let sql = format!("
            SELECT
                COALESCE(a.id, et.artist, 'artist_' || COALESCE(a.name, et.artist, 'Unknown')) as artist_id,
                COALESCE(a.name, et.artist, 'Unknown Artist') as name,
                CAST(SUM(ts.play_count) AS INTEGER) as play_count,
                CAST(SUM(ts.total_time_listened) AS REAL) as total_seconds,
                (
                    (? * SUM(ts.play_count)) +
                    (? * (SUM(ts.total_time_listened) / 60.0)) -
                    (? * SUM(ts.skip_count))
                ) as score
            FROM track_statistics ts
            LEFT JOIN tracks t ON ts.track_id = t.id
            LEFT JOIN artists a ON t.artist_id = a.id
            LEFT JOIN external_tracks et ON ts.track_id = et.id
            WHERE ts.user_id = ?
            GROUP BY artist_id
            HAVING score > 0 OR play_count > 0 OR total_seconds > 0
            ORDER BY score DESC, total_seconds DESC, play_count DESC
            LIMIT ?
        ");

        let records = sqlx::query_as::<_, RankedArtistItem>(&sql)
            .bind(weights.play_count_weight)
            .bind(weights.listening_duration_weight)
            .bind(weights.skip_penalty)
            .bind(user_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("All-time ranked artists query failed: {}", e)))?;

        Ok(records)
    }
}
