use super::*;

fn payload(plays: i64, updated_at: i64) -> SyncPayload {
    serde_json::from_value(serde_json::json!({
        "playlists": [{ "id": "p1", "name": "Mix", "updated_at": 1 }],
        "song_stats": [
            { "song_id": "a", "play_count": plays, "updated_at": updated_at },
            { "song_id": "b", "play_count": 3, "updated_at": updated_at },
        ],
        "user_settings": [{ "key": "audio_volume", "value": "0.5" }],
    }))
    .expect("payload")
}

#[test]
fn the_first_sync_uploads_everything_and_an_unchanged_one_nothing() {
    let account = "worker-a|first";
    let (first, pending) = changed_only(account, payload(1, 100));
    assert_eq!((first.playlists.len(), first.song_stats.len()), (1, 2));
    assert!(first.user_settings.is_some());
    commit(pending);
    // Same rows, new sync time: nothing goes up.
    let (second, _) = changed_only(account, payload(1, 200));
    assert!(is_empty(&second), "{second:?}");
}

#[test]
fn only_changed_rows_go_up_and_a_failed_upload_is_retried() {
    let account = "worker-a|changes";
    let (_, pending) = changed_only(account, payload(1, 100));
    commit(pending);
    let (delta, _not_committed) = changed_only(account, payload(2, 300));
    assert_eq!(delta.song_stats.iter().map(|s| s.song_id.as_str()).collect::<Vec<_>>(), ["a"]);
    assert!(delta.playlists.is_empty() && delta.user_settings.is_none());
    // The upload above failed (never committed), so the change is sent again.
    let (retry, _) = changed_only(account, payload(2, 400));
    assert_eq!(retry.song_stats.len(), 1);
}

#[test]
fn deletions_always_go_and_accounts_are_separate() {
    let (_, pending) = changed_only("worker-a|one", payload(1, 1));
    commit(pending);
    let mut with_delete = payload(1, 1);
    with_delete.deleted_playlists = vec!["gone".into()];
    let (delta, _) = changed_only("worker-a|one", with_delete);
    assert_eq!(delta.deleted_playlists, ["gone"]);
    assert!(!is_empty(&delta));
    let (other, _) = changed_only("worker-a|two", payload(1, 1));
    assert_eq!(other.song_stats.len(), 2, "another account has its own ledger");
}

#[test]
fn a_re_stamped_yearly_summary_is_not_a_change() {
    let summary = |stamp: i64, seconds: f64| -> SyncPayload {
        serde_json::from_value(serde_json::json!({
            "user_stats": [{ "year": 2025, "total_seconds": seconds, "updated_at": stamp }],
        }))
        .expect("payload")
    };
    let account = "worker-a|yearly";
    let (_, pending) = changed_only(account, summary(1, 10.0));
    commit(pending);
    assert!(is_empty(&changed_only(account, summary(2, 10.0)).0), "only the stamp moved");
    assert!(changed_only(account, summary(3, 20.0)).0.user_stats.is_some(), "real change still goes");
}
