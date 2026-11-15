//! Songs the user queues play before the app's own queue (collection
//! remainders, Flow refills, autoplay). They play in the order they were
//! added; when they run out, the app's queue continues.
use super::*;

impl PlaybackQueue {
    /// Queues a song the user picked: after the playing song and any songs the
    /// user queued before it, ahead of everything the app queued.
    /// A song the app already queued further down moves up instead of repeating.
    pub fn enqueue_manual(&mut self, item: QueueItem) -> bool {
        self.prune_manual();
        let upcoming_start = self.current_index.map_or(0, |index| index + 1);
        let already = self
            .items
            .iter()
            .enumerate()
            .skip(upcoming_start)
            .find(|(_, queued)| queued.track_id == item.track_id)
            .map(|(index, queued)| (index, self.is_manual(&queued.queue_id)));
        match already {
            Some((_, true)) => return false,
            Some((index, false)) => {
                self.remove(index);
            }
            None => {}
        }
        let upcoming_start = self.current_index.map_or(0, |index| index + 1);
        let mut at = upcoming_start.min(self.items.len());
        while self.items.get(at).is_some_and(|queued| self.is_manual(&queued.queue_id)) {
            at += 1;
        }
        self.manual_ids.push(item.queue_id.clone());
        self.items.insert(at, item);
        self.rebuild_shuffle_indices();
        true
    }

    /// Track ids of the user's songs still waiting to play, in the order they
    /// were added (a song stops waiting once it starts playing).
    pub fn waiting_manual_track_ids(&self) -> Vec<String> {
        self.manual_ids
            .iter()
            .filter_map(|id| self.items.iter().find(|item| &item.queue_id == id))
            .filter(|item| self.current().map(|current| &current.queue_id) != Some(&item.queue_id))
            .map(|item| item.track_id.clone())
            .collect()
    }

    pub(super) fn is_manual(&self, queue_id: &str) -> bool {
        self.manual_ids.iter().any(|id| id == queue_id)
    }

    /// Forgets songs that left the queue or are playing now. Starting a new
    /// playlist replaces every queue id, so it starts with an empty user queue.
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
