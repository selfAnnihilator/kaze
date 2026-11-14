//! Flow sessions: start, continue, steer and stop.
use super::*;

impl PlaybackService {
    // Serializes generation and authoritative queue mutations, never audio/network
    // resolution. A newer manual selection cannot be overwritten by a stale refill.
    pub async fn start_flow(
        &self,
        fe: &Arc<crate::recommendations::FlowEngine>,
        user_id: &str,
        ratio: Option<f64>,
    ) -> AppResult<()> {
        if ratio.is_some_and(|value| !value.is_finite()) {
            return Err(AppError::Validation("Flow steering must be finite".into()));
        }
        // Invalidate older generation/refill work at request arrival, even while
        // that work holds the operation lock. Audio has its own unchanged token.
        let generation = self
            .flow_generation
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            + 1;
        // Catalog migration runs in the background; the selected recording is
        // bound by play_item. Unrelated legacy songs must not delay Start Flow.
        let preparation = self.starter_preparation.read_or_recover().clone();
        if let Some(coordinator) = preparation {
            coordinator
                .ensure_starter_candidates(user_id, false)
                .await?;
        }
        let operation = self.flow_operation.lock().await;
        if self
            .flow_generation
            .load(std::sync::atomic::Ordering::SeqCst)
            != generation
        {
            return Ok(());
        }
        let persisted = fe.get_flow_session(user_id).await?;
        let ratio = ratio
            .or_else(|| persisted.as_ref().map(|s| s.familiarity_ratio))
            .unwrap_or(0.5);
        let current = self.queue.read().await.current().cloned();
        let preserve_current = if let Some(item) = current.as_ref() {
            let loaded = self.active_session_id.read().await.as_ref()
                .is_some_and(|(_, id)| id == &item.queue_id);
            (loaded && !self.backend.lock().await.is_finished())
                || self.is_resolving.load(std::sync::atomic::Ordering::SeqCst)
        } else {
            false
        };

        // Clear old upcoming entries while preserving the current recording,
        // queue ID, audio token and Previous history.
        *self.active_flow.write().await = None;
        *self.current_source.write().await = "library".into();
        self.queue.write().await.clear_upcoming();
        self.emit_queue_updated().await;
        let exclude = current
            .as_ref()
            .filter(|_| preserve_current)
            .map(|i| i.track_id.clone())
            .into_iter()
            .collect::<Vec<_>>();
        // Artists heard just before Flow starts carry into its diversity window.
        let recent_artists = {
            let q = self.queue.read().await;
            let end = q
                .current_index()
                .map(|index| if preserve_current { index + 1 } else { index })
                .unwrap_or(0)
                .min(q.items().len());
            Self::recent_artist_window(&q.items()[..end])
        };
        let tracks = fe
            .generate_playback_tracks_seeded(
                user_id,
                ratio,
                20,
                &exclude,
                &recent_artists,
                None,
                exclude.first().map(String::as_str),
            )
            .await?;
        if self
            .flow_generation
            .load(std::sync::atomic::Ordering::SeqCst)
            != generation
        {
            return Ok(());
        }
        if tracks.is_empty() && !preserve_current {
            return Err(AppError::Playback(
                "No eligible songs for Flow right now. Try refreshing recommendations, check your connection, or select a song.".into(),
            ));
        }
        let state = Self::flow_settings(ratio, false);
        fe.save_flow_session(user_id, &state).await?;
        if self
            .flow_generation
            .load(std::sync::atomic::Ordering::SeqCst)
            != generation
        {
            return Ok(());
        }
        let mut flow = ActiveFlow {
            generation,
            user_id: user_id.into(),
            ratio: state.familiarity_ratio,
            ..Default::default()
        };
        let mut first = None;
        {
            let mut q = self.queue.write().await;
            for t in tracks {
                let item = Self::flow_item(&t);
                let id = item.queue_id.clone();
                let added = if !preserve_current && first.is_none() {
                    q.play_independent(item.clone(), true);
                    first = Some(item);
                    true
                } else {
                    q.enqueue(item, false)
                };
                if added {
                    flow.generated_queue_ids.insert(id);
                    flow.reasons.insert(t.track_id, t.explanation);
                }
            }
        }
        *self.current_source.write().await = "flow".into();
        *self.active_flow.write().await = Some(flow);
        self.emit_queue_updated().await;
        // Start is a playback action, including after Stop Flow. Resume a loaded
        // paused session in place; do not create another history/stats session.
        // A resolving song already has an active play request. Restored bookmarks
        // go through play_item above instead, avoiding a recursive operation lock.
        if preserve_current
            && !self.is_resolving.load(std::sync::atomic::Ordering::SeqCst)
            && self.restored_track.read().await.is_none()
        {
            let paused = {
                let backend = self.backend.lock().await;
                backend.is_paused() && !backend.is_finished()
            };
            if paused {
                self.resume().await?;
            }
        }
        drop(operation);
        if let Some(first) = first {
            if let Err(error) = self.play_item(&first).await {
                let _operation = self.flow_operation.lock().await;
                if self
                    .flow_generation
                    .load(std::sync::atomic::Ordering::SeqCst)
                    == generation
                {
                    self.stop_flow_inner().await;
                    self.emit_queue_updated().await;
                }
                return Err(error);
            }
        }
        Ok(())
    }

    /// Continue never creates a generation, changes steering, or rebuilds a queue.
    pub async fn continue_flow(&self, user_id: &str) -> AppResult<()> {
        let _operation = self.flow_operation.lock().await;
        if !self
            .active_flow
            .read()
            .await
            .as_ref()
            .is_some_and(|f| f.user_id == user_id)
        {
            return Err(AppError::Playback(
                "No active Flow to continue. Start a new Flow first.".into(),
            ));
        }
        self.resume().await
    }

    pub(super) fn flow_item(t: &FlowTrackItem) -> QueueItem {
        QueueItem {
            queue_id: Uuid::new_v4().to_string(),
            track_id: t.track_id.clone(),
            title: t.title.clone(),
            artist: t.artist.clone(),
            duration_secs: t.duration_secs,
        }
    }

    pub(super) fn flow_settings(ratio: f64, is_active: bool) -> FlowSessionState {
        FlowSessionState {
            is_active,
            familiarity_ratio: ratio.clamp(0.0, 1.0),
            updated_at: chrono::Utc::now().timestamp(),
            generation: None,
            last_track_id: None,
            last_track_title: None,
            last_track_artist: None,
            flow_label: if ratio >= 0.75 {
                "Familiar mix · Styles you love"
            } else if ratio <= 0.25 {
                "Unfamiliar mix · New songs & artists"
            } else {
                "Balanced mix · Familiar & new"
            }
            .into(),
        }
    }

    pub async fn flow_session(
        &self,
        fe: &crate::recommendations::FlowEngine,
        user_id: &str,
    ) -> AppResult<Option<FlowSessionState>> {
        let mut settings = fe.get_flow_session(user_id).await?;
        if let Some(s) = settings.as_mut() {
            s.is_active = self
                .active_flow
                .read()
                .await
                .as_ref()
                .map(|f| f.user_id.as_str())
                == Some(user_id);
            s.generation = self
                .active_flow
                .read()
                .await
                .as_ref()
                .filter(|f| f.user_id == user_id)
                .map(|f| f.generation);
            // Track metadata comes from the playback queue, never duplicated persistence.
            s.last_track_id = None;
            s.last_track_title = None;
            s.last_track_artist = None;
            if s.is_active {
                if let Some(item) = self.queue.read().await.current() {
                    s.last_track_id = Some(item.track_id.clone());
                    s.last_track_title = Some(item.title.clone());
                    s.last_track_artist = Some(item.artist.clone());
                }
            }
        }
        Ok(settings)
    }

    pub async fn flow_reason(&self, user_id: &str, track_id: &str) -> Option<FlowExplanation> {
        let flow = self.active_flow.read().await;
        let flow = flow.as_ref().filter(|f| f.user_id == user_id)?;
        let queue = self.queue.read().await;
        let current = queue.current().filter(|i| i.track_id == track_id)?;
        if flow.generated_queue_ids.contains(&current.queue_id) {
            flow.reasons.get(track_id).cloned()
        } else if flow.manual_queue_ids.contains(&current.queue_id) {
            Some(FlowExplanation {
                primary: "existing_queue".into(),
                from_existing_queue: true,
                ..Default::default()
            })
        } else {
            Some(FlowExplanation {
                primary: "continuing_track".into(),
                ..Default::default()
            })
        }
    }

    pub(super) async fn stop_flow_inner(&self) {
        self.flow_generation
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if let Some(flow) = self.active_flow.write().await.take() {
            let mut q = self.queue.write().await;
            let start = q.current_index().map_or(0, |i| i + 1);
            for i in (start..q.len()).rev() {
                if flow.generated_queue_ids.contains(&q.items()[i].queue_id) {
                    q.remove(i);
                }
            }
            *self.current_source.write().await = "library".into();
        }
    }

    pub async fn stop_flow(&self) {
        self.flow_generation
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let _operation = self.flow_operation.lock().await;
        self.stop_flow_inner().await;
        self.emit_queue_updated().await;
    }

    pub async fn set_flow_steering(
        &self,
        fe: &crate::recommendations::FlowEngine,
        user_id: &str,
        ratio: f64,
    ) -> AppResult<()> {
        if !ratio.is_finite() {
            return Err(AppError::Validation("Flow steering must be finite".into()));
        }
        let _operation = self.flow_operation.lock().await;
        let state = Self::flow_settings(ratio, false);
        fe.save_flow_session(user_id, &state).await?;
        let active = self
            .active_flow
            .read()
            .await
            .as_ref()
            .filter(|f| f.user_id == user_id)
            .map(|f| (f.generation, f.generated_queue_ids.clone(), f.session.clone()));
        let Some((generation, generated_ids, session)) = active else {
            return Ok(());
        };
        let (excluded, recent_artists) = self.flow_regen_context(&generated_ids).await;
        let seed = self.queue.read().await.current().map(|i| i.track_id.clone());
        let tracks = fe
            .generate_playback_tracks_seeded(
                user_id,
                state.familiarity_ratio,
                20,
                &excluded,
                &recent_artists,
                Some(&session),
                seed.as_deref(),
            )
            .await?;
        if self
            .replace_generated_upcoming(
                generation,
                &generated_ids,
                tracks,
                Some(state.familiarity_ratio),
            )
            .await
        {
            self.emit_queue_updated().await;
        }
        Ok(())
    }
}
