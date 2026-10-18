//! Device-local playback bookmark. This is not a history/listening session.
use super::queue::PlaybackQueue;
use serde::{Deserialize, Serialize};

pub(super) const SETTING_KEY: &str = "playback_session_v1";

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
