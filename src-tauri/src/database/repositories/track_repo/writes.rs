//! Saving scanned files: the artist/genre/album rows they need, and the
//! track row itself (with stored source enrichment).
use super::*;

impl SqliteTrackRepository {
    pub(super) async fn find_or_create_artist(&self, name: &str) -> AppResult<String> {
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

    pub(super) async fn find_or_create_genre(&self, name: &str) -> AppResult<String> {
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

    pub(super) async fn find_or_create_album(
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

    pub(super) async fn save_scanned_track_internal(
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
