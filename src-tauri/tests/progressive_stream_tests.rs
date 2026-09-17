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
