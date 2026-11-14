//! Device-scoped and all-time rankings.
use super::*;

impl SqliteStatsRepository {
    pub(super) async fn get_ranked_tracks_scoped_query(
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

        let scored_history = crate::recommendations::engagement::scored_history_sql();
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
                    (? * COALESCE(COUNT(CASE WHEN h.qualified = 1 THEN 1 END), 0)) +
                    (? * (COALESCE(SUM(h.qualified_seconds), 0.0) / 60.0)) +
                    (? * COALESCE(SUM(h.scored_completion), 0)) +
                    (? * COALESCE(ts.manual_like, 0)) -
                    (? * COALESCE(SUM(h.scored_skip), 0))
                ) as score
            FROM ({scored_history}) h
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

    pub(super) async fn get_ranked_artists_scoped_query(
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

        let scored_history = crate::recommendations::engagement::scored_history_sql();
        let sql = format!("
            SELECT
                MIN(COALESCE(a.id, et.artist, 'artist_' || COALESCE(a.name, et.artist, 'Unknown'))) as artist_id,
                MIN(COALESCE(a.name, et.artist, 'Unknown Artist')) as name,
                COALESCE(COUNT(CASE WHEN h.completed = 1 OR h.seconds_listened >= 30.0 THEN 1 END), 0) as play_count,
                COALESCE(SUM(h.seconds_listened), 0.0) as total_seconds,
                (
                    (? * COALESCE(COUNT(CASE WHEN h.qualified = 1 THEN 1 END), 0)) +
                    (? * (COALESCE(SUM(h.qualified_seconds), 0.0) / 60.0)) -
                    (? * COALESCE(SUM(h.scored_skip), 0))
                ) as score
            FROM ({scored_history}) h
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

    pub(super) async fn get_all_time_ranked_tracks_query(
        &self,
        user_id: &str,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedTrackItem>> {
        let scored_stats = crate::recommendations::engagement::scored_stats_sql();
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
                    (? * ts.scored_play_count) +
                    (? * (ts.qualified_time_listened / 60.0)) +
                    (? * ts.scored_completion_count) +
                    (? * ts.manual_like) -
                    (? * ts.scored_skip_count)
                ) as score
            FROM ({scored_stats}) ts
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

    pub(super) async fn get_all_time_ranked_artists_query(
        &self,
        user_id: &str,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedArtistItem>> {
        let scored_stats = crate::recommendations::engagement::scored_stats_sql();
        let sql = format!("
            SELECT
                COALESCE(a.id, et.artist, 'artist_' || COALESCE(a.name, et.artist, 'Unknown')) as artist_id,
                COALESCE(a.name, et.artist, 'Unknown Artist') as name,
                CAST(SUM(ts.play_count) AS INTEGER) as play_count,
                CAST(SUM(ts.total_time_listened) AS REAL) as total_seconds,
                (
                    (? * SUM(ts.scored_play_count)) +
                    (? * (SUM(ts.qualified_time_listened) / 60.0)) -
                    (? * SUM(ts.scored_skip_count))
                ) as score
            FROM ({scored_stats}) ts
            LEFT JOIN tracks t ON ts.track_id = t.id
            LEFT JOIN artists a ON t.artist_id = a.id
            LEFT JOIN external_tracks et ON ts.track_id = et.id
            WHERE ts.user_id = ?
            GROUP BY LOWER(TRIM(COALESCE(a.name, et.artist, 'Unknown Artist')))
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

    pub(super) async fn get_all_time_ranked_genres_query(
        &self,
        user_id: &str,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedGenreItem>> {
        let scored_stats = crate::recommendations::engagement::scored_stats_sql();
        let sql = format!("
            SELECT
                MIN(COALESCE(g.id, 'genre_' || LOWER(TRIM(et.genre)))) as genre_id,
                MIN(COALESCE(g.name, TRIM(et.genre))) as name,
                CAST(SUM(ts.play_count) AS INTEGER) as play_count,
                CAST(SUM(ts.total_time_listened) AS REAL) as total_seconds,
                (
                    (? * SUM(ts.scored_play_count)) +
                    (? * (SUM(ts.qualified_time_listened) / 60.0)) -
                    (? * SUM(ts.scored_skip_count))
                ) as score
            FROM ({scored_stats}) ts
            LEFT JOIN tracks t ON ts.track_id = t.id
            LEFT JOIN genres g ON t.genre_id = g.id
            LEFT JOIN external_tracks et ON ts.track_id = et.id
            WHERE ts.user_id = ?
              AND COALESCE(g.name, et.genre, '') <> ''
              AND TRIM(COALESCE(g.name, et.genre, '')) <> ''
            GROUP BY LOWER(TRIM(COALESCE(g.name, et.genre)))
            HAVING score > 0 OR play_count > 0 OR total_seconds > 0
            ORDER BY score DESC, total_seconds DESC, play_count DESC
            LIMIT ?
        ");

        sqlx::query_as::<_, RankedGenreItem>(&sql)
            .bind(weights.play_count_weight)
            .bind(weights.listening_duration_weight)
            .bind(weights.skip_penalty)
            .bind(user_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("All-time ranked genres query failed: {}", e)))
    }
}
