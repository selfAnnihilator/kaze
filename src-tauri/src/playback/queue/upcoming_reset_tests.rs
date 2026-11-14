use super::*;
fn item(id: &str) -> QueueItem {
    QueueItem { queue_id: id.into(), track_id: id.into(), title: id.into(), artist: "Artist".into(), duration_secs: 180.0 }
}
#[test]
fn reset_keeps_current_identity_previous_and_repeat_but_discards_upcoming() {
    let mut q = PlaybackQueue::new();
    q.set_queue(vec![item("previous"), item("current"), item("old-manual")], Some(1));
    q.set_repeat_mode(RepeatMode::One);
    q.set_shuffle(true);
    q.clear_upcoming();
    assert_eq!(q.current().unwrap().queue_id, "current");
    assert_eq!(q.repeat_mode(), RepeatMode::One);
    assert!(!q.is_shuffle());
    q.enqueue(item("fresh-flow"), false);
    assert_eq!(q.skip_forward().unwrap().track_id, "fresh-flow");
    assert_eq!(q.previous().unwrap().track_id, "current");
    assert_eq!(q.previous().unwrap().track_id, "previous");
}
#[test]
fn reset_without_current_discards_every_pending_entry() {
    let mut q = PlaybackQueue::new();
    q.enqueue(item("old"), false);
    q.clear_upcoming();
    assert!(q.is_empty());
    assert_eq!(q.current_index(), None);
}
