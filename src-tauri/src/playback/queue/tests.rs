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
fn replace_track_swaps_ids_in_place() {
    let mut queue = PlaybackQueue::new();
    for id in ["a", "local", "b"] {
        queue.enqueue(item(id), false);
    }
    queue.set_current_index(1);
    assert!(queue.replace_track("local", "jiosaavn:x"));
    let ids: Vec<_> = queue.items().iter().map(|i| i.track_id.as_str()).collect();
    assert_eq!(ids, ["a", "jiosaavn:x", "b"]);
    assert_eq!(queue.current().map(|i| i.track_id.as_str()), Some("jiosaavn:x"));
    assert!(!queue.replace_track("missing", "y"));
}

#[test]
fn repeat_one_only_replays_on_natural_completion() {
    let mut queue = PlaybackQueue::new();
    queue.set_queue(vec![item("one"), item("two")], Some(0));
    queue.set_repeat_mode(RepeatMode::One);

    assert_eq!(queue.next_after_finish().map(|entry| entry.track_id.as_str()), Some("one"));
    assert_eq!(queue.skip_forward().map(|entry| entry.track_id.as_str()), Some("two"));
}

#[test]
fn independent_selection_keeps_history_and_upcoming_queue() {
    let mut queue = PlaybackQueue::new();
    queue.set_queue(vec![item("first"), item("queued")], Some(0));
    queue.play_independent(item("second"), true);

    assert_eq!(queue.current().map(|entry| entry.track_id.as_str()), Some("second"));
    assert_eq!(queue.previous().map(|entry| entry.track_id.as_str()), Some("first"));
    assert_eq!(queue.skip_forward().map(|entry| entry.track_id.as_str()), Some("second"));
    assert_eq!(queue.skip_forward().map(|entry| entry.track_id.as_str()), Some("queued"));
}

// --- shuffle tests ---

fn ids(n: usize) -> Vec<QueueItem> {
    (0..n).map(|i| item(&format!("t{i}"))).collect()
}

fn current_id(queue: &PlaybackQueue) -> String {
    queue.current().expect("current item").track_id.clone()
}

#[test]
fn shuffle_visits_every_item_once_with_current_first() {
    for _ in 0..20 {
        let mut queue = PlaybackQueue::new();
        queue.set_queue(ids(12), Some(5));
        queue.set_shuffle(true);
        assert!(queue.valid_snapshot());

        let mut seen = vec![current_id(&queue)];
        assert_eq!(seen[0], "t5", "current item stays current when shuffle starts");
        while let Some(next) = queue.skip_forward() {
            seen.push(next.track_id.clone());
        }

        let mut sorted = seen.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(seen.len(), 12, "each item visited exactly once: {seen:?}");
        assert_eq!(sorted.len(), 12, "no repeats before the end: {seen:?}");
    }
}

#[test]
fn shuffle_with_repeat_all_wraps_to_first_shuffled_item() {
    let mut queue = PlaybackQueue::new();
    queue.set_queue(ids(5), Some(0));
    queue.set_shuffle(true);
    queue.set_repeat_mode(RepeatMode::All);
    for _ in 0..4 {
        queue.skip_forward();
    }
    assert_eq!(queue.skip_forward().map(|entry| entry.track_id.as_str()), Some("t0"));
}

#[test]
fn disabling_shuffle_restores_original_order() {
    let mut queue = PlaybackQueue::new();
    queue.set_queue(ids(6), Some(0));
    queue.set_shuffle(true);
    queue.skip_forward();
    queue.skip_forward();
    let resumed_at = queue.current_index().expect("current index");

    queue.set_shuffle(false);
    assert!(!queue.is_shuffle());
    let rest: Vec<String> =
        std::iter::from_fn(|| queue.skip_forward().map(|entry| entry.track_id.clone())).collect();
    let expected: Vec<String> = ((resumed_at + 1)..6).map(|i| format!("t{i}")).collect();
    assert_eq!(rest, expected, "sequential order resumes after the current item");
}

#[test]
fn removing_items_mid_shuffle_keeps_queue_consistent() {
    let mut queue = PlaybackQueue::new();
    queue.set_queue(ids(8), Some(0));
    queue.set_shuffle(true);
    queue.skip_forward();
    queue.skip_forward();
    let playing = current_id(&queue);

    // Remove an item that is not current, then the current one.
    let other = (0..queue.len())
        .find(|&i| Some(i) != queue.current_index())
        .expect("another item");
    let removed = queue.remove(other).expect("removed").track_id;
    assert!(queue.valid_snapshot());
    assert_eq!(current_id(&queue), playing, "current survives removal of another item");

    let current = queue.current_index().expect("current index");
    queue.remove(current);
    assert!(queue.valid_snapshot());
    assert_eq!(queue.len(), 6);

    let mut steps = 0;
    while let Some(next) = queue.skip_forward() {
        assert_ne!(next.track_id, removed);
        assert_ne!(next.track_id, playing);
        steps += 1;
        assert!(steps <= queue.len(), "shuffle must terminate without repeat");
    }

    // Out-of-range removal is a no-op.
    assert!(queue.remove(99).is_none());
    assert!(queue.valid_snapshot());
}

#[test]
fn reorder_upcoming_keeps_played_and_current_in_place() {
    let mut queue = PlaybackQueue::new();
    queue.set_queue(ids(6), Some(2));
    queue.reorder_upcoming(|upcoming| upcoming.reverse());

    let order: Vec<&str> = queue.items().iter().map(|entry| entry.track_id.as_str()).collect();
    assert_eq!(order, ["t0", "t1", "t2", "t5", "t4", "t3"]);
    assert_eq!(queue.current_index(), Some(2));
    assert_eq!(current_id(&queue), "t2");
    assert!(queue.valid_snapshot());
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
