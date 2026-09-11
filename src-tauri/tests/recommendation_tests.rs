use music_player_backend::config::AppConfig;
use music_player_backend::core::command::{Command, CommandResponse, SmartMixType};
use music_player_backend::core::processor::CoreProcessor;
use music_player_backend::core::query::{Query, QueryResponse};
use music_player_backend::database::create_in_memory_pool;
use music_player_backend::database::models::PlaybackHistoryRecord;
use music_player_backend::database::repositories::{HistoryRepository, ScannedMetadata};
use music_player_backend::playback::backend::MockAudioBackend;
use music_player_backend::recommendations::scoring::{CandidateTrack, ScoringEngine};
use music_player_backend::recommendations::taste::TasteProfile;
use uuid::Uuid;

async fn setup_test_library(processor: &CoreProcessor) -> (Vec<String>, Vec<String>) {
    let track_repo = processor.library_service().track_repo();

    let mut pink_floyd_ids = Vec::new();
    let mut other_ids = Vec::new();

    // 3 Pink Floyd tracks (Rock, 1970s)
    for i in 1..=3 {
        let id = track_repo
            .save_scanned_track(ScannedMetadata {
                file_path: format!("/music/pink_floyd_{}.flac", i),
                file_size: 2000,
                modified_timestamp: 100,
                file_hash: None,
                title: format!("Comfortably Numb Pt {}", i),
                artist: Some("Pink Floyd".into()),
                album: Some("The Wall".into()),
                album_artist: None,
                genre: Some("Rock".into()),
                track_number: Some(i as u32),
                disc_number: Some(1),
                year: Some(1979),
                duration_secs: 360.0,
                bitrate: Some(1000),
                sample_rate: Some(44100),
                format: "flac".into(),
                has_cover_art: true,
                musicbrainz_track_id: None,
            })
            .await
            .expect("save pink floyd track");
        pink_floyd_ids.push(id);
    }

    // 2 Daft Punk tracks (Electronic, 2000s)
    for i in 1..=2 {
        let id = track_repo
            .save_scanned_track(ScannedMetadata {
                file_path: format!("/music/daft_punk_{}.mp3", i),
                file_size: 1500,
                modified_timestamp: 100,
                file_hash: None,
                title: format!("One More Time {}", i),
                artist: Some("Daft Punk".into()),
                album: Some("Discovery".into()),
                album_artist: None,
                genre: Some("Electronic".into()),
                track_number: Some(i as u32),
                disc_number: Some(1),
                year: Some(2001),
                duration_secs: 300.0,
                bitrate: Some(320),
                sample_rate: Some(44100),
                format: "mp3".into(),
                has_cover_art: false,
                musicbrainz_track_id: None,
            })
            .await
            .expect("save daft punk track");
        other_ids.push(id);
    }

    // 1 Disliked track (Noise, 2020s)
    let disliked_id = track_repo
        .save_scanned_track(ScannedMetadata {
            file_path: "/music/annoying_noise.mp3".into(),
            file_size: 500,
            modified_timestamp: 100,
            file_hash: None,
            title: "Harsh Static".into(),
            artist: Some("Annoying Artist".into()),
            album: Some("Static Sounds".into()),
            album_artist: None,
            genre: Some("Noise".into()),
            track_number: Some(1),
            disc_number: Some(1),
            year: Some(2023),
            duration_secs: 60.0,
            bitrate: Some(128),
            sample_rate: Some(44100),
            format: "mp3".into(),
            has_cover_art: false,
            musicbrainz_track_id: None,
        })
        .await
        .expect("save noise track");
    other_ids.push(disliked_id);

    (pink_floyd_ids, other_ids)
}

#[tokio::test]
async fn test_taste_profile_generation() {
    let pool = create_in_memory_pool().await.expect("DB pool");
    let mock_backend = Box::new(MockAudioBackend::new());
    let processor = CoreProcessor::new_with_backend(pool, AppConfig::default_with_dirs(), mock_backend);

    let (pink_floyd_ids, other_ids) = setup_test_library(&processor).await;
    let now = chrono::Utc::now().timestamp();

    // Log 3 completed listens for Pink Floyd
    let history_repo = music_player_backend::database::repositories::SqliteHistoryRepository::new(processor.db_pool().clone());
    for id in &pink_floyd_ids {
        history_repo
            .record_playback(&PlaybackHistoryRecord {
                id: Uuid::new_v4().to_string(),
                track_id: id.clone(),
                started_at: now - 3600,
                ended_at: now - 3240,
                seconds_listened: 360.0,
                percentage_listened: 100.0,
                completed: 1,
                skipped: 0,
                source: "test".into(),
                playlist_id: None,
                recommendation_session_id: None,
            })
            .await
            .expect("record history");
    }

    // Explicit like on Pink Floyd track
    processor
        .dispatch_command(Command::LikeTrack {
            track_id: pink_floyd_ids[0].clone(),
        })
        .await
        .expect("like track");

    // Explicit dislike on noise track
    let noise_id = other_ids.last().unwrap();
    processor
        .dispatch_command(Command::DislikeTrack {
            track_id: noise_id.clone(),
        })
        .await
        .expect("dislike track");

    // Query taste profile
    let response = processor
        .execute_query(Query::GetTasteProfile)
        .await
        .expect("taste profile query");

    if let QueryResponse::TasteProfile(val) = response {
        let profile: music_player_backend::recommendations::taste::TasteProfile =
            serde_json::from_value(val).expect("parse taste profile");

        // Assert Pink Floyd is #1 top artist
        assert!(!profile.top_artists.is_empty(), "Top artists should not be empty");
        assert_eq!(profile.top_artists[0].display_name, "Pink Floyd");
        assert!(profile.top_artists[0].affinity > 0.5, "Pink Floyd affinity should be high");

        // Assert Rock is #1 top genre
        assert!(!profile.top_genres.is_empty(), "Top genres should not be empty");
        assert_eq!(profile.top_genres[0].display_name, "Rock");

        // Assert 1970s era was recorded
        assert!(profile.era_affinities.contains_key("1970s"));
    } else {
        panic!("Expected QueryResponse::TasteProfile");
    }
}

#[tokio::test]
async fn test_offline_local_recommendations_and_diversity() {
    let pool = create_in_memory_pool().await.expect("DB pool");
    let mock_backend = Box::new(MockAudioBackend::new());
    let processor = CoreProcessor::new_with_backend(pool, AppConfig::default_with_dirs(), mock_backend);

    let (pink_floyd_ids, other_ids) = setup_test_library(&processor).await;
    let now = chrono::Utc::now().timestamp();

    // Log listens for Pink Floyd
    let history_repo = music_player_backend::database::repositories::SqliteHistoryRepository::new(processor.db_pool().clone());
    history_repo
        .record_playback(&PlaybackHistoryRecord {
            id: Uuid::new_v4().to_string(),
            track_id: pink_floyd_ids[0].clone(),
            started_at: now - 3600,
            ended_at: now - 3240,
            seconds_listened: 360.0,
            percentage_listened: 100.0,
            completed: 1,
            skipped: 0,
            source: "test".into(),
            playlist_id: None,
            recommendation_session_id: None,
        })
        .await
        .expect("record history");

    // Like first Pink Floyd track
    processor
        .dispatch_command(Command::LikeTrack {
            track_id: pink_floyd_ids[0].clone(),
        })
        .await
        .expect("like track");

    // Dislike noise track
    let noise_id = other_ids.last().unwrap();
    processor
        .dispatch_command(Command::DislikeTrack {
            track_id: noise_id.clone(),
        })
        .await
        .expect("dislike noise track");

    // Fetch local recommendations
    let response = processor
        .execute_query(Query::GetLocalRecommendations { limit: 10 })
        .await
        .expect("recommendations query");

    if let QueryResponse::Recommendations(items) = response {
        assert!(!items.is_empty(), "Recommendations should not be empty");

        // Check for disliked track exclusion
        for item in &items {
            let track_id = item["track_id"].as_str().unwrap();
            assert_ne!(track_id, noise_id, "Disliked track must NEVER be recommended");

            // Verify score and explanations exist
            let score = item["score"].as_f64().unwrap();
            assert!(score > 0.0, "Score must be positive");

            let reasons = item["reasons"].as_array().expect("reasons array");
            assert!(!reasons.is_empty(), "Explainability reasons must be provided");
        }

        // Check artist diversity: Pink Floyd tracks should be capped at 2
        let pink_floyd_recs = items
            .iter()
            .filter(|i| i["artist_name"].as_str() == Some("Pink Floyd"))
            .count();
        assert!(
            pink_floyd_recs <= 2,
            "Artist diversity cap: max 2 tracks per artist, got {}",
            pink_floyd_recs
        );
    } else {
        panic!("Expected QueryResponse::Recommendations");
    }
}

#[tokio::test]
async fn test_smart_mix_generation_and_persistence() {
    let pool = create_in_memory_pool().await.expect("DB pool");
    let mock_backend = Box::new(MockAudioBackend::new());
    let processor = CoreProcessor::new_with_backend(pool, AppConfig::default_with_dirs(), mock_backend);

    let (pink_floyd_ids, _other_ids) = setup_test_library(&processor).await;
    let now = chrono::Utc::now().timestamp();

    // Add some history
    let history_repo = music_player_backend::database::repositories::SqliteHistoryRepository::new(processor.db_pool().clone());
    for id in &pink_floyd_ids {
        history_repo
            .record_playback(&PlaybackHistoryRecord {
                id: Uuid::new_v4().to_string(),
                track_id: id.clone(),
                started_at: now - 1000,
                ended_at: now - 800,
                seconds_listened: 200.0,
                percentage_listened: 90.0,
                completed: 1,
                skipped: 0,
                source: "test".into(),
                playlist_id: None,
                recommendation_session_id: None,
            })
            .await
            .expect("record history");
    }

    // 1. Generate Daily Mix
    let daily_res = processor
        .dispatch_command(Command::GenerateSmartMix {
            mix_type: SmartMixType::Daily,
        })
        .await
        .expect("generate daily mix");

    let daily_playlist_id = match daily_res {
        CommandResponse::MixGenerated { playlist_id, track_count } => {
            assert!(track_count > 0, "Daily mix should contain tracks");
            playlist_id
        }
        _ => panic!("Expected CommandResponse::MixGenerated"),
    };

    // 2. Generate On Repeat Mix
    let on_repeat_res = processor
        .dispatch_command(Command::GenerateSmartMix {
            mix_type: SmartMixType::OnRepeat,
        })
        .await
        .expect("generate on repeat mix");

    let on_repeat_playlist_id = match on_repeat_res {
        CommandResponse::MixGenerated { playlist_id, track_count } => {
            assert!(track_count > 0, "On Repeat mix should contain tracks");
            playlist_id
        }
        _ => panic!("Expected CommandResponse::MixGenerated"),
    };

    // 3. Query GetSmartMixes
    let mixes_res = processor
        .execute_query(Query::GetSmartMixes)
        .await
        .expect("get smart mixes query");

    if let QueryResponse::SmartMixes(mixes) = mixes_res {
        assert!(mixes.len() >= 2, "Should contain at least 2 smart mixes");
        let ids: Vec<&str> = mixes.iter().filter_map(|m| m["id"].as_str()).collect();
        assert!(ids.contains(&daily_playlist_id.as_str()));
        assert!(ids.contains(&on_repeat_playlist_id.as_str()));
    } else {
        panic!("Expected QueryResponse::SmartMixes");
    }

    // 4. Query GetPlaylistTracks for daily mix
    let tracks_res = processor
        .execute_query(Query::GetPlaylistTracks {
            playlist_id: daily_playlist_id,
        })
        .await
        .expect("get playlist tracks query");

    if let QueryResponse::PlaylistTracks(tracks) = tracks_res {
        assert!(!tracks.is_empty(), "Playlist tracks should not be empty");
    } else {
        panic!("Expected QueryResponse::PlaylistTracks");
    }
}

#[test]
fn test_scoring_repetition_penalty_and_discovery_bonus() {
    let now = 1_000_000;
    let mut profile = TasteProfile::default();
    profile.artist_affinities.insert("artist_pink_floyd".to_string(), 0.90);
    profile.genre_affinities.insert("Rock".to_string(), 0.80);

    // Case 1: Freshly played candidate (< 24 hours ago) -> should have -50% repetition penalty
    let recent_candidate = CandidateTrack {
        id: "track_1".to_string(),
        title: "Time".to_string(),
        artist_id: Some("artist_pink_floyd".to_string()),
        artist_name: Some("Pink Floyd".to_string()),
        genre_name: Some("Rock".to_string()),
        year: Some(1973),
        play_count: 5,
        completion_count: 5,
        skip_count: 0,
        last_played_at: Some(now - 3600), // 1 hour ago
        user_feedback: None,
    };

    let score_recent = ScoringEngine::score_track(&recent_candidate, &profile, now);
    let has_penalty = score_recent.reasons.iter().any(|r| r.factor == "repetition_penalty");
    assert!(has_penalty, "Recent candidate must have repetition penalty");

    // Case 2: Unplayed track matching artist/genre -> should have exploration bonus and is_discovery = true
    let discovery_candidate = CandidateTrack {
        id: "track_2".to_string(),
        title: "Echoes".to_string(),
        artist_id: Some("artist_pink_floyd".to_string()),
        artist_name: Some("Pink Floyd".to_string()),
        genre_name: Some("Rock".to_string()),
        year: Some(1971),
        play_count: 0,
        completion_count: 0,
        skip_count: 0,
        last_played_at: None,
        user_feedback: None,
    };

    let score_discovery = ScoringEngine::score_track(&discovery_candidate, &profile, now);
    assert!(score_discovery.is_discovery, "Unplayed high-affinity candidate should be marked as discovery");
    let has_discovery_reason = score_discovery.reasons.iter().any(|r| r.factor == "exploration_pick");
    assert!(has_discovery_reason, "Discovery pick explanation must be present in reasons");
}
