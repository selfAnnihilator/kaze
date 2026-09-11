use crate::core::error::{AppError, AppResult};
use crate::database::models::{ExternalTrackRecord, WishlistItemRecord};
use async_trait::async_trait;
use sqlx::SqlitePool;

#[async_trait]
pub trait WishlistRepository: Send + Sync {
    async fn add_item(&self, item: &WishlistItemRecord) -> AppResult<()>;
    async fn get_all(&self, status_filter: Option<&str>) -> AppResult<Vec<WishlistItemRecord>>;
    async fn get_by_id(&self, id: &str) -> AppResult<Option<WishlistItemRecord>>;
    async fn update_status(&self, id: &str, status: &str) -> AppResult<()>;
    async fn delete_item(&self, id: &str) -> AppResult<()>;

    async fn upsert_external_track(&self, track: &ExternalTrackRecord) -> AppResult<()>;
    async fn get_external_track(&self, id: &str) -> AppResult<Option<ExternalTrackRecord>>;
    async fn update_match_status(
        &self,
        id: &str,
        match_status: &str,
        local_track_id: Option<&str>,
    ) -> AppResult<()>;
    async fn list_external_tracks(
        &self,
        match_status: Option<&str>,
        limit: u32,
    ) -> AppResult<Vec<ExternalTrackRecord>>;
}

#[derive(Clone)]
pub struct SqliteWishlistRepository {
    pool: SqlitePool,
}

impl SqliteWishlistRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WishlistRepository for SqliteWishlistRepository {
    async fn add_item(&self, item: &WishlistItemRecord) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO wishlist (id, title, artist, album, external_track_id, status, notes, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                artist = excluded.artist,
                album = excluded.album,
                status = excluded.status,
                notes = excluded.notes,
                updated_at = excluded.updated_at"
        )
        .bind(&item.id)
        .bind(&item.title)
        .bind(&item.artist)
        .bind(&item.album)
        .bind(&item.external_track_id)
        .bind(&item.status)
        .bind(&item.notes)
        .bind(item.created_at)
        .bind(item.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_all(&self, status_filter: Option<&str>) -> AppResult<Vec<WishlistItemRecord>> {
        let items = if let Some(status) = status_filter {
            sqlx::query_as::<_, WishlistItemRecord>(
                "SELECT id, title, artist, album, external_track_id, status, notes, created_at, updated_at
                 FROM wishlist
                 WHERE status = ?
                 ORDER BY created_at DESC"
            )
            .bind(status)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, WishlistItemRecord>(
                "SELECT id, title, artist, album, external_track_id, status, notes, created_at, updated_at
                 FROM wishlist
                 ORDER BY created_at DESC"
            )
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(items)
    }

    async fn get_by_id(&self, id: &str) -> AppResult<Option<WishlistItemRecord>> {
        let item = sqlx::query_as::<_, WishlistItemRecord>(
            "SELECT id, title, artist, album, external_track_id, status, notes, created_at, updated_at
             FROM wishlist WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(item)
    }

    async fn update_status(&self, id: &str, status: &str) -> AppResult<()> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "UPDATE wishlist SET status = ?, updated_at = ? WHERE id = ?"
        )
        .bind(status)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete_item(&self, id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM wishlist WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn upsert_external_track(&self, track: &ExternalTrackRecord) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO external_tracks (
                id, provider, provider_id, title, artist, album,
                duration_secs, cover_art_url, match_status, matched_local_track_id, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(provider, provider_id) DO UPDATE SET
                title = excluded.title,
                artist = excluded.artist,
                album = excluded.album,
                duration_secs = excluded.duration_secs,
                cover_art_url = excluded.cover_art_url,
                match_status = excluded.match_status,
                matched_local_track_id = excluded.matched_local_track_id"
        )
        .bind(&track.id)
        .bind(&track.provider)
        .bind(&track.provider_id)
        .bind(&track.title)
        .bind(&track.artist)
        .bind(&track.album)
        .bind(track.duration_secs)
        .bind(&track.cover_art_url)
        .bind(&track.match_status)
        .bind(&track.matched_local_track_id)
        .bind(track.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_external_track(&self, id: &str) -> AppResult<Option<ExternalTrackRecord>> {
        let track = sqlx::query_as::<_, ExternalTrackRecord>(
            "SELECT id, provider, provider_id, title, artist, album,
                    duration_secs, cover_art_url, match_status, matched_local_track_id, created_at
             FROM external_tracks WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(track)
    }

    async fn update_match_status(
        &self,
        id: &str,
        match_status: &str,
        local_track_id: Option<&str>,
    ) -> AppResult<()> {
        sqlx::query(
            "UPDATE external_tracks SET match_status = ?, matched_local_track_id = ? WHERE id = ?"
        )
        .bind(match_status)
        .bind(local_track_id)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn list_external_tracks(
        &self,
        match_status: Option<&str>,
        limit: u32,
    ) -> AppResult<Vec<ExternalTrackRecord>> {
        let tracks = if let Some(status) = match_status {
            sqlx::query_as::<_, ExternalTrackRecord>(
                "SELECT id, provider, provider_id, title, artist, album,
                        duration_secs, cover_art_url, match_status, matched_local_track_id, created_at
                 FROM external_tracks
                 WHERE match_status = ?
                 ORDER BY created_at DESC
                 LIMIT ?"
            )
            .bind(status)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, ExternalTrackRecord>(
                "SELECT id, provider, provider_id, title, artist, album,
                        duration_secs, cover_art_url, match_status, matched_local_track_id, created_at
                 FROM external_tracks
                 ORDER BY created_at DESC
                 LIMIT ?"
            )
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(tracks)
    }
}
