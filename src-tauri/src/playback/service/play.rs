//! Starting playback of a track, a list, an online song or a queue position.
use super::*;

impl PlaybackService {
    /// Plays a track directly, clearing existing queue or adding to top.
    pub async fn play_track(&self, track_id: &str, source: Option<String>) -> AppResult<()> {
        self.interrupt_preparation();
        self.prepare_track_source(track_id).await?;
        let track = self.find_or_register_track(track_id).await?;

        let operation = self.flow_operation.lock().await;
        self.stop_flow_inner().await;
        let src = source.unwrap_or_else(|| "library".to_string());
        let previous_src = self.current_source.read().await.clone();
        *self.current_source.write().await = src.clone();
        *self.current_duration_secs.write().await = track.duration_secs;

        let artist_name = track.artist_name.unwrap_or_else(|| "Unknown Artist".into());
        let queue_item = QueueItem {
            queue_id: Uuid::new_v4().to_string(),
            track_id: track.id.clone(),
            title: track.title.clone(),
            artist: artist_name.clone(),
            duration_secs: track.duration_secs,
        };

        {
            let mut q_guard = self.queue.write().await;
            if src == "collection" || src == "playlist" {
                q_guard.set_queue(vec![queue_item.clone()], Some(0));
            } else {
                let preserve_history = previous_src != "collection" && previous_src != "playlist";
                q_guard.play_independent(queue_item.clone(), preserve_history);
            }
        }

        drop(operation);
        self.play_item(&queue_item).await
    }

    /// Plays an ordered list of local tracks (Downloads "Play all" / "Shuffle").
    /// The queue is the list itself: it plays in order and stops at the end
    /// (source "archive" never auto-refills with random tracks).
    pub async fn play_track_list(
        &self,
        track_ids: Option<Vec<String>>,
        start_track_id: Option<String>,
        shuffle: bool,
        scope: &crate::database::repositories::LibraryScope,
    ) -> AppResult<()> {
        self.interrupt_preparation();
        let ids: Vec<String> = match track_ids {
            Some(ids) => ids,
            None => match &self.pool {
                Some(pool) => {
                    // Same order and section as the list on screen.
                    let condition = scope.condition("file_path");
                    let sql = format!(
                        "SELECT id FROM tracks WHERE file_path NOT LIKE 'online://%'{} ORDER BY title ASC, id ASC",
                        condition.as_ref().map(|(sql, ..)| format!(" AND {sql}")).unwrap_or_default()
                    );
                    let mut query = sqlx::query_scalar(&sql);
                    if let Some((_, len, prefix)) = &condition {
                        query = query.bind(*len).bind(prefix.clone());
                    }
                    query
                        .fetch_all(pool)
                        .await
                        .map_err(|e| AppError::Database(e.to_string()))?
                }
                None => Vec::new(),
            },
        };

        let mut items: Vec<QueueItem> = Vec::with_capacity(ids.len());
        for id in &ids {
            if let Some(track) = self.track_repo.find_by_id(id).await? {
                items.push(QueueItem {
                    queue_id: Uuid::new_v4().to_string(),
                    track_id: track.id.clone(),
                    title: track.title.clone(),
                    artist: track.artist_name.clone().unwrap_or_else(|| "Unknown Artist".into()),
                    duration_secs: track.duration_secs,
                });
            }
        }
        if items.is_empty() {
            return Err(AppError::NotFound("No tracks to play".into()));
        }

        *self.archive_order.write().await = items.iter().map(|i| i.track_id.clone()).collect();
        let mut start_idx = start_track_id
            .as_ref()
            .and_then(|sid| items.iter().position(|i| &i.track_id == sid))
            .unwrap_or(0);
        if shuffle {
            items.shuffle(&mut thread_rng());
            if let Some(sid) = start_track_id.as_ref() {
                if let Some(pos) = items.iter().position(|i| &i.track_id == sid) {
                    items.swap(0, pos);
                }
            }
            start_idx = 0;
        }

        let first = items[start_idx].clone();
        self.prepare_track_source(&first.track_id).await?;

        let operation = self.flow_operation.lock().await;
        self.stop_flow_inner().await;
        *self.current_source.write().await = "archive".to_string();
        *self.current_duration_secs.write().await = first.duration_secs;
        {
            let mut q_guard = self.queue.write().await;
            q_guard.set_queue(items, Some(start_idx));
            // The list owns its own shuffle (SetArchiveShuffle); never inherit the
            // queue-wide shuffle left over from another section.
            q_guard.set_shuffle(false);
        }
        drop(operation);
        self.play_item(&first).await
    }

    /// Shuffles (or restores library order of) the songs after the current one in
    /// the Downloads list. Only affects source "archive"; the playing song is untouched.
    pub async fn set_archive_shuffle(&self, enabled: bool) {
        if *self.current_source.read().await != "archive" {
            return;
        }
        let order = self.archive_order.read().await.clone();
        {
            let mut q = self.queue.write().await;
            q.reorder_upcoming(|upcoming| {
                if enabled {
                    upcoming.shuffle(&mut thread_rng());
                } else {
                    let rank = |id: &str| order.iter().position(|o| o == id).unwrap_or(usize::MAX);
                    upcoming.sort_by_key(|item| rank(&item.track_id));
                }
            });
        }
        self.emit_queue_updated().await;
    }

    /// Plays an online or remote track, ensuring it is registered and streamed through the unified engine.
    pub async fn play_online_track(
        &self,
        track: &crate::recommendations::mixes::OnlineTrackInput<'_>,
        source: Option<String>,
    ) -> AppResult<()> {
        if let Some(pool) = &self.pool {
            crate::recommendations::mixes::ensure_online_track(pool, track).await?;
        }

        self.play_track(track.id, source).await
    }

    /// Plays an item from the current queue by index.
    pub async fn play_queue_index(&self, index: usize) -> AppResult<()> {
        self.interrupt_preparation();
        let operation = self.flow_operation.lock().await;
        let item = {
            let mut q_guard = self.queue.write().await;
            q_guard.set_current_index(index).cloned()
        };

        drop(operation);
        if let Some(item) = item {
            self.play_item(&item).await?;
        }
        Ok(())
    }

    pub(super) async fn play_item(&self, item: &QueueItem) -> AppResult<()> {
        self.play_item_at(item, None).await
    }

    pub(super) async fn play_item_at(&self, item: &QueueItem, start_position: Option<f64>) -> AppResult<()> {
        self.interrupt_preparation();
        self.prepare_track_source(&item.track_id).await?;
        let operation = self.flow_operation.lock().await;
        if self
            .shutting_down
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(AppError::Playback("Kaze is shutting down".into()));
        }

        if self.queue.read().await.current().map(|i| &i.queue_id) != Some(&item.queue_id) {
            return Ok(());
        }
        if start_position.is_none() {
            *self.restored_track.write().await = None;
        }
        let generation = self
            .play_generation
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            + 1;
        self.is_resolving
            .store(true, std::sync::atomic::Ordering::SeqCst);

        let track = match self.track_repo.find_by_id(&item.track_id).await {
            Ok(Some(t)) => t,
            Ok(None) => {
                self.is_resolving
                    .store(false, std::sync::atomic::Ordering::SeqCst);
                return Err(AppError::NotFound(format!(
                    "Track not found: {}",
                    item.track_id
                )));
            }
            Err(e) => {
                self.is_resolving
                    .store(false, std::sync::atomic::Ordering::SeqCst);
                return Err(e);
            }
        };

        // Wake a decoder waiting for seek data before taking its backend lock.
        self.cancel_active_stream().await;

        // Immediately stop backend playback and reset current duration / session
        // so stale audio never plays while resolving the new track
        {
            let mut backend = self.backend.lock().await;
            let _ = backend.stop();
        }
        *self.current_duration_secs.write().await = 0.0;
        if let Some((session_id, _)) = self.active_session_id.write().await.take() {
            let _ = self
                .event_bus
                .publish(Event::PlaybackStopped { session_id });
        }

        // Cloud imports may retain an online label for a real local file.
        let has_local_file = PathBuf::from(&track.file_path).is_absolute()
            && PathBuf::from(&track.file_path).is_file();
        let is_online = !has_local_file
            && (track.format == "online" || track.file_path.starts_with("online://"));
        if is_online {
            let _ = self.event_bus.publish(Event::PlaybackBuffering {
                session_id: String::new(),
                track_id: item.track_id.clone(),
            });
        }
        drop(operation);
        let prepared_res = if is_online {
            let sm_opt = self.stream_manager.read_or_recover().clone();
            if let Some(sm) = sm_opt {
                let artist = track.artist_name.as_deref().unwrap_or("Unknown Artist");
                sm.resolve_and_prepare_playback(&track.id, &track.title, artist)
                    .await
            } else {
                Err(AppError::Playback(
                    "Stream manager not initialized for online audio playback".to_string(),
                ))
            }
        } else {
            Ok(crate::playback::stream::PreparedPlayback::LocalFile {
                path: PathBuf::from(&track.file_path),
                duration: track.duration_secs,
            })
        };

        // A saved online position requires a seekable source. A cold cache may
        // buffer fully here; never silently replay it from the beginning.
        let prepared_res = match prepared_res {
            Ok(crate::playback::stream::PreparedPlayback::ProgressiveStream {
                target_path,
                duration,
                state,
                ..
            }) if start_position.is_some_and(|position| position > 0.0) => {
                let ready = tokio::time::timeout(Duration::from_secs(120), async {
                    loop {
                        if self
                            .play_generation
                            .load(std::sync::atomic::Ordering::SeqCst)
                            != generation
                            || self
                                .shutting_down
                                .load(std::sync::atomic::Ordering::Acquire)
                        {
                            return Err(AppError::Playback("Playback request superseded".into()));
                        }
                        match state.status.lock_or_recover().clone() {
                            crate::playback::progressive::StreamStatus::Completed => return Ok(()),
                            crate::playback::progressive::StreamStatus::Failed(error) => {
                                return Err(AppError::Playback(error))
                            }
                            crate::playback::progressive::StreamStatus::Cancelled => {
                                return Err(AppError::Playback("Stream cancelled".into()))
                            }
                            _ => {}
                        }
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                })
                .await;
                match ready {
                    Ok(Ok(())) => Ok(crate::playback::stream::PreparedPlayback::LocalFile {
                        path: target_path,
                        duration,
                    }),
                    Ok(Err(error)) => Err(error),
                    Err(_) => Err(AppError::Playback(
                        "Timed out buffering the saved song. Try playing again.".into(),
                    )),
                }
            }
            other => other,
        };

        let _operation = self.flow_operation.lock().await;

        // Check if a newer track transition was requested while resolving
        if self
            .play_generation
            .load(std::sync::atomic::Ordering::SeqCst)
            != generation
        {
            debug!(
                item_title = %item.title,
                expected_gen = generation,
                current_gen = self.play_generation.load(std::sync::atomic::Ordering::SeqCst),
                "Aborting obsolete play_item: newer playback request received during resolution"
            );
            return Ok(());
        }

        self.is_resolving
            .store(false, std::sync::atomic::Ordering::SeqCst);
        if self.queue.read().await.current().map(|i| &i.queue_id) != Some(&item.queue_id) {
            return Ok(());
        }
        let prepared = match prepared_res {
            Ok(p) => p,
            Err(e) => {
                let _ = self.event_bus.publish(Event::PlaybackError {
                    session_id: String::new(),
                    message: e.to_string(),
                });
                return Err(e);
            }
        };

        let (file_path_to_protect, actual_duration) = match prepared {
            crate::playback::stream::PreparedPlayback::LocalFile { path, duration } => {
                *self.active_stream_state.write_or_recover() = None;
                let mut backend = self.backend.lock().await;
                backend.load_and_play(&path)?;
                if let Some(position) = start_position {
                    if let Err(error) =
                        backend.seek(Duration::from_secs_f64(if duration > 0.0 { position.min(duration).max(0.0) } else { position.max(0.0) }))
                    {
                        let _ = backend.stop();
                        return Err(error);
                    }
                }
                (path.to_string_lossy().to_string(), duration)
            }
            crate::playback::stream::PreparedPlayback::ProgressiveStream {
                source,
                part_path,
                target_path,
                duration,
                state,
            } => {
                *self.active_stream_state.write_or_recover() = Some(state);
                let mut backend = self.backend.lock().await;
                backend.load_and_play_source(source, Some(&part_path))?;
                (target_path.to_string_lossy().to_string(), duration)
            }
        };

        *self.current_duration_secs.write().await = actual_duration;
        *self.restored_track.write().await = None;

        let sm_opt = self.stream_manager.read_or_recover().clone();
        if let Some(sm) = sm_opt {
            sm.set_active_playing_path(Some(PathBuf::from(&file_path_to_protect)))
                .await;
        }

        // Generate a fresh session token for this playback session.
        let session_id = Uuid::new_v4().to_string();
        *self.active_session_id.write().await = Some((session_id.clone(), item.queue_id.clone()));

        let src = self.current_source.read().await.clone();
        let _ = self.event_bus.publish(Event::PlaybackStarted {
            session_id: session_id.clone(),
            track_id: item.track_id.clone(),
            title: item.title.clone(),
            artist: item.artist.clone(),
            duration_secs: actual_duration,
            source: src,
        });
        if let Some(state) = self.active_stream_state.read_or_recover().as_ref() {
            state.activate_playback(session_id);
        }

        if start_position.is_some() {
            let position_secs = self.backend.lock().await.position().as_secs_f64();
            let _ = self
                .event_bus
                .publish(Event::PlaybackSeeked { position_secs });
        }

        self.emit_queue_updated().await;
        Ok(())
    }
}
