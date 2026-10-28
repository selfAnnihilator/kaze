use super::track_repo::TrackDetail;
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
    async fn tracks_scoped(&self, album_id: &str, user_id: &str) -> AppResult<Vec<TrackDetail>>;
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
                       AND (NOT EXISTS (SELECT 1 FROM track_enrichment e WHERE e.track_id=t2.id AND e.state='matched' AND e.cover_art_path IS NOT NULL)
                            OR EXISTS (SELECT 1 FROM track_source_metadata s WHERE s.track_id=t2.id AND json_extract(s.metadata_json,'$.has_cover_art')=1)
                            OR EXISTS (SELECT 1 FROM track_enrichment e WHERE e.track_id=t2.id AND e.state='matched' AND json_extract(e.metadata_json,'$.album_id') IS NOT NULL AND lower(trim(json_extract(e.metadata_json,'$.album_title')))=al.normalized_title))
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

    async fn tracks_scoped(&self, album_id: &str, user_id: &str) -> AppResult<Vec<TrackDetail>> {
        sqlx::query_as::<_, TrackDetail>(
            "SELECT t.id,t.file_path,t.file_size,t.modified_timestamp,t.title,
                a.name AS artist_name,al.title AS album_title,g.name AS genre_name,
                t.track_number,t.disc_number,t.year,t.duration_secs,t.bitrate,t.sample_rate,
                t.format,t.has_cover_art,t.musicbrainz_track_id,t.spotify_id,
                COALESCE(ts.manual_like,0) AS manual_like,t.created_at,ext.cover_art_url,ext.preview_url
             FROM tracks t
             LEFT JOIN artists a ON a.id=t.artist_id
             LEFT JOIN albums al ON al.id=t.album_id
             LEFT JOIN genres g ON g.id=t.genre_id
             LEFT JOIN track_statistics ts ON ts.track_id=t.id AND ts.user_id=?
             LEFT JOIN external_tracks ext ON ext.id=t.id
             WHERE t.album_id=?
             ORDER BY COALESCE(t.disc_number,1),COALESCE(t.track_number,2147483647),t.title,t.id"
        ).bind(user_id).bind(album_id).fetch_all(&self.pool).await
            .map_err(|e| AppError::Database(e.to_string()))
    }

    async fn find_by_id(&self, album_id: &str) -> AppResult<Option<AlbumRecord>> {
        let record = sqlx::query_as::<_, AlbumRecord>(
            "SELECT id, title, normalized_title, artist_id, album_artist, release_year,
                    total_tracks, cover_art_path, musicbrainz_id, created_at
             FROM albums WHERE id = ?",
        )
        .bind(album_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(record)
    }
}
