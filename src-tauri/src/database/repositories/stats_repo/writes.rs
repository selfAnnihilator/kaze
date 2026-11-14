//! Recording playback statistics, daily totals and likes.
use super::*;

impl SqliteStatsRepository {
    pub(super) async fn update_track_playback_stats_scoped_query(
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

            if let Some((artist_id, genre_id)) = track_info {
                for (table, column, id) in [("artist_statistics", "artist_id", artist_id),
                                            ("genre_statistics", "genre_id", genre_id)] {
                    if let Some(id) = id {
                        let sql = format!(
                            "INSERT INTO {table} (user_id, {column}, play_count, total_time_listened, last_played_at, affinity_score)
                             VALUES (?, ?, 1, ?, ?, 1.0)
                             ON CONFLICT(user_id, {column}) DO UPDATE SET
                                play_count = play_count + 1,
                                total_time_listened = total_time_listened + excluded.total_time_listened,
                                last_played_at = excluded.last_played_at,
                                affinity_score = affinity_score + 1.0");
                        sqlx::query(&sql)
                            .bind(user_id)
                            .bind(&id)
                            .bind(listened_secs)
                            .bind(now)
                            .execute(&self.pool)
                            .await
                            .map_err(|e| AppError::Database(format!("Failed to update {table}: {e}")))?;
                    }
                }
            }
        }

        Ok(())
    }

    pub(super) async fn record_daily_playback_query(
        &self,
        user_id: &str,
        device_id: Option<&str>,
        stat_date: &str,
        playback: DailyPlayback,
    ) -> AppResult<()> {
        let dev_id = self.resolve_device_id(device_id).await;
        let now = Utc::now().timestamp();
        let play_inc = i64::from(playback.is_meaningful);
        let comp_inc = i64::from(playback.completed);
        let skip_inc = i64::from(playback.skipped);

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
        .bind(playback.seconds)
        .bind(play_inc)
        .bind(comp_inc)
        .bind(skip_inc)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to record daily playback: {}", e)))?;

        Ok(())
    }

    pub(super) async fn backfill_daily_stats_from_history_query(&self, default_device_id: Option<&str>) -> AppResult<usize> {
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

    pub(super) async fn set_track_like_scoped_query(&self, user_id: &str, track_id: &str, like_status: i64) -> AppResult<()> {
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

    pub(super) async fn get_track_stats_scoped_query(&self, user_id: &str, track_id: &str) -> AppResult<Option<TrackStatisticsRecord>> {
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
}
