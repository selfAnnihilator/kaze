use super::*;
use crate::{
    database::create_in_memory_pool, playback::backend::MockAudioBackend,
    providers::jiosaavn::JioSaavnProvider,
};
use std::sync::atomic::AtomicUsize;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

async fn playlist_fixture() -> (
    tempfile::TempDir,
    CoreProcessor,
    SqlitePool,
    Arc<AtomicUsize>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let count = Arc::new(AtomicUsize::new(0));
    let requests = count.clone();
    let task = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut request = Vec::new();
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                while !request.ends_with(b"\r\n\r\n") {
                    assert!(request.len() < 16 * 1024, "header too large");
                    let mut byte = [0];
                    assert_eq!(socket.read(&mut byte).await.unwrap(), 1, "EOF before headers");
                    request.push(byte[0]);
                }
            }).await.expect("request header timeout");
            requests.fetch_add(1, Ordering::SeqCst);
            let body = serde_json::json!({"id":"123","listname":"Fixture playlist","list":[{
                "id":"selected", "song":"Song", "primary_artists":"Artist", "album":"Selected release",
                "duration":"200", "rights":{"code":0},
                "encrypted_media_url":"ID2ieOjCrwfgWvL5sXl4B1ImC5QfbsDyryhkSYK5IH2E7FCO52VR6yhNbcEbes5iCcja4+W8xhE0SwtCJToN4Bw7tS9a8Gtq"
            }]}).to_string();
            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            socket.write_all(response.as_bytes()).await.unwrap();
        }
    });
    let temp = tempfile::tempdir().unwrap();
    let pool = create_in_memory_pool().await.unwrap();
    let mut config = AppConfig::default_with_dirs();
    config.cache_dir = temp.path().join("cache");
    let mut core = CoreProcessor::new_with_backend(
        pool.clone(),
        config.clone(),
        Box::new(MockAudioBackend::new()),
    );
    // Replace only the import dependency; no production endpoint/configuration override.
    let mut provider =
        ProviderCoordinator::new(pool.clone(), core.event_bus.clone());
    provider.jiosaavn = Arc::new(JioSaavnProvider::with_endpoint(&endpoint));
    core.provider_coordinator = Arc::new(provider);
    (temp, core, pool, count, task)
}

async fn counts(pool: &SqlitePool) -> (i64, i64, i64) {
    sqlx::query_as("SELECT (SELECT count(*) FROM external_tracks), (SELECT count(*) FROM online_track_sources), (SELECT count(*) FROM tracks)")
        .fetch_one(pool).await.unwrap()
}

#[tokio::test]
async fn disabled_playlist_import_makes_no_requests_or_writes_and_can_be_reenabled() {
    let (_temp, core, pool, requests, server) = playlist_fixture().await;
    core.dispatch_command(Command::SetMetadataPreference {
        key: "enable_jiosaavn".into(),
        enabled: false,
    })
    .await
    .unwrap();
    let before = counts(&pool).await;
    let error = core
        .execute_query(Query::ImportJioSaavnPlaylist {
            url_or_id: "123".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Validation(ref message) if message.contains("Settings")));
    assert_eq!(requests.load(Ordering::SeqCst), 0);
    assert_eq!(counts(&pool).await, before);
    core.dispatch_command(Command::SetMetadataPreference {
        key: "enable_jiosaavn".into(),
        enabled: true,
    })
    .await
    .unwrap();
    let response = core
        .execute_query(Query::ImportJioSaavnPlaylist {
            url_or_id: "123".into(),
        })
        .await
        .unwrap();
    let QueryResponse::JioSaavnPlaylistImport(value) = response else {
        panic!("expected import")
    };
    assert_eq!(value["total_tracks"], 1);
    assert_eq!(value["tracks"][0]["track_id"], "jiosaavn:selected");
    assert_eq!(value["tracks"][0]["album"], "Selected release");
    assert_eq!(requests.load(Ordering::SeqCst), 1);
    assert_eq!(counts(&pool).await.0, 1);
    assert_eq!(counts(&pool).await.1, 1);
    core.shutdown().await.unwrap();
    server.abort();
}

#[tokio::test]
async fn playlist_import_remains_enabled_when_setting_is_absent() {
    let (_temp, core, pool, requests, server) = playlist_fixture().await;
    assert!(sqlx::query_scalar::<_, String>(
        "SELECT value FROM application_settings WHERE key='enable_jiosaavn'"
    )
    .fetch_optional(&pool)
    .await
    .unwrap()
    .is_none());
    let response = core
        .execute_query(Query::ImportJioSaavnPlaylist {
            url_or_id: "123".into(),
        })
        .await
        .unwrap();
    let QueryResponse::JioSaavnPlaylistImport(value) = response else {
        panic!("expected import")
    };
    assert_eq!(value["playlist_id"], "123");
    assert_eq!(value["total_tracks"], 1);
    assert_eq!(requests.load(Ordering::SeqCst), 1);
    core.shutdown().await.unwrap();
    server.abort();
}

#[tokio::test]
async fn spotify_playlist_import_matches_onto_jiosaavn_and_reports_unmatched() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                if socket.read(&mut byte).await.unwrap_or(0) == 0 {
                    break;
                }
                request.push(byte[0]);
            }
            let head = String::from_utf8_lossy(&request).to_string();
            let (kind, body) = if head.starts_with("GET /embed/playlist/") {
                ("text/html", crate::providers::spotify_embed::fixture_html(
                    r#"{"title":"Song - Original Version","subtitle":"Artist, Guest","duration":201000,"isPlayable":true},{"title":"Nowhere","subtitle":"Nobody","duration":180000,"isPlayable":true}"#,
                ))
            } else if head.contains("q=Song") {
                ("application/json", serde_json::json!({"results":[{
                    "id":"matched", "song":"Song", "primary_artists":"Artist", "album":"Album",
                    "duration":"200", "rights":{"code":0},
                    "encrypted_media_url":"ID2ieOjCrwfgWvL5sXl4B1ImC5QfbsDyryhkSYK5IH2E7FCO52VR6yhNbcEbes5iCcja4+W8xhE0SwtCJToN4Bw7tS9a8Gtq"
                }]}).to_string())
            } else {
                ("application/json", serde_json::json!({"results":[]}).to_string())
            };
            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            let _ = socket.write_all(response.as_bytes()).await;
        }
    });
    let temp = tempfile::tempdir().unwrap();
    let pool = create_in_memory_pool().await.unwrap();
    let mut config = AppConfig::default_with_dirs();
    config.cache_dir = temp.path().join("cache");
    let mut core = CoreProcessor::new_with_backend(pool.clone(), config, Box::new(MockAudioBackend::new()));
    let mut provider = ProviderCoordinator::new(pool.clone(), core.event_bus.clone());
    provider.jiosaavn = Arc::new(JioSaavnProvider::with_endpoint(&endpoint));
    provider.spotify = Arc::new(crate::providers::spotify_embed::SpotifyEmbedClient::with_endpoint(&endpoint));
    core.provider_coordinator = Arc::new(provider);

    let response = core
        .execute_query(Query::ImportJioSaavnPlaylist {
            url_or_id: "https://open.spotify.com/playlist/37i9dQZF1DXcBWIGoYBM5M?si=x".into(),
        })
        .await
        .unwrap();
    let QueryResponse::JioSaavnPlaylistImport(value) = response else {
        panic!("expected import")
    };
    assert_eq!(value["source"], "spotify");
    assert_eq!(value["title"], "Fixture mix");
    assert_eq!(value["total_tracks"], 1);
    assert_eq!(value["source_total"], 2);
    assert_eq!(value["tracks"][0]["track_id"], "jiosaavn:matched");
    assert_eq!(value["unmatched"][0]["title"], "Nowhere");
    assert_eq!(counts(&pool).await.0, 1);

    let error = core
        .execute_query(Query::ImportJioSaavnPlaylist { url_or_id: "https://example.com/x".into() })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("JioSaavn or Spotify"));
    core.shutdown().await.unwrap();
    server.abort();
}
