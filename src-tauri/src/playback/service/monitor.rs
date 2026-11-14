//! The playback monitor loop and interrupting in-flight stream preparation.
use super::*;

impl PlaybackService {
    pub(super) fn interrupt_preparation(&self) {
        self.play_generation.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if let Some(manager) = self.stream_manager.read_or_recover().as_ref() {
            manager.request_cancel();
        }
        if let Some(state) = self.active_stream_state.read_or_recover().as_ref() {
            state.deactivate_playback();
            state.set_cancelled();
        }
    }

    pub async fn cancel_active_stream(&self) {
        if let Some(state) = self.active_stream_state.write_or_recover().take() {
            state.deactivate_playback();
            state.set_cancelled();
        }
        let sm_opt = self.stream_manager.read_or_recover().clone();
        if let Some(sm) = sm_opt {
            sm.cancel_active_stream().await;
        }
    }

    /// Background task monitoring playback position and track completion.
    /// It ticks at 4 Hz only while something plays; otherwise it parks until
    /// the next domain event, so an idle or paused player costs no CPU.
    pub(super) async fn run_playback_monitor_loop(&self) {
        let mut interval = tokio::time::interval(Duration::from_millis(250));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut last_checkpoint = tokio::time::Instant::now();
        let mut wake = self.event_bus.subscribe();
        let mut idle_ticks = 0u32;

        loop {
            if idle_ticks >= PARK_AFTER_IDLE_TICKS {
                idle_ticks = 0;
                if let Err(error) = self.save_playback_bookmark().await {
                    tracing::warn!(%error, "Could not checkpoint playback bookmark");
                }
                if !park(&mut wake).await {
                    // Heartbeat only: check once, then park again.
                    idle_ticks = PARK_AFTER_IDLE_TICKS - 1;
                }
                interval.reset();
            }
            interval.tick().await;
            let operation = self.flow_operation.lock().await;
            if self
                .shutting_down
                .load(std::sync::atomic::Ordering::Acquire)
            {
                break;
            }
            if last_checkpoint.elapsed() >= Duration::from_secs(10) {
                if let Err(error) = self.save_playback_bookmark().await {
                    tracing::warn!(%error, "Could not checkpoint playback bookmark");
                }
                last_checkpoint = tokio::time::Instant::now();
            }
            let session_queue_id = self
                .active_session_id
                .read()
                .await
                .as_ref()
                .map(|(_, q)| q.clone());
            if let Some(queue_id) = &session_queue_id {
                if self.queue.read().await.current().map(|i| &i.queue_id) != Some(queue_id) {
                    // A queue selection is pending; do not finalize/advance a different entry.
                    continue;
                }
            }

            let (pos_secs, is_paused, is_finished) = {
                let backend = self.backend.lock().await;
                (
                    backend.position().as_secs_f64(),
                    backend.is_paused(),
                    backend.is_finished(),
                )
            };

            let duration = *self.current_duration_secs.read().await;
            idle_ticks = if session_queue_id.is_some() && !is_paused {
                0
            } else {
                idle_ticks + 1
            };

            if !is_paused && !is_finished && session_queue_id.is_some() {

                // Always read from authoritative source — no stale local cache.
                let session_id = self
                    .active_session_id
                    .read()
                    .await
                    .as_ref()
                    .map(|(id, _)| id.clone())
                    .unwrap_or_default();

                let _ = self.event_bus.publish(Event::PlaybackPositionChanged {
                    session_id,
                    position_secs: pos_secs,
                    duration_secs: duration,
                });
            } else if is_finished && session_queue_id.is_some() {
                // Track finished!

                // Read track_id and session_id from authoritative sources.
                let track_id_opt = {
                    let q_guard = self.queue.read().await;
                    q_guard.current().map(|i| i.track_id.clone())
                };
                // Take (clear) the session_id: this session is now over.
                let session_id = {
                    let mut guard = self.active_session_id.write().await;
                    guard.take().map(|(id, _)| id).unwrap_or_default()
                };

                if let Some(track_id) = track_id_opt {
                    info!(%track_id, %session_id, "Track finished playing to end of stream");
                    let _ = self.event_bus.publish(Event::TrackFinished {
                        session_id,
                        track_id,
                        // History measures audible wall time, excluding pauses and
                        // buffering. Duration/position cannot prove time listened.
                        seconds_listened: 0.0,
                        completed: true,
                    });
                }

                // A manual selection after this snapshot must supersede automatic advancement.
                let generation = self
                    .play_generation
                    .load(std::sync::atomic::Ordering::SeqCst);
                drop(operation);
                let _ = self
                    .advance_after_finished(generation, session_queue_id.as_deref())
                    .await;
            }
        }
    }
}

/// Idle ticks before the monitor parks. Staying awake briefly after a wake-up
/// covers state that changes just after its event is published.
const PARK_AFTER_IDLE_TICKS: u32 = 8;

/// Waits for the next domain event (play, resume, seek, queue change...) and
/// returns whether one arrived. The backlog is dropped first so old events
/// don't wake it. A slow heartbeat bounds the wait in case a transition
/// publishes nothing.
async fn park(wake: &mut tokio::sync::broadcast::Receiver<Event>) -> bool {
    *wake = wake.resubscribe();
    tokio::time::timeout(Duration::from_secs(5), wake.recv()).await.is_ok()
}
