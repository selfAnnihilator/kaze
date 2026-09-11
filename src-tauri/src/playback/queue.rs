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

    /// Enqueues a single item. If `play_next` is true, inserts after current index; otherwise appends.
    pub fn enqueue(&mut self, item: QueueItem, play_next: bool) {
        if self.items.is_empty() {
            self.items.push(item);
            self.current_index = Some(0);
        } else if play_next {
            let insert_at = self.current_index.map(|idx| idx + 1).unwrap_or(self.items.len());
            self.items.insert(insert_at, item);
        } else {
            self.items.push(item);
        }
        self.rebuild_shuffle_indices();
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

    /// Advances to the next track respecting repeat and shuffle modes.
    pub fn next(&mut self) -> Option<&QueueItem> {
        if self.items.is_empty() {
            return None;
        }

        // Repeat One: replay same track
        if self.repeat_mode == RepeatMode::One {
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
