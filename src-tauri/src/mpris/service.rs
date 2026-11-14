use crate::core::sync::MutexExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use zbus::connection::Builder;
use zbus::interface;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{ObjectPath, OwnedValue, Value};

use crate::core::error::{AppError, AppResult};
use crate::core::event::Event;
use crate::core::processor::CoreProcessor;
use crate::playback::service::{PlaybackService, PlaybackStateDto};

use super::{
    build_mpris_metadata, loop_status_to_repeat_mode, playback_status_from_state,
    repeat_mode_to_loop_status, seconds_to_microseconds, track_id_to_object_path, MprisTrackInfo,
};

mod root;
pub use root::*;
mod player;
mod metadata;
#[allow(unused_imports)]
use metadata::*;

/// Player interface implementation: org.mpris.MediaPlayer2.Player
pub struct MprisPlayer {
    playback_service: Arc<PlaybackService>,
    current_track: Arc<RwLock<Option<MprisTrackInfo>>>,
    cached_metadata: Arc<RwLock<HashMap<String, OwnedValue>>>,
    command_tasks: std::sync::Mutex<Vec<tokio::task::JoinHandle<()>>>,
}

impl MprisPlayer {
    pub fn new(
        playback_service: Arc<PlaybackService>,
        current_track: Arc<RwLock<Option<MprisTrackInfo>>>,
        cached_metadata: Arc<RwLock<HashMap<String, OwnedValue>>>,
    ) -> Self {
        Self {
            playback_service,
            current_track,
            cached_metadata,
            command_tasks: std::sync::Mutex::new(Vec::new()),
        }
    }

    fn spawn_command(&self, future: impl std::future::Future<Output = ()> + Send + 'static) {
        let mut tasks = self.command_tasks.lock_or_recover();
        tasks.retain(|task| !task.is_finished());
        tasks.push(tokio::spawn(future));
    }

    async fn cancel_commands(&self) {
        let tasks =
            std::mem::take(&mut *self.command_tasks.lock_or_recover());
        for task in &tasks {
            task.abort();
        }
        for task in tasks {
            let _ = task.await;
        }
    }

    async fn resume_selected(&self) {
        if self.playback_service.resume_needs_loading().await {
            // A cold bookmark may need network I/O. Ordinary paused playback
            // retains its immediate response and rapid-toggle ordering.
            let ps = self.playback_service.clone();
            self.spawn_command(async move { let _ = ps.resume().await; });
        } else {
            let _ = self.playback_service.resume().await;
        }
    }
}

impl Drop for MprisPlayer {
    fn drop(&mut self) {
        for task in self
            .command_tasks
            .get_mut()
            .unwrap_or_else(|e| e.into_inner())
        {
            task.abort();
        }
    }
}

/// Handle to the running MPRIS service task.
pub struct MprisServiceHandle {
    pub connection: zbus::Connection,
    task_handle: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
    shutdown_done: tokio::sync::Mutex<bool>,
}

impl MprisServiceHandle {
    pub async fn shutdown(&self) {
        let mut done = self.shutdown_done.lock().await;
        if *done {
            return;
        }
        let player = self
            .connection
            .object_server()
            .interface::<_, MprisPlayer>("/org/mpris/MediaPlayer2")
            .await
            .ok();
        // Stop accepting new control calls before joining in-flight commands.
        let _ = self.connection.clone().close().await;
        let task = self
            .task_handle
            .lock_or_recover()
            .take();
        if let Some(task) = task {
            task.abort();
            let _ = task.await;
        }
        if let Some(player) = player {
            player.get().await.cancel_commands().await;
        }
        *done = true;
    }
}

impl Drop for MprisServiceHandle {
    fn drop(&mut self) {
        // A JoinHandle otherwise detaches, keeping the cloned connection alive.
        if let Some(task) = self
            .task_handle
            .get_mut()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            task.abort();
        }
    }
}

/// Starts the native Linux MPRIS D-Bus service for Kaze. The well-known name
/// belongs to this connection until shutdown; playback state never affects it.
pub async fn start_mpris_service(
    processor: Arc<CoreProcessor>,
    app_handle: Option<tauri::AppHandle>,
) -> AppResult<MprisServiceHandle> {
    info!("Initializing Linux MPRIS D-Bus service: org.mpris.MediaPlayer2.kaze");
    // Subscribe before the initial snapshot and registration so neither database
    // I/O nor acquiring the bus name can lose a playback/queue transition.
    let mut event_rx = processor.event_bus().subscribe();
    let player = MprisPlayer::new(
        processor.playback_service(),
        Arc::new(RwLock::new(None)),
        Arc::new(RwLock::new(build_mpris_metadata(None))),
    );
    player.refresh_metadata(&processor).await;

    let conn = Builder::session()
        .map_err(|e| AppError::Playback(format!("Failed to connect to D-Bus session bus: {}", e)))?
        .serve_at("/org/mpris/MediaPlayer2", MprisRoot::new(app_handle))
        .map_err(|e| AppError::Playback(format!("Failed to register MPRIS root interface: {}", e)))?
        .serve_at("/org/mpris/MediaPlayer2", player)
        .map_err(|e| {
            AppError::Playback(format!("Failed to register MPRIS player interface: {}", e))
        })?
        .build()
        .await
        .map_err(|e| AppError::Playback(format!("Failed to build D-Bus connection: {}", e)))?;
    // Request explicitly: this zbus builder queues names by default despite its
    // older documentation. A duplicate must fail, never wait to become owner.
    conn.request_name_with_flags(
        "org.mpris.MediaPlayer2.kaze",
        zbus::fdo::RequestNameFlags::DoNotQueue.into(),
    )
    .await
    .map_err(|e| AppError::Playback(format!("Failed to request D-Bus name: {}", e)))?;
    info!("Linux MPRIS D-Bus service registered successfully");

    let conn_for_loop = conn.clone();
    let task_handle = tokio::spawn(async move {
        let player_iface = match conn_for_loop
            .object_server()
            .interface::<_, MprisPlayer>("/org/mpris/MediaPlayer2")
            .await
        {
            Ok(iface) => iface,
            Err(e) => {
                error!("Failed to obtain MPRIS Player interface reference: {}", e);
                return;
            }
        };
        let mut previous = HashMap::new();
        loop {
            let player = player_iface.get().await;
            player.refresh_metadata(&processor).await;
            let emitter = player_iface.signal_emitter();
            if let Err(e) = player.changed_properties(&mut previous, emitter).await {
                debug!("Failed to publish MPRIS changes: {}", e);
            }
            drop(player);
            loop {
                match event_rx.recv().await {
                    Ok(Event::LibraryMetadataUpdated) => {
                        let player = player_iface.get().await;
                        *player.current_track.write().await = None;
                        break;
                    }
                    Ok(Event::PlaybackSeeked { position_secs }) => {
                        let _ =
                            MprisPlayer::seeked(emitter, seconds_to_microseconds(position_secs))
                                .await;
                        break;
                    }
                    // Existing ticks detect only real state/capability edges,
                    // such as progressive audio becoming seekable. Position is
                    // never part of PropertiesChanged, and unchanged snapshots
                    // emit nothing.
                    Ok(
                        Event::PlaybackPositionChanged { .. }
                        | Event::PlaybackStarted { .. }
                        | Event::PlaybackPaused { .. }
                        | Event::PlaybackResumed { .. }
                        | Event::PlaybackStopped { .. }
                        | Event::PlaybackBuffering { .. }
                        | Event::PlaybackBufferingChanged { .. }
                        | Event::PlaybackError { .. }
                        | Event::PlaybackVolumeChanged { .. }
                        | Event::PlaybackCapabilitiesChanged { .. }
                        | Event::QueueUpdated { .. }
                        | Event::TrackFinished { .. },
                    ) => break,
                    Ok(_) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        debug!(
                            "MPRIS event listener lagged by {} events; resynchronizing",
                            n
                        );
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
                }
            }
        }
    });
    Ok(MprisServiceHandle {
        connection: conn,
        task_handle: std::sync::Mutex::new(Some(task_handle)),
        shutdown_done: tokio::sync::Mutex::new(false),
    })
}

#[cfg(test)]
mod tests;
