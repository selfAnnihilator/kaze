//! Applying the pulled cloud state to the local profile.
use super::*;

impl SyncManager {
    pub async fn apply_remote_sync_payload(
        pool: &SqlitePool,
        user_id: &str,
        payload: &SyncPayload,
    ) -> AppResult<(usize, usize)> {
        let now = Utc::now().timestamp();

        // Retrieve local tombstones so explicitly deleted items are never re-inserted
        let tombstones = Self::get_tombstones(pool, user_id)
            .await
            .unwrap_or_default();
        let deleted_playlists_set: std::collections::HashSet<String> = tombstones
            .iter()
            .filter(|(_, t, _)| t == "playlist")
            .map(|(_, _, id)| id.clone())
            .collect();
        let deleted_songs_set: std::collections::HashSet<String> = tombstones
            .iter()
            .filter(|(_, t, _)| t == "playlist_song")
            .map(|(_, _, id)| id.clone())
            .collect();

        // 1. Ingest remote songs into external_tracks if needed
        for song in &payload.songs {
            crate::cloud::song_restore::restore_song(pool, song, now).await?;
        }

        // 2. Playlists
        let mut inserted_playlists = 0;
        for p in &payload.playlists {
            if deleted_playlists_set.contains(&p.id) {
                continue; // User explicitly deleted this playlist locally
            }

            let res = sqlx::query(
                "INSERT INTO playlists (id, user_id, name, description, is_smart_mix, mix_type, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET
                     user_id = excluded.user_id,
                     name = excluded.name,
                     description = excluded.description,
                     updated_at = excluded.updated_at"
            )
            .bind(&p.id)
            .bind(user_id)
            .bind(&p.name)
            .bind(&p.description)
            .bind(p.is_smart_mix)
            .bind(&p.mix_type)
            .bind(p.created_at)
            .bind(p.updated_at)
            .execute(pool)
            .await;

            if res.is_ok() {
                inserted_playlists += 1;
            }
        }

        // 3. Playlist songs
        let mut inserted_memberships = 0;
        for ps in &payload.playlist_songs {
            if deleted_playlists_set.contains(&ps.playlist_id) {
                continue;
            }
            let key = format!("{}:{}", ps.playlist_id, ps.song_id);
            if deleted_songs_set.contains(&key) {
                continue; // User explicitly removed this track from playlist locally
            }

            let res = sqlx::query(
                "INSERT INTO playlist_tracks (playlist_id, track_id, position, added_at)
                 VALUES (?, ?, ?, ?)
                 ON CONFLICT(playlist_id, position) DO UPDATE SET
                     track_id = excluded.track_id,
                     added_at = excluded.added_at",
            )
            .bind(&ps.playlist_id)
            .bind(&ps.song_id)
            .bind(ps.position)
            .bind(ps.added_at)
            .execute(pool)
            .await;

            if res.is_ok() {
                inserted_memberships += 1;
            }
        }

        // 4. Song stats (stores and syncs liked: 1, disliked: -1, and neutral: 0 across users)
        for ss in &payload.song_stats {
            let stat_user_id = if ss.user_id.is_empty() {
                user_id
            } else {
                &ss.user_id
            };
            sqlx::query(
                "INSERT INTO track_statistics (user_id, track_id, play_count, total_time_listened, completion_count, skip_count, last_played_at, manual_like)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(user_id, track_id) DO UPDATE SET
                     play_count = MAX(track_statistics.play_count, excluded.play_count),
                     total_time_listened = MAX(track_statistics.total_time_listened, excluded.total_time_listened),
                     completion_count = MAX(track_statistics.completion_count, excluded.completion_count),
                     skip_count = MAX(track_statistics.skip_count, excluded.skip_count),
                     last_played_at = MAX(coalesce(track_statistics.last_played_at, 0), coalesce(excluded.last_played_at, 0)),
                     manual_like = excluded.manual_like"
            )
            .bind(stat_user_id)
            .bind(&ss.song_id)
            .bind(ss.play_count)
            .bind(ss.total_time_listened)
            .bind(ss.completion_count)
            .bind(ss.skip_count)
            .bind(ss.last_played_at)
            .bind(ss.manual_like)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to reconcile song stats: {}", e)))?;
        }

        crate::recommendations::signals::remember_imported_dislikes(pool, user_id).await?;

        // 4.5. Daily user stats
        for ds in &payload.daily_stats {
            let stat_user_id = user_id;
            sqlx::query(
                "INSERT INTO daily_user_stats (
                    user_id, device_id, stat_date, listening_seconds, play_count, completion_count, skip_count, updated_at
                 ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(user_id, device_id, stat_date) DO UPDATE SET
                     listening_seconds = MAX(daily_user_stats.listening_seconds, excluded.listening_seconds),
                     play_count = MAX(daily_user_stats.play_count, excluded.play_count),
                     completion_count = MAX(daily_user_stats.completion_count, excluded.completion_count),
                     skip_count = MAX(daily_user_stats.skip_count, excluded.skip_count),
                     updated_at = MAX(daily_user_stats.updated_at, excluded.updated_at)"
            )
            .bind(stat_user_id)
            .bind(&ds.device_id)
            .bind(&ds.stat_date)
            .bind(ds.listening_seconds)
            .bind(ds.play_count)
            .bind(ds.completion_count)
            .bind(ds.skip_count)
            .bind(ds.updated_at)
            .execute(pool)
            .await.map_err(|e| AppError::Database(format!("Failed to reconcile daily stats: {}", e)))?;
        }

        // 4.6. Track device stats
        for tds in &payload.track_device_stats {
            let stat_user_id = user_id;
            sqlx::query(
                "INSERT INTO track_device_statistics (
                    user_id, device_id, track_id, play_count, total_time_listened, completion_count, skip_count, last_played_at, updated_at
                 ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(user_id, device_id, track_id) DO UPDATE SET
                     play_count = MAX(track_device_statistics.play_count, excluded.play_count),
                     total_time_listened = MAX(track_device_statistics.total_time_listened, excluded.total_time_listened),
                     completion_count = MAX(track_device_statistics.completion_count, excluded.completion_count),
                     skip_count = MAX(track_device_statistics.skip_count, excluded.skip_count),
                     last_played_at = CASE
                         WHEN track_device_statistics.last_played_at IS NULL THEN excluded.last_played_at
                         WHEN excluded.last_played_at IS NULL THEN track_device_statistics.last_played_at
                         ELSE MAX(track_device_statistics.last_played_at, excluded.last_played_at)
                     END,
                     updated_at = MAX(track_device_statistics.updated_at, excluded.updated_at)"
            )
            .bind(stat_user_id)
            .bind(&tds.device_id)
            .bind(&tds.track_id)
            .bind(tds.play_count)
            .bind(tds.total_seconds)
            .bind(tds.completion_count)
            .bind(tds.skip_count)
            .bind(tds.last_played_at)
            .bind(tds.updated_at)
            .execute(pool)
            .await.map_err(|e| AppError::Database(format!("Failed to reconcile track device stats: {}", e)))?;

            // Keep the track_statistics compatibility layer at least the sum across devices;
            // MAX so plays recorded before per-device rows existed are never wiped.
            sqlx::query(
                "INSERT INTO track_statistics (
                    user_id, track_id, play_count, total_time_listened, completion_count, skip_count, last_played_at
                 )
                 SELECT user_id, track_id, SUM(play_count), SUM(total_time_listened), SUM(completion_count), SUM(skip_count), MAX(last_played_at)
                 FROM track_device_statistics
                 WHERE user_id = ? AND track_id = ?
                 GROUP BY user_id, track_id
                 ON CONFLICT(user_id, track_id) DO UPDATE SET
                     play_count = MAX(track_statistics.play_count, excluded.play_count),
                     total_time_listened = MAX(track_statistics.total_time_listened, excluded.total_time_listened),
                     completion_count = MAX(track_statistics.completion_count, excluded.completion_count),
                     skip_count = MAX(track_statistics.skip_count, excluded.skip_count),
                     last_played_at = MAX(COALESCE(track_statistics.last_played_at, 0), COALESCE(excluded.last_played_at, 0))"
            )
            .bind(stat_user_id)
            .bind(&tds.track_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to sync track statistics from device rows: {}", e)))?;
        }

        // 5. User stats (yearly archive)
        if let Some(ref us_val) = payload.user_stats {
            let items: Vec<&serde_json::Value> = if let Some(arr) = us_val.as_array() {
                arr.iter().collect()
            } else if us_val.is_object() {
                vec![us_val]
            } else {
                Vec::new()
            };

            for item in items {
                let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("stat_1");
                let year = item.get("year").and_then(|v| v.as_i64()).unwrap_or(2026);
                let total_seconds = item
                    .get("total_seconds")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let top_songs = item
                    .get("top_songs_json")
                    .and_then(|v| v.as_str())
                    .unwrap_or("[]");
                let top_artists = item
                    .get("top_artists_json")
                    .and_then(|v| v.as_str())
                    .unwrap_or("[]");
                let archived_at = item
                    .get("updated_at")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(now);

                let _ = sqlx::query(
                    "INSERT INTO yearly_stats_archive (id, user_id, year, total_seconds, top_songs_json, top_artists_json, archived_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?)
                     ON CONFLICT(user_id, year) DO UPDATE SET
                         total_seconds = MAX(yearly_stats_archive.total_seconds, excluded.total_seconds),
                         archived_at = excluded.archived_at"
                )
                .bind(id)
                .bind(user_id)
                .bind(year)
                .bind(total_seconds)
                .bind(top_songs)
                .bind(top_artists)
                .bind(archived_at)
                .execute(pool)
                .await;
            }
        }

        // 6. User settings
        if let Some(ref uset_val) = payload.user_settings {
            let items: Vec<&serde_json::Value> = if let Some(arr) = uset_val.as_array() {
                arr.iter().collect()
            } else if uset_val.is_object() {
                vec![uset_val]
            } else {
                Vec::new()
            };

            for item in items {
                if let (Some(k), Some(v)) = (
                    item.get("key").and_then(|v| v.as_str()),
                    item.get("value").and_then(|v| v.as_str()),
                ) {
                    let _ = sqlx::query(
                        "INSERT INTO application_settings (key, value, updated_at)
                         VALUES (?, ?, ?)
                         ON CONFLICT(key) DO UPDATE SET
                             value = excluded.value,
                             updated_at = excluded.updated_at",
                    )
                    .bind(k)
                    .bind(v)
                    .bind(now)
                    .execute(pool)
                    .await;
                }
            }
        }

        Ok((inserted_playlists, inserted_memberships))
    }
}
