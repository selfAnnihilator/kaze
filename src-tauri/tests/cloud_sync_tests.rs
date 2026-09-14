use music_player_backend::cloud::{
    CloudPlaylist, CloudPlaylistSong, CloudSong, CloudSongStat, CloudUserStat, SyncManager,
    SyncPayload,
};
use music_player_backend::config::AppConfig;
use music_player_backend::core::command::Command;
use music_player_backend::core::processor::CoreProcessor;
use music_player_backend::core::query::{Query, QueryResponse};
use music_player_backend::database::create_in_memory_pool;
use music_player_backend::playback::backend::MockAudioBackend;

#[tokio::test]
async fn test_cloud_sync_payload_and_database_persistence() {
    let pool = create_in_memory_pool().await.expect("create db pool");
    let config = AppConfig::default_with_dirs();
    let backend = Box::new(MockAudioBackend::new());
    let processor = CoreProcessor::new_with_backend(pool.clone(), config, backend);

    // 1. Check initial cloud sync status
    let status_res = processor
        .execute_query(Query::GetCloudSyncStatus)
        .await
        .expect("query status");
    match status_res {
        QueryResponse::CloudSyncStatus(val) => {
            assert_eq!(val["connected"], false);
            assert!(val["worker_url"].as_str().unwrap().contains("workers.dev"));
        }
        _ => panic!("Expected CloudSyncStatus"),
    }

    // 2. Change server URL
    let set_url = Command::SetCloudServerUrl {
        url: "https://music-player-sync.example.workers.dev".to_string(),
    };
    processor.dispatch_command(set_url).await.expect("set url ok");

    let status_res2 = processor
        .execute_query(Query::GetCloudSyncStatus)
        .await
        .expect("query status 2");
    match status_res2 {
        QueryResponse::CloudSyncStatus(val) => {
            assert_eq!(val["worker_url"], "https://music-player-sync.example.workers.dev");
        }
        _ => panic!("Expected CloudSyncStatus"),
    }

    // 3. Test apply remote sync payload
    let test_user_id = "user_cloud_123";
    let remote_payload = SyncPayload {
        songs: vec![CloudSong {
            id: "cloud_track_1".to_string(),
            user_id: test_user_id.to_string(),
            title: "Midnight City".to_string(),
            artist: Some("M83".to_string()),
            album: Some("Hurry Up, We're Dreaming".to_string()),
            duration_secs: 244.0,
            provider: Some("online".to_string()),
            provider_id: Some("cloud_track_1".to_string()),
            cover_art_url: Some("https://example.com/art.jpg".to_string()),
            preview_url: None,
            created_at: 1700000000,
            updated_at: 1700000000,
        }],
        playlists: vec![CloudPlaylist {
            id: "pl_cloud_1".to_string(),
            user_id: test_user_id.to_string(),
            name: "Synthwave Favorites".to_string(),
            description: Some("Synced from cloud".to_string()),
            is_smart_mix: 0,
            mix_type: None,
            created_at: 1700000000,
            updated_at: 1700000000,
        }],
        playlist_songs: vec![CloudPlaylistSong {
            id: "pl_cloud_1:cloud_track_1".to_string(),
            user_id: test_user_id.to_string(),
            playlist_id: "pl_cloud_1".to_string(),
            song_id: "cloud_track_1".to_string(),
            position: 0,
            added_at: 1700000000,
        }],
        song_stats: vec![CloudSongStat {
            id: format!("{}:cloud_track_1", test_user_id),
            user_id: test_user_id.to_string(),
            song_id: "cloud_track_1".to_string(),
            play_count: 42,
            total_time_listened: 10248.0,
            completion_count: 40,
            skip_count: 2,
            last_played_at: Some(1700000000),
            manual_like: 1,
            updated_at: 1700000000,
        }],
        user_stats: vec![CloudUserStat {
            id: format!("{}:2026", test_user_id),
            user_id: test_user_id.to_string(),
            year: 2026,
            month: 0,
            total_seconds: 55000.0,
            top_songs_json: "[]".to_string(),
            top_artists_json: "[]".to_string(),
            updated_at: 1700000000,
        }],
        user_settings: vec![],
    };

    SyncManager::apply_remote_sync_payload(&pool, test_user_id, &remote_payload)
        .await
        .expect("apply remote payload ok");

    // Set active user profile on processor to verify user-scoped playlists
    *processor.current_user.write().await = Some(music_player_backend::database::repositories::user_repo::UserProfile {
        id: test_user_id.to_string(),
        username: "cloud_user".to_string(),
        created_at: 1700000000,
    });

    // Verify local database now reflects the synced remote playlist and songs
    let playlists_res = processor.execute_query(Query::GetPlaylists).await.expect("query playlists");
    match playlists_res {
        QueryResponse::Playlists(list) => {
            assert!(list.iter().any(|p| p["name"] == "Synthwave Favorites"));
        }
        _ => panic!("Expected Playlists response"),
    }

    // Verify local payload preparation extracts this data cleanly
    let prepared = SyncManager::prepare_local_sync_payload(&pool, test_user_id)
        .await
        .expect("prepare local payload ok");

    assert!(prepared.playlists.iter().any(|p| p.name == "Synthwave Favorites"));
    assert!(prepared.playlist_songs.iter().any(|ps| ps.song_id == "cloud_track_1"));
    assert!(prepared.songs.iter().any(|s| s.title == "Midnight City"));
    assert!(prepared.song_stats.iter().any(|ss| ss.song_id == "cloud_track_1" && ss.play_count >= 42));
    assert!(prepared.user_stats.iter().any(|us| us.year == 2026 && us.total_seconds >= 55000.0));
}
