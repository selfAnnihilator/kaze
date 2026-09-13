use crate::core::error::{AppError, AppResult};
use crate::database::models::{PlaylistRecord, TrackRecord};
use async_trait::async_trait;
use sqlx::SqlitePool;

#[async_trait]
pub trait PlaylistRepository: Send + Sync {
    async fn create_playlist(&self, playlist: &PlaylistRecord) -> AppResult<()>;
    async fn get_playlist(&self, id: &str) -> AppResult<Option<PlaylistRecord>>;
    async fn find_by_name(&self, name: &str) -> AppResult<Option<PlaylistRecord>>;
    async fn get_all_playlists(&self) -> AppResult<Vec<PlaylistRecord>>;
    async fn get_smart_mixes(&self) -> AppResult<Vec<PlaylistRecord>>;
    async fn get_smart_mix_by_type(&self, mix_type: &str) -> AppResult<Option<PlaylistRecord>>;
    async fn delete_playlist(&self, id: &str) -> AppResult<()>;
    async fn add_track(&self, playlist_id: &str, track_id: &str, position: Option<i64>) -> AppResult<()>;
    async fn remove_track(&self, playlist_id: &str, track_id: &str) -> AppResult<()>;
    async fn set_tracks(&self, playlist_id: &str, track_ids: &[String]) -> AppResult<()>;
    async fn get_playlist_tracks(&self, playlist_id: &str) -> AppResult<Vec<TrackRecord>>;
    async fn get_track_count(&self, playlist_id: &str) -> AppResult<i64>;
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
        let mixes = sqlx::query_as::<_, PlaylistRecord>(
            "SELECT id, name, description, is_smart_mix, mix_type,
                    generation_reason, expires_at, created_at, updated_at
             FROM playlists
             WHERE is_smart_mix = 1
             ORDER BY updated_at DESC"
        )
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
             WHERE is_smart_mix = 1 AND mix_type = ?
             ORDER BY updated_at DESC
             LIMIT 1"
        )
        .bind(mix_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(mix)
    }

    async fn delete_playlist(&self, id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM playlists WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
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

    async fn get_playlist_tracks(&self, playlist_id: &str) -> AppResult<Vec<TrackRecord>> {
        let tracks = sqlx::query_as::<_, TrackRecord>(
            "SELECT t.*
             FROM tracks t
             JOIN playlist_tracks pt ON pt.track_id = t.id
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
}
