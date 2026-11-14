use super::*;
#[tokio::test]
async fn stale_background_merge_cannot_replace_a_new_scan_snapshot() {
    let pool = crate::database::create_in_memory_pool().await.unwrap();
    let repo = SqliteTrackRepository::new(pool.clone());
    let original: ScannedMetadata = serde_json::from_value(serde_json::json!({
        "file_path":"/fixture/track.flac", "file_size":100, "modified_timestamp":1,
        "title":"Filename", "title_is_fallback":true, "duration_secs":200.0,
        "format":"flac", "has_cover_art":false
    })).unwrap();
    let id = repo.save_scanned_track(original.clone()).await.unwrap();
    let old_snapshot = serde_json::to_string(&original).unwrap();
    let mut scanned = original.clone();
    scanned.title = "New Embedded Title".into(); scanned.title_is_fallback = false;
    scanned.year = Some(2026); scanned.modified_timestamp = 2;
    repo.save_scanned_track(scanned).await.unwrap();
    assert!(repo.save_scanned_track_internal(original, Some(old_snapshot)).await.is_err());
    let current = repo.find_by_id(&id).await.unwrap().unwrap();
    assert_eq!(current.title, "New Embedded Title"); assert_eq!(current.year, Some(2026));
    pool.close().await;
}
