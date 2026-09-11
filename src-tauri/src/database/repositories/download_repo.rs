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
    async fn delete_task(&self, id: &str) -> AppResult<()>;
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
                id, provider, provider_task_id, title, artist, album,
                filename, destination_path, file_size, bytes_downloaded,
                status, error_message, wishlist_id, created_at, completed_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&task.id)
        .bind(&task.provider)
        .bind(&task.provider_task_id)
        .bind(&task.title)
        .bind(&task.artist)
        .bind(&task.album)
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
            "SELECT id, provider, provider_task_id, title, artist, album,
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
                "SELECT id, provider, provider_task_id, title, artist, album,
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
                "SELECT id, provider, provider_task_id, title, artist, album,
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

    async fn delete_task(&self, id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM download_tasks WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }
}
