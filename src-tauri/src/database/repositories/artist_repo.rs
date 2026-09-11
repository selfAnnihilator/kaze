use crate::core::error::{AppError, AppResult};
use crate::database::models::ArtistRecord;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ArtistSummary {
    pub id: String,
    pub name: String,
    pub bio: Option<String>,
    pub image_url: Option<String>,
    pub track_count: i64,
    pub album_count: i64,
}

#[async_trait]
pub trait ArtistRepository: Send + Sync {
    async fn list_artists(&self, offset: u32, limit: u32) -> AppResult<Vec<ArtistSummary>>;
    async fn find_by_id(&self, artist_id: &str) -> AppResult<Option<ArtistRecord>>;
}

#[derive(Clone)]
pub struct SqliteArtistRepository {
    pool: SqlitePool,
}

impl SqliteArtistRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ArtistRepository for SqliteArtistRepository {
    async fn list_artists(&self, offset: u32, limit: u32) -> AppResult<Vec<ArtistSummary>> {
        let sql = "
            SELECT a.id, a.name, a.bio, a.image_url,
                   COUNT(DISTINCT t.id) as track_count,
                   COUNT(DISTINCT al.id) as album_count
            FROM artists a
            LEFT JOIN tracks t ON t.artist_id = a.id
            LEFT JOIN albums al ON al.artist_id = a.id
            GROUP BY a.id
            ORDER BY a.name ASC
            LIMIT ? OFFSET ?
        ";

        let records = sqlx::query_as::<_, ArtistSummary>(sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(records)
    }

    async fn find_by_id(&self, artist_id: &str) -> AppResult<Option<ArtistRecord>> {
        let record = sqlx::query_as::<_, ArtistRecord>(
            "SELECT id, name, normalized_name, musicbrainz_id, bio, image_url, created_at
             FROM artists WHERE id = ?"
        )
        .bind(artist_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(record)
    }
}
