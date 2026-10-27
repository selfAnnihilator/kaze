use music_player_backend::{
    cloud::SyncManager,
    core::EventBus,
    database::{
        create_in_memory_pool, init_db_pool,
        repositories::{ScannedMetadata, SqliteTrackRepository, TrackRepository},
    },
    providers::{
        enrichment::GapEnrichmentService,
        jiosaavn::{JioSaavnError, JioSaavnProvider},
        ProviderCoordinator,
    },
    recommendations::mixes::ensure_online_track,
};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

async fn server(
    body: Value,
    status: u16,
) -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let count = Arc::new(AtomicUsize::new(0));
    let requests = count.clone();
    let task = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut request = [0u8; 8192];
            let _ = socket.read(&mut request).await;
            requests.fetch_add(1, Ordering::SeqCst);
            let body = body.to_string();
            let response = format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });
    (url, count, task)
}

fn source(path: &str) -> ScannedMetadata {
    ScannedMetadata {
        title_is_fallback: true,
        file_path: path.into(),
        file_size: 1234,
        modified_timestamp: 100,
        file_hash: None,
        title: "Song".into(),
        artist: Some("Artist".into()),
        album: None,
        album_artist: None,
        genre: None,
        track_number: None,
        disc_number: None,
        year: None,
        duration_secs: 200.0,
        bitrate: Some(777),
        sample_rate: Some(48000),
        format: "flac".into(),
        has_cover_art: true,
        musicbrainz_track_id: None,
    }
}
fn candidate(id: &str, title: &str) -> Value {
    json!({"id":id,"song":title,"primary_artists":"Artist","album":"Album","albumid":"42","duration":"200","year":"2024","track_number":"3"})
}

#[tokio::test]
async fn song_search_excludes_unavailable_catalog_entries_without_extra_requests() {
    let mut playable = candidate("available", "Available Song");
    playable["rights"] = json!({"code":0});
    playable["encrypted_media_url"] = json!("ID2ieOjCrwfgWvL5sXl4B1ImC5QfbsDyryhkSYK5IH2E7FCO52VR6yhNbcEbes5iCcja4+W8xhE0SwtCJToN4Bw7tS9a8Gtq");
    let mut restricted = playable.clone();
    restricted["id"] = json!("restricted");
    restricted["rights"] = json!({"code":1});
    let metadata_only = candidate("metadataonly", "Metadata Only");
    let mut drm_only = metadata_only.clone();
    drm_only["id"] = json!("drmonly");
    drm_only["encrypted_drm_media_url"] = playable["encrypted_media_url"].clone();
    let mut preview_only = metadata_only.clone();
    preview_only["id"] = json!("previewonly");
    preview_only["media_preview_url"] = json!("https://example.invalid/preview.mp3");
    let mut invalid_media = playable.clone();
    invalid_media["id"] = json!("invalidmedia");
    invalid_media["encrypted_media_url"] = json!("invalid");
    let mut no_duration = playable.clone();
    no_duration["id"] = json!("noduration");
    no_duration["duration"] = Value::Null;
    let mut non_song = playable.clone();
    non_song["id"] = json!("album");
    non_song["type"] = json!("album");
    let (url, count, task) = server(json!({"results":[playable.clone(),restricted,metadata_only,drm_only,preview_only,invalid_media,no_duration,non_song,playable]}),200).await;
    let provider = JioSaavnProvider::with_endpoint(&url);
    let tracks = provider.search("Song", 35).await.unwrap();
    assert_eq!(
        tracks
            .iter()
            .map(|t| t.provider_track_id.as_str())
            .collect::<Vec<_>>(),
        vec!["available"]
    );
    assert_eq!(
        count.load(Ordering::SeqCst),
        1,
        "availability uses the search response, without audio downloads or per-song requests"
    );
    assert_eq!(
        provider.search_metadata("Song", 35).await.unwrap().len(),
        9,
        "metadata-only entries remain available for local-file enrichment"
    );
    assert_eq!(
        count.load(Ordering::SeqCst),
        1,
        "the metadata response is reused"
    );
    task.abort();
}

#[tokio::test]
async fn real_local_file_plays_despite_stale_online_format_and_disabled_jiosaavn() {
    use music_player_backend::{
        config::AppConfig,
        core::{Command, CoreProcessor},
        playback::backend::MockAudioBackend,
    };
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("original.flac");
    std::fs::write(&path, b"local file fixture").unwrap();
    let pool = create_in_memory_pool().await.unwrap();
    let repo = SqliteTrackRepository::new(pool.clone());
    let id = repo
        .save_scanned_track(source(path.to_str().unwrap()))
        .await
        .unwrap();
    sqlx::query("UPDATE tracks SET format='online' WHERE id=?")
        .bind(&id)
        .execute(&pool)
        .await
        .unwrap();
    let backend = MockAudioBackend::new();
    let loaded = backend.loaded_path.clone();
    let mut config = AppConfig::default_with_dirs();
    config.cache_dir = temp.path().join("cache");
    let core = CoreProcessor::new_with_backend(pool, config, Box::new(backend));
    core.dispatch_command(Command::SetMetadataPreference {
        key: "enable_jiosaavn".into(),
        enabled: false,
    })
    .await
    .unwrap();
    core.dispatch_command(Command::PlayTrack {
        track_id: id,
        source: Some("library".into()),
    })
    .await
    .unwrap();
    assert_eq!(loaded.lock().unwrap().as_ref(), Some(&path));
    assert_eq!(std::fs::read(&path).unwrap(), b"local file fixture");
    core.shutdown().await.unwrap();
}

#[tokio::test]
async fn trial_rejects_old_provider_requests() {
    use music_player_backend::{
        config::AppConfig,
        core::{CoreProcessor, Query},
        playback::backend::MockAudioBackend,
    };
    let temp = tempfile::tempdir().unwrap();
    let mut config = AppConfig::default_with_dirs();
    config.cache_dir = temp.path().join("cache");
    let core = CoreProcessor::new_with_backend(
        create_in_memory_pool().await.unwrap(),
        config,
        Box::new(MockAudioBackend::new()),
    );
    assert!(core
        .execute_query(Query::SearchOnlineMusic {
            query: "Song".into(),
            provider: Some("alternative".into()),
            limit: Some(5)
        })
        .await
        .is_err());
    for retired in [
        "ResolveFullTrackAudio",
        "LaunchSoulseek",
        "ImportSoulseekDownloads",
    ] {
        let payload = json!({"command":retired,"payload":{}});
        assert!(serde_json::from_value::<music_player_backend::core::Command>(payload).is_err());
    }
    core.shutdown().await.unwrap();
}

#[tokio::test]
async fn provider_reports_rate_limit_changed_shape_unavailable_and_caches_metadata() {
    let item = candidate("song1", "Song");
    let (url, count, task) = server(
        json!({"results":[item.clone()],"song1":item,"songs":[candidate("song1","Song")]}),
        200,
    )
    .await;
    let p = JioSaavnProvider::with_endpoint(&url);
    assert_eq!(p.search_metadata("Song", 10).await.unwrap().len(), 1);
    p.search_metadata("Song", 10).await.unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 1);
    p.detail("song1", false).await.unwrap();
    p.detail("song1", false).await.unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 2);
    p.detail("song1", true).await.unwrap();
    assert_eq!(count.load(Ordering::SeqCst), 3);
    assert!(matches!(
        p.resolve("song1", false).await,
        Err(JioSaavnError::Unavailable(_))
    ));
    assert!(p.detail("different", false).await.is_err());
    task.abort();
    for (status, body) in [
        (429, json!({})),
        (200, json!({"changed":[]})),
        (403, json!({})),
    ] {
        let (url, _, task) = server(body, status).await;
        let result = JioSaavnProvider::with_endpoint(&url)
            .search("Song", 10)
            .await;
        match status {
            429 => assert!(matches!(result, Err(JioSaavnError::RateLimited))),
            403 => assert!(matches!(result, Err(JioSaavnError::Unavailable(_)))),
            _ => assert!(matches!(result, Err(JioSaavnError::Malformed(_)))),
        }
        task.abort();
    }
}

#[tokio::test]
async fn enrichment_rescan_removal_and_offline_restart_preserve_source_tags() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("library.db");
    let pool = init_db_pool(&db).await.unwrap();
    let repo = SqliteTrackRepository::new(pool.clone());
    let original = source("/music/song.flac");
    let id = repo.save_scanned_track(original.clone()).await.unwrap();
    let item = candidate("song1", "Song");
    let (url, requests, task) = server(json!({"results":[item.clone()],"song1":item}), 200).await;
    let bus = Arc::new(EventBus::default());
    let mut providers = ProviderCoordinator::new(pool.clone(), bus.clone(), temp.path().into());
    providers.jiosaavn = Arc::new(JioSaavnProvider::with_endpoint(&url));
    let service =
        GapEnrichmentService::new(pool.clone(), Arc::new(providers), temp.path().into(), bus);
    assert!(service.enrich(&id).await.unwrap());
    let filled = repo.find_by_id(&id).await.unwrap().unwrap();
    assert_eq!(filled.year, Some(2024));
    assert_eq!(filled.album_title.as_deref(), Some("Album"));
    assert_eq!(filled.track_number, Some(3));
    assert_eq!(filled.duration_secs, 200.0);
    assert_eq!(filled.bitrate, Some(777));
    assert_eq!(filled.sample_rate, Some(48000));
    // Simulate interruption after provenance was prepared but before canonical merge.
    let request_count = requests.load(Ordering::SeqCst);
    sqlx::query("UPDATE track_enrichment SET state='applying' WHERE track_id=?")
        .bind(&id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE tracks SET year=NULL, track_number=NULL, album_id=NULL WHERE id=?")
        .bind(&id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(service.enrich(&id).await.unwrap());
    assert_eq!(
        repo.find_by_id(&id).await.unwrap().unwrap().year,
        Some(2024)
    );
    assert_eq!(
        requests.load(Ordering::SeqCst),
        request_count,
        "resume applies prepared metadata without another request"
    );
    let mut rescanned = original.clone();
    rescanned.year = Some(1999);
    rescanned.title = "SONG".into();
    rescanned.title_is_fallback = false;
    assert_eq!(repo.save_scanned_track(rescanned).await.unwrap(), id);
    let after = repo.find_by_id(&id).await.unwrap().unwrap();
    assert_eq!(after.year, Some(1999));
    assert_eq!(after.title, "SONG");
    task.abort();
    drop(service);
    drop(repo);
    pool.close().await;
    let pool = init_db_pool(&db).await.unwrap();
    let repo = SqliteTrackRepository::new(pool.clone());
    let after = repo.find_by_id(&id).await.unwrap().unwrap();
    assert_eq!(after.album_title.as_deref(), Some("Album"));
    assert_eq!(after.year, Some(1999));
    let bus = Arc::new(EventBus::default());
    let service = GapEnrichmentService::new(
        pool.clone(),
        Arc::new(ProviderCoordinator::new(
            pool.clone(),
            bus.clone(),
            temp.path().into(),
        )),
        temp.path().into(),
        bus,
    );
    service.remove_all().await.unwrap();
    let after = repo.find_by_id(&id).await.unwrap().unwrap();
    assert_eq!(after.album_title, None);
    assert_eq!(after.track_number, None);
    assert_eq!(after.year, Some(1999));
    assert_eq!(after.title, "SONG");
    assert_eq!(after.has_cover_art, 1);
    assert!(!service.enabled("auto_fill_metadata", true).await);
    pool.close().await;
}

#[tokio::test]
async fn enrichment_rejects_ambiguity_versions_and_insufficient_evidence() {
    for items in [
        vec![candidate("one", "Song"), candidate("two", "Song")],
        vec![candidate("one", "Song (Live)")],
    ] {
        let pool = create_in_memory_pool().await.unwrap();
        let temp = tempfile::tempdir().unwrap();
        sqlx::query("INSERT INTO application_settings(key,value,updated_at) VALUES ('enable_musicbrainz','false',0)").execute(&pool).await.unwrap();
        let (url, _, task) = server(json!({"results":items}), 200).await;
        let bus = Arc::new(EventBus::default());
        let mut providers = ProviderCoordinator::new(pool.clone(), bus.clone(), temp.path().into());
        providers.jiosaavn = Arc::new(JioSaavnProvider::with_endpoint(&url));
        let service =
            GapEnrichmentService::new(pool.clone(), Arc::new(providers), temp.path().into(), bus);
        let repo = SqliteTrackRepository::new(pool.clone());
        let id = repo
            .save_scanned_track(source("/music/song.flac"))
            .await
            .unwrap();
        assert!(!service.enrich(&id).await.unwrap());
        assert_eq!(repo.find_by_id(&id).await.unwrap().unwrap().year, None);
        let mut no_artist = source("/music/unknown.flac");
        no_artist.artist = None;
        let id = repo.save_scanned_track(no_artist).await.unwrap();
        assert!(!service.enrich(&id).await.unwrap());
        task.abort();
        pool.close().await;
    }
}

#[tokio::test]
async fn registration_and_cloud_roundtrip_keep_exact_provider_identity() {
    let pool = create_in_memory_pool().await.unwrap();
    ensure_online_track(
        &pool,
        "jiosaavn:abc123",
        "Song",
        Some("Artist"),
        None,
        Some(200.0),
        None,
        None,
    )
    .await
    .unwrap();
    ensure_online_track(
        &pool,
        "jiosaavn:abc123",
        "Song",
        Some("Artist"),
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();
    let identity: (String, String) = sqlx::query_as(
        "SELECT provider,provider_id FROM external_tracks WHERE id='jiosaavn:abc123'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(identity, ("jiosaavn".into(), "abc123".into()));
    sqlx::query("INSERT INTO track_statistics(user_id,track_id,manual_like) VALUES ('default','jiosaavn:abc123',1)").execute(&pool).await.unwrap();
    let payload = SyncManager::prepare_local_sync_payload(&pool, "default")
        .await
        .unwrap();
    let song = payload
        .songs
        .iter()
        .find(|s| s.id == "jiosaavn:abc123")
        .unwrap();
    assert_eq!(song.provider.as_deref(), Some("jiosaavn"));
    assert_eq!(song.provider_id.as_deref(), Some("abc123"));
    assert!(song.preview_url.is_none());
    let other = create_in_memory_pool().await.unwrap();
    SyncManager::apply_remote_sync_payload(&other, "default", &payload)
        .await
        .unwrap();
    let identity: (String, String) = sqlx::query_as(
        "SELECT provider,provider_id FROM external_tracks WHERE id='jiosaavn:abc123'",
    )
    .fetch_one(&other)
    .await
    .unwrap();
    assert_eq!(identity, ("jiosaavn".into(), "abc123".into()));
    pool.close().await;
    other.close().await;
}

#[tokio::test]
async fn validated_artwork_uses_existing_loader_after_offline_restart() {
    use music_player_backend::{
        config::AppConfig,
        core::{CoreProcessor, Query, QueryResponse},
        playback::backend::MockAudioBackend,
    };
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("art.db");
    let pool = init_db_pool(&db).await.unwrap();
    sqlx::query("INSERT INTO application_settings(key,value,updated_at) VALUES ('enable_jiosaavn','false',0),('auto_fill_metadata','false',0)").execute(&pool).await.unwrap();
    let image = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        2,
        2,
        image::Rgba([20, 40, 60, 255]),
    ));
    let mut png = std::io::Cursor::new(Vec::new());
    image.write_to(&mut png, image::ImageFormat::Png).unwrap();
    let png = png.into_inner();
    let repo = SqliteTrackRepository::new(pool.clone());
    let mut original = source("/music/with-art.flac");
    original.has_cover_art = false;
    let id = repo.save_scanned_track(original).await.unwrap();
    let art = temp.path().join("enrichment-artwork").join("song.png");
    std::fs::create_dir_all(art.parent().unwrap()).unwrap();
    std::fs::write(&art, png).unwrap();
    sqlx::query("INSERT INTO track_enrichment(track_id,provider,provider_id,cover_art_path,state,attempted_at) VALUES (?,'jiosaavn','song',?,'matched',0)")
        .bind(&id).bind(art.to_string_lossy().as_ref()).execute(&pool).await.unwrap();
    drop(repo);
    pool.close().await;
    let pool = init_db_pool(&db).await.unwrap();
    let mut config = AppConfig::default_with_dirs();
    config.cache_dir = temp.path().into();
    config.database_path = db;
    let core =
        CoreProcessor::new_with_backend(pool.clone(), config, Box::new(MockAudioBackend::new()));
    let art = core
        .execute_query(Query::GetTrackCoverArt {
            track_id: id.clone(),
        })
        .await
        .unwrap();
    let QueryResponse::CoverArt(Some(art)) = art else {
        panic!("persisted local artwork");
    };
    assert!(art.starts_with("data:image/png;base64,"));
    let payload = SyncManager::prepare_local_sync_payload(&pool, "default")
        .await
        .unwrap();
    assert!(!serde_json::to_string(&payload)
        .unwrap()
        .contains("enrichment-artwork"));
    core.shutdown().await.unwrap();
}

#[tokio::test]
async fn displayed_catalog_version_is_persistent_and_plays_without_matching_again() {
    use music_player_backend::{
        config::AppConfig,
        core::{Command, CoreProcessor, Query, QueryResponse},
        playback::{backend::MockAudioBackend, stream::StreamPlaybackManager},
        providers::online_catalog::OnlineCatalog,
    };
    let mut single = candidate("single", "Dance With Me");
    single["album"] = json!("Single");
    single["duration"] = json!(219);
    single["encrypted_media_url"] = json!("ID2ieOjCrwfgWvL5sXl4B1ImC5QfbsDyryhkSYK5IH2E7FCO52VR6yhNbcEbes5iCcja4+W8xhE0SwtCJToN4Bw7tS9a8Gtq");
    let mut album = single.clone();
    album["id"] = json!("albumversion");
    album["album"] = json!("The Romantic");
    let (url, count, server_task) = server(json!({"results":[single,album]}), 200).await;
    let temp = tempfile::tempdir().unwrap();
    let pool = create_in_memory_pool().await.unwrap();
    ensure_online_track(
        &pool,
        "itunes:old",
        "Dance With Me",
        Some("Artist"),
        Some("The Romantic"),
        Some(210.0),
        None,
        None,
    )
    .await
    .unwrap();
    // The selected provider song can already exist from a previous search.
    // Existing UNIQUE(provider,provider_id) must not prevent linking a saved alias.
    ensure_online_track(
        &pool,
        "jiosaavn:albumversion",
        "Dance With Me",
        Some("Artist"),
        Some("The Romantic"),
        Some(219.0),
        None,
        None,
    )
    .await
    .unwrap();
    let catalog = OnlineCatalog::new(
        pool.clone(),
        Arc::new(JioSaavnProvider::with_endpoint(&url)),
    );
    let shown = catalog.present(json!([{"track_id":"itunes:old","title":"Dance With Me","artist":"Artist","duration_secs":210}])).await.unwrap();
    assert_eq!(shown[0]["playback_source"]["track_id"], "albumversion");
    assert_eq!(shown[0]["album"], "The Romantic");
    assert_eq!(shown[0]["duration_secs"], 219.0);
    let original: (String, String) =
        sqlx::query_as("SELECT provider,provider_id FROM external_tracks WHERE id='itunes:old'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        original,
        ("online".into(), "itunes:old".into()),
        "the source link must not steal the unique identity of an existing provider row"
    );
    assert_eq!(count.load(Ordering::SeqCst), 1);
    server_task.abort();
    // A restart/offline catalog does not perform another title search or choose the other release.
    let reopened = OnlineCatalog::new(
        pool.clone(),
        Arc::new(JioSaavnProvider::with_endpoint("http://127.0.0.1:1")),
    );
    assert_eq!(
        reopened
            .bind("itunes:old")
            .await
            .unwrap()
            .unwrap()
            .provider_track_id,
        "albumversion"
    );
    let backend = MockAudioBackend::new();
    let loaded = backend.loaded_path.clone();
    let mut config = AppConfig::default_with_dirs();
    config.cache_dir = temp.path().join("cache");
    let core = CoreProcessor::new_with_backend(pool.clone(), config.clone(), Box::new(backend));
    let mut manager = StreamPlaybackManager::new(config.cache_dir.clone(), Some(pool), None);
    manager.catalog = Some(reopened);
    let cache = manager.get_cache_path("full:v3:jiosaavn:albumversion");
    std::fs::create_dir_all(manager.cache_dir()).unwrap();
    let data = vec![0u8; 32000];
    let mut wav = Vec::new();
    wav.extend(b"RIFF");
    wav.extend((36 + data.len() as u32).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(8000u32.to_le_bytes());
    wav.extend(16000u32.to_le_bytes());
    wav.extend(2u16.to_le_bytes());
    wav.extend(16u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend((data.len() as u32).to_le_bytes());
    wav.extend(data);
    std::fs::write(&cache, wav).unwrap();
    core.playback_service()
        .set_stream_manager(Arc::new(manager));
    core.dispatch_command(Command::PlayTrack {
        track_id: "itunes:old".into(),
        source: Some("playlist".into()),
    })
    .await
    .unwrap();
    assert_eq!(loaded.lock().unwrap().as_ref(), Some(&cache));
    let QueryResponse::PlaybackState(state) =
        core.execute_query(Query::GetPlaybackState).await.unwrap()
    else {
        panic!("state")
    };
    assert_eq!(
        state["current_track"]["playback_source"]["track_id"],
        "albumversion"
    );
    assert_eq!(state["current_track"]["duration_secs"], 219.0);
    core.shutdown().await.unwrap();
}

#[tokio::test]
async fn catalog_hides_unavailable_rows_but_preserves_saved_records_and_local_files() {
    use music_player_backend::providers::online_catalog::OnlineCatalog;
    let (url, _, task) = server(json!({"results":[]}), 200).await;
    let pool = create_in_memory_pool().await.unwrap();
    ensure_online_track(
        &pool,
        "itunes:missing",
        "Missing",
        Some("Artist"),
        None,
        Some(210.0),
        None,
        None,
    )
    .await
    .unwrap();
    let repo = SqliteTrackRepository::new(pool.clone());
    let local = repo
        .save_scanned_track(source("/music/keep.flac"))
        .await
        .unwrap();
    let catalog = OnlineCatalog::new(
        pool.clone(),
        Arc::new(JioSaavnProvider::with_endpoint(&url)),
    );
    let shown=catalog.present(json!([{"track_id":"itunes:missing","title":"Missing"},{"track_id":local,"title":"Local"}])).await.unwrap();
    assert_eq!(shown.as_array().unwrap().len(), 1);
    assert_eq!(shown[0]["title"], "Local");
    let retained: i64 = sqlx::query_scalar("SELECT count(*) FROM tracks WHERE id='itunes:missing'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(retained, 1, "unavailable saved items are not deleted");
    task.abort();
}

#[tokio::test]
async fn jiosaavn_charts_use_provider_ids_and_playlist_songs_keep_actual_metadata() {
    let (url, _, chart_server) = server(
        json!([
            {"id":"1134548194","title":"India Superhits Top 50"},
            {"title":"missing ID"}
        ]),
        200,
    )
    .await;
    let charts = JioSaavnProvider::with_endpoint(&url)
        .charts(false)
        .await
        .unwrap();
    assert_eq!(charts.len(), 1);
    assert_eq!(charts[0]["id"], "1134548194");
    chart_server.abort();
    let song = json!({"id":"provider-song","title":"Shown version","type":"song","language":"malayalam","year":"2026",
      "more_info":{"album":"Actual album","duration":"183","rights":{"code":"0"},
      "encrypted_media_url":"ID2ieOjCrwfgWvL5sXl4B1ImC5QfbsDyryhkSYK5IH2E7FCO52VR6yhNbcEbes5iCcja4+W8xhE0SwtCJToN4Bw7tS9a8Gtq",
      "artistMap":{"primary_artists":[{"name":"Primary Artist"},{"name":"Second Artist"}]}}});
    let (url, _, playlist_server) = server(
        json!({"list":[song.clone(),song,{"id":"metadata-only","title":"Unavailable"}]}),
        200,
    )
    .await;
    let provider = JioSaavnProvider::with_endpoint(&url);
    let songs = provider.playlist("123", 50, false).await.unwrap();
    assert_eq!(songs.len(), 1);
    assert_eq!(songs[0].provider_track_id, "provider-song");
    assert_eq!(songs[0].artist_name, "Primary Artist, Second Artist");
    assert_eq!(songs[0].language.as_deref(), Some("malayalam"));
    assert_eq!(songs[0].duration_secs, Some(183.0));
    assert!(provider.playlist("../old-source", 50, false).await.is_err());
    playlist_server.abort();
}

#[tokio::test]
async fn editorial_charts_send_language_cookie_parse_legacy_rows_and_refresh_cache() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let count = Arc::new(AtomicUsize::new(0));
    let english = Arc::new(AtomicUsize::new(0));
    let requests = count.clone();
    let international = english.clone();
    let server = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut buffer = [0u8; 8192];
            let length = socket.read(&mut buffer).await.unwrap();
            let request = String::from_utf8_lossy(&buffer[..length]).to_ascii_lowercase();
            requests.fetch_add(1, Ordering::SeqCst);
            let is_english = request.contains("cookie: l=english");
            if is_english {
                international.fetch_add(1, Ordering::SeqCst);
            }
            let body = if request.contains("getlaunchdata") {
                json!({"charts":[],"top_playlists":[{"listid":"3","listname":"Chartbusters - International","image":"https://example.test/global.jpg"}]})
            } else if is_english {
                json!([{"listid":"1","listname":"Most Searched Songs - English","image":"https://example.test/english-150x150.jpg"}])
            } else {
                json!([{"id":"2","title":"Hindi Top 50","image":"https://example.test/hindi.jpg"}])
            };
            let body = body.to_string();
            let response=format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
            socket.write_all(response.as_bytes()).await.unwrap();
        }
    });
    let provider = JioSaavnProvider::with_endpoint(&endpoint);
    let charts = provider.charts(false).await.unwrap();
    assert_eq!(
        charts.len(),
        3,
        "both response field shapes and international programming must survive"
    );
    assert!(charts
        .iter()
        .any(|chart| chart["title"] == "Chartbusters - International"));
    assert!(charts
        .iter()
        .any(|chart| chart["image"] == "https://example.test/english-500x500.jpg"));
    assert_eq!(english.load(Ordering::SeqCst), 2);
    provider.charts(false).await.unwrap();
    assert_eq!(
        count.load(Ordering::SeqCst),
        3,
        "normal reads reuse cached feeds"
    );
    provider.charts(true).await.unwrap();
    assert_eq!(
        count.load(Ordering::SeqCst),
        6,
        "refresh fetches each feed again"
    );
    server.abort();
}

#[tokio::test]
async fn artist_release_search_prefers_single_over_recent_compilation_without_mixing_artwork() {
    let mut compilation = candidate("compilation", "Breathe Me");
    compilation["primary_artists"] = json!("Sia");
    compilation["album"] = json!("Night Train");
    compilation["year"] = json!("2026");
    compilation["image"] = json!("https://example.test/train.jpg");
    compilation["encrypted_media_url"]=json!("ID2ieOjCrwfgWvL5sXl4B1ImC5QfbsDyryhkSYK5IH2E7FCO52VR6yhNbcEbes5iCcja4+W8xhE0SwtCJToN4Bw7tS9a8Gtq");
    compilation["is_drm"] = json!("0");
    let mut single = compilation.clone();
    single["id"] = json!("single");
    single["album"] = json!("Breathe Me");
    single["year"] = json!("2006");
    single["image"] = json!("https://example.test/sia-single.jpg");
    let (url, count, server_task) = server(json!({"results":[compilation,single]}), 200).await;
    let songs = JioSaavnProvider::with_endpoint(&url)
        .search("Breathe Me Sia", 25)
        .await
        .unwrap();
    assert_eq!(songs[0].provider_track_id, "single");
    assert_eq!(
        songs[0].cover_art_url.as_deref(),
        Some("https://example.test/sia-single.jpg")
    );
    assert_eq!(
        songs[1].provider_track_id, "compilation",
        "explicit compilation results remain accessible"
    );
    assert_eq!(
        count.load(Ordering::SeqCst),
        1,
        "search ranking must not add per-song requests"
    );
    server_task.abort();
}

#[tokio::test]
async fn artist_release_discovery_checks_album_credits_and_preserves_the_saved_compilation() {
    use music_player_backend::{
        config::AppConfig,
        core::{CoreProcessor, EventBus},
        database::repositories::SqliteWishlistRepository,
        discovery::DiscoveryCoordinator,
        playback::backend::MockAudioBackend,
        providers::{jiosaavn::parse_track, online_catalog::OnlineCatalog},
    };
    let mut compilation = candidate("compilation", "Breathe Me");
    compilation["primary_artists"] = json!("Sia");
    compilation["album"] = json!("Night Train");
    compilation["albumid"] = json!("99");
    compilation["year"] = json!("2026");
    compilation["image"] = json!("https://example.test/train.jpg");
    compilation["encrypted_media_url"]=json!("ID2ieOjCrwfgWvL5sXl4B1ImC5QfbsDyryhkSYK5IH2E7FCO52VR6yhNbcEbes5iCcja4+W8xhE0SwtCJToN4Bw7tS9a8Gtq");
    let mut single = compilation.clone();
    single["id"] = json!("single");
    single["album"] = json!("Artist Collection");
    single["albumid"] = json!("11");
    single["year"] = json!("2026");
    single["image"] = json!("https://example.test/sia-single.jpg");
    let source = parse_track(&compilation).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let count = Arc::new(AtomicUsize::new(0));
    let requests = count.clone();
    let server_task = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut buffer = [0u8; 8192];
            let n = socket.read(&mut buffer).await.unwrap();
            let request = String::from_utf8_lossy(&buffer[..n]);
            let path = request.split_whitespace().nth(1).unwrap();
            let url = reqwest::Url::parse(&format!("http://fixture{path}")).unwrap();
            let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
            requests.fetch_add(1, Ordering::SeqCst);
            let body=match query.get("__call").map(String::as_str) {
                Some("content.getAlbumDetails") => json!({"primary_artists":if query.get("albumid").is_some_and(|id|id=="99") {"Various Artists"} else {"Sia"}}),
                Some("search.getResults") if query.get("q").is_some_and(|q|q=="Breathe Me Sia") => json!({"results":[compilation,single]}),
                _=>json!({"results":[]}),
            }.to_string();
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
        }
    });
    let pool = create_in_memory_pool().await.unwrap();
    let temp = tempfile::tempdir().unwrap();
    let mut config = AppConfig::default_with_dirs();
    config.cache_dir = temp.path().join("cache");
    let core =
        CoreProcessor::new_with_backend(pool.clone(), config, Box::new(MockAudioBackend::new()));
    let mut providers = ProviderCoordinator::new(
        pool.clone(),
        Arc::new(EventBus::default()),
        temp.path().into(),
    );
    providers.jiosaavn = Arc::new(JioSaavnProvider::with_endpoint(&endpoint));
    let providers = Arc::new(providers);
    let saved = providers.persist(source).await.unwrap();
    ensure_online_track(
        &pool,
        &saved.id,
        &saved.title,
        Some(&saved.artist),
        saved.album.as_deref(),
        saved.duration_secs,
        saved.cover_art_url.as_deref(),
        None,
    )
    .await
    .unwrap();
    sqlx::query("INSERT INTO track_statistics(user_id,track_id,play_count,completion_count,manual_like,last_played_at) VALUES('default',?,12,10,1,?)").bind(&saved.id).bind(chrono::Utc::now().timestamp()).execute(&pool).await.unwrap();
    let discovery = DiscoveryCoordinator::new(
        Arc::new(SqliteWishlistRepository::new(pool.clone())),
        core.library_service().track_repo(),
        core.taste_engine(),
        Some(providers.clone()),
    );
    let recommendations = discovery
        .get_discovery_recommendations("default", 20, false)
        .await
        .unwrap();
    let preferred = recommendations
        .iter()
        .find(|song| song.title == "Breathe Me")
        .expect("artist-owned familiar release");
    assert_eq!(preferred.provider_id, "single");
    assert_eq!(
        preferred.cover_art_url.as_deref(),
        Some("https://example.test/sia-single.jpg")
    );
    let catalog = OnlineCatalog::new(pool.clone(), providers.jiosaavn.clone());
    let chosen = catalog
        .bind(&preferred.external_track_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(chosen.album_id.as_deref(), Some("11"));
    assert_eq!(chosen.year, Some(2026));
    let original = catalog.bind(&saved.id).await.unwrap().unwrap();
    assert_eq!(original.provider_track_id, "compilation");
    assert_eq!(
        original.cover_art_url.as_deref(),
        Some("https://example.test/train.jpg")
    );
    let calls = count.load(Ordering::SeqCst);
    discovery
        .get_discovery_recommendations("default", 20, false)
        .await
        .unwrap();
    assert_eq!(
        count.load(Ordering::SeqCst),
        calls,
        "normal refreshes reuse bounded edition/artist caches"
    );
    core.shutdown().await.unwrap();
    server_task.abort();
}
