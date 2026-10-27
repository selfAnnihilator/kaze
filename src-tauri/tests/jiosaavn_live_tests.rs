//! Opt-in network/native gate. Uses only a disposable directory and muted output.
use music_player_backend::{
    playback::{
        backend::{AudioBackend, RodioAudioBackend},
        decoder::SymphoniaSource,
    },
    providers::jiosaavn::JioSaavnProvider,
};
use rodio::Source;
use std::time::Duration;

#[tokio::test]
#[ignore = "requires live JioSaavn network and native audio output"]
async fn full_track_decoder_and_native_seek_gate() {
    let provider = JioSaavnProvider::new();
    let temp = tempfile::tempdir().unwrap();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .unwrap();
    let mut backend = RodioAudioBackend::try_new().unwrap();
    backend.set_volume(0.0).unwrap();
    for query in [
        "Kesariya",
        "Vaathi Coming",
        "Buttabomma",
        "Believer Imagine Dragons",
    ] {
        let results = provider.search(query, 5).await.expect("live search");
        let first = results.first().expect("search returned tracks");
        let detail = provider
            .detail(&first.provider_track_id, false)
            .await
            .expect("exact song details");
        let metadata = music_player_backend::providers::jiosaavn::parse_track(&detail).unwrap();
        let art = client
            .get(metadata.cover_art_url.as_ref().expect("artwork URL"))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .bytes()
            .await
            .unwrap();
        image::load_from_memory(&art).expect("valid artwork image");
        let (url, duration) = provider
            .resolve(&first.provider_track_id, false)
            .await
            .expect("ordinary full-track media");
        let audio = client
            .get(url)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert!(audio.len() < 64 * 1024 * 1024);
        let path = temp.path().join(format!("{}.m4a", first.provider_track_id));
        std::fs::write(&path, &audio).unwrap();
        let source = SymphoniaSource::new(&path).expect("production Rust decoder");
        let rate = source.sample_rate() as f64 * source.channels() as f64;
        let decoded_secs = source.count() as f64 / rate;
        assert!(
            (decoded_secs - duration).abs() < 3.0,
            "full track required: decoded {decoded_secs}s vs {duration}s"
        );
        let mut source = SymphoniaSource::new(&path).unwrap();
        for secs in [duration / 2.0, duration - 5.0, 1.0] {
            source.try_seek(Duration::from_secs_f64(secs)).unwrap();
            assert!(source.next().is_some(), "decoder produces audio after seek");
        }
        backend
            .load_and_play(&path)
            .expect("native output must be available");
        backend.pause().unwrap();
        assert!(backend.is_paused());
        backend
            .seek(Duration::from_secs_f64(duration / 2.0))
            .expect("native seek");
        backend.resume().unwrap();
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(!backend.is_finished());
        backend.stop().unwrap();
        println!(
            "PASS {} [{}]: {} bytes, {:.2}/{:.0}s decoded, artwork/native seek",
            metadata.title,
            first.provider_track_id,
            audio.len(),
            decoded_secs,
            duration
        );
    }
}

#[tokio::test]
#[ignore = "requires live JioSaavn network and native audio output"]
async fn unified_player_stream_cache_and_identity_gate() {
    use music_player_backend::{
        config::AppConfig,
        core::{Command, CoreProcessor, Query, QueryResponse},
        database::init_db_pool,
    };
    let temp = tempfile::tempdir().unwrap();
    let pool = init_db_pool(temp.path().join("test.db")).await.unwrap();
    let mut config = AppConfig::default_with_dirs();
    config.cache_dir = temp.path().join("cache");
    config.database_path = temp.path().join("test.db");
    config.history.min_meaningful_seconds = 0.0;
    config.history.min_meaningful_percentage = 0.0;
    let core = CoreProcessor::new_with_backend(
        pool.clone(),
        config,
        Box::new(RodioAudioBackend::try_new().unwrap()),
    );
    core.dispatch_command(Command::SetVolume { volume: 0.0 })
        .await
        .unwrap();
    core.dispatch_command(Command::SetMetadataPreference {
        key: "auto_fill_metadata".into(),
        enabled: false,
    })
    .await
    .unwrap();
    let response = core
        .execute_query(Query::SearchOnlineMusic {
            query: "Kesariya".into(),
            provider: None,
            limit: Some(5),
        })
        .await
        .unwrap();
    let QueryResponse::DiscoveryRecommendations(results) = response else {
        panic!("search result type");
    };
    let rec = &results[0];
    let id = rec["external_track_id"].as_str().unwrap().to_string();
    assert!(id.starts_with("jiosaavn:"));
    let manager = std::sync::Arc::new(
        music_player_backend::playback::stream::StreamPlaybackManager::new(
            temp.path().join("cache"),
            Some(pool.clone()),
            None,
        ),
    );
    let source = manager
        .resolve_source_only(
            &id,
            rec["title"].as_str().unwrap(),
            rec["artist"].as_str().unwrap(),
        )
        .await
        .unwrap();
    assert!(matches!(
        source,
        music_player_backend::playback::stream::ResolvedAudioSource::RemoteStream { .. }
    ));
    assert!(
        !manager.cache_dir().exists(),
        "pre-resolution must not create or download audio"
    );
    manager.resolution_cache().insert(
        format!("full:v3:{id}"),
        music_player_backend::playback::resolution_cache::ResolvedEntry::new(
            "https://aac.saavncdn.com/kaze-expired-url-gate.mp4".into(),
            268.0,
            None,
        ),
    );
    core.playback_service().set_stream_manager(manager);
    let mut playback_events = core.event_bus().subscribe();
    core.dispatch_command(Command::PlayOnlineTrack {
        track_id: id.clone(),
        title: rec["title"].as_str().unwrap().into(),
        artist: rec["artist"].as_str().unwrap().into(),
        album: rec["album"].as_str().map(str::to_string),
        duration_secs: rec["duration_secs"].as_f64(),
        cover_art_url: rec["cover_art_url"].as_str().map(str::to_string),
        preview_url: None,
        source: Some("discovery".into()),
    })
    .await
    .expect("unified progressive playback");
    core.dispatch_command(Command::Pause).await.unwrap();
    let playback = core.playback_service();
    let state = playback.get_playback_state().await;
    assert_eq!(state.current_track_id.as_deref(), Some(id.as_str()));
    assert!(state.is_paused);
    tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            if playback.get_playback_state().await.can_seek {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("bounded complete audio cache");
    let ready = playback.get_playback_state().await;
    assert!(!ready.is_buffering, "completed audio cannot keep buffering");
    assert!(ready.is_paused, "background download must preserve pause");
    while let Ok(event) = playback_events.try_recv() {
        use music_player_backend::core::event::Event;
        match event {
            Event::PlaybackResumed { .. } => {
                panic!("download recovery must not pretend the user resumed")
            }
            Event::PlaybackBufferingChanged {
                session_id,
                track_id,
                ..
            } => {
                assert!(
                    !session_id.is_empty(),
                    "reader notifications require an active playback session"
                );
                assert_eq!(track_id, id);
            }
            _ => (),
        }
    }
    core.dispatch_command(Command::Seek {
        position_secs: 140.0,
    })
    .await
    .unwrap();
    core.dispatch_command(Command::Resume).await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    let state = playback.get_playback_state().await;
    assert!(state.is_playing);
    assert!(state.position_secs >= 139.0);
    core.dispatch_command(Command::Stop).await.unwrap();
    // Disabling network provider still allows the exact full cached song to replay.
    core.dispatch_command(Command::SetMetadataPreference {
        key: "enable_jiosaavn".into(),
        enabled: false,
    })
    .await
    .unwrap();
    core.dispatch_command(Command::PlayTrack {
        track_id: id.clone(),
        source: Some("library".into()),
    })
    .await
    .expect("full cache replay without provider");
    core.dispatch_command(Command::Pause).await.unwrap();
    assert_eq!(
        playback
            .get_playback_state()
            .await
            .current_track_id
            .as_deref(),
        Some(id.as_str())
    );
    core.dispatch_command(Command::Stop).await.unwrap();
    let identity: (String, String) =
        sqlx::query_as("SELECT provider,provider_id FROM external_tracks WHERE id=?")
            .bind(&id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(identity.0, "jiosaavn");
    assert_eq!(format!("jiosaavn:{}", identity.1), id);
    core.history_service().shutdown().await;
    let sessions: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM playback_history WHERE track_id=?")
            .bind(&id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let plays: i64 = sqlx::query_scalar(
        "SELECT play_count FROM track_statistics WHERE user_id='default' AND track_id=?",
    )
    .bind(&id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        sessions, 2,
        "pause/resume and seek must not create extra sessions"
    );
    assert_eq!(plays, 2, "each explicit play increments statistics once");
    println!("PASS unified PlaybackService: metadata-only pre-resolution, rejected URL refresh, progressive full song, pause/resume, seek, exact cache reuse, provider identity");
    core.shutdown().await.unwrap();
}

#[tokio::test]
#[ignore = "requires live JioSaavn search and full audio"]
async fn reported_bruno_mars_song_has_full_provider_audio() {
    let provider = JioSaavnProvider::new();
    let tracks = provider
        .search("dance with me by bruno mars", 10)
        .await
        .unwrap();
    let exact: Vec<_> = tracks
        .iter()
        .filter(|track| track.title == "Dance With Me" && track.artist_name == "Bruno Mars")
        .collect();
    assert!(
        !exact.is_empty(),
        "verify the reported song against the provider's current response"
    );
    let temp = tempfile::tempdir().unwrap();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap();
    for track in exact.iter().take(2) {
        let (url, duration) = provider
            .resolve(&track.provider_track_id, false)
            .await
            .unwrap();
        let bytes = client
            .get(url)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert!(bytes.len() < 64 * 1024 * 1024);
        let path = temp.path().join(format!("{}.m4a", track.provider_track_id));
        std::fs::write(&path, &bytes).unwrap();
        let source = SymphoniaSource::new(&path).expect("actual provider audio must decode");
        let rate = source.sample_rate() as f64 * source.channels() as f64;
        let seconds = source.count() as f64 / rate;
        assert!((seconds - duration).abs() < 3.0);
        println!(
            "PASS {} / {} [{}]: {} bytes; {:.2}/{:.0}s decoded",
            track.title,
            track.artist_name,
            track.provider_track_id,
            bytes.len(),
            seconds,
            duration
        );
    }
}

#[tokio::test]
#[ignore = "requires live JioSaavn and native audio output"]
async fn saved_catalog_entries_play_the_exact_version_shown() {
    use music_player_backend::{
        config::AppConfig,
        core::{Command, CoreProcessor, Query, QueryResponse},
        database::init_db_pool,
        recommendations::mixes::ensure_online_track,
    };
    let temp = tempfile::tempdir().unwrap();
    let pool = init_db_pool(temp.path().join("sources.db")).await.unwrap();
    let mut config = AppConfig::default_with_dirs();
    config.cache_dir = temp.path().join("cache");
    let core = CoreProcessor::new_with_backend(
        pool.clone(),
        config,
        Box::new(RodioAudioBackend::try_new().unwrap()),
    );
    core.dispatch_command(Command::SetVolume { volume: 0.0 })
        .await
        .unwrap();
    let examples = [
        ("Babydoll", "Jamie Miller", None),
        ("Dance With Me", "Bruno Mars", Some("The Romantic")),
        ("Missing Piece", "John Michael Howell", None),
        ("Bloody Mary", "Lady Gaga", None),
        ("Bad Romance", "Lady Gaga", None),
    ];
    let mut tested = std::collections::HashSet::new();
    for (i, (title, artist, album)) in examples.iter().enumerate() {
        let id = format!("itunes:fixture{i}");
        ensure_online_track(
            &pool,
            &id,
            title,
            Some(artist),
            *album,
            Some(210.0),
            None,
            None,
        )
        .await
        .unwrap();
        let QueryResponse::Track(shown) = core
            .execute_query(Query::GetTrackById {
                track_id: id.clone(),
            })
            .await
            .unwrap()
        else {
            panic!("track");
        };
        let Some(shown) = shown else {
            println!("OMITTED unavailable provider entry: {title} / {artist}");
            continue;
        };
        let source = shown["playback_source"]["track_id"]
            .as_str()
            .expect("shown online song must carry a source")
            .to_string();
        core.dispatch_command(Command::PlayTrack {
            track_id: id.clone(),
            source: Some("playlist".into()),
        })
        .await
        .expect("shown song plays without another search or unique-match error");
        tokio::time::sleep(Duration::from_millis(350)).await;
        let QueryResponse::PlaybackState(state) =
            core.execute_query(Query::GetPlaybackState).await.unwrap()
        else {
            panic!("state");
        };
        assert!(state["is_playing"].as_bool().unwrap());
        assert_eq!(
            state["current_track"]["playback_source"]["track_id"],
            source
        );
        assert_eq!(state["current_track"]["title"], shown["title"]);
        assert_eq!(state["current_track"]["album_title"], shown["album_title"]);
        assert_eq!(
            state["current_track"]["duration_secs"],
            shown["duration_secs"]
        );
        core.dispatch_command(Command::Pause).await.unwrap();
        core.dispatch_command(Command::Resume).await.unwrap();
        core.dispatch_command(Command::Stop).await.unwrap();
        println!("PASS saved {id}: {title} / {artist} -> JioSaavn {source}; displayed version and actual native playback agree");
        if *title == "Dance With Me" {
            sqlx::query("INSERT INTO external_tracks(id,provider,provider_id,title,artist,album,duration_secs,match_status,created_at) VALUES('cloud:exact-source','jiosaavn',?,'Dance With Me','Bruno Mars','The Romantic',219,'NOT_FOUND',1)")
                .bind(&source).execute(&pool).await.unwrap();
            let QueryResponse::DiscoveryRecommendations(results) = core
                .execute_query(Query::SearchOnlineMusic {
                    query: "Dance With Me Bruno Mars".into(),
                    provider: None,
                    limit: Some(10),
                })
                .await
                .unwrap()
            else {
                panic!("search")
            };
            let same = results
                .iter()
                .find(|row| row["provider_id"] == source)
                .unwrap();
            assert_eq!(
                same["external_track_id"], "cloud:exact-source",
                "search reuses an existing source identity without a provider-pair collision"
            );
        }
        tested.insert(*title);
    }
    assert!(
        tested.contains("Babydoll") && tested.contains("Dance With Me"),
        "must cover both reported cases"
    );
    for i in 0..examples.len() {
        sqlx::query("INSERT INTO track_statistics(user_id,track_id,play_count,completion_count,manual_like,last_played_at) VALUES('default',?,1,1,1,?) ON CONFLICT(user_id,track_id) DO UPDATE SET manual_like=1")
            .bind(format!("itunes:fixture{i}")).bind(chrono::Utc::now().timestamp()).execute(&pool).await.unwrap();
    }
    core.dispatch_command(Command::StartFlow {
        familiarity_ratio: Some(1.0),
    })
    .await
    .expect("Flow uses linked saved identities");
    for _ in 0..3 {
        let QueryResponse::PlaybackState(state) =
            core.execute_query(Query::GetPlaybackState).await.unwrap()
        else {
            panic!("Flow state")
        };
        assert!(state["is_playing"].as_bool().unwrap());
        assert!(state["current_track"]["playback_source"]["track_id"]
            .as_str()
            .is_some());
        core.dispatch_command(Command::NextTrack)
            .await
            .expect("Next follows the saved provider source");
    }
    core.dispatch_command(Command::Stop).await.unwrap();
    println!("PASS native Flow and Next: saved aliases retain source links without title matching");
    core.shutdown().await.unwrap();
}

#[tokio::test]
#[ignore = "requires live JioSaavn catalog and audio download"]
async fn catalog_playlist_and_exact_download_gate() {
    use music_player_backend::{
        config::AppConfig,
        core::{Command, CoreProcessor, Query, QueryResponse},
        database::init_db_pool,
        playback::backend::MockAudioBackend,
    };
    let temp = tempfile::tempdir().unwrap();
    let pool = init_db_pool(temp.path().join("catalog.db")).await.unwrap();
    let mut config = AppConfig::default_with_dirs();
    config.cache_dir = temp.path().join("cache");
    config.downloads.download_dir = Some(temp.path().join("downloads"));
    config.downloads.auto_import = false;
    let core = CoreProcessor::new_with_backend(pool, config, Box::new(MockAudioBackend::new()));
    core.dispatch_command(Command::SetMetadataPreference {
        key: "auto_fill_metadata".into(),
        enabled: false,
    })
    .await
    .unwrap();
    let QueryResponse::Charts(charts) = core.execute_query(Query::GetCharts).await.unwrap() else {
        panic!("charts")
    };
    assert!(!charts.is_empty());
    let id = charts[0]["id"].as_str().unwrap();
    let QueryResponse::DiscoveryRecommendations(songs) = core
        .execute_query(Query::GetChartSongs {
            chart_id: format!("jiosaavn:playlist:{id}"),
            limit: 5,
        })
        .await
        .unwrap()
    else {
        panic!("chart songs")
    };
    assert!(!songs.is_empty());
    assert!(songs.iter().all(|song| song["provider"] == "jiosaavn"
        && song["duration_secs"].as_f64().unwrap_or(0.0) > 0.0));
    let QueryResponse::JioSaavnPlaylistImport(import) = core
        .execute_query(Query::ImportJioSaavnPlaylist {
            url_or_id: charts[0]["perma_url"].as_str().unwrap_or(id).into(),
        })
        .await
        .unwrap()
    else {
        panic!("playlist")
    };
    assert!(import["tracks"][0]["track_id"]
        .as_str()
        .unwrap()
        .starts_with("jiosaavn:"));
    assert!(
        charts.iter().any(|chart| chart["title"]
            .as_str()
            .is_some_and(|title| title.contains("International") || title.contains("English"))),
        "international programming must be requested explicitly"
    );
    assert!(charts.iter().all(|chart| chart["image"]
        .as_str()
        .is_some_and(|url| url.starts_with("https://"))));
    let selected_id = songs[0]["external_track_id"].as_str().unwrap();
    let response = core
        .dispatch_command(Command::DownloadTrack {
            track_id: selected_id.into(),
        })
        .await
        .unwrap();
    let music_player_backend::core::command::CommandResponse::DownloadStarted { task_id } =
        response
    else {
        panic!("direct download")
    };
    let task = core
        .download_service()
        .list_downloads(None, 10)
        .await
        .unwrap()
        .into_iter()
        .find(|task| task.id == task_id)
        .unwrap();
    assert_eq!(
        task.source_track_id.as_deref(),
        songs[0]["provider_id"].as_str()
    );
    println!(
        "Actual charts: {}",
        charts
            .iter()
            .take(5)
            .filter_map(|chart| chart["title"].as_str())
            .collect::<Vec<_>>()
            .join("; ")
    );
    tokio::time::timeout(Duration::from_secs(90), async {
        loop {
            let _ = core.download_service().poll_task(&task.id).await.unwrap();
            let saved = core
                .download_service()
                .list_downloads(None, 10)
                .await
                .unwrap()
                .into_iter()
                .find(|saved| saved.id == task.id)
                .unwrap();
            assert_ne!(saved.status, "FAILED");
            if saved.status == "COMPLETED" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .expect("bounded download");
    let path = std::path::PathBuf::from(task.destination_path.unwrap());
    assert!(std::fs::metadata(&path).unwrap().len() > 100_000);
    assert!(SymphoniaSource::new(&path).unwrap().next().is_some());
    assert!(std::fs::read_dir(path.parent().unwrap())
        .unwrap()
        .all(|entry| !entry
            .unwrap()
            .path()
            .extension()
            .is_some_and(|extension| extension == "part")));
    core.shutdown().await.unwrap();
    println!("PASS provider charts, exact playlist identities and selected JioSaavn full-audio download/decoder");
}
