use crate::core::command::RepeatMode;
use crate::core::error::{AppError, AppResult};
use crate::core::event::{Event, QueueItem};
use crate::core::event_bus::EventBus;
use crate::database::repositories::TrackRepository;
use crate::playback::backend::AudioBackend;
use crate::playback::queue::PlaybackQueue;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackStateDto {
    pub current_track_id: Option<String>,
    pub is_playing: bool,
    pub is_paused: bool,
    pub position_secs: f64,
    pub duration_secs: f64,
    pub volume: f32,
    pub repeat_mode: RepeatMode,
    pub is_shuffle: bool,
    pub queue_length: usize,
    pub current_queue_index: Option<usize>,
    pub queue_track_ids: Vec<String>,
}

pub struct PlaybackService {
    backend: Arc<Mutex<Box<dyn AudioBackend>>>,
    queue: Arc<RwLock<PlaybackQueue>>,
    event_bus: Arc<EventBus>,
    track_repo: Arc<dyn TrackRepository>,
    current_duration_secs: Arc<RwLock<f64>>,
    current_source: Arc<RwLock<String>>,
    volume: Arc<RwLock<f32>>,
}

impl PlaybackService {
    pub fn new(
        backend: Box<dyn AudioBackend>,
        track_repo: Arc<dyn TrackRepository>,
        event_bus: Arc<EventBus>,
    ) -> Arc<Self> {
        let service = Arc::new(Self {
            backend: Arc::new(Mutex::new(backend)),
            queue: Arc::new(RwLock::new(PlaybackQueue::new())),
            event_bus,
            track_repo,
            current_duration_secs: Arc::new(RwLock::new(0.0)),
            current_source: Arc::new(RwLock::new("library".to_string())),
            volume: Arc::new(RwLock::new(0.8)),
        });

        // Spawn periodic position monitor and track finish detector
        let s_clone = service.clone();
        tokio::spawn(async move {
            s_clone.run_playback_monitor_loop().await;
        });

        service
    }

    /// Background task monitoring playback position and track completion.
    async fn run_playback_monitor_loop(&self) {
        let mut interval = tokio::time::interval(Duration::from_millis(250));
        let mut was_playing = false;
        let mut current_track_id_cache: Option<String> = None;

        loop {
            interval.tick().await;

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
                if current_track_id_cache.is_none() {
                    let q_guard = self.queue.read().await;
                    if let Some(item) = q_guard.current() {
                        current_track_id_cache = Some(item.track_id.clone());
                    }
                }

                let _ = self.event_bus.publish(Event::PlaybackPositionChanged {
                    position_secs: pos_secs,
                    duration_secs: duration,
                });
            } else if was_playing && is_finished && duration > 0.0 {
                // Track finished!
                was_playing = false;
                let track_id_finished = current_track_id_cache.take();
                if let Some(track_id) = track_id_finished {
                    info!(%track_id, "Track finished playing to end of stream");
                    let _ = self.event_bus.publish(Event::TrackFinished {
                        track_id,
                        seconds_listened: duration,
                        completed: true,
                    });
                }

                // Automatically advance, honoring repeat-one on natural completion.
                let _ = self.advance_after_finished().await;
            }
        }
    }

    /// Plays a track directly, clearing existing queue or adding to top.
    pub async fn play_track(&self, track_id: &str, source: Option<String>) -> AppResult<()> {
        let track = self
            .track_repo
            .find_by_id(track_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Track not found: {}", track_id)))?;

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

        self.play_item(&queue_item).await
    }

    /// Plays an item from the current queue by index.
    pub async fn play_queue_index(&self, index: usize) -> AppResult<()> {
        let item = {
            let mut q_guard = self.queue.write().await;
            q_guard.set_current_index(index).cloned()
        };

        if let Some(item) = item {
            self.play_item(&item).await?;
        }
        Ok(())
    }

    async fn play_item(&self, item: &QueueItem) -> AppResult<()> {
        let track = self
            .track_repo
            .find_by_id(&item.track_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Track not found: {}", item.track_id)))?;

        let is_online = track.format == "online" || track.file_path.starts_with("online://");
        if is_online {
            {
                let mut backend = self.backend.lock().await;
                let _ = backend.stop();
            }
            *self.current_duration_secs.write().await = 0.0;
            let _ = self.event_bus.publish(Event::OnlinePlaybackRequested {
                track_id: track.id,
                title: track.title,
                artist: track.artist_name.unwrap_or_else(|| "Unknown Artist".into()),
                album: track.album_title,
                duration_secs: track.duration_secs,
            });
            self.emit_queue_updated().await;
            return Ok(());
        }

        *self.current_duration_secs.write().await = track.duration_secs;
        self.load_and_play_file(&track.file_path).await?;

        let src = self.current_source.read().await.clone();
        let _ = self.event_bus.publish(Event::PlaybackStarted {
            track_id: item.track_id.clone(),
            title: item.title.clone(),
            artist: item.artist.clone(),
            duration_secs: item.duration_secs,
            source: src,
        });

        self.emit_queue_updated().await;
        Ok(())
    }

    async fn load_and_play_file(&self, path_str: &str) -> AppResult<()> {
        let path = PathBuf::from(path_str);
        let mut backend = self.backend.lock().await;
        backend.load_and_play(&path)
    }

    pub async fn pause(&self) -> AppResult<()> {
        let mut backend = self.backend.lock().await;
        backend.pause()?;

        let pos = backend.position().as_secs_f64();
        let track_id = {
            let q_guard = self.queue.read().await;
            q_guard.current().map(|i| i.track_id.clone()).unwrap_or_default()
        };

        let _ = self.event_bus.publish(Event::PlaybackPaused {
            track_id,
            position_secs: pos,
        });
        Ok(())
    }

    pub async fn resume(&self) -> AppResult<()> {
        let mut backend = self.backend.lock().await;
        backend.resume()?;

        let pos = backend.position().as_secs_f64();
        let track_id = {
            let q_guard = self.queue.read().await;
            q_guard.current().map(|i| i.track_id.clone()).unwrap_or_default()
        };

        let _ = self.event_bus.publish(Event::PlaybackResumed {
            track_id,
            position_secs: pos,
        });
        Ok(())
    }

    pub async fn stop(&self) -> AppResult<()> {
        let mut backend = self.backend.lock().await;
        backend.stop()?;
        *self.current_duration_secs.write().await = 0.0;

        let _ = self.event_bus.publish(Event::PlaybackStopped);
        Ok(())
    }

    pub async fn seek(&self, position_secs: f64) -> AppResult<()> {
        let dur = Duration::from_secs_f64(position_secs.max(0.0));
        let mut backend = self.backend.lock().await;
        backend.seek(dur)?;

        let _ = self.event_bus.publish(Event::PlaybackSeeked {
            position_secs,
        });
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
        let mut next_item = {
            let mut q_guard = self.queue.write().await;
            q_guard.next().cloned()
        };

        if next_item.is_none() {
            next_item = self.refill_random_queue().await?;
        }

        if let Some(item) = next_item {
            self.play_item(&item).await?;
        } else {
            self.stop().await?;
        }
        Ok(())
    }

    async fn advance_after_finished(&self) -> AppResult<()> {
        let mut next_item = {
            let mut q_guard = self.queue.write().await;
            q_guard.next_after_finish().cloned()
        };

        if next_item.is_none() {
            next_item = self.refill_random_queue().await?;
        }

        if let Some(item) = next_item {
            self.play_item(&item).await?;
        } else {
            self.stop().await?;
        }
        Ok(())
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
        let prev_item = {
            let mut q_guard = self.queue.write().await;
            q_guard.previous().cloned()
        };

        if let Some(item) = prev_item {
            self.play_item(&item).await?;
        }
        Ok(())
    }

    pub async fn enqueue_track(&self, track_id: &str, play_next: bool) -> AppResult<bool> {
        let track = self
            .track_repo
            .find_by_id(track_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Track not found: {}", track_id)))?;

        let queue_item = QueueItem {
            queue_id: Uuid::new_v4().to_string(),
            track_id: track.id,
            title: track.title,
            artist: track.artist_name.unwrap_or_else(|| "Unknown Artist".into()),
            duration_secs: track.duration_secs,
        };

        let added = {
            let mut q_guard = self.queue.write().await;
            q_guard.enqueue(queue_item, play_next)
        };

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

    pub async fn clear_queue(&self) {
        {
            let mut q_guard = self.queue.write().await;
            q_guard.clear();
        }
        self.emit_queue_updated().await;
    }

    pub async fn set_shuffle(&self, enabled: bool) {
        {
            let mut q_guard = self.queue.write().await;
            q_guard.set_shuffle(enabled);
        }
        self.emit_queue_updated().await;
    }

    pub async fn set_repeat_mode(&self, mode: RepeatMode) {
        let mut q_guard = self.queue.write().await;
        q_guard.set_repeat_mode(mode);
    }

    pub async fn get_playback_state(&self) -> PlaybackStateDto {
        let (is_paused, is_finished, pos) = {
            let backend = self.backend.lock().await;
            (
                backend.is_paused(),
                backend.is_finished(),
                backend.position().as_secs_f64(),
            )
        };

        let duration = *self.current_duration_secs.read().await;
        let vol = *self.volume.read().await;
        let q_guard = self.queue.read().await;
        let is_playing = !is_paused && !is_finished && duration > 0.0;
        let has_active_track = duration > 0.0 && (!is_finished || is_paused);

        let queue_track_ids: Vec<String> = if has_active_track {
            if let Some(curr) = q_guard.current_index() {
                q_guard.items().iter().skip(curr + 1).map(|i| i.track_id.clone()).collect()
            } else {
                q_guard.items().iter().map(|i| i.track_id.clone()).collect()
            }
        } else {
            q_guard.items().iter().map(|i| i.track_id.clone()).collect()
        };

        PlaybackStateDto {
            current_track_id: if has_active_track {
                q_guard.current().map(|i| i.track_id.clone())
            } else {
                None
            },
            is_playing,
            is_paused,
            position_secs: pos,
            duration_secs: duration,
            volume: vol,
            repeat_mode: q_guard.repeat_mode(),
            is_shuffle: q_guard.is_shuffle(),
            queue_length: q_guard.len(),
            current_queue_index: q_guard.current_index(),
            queue_track_ids,
        }
    }

    async fn emit_queue_updated(&self) {
        let q_guard = self.queue.read().await;
        let has_active_track = q_guard.current_index().is_some();

        let queue_track_ids: Vec<String> = if has_active_track {
            if let Some(curr) = q_guard.current_index() {
                q_guard.items().iter().skip(curr + 1).map(|i| i.track_id.clone()).collect()
            } else {
                q_guard.items().iter().map(|i| i.track_id.clone()).collect()
            }
        } else {
            q_guard.items().iter().map(|i| i.track_id.clone()).collect()
        };

        let _ = self.event_bus.publish(Event::QueueUpdated {
            items: q_guard.items().to_vec(),
            current_index: q_guard.current_index(),
            queue_track_ids,
        });
    }
}
