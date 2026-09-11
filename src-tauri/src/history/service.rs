use crate::config::HistoryConfig;
use crate::core::error::AppResult;
use crate::core::event::Event;
use crate::core::event_bus::EventBus;
use crate::database::models::PlaybackHistoryRecord;
use crate::database::repositories::{HistoryDetail, HistoryRepository, StatsRepository};
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone)]
struct ActiveSession {
    track_id: String,
    started_at: i64,
    duration_secs: f64,
    max_position_secs: f64,
    source: String,
}

pub struct HistoryService {
    history_repo: Arc<dyn HistoryRepository>,
    stats_repo: Arc<dyn StatsRepository>,
    config: Arc<RwLock<HistoryConfig>>,
    current_session: Arc<Mutex<Option<ActiveSession>>>,
}

impl HistoryService {
    pub fn new(
        history_repo: Arc<dyn HistoryRepository>,
        stats_repo: Arc<dyn StatsRepository>,
        config: HistoryConfig,
        event_bus: Arc<EventBus>,
    ) -> Arc<Self> {
        let service = Arc::new(Self {
            history_repo,
            stats_repo,
            config: Arc::new(RwLock::new(config)),
            current_session: Arc::new(Mutex::new(None)),
        });

        let s_clone = service.clone();
        tokio::spawn(async move {
            s_clone.listen_to_events(event_bus).await;
        });

        service
    }

    async fn listen_to_events(&self, event_bus: Arc<EventBus>) {
        let mut rx = event_bus.subscribe();

        while let Ok(event) = rx.recv().await {
            match event {
                Event::PlaybackStarted {
                    track_id,
                    duration_secs,
                    source,
                    ..
                } => {
                    self.on_playback_started(track_id, duration_secs, source).await;
                }
                Event::PlaybackPositionChanged { position_secs, .. } => {
                    let mut session_guard = self.current_session.lock().await;
                    if let Some(ref mut session) = *session_guard {
                        if position_secs > session.max_position_secs {
                            session.max_position_secs = position_secs;
                        }
                    }
                }
                Event::TrackFinished {
                    track_id,
                    seconds_listened,
                    completed,
                } => {
                    self.finalize_session(&track_id, seconds_listened, completed, false).await;
                }
                Event::PlaybackStopped => {
                    let session_opt = {
                        let mut guard = self.current_session.lock().await;
                        guard.take()
                    };
                    if let Some(session) = session_opt {
                        let seconds = session.max_position_secs;
                        self.finalize_active_session(session, seconds, false, true).await;
                    }
                }
                _ => {}
            }
        }
    }

    async fn on_playback_started(&self, track_id: String, duration_secs: f64, source: String) {
        let prev_session = {
            let mut guard = self.current_session.lock().await;
            guard.replace(ActiveSession {
                track_id,
                started_at: Utc::now().timestamp(),
                duration_secs,
                max_position_secs: 0.0,
                source,
            })
        };

        if let Some(session) = prev_session {
            let seconds = session.max_position_secs;
            self.finalize_active_session(session, seconds, false, true).await;
        }
    }

    async fn finalize_session(
        &self,
        track_id: &str,
        seconds_listened: f64,
        completed: bool,
        skipped_hint: bool,
    ) {
        let session_opt = {
            let mut guard = self.current_session.lock().await;
            if guard.as_ref().map(|s| s.track_id == track_id).unwrap_or(false) {
                guard.take()
            } else {
                None
            }
        };

        if let Some(session) = session_opt {
            self.finalize_active_session(session, seconds_listened, completed, skipped_hint).await;
        }
    }

    async fn finalize_active_session(
        &self,
        session: ActiveSession,
        seconds_listened: f64,
        completed: bool,
        skipped_hint: bool,
    ) {
        let now = Utc::now().timestamp();
        let dur = session.duration_secs;
        let percentage = if dur > 0.0 {
            (seconds_listened / dur).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let cfg = self.config.read().await;
        let is_meaningful = seconds_listened >= cfg.min_meaningful_seconds
            || percentage >= cfg.min_meaningful_percentage
            || completed;

        let is_skipped = !is_meaningful && skipped_hint;

        let entry = PlaybackHistoryRecord {
            id: Uuid::new_v4().to_string(),
            track_id: session.track_id.clone(),
            started_at: session.started_at,
            ended_at: now,
            seconds_listened,
            percentage_listened: percentage,
            completed: if completed { 1 } else { 0 },
            skipped: if is_skipped { 1 } else { 0 },
            source: session.source,
            playlist_id: None,
            recommendation_session_id: None,
        };

        info!(
            track_id = %session.track_id,
            seconds = seconds_listened,
            is_meaningful = is_meaningful,
            completed = completed,
            "Recording playback session"
        );

        let _ = self.history_repo.record_playback(&entry).await;
        let _ = self
            .stats_repo
            .update_track_playback_stats(
                &session.track_id,
                seconds_listened,
                is_meaningful,
                completed,
                is_skipped,
            )
            .await;
    }

    pub async fn get_recent_history(&self, limit: u32) -> AppResult<Vec<HistoryDetail>> {
        self.history_repo.get_recent_history(limit).await
    }

    pub async fn set_track_like(&self, track_id: &str, like_status: i64) -> AppResult<()> {
        self.stats_repo.set_track_like(track_id, like_status).await
    }

    pub fn stats_repo(&self) -> Arc<dyn StatsRepository> {
        self.stats_repo.clone()
    }
}
