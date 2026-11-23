//! Listening time on a song that never counted as a full play must still sync,
//! or lifetime totals drift apart between devices.
use music_player_backend::cloud::{CloudSong, CloudSongStat, SyncManager, SyncPayload};
use music_player_backend::database::create_in_memory_pool;

#[tokio::test]
async fn partial_listens_are_uploaded() {
    let pool = create_in_memory_pool().await.expect("create db pool");
    let user = "user-partial";
    let payload = SyncPayload {
        songs: vec![CloudSong {
            id: "brief_track".to_string(),
            user_id: user.to_string(),
            title: "Brief".to_string(),
            artist: Some("Someone".to_string()),
            album: None,
            duration_secs: 200.0,
            provider: Some("online".to_string()),
            provider_id: Some("brief_track".to_string()),
            online_source: None,
            cover_art_url: None,
            preview_url: None,
            created_at: 1_700_000_000,
            updated_at: 1_700_000_000,
        }],
        song_stats: vec![CloudSongStat {
            id: format!("{user}:brief_track"),
            user_id: user.to_string(),
            song_id: "brief_track".to_string(),
            play_count: 0,
            total_time_listened: 95.0,
            completion_count: 0,
            skip_count: 1,
            last_played_at: Some(1_700_000_000),
            manual_like: 0,
            updated_at: 1_700_000_000,
        }],
        ..Default::default()
    };
    SyncManager::apply_remote_sync_payload(&pool, user, &payload).await.expect("apply");

    let prepared = SyncManager::prepare_local_sync_payload(&pool, user).await.expect("prepare");
    assert!(
        prepared.song_stats.iter().any(|s| s.song_id == "brief_track" && s.total_time_listened == 95.0),
        "a song with listening time but no counted play is uploaded"
    );
}
