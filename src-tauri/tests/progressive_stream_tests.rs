use music_player_backend::playback::progressive::{ProgressiveStreamReader, ProgressiveStreamState};
use music_player_backend::playback::stream::{RemoteAudioCacheConfig, StreamPlaybackManager};
use std::io::Read;
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn test_progressive_reader_incremental_read_and_wake() {
    let dir = tempdir().unwrap();
    let part_path = dir.path().join("test.part");
    std::fs::write(&part_path, b"initial_bytes").unwrap();

    let state = Arc::new(ProgressiveStreamState::new("test-track".to_string(), None));
    state.update_downloaded(13);
    let mut reader = ProgressiveStreamReader::new(&part_path, state.clone()).unwrap();

    let mut buf = [0u8; 13];
    reader.read_exact(&mut buf).unwrap();
    assert_eq!(&buf, b"initial_bytes");

    // Spawn a producer thread that writes more data after a short sleep
    let state_clone = state.clone();
    let part_clone = part_path.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&part_clone)
            .unwrap();
        use std::io::Write;
        file.write_all(b"_additional_data").unwrap();
        file.flush().unwrap();
        state_clone.update_downloaded(13 + 16);
        std::thread::sleep(Duration::from_millis(20));
        state_clone.set_completed();
    });

    let mut second_buf = [0u8; 16];
    reader.read_exact(&mut second_buf).unwrap();
    assert_eq!(&second_buf, b"_additional_data");

    // Should read 0 bytes (EOF) now that state is completed
    let mut eof_buf = [0u8; 10];
    let n = reader.read(&mut eof_buf).unwrap();
    assert_eq!(n, 0);
}

#[test]
fn test_progressive_reader_cancellation_terminates_promptly() {
    let dir = tempdir().unwrap();
    let part_path = dir.path().join("cancel_test.part");
    std::fs::write(&part_path, b"partial").unwrap();

    let state = Arc::new(ProgressiveStreamState::new("test-cancel".to_string(), None));
    state.update_downloaded(7);
    let mut reader = ProgressiveStreamReader::new(&part_path, state.clone()).unwrap();

    let mut buf = [0u8; 7];
    reader.read_exact(&mut buf).unwrap();

    // Reader has consumed all available bytes.
    // Producer cancels download instead of completing.
    let state_clone = state.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        state_clone.set_cancelled();
    });

    let mut next_buf = [0u8; 10];
    let start = std::time::Instant::now();
    let res = reader.read(&mut next_buf);
    assert!(res.is_err(), "Reader must return error on cancellation");
    assert_eq!(res.unwrap_err().kind(), std::io::ErrorKind::Interrupted);
    assert!(start.elapsed() < Duration::from_secs(2), "Reader wait loop must wake promptly on cancel");
}

#[test]
fn test_progressive_reader_failure_terminates_promptly() {
    let dir = tempdir().unwrap();
    let part_path = dir.path().join("fail_test.part");
    std::fs::write(&part_path, b"header").unwrap();

    let state = Arc::new(ProgressiveStreamState::new("test-fail".to_string(), None));
    state.update_downloaded(6);
    let mut reader = ProgressiveStreamReader::new(&part_path, state.clone()).unwrap();

    let mut buf = [0u8; 6];
    reader.read_exact(&mut buf).unwrap();

    // Producer fails
    let state_clone = state.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        state_clone.set_failed("HTTP 500 Internal Server Error".to_string());
    });

    let mut next_buf = [0u8; 10];
    let start = std::time::Instant::now();
    let res = reader.read(&mut next_buf);
    assert!(res.is_err(), "Reader must return error on failure");
    assert_eq!(res.unwrap_err().kind(), std::io::ErrorKind::BrokenPipe);
    assert!(start.elapsed() < Duration::from_secs(2), "Reader wait loop must wake promptly on failure");
}

#[test]
fn test_progressive_reader_buffering_state_transitions() {
    let dir = tempdir().unwrap();
    let part_path = dir.path().join("buf_test.part");
    std::fs::write(&part_path, b"initial").unwrap();

    let state = Arc::new(ProgressiveStreamState::new("test-buf".to_string(), None));
    state.update_downloaded(7);
    let mut reader = ProgressiveStreamReader::new(&part_path, state.clone()).unwrap();

    let mut buf = [0u8; 7];
    reader.read_exact(&mut buf).unwrap();
    assert!(!state.is_buffering());

    // Reader attempts to read more when no more bytes exist
    // It should enter buffering state while waiting
    let state_clone = state.clone();
    let part_clone = part_path.clone();
    let handle = std::thread::spawn(move || {
        // Wait until reader has starved and entered buffering
        for _ in 0..50 {
            if state_clone.is_buffering() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(state_clone.is_buffering(), "State must enter buffering when reader is starved");

        // Write new bytes and update
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&part_clone)
            .unwrap();
        use std::io::Write;
        file.write_all(b"_resumed").unwrap();
        file.flush().unwrap();
        state_clone.update_downloaded(7 + 8);
        state_clone.set_completed();
    });

    let mut next_buf = [0u8; 8];
    reader.read_exact(&mut next_buf).unwrap();
    assert_eq!(&next_buf, b"_resumed");
    assert!(!state.is_buffering(), "State must exit buffering once new data is consumed");

    handle.join().unwrap();
}

#[tokio::test]
async fn test_progressive_cancellation_removes_part_file() {
    let dir = tempdir().unwrap();
    let manager = StreamPlaybackManager::with_config(
        dir.path().to_path_buf(),
        None,
        None,
        RemoteAudioCacheConfig::default(),
    );

    let part_path = dir.path().join("active_stream.part");
    tokio::fs::write(&part_path, b"partial data in progress").await.unwrap();
    assert!(part_path.exists());

    // Initially can_seek_current is true (no stream active)
    assert!(manager.can_seek_current().await);
    manager.cancel_active_stream().await;
    assert!(manager.can_seek_current().await);
}

#[test]
fn completed_stream_announces_seekability_without_resuming_playback() {
    use music_player_backend::core::{event::Event, event_bus::EventBus};
    let bus = Arc::new(EventBus::new(8));
    let mut events = bus.subscribe();
    let state = ProgressiveStreamState::new("paused-track".into(), Some(bus));
    // This can happen while paused: there are no position ticks to rely on.
    assert!(!state.is_completed());
    state.set_completed();
    assert!(state.is_completed());
    assert_eq!(events.try_recv().unwrap(), Event::PlaybackCapabilitiesChanged { track_id: "paused-track".into() });
    assert!(events.try_recv().is_err(), "completion must not emit fake resume/position activity");
}

#[test]
fn download_terminal_states_clear_a_waiting_readers_buffering_flag() {
    use std::sync::atomic::Ordering;
    for terminal in ["completed", "failed", "cancelled"] {
        let state = ProgressiveStreamState::new("current-song".into(), None);
        state.is_buffering.store(true, Ordering::SeqCst);
        match terminal {
            "completed" => state.set_completed(),
            "failed" => state.set_failed("connection lost".into()),
            _ => state.set_cancelled(),
        }
        assert!(!state.is_buffering(), "{terminal} must clear the buffering indicator");
    }
}

#[test]
fn preparing_a_stream_does_not_publish_player_transport_events() {
    use music_player_backend::core::event_bus::EventBus;
    let dir = tempdir().unwrap();
    let path = dir.path().join("preparing.part");
    std::fs::write(&path, b"").unwrap();
    let bus = Arc::new(EventBus::new(16));
    let mut events = bus.subscribe();
    let state = Arc::new(ProgressiveStreamState::new("not-current-yet".into(), Some(bus)));
    let mut reader = ProgressiveStreamReader::new(&path, state.clone()).unwrap();
    let producer = state.clone();
    let handle = std::thread::spawn(move || {
        for _ in 0..100 {
            if producer.is_buffering() { break; }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(producer.is_buffering());
        producer.set_cancelled();
    });
    assert!(reader.read(&mut [0; 1]).is_err());
    handle.join().unwrap();
    assert!(events.try_recv().is_err(), "preparing/obsolete readers must not alter the current song's UI");
}

#[test]
fn current_reader_buffering_is_session_scoped_and_recovers_without_resume() {
    use music_player_backend::core::{event::Event, event_bus::EventBus};
    use std::sync::atomic::Ordering;
    let bus = Arc::new(EventBus::new(16));
    let mut events = bus.subscribe();
    let state = ProgressiveStreamState::new("song".into(), Some(bus));
    state.activate_playback("current-session".into());
    assert_eq!(events.try_recv().unwrap(), Event::PlaybackBufferingChanged {
        session_id: "current-session".into(), track_id: "song".into(), is_buffering: false,
    });
    // A waiting decoder has reached the available bytes. EOF (without another
    // chunk) must clear this state, even if the user paused in the meantime.
    state.is_buffering.store(true, Ordering::SeqCst);
    state.set_completed();
    assert_eq!(events.try_recv().unwrap(), Event::PlaybackBufferingChanged {
        session_id: "current-session".into(), track_id: "song".into(), is_buffering: false,
    });
    assert!(matches!(events.try_recv().unwrap(), Event::PlaybackCapabilitiesChanged { .. }));
    assert!(events.try_recv().is_err(), "EOF must not emit a fake resume or reset position");
    state.deactivate_playback();
    state.is_buffering.store(true, Ordering::SeqCst);
    state.set_cancelled();
    assert!(events.try_recv().is_err(), "an old session must become silent");
}

#[test]
fn active_stream_can_seek_before_download_completion() {
    use std::io::{Seek, SeekFrom};
    use symphonia::core::io::MediaSource;
    let dir = tempdir().unwrap();
    let path = dir.path().join("seek.part");
    std::fs::write(&path, b"abcdefgh").unwrap();
    let state = Arc::new(ProgressiveStreamState::new("seek".into(), None));
    state.content_length.store(16, std::sync::atomic::Ordering::Release);
    state.update_downloaded(8);
    let mut reader = ProgressiveStreamReader::new(&path, state.clone()).unwrap();
    assert!(!reader.is_seekable(), "probing must not seek into unavailable data before the downloader starts");
    state.activate_playback("session".into());
    assert!(reader.is_seekable(), "active streaming playback must allow seeking");
    assert_eq!(reader.seek(SeekFrom::End(-4)).unwrap(), 12);
    let producer = state.clone();
    let writer = std::thread::spawn(move || {
        for _ in 0..500 {
            if producer.is_buffering() { break; }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(producer.is_buffering());
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().append(true).open(path).unwrap();
        file.write_all(b"ijklmnop").unwrap();
        file.flush().unwrap();
        producer.update_downloaded(16);
        // Keep downloading: seeking must work without a Completed transition.
    });
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes).unwrap();
    assert_eq!(&bytes, b"mnop");
    writer.join().unwrap();
    reader.seek(SeekFrom::Start(0)).unwrap();
    reader.read_exact(&mut bytes).unwrap();
    assert_eq!(&bytes, b"abcd");
    assert!(!state.is_completed());
}

#[test]
fn decoder_seeks_forward_and_backward_on_an_active_partial_wave() {
    use music_player_backend::playback::decoder::SymphoniaSource;
    use rodio::Source;
    let dir = tempdir().unwrap();
    let path = dir.path().join("decode.part");
    let mut wav = Vec::new();
    wav.extend(b"RIFF"); wav.extend((36 + 160000u32).to_le_bytes());
    wav.extend(b"WAVEfmt "); wav.extend(16u32.to_le_bytes());
    wav.extend(1u16.to_le_bytes()); wav.extend(1u16.to_le_bytes());
    wav.extend(8000u32.to_le_bytes()); wav.extend(16000u32.to_le_bytes());
    wav.extend(2u16.to_le_bytes()); wav.extend(16u16.to_le_bytes());
    wav.extend(b"data"); wav.extend(160000u32.to_le_bytes());
    for i in 0..80000 { wav.extend(((i / 8000 + 1) as i16).to_le_bytes()); }
    // Only the first half has arrived. Probe while not yet active.
    std::fs::write(&path, &wav[..80044]).unwrap();
    let state = Arc::new(ProgressiveStreamState::new("decode".into(), None));
    state.content_length.store(wav.len() as u64, std::sync::atomic::Ordering::Release);
    state.update_downloaded(80044);
    let reader = ProgressiveStreamReader::new(&path, state.clone()).unwrap();
    let mut source = SymphoniaSource::from_media_source(Box::new(reader), Default::default(), "test").unwrap();
    state.activate_playback("session".into());
    source.try_seek(Duration::from_secs(3)).unwrap();
    assert_eq!(source.next(), Some(4));
    source.try_seek(Duration::from_secs(1)).unwrap();
    assert_eq!(source.next(), Some(2));
    assert!(!state.is_completed());
}

#[test]
fn seeking_into_missing_data_wakes_on_cancel_or_failure() {
    use std::io::{Seek, SeekFrom};
    for failure in [false, true] {
        let dir = tempdir().unwrap();
        let path = dir.path().join("waiting.part");
        std::fs::write(&path, b"abc").unwrap();
        let state = Arc::new(ProgressiveStreamState::new("waiting".into(), None));
        state.update_downloaded(3);
        state.activate_playback("session".into());
        let mut reader = ProgressiveStreamReader::new(&path, state.clone()).unwrap();
        reader.seek(SeekFrom::Start(100)).unwrap();
        let producer = state.clone();
        let handle = std::thread::spawn(move || {
            for _ in 0..500 {
                if producer.is_buffering() { break; }
                std::thread::sleep(Duration::from_millis(2));
            }
            assert!(producer.is_buffering());
            if failure { producer.set_failed("connection lost".into()); }
            else { producer.set_cancelled(); }
        });
        let started = std::time::Instant::now();
        let error = reader.read(&mut [0; 1]).unwrap_err();
        assert_eq!(error.kind(), if failure { std::io::ErrorKind::BrokenPipe } else { std::io::ErrorKind::Interrupted });
        handle.join().unwrap();
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(!state.can_seek());
        assert!(!state.is_buffering());
    }
}

#[test]
fn compressed_decoder_seeks_while_download_is_incomplete() {
    use music_player_backend::playback::decoder::SymphoniaSource;
    use rodio::Source;
    for (extension, audio) in [
        ("mp3", include_bytes!("fixtures/seek-tone.mp3").as_slice()),
        ("m4a", include_bytes!("fixtures/seek-tone.m4a").as_slice()),
    ] {
        let dir = tempdir().unwrap();
        let path = dir.path().join("compressed.part");
        let split = audio.len() * 3 / 4;
        std::fs::write(&path, &audio[..split]).unwrap();
        let state = Arc::new(ProgressiveStreamState::new(extension.into(), None));
        state.content_length.store(audio.len() as u64, std::sync::atomic::Ordering::Release);
        state.update_downloaded(split as u64);
        let reader = ProgressiveStreamReader::new(&path, state.clone()).unwrap();
        let mut hint = symphonia::core::probe::Hint::new();
        hint.with_extension(extension);
        let mut source = SymphoniaSource::from_media_source(Box::new(reader), hint, extension).unwrap();
        state.activate_playback("session".into());
        for second in [4, 1] {
            source.try_seek(Duration::from_secs(second)).unwrap();
            assert!(source.by_ref().take(2400).any(|sample| sample != 0), "{extension} must decode after seek");
        }
        let producer = state.clone();
        let tail = audio[split..].to_vec();
        let handle = std::thread::spawn(move || {
            // The forward seek must wait for real bytes rather than seeing EOF.
            for _ in 0..500 {
                if producer.is_buffering() { break; }
                std::thread::sleep(Duration::from_millis(2));
            }
            assert!(producer.is_buffering());
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new().append(true).open(path).unwrap();
            file.write_all(&tail).unwrap();
            file.flush().unwrap();
            producer.update_downloaded(audio.len() as u64);
        });
        source.try_seek(Duration::from_secs(10)).unwrap();
        assert!(source.take(2400).any(|sample| sample != 0));
        handle.join().unwrap();
        assert!(!state.is_completed(), "{extension} did not require full-cache promotion");
    }
}

#[test]
#[ignore = "requires native audio output; run with a disposable ALSA null device"]
fn native_paused_stream_seeks_before_completion() {
    use music_player_backend::playback::{
        backend::{AudioBackend, RodioAudioBackend},
        decoder::{PlayerSource, SymphoniaSource},
    };
    let audio = include_bytes!("fixtures/seek-tone.m4a");
    let dir = tempdir().unwrap();
    let path = dir.path().join("native.part");
    let split = audio.len() * 3 / 4;
    std::fs::write(&path, &audio[..split]).unwrap();
    let state = Arc::new(ProgressiveStreamState::new("native".into(), None));
    state.content_length.store(audio.len() as u64, std::sync::atomic::Ordering::Release);
    state.update_downloaded(split as u64);
    let reader = ProgressiveStreamReader::new(&path, state.clone()).unwrap();
    let source = SymphoniaSource::from_media_source(Box::new(reader), Default::default(), "native").unwrap();
    let mut backend = RodioAudioBackend::try_new().unwrap();
    backend.set_volume(0.0).unwrap();
    backend.load_and_play_source(PlayerSource::Symphonia(source), Some(&path)).unwrap();
    state.activate_playback("native-session".into());
    backend.pause().unwrap();
    for second in [4, 1] {
        backend.seek(Duration::from_secs(second)).unwrap();
        assert!(backend.is_paused());
        assert!((backend.position().as_secs_f64() - second as f64).abs() < 0.5);
    }
    let producer = state.clone();
    let tail = audio[split..].to_vec();
    let handle = std::thread::spawn(move || {
        for _ in 0..500 {
            if producer.is_buffering() { break; }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(producer.is_buffering());
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().append(true).open(path).unwrap();
        file.write_all(&tail).unwrap();
        file.flush().unwrap();
        producer.update_downloaded(audio.len() as u64);
    });
    backend.seek(Duration::from_secs(10)).unwrap();
    handle.join().unwrap();
    assert!(backend.is_paused());
    assert!((backend.position().as_secs_f64() - 10.0).abs() < 0.5);
    assert!(!state.is_completed());
    backend.resume().unwrap();
    std::thread::sleep(Duration::from_millis(100));
    assert!(!backend.is_finished());
    backend.stop().unwrap();
}
