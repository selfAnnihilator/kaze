use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};
use zbus::connection::Builder;
use zbus::interface;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{ObjectPath, OwnedValue};

use crate::core::command::RepeatMode;
use crate::core::error::{AppError, AppResult};
use crate::core::event::Event;
use crate::core::processor::CoreProcessor;
use crate::playback::service::PlaybackService;

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
        vec!["file".to_string(), "http".to_string(), "https".to_string()]
    }

    #[zbus(property)]
    fn supported_mime_types(&self) -> Vec<String> {
        vec![
            "audio/mpeg".to_string(),
            "audio/flac".to_string(),
            "audio/ogg".to_string(),
            "audio/wav".to_string(),
            "audio/aac".to_string(),
            "audio/m4a".to_string(),
            "audio/x-m4a".to_string(),
        ]
    }
}

/// Player interface implementation: org.mpris.MediaPlayer2.Player
pub struct MprisPlayer {
    playback_service: Arc<PlaybackService>,
    current_track: Arc<RwLock<Option<MprisTrackInfo>>>,
    cached_metadata: Arc<RwLock<HashMap<String, OwnedValue>>>,
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
        }
    }
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl MprisPlayer {
    // --- Methods ---

    /// Skips to the next track in the tracklist.
    async fn next(&self) -> zbus::fdo::Result<()> {
        let ps = self.playback_service.clone();
        tokio::spawn(async move {
            let _ = ps.next().await;
        });
        Ok(())
    }

    /// Skips to the previous track in the tracklist.
    async fn previous(&self) -> zbus::fdo::Result<()> {
        let ps = self.playback_service.clone();
        tokio::spawn(async move {
            let _ = ps.previous().await;
        });
        Ok(())
    }

    /// Pauses playback.
    async fn pause(&self) -> zbus::fdo::Result<()> {
        let _ = self.playback_service.pause().await;
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
            tokio::spawn(async move {
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
            tokio::spawn(async move {
                let _ = ps.play_queue_index(idx).await;
            });
        }
        Ok(())
    }

    /// Seeks forward/backward in the current track by offset in microseconds.
    async fn seek(&self, offset: i64) -> zbus::fdo::Result<()> {
        let state = self.playback_service.get_playback_state().await;
        if !state.can_seek || state.duration_secs <= 0.0 {
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
        let current_path = {
            let guard = self.current_track.read().await;
            match &*guard {
                Some(info) => track_id_to_object_path(&info.track_id),
                None => "/org/mpris/MediaPlayer2/TrackList/NoTrack".to_string(),
            }
        };

        if track_id.as_str() != current_path.as_str() {
            debug!(
                mpris_track_id = track_id.as_str(),
                expected = current_path.as_str(),
                "SetPosition ignored: TrackId does not match current track"
            );
            return Ok(());
        }

        let state = self.playback_service.get_playback_state().await;
        let target_secs = position as f64 / 1_000_000.0;
        if state.duration_secs > 0.0 && target_secs > state.duration_secs {
            return Ok(());
        }

        let _ = self.playback_service.seek(target_secs).await;
        Ok(())
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
    async fn set_loop_status(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        loop_status: String,
    ) -> zbus::Result<()> {
        let mode = loop_status_to_repeat_mode(&loop_status);
        self.playback_service.set_repeat_mode(mode).await;
        self.loop_status_changed(&emitter).await
    }

    #[zbus(property)]
    async fn shuffle(&self) -> bool {
        let state = self.playback_service.get_playback_state().await;
        state.is_shuffle
    }

    #[zbus(property)]
    async fn set_shuffle(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        shuffle: bool,
    ) -> zbus::Result<()> {
        self.playback_service.set_shuffle(shuffle).await;
        self.shuffle_changed(&emitter).await
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
    async fn set_volume(
        &self,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        volume: f64,
    ) -> zbus::Result<()> {
        let clamped = volume.max(0.0) as f32;
        let _ = self.playback_service.set_volume(clamped).await;
        self.volume_changed(&emitter).await
    }

    #[zbus(property)]
    async fn position(&self) -> i64 {
        let state = self.playback_service.get_playback_state().await;
        seconds_to_microseconds(state.position_secs)
    }

    #[zbus(property)]
    fn rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    async fn set_rate(&self, _rate: f64) -> zbus::Result<()> {
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

    #[zbus(property)]
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
        state.is_playing
    }

    #[zbus(property)]
    async fn can_seek(&self) -> bool {
        let state = self.playback_service.get_playback_state().await;
        state.can_seek
    }

    #[zbus(property)]
    async fn can_go_next(&self) -> bool {
        let state = self.playback_service.get_playback_state().await;
        state.queue_length > 1 || (state.queue_length > 0 && state.repeat_mode != RepeatMode::Off)
    }

    #[zbus(property)]
    async fn can_go_previous(&self) -> bool {
        let state = self.playback_service.get_playback_state().await;
        state.queue_length > 1
            || state.current_track_id.is_some()
            || (state.queue_length > 0 && state.repeat_mode != RepeatMode::Off)
    }

    // --- Signals ---

    #[zbus(signal)]
    pub async fn seeked(emitter: &SignalEmitter<'_>, position: i64) -> zbus::Result<()>;
}

/// Handle to the running MPRIS service task.
pub struct MprisServiceHandle {
    pub connection: zbus::Connection,
    pub task_handle: tokio::task::JoinHandle<()>,
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

/// Starts the native Linux MPRIS D-Bus service for Kaze.
pub async fn start_mpris_service(
    processor: Arc<CoreProcessor>,
    app_handle: Option<tauri::AppHandle>,
) -> AppResult<MprisServiceHandle> {
    info!("Initializing Linux MPRIS D-Bus service: org.mpris.MediaPlayer2.kaze");

    let current_track = Arc::new(RwLock::new(None));
    let cached_metadata = Arc::new(RwLock::new(build_mpris_metadata(None)));

    // Pre-populate if already playing
    let initial_state = processor.playback_service().get_playback_state().await;
    if let Some(ref track_id) = initial_state.current_track_id {
        let info =
            fetch_track_info(&processor, track_id, "", "", initial_state.duration_secs).await;
        *cached_metadata.write().await = build_mpris_metadata(Some(&info));
        *current_track.write().await = Some(info);
    }

    let root = MprisRoot::new(app_handle);
    let player = MprisPlayer::new(
        processor.playback_service(),
        current_track.clone(),
        cached_metadata.clone(),
    );

    let conn = Builder::session()
        .map_err(|e| AppError::Playback(format!("Failed to connect to D-Bus session bus: {}", e)))?
        .name("org.mpris.MediaPlayer2.kaze")
        .map_err(|e| AppError::Playback(format!("Failed to request D-Bus name: {}", e)))?
        .serve_at("/org/mpris/MediaPlayer2", root)
        .map_err(|e| AppError::Playback(format!("Failed to register MPRIS root interface: {}", e)))?
        .serve_at("/org/mpris/MediaPlayer2", player)
        .map_err(|e| {
            AppError::Playback(format!("Failed to register MPRIS player interface: {}", e))
        })?
        .build()
        .await
        .map_err(|e| AppError::Playback(format!("Failed to build D-Bus connection: {}", e)))?;

    info!("Linux MPRIS D-Bus service registered successfully");

    let mut event_rx = processor.event_bus().subscribe();
    let conn_for_loop = conn.clone();
    let processor_for_loop = processor.clone();

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

        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    let emitter = player_iface.signal_emitter();
                    let player = player_iface.get().await;

                    match event {
                        Event::PlaybackStarted {
                            track_id,
                            title,
                            artist,
                            duration_secs,
                            ..
                        } => {
                            let info = fetch_track_info(
                                &processor_for_loop,
                                &track_id,
                                &title,
                                &artist,
                                duration_secs,
                            )
                            .await;

                            *cached_metadata.write().await = build_mpris_metadata(Some(&info));
                            *current_track.write().await = Some(info);

                            let _ = player.playback_status_changed(&emitter).await;
                            let _ = player.metadata_changed(&emitter).await;
                            let _ = player.can_play_changed(&emitter).await;
                            let _ = player.can_pause_changed(&emitter).await;
                            let _ = player.can_seek_changed(&emitter).await;
                            let _ = player.can_go_next_changed(&emitter).await;
                            let _ = player.can_go_previous_changed(&emitter).await;
                        }
                        Event::PlaybackPaused { .. } => {
                            let _ = player.playback_status_changed(&emitter).await;
                            let _ = player.can_pause_changed(&emitter).await;
                            let _ = player.can_play_changed(&emitter).await;
                        }
                        Event::PlaybackResumed { .. } => {
                            let _ = player.playback_status_changed(&emitter).await;
                            let _ = player.can_pause_changed(&emitter).await;
                            let _ = player.can_play_changed(&emitter).await;
                        }
                        Event::PlaybackStopped { .. } => {
                            *current_track.write().await = None;
                            *cached_metadata.write().await = build_mpris_metadata(None);

                            let _ = player.playback_status_changed(&emitter).await;
                            let _ = player.metadata_changed(&emitter).await;
                            let _ = player.can_pause_changed(&emitter).await;
                            let _ = player.can_play_changed(&emitter).await;
                        }
                        Event::TrackFinished { .. } => {
                            let state = processor_for_loop
                                .playback_service()
                                .get_playback_state()
                                .await;
                            if !state.is_playing && !state.is_paused {
                                *current_track.write().await = None;
                                *cached_metadata.write().await = build_mpris_metadata(None);
                                let _ = player.playback_status_changed(&emitter).await;
                                let _ = player.metadata_changed(&emitter).await;
                            }
                        }
                        Event::PlaybackSeeked { position_secs } => {
                            let micros = seconds_to_microseconds(position_secs);
                            let _ = MprisPlayer::seeked(&emitter, micros).await;
                        }
                        Event::PlaybackVolumeChanged { .. } => {
                            let _ = player.volume_changed(&emitter).await;
                        }
                        Event::QueueUpdated { .. } => {
                            let _ = player.can_go_next_changed(&emitter).await;
                            let _ = player.can_go_previous_changed(&emitter).await;
                            let _ = player.can_play_changed(&emitter).await;
                            let _ = player.shuffle_changed(&emitter).await;
                            let _ = player.loop_status_changed(&emitter).await;
                        }
                        Event::PlaybackBuffering { track_id, .. } => {
                            let info =
                                fetch_track_info(&processor_for_loop, &track_id, "", "", 0.0).await;

                            *cached_metadata.write().await = build_mpris_metadata(Some(&info));
                            *current_track.write().await = Some(info);

                            let _ = player.playback_status_changed(&emitter).await;
                            let _ = player.metadata_changed(&emitter).await;
                            let _ = player.can_pause_changed(&emitter).await;
                            let _ = player.can_play_changed(&emitter).await;
                        }
                        Event::PlaybackError { .. } => {
                            let _ = player.playback_status_changed(&emitter).await;
                            let _ = player.can_pause_changed(&emitter).await;
                            let _ = player.can_play_changed(&emitter).await;
                        }
                        // Explicitly ignore position ticks to avoid flooding D-Bus;
                        // clients query the `Position` property getter on demand.
                        Event::PlaybackPositionChanged { .. } => {}
                        _ => {}
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    debug!("MPRIS event listener lagged by {} events", n);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    info!("MPRIS event listener channel closed; shutting down task");
                    break;
                }
            }
        }
    });

    Ok(MprisServiceHandle {
        connection: conn,
        task_handle,
    })
}
