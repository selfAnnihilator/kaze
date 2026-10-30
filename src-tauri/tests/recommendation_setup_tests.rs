use music_player_backend::database::repositories::ScannedMetadata;
use music_player_backend::{
    config::AppConfig,
    core::{command::Command, event::Event, processor::CoreProcessor, query::Query},
    database::create_in_memory_pool,
    playback::backend::MockAudioBackend,
};
use serde_json::{json, Value};

async fn processor() -> CoreProcessor {
    let mut config = AppConfig::default_with_dirs();
    config.cache_dir = tempfile::tempdir().unwrap().keep();

    let p = CoreProcessor::new_with_backend(
        create_in_memory_pool().await.unwrap(),
        config,
        Box::new(MockAudioBackend::new()),
    );
    sqlx::query("INSERT OR REPLACE INTO application_settings (key,value,updated_at) VALUES ('enable_jiosaavn','false',0)").execute(p.db_pool()).await.unwrap();
    p
}
async fn setup(p: &CoreProcessor) -> Value {
    serde_json::to_value(
        p.execute_query(
            serde_json::from_value::<Query>(json!({"query":"GetRecommendationSetup"})).unwrap(),
        )
        .await
        .unwrap(),
    )
    .unwrap()["data"]
        .clone()
}
#[tokio::test]
async fn empty_player_rejects_resume_and_toggle_without_resumed_event() {
    let p = processor().await;
    let mut events = p.event_bus().subscribe();
    assert!(p.dispatch_command(Command::Resume).await.is_err());
    assert!(p.dispatch_command(Command::TogglePlayPause).await.is_err());
    assert!(!p.playback_service().get_playback_state().await.is_playing);
    while let Ok(event) = events.try_recv() {
        assert!(!matches!(event, Event::PlaybackResumed { .. }));
    }
}
#[tokio::test]
async fn taste_setup_roundtrip_is_separate_from_listening_and_skip_is_persistent() {
    let p = processor().await;
    assert_eq!(setup(&p).await["should_prompt"], true);
    let stale=serde_json::from_value::<Command>(json!({"command":"SaveRecommendationSetup","payload":{"expected_user_id":"different-account","artists":[],"genres":["Rock"],"skipped":false}})).unwrap();
    assert!(p.dispatch_command(stale).await.is_err());
    assert_eq!(
        setup(&p).await["status"],
        "pending",
        "a stale account cannot change the active account's setup"
    );
    let command=serde_json::from_value::<Command>(json!({"command":"SaveRecommendationSetup","payload":{"artists":["Favorite Artist"," Favorite Artist "],"genres":["Rock"],"skipped":false}})).unwrap();
    p.dispatch_command(command).await.unwrap();
    let value = setup(&p).await;
    assert_eq!(value["status"], "completed");
    assert_eq!(value["should_prompt"], false);
    assert_eq!(value["artists"], json!(["Favorite Artist"]));
    let profile = p
        .taste_engine()
        .compute_taste_profile("default")
        .await
        .unwrap();
    assert_eq!(profile.total_sessions_analyzed, 0);
    assert_eq!(profile.artist_affinity("Favorite Artist"), 0.35);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM playback_history")
        .fetch_one(p.db_pool())
        .await
        .unwrap();
    assert_eq!(count, 0);
    p.dispatch_command(serde_json::from_value::<Command>(json!({"command":"SaveRecommendationSetup","payload":{"artists":[],"genres":[],"skipped":true}})).unwrap()).await.unwrap();
    assert_eq!(setup(&p).await["status"], "skipped");
}

async fn catalog_server() -> (
    String,
    std::sync::Arc<std::sync::atomic::AtomicUsize>,
    tokio::task::JoinHandle<()>,
) {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let count = Arc::new(AtomicUsize::new(0));
    let requests = count.clone();
    let task = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut bytes = [0; 8192];
            let n = socket.read(&mut bytes).await.unwrap();
            let request = String::from_utf8_lossy(&bytes[..n]);
            requests.fetch_add(1, Ordering::SeqCst);
            let tracks:Vec<_>=(0..24).map(|i|json!({"id":format!("starter{i}"),"song":format!("Starter song {i}"),"primary_artists":format!("Favorite Artist {}",i%6),"duration":"200","rights":{"code":0},"encrypted_media_url":"ID2ieOjCrwfgWvL5sXl4B1ImC5QfbsDyryhkSYK5IH2E7FCO52VR6yhNbcEbes5iCcja4+W8xhE0SwtCJToN4Bw7tS9a8Gtq"})).collect();
            let chart = json!({"id":"123","title":"Global Top Songs"});
            let body = if request.contains("content.getCharts") {
                json!([chart])
            } else if request.contains("webapi.getLaunchData") {
                json!({"charts":[chart]})
            } else if request.contains("playlist.getDetails") {
                json!({"list":tracks})
            } else {
                json!({"results":tracks})
            }
            .to_string();
            let response=format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body);
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });
    (url, count, task)
}
#[tokio::test]
async fn fresh_catalog_bootstrap_is_shared_by_discovery_home_and_flow_without_fake_history() {
    use music_player_backend::{
        database::repositories::SqliteWishlistRepository,
        discovery::DiscoveryCoordinator,
        providers::{jiosaavn::JioSaavnProvider, ProviderCoordinator},
    };
    use std::sync::{atomic::Ordering, Arc};
    let p = processor().await;
    let (endpoint, requests, server) = catalog_server().await;
    sqlx::query("UPDATE application_settings SET value='true' WHERE key='enable_jiosaavn'")
        .execute(p.db_pool())
        .await
        .unwrap();
    let mut provider =
        ProviderCoordinator::new(p.db_pool().clone(), p.event_bus(), std::env::temp_dir());
    provider.jiosaavn = Arc::new(JioSaavnProvider::with_endpoint(&endpoint));
    let d = DiscoveryCoordinator::new(
        Arc::new(SqliteWishlistRepository::new(p.db_pool().clone())),
        p.library_service().track_repo(),
        p.taste_engine(),
        Some(Arc::new(provider)),
    );
    let (a, b) = tokio::join!(
        d.ensure_starter_candidates("default", false),
        d.ensure_starter_candidates("default", false)
    );
    a.unwrap();
    b.unwrap();
    assert!(requests.load(Ordering::SeqCst) > 0);
    let before = requests.load(Ordering::SeqCst);
    d.ensure_starter_candidates("default", false).await.unwrap();
    assert_eq!(before, requests.load(Ordering::SeqCst));
    // Switch network off: metadata is sufficient, selected songs still use the normal Rust resolver.
    sqlx::query("UPDATE application_settings SET value='false' WHERE key='enable_jiosaavn'")
        .execute(p.db_pool())
        .await
        .unwrap();
    let recs = d
        .get_discovery_recommendations("default", 20, false)
        .await
        .unwrap();
    assert!(!recs.is_empty());
    assert!(recs
        .iter()
        .all(|r| r.recommendation_reason.contains("Popular")));
    let home = p.flow_engine().get_home_overview("default").await.unwrap();
    assert!(!home.for_you.is_empty());
    let flow = p
        .flow_engine()
        .generate_playback_tracks_after("default", 0.5, 20, &[], None)
        .await
        .unwrap();
    assert!(!flow.is_empty());
    assert!(flow.iter().all(|t| t.reason_category == "starter_popular"));
    let mixes = p
        .smart_mix_generator()
        .ensure_default_mixes("default")
        .await
        .unwrap();
    assert!(mixes.iter().any(|m| m.mix_type.as_deref() == Some("daily")));
    assert!(mixes
        .iter()
        .any(|m| m.mix_type.as_deref() == Some("discovery")));
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM playback_history")
        .fetch_one(p.db_pool())
        .await
        .unwrap();
    assert_eq!(count, 0);
    assert_eq!(
        p.taste_engine()
            .compute_taste_profile("other")
            .await
            .unwrap()
            .artist_affinity("Favorite Artist 0"),
        0.0
    );
    server.abort();
}
#[tokio::test]
async fn genre_only_search_provenance_works_without_forging_song_metadata() {
    use music_player_backend::{
        database::repositories::SqliteWishlistRepository,
        discovery::DiscoveryCoordinator,
        providers::{jiosaavn::JioSaavnProvider, ProviderCoordinator},
        recommendations::setup,
    };
    use std::sync::Arc;
    let p = processor().await;
    let (endpoint, _, server) = catalog_server().await;
    setup::save(p.db_pool(), "default", vec![], vec!["Jazz".into()], false)
        .await
        .unwrap();
    sqlx::query("UPDATE application_settings SET value='true' WHERE key='enable_jiosaavn'")
        .execute(p.db_pool())
        .await
        .unwrap();
    let mut provider =
        ProviderCoordinator::new(p.db_pool().clone(), p.event_bus(), std::env::temp_dir());
    provider.jiosaavn = Arc::new(JioSaavnProvider::with_endpoint(&endpoint));
    let d = DiscoveryCoordinator::new(
        Arc::new(SqliteWishlistRepository::new(p.db_pool().clone())),
        p.library_service().track_repo(),
        p.taste_engine(),
        Some(Arc::new(provider)),
    );
    d.ensure_starter_candidates("default", false).await.unwrap();
    let flow = p
        .flow_engine()
        .generate_playback_tracks_after("default", 1.0, 10, &[], None)
        .await
        .unwrap();
    assert!(!flow.is_empty());
    assert!(flow.iter().all(|t| t.reason_category == "starter_genre"));
    let fake: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM external_tracks WHERE genre IS NOT NULL")
            .fetch_one(p.db_pool())
            .await
            .unwrap();
    assert_eq!(fake, 0);
    setup::save(
        p.db_pool(),
        "other",
        vec!["Other Artist".into()],
        vec![],
        false,
    )
    .await
    .unwrap();
    let profile = p
        .taste_engine()
        .compute_taste_profile("other")
        .await
        .unwrap();
    assert!(setup::members(p.db_pool(), "other", &profile)
        .await
        .unwrap()
        .is_empty());
    server.abort();
}

#[tokio::test]
async fn artist_only_choices_rank_starters_and_recorded_feedback_takes_precedence() {
    use music_player_backend::{
        database::repositories::SqliteWishlistRepository,
        discovery::DiscoveryCoordinator,
        providers::{jiosaavn::JioSaavnProvider, ProviderCoordinator},
        recommendations::setup,
    };
    use std::sync::Arc;
    let p = processor().await;
    let (endpoint, _, server) = catalog_server().await;
    setup::save(
        p.db_pool(),
        "default",
        vec!["Favorite Artist 0".into()],
        vec![],
        false,
    )
    .await
    .unwrap();
    sqlx::query("UPDATE application_settings SET value='true' WHERE key='enable_jiosaavn'")
        .execute(p.db_pool())
        .await
        .unwrap();
    let mut provider =
        ProviderCoordinator::new(p.db_pool().clone(), p.event_bus(), std::env::temp_dir());
    provider.jiosaavn = Arc::new(JioSaavnProvider::with_endpoint(&endpoint));
    let d = DiscoveryCoordinator::new(
        Arc::new(SqliteWishlistRepository::new(p.db_pool().clone())),
        p.library_service().track_repo(),
        p.taste_engine(),
        Some(Arc::new(provider)),
    );
    d.ensure_starter_candidates("default", false).await.unwrap();
    let tracks = p
        .flow_engine()
        .generate_playback_tracks_after("default", 1.0, 10, &[], None)
        .await
        .unwrap();
    let favorite = tracks
        .iter()
        .find(|t| t.artist == "Favorite Artist 0")
        .unwrap();
    assert_eq!(favorite.reason_category, "starter_artist");
    p.dispatch_command(Command::DislikeTrack {
        track_id: favorite.track_id.clone(),
    })
    .await
    .unwrap();
    let profile = p
        .taste_engine()
        .compute_taste_profile("default")
        .await
        .unwrap();
    assert_eq!(
        profile.artist_affinity("Favorite Artist 0"),
        0.0,
        "explicit seed must not overwrite a recorded zero affinity"
    );
    let next = p
        .flow_engine()
        .generate_playback_tracks_after("default", 0.5, 20, &[], None)
        .await
        .unwrap();
    assert!(
        !next.iter().any(|t| t.track_id == favorite.track_id),
        "starter pools must respect dislikes"
    );
    setup::save(p.db_pool(), "default", vec![], vec![], true)
        .await
        .unwrap();
    sqlx::query("UPDATE application_settings SET value='false' WHERE key='enable_jiosaavn'")
        .execute(p.db_pool())
        .await
        .unwrap();
    d.ensure_starter_candidates("default", false).await.unwrap();
    assert!(
        !p.flow_engine()
            .generate_playback_tracks_after("default", 0.5, 20, &[], None)
            .await
            .unwrap()
            .is_empty(),
        "skipping still uses cached public starters"
    );
    server.abort();
}
#[tokio::test]
async fn setup_persists_across_database_reopen_without_prompting_again() {
    use music_player_backend::{
        database::init_db_pool,
        recommendations::{setup, TasteProfileEngine},
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("music.sqlite");
    let pool = init_db_pool(&path).await.unwrap();
    setup::save(
        &pool,
        "listener",
        vec!["Favorite Artist".into()],
        vec!["Jazz".into()],
        false,
    )
    .await
    .unwrap();
    pool.close().await;
    let reopened = init_db_pool(&path).await.unwrap();
    let stored = setup::load(&reopened, "listener").await.unwrap();
    assert_eq!(stored.status, "completed");
    assert_eq!(stored.genres, vec!["Jazz"]);
    assert_eq!(
        TasteProfileEngine::new(reopened.clone())
            .compute_taste_profile("listener")
            .await
            .unwrap()
            .artist_affinity("Favorite Artist"),
        0.35
    );
    assert_eq!(
        setup::load(&reopened, "different-user")
            .await
            .unwrap()
            .status,
        "pending"
    );
}

async fn local(p: &CoreProcessor, title: &str, artist: &str) -> String {
    p.library_service()
        .track_repo()
        .save_scanned_track(ScannedMetadata {
            title_is_fallback: false,
            file_path: format!("/fixture/{artist}-{title}.flac"),
            file_size: 1,
            modified_timestamp: 0,
            file_hash: None,
            title: title.into(),
            artist: Some(artist.into()),
            album: None,
            album_artist: None,
            genre: Some("Rock".into()),
            track_number: None,
            disc_number: None,
            year: Some(2020),
            duration_secs: 240.0,
            bitrate: None,
            sample_rate: None,
            format: "flac".into(),
            has_cover_art: false,
            musicbrainz_track_id: None,
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn queued_unloaded_song_starts_with_toggle_and_paused_song_resumes_in_place() {
    let p = processor().await;
    let id = local(&p, "Queued Song", "Queued Artist").await;
    p.playback_service()
        .enqueue_track(&id, false)
        .await
        .unwrap();
    assert!(
        p.dispatch_command(Command::Resume).await.is_err(),
        "Resume cannot claim an unloaded session is playing"
    );
    p.dispatch_command(Command::TogglePlayPause).await.unwrap();
    let state = p.playback_service().get_playback_state().await;
    assert!(state.is_playing);
    assert_eq!(state.current_track_id, Some(id.clone()));
    p.dispatch_command(Command::Pause).await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM playback_history")
        .fetch_one(p.db_pool())
        .await
        .unwrap();
    p.dispatch_command(Command::TogglePlayPause).await.unwrap();
    assert!(p.playback_service().get_playback_state().await.is_playing);
    let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM playback_history")
        .fetch_one(p.db_pool())
        .await
        .unwrap();
    assert_eq!(count, after);
}

#[tokio::test]
async fn start_flow_bootstraps_a_fresh_catalog_before_selecting_and_uses_existing_audio_path() {
    use music_player_backend::{
        database::repositories::SqliteWishlistRepository,
        discovery::DiscoveryCoordinator,
        providers::{jiosaavn::JioSaavnProvider, ProviderCoordinator},
    };
    use sha2::{Digest, Sha256};
    use std::sync::Arc;
    let p = processor().await;
    let (endpoint, requests, server) = catalog_server().await;
    sqlx::query("UPDATE application_settings SET value='true' WHERE key='enable_jiosaavn'")
        .execute(p.db_pool())
        .await
        .unwrap();
    let mut provider =
        ProviderCoordinator::new(p.db_pool().clone(), p.event_bus(), std::env::temp_dir());
    provider.jiosaavn = Arc::new(JioSaavnProvider::with_endpoint(&endpoint));
    let coordinator = Arc::new(DiscoveryCoordinator::new(
        Arc::new(SqliteWishlistRepository::new(p.db_pool().clone())),
        p.library_service().track_repo(),
        p.taste_engine(),
        Some(Arc::new(provider)),
    ));
    p.playback_service().set_starter_preparation(coordinator);
    let settings =
        serde_json::to_value(p.execute_query(Query::GetSettings).await.unwrap()).unwrap();
    let folder =
        std::path::Path::new(settings["data"]["cache_dir"].as_str().unwrap()).join("remote-audio");
    tokio::fs::create_dir_all(&folder).await.unwrap();
    let samples = vec![0u8; 16000];
    let mut bytes = Vec::new();
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
    for i in 0..24 {
        let hash = format!(
            "{:x}",
            Sha256::digest(format!("full:v3:jiosaavn:starter{i}").as_bytes())
        );
        tokio::fs::write(folder.join(format!("{hash}.audio")), &bytes)
            .await
            .unwrap();
    }
    let tracks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM external_tracks")
        .fetch_one(p.db_pool())
        .await
        .unwrap();
    assert_eq!(tracks, 0);
    p.dispatch_command(Command::StartFlow {
        familiarity_ratio: None,
    })
    .await
    .unwrap();
    let state = p.playback_service().get_playback_state().await;
    assert!(state.is_playing);
    assert!(state.queue_length > 0);
    assert!(state
        .current_track_id
        .unwrap()
        .starts_with("jiosaavn:starter"));
    assert!(requests.load(std::sync::atomic::Ordering::SeqCst) > 0);
    p.dispatch_command(Command::Pause).await.unwrap();
    server.abort();
}

#[tokio::test]
async fn existing_account_with_positive_feedback_does_not_get_first_use_prompt() {
    let p = processor().await;
    let id = local(&p, "Liked Song", "Known Artist").await;
    p.dispatch_command(Command::LikeTrack { track_id: id })
        .await
        .unwrap();
    let setup = setup(&p).await;
    assert_eq!(setup["status"], "pending");
    assert_eq!(setup["should_prompt"], false);
}
