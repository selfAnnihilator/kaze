use crate::core::error::{AppError, AppResult};
use crate::database::models::AlbumRecord;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AlbumSummary {
    pub id: String,
    pub title: String,
    pub artist_name: Option<String>,
    pub release_year: Option<i64>,
    pub cover_art_path: Option<String>,
    pub track_count: i64,
    pub first_track_id: Option<String>,
}

#[async_trait]
pub trait AlbumRepository: Send + Sync {
    async fn list_albums(&self, offset: u32, limit: u32) -> AppResult<Vec<AlbumSummary>>;
    async fn find_by_id(&self, album_id: &str) -> AppResult<Option<AlbumRecord>>;
}

#[derive(Clone)]
pub struct SqliteAlbumRepository {
    pool: SqlitePool,
}

impl SqliteAlbumRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AlbumRepository for SqliteAlbumRepository {
    async fn list_albums(&self, offset: u32, limit: u32) -> AppResult<Vec<AlbumSummary>> {
        let sql = "
            SELECT al.id, al.title, a.name as artist_name, al.release_year,
                   al.cover_art_path, COUNT(t.id) as track_count,
                   (
                       SELECT t2.id FROM tracks t2
                       WHERE t2.album_id = al.id AND t2.has_cover_art = 1
                       ORDER BY t2.track_number ASC, t2.title ASC
                       LIMIT 1
                   ) as first_track_id
            FROM albums al
            LEFT JOIN artists a ON al.artist_id = a.id
            LEFT JOIN tracks t ON t.album_id = al.id
            GROUP BY al.id
            ORDER BY al.title ASC
            LIMIT ? OFFSET ?
        ";

        let records = sqlx::query_as::<_, AlbumSummary>(sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(records)
    }

    async fn find_by_id(&self, album_id: &str) -> AppResult<Option<AlbumRecord>> {
        let record = sqlx::query_as::<_, AlbumRecord>(
            "SELECT id, title, normalized_title, artist_id, album_artist, release_year,
                    total_tracks, cover_art_path, musicbrainz_id, created_at
             FROM albums WHERE id = ?"
        )
        .bind(album_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(record)
    }
}
