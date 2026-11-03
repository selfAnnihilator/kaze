use super::*;
use std::time::{Duration, Instant};

#[test]
fn preparation_subprocess() {
    // Concurrent cargo builds can replace the original test executable while
    // later cases launch. Own a copy for the entire subprocess regression.
    let binary_dir = tempfile::tempdir().unwrap();
    let binary = binary_dir.path().join("preparation-tests");
    #[cfg(target_os = "linux")]
    let source_binary = std::path::PathBuf::from("/proc/self/exe");
    #[cfg(not(target_os = "linux"))]
    let source_binary = std::env::current_exe().unwrap();
    std::fs::copy(source_binary, &binary).unwrap();
    for mode in ["delayed", "truncated", "cancel", "stop", "next", "replace", "quit", "stalled", "deadline"] {
        let mut child = std::process::Command::new(&binary)
            .args(["--exact", "playback::stream::preparation_tests::fixture_child", "--ignored", "--nocapture"])
            .env("KAZE_PREPARATION_CASE", mode).env("NO_PROXY", "127.0.0.1,localhost").env("no_proxy", "127.0.0.1,localhost").spawn().unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "fixture {mode} failed");
                break;
            }
            if start.elapsed() > Duration::from_secs(if matches!(mode, "stalled" | "deadline") { 38 } else { 8 }) {
                child.kill().unwrap(); child.wait().unwrap();
                panic!("fixture {mode}: preparation exceeded external 8s deadline");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

#[test]
#[ignore = "isolated child, run by preparation_subprocess"]
fn fixture_child() {
    let mode = std::env::var("KAZE_PREPARATION_CASE").unwrap();
    tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap().block_on(async {
        use tokio::io::AsyncWriteExt;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/fixture.wav", listener.local_addr().unwrap());
        // Valid PCM WAV with a large legal JUNK chunk before fmt/data:
        // probing must read beyond the initial 64/128 KiB.
        let mut wav = b"RIFF".to_vec();
        wav.extend((36u32 + 8 + 262144 + 32000).to_le_bytes()); wav.extend(b"WAVEJUNK");
        wav.extend(262144u32.to_le_bytes()); wav.resize(wav.len()+262144, 0);
        wav.extend(b"fmt "); wav.extend(16u32.to_le_bytes()); wav.extend(1u16.to_le_bytes()); wav.extend(1u16.to_le_bytes());
        wav.extend(8000u32.to_le_bytes()); wav.extend(16000u32.to_le_bytes()); wav.extend(2u16.to_le_bytes()); wav.extend(16u16.to_le_bytes());
        wav.extend(b"data"); wav.extend(32000u32.to_le_bytes()); wav.resize(wav.len()+32000, 0);
        let server_mode = mode.clone();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 4096];
            tokio::io::AsyncReadExt::read(&mut socket, &mut request).await.unwrap();
            let header = format!("HTTP/1.1 200 OK\r\nContent-Type: audio/wav\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", wav.len());
            socket.write_all(header.as_bytes()).await.unwrap();
            socket.write_all(&wav[..65536]).await.unwrap();
            tokio::time::sleep(Duration::from_millis(150)).await;
            if server_mode == "delayed" { socket.write_all(&wav[65536..]).await.unwrap(); }
            else if server_mode != "truncated" {
                if server_mode == "deadline" {
                    for _ in 0..10 {
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        if socket.write_all(&[0; 16]).await.is_err() { break; }
                    }
                } else { tokio::time::sleep(Duration::from_secs(60)).await; }
            }
        });
        let dir = tempfile::tempdir().unwrap();
        let mut fixture_manager = StreamPlaybackManager::new(dir.path().into(), None, None);
        fixture_manager.http_client = reqwest::Client::builder().no_proxy().build().unwrap();
        let manager = Arc::new(fixture_manager);
        let target = dir.path().join("fixture.audio"); let part = dir.path().join("fixture.part");
        if matches!(mode.as_str(), "stop" | "next" | "replace" | "quit") {
            use crate::{config::AppConfig, core::{processor::CoreProcessor, command::Command}, playback::backend::MockAudioBackend};
            let mut config = AppConfig::default_with_dirs();
            config.database_path = dir.path().join("profile.db");
            config.cache_dir = dir.path().join("cache");
            let pool = crate::database::init_db_pool(&config.database_path).await.unwrap();
            let core = Arc::new(CoreProcessor::new_with_backend(pool, config, Box::new(MockAudioBackend::new())));
            core.playback_service().set_stream_manager(manager.clone());
            manager.resolution_cache.insert("full:v3:jiosaavn:fixture".into(), ResolvedEntry::new(url.clone(), 2.0, None));
            sqlx::query("INSERT INTO tracks(id,file_path,file_size,modified_timestamp,title,normalized_title,duration_secs,format,created_at,updated_at) VALUES('jiosaavn:fixture','online://fixture',1,1,'fixture','fixture',2,'online',1,1),('local','/missing/local.wav',1,1,'local','local',2,'wav',1,1)")
                .execute(core.db_pool()).await.unwrap();
            let mut events = core.event_bus().subscribe();
            let playing = core.clone();
            let task = tokio::spawn(async move { playing.dispatch_command(Command::PlayTrack { track_id: "jiosaavn:fixture".into(), source: None }).await });
            tokio::time::timeout(Duration::from_secs(3), async {
                loop {
                    let ready = manager.preparation.lock().unwrap().as_ref()
                        .and_then(|p| p.state.lock().unwrap().clone()).is_some_and(|s| s.is_buffering());
                    if ready { break; }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }).await.unwrap();
            tokio::time::timeout(Duration::from_secs(5), async {
                match mode.as_str() {
                    "stop" => { core.dispatch_command(Command::Stop).await.unwrap(); },
                    "next" => {
                        core.dispatch_command(Command::EnqueueTrack { track_id: "local".into(), play_next: false }).await.unwrap();
                        core.dispatch_command(Command::NextTrack).await.unwrap();
                    }
                    "replace" => { core.dispatch_command(Command::PlayTrack { track_id: "local".into(), source: None }).await.unwrap(); },
                    _ => { core.begin_quit(); core.shutdown().await.unwrap(); }
                }
                task.await.unwrap().ok();
            }).await.expect("command/shutdown must interrupt preparation within five seconds");
            while let Ok(event) = events.try_recv() {
                if let crate::core::event::Event::PlaybackStarted { track_id, .. } = event { assert_ne!(track_id, "jiosaavn:fixture"); }
            }
            if mode != "quit" { core.shutdown().await.unwrap(); }
            manager.cancel_active_stream().await;
            server.abort();
            return;
        }
        let cancel_manager = manager.clone();
        if mode == "cancel" { tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(350)).await;
            cancel_manager.cancel_active_stream().await;
        }); }
        let preparation = manager.begin_preparation();
        let result = manager.finish_preparation(&preparation, manager.try_start_progressive(&url, &part, &target, "fixture", 2.0, &preparation)).await;
        if mode == "delayed" {
            let PreparedPlayback::ProgressiveStream { mut source, .. } = result.unwrap() else { panic!("expected progressive source") };
            let count = tokio::task::spawn_blocking(move || source.by_ref().count()).await.unwrap();
            assert_eq!(count, 16000);
        } else { assert!(result.is_err(), "{mode} must return an error"); }
        manager.cancel_active_stream().await;
        assert!(!std::fs::read_dir(dir.path()).unwrap().any(|f| f.unwrap().path().extension().is_some_and(|e| e == "part")), "partial file leaked");
        server.abort();
    });
}

#[tokio::test]
#[ignore = "isolated live provider check; no personal profile"]
async fn reported_house_of_balloons_live() {
    use crate::{config::AppConfig, core::{processor::CoreProcessor, command::Command}, playback::backend::MockAudioBackend};
    let dir = tempfile::tempdir().unwrap();
    let mut config = AppConfig::default_with_dirs();
    config.database_path = dir.path().join("live.db");
    config.cache_dir = dir.path().join("cache");
    let pool = crate::database::init_db_pool(&config.database_path).await.unwrap();
    let core = CoreProcessor::new_with_backend(pool, config, Box::new(MockAudioBackend::new()));
    let manager = Arc::new(StreamPlaybackManager::new(dir.path().join("audio"), None, None));
    core.playback_service().set_stream_manager(manager.clone());
    let found = tokio::time::timeout(Duration::from_secs(30), manager.jiosaavn.search("House of Balloons The Weeknd", 10)).await;
    match found {
        Ok(Ok(tracks)) => {
            let song = tracks.iter().find(|track| track.title.to_lowercase().contains("house of balloons") && track.artist_name.to_lowercase().contains("weeknd"))
                .expect("provider results must include the exact reported recording");
            let id = format!("jiosaavn:{}", song.provider_track_id);
            sqlx::query("INSERT INTO tracks(id,file_path,file_size,modified_timestamp,title,normalized_title,duration_secs,format,created_at,updated_at) VALUES(?,?,1,1,?,?,?,'online',1,1)")
                .bind(&id).bind(format!("online://{}", song.provider_track_id)).bind(&song.title).bind(song.title.to_lowercase()).bind(song.duration_secs)
                .execute(core.db_pool()).await.unwrap();
            let result = tokio::time::timeout(Duration::from_secs(35), core.dispatch_command(Command::PlayTrack { track_id: id, source: None })).await.unwrap();
            println!("Reported song preparation: {}", if result.is_ok() { "decoded initial provider audio" } else { "provider failure surfaced as playback error" });
            if let Err(error) = result { println!("Playback error: {error}"); }
        }
        _ => println!("Reported song: provider search unavailable within deadline; playback not demonstrated"),
    }
    tokio::time::timeout(Duration::from_secs(5), core.dispatch_command(Command::Stop)).await.unwrap().unwrap();
    tokio::time::timeout(Duration::from_secs(5), core.shutdown()).await.unwrap().unwrap();
    println!("Control and graceful shutdown remain available");
}
