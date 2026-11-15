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

fn order(queue: &PlaybackQueue) -> Vec<&str> {
    queue.items().iter().map(|item| item.track_id.as_str()).collect()
}

/// A playlist A, B, C playing A.
fn playlist() -> PlaybackQueue {
    let mut queue = PlaybackQueue::new();
    queue.set_queue(vec![item("a"), item("b"), item("c")], Some(0));
    queue
}

#[test]
fn queued_songs_play_next_in_order_then_the_playlist_continues() {
    let mut queue = playlist();
    assert!(queue.enqueue_manual(item("x")));
    assert!(queue.enqueue_manual(item("y")));
    assert_eq!(order(&queue), ["a", "x", "y", "b", "c"]);

    assert_eq!(queue.peek_next().unwrap().track_id, "x");
    assert_eq!(queue.next_after_finish().unwrap().track_id, "x");
    assert_eq!(queue.skip_forward().unwrap().track_id, "y");
    assert_eq!(queue.next_after_finish().unwrap().track_id, "b");
    assert_eq!(queue.next_after_finish().unwrap().track_id, "c");
}

#[test]
fn app_songs_land_after_the_users_and_new_picks_jump_ahead_of_them() {
    let mut queue = playlist();
    queue.enqueue_manual(item("x"));
    queue.enqueue(item("refill"), false);
    assert_eq!(order(&queue), ["a", "x", "b", "c", "refill"]);

    queue.skip_forward(); // x plays and stops counting as waiting
    queue.enqueue_manual(item("w"));
    assert_eq!(order(&queue), ["a", "x", "w", "b", "c", "refill"]);
    assert_eq!(queue.peek_next().unwrap().track_id, "w");
}

#[test]
fn a_song_already_waiting_is_not_queued_twice() {
    let mut queue = playlist();
    assert!(queue.enqueue_manual(item("x")));
    assert!(!queue.enqueue_manual(item("x")));
    assert!(!queue.enqueue_manual(item("b")), "already upcoming in the playlist");
}

#[test]
fn queued_songs_come_next_even_with_shuffle_on() {
    let mut queue = PlaybackQueue::new();
    let list: Vec<QueueItem> = (0..30).map(|n| item(&format!("song-{n}"))).collect();
    queue.set_queue(list, Some(0));
    queue.set_shuffle(true);
    queue.enqueue_manual(item("x"));
    queue.enqueue_manual(item("y"));
    assert_eq!(queue.peek_next().unwrap().track_id, "x");
    assert_eq!(queue.next_after_finish().unwrap().track_id, "x");
    assert_eq!(queue.next_after_finish().unwrap().track_id, "y");
}

#[test]
fn reordering_the_list_keeps_queued_songs_first() {
    let mut queue = playlist();
    queue.enqueue_manual(item("x"));
    queue.reorder_upcoming(|upcoming| upcoming.reverse());
    assert_eq!(order(&queue), ["a", "x", "c", "b"]);
}

#[test]
fn starting_a_new_list_puts_waiting_songs_behind_it() {
    let mut queue = playlist();
    queue.enqueue_manual(item("x"));
    // A new playlist replaces the queue; the frontend re-adds what was waiting.
    queue.set_queue(vec![item("p1"), item("p2")], Some(0));
    queue.enqueue(item("x"), false);
    queue.enqueue_manual(item("new"));
    assert_eq!(order(&queue), ["p1", "new", "p2", "x"]);
}

#[test]
fn playing_one_song_keeps_queued_songs_next() {
    let mut queue = playlist();
    queue.enqueue_manual(item("x"));
    queue.play_independent(item("search-pick"), true);
    assert_eq!(queue.peek_next().unwrap().track_id, "x");
    queue.enqueue_manual(item("y"));
    assert_eq!(queue.next_after_finish().unwrap().track_id, "x");
    assert_eq!(queue.next_after_finish().unwrap().track_id, "y");
}

#[test]
fn removing_a_queued_song_forgets_it() {
    let mut queue = playlist();
    queue.enqueue_manual(item("x"));
    assert!(queue.remove_track("x"));
    queue.enqueue_manual(item("y"));
    assert_eq!(order(&queue), ["a", "y", "b", "c"]);
}

#[test]
fn old_saved_queues_without_the_field_still_load() {
    let mut queue = playlist();
    queue.enqueue_manual(item("x"));
    let mut json = serde_json::to_value(&queue).unwrap();
    json.as_object_mut().unwrap().remove("manual_ids");
    let restored: PlaybackQueue = serde_json::from_value(json).unwrap();
    assert!(restored.valid_snapshot());
    assert_eq!(order(&restored), ["a", "x", "b", "c"]);

    let saved: PlaybackQueue = serde_json::from_value(serde_json::to_value(&queue).unwrap()).unwrap();
    assert!(saved.is_manual("queue-x"), "the flag survives a restart");
}
