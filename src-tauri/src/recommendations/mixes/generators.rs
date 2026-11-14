//! The individual smart mix generators.
use super::*;

impl SmartMixGenerator {
    /// Daily mix: 60% high-affinity tracks, 20% forgotten favorites, 20% discovery & online picks.
    pub(super) async fn generate_daily_mix(&self, user_id: &str) -> AppResult<Vec<String>> {
        if let Some(ids) = self.starter_mix(user_id).await? {
            return Ok(ids);
        }
        let recs = self.recommender.recommend(user_id, 20).await?;
        let mut track_ids = Vec::new();
        let mut seen = HashSet::new();

        for r in recs {
            if seen.insert(r.track_id.clone()) {
                track_ids.push(r.track_id);
            }
        }

        // Blend in online discovery songs from external_tracks
        let online_candidates: Vec<OnlineCandidateRow> =
            sqlx::query_as(
                "SELECT id, title, artist, album, duration_secs, cover_art_url, preview_url
                 FROM external_tracks
                 WHERE match_status != 'IGNORE' AND (provider='jiosaavn' OR EXISTS(SELECT 1 FROM online_track_sources s WHERE s.track_id=external_tracks.id))
                 ORDER BY RANDOM()
                 LIMIT 12"
            )
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

        for (oid, otitle, oartist, oalbum, odur, ocover, opreview) in online_candidates {
            if seen.insert(oid.clone()) {
                let _ = ensure_online_track(
                    &self.pool,
                    &OnlineTrackInput {
                        id: &oid,
                        title: &otitle,
                        artist: oartist.as_deref(),
                        album: oalbum.as_deref(),
                        duration_secs: odur,
                        cover_art_url: ocover.as_deref(),
                        preview_url: opreview.as_deref(),
                    },
                )
                .await;
                track_ids.push(oid);
            }
        }

        // If library has fewer recommendations, pad with random unplayed tracks
        if track_ids.len() < 10 {
            let padding: Vec<String> =
                sqlx::query_scalar("SELECT id FROM tracks ORDER BY RANDOM() LIMIT 20")
                    .fetch_all(&self.pool)
                    .await
                    .map_err(|e| AppError::Database(e.to_string()))?;

            for id in padding {
                if seen.insert(id.clone()) {
                    track_ids.push(id);
                }
            }
        }

        Ok(track_ids)
    }

    /// On repeat: tracks played often and completed in the last 14 days.
    pub(super) async fn generate_on_repeat_mix(&self, user_id: &str) -> AppResult<Vec<String>> {
        let cutoff = Utc::now().timestamp() - (14 * 86400);

        let qualified = super::super::engagement::qualified_sql("h.seconds_listened", "COALESCE(t.duration_secs,0)");
        let near_complete = super::super::engagement::NEAR_COMPLETE_FRACTION;
        let sql = format!(
            "SELECT h.track_id FROM playback_history h JOIN tracks t ON t.id=h.track_id
             WHERE h.user_id=? AND h.started_at>=? AND {qualified}
             GROUP BY h.track_id
             HAVING SUM(CASE WHEN h.completed=1 AND t.duration_secs>0 AND h.seconds_listened>={near_complete}*t.duration_secs THEN 1 ELSE 0 END)>=1 OR COUNT(h.id)>=2
             ORDER BY COUNT(h.id) DESC,SUM(h.seconds_listened) DESC LIMIT 25");
        let rows: Vec<String> = sqlx::query_scalar(&sql)
        .bind(user_id)
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        if rows.is_empty() {
            // Fallback 1: top tracks from track_statistics
            let fallback: Vec<String> = sqlx::query_scalar(
                "SELECT track_id FROM track_statistics WHERE user_id = ? AND manual_like != -1 ORDER BY play_count DESC LIMIT 25"
            )
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

            if !fallback.is_empty() {
                return Ok(fallback);
            }

            // Fallback 2: random tracks from tracks
            let random_fallback: Vec<String> =
                sqlx::query_scalar("SELECT id FROM tracks ORDER BY RANDOM() LIMIT 25")
                    .fetch_all(&self.pool)
                    .await
                    .map_err(|e| AppError::Database(e.to_string()))?;

            return Ok(random_fallback);
        }

        Ok(rows)
    }

    /// Forgotten favorites: played or liked, but unplayed in over 30 days.
    pub(super) async fn generate_forgotten_favorites_mix(&self, user_id: &str) -> AppResult<Vec<String>> {
        let cutoff = Utc::now().timestamp() - (30 * 86400);

        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT ts.track_id
             FROM track_statistics ts
             WHERE ts.user_id = ?
               AND (ts.play_count >= 2 OR ts.manual_like = 1)
               AND (ts.last_played_at IS NULL OR ts.last_played_at < ?)
               AND ts.manual_like != -1
             ORDER BY ts.play_count DESC, ts.total_time_listened DESC
             LIMIT 25",
        )
        .bind(user_id)
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        if rows.is_empty() {
            // Fallback: tracks from /fav/ folder or with manual_like = 1, or random sample
            let fallback: Vec<String> = sqlx::query_scalar(
                "SELECT t.id
                 FROM tracks t
                 LEFT JOIN track_statistics ts ON ts.track_id = t.id AND ts.user_id = ?
                 WHERE (LOWER(t.file_path) LIKE '%/fav/%' OR ts.manual_like = 1)
                   AND COALESCE(ts.manual_like, 0) != -1
                 ORDER BY RANDOM()
                 LIMIT 25",
            )
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

            if !fallback.is_empty() {
                return Ok(fallback);
            }

            let general: Vec<String> =
                sqlx::query_scalar("SELECT id FROM tracks ORDER BY RANDOM() LIMIT 25")
                    .fetch_all(&self.pool)
                    .await
                    .map_err(|e| AppError::Database(e.to_string()))?;

            return Ok(general);
        }

        Ok(rows)
    }

    /// Genre mix: tracks matching genre, ordered by performance and affinity.
    pub(super) async fn generate_genre_mix(&self, user_id: &str, genre_name: &str) -> AppResult<Vec<String>> {
        let clean_genre = genre_name.trim();
        let pattern = format!("%{}%", clean_genre.to_lowercase());
        let folder_pattern = format!("%/{}%", clean_genre.to_lowercase().replace('-', ""));
        let folder_pattern_raw = format!("%/{}%", clean_genre.to_lowercase());
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT t.id
             FROM tracks t
             LEFT JOIN genres g ON g.id = t.genre_id
             LEFT JOIN track_statistics ts ON ts.track_id = t.id AND ts.user_id = ?
             WHERE (
                 LOWER(COALESCE(g.name, '')) LIKE ?
                 OR LOWER(t.file_path) LIKE ?
                 OR LOWER(t.file_path) LIKE ?
             )
             AND COALESCE(ts.manual_like, 0) != -1
             ORDER BY COALESCE(ts.play_count, 0) DESC, RANDOM()
             LIMIT 30",
        )
        .bind(user_id)
        .bind(&pattern)
        .bind(&folder_pattern)
        .bind(&folder_pattern_raw)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let mut track_ids = if rows.is_empty() {
            // Fallback: random tracks
            sqlx::query_scalar("SELECT id FROM tracks ORDER BY RANDOM() LIMIT 25")
                .fetch_all(&self.pool)
                .await
                .unwrap_or_default()
        } else {
            rows
        };

        let mut seen: HashSet<String> = track_ids.iter().cloned().collect();

        // Blend in online songs for this genre from external_tracks
        let online_candidates: Vec<OnlineCandidateRow> =
            sqlx::query_as(
                "SELECT id, title, artist, album, duration_secs, cover_art_url, preview_url
                 FROM external_tracks
                 WHERE (LOWER(COALESCE(genre, '')) LIKE ? OR LOWER(title) LIKE ? OR LOWER(artist) LIKE ?)
                   AND match_status != 'IGNORE'
                 ORDER BY RANDOM()
                 LIMIT 15"
            )
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

        for (oid, otitle, oartist, oalbum, odur, ocover, opreview) in online_candidates {
            if seen.insert(oid.clone()) {
                let _ = ensure_online_track(
                    &self.pool,
                    &OnlineTrackInput {
                        id: &oid,
                        title: &otitle,
                        artist: oartist.as_deref(),
                        album: oalbum.as_deref(),
                        duration_secs: odur,
                        cover_art_url: ocover.as_deref(),
                        preview_url: opreview.as_deref(),
                    },
                )
                .await;
                track_ids.push(oid);
            }
        }

        Ok(track_ids)
    }

    /// Artist mix: tracks by artist, plus related artists sharing genres.
    pub(super) async fn generate_artist_mix(
        &self,
        user_id: &str,
        artist_name: &str,
    ) -> AppResult<Vec<String>> {
        let mut track_ids: Vec<String> = Vec::new();

        // 1. Direct tracks by artist
        let direct: Vec<String> = sqlx::query_scalar(
            "SELECT t.id
             FROM tracks t
             JOIN artists a ON a.id = t.artist_id
             LEFT JOIN track_statistics ts ON ts.track_id = t.id AND ts.user_id = ?
             WHERE LOWER(a.name) = LOWER(?)
               AND COALESCE(ts.manual_like, 0) != -1
             ORDER BY RANDOM()
             LIMIT 15",
        )
        .bind(user_id)
        .bind(artist_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        track_ids.extend(direct);

        // 2. Related tracks from shared genres
        let related: Vec<String> = sqlx::query_scalar(
            "SELECT t.id
             FROM tracks t
             JOIN artists a ON a.id = t.artist_id
             JOIN genres g ON g.id = t.genre_id
             LEFT JOIN track_statistics ts ON ts.track_id = t.id AND ts.user_id = ?
             WHERE LOWER(a.name) != LOWER(?)
               AND g.id IN (
                   SELECT t2.genre_id
                   FROM tracks t2
                   JOIN artists a2 ON a2.id = t2.artist_id
                   WHERE LOWER(a2.name) = LOWER(?) AND t2.genre_id IS NOT NULL
               )
               AND COALESCE(ts.manual_like, 0) != -1
             ORDER BY RANDOM()
             LIMIT 15",
        )
        .bind(user_id)
        .bind(artist_name)
        .bind(artist_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        track_ids.extend(related);
        Ok(track_ids)
    }

    /// Late night mix: the styles this user actually plays at night (when the
    /// night context bucket has enough listens), topped up with mellow picks.
    pub(super) async fn generate_late_night_mix(&self, user_id: &str) -> AppResult<Vec<String>> {
        let profile = self.taste_engine.compute_taste_profile(user_id).await?;
        let mut night_genres: Vec<(String, f64)> = profile
            .context_affinity
            .get("night")
            .map(|night| night.genres.iter().map(|(g, v)| (g.clone(), *v)).collect())
            .unwrap_or_default();
        night_genres.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        night_genres.truncate(3);
        let mut track_ids: Vec<String> = Vec::new();
        if !night_genres.is_empty() {
            let placeholders = vec!["?"; night_genres.len()].join(",");
            let sql = format!(
                "SELECT t.id
                 FROM tracks t
                 LEFT JOIN genres g ON g.id = t.genre_id
                 LEFT JOIN track_statistics ts ON ts.track_id = t.id AND ts.user_id = ?
                 WHERE COALESCE(ts.manual_like, 0) != -1
                   AND LOWER(TRIM(COALESCE(g.name, ''))) IN ({placeholders})
                 ORDER BY RANDOM()
                 LIMIT 15"
            );
            let mut query = sqlx::query_scalar::<_, String>(&sql).bind(user_id);
            for (genre, _) in &night_genres {
                query = query.bind(genre);
            }
            track_ids = query
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT t.id
             FROM tracks t
             LEFT JOIN genres g ON g.id = t.genre_id
             LEFT JOIN track_statistics ts ON ts.track_id = t.id AND ts.user_id = ?
             WHERE COALESCE(ts.manual_like, 0) != -1
               AND (
                   LOWER(COALESCE(g.name, '')) LIKE '%ambient%'
                   OR LOWER(COALESCE(g.name, '')) LIKE '%chill%'
                   OR LOWER(COALESCE(g.name, '')) LIKE '%acoustic%'
                   OR LOWER(COALESCE(g.name, '')) LIKE '%jazz%'
                   OR LOWER(COALESCE(g.name, '')) LIKE '%downtempo%'
                   OR t.duration_secs >= 240.0
               )
             ORDER BY RANDOM()
             LIMIT 25",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        for id in rows {
            if track_ids.len() >= 25 {
                break;
            }
            if !track_ids.contains(&id) {
                track_ids.push(id);
            }
        }
        Ok(track_ids)
    }

    /// Discovery mix: local tracks with 0 plays that align with user taste.
    pub(super) async fn generate_discovery_mix(&self, user_id: &str) -> AppResult<Vec<String>> {
        if let Some(ids) = self.starter_mix(user_id).await? {
            return Ok(ids);
        }
        let profile = self.taste_engine.compute_taste_profile(user_id).await?;
        let recs = self
            .recommender
            .recommend_with_profile(user_id, &profile, 50)
            .await?;

        let mut discovery_tracks: Vec<String> = recs
            .into_iter()
            .filter(|r| r.is_discovery)
            .take(20)
            .map(|r| r.track_id)
            .collect();

        if discovery_tracks.is_empty() {
            // Fallback: any unplayed tracks
            let fallback: Vec<String> = sqlx::query_scalar(
                "SELECT t.id
                 FROM tracks t
                 LEFT JOIN track_statistics ts ON ts.track_id = t.id AND ts.user_id = ?
                 WHERE (ts.play_count IS NULL OR ts.play_count = 0)
                   AND COALESCE(ts.manual_like, 0) != -1
                 ORDER BY RANDOM()
                 LIMIT 25",
            )
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

            discovery_tracks = fallback;
        }

        let mut seen: HashSet<String> = discovery_tracks.iter().cloned().collect();

        // Blend in online discovery tracks from external_tracks
        let online_candidates: Vec<OnlineCandidateRow> =
            sqlx::query_as(
                "SELECT id, title, artist, album, duration_secs, cover_art_url, preview_url
                 FROM external_tracks
                 WHERE match_status != 'IGNORE' AND (provider='jiosaavn' OR EXISTS(SELECT 1 FROM online_track_sources s WHERE s.track_id=external_tracks.id))
                 ORDER BY RANDOM()
                 LIMIT 15"
            )
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

        for (oid, otitle, oartist, oalbum, odur, ocover, opreview) in online_candidates {
            if seen.insert(oid.clone()) {
                let _ = ensure_online_track(
                    &self.pool,
                    &OnlineTrackInput {
                        id: &oid,
                        title: &otitle,
                        artist: oartist.as_deref(),
                        album: oalbum.as_deref(),
                        duration_secs: odur,
                        cover_art_url: ocover.as_deref(),
                        preview_url: opreview.as_deref(),
                    },
                )
                .await;
                discovery_tracks.push(oid);
            }
        }

        Ok(discovery_tracks)
    }
}
