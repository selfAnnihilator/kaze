use super::*;
fn item(id: &str) -> QueueItem {
    QueueItem { queue_id: id.into(), track_id: id.into(), title: id.into(), artist: "Artist".into(), duration_secs: 180.0 }
}
fn queue(n: usize, current: usize) -> PlaybackQueue {
    let mut q = PlaybackQueue::new();
    q.set_queue((0..n).map(|i| item(&format!("t{i}"))).collect(), Some(current));
    q
}

#[test]
fn up_next_is_queue_order_without_shuffle() {
    let q = queue(5, 1);
    assert_eq!(q.upcoming_indices(), vec![2, 3, 4]);
    assert_eq!(q.track_ids_at(&q.upcoming_indices()), vec!["t2", "t3", "t4"]);
}

#[test]
fn up_next_follows_the_shuffled_play_order() {
    let mut q = queue(30, 0);
    q.set_shuffle(true);
    let order = q.upcoming_indices();
    assert_eq!(order.len(), 29, "every other song is still upcoming");
    // Up Next's first entry is exactly what Next plays, and so on down the list.
    for expected in order {
        assert_eq!(q.peek_next().map(|i| i.track_id.clone()), Some(format!("t{expected}")));
        q.skip_forward();
    }
}

#[test]
fn a_stopped_queue_lists_everything() {
    let mut q = PlaybackQueue::new();
    q.enqueue(item("a"), false);
    q.enqueue(item("b"), false);
    assert_eq!(q.upcoming_indices(), vec![0, 1]);
}
