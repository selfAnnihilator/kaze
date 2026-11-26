//! Pure mapping between backend playback events and the Android media
//! session. Compiled on every target so it is unit-tested on the host; only
//! the Android shell wires it to the kaze-media plugin.
#![cfg_attr(not(target_os = "android"), allow(dead_code))]

use crate::core::command::Command;
use crate::core::event::Event;

/// Playback state mirrored into the media session.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MirrorUpdate {
    pub playing: bool,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub duration_ms: Option<u64>,
    pub position_ms: Option<u64>,
}

fn millis(secs: f64) -> u64 {
    (secs.max(0.0) * 1000.0).round() as u64
}

/// Tracks the current mirror state and folds backend events into it.
#[derive(Default)]
pub struct MediaMirror {
    current: MirrorUpdate,
}

impl MediaMirror {
    /// Returns the new state when the event changes what the session shows.
    /// Position ticks are ignored: the session extrapolates while playing.
    pub fn apply(&mut self, event: &Event) -> Option<MirrorUpdate> {
        match event {
            Event::PlaybackStarted { title, artist, duration_secs, .. } => {
                self.current = MirrorUpdate {
                    playing: true,
                    title: Some(title.clone()),
                    artist: Some(artist.clone()),
                    duration_ms: Some(millis(*duration_secs)),
                    position_ms: Some(0),
                };
            }
            Event::PlaybackPaused { position_secs, .. } => {
                self.current.playing = false;
                self.current.position_ms = Some(millis(*position_secs));
            }
            Event::PlaybackResumed { position_secs, .. } => {
                self.current.playing = true;
                self.current.position_ms = Some(millis(*position_secs));
            }
            Event::PlaybackSeeked { position_secs } => {
                self.current.position_ms = Some(millis(*position_secs));
            }
            Event::PlaybackStopped { .. } => self.current = MirrorUpdate::default(),
            _ => return None,
        }
        Some(self.current.clone())
    }
}

/// Maps a media-session transport command to a backend command.
pub fn command_for(name: &str, position_ms: Option<u64>) -> Option<Command> {
    match name {
        "play" => Some(Command::Resume),
        "pause" => Some(Command::Pause),
        "stop" => Some(Command::Stop),
        "next" => Some(Command::NextTrack),
        "previous" => Some(Command::PreviousTrack),
        "seek" => Some(Command::Seek { position_secs: position_ms? as f64 / 1000.0 }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn started() -> Event {
        Event::PlaybackStarted {
            session_id: "s".into(),
            track_id: "t".into(),
            title: "Song".into(),
            artist: "Artist".into(),
            duration_secs: 12.5,
            source: "library".into(),
        }
    }

    #[test]
    fn started_sets_metadata_and_playing() {
        let update = MediaMirror::default().apply(&started()).unwrap();
        assert!(update.playing);
        assert_eq!(update.title.as_deref(), Some("Song"));
        assert_eq!(update.duration_ms, Some(12_500));
    }

    #[test]
    fn pause_keeps_metadata_and_records_position() {
        let mut mirror = MediaMirror::default();
        mirror.apply(&started());
        let update = mirror
            .apply(&Event::PlaybackPaused { session_id: "s".into(), track_id: "t".into(), position_secs: 3.2 })
            .unwrap();
        assert!(!update.playing);
        assert_eq!(update.title.as_deref(), Some("Song"));
        assert_eq!(update.position_ms, Some(3_200));
    }

    #[test]
    fn stop_clears_state_and_ticks_are_ignored() {
        let mut mirror = MediaMirror::default();
        mirror.apply(&started());
        let tick = Event::PlaybackPositionChanged { session_id: "s".into(), position_secs: 1.0, duration_secs: 12.5 };
        assert!(mirror.apply(&tick).is_none());
        let stopped = mirror.apply(&Event::PlaybackStopped { session_id: "s".into() }).unwrap();
        assert_eq!(stopped, MirrorUpdate::default());
    }

    #[test]
    fn commands_map_to_backend_commands() {
        assert!(matches!(command_for("play", None), Some(Command::Resume)));
        assert!(matches!(command_for("pause", None), Some(Command::Pause)));
        assert!(matches!(command_for("stop", None), Some(Command::Stop)));
        assert!(matches!(command_for("next", None), Some(Command::NextTrack)));
        assert!(matches!(command_for("previous", None), Some(Command::PreviousTrack)));
        match command_for("seek", Some(2_500)) {
            Some(Command::Seek { position_secs }) => assert_eq!(position_secs, 2.5),
            other => panic!("unexpected {other:?}"),
        }
        assert!(command_for("seek", None).is_none());
        assert!(command_for("bogus", None).is_none());
    }
}
