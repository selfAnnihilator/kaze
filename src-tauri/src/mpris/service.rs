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

/// Root interface implementation: org.mpris.MediaPlayer2
pub struct MprisRoot {
    app_handle: Option<tauri::AppHandle>,
}

impl MprisRoot {
    pub fn new(app_handle: Option<tauri::AppHandle>) -> Self {
        Self { app_handle }
    }
}

#[interface(name = "org.mpris.MediaPlayer2")]
impl MprisRoot {
    /// Brings the media player's user interface to the front.
    async fn raise(&self) -> zbus::fdo::Result<()> {
        if let Some(ref handle) = self.app_handle {
            use tauri::Manager;
            if let Some(window) = handle.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        Ok(())
    }

    /// Causes the media player to close immediately.
    async fn quit(&self) -> zbus::fdo::Result<()> {
        if let Some(ref handle) = self.app_handle {
            handle.exit(0);
        }
        Ok(())
    }

    #[zbus(property)]
    fn can_quit(&self) -> bool {
        self.app_handle.is_some()
    }

    #[zbus(property)]
    fn can_raise(&self) -> bool {
        self.app_handle.is_some()
    }

    #[zbus(property)]
    fn has_track_list(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn identity(&self) -> &'static str {
        "Kaze"
    }

    #[zbus(property)]
    fn desktop_entry(&self) -> &'static str {
        "kaze"
    }

    #[zbus(property)]
    fn supported_uri_schemes(&self) -> Vec<String> {
        // OpenUri is unsupported; these properties describe that method, not
        // formats the internal playback engine can decode.
        Vec::new()
    }

    #[zbus(property)]
    fn supported_mime_types(&self) -> Vec<String> {
        Vec::new()
    }
}

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
        let mut tasks = self.command_tasks.lock().unwrap_or_else(|e| e.into_inner());
        tasks.retain(|task| !task.is_finished());
        tasks.push(tokio::spawn(future));
    }

    async fn cancel_commands(&self) {
        let tasks =
            std::mem::take(&mut *self.command_tasks.lock().unwrap_or_else(|e| e.into_inner()));
        for task in &tasks {
            task.abort();
        }
        for task in tasks {
            let _ = task.await;
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

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl MprisPlayer {
    // --- Methods ---

    /// Skips to the next track in the tracklist.
    async fn next(&self) -> zbus::fdo::Result<()> {
        let ps = self.playback_service.clone();
        self.spawn_command(async move {
            let _ = ps.next().await;
        });
        Ok(())
    }

    /// Skips to the previous track in the tracklist.
    async fn previous(&self) -> zbus::fdo::Result<()> {
        let ps = self.playback_service.clone();
        self.spawn_command(async move {
            let _ = ps.previous().await;
        });
        Ok(())
    }

    /// Pauses playback.
    async fn pause(&self) -> zbus::fdo::Result<()> {
        if self.playback_service.get_playback_state().await.is_playing {
            let _ = self.playback_service.pause().await;
        }
        Ok(())
    }

    /// Pauses if playing, otherwise plays.
    async fn play_pause(&self) -> zbus::fdo::Result<()> {
        let state = self.playback_service.get_playback_state().await;
        if state.is_playing {
            let _ = self.playback_service.pause().await;
        } else if state.is_paused {
            let _ = self.playback_service.resume().await;
        } else if state.queue_length > 0 {
            let idx = state.current_queue_index.unwrap_or(0);
            let ps = self.playback_service.clone();
            self.spawn_command(async move {
                let _ = ps.play_queue_index(idx).await;
            });
        }
        Ok(())
    }

    /// Stops playback.
    async fn stop(&self) -> zbus::fdo::Result<()> {
        let _ = self.playback_service.stop().await;
        Ok(())
    }

    /// Starts or resumes playback.
    async fn play(&self) -> zbus::fdo::Result<()> {
        let state = self.playback_service.get_playback_state().await;
        if state.is_paused {
            let _ = self.playback_service.resume().await;
        } else if !state.is_playing && state.queue_length > 0 {
            let idx = state.current_queue_index.unwrap_or(0);
            let ps = self.playback_service.clone();
            self.spawn_command(async move {
                let _ = ps.play_queue_index(idx).await;
            });
        }
        Ok(())
    }

    /// Seeks forward/backward in the current track by offset in microseconds.
    async fn seek(&self, offset: i64) -> zbus::fdo::Result<()> {
        let state = self.playback_service.get_playback_state().await;
        if !can_seek(&state) {
            return Ok(());
        }
        let offset_secs = offset as f64 / 1_000_000.0;
        let target = state.position_secs + offset_secs;
        if target > state.duration_secs {
            let _ = self.playback_service.next().await;
        } else {
            let clamped = target.max(0.0);
            let _ = self.playback_service.seek(clamped).await;
        }
        Ok(())
    }

    /// Sets the current track position in microseconds.
    /// If TrackId does not match the current track, the call is ignored per spec.
    async fn set_position(&self, track_id: ObjectPath<'_>, position: i64) -> zbus::fdo::Result<()> {
        if position < 0 {
            return Ok(());
        }
        let state = self.playback_service.get_playback_state().await;
        if !can_seek(&state) {
            return Ok(());
        }
        let Some(current_id) = state.current_track_id.as_deref() else {
            return Ok(());
        };
        let current_path = track_id_to_object_path(current_id);

        if track_id.as_str() != current_path.as_str() {
            debug!(
                mpris_track_id = track_id.as_str(),
                expected = current_path.as_str(),
                "SetPosition ignored: TrackId does not match current track"
            );
            return Ok(());
        }

        let target_secs = position as f64 / 1_000_000.0;
        if state.duration_secs > 0.0 && target_secs > state.duration_secs {
            return Ok(());
        }

        let _ = self.playback_service.seek(target_secs).await;
        Ok(())
    }

    async fn open_uri(&self, _uri: &str) -> zbus::fdo::Result<()> {
        Err(zbus::fdo::Error::NotSupported(
            "OpenUri is not supported".into(),
        ))
    }

    // --- Properties ---

    #[zbus(property)]
    async fn playback_status(&self) -> String {
        let state = self.playback_service.get_playback_state().await;
        playback_status_from_state(state.is_playing, state.is_paused, state.is_buffering)
            .to_string()
    }

    #[zbus(property)]
    async fn loop_status(&self) -> String {
        let state = self.playback_service.get_playback_state().await;
        repeat_mode_to_loop_status(state.repeat_mode).to_string()
    }

    #[zbus(property)]
    async fn set_loop_status(&self, loop_status: String) -> zbus::Result<()> {
        let mode = loop_status_to_repeat_mode(&loop_status);
        self.playback_service.set_repeat_mode(mode).await;
        Ok(())
    }

    #[zbus(property)]
    async fn shuffle(&self) -> bool {
        let state = self.playback_service.get_playback_state().await;
        state.is_shuffle
    }

    #[zbus(property)]
    async fn set_shuffle(&self, shuffle: bool) -> zbus::Result<()> {
        self.playback_service.set_shuffle(shuffle).await;
        Ok(())
    }

    #[zbus(property)]
    async fn metadata(&self) -> HashMap<String, OwnedValue> {
        self.cached_metadata.read().await.clone()
    }

    #[zbus(property)]
    async fn volume(&self) -> f64 {
        let state = self.playback_service.get_playback_state().await;
        state.volume as f64
    }

    #[zbus(property)]
    async fn set_volume(&self, volume: f64) -> zbus::Result<()> {
        let clamped = volume.max(0.0) as f32;
        let _ = self.playback_service.set_volume(clamped).await;
        Ok(())
    }

    #[zbus(property(emits_changed_signal = "false"))]
    async fn position(&self) -> i64 {
        let state = self.playback_service.get_playback_state().await;
        seconds_to_microseconds(state.position_secs)
    }

    #[zbus(property)]
    fn rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    async fn set_rate(&self, rate: f64) -> zbus::Result<()> {
        if rate == 0.0 && self.playback_service.get_playback_state().await.is_playing {
            let _ = self.playback_service.pause().await;
        }
        Ok(())
    }

    #[zbus(property)]
    fn minimum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn maximum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property(emits_changed_signal = "const"))]
    fn can_control(&self) -> bool {
        true
    }

    #[zbus(property)]
    async fn can_play(&self) -> bool {
        let state = self.playback_service.get_playback_state().await;
        state.queue_length > 0 || state.current_track_id.is_some()
    }

    #[zbus(property)]
    async fn can_pause(&self) -> bool {
        let state = self.playback_service.get_playback_state().await;
        state.current_track_id.is_some() && (state.is_playing || state.is_paused)
    }

    #[zbus(property)]
    async fn can_seek(&self) -> bool {
        can_seek(&self.playback_service.get_playback_state().await)
    }

    #[zbus(property)]
    async fn can_go_next(&self) -> bool {
        let state = self.playback_service.get_playback_state().await;
        // The backend can refill both Flow and manual queues at their end.
        state.queue_length > 0
    }

    #[zbus(property)]
    async fn can_go_previous(&self) -> bool {
        let state = self.playback_service.get_playback_state().await;
        state.queue_length > 0 && state.current_queue_index.is_some()
    }

    // --- Signals ---

    #[zbus(signal)]
    pub async fn seeked(emitter: &SignalEmitter<'_>, position: i64) -> zbus::Result<()>;
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
            .lock()
            .unwrap_or_else(|e| e.into_inner())
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

/// Fetches enriched track info from database when a track starts playing.
async fn fetch_track_info(
    processor: &Arc<CoreProcessor>,
    track_id: &str,
    title: &str,
    artist: &str,
    duration_secs: f64,
) -> MprisTrackInfo {
    let mut info = MprisTrackInfo {
        track_id: track_id.to_string(),
        title: title.to_string(),
        artist: artist.to_string(),
        album: None,
        album_artist: None,
        duration_secs,
        cover_art: None,
        track_number: None,
        url: None,
    };

    // Query tracks table
    let track_row: Option<(
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<String>,
        f64,
    )> = sqlx::query_as(
        "SELECT 
            t.title,
            a.name AS artist_name,
            al.title AS album_title,
            al.cover_art_path,
            ext.cover_art_url,
            t.track_number,
            t.file_path,
            t.duration_secs
         FROM tracks t
         LEFT JOIN artists a ON a.id = t.artist_id
         LEFT JOIN albums al ON al.id = t.album_id
         LEFT JOIN external_tracks ext ON ext.id = t.id
         WHERE t.id = ?
         LIMIT 1",
    )
    .bind(track_id)
    .fetch_optional(processor.db_pool())
    .await
    .ok()
    .flatten();

    if let Some((
        t_title,
        a_name,
        album_title,
        cover_art_path,
        cover_art_url,
        track_number,
        file_path,
        d_secs,
    )) = track_row
    {
        if info.title.is_empty() {
            info.title = t_title;
        }
        if info.artist.is_empty() {
            info.artist = a_name.unwrap_or_else(|| "Unknown Artist".to_string());
        }
        if info.duration_secs <= 0.0 {
            info.duration_secs = d_secs;
        }
        info.album = album_title;
        info.cover_art = cover_art_path.or(cover_art_url);
        info.track_number = track_number.map(|n| n as i32);
        info.url = file_path;
        return info;
    }

    // Fallback: query external_tracks table
    let ext_row: Option<(
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<f64>,
    )> = sqlx::query_as(
        "SELECT title, artist, album, cover_art_url, duration_secs FROM external_tracks WHERE id = ? LIMIT 1",
    )
    .bind(track_id)
    .fetch_optional(processor.db_pool())
    .await
    .ok()
    .flatten();

    if let Some((e_title, e_artist, album, cover_art_url, e_dur)) = ext_row {
        if info.title.is_empty() {
            if let Some(t) = e_title {
                info.title = t;
            }
        }
        if info.artist.is_empty() {
            if let Some(a) = e_artist {
                info.artist = a;
            }
        }
        if info.duration_secs <= 0.0 {
            if let Some(d) = e_dur {
                info.duration_secs = d;
            }
        }
        info.album = album;
        info.cover_art = cover_art_url;
    }

    info
}

fn can_seek(state: &PlaybackStateDto) -> bool {
    state.can_seek
        && state.current_track_id.is_some()
        && state.duration_secs > 0.0
        && (state.is_playing || state.is_paused)
}

/// A stopped queue still has a real selection that Play will start. History and
/// arbitrary library entries are not current tracks and must not become metadata.
fn selected_track_id(state: &PlaybackStateDto) -> Option<&str> {
    state.current_track_id.as_deref().or_else(|| {
        state
            .queue_track_ids
            .get(state.current_queue_index.unwrap_or(0))
            .map(String::as_str)
    })
}

impl MprisPlayer {
    async fn refresh_metadata(&self, processor: &Arc<CoreProcessor>) {
        // Re-read after database I/O: an older event must not restore metadata for
        // a track superseded by a rapid skip or a queue clear during startup.
        loop {
            let state = self.playback_service.get_playback_state().await;
            let id = selected_track_id(&state);
            let old = self.current_track.read().await.clone();
            let info = match id {
                Some(id) if old.as_ref().is_some_and(|info| info.track_id == id) => {
                    old.map(|mut info| {
                        if state.duration_secs > 0.0 {
                            info.duration_secs = state.duration_secs;
                        }
                        info
                    })
                }
                Some(id) => {
                    Some(fetch_track_info(processor, id, "", "", state.duration_secs).await)
                }
                None => None,
            };
            let latest = self.playback_service.get_playback_state().await;
            if selected_track_id(&latest) != id {
                continue;
            }
            *self.cached_metadata.write().await = build_mpris_metadata(info.as_ref());
            *self.current_track.write().await = info;
            break;
        }
    }

    async fn changed_properties(
        &self,
        previous: &mut HashMap<String, OwnedValue>,
        emitter: &SignalEmitter<'_>,
    ) -> zbus::Result<()> {
        let state = self.playback_service.get_playback_state().await;
        let mut snapshot = HashMap::<String, OwnedValue>::new();
        macro_rules! property {
            ($name:literal, $value:expr) => {
                snapshot.insert(
                    $name.into(),
                    Value::from($value)
                        .try_into()
                        .expect("MPRIS property value"),
                );
            };
        }
        property!(
            "PlaybackStatus",
            playback_status_from_state(state.is_playing, state.is_paused, state.is_buffering)
        );
        property!("Metadata", self.metadata().await);
        property!(
            "CanPlay",
            state.queue_length > 0 || state.current_track_id.is_some()
        );
        property!(
            "CanPause",
            state.current_track_id.is_some() && (state.is_playing || state.is_paused)
        );
        property!("CanSeek", can_seek(&state));
        property!("CanGoNext", state.queue_length > 0);
        property!(
            "CanGoPrevious",
            state.queue_length > 0 && state.current_queue_index.is_some()
        );
        property!("Volume", state.volume as f64);
        property!("Shuffle", state.is_shuffle);
        property!("LoopStatus", repeat_mode_to_loop_status(state.repeat_mode));
        let changed = snapshot
            .iter()
            .filter(|(name, value)| previous.get(*name) != Some(*value))
            .map(|(name, value)| (name.as_str(), Value::from(value.clone())))
            .collect::<HashMap<_, _>>();
        if !changed.is_empty() {
            zbus::fdo::Properties::properties_changed(
                emitter,
                "org.mpris.MediaPlayer2.Player".try_into()?,
                changed,
                std::borrow::Cow::Borrowed(&[]),
            )
            .await?;
            *previous = snapshot;
        }
        Ok(())
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
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::database::create_in_memory_pool;
    use crate::playback::backend::MockAudioBackend;
    use std::time::Duration;

    async fn fixture() -> (Arc<CoreProcessor>, MprisPlayer, MockAudioBackend) {
        let pool = create_in_memory_pool().await.unwrap();
        for id in ["one", "two"] {
            sqlx::query("INSERT INTO tracks (id, file_path, file_size, modified_timestamp, title, normalized_title, duration_secs, format, created_at, updated_at) VALUES (?, ?, 1, 1, ?, ?, 180.0, 'wav', 1, 1)")
                .bind(id).bind(format!("/test/{id}.wav")).bind(id).bind(id)
                .execute(&pool).await.unwrap();
        }
        let backend = MockAudioBackend::new();
        let processor = Arc::new(CoreProcessor::new_with_backend(
            pool,
            AppConfig::default_with_dirs(),
            Box::new(backend.clone()),
        ));
        let player = MprisPlayer::new(
            processor.playback_service(),
            Arc::new(RwLock::new(None)),
            Arc::new(RwLock::new(build_mpris_metadata(None))),
        );
        (processor, player, backend)
    }

    #[tokio::test]
    async fn mpris_metadata_and_capabilities_follow_real_selection() {
        let (processor, player, _) = fixture().await;
        player.refresh_metadata(&processor).await;
        assert_eq!(player.playback_status().await, "Stopped");
        assert!(!player.can_play().await);
        assert!(!player.can_pause().await);
        assert!(!player.can_seek().await);
        assert!(!player.can_go_next().await);
        assert!(!player.metadata().await.contains_key("xesam:title"));
        // Merely having a populated library must not select an arbitrary song.
        player.play().await.unwrap();
        assert_eq!(player.playback_status().await, "Stopped");
        player.pause().await.unwrap();
        assert_eq!(player.playback_status().await, "Stopped");

        processor
            .playback_service()
            .play_track("one", None)
            .await
            .unwrap();
        player.refresh_metadata(&processor).await;
        let metadata = player.metadata().await;
        assert_eq!(player.playback_status().await, "Playing");
        assert!(player.can_play().await && player.can_pause().await && player.can_seek().await);
        assert!(player.can_go_next().await); // backend can refill the queue
        player.pause().await.unwrap();
        assert_eq!(player.playback_status().await, "Paused");
        assert!(player.can_play().await && player.can_pause().await && player.can_seek().await);
        assert_eq!(metadata, player.metadata().await);
        player.play().await.unwrap();
        assert_eq!(player.playback_status().await, "Playing");
        player.stop().await.unwrap();
        player.refresh_metadata(&processor).await;
        assert_eq!(player.playback_status().await, "Stopped");
        assert!(player.can_play().await);
        assert!(!player.can_pause().await && !player.can_seek().await);
        assert_eq!(metadata, player.metadata().await);
        processor.playback_service().clear_queue().await;
        player.refresh_metadata(&processor).await;
        assert!(!player.can_play().await);
        assert!(!player.metadata().await.contains_key("xesam:title"));
        player.play_pause().await.unwrap();
        assert_eq!(player.playback_status().await, "Stopped");
    }

    #[tokio::test]
    async fn mpris_seeking_and_navigation_use_backend_identity() {
        let (processor, player, _) = fixture().await;
        let ps = processor.playback_service();
        ps.play_track("one", None).await.unwrap();
        ps.enqueue_track("two", false).await.unwrap();
        player.refresh_metadata(&processor).await;
        player.seek(10_000_000).await.unwrap();
        assert_eq!(player.position().await, 10_000_000);
        player
            .set_position(
                ObjectPath::try_from(track_id_to_object_path("two")).unwrap(),
                20_000_000,
            )
            .await
            .unwrap();
        assert_eq!(player.position().await, 10_000_000);
        player
            .set_position(
                ObjectPath::try_from(track_id_to_object_path("one")).unwrap(),
                20_000_000,
            )
            .await
            .unwrap();
        assert_eq!(player.position().await, 20_000_000);
        player
            .set_position(
                ObjectPath::try_from(track_id_to_object_path("one")).unwrap(),
                -1,
            )
            .await
            .unwrap();
        assert_eq!(player.position().await, 20_000_000);
        let mut events = processor.event_bus().subscribe();
        player.next().await.unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if matches!(events.recv().await.unwrap(), Event::PlaybackStarted { track_id, .. } if track_id == "two") { break; }
            }
        }).await.unwrap();
        // Metadata may still be catching up; old TrackId must never seek the new track.
        player
            .set_position(
                ObjectPath::try_from(track_id_to_object_path("one")).unwrap(),
                30_000_000,
            )
            .await
            .unwrap();
        assert_eq!(player.position().await, 0);
        player.previous().await.unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                if matches!(events.recv().await.unwrap(), Event::PlaybackStarted { track_id, .. } if track_id == "one") { break; }
            }
        }).await.unwrap();
        player.stop().await.unwrap();
        player.seek(1_000_000).await.unwrap();
        assert_eq!(player.position().await, 0);
    }

    #[tokio::test]
    #[ignore = "requires a private dbus-run-session; run scripts/test-mpris-session.sh"]
    async fn mpris_private_bus_lifecycle_and_signals() {
        use zbus::export::futures_core::Stream;
        assert_eq!(
            std::env::var("KAZE_PRIVATE_MPRIS_TEST").as_deref(),
            Ok("1"),
            "use scripts/test-mpris-session.sh to isolate this test from desktop players"
        );
        let (processor, _, backend) = fixture().await;
        let service = start_mpris_service(processor.clone(), None).await.unwrap();
        let conn = zbus::Connection::session().await.unwrap();
        let proxy = zbus::proxy::Builder::<zbus::Proxy<'_>>::new(&conn)
            .destination("org.mpris.MediaPlayer2.kaze")
            .unwrap()
            .path("/org/mpris/MediaPlayer2")
            .unwrap()
            .interface("org.mpris.MediaPlayer2.Player")
            .unwrap()
            .cache_properties(zbus::proxy::CacheProperties::No)
            .build()
            .await
            .unwrap();
        let root = zbus::Proxy::new(
            &conn,
            "org.mpris.MediaPlayer2.kaze",
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2",
        )
        .await
        .unwrap();
        assert_eq!(
            root.get_property::<String>("Identity").await.unwrap(),
            "Kaze"
        );
        assert_eq!(
            root.get_property::<String>("DesktopEntry").await.unwrap(),
            "kaze"
        );
        assert!(proxy.get_property::<bool>("CanControl").await.unwrap());
        assert_eq!(
            proxy
                .get_property::<String>("PlaybackStatus")
                .await
                .unwrap(),
            "Stopped"
        );
        assert!(start_mpris_service(processor.clone(), None).await.is_err());
        let rule = zbus::MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface("org.freedesktop.DBus.Properties")
            .unwrap()
            .member("PropertiesChanged")
            .unwrap()
            .path("/org/mpris/MediaPlayer2")
            .unwrap()
            .build();
        let mut signals = zbus::MessageStream::for_match_rule(rule, &conn, Some(128))
            .await
            .unwrap();
        processor
            .playback_service()
            .play_track("one", None)
            .await
            .unwrap();
        let signal = tokio::time::timeout(
            Duration::from_secs(2),
            std::future::poll_fn(|cx| std::pin::Pin::new(&mut signals).poll_next(cx)),
        )
        .await
        .unwrap()
        .unwrap()
        .unwrap();
        let (_, changed, _): (String, HashMap<String, OwnedValue>, Vec<String>) =
            signal.body().deserialize().unwrap();
        assert!(!changed.contains_key("Position"));
        proxy.call::<_, _, ()>("Pause", &()).await.unwrap();
        assert_eq!(
            proxy
                .get_property::<String>("PlaybackStatus")
                .await
                .unwrap(),
            "Paused"
        );
        assert!(proxy.get_property::<bool>("CanPause").await.unwrap());
        processor
            .playback_service()
            .set_repeat_mode(crate::core::command::RepeatMode::One)
            .await;
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let message =
                    std::future::poll_fn(|cx| std::pin::Pin::new(&mut signals).poll_next(cx))
                        .await
                        .unwrap()
                        .unwrap();
                let (_, changed, _): (String, HashMap<String, OwnedValue>, Vec<String>) =
                    message.body().deserialize().unwrap();
                if changed
                    .get("LoopStatus")
                    .and_then(|v| <&str>::try_from(v).ok())
                    == Some("Track")
                {
                    break;
                }
            }
        })
        .await
        .unwrap();
        proxy.call::<_, _, ()>("Play", &()).await.unwrap();
        assert_eq!(
            proxy
                .get_property::<String>("PlaybackStatus")
                .await
                .unwrap(),
            "Playing"
        );
        backend.advance_position(Duration::from_secs(4));
        assert_eq!(
            proxy.get_property::<i64>("Position").await.unwrap(),
            4_000_000
        );
        let mut seeks = proxy.receive_signal("Seeked").await.unwrap();
        proxy
            .call::<_, _, ()>("Seek", &(1_000_000_i64,))
            .await
            .unwrap();
        let seeked = tokio::time::timeout(
            Duration::from_secs(2),
            std::future::poll_fn(|cx| std::pin::Pin::new(&mut seeks).poll_next(cx)),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(seeked.body().deserialize::<i64>().unwrap(), 5_000_000);
        // Let all real transitions drain, then prove ticks do not generate activity.
        tokio::time::sleep(Duration::from_millis(100)).await;
        while tokio::time::timeout(
            Duration::from_millis(30),
            std::future::poll_fn(|cx| std::pin::Pin::new(&mut signals).poll_next(cx)),
        )
        .await
        .is_ok()
        {}
        for _ in 0..20 {
            processor
                .event_bus()
                .publish(Event::PlaybackPositionChanged {
                    session_id: "test".into(),
                    position_secs: 4.0,
                    duration_secs: 180.0,
                })
                .unwrap();
        }
        assert!(tokio::time::timeout(
            Duration::from_millis(350),
            std::future::poll_fn(|cx| std::pin::Pin::new(&mut signals).poll_next(cx))
        )
        .await
        .is_err());
        proxy.call::<_, _, ()>("Stop", &()).await.unwrap();
        assert_eq!(
            proxy
                .get_property::<String>("PlaybackStatus")
                .await
                .unwrap(),
            "Stopped"
        );
        assert!(!proxy.get_property::<bool>("CanSeek").await.unwrap());
        tokio::join!(service.shutdown(), service.shutdown());
        service.shutdown().await;
        let bus = zbus::fdo::DBusProxy::new(&conn).await.unwrap();
        assert!(!bus
            .name_has_owner("org.mpris.MediaPlayer2.kaze".try_into().unwrap())
            .await
            .unwrap());
        let relaunched = start_mpris_service(processor.clone(), None).await.unwrap();
        drop(relaunched);
        tokio::time::timeout(Duration::from_secs(2), async {
            while bus
                .name_has_owner("org.mpris.MediaPlayer2.kaze".try_into().unwrap())
                .await
                .unwrap()
            {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        // Race registration with a real playback transition; final metadata catches up.
        let ps = processor.playback_service();
        let (service, played) = tokio::join!(
            start_mpris_service(processor.clone(), None),
            ps.play_track("two", None)
        );
        played.unwrap();
        let service = service.unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let metadata = proxy
                    .get_property::<HashMap<String, OwnedValue>>("Metadata")
                    .await
                    .unwrap();
                if metadata
                    .get("xesam:title")
                    .and_then(|v| <&str>::try_from(v).ok())
                    == Some("two")
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        service.shutdown().await;
    }
}
