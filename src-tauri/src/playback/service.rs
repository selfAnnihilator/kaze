use crate::core::command::RepeatMode;
use crate::core::error::{AppError, AppResult};
use crate::core::event::{Event, QueueItem};
use crate::core::event_bus::EventBus;
use crate::database::repositories::TrackRepository;
use crate::playback::backend::AudioBackend;
use crate::playback::queue::PlaybackQueue;
use crate::playback::session::{RestoredTrack, SavedPlayback, SETTING_KEY};
use crate::database::repositories::{SettingsRepository, SqliteSettingsRepository};
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
}

#[derive(Default)]
struct ActiveFlow {
    generation: u64,
    manual_queue_ids: HashSet<String>,
    user_id: String,
    ratio: f64,
    generated_queue_ids: HashSet<String>,
    reasons: HashMap<String, FlowExplanation>,
}

pub struct PlaybackService {
    backend: Arc<Mutex<Box<dyn AudioBackend>>>,
    queue: Arc<RwLock<PlaybackQueue>>,
    event_bus: Arc<EventBus>,
    track_repo: Arc<dyn TrackRepository>,
    current_duration_secs: Arc<RwLock<f64>>,
    current_source: Arc<RwLock<String>>,
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
    shutdown_done: Mutex<bool>,
    shutting_down: std::sync::atomic::AtomicBool,
    /// Authoritative session token and queue-entry identity for real playback.
    /// Generated fresh at each `play_item` call; cleared on stop/track-finish.
    active_session_id: Arc<RwLock<Option<(String, String)>>>,
    /// Generation token to cancel obsolete track transitions during rapid skips.
    play_generation: Arc<std::sync::atomic::AtomicU64>,
    /// Indicates whether an audio source is actively being resolved over network.
    is_resolving: Arc<std::sync::atomic::AtomicBool>,
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
            volume: Arc::new(RwLock::new(0.8)),
            stream_manager: Arc::new(std::sync::RwLock::new(None)),
            active_stream_state: Arc::new(std::sync::RwLock::new(None)),
            pool,
            restore_ready: OnceCell::new(),
            restored_track: RwLock::new(None),
            last_saved_bookmark: Mutex::new(None),
            preresolution_handle: Arc::new(Mutex::new(None)),
            monitor_task: std::sync::Mutex::new(None),
            shutdown_done: Mutex::new(false),
            shutting_down: std::sync::atomic::AtomicBool::new(false),
            active_session_id: Arc::new(RwLock::new(None)),
            play_generation: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            is_resolving: Arc::new(std::sync::atomic::AtomicBool::new(false)),
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
        *service.monitor_task.lock().unwrap_or_else(|e| e.into_inner()) = Some(task);

        service
    }

    pub fn set_stream_manager(&self, sm: Arc<StreamPlaybackManager>) {
        sm.set_event_bus(self.event_bus.clone());
        let mut w = self.stream_manager.write().unwrap();
        *w = Some(sm);
    }

    /// Restore metadata/controls only. Audio and history begin on an explicit Play.
    pub async fn initialize_saved_playback(&self) {
        self.restore_ready.get_or_init(|| async {
            if let Err(error) = self.restore_saved_playback().await {
                tracing::warn!(%error, "Could not restore playback bookmark; starting empty");
            }
        }).await;
    }

    async fn restore_saved_playback(&self) -> AppResult<()> {
        let Some(pool) = &self.pool else { return Ok(()); };
        let Some(value) = SqliteSettingsRepository::new(pool.clone()).get_setting(SETTING_KEY).await? else { return Ok(()); };
        let saved: SavedPlayback = serde_json::from_str(&value)
            .map_err(|_| AppError::Validation("Invalid saved playback bookmark".into()))?;
        if !saved.valid() {
            return Err(AppError::Validation("Invalid saved playback bookmark".into()));
        }
        let _operation = self.flow_operation.lock().await;
        self.backend.lock().await.set_volume(saved.volume)?;
        *self.volume.write().await = saved.volume;
        if saved.has_current_track {
            // Do not open files or resolve online sources until the user plays.
            // Missing media remains visible and can be retried or skipped.
            *self.restored_track.write().await = Some(RestoredTrack {
                queue_id: saved.queue.current().unwrap().queue_id.clone(),
                position_secs: saved.position_secs.min(saved.duration_secs),
                duration_secs: saved.duration_secs,
            });
        }
        *self.queue.write().await = saved.queue;
        *self.last_saved_bookmark.lock().await = Some(value);
        Ok(())
    }

    /// Called under the queue operation lock, periodically and before clean Stop.
    async fn save_playback_bookmark(&self) -> AppResult<()> {
        if self.restore_ready.get().is_none() || self.is_resolving.load(std::sync::atomic::Ordering::Acquire) { return Ok(()); }
        let Some(pool) = &self.pool else { return Ok(()); };
        let state = self.get_playback_state().await;
        let saved = SavedPlayback {
            version: 1,
            queue: self.queue.read().await.clone(),
            has_current_track: state.current_track_id.is_some(),
            position_secs: state.position_secs,
            duration_secs: state.duration_secs,
            volume: state.volume,
        };
        if !saved.valid() { return Ok(()); }
        let value = serde_json::to_string(&saved).map_err(|e| AppError::Internal(e.to_string()))?;
        let mut previous = self.last_saved_bookmark.lock().await;
        if previous.as_ref() == Some(&value) { return Ok(()); }
        SqliteSettingsRepository::new(pool.clone()).set_setting(SETTING_KEY, &value).await?;
        *previous = Some(value);
        Ok(())
    }

    pub fn set_flow_engine(&self, fe: Arc<crate::recommendations::FlowEngine>) {
        let mut w = self.flow_engine.write().unwrap();
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
            self.active_session_id
                .read()
                .await
                .as_ref()
                .is_some_and(|(_, id)| id == &item.queue_id)
                || self.is_resolving.load(std::sync::atomic::Ordering::SeqCst)
        } else {
            false
        };

        // Clear all old upcoming entries before generating. Preserve the current
        // queue ID/audio token and Previous history; no playback event is emitted.
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
        let tracks = fe
            .generate_playback_tracks_after(
                user_id,
                ratio,
                20,
                &exclude,
                current
                    .as_ref()
                    .filter(|_| preserve_current)
                    .map(|i| i.artist.as_str()),
            )
            .await?;
        if self
            .flow_generation
            .load(std::sync::atomic::Ordering::SeqCst)
            != generation
        {
            return Ok(());
        }
        if tracks.is_empty() {
            return Err(AppError::Playback(
                "No eligible songs for Flow. Play or like more music, or add songs to your library.".into(),
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
                "Familiar mix · Your favorites"
            } else if ratio <= 0.25 {
                "Rediscovery mix · Deep cuts"
            } else {
                "Balanced mix · Familiar & Rediscover"
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
            s.generation = self.active_flow.read().await.as_ref().filter(|f| f.user_id == user_id).map(|f| f.generation);
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
            Some(FlowExplanation { primary: "existing_queue".into(), from_existing_queue: true, ..Default::default() })
        } else {
            Some(FlowExplanation { primary: "continuing_track".into(), ..Default::default() })
        }
    }

    async fn stop_flow_inner(&self) {
        self.flow_generation.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
        self.flow_generation.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
            .map(|f| (f.generation, f.generated_queue_ids.clone()));
        let Some((generation, generated_ids)) = active else {
            return Ok(());
        };
        let (excluded, previous_artist) = {
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
                q.current().map(|item| item.artist.clone()),
            )
        };
        let tracks = fe
            .generate_playback_tracks_after(
                user_id,
                state.familiarity_ratio,
                20,
                &excluded,
                previous_artist.as_deref(),
            )
            .await?;
        let mut active = self.active_flow.write().await;
        let Some(flow) = active.as_mut().filter(|flow| {
            flow.generation == generation
                && self
                    .flow_generation
                    .load(std::sync::atomic::Ordering::SeqCst)
                    == generation
        }) else {
            return Ok(());
        };
        flow.ratio = state.familiarity_ratio;
        let mut q = self.queue.write().await;
        let current_index = q.current_index();
        let start = current_index.map_or(0, |index| index + 1);
        let mut rebuilt = q.items()[..start].to_vec();
        // Keep original evidence for the current/history entries. Replace only
        // generated upcoming slots, preserving manual entries at their positions.
        flow.generated_queue_ids.retain(|id| rebuilt.iter().any(|item| &item.queue_id == id));
        flow.reasons.retain(|id, _| rebuilt.iter().any(|item| &item.track_id == id));
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
                if let Some(item) = replacements.next() { rebuilt.push(item); }
            } else {
                rebuilt.push(old.clone());
            }
        }
        rebuilt.extend(replacements);
        q.set_queue(rebuilt, current_index);
        drop(q);
        drop(active);
        self.emit_queue_updated().await;
        Ok(())
    }

    pub async fn cancel_active_stream(&self) {
        if let Some(state) = self.active_stream_state.write().unwrap().take() {
            state.set_cancelled();
        }
        let sm_opt = self.stream_manager.read().unwrap().clone();
        if let Some(sm) = sm_opt {
            sm.cancel_active_stream().await;
        }
    }

    /// Background task monitoring playback position and track completion.
    async fn run_playback_monitor_loop(&self) {
        let mut interval = tokio::time::interval(Duration::from_millis(250));
        let mut was_playing = false;
        let mut last_checkpoint = tokio::time::Instant::now();

        loop {
            interval.tick().await;
            let operation = self.flow_operation.lock().await;
            if self.shutting_down.load(std::sync::atomic::Ordering::Acquire) {
                break;
            }
            if last_checkpoint.elapsed() >= Duration::from_secs(10) {
                if let Err(error) = self.save_playback_bookmark().await {
                    tracing::warn!(%error, "Could not checkpoint playback bookmark");
                }
                last_checkpoint = tokio::time::Instant::now();
            }
            let session_queue_id = self.active_session_id.read().await.as_ref().map(|(_, q)| q.clone());
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

            if !is_paused && !is_finished && duration > 0.0 {
                was_playing = true;

                // Always read from authoritative source — no stale local cache.
                let session_id = self
                    .active_session_id
                    .read()
                    .await
                    .as_ref().map(|(id, _)| id.clone())
                    .unwrap_or_default();

                let _ = self.event_bus.publish(Event::PlaybackPositionChanged {
                    session_id,
                    position_secs: pos_secs,
                    duration_secs: duration,
                });
            } else if was_playing && is_finished && duration > 0.0 {
                // Track finished!
                was_playing = false;

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
                        seconds_listened: duration,
                        completed: true,
                    });
                }

                // A manual selection after this snapshot must supersede automatic advancement.
                let generation = self.play_generation.load(std::sync::atomic::Ordering::SeqCst);
                drop(operation);
                let _ = self.advance_after_finished(generation, session_queue_id.as_deref()).await;
            }
        }
    }

    /// Plays a track directly, clearing existing queue or adding to top.
    pub async fn play_track(&self, track_id: &str, source: Option<String>) -> AppResult<()> {
        let track = match self.track_repo.find_by_id(track_id).await? {
            Some(t) => t,
            None => {
                // If not in track_repo, check external_tracks for online / discovery track
                if let Some(pool) = &self.pool {
                    let ext: Option<(String, String, Option<String>, Option<f64>, Option<String>, Option<String>)> = sqlx::query_as(
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
                            track_id,
                            &title,
                            Some(&artist),
                            album.as_deref(),
                            duration_secs,
                            cover_art_url.as_deref(),
                            preview_url.as_deref(),
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

    /// Plays an online or remote track, ensuring it is registered and streamed through the unified engine.
    pub async fn play_online_track(
        &self,
        track_id: &str,
        title: &str,
        artist: &str,
        album: Option<&str>,
        duration_secs: Option<f64>,
        cover_art_url: Option<&str>,
        preview_url: Option<&str>,
        source: Option<String>,
    ) -> AppResult<()> {
        if let Some(pool) = &self.pool {
            crate::recommendations::mixes::ensure_online_track(
                pool,
                track_id,
                title,
                Some(artist),
                album,
                duration_secs,
                cover_art_url,
                preview_url,
            )
            .await?;
        }

        self.play_track(track_id, source).await
    }

    /// Enqueues an online or remote track, ensuring it is registered in the database.
    pub async fn enqueue_online_track(
        &self,
        track_id: &str,
        title: &str,
        artist: &str,
        album: Option<&str>,
        duration_secs: Option<f64>,
        cover_art_url: Option<&str>,
        preview_url: Option<&str>,
        play_next: bool,
    ) -> AppResult<bool> {
        if let Some(pool) = &self.pool {
            crate::recommendations::mixes::ensure_online_track(
                pool,
                track_id,
                title,
                Some(artist),
                album,
                duration_secs,
                cover_art_url,
                preview_url,
            )
            .await?;
        }

        self.enqueue_track(track_id, play_next).await
    }

    /// Plays an item from the current queue by index.
    pub async fn play_queue_index(&self, index: usize) -> AppResult<()> {
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
        let operation = self.flow_operation.lock().await;
        if self.shutting_down.load(std::sync::atomic::Ordering::Acquire) {
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

        // Cancel any active progressive stream before starting a new track
        self.cancel_active_stream().await;

        let is_online = track.format == "online" || track.file_path.starts_with("online://");
        if is_online {
            let _ = self.event_bus.publish(Event::PlaybackBuffering {
                session_id: String::new(),
                track_id: item.track_id.clone(),
            });
        }
        drop(operation);
        let prepared_res = if is_online {
            let sm_opt = self.stream_manager.read().unwrap().clone();
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
            Ok(crate::playback::stream::PreparedPlayback::ProgressiveStream { target_path, duration, state, .. })
                if start_position.is_some_and(|position| position > 0.0) => {
                let ready = tokio::time::timeout(Duration::from_secs(120), async {
                    loop {
                        if self.play_generation.load(std::sync::atomic::Ordering::SeqCst) != generation
                            || self.shutting_down.load(std::sync::atomic::Ordering::Acquire) {
                            return Err(AppError::Playback("Playback request superseded".into()));
                        }
                        match state.status.lock().unwrap().clone() {
                            crate::playback::progressive::StreamStatus::Completed => return Ok(()),
                            crate::playback::progressive::StreamStatus::Failed(error) => return Err(AppError::Playback(error)),
                            crate::playback::progressive::StreamStatus::Cancelled => return Err(AppError::Playback("Stream cancelled".into())),
                            _ => {}
                        }
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }).await;
                match ready {
                    Ok(Ok(())) => Ok(crate::playback::stream::PreparedPlayback::LocalFile { path: target_path, duration }),
                    Ok(Err(error)) => Err(error),
                    Err(_) => Err(AppError::Playback("Timed out buffering the saved song. Try playing again.".into())),
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
                *self.active_stream_state.write().unwrap() = None;
                let mut backend = self.backend.lock().await;
                backend.load_and_play(&path)?;
                if let Some(position) = start_position {
                    if let Err(error) = backend.seek(Duration::from_secs_f64(position.min(duration).max(0.0))) {
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
                *self.active_stream_state.write().unwrap() = Some(state);
                let mut backend = self.backend.lock().await;
                backend.load_and_play_source(source, Some(&part_path))?;
                (target_path.to_string_lossy().to_string(), duration)
            }
        };

        *self.current_duration_secs.write().await = actual_duration;
        *self.restored_track.write().await = None;

        let sm_opt = self.stream_manager.read().unwrap().clone();
        if let Some(sm) = sm_opt {
            sm.set_active_playing_path(Some(PathBuf::from(&file_path_to_protect)))
                .await;
        }

        // Generate a fresh session token for this playback session.
        let session_id = Uuid::new_v4().to_string();
        *self.active_session_id.write().await = Some((session_id.clone(), item.queue_id.clone()));

        let src = self.current_source.read().await.clone();
        let _ = self.event_bus.publish(Event::PlaybackStarted {
            session_id,
            track_id: item.track_id.clone(),
            title: item.title.clone(),
            artist: item.artist.clone(),
            duration_secs: actual_duration,
            source: src,
        });

        if start_position.is_some() {
            let position_secs = self.backend.lock().await.position().as_secs_f64();
            let _ = self.event_bus.publish(Event::PlaybackSeeked { position_secs });
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
            .as_ref().map(|(id, _)| id.clone())
            .unwrap_or_default();

        let _ = self.event_bus.publish(Event::PlaybackPaused {
            session_id,
            track_id,
            position_secs: pos,
        });
        Ok(())
    }

    pub async fn resume(&self) -> AppResult<()> {
        let restored = self.restored_track.read().await.clone();
        if let Some(restored) = restored {
            let item = self.queue.read().await.current().filter(|item| item.queue_id == restored.queue_id).cloned();
            if let Some(item) = item {
                return self.play_item_at(&item, Some(restored.position_secs)).await;
            }
        }
        let mut backend = self.backend.lock().await;
        if self.shutting_down.load(std::sync::atomic::Ordering::Acquire) {
            return Err(AppError::Playback("Kaze is shutting down".into()));
        }

        backend.resume()?;

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
            .as_ref().map(|(id, _)| id.clone())
            .unwrap_or_default();

        let _ = self.event_bus.publish(Event::PlaybackResumed {
            session_id,
            track_id,
            position_secs: pos,
        });
        Ok(())
    }

    pub async fn resume_needs_loading(&self) -> bool {
        self.restored_track.read().await.is_some()
    }

    /// Cancel pending media preparation before the core waits for commands to drain.
    pub fn request_shutdown(&self) {
        if !self.shutting_down.swap(true, std::sync::atomic::Ordering::AcqRel) {
            self.play_generation.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    /// Stop the monitor and invalidate pending resolution before stopping audio.
    pub async fn shutdown(&self) -> AppResult<()> {
        let mut done = self.shutdown_done.lock().await;
        if *done { return Ok(()); }
        self.request_shutdown();
        let task = self.monitor_task.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(task) = task { task.abort(); let _ = task.await; }
        let bookmark = {
            let _operation = self.flow_operation.lock().await;
            self.save_playback_bookmark().await
        };
        let result = self.stop().await;
        *done = true;
        result.and(bookmark)
    }

    pub async fn stop(&self) -> AppResult<()> {
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

        let sm_opt = self.stream_manager.read().unwrap().clone();
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
                restored.position_secs = position_secs.clamp(0.0, restored.duration_secs);
                let _ = self.event_bus.publish(Event::PlaybackSeeked { position_secs: restored.position_secs });
                return Ok(());
            }
        }
        let can_seek = match &*self.active_stream_state.read().unwrap() {
            Some(state) => state.is_completed(),
            None => true,
        };
        if !can_seek {
            return Err(AppError::Playback("Seeking is unavailable while audio is streaming. It will become available once the track is fully cached.".into()));
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
        let operation = self.flow_operation.lock().await;
        let mut next_item = {
            let mut q_guard = self.queue.write().await;
            q_guard.next().cloned()
        };

        if next_item.is_none() {
            let src = self.current_source.read().await.clone();
            if src == "flow" {
                let fe_opt = self.flow_engine.read().unwrap().clone();
                if let Some(fe) = fe_opt {
                    next_item = self.refill_flow_queue(&fe).await?;
                }
            }
            if next_item.is_none() && src != "flow" {
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

    async fn advance_after_finished(&self, generation: u64, queue_id: Option<&str>) -> AppResult<()> {
        let operation = self.flow_operation.lock().await;
        if self.shutting_down.load(std::sync::atomic::Ordering::Acquire)
            || self.play_generation.load(std::sync::atomic::Ordering::SeqCst) != generation
            || self.queue.read().await.current().map(|i| i.queue_id.as_str()) != queue_id {
            return Ok(());
        }
        let mut next_item = {
            let mut q_guard = self.queue.write().await;
            q_guard.next_after_finish().cloned()
        };

        if next_item.is_none() {
            let src = self.current_source.read().await.clone();
            if src == "flow" {
                let fe_opt = self.flow_engine.read().unwrap().clone();
                if let Some(fe) = fe_opt {
                    next_item = self.refill_flow_queue(&fe).await?;
                }
            }
            if next_item.is_none() && src != "flow" {
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

    async fn refill_flow_queue(
        &self,
        fe: &Arc<crate::recommendations::FlowEngine>,
    ) -> AppResult<Option<QueueItem>> {
        let (user_id, ratio, generation) = match self.active_flow.read().await.as_ref() {
            Some(f) => (f.user_id.clone(), f.ratio, f.generation),
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
                q.current().map(|i| i.artist.clone()),
            )
        };
        let mut tracks = fe
            .generate_playback_tracks_after(&user_id, ratio, 15, &recent, artist.as_deref())
            .await?;
        if tracks.is_empty() {
            tracks = fe
                .generate_playback_tracks_after(
                    &user_id,
                    ratio,
                    15,
                    &current.into_iter().collect::<Vec<_>>(),
                    artist.as_deref(),
                )
                .await?;
        }
        let mut flow = self.active_flow.write().await;
        let Some(f) = flow.as_mut().filter(|f| f.generation == generation && self.flow_generation.load(std::sync::atomic::Ordering::SeqCst) == generation) else {
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
            q.next().cloned()
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
        let mut candidates = self
            .track_repo
            .list_tracks(0, 10_000, None, true)
            .await?
            .into_iter()
            .filter(|track| current_track_id.as_deref() != Some(track.id.as_str()))
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
            q_guard.next().cloned()
        };
        self.emit_queue_updated().await;
        Ok(next)
    }

    pub async fn previous(&self) -> AppResult<()> {
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
        let track = match self.track_repo.find_by_id(track_id).await? {
            Some(t) => t,
            None => {
                if let Some(pool) = &self.pool {
                    let ext: Option<(String, String, Option<String>, Option<f64>, Option<String>, Option<String>)> = sqlx::query_as(
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
                            track_id,
                            &title,
                            Some(&artist),
                            album.as_deref(),
                            duration_secs,
                            cover_art_url.as_deref(),
                            preview_url.as_deref(),
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
        let ids: std::collections::HashSet<String> = self.queue.read().await.items().iter().map(|item| item.track_id.clone()).collect();
        let mut changed = false;
        for id in ids {
            if let Ok(Some(track)) = self.track_repo.find_by_id(&id).await {
                changed |= self.queue.write().await.update_metadata(&id, &track.title, track.artist_name.as_deref().unwrap_or("Unknown Artist"));
            }
        }
        if changed { self.emit_queue_updated().await; }
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
        let q_guard = self.queue.read().await;
        if let Some(restored) = restored.filter(|saved| q_guard.current().is_some_and(|item| item.queue_id == saved.queue_id)) {
            is_paused = true;
            is_finished = false;
            pos = restored.position_secs;
            duration = restored.duration_secs;
        }
        let is_playing = !is_paused && !is_finished && duration > 0.0;
        let has_active_track = duration > 0.0 && (!is_finished || is_paused);

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

        let can_seek = match &*self.active_stream_state.read().unwrap() {
            Some(state) => state.is_completed(),
            None => true,
        };
        let is_buffering = self.is_resolving.load(std::sync::atomic::Ordering::Relaxed)
            || match &*self.active_stream_state.read().unwrap() {
                Some(state) => state.is_buffering(),
                None => false,
            };
        let has_active_track = (duration > 0.0 && (!is_finished || is_paused)) || is_buffering;

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
        }
    }

    async fn emit_queue_updated(&self) {
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

        let sm_opt = self.stream_manager.read().unwrap().clone();
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
        let sm_opt = self.stream_manager.read().unwrap().clone();
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
        let sm_opt = self.stream_manager.read().unwrap().clone();
        if let Some(sm) = sm_opt {
            sm.get_cache_stats().await
        } else {
            Ok(crate::playback::stream::RemoteAudioCacheStats::default())
        }
    }
}
