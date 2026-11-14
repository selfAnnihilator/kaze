use crate::core::error::{AppError, AppResult};
use crate::database::models::DownloadTaskRecord;
use async_trait::async_trait;
use sqlx::SqlitePool;

#[async_trait]
pub trait DownloadRepository: Send + Sync {
    async fn create_task(&self, task: &DownloadTaskRecord) -> AppResult<()>;
    async fn get_task_by_id(&self, id: &str) -> AppResult<Option<DownloadTaskRecord>>;
    async fn list_tasks(
        &self,
        status_filter: Option<&str>,
        limit: u32,
    ) -> AppResult<Vec<DownloadTaskRecord>>;
    async fn update_progress(&self, id: &str, bytes_downloaded: i64) -> AppResult<()>;
    async fn update_status(
        &self,
        id: &str,
        status: &str,
        destination_path: Option<&str>,
        error_message: Option<&str>,
        completed_at: Option<i64>,
    ) -> AppResult<()>;
    async fn fail_interrupted_task(&self, id: &str, message: &str) -> AppResult<bool>;
    async fn delete_task(&self, id: &str) -> AppResult<()>;
    /// Points a moved download at its new file in one transaction, keeping the
    /// library row (and so its id, history, likes and playlists) intact.
    async fn relocate(
        &self,
        id: &str,
        new_destination: &str,
        old_track_path: &str,
        new_track_path: &str,
    ) -> AppResult<()>;
    /// Turns a downloaded song's library row back into its online recording.
    /// Likes, playlist entries, history and statistics move to the online song
    /// first, so deleting the local row loses nothing. Returns the released
    /// `(local id, online id)`, or `None` when no library row exists at those paths.
    async fn release_to_online(
        &self,
        track_paths: &[String],
        recording: &OnlineRecording,
    ) -> AppResult<Option<(String, String)>>;
}

/// The online recording a downloaded file came from.
#[derive(Debug, Clone)]
pub struct OnlineRecording {
    pub source_id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
}

impl OnlineRecording {
    pub fn track_id(&self) -> String {
        format!("jiosaavn:{}", self.source_id)
    }
}

#[derive(Clone)]
pub struct SqliteDownloadRepository {
    pool: SqlitePool,
}

impl SqliteDownloadRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DownloadRepository for SqliteDownloadRepository {
    async fn create_task(&self, task: &DownloadTaskRecord) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO download_tasks (
                id, provider, provider_task_id, title, artist, album, source_track_id,
                filename, destination_path, file_size, bytes_downloaded,
                status, error_message, wishlist_id, created_at, completed_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&task.id)
        .bind(&task.provider)
        .bind(&task.provider_task_id)
        .bind(&task.title)
        .bind(&task.artist)
        .bind(&task.album)
        .bind(&task.source_track_id)
        .bind(&task.filename)
        .bind(&task.destination_path)
        .bind(task.file_size)
        .bind(task.bytes_downloaded)
        .bind(&task.status)
        .bind(&task.error_message)
        .bind(&task.wishlist_id)
        .bind(task.created_at)
        .bind(task.completed_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn get_task_by_id(&self, id: &str) -> AppResult<Option<DownloadTaskRecord>> {
        let task = sqlx::query_as::<_, DownloadTaskRecord>(
            "SELECT id, provider, provider_task_id, title, artist, album, source_track_id,
                    filename, destination_path, file_size, bytes_downloaded,
                    status, error_message, wishlist_id, created_at, completed_at
             FROM download_tasks WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(task)
    }

    async fn list_tasks(
        &self,
        status_filter: Option<&str>,
        limit: u32,
    ) -> AppResult<Vec<DownloadTaskRecord>> {
        let tasks = if let Some(status) = status_filter {
            sqlx::query_as::<_, DownloadTaskRecord>(
                "SELECT id, provider, provider_task_id, title, artist, album, source_track_id,
                        filename, destination_path, file_size, bytes_downloaded,
                        status, error_message, wishlist_id, created_at, completed_at
                 FROM download_tasks
                 WHERE status = ?
                 ORDER BY created_at DESC
                 LIMIT ?"
            )
            .bind(status)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, DownloadTaskRecord>(
                "SELECT id, provider, provider_task_id, title, artist, album, source_track_id,
                        filename, destination_path, file_size, bytes_downloaded,
                        status, error_message, wishlist_id, created_at, completed_at
                 FROM download_tasks
                 ORDER BY created_at DESC
                 LIMIT ?"
            )
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(tasks)
    }

    async fn update_progress(&self, id: &str, bytes_downloaded: i64) -> AppResult<()> {
        sqlx::query(
            "UPDATE download_tasks SET bytes_downloaded = ? WHERE id = ?"
        )
        .bind(bytes_downloaded)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_status(
        &self,
        id: &str,
        status: &str,
        destination_path: Option<&str>,
        error_message: Option<&str>,
        completed_at: Option<i64>,
    ) -> AppResult<()> {
        sqlx::query(
            "UPDATE download_tasks
             SET status = ?,
                 destination_path = COALESCE(?, destination_path),
                 error_message = ?,
                 completed_at = ?
             WHERE id = ?"
        )
        .bind(status)
        .bind(destination_path)
        .bind(error_message)
        .bind(completed_at)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn fail_interrupted_task(&self, id: &str, message: &str) -> AppResult<bool> {
        let result = sqlx::query("UPDATE download_tasks SET status='FAILED',error_message=? WHERE id=? AND status IN ('DOWNLOADING','QUEUED')")
            .bind(message).bind(id).execute(&self.pool).await?;
        Ok(result.rows_affected() > 0)
    }

    async fn relocate(
        &self,
        id: &str,
        new_destination: &str,
        old_track_path: &str,
        new_track_path: &str,
    ) -> AppResult<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE download_tasks SET destination_path = ? WHERE id = ?")
            .bind(new_destination)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE track_source_metadata SET metadata_json = json_set(metadata_json, '$.file_path', ?) \
             WHERE json_valid(metadata_json) AND track_id IN (SELECT id FROM tracks WHERE file_path = ?)",
        )
        .bind(new_track_path)
        .bind(old_track_path)
        .execute(&mut *tx)
        .await?;
        let old_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tracks WHERE file_path = ?)")
            .bind(old_track_path)
            .fetch_one(&mut *tx)
            .await?;
        if old_exists && old_track_path != new_track_path {
            // A scan may have already indexed the new path; the original row
            // carries the song's history, so it wins.
            sqlx::query("DELETE FROM tracks WHERE file_path = ?")
                .bind(new_track_path)
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE tracks SET file_path = ?, updated_at = strftime('%s','now') WHERE file_path = ?")
                .bind(new_track_path)
                .bind(old_track_path)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn delete_task(&self, id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM download_tasks WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    async fn release_to_online(
        &self,
        track_paths: &[String],
        recording: &OnlineRecording,
    ) -> AppResult<Option<(String, String)>> {
        let mut local = None;
        for path in track_paths {
            local = sqlx::query_as::<_, (String, f64)>(
                "SELECT id, duration_secs FROM tracks WHERE file_path = ? AND file_path NOT LIKE 'online://%'",
            )
            .bind(path)
            .fetch_optional(&self.pool)
            .await?;
            if local.is_some() {
                break;
            }
        }
        let Some((local_id, duration)) = local else { return Ok(None) };
        let online_id = recording.track_id();

        // Create the online song only when missing: updating an existing one
        // would reset its artwork flag.
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tracks WHERE id = ?)")
            .bind(&online_id)
            .fetch_one(&self.pool)
            .await?;
        if !exists {
            crate::recommendations::mixes::ensure_online_track(
                &self.pool,
                &crate::recommendations::mixes::OnlineTrackInput {
                    id: &online_id,
                    title: &recording.title,
                    artist: Some(&recording.artist),
                    album: recording.album.as_deref(),
                    duration_secs: Some(duration),
                    cover_art_url: None,
                    preview_url: None,
                },
            )
            .await?;
        }

        let mut tx = self.pool.begin().await?;
        let rekey = |sql: &'static str| sqlx::query(sql).bind(online_id.clone()).bind(local_id.clone());
        // Playlists and Liked Songs: a playlist that already has the online song
        // keeps that entry (the uniqueness trigger ignores the update), and the
        // leftover duplicate goes with the local row below.
        rekey("UPDATE OR IGNORE playlist_tracks SET track_id = ? WHERE track_id = ?").execute(&mut *tx).await?;
        rekey("UPDATE playback_history SET track_id = ? WHERE track_id = ?").execute(&mut *tx).await?;
        rekey("UPDATE recommendation_feedback SET track_id = ? WHERE track_id = ?").execute(&mut *tx).await?;
        rekey("UPDATE OR IGNORE flow_slot_assignments SET track_id = ? WHERE track_id = ?").execute(&mut *tx).await?;
        rekey(
            "INSERT INTO track_statistics (user_id, track_id, play_count, total_time_listened, completion_count, \
             skip_count, last_played_at, manual_like, playlist_addition_count) \
             SELECT user_id, ?, play_count, total_time_listened, completion_count, skip_count, last_played_at, \
             manual_like, playlist_addition_count FROM track_statistics WHERE track_id = ? \
             ON CONFLICT(user_id, track_id) DO UPDATE SET \
             play_count = play_count + excluded.play_count, \
             total_time_listened = total_time_listened + excluded.total_time_listened, \
             completion_count = completion_count + excluded.completion_count, \
             skip_count = skip_count + excluded.skip_count, \
             last_played_at = NULLIF(MAX(COALESCE(last_played_at, 0), COALESCE(excluded.last_played_at, 0)), 0), \
             manual_like = CASE WHEN excluded.manual_like != 0 THEN excluded.manual_like ELSE manual_like END, \
             playlist_addition_count = playlist_addition_count + excluded.playlist_addition_count",
        )
        .execute(&mut *tx)
        .await?;
        rekey(
            "INSERT INTO track_device_statistics (user_id, device_id, track_id, play_count, total_time_listened, \
             completion_count, skip_count, last_played_at, updated_at) \
             SELECT user_id, device_id, ?, play_count, total_time_listened, completion_count, skip_count, \
             last_played_at, updated_at FROM track_device_statistics WHERE track_id = ? \
             ON CONFLICT(user_id, device_id, track_id) DO UPDATE SET \
             play_count = play_count + excluded.play_count, \
             total_time_listened = total_time_listened + excluded.total_time_listened, \
             completion_count = completion_count + excluded.completion_count, \
             skip_count = skip_count + excluded.skip_count, \
             last_played_at = NULLIF(MAX(COALESCE(last_played_at, 0), COALESCE(excluded.last_played_at, 0)), 0), \
             updated_at = MAX(updated_at, excluded.updated_at)",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM track_device_statistics WHERE track_id = ?")
            .bind(&local_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM tracks WHERE id = ?").bind(&local_id).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(Some((local_id, online_id)))
    }
}
