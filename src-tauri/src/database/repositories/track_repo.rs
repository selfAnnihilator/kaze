use crate::core::error::{AppError, AppResult};
use crate::database::models::TrackRecord;
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

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

    async fn find_or_create_artist(&self, name: &str) -> AppResult<String> {
        let normalized = name.trim().to_lowercase();
        if let Some((id,)) = sqlx::query_as::<_, (String,)>(
            "SELECT id FROM artists WHERE normalized_name = ?"
        )
        .bind(&normalized)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))? {
            return Ok(id);
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO artists (id, name, normalized_name, created_at) VALUES (?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(name.trim())
        .bind(&normalized)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(id)
    }

    async fn find_or_create_genre(&self, name: &str) -> AppResult<String> {
        let normalized = name.trim().to_lowercase();
        if let Some((id,)) = sqlx::query_as::<_, (String,)>(
            "SELECT id FROM genres WHERE normalized_name = ?"
        )
        .bind(&normalized)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))? {
            return Ok(id);
        }

        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO genres (id, name, normalized_name) VALUES (?, ?, ?)"
        )
        .bind(&id)
        .bind(name.trim())
        .bind(&normalized)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(id)
    }

    async fn find_or_create_album(
        &self,
        title: &str,
        artist_id: Option<&str>,
        album_artist: Option<&str>,
        release_year: Option<u32>,
    ) -> AppResult<String> {
        let normalized = title.trim().to_lowercase();
        if let Some((id,)) = sqlx::query_as::<_, (String,)>(
            "SELECT id FROM albums WHERE normalized_title = ? AND (artist_id IS ? OR artist_id = ?)"
        )
        .bind(&normalized)
        .bind(artist_id)
        .bind(artist_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))? {
            return Ok(id);
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO albums (id, title, normalized_title, artist_id, album_artist, release_year, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(title.trim())
        .bind(&normalized)
        .bind(artist_id)
        .bind(album_artist)
        .bind(release_year.map(|y| y as i64))
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(id)
    }
}

impl SqliteTrackRepository {
    pub async fn apply_stored_enrichment(&self, id: &str) -> AppResult<()> {
        let snapshot: String =
            sqlx::query_scalar("SELECT metadata_json FROM track_source_metadata WHERE track_id=?")
                .bind(id)
                .fetch_one(&self.pool)
                .await?;
        let source =
            serde_json::from_str(&snapshot).map_err(|e| AppError::Metadata(e.to_string()))?;
        self.save_scanned_track_internal(source, Some(snapshot))
            .await?;
        Ok(())
    }

    async fn save_scanned_track_internal(
        &self,
        mut meta: ScannedMetadata,
        expected_source: Option<String>,
    ) -> AppResult<String> {
        let source_json =
            serde_json::to_string(&meta).map_err(|e| AppError::Metadata(e.to_string()))?;
        // On every rescan, fresh embedded tags win; only absent fields receive provider data.
        if let Some(existing) = self.find_by_path(&meta.file_path).await? {
            let fill: Option<(Option<String>, Option<String>)> = sqlx::query_as("SELECT metadata_json, cover_art_path FROM track_enrichment WHERE track_id = ? AND state IN ('matched','applying')")
                .bind(&existing.id).fetch_optional(&self.pool).await?;
            if let Some((Some(json), artwork)) = fill {
                if let Ok(t) =
                    serde_json::from_str::<crate::providers::ExternalTrackMetadata>(&json)
                {
                    let still_matches = crate::providers::strict_match::unique_match(
                        &meta.title,
                        meta.artist.as_deref(),
                        meta.album.as_deref(),
                        meta.duration_secs,
                        std::slice::from_ref(&t),
                    )
                    .is_some();
                    if !still_matches {
                        sqlx::query("UPDATE track_enrichment SET state='stale', retry_after=0 WHERE track_id=?").bind(&existing.id).execute(&self.pool).await?;
                    } else {
                        if artwork.is_some_and(|p| std::path::Path::new(&p).is_file()) {
                            meta.has_cover_art = true;
                        }
                        if t.provider == "musicbrainz" && meta.musicbrainz_track_id.is_none() {
                            meta.musicbrainz_track_id = Some(t.provider_track_id.clone());
                        }
                        if meta.title_is_fallback && !t.title.trim().is_empty() {
                            meta.title = t.title;
                        }
                        if meta.artist.is_none() && !t.artist_name.trim().is_empty() {
                            meta.artist = Some(t.artist_name);
                        }
                        if meta.album.is_none() {
                            meta.album = t.album_title;
                        }
                        if meta.year.is_none() {
                            meta.year = t.year.and_then(|y| u32::try_from(y).ok());
                        }
                        if meta.track_number.is_none() {
                            meta.track_number = t.track_number;
                        }
                    }
                }
            }
            sqlx::query("UPDATE track_enrichment SET retry_after=0 WHERE track_id=? AND state!='matched' AND EXISTS (SELECT 1 FROM track_source_metadata s WHERE s.track_id=? AND s.metadata_json!=?)")
                .bind(&existing.id).bind(&existing.id).bind(&source_json).execute(&self.pool).await?;
        }

        let artist_id = if let Some(ref artist) = meta.artist {
            Some(self.find_or_create_artist(artist).await?)
        } else {
            None
        };

        let genre_id = if let Some(ref genre) = meta.genre {
            Some(self.find_or_create_genre(genre).await?)
        } else {
            None
        };

        let album_id = if let Some(ref album) = meta.album {
            Some(
                self.find_or_create_album(
                    album,
                    artist_id.as_deref(),
                    meta.album_artist.as_deref(),
                    meta.year,
                )
                .await?,
            )
        } else {
            None
        };

        let normalized_title = meta.title.trim().to_lowercase();
        let now = Utc::now().timestamp();

        if let Some(existing) = self.find_by_path(&meta.file_path).await? {
            let mut transaction = self.pool.begin().await?;
            let result = sqlx::query(
                "UPDATE tracks SET
                    file_size = ?, modified_timestamp = ?, file_hash = ?, title = ?, normalized_title = ?,
                    artist_id = ?, album_id = ?, genre_id = ?, track_number = ?, disc_number = ?, year = ?,
                    duration_secs = ?, bitrate = ?, sample_rate = ?, format = ?, has_cover_art = ?,
                    musicbrainz_track_id = ?, updated_at = ?
                 WHERE id = ? AND (? IS NULL OR EXISTS (SELECT 1 FROM track_source_metadata s WHERE s.track_id=tracks.id AND s.metadata_json=?))"
            )
            .bind(meta.file_size)
            .bind(meta.modified_timestamp)
            .bind(meta.file_hash)
            .bind(meta.title)
            .bind(&normalized_title)
            .bind(artist_id)
            .bind(album_id)
            .bind(genre_id)
            .bind(meta.track_number.map(|n| n as i64))
            .bind(meta.disc_number.map(|n| n as i64).unwrap_or(1))
            .bind(meta.year.map(|y| y as i64))
            .bind(meta.duration_secs)
            .bind(meta.bitrate.map(|b| b as i64))
            .bind(meta.sample_rate.map(|s| s as i64))
            .bind(meta.format)
            .bind(if meta.has_cover_art { 1 } else { 0 })
            .bind(meta.musicbrainz_track_id)
            .bind(now)
            .bind(&existing.id)
            .bind(expected_source.as_deref())
            .bind(expected_source.as_deref())
            .execute(&mut *transaction)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

            if result.rows_affected() == 0 {
                return Err(AppError::Metadata(
                    "Source metadata changed during enrichment; retry pending merge".into(),
                ));
            }
            sqlx::query("INSERT INTO track_source_metadata(track_id, metadata_json) VALUES (?, ?) ON CONFLICT(track_id) DO UPDATE SET metadata_json = excluded.metadata_json")
                .bind(&existing.id).bind(&source_json).execute(&mut *transaction).await?;
            transaction.commit().await?;
            Ok(existing.id)
        } else {
            if expected_source.is_some() {
                return Err(AppError::NotFound("Track removed during enrichment".into()));
            }
            let id = Uuid::new_v4().to_string();
            let mut transaction = self.pool.begin().await?;
            sqlx::query(
                "INSERT INTO tracks (
                    id, file_path, file_size, modified_timestamp, file_hash, title, normalized_title,
                    artist_id, album_id, genre_id, track_number, disc_number, year, duration_secs,
                    bitrate, sample_rate, format, has_cover_art, musicbrainz_track_id, created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&id)
            .bind(meta.file_path)
            .bind(meta.file_size)
            .bind(meta.modified_timestamp)
            .bind(meta.file_hash)
            .bind(meta.title)
            .bind(&normalized_title)
            .bind(artist_id)
            .bind(album_id)
            .bind(genre_id)
            .bind(meta.track_number.map(|n| n as i64))
            .bind(meta.disc_number.map(|n| n as i64).unwrap_or(1))
            .bind(meta.year.map(|y| y as i64))
            .bind(meta.duration_secs)
            .bind(meta.bitrate.map(|b| b as i64))
            .bind(meta.sample_rate.map(|s| s as i64))
            .bind(meta.format)
            .bind(if meta.has_cover_art { 1 } else { 0 })
            .bind(meta.musicbrainz_track_id)
            .bind(now)
            .bind(now)
            .execute(&mut *transaction)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

            sqlx::query("INSERT INTO track_source_metadata(track_id, metadata_json) VALUES (?, ?)")
                .bind(&id)
                .bind(&source_json)
                .execute(&mut *transaction)
                .await?;
            transaction.commit().await?;
            Ok(id)
        }
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
             ORDER BY {} {} LIMIT ? OFFSET ?",
            col, order
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
mod enrichment_concurrency_tests {
    use super::*;
    #[tokio::test]
    async fn stale_background_merge_cannot_replace_a_new_scan_snapshot() {
        let pool = crate::database::create_in_memory_pool().await.unwrap();
        let repo = SqliteTrackRepository::new(pool.clone());
        let original: ScannedMetadata = serde_json::from_value(serde_json::json!({
            "file_path":"/fixture/track.flac", "file_size":100, "modified_timestamp":1,
            "title":"Filename", "title_is_fallback":true, "duration_secs":200.0,
            "format":"flac", "has_cover_art":false
        })).unwrap();
        let id = repo.save_scanned_track(original.clone()).await.unwrap();
        let old_snapshot = serde_json::to_string(&original).unwrap();
        let mut scanned = original.clone();
        scanned.title = "New Embedded Title".into(); scanned.title_is_fallback = false;
        scanned.year = Some(2026); scanned.modified_timestamp = 2;
        repo.save_scanned_track(scanned).await.unwrap();
        assert!(repo.save_scanned_track_internal(original, Some(old_snapshot)).await.is_err());
        let current = repo.find_by_id(&id).await.unwrap().unwrap();
        assert_eq!(current.title, "New Embedded Title"); assert_eq!(current.year, Some(2026));
        pool.close().await;
    }
}
