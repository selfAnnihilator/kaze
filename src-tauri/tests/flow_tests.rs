use music_player_backend::config::AppConfig;
use music_player_backend::core::command::{Command, CommandResponse};
use music_player_backend::core::processor::CoreProcessor;
use music_player_backend::core::query::{Query, QueryResponse};
use music_player_backend::database::create_in_memory_pool;
use music_player_backend::database::models::PlaybackHistoryRecord;
use music_player_backend::database::repositories::{
    HistoryRepository, ScannedMetadata, SqliteHistoryRepository,
};
use music_player_backend::playback::backend::MockAudioBackend;
use uuid::Uuid;

async fn setup_processor() -> CoreProcessor {
    let pool = create_in_memory_pool()
        .await
        .expect("create in-memory pool");
    sqlx::query("INSERT OR REPLACE INTO application_settings(key,value,updated_at) VALUES('enable_jiosaavn','false',0)").execute(&pool).await.unwrap();
    let config = fixture_config();
    let backend = Box::new(MockAudioBackend::new());
    CoreProcessor::new_with_backend(pool, config, backend)
}

async fn add_track(
    processor: &CoreProcessor,
    title: &str,
    artist: &str,
    album: &str,
    genre: &str,
    year: u32,
) -> String {
    let track_repo = processor.library_service().track_repo();
    track_repo
        .save_scanned_track(ScannedMetadata {
            title_is_fallback: false,
            file_path: format!(
                "/music/{}_{}.flac",
                artist.to_lowercase().replace(' ', "_"),
                title.to_lowercase().replace(' ', "_")
            ),
            file_size: 2000,
            modified_timestamp: 100,
            file_hash: None,
            title: title.to_string(),
            artist: Some(artist.to_string()),
            album: Some(album.to_string()),
            album_artist: None,
            genre: Some(genre.to_string()),
            track_number: Some(1),
            disc_number: Some(1),
            year: Some(year),
            duration_secs: 240.0,
            bitrate: Some(1000),
            sample_rate: Some(44100),
            format: "flac".to_string(),
            has_cover_art: true,
            musicbrainz_track_id: None,
        })
        .await
        .expect("save track")
}

#[tokio::test]
async fn test_flow_empty_library() {
    let processor = setup_processor().await;
    let flow_engine = processor.flow_engine();

    // 1. Generate flow on empty library returns empty without crashing
    let tracks = flow_engine
        .generate_flow_tracks("default", 0.5, 10, &[])
        .await
        .unwrap();
    assert!(tracks.is_empty());

    // 2. Home overview returns safe structure
    let overview = flow_engine.get_home_overview("default").await.unwrap();
    assert!(overview.recent_rotation.is_empty());
    assert!(overview.rediscover.is_empty());
    assert!(overview.for_you.is_empty());

    // 3. StartFlow on empty library returns clean error
    let res = processor
        .dispatch_command(Command::StartFlow {
            familiarity_ratio: Some(0.5),
        })
        .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_flow_library_no_history() {
    let processor = setup_processor().await;
    let flow_engine = processor.flow_engine();

    // Add 5 tracks across 3 artists
    let _t1 = add_track(
        &processor,
        "Track A1",
        "Artist One",
        "Album A",
        "Rock",
        2020,
    )
    .await;
    let _t2 = add_track(
        &processor,
        "Track A2",
        "Artist One",
        "Album A",
        "Rock",
        2020,
    )
    .await;
    let _t3 = add_track(
        &processor,
        "Track B1",
        "Artist Two",
        "Album B",
        "Jazz",
        2018,
    )
    .await;
    let _t4 = add_track(
        &processor,
        "Track C1",
        "Artist Three",
        "Album C",
        "Ambient",
        2022,
    )
    .await;
    let _t5 = add_track(
        &processor,
        "Track C2",
        "Artist Three",
        "Album C",
        "Ambient",
        2022,
    )
    .await;

    // Generate flow tracks
    let tracks = flow_engine
        .generate_flow_tracks("default", 0.5, 5, &[])
        .await
        .unwrap();
    assert_eq!(tracks.len(), 5);

    // Verify each track has valid score and reasons
    for t in &tracks {
        assert!(t.score > 0.0);
        assert!(!t.reason.is_empty());
        assert_eq!(t.reason_category, "library_exploration");
        assert!(t.explanation.is_unplayed);
        assert_eq!(t.explanation.days_since_played, None);
        assert_eq!(t.explanation.familiarity_ratio, Some(0.5));
        assert!(t.explanation.selected_at.is_some());
    }

    // Starting flow works and sets queue
    let start_res = processor
        .dispatch_command(Command::StartFlow {
            familiarity_ratio: Some(0.5),
        })
        .await;
    assert!(matches!(start_res, Ok(CommandResponse::Ok)));

    // Verify session was persisted
    let session = flow_engine.get_flow_session("default").await.unwrap();
    assert!(session.is_some());
    let s = session.unwrap();
    assert!(!s.is_active, "persist settings, not live playback state");
    let live = processor
        .execute_query(Query::GetFlowSession)
        .await
        .unwrap();
    assert!(matches!(live, QueryResponse::FlowSession(Some(ref v)) if v["is_active"] == true));
    assert_eq!(s.familiarity_ratio, 0.5);
}

#[tokio::test]
async fn test_flow_mature_history_steering_and_rediscovery() {
    let processor = setup_processor().await;
    let flow_engine = processor.flow_engine();
    let history_repo = SqliteHistoryRepository::new(processor.db_pool().clone());

    let now = chrono::Utc::now().timestamp();

    // 1. Frequently played track (Pink Floyd)
    let pf1 = add_track(
        &processor,
        "Comfortably Numb",
        "Pink Floyd",
        "The Wall",
        "Rock",
        1979,
    )
    .await;
    for i in 0..10 {
        history_repo
            .record_playback(&PlaybackHistoryRecord {
                id: Uuid::new_v4().to_string(),
                user_id: "default".into(),
                track_id: pf1.clone(),
                started_at: now - (5 * 86400) - (i * 3600),
                ended_at: now - (5 * 86400) - (i * 3600) + 240,
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
    }
    sqlx::query(
        "INSERT INTO track_statistics (user_id, track_id, play_count, total_time_listened, completion_count, skip_count, last_played_at, manual_like, playlist_addition_count)
         VALUES ('default', ?, 10, 2400.0, 10, 0, ?, 0, 0)"
    )
    .bind(&pf1)
    .bind(now - (5 * 86400))
    .execute(processor.db_pool())
    .await
    .unwrap();

    // 2. Forgotten favorite (Daft Punk) - listened 100 days ago, never since
    let dp1 = add_track(
        &processor,
        "One More Time",
        "Daft Punk",
        "Discovery",
        "Electronic",
        2001,
    )
    .await;
    for i in 0..5 {
        history_repo
            .record_playback(&PlaybackHistoryRecord {
                id: Uuid::new_v4().to_string(),
                user_id: "default".into(),
                track_id: dp1.clone(),
                started_at: now - (100 * 86400) - (i * 3600),
                ended_at: now - (100 * 86400) - (i * 3600) + 240,
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
    }
    sqlx::query(
        "INSERT INTO track_statistics (user_id, track_id, play_count, total_time_listened, completion_count, skip_count, last_played_at, manual_like, playlist_addition_count)
         VALUES ('default', ?, 5, 1200.0, 5, 0, ?, 0, 0)"
    )
    .bind(&dp1)
    .bind(now - (100 * 86400))
    .execute(processor.db_pool())
    .await
    .unwrap();

    // 3. Liked track (Radiohead)
    let rh1 = add_track(
        &processor,
        "Karma Police",
        "Radiohead",
        "OK Computer",
        "Alternative",
        1997,
    )
    .await;
    processor
        .dispatch_command(Command::LikeTrack {
            track_id: rh1.clone(),
        })
        .await
        .unwrap();

    // 4. Test Familiar steering (ratio = 1.0):
    // Should favor known favorites and liked tracks
    let familiar_tracks = flow_engine
        .generate_flow_tracks("default", 1.0, 3, &[])
        .await
        .unwrap();
    assert!(!familiar_tracks.is_empty());
    // Radiohead (liked) or Pink Floyd (high affinity) should score at or near top
    let top_ids: Vec<&str> = familiar_tracks
        .iter()
        .map(|t| t.track_id.as_str())
        .collect();
    assert!(top_ids.contains(&rh1.as_str()) || top_ids.contains(&pf1.as_str()));

    // 5. Test Rediscover steering (ratio = 0.0):
    // Daft Punk (played 100 days ago) should have high rediscovery score
    let rediscover_tracks = flow_engine
        .generate_flow_tracks("default", 0.0, 3, &[])
        .await
        .unwrap();
    let dp_item = rediscover_tracks.iter().find(|t| t.track_id == dp1);
    assert!(dp_item.is_some());
    let dp = dp_item.unwrap();
    assert!(dp.reason.contains("days") || dp.reason_category == "rediscovery");

    // 6. Test Why This Track explanation
    let why_liked = flow_engine
        .get_why_this_track("default", &rh1)
        .await
        .unwrap();
    assert_eq!(why_liked, "One of your liked tracks");

    let why_dp = flow_engine
        .get_why_this_track("default", &dp1)
        .await
        .unwrap();
    assert!(why_dp.contains("days"));

    // 7. Test HomeOverview composite query
    let home_res = processor
        .execute_query(Query::GetHomeOverview)
        .await
        .unwrap();
    if let QueryResponse::HomeOverview(v) = home_res {
        assert!(v.get("recent_rotation").is_some());
        assert!(v.get("rediscover").is_some());
        assert!(v.get("for_you").is_some());
    } else {
        panic!("Expected QueryResponse::HomeOverview");
    }
}

#[tokio::test]
async fn flow_preview_is_read_only_local_deterministic_and_bounded() {
    let processor = setup_processor().await;
    let fe = processor.flow_engine();
    let a = add_track(&processor, "A", "One", "Album", "Rock", 2020).await;
    let b = add_track(&processor, "B", "Two", "Album", "Jazz", 2020).await;
    let online = add_track(&processor, "Remote", "Remote", "Remote", "Pop", 2020).await;
    sqlx::query("UPDATE tracks SET format = 'online', file_path = 'online://remote' WHERE id = ?")
        .bind(&online)
        .execute(processor.db_pool())
        .await
        .unwrap();
    let first = fe
        .generate_flow_tracks("default", 0.5, 10000, &[])
        .await
        .unwrap();
    let second = fe
        .generate_flow_tracks("default", 0.5, 10000, &[])
        .await
        .unwrap();
    assert_eq!(
        first.iter().map(|t| &t.track_id).collect::<Vec<_>>(),
        second.iter().map(|t| &t.track_id).collect::<Vec<_>>()
    );
    assert_eq!(first.len(), 2);
    assert!(first.iter().all(|t| t.track_id == a || t.track_id == b));
    assert!(fe
        .generate_flow_tracks("default", 0.5, 0, &[])
        .await
        .unwrap()
        .is_empty());
    assert!(fe
        .generate_flow_tracks("default", f64::NAN, 3, &[])
        .await
        .is_err());
    fe.get_home_overview("default").await.unwrap();
    processor
        .dispatch_command(Command::SetFlowSteering {
            familiarity_ratio: 0.8,
        })
        .await
        .unwrap();
    for table in [
        "playback_history",
        "track_statistics",
        "active_playback_journal",
        "recommendation_sessions",
    ] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(processor.db_pool())
            .await
            .unwrap();
        assert_eq!(count, 0, "preview/settings must not write {table}");
    }
}

#[tokio::test]
async fn flow_stop_and_manual_selection_preserve_manual_queue() {
    let processor = setup_processor().await;
    for i in 0..5 {
        add_track(
            &processor,
            &format!("Flow {i}"),
            &format!("Artist {i}"),
            "Album",
            "Rock",
            2020,
        )
        .await;
    }
    processor
        .dispatch_command(Command::StartFlow {
            familiarity_ratio: Some(0.5),
        })
        .await
        .unwrap();
    // Explicit choices made after Start survive Stop; pre-Start entries do not.
    let manual = add_track(&processor, "Manual", "Manual", "Manual", "Jazz", 2020).await;
    processor
        .dispatch_command(Command::EnqueueTrack {
            track_id: manual.clone(),
            play_next: false,
            manual: false,
        })
        .await
        .unwrap();
    processor.dispatch_command(Command::StopFlow).await.unwrap();
    if let QueryResponse::PlaybackState(state) = processor
        .execute_query(Query::GetPlaybackState)
        .await
        .unwrap()
    {
        assert_eq!(
            state["queue_track_ids"],
            serde_json::json!([manual.clone()])
        );
    } else {
        panic!("playback state expected");
    }
    processor
        .dispatch_command(Command::StartFlow {
            familiarity_ratio: Some(0.5),
        })
        .await
        .unwrap();
    processor
        .dispatch_command(Command::PlayTrack {
            track_id: manual,
            source: None,
        })
        .await
        .unwrap();
    if let QueryResponse::FlowSession(Some(state)) = processor
        .execute_query(Query::GetFlowSession)
        .await
        .unwrap()
    {
        assert_eq!(state["is_active"], false);
    } else {
        panic!("session expected");
    }
}

#[tokio::test]
async fn continuing_flow_does_not_start_duplicate_playback_session() {
    let processor = setup_processor().await;
    for i in 0..4 {
        add_track(
            &processor,
            &format!("Track {i}"),
            &format!("Artist {i}"),
            "Album",
            "Rock",
            2020,
        )
        .await;
    }
    let mut receiver = processor.event_bus().subscribe();
    processor
        .dispatch_command(Command::StartFlow {
            familiarity_ratio: Some(0.5),
        })
        .await
        .unwrap();
    let service = processor.playback_service();
    let queue_before = service.get_playback_state().await.queue_track_ids;
    let session_before = serde_json::to_value(
        service
            .flow_session(&processor.flow_engine(), "default")
            .await
            .unwrap(),
    )
    .unwrap();
    processor.dispatch_command(Command::Pause).await.unwrap();
    processor
        .dispatch_command(Command::ContinueFlow)
        .await
        .unwrap();
    assert_eq!(
        service.get_playback_state().await.queue_track_ids,
        queue_before
    );
    assert_eq!(
        serde_json::to_value(
            service
                .flow_session(&processor.flow_engine(), "default")
                .await
                .unwrap()
        )
        .unwrap(),
        session_before
    );
    let mut starts = 0;
    while let Ok(event) = receiver.try_recv() {
        if matches!(
            event,
            music_player_backend::core::event::Event::PlaybackStarted { .. }
        ) {
            starts += 1;
        }
    }
    assert_eq!(starts, 1);
}

#[tokio::test]
async fn refill_uses_runtime_user_and_steering_and_never_random_disliked_tracks() {
    let processor = setup_processor().await;
    let first = add_track(&processor, "First", "One", "A", "Rock", 2020).await;
    let second = add_track(&processor, "Second", "Two", "B", "Jazz", 2020).await;
    let fe = processor.flow_engine();
    let playback = processor.playback_service();
    // Another account has opposite feedback. A hardcoded default refill would pick First.
    sqlx::query(
        "INSERT INTO track_statistics (user_id, track_id, manual_like) VALUES ('account', ?, -1)",
    )
    .bind(&first)
    .execute(processor.db_pool())
    .await
    .unwrap();
    playback
        .start_flow(&fe, "account", Some(0.0))
        .await
        .unwrap();
    assert_eq!(
        playback
            .get_playback_state()
            .await
            .current_track_id
            .as_deref(),
        Some(second.as_str())
    );
    playback
        .set_flow_steering(&fe, "account", 0.9)
        .await
        .unwrap();
    assert_eq!(
        playback
            .flow_session(&fe, "account")
            .await
            .unwrap()
            .unwrap()
            .familiarity_ratio,
        0.9
    );
    playback.next().await.unwrap();
    assert!(
        !playback.get_playback_state().await.is_playing,
        "single eligible track exhaustion must not play disliked/random tracks"
    );
}

#[tokio::test]
async fn steering_changes_order_and_explanations_are_proven() {
    let processor = setup_processor().await;
    let familiar = add_track(&processor, "Liked", "One", "A", "Rock", 2020).await;
    let forgotten = add_track(&processor, "Old", "Two", "B", "Jazz", 2020).await;
    let now = chrono::Utc::now().timestamp();
    sqlx::query("INSERT INTO track_statistics (user_id, track_id, manual_like, play_count, completion_count, last_played_at) VALUES ('default', ?, 1, 10, 10, ?), ('default', ?, 0, 1, 1, ?)")
        .bind(&familiar).bind(now - 10 * 86400).bind(&forgotten).bind(now - 100 * 86400)
        .execute(processor.db_pool()).await.unwrap();
    let fe = processor.flow_engine();
    assert_eq!(
        fe.generate_flow_tracks("default", 1.0, 2, &[])
            .await
            .unwrap()[0]
            .track_id,
        familiar
    );
    assert_eq!(
        fe.generate_flow_tracks("default", 0.0, 2, &[])
            .await
            .unwrap()[0]
            .track_id,
        forgotten
    );
    let reason = fe.get_why_this_track("default", &forgotten).await.unwrap();
    assert!(reason.contains("100 days"));
    assert!(!reason.contains("favorite"));
    let evidence = fe
        .get_track_explanation("default", &forgotten)
        .await
        .unwrap();
    assert_eq!(evidence.primary, "rediscovery");
    assert_eq!(evidence.days_since_played, Some(100));
    assert!(!evidence.is_liked);
    assert!(!evidence.is_unplayed);
    let liked = fe
        .get_track_explanation("default", &familiar)
        .await
        .unwrap();
    assert_eq!(liked.primary, "liked");
    assert!(liked.is_liked);
    assert_eq!(liked.days_since_played, Some(10));
    assert!(fe.get_why_this_track("default", "missing").await.is_err());
}

#[tokio::test]
async fn exhausted_flow_refill_honors_new_steering() {
    let processor = setup_processor().await;
    add_track(&processor, "Initial", "Initial", "A", "Rock", 2020).await;
    processor
        .dispatch_command(Command::StartFlow {
            familiarity_ratio: Some(0.0),
        })
        .await
        .unwrap();
    let liked = add_track(&processor, "Liked refill", "Liked", "B", "Jazz", 2020).await;
    let old = add_track(&processor, "Old refill", "Old", "C", "Pop", 2020).await;
    sqlx::query("INSERT INTO track_statistics (user_id, track_id, manual_like, play_count, completion_count, last_played_at) VALUES ('default', ?, 1, 10, 10, ?), ('default', ?, 0, 1, 1, ?)")
        .bind(&liked).bind(chrono::Utc::now().timestamp() - 10 * 86400)
        .bind(&old).bind(chrono::Utc::now().timestamp() - 100 * 86400)
        .execute(processor.db_pool()).await.unwrap();
    processor
        .dispatch_command(Command::SetFlowSteering {
            familiarity_ratio: 1.0,
        })
        .await
        .unwrap();
    processor.playback_service().next().await.unwrap();
    assert_eq!(
        processor
            .playback_service()
            .get_playback_state()
            .await
            .current_track_id,
        Some(liked)
    );
}

#[tokio::test]
async fn large_library_rapid_skips_refill_without_duplicate_sessions_or_upcoming_tracks() {
    use music_player_backend::core::event::Event;
    use std::collections::HashSet;
    let processor = setup_processor().await;
    for i in 0..60 {
        add_track(
            &processor,
            &format!("Track {i}"),
            &format!("Artist {i}"),
            "Album",
            "Rock",
            2020,
        )
        .await;
    }
    let mut receiver = processor.event_bus().subscribe();
    processor
        .dispatch_command(Command::StartFlow {
            familiarity_ratio: Some(0.5),
        })
        .await
        .unwrap();
    let playback = processor.playback_service();
    for _ in 0..10 {
        let (a, b, c) = tokio::join!(playback.next(), playback.next(), playback.next());
        a.unwrap();
        b.unwrap();
        c.unwrap();
    }
    let state = playback.get_playback_state().await;
    assert!(state.is_playing);
    assert_eq!(
        state.queue_track_ids.iter().collect::<HashSet<_>>().len(),
        state.queue_track_ids.len()
    );
    let mut sessions = HashSet::new();
    let mut played = HashSet::new();
    while let Ok(event) = receiver.try_recv() {
        if let Event::PlaybackStarted {
            session_id,
            track_id,
            ..
        } = event
        {
            assert!(sessions.insert(session_id), "duplicate actual session");
            assert!(
                played.insert(track_id),
                "avoid replay across refill when enough library tracks exist"
            );
        }
    }
    assert!(state.queue_length > 20, "must exercise refill");
    assert!(played.len() > 1);
    // Superseded rapid skips need not start audio or create sessions.
}

#[tokio::test]
async fn artist_spacing_spans_batches_when_alternatives_exist() {
    let processor = setup_processor().await;
    for i in 0..4 {
        add_track(&processor, &format!("One {i}"), "One", "A", "Rock", 2020).await;
        add_track(&processor, &format!("Two {i}"), "Two", "B", "Jazz", 2020).await;
    }
    let tracks = processor
        .flow_engine()
        .generate_flow_tracks_after("default", 0.5, 8, &[], &["One".to_string()])
        .await
        .unwrap();
    assert_eq!(tracks[0].artist, "Two");
    assert!(tracks
        .windows(2)
        .all(|pair| pair[0].artist != pair[1].artist));
}

#[tokio::test]
async fn natural_completion_advances_flow_with_the_real_session_identity() {
    use music_player_backend::core::event::Event;
    let pool = create_in_memory_pool().await.unwrap();
    let backend = MockAudioBackend::new();
    let processor = CoreProcessor::new_with_backend(
        pool,
        fixture_config(),
        Box::new(backend.clone()),
    );
    for i in 0..3 {
        add_track(
            &processor,
            &format!("Track {i}"),
            &format!("Artist {i}"),
            "Album",
            "Rock",
            2020,
        )
        .await;
    }
    let mut receiver = processor.event_bus().subscribe();
    processor
        .dispatch_command(Command::StartFlow {
            familiarity_ratio: Some(0.5),
        })
        .await
        .unwrap();
    let initial = processor
        .playback_service()
        .get_playback_state()
        .await
        .current_track_id
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    backend.set_finished(true);
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_ne!(
        processor
            .playback_service()
            .get_playback_state()
            .await
            .current_track_id,
        Some(initial.clone())
    );
    let mut initial_session = None;
    let mut completed_session = None;
    while let Ok(event) = receiver.try_recv() {
        match event {
            Event::PlaybackStarted {
                track_id,
                session_id,
                ..
            } if track_id == initial => initial_session = Some(session_id),
            Event::TrackFinished {
                track_id,
                session_id,
                completed,
                ..
            } => {
                assert_eq!(track_id, initial);
                assert!(completed);
                completed_session = Some(session_id);
            }
            _ => {}
        }
    }
    assert!(initial_session.is_some());
    assert_eq!(initial_session, completed_session);
}

#[tokio::test]
async fn why_distinguishes_manual_queue_entries_from_generated_selections() {
    let processor = setup_processor().await;
    let generated = add_track(&processor, "Generated", "One", "A", "Rock", 2020).await;
    processor
        .dispatch_command(Command::StartFlow {
            familiarity_ratio: Some(0.5),
        })
        .await
        .unwrap();
    let service = processor.playback_service();
    assert!(service
        .flow_reason("default", &generated)
        .await
        .unwrap()
        .selected_at
        .is_some());
    let manual = add_track(&processor, "Manual later", "Two", "B", "Jazz", 2020).await;
    service.enqueue_track(&manual, true).await.unwrap();
    service.next().await.unwrap();
    assert!(
        service
            .flow_reason("default", &manual)
            .await
            .unwrap()
            .from_existing_queue
    );
}

#[tokio::test]
async fn explanations_do_not_invent_unheard_status_or_missing_timestamps() {
    let processor = setup_processor().await;
    let skipped = add_track(&processor, "Skipped", "One", "A", "Rock", 2020).await;
    let missing_time = add_track(&processor, "Unknown date", "Two", "B", "Jazz", 2020).await;
    sqlx::query("INSERT INTO track_statistics (user_id, track_id, play_count, skip_count, last_played_at) VALUES ('default', ?, 0, 3, ?), ('default', ?, 2, 0, NULL)")
        .bind(&skipped).bind(chrono::Utc::now().timestamp()).bind(&missing_time)
        .execute(processor.db_pool()).await.unwrap();
    let fe = processor.flow_engine();
    assert!(!fe
        .get_why_this_track("default", &skipped)
        .await
        .unwrap()
        .contains("Unplayed"));
    let home = fe.get_home_overview("default").await.unwrap();
    let reason = &home
        .rediscover
        .iter()
        .find(|t| t.track_id == missing_time)
        .unwrap()
        .reason;
    assert_eq!(reason, "Previously played from your library");
    let missing = fe
        .get_track_explanation("default", &missing_time)
        .await
        .unwrap();
    assert!(!missing.is_unplayed);
    assert_eq!(missing.days_since_played, None);
    assert!(
        !fe.get_track_explanation("default", &skipped)
            .await
            .unwrap()
            .is_unplayed
    );
}

#[tokio::test]
async fn mature_thousand_track_library_produces_bounded_unique_batches() {
    use std::collections::HashSet;
    let processor = setup_processor().await;
    for i in 0..1000 {
        add_track(
            &processor,
            &format!("Track {i}"),
            &format!("Artist {}", i % 40),
            "Album",
            "Rock",
            2020,
        )
        .await;
    }
    let old = chrono::Utc::now().timestamp() - 35 * 86400;
    sqlx::query("INSERT INTO track_statistics (user_id, track_id, play_count, completion_count, last_played_at) SELECT 'default', id, 5, 5, ? FROM tracks")
        .bind(old).execute(processor.db_pool()).await.unwrap();
    sqlx::query("INSERT INTO playback_history (id, user_id, track_id, started_at, ended_at, seconds_listened, percentage_listened, completed, skipped, source) SELECT 'history-' || id, 'default', id, ?, ? + 240, 240, 100, 1, 0, 'library' FROM tracks")
        .bind(old).bind(old).execute(processor.db_pool()).await.unwrap();
    let fe = processor.flow_engine();
    let first = fe
        .generate_flow_tracks("default", 0.5, 20, &[])
        .await
        .unwrap();
    assert_eq!(first.len(), 20);
    let exclude = first.into_iter().map(|t| t.track_id).collect::<Vec<_>>();
    let second = fe
        .generate_flow_tracks("default", 0.5, 20, &exclude)
        .await
        .unwrap();
    assert_eq!(second.len(), 20);
    assert!(second.iter().all(|t| !exclude.contains(&t.track_id)));
    assert_eq!(
        second
            .iter()
            .map(|t| &t.track_id)
            .collect::<HashSet<_>>()
            .len(),
        20
    );
    assert!(second.windows(2).all(|p| p[0].artist != p[1].artist));
    let history: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM playback_history")
        .fetch_one(processor.db_pool())
        .await
        .unwrap();
    assert_eq!(history, 1000, "generation must not add playback history");
}

#[tokio::test]
async fn pause_and_resume_preserve_flow_queue_steering_and_selection_evidence() {
    let processor = setup_processor().await;
    for i in 0..4 {
        add_track(
            &processor,
            &format!("Track {i}"),
            &format!("Artist {i}"),
            "Album",
            "Rock",
            2020,
        )
        .await;
    }
    processor
        .dispatch_command(Command::StartFlow {
            familiarity_ratio: Some(0.25),
        })
        .await
        .unwrap();
    let service = processor.playback_service();
    let before = service.get_playback_state().await;
    let current = before.current_track_id.clone().unwrap();
    let evidence = service.flow_reason("default", &current).await.unwrap();
    let evidence_before = serde_json::to_value(&evidence).unwrap();
    let session_before = serde_json::to_value(
        service
            .flow_session(&processor.flow_engine(), "default")
            .await
            .unwrap(),
    )
    .unwrap();
    processor.dispatch_command(Command::Pause).await.unwrap();
    let paused = service.get_playback_state().await;
    assert!(!paused.is_playing);
    assert_eq!(paused.current_track_id, before.current_track_id);
    assert_eq!(paused.queue_track_ids, before.queue_track_ids);
    assert_eq!(
        serde_json::to_value(
            service
                .flow_session(&processor.flow_engine(), "default")
                .await
                .unwrap()
        )
        .unwrap(),
        session_before
    );
    processor.dispatch_command(Command::Resume).await.unwrap();
    let resumed = service.get_playback_state().await;
    assert!(resumed.is_playing);
    assert_eq!(resumed.current_track_id, before.current_track_id);
    assert_eq!(resumed.queue_track_ids, before.queue_track_ids);
    if let QueryResponse::FlowTrackReason(Some(reason)) = processor
        .execute_query(Query::GetFlowTrackReason { track_id: current })
        .await
        .unwrap()
    {
        assert_eq!(serde_json::to_value(reason).unwrap(), evidence_before);
    } else {
        panic!("expected typed selection evidence");
    }
}

#[tokio::test]
async fn structured_explanation_preserves_artist_affinity_without_frequency_claims() {
    let processor = setup_processor().await;
    let favorite = add_track(&processor, "Favorite", "Same Artist", "A", "Rock", 2020).await;
    let recent = add_track(&processor, "Recent", "Same Artist", "A", "Rock", 2020).await;
    let now = chrono::Utc::now().timestamp();
    sqlx::query("INSERT INTO track_statistics (user_id, track_id, manual_like, play_count, completion_count, last_played_at) VALUES ('default', ?, 1, 5, 5, ?), ('default', ?, 0, 1, 1, ?)")
        .bind(favorite).bind(now - 86400).bind(&recent).bind(now - 86400)
        .execute(processor.db_pool()).await.unwrap();
    let evidence = processor
        .flow_engine()
        .get_track_explanation("default", &recent)
        .await
        .unwrap();
    assert_eq!(evidence.primary, "artist_affinity");
    assert_eq!(evidence.artist_name.as_deref(), Some("Same Artist"));
    assert!(evidence.artist_affinity.unwrap() > 0.3);
    assert_eq!(evidence.play_count, Some(1));
    assert_eq!(evidence.days_since_played, Some(1));
    assert!(!evidence.is_unplayed);
}

#[tokio::test]
async fn new_flow_replaces_old_manual_queue_without_interrupting_current_audio_or_history() {
    use music_player_backend::core::event::Event;
    let processor = setup_processor().await;
    let a = add_track(&processor, "Current", "A", "Album", "Rock", 2020).await;
    let b = add_track(&processor, "Old B", "B", "Album", "Rock", 2020).await;
    let c = add_track(&processor, "Old C", "C", "Album", "Rock", 2020).await;
    let fresh = add_track(&processor, "Fresh", "D", "Album", "Rock", 2020).await;
    // B/C cannot be selected again: this isolates preservation from a legitimate
    // fresh recommendation selecting the same song with a new queue-entry ID.
    for id in [&b, &c] {
        processor
            .dispatch_command(Command::DislikeTrack {
                track_id: id.clone(),
            })
            .await
            .unwrap();
    }
    let service = processor.playback_service();
    processor
        .dispatch_command(Command::PlayTrack {
            track_id: a.clone(),
            source: None,
        })
        .await
        .unwrap();
    service.enqueue_track(&b, false).await.unwrap();
    service.enqueue_track(&c, false).await.unwrap();
    let before = service.get_playback_state().await;
    let mut events = processor.event_bus().subscribe();
    processor
        .dispatch_command(Command::StartFlow {
            familiarity_ratio: Some(0.2),
        })
        .await
        .unwrap();
    let after = service.get_playback_state().await;
    assert!(after.is_playing);
    assert_eq!(after.current_track_id, Some(a.clone()));
    assert_eq!(after.position_secs, before.position_secs);
    assert_eq!(after.queue_track_ids, vec![fresh.clone()]);
    let current_reason = service.flow_reason("default", &a).await.unwrap();
    assert_eq!(current_reason.primary, "continuing_track");
    assert!(!current_reason.from_existing_queue);
    while let Ok(event) = events.try_recv() {
        assert!(!matches!(
            event,
            Event::PlaybackStarted { .. }
                | Event::PlaybackStopped { .. }
                | Event::TrackFinished { .. }
                | Event::PlaybackResumed { .. }
        ));
    }
    // An explicit choice made DURING Flow retains priority and honest provenance.
    let x = add_track(&processor, "Manual X", "X", "Album", "Jazz", 2020).await;
    service.enqueue_track(&x, true).await.unwrap();
    assert_eq!(
        service.get_playback_state().await.queue_track_ids,
        vec![x.clone(), fresh]
    );
    service.next().await.unwrap();
    assert_eq!(
        service.get_playback_state().await.current_track_id,
        Some(x.clone())
    );
    assert!(
        service
            .flow_reason("default", &x)
            .await
            .unwrap()
            .from_existing_queue
    );
    processor.dispatch_command(Command::Pause).await.unwrap();
    let queue = service.get_playback_state().await.queue_track_ids;
    processor
        .dispatch_command(Command::ContinueFlow)
        .await
        .unwrap();
    assert_eq!(service.get_playback_state().await.queue_track_ids, queue);
    assert!(
        service
            .flow_reason("default", &x)
            .await
            .unwrap()
            .from_existing_queue
    );
    // Stop + unrelated manual playback + Start clears that old upcoming sequence.
    processor.dispatch_command(Command::StopFlow).await.unwrap();
    processor
        .dispatch_command(Command::PlayTrack {
            track_id: a,
            source: None,
        })
        .await
        .unwrap();
    service.enqueue_track(&b, false).await.unwrap();
    service.enqueue_track(&c, false).await.unwrap();
    processor
        .dispatch_command(Command::StartFlow {
            familiarity_ratio: None,
        })
        .await
        .unwrap();
    let queue = service.get_playback_state().await.queue_track_ids;
    assert!(!queue.contains(&b));
    assert!(!queue.contains(&c));
}

#[tokio::test]
async fn rapid_start_uses_newest_generation_and_settings() {
    let processor = setup_processor().await;
    for i in 0..8 {
        add_track(
            &processor,
            &format!("Track {i}"),
            &format!("Artist {i}"),
            "Album",
            "Rock",
            2020,
        )
        .await;
    }
    let mut events = processor.event_bus().subscribe();
    let (older, newer) = tokio::join!(
        processor.dispatch_command(Command::StartFlow {
            familiarity_ratio: Some(0.1)
        }),
        processor.dispatch_command(Command::StartFlow {
            familiarity_ratio: Some(0.9)
        })
    );
    older.unwrap();
    newer.unwrap();
    let service = processor.playback_service();
    let session = service
        .flow_session(&processor.flow_engine(), "default")
        .await
        .unwrap()
        .unwrap();
    let mut playback_starts = 0;
    while let Ok(event) = events.try_recv() {
        if matches!(
            event,
            music_player_backend::core::event::Event::PlaybackStarted { .. }
        ) {
            playback_starts += 1;
        }
    }
    assert_eq!(
        playback_starts, 1,
        "superseded generation must not start audio"
    );
    assert_eq!(session.generation, Some(2));
    assert_eq!(session.familiarity_ratio, 0.9);
    let state = service.get_playback_state().await;
    let reason = service
        .flow_reason("default", &state.current_track_id.unwrap())
        .await
        .unwrap();
    assert_eq!(reason.familiarity_ratio, Some(0.9));
    assert_eq!(state.queue_track_ids.len(), 7);
    assert_eq!(
        state
            .queue_track_ids
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        7
    );
}

#[tokio::test]
async fn new_start_changes_generation_but_continue_does_not_and_inactive_continue_errors() {
    let processor = setup_processor().await;
    for i in 0..4 {
        add_track(
            &processor,
            &format!("Track {i}"),
            &format!("Artist {i}"),
            "Album",
            "Rock",
            2020,
        )
        .await;
    }
    assert!(processor
        .dispatch_command(Command::ContinueFlow)
        .await
        .is_err());
    let service = processor.playback_service();
    processor
        .dispatch_command(Command::StartFlow {
            familiarity_ratio: Some(0.3),
        })
        .await
        .unwrap();
    let first = service
        .flow_session(&processor.flow_engine(), "default")
        .await
        .unwrap()
        .unwrap();
    let current = service.get_playback_state().await.current_track_id;
    processor.dispatch_command(Command::Pause).await.unwrap();
    processor
        .dispatch_command(Command::StartFlow {
            familiarity_ratio: Some(0.8),
        })
        .await
        .unwrap();
    assert!(
        service.get_playback_state().await.is_playing,
        "Start Flow must resume paused audio"
    );
    let second = service
        .flow_session(&processor.flow_engine(), "default")
        .await
        .unwrap()
        .unwrap();
    assert!(second.generation > first.generation);
    assert_eq!(second.familiarity_ratio, 0.8);
    assert_eq!(service.get_playback_state().await.current_track_id, current);
    processor.dispatch_command(Command::Pause).await.unwrap();
    processor
        .dispatch_command(Command::ContinueFlow)
        .await
        .unwrap();
    assert_eq!(
        service
            .flow_session(&processor.flow_engine(), "default")
            .await
            .unwrap()
            .unwrap()
            .generation,
        second.generation
    );
}

#[tokio::test]
async fn preserved_current_finishes_with_original_history_session_then_flow_advances() {
    use music_player_backend::core::event::Event;
    let backend = MockAudioBackend::new();
    let processor = CoreProcessor::new_with_backend(
        create_in_memory_pool().await.unwrap(),
        fixture_config(),
        Box::new(backend.clone()),
    );
    let current = add_track(&processor, "Current", "One", "A", "Rock", 2020).await;
    let fresh = add_track(&processor, "Fresh", "Two", "B", "Jazz", 2020).await;
    let mut events = processor.event_bus().subscribe();
    processor
        .dispatch_command(Command::PlayTrack {
            track_id: current.clone(),
            source: None,
        })
        .await
        .unwrap();
    processor
        .dispatch_command(Command::StartFlow {
            familiarity_ratio: Some(0.5),
        })
        .await
        .unwrap();
    let (original, completed, starts) =
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            let mut original = None;
            let mut completed = None;
            let mut starts = 0;
            let mut observed_playing = false;
            loop {
                match events.recv().await.unwrap() {
                    Event::PlaybackPositionChanged { .. } if !observed_playing => {
                        // Wait for the real monitor to observe active audio before
                        // simulating completion; avoid a timing-based sleep.
                        observed_playing = true;
                        backend.set_finished(true);
                    }
                    Event::PlaybackStarted {
                        track_id,
                        session_id,
                        ..
                    } => {
                        starts += 1;
                        if track_id == current {
                            original = Some(session_id);
                        }
                        if track_id == fresh {
                            break (original, completed, starts);
                        }
                    }
                    Event::TrackFinished {
                        track_id,
                        session_id,
                        completed: true,
                        ..
                    } if track_id == current => completed = Some(session_id),
                    _ => {}
                }
            }
        })
        .await
        .unwrap();
    assert!(original.is_some());
    assert_eq!(original, completed);
    assert_eq!(starts, 2, "queue reset must not add a playback start");
    assert_eq!(
        processor
            .playback_service()
            .get_playback_state()
            .await
            .current_track_id,
        Some(fresh)
    );
}
#[tokio::test]
async fn home_for_you_recomputes_when_listening_changes_before_cache_expiry() {
    let processor = setup_processor().await;
    let played = add_track(&processor, "Played", "New Favorite", "Album", "Rock", 2020).await;
    let candidate = add_track(&processor, "Unheard", "New Favorite", "Album", "Rock", 2020).await;
    add_track(
        &processor,
        "Other",
        "Other Artist",
        "Other Album",
        "Jazz",
        2020,
    )
    .await;
    let engine = processor.flow_engine();
    let before = engine.get_home_overview("default").await.unwrap();
    let old_score = before
        .for_you
        .iter()
        .find(|t| t.track_id == candidate)
        .unwrap()
        .score;
    let now = chrono::Utc::now().timestamp();
    SqliteHistoryRepository::new(processor.db_pool().clone())
        .record_playback(&PlaybackHistoryRecord {
            id: Uuid::new_v4().to_string(),
            user_id: "default".into(),
            track_id: played,
            started_at: now - 240,
            ended_at: now,
            seconds_listened: 240.0,
            percentage_listened: 1.0,
            completed: 1,
            skipped: 0,
            source: "collection".into(),
            playlist_id: None,
            recommendation_session_id: None,
        })
        .await
        .unwrap();
    let after = engine.get_home_overview("default").await.unwrap();
    let new_score = after
        .for_you
        .iter()
        .find(|t| t.track_id == candidate)
        .unwrap()
        .score;
    assert!(
        new_score > old_score,
        "Home cannot retain a stale taste score for the cache's 60-second lifetime"
    );
}

#[tokio::test]
async fn start_flow_resumes_paused_current_without_a_second_play_or_new_history_session() {
    use music_player_backend::core::Event;
    let processor = setup_processor().await;
    let current = add_track(&processor, "Current", "One", "Album", "Rock", 2020).await;
    // Even a one-song library can resume its loaded song: excluding the
    // current recording leaves no additional candidates, which is not an error.
    let service = processor.playback_service();
    let mut events = processor.event_bus().subscribe();
    processor
        .dispatch_command(Command::PlayTrack {
            track_id: current.clone(),
            source: None,
        })
        .await
        .unwrap();
    processor.dispatch_command(Command::Pause).await.unwrap();
    while events.try_recv().is_ok() {}
    for _ in 0..2 {
        processor
            .dispatch_command(Command::StartFlow {
                familiarity_ratio: Some(0.5),
            })
            .await
            .unwrap();
        assert!(
            service.get_playback_state().await.is_playing,
            "Start Flow must play without a separate Resume click"
        );
        assert_eq!(
            service
                .get_playback_state()
                .await
                .current_track_id
                .as_deref(),
            Some(current.as_str())
        );
        let mut resumed = 0;
        while let Ok(event) = events.try_recv() {
            match event {
                Event::PlaybackResumed { .. } => resumed += 1,
                Event::PlaybackStarted { .. } => panic!("preserved current song must not restart"),
                _ => {}
            }
        }
        assert_eq!(resumed, 1);
        processor.dispatch_command(Command::Pause).await.unwrap();
        processor.dispatch_command(Command::StopFlow).await.unwrap();
    }
    processor.shutdown().await.unwrap();
}

#[tokio::test]
async fn ready_flow_does_not_wait_for_unrelated_legacy_catalog_network() {
    use music_player_backend::{
        database::{
            models::ExternalTrackRecord,
            repositories::{SqliteWishlistRepository, WishlistRepository},
        },
        playback::stream::StreamPlaybackManager,
        providers::{jiosaavn::JioSaavnProvider, online_catalog::OnlineCatalog},
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio::{io::AsyncReadExt, net::TcpListener};
    let processor = setup_processor().await;
    let directory = tempfile::tempdir().unwrap();
    add_track(&processor, "Ready One", "One", "Album", "Rock", 2020).await;
    add_track(&processor, "Ready Two", "Two", "Album", "Pop", 2021).await;
    SqliteWishlistRepository::new(processor.db_pool().clone())
        .upsert_external_track(&ExternalTrackRecord {
            id: "legacy-unrelated".into(),
            provider: "itunes".into(),
            provider_id: "legacy-unrelated".into(),
            title: "Unrelated slow catalog entry".into(),
            artist: "Artist".into(),
            album: None,
            duration_secs: Some(200.0),
            cover_art_url: None,
            preview_url: None,
            genre: None,
            match_status: "NOT_FOUND".into(),
            matched_local_track_id: None,
            created_at: 0,
        })
        .await
        .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let calls = Arc::new(AtomicUsize::new(0));
    let requests = calls.clone();
    let server = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut buffer = [0u8; 4096];
            let _ = socket.read(&mut buffer).await;
            requests.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });
    let mut manager = StreamPlaybackManager::new(
        directory.path().into(),
        Some(processor.db_pool().clone()),
        None,
    );
    manager.catalog = Some(OnlineCatalog::new(
        processor.db_pool().clone(),
        Arc::new(JioSaavnProvider::with_endpoint(&endpoint)),
    ));
    processor
        .playback_service()
        .set_stream_manager(Arc::new(manager));
    let started = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        processor.dispatch_command(Command::StartFlow {
            familiarity_ratio: Some(0.5),
        }),
    )
    .await;
    let calls = calls.load(Ordering::SeqCst);
    server.abort();
    processor.shutdown().await.unwrap();
    assert!(
        started.is_ok(),
        "a ready local Flow must not wait for unrelated catalog work; requests={calls}"
    );
    started.unwrap().unwrap();
    assert_eq!(calls, 0);
}

#[tokio::test]
async fn library_moment_counts_real_library_songs_and_ignores_online_placeholders() {
    let processor = setup_processor().await;
    add_track(&processor, "One", "Real Artist", "Real Album", "Rock", 2020).await;
    add_track(&processor, "Two", "Real Artist", "Real Album", "Rock", 2020).await;
    let ghost = add_track(
        &processor,
        "Metadata Only",
        "Remote Artist",
        "Ghost Album",
        "Pop",
        2024,
    )
    .await;
    sqlx::query("UPDATE tracks SET format='online',file_path='online://ghost' WHERE id=?")
        .bind(&ghost)
        .execute(processor.db_pool())
        .await
        .unwrap();
    sqlx::query("UPDATE albums SET total_tracks=0")
        .execute(processor.db_pool())
        .await
        .unwrap();
    for _ in 0..8 {
        let home = processor
            .flow_engine()
            .get_home_overview("default")
            .await
            .unwrap();
        let moment = home.library_moment.unwrap();
        assert_eq!(
            moment.album_title, "Real Album",
            "metadata placeholders are not playable library albums"
        );
        assert_eq!(
            moment.total_tracks, 2,
            "count actual album members instead of unset metadata"
        );
    }
}

#[tokio::test]
async fn album_details_uses_exact_id_all_members_and_current_account_feedback() {
    let processor = setup_processor().await;
    let first = add_track(
        &processor,
        "First",
        "Sean Paul",
        "No Lie (feat. Dua Lipa) - Single",
        "Pop",
        2020,
    )
    .await;
    let second = add_track(
        &processor,
        "Second",
        "Sean Paul",
        "No Lie (feat. Dua Lipa) - Single",
        "Pop",
        2020,
    )
    .await;
    add_track(
        &processor,
        "Wrong album",
        "Another Artist",
        "No Lie (feat. Dua Lipa) - Single",
        "Pop",
        2020,
    )
    .await;
    let album_id: String = sqlx::query_scalar("SELECT album_id FROM tracks WHERE id=?")
        .bind(&first)
        .fetch_one(processor.db_pool())
        .await
        .unwrap();
    sqlx::query("UPDATE tracks SET track_number=2 WHERE id=?")
        .bind(&first)
        .execute(processor.db_pool())
        .await
        .unwrap();
    for number in 3..=105 {
        let track = add_track(
            &processor,
            &format!("Song {number}"),
            "Sean Paul",
            "No Lie (feat. Dua Lipa) - Single",
            "Pop",
            2020,
        )
        .await;
        sqlx::query("UPDATE tracks SET track_number=? WHERE id=?")
            .bind(number)
            .bind(track)
            .execute(processor.db_pool())
            .await
            .unwrap();
    }
    sqlx::query("INSERT INTO track_statistics(user_id,track_id,manual_like) VALUES('default',?,1),('another-user',?,-1)").bind(&first).bind(&first).execute(processor.db_pool()).await.unwrap();
    let result = processor
        .execute_query(Query::GetAlbumDetails {
            album_id: album_id.clone(),
        })
        .await
        .unwrap();
    let QueryResponse::AlbumDetails(value) = result else {
        panic!("expected album details")
    };
    let rows = value["tracks"].as_array().unwrap();
    assert_eq!(
        rows.len(),
        105,
        "return every exact album member beyond the former 100-result title-search cap"
    );
    assert_eq!(rows[0]["id"], second);
    assert_eq!(rows[1]["id"], first);
    assert_eq!(
        rows[1]["manual_like"], 1,
        "feedback belongs to the current account"
    );
    assert_eq!(value["album"]["id"], album_id);
    assert!(
        processor
            .execute_query(Query::GetAlbumDetails {
                album_id: "removed-album".into()
            })
            .await
            .is_err(),
        "missing albums report an error instead of silently doing nothing"
    );
}

async fn full_listens(processor: &CoreProcessor, track_id: &str, count: i64) {
    let history_repo = SqliteHistoryRepository::new(processor.db_pool().clone());
    let now = chrono::Utc::now().timestamp();
    for i in 0..count {
        history_repo
            .record_playback(&PlaybackHistoryRecord {
                id: Uuid::new_v4().to_string(),
                user_id: "default".into(),
                track_id: track_id.to_string(),
                started_at: now - 86400 - i * 3600,
                ended_at: now - 86400 - i * 3600 + 240,
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
    }
}

#[tokio::test]
async fn two_listens_do_not_flood_flow_across_refills() {
    let processor = setup_processor().await;
    let mut heard = Vec::new();
    for i in 0..10 {
        heard.push(
            add_track(
                &processor,
                &format!("Heard {i}"),
                "Heard",
                "H",
                "Rock",
                2020,
            )
            .await,
        );
    }
    full_listens(&processor, &heard[0], 1).await;
    full_listens(&processor, &heard[1], 1).await;
    for a in 0..15 {
        for i in 0..2 {
            add_track(
                &processor,
                &format!("Other {a}-{i}"),
                &format!("Other {a}"),
                "O",
                "Rock",
                2020,
            )
            .await;
        }
    }
    for ratio in [0.0, 0.5, 1.0] {
        let mut exclude: Vec<String> = Vec::new();
        let mut recent: Vec<String> = Vec::new();
        let mut picked: Vec<String> = Vec::new();
        for _ in 0..3 {
            let batch = processor
                .flow_engine()
                .generate_flow_tracks_after("default", ratio, 7, &exclude, &recent)
                .await
                .unwrap();
            for item in batch {
                exclude.push(item.track_id.clone());
                recent.push(item.artist.clone());
                picked.push(item.artist);
            }
            let keep = recent.len().saturating_sub(15);
            recent.drain(..keep);
        }
        let window = &picked[..picked.len().min(20)];
        let heard_count = window.iter().filter(|a| *a == "Heard").count();
        assert!(
            heard_count <= 2,
            "ratio {ratio}: Heard appeared {heard_count}x in {window:?}"
        );
        assert!(
            window.windows(2).all(|p| p[0] != p[1]),
            "ratio {ratio}: back-to-back {window:?}"
        );
    }
}

#[tokio::test]
async fn steering_runs_unfamiliar_to_familiar() {
    let processor = setup_processor().await;
    let mut known = Vec::new();
    for a in 0..6 {
        let id = add_track(
            &processor,
            &format!("Known {a}"),
            &format!("Fav {a}"),
            "F",
            "Rock",
            2020,
        )
        .await;
        full_listens(&processor, &id, 3).await;
        known.push(id);
    }
    for a in 0..6 {
        add_track(
            &processor,
            &format!("Fresh {a}"),
            &format!("New {a}"),
            "N",
            "Rock",
            2020,
        )
        .await;
    }
    let mut known_share = Vec::new();
    for ratio in [0.0, 1.0] {
        let batch = processor
            .flow_engine()
            .generate_flow_tracks_after("default", ratio, 6, &[], &[])
            .await
            .unwrap();
        let count = batch.iter().filter(|t| known.contains(&t.track_id)).count();
        known_share.push(count);
    }
    assert!(
        known_share[0] <= 2,
        "Unfamiliar picked {} known songs",
        known_share[0]
    );
    assert!(
        known_share[1] >= 4,
        "Familiar picked only {} known songs",
        known_share[1]
    );
}

#[tokio::test]
async fn most_played_artist_does_not_flood_familiar_flow() {
    let processor = setup_processor().await;
    for i in 0..12 {
        let id = add_track(&processor, &format!("Top {i}"), "Top", "T", "Rock", 2020).await;
        full_listens(&processor, &id, 4).await;
        sqlx::query("INSERT INTO track_statistics(user_id,track_id,manual_like) VALUES('default',?,1) ON CONFLICT(user_id,track_id) DO UPDATE SET manual_like=1")
            .bind(&id).execute(processor.db_pool()).await.unwrap();
    }
    for a in 0..15 {
        for i in 0..2 {
            let id = add_track(
                &processor,
                &format!("Other {a}-{i}"),
                &format!("Other {a}"),
                "O",
                "Rock",
                2020,
            )
            .await;
            if i == 0 {
                full_listens(&processor, &id, 1).await;
            }
        }
    }
    let mut exclude: Vec<String> = Vec::new();
    let mut recent: Vec<String> = Vec::new();
    let mut picked: Vec<String> = Vec::new();
    for _ in 0..3 {
        let batch = processor
            .flow_engine()
            .generate_flow_tracks_after("default", 1.0, 7, &exclude, &recent)
            .await
            .unwrap();
        for item in batch {
            exclude.push(item.track_id.clone());
            recent.push(item.artist.clone());
            picked.push(item.artist);
        }
        let keep = recent.len().saturating_sub(15);
        recent.drain(..keep);
    }
    let window = &picked[..picked.len().min(20)];
    let top = window.iter().filter(|a| *a == "Top").count();
    assert!(
        top >= 1,
        "Familiar should still include the favorite: {window:?}"
    );
    assert!(top <= 2, "most played artist appeared {top}x in {window:?}");
    assert!(
        window.windows(2).all(|p| p[0] != p[1]),
        "back-to-back {window:?}"
    );
}

#[tokio::test]
async fn most_played_artist_does_not_flood_smart_mixes() {
    use music_player_backend::{core::command::SmartMixType, recommendations::SmartMixGenerator};
    let processor = setup_processor().await;
    for i in 0..10 {
        let id = add_track(&processor, &format!("Top {i}"), "Top", "T", "Rock", 2020).await;
        full_listens(&processor, &id, 4).await;
    }
    for a in 0..6 {
        let id = add_track(
            &processor,
            &format!("Other {a}"),
            &format!("Other {a}"),
            "O",
            "Rock",
            2020,
        )
        .await;
        full_listens(&processor, &id, 2).await;
    }
    let pool = processor.db_pool().clone();
    for mix_type in [SmartMixType::OnRepeat, SmartMixType::Daily] {
        let mix = SmartMixGenerator::new(pool.clone())
            .generate_mix("default", &mix_type)
            .await
            .unwrap();
        let artists: Vec<String> = sqlx::query_scalar(
            "SELECT a.name FROM playlist_tracks pt JOIN tracks t ON t.id=pt.track_id
             JOIN artists a ON a.id=t.artist_id WHERE pt.playlist_id=? ORDER BY pt.position",
        )
        .bind(&mix.id)
        .fetch_all(&pool)
        .await
        .unwrap();
        let top = artists.iter().filter(|a| *a == "Top").count();
        assert!(!artists.is_empty(), "{mix_type:?} empty");
        assert!(top <= 2, "{mix_type:?}: Top appeared {top}x in {artists:?}");
    }
}

// ---- Session steering (live skips / full listens) ----

use music_player_backend::recommendations::session::{SessionOutcome, SessionSteer, StyleKeys};

const SKIP: SessionOutcome = SessionOutcome {
    early_skip: true,
    qualified: false,
    near_complete: false,
};
const FULL: SessionOutcome = SessionOutcome {
    early_skip: false,
    qualified: true,
    near_complete: true,
};

/// 15 Rock + 15 Jazz single-track artists. Returns (rock ids, jazz ids).
async fn two_genre_library(processor: &CoreProcessor) -> (Vec<String>, Vec<String>) {
    let mut rock = Vec::new();
    let mut jazz = Vec::new();
    for i in 0..15 {
        rock.push(
            add_track(
                processor,
                &format!("Riff {i}"),
                &format!("Rocker {i}"),
                "R",
                "Rock",
                2020,
            )
            .await,
        );
        jazz.push(
            add_track(
                processor,
                &format!("Swing {i}"),
                &format!("Cat {i}"),
                "J",
                "Jazz",
                2020,
            )
            .await,
        );
    }
    (rock, jazz)
}

fn count_in(
    batch: &[music_player_backend::recommendations::FlowTrackItem],
    ids: &[String],
) -> usize {
    batch.iter().filter(|t| ids.contains(&t.track_id)).count()
}

#[tokio::test]
async fn session_skips_steer_away_from_genre() {
    let processor = setup_processor().await;
    let (rock, _jazz) = two_genre_library(&processor).await;
    // Long-term taste leans Rock so the baseline is Rock-heavy.
    full_listens(&processor, &rock[0], 2).await;
    let fe = processor.flow_engine();
    let baseline = fe
        .generate_flow_tracks_with_session("default", 0.5, 10, &[], &[], None)
        .await
        .unwrap();
    let mut steer = SessionSteer::default();
    steer.record(&StyleKeys::new(Some("Rocker 1"), Some("Rock"), None), SKIP);
    let trigger = steer.record(&StyleKeys::new(Some("Rocker 2"), Some("Rock"), None), SKIP);
    assert!(trigger.fired(), "two Rock skips must trigger: {trigger:?}");
    let steered = fe
        .generate_flow_tracks_with_session("default", 0.5, 10, &[], &[], Some(&steer))
        .await
        .unwrap();
    let (before, after) = (count_in(&baseline, &rock), count_in(&steered, &rock));
    assert!(after < before, "Rock share must drop: {before} -> {after}");
    assert!(after <= 3, "Rock still dominates after skips: {after}/10");
    let noted = steered
        .iter()
        .find(|t| t.explanation.session_adjustment.is_some_and(|a| a < 0.0));
    assert!(
        noted.is_none()
            || noted
                .unwrap()
                .explanation
                .session_style
                .as_deref()
                .is_some_and(|s| s.eq_ignore_ascii_case("rock")),
        "negative session note names the skipped style"
    );
}

#[tokio::test]
async fn session_full_listens_steer_toward_genre() {
    let processor = setup_processor().await;
    let (rock, jazz) = two_genre_library(&processor).await;
    // One Jazz artist with a deep catalog: the artist cap must still hold.
    let mut deep = Vec::new();
    for i in 0..6 {
        deep.push(
            add_track(
                &processor,
                &format!("Deep {i}"),
                "Deep Cat",
                "D",
                "Jazz",
                2020,
            )
            .await,
        );
    }
    full_listens(&processor, &rock[0], 2).await;
    let fe = processor.flow_engine();
    let baseline = fe
        .generate_flow_tracks_with_session("default", 0.5, 10, &[], &[], None)
        .await
        .unwrap();
    let mut steer = SessionSteer::default();
    let mut trigger = None;
    for artist in ["Cat 1", "Cat 2", "Cat 3"] {
        trigger = Some(steer.record(&StyleKeys::new(Some(artist), Some("Jazz"), None), FULL));
    }
    assert!(trigger.unwrap().fired(), "three Jazz finishes must trigger");
    let steered = fe
        .generate_flow_tracks_with_session("default", 0.5, 10, &[], &[], Some(&steer))
        .await
        .unwrap();
    let mut all_jazz = jazz.clone();
    all_jazz.extend(deep.iter().cloned());
    let (before, after) = (
        count_in(&baseline, &all_jazz),
        count_in(&steered, &all_jazz),
    );
    assert!(after > before, "Jazz share must grow: {before} -> {after}");
    assert!(
        count_in(&steered, &deep) <= 2,
        "artist cap broken by session boost"
    );
    assert!(steered.iter().any(|t| t
        .explanation
        .session_style
        .as_deref()
        .is_some_and(|s| s.eq_ignore_ascii_case("jazz"))
        && t.explanation.session_adjustment.is_some_and(|a| a > 0.0)));
}

async fn upcoming_ids(processor: &CoreProcessor) -> Vec<String> {
    let state = processor.playback_service().get_playback_state().await;
    let start = state.current_queue_index.map_or(0, |i| i + 1);
    state.queue_track_ids[start..].to_vec()
}

async fn skip_outcome(processor: &CoreProcessor, track_id: &str) {
    let _ =
        processor
            .event_bus()
            .publish(music_player_backend::core::event::Event::ListenOutcome {
                user_id: "default".into(),
                track_id: track_id.to_string(),
                early_skip: true,
                qualified: false,
                near_complete: false,
                weight: -1.0,
            });
}

async fn wait_for_change(processor: &CoreProcessor, before: &[String]) -> Vec<String> {
    for _ in 0..100 {
        let now = upcoming_ids(processor).await;
        if now != before {
            return now;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("upcoming Flow queue was not re-ranked");
}

#[tokio::test]
async fn rapid_rock_skips_rerank_upcoming_flow() {
    let processor = setup_processor().await;
    let (rock, _jazz) = two_genre_library(&processor).await;
    full_listens(&processor, &rock[0], 2).await;
    processor
        .dispatch_command(Command::StartFlow {
            familiarity_ratio: Some(0.5),
        })
        .await
        .unwrap();
    let before = upcoming_ids(&processor).await;
    let rock_before = before.iter().take(8).filter(|id| rock.contains(id)).count();
    assert!(
        rock_before >= 3,
        "baseline should include Rock: {rock_before}"
    );
    skip_outcome(&processor, &rock[1]).await;
    skip_outcome(&processor, &rock[2]).await;
    let after = wait_for_change(&processor, &before).await;
    let rock_after = after.iter().take(8).filter(|id| rock.contains(id)).count();
    assert!(
        rock_after < rock_before && rock_after <= 2,
        "next 8 Rock count {rock_before} -> {rock_after}"
    );
    let service = processor.playback_service();
    if let Some(reason) = service.flow_reason("default", &after[0]).await {
        assert!(reason.session_adjustment.is_some() || !rock.contains(&after[0]));
    }
}

#[tokio::test]
async fn session_resets_on_start_flow() {
    let processor = setup_processor().await;
    let (rock, _jazz) = two_genre_library(&processor).await;
    full_listens(&processor, &rock[0], 2).await;
    let start = || {
        processor.dispatch_command(Command::StartFlow {
            familiarity_ratio: Some(0.5),
        })
    };
    start().await.unwrap();
    let before = upcoming_ids(&processor).await;
    skip_outcome(&processor, &rock[1]).await;
    skip_outcome(&processor, &rock[2]).await;
    wait_for_change(&processor, &before).await;
    start().await.unwrap();
    let service = processor.playback_service();
    let current = service
        .get_playback_state()
        .await
        .current_track_id
        .expect("restarted Flow plays a track");
    let reason = service
        .flow_reason("default", &current)
        .await
        .expect("restarted Flow explains its pick");
    assert!(
        reason.session_adjustment.is_none(),
        "fresh Flow must not carry the old session steer"
    );
}

/// The fixture songs sit in Kaze's download folder: songs from the user's own
/// folders take no likes or dislikes, and these tests rate them.
fn fixture_config() -> AppConfig {
    let mut config = AppConfig::default_with_dirs();
    config.downloads.download_dir = Some("/music".into());
    config
}
