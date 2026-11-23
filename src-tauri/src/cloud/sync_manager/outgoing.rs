//! Building the local changes to push: playlists, likes, history and stats.
use super::*;

impl SyncManager {
    pub async fn prepare_local_sync_payload(
        pool: &SqlitePool,
        user_id: &str,
    ) -> AppResult<SyncPayload> {
        let now = Utc::now().timestamp();

        // 1. Playlists: user-owned custom playlists (non-smart mixes)
        type PlaylistRow = (String, String, Option<String>, i64, Option<String>, i64, i64);
        let playlist_rows: Vec<PlaylistRow> = sqlx::query_as(
            "SELECT id, name, description, is_smart_mix, mix_type, created_at, updated_at
                 FROM playlists WHERE user_id = ? AND is_smart_mix = 0",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to load playlists: {}", e)))?;

        let playlists: Vec<CloudPlaylist> = playlist_rows
            .into_iter()
            .map(|r| CloudPlaylist {
                id: r.0,
                user_id: user_id.to_string(),
                name: r.1,
                description: r.2,
                is_smart_mix: r.3,
                mix_type: r.4,
                created_at: r.5,
                updated_at: r.6,
            })
            .collect();

        // 2. Playlist tracks
        let pt_rows: Vec<(String, String, i64, i64)> = sqlx::query_as(
            "SELECT pt.playlist_id, pt.track_id, pt.position, pt.added_at
             FROM playlist_tracks pt
             JOIN playlists p ON pt.playlist_id = p.id
             WHERE p.user_id = ? AND p.is_smart_mix = 0",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to load playlist_tracks: {}", e)))?;

        let mut playlist_songs: Vec<CloudPlaylistSong> = Vec::new();
        let mut song_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

        for r in pt_rows {
            song_ids.insert(r.1.clone());
            playlist_songs.push(CloudPlaylistSong {
                id: format!("{}:{}", r.0, r.1),
                user_id: user_id.to_string(),
                playlist_id: r.0,
                song_id: r.1,
                position: r.2,
                added_at: r.3,
            });
        }

        // Also collect song IDs that have user feedback (liked or disliked) or play statistics
        let rated_rows: Vec<(String,)> = sqlx::query_as(
            "SELECT track_id FROM track_statistics WHERE user_id = ? AND (manual_like != 0 OR play_count > 0 OR total_time_listened > 0)",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        for r in rated_rows {
            song_ids.insert(r.0);
        }

        // 3. Collect songs info for songs referenced in playlists or user ratings
        let mut songs: Vec<CloudSong> = Vec::new();
        for s_id in &song_ids {
            // Check external_tracks first
            type ExtRow = (String, String, String, String, Option<String>, Option<String>, Option<f64>, Option<String>, Option<String>, i64);
            let ext_row: Option<ExtRow> = sqlx::query_as(
                "SELECT id, provider, provider_id, title, artist, album, duration_secs, cover_art_url, preview_url, created_at
                 FROM external_tracks WHERE id = ?"
            )
            .bind(s_id)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

            if let Some(ext) = ext_row {
                let source: Option<(String, String, i64)> = sqlx::query_as(
                    "SELECT provider_id, metadata_json, linked_at FROM online_track_sources WHERE track_id=?",
                )
                .bind(s_id)
                .fetch_optional(pool)
                .await?;
                let online_source = source
                    .map(|(provider_id, metadata_json, linked_at)| {
                        let source = CloudOnlineSource {
                            provider_id,
                            metadata: serde_json::from_str(&metadata_json).map_err(|e| {
                                AppError::Database(format!("Invalid saved recording metadata: {e}"))
                            })?,
                            linked_at,
                        };
                        source.validate()?;
                        Ok::<_, AppError>(source)
                    })
                    .transpose()?;
                songs.push(CloudSong {
                    id: ext.0,
                    user_id: user_id.to_string(),
                    title: ext.3,
                    artist: ext.4,
                    album: ext.5,
                    duration_secs: ext.6.unwrap_or(0.0),
                    provider: Some(ext.1),
                    provider_id: Some(ext.2),
                    online_source,
                    cover_art_url: ext.7,
                    preview_url: ext.8,
                    created_at: ext.9,
                    updated_at: now,
                });
            } else {
                // Check local tracks table
                type TrkRow = (String, String, Option<String>, Option<String>, f64, i64, i64);
                let trk_row: Option<TrkRow> = sqlx::query_as(
                    "SELECT t.id, t.title, a.name, al.title, t.duration_secs, t.created_at, t.updated_at
                     FROM tracks t
                     LEFT JOIN artists a ON t.artist_id = a.id
                     LEFT JOIN albums al ON t.album_id = al.id
                     WHERE t.id = ?"
                )
                .bind(s_id)
                .fetch_optional(pool)
                .await
                .unwrap_or(None);

                if let Some(trk) = trk_row {
                    songs.push(CloudSong {
                        id: trk.0,
                        user_id: user_id.to_string(),
                        title: trk.1,
                        artist: trk.2,
                        album: trk.3,
                        duration_secs: trk.4,
                        provider: Some("local".to_string()),
                        provider_id: None,
                        online_source: None,
                        cover_art_url: None,
                        preview_url: None,
                        created_at: trk.5,
                        updated_at: trk.6,
                    });
                }
            }
        }

        // 4. Song stats (including liked/disliked songs: manual_like = 1 or -1)
        let mut song_stats: Vec<CloudSongStat> = Vec::new();
        if !song_ids.is_empty() {
            type StatRow = (String, i64, f64, i64, i64, Option<i64>, i64);
            let stat_rows: Vec<StatRow> = sqlx::query_as(
                "SELECT track_id, play_count, total_time_listened, completion_count, skip_count, last_played_at, manual_like
                 FROM track_statistics WHERE user_id = ?"
            )
            .bind(user_id)
            .fetch_all(pool)
            .await
            .unwrap_or_default();

            for s in stat_rows {
                if song_ids.contains(&s.0) {
                    song_stats.push(CloudSongStat {
                        id: format!("{}:{}", user_id, s.0),
                        user_id: user_id.to_string(),
                        song_id: s.0,
                        play_count: s.1,
                        total_time_listened: s.2,
                        completion_count: s.3,
                        skip_count: s.4,
                        last_played_at: s.5,
                        manual_like: s.6,
                        updated_at: now,
                    });
                }
            }
        }

        // 4.5. Daily stats
        let daily_rows: Vec<(String, String, f64, i64, i64, i64, i64)> = sqlx::query_as(
            "SELECT device_id, stat_date, listening_seconds, play_count, completion_count, skip_count, updated_at
             FROM daily_user_stats WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        let daily_stats: Vec<CloudDailyStat> = daily_rows
            .into_iter()
            .map(|r| CloudDailyStat {
                user_id: user_id.to_string(),
                device_id: r.0,
                stat_date: r.1,
                listening_seconds: r.2,
                play_count: r.3,
                completion_count: r.4,
                skip_count: r.5,
                updated_at: r.6,
            })
            .collect();

        // 4.6. Track device stats
        type TdsRow = (String, String, i64, f64, i64, i64, Option<i64>, i64);
        let tds_rows: Vec<TdsRow> = sqlx::query_as(
            "SELECT device_id, track_id, play_count, total_time_listened, completion_count, skip_count, last_played_at, updated_at
             FROM track_device_statistics WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        let track_device_stats: Vec<CloudTrackDeviceStat> = tds_rows
            .into_iter()
            .map(|r| CloudTrackDeviceStat {
                user_id: user_id.to_string(),
                device_id: r.0,
                track_id: r.1,
                play_count: r.2,
                total_seconds: r.3,
                completion_count: r.4,
                skip_count: r.5,
                last_played_at: r.6,
                updated_at: r.7,
            })
            .collect();

        // 5. User stats (yearly archive)
        let yearly_rows: Vec<(String, i64, f64, String, String, i64)> = sqlx::query_as(
            "SELECT id, year, total_seconds, top_songs_json, top_artists_json, archived_at
             FROM yearly_stats_archive WHERE user_id = ? OR user_id = 'default'",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        let user_stats_vec: Vec<serde_json::Value> = yearly_rows
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.0,
                    "user_id": user_id,
                    "year": r.1,
                    "total_seconds": r.2,
                    "top_songs_json": r.3,
                    "top_artists_json": r.4,
                    "updated_at": r.5,
                })
            })
            .collect();

        let user_stats = if !user_stats_vec.is_empty() {
            Some(serde_json::Value::Array(user_stats_vec))
        } else {
            None
        };

        // 6. User settings
        let setting_rows: Vec<(String, String, i64)> = sqlx::query_as(
            "SELECT key, value, updated_at FROM application_settings WHERE key IN ('default_music_dir', 'audio_volume', 'repeat_mode', 'shuffle_enabled')",
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        let user_settings_vec: Vec<serde_json::Value> = setting_rows
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "id": format!("{}:{}", user_id, r.0),
                    "user_id": user_id,
                    "key": r.0,
                    "value": r.1,
                    "updated_at": r.2,
                })
            })
            .collect();

        let user_settings = if !user_settings_vec.is_empty() {
            Some(serde_json::Value::Array(user_settings_vec))
        } else {
            None
        };

        // 7. Sync Tombstones (explicit deletions)
        let mut deleted_playlists = Vec::new();
        let mut deleted_playlist_songs = Vec::new();
        if let Ok(tombstones) = Self::get_tombstones(pool, user_id).await {
            for (_, entity_type, entity_id) in tombstones {
                if entity_type == "playlist" {
                    deleted_playlists.push(entity_id);
                } else if entity_type == "playlist_song" {
                    if let Some((p, s)) = entity_id.split_once(':') {
                        deleted_playlist_songs.push(CloudPlaylistSongRef {
                            playlist_id: p.to_string(),
                            song_id: s.to_string(),
                        });
                    }
                }
            }
        }

        Ok(SyncPayload {
            songs,
            playlists,
            playlist_songs,
            song_stats,
            daily_stats,
            track_device_stats,
            user_stats,
            user_settings,
            deleted_playlists,
            deleted_playlist_songs,
        })
    }
}
