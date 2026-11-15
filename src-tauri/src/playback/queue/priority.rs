//! Songs the user queues play before the app's own queue (collection
//! remainders, Flow refills, autoplay). They play in the order they were
//! added; when they run out, the app's queue continues.
use super::*;

impl PlaybackQueue {
    /// Queues a song the user picked: after the playing song and any songs the
    /// user queued before it, ahead of everything the app queued.
    pub fn enqueue_manual(&mut self, item: QueueItem) -> bool {
        let upcoming_start = self.current_index.map_or(0, |index| index + 1);
        if self.items.iter().skip(upcoming_start).any(|queued| queued.track_id == item.track_id) {
            return false;
        }
        self.prune_manual();
        let mut at = upcoming_start.min(self.items.len());
        while self.items.get(at).is_some_and(|queued| self.is_manual(&queued.queue_id)) {
            at += 1;
        }
        self.manual_ids.push(item.queue_id.clone());
        self.items.insert(at, item);
        self.rebuild_shuffle_indices();
        true
    }

    pub(super) fn is_manual(&self, queue_id: &str) -> bool {
        self.manual_ids.iter().any(|id| id == queue_id)
    }

    /// Forgets songs that left the queue or are playing now. Starting a new
    /// list replaces every queue id, so the user's waiting songs fall behind it.
    pub(super) fn prune_manual(&mut self) {
        if self.manual_ids.is_empty() {
            return;
        }
        let current = self.current().map(|item| item.queue_id.clone());
        let items = &self.items;
        self.manual_ids
            .retain(|id| current.as_ref() != Some(id) && items.iter().any(|item| &item.queue_id == id));
    }

    /// A song that starts playing is no longer waiting.
    pub(super) fn unflag_current(&mut self) {
        if let Some(id) = self.current().map(|item| item.queue_id.clone()) {
            self.manual_ids.retain(|queued| queued != &id);
        }
    }

    /// In a shuffled order, puts the user's waiting songs right after the
    /// playing one (position 0), in the order they were added.
    pub(super) fn manual_first(&self, indices: &mut Vec<usize>) {
        let waiting: Vec<usize> = self
            .manual_ids
            .iter()
            .filter_map(|id| self.items.iter().position(|item| &item.queue_id == id))
            .filter(|&index| Some(index) != self.current_index)
            .collect();
        if waiting.is_empty() {
            return;
        }
        indices.retain(|index| !waiting.contains(index));
        let at = usize::from(self.current_index.is_some()).min(indices.len());
        indices.splice(at..at, waiting);
    }
}
