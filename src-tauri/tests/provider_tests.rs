use music_player_backend::config::AppConfig;
use music_player_backend::core::command::Command;
use music_player_backend::core::event::Event;
use music_player_backend::core::processor::CoreProcessor;
use music_player_backend::database::create_in_memory_pool;
use music_player_backend::database::repositories::ScannedMetadata;
use music_player_backend::playback::backend::MockAudioBackend;
use music_player_backend::providers::{
    CoverArtArchiveProvider, MetadataProvider, MusicBrainzProvider, ProviderCoordinator, SpotifyProvider,
};
use std::sync::Arc;
use tempfile::tempdir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::test]
async fn test_provider_status_emission() {
    let pool = create_in_memory_pool().await.expect("DB pool");
    let mock_backend = Box::new(MockAudioBackend::new());
    let processor = CoreProcessor::new_with_backend(pool, AppConfig::default_with_dirs(), mock_backend);

    let event_bus = processor.event_bus();
    let mut rx = event_bus.subscribe();

    processor.provider_coordinator().emit_status();

    let mut seen_providers = Vec::new();
    for _ in 0..3 {
        if let Ok(Event::ProviderStatusChanged { provider, .. }) = rx.recv().await {
            seen_providers.push(provider);
        }
    }

    assert!(seen_providers.contains(&"musicbrainz".to_string()));
    assert!(seen_providers.contains(&"cover_art_archive".to_string()));
    assert!(seen_providers.contains(&"spotify".to_string()));
}

#[tokio::test]
async fn test_cover_art_archive_disk_caching() {
    let temp_dir = tempdir().expect("temp dir");
    let cache_path = temp_dir.path().to_path_buf();
    let provider = CoverArtArchiveProvider::new(cache_path.clone());

    let test_mbid = "a1b2c3d4-e5f6-7890-abcd-ef1234567890";
    let cached_file_path = provider.get_cached_path(test_mbid);

    // Pre-seed disk cache with fake JPEG bytes
    tokio::fs::create_dir_all(cached_file_path.parent().unwrap())
        .await
        .expect("create artwork dir");
    let fake_image_data = b"FAKE_JPEG_BINARY_DATA";
    tokio::fs::write(&cached_file_path, fake_image_data)
        .await
        .expect("write cached image");

    // Fetch cover art: must hit disk cache without remote network access
    let fetched = provider
        .fetch_cover_art(test_mbid)
        .await
        .expect("fetch cached artwork");

    assert!(fetched.is_some(), "Artwork should be found in cache");
    assert_eq!(fetched.unwrap(), fake_image_data);
}

#[tokio::test]
async fn test_spotify_unconfigured_graceful_degradation() {
    let provider = SpotifyProvider::new(None, None);

    assert!(!provider.is_available(), "Spotify should report unavailable when credentials are None");

    let tracks = provider
        .search_track("Test Track", "Test Artist", None)
        .await
        .expect("search track when unconfigured");
    assert!(tracks.is_empty(), "Should return empty vector cleanly without errors");

    let artist = provider
        .search_artist("Test Artist")
        .await
        .expect("search artist when unconfigured");
    assert!(artist.is_none(), "Should return None cleanly");

    let album = provider
        .search_album("Test Album", "Test Artist")
        .await
        .expect("search album when unconfigured");
    assert!(album.is_none(), "Should return None cleanly");
}

#[tokio::test]
async fn test_provider_enrichment_flow_with_mock_server() {
    // 1. Spawn a local mock HTTP server that simulates MusicBrainz API
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind mock server");
    let mock_addr = listener.local_addr().expect("mock addr");
    let mock_base_url = format!("http://{}", mock_addr);

    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = [0u8; 2048];
            let _ = socket.read(&mut buf).await;

            let response_body = serde_json::json!({
                "recordings": [
                    {
                        "id": "rec-12345-mbid",
                        "title": "Comfortably Numb",
                        "length": 362000,
                        "artist-credit": [
                            {
                                "name": "Pink Floyd",
                                "artist": { "id": "art-999-mbid" }
                            }
                        ],
                        "releases": [
                            {
                                "id": "rel-777-mbid",
                                "title": "The Wall",
                                "date": "1979-11-30"
                            }
                        ]
                    }
                ]
            });

            let body_str = serde_json::to_string(&response_body).unwrap();
            let http_response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body_str.len(),
                body_str
            );

            let _ = socket.write_all(http_response.as_bytes()).await;
        }
    });

    // 2. Setup environment with custom coordinator pointing to mock server
    let pool = create_in_memory_pool().await.expect("DB pool");
    let mock_backend = Box::new(MockAudioBackend::new());
    let processor = CoreProcessor::new_with_backend(pool.clone(), AppConfig::default_with_dirs(), mock_backend);

    let temp_dir = tempdir().expect("temp dir");
    let mb_provider = Arc::new(MusicBrainzProvider::with_base_url(&mock_base_url));
    let cover_art_provider = Arc::new(CoverArtArchiveProvider::new(temp_dir.path().to_path_buf()));
    let spotify_provider = Arc::new(SpotifyProvider::new(None, None));

    let coordinator = Arc::new(ProviderCoordinator::with_providers(
        pool.clone(),
        processor.event_bus(),
        mb_provider,
        cover_art_provider,
        spotify_provider,
    ));

    // 3. Save a track with missing external IDs
    let track_repo = processor.library_service().track_repo();
    let track_id = track_repo
        .save_scanned_track(ScannedMetadata {
            file_path: "/music/comfortably_numb.flac".into(),
            file_size: 5000,
            modified_timestamp: 100,
            file_hash: None,
            title: "Comfortably Numb".into(),
            artist: Some("Pink Floyd".into()),
            album: Some("The Wall".into()),
            album_artist: None,
            genre: Some("Progressive Rock".into()),
            track_number: Some(6),
            disc_number: Some(2),
            year: None,
            duration_secs: 362.0,
            bitrate: Some(1000),
            sample_rate: Some(44100),
            format: "flac".into(),
            has_cover_art: false,
            musicbrainz_track_id: None,
        })
        .await
        .expect("save track");

    // 4. Run enrichment via coordinator
    let enriched = coordinator.enrich_track(&track_id).await.expect("enrich track");
    assert!(enriched, "Track should be marked enriched");

    // 5. Verify database was updated with MusicBrainz ID
    let updated_track = track_repo.find_by_id(&track_id).await.expect("find track").expect("track exists");
    assert_eq!(updated_track.musicbrainz_track_id, Some("rec-12345-mbid".to_string()));

    // 6. Test dispatching Command::TriggerMetadataRefresh
    let res = processor
        .dispatch_command(Command::TriggerMetadataRefresh {
            track_id: track_id.clone(),
        })
        .await
        .expect("trigger metadata refresh command");

    assert!(matches!(res, music_player_backend::core::command::CommandResponse::Ok));
}
