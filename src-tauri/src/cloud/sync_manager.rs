use super::models::*;
use crate::core::error::{AppError, AppResult};
use chrono::Utc;
use sqlx::SqlitePool;

pub struct SyncManager;

impl SyncManager {
    pub async fn get_active_session(pool: &SqlitePool) -> AppResult<Option<CloudSessionMetadata>> {
        let record: Option<CloudSessionMetadata> = sqlx::query_as(
            "SELECT user_id, username, expires_at, worker_url, synced_at, created_at FROM cloud_sessions ORDER BY created_at DESC LIMIT 1"
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to fetch cloud session: {}", e)))?;

        Ok(record)
    }

    pub async fn save_session(pool: &SqlitePool, metadata: &CloudSessionMetadata) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO cloud_sessions (user_id, username, expires_at, worker_url, synced_at, created_at)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(user_id) DO UPDATE SET
                 username = excluded.username,
                 expires_at = excluded.expires_at,
                 worker_url = excluded.worker_url,
                 synced_at = excluded.synced_at"
        )
        .bind(&metadata.user_id)
        .bind(&metadata.username)
        .bind(metadata.expires_at)
        .bind(&metadata.worker_url)
        .bind(metadata.synced_at)
        .bind(metadata.created_at)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to save cloud session: {}", e)))?;

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

        // 1. Playlists
        let playlist_rows: Vec<(String, String, Option<String>, i64, Option<String>, i64, i64)> =
            sqlx::query_as(
                "SELECT id, name, description, is_smart_mix, mix_type, created_at, updated_at
                 FROM playlists WHERE user_id = ? OR user_id = 'default'",
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
             WHERE p.user_id = ? OR p.user_id = 'default'",
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

        // 3. Collect songs info for songs referenced in playlists
        let mut songs: Vec<CloudSong> = Vec::new();
        for s_id in &song_ids {
            // Check external_tracks first
            let ext_row: Option<(String, String, String, String, Option<String>, f64, Option<String>, i64)> = sqlx::query_as(
                "SELECT id, provider, provider_id, title, artist, duration_secs, cover_art_url, created_at
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
                    album: None,
                    duration_secs: ext.5,
                    provider: Some(ext.1),
                    provider_id: Some(ext.2),
                    cover_art_url: ext.6,
                    preview_url: None,
                    created_at: ext.7,
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

        // 4. Song stats
        let mut song_stats: Vec<CloudSongStat> = Vec::new();
        if !song_ids.is_empty() {
            let stat_rows: Vec<(String, i64, f64, i64, i64, Option<i64>, i64)> = sqlx::query_as(
                "SELECT track_id, play_count, total_time_listened, completion_count, skip_count, last_played_at, manual_like
                 FROM track_statistics"
            )
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

        let user_stats: Vec<CloudUserStat> = yearly_rows
            .into_iter()
            .map(|r| CloudUserStat {
                id: r.0,
                user_id: user_id.to_string(),
                year: r.1,
                month: 0,
                total_seconds: r.2,
                top_songs_json: r.3,
                top_artists_json: r.4,
                updated_at: r.5,
            })
            .collect();

        // 6. User settings
        let setting_rows: Vec<(String, String, i64)> = sqlx::query_as(
            "SELECT key, value, updated_at FROM application_settings WHERE key IN ('default_music_dir', 'audio_volume', 'repeat_mode', 'shuffle_enabled')",
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        let user_settings: Vec<CloudUserSetting> = setting_rows
            .into_iter()
            .map(|r| CloudUserSetting {
                id: format!("{}:{}", user_id, r.0),
                user_id: user_id.to_string(),
                key: r.0,
                value: r.1,
                updated_at: r.2,
            })
            .collect();

        Ok(SyncPayload {
            songs,
            playlists,
            playlist_songs,
            song_stats,
            user_stats,
            user_settings,
        })
    }

    pub async fn apply_remote_sync_payload(
        pool: &SqlitePool,
        user_id: &str,
        payload: &SyncPayload,
    ) -> AppResult<()> {
        let now = Utc::now().timestamp();

        // 1. Ingest remote songs into external_tracks if needed
        for song in &payload.songs {
            let provider = song.provider.as_deref().unwrap_or("cloud");
            let provider_id = song.provider_id.as_deref().unwrap_or(&song.id);

            let _ = sqlx::query(
                "INSERT INTO external_tracks (id, provider, provider_id, title, artist, album, duration_secs, cover_art_url, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET
                     title = excluded.title,
                     artist = excluded.artist,
                     cover_art_url = coalesce(excluded.cover_art_url, external_tracks.cover_art_url)"
            )
            .bind(&song.id)
            .bind(provider)
            .bind(provider_id)
            .bind(&song.title)
            .bind(song.artist.as_deref().unwrap_or("Unknown Artist"))
            .bind(song.album.as_deref().unwrap_or(""))
            .bind(song.duration_secs)
            .bind(&song.cover_art_url)
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
        for p in &payload.playlists {
            let _ = sqlx::query(
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
        }

        // 3. Playlist songs
        for ps in &payload.playlist_songs {
            let _ = sqlx::query(
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
        }

        // 4. Song stats
        for ss in &payload.song_stats {
            let _ = sqlx::query(
                "INSERT INTO track_statistics (track_id, play_count, total_time_listened, completion_count, skip_count, last_played_at, manual_like)
                 VALUES (?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(track_id) DO UPDATE SET
                     play_count = MAX(track_statistics.play_count, excluded.play_count),
                     total_time_listened = MAX(track_statistics.total_time_listened, excluded.total_time_listened),
                     completion_count = MAX(track_statistics.completion_count, excluded.completion_count),
                     skip_count = MAX(track_statistics.skip_count, excluded.skip_count),
                     manual_like = MAX(track_statistics.manual_like, excluded.manual_like)"
            )
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
        for us in &payload.user_stats {
            let _ = sqlx::query(
                "INSERT INTO yearly_stats_archive (id, user_id, year, total_seconds, top_songs_json, top_artists_json, archived_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(user_id, year) DO UPDATE SET
                     total_seconds = MAX(yearly_stats_archive.total_seconds, excluded.total_seconds),
                     archived_at = excluded.archived_at"
            )
            .bind(&us.id)
            .bind(user_id)
            .bind(us.year)
            .bind(us.total_seconds)
            .bind(&us.top_songs_json)
            .bind(&us.top_artists_json)
            .bind(us.updated_at)
            .execute(pool)
            .await;
        }

        // 6. User settings
        for uset in &payload.user_settings {
            let _ = sqlx::query(
                "INSERT INTO application_settings (key, value, updated_at)
                 VALUES (?, ?, ?)
                 ON CONFLICT(key) DO UPDATE SET
                     value = excluded.value,
                     updated_at = excluded.updated_at"
            )
            .bind(&uset.key)
            .bind(&uset.value)
            .bind(now)
            .execute(pool)
            .await;
        }

        Ok(())
    }
}
