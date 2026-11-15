//! Queue edits and queue-updated events.
use super::*;

impl PlaybackService {
    /// Queues a song for the app (collection remainders): after everything queued.
    pub async fn enqueue_track(&self, track_id: &str, play_next: bool) -> AppResult<bool> {
        self.enqueue_track_as(track_id, play_next, false).await
    }

    /// Queues a song. A `manual` song (picked by the user) plays before the
    /// app's own queue, after songs the user queued earlier.
    pub async fn enqueue_track_as(&self, track_id: &str, play_next: bool, manual: bool) -> AppResult<bool> {
        self.prepare_track_source(track_id).await?;
        let track = self.find_or_register_track(track_id).await?;

        let queue_item = QueueItem {
            queue_id: Uuid::new_v4().to_string(),
            track_id: track.id,
            title: track.title,
            artist: track.artist_name.unwrap_or_else(|| "Unknown Artist".into()),
            duration_secs: track.duration_secs,
        };

        let _operation = self.flow_operation.lock().await;
        let queue_id = queue_item.queue_id.clone();
        let added = {
            let mut q_guard = self.queue.write().await;
            if manual && !play_next {
                q_guard.enqueue_manual(queue_item)
            } else {
                q_guard.enqueue(queue_item, play_next)
            }
        };
        // "Play next" is always the user's choice, so Flow treats it as theirs too.
        if added && (manual || play_next) {
            if let Some(flow) = self.active_flow.write().await.as_mut() {
                flow.manual_queue_ids.insert(queue_id);
            }
        }

        if added {
            self.emit_queue_updated().await;
        }
        Ok(added)
    }

    pub async fn dequeue_track(&self, track_id: &str) -> bool {
        let removed = {
            let mut q_guard = self.queue.write().await;
            q_guard.remove_track(track_id)
        };
        if removed {
            self.emit_queue_updated().await;
        }
        removed
    }

    pub async fn refresh_queue_metadata(&self) {
        let ids: std::collections::HashSet<String> = self
            .queue
            .read()
            .await
            .items()
            .iter()
            .map(|item| item.track_id.clone())
            .collect();
        let mut changed = false;
        for id in ids {
            if let Ok(Some(track)) = self.track_repo.find_by_id(&id).await {
                changed |= self.queue.write().await.update_metadata(
                    &id,
                    &track.title,
                    track.artist_name.as_deref().unwrap_or("Unknown Artist"),
                );
            }
        }
        if changed {
            self.emit_queue_updated().await;
        }
    }

    pub async fn clear_queue(&self) {
        let _operation = self.flow_operation.lock().await;
        *self.restored_track.write().await = None;
        self.stop_flow_inner().await;
        {
            let mut q_guard = self.queue.write().await;
            q_guard.clear();
        }
        self.emit_queue_updated().await;
        if let Err(error) = self.save_playback_bookmark().await {
            tracing::warn!(%error, "Could not save cleared queue");
        }
    }

    pub async fn set_shuffle(&self, enabled: bool) {
        {
            let mut q_guard = self.queue.write().await;
            q_guard.set_shuffle(enabled);
        }
        self.emit_queue_updated().await;
    }

    pub async fn set_repeat_mode(&self, mode: RepeatMode) {
        {
            let mut q_guard = self.queue.write().await;
            q_guard.set_repeat_mode(mode);
        }
        self.emit_queue_updated().await;
    }

    pub(super) async fn emit_queue_updated(&self) {
        // Archive queues can hold the whole library and are built from fresh
        // metadata, so skip the per-track refresh that would cost one lookup each.
        let source = self.current_source.read().await.clone();
        let ids: HashSet<String> = if source == "archive" { HashSet::new() } else { self
            .queue
            .read()
            .await
            .items()
            .iter()
            .map(|item| item.track_id.clone())
            .collect() };
        for id in ids {
            if let Ok(Some(track)) = self.track_repo.find_by_id(&id).await {
                self.queue.write().await.update_source_metadata(
                    &id,
                    &track.title,
                    track.artist_name.as_deref().unwrap_or("Unknown Artist"),
                    track.duration_secs,
                );
            }
        }
        let (items, current_index, queue_track_ids, user_queue_track_ids) = {
            let q_guard = self.queue.read().await;
            let has_active_track = q_guard.current_index().is_some();

            let queue_track_ids: Vec<String> = if has_active_track {
                if let Some(curr) = q_guard.current_index() {
                    q_guard
                        .items()
                        .iter()
                        .skip(curr + 1)
                        .map(|i| i.track_id.clone())
                        .collect()
                } else {
                    q_guard.items().iter().map(|i| i.track_id.clone()).collect()
                }
            } else {
                q_guard.items().iter().map(|i| i.track_id.clone()).collect()
            };

            (
                q_guard.items().to_vec(),
                q_guard.current_index(),
                queue_track_ids,
                q_guard.waiting_manual_track_ids(),
            )
        };

        let _ = self.event_bus.publish(Event::QueueUpdated {
            items: Arc::new(items),
            current_index,
            queue_track_ids: Arc::new(queue_track_ids),
            user_queue_track_ids: Arc::new(user_queue_track_ids),
            source,
        });

        self.trigger_next_track_preresolution().await;
    }

    /// Speculatively pre-resolves the next upcoming track in the background.
    ///
    /// Guardrails:
    /// - Peeks the next item using `PlaybackQueue::peek_next()`, honoring normal/shuffle/repeat.
    /// - Only resolves the audio source (URL / local path) and caches it in memory.
    /// - Never starts downloading audio, creating `.part` files, or mutating playback.
    /// - Previous speculative resolution tasks are cancelled if still running.
    pub async fn trigger_next_track_preresolution(&self) {
        let (next_item, is_active) = {
            let q = self.queue.read().await;
            (q.peek_next().cloned(), q.current_index().is_some())
        };

        if !is_active {
            return;
        }

        let Some(item) = next_item else {
            return;
        };

        let sm_opt = self.stream_manager.read_or_recover().clone();
        let Some(sm) = sm_opt else {
            return;
        };

        // Abort previous in-flight pre-resolution task if still running
        let mut handle_guard = self.preresolution_handle.lock().await;
        if let Some(prev) = handle_guard.take() {
            prev.abort();
        }

        let task = tokio::spawn(async move {
            debug!(track_id = %item.track_id, title = %item.title, artist = %item.artist, "Background pre-resolving next track audio source");
            let _ = sm
                .resolve_source_only(&item.track_id, &item.title, &item.artist)
                .await;
        });

        *handle_guard = Some(task);
    }
}
