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
        let settings = SqliteSettingsRepository::new(pool.clone());
        let Some(value) = settings.get_setting(SETTING_KEY).await? else {
            return Ok(());
        };
        let mut saved: SavedPlayback = serde_json::from_str(&value)
            .map_err(|_| AppError::Validation("Invalid saved playback bookmark".into()))?;
        if let Some(position) = settings.get_setting(POSITION_KEY).await? {
            if let Ok(position) = serde_json::from_str::<SavedPosition>(&position) {
                position.apply_to(&mut saved);
            }
        }
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
        let mut saved = SavedPlayback {
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
        let settings = SqliteSettingsRepository::new(pool.clone());
        let position = saved.queue.current().map(|item| SavedPosition {
            queue_id: item.queue_id.clone(),
            position_secs: saved.position_secs,
            duration_secs: saved.duration_secs,
        });
        // The snapshot without its position: unchanged while only time passes.
        let full = serde_json::to_string(&saved).map_err(|e| AppError::Internal(e.to_string()))?;
        (saved.position_secs, saved.duration_secs) = (0.0, 0.0);
        let shape = serde_json::to_string(&saved).map_err(|e| AppError::Internal(e.to_string()))?;
        let mut previous = self.last_saved_bookmark.lock().await;
        if previous.as_ref() != Some(&shape) {
            settings.set_setting(SETTING_KEY, &full).await?;
            *previous = Some(shape);
        }
        drop(previous);
        let position = serde_json::to_string(&position).map_err(|e| AppError::Internal(e.to_string()))?;
        let mut previous = self.last_saved_position.lock().await;
        if previous.as_ref() != Some(&position) {
            settings.set_setting(POSITION_KEY, &position).await?;
            *previous = Some(position);
        }
        Ok(())
    }
}
