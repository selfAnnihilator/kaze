use crate::config::RankingWeightsConfig;
use crate::core::error::{AppError, AppResult};
use crate::database::models::TrackStatisticsRecord;
use async_trait::async_trait;
use chrono::{Datelike, Duration as ChronoDuration, NaiveDate, Utc};
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

    async fn get_stats_overview(
        &self,
        user_id: &str,
        user_joined_at: i64,
        app_start_date: i64,
        selected_year: Option<i32>,
        selected_month: Option<u32>,
        weights: &RankingWeightsConfig,
    ) -> AppResult<StatsOverview> {
        let now = Utc::now();
        let current_year = now.date_naive().year();
        let target_year = selected_year.unwrap_or(current_year);
        let target_month = selected_month.unwrap_or_else(|| {
            if target_year == current_year {
                now.month()
            } else {
                12
            }
        });

        // 1. Fetch available years from playback history and yearly archives
        let year_rows: Vec<(Option<i32>,)> = sqlx::query_as(
            "SELECT DISTINCT CAST(strftime('%Y', datetime(started_at, 'unixepoch', 'localtime')) AS INTEGER) as yr
             FROM playback_history
             WHERE (user_id = ? OR user_id = 'default')
             UNION
             SELECT DISTINCT year as yr
             FROM yearly_stats_archive
             WHERE (user_id = ? OR user_id = 'default')
             ORDER BY yr DESC"
        )
        .bind(user_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

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

        let start_of_month = NaiveDate::from_ymd_opt(target_year, target_month, 1)
            .unwrap_or_default()
            .and_hms_opt(0, 0, 0)
            .unwrap_or_default()
            .and_utc()
            .timestamp();
        let end_of_month = NaiveDate::from_ymd_opt(target_year, target_month, days_in_month)
            .unwrap_or_default()
            .and_hms_opt(23, 59, 59)
            .unwrap_or_default()
            .and_utc()
            .timestamp();

        let daily_rows: Vec<(String, f64)> = sqlx::query_as(
            "SELECT
                strftime('%Y-%m-%d', datetime(started_at, 'unixepoch', 'localtime')) as day_date,
                COALESCE(SUM(seconds_listened), 0.0) as total_seconds
             FROM playback_history
             WHERE (user_id = ? OR user_id = 'default') AND started_at >= ? AND started_at <= ?
             GROUP BY day_date"
        )
        .bind(user_id)
        .bind(start_of_month)
        .bind(end_of_month)
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

        // 2. Check if selected year is a past year
        if target_year < current_year {
            let archive: Option<(f64, String, String)> = sqlx::query_as(
                "SELECT total_seconds, top_songs_json, top_artists_json FROM yearly_stats_archive
                 WHERE (user_id = ? OR user_id = 'default') AND year = ?"
            )
            .bind(user_id)
            .bind(target_year)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten();

            if let Some((total_seconds, songs_json, artists_json)) = archive {
                let top_songs: Vec<RankedTrackItem> = serde_json::from_str(&songs_json).unwrap_or_default();
                let top_artists: Vec<RankedArtistItem> = serde_json::from_str(&artists_json).unwrap_or_default();
                return Ok(StatsOverview {
                    app_start_date,
                    user_joined_date: user_joined_at,
                    current_year,
                    selected_year: target_year,
                    available_years,
                    selected_month: target_month,
                    available_months,
                    daily_seconds: 0.0,
                    weekly_seconds: 0.0,
                    monthly_seconds,
                    total_year_seconds: total_seconds,
                    monthly_graph,
                    top_songs,
                    top_artists,
                    top_days: vec![],
                });
            }

            // Not yet archived: compute past year and archive it
            let start_of_yr = NaiveDate::from_ymd_opt(target_year, 1, 1)
                .unwrap_or_default()
                .and_hms_opt(0, 0, 0)
                .unwrap_or_default()
                .and_utc()
                .timestamp();
            let end_of_yr = NaiveDate::from_ymd_opt(target_year, 12, 31)
                .unwrap_or_default()
                .and_hms_opt(23, 59, 59)
                .unwrap_or_default()
                .and_utc()
                .timestamp();

            let total_seconds: f64 = sqlx::query_scalar(
                "SELECT COALESCE(SUM(seconds_listened), 0.0) FROM playback_history
                 WHERE (user_id = ? OR user_id = 'default') AND started_at >= ? AND started_at <= ?"
            )
            .bind(user_id)
            .bind(start_of_yr)
            .bind(end_of_yr)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0.0);

            let top_songs = self.get_ranked_tracks_scoped(Some(user_id), Some(start_of_yr), Some(end_of_yr), weights, 10).await.unwrap_or_default();
            let top_artists = self.get_ranked_artists_scoped(Some(user_id), Some(start_of_yr), Some(end_of_yr), weights, 10).await.unwrap_or_default();

            let id = format!("{}_{}", user_id, target_year);
            let songs_json = serde_json::to_string(&top_songs).unwrap_or_else(|_| "[]".to_string());
            let artists_json = serde_json::to_string(&top_artists).unwrap_or_else(|_| "[]".to_string());
            let _ = sqlx::query(
                "INSERT OR REPLACE INTO yearly_stats_archive (id, user_id, year, total_seconds, top_songs_json, top_artists_json, archived_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&id)
            .bind(user_id)
            .bind(target_year)
            .bind(total_seconds)
            .bind(&songs_json)
            .bind(&artists_json)
            .bind(now.timestamp())
            .execute(&self.pool)
            .await;

            return Ok(StatsOverview {
                app_start_date,
                user_joined_date: user_joined_at,
                current_year,
                selected_year: target_year,
                available_years,
                selected_month: target_month,
                available_months,
                daily_seconds: 0.0,
                weekly_seconds: 0.0,
                monthly_seconds,
                total_year_seconds: total_seconds,
                monthly_graph,
                top_songs,
                top_artists,
                top_days: vec![],
            });
        }

        // 3. Current year stats
        let start_of_day = now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap_or_default()
            .and_utc()
            .timestamp();
        let start_of_week = (now - ChronoDuration::days(7)).timestamp();
        let start_of_year = NaiveDate::from_ymd_opt(current_year, 1, 1)
            .unwrap_or_default()
            .and_hms_opt(0, 0, 0)
            .unwrap_or_default()
            .and_utc()
            .timestamp();

        let daily_seconds: f64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(seconds_listened), 0.0) FROM playback_history
             WHERE (user_id = ? OR user_id = 'default') AND started_at >= ?"
        )
        .bind(user_id)
        .bind(start_of_day)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("daily_seconds query failed: {}", e)))?;

        let weekly_seconds: f64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(seconds_listened), 0.0) FROM playback_history
             WHERE (user_id = ? OR user_id = 'default') AND started_at >= ?"
        )
        .bind(user_id)
        .bind(start_of_week)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0.0);

        let total_year_seconds: f64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(seconds_listened), 0.0) FROM playback_history
             WHERE (user_id = ? OR user_id = 'default') AND started_at >= ?"
        )
        .bind(user_id)
        .bind(start_of_year)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0.0);

        let top_songs = self.get_ranked_tracks_scoped(Some(user_id), Some(start_of_year), None, weights, 10).await.unwrap_or_default();
        let top_artists = self.get_ranked_artists_scoped(Some(user_id), Some(start_of_year), None, weights, 10).await.unwrap_or_default();
        let top_days = self.get_top_listening_days(user_id, 5).await.unwrap_or_default();

        Ok(StatsOverview {
            app_start_date,
            user_joined_date: user_joined_at,
            current_year,
            selected_year: current_year,
            available_years,
            selected_month: target_month,
            available_months,
            daily_seconds,
            weekly_seconds,
            monthly_seconds,
            total_year_seconds,
            monthly_graph,
            top_songs,
            top_artists,
            top_days,
        })
    }

    async fn get_top_listening_days(&self, user_id: &str, limit: u32) -> AppResult<Vec<TopListeningDay>> {
        let rows: Vec<(String, String, f64)> = sqlx::query_as(
            "SELECT
                strftime('%Y-%m-%d', datetime(started_at, 'unixepoch', 'localtime')) as day_date,
                strftime('%w', datetime(started_at, 'unixepoch', 'localtime')) as day_of_week,
                COALESCE(SUM(seconds_listened), 0.0) as total_seconds
             FROM playback_history
             WHERE (user_id = ? OR user_id = 'default')
             GROUP BY day_date
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
            LEFT JOIN track_statistics ts ON ts.track_id = t.id
            WHERE (h.user_id = ? OR h.user_id = 'default')
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
            WHERE (h.user_id = ? OR h.user_id = 'default')
              AND h.started_at >= ?
              AND h.started_at <= ?
            GROUP BY COALESCE(a.id, a.name, et.artist)
            HAVING total_seconds > 0 OR play_count > 0
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
}
