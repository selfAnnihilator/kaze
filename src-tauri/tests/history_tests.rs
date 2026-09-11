use music_player_backend::config::AppConfig;
use music_player_backend::core::command::Command;
use music_player_backend::core::event::Event;
use music_player_backend::core::processor::CoreProcessor;
use music_player_backend::core::query::{Query, QueryResponse, RankingEntity, TimeWindow};
use music_player_backend::database::create_in_memory_pool;
use music_player_backend::database::models::PlaybackHistoryRecord;
use music_player_backend::database::repositories::ScannedMetadata;
use music_player_backend::playback::backend::MockAudioBackend;
use std::time::Duration;
use uuid::Uuid;

async fn setup_history_env() -> (CoreProcessor, String, String) {
    let pool = create_in_memory_pool().await.expect("DB pool");
    let mock_backend = Box::new(MockAudioBackend::new());

    let processor = CoreProcessor::new_with_backend(
        pool,
        AppConfig::default_with_dirs(),
        mock_backend,
    );

    let track_repo = processor.library_service().track_repo();

    let track1_id = track_repo
        .save_scanned_track(ScannedMetadata {
            file_path: "/music/fav_song.flac".into(),
            file_size: 1500,
            modified_timestamp: 100,
            file_hash: None,
            title: "Favorite Song".into(),
            artist: Some("The Prodigy".into()),
            album: Some("The Fat of the Land".into()),
            album_artist: None,
            genre: Some("Big Beat".into()),
            track_number: Some(1),
            disc_number: Some(1),
            year: Some(1997),
            duration_secs: 240.0,
            bitrate: Some(1000),
            sample_rate: Some(44100),
            format: "flac".into(),
            has_cover_art: false,
            musicbrainz_track_id: None,
        })
        .await
        .expect("save track1");

    let track2_id = track_repo
        .save_scanned_track(ScannedMetadata {
            file_path: "/music/skipped_song.mp3".into(),
            file_size: 800,
            modified_timestamp: 100,
            file_hash: None,
            title: "Skipped Song".into(),
            artist: Some("Unknown Band".into()),
            album: Some("Demo Tape".into()),
            album_artist: None,
            genre: Some("Punk".into()),
            track_number: Some(2),
            disc_number: Some(1),
            year: Some(2005),
            duration_secs: 180.0,
            bitrate: Some(320),
            sample_rate: Some(44100),
            format: "mp3".into(),
            has_cover_art: false,
            musicbrainz_track_id: None,
        })
        .await
        .expect("save track2");

    (processor, track1_id, track2_id)
}

#[tokio::test]
async fn test_meaningful_play_and_history_logging() {
    let (processor, track1_id, track2_id) = setup_history_env().await;
    let event_bus = processor.event_bus();
    let stats_repo = processor.history_service().stats_repo();

    // 1. Simulate a completed meaningful play on Track 1 (240 seconds listened)
    event_bus
        .publish(Event::PlaybackStarted {
            track_id: track1_id.clone(),
            title: "Favorite Song".into(),
            artist: "The Prodigy".into(),
            duration_secs: 240.0,
            source: "library".into(),
        })
        .unwrap();

    // Wait briefly for async event bus processing
    tokio::time::sleep(Duration::from_millis(50)).await;

    event_bus
        .publish(Event::TrackFinished {
            track_id: track1_id.clone(),
            seconds_listened: 240.0,
            completed: true,
        })
        .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Verify stats for track 1: play_count = 1, completion_count = 1, skip_count = 0
    let stats1 = stats_repo.get_track_stats(&track1_id).await.unwrap().expect("stats1");
    assert_eq!(stats1.play_count, 1);
    assert_eq!(stats1.completion_count, 1);
    assert_eq!(stats1.skip_count, 0);
    assert!(stats1.total_time_listened >= 240.0);

    // 2. Simulate an accidental click / skip on Track 2 (only 5 seconds listened)
    event_bus
        .publish(Event::PlaybackStarted {
            track_id: track2_id.clone(),
            title: "Skipped Song".into(),
            artist: "Unknown Band".into(),
            duration_secs: 180.0,
            source: "library".into(),
        })
        .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Stopped after 5 seconds
    event_bus
        .publish(Event::PlaybackPositionChanged {
            position_secs: 5.0,
            duration_secs: 180.0,
        })
        .unwrap();

    tokio::time::sleep(Duration::from_millis(20)).await;

    event_bus.publish(Event::PlaybackStopped).unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Verify stats for track 2: play_count = 0, skip_count = 1
    let stats2 = stats_repo.get_track_stats(&track2_id).await.unwrap().expect("stats2");
    assert_eq!(stats2.play_count, 0, "5s playback must not count as play");
    assert_eq!(stats2.skip_count, 1, "Short playback should count as skip");

    // Check history list
    let history_items = processor.history_service().get_recent_history(10).await.unwrap();
    assert_eq!(history_items.len(), 2);
}

#[tokio::test]
async fn test_user_preferences_likes_dislikes() {
    let (processor, track1_id, _track2_id) = setup_history_env().await;
    let stats_repo = processor.history_service().stats_repo();

    // 1. Like track
    processor
        .dispatch_command(Command::LikeTrack {
            track_id: track1_id.clone(),
        })
        .await
        .expect("like track");

    let stats = stats_repo.get_track_stats(&track1_id).await.unwrap().unwrap();
    assert_eq!(stats.manual_like, 1);

    // 2. Dislike track
    processor
        .dispatch_command(Command::DislikeTrack {
            track_id: track1_id.clone(),
        })
        .await
        .expect("dislike track");

    let stats_disliked = stats_repo.get_track_stats(&track1_id).await.unwrap().unwrap();
    assert_eq!(stats_disliked.manual_like, -1);

    // 3. Remove feedback
    processor
        .dispatch_command(Command::RemoveTrackFeedback {
            track_id: track1_id.clone(),
        })
        .await
        .expect("remove feedback");

    let stats_neutral = stats_repo.get_track_stats(&track1_id).await.unwrap().unwrap();
    assert_eq!(stats_neutral.manual_like, 0);
}

#[tokio::test]
async fn test_multi_factor_rankings() {
    let (processor, track1_id, track2_id) = setup_history_env().await;
    let stats_repo = processor.history_service().stats_repo();

    // Directly insert playback history records to test ranking math deterministically
    let now = chrono::Utc::now().timestamp();
    let history_repo = music_player_backend::database::repositories::SqliteHistoryRepository::new(
        processor.db_pool().clone(),
    );

    use music_player_backend::database::repositories::HistoryRepository;

    // Track 1: 5 plays, 1200 seconds total, liked
    for _ in 0..5 {
        history_repo
            .record_playback(&PlaybackHistoryRecord {
                id: Uuid::new_v4().to_string(),
                track_id: track1_id.clone(),
                started_at: now - 3600,
                ended_at: now,
                seconds_listened: 240.0,
                percentage_listened: 1.0,
                completed: 1,
                skipped: 0,
                source: "library".into(),
                playlist_id: None,
                recommendation_session_id: None,
            })
            .await
            .unwrap();

        stats_repo
            .update_track_playback_stats(&track1_id, 240.0, true, true, false)
            .await
            .unwrap();
    }
    stats_repo.set_track_like(&track1_id, 1).await.unwrap();

    // Track 2: 1 play, 180 seconds, skipped 2 times
    history_repo
        .record_playback(&PlaybackHistoryRecord {
            id: Uuid::new_v4().to_string(),
            track_id: track2_id.clone(),
            started_at: now - 1800,
            ended_at: now,
            seconds_listened: 180.0,
            percentage_listened: 1.0,
            completed: 1,
            skipped: 0,
            source: "library".into(),
            playlist_id: None,
            recommendation_session_id: None,
        })
        .await
        .unwrap();

    stats_repo
        .update_track_playback_stats(&track2_id, 180.0, true, true, false)
        .await
        .unwrap();

    // 1. Test Track Rankings across TimeWindow::Today
    let query_res = processor
        .execute_query(Query::GetTopRankings {
            window: TimeWindow::Today,
            entity: RankingEntity::Tracks,
            limit: 10,
        })
        .await
        .expect("ranking query");

    match query_res {
        QueryResponse::Rankings(items) => {
            assert_eq!(items.len(), 2);
            // Track 1 should be ranked #1 with higher score
            let top_track = &items[0];
            assert_eq!(top_track["track_id"].as_str().unwrap(), track1_id);
            assert!(top_track["score"].as_f64().unwrap() > items[1]["score"].as_f64().unwrap());
        }
        _ => panic!("Expected Rankings response"),
    }

    // 2. Test Artist Rankings
    let artist_rankings = processor
        .execute_query(Query::GetTopRankings {
            window: TimeWindow::AllTime,
            entity: RankingEntity::Artists,
            limit: 10,
        })
        .await
        .expect("artist ranking query");

    match artist_rankings {
        QueryResponse::Rankings(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0]["name"].as_str().unwrap(), "The Prodigy");
        }
        _ => panic!("Expected Rankings response"),
    }

    // 3. Test Genre Rankings
    let genre_rankings = processor
        .execute_query(Query::GetTopRankings {
            window: TimeWindow::AllTime,
            entity: RankingEntity::Genres,
            limit: 10,
        })
        .await
        .expect("genre ranking query");

    match genre_rankings {
        QueryResponse::Rankings(items) => {
            assert_eq!(items.len(), 2);
            assert_eq!(items[0]["name"].as_str().unwrap(), "Big Beat");
        }
        _ => panic!("Expected Rankings response"),
    }
}
