//! A phone that does not have every song another device played must still sync.
use music_player_backend::cloud::{CloudTrackDeviceStat, SyncManager, SyncPayload};
use music_player_backend::database::create_in_memory_pool;

#[tokio::test]
async fn device_stats_for_songs_missing_here_do_not_break_sync() {
    let pool = create_in_memory_pool().await.expect("create db pool");
    let user = "user-phone";
    let payload = SyncPayload {
        track_device_stats: vec![CloudTrackDeviceStat {
            user_id: user.to_string(),
            device_id: "linux-desktop".to_string(),
            track_id: "only-on-the-desktop".to_string(),
            play_count: 7,
            total_seconds: 1400.0,
            completion_count: 6,
            skip_count: 1,
            last_played_at: Some(1_790_000_000),
            updated_at: 1_790_000_000,
        }],
        ..Default::default()
    };

    SyncManager::apply_remote_sync_payload(&pool, user, &payload)
        .await
        .expect("sync applies even when the song is not in this library");

    let device_plays: i64 = sqlx::query_scalar(
        "SELECT play_count FROM track_device_statistics WHERE user_id = ? AND track_id = ?",
    )
    .bind(user)
    .bind("only-on-the-desktop")
    .fetch_one(&pool)
    .await
    .expect("the device row is kept");
    assert_eq!(device_plays, 7);
}
