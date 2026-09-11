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

    // Test RepeatMode::One
    processor
        .dispatch_command(Command::SetRepeatMode {
            mode: RepeatMode::One,
        })
        .await
        .expect("set repeat one");

    processor
        .dispatch_command(Command::NextTrack)
        .await
        .expect("next with repeat one");

    let state_rep_one = match processor.execute_query(Query::GetPlaybackState).await.unwrap() {
        QueryResponse::PlaybackState(v) => v,
        _ => panic!("state"),
    };
    assert_eq!(state_rep_one["current_queue_index"].as_u64().unwrap(), 0);

    // Test RepeatMode::All (wrap around)
    processor
        .dispatch_command(Command::SetRepeatMode {
            mode: RepeatMode::All,
        })
        .await
        .expect("set repeat all");

    // Move to track 2
    processor.dispatch_command(Command::NextTrack).await.expect("next");
    // Next from track 2 should wrap around to track 1!
    processor.dispatch_command(Command::NextTrack).await.expect("wrap around next");

    let state_wrap = match processor.execute_query(Query::GetPlaybackState).await.unwrap() {
        QueryResponse::PlaybackState(v) => v,
        _ => panic!("state"),
    };
    assert_eq!(state_wrap["current_queue_index"].as_u64().unwrap(), 0);
}
