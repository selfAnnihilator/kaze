//! A song the user queues plays before the app's own queue.
use music_player_backend::config::AppConfig;
use music_player_backend::core::command::Command;
use music_player_backend::core::processor::CoreProcessor;
use music_player_backend::core::query::{Query, QueryResponse};
use music_player_backend::database::create_in_memory_pool;
use music_player_backend::database::repositories::ScannedMetadata;
use music_player_backend::playback::backend::MockAudioBackend;

fn song(name: &str) -> ScannedMetadata {
    ScannedMetadata {
        title_is_fallback: false,
        file_path: format!("/music/{name}.wav"),
        file_size: 1000,
        modified_timestamp: 100,
        file_hash: None,
        title: name.into(),
        artist: Some("Artist".into()),
        album: None,
        album_artist: None,
        genre: None,
        track_number: None,
        disc_number: None,
        year: None,
        duration_secs: 200.0,
        bitrate: None,
        sample_rate: None,
        format: "wav".into(),
        has_cover_art: false,
        musicbrainz_track_id: None,
    }
}

async fn current_track(processor: &CoreProcessor) -> String {
    match processor.execute_query(Query::GetPlaybackState).await.expect("state") {
        QueryResponse::PlaybackState(state) => state["current_track_id"].as_str().unwrap_or_default().to_string(),
        _ => panic!("expected playback state"),
    }
}

#[tokio::test]
async fn a_song_the_user_queues_plays_before_the_rest_of_the_playlist() {
    let pool = create_in_memory_pool().await.expect("DB pool");
    let processor = CoreProcessor::new_with_backend(pool, AppConfig::default_with_dirs(), Box::new(MockAudioBackend::new()));
    let repo = processor.library_service().track_repo();
    let mut ids = Vec::new();
    for name in ["first", "second", "third", "picked"] {
        ids.push(repo.save_scanned_track(song(name)).await.expect("save"));
    }
    let [first, second, third, picked] = [&ids[0], &ids[1], &ids[2], &ids[3]];

    // A playlist plays: the app queues its remaining songs.
    processor
        .dispatch_command(Command::PlayTrack { track_id: first.clone(), source: Some("playlist".into()) })
        .await
        .expect("play");
    for id in [second, third] {
        processor
            .dispatch_command(Command::EnqueueTrack { track_id: id.clone(), play_next: false, manual: false })
            .await
            .expect("app enqueue");
    }
    // The user queues a song: it plays next, then the playlist continues.
    processor
        .dispatch_command(Command::EnqueueTrack { track_id: picked.clone(), play_next: false, manual: true })
        .await
        .expect("user enqueue");

    for expected in [picked, second, third] {
        processor.dispatch_command(Command::NextTrack).await.expect("next");
        assert_eq!(&current_track(&processor).await, expected);
    }
}

#[tokio::test]
async fn queuing_a_song_the_app_already_queued_plays_it_next() {
    let pool = create_in_memory_pool().await.expect("DB pool");
    let processor = CoreProcessor::new_with_backend(pool, AppConfig::default_with_dirs(), Box::new(MockAudioBackend::new()));
    let repo = processor.library_service().track_repo();
    let mut ids = Vec::new();
    for name in ["playing", "recommended-1", "recommended-2"] {
        ids.push(repo.save_scanned_track(song(name)).await.expect("save"));
    }
    processor
        .dispatch_command(Command::PlayTrack { track_id: ids[0].clone(), source: None })
        .await
        .expect("play");
    for id in &ids[1..] {
        processor
            .dispatch_command(Command::EnqueueTrack { track_id: id.clone(), play_next: false, manual: false })
            .await
            .expect("app enqueue");
    }
    processor
        .dispatch_command(Command::EnqueueTrack { track_id: ids[2].clone(), play_next: false, manual: true })
        .await
        .expect("the user may queue a song the app queued");

    match processor.execute_query(Query::GetPlaybackState).await.expect("state") {
        QueryResponse::PlaybackState(state) => {
            assert_eq!(state["queue_track_ids"], serde_json::json!([ids[2], ids[1]]));
            assert_eq!(state["user_queue_track_ids"], serde_json::json!([ids[2]]), "only the user's songs");
        }
        _ => panic!("expected playback state"),
    }
    processor.dispatch_command(Command::NextTrack).await.expect("next");
    assert_eq!(current_track(&processor).await, ids[2]);
}

#[tokio::test]
async fn clear_queue_keeps_the_playing_song() {
    let pool = create_in_memory_pool().await.expect("DB pool");
    let processor = CoreProcessor::new_with_backend(pool, AppConfig::default_with_dirs(), Box::new(MockAudioBackend::new()));
    let repo = processor.library_service().track_repo();
    let playing = repo.save_scanned_track(song("playing")).await.expect("save");
    let queued = repo.save_scanned_track(song("queued")).await.expect("save");
    processor
        .dispatch_command(Command::PlayTrack { track_id: playing.clone(), source: None })
        .await
        .expect("play");
    processor
        .dispatch_command(Command::EnqueueTrack { track_id: queued, play_next: false, manual: true })
        .await
        .expect("enqueue");
    processor.dispatch_command(Command::ClearQueue).await.expect("clear");

    assert_eq!(current_track(&processor).await, playing, "the active song keeps playing");
    match processor.execute_query(Query::GetPlaybackState).await.expect("state") {
        QueryResponse::PlaybackState(state) => {
            assert_eq!(state["queue_track_ids"], serde_json::json!([]));
            assert_eq!(state["user_queue_track_ids"], serde_json::json!([]));
            assert_eq!(state["is_playing"], true);
        }
        _ => panic!("expected playback state"),
    }
}

#[test]
fn the_manual_flag_is_optional_in_ipc() {
    let command: Command = serde_json::from_str(r#"{"command":"EnqueueTrack","payload":{"track_id":"a","play_next":false}}"#)
        .expect("old payload still parses");
    assert!(matches!(command, Command::EnqueueTrack { manual: false, .. }));
}
