//! Flow adapting to listening: re-ranking and refilling the upcoming songs.
use super::*;

impl PlaybackService {
    /// Feed one finalized listen into the active Flow session; on a steering
    /// trigger, re-rank the generated upcoming slots.
    pub(super) async fn on_listen_outcome(
        &self,
        user_id: &str,
        track_id: &str,
        outcome: crate::recommendations::session::SessionOutcome,
    ) {
        if self
            .shutting_down
            .load(std::sync::atomic::Ordering::SeqCst)
            || *self.current_source.read().await != "flow"
        {
            return;
        }
        let generation = match self.active_flow.read().await.as_ref() {
            Some(flow) if flow.user_id == user_id => flow.generation,
            _ => return,
        };
        let Some(fe) = self.flow_engine.read_or_recover().clone() else {
            return;
        };
        let keys = match fe.style_keys(track_id).await {
            Ok(keys) => keys,
            Err(e) => {
                debug!("Flow session: style lookup failed for {}: {}", track_id, e);
                return;
            }
        };
        let trigger = {
            let mut active = self.active_flow.write().await;
            let Some(flow) = active
                .as_mut()
                .filter(|flow| flow.generation == generation)
            else {
                return;
            };
            flow.session.record(&keys, outcome)
        };
        if trigger.fired() {
            debug!("Flow session steer {:?}; re-ranking upcoming", trigger);
            if let Err(e) = self.rerank_upcoming(&fe, generation).await {
                debug!("Flow session re-rank failed: {}", e);
            }
        }
    }

    /// Regenerate generated upcoming Flow slots with the current session
    /// steer. Manual entries and history stay in place.
    pub(super) async fn rerank_upcoming(
        &self,
        fe: &crate::recommendations::FlowEngine,
        generation: u64,
    ) -> AppResult<()> {
        let _operation = self.flow_operation.lock().await;
        let active = self
            .active_flow
            .read()
            .await
            .as_ref()
            .filter(|flow| flow.generation == generation)
            .map(|flow| {
                (
                    flow.user_id.clone(),
                    flow.ratio,
                    flow.generated_queue_ids.clone(),
                    flow.session.clone(),
                )
            });
        let Some((user_id, ratio, generated_ids, session)) = active else {
            return Ok(());
        };
        let (excluded, recent_artists) = self.flow_regen_context(&generated_ids).await;
        let seed = self.queue.read().await.current().map(|i| i.track_id.clone());
        let tracks = fe
            .generate_playback_tracks_seeded(
                &user_id,
                ratio,
                15,
                &excluded,
                &recent_artists,
                Some(&session),
                seed.as_deref(),
            )
            .await?;
        if tracks.is_empty() {
            return Ok(());
        }
        if self.replace_generated_upcoming(generation, &generated_ids, tracks, None)
            .await
        {
            self.emit_queue_updated().await;
        }
        Ok(())
    }

    /// Exclusions (history + manual upcoming) and recent-artist memory used
    /// when regenerating generated upcoming Flow slots.
    pub(super) async fn flow_regen_context(&self, generated_ids: &HashSet<String>) -> (Vec<String>, Vec<String>) {
        let q = self.queue.read().await;
        let current = q.current_index();
        (
            q.items()
                .iter()
                .enumerate()
                .filter(|(index, item)| {
                    current.is_some_and(|current| *index <= current)
                        || !generated_ids.contains(&item.queue_id)
                })
                .map(|(_, item)| item.track_id.clone())
                .collect::<Vec<_>>(),
            Self::recent_artist_window(
                &q.items()[..current.map_or(0, |index| index + 1).min(q.items().len())],
            ),
        )
    }

    /// Replace generated upcoming slots with `tracks`, preserving manual
    /// entries at their positions. Returns false when the flow changed meanwhile.
    pub(super) async fn replace_generated_upcoming(
        &self,
        generation: u64,
        generated_ids: &HashSet<String>,
        tracks: Vec<FlowTrackItem>,
        ratio: Option<f64>,
    ) -> bool {
        let mut active = self.active_flow.write().await;
        let Some(flow) = active.as_mut().filter(|flow| {
            flow.generation == generation
                && self
                    .flow_generation
                    .load(std::sync::atomic::Ordering::SeqCst)
                    == generation
        }) else {
            return false;
        };
        if let Some(ratio) = ratio {
            flow.ratio = ratio;
        }
        let mut q = self.queue.write().await;
        let current_index = q.current_index();
        let start = current_index.map_or(0, |index| index + 1);
        let mut rebuilt = q.items()[..start].to_vec();
        // Keep original evidence for the current/history entries. Replace only
        // generated upcoming slots, preserving manual entries at their positions.
        flow.generated_queue_ids
            .retain(|id| rebuilt.iter().any(|item| &item.queue_id == id));
        flow.reasons
            .retain(|id, _| rebuilt.iter().any(|item| &item.track_id == id));
        let mut replacements = Vec::with_capacity(tracks.len());
        for track in tracks {
            let item = Self::flow_item(&track);
            flow.generated_queue_ids.insert(item.queue_id.clone());
            flow.reasons.insert(track.track_id, track.explanation);
            replacements.push(item);
        }
        let mut replacements = replacements.into_iter();
        for old in q.items().iter().skip(start) {
            if generated_ids.contains(&old.queue_id) {
                if let Some(item) = replacements.next() {
                    rebuilt.push(item);
                }
            } else {
                rebuilt.push(old.clone());
            }
        }
        rebuilt.extend(replacements);
        q.set_queue(rebuilt, current_index);
        true
    }

    /// Last artists of `items` (oldest → newest) used as Flow's cross-batch
    /// diversity memory, so artist caps do not reset on every refill.
    pub(super) fn recent_artist_window(items: &[QueueItem]) -> Vec<String> {
        const WINDOW: usize = 15;
        items[items.len().saturating_sub(WINDOW)..]
            .iter()
            .map(|item| item.artist.clone())
            .collect()
    }

    pub(super) async fn refill_flow_queue(
        &self,
        fe: &Arc<crate::recommendations::FlowEngine>,
    ) -> AppResult<Option<QueueItem>> {
        let (user_id, ratio, generation, session) = match self.active_flow.read().await.as_ref() {
            Some(f) => (f.user_id.clone(), f.ratio, f.generation, f.session.clone()),
            None => return Ok(None),
        };
        let (recent, current, artist) = {
            let q = self.queue.read().await;
            (
                q.items()
                    .iter()
                    .rev()
                    .take(40)
                    .map(|i| i.track_id.clone())
                    .collect::<Vec<_>>(),
                q.current().map(|i| i.track_id.clone()),
                Self::recent_artist_window(q.items()),
            )
        };
        // New tracks append after the last queued item, so it seeds transitions.
        let seed = recent.first().cloned();
        let mut tracks = fe
            .generate_playback_tracks_seeded(
                &user_id,
                ratio,
                15,
                &recent,
                &artist,
                Some(&session),
                seed.as_deref(),
            )
            .await?;
        if tracks.is_empty() {
            tracks = fe
                .generate_playback_tracks_seeded(
                    &user_id,
                    ratio,
                    15,
                    &current.into_iter().collect::<Vec<_>>(),
                    &artist,
                    Some(&session),
                    seed.as_deref(),
                )
                .await?;
        }
        let mut flow = self.active_flow.write().await;
        let Some(f) = flow.as_mut().filter(|f| {
            f.generation == generation
                && self
                    .flow_generation
                    .load(std::sync::atomic::Ordering::SeqCst)
                    == generation
        }) else {
            return Ok(None);
        };
        let next = {
            let mut q = self.queue.write().await;
            f.generated_queue_ids
                .retain(|id| q.items().iter().rev().take(40).any(|i| &i.queue_id == id));
            f.reasons
                .retain(|id, _| q.items().iter().rev().take(40).any(|i| &i.track_id == id));
            for t in tracks {
                let item = Self::flow_item(&t);
                let id = item.queue_id.clone();
                if q.enqueue(item, false) {
                    f.generated_queue_ids.insert(id);
                    f.reasons.insert(t.track_id, t.explanation);
                }
            }
            q.skip_forward().cloned()
        };
        drop(flow);
        self.emit_queue_updated().await;
        Ok(next)
    }
}
