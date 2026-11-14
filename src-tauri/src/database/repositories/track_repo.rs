use crate::core::error::{AppError, AppResult};
use crate::database::models::TrackRecord;
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

mod writes;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TrackDetail {
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
    pub musicbrainz_track_id: Option<String>,
    pub spotify_id: Option<String>,
    pub manual_like: i64,
    pub created_at: i64,
    pub cover_art_url: Option<String>,
    pub preview_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedMetadata {
    #[serde(default)]
    pub title_is_fallback: bool,
    pub file_path: String,
    pub file_size: i64,
    pub modified_timestamp: i64,
    pub file_hash: Option<String>,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub genre: Option<String>,
    pub track_number: Option<u32>,
    pub disc_number: Option<u32>,
    pub year: Option<u32>,
    pub duration_secs: f64,
    pub bitrate: Option<u32>,
    pub sample_rate: Option<u32>,
    pub format: String,
    pub has_cover_art: bool,
    pub musicbrainz_track_id: Option<String>,
}

#[async_trait]
pub trait TrackRepository: Send + Sync {
    async fn find_by_path(&self, file_path: &str) -> AppResult<Option<TrackRecord>>;
    async fn find_by_id(&self, track_id: &str) -> AppResult<Option<TrackDetail>>;
    async fn find_by_id_scoped(&self, track_id: &str, user_id: Option<&str>) -> AppResult<Option<TrackDetail>>;
    async fn get_all_paths_in_folder_prefix(&self, folder_prefix: &str) -> AppResult<Vec<String>>;
    async fn save_scanned_track(&self, meta: ScannedMetadata) -> AppResult<String>;
    async fn delete_by_path(&self, file_path: &str) -> AppResult<()>;
    async fn list_tracks(&self, offset: u32, limit: u32, sort_by: Option<&str>, ascending: bool) -> AppResult<Vec<TrackDetail>>;
    async fn library_page(&self, user_id: &str, offset: u32, limit: u32) -> AppResult<(Vec<TrackDetail>, i64)>;
    /// A page of one section (Local Library or Kaze downloads).
    async fn library_page_scoped(&self, user_id: &str, offset: u32, limit: u32, scope: &super::LibraryScope) -> AppResult<(Vec<TrackDetail>, i64)>;
    async fn list_tracks_scoped(&self, user_id: Option<&str>, offset: u32, limit: u32, sort_by: Option<&str>, ascending: bool) -> AppResult<Vec<TrackDetail>>;
    async fn search_tracks(&self, query: &str, limit: u32) -> AppResult<Vec<TrackDetail>>;
    async fn search_tracks_scoped(&self, user_id: Option<&str>, query: &str, limit: u32) -> AppResult<Vec<TrackDetail>>;
    async fn get_track_count(&self) -> AppResult<i64>;
}

#[derive(Clone)]
pub struct SqliteTrackRepository {
    pool: SqlitePool,
}

impl SqliteTrackRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TrackRepository for SqliteTrackRepository {
    async fn find_by_path(&self, file_path: &str) -> AppResult<Option<TrackRecord>> {
        let record = sqlx::query_as::<_, TrackRecord>(
            "SELECT id, file_path, file_size, modified_timestamp, file_hash, title, normalized_title,
                    artist_id, album_id, genre_id, track_number, disc_number, year, duration_secs,
                    bitrate, sample_rate, format, has_cover_art, musicbrainz_track_id, spotify_id,
                    created_at, updated_at
             FROM tracks WHERE file_path = ?"
        )
        .bind(file_path)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(record)
    }

    async fn find_by_id(&self, track_id: &str) -> AppResult<Option<TrackDetail>> {
        self.find_by_id_scoped(track_id, None).await
    }

    async fn find_by_id_scoped(&self, track_id: &str, user_id: Option<&str>) -> AppResult<Option<TrackDetail>> {
        let user_id = user_id.unwrap_or("default");
        let detail = sqlx::query_as::<_, TrackDetail>(
            "SELECT t.id, t.file_path, t.file_size, t.modified_timestamp, t.title,
                    a.name as artist_name, al.title as album_title, g.name as genre_name,
                    t.track_number, t.disc_number, t.year, t.duration_secs, t.bitrate,
                    t.sample_rate, t.format, t.has_cover_art, t.musicbrainz_track_id, t.spotify_id,
                    COALESCE(ts.manual_like, 0) as manual_like, t.created_at,
                    ext.cover_art_url, ext.preview_url
             FROM tracks t
             LEFT JOIN artists a ON t.artist_id = a.id
             LEFT JOIN albums al ON t.album_id = al.id
             LEFT JOIN genres g ON t.genre_id = g.id
             LEFT JOIN track_statistics ts ON t.id = ts.track_id AND ts.user_id = ?
             LEFT JOIN external_tracks ext ON t.id = ext.id
             WHERE t.id = ?"
        )
        .bind(user_id)
        .bind(track_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(detail)
    }

    async fn get_all_paths_in_folder_prefix(&self, folder_prefix: &str) -> AppResult<Vec<String>> {
        let pattern = format!("{}%", folder_prefix);
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT file_path FROM tracks WHERE file_path LIKE ?"
        )
        .bind(pattern)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|(p,)| p).collect())
    }

    async fn save_scanned_track(&self, meta: ScannedMetadata) -> AppResult<String> {
        self.save_scanned_track_internal(meta, None).await
    }

    async fn delete_by_path(&self, file_path: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM tracks WHERE file_path = ?")
            .bind(file_path)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn list_tracks(&self, offset: u32, limit: u32, sort_by: Option<&str>, ascending: bool) -> AppResult<Vec<TrackDetail>> {
        self.list_tracks_scoped(None, offset, limit, sort_by, ascending).await
    }

    async fn library_page(&self, user_id: &str, offset: u32, limit: u32) -> AppResult<(Vec<TrackDetail>, i64)> {
        self.library_page_scoped(user_id, offset, limit, &super::LibraryScope::All).await
    }

    async fn library_page_scoped(&self, user_id: &str, offset: u32, limit: u32, scope: &super::LibraryScope) -> AppResult<(Vec<TrackDetail>, i64)> {
        let condition = scope.condition("t.file_path");
        let extra = condition.as_ref().map(|(sql, ..)| format!(" AND {sql}")).unwrap_or_default();
        let mut tx = self.pool.begin().await?;
        let sql = format!(
            "SELECT t.id, t.file_path, t.file_size, t.modified_timestamp, t.title,
                    a.name as artist_name, al.title as album_title, g.name as genre_name,
                    t.track_number, t.disc_number, t.year, t.duration_secs, t.bitrate,
                    t.sample_rate, t.format, t.has_cover_art, t.musicbrainz_track_id, t.spotify_id,
                    COALESCE(ts.manual_like, 0) as manual_like, t.created_at,
                    ext.cover_art_url, ext.preview_url
             FROM tracks t
             LEFT JOIN artists a ON t.artist_id = a.id
             LEFT JOIN albums al ON t.album_id = al.id
             LEFT JOIN genres g ON t.genre_id = g.id
             LEFT JOIN track_statistics ts ON t.id = ts.track_id AND ts.user_id = ?
             LEFT JOIN external_tracks ext ON t.id = ext.id
             WHERE t.file_path NOT LIKE 'online://%'{extra}
             ORDER BY t.title ASC, t.id ASC LIMIT ? OFFSET ?");

        let mut query = sqlx::query_as::<_, TrackDetail>(&sql).bind(user_id);
        if let Some((_, len, prefix)) = &condition {
            query = query.bind(*len).bind(prefix.clone());
        }
        let records = query
            .bind(limit)
            .bind(offset)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        let count_sql = format!("SELECT COUNT(*) FROM tracks t WHERE t.file_path NOT LIKE 'online://%'{extra}");
        let mut count = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some((_, len, prefix)) = &condition {
            count = count.bind(*len).bind(prefix.clone());
        }
        let total = count.fetch_one(&mut *tx).await?;
        tx.commit().await?;
        Ok((records, total))
    }

    async fn list_tracks_scoped(&self, user_id: Option<&str>, offset: u32, limit: u32, sort_by: Option<&str>, ascending: bool) -> AppResult<Vec<TrackDetail>> {
        let user_id = user_id.unwrap_or("default");
        let order = if ascending { "ASC" } else { "DESC" };
        let col = match sort_by {
            Some("artist") => "artist_name",
            Some("album") => "album_title",
            Some("duration") => "t.duration_secs",
            Some("year") => "t.year",
            _ => "t.title",
        };

        let sql = format!(
            "SELECT t.id, t.file_path, t.file_size, t.modified_timestamp, t.title,
                    a.name as artist_name, al.title as album_title, g.name as genre_name,
                    t.track_number, t.disc_number, t.year, t.duration_secs, t.bitrate,
                    t.sample_rate, t.format, t.has_cover_art, t.musicbrainz_track_id, t.spotify_id,
                    COALESCE(ts.manual_like, 0) as manual_like, t.created_at,
                    ext.cover_art_url, ext.preview_url
             FROM tracks t
             LEFT JOIN artists a ON t.artist_id = a.id
             LEFT JOIN albums al ON t.album_id = al.id
             LEFT JOIN genres g ON t.genre_id = g.id
             LEFT JOIN track_statistics ts ON t.id = ts.track_id AND ts.user_id = ?
             LEFT JOIN external_tracks ext ON t.id = ext.id
             ORDER BY {} {}, t.id {} LIMIT ? OFFSET ?",
            col, order, order
        );

        let records = sqlx::query_as::<_, TrackDetail>(&sql)
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(records)
    }

    async fn search_tracks(&self, query: &str, limit: u32) -> AppResult<Vec<TrackDetail>> {
        self.search_tracks_scoped(None, query, limit).await
    }

    async fn search_tracks_scoped(&self, user_id: Option<&str>, query: &str, limit: u32) -> AppResult<Vec<TrackDetail>> {
        let cleaned = query.replace('\"', "").trim().to_string();
        if cleaned.is_empty() {
            return Ok(vec![]);
        }

        let user_id = user_id.unwrap_or("default");
        let fts_query = format!("\"{}\"*", cleaned);

        let records = sqlx::query_as::<_, TrackDetail>(
            "SELECT t.id, t.file_path, t.file_size, t.modified_timestamp, t.title,
                    a.name as artist_name, al.title as album_title, g.name as genre_name,
                    t.track_number, t.disc_number, t.year, t.duration_secs, t.bitrate,
                    t.sample_rate, t.format, t.has_cover_art, t.musicbrainz_track_id, t.spotify_id,
                    COALESCE(ts.manual_like, 0) as manual_like, t.created_at,
                    ext.cover_art_url, ext.preview_url
             FROM tracks_fts fts
             JOIN tracks t ON fts.rowid = t.rowid
             LEFT JOIN artists a ON t.artist_id = a.id
             LEFT JOIN albums al ON t.album_id = al.id
             LEFT JOIN genres g ON t.genre_id = g.id
             LEFT JOIN track_statistics ts ON t.id = ts.track_id AND ts.user_id = ?
             LEFT JOIN external_tracks ext ON t.id = ext.id
             WHERE tracks_fts MATCH ?
             LIMIT ?"
        )
        .bind(user_id)
        .bind(&fts_query)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("FTS search error: {}", e)))?;

        Ok(records)
    }

    async fn get_track_count(&self) -> AppResult<i64> {
        let row: (i64,) = sqlx::query_as("SELECT count(*) FROM tracks")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(row.0)
    }
}

#[cfg(test)]
mod enrichment_concurrency_tests;
