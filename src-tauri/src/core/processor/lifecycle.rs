//! Startup dates, close-button behaviour, background sync and shutdown.

use super::*;

impl CoreProcessor {
    pub(super) async fn get_or_init_app_start_date(&self) -> i64 {
        let mut guard = self.app_start_date.write().await;
        if *guard > 0 {
            return *guard;
        }
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM application_settings WHERE key = 'app_first_start_date'",
        )
        .fetch_optional(&self.db_pool)
        .await
        .ok()
        .flatten();

        if let Some((val,)) = row {
            if let Ok(ts) = val.parse::<i64>() {
                *guard = ts;
                return ts;
            }
        }

        let now = chrono::Utc::now().timestamp();
        let _ = sqlx::query(
            "INSERT INTO application_settings (key, value, updated_at) VALUES ('app_first_start_date', ?, ?)
             ON CONFLICT(key) DO NOTHING"
        )
        .bind(now.to_string())
        .bind(now)
        .execute(&self.db_pool)
        .await;

        *guard = now;
        now
    }

    /// Retrieve the configured Cloudflare Worker URL (or fallback to production instance default)
    pub async fn get_cloud_worker_url(&self) -> String {
        self.settings_repo
            .get_setting("cloud_sync_url")
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| {
                "https://soundflow-cloud-worker.abhi-atlas-2026.workers.dev".to_string()
            })
    }

    pub fn is_quitting(&self) -> bool {
        self.is_quitting.load(Ordering::Acquire)
    }

    /// Returns true only for the first request; quitting cannot be undone.
    pub fn begin_quit(&self) -> bool {
        let first = !self.is_quitting.swap(true, Ordering::AcqRel);
        self.playback_service.request_shutdown();
        first
    }

    pub fn close_button_behavior(&self) -> &'static str {
        if self.close_button_quits.load(Ordering::Acquire) {
            "quit_app"
        } else {
            "minimize_to_tray"
        }
    }

    /// Awaited at startup and by settings IPC. A late startup read cannot
    /// overwrite a newer setting. Missing or invalid DB values mean hide-to-tray.
    pub async fn initialize_close_behavior(&self) -> AppResult<()> {
        self.close_behavior_ready
            .get_or_try_init(|| async {
                let saved = self
                    .settings_repo
                    .get_setting("close_button_behavior")
                    .await?;
                self.close_button_quits
                    .store(saved.as_deref() == Some("quit_app"), Ordering::Release);
                Ok::<(), AppError>(())
            })
            .await?;
        Ok(())
    }

    pub async fn set_close_button_behavior(&self, behavior: String) -> AppResult<()> {
        if behavior != "minimize_to_tray" && behavior != "quit_app" {
            return Err(AppError::Validation(format!(
                "Invalid close_button_behavior: '{}'. Allowed: 'minimize_to_tray', 'quit_app'",
                behavior
            )));
        }
        self.initialize_close_behavior().await?;
        let _update = self.close_behavior_updates.lock().await;
        self.settings_repo
            .set_setting("close_button_behavior", &behavior)
            .await?;
        self.close_button_quits
            .store(behavior == "quit_app", Ordering::Release);
        self.config.write().await.close_button_behavior = behavior;
        Ok(())
    }

    /// Quiesce commands, stop audio once, await history writes, then close SQLite.
    pub async fn shutdown(&self) -> AppResult<()> {
        let mut done = self.shutdown_done.lock().await;
        if *done {
            return Ok(());
        }
        self.begin_quit();
        info!("Shutdown: waiting for commands after playback cancellation");
        let _commands = self.command_gate.write().await;
        info!("Shutdown: commands drained; persisting playback/history");
        let tasks = std::mem::take(
            &mut *self
                .background_tasks
                .lock_or_recover(),
        );
        for task in &tasks {
            task.abort();
        }
        for task in tasks {
            let _ = task.await;
        }
        self.history_service.begin_shutdown();
        let stopped = self.playback_service.shutdown().await;
        self.history_service.shutdown().await;
        self.db_pool.close().await;
        info!("Shutdown: history and database closed");
        *done = true;
        stopped
    }

    /// Trigger background push sync for the active user if authenticated.
    pub async fn trigger_background_sync(&self) {
        let user_id = {
            let current_user_guard = self.current_user.read().await;
            current_user_guard.as_ref().map(|u| u.id.clone())
        };
        if let Some(uid) = user_id {
            let pool_clone = self.db_pool.clone();
            let client_clone = self.cloud_client.clone();
            let worker_url = self.get_cloud_worker_url().await;
            tokio::spawn(async move {
                if let Some(token) = credentials::get_session_token(&uid) {
                    if let Ok(payload) =
                        SyncManager::prepare_local_sync_payload(&pool_clone, &uid).await
                    {
                        if let Ok(synced_at) =
                            client_clone.push_sync(&worker_url, &token, &payload).await
                        {
                            let _ = SyncManager::clear_tombstones(&pool_clone, &uid).await;
                            let _ =
                                SyncManager::update_session_synced_at(&pool_clone, &uid, synced_at)
                                    .await;
                        }
                    }
                }
            });
        }
    }
}
