use crate::core::command::RepeatMode;
use crate::core::event::QueueItem;
use rand::seq::SliceRandom;
use rand::thread_rng;

/// Manages the playback track sequence, history pointer, shuffle map, and repeat state.
#[derive(Debug, Clone, Default)]
pub struct PlaybackQueue {
    items: Vec<QueueItem>,
    current_index: Option<usize>,
    shuffle_indices: Vec<usize>,
    shuffle_enabled: bool,
    repeat_mode: RepeatMode,
}

impl PlaybackQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the current queue with new items, optionally starting at a specific index.
    pub fn set_queue(&mut self, items: Vec<QueueItem>, start_index: Option<usize>) {
        self.items = items;
        self.current_index = start_index;
        self.rebuild_shuffle_indices();
    }

    /// Makes an independently selected track current while retaining already-played
    /// independent tracks as Previous history and preserving the upcoming queue.
    pub fn play_independent(&mut self, item: QueueItem, preserve_history: bool) {
        let split_at = self.current_index.map_or(0, |index| index + 1);
        let mut upcoming = self.items.split_off(split_at.min(self.items.len()));
        upcoming.retain(|queued| queued.track_id != item.track_id);

        if !preserve_history {
            self.items.clear();
        }

        self.items.push(item);
        self.current_index = Some(self.items.len() - 1);
        self.items.append(&mut upcoming);
        self.shuffle_enabled = false;
        self.rebuild_shuffle_indices();
    }

    /// Enqueues a single item. If `play_next` is true, inserts after current index; otherwise appends.
    pub fn enqueue(&mut self, item: QueueItem, play_next: bool) -> bool {
        let upcoming_start = self.current_index.map_or(0, |index| index + 1);
        if self.items.iter().skip(upcoming_start).any(|queued| queued.track_id == item.track_id) {
            return false;
        }
        if self.items.is_empty() {
            self.items.push(item);
            self.current_index = None;
        } else if play_next {
            let insert_at = self.current_index.map(|idx| idx + 1).unwrap_or(self.items.len());
            self.items.insert(insert_at, item);
        } else {
            self.items.push(item);
        }
        self.rebuild_shuffle_indices();
        true
    }

    /// Removes an item by index.
    pub fn remove(&mut self, index: usize) -> Option<QueueItem> {
        if index >= self.items.len() {
            return None;
        }

        let removed = self.items.remove(index);
        if let Some(curr) = self.current_index {
            if curr == index {
                if self.items.is_empty() {
                    self.current_index = None;
                } else if curr >= self.items.len() {
                    self.current_index = Some(self.items.len() - 1);
                }
            } else if curr > index {
                self.current_index = Some(curr - 1);
            }
        }
        self.rebuild_shuffle_indices();
        Some(removed)
    }

    /// Removes all occurrences of a track ID from the queue.
    pub fn remove_track(&mut self, track_id: &str) -> bool {
        let initial_len = self.items.len();
        let mut idx = 0;
        while idx < self.items.len() {
            if self.items[idx].track_id == track_id {
                self.remove(idx);
            } else {
                idx += 1;
            }
        }
        self.items.len() != initial_len
    }

    /// Clears the queue.
    pub fn clear(&mut self) {
        self.items.clear();
        self.current_index = None;
        self.shuffle_indices.clear();
    }

    /// Returns the currently active QueueItem.
    pub fn current(&self) -> Option<&QueueItem> {
        self.current_index.and_then(|idx| self.items.get(idx))
    }

    /// Returns current queue index.
    pub fn current_index(&self) -> Option<usize> {
        self.current_index
    }

    /// Sets the current queue pointer directly.
    pub fn set_current_index(&mut self, index: usize) -> Option<&QueueItem> {
        if index < self.items.len() {
            self.current_index = Some(index);
            self.items.get(index)
        } else {
            None
        }
    }

    /// Peeks at the item that would follow the current one without mutating any
    /// queue state.  Respects shuffle order and the current repeat mode:
    ///
    /// * **RepeatMode::One**  — returns the *current* item (it will replay).
    /// * **RepeatMode::All**  — wraps around when the current track is last.
    /// * **RepeatMode::None** — returns `None` when the current track is last.
    ///
    /// This is the single authoritative implementation for "what comes next?"
    /// lookups; callers must never reproduce the shuffle-index arithmetic
    /// themselves.
    pub fn peek_next(&self) -> Option<&QueueItem> {
        if self.items.is_empty() {
            return None;
        }

        // RepeatOne — same track replays on natural completion.
        if self.repeat_mode == crate::core::command::RepeatMode::One {
            return self.current();
        }

        if self.shuffle_enabled && !self.shuffle_indices.is_empty() {
            let current_pos_in_shuffle = self
                .current_index
                .and_then(|curr| self.shuffle_indices.iter().position(|&idx| idx == curr))
                .unwrap_or(0);

            return if current_pos_in_shuffle + 1 < self.shuffle_indices.len() {
                let next_idx = self.shuffle_indices[current_pos_in_shuffle + 1];
                self.items.get(next_idx)
            } else if self.repeat_mode == crate::core::command::RepeatMode::All {
                // Wrap: first position in the shuffle order.
                let first_idx = self.shuffle_indices[0];
                self.items.get(first_idx)
            } else {
                None
            };
        }

        // Sequential playback.
        match self.current_index {
            Some(curr) if curr + 1 < self.items.len() => self.items.get(curr + 1),
            Some(_) if self.repeat_mode == crate::core::command::RepeatMode::All => {
                self.items.first()
            }
            None if !self.items.is_empty() => self.items.first(),
            _ => None,
        }
    }

    /// Advances to the next track for an explicit user action. Repeat-one does not
    /// trap the Next button on the current track.
    pub fn next(&mut self) -> Option<&QueueItem> {
        self.advance(false)
    }

    /// Advances after natural completion, honoring repeat-one.
    pub fn next_after_finish(&mut self) -> Option<&QueueItem> {
        self.advance(true)
    }

    fn advance(&mut self, honor_repeat_one: bool) -> Option<&QueueItem> {
        if self.items.is_empty() {
            return None;
        }

        // Repeat One: replay same track
        if honor_repeat_one && self.repeat_mode == RepeatMode::One {
            return self.current();
        }

        if self.shuffle_enabled && !self.shuffle_indices.is_empty() {
            let current_pos_in_shuffle = self
                .current_index
                .and_then(|curr| self.shuffle_indices.iter().position(|&idx| idx == curr))
                .unwrap_or(0);

            if current_pos_in_shuffle + 1 < self.shuffle_indices.len() {
                let next_idx = self.shuffle_indices[current_pos_in_shuffle + 1];
                self.current_index = Some(next_idx);
                return self.items.get(next_idx);
            } else if self.repeat_mode == RepeatMode::All {
                let next_idx = self.shuffle_indices[0];
                self.current_index = Some(next_idx);
                return self.items.get(next_idx);
            } else {
                return None;
            }
        }

        // Sequential playback
        match self.current_index {
            Some(curr) if curr + 1 < self.items.len() => {
                self.current_index = Some(curr + 1);
                self.items.get(curr + 1)
            }
            Some(_) if self.repeat_mode == RepeatMode::All => {
                self.current_index = Some(0);
                self.items.first()
            }
            None if !self.items.is_empty() => {
                self.current_index = Some(0);
                self.items.first()
            }
            _ => None,
        }
    }

    /// Moves to the previous track in the queue.
    pub fn previous(&mut self) -> Option<&QueueItem> {
        if self.items.is_empty() {
            return None;
        }

        if self.shuffle_enabled && !self.shuffle_indices.is_empty() {
            let current_pos_in_shuffle = self
                .current_index
                .and_then(|curr| self.shuffle_indices.iter().position(|&idx| idx == curr))
                .unwrap_or(0);

            if current_pos_in_shuffle > 0 {
                let prev_idx = self.shuffle_indices[current_pos_in_shuffle - 1];
                self.current_index = Some(prev_idx);
                return self.items.get(prev_idx);
            } else if self.repeat_mode == RepeatMode::All {
                let prev_idx = self.shuffle_indices[self.shuffle_indices.len() - 1];
                self.current_index = Some(prev_idx);
                return self.items.get(prev_idx);
            } else {
                return self.current();
            }
        }

        match self.current_index {
            Some(curr) if curr > 0 => {
                self.current_index = Some(curr - 1);
                self.items.get(curr - 1)
            }
            Some(_) if self.repeat_mode == RepeatMode::All => {
                let last = self.items.len() - 1;
                self.current_index = Some(last);
                self.items.get(last)
            }
            _ => self.current(),
        }
    }

    /// Toggles or sets shuffle mode.
    pub fn set_shuffle(&mut self, enabled: bool) {
        self.shuffle_enabled = enabled;
        if enabled {
            self.rebuild_shuffle_indices();
        }
    }

    pub fn is_shuffle(&self) -> bool {
        self.shuffle_enabled
    }

    /// Sets the repeat mode.
    pub fn set_repeat_mode(&mut self, mode: RepeatMode) {
        self.repeat_mode = mode;
    }

    pub fn repeat_mode(&self) -> RepeatMode {
        self.repeat_mode
    }

    /// Returns a slice of the queue items.
    pub fn items(&self) -> &[QueueItem] {
        &self.items
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn rebuild_shuffle_indices(&mut self) {
        let mut indices: Vec<usize> = (0..self.items.len()).collect();
        if self.shuffle_enabled {
            let mut rng = thread_rng();
            indices.shuffle(&mut rng);

            // Keep current item at the beginning of the shuffled sequence if currently playing
            if let Some(curr) = self.current_index {
                if let Some(pos) = indices.iter().position(|&x| x == curr) {
                    indices.swap(0, pos);
                }
            }
        }
        self.shuffle_indices = indices;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str) -> QueueItem {
        QueueItem {
            queue_id: format!("queue-{id}"),
            track_id: id.to_string(),
            title: id.to_string(),
            artist: "Artist".to_string(),
            duration_secs: 180.0,
        }
    }

    #[test]
    fn repeat_one_only_replays_on_natural_completion() {
        let mut queue = PlaybackQueue::new();
        queue.set_queue(vec![item("one"), item("two")], Some(0));
        queue.set_repeat_mode(RepeatMode::One);

        assert_eq!(queue.next_after_finish().map(|entry| entry.track_id.as_str()), Some("one"));
        assert_eq!(queue.next().map(|entry| entry.track_id.as_str()), Some("two"));
    }

    #[test]
    fn independent_selection_keeps_history_and_upcoming_queue() {
        let mut queue = PlaybackQueue::new();
        queue.set_queue(vec![item("first"), item("queued")], Some(0));
        queue.play_independent(item("second"), true);

        assert_eq!(queue.current().map(|entry| entry.track_id.as_str()), Some("second"));
        assert_eq!(queue.previous().map(|entry| entry.track_id.as_str()), Some("first"));
        assert_eq!(queue.next().map(|entry| entry.track_id.as_str()), Some("second"));
        assert_eq!(queue.next().map(|entry| entry.track_id.as_str()), Some("queued"));
    }

    // --- peek_next tests ---

    #[test]
    fn peek_next_sequential_does_not_mutate() {
        let mut queue = PlaybackQueue::new();
        queue.set_queue(vec![item("a"), item("b"), item("c")], Some(0));
        // Peeking twice must yield the same result.
        assert_eq!(queue.peek_next().map(|e| e.track_id.as_str()), Some("b"));
        assert_eq!(queue.peek_next().map(|e| e.track_id.as_str()), Some("b"));
        // Current must still be "a".
        assert_eq!(queue.current().map(|e| e.track_id.as_str()), Some("a"));
    }

    #[test]
    fn peek_next_at_end_with_no_repeat_returns_none() {
        let mut queue = PlaybackQueue::new();
        queue.set_queue(vec![item("only")], Some(0));
        assert_eq!(queue.peek_next(), None);
    }

    #[test]
    fn peek_next_at_end_with_repeat_all_wraps() {
        let mut queue = PlaybackQueue::new();
        queue.set_queue(vec![item("x"), item("y")], Some(1));
        queue.set_repeat_mode(RepeatMode::All);
        assert_eq!(queue.peek_next().map(|e| e.track_id.as_str()), Some("x"));
    }

    #[test]
    fn peek_next_with_repeat_one_returns_current() {
        let mut queue = PlaybackQueue::new();
        queue.set_queue(vec![item("solo"), item("next")], Some(0));
        queue.set_repeat_mode(RepeatMode::One);
        // Peek must see the *current* track because it will replay.
        assert_eq!(queue.peek_next().map(|e| e.track_id.as_str()), Some("solo"));
        // Queue must still be unmutated.
        assert_eq!(queue.current().map(|e| e.track_id.as_str()), Some("solo"));
    }

    #[test]
    fn peek_next_no_current_returns_first() {
        let mut queue = PlaybackQueue::new();
        queue.set_queue(vec![item("first"), item("second")], None);
        assert_eq!(queue.peek_next().map(|e| e.track_id.as_str()), Some("first"));
    }

    #[test]
    fn peek_next_empty_queue_returns_none() {
        let queue = PlaybackQueue::new();
        assert_eq!(queue.peek_next(), None);
    }
}
