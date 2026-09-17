use music_player_backend::config::AppConfig;
use music_player_backend::core::command::{Command, CommandResponse};
use music_player_backend::core::processor::CoreProcessor;
use music_player_backend::core::query::{Query, QueryResponse};
use music_player_backend::database::create_in_memory_pool;
use music_player_backend::playback::backend::MockAudioBackend;
use music_player_backend::playback::stream::{
    PartFileCleanupGuard, RemoteAudioCacheConfig, StreamPlaybackManager,
};
use std::fs::FileTimes;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tempfile::tempdir;

fn create_test_manager(temp_path: std::path::PathBuf, config: RemoteAudioCacheConfig) -> StreamPlaybackManager {
    StreamPlaybackManager::with_config(temp_path, None, None, config)
}

#[tokio::test]
async fn test_cache_key_generation() {
    let key1 = StreamPlaybackManager::compute_cache_key("online:song-1", " The Beatles ", "HEY JUDE ");
    let key2 = StreamPlaybackManager::compute_cache_key("online:song-1", "the beatles", "hey jude");
    assert_eq!(key1, key2);
    assert_eq!(key1, "online:song-1:the beatles:hey jude");
}

#[tokio::test]
async fn test_cache_path_and_part_path_naming() {
    let dir = tempdir().unwrap();
    let manager = create_test_manager(dir.path().to_path_buf(), RemoteAudioCacheConfig::default());

    let cache_path = manager.get_cache_path("online:123:artist:title");
    assert!(cache_path.starts_with(manager.cache_dir()));
    assert!(cache_path.extension().unwrap() == "audio");

    let part_path = StreamPlaybackManager::get_part_path(&cache_path);
    assert_eq!(
        part_path.file_name().unwrap().to_str().unwrap(),
        format!("{}.part", cache_path.file_name().unwrap().to_str().unwrap())
    );
}

#[tokio::test]
async fn test_cache_hit_validation_and_mtime_update() {
    let dir = tempdir().unwrap();
    let manager = create_test_manager(dir.path().to_path_buf(), RemoteAudioCacheConfig::default());
    tokio::fs::create_dir_all(manager.cache_dir()).await.unwrap();

    let cache_path = manager.get_cache_path("track-test-mtime");
    // File <= 8192 bytes is not considered valid complete audio
    tokio::fs::write(&cache_path, vec![0u8; 100]).await.unwrap();
    assert!(!StreamPlaybackManager::is_valid_cache_entry(&cache_path));

    // File > 8192 bytes is valid
    tokio::fs::write(&cache_path, vec![0u8; 10000]).await.unwrap();
    assert!(StreamPlaybackManager::is_valid_cache_entry(&cache_path));

    // Set mtime to 1 hour ago
    let one_hour_ago = SystemTime::now() - Duration::from_secs(3600);
    let times = FileTimes::new().set_modified(one_hour_ago);
    let file = std::fs::File::options().write(true).open(&cache_path).unwrap();
    file.set_times(times).unwrap();
    drop(file);

    let mtime_before = std::fs::metadata(&cache_path).unwrap().modified().unwrap();
    assert!(mtime_before < SystemTime::now() - Duration::from_secs(3500));

    // Touch cache entry (simulating cache hit)
    StreamPlaybackManager::touch_cache_entry(&cache_path);

    let mtime_after = std::fs::metadata(&cache_path).unwrap().modified().unwrap();
    assert!(mtime_after > SystemTime::now() - Duration::from_secs(5));
}

#[tokio::test]
async fn test_lru_eviction_when_exceeding_max_cache_bytes() {
    let dir = tempdir().unwrap();
    let config = RemoteAudioCacheConfig {
        max_size_bytes: 35000,
        eviction_target_bytes: 25000,
        max_single_file_bytes: 50000,
        stale_part_max_age_secs: 86400,
    };
    let manager = create_test_manager(dir.path().to_path_buf(), config);
    tokio::fs::create_dir_all(manager.cache_dir()).await.unwrap();

    // Create 4 files of 10000 bytes each (Total = 40000 bytes > max 35000)
    let f1 = manager.get_cache_path("track-1");
    let f2 = manager.get_cache_path("track-2");
    let f3 = manager.get_cache_path("track-3");
    let f4 = manager.get_cache_path("track-4");

    tokio::fs::write(&f1, vec![1u8; 10000]).await.unwrap();
    tokio::fs::write(&f2, vec![2u8; 10000]).await.unwrap();
    tokio::fs::write(&f3, vec![3u8; 10000]).await.unwrap();
    tokio::fs::write(&f4, vec![4u8; 10000]).await.unwrap();

    // Set spaced mtimes: f1 oldest, then f2, then f3, f4 newest
    let now = SystemTime::now();
    for (i, path) in [&f1, &f2, &f3, &f4].iter().enumerate() {
        let t = now - Duration::from_secs(1000 - (i as u64 * 100));
        let file = std::fs::File::options().write(true).open(path).unwrap();
        file.set_times(FileTimes::new().set_modified(t)).unwrap();
    }

    let eviction = manager.enforce_cache_limits().await.unwrap();
    assert_eq!(eviction.total_before, 40000);
    assert_eq!(eviction.files_evicted, 2); // Evict f1 (10000) -> 30000; then f2 (10000) -> 20000 (<= target 25000)
    assert_eq!(eviction.bytes_evicted, 20000);
    assert_eq!(eviction.total_after, 20000);

    assert!(!f1.exists(), "Oldest file f1 should have been evicted");
    assert!(!f2.exists(), "Second oldest file f2 should have been evicted");
    assert!(f3.exists(), "f3 should be retained");
    assert!(f4.exists(), "f4 should be retained");
}

#[tokio::test]
async fn test_active_playing_track_protected_from_eviction() {
    let dir = tempdir().unwrap();
    let config = RemoteAudioCacheConfig {
        max_size_bytes: 25000,
        eviction_target_bytes: 20000,
        max_single_file_bytes: 50000,
        stale_part_max_age_secs: 86400,
    };
    let manager = create_test_manager(dir.path().to_path_buf(), config);
    tokio::fs::create_dir_all(manager.cache_dir()).await.unwrap();

    let f1 = manager.get_cache_path("track-1-playing");
    let f2 = manager.get_cache_path("track-2");
    let f3 = manager.get_cache_path("track-3");

    tokio::fs::write(&f1, vec![1u8; 10000]).await.unwrap();
    tokio::fs::write(&f2, vec![2u8; 10000]).await.unwrap();
    tokio::fs::write(&f3, vec![3u8; 10000]).await.unwrap();

    // f1 is the oldest file
    let now = SystemTime::now();
    let file1 = std::fs::File::options().write(true).open(&f1).unwrap();
    file1.set_times(FileTimes::new().set_modified(now - Duration::from_secs(1000))).unwrap();
    let file2 = std::fs::File::options().write(true).open(&f2).unwrap();
    file2.set_times(FileTimes::new().set_modified(now - Duration::from_secs(500))).unwrap();
    let file3 = std::fs::File::options().write(true).open(&f3).unwrap();
    file3.set_times(FileTimes::new().set_modified(now)).unwrap();

    // Protect f1 as actively playing!
    manager.set_active_playing_path(Some(f1.clone())).await;

    let eviction = manager.enforce_cache_limits().await.unwrap();
    assert_eq!(eviction.files_evicted, 1); // Only f2 is evicted

    assert!(f1.exists(), "Active playing file f1 must be protected from eviction");
    assert!(!f2.exists(), "f2 should be evicted instead");
    assert!(f3.exists(), "f3 should be retained");
}

#[tokio::test]
async fn test_active_download_protected_from_eviction() {
    let dir = tempdir().unwrap();
    let config = RemoteAudioCacheConfig {
        max_size_bytes: 25000,
        eviction_target_bytes: 15000,
        max_single_file_bytes: 50000,
        stale_part_max_age_secs: 86400,
    };
    let manager = create_test_manager(dir.path().to_path_buf(), config);
    tokio::fs::create_dir_all(manager.cache_dir()).await.unwrap();

    let f1 = manager.get_cache_path("track-downloading");
    let f2 = manager.get_cache_path("track-other");
    let f3 = manager.get_cache_path("track-fresh");

    tokio::fs::write(&f1, vec![1u8; 10000]).await.unwrap();
    tokio::fs::write(&f2, vec![2u8; 10000]).await.unwrap();
    tokio::fs::write(&f3, vec![3u8; 10000]).await.unwrap();

    // Make f1 oldest
    let now = SystemTime::now();
    let file1 = std::fs::File::options().write(true).open(&f1).unwrap();
    file1.set_times(FileTimes::new().set_modified(now - Duration::from_secs(1000))).unwrap();

    // Protect f1 as an active download
    manager.mark_active_download_for_test(f1.clone()).await;

    let _ = manager.enforce_cache_limits().await.unwrap();

    assert!(f1.exists(), "Actively downloading file must be protected from eviction");
    assert!(!f2.exists(), "f2 should have been evicted instead");
}

#[tokio::test]
async fn test_clean_stale_partial_files_removes_old_parts_and_preserves_fresh() {
    let dir = tempdir().unwrap();
    let manager = create_test_manager(dir.path().to_path_buf(), RemoteAudioCacheConfig::default());
    tokio::fs::create_dir_all(manager.cache_dir()).await.unwrap();

    let old_part = manager.cache_dir().join("old_download.audio.part");
    let fresh_part = manager.cache_dir().join("fresh_download.audio.part");

    tokio::fs::write(&old_part, vec![0u8; 5000]).await.unwrap();
    tokio::fs::write(&fresh_part, vec![0u8; 5000]).await.unwrap();

    // Set old_part to 48 hours ago
    let old_time = SystemTime::now() - Duration::from_secs(48 * 3600);
    let file = std::fs::File::options().write(true).open(&old_part).unwrap();
    file.set_times(FileTimes::new().set_modified(old_time)).unwrap();
    drop(file);

    let removed = manager.clean_stale_partial_files(Duration::from_secs(24 * 3600)).await;
    assert_eq!(removed, 1);
    assert!(!old_part.exists(), "Stale part file older than 24h should be removed");
    assert!(fresh_part.exists(), "Fresh part file should be preserved");
}

#[tokio::test]
async fn test_part_file_cleanup_guard_on_drop_and_disarm() {
    let dir = tempdir().unwrap();
    let part_cancelled = dir.path().join("cancelled.audio.part");
    let part_completed = dir.path().join("completed.audio.part");

    tokio::fs::write(&part_cancelled, b"partial data").await.unwrap();
    tokio::fs::write(&part_completed, b"partial data").await.unwrap();

    // Guard 1: Drop without disarm (cancellation / error)
    {
        let _guard = PartFileCleanupGuard::new(part_cancelled.clone());
    }
    assert!(!part_cancelled.exists(), "Drop guard must delete file if not disarmed");

    // Guard 2: Disarmed upon successful completion
    {
        let mut guard = PartFileCleanupGuard::new(part_completed.clone());
        guard.disarm();
    }
    assert!(part_completed.exists(), "Disarmed guard must keep the file");
}

#[tokio::test]
async fn test_clear_cache_removes_unprotected_files_and_preserves_active() {
    let dir = tempdir().unwrap();
    let manager = create_test_manager(dir.path().to_path_buf(), RemoteAudioCacheConfig::default());
    tokio::fs::create_dir_all(manager.cache_dir()).await.unwrap();

    let f1 = manager.get_cache_path("track-1");
    let f2 = manager.get_cache_path("track-2-playing");
    let part = manager.cache_dir().join("some.audio.part");

    tokio::fs::write(&f1, vec![1u8; 10000]).await.unwrap();
    tokio::fs::write(&f2, vec![2u8; 15000]).await.unwrap();
    tokio::fs::write(&part, vec![3u8; 5000]).await.unwrap();

    manager.set_active_playing_path(Some(f2.clone())).await;

    let res = manager.clear_cache().await.unwrap();
    assert_eq!(res.files_removed, 2); // f1 and part removed
    assert_eq!(res.bytes_freed, 15000); // 10000 + 5000

    assert!(!f1.exists());
    assert!(!part.exists());
    assert!(f2.exists(), "Active playing file must not be removed by clear_cache");
}

#[tokio::test]
async fn test_get_cache_stats_accurate() {
    let dir = tempdir().unwrap();
    let config = RemoteAudioCacheConfig {
        max_size_bytes: 100000,
        eviction_target_bytes: 90000,
        max_single_file_bytes: 50000,
        stale_part_max_age_secs: 86400,
    };
    let manager = create_test_manager(dir.path().to_path_buf(), config);
    tokio::fs::create_dir_all(manager.cache_dir()).await.unwrap();

    let f1 = manager.get_cache_path("track-1");
    let f2 = manager.get_cache_path("track-2");
    let part1 = manager.cache_dir().join("dl1.audio.part");
    let part2 = manager.cache_dir().join("dl2.audio.part");

    tokio::fs::write(&f1, vec![0u8; 20000]).await.unwrap();
    tokio::fs::write(&f2, vec![0u8; 30000]).await.unwrap();
    tokio::fs::write(&part1, vec![0u8; 4000]).await.unwrap();
    tokio::fs::write(&part2, vec![0u8; 6000]).await.unwrap();

    let stats = manager.get_cache_stats().await.unwrap();
    assert_eq!(stats.file_count, 2);
    assert_eq!(stats.partial_file_count, 2);
    assert_eq!(stats.total_size_bytes, 60000);
    assert_eq!(stats.max_size_bytes, 100000);
}

#[tokio::test]
async fn test_legacy_stream_cache_migration() {
    let dir = tempdir().unwrap();
    let legacy_dir = dir.path().join("stream_cache");
    let new_dir = dir.path().join("remote-audio");
    tokio::fs::create_dir_all(&legacy_dir).await.unwrap();

    let legacy_file = legacy_dir.join("legacy_hash.audio");
    tokio::fs::write(&legacy_file, vec![42u8; 12000]).await.unwrap();

    let manager = create_test_manager(dir.path().to_path_buf(), RemoteAudioCacheConfig::default());
    manager.migrate_legacy_cache().await;

    let migrated_file = new_dir.join("legacy_hash.audio");
    assert!(migrated_file.exists(), "Legacy file must be moved to remote-audio dir");
    assert!(!legacy_file.exists(), "Legacy file must no longer exist in stream_cache");
}

#[tokio::test]
async fn test_safety_invariant_cannot_evict_outside_cache_dir() {
    let dir = tempdir().unwrap();
    let manager = create_test_manager(dir.path().to_path_buf(), RemoteAudioCacheConfig::default());
    tokio::fs::create_dir_all(manager.cache_dir()).await.unwrap();

    // Create a sibling directory containing user files
    let user_music_dir = dir.path().join("Music");
    tokio::fs::create_dir_all(&user_music_dir).await.unwrap();
    let user_file = user_music_dir.join("favorite_song.mp3");
    tokio::fs::write(&user_file, b"User music content").await.unwrap();

    // Run eviction
    let _ = manager.enforce_cache_limits().await.unwrap();

    // User file in sibling directory must NEVER be touched
    assert!(user_file.exists());
}

#[tokio::test]
async fn test_ipc_command_clear_cache_and_query_stats() {
    let pool = create_in_memory_pool().await.expect("DB pool");
    let mock_backend = Arc::new(MockAudioBackend::new());
    let backend_box = Box::new((*mock_backend).clone());

    let temp = tempdir().unwrap();
    let mut config = AppConfig::default_with_dirs();
    config.cache_dir = temp.path().to_path_buf();

    let processor = CoreProcessor::new_with_backend(pool, config.clone(), backend_box);

    // Query stats when empty
    let query_res = processor
        .execute_query(Query::GetRemoteAudioCacheStats)
        .await
        .expect("query cache stats");

    match query_res {
        QueryResponse::RemoteAudioCacheStats {
            total_size_bytes,
            file_count,
            max_size_bytes,
            partial_file_count,
        } => {
            assert_eq!(file_count, 0);
            assert_eq!(total_size_bytes, 0);
            assert_eq!(partial_file_count, 0);
            assert_eq!(max_size_bytes, 1024 * 1024 * 1024);
        }
        _ => panic!("Expected RemoteAudioCacheStats variant"),
    }

    // Populate a test audio file in cache
    let cache_dir = config.cache_dir.join("remote-audio");
    tokio::fs::create_dir_all(&cache_dir).await.unwrap();
    let test_file = cache_dir.join("test_hash.audio");
    tokio::fs::write(&test_file, vec![0u8; 15000]).await.unwrap();

    // Clear cache command
    let cmd_res = processor
        .dispatch_command(Command::ClearRemoteAudioCache)
        .await
        .expect("clear cache command");

    match cmd_res {
        CommandResponse::RemoteAudioCacheCleared {
            bytes_freed,
            files_removed,
        } => {
            assert_eq!(files_removed, 1);
            assert_eq!(bytes_freed, 15000);
        }
        _ => panic!("Expected RemoteAudioCacheCleared variant"),
    }

    assert!(!test_file.exists(), "Cache file must have been cleared");
}

#[tokio::test]
async fn test_eviction_when_cache_below_limit_does_nothing() {
    let dir = tempdir().unwrap();
    let config = RemoteAudioCacheConfig {
        max_size_bytes: 50000,
        eviction_target_bytes: 40000,
        max_single_file_bytes: 50000,
        stale_part_max_age_secs: 86400,
    };
    let manager = create_test_manager(dir.path().to_path_buf(), config);
    tokio::fs::create_dir_all(manager.cache_dir()).await.unwrap();

    let f1 = manager.get_cache_path("track-small");
    tokio::fs::write(&f1, vec![0u8; 10000]).await.unwrap();

    let res = manager.enforce_cache_limits().await.unwrap();
    assert_eq!(res.files_evicted, 0);
    assert_eq!(res.bytes_evicted, 0);
    assert_eq!(res.total_after, 10000);
    assert!(f1.exists());
}

#[tokio::test]
async fn test_stream_playback_manager_downloads_preview_stream() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await;
            let response = "HTTP/1.1 200 OK\r\nContent-Length: 100\r\nContent-Type: audio/aac\r\n\r\n";
            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.write_all(&vec![42u8; 100]).await;
            let _ = socket.flush().await;
        }
    });

    let dir = tempdir().unwrap();
    let manager = create_test_manager(dir.path().to_path_buf(), RemoteAudioCacheConfig::default());
    let url = format!("http://{}/test.aac", addr);

    let (path, dur) = manager
        .resolve_and_prepare_audio("test-preview-id", "Test Song", "Test Artist", Some(&url))
        .await
        .unwrap();

    assert_eq!(dur, 30.0);
    assert!(path.exists());
    let metadata = std::fs::metadata(&path).unwrap();
    assert_eq!(metadata.len(), 100);
}

