use music_player_backend::cloud::{
    CloudPlaylist, CloudPlaylistSong, CloudSong, CloudSongStat, SyncManager, SyncPayload,
};
use music_player_backend::config::AppConfig;
use music_player_backend::core::command::Command;
use music_player_backend::core::processor::CoreProcessor;
use music_player_backend::core::query::{Query, QueryResponse};
use music_player_backend::database::create_in_memory_pool;
use music_player_backend::database::repositories::user_repo::UserProfile;
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

    // 3. Test apply remote sync payload (including liked and disliked songs)
    let test_user_id = "user_cloud_123";
    let remote_payload = SyncPayload {
        songs: vec![
            CloudSong {
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
            },
            CloudSong {
                id: "cloud_track_disliked".to_string(),
                user_id: test_user_id.to_string(),
                title: "Bad Song".to_string(),
                artist: Some("Annoying Artist".to_string()),
                album: None,
                duration_secs: 180.0,
                provider: Some("online".to_string()),
                provider_id: Some("cloud_track_disliked".to_string()),
                cover_art_url: None,
                preview_url: None,
                created_at: 1700000000,
                updated_at: 1700000000,
            },
        ],
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
        song_stats: vec![
            CloudSongStat {
                id: format!("{}:cloud_track_1", test_user_id),
                user_id: test_user_id.to_string(),
                song_id: "cloud_track_1".to_string(),
                play_count: 42,
                total_time_listened: 10248.0,
                completion_count: 40,
                skip_count: 2,
                last_played_at: Some(1700000000),
                manual_like: 1, // Liked
                updated_at: 1700000000,
            },
            CloudSongStat {
                id: format!("{}:cloud_track_disliked", test_user_id),
                user_id: test_user_id.to_string(),
                song_id: "cloud_track_disliked".to_string(),
                play_count: 1,
                total_time_listened: 10.0,
                completion_count: 0,
                skip_count: 1,
                last_played_at: Some(1700000000),
                manual_like: -1, // Disliked
                updated_at: 1700000000,
            },
        ],
        user_stats: Some(serde_json::json!([{
            "id": format!("{}:2026", test_user_id),
            "user_id": test_user_id,
            "year": 2026,
            "total_seconds": 55000.0,
            "top_songs_json": "[]",
            "top_artists_json": "[]",
            "updated_at": 1700000000,
        }])),
        daily_stats: vec![],
        track_device_stats: vec![],
        user_settings: None,
        deleted_playlists: vec![],
        deleted_playlist_songs: vec![],
    };

    SyncManager::apply_remote_sync_payload(&pool, test_user_id, &remote_payload)
        .await
        .expect("apply remote payload ok");

    // Set active user profile on processor to verify user-scoped playlists
    *processor.current_user.write().await = Some(UserProfile::new(
        test_user_id.to_string(),
        "cloud_user".to_string(),
        1700000000,
    ));

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
    assert!(prepared.songs.iter().any(|s| s.title == "Bad Song"));
    assert!(prepared.song_stats.iter().any(|ss| ss.song_id == "cloud_track_1" && ss.play_count >= 42 && ss.manual_like == 1));
    assert!(prepared.song_stats.iter().any(|ss| ss.song_id == "cloud_track_disliked" && ss.manual_like == -1));
    assert!(prepared.user_stats.is_some());
}

#[tokio::test]
async fn test_logout_preserves_local_user_data_while_gating_ui() {
    let pool = create_in_memory_pool().await.expect("create db pool");
    let config = AppConfig::default_with_dirs();
    let backend = Box::new(MockAudioBackend::new());
    let processor = CoreProcessor::new_with_backend(pool.clone(), config, backend);

    let test_user_id = "user_persist_123";
    let profile = UserProfile::new(test_user_id.to_string(), "persisting_user".to_string(), 1700000000);

    // 1. Authenticate user
    *processor.current_user.write().await = Some(profile.clone());

    // 2. Create playlist as authenticated user
    let create_cmd = Command::CreatePlaylist {
        name: "My Drive Mix".to_string(),
        description: Some("Favorite driving songs".to_string()),
    };
    let pl_res = processor.dispatch_command(create_cmd).await.expect("create playlist ok");
    let pl_id = match pl_res {
        music_player_backend::core::command::CommandResponse::EntityId(id) => id,
        _ => panic!("Expected EntityId"),
    };

    // 3. Add track to playlist and like the track
    let track_id = "track_synth_99";
    processor
        .dispatch_command(Command::AddTrackToPlaylist {
            playlist_id: pl_id.clone(),
            track_id: track_id.to_string(),
            title: Some("Nightcall".to_string()),
            artist: Some("Kavinsky".to_string()),
            album: Some("OutRun".to_string()),
            duration_secs: Some(259.0),
            cover_art_url: None,
            preview_url: None,
        })
        .await
        .expect("add track ok");

    processor
        .dispatch_command(Command::LikeTrack {
            track_id: track_id.to_string(),
        })
        .await
        .expect("like track ok");

    // Verify playlist is visible when logged in
    let logged_in_pls = processor.execute_query(Query::GetPlaylists).await.expect("query pls");
    match logged_in_pls {
        QueryResponse::Playlists(list) => {
            let custom: Vec<_> = list.iter().filter(|p| p["is_smart_mix"] != 1).collect();
            assert_eq!(custom.len(), 2);
            assert!(custom.iter().any(|p| p["name"] == "My Drive Mix"));
            assert!(custom.iter().any(|p| p["name"] == "Liked Songs"));
        }
        _ => panic!("Expected Playlists"),
    }

    // 4. Logout
    processor.dispatch_command(Command::Logout).await.expect("logout ok");

    // Verify current user is None
    assert!(processor.current_user.read().await.is_none());

    // CRITICAL: Verify local persisted records in SQLite REMAIN
    let count_in_db: (i64,) = sqlx::query_as("SELECT count(*) FROM playlists WHERE is_smart_mix = 0")
        .fetch_one(&pool)
        .await
        .expect("db query");
    assert_eq!(count_in_db.0, 2, "Logout must preserve user playlists and the default Liked Songs playlist!");

    let track_count_in_db: (i64,) = sqlx::query_as("SELECT count(*) FROM playlist_tracks WHERE playlist_id = ?")
        .bind(&pl_id)
        .fetch_one(&pool)
        .await
        .expect("db query");
    assert_eq!(track_count_in_db.0, 1, "Logout must NOT delete playlist tracks from local SQLite!");

    let stats_count: (i64,) = sqlx::query_as("SELECT manual_like FROM track_statistics WHERE track_id = ?")
        .bind(track_id)
        .fetch_one(&pool)
        .await
        .expect("db query");
    assert_eq!(stats_count.0, 1, "Logout must NOT delete track likes from local SQLite!");

    // CRITICAL: Verify UI query does NOT return custom playlists while signed out
    let logged_out_pls = processor.execute_query(Query::GetPlaylists).await.expect("query pls");
    match logged_out_pls {
        QueryResponse::Playlists(list) => {
            let custom: Vec<_> = list.iter().filter(|p| p["is_smart_mix"] != 1).collect();
            assert_eq!(custom.len(), 0, "Signed-out queries must only return Smart Mixes!");
        }
        _ => panic!("Expected Playlists"),
    }

    // 5. Re-login as the same user
    *processor.current_user.write().await = Some(profile.clone());

    // Verify user's custom playlist and songs are immediately visible again
    let re_login_pls = processor.execute_query(Query::GetPlaylists).await.expect("query pls");
    match re_login_pls {
        QueryResponse::Playlists(list) => {
            let custom: Vec<_> = list.iter().filter(|p| p["is_smart_mix"] != 1).collect();
            assert_eq!(custom.len(), 2);
            assert!(custom.iter().any(|p| p["name"] == "My Drive Mix"));
            assert!(custom.iter().any(|p| p["name"] == "Liked Songs"));
        }
        _ => panic!("Expected Playlists"),
    }
}

#[tokio::test]
async fn test_explicit_deletion_creates_tombstones() {
    let pool = create_in_memory_pool().await.expect("create db pool");
    let config = AppConfig::default_with_dirs();
    let backend = Box::new(MockAudioBackend::new());
    let processor = CoreProcessor::new_with_backend(pool.clone(), config, backend);

    let test_user_id = "user_tombstone_test";
    let profile = UserProfile::new(test_user_id.to_string(), "tombstone_tester".to_string(), 1700000000);

    *processor.current_user.write().await = Some(profile);

    // Create a playlist
    let pl_res = processor
        .dispatch_command(Command::CreatePlaylist {
            name: "To Be Deleted".to_string(),
            description: None,
        })
        .await
        .expect("create pl ok");
    let pl_id = match pl_res {
        music_player_backend::core::command::CommandResponse::EntityId(id) => id,
        _ => panic!("Expected EntityId"),
    };

    // Explicitly delete it
    processor
        .dispatch_command(Command::DeletePlaylist {
            playlist_id: pl_id.clone(),
        })
        .await
        .expect("delete pl ok");

    // Verify tombstone was recorded
    let tombstones = SyncManager::get_tombstones(&pool, test_user_id)
        .await
        .expect("get tombstones ok");
    assert_eq!(tombstones.len(), 1);
    assert_eq!(tombstones[0].1, "playlist");
    assert_eq!(tombstones[0].2, pl_id);

    // Verify local sync payload includes deleted_playlists
    let payload = SyncManager::prepare_local_sync_payload(&pool, test_user_id)
        .await
        .expect("prepare payload ok");
    assert!(payload.deleted_playlists.contains(&pl_id));

    // Clear tombstones simulates push success
    SyncManager::clear_tombstones(&pool, test_user_id).await.expect("clear ok");
    let cleared = SyncManager::get_tombstones(&pool, test_user_id)
        .await
        .expect("get tombstones ok");
    assert_eq!(cleared.len(), 0);
}

#[tokio::test]
async fn test_liked_songs_is_per_account_and_protected() {
    let pool = create_in_memory_pool().await.expect("create db pool");
    let config = AppConfig::default_with_dirs();
    let backend = Box::new(MockAudioBackend::new());
    let processor = CoreProcessor::new_with_backend(pool, config, backend);

    let first_user = UserProfile::new("liked_user_one".to_string(), "first".to_string(), 1700000000);
    *processor.current_user.write().await = Some(first_user);
    let first_playlists = processor.execute_query(Query::GetPlaylists).await.expect("first playlists");
    let first_liked_id = match first_playlists {
        QueryResponse::Playlists(list) => list
            .iter()
            .find(|playlist| playlist["name"] == "Liked Songs")
            .and_then(|playlist| playlist["id"].as_str())
            .expect("first account liked songs")
            .to_string(),
        _ => panic!("Expected Playlists"),
    };

    assert!(processor
        .dispatch_command(Command::DeletePlaylist { playlist_id: first_liked_id.clone() })
        .await
        .is_err());
    assert!(processor
        .dispatch_command(Command::RenamePlaylist {
            playlist_id: first_liked_id.clone(),
            name: "Favorites".to_string(),
        })
        .await
        .is_err());

    let second_user = UserProfile::new("liked_user_two".to_string(), "second".to_string(), 1700000000);
    *processor.current_user.write().await = Some(second_user);
    let second_playlists = processor.execute_query(Query::GetPlaylists).await.expect("second playlists");
    match second_playlists {
        QueryResponse::Playlists(list) => {
            let second_liked = list.iter().find(|playlist| playlist["name"] == "Liked Songs").expect("second account liked songs");
            assert_ne!(second_liked["id"].as_str(), Some(first_liked_id.as_str()));
            assert!(list.iter().all(|playlist| playlist["id"] != first_liked_id));
        }
        _ => panic!("Expected Playlists"),
    }
}
