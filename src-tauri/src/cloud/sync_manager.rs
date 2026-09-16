use super::models::*;
use crate::core::error::{AppError, AppResult};
use chrono::Utc;
use sqlx::SqlitePool;

#[derive(Debug, Clone, Default)]
pub struct SyncReport {
    pub cloud_playlists_returned: usize,
    pub cloud_playlist_songs_returned: usize,
    pub local_playlists_before: usize,
    pub playlists_inserted_updated: usize,
    pub playlist_memberships_inserted_updated: usize,
    pub pending_local_changes_uploaded: usize,
    pub local_playlists_after: usize,
    pub synced_at: i64,
}

pub struct SyncManager;

impl SyncManager {
    pub async fn get_active_session(pool: &SqlitePool) -> AppResult<Option<CloudSessionMetadata>> {
        let record: Option<CloudSessionMetadata> = sqlx::query_as(
            "SELECT user_id, username, session_id, device_id, device_name,
                    idle_expires_at, absolute_expires_at, last_cloud_validation_at,
                    worker_url, synced_at, authenticated_before, created_at
             FROM cloud_sessions ORDER BY created_at DESC LIMIT 1"
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to fetch cloud session: {}", e)))?;

        Ok(record)
    }

    pub async fn save_session(pool: &SqlitePool, metadata: &CloudSessionMetadata) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO cloud_sessions (
                user_id, username, session_id, device_id, device_name,
                idle_expires_at, absolute_expires_at, last_cloud_validation_at,
                worker_url, synced_at, authenticated_before, created_at
             )
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(user_id) DO UPDATE SET
                 username = excluded.username,
                 session_id = excluded.session_id,
                 device_id = excluded.device_id,
                 device_name = excluded.device_name,
                 idle_expires_at = excluded.idle_expires_at,
                 absolute_expires_at = excluded.absolute_expires_at,
                 last_cloud_validation_at = excluded.last_cloud_validation_at,
                 worker_url = excluded.worker_url,
                 synced_at = excluded.synced_at,
                 authenticated_before = excluded.authenticated_before"
        )
        .bind(&metadata.user_id)
        .bind(&metadata.username)
        .bind(&metadata.session_id)
        .bind(&metadata.device_id)
        .bind(&metadata.device_name)
        .bind(metadata.idle_expires_at)
        .bind(metadata.absolute_expires_at)
        .bind(metadata.last_cloud_validation_at)
        .bind(&metadata.worker_url)
        .bind(metadata.synced_at)
        .bind(metadata.authenticated_before)
        .bind(metadata.created_at)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to save cloud session: {}", e)))?;

        Ok(())
    }

    pub async fn update_validation_timestamp(pool: &SqlitePool, user_id: &str, timestamp: i64) -> AppResult<()> {
        sqlx::query("UPDATE cloud_sessions SET last_cloud_validation_at = ? WHERE user_id = ?")
            .bind(timestamp)
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to update validation timestamp: {}", e)))?;

        Ok(())
    }

    pub async fn update_session_expiry(
        pool: &SqlitePool,
        user_id: &str,
        idle_expires_at: i64,
        absolute_expires_at: i64,
    ) -> AppResult<()> {
        sqlx::query("UPDATE cloud_sessions SET idle_expires_at = ?, absolute_expires_at = ? WHERE user_id = ?")
            .bind(idle_expires_at)
            .bind(absolute_expires_at)
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to update session expiry: {}", e)))?;

        Ok(())
    }

    pub async fn delete_session(pool: &SqlitePool, user_id: &str) -> AppResult<()> {
        let _ = super::credentials::delete_session_token(user_id);
        sqlx::query("DELETE FROM cloud_sessions WHERE user_id = ?")
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete cloud session: {}", e)))?;

        Ok(())
    }

    pub async fn clear_all_sessions(pool: &SqlitePool) -> AppResult<()> {
        if let Ok(Some(s)) = Self::get_active_session(pool).await {
            let _ = super::credentials::delete_session_token(&s.user_id);
        }
        sqlx::query("DELETE FROM cloud_sessions")
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to clear cloud sessions: {}", e)))?;

        Ok(())
    }

    /// Clears session token and local session metadata on logout or revocation.
    /// In accordance with the local-first security architecture, local songs,
    /// playlists, statistics, and pending sync data are NOT deleted.
    pub async fn clear_user_local_data(pool: &SqlitePool, user_id: &str) -> AppResult<()> {
        let _ = super::credentials::delete_session_token(user_id);
        let _ = sqlx::query("DELETE FROM cloud_sessions WHERE user_id = ?")
            .bind(user_id)
            .execute(pool)
            .await;

        Ok(())
    }

    pub async fn record_tombstone(
        pool: &SqlitePool,
        user_id: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> AppResult<()> {
        let id = format!("{}:{}:{}", user_id, entity_type, entity_id);
        let now = Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO sync_tombstones (id, user_id, entity_type, entity_id, deleted_at)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET deleted_at = excluded.deleted_at",
        )
        .bind(&id)
        .bind(user_id)
        .bind(entity_type)
        .bind(entity_id)
        .bind(now)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to record sync tombstone: {}", e)))?;

        Ok(())
    }

    pub async fn get_tombstones(
        pool: &SqlitePool,
        user_id: &str,
    ) -> AppResult<Vec<(String, String, String)>> {
        let rows = sqlx::query_as::<_, (String, String, String)>(
            "SELECT id, entity_type, entity_id FROM sync_tombstones WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        Ok(rows)
    }

    pub async fn clear_tombstones(pool: &SqlitePool, user_id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM sync_tombstones WHERE user_id = ?")
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to clear sync tombstones: {}", e)))?;

        Ok(())
    }

    /// Performs deterministic two-way synchronization with Cloudflare D1
    /// Flow: PULL cloud state first -> RECONCILE locally -> PUSH legitimate changes
    pub async fn sync_with_cloud(
        pool: &SqlitePool,
        client: &super::client::CloudClient,
        worker_url: &str,
        user_id: &str,
        token: &str,
    ) -> AppResult<SyncReport> {
        tracing::info!("SYNC START: user_id = {}", user_id);

        let local_before: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM playlists WHERE user_id = ? AND is_smart_mix = 0",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap_or((0,));
        let local_playlists_before = local_before.0 as usize;

        // Step 1: PULL cloud state FIRST.
        // Safeguard: If the pull fails, abort immediately to preserve local data and prevent empty-state overwrite.
        let remote_data = client.pull_sync(worker_url, token).await?;
        let cloud_playlists_returned = remote_data.playlists.len();
        let cloud_playlist_songs_returned = remote_data.playlist_songs.len();

        tracing::info!("cloud playlists returned: {}", cloud_playlists_returned);
        tracing::info!("cloud playlist_songs returned: {}", cloud_playlist_songs_returned);
        tracing::info!("local playlists before sync: {}", local_playlists_before);

        // Step 2: Reconcile local SQLite with remote cloud data
        let (playlists_inserted_updated, playlist_memberships_inserted_updated) =
            Self::apply_remote_sync_payload(pool, user_id, &remote_data).await?;

        tracing::info!("playlists inserted/updated locally: {}", playlists_inserted_updated);
        tracing::info!("playlist memberships inserted/updated: {}", playlist_memberships_inserted_updated);

        // Step 3: Prepare local changes (including any local playlists and pending tombstones)
        let local_payload = Self::prepare_local_sync_payload(pool, user_id).await?;
        let pending_local_changes_uploaded =
            local_payload.playlists.len() + local_payload.deleted_playlists.len() + local_payload.deleted_playlist_songs.len();

        // Step 4: PUSH legitimate local changes to cloud
        let synced_at = client.push_sync(worker_url, token, &local_payload).await?;
        tracing::info!("pending local changes uploaded: {}", pending_local_changes_uploaded);

        // Step 5: Clear tombstones that have now been pushed
        let _ = Self::clear_tombstones(pool, user_id).await;

        // Step 6: Update session synced_at timestamp
        Self::update_session_synced_at(pool, user_id, synced_at).await?;

        let local_after: (i64,) = sqlx::query_as(
            "SELECT count(*) FROM playlists WHERE user_id = ? AND is_smart_mix = 0",
        )
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap_or((0,));
        let local_playlists_after = local_after.0 as usize;

        tracing::info!("local playlists after sync: {}", local_playlists_after);
        tracing::info!("frontend playlist refresh triggered");

        Ok(SyncReport {
            cloud_playlists_returned,
            cloud_playlist_songs_returned,
            local_playlists_before,
            playlists_inserted_updated,
            playlist_memberships_inserted_updated,
            pending_local_changes_uploaded,
            local_playlists_after,
            synced_at,
        })
    }

    pub async fn update_session_synced_at(
        pool: &SqlitePool,
        user_id: &str,
        synced_at: i64,
    ) -> AppResult<()> {
        sqlx::query("UPDATE cloud_sessions SET synced_at = ? WHERE user_id = ?")
            .bind(synced_at)
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to update synced_at: {}", e)))?;

        Ok(())
    }

    pub async fn prepare_local_sync_payload(
        pool: &SqlitePool,
        user_id: &str,
    ) -> AppResult<SyncPayload> {
        let now = Utc::now().timestamp();

        // 1. Playlists: user-owned custom playlists (non-smart mixes)
        let playlist_rows: Vec<(String, String, Option<String>, i64, Option<String>, i64, i64)> =
            sqlx::query_as(
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
            "SELECT track_id FROM track_statistics WHERE user_id = ? AND (manual_like != 0 OR play_count > 0)",
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
            let ext_row: Option<(String, String, String, String, Option<String>, Option<String>, Option<f64>, Option<String>, Option<String>, i64)> = sqlx::query_as(
                "SELECT id, provider, provider_id, title, artist, album, duration_secs, cover_art_url, preview_url, created_at
                 FROM external_tracks WHERE id = ?"
            )
            .bind(s_id)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

            if let Some(ext) = ext_row {
                songs.push(CloudSong {
                    id: ext.0,
                    user_id: user_id.to_string(),
                    title: ext.3,
                    artist: ext.4,
                    album: ext.5,
                    duration_secs: ext.6.unwrap_or(0.0),
                    provider: Some(ext.1),
                    provider_id: Some(ext.2),
                    cover_art_url: ext.7,
                    preview_url: ext.8,
                    created_at: ext.9,
                    updated_at: now,
                });
            } else {
                // Check local tracks table
                let trk_row: Option<(String, String, Option<String>, Option<String>, f64, i64, i64)> = sqlx::query_as(
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
            let stat_rows: Vec<(String, i64, f64, i64, i64, Option<i64>, i64)> = sqlx::query_as(
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
            user_stats,
            user_settings,
            deleted_playlists,
            deleted_playlist_songs,
        })
    }

    pub async fn apply_remote_sync_payload(
        pool: &SqlitePool,
        user_id: &str,
        payload: &SyncPayload,
    ) -> AppResult<(usize, usize)> {
        let now = Utc::now().timestamp();

        // Retrieve local tombstones so explicitly deleted items are never re-inserted
        let tombstones = Self::get_tombstones(pool, user_id).await.unwrap_or_default();
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
            let provider = song.provider.as_deref().unwrap_or("cloud");
            let provider_id = song.provider_id.as_deref().unwrap_or(&song.id);

            let _ = sqlx::query(
                "INSERT INTO external_tracks (id, provider, provider_id, title, artist, album, duration_secs, cover_art_url, preview_url, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET
                     title = excluded.title,
                     artist = excluded.artist,
                     album = excluded.album,
                     duration_secs = excluded.duration_secs,
                     cover_art_url = coalesce(excluded.cover_art_url, external_tracks.cover_art_url),
                     preview_url = coalesce(excluded.preview_url, external_tracks.preview_url)"
            )
            .bind(&song.id)
            .bind(provider)
            .bind(provider_id)
            .bind(&song.title)
            .bind(song.artist.as_deref().unwrap_or("Unknown Artist"))
            .bind(song.album.as_deref().unwrap_or(""))
            .bind(song.duration_secs)
            .bind(&song.cover_art_url)
            .bind(&song.preview_url)
            .bind(song.created_at)
            .execute(pool)
            .await;

            let _ = crate::recommendations::mixes::ensure_online_track(
                pool,
                &song.id,
                &song.title,
                song.artist.as_deref(),
                song.album.as_deref(),
                Some(song.duration_secs),
                song.cover_art_url.as_deref(),
                song.preview_url.as_deref(),
            )
            .await;
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
                     added_at = excluded.added_at"
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
            let stat_user_id = if ss.user_id.is_empty() { user_id } else { &ss.user_id };
            let _ = sqlx::query(
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
            .await;
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
                let total_seconds = item.get("total_seconds").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let top_songs = item.get("top_songs_json").and_then(|v| v.as_str()).unwrap_or("[]");
                let top_artists = item.get("top_artists_json").and_then(|v| v.as_str()).unwrap_or("[]");
                let archived_at = item.get("updated_at").and_then(|v| v.as_i64()).unwrap_or(now);

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
                             updated_at = excluded.updated_at"
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
