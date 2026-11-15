use crate::recommendations::mixes::OnlineTrackInput;
use crate::core::sync::{MutexExt, RwLockExt};
use crate::core::command::RepeatMode;
use crate::core::error::{AppError, AppResult};
use crate::core::event::{Event, QueueItem};
use crate::core::event_bus::EventBus;
use crate::database::repositories::TrackRepository;
use crate::database::repositories::{SettingsRepository, SqliteSettingsRepository};
use crate::playback::backend::AudioBackend;
use crate::playback::queue::PlaybackQueue;
use crate::playback::session::{RestoredTrack, SavedPlayback, SavedPosition, POSITION_KEY, SETTING_KEY};
use crate::playback::stream::StreamPlaybackManager;
use crate::recommendations::{FlowExplanation, FlowSessionState, FlowTrackItem};
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, OnceCell, RwLock};
use tracing::{debug, info};
use uuid::Uuid;

mod flow;
mod flow_adaptive;
mod monitor;
mod play;
mod transport;
mod queue_ops;
mod state;
mod bookmark;
mod tracks;

fn default_can_seek() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackStateDto {
    pub current_track_id: Option<String>,
    pub is_playing: bool,
    pub is_paused: bool,
    #[serde(default = "default_can_seek")]
    pub can_seek: bool,
    #[serde(default)]
    pub is_buffering: bool,
    pub position_secs: f64,
    pub duration_secs: f64,
    pub volume: f32,
    pub is_muted: bool,
    pub repeat_mode: RepeatMode,
    pub is_shuffle: bool,
    pub queue_length: usize,
    pub current_queue_index: Option<usize>,
    pub queue_track_ids: Vec<String>,
    /// Songs the user queued that are still waiting, in the order they were added.
    #[serde(default)]
    pub user_queue_track_ids: Vec<String>,
    /// Which source owns the queue: "library", "flow", "archive" (Downloads list), ...
    #[serde(default)]
    pub source: String,
}

#[derive(Default)]
struct ActiveFlow {
    generation: u64,
    manual_queue_ids: HashSet<String>,
    user_id: String,
    ratio: f64,
    generated_queue_ids: HashSet<String>,
    reasons: HashMap<String, FlowExplanation>,
    /// In-memory session evidence (live skips / full listens). Reset with each
    /// `start_flow`, dropped with the flow.
    session: crate::recommendations::session::SessionSteer,
}

pub struct PlaybackService {
    backend: Arc<Mutex<Box<dyn AudioBackend>>>,
    queue: Arc<RwLock<PlaybackQueue>>,
    event_bus: Arc<EventBus>,
    track_repo: Arc<dyn TrackRepository>,
    current_duration_secs: Arc<RwLock<f64>>,
    current_source: Arc<RwLock<String>>,
    /// Track ids of the active Downloads list in library order (to undo shuffle).
    archive_order: Arc<RwLock<Vec<String>>>,
    volume: Arc<RwLock<f32>>,
    stream_manager: Arc<std::sync::RwLock<Option<Arc<StreamPlaybackManager>>>>,
    active_stream_state:
        Arc<std::sync::RwLock<Option<Arc<crate::playback::progressive::ProgressiveStreamState>>>>,
    pool: Option<SqlitePool>,
    restore_ready: OnceCell<()>,
    restored_track: RwLock<Option<RestoredTrack>>,
    last_saved_bookmark: Mutex<Option<String>>,
    last_saved_position: Mutex<Option<String>>,
    preresolution_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    monitor_task: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
    session_task: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
    shutdown_done: Mutex<bool>,
    shutting_down: std::sync::atomic::AtomicBool,
    /// Authoritative session token and queue-entry identity for real playback.
    /// Generated fresh at each `play_item` call; cleared on stop/track-finish.
    active_session_id: Arc<RwLock<Option<(String, String)>>>,
    /// Generation token to cancel obsolete track transitions during rapid skips.
    play_generation: Arc<std::sync::atomic::AtomicU64>,
    /// Indicates whether an audio source is actively being resolved over network.
    is_resolving: Arc<std::sync::atomic::AtomicBool>,
    recommendation_user:
        std::sync::RwLock<Option<Arc<RwLock<Option<crate::database::repositories::UserProfile>>>>>,
    starter_preparation: std::sync::RwLock<Option<Arc<crate::discovery::DiscoveryCoordinator>>>,
    flow_operation: Mutex<()>,
    flow_generation: std::sync::atomic::AtomicU64,
    active_flow: RwLock<Option<ActiveFlow>>,
    flow_engine: Arc<std::sync::RwLock<Option<Arc<crate::recommendations::FlowEngine>>>>,
}

impl PlaybackService {
    pub fn new(
        backend: Box<dyn AudioBackend>,
        track_repo: Arc<dyn TrackRepository>,
        event_bus: Arc<EventBus>,
        pool: Option<SqlitePool>,
    ) -> Arc<Self> {
        let service = Arc::new(Self {
            backend: Arc::new(Mutex::new(backend)),
            queue: Arc::new(RwLock::new(PlaybackQueue::new())),
            event_bus,
            track_repo,
            current_duration_secs: Arc::new(RwLock::new(0.0)),
            current_source: Arc::new(RwLock::new("library".to_string())),
            archive_order: Arc::new(RwLock::new(Vec::new())),
            volume: Arc::new(RwLock::new(0.8)),
            stream_manager: Arc::new(std::sync::RwLock::new(None)),
            active_stream_state: Arc::new(std::sync::RwLock::new(None)),
            pool,
            restore_ready: OnceCell::new(),
            restored_track: RwLock::new(None),
            last_saved_bookmark: Mutex::new(None),
            last_saved_position: Mutex::new(None),
            preresolution_handle: Arc::new(Mutex::new(None)),
            monitor_task: std::sync::Mutex::new(None),
            session_task: std::sync::Mutex::new(None),
            shutdown_done: Mutex::new(false),
            shutting_down: std::sync::atomic::AtomicBool::new(false),
            active_session_id: Arc::new(RwLock::new(None)),
            play_generation: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            is_resolving: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            recommendation_user: std::sync::RwLock::new(None),
            starter_preparation: std::sync::RwLock::new(None),
            flow_operation: Mutex::new(()),
            flow_generation: std::sync::atomic::AtomicU64::new(0),
            active_flow: RwLock::new(None),
            flow_engine: Arc::new(std::sync::RwLock::new(None)),
        });

        // Spawn periodic position monitor and track finish detector
        let s_clone = service.clone();
        let task = tokio::spawn(async move {
            s_clone.run_playback_monitor_loop().await;
        });
        *service
            .monitor_task
            .lock_or_recover() = Some(task);

        // Flow session steering: listen for finalized listen outcomes. Subscribe
        // synchronously so no outcome published after construction is missed;
        // hold only a weak ref so the task never keeps the service alive.
        let mut outcomes = service.event_bus.subscribe();
        let weak = Arc::downgrade(&service);
        let session_task = tokio::spawn(async move {
            use tokio::sync::broadcast::error::RecvError;
            loop {
                let event = match outcomes.recv().await {
                    Ok(event) => event,
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                };
                let Event::ListenOutcome {
                    user_id,
                    track_id,
                    early_skip,
                    qualified,
                    near_complete,
                    ..
                } = event
                else {
                    continue;
                };
                let Some(service) = weak.upgrade() else {
                    break;
                };
                service
                    .on_listen_outcome(
                        &user_id,
                        &track_id,
                        crate::recommendations::session::SessionOutcome {
                            early_skip,
                            qualified,
                            near_complete,
                        },
                    )
                    .await;
            }
        });
        *service
            .session_task
            .lock_or_recover() = Some(session_task);

        service
    }

    pub fn set_stream_manager(&self, sm: Arc<StreamPlaybackManager>) {
        sm.set_event_bus(self.event_bus.clone());
        let mut w = self.stream_manager.write_or_recover();
        *w = Some(sm);
    }

    /// Restore metadata/controls only. Audio and history begin on an explicit Play.
    fn online_catalog(&self) -> Option<Arc<crate::providers::online_catalog::OnlineCatalog>> {
        self.stream_manager
            .read_or_recover()
            .as_ref()
            .and_then(|manager| manager.catalog.clone())
    }

    pub fn set_recommendation_user(
        &self,
        user: Arc<RwLock<Option<crate::database::repositories::UserProfile>>>,
    ) {
        *self.recommendation_user.write_or_recover() = Some(user);
    }
    async fn recommendation_user_id(&self) -> String {
        let context = self.recommendation_user.read_or_recover().clone();
        if let Some(user) = context {
            return user
                .read()
                .await
                .as_ref()
                .map(|user| user.id.clone())
                .unwrap_or_else(|| "default".into());
        }
        self.active_flow
            .read()
            .await
            .as_ref()
            .map(|flow| flow.user_id.clone())
            .unwrap_or_else(|| "default".into())
    }
    /// A removed download becomes its online recording: queued copies follow it.
    pub async fn replace_queued_track(&self, old_id: &str, new_id: &str) {
        let changed = self.queue.write().await.replace_track(old_id, new_id);
        if changed {
            self.emit_queue_updated().await;
        }
    }
    pub async fn remove_recommended_songs(&self, user: &str, ids: &[String]) -> AppResult<()> {
        if self.recommendation_user_id().await != user {
            return Ok(());
        }
        let _operation = self.flow_operation.lock().await;
        let mut queue = self.queue.write().await;
        let current = queue.current().map(|item| item.queue_id.clone());
        let items = queue
            .items()
            .iter()
            .filter(|item| {
                Some(&item.queue_id) == current.as_ref() || !ids.contains(&item.track_id)
            })
            .cloned()
            .collect();
        queue.reconcile_sources(items);
        drop(queue);
        self.emit_queue_updated().await;
        Ok(())
    }
    async fn skip_blocked_queue_items(
        &self,
        mut item: Option<QueueItem>,
    ) -> AppResult<Option<QueueItem>> {
        let Some(pool) = &self.pool else {
            return Ok(item);
        };
        let user = self.recommendation_user_id().await;
        let signals = crate::recommendations::signals::SongSignals::load(pool, &user).await?;
        let limit = self.queue.read().await.len();
        for _ in 0..limit {
            if !item
                .as_ref()
                .is_some_and(|item| signals.blocked(&item.track_id, &item.title, &item.artist))
            {
                return Ok(item);
            }
            // Bypass repeat-one for a song the user has explicitly excluded.
            item = self.queue.write().await.skip_forward().cloned();
        }
        Ok(None)
    }

    async fn prepare_track_source(&self, id: &str) -> AppResult<()> {
        if let Some(catalog) = self.online_catalog() {
            if catalog.bind(id).await?.is_none() {
                if let Some(pool) = &self.pool {
                    let unavailable: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tracks WHERE id=? AND file_path LIKE 'online://%')").bind(id).fetch_one(pool).await?;
                    if unavailable {
                        return Err(AppError::Playback(
                            "This song is not available from JioSaavn".into(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    pub fn set_starter_preparation(
        &self,
        coordinator: Arc<crate::discovery::DiscoveryCoordinator>,
    ) {
        *self.starter_preparation.write_or_recover() = Some(coordinator);
    }

    pub fn set_flow_engine(&self, fe: Arc<crate::recommendations::FlowEngine>) {
        let mut w = self.flow_engine.write_or_recover();
        *w = Some(fe);
    }

}

#[cfg(test)]
mod streaming_seek_tests {
    use super::*;
    use crate::database::{create_in_memory_pool, repositories::SqliteTrackRepository};
    use crate::playback::{backend::MockAudioBackend, progressive::ProgressiveStreamState};

    #[tokio::test]
    async fn active_stream_exposes_seek_controls_and_accepts_seek_before_completion() {
        let pool = create_in_memory_pool().await.unwrap();
        let bus = Arc::new(EventBus::new(16));
        let service = PlaybackService::new(
            Box::new(MockAudioBackend::new()),
            Arc::new(SqliteTrackRepository::new(pool.clone())),
            bus.clone(), Some(pool),
        );
        let state = Arc::new(ProgressiveStreamState::new("stream".into(), None));
        state.activate_playback("session".into());
        *service.active_stream_state.write_or_recover() = Some(state.clone());
        let mut events = bus.subscribe();
        assert!(service.get_playback_state().await.can_seek);
        service.seek(42.0).await.unwrap();
        assert_eq!(events.try_recv().unwrap(), Event::PlaybackSeeked { position_secs: 42.0 });
        assert!(!state.is_completed());
        service.request_shutdown();
        assert!(state.is_cancelled(), "shutdown must wake a decoder before command draining");
        assert!(!service.get_playback_state().await.can_seek);
        service.shutdown().await.unwrap();
    }
}
