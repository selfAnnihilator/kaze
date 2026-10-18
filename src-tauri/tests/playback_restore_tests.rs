use music_player_backend::{
    config::AppConfig,
    core::{
        command::{Command, RepeatMode},
        event::Event,
        processor::CoreProcessor,
        query::{Query, QueryResponse},
    },
    database::init_db_pool,
    playback::backend::MockAudioBackend,
};

async fn open(dir: &tempfile::TempDir) -> (CoreProcessor, MockAudioBackend) {
    let mut config = AppConfig::default_with_dirs();
    config.database_path = dir.path().join("app.db");
    config.cache_dir = dir.path().join("cache");
    config.downloads.download_dir = Some(dir.path().join("downloads"));
    let pool = init_db_pool(&config.database_path).await.unwrap();
    let backend = MockAudioBackend::new();
    (
        CoreProcessor::new_with_backend(pool, config, Box::new(backend.clone())),
        backend,
    )
}

async fn seed(processor: &CoreProcessor) {
    for id in ["one", "two", "three"] {
        sqlx::query("INSERT INTO tracks (id,file_path,file_size,modified_timestamp,title,normalized_title,duration_secs,format,created_at,updated_at) VALUES (?,?,1,1,?,?,180,'wav',1,1)")
            .bind(id).bind(format!("/missing/{id}.wav")).bind(id).bind(id)
            .execute(processor.db_pool()).await.unwrap();
    }
    processor
        .dispatch_command(Command::PlayTrack {
            track_id: "one".into(),
            source: None,
        })
        .await
        .unwrap();
    for id in ["two", "three"] {
        processor
            .dispatch_command(Command::EnqueueTrack {
                track_id: id.into(),
                play_next: false,
            })
            .await
            .unwrap();
    }
    processor
        .dispatch_command(Command::Seek {
            position_secs: 42.25,
        })
        .await
        .unwrap();
}

async fn state(processor: &CoreProcessor) -> serde_json::Value {
    match processor
        .execute_query(Query::GetPlaybackState)
        .await
        .unwrap()
    {
        QueryResponse::PlaybackState(value) => value,
        _ => panic!("PlaybackState expected"),
    }
}

#[tokio::test]
async fn cold_launch_restores_paused_identity_queue_settings_and_exact_position_without_history() {
    let dir = tempfile::tempdir().unwrap();
    let (first, _) = open(&dir).await;
    seed(&first).await;
    first
        .dispatch_command(Command::SetVolume { volume: 0.37 })
        .await
        .unwrap();
    first
        .dispatch_command(Command::SetRepeatMode {
            mode: RepeatMode::All,
        })
        .await
        .unwrap();
    first
        .dispatch_command(Command::SetShuffle { enabled: true })
        .await
        .unwrap();
    let before = state(&first).await;
    first.shutdown().await.unwrap();
    let (second, backend) = open(&dir).await;
    let mut events = second.event_bus().subscribe();
    let after = state(&second).await;
    assert_eq!(after["current_track"]["title"], "one");
    for key in [
        "current_track_id",
        "queue_track_ids",
        "current_queue_index",
        "queue_length",
        "position_secs",
        "duration_secs",
        "repeat_mode",
        "is_shuffle",
        "volume",
    ] {
        assert_eq!(after[key], before[key], "lost {key}");
    }
    assert_eq!(after["is_playing"], false);
    assert_eq!(after["is_paused"], true);
    assert!(
        backend.loaded_path.lock().unwrap().is_none(),
        "restore must not start audio"
    );
    assert!(!std::iter::from_fn(|| events.try_recv().ok())
        .any(|e| matches!(e, Event::PlaybackStarted { .. })));
    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM active_playback_journal")
        .fetch_one(second.db_pool())
        .await
        .unwrap();
    assert_eq!(
        count.0, 0,
        "paused bookmark must not create listening history"
    );
    second
        .dispatch_command(Command::TogglePlayPause)
        .await
        .unwrap();
    assert_eq!(
        backend
            .position_millis
            .load(std::sync::atomic::Ordering::SeqCst),
        42250
    );
    assert_eq!(state(&second).await["is_playing"], true);
    second.shutdown().await.unwrap();
    let (third, _) = open(&dir).await;
    let times: Vec<f64> = sqlx::query_scalar("SELECT seconds_listened FROM playback_history")
        .fetch_all(third.db_pool())
        .await
        .unwrap();
    assert!(
        times.iter().all(|seconds| *seconds < 5.0),
        "seek offset must never count as listening: {times:?}"
    );
    third.shutdown().await.unwrap();
}

#[tokio::test]
async fn reopening_again_preserves_bookmark_and_seek_before_play_changes_resume_position() {
    let dir = tempfile::tempdir().unwrap();
    let (first, _) = open(&dir).await;
    seed(&first).await;
    first.dispatch_command(Command::Pause).await.unwrap();
    first.shutdown().await.unwrap();
    let (second, _) = open(&dir).await;
    state(&second).await;
    second
        .dispatch_command(Command::Seek {
            position_secs: 75.5,
        })
        .await
        .unwrap();
    second.shutdown().await.unwrap();
    let (third, backend) = open(&dir).await;
    assert_eq!(state(&third).await["position_secs"], 75.5);
    assert!(backend.loaded_path.lock().unwrap().is_none());
    third.dispatch_command(Command::Resume).await.unwrap();
    assert_eq!(
        backend
            .position_millis
            .load(std::sync::atomic::Ordering::SeqCst),
        75500
    );
    third.dispatch_command(Command::NextTrack).await.unwrap();
    assert_eq!(state(&third).await["current_track_id"], "two");
    assert_eq!(
        backend
            .position_millis
            .load(std::sync::atomic::Ordering::SeqCst),
        0
    );
    third.shutdown().await.unwrap();
}

#[tokio::test]
async fn online_and_missing_media_remain_visible_without_network_or_audio_on_startup() {
    let dir = tempfile::tempdir().unwrap();
    let (first, _) = open(&dir).await;
    seed(&first).await;
    sqlx::query("UPDATE tracks SET format='online',file_path='online://one' WHERE id='one'")
        .execute(first.db_pool())
        .await
        .unwrap();
    first.shutdown().await.unwrap();
    let (second, backend) = open(&dir).await;
    let restored = state(&second).await;
    assert_eq!(restored["current_track"]["format"], "online");
    assert_eq!(restored["position_secs"], 42.25);
    assert!(backend.loaded_path.lock().unwrap().is_none());
    second.dispatch_command(Command::ClearQueue).await.unwrap();
    second.shutdown().await.unwrap();
    let (third, _) = open(&dir).await;
    let cleared = state(&third).await;
    assert!(cleared["current_track_id"].is_null());
    assert_eq!(cleared["queue_length"], 0);
    third.shutdown().await.unwrap();
}

#[tokio::test]
async fn invalid_or_newer_bookmarks_do_not_break_startup_or_new_playback() {
    for invalid in [
        "not json",
        r#"{"version":99}"#,
        r#"{"version":1,"queue":{"items":[],"current_index":8,"shuffle_indices":[],"shuffle_enabled":false,"repeat_mode":"None"},"has_current_track":true,"position_secs":4,"duration_secs":180,"volume":0.8}"#,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (processor, _) = open(&dir).await;
        sqlx::query("INSERT INTO application_settings (key,value,updated_at) VALUES ('playback_session_v1',?,1)").bind(invalid).execute(processor.db_pool()).await.unwrap();
        assert!(state(&processor).await["current_track_id"].is_null());
        seed(&processor).await;
        assert_eq!(state(&processor).await["current_track_id"], "one");
        processor.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn failed_bookmark_write_still_stops_audio_and_finishes_shutdown() {
    let dir = tempfile::tempdir().unwrap();
    let (processor, backend) = open(&dir).await;
    seed(&processor).await;
    sqlx::query("CREATE TRIGGER fail_bookmark BEFORE INSERT ON application_settings WHEN NEW.key='playback_session_v1' BEGIN SELECT RAISE(ABORT,'test write failure'); END").execute(processor.db_pool()).await.unwrap();
    assert!(processor.shutdown().await.is_err());
    assert!(backend
        .is_finished_flag
        .load(std::sync::atomic::Ordering::SeqCst));
    assert!(processor.db_pool().is_closed());
    processor.shutdown().await.unwrap();
}

#[tokio::test]
async fn checkpoint_uses_existing_monitor_for_recovery_before_clean_quit() {
    let dir = tempfile::tempdir().unwrap();
    let (processor, _) = open(&dir).await;
    seed(&processor).await;
    tokio::time::timeout(std::time::Duration::from_secs(13), async {
        loop {
            let saved: Option<String> = sqlx::query_scalar(
                "SELECT value FROM application_settings WHERE key='playback_session_v1'",
            )
            .fetch_optional(processor.db_pool())
            .await
            .unwrap();
            if let Some(saved) = saved {
                let saved: serde_json::Value = serde_json::from_str(&saved).unwrap();
                assert_eq!(saved["position_secs"], 42.25);
                assert_eq!(saved["has_current_track"], true);
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("periodic bookmark checkpoint");
    processor.shutdown().await.unwrap();
}

async fn cold_online_fixture(quit_while_loading: bool) {
    use music_player_backend::playback::{ResolvedEntry, StreamPlaybackManager};
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::Arc,
    };
    let dir = tempfile::tempdir().unwrap();
    let (first, _) = open(&dir).await;
    seed(&first).await;
    first.shutdown().await.unwrap();
    let (second, backend) = open(&dir).await;
    sqlx::query("UPDATE tracks SET format='online',file_path='online://one' WHERE id='one'")
        .execute(second.db_pool())
        .await
        .unwrap();
    assert_eq!(state(&second).await["position_secs"], 42.25);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (sent, received) = tokio::sync::oneshot::channel();
    let server = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut request = [0; 4096];
        socket.read(&mut request).unwrap();
        let data_size = 8000u32 * 2 * 180;
        let mut audio = Vec::new();
        audio.extend_from_slice(b"RIFF");
        audio.extend_from_slice(&(36 + data_size).to_le_bytes());
        audio.extend_from_slice(b"WAVEfmt ");
        audio.extend_from_slice(&16u32.to_le_bytes());
        audio.extend_from_slice(&1u16.to_le_bytes());
        audio.extend_from_slice(&1u16.to_le_bytes());
        audio.extend_from_slice(&8000u32.to_le_bytes());
        audio.extend_from_slice(&16000u32.to_le_bytes());
        audio.extend_from_slice(&2u16.to_le_bytes());
        audio.extend_from_slice(&16u16.to_le_bytes());
        audio.extend_from_slice(b"data");
        audio.extend_from_slice(&data_size.to_le_bytes());
        audio.resize(44 + data_size as usize, 0);
        write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: audio/wav\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", audio.len()).unwrap();
        socket.write_all(&audio[..256 * 1024]).unwrap();
        sent.send(()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(700));
        let sent_rest = socket.write_all(&audio[256 * 1024..]);
        if !quit_while_loading {
            sent_rest.unwrap();
        }
    });
    let manager = Arc::new(StreamPlaybackManager::new(
        dir.path().join("cold-cache"),
        Some(second.db_pool().clone()),
        Some(second.download_service()),
    ));
    manager.resolution_cache().insert(
        format!(
            "full:v2:{}",
            StreamPlaybackManager::compute_cache_key("one", "Unknown Artist", "one")
        ),
        ResolvedEntry::new(format!("http://{address}/audio.wav"), 180.0, None),
    );
    second.playback_service().set_stream_manager(manager);
    let second = Arc::new(second);
    let resume_processor = second.clone();
    let resume =
        tokio::spawn(async move { resume_processor.dispatch_command(Command::Resume).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), received)
        .await
        .unwrap()
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(
        backend.loaded_path.lock().unwrap().is_none(),
        "do not play the beginning while the saved offset is unavailable"
    );
    if quit_while_loading {
        second.begin_quit();
        tokio::time::timeout(std::time::Duration::from_secs(1), second.shutdown())
            .await
            .expect("Quit must not wait for the cold download")
            .unwrap();
        let _ = resume.await.unwrap();
        assert!(backend.loaded_path.lock().unwrap().is_none());
        assert!(backend
            .is_finished_flag
            .load(std::sync::atomic::Ordering::SeqCst));
        server.join().unwrap();
        return;
    }
    tokio::time::timeout(std::time::Duration::from_secs(8), resume)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    server.join().unwrap();
    assert_eq!(
        backend
            .position_millis
            .load(std::sync::atomic::Ordering::SeqCst),
        42250
    );
    assert_eq!(state(&second).await["is_playing"], true);
    second.shutdown().await.unwrap();
}

#[tokio::test]
async fn cold_online_resume_waits_for_seekable_cache_then_starts_at_saved_offset() {
    cold_online_fixture(false).await;
}

#[tokio::test]
async fn quit_cancels_a_cold_online_resume_before_draining_commands() {
    cold_online_fixture(true).await;
}
