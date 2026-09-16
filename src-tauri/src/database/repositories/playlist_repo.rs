use crate::core::error::{AppError, AppResult};
use crate::database::models::PlaylistRecord;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PlaylistTrackDetail {
    pub id: String,
    pub file_path: String,
    pub file_size: i64,
    pub modified_timestamp: i64,
    pub title: String,
    pub artist_name: Option<String>,
    pub album_title: Option<String>,
    pub genre_name: Option<String>,
    pub track_number: Option<i64>,
    pub disc_number: Option<i64>,
    pub year: Option<i64>,
    pub duration_secs: f64,
    pub bitrate: Option<i64>,
    pub sample_rate: Option<i64>,
    pub format: String,
    pub has_cover_art: i64,
    pub cover_art_url: Option<String>,
    pub preview_url: Option<String>,
    pub created_at: i64,
}

#[async_trait]
pub trait PlaylistRepository: Send + Sync {
    async fn create_playlist(&self, playlist: &PlaylistRecord) -> AppResult<()>;
    async fn get_playlist(&self, id: &str) -> AppResult<Option<PlaylistRecord>>;
    async fn find_by_name(&self, name: &str) -> AppResult<Option<PlaylistRecord>>;
    async fn get_all_playlists(&self) -> AppResult<Vec<PlaylistRecord>>;
    async fn get_smart_mixes(&self) -> AppResult<Vec<PlaylistRecord>>;
    async fn get_smart_mixes_for_user(&self, user_id: &str) -> AppResult<Vec<PlaylistRecord>>;
    async fn get_smart_mix_by_type(&self, mix_type: &str) -> AppResult<Option<PlaylistRecord>>;
    async fn find_smart_mix(&self, mix_type: &str, name: &str) -> AppResult<Option<PlaylistRecord>>;
    async fn find_smart_mix_for_user(&self, user_id: &str, mix_type: &str, name: &str) -> AppResult<Option<PlaylistRecord>>;
    async fn delete_duplicate_smart_mixes(&self, mix_type: &str, name: &str, keep_id: &str) -> AppResult<()>;
    async fn delete_duplicate_smart_mixes_for_user(&self, user_id: &str, mix_type: &str, name: &str, keep_id: &str) -> AppResult<()>;
    async fn delete_playlist(&self, id: &str) -> AppResult<()>;
    async fn rename_playlist(&self, id: &str, name: &str) -> AppResult<()>;
    async fn ensure_liked_songs_playlist(&self, user_id: &str) -> AppResult<String>;
    async fn add_track(&self, playlist_id: &str, track_id: &str, position: Option<i64>) -> AppResult<()>;
    async fn remove_track(&self, playlist_id: &str, track_id: &str) -> AppResult<()>;
    async fn set_tracks(&self, playlist_id: &str, track_ids: &[String]) -> AppResult<()>;
    async fn get_playlist_tracks(&self, playlist_id: &str) -> AppResult<Vec<PlaylistTrackDetail>>;
    async fn get_track_count(&self, playlist_id: &str) -> AppResult<i64>;
    async fn get_track_playlist_memberships(&self) -> AppResult<std::collections::HashMap<String, Vec<String>>>;
    async fn create_playlist_with_user(&self, playlist: &PlaylistRecord, user_id: &str) -> AppResult<()>;
    async fn get_user_playlists(&self, user_id: &str) -> AppResult<Vec<PlaylistRecord>>;
    async fn find_by_name_and_user(&self, name: &str, user_id: &str) -> AppResult<Option<PlaylistRecord>>;
    async fn get_track_playlist_memberships_for_user(&self, user_id: &str) -> AppResult<std::collections::HashMap<String, Vec<String>>>;
    async fn delete_all_user_playlists(&self, user_id: Option<&str>) -> AppResult<()>;
    async fn reassign_orphan_playlists_to_user(&self, user_id: &str) -> AppResult<()>;
}

#[derive(Clone)]
pub struct SqlitePlaylistRepository {
    pool: SqlitePool,
}

impl SqlitePlaylistRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PlaylistRepository for SqlitePlaylistRepository {
    async fn create_playlist(&self, playlist: &PlaylistRecord) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO playlists (
                id, name, description, is_smart_mix, mix_type,
                generation_reason, expires_at, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                description = excluded.description,
                mix_type = excluded.mix_type,
                generation_reason = excluded.generation_reason,
                expires_at = excluded.expires_at,
                updated_at = excluded.updated_at"
        )
        .bind(&playlist.id)
        .bind(&playlist.name)
        .bind(&playlist.description)
        .bind(playlist.is_smart_mix)
        .bind(&playlist.mix_type)
        .bind(&playlist.generation_reason)
        .bind(playlist.expires_at)
        .bind(playlist.created_at)
        .bind(playlist.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_playlist(&self, id: &str) -> AppResult<Option<PlaylistRecord>> {
        let playlist = sqlx::query_as::<_, PlaylistRecord>(
            "SELECT id, name, description, is_smart_mix, mix_type,
                    generation_reason, expires_at, created_at, updated_at
             FROM playlists WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(playlist)
    }

    async fn find_by_name(&self, name: &str) -> AppResult<Option<PlaylistRecord>> {
        let playlist = sqlx::query_as::<_, PlaylistRecord>(
            "SELECT id, name, description, is_smart_mix, mix_type,
                    generation_reason, expires_at, created_at, updated_at
             FROM playlists WHERE name = ? COLLATE BINARY AND is_smart_mix = 0
             LIMIT 1"
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(playlist)
    }

    async fn get_all_playlists(&self) -> AppResult<Vec<PlaylistRecord>> {
        let playlists = sqlx::query_as::<_, PlaylistRecord>(
            "SELECT id, name, description, is_smart_mix, mix_type,
                    generation_reason, expires_at, created_at, updated_at
             FROM playlists
             ORDER BY is_smart_mix DESC, updated_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(playlists)
    }

    async fn get_smart_mixes(&self) -> AppResult<Vec<PlaylistRecord>> {
        self.get_smart_mixes_for_user("default").await
    }

    async fn get_smart_mixes_for_user(&self, user_id: &str) -> AppResult<Vec<PlaylistRecord>> {
        let mixes = sqlx::query_as::<_, PlaylistRecord>(
            "SELECT id, name, description, is_smart_mix, mix_type,
                    generation_reason, expires_at, created_at, updated_at
             FROM playlists
             WHERE is_smart_mix = 1 AND user_id = ?
             ORDER BY updated_at DESC"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(mixes)
    }

    async fn get_smart_mix_by_type(&self, mix_type: &str) -> AppResult<Option<PlaylistRecord>> {
        let mix = sqlx::query_as::<_, PlaylistRecord>(
            "SELECT id, name, description, is_smart_mix, mix_type,
                    generation_reason, expires_at, created_at, updated_at
             FROM playlists
             WHERE is_smart_mix = 1 AND mix_type = ? COLLATE NOCASE
             ORDER BY updated_at DESC
             LIMIT 1"
        )
        .bind(mix_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(mix)
    }

    async fn find_smart_mix(&self, mix_type: &str, name: &str) -> AppResult<Option<PlaylistRecord>> {
        self.find_smart_mix_for_user("default", mix_type, name).await
    }

    async fn find_smart_mix_for_user(&self, user_id: &str, mix_type: &str, name: &str) -> AppResult<Option<PlaylistRecord>> {
        let mix = sqlx::query_as::<_, PlaylistRecord>(
            "SELECT id, name, description, is_smart_mix, mix_type,
                    generation_reason, expires_at, created_at, updated_at
             FROM playlists
             WHERE is_smart_mix = 1 AND user_id = ? AND (mix_type = ? COLLATE NOCASE OR name = ? COLLATE NOCASE)
             ORDER BY updated_at DESC
             LIMIT 1"
        )
        .bind(user_id)
        .bind(mix_type)
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(mix)
    }

    async fn delete_duplicate_smart_mixes(&self, mix_type: &str, name: &str, keep_id: &str) -> AppResult<()> {
        self.delete_duplicate_smart_mixes_for_user("default", mix_type, name, keep_id).await
    }

    async fn delete_duplicate_smart_mixes_for_user(&self, user_id: &str, mix_type: &str, name: &str, keep_id: &str) -> AppResult<()> {
        let _ = sqlx::query(
            "DELETE FROM playlist_tracks WHERE playlist_id IN (
                SELECT id FROM playlists
                WHERE is_smart_mix = 1 AND user_id = ? AND (mix_type = ? COLLATE NOCASE OR name = ? COLLATE NOCASE) AND id != ?
            )"
        )
        .bind(user_id)
        .bind(mix_type)
        .bind(name)
        .bind(keep_id)
        .execute(&self.pool)
        .await;

        let _ = sqlx::query(
            "DELETE FROM playlists
             WHERE is_smart_mix = 1 AND user_id = ? AND (mix_type = ? COLLATE NOCASE OR name = ? COLLATE NOCASE) AND id != ?"
        )
        .bind(user_id)
        .bind(mix_type)
        .bind(name)
        .bind(keep_id)
        .execute(&self.pool)
        .await;

        Ok(())
    }

    async fn delete_playlist(&self, id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM playlists WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn rename_playlist(&self, id: &str, name: &str) -> AppResult<()> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query("UPDATE playlists SET name = ?, updated_at = ? WHERE id = ?")
            .bind(name)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    async fn ensure_liked_songs_playlist(&self, user_id: &str) -> AppResult<String> {
        // Check if it already exists
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT id FROM playlists WHERE name = 'Liked Songs' AND is_smart_mix = 0 AND user_id = ? LIMIT 1"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        if let Some(id) = existing {
            return Ok(id);
        }

        // Create it
        let now = chrono::Utc::now().timestamp();
        let id = format!("pl_liked_{}", user_id);
        sqlx::query(
            "INSERT OR IGNORE INTO playlists (id, user_id, name, description, is_smart_mix, created_at, updated_at)
             VALUES (?, ?, 'Liked Songs', 'Your liked tracks', 0, ?, ?)"
        )
        .bind(&id)
        .bind(user_id)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(id)
    }

    async fn add_track(&self, playlist_id: &str, track_id: &str, position: Option<i64>) -> AppResult<()> {
        let pos = match position {
            Some(p) => p,
            None => {
                let max_pos: Option<i64> = sqlx::query_scalar(
                    "SELECT MAX(position) FROM playlist_tracks WHERE playlist_id = ?"
                )
                .bind(playlist_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;

                max_pos.map_or(0, |m| m + 1)
            }
        };

        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO playlist_tracks (playlist_id, track_id, position, added_at)
             VALUES (?, ?, ?, ?)"
        )
        .bind(playlist_id)
        .bind(track_id)
        .bind(pos)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn remove_track(&self, playlist_id: &str, track_id: &str) -> AppResult<()> {
        sqlx::query(
            "DELETE FROM playlist_tracks WHERE playlist_id = ? AND track_id = ?"
        )
        .bind(playlist_id)
        .bind(track_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn set_tracks(&self, playlist_id: &str, track_ids: &[String]) -> AppResult<()> {
        let mut tx = self.pool.begin().await.map_err(|e| AppError::Database(e.to_string()))?;

        sqlx::query("DELETE FROM playlist_tracks WHERE playlist_id = ?")
            .bind(playlist_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let now = chrono::Utc::now().timestamp();
        for (i, track_id) in track_ids.iter().enumerate() {
            sqlx::query(
                "INSERT INTO playlist_tracks (playlist_id, track_id, position, added_at)
                 VALUES (?, ?, ?, ?)"
            )
            .bind(playlist_id)
            .bind(track_id)
            .bind(i as i64)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        }

        tx.commit().await.map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    async fn get_playlist_tracks(&self, playlist_id: &str) -> AppResult<Vec<PlaylistTrackDetail>> {
        let tracks = sqlx::query_as::<_, PlaylistTrackDetail>(
            "SELECT t.id, t.file_path, t.file_size, t.modified_timestamp,
                    CASE WHEN t.title LIKE 'itunes:%' OR t.title LIKE 'online:%' THEN COALESCE(ext.title, t.title) ELSE t.title END as title,
                    COALESCE(a.name, ext.artist, 'Unknown Artist') as artist_name,
                    COALESCE(al.title, ext.album) as album_title,
                    COALESCE(g.name, ext.genre) as genre_name,
                    t.track_number, t.disc_number, t.year,
                    CASE WHEN t.duration_secs > 0.0 THEN t.duration_secs ELSE COALESCE(ext.duration_secs, 210.0) END as duration_secs,
                    t.bitrate, t.sample_rate, t.format, t.has_cover_art,
                    COALESCE(ext.cover_art_url, al.cover_art_path) as cover_art_url,
                    ext.preview_url,
                    t.created_at
             FROM tracks t
             JOIN playlist_tracks pt ON pt.track_id = t.id
             LEFT JOIN artists a ON t.artist_id = a.id
             LEFT JOIN albums al ON t.album_id = al.id
             LEFT JOIN genres g ON t.genre_id = g.id
             LEFT JOIN external_tracks ext ON ext.id = t.id
             WHERE pt.playlist_id = ?
             ORDER BY pt.position ASC"
        )
        .bind(playlist_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(tracks)
    }

    async fn get_track_count(&self, playlist_id: &str) -> AppResult<i64> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM playlist_tracks WHERE playlist_id = ?"
        )
        .bind(playlist_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        Ok(count)
    }

    async fn get_track_playlist_memberships(&self) -> AppResult<std::collections::HashMap<String, Vec<String>>> {
        let rows = sqlx::query_as::<_, (String, String)>(
            "SELECT pt.track_id, pt.playlist_id
             FROM playlist_tracks pt
             JOIN playlists p ON p.id = pt.playlist_id
             WHERE p.is_smart_mix = 0"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let mut map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        for (track_id, playlist_id) in rows {
            map.entry(track_id).or_default().push(playlist_id);
        }

        Ok(map)
    }

    async fn create_playlist_with_user(&self, playlist: &PlaylistRecord, user_id: &str) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO playlists (
                id, user_id, name, description, is_smart_mix, mix_type,
                generation_reason, expires_at, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                user_id = excluded.user_id,
                name = excluded.name,
                description = excluded.description,
                mix_type = excluded.mix_type,
                generation_reason = excluded.generation_reason,
                expires_at = excluded.expires_at,
                updated_at = excluded.updated_at"
        )
        .bind(&playlist.id)
        .bind(user_id)
        .bind(&playlist.name)
        .bind(&playlist.description)
        .bind(playlist.is_smart_mix)
        .bind(&playlist.mix_type)
        .bind(&playlist.generation_reason)
        .bind(playlist.expires_at)
        .bind(playlist.created_at)
        .bind(playlist.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_user_playlists(&self, user_id: &str) -> AppResult<Vec<PlaylistRecord>> {
        let playlists = sqlx::query_as::<_, PlaylistRecord>(
            "SELECT id, name, description, is_smart_mix, mix_type,
                    generation_reason, expires_at, created_at, updated_at
             FROM playlists
             WHERE user_id = ?
             ORDER BY is_smart_mix DESC, updated_at DESC"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(playlists)
    }

    async fn find_by_name_and_user(&self, name: &str, user_id: &str) -> AppResult<Option<PlaylistRecord>> {
        let playlist = sqlx::query_as::<_, PlaylistRecord>(
            "SELECT id, name, description, is_smart_mix, mix_type,
                    generation_reason, expires_at, created_at, updated_at
             FROM playlists
             WHERE name = ? COLLATE BINARY AND is_smart_mix = 0 AND user_id = ?
             LIMIT 1"
        )
        .bind(name)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(playlist)
    }

    async fn get_track_playlist_memberships_for_user(&self, user_id: &str) -> AppResult<std::collections::HashMap<String, Vec<String>>> {
        let rows = sqlx::query_as::<_, (String, String)>(
            "SELECT pt.track_id, pt.playlist_id
             FROM playlist_tracks pt
             JOIN playlists p ON p.id = pt.playlist_id
             WHERE p.is_smart_mix = 0 AND p.user_id = ?"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let mut map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        for (track_id, playlist_id) in rows {
            map.entry(track_id).or_default().push(playlist_id);
        }

        Ok(map)
    }

    async fn delete_all_user_playlists(&self, user_id: Option<&str>) -> AppResult<()> {
        if let Some(uid) = user_id {
            let _ = sqlx::query(
                "DELETE FROM playlist_tracks WHERE playlist_id IN (
                    SELECT id FROM playlists WHERE is_smart_mix = 0 AND user_id = ?
                )"
            )
            .bind(uid)
            .execute(&self.pool)
            .await;

            let _ = sqlx::query(
                "DELETE FROM playlists WHERE is_smart_mix = 0 AND user_id = ?"
            )
            .bind(uid)
            .execute(&self.pool)
            .await;
        } else {
            let _ = sqlx::query(
                "DELETE FROM playlist_tracks WHERE playlist_id IN (
                    SELECT id FROM playlists WHERE is_smart_mix = 0
                )"
            )
            .execute(&self.pool)
            .await;

            let _ = sqlx::query("DELETE FROM playlists WHERE is_smart_mix = 0")
                .execute(&self.pool)
                .await;
        }

        Ok(())
    }

    async fn reassign_orphan_playlists_to_user(&self, user_id: &str) -> AppResult<()> {
        let _ = sqlx::query(
            "UPDATE playlists SET user_id = ? WHERE is_smart_mix = 0 AND (user_id = 'default' OR user_id IS NULL)"
        )
        .bind(user_id)
        .execute(&self.pool)
        .await;

        Ok(())
    }
}
