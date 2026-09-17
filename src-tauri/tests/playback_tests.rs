use music_player_backend::config::AppConfig;
use music_player_backend::core::command::{Command, RepeatMode};
use music_player_backend::core::event::Event;
use music_player_backend::core::processor::CoreProcessor;
use music_player_backend::core::query::{Query, QueryResponse};
use music_player_backend::database::create_in_memory_pool;
use music_player_backend::database::repositories::ScannedMetadata;
use music_player_backend::playback::backend::MockAudioBackend;
use std::sync::Arc;

async fn setup_processor_with_tracks() -> (CoreProcessor, Arc<MockAudioBackend>, String, String) {
    let pool = create_in_memory_pool().await.expect("DB pool");
    let mock_backend = Arc::new(MockAudioBackend::new());
    let backend_box = Box::new((*mock_backend).clone());

    let processor = CoreProcessor::new_with_backend(
        pool,
        AppConfig::default_with_dirs(),
        backend_box,
    );

    let track_repo = processor.library_service().track_repo();

    let track1_id = track_repo
        .save_scanned_track(ScannedMetadata {
            file_path: "/music/track1.wav".into(),
            file_size: 1000,
            modified_timestamp: 100,
            file_hash: None,
            title: "Track One".into(),
            artist: Some("Band Alpha".into()),
            album: Some("Album One".into()),
            album_artist: None,
            genre: Some("Rock".into()),
            track_number: Some(1),
            disc_number: Some(1),
            year: Some(2020),
            duration_secs: 200.0,
            bitrate: Some(320),
            sample_rate: Some(44100),
            format: "wav".into(),
            has_cover_art: false,
            musicbrainz_track_id: None,
        })
        .await
        .expect("save track1");

    let track2_id = track_repo
        .save_scanned_track(ScannedMetadata {
            file_path: "/music/track2.wav".into(),
            file_size: 2000,
            modified_timestamp: 200,
            file_hash: None,
            title: "Track Two".into(),
            artist: Some("Band Beta".into()),
            album: Some("Album Two".into()),
            album_artist: None,
            genre: Some("Electronic".into()),
            track_number: Some(2),
            disc_number: Some(1),
            year: Some(2021),
            duration_secs: 180.0,
            bitrate: Some(320),
            sample_rate: Some(44100),
            format: "wav".into(),
            has_cover_art: false,
            musicbrainz_track_id: None,
        })
        .await
        .expect("save track2");

    (processor, mock_backend, track1_id, track2_id)
}

#[tokio::test]
async fn test_playback_controls_lifecycle() {
    let (processor, _backend, track1_id, _track2_id) = setup_processor_with_tracks().await;
    let mut rx = processor.event_bus().subscribe();

    // 1. Play Track
    processor
        .dispatch_command(Command::PlayTrack {
            track_id: track1_id.clone(),
            source: Some("library".into()),
        })
        .await
        .expect("play track failed");

    // Check PlaybackStarted event
    let event = rx.recv().await.expect("event recv");
    match event {
        Event::PlaybackStarted { track_id, title, .. } => {
            assert_eq!(track_id, track1_id);
            assert_eq!(title, "Track One");
        }
        _ => panic!("Expected PlaybackStarted, got {:?}", event),
    }

    // 2. Pause
    processor
        .dispatch_command(Command::Pause)
        .await
        .expect("pause failed");

    let event = rx.recv().await.expect("event recv");
    match event {
        Event::QueueUpdated { .. } => {
            // Queue update follows start
            let next_event = rx.recv().await.expect("event recv");
            assert!(matches!(next_event, Event::PlaybackPaused { .. }));
        }
        Event::PlaybackPaused { .. } => {}
        _ => panic!("Expected PlaybackPaused or QueueUpdated, got {:?}", event),
    }

    // 3. Resume
    processor
        .dispatch_command(Command::Resume)
        .await
        .expect("resume failed");

    // 4. Seek
    processor
        .dispatch_command(Command::Seek {
            position_secs: 45.0,
        })
        .await
        .expect("seek failed");

    // 5. Volume & Mute
    processor
        .dispatch_command(Command::SetVolume { volume: 0.65 })
        .await
        .expect("set volume failed");

    processor
        .dispatch_command(Command::ToggleMute)
        .await
        .expect("toggle mute failed");

    // Verify PlaybackState query
    let state_res = processor
        .execute_query(Query::GetPlaybackState)
        .await
        .expect("query state failed");

    match state_res {
        QueryResponse::PlaybackState(val) => {
            assert_eq!(val["volume"].as_f64().unwrap(), 0.0);
            assert_eq!(val["current_track_id"].as_str().unwrap(), track1_id);
        }
        _ => panic!("Expected PlaybackState response"),
    }

    // 6. Stop
    processor
        .dispatch_command(Command::Stop)
        .await
        .expect("stop failed");
}

#[tokio::test]
async fn test_playback_queue_and_navigation() {
    let (processor, _backend, track1_id, track2_id) = setup_processor_with_tracks().await;

    // Start with track 1
    processor
        .dispatch_command(Command::PlayTrack {
            track_id: track1_id.clone(),
            source: None,
        })
        .await
        .expect("play track1");

    // Enqueue track 2
    processor
        .dispatch_command(Command::EnqueueTrack {
            track_id: track2_id.clone(),
            play_next: false,
        })
        .await
        .expect("enqueue track2");

    let duplicate = processor
        .dispatch_command(Command::EnqueueTrack {
            track_id: track2_id.clone(),
            play_next: false,
        })
        .await;
    assert!(duplicate.is_err(), "the same track must not appear twice in the upcoming queue");

    let state = match processor.execute_query(Query::GetPlaybackState).await.unwrap() {
        QueryResponse::PlaybackState(v) => v,
        _ => panic!("expected state"),
    };
    assert_eq!(state["queue_length"].as_u64().unwrap(), 2);
    assert_eq!(state["current_queue_index"].as_u64().unwrap(), 0);

    // Next Track
    processor
        .dispatch_command(Command::NextTrack)
        .await
        .expect("next track");

    let state_after_next = match processor.execute_query(Query::GetPlaybackState).await.unwrap() {
        QueryResponse::PlaybackState(v) => v,
        _ => panic!("expected state"),
    };
    assert_eq!(state_after_next["current_queue_index"].as_u64().unwrap(), 1);
    assert_eq!(state_after_next["current_track_id"].as_str().unwrap(), track2_id);

    // Previous Track
    processor
        .dispatch_command(Command::PreviousTrack)
        .await
        .expect("prev track");

    let state_after_prev = match processor.execute_query(Query::GetPlaybackState).await.unwrap() {
        QueryResponse::PlaybackState(v) => v,
        _ => panic!("expected state"),
    };
    assert_eq!(state_after_prev["current_queue_index"].as_u64().unwrap(), 0);
    assert_eq!(state_after_prev["current_track_id"].as_str().unwrap(), track1_id);
}

#[tokio::test]
async fn test_independent_playback_keeps_played_history_for_previous() {
    let (processor, _backend, track1_id, track2_id) = setup_processor_with_tracks().await;

    processor
        .dispatch_command(Command::PlayTrack {
            track_id: track1_id.clone(),
            source: Some("library".into()),
        })
        .await
        .expect("play first independent track");
    processor
        .dispatch_command(Command::PlayTrack {
            track_id: track2_id.clone(),
            source: Some("library".into()),
        })
        .await
        .expect("play second independent track");

    processor.dispatch_command(Command::PreviousTrack).await.expect("return to played track");
    let previous = match processor.execute_query(Query::GetPlaybackState).await.unwrap() {
        QueryResponse::PlaybackState(value) => value,
        _ => panic!("expected playback state"),
    };
    assert_eq!(previous["current_track_id"].as_str(), Some(track1_id.as_str()));

    processor.dispatch_command(Command::PreviousTrack).await.expect("restart first track");
    let restarted = match processor.execute_query(Query::GetPlaybackState).await.unwrap() {
        QueryResponse::PlaybackState(value) => value,
        _ => panic!("expected playback state"),
    };
    assert_eq!(restarted["current_track_id"].as_str(), Some(track1_id.as_str()));
    assert_eq!(restarted["current_queue_index"].as_u64(), Some(0));
}

#[tokio::test]
async fn test_queue_refills_randomly_after_the_planned_tracks_finish() {
    let (processor, _backend, track1_id, track2_id) = setup_processor_with_tracks().await;

    processor
        .dispatch_command(Command::PlayTrack {
            track_id: track1_id.clone(),
            source: Some("playlist".into()),
        })
        .await
        .expect("play first track");
    processor
        .dispatch_command(Command::EnqueueTrack {
            track_id: track2_id.clone(),
            play_next: false,
        })
        .await
        .expect("enqueue second track");

    processor.dispatch_command(Command::NextTrack).await.expect("play planned second track");
    processor.dispatch_command(Command::NextTrack).await.expect("refill continuous queue");

    let state = match processor.execute_query(Query::GetPlaybackState).await.unwrap() {
        QueryResponse::PlaybackState(value) => value,
        _ => panic!("expected playback state"),
    };
    assert_eq!(state["current_track_id"].as_str(), Some(track1_id.as_str()));
    assert_ne!(state["current_track_id"].as_str(), Some(track2_id.as_str()));
}

#[tokio::test]
async fn test_online_tracks_can_be_queued_and_played_through_unified_backend() {
    let (processor, _backend, track1_id, _track2_id) = setup_processor_with_tracks().await;
    let mut events = processor.event_bus().subscribe();

    use sha2::Digest;
    // Pre-populate disk cache so network fetch is not attempted in unit tests
    let config = AppConfig::default_with_dirs();
    let cache_dir = config.cache_dir.join("remote-audio");
    tokio::fs::create_dir_all(&cache_dir).await.unwrap();
    let cache_key = format!("full:v2:{}", "online:test-song:queue artist:queued stream");
    let mut hasher = sha2::Sha256::new();
    hasher.update(cache_key.as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    let cached_file = cache_dir.join(format!("{}.audio", hash));
    
    // Write valid WAV header so Symphonia accepts the cached file
    let samples = vec![0u8; 16_000];
    let mut bytes = Vec::with_capacity(44 + samples.len());
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36u32 + samples.len() as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&8000u32.to_le_bytes());
    bytes.extend_from_slice(&16000u32.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(samples.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&samples);
    tokio::fs::write(&cached_file, bytes).await.unwrap();

    processor
        .dispatch_command(Command::PlayTrack {
            track_id: track1_id,
            source: Some("library".into()),
        })
        .await
        .expect("play local track");
    processor
        .dispatch_command(Command::EnqueueOnlineTrack {
            track_id: "online:test-song".into(),
            title: "Queued Stream".into(),
            artist: "Queue Artist".into(),
            album: Some("Queue Album".into()),
            duration_secs: Some(180.0),
            cover_art_url: None,
            preview_url: Some("https://example.invalid/preview".into()),
            play_next: false,
        })
        .await
        .expect("enqueue online track");
    processor.dispatch_command(Command::NextTrack).await.expect("advance to online track");

    let mut saw_started = false;
    for _ in 0..10 {
        if let Ok(Ok(event)) = tokio::time::timeout(std::time::Duration::from_millis(100), events.recv()).await {
            if let Event::PlaybackStarted { track_id, title, .. } = event {
                if track_id == "online:test-song" {
                    assert_eq!(title, "Queued Stream");
                    saw_started = true;
                    break;
                }
            }
        }
    }
    assert!(saw_started, "advancing the shared queue must play online tracks through the unified audio backend");
}

#[tokio::test]
async fn test_repeat_modes() {
    let (processor, _backend, track1_id, track2_id) = setup_processor_with_tracks().await;

    processor
        .dispatch_command(Command::PlayTrack {
            track_id: track1_id.clone(),
            source: None,
        })
        .await
        .expect("play");

    processor
        .dispatch_command(Command::EnqueueTrack {
            track_id: track2_id.clone(),
            play_next: false,
        })
        .await
        .expect("enqueue");

    // Repeat-one applies to natural completion, but an explicit Next still advances.
    processor
        .dispatch_command(Command::SetRepeatMode {
            mode: RepeatMode::One,
        })
        .await
        .expect("set repeat one");

    processor
        .dispatch_command(Command::NextTrack)
        .await
        .expect("manual next with repeat one");

    let state_rep_one = match processor.execute_query(Query::GetPlaybackState).await.unwrap() {
        QueryResponse::PlaybackState(v) => v,
        _ => panic!("state"),
    };
    assert_eq!(state_rep_one["current_queue_index"].as_u64().unwrap(), 1);

    // Test RepeatMode::All (wrap around)
    processor
        .dispatch_command(Command::SetRepeatMode {
            mode: RepeatMode::All,
        })
        .await
        .expect("set repeat all");

    // Next from track 2 should wrap around to track 1.
    processor.dispatch_command(Command::NextTrack).await.expect("wrap around next");

    let state_wrap = match processor.execute_query(Query::GetPlaybackState).await.unwrap() {
        QueryResponse::PlaybackState(v) => v,
        _ => panic!("state"),
    };
    assert_eq!(state_wrap["current_queue_index"].as_u64().unwrap(), 0);
}

#[tokio::test]
async fn test_queue_enqueue_dequeue_and_playback_state_enrichment() {
    let (processor, _backend, track1_id, track2_id) = setup_processor_with_tracks().await;

    // 1. Play track 1
    processor
        .dispatch_command(Command::PlayTrack {
            track_id: track1_id.clone(),
            source: Some("library".into()),
        })
        .await
        .expect("play track");

    // 2. Query GetPlaybackState and verify enriched current_track
    let state = match processor.execute_query(Query::GetPlaybackState).await.unwrap() {
        QueryResponse::PlaybackState(v) => v,
        _ => panic!("Expected PlaybackState"),
    };
    assert_eq!(state["current_track_id"].as_str().unwrap(), track1_id);
    assert_eq!(state["current_track"]["title"].as_str().unwrap(), "Track One");
    assert_eq!(state["current_track"]["artist_name"].as_str().unwrap(), "Band Alpha");

    // 3. Enqueue track 2
    processor
        .dispatch_command(Command::EnqueueTrack {
            track_id: track2_id.clone(),
            play_next: false,
        })
        .await
        .expect("enqueue track 2");

    let state_enqueued = match processor.execute_query(Query::GetPlaybackState).await.unwrap() {
        QueryResponse::PlaybackState(v) => v,
        _ => panic!("Expected PlaybackState"),
    };
    let queue_ids: Vec<String> = state_enqueued["queue_track_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert_eq!(queue_ids, vec![track2_id.clone()]);

    // 4. Dequeue track 2
    processor
        .dispatch_command(Command::DequeueTrack {
            track_id: track2_id.clone(),
        })
        .await
        .expect("dequeue track 2");

    let state_dequeued = match processor.execute_query(Query::GetPlaybackState).await.unwrap() {
        QueryResponse::PlaybackState(v) => v,
        _ => panic!("Expected PlaybackState"),
    };
    let queue_ids_after: Vec<String> = state_dequeued["queue_track_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(queue_ids_after.is_empty());

    // 5. Enqueue again and clear queue
    processor
        .dispatch_command(Command::EnqueueTrack {
            track_id: track2_id.clone(),
            play_next: false,
        })
        .await
        .expect("enqueue track 2 again");
    processor
        .dispatch_command(Command::ClearQueue)
        .await
        .expect("clear queue");

    let state_cleared = match processor.execute_query(Query::GetPlaybackState).await.unwrap() {
        QueryResponse::PlaybackState(v) => v,
        _ => panic!("Expected PlaybackState"),
    };
    let queue_ids_cleared: Vec<String> = state_cleared["queue_track_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(queue_ids_cleared.is_empty());
}
