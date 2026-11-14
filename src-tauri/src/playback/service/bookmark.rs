//! Saving and restoring the playback position across restarts.
use super::*;

impl PlaybackService {
    pub async fn initialize_saved_playback(&self) {
        self.restore_ready
            .get_or_init(|| async {
                if let Err(error) = self.restore_saved_playback().await {
                    tracing::warn!(%error, "Could not restore playback bookmark; starting empty");
                }
            })
            .await;
    }

    pub(super) async fn restore_saved_playback(&self) -> AppResult<()> {
        let Some(pool) = &self.pool else {
            return Ok(());
        };
        let Some(value) = SqliteSettingsRepository::new(pool.clone())
            .get_setting(SETTING_KEY)
            .await?
        else {
            return Ok(());
        };
        let mut saved: SavedPlayback = serde_json::from_str(&value)
            .map_err(|_| AppError::Validation("Invalid saved playback bookmark".into()))?;
        if !saved.valid() {
            return Err(AppError::Validation(
                "Invalid saved playback bookmark".into(),
            ));
        }
        if let Some(catalog) = self.online_catalog() {
            let values = catalog
                .present(
                    serde_json::to_value(saved.queue.items())
                        .map_err(|e| AppError::Internal(e.to_string()))?,
                )
                .await?;
            let items =
                serde_json::from_value(values).map_err(|e| AppError::Internal(e.to_string()))?;
            saved.queue.reconcile_sources(items);
            saved.has_current_track = saved.has_current_track && saved.queue.current().is_some();
            if let Some(current) = saved.queue.current() {
                saved.duration_secs = current.duration_secs;
            }
        }
        let _operation = self.flow_operation.lock().await;
        self.backend.lock().await.set_volume(saved.volume)?;
        *self.volume.write().await = saved.volume;
        if saved.has_current_track {
            // Do not open files or resolve online sources until the user plays.
            // Missing media remains visible and can be retried or skipped.
            *self.restored_track.write().await = Some(RestoredTrack {
                queue_id: saved.queue.current().unwrap().queue_id.clone(),
                position_secs: if saved.duration_secs > 0.0 { saved.position_secs.min(saved.duration_secs) } else { saved.position_secs },
                duration_secs: saved.duration_secs,
            });
        }
        *self.queue.write().await = saved.queue;
        *self.last_saved_bookmark.lock().await = Some(value);
        Ok(())
    }

    /// Called under the queue operation lock, periodically and before clean Stop.
    pub(super) async fn save_playback_bookmark(&self) -> AppResult<()> {
        if self.restore_ready.get().is_none()
            || self.is_resolving.load(std::sync::atomic::Ordering::Acquire)
        {
            return Ok(());
        }
        let Some(pool) = &self.pool else {
            return Ok(());
        };
        let state = self.get_playback_state().await;
        let saved = SavedPlayback {
            version: 1,
            queue: self.queue.read().await.clone(),
            has_current_track: state.current_track_id.is_some(),
            position_secs: state.position_secs,
            duration_secs: state.duration_secs,
            volume: state.volume,
        };
        if !saved.valid() {
            return Ok(());
        }
        let value = serde_json::to_string(&saved).map_err(|e| AppError::Internal(e.to_string()))?;
        let mut previous = self.last_saved_bookmark.lock().await;
        if previous.as_ref() == Some(&value) {
            return Ok(());
        }
        SqliteSettingsRepository::new(pool.clone())
            .set_setting(SETTING_KEY, &value)
            .await?;
        *previous = Some(value);
        Ok(())
    }
}
