//! Device-local playback bookmark. This is not a history/listening session.
use super::queue::PlaybackQueue;
use serde::{Deserialize, Serialize};

pub(super) const SETTING_KEY: &str = "playback_session_v1";
/// The position changes every tick, so it is saved under its own small key and
/// the queue snapshot is rewritten only when the queue itself changes. Versions
/// that do not know this key restore the position stored in the snapshot.
pub(super) const POSITION_KEY: &str = "playback_position_v1";

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub(super) struct SavedPosition {
    pub queue_id: String,
    pub position_secs: f64,
    pub duration_secs: f64,
}

impl SavedPosition {
    /// The saved position, if it belongs to the snapshot's current entry.
    pub fn apply_to(&self, saved: &mut SavedPlayback) {
        let matches = saved.queue.current().is_some_and(|item| item.queue_id == self.queue_id);
        let finite = |v: f64| v.is_finite() && v >= 0.0 && std::time::Duration::try_from_secs_f64(v).is_ok();
        if matches && finite(self.position_secs) && finite(self.duration_secs) {
            saved.position_secs = self.position_secs;
            if self.duration_secs > 0.0 {
                saved.duration_secs = self.duration_secs;
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(super) struct SavedPlayback {
    pub version: u8,
    pub queue: PlaybackQueue,
    pub has_current_track: bool,
    pub position_secs: f64,
    pub duration_secs: f64,
    pub volume: f32,
}

impl SavedPlayback {
    pub fn valid(&self) -> bool {
        self.version == 1
            && self.queue.valid_snapshot()
            && (!self.has_current_track || self.queue.current().is_some())
            && self.position_secs.is_finite()
            && self.position_secs >= 0.0
            && self.duration_secs.is_finite()
            && self.duration_secs >= 0.0
            && std::time::Duration::try_from_secs_f64(self.position_secs).is_ok()
            && std::time::Duration::try_from_secs_f64(self.duration_secs).is_ok()
            && self.volume.is_finite()
            && (0.0..=1.0).contains(&self.volume)
    }
}

#[derive(Clone)]
pub(super) struct RestoredTrack {
    pub queue_id: String,
    pub position_secs: f64,
    pub duration_secs: f64,
}
