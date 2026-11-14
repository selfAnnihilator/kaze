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
use crate::playback::session::{RestoredTrack, SavedPlayback, SETTING_KEY};
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

    /// Feed one finalized listen into the active Flow session; on a steering
    /// trigger, re-rank the generated upcoming slots.
    async fn on_listen_outcome(
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
    async fn rerank_upcoming(
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
    async fn flow_regen_context(&self, generated_ids: &HashSet<String>) -> (Vec<String>, Vec<String>) {
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
    async fn replace_generated_upcoming(
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

    pub async fn initialize_saved_playback(&self) {
        self.restore_ready
            .get_or_init(|| async {
                if let Err(error) = self.restore_saved_playback().await {
                    tracing::warn!(%error, "Could not restore playback bookmark; starting empty");
                }
            })
            .await;
    }

    async fn restore_saved_playback(&self) -> AppResult<()> {
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
    async fn save_playback_bookmark(&self) -> AppResult<()> {
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

    fn flow_item(t: &FlowTrackItem) -> QueueItem {
        QueueItem {
            queue_id: Uuid::new_v4().to_string(),
            track_id: t.track_id.clone(),
            title: t.title.clone(),
            artist: t.artist.clone(),
            duration_secs: t.duration_secs,
        }
    }

    fn flow_settings(ratio: f64, is_active: bool) -> FlowSessionState {
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

    async fn stop_flow_inner(&self) {
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

    fn interrupt_preparation(&self) {
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
    async fn run_playback_monitor_loop(&self) {
        let mut interval = tokio::time::interval(Duration::from_millis(250));
        let mut last_checkpoint = tokio::time::Instant::now();

        loop {
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

    /// Plays a track directly, clearing existing queue or adding to top.
    pub async fn play_track(&self, track_id: &str, source: Option<String>) -> AppResult<()> {
        self.interrupt_preparation();
        self.prepare_track_source(track_id).await?;
        let track = match self.track_repo.find_by_id(track_id).await? {
            Some(t) => t,
            None => {
                // If not in track_repo, check external_tracks for online / discovery track
                if let Some(pool) = &self.pool {
                    type ExtRow = (String, String, Option<String>, Option<f64>, Option<String>, Option<String>);
                    let ext: Option<ExtRow> = sqlx::query_as(
                        "SELECT title, artist, album, duration_secs, cover_art_url, preview_url FROM external_tracks WHERE id = ?"
                    )
                    .bind(track_id)
                    .fetch_optional(pool)
                    .await
                    .unwrap_or(None);

                    if let Some((title, artist, album, duration_secs, cover_art_url, preview_url)) =
                        ext
                    {
                        let _ = crate::recommendations::mixes::ensure_online_track(
                            pool,
                            &OnlineTrackInput {
                                id: track_id,
                                title: &title,
                                artist: Some(&artist),
                                album: album.as_deref(),
                                duration_secs,
                                cover_art_url: cover_art_url.as_deref(),
                                preview_url: preview_url.as_deref(),
                            },
                        )
                        .await;
                        self.track_repo.find_by_id(track_id).await?.ok_or_else(|| {
                            AppError::NotFound(format!(
                                "Track not found after registration: {}",
                                track_id
                            ))
                        })?
                    } else {
                        return Err(AppError::NotFound(format!("Track not found: {}", track_id)));
                    }
                } else {
                    return Err(AppError::NotFound(format!("Track not found: {}", track_id)));
                }
            }
        };

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

    async fn play_item(&self, item: &QueueItem) -> AppResult<()> {
        self.play_item_at(item, None).await
    }

    async fn play_item_at(&self, item: &QueueItem, start_position: Option<f64>) -> AppResult<()> {
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

    async fn stop_inner(&self) -> AppResult<()> {
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

    async fn advance_after_finished(
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

    /// Last artists of `items` (oldest → newest) used as Flow's cross-batch
    /// diversity memory, so artist caps do not reset on every refill.
    fn recent_artist_window(items: &[QueueItem]) -> Vec<String> {
        const WINDOW: usize = 15;
        items[items.len().saturating_sub(WINDOW)..]
            .iter()
            .map(|item| item.artist.clone())
            .collect()
    }

    async fn refill_flow_queue(
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

    async fn refill_random_queue(&self) -> AppResult<Option<QueueItem>> {
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

    pub async fn enqueue_track(&self, track_id: &str, play_next: bool) -> AppResult<bool> {
        self.prepare_track_source(track_id).await?;
        let track = match self.track_repo.find_by_id(track_id).await? {
            Some(t) => t,
            None => {
                if let Some(pool) = &self.pool {
                    type ExtRow = (String, String, Option<String>, Option<f64>, Option<String>, Option<String>);
                    let ext: Option<ExtRow> = sqlx::query_as(
                        "SELECT title, artist, album, duration_secs, cover_art_url, preview_url FROM external_tracks WHERE id = ?"
                    )
                    .bind(track_id)
                    .fetch_optional(pool)
                    .await
                    .unwrap_or(None);

                    if let Some((title, artist, album, duration_secs, cover_art_url, preview_url)) =
                        ext
                    {
                        let _ = crate::recommendations::mixes::ensure_online_track(
                            pool,
                            &OnlineTrackInput {
                                id: track_id,
                                title: &title,
                                artist: Some(&artist),
                                album: album.as_deref(),
                                duration_secs,
                                cover_art_url: cover_art_url.as_deref(),
                                preview_url: preview_url.as_deref(),
                            },
                        )
                        .await;
                        self.track_repo.find_by_id(track_id).await?.ok_or_else(|| {
                            AppError::NotFound(format!(
                                "Track not found after registration: {}",
                                track_id
                            ))
                        })?
                    } else {
                        return Err(AppError::NotFound(format!("Track not found: {}", track_id)));
                    }
                } else {
                    return Err(AppError::NotFound(format!("Track not found: {}", track_id)));
                }
            }
        };

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
            q_guard.enqueue(queue_item, play_next)
        };
        if added {
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

    pub async fn get_playback_state(&self) -> PlaybackStateDto {
        let restored = self.restored_track.read().await.clone();
        let (mut is_paused, mut is_finished, mut pos) = {
            let backend = self.backend.lock().await;
            (
                backend.is_paused(),
                backend.is_finished(),
                backend.position().as_secs_f64(),
            )
        };

        let mut duration = *self.current_duration_secs.read().await;
        let vol = *self.volume.read().await;
        let active_session = self.active_session_id.read().await.clone();
        let q_guard = self.queue.read().await;
        let restored = restored.filter(|saved| {
            q_guard
                .current()
                .is_some_and(|item| item.queue_id == saved.queue_id)
        });
        let has_loaded_track = restored.is_some() || active_session.as_ref().is_some_and(|(_, id)| q_guard.current().is_some_and(|item| &item.queue_id == id));
        if let Some(restored) = restored {
            is_paused = true;
            is_finished = false;
            pos = restored.position_secs;
            duration = restored.duration_secs;
        }
        let is_playing = !is_paused && !is_finished && has_loaded_track;
        let has_active_track = has_loaded_track && (!is_finished || is_paused);

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

        let can_seek = match &*self.active_stream_state.read_or_recover() {
            Some(state) => state.can_seek(),
            None => true,
        };
        let is_buffering = self.is_resolving.load(std::sync::atomic::Ordering::Relaxed)
            || match &*self.active_stream_state.read_or_recover() {
                Some(state) => !is_paused && !is_finished && state.is_buffering(),
                None => false,
            };
        let has_active_track = (has_loaded_track && (!is_finished || is_paused)) || is_buffering;

        PlaybackStateDto {
            current_track_id: if has_active_track {
                q_guard.current().map(|i| i.track_id.clone())
            } else {
                None
            },
            is_playing,
            is_paused,
            can_seek,
            is_buffering,
            position_secs: pos,
            duration_secs: duration,
            volume: vol,
            is_muted: vol == 0.0,
            repeat_mode: q_guard.repeat_mode(),
            is_shuffle: q_guard.is_shuffle(),
            queue_length: q_guard.len(),
            current_queue_index: q_guard.current_index(),
            queue_track_ids,
            source: self.current_source.read().await.clone(),
        }
    }

    async fn emit_queue_updated(&self) {
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
        let (items, current_index, queue_track_ids) = {
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
            )
        };

        let _ = self.event_bus.publish(Event::QueueUpdated {
            items,
            current_index,
            queue_track_ids,
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

    pub async fn clear_remote_audio_cache(&self) -> AppResult<(u64, usize)> {
        let sm_opt = self.stream_manager.read_or_recover().clone();
        if let Some(sm) = sm_opt {
            let res = sm.clear_cache().await?;
            Ok((res.bytes_freed, res.files_removed))
        } else {
            Ok((0, 0))
        }
    }

    pub async fn get_remote_audio_cache_stats(
        &self,
    ) -> AppResult<crate::playback::stream::RemoteAudioCacheStats> {
        let sm_opt = self.stream_manager.read_or_recover().clone();
        if let Some(sm) = sm_opt {
            sm.get_cache_stats().await
        } else {
            Ok(crate::playback::stream::RemoteAudioCacheStats::default())
        }
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
