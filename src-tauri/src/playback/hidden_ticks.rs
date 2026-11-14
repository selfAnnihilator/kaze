//! Position ticks for a window nobody can see.
//!
//! The playback monitor publishes the position 4x/s. While the main window is
//! hidden (closed to the tray), forwarding each tick to the webview only wakes
//! the renderer to discard it. The forwarder keeps the newest tick instead and
//! sends it the moment the window is shown again, so the progress bar is
//! exactly as fresh as before.

use crate::core::event::Event;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

pub struct HiddenTicks {
    visible: AtomicBool,
    latest: Mutex<Option<Event>>,
}

impl Default for HiddenTicks {
    fn default() -> Self {
        Self::new(true)
    }
}

impl HiddenTicks {
    pub fn new(visible: bool) -> Self {
        Self {
            visible: AtomicBool::new(visible),
            latest: Mutex::new(None),
        }
    }

    /// Whether to forward `event` now. A tick withheld while hidden is kept.
    pub fn forward(&self, event: &Event) -> bool {
        if !matches!(event, Event::PlaybackPositionChanged { .. }) {
            return true;
        }
        let mut latest = self.latest.lock().unwrap_or_else(|e| e.into_inner());
        if self.visible.load(Ordering::Acquire) {
            *latest = None;
            return true;
        }
        *latest = Some(event.clone());
        false
    }

    /// The window was shown: returns the tick to send right away, if any.
    pub fn shown(&self) -> Option<Event> {
        let mut latest = self.latest.lock().unwrap_or_else(|e| e.into_inner());
        self.visible.store(true, Ordering::Release);
        latest.take()
    }

    pub fn hidden(&self) {
        self.visible.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(position_secs: f64) -> Event {
        Event::PlaybackPositionChanged {
            session_id: "s".into(),
            position_secs,
            duration_secs: 100.0,
        }
    }

    #[test]
    fn hidden_window_gets_only_the_newest_tick_when_shown() {
        let ticks = HiddenTicks::new(true);
        assert!(ticks.forward(&tick(1.0)));
        ticks.hidden();
        assert!(!ticks.forward(&tick(2.0)));
        assert!(!ticks.forward(&tick(3.0)));
        assert!(ticks.forward(&Event::LibraryMetadataUpdated), "other events always pass");
        assert_eq!(ticks.shown(), Some(tick(3.0)));
        assert_eq!(ticks.shown(), None);
        assert!(ticks.forward(&tick(4.0)));
    }

    #[test]
    fn showing_without_withheld_ticks_sends_nothing() {
        let ticks = HiddenTicks::new(false);
        assert_eq!(ticks.shown(), None);
    }
}
