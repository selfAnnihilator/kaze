//! The org.mpris.MediaPlayer2.Player interface, metadata refresh and property-change signals.
use super::*;

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl MprisPlayer {
    // --- Methods ---

    /// Skips to the next track in the tracklist.
    pub(super) async fn next(&self) -> zbus::fdo::Result<()> {
        let ps = self.playback_service.clone();
        self.spawn_command(async move {
            let _ = ps.next().await;
        });
        Ok(())
    }

    /// Skips to the previous track in the tracklist.
    pub(super) async fn previous(&self) -> zbus::fdo::Result<()> {
        let ps = self.playback_service.clone();
        self.spawn_command(async move {
            let _ = ps.previous().await;
        });
        Ok(())
    }

    /// Pauses playback.
    pub(super) async fn pause(&self) -> zbus::fdo::Result<()> {
        if self.playback_service.get_playback_state().await.is_playing {
            let _ = self.playback_service.pause().await;
        }
        Ok(())
    }

    /// Pauses if playing, otherwise plays.
    pub(super) async fn play_pause(&self) -> zbus::fdo::Result<()> {
        let state = self.playback_service.get_playback_state().await;
        if state.is_playing {
            let _ = self.playback_service.pause().await;
        } else if state.is_paused {
            self.resume_selected().await;
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
    pub(super) async fn stop(&self) -> zbus::fdo::Result<()> {
        let _ = self.playback_service.stop().await;
        Ok(())
    }

    /// Starts or resumes playback.
    pub(super) async fn play(&self) -> zbus::fdo::Result<()> {
        let state = self.playback_service.get_playback_state().await;
        if state.is_paused {
            self.resume_selected().await;
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
    pub(super) async fn seek(&self, offset: i64) -> zbus::fdo::Result<()> {
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
    pub(super) async fn set_position(&self, track_id: ObjectPath<'_>, position: i64) -> zbus::fdo::Result<()> {
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

    pub(super) async fn open_uri(&self, _uri: &str) -> zbus::fdo::Result<()> {
        Err(zbus::fdo::Error::NotSupported(
            "OpenUri is not supported".into(),
        ))
    }

    // --- Properties ---

    #[zbus(property)]
    pub(super) async fn playback_status(&self) -> String {
        let state = self.playback_service.get_playback_state().await;
        playback_status_from_state(state.is_playing, state.is_paused, state.is_buffering)
            .to_string()
    }

    #[zbus(property)]
    pub(super) async fn loop_status(&self) -> String {
        let state = self.playback_service.get_playback_state().await;
        repeat_mode_to_loop_status(state.repeat_mode).to_string()
    }

    #[zbus(property)]
    pub(super) async fn set_loop_status(&self, loop_status: String) -> zbus::Result<()> {
        let mode = loop_status_to_repeat_mode(&loop_status);
        self.playback_service.set_repeat_mode(mode).await;
        Ok(())
    }

    #[zbus(property)]
    pub(super) async fn shuffle(&self) -> bool {
        let state = self.playback_service.get_playback_state().await;
        state.is_shuffle
    }

    #[zbus(property)]
    pub(super) async fn set_shuffle(&self, shuffle: bool) -> zbus::Result<()> {
        self.playback_service.set_shuffle(shuffle).await;
        Ok(())
    }

    #[zbus(property)]
    pub(super) async fn metadata(&self) -> HashMap<String, OwnedValue> {
        self.cached_metadata.read().await.clone()
    }

    #[zbus(property)]
    pub(super) async fn volume(&self) -> f64 {
        let state = self.playback_service.get_playback_state().await;
        state.volume as f64
    }

    #[zbus(property)]
    pub(super) async fn set_volume(&self, volume: f64) -> zbus::Result<()> {
        let clamped = volume.max(0.0) as f32;
        let _ = self.playback_service.set_volume(clamped).await;
        Ok(())
    }

    #[zbus(property(emits_changed_signal = "false"))]
    pub(super) async fn position(&self) -> i64 {
        let state = self.playback_service.get_playback_state().await;
        seconds_to_microseconds(state.position_secs)
    }

    #[zbus(property)]
    pub(super) fn rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    pub(super) async fn set_rate(&self, rate: f64) -> zbus::Result<()> {
        if rate == 0.0 && self.playback_service.get_playback_state().await.is_playing {
            let _ = self.playback_service.pause().await;
        }
        Ok(())
    }

    #[zbus(property)]
    pub(super) fn minimum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    pub(super) fn maximum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property(emits_changed_signal = "const"))]
    pub(super) fn can_control(&self) -> bool {
        true
    }

    #[zbus(property)]
    pub(super) async fn can_play(&self) -> bool {
        let state = self.playback_service.get_playback_state().await;
        state.queue_length > 0 || state.current_track_id.is_some()
    }

    #[zbus(property)]
    pub(super) async fn can_pause(&self) -> bool {
        let state = self.playback_service.get_playback_state().await;
        state.current_track_id.is_some() && (state.is_playing || state.is_paused)
    }

    #[zbus(property)]
    pub(super) async fn can_seek(&self) -> bool {
        can_seek(&self.playback_service.get_playback_state().await)
    }

    #[zbus(property)]
    pub(super) async fn can_go_next(&self) -> bool {
        let state = self.playback_service.get_playback_state().await;
        // The backend can refill both Flow and manual queues at their end.
        state.queue_length > 0
    }

    #[zbus(property)]
    pub(super) async fn can_go_previous(&self) -> bool {
        let state = self.playback_service.get_playback_state().await;
        state.queue_length > 0 && state.current_queue_index.is_some()
    }

    // --- Signals ---

    #[zbus(signal)]
    pub async fn seeked(emitter: &SignalEmitter<'_>, position: i64) -> zbus::Result<()>;
}

impl MprisPlayer {
    pub(super) async fn refresh_metadata(&self, processor: &Arc<CoreProcessor>) {
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

    pub(super) async fn changed_properties(
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
