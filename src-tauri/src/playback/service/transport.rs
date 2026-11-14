//! Transport controls (pause, resume, seek, volume, next, previous, stop) and auto-advance.
use super::*;

impl PlaybackService {
    pub async fn pause(&self) -> AppResult<()> {
        let mut backend = self.backend.lock().await;
        backend.pause()?;

        let pos = backend.position().as_secs_f64();
        let track_id = {
            let q_guard = self.queue.read().await;
            q_guard
                .current()
                .map(|i| i.track_id.clone())
                .unwrap_or_default()
        };

        let session_id = self
            .active_session_id
            .read()
            .await
            .as_ref()
            .map(|(id, _)| id.clone())
            .unwrap_or_default();

        let _ = self.event_bus.publish(Event::PlaybackPaused {
            session_id,
            track_id,
            position_secs: pos,
        });
        Ok(())
    }

    pub async fn resume(&self) -> AppResult<()> {
        if self.queue.read().await.current().is_none() {
            return Err(AppError::Playback("Select a song to play".into()));
        }
        let restored = self.restored_track.read().await.clone();
        if let Some(restored) = restored {
            let item = self
                .queue
                .read()
                .await
                .current()
                .filter(|item| item.queue_id == restored.queue_id)
                .cloned();
            if let Some(item) = item {
                return self.play_item_at(&item, Some(restored.position_secs)).await;
            }
        }
        let mut backend = self.backend.lock().await;
        if self
            .shutting_down
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(AppError::Playback("Kaze is shutting down".into()));
        }

        let track_id = {
            let q_guard = self.queue.read().await;
            q_guard
                .current()
                .map(|i| i.track_id.clone())
                .ok_or_else(|| AppError::Playback("Select a song to play".into()))?
        };
        if self.active_session_id.read().await.is_none() {
            return Err(AppError::Playback("Select a song to play".into()));
        }
        backend.resume()?;
        let pos = backend.position().as_secs_f64();

        let session_id = self
            .active_session_id
            .read()
            .await
            .as_ref()
            .map(|(id, _)| id.clone())
            .unwrap_or_default();

        let _ = self.event_bus.publish(Event::PlaybackResumed {
            session_id: session_id.clone(),
            track_id,
            position_secs: pos,
        });
        if let Some(state) = self.active_stream_state.read_or_recover().as_ref() {
            state.activate_playback(session_id);
        }
        Ok(())
    }

    pub async fn resume_needs_loading(&self) -> bool {
        self.restored_track.read().await.is_some()
    }

    /// Cancel pending media preparation before the core waits for commands to drain.
    pub fn request_shutdown(&self) {
        self.interrupt_preparation();
        // Command draining may be waiting for a seek on the audio thread.
        // Wake that reader before shutdown waits for in-flight commands.
        if let Some(state) = self.active_stream_state.read_or_recover().as_ref() {
            state.deactivate_playback();
            state.set_cancelled();
        }
        if !self
            .shutting_down
            .swap(true, std::sync::atomic::Ordering::AcqRel)
        {
            self.play_generation
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    /// Stop the monitor and invalidate pending resolution before stopping audio.
    pub async fn shutdown(&self) -> AppResult<()> {
        let mut done = self.shutdown_done.lock().await;
        if *done {
            return Ok(());
        }
        self.request_shutdown();
        let task = self
            .monitor_task
            .lock_or_recover()
            .take();
        if let Some(task) = task {
            task.abort();
            let _ = task.await;
        }
        let session_task = self
            .session_task
            .lock_or_recover()
            .take();
        if let Some(task) = session_task {
            task.abort();
            let _ = task.await;
        }
        let bookmark = {
            let _operation = self.flow_operation.lock().await;
            self.save_playback_bookmark().await
        };
        let result = self.stop().await;
        *done = true;
        result.and(bookmark)
    }

    pub async fn stop(&self) -> AppResult<()> {
        self.interrupt_preparation();
        let _operation = self.flow_operation.lock().await;
        self.stop_inner().await
    }

    pub(super) async fn stop_inner(&self) -> AppResult<()> {
        *self.restored_track.write().await = None;
        self.stop_flow_inner().await;
        self.play_generation
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.is_resolving
            .store(false, std::sync::atomic::Ordering::SeqCst);
        if let Some(task) = self.preresolution_handle.lock().await.take() {
            task.abort();
            let _ = task.await;
        }
        self.cancel_active_stream().await;
        let mut backend = self.backend.lock().await;
        backend.stop()?;
        *self.current_duration_secs.write().await = 0.0;

        let sm_opt = self.stream_manager.read_or_recover().clone();
        if let Some(sm) = sm_opt {
            sm.set_active_playing_path(None).await;
        }

        // Take (clear) the active session id — this session is ending.
        let session_id = {
            let mut guard = self.active_session_id.write().await;
            guard.take().map(|(id, _)| id).unwrap_or_default()
        };

        let _ = self
            .event_bus
            .publish(Event::PlaybackStopped { session_id });
        self.emit_queue_updated().await;
        Ok(())
    }

    pub async fn seek(&self, position_secs: f64) -> AppResult<()> {
        if !position_secs.is_finite() {
            return Err(AppError::Validation("Seek position must be finite".into()));
        }
        {
            let mut restored = self.restored_track.write().await;
            if let Some(restored) = restored.as_mut() {
                restored.position_secs = if restored.duration_secs > 0.0 { position_secs.clamp(0.0, restored.duration_secs) } else { position_secs.max(0.0) };
                let _ = self.event_bus.publish(Event::PlaybackSeeked {
                    position_secs: restored.position_secs,
                });
                return Ok(());
            }
        }
        let can_seek = match &*self.active_stream_state.read_or_recover() {
            Some(state) => state.can_seek(),
            None => true,
        };
        if !can_seek {
            return Err(AppError::Playback(
                "Seeking is unavailable for this audio stream.".into(),
            ));
        }

        let dur = Duration::try_from_secs_f64(position_secs.max(0.0))
            .map_err(|_| AppError::Validation("Seek position is out of range".into()))?;
        let mut backend = self.backend.lock().await;
        backend.seek(dur)?;

        let _ = self
            .event_bus
            .publish(Event::PlaybackSeeked { position_secs });
        Ok(())
    }

    pub async fn set_volume(&self, volume: f32) -> AppResult<()> {
        let clamped = volume.clamp(0.0, 1.0);
        *self.volume.write().await = clamped;

        let mut backend = self.backend.lock().await;
        backend.set_volume(clamped)?;

        let _ = self.event_bus.publish(Event::PlaybackVolumeChanged {
            volume: clamped,
            is_muted: clamped == 0.0,
        });
        Ok(())
    }

    pub async fn next(&self) -> AppResult<()> {
        self.interrupt_preparation();
        let operation = self.flow_operation.lock().await;
        let mut next_item = {
            let mut q_guard = self.queue.write().await;
            q_guard.skip_forward().cloned()
        };

        next_item = self.skip_blocked_queue_items(next_item).await?;
        if next_item.is_none() {
            let src = self.current_source.read().await.clone();
            if src == "flow" {
                let fe_opt = self.flow_engine.read_or_recover().clone();
                if let Some(fe) = fe_opt {
                    next_item = self.refill_flow_queue(&fe).await?;
                }
            }
            if next_item.is_none() && src != "flow" && src != "archive" {
                next_item = self.refill_random_queue().await?;
            }
        }

        if let Some(item) = next_item {
            drop(operation);
            self.play_item(&item).await?;
        } else {
            self.stop_inner().await?;
        }
        Ok(())
    }

    pub(super) async fn advance_after_finished(
        &self,
        generation: u64,
        queue_id: Option<&str>,
    ) -> AppResult<()> {
        let operation = self.flow_operation.lock().await;
        if self
            .shutting_down
            .load(std::sync::atomic::Ordering::Acquire)
            || self
                .play_generation
                .load(std::sync::atomic::Ordering::SeqCst)
                != generation
            || self
                .queue
                .read()
                .await
                .current()
                .map(|i| i.queue_id.as_str())
                != queue_id
        {
            return Ok(());
        }
        let mut next_item = {
            let mut q_guard = self.queue.write().await;
            q_guard.next_after_finish().cloned()
        };

        next_item = self.skip_blocked_queue_items(next_item).await?;
        if next_item.is_none() {
            let src = self.current_source.read().await.clone();
            if src == "flow" {
                let fe_opt = self.flow_engine.read_or_recover().clone();
                if let Some(fe) = fe_opt {
                    next_item = self.refill_flow_queue(&fe).await?;
                }
            }
            if next_item.is_none() && src != "flow" && src != "archive" {
                next_item = self.refill_random_queue().await?;
            }
        }

        if let Some(item) = next_item {
            drop(operation);
            self.play_item(&item).await?;
        } else {
            self.stop_inner().await?;
        }
        Ok(())
    }

    pub(super) async fn refill_random_queue(&self) -> AppResult<Option<QueueItem>> {
        let current_track_id = {
            let q_guard = self.queue.read().await;
            q_guard.current().map(|item| item.track_id.clone())
        };
        let linked: HashSet<String> = if let Some(pool) = &self.pool {
            sqlx::query_scalar("SELECT id FROM external_tracks WHERE provider='jiosaavn' UNION SELECT track_id FROM online_track_sources").fetch_all(pool).await?.into_iter().collect()
        } else {
            HashSet::new()
        };
        let feedback = if let Some(pool) = &self.pool {
            Some(
                crate::recommendations::signals::SongSignals::load(
                    pool,
                    &self.recommendation_user_id().await,
                )
                .await?,
            )
        } else {
            None
        };
        let mut candidates = self
            .track_repo
            .list_tracks(0, 10_000, None, true)
            .await?
            .into_iter()
            .filter(|track| current_track_id.as_deref() != Some(track.id.as_str()))
            .filter(|track| {
                !feedback.as_ref().is_some_and(|feedback| {
                    feedback.blocked(
                        &track.id,
                        &track.title,
                        track.artist_name.as_deref().unwrap_or(""),
                    )
                })
            })
            .filter(|track| !track.file_path.starts_with("online://") || linked.contains(&track.id))
            .collect::<Vec<_>>();
        candidates.shuffle(&mut thread_rng());
        candidates.truncate(50);

        let items = candidates
            .into_iter()
            .map(|track| QueueItem {
                queue_id: Uuid::new_v4().to_string(),
                track_id: track.id,
                title: track.title,
                artist: track.artist_name.unwrap_or_else(|| "Unknown Artist".into()),
                duration_secs: track.duration_secs,
            })
            .collect::<Vec<_>>();

        if items.is_empty() {
            return Ok(None);
        }

        let next = {
            let mut q_guard = self.queue.write().await;
            for item in items {
                q_guard.enqueue(item, false);
            }
            q_guard.skip_forward().cloned()
        };
        self.emit_queue_updated().await;
        Ok(next)
    }

    pub async fn previous(&self) -> AppResult<()> {
        self.interrupt_preparation();
        let operation = self.flow_operation.lock().await;
        let prev_item = {
            let mut q_guard = self.queue.write().await;
            q_guard.previous().cloned()
        };

        drop(operation);
        if let Some(item) = prev_item {
            self.play_item(&item).await?;
        }
        Ok(())
    }
}
