//! Per-user ranked tracks, artists, albums, genres and top days.
use super::*;

impl SqliteStatsRepository {
    pub(super) async fn get_ranked_tracks_for_user_query(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedTrackItem>> {
        let uid = user_id.unwrap_or("default");
        let start_filter = window_start.unwrap_or(0);

        // Compute aggregated metrics from playback_history within the window, joined with track metadata
        let scored_history = crate::recommendations::engagement::scored_history_sql();
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
                    (? * COALESCE(COUNT(CASE WHEN h.qualified = 1 THEN 1 END), 0)) +
                    (? * (COALESCE(SUM(h.qualified_seconds), 0.0) / 60.0)) +
                    (? * COALESCE(SUM(h.scored_completion), 0)) +
                    (? * COALESCE(ts.manual_like, 0)) -
                    (? * COALESCE(SUM(h.scored_skip), 0))
                ) as score
            FROM tracks t
            LEFT JOIN artists a ON t.artist_id = a.id
            LEFT JOIN albums al ON t.album_id = al.id
            LEFT JOIN track_statistics ts ON ts.track_id = t.id AND ts.user_id = ?
            LEFT JOIN ({scored_history}) h ON h.track_id = t.id AND h.started_at >= ? AND h.user_id = ?
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

    pub(super) async fn get_ranked_artists_for_user_query(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedArtistItem>> {
        let uid = user_id.unwrap_or("default");
        let start_filter = window_start.unwrap_or(0);

        let scored_history = crate::recommendations::engagement::scored_history_sql();
        let sql = format!("
            SELECT
                a.id as artist_id,
                a.name as name,
                COALESCE(COUNT(CASE WHEN h.completed = 1 OR h.seconds_listened >= 30.0 THEN 1 END), 0) as play_count,
                COALESCE(SUM(h.seconds_listened), 0.0) as total_seconds,
                (
                    (? * COALESCE(COUNT(CASE WHEN h.qualified = 1 THEN 1 END), 0)) +
                    (? * (COALESCE(SUM(h.qualified_seconds), 0.0) / 60.0)) -
                    (? * COALESCE(SUM(h.scored_skip), 0))
                ) as score
            FROM artists a
            JOIN tracks t ON t.artist_id = a.id
            JOIN ({scored_history}) h ON h.track_id = t.id AND h.started_at >= ? AND h.user_id = ?
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

    pub(super) async fn get_ranked_albums_for_user_query(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedAlbumItem>> {
        let uid = user_id.unwrap_or("default");
        let start_filter = window_start.unwrap_or(0);

        let scored_history = crate::recommendations::engagement::scored_history_sql();
        let sql = format!("
            SELECT
                al.id as album_id,
                al.title as title,
                a.name as artist_name,
                COALESCE(COUNT(CASE WHEN h.completed = 1 OR h.seconds_listened >= 30.0 THEN 1 END), 0) as play_count,
                COALESCE(SUM(h.seconds_listened), 0.0) as total_seconds,
                (
                    (? * COALESCE(COUNT(CASE WHEN h.qualified = 1 THEN 1 END), 0)) +
                    (? * (COALESCE(SUM(h.qualified_seconds), 0.0) / 60.0)) -
                    (? * COALESCE(SUM(h.scored_skip), 0))
                ) as score
            FROM albums al
            LEFT JOIN artists a ON al.artist_id = a.id
            JOIN tracks t ON t.album_id = al.id
            JOIN ({scored_history}) h ON h.track_id = t.id AND h.started_at >= ? AND h.user_id = ?
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

    pub(super) async fn get_ranked_genres_for_user_query(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedGenreItem>> {
        let uid = user_id.unwrap_or("default");
        let start_filter = window_start.unwrap_or(0);

        let scored_history = crate::recommendations::engagement::scored_history_sql();
        let sql = format!("
            SELECT
                g.id as genre_id,
                g.name as name,
                COALESCE(COUNT(CASE WHEN h.completed = 1 OR h.seconds_listened >= 30.0 THEN 1 END), 0) as play_count,
                COALESCE(SUM(h.seconds_listened), 0.0) as total_seconds,
                (
                    (? * COALESCE(COUNT(CASE WHEN h.qualified = 1 THEN 1 END), 0)) +
                    (? * (COALESCE(SUM(h.qualified_seconds), 0.0) / 60.0)) -
                    (? * COALESCE(SUM(h.scored_skip), 0))
                ) as score
            FROM genres g
            JOIN tracks t ON t.genre_id = g.id
            JOIN ({scored_history}) h ON h.track_id = t.id AND h.started_at >= ? AND h.user_id = ?
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

    pub(super) async fn get_top_listening_days_query(&self, user_id: &str, limit: u32) -> AppResult<Vec<TopListeningDay>> {
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
}
