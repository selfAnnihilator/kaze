use music_player_backend::config::AppConfig;
use music_player_backend::core::command::Command;
use music_player_backend::core::processor::CoreProcessor;
use music_player_backend::core::query::{Query, QueryResponse};
use music_player_backend::database::create_in_memory_pool;
use music_player_backend::database::repositories::ScannedMetadata;
use music_player_backend::playback::backend::MockAudioBackend;

fn meta(title: &str, path: &str) -> ScannedMetadata {
    ScannedMetadata {
        title_is_fallback: false,
        file_path: path.into(),
        file_size: 1000,
        modified_timestamp: 100,
        file_hash: None,
        title: title.into(),
        artist: Some("Band".into()),
        album: Some("Album".into()),
        album_artist: None,
        genre: Some("Rock".into()),
        track_number: Some(1),
        disc_number: Some(1),
        year: Some(2020),
        duration_secs: 200.0,
        bitrate: Some(320),
        sample_rate: Some(44100),
        format: "wav".into(),
        has_cover_art: false,
        musicbrainz_track_id: None,
    }
}

async fn setup() -> (CoreProcessor, Vec<String>) {
    let pool = create_in_memory_pool().await.unwrap();
    let processor = CoreProcessor::new_with_backend(
        pool,
        AppConfig::default_with_dirs(),
        Box::new(MockAudioBackend::new()),
    );
    let repo = processor.library_service().track_repo();
    let mut ids = Vec::new();
    // Titles sort A < B < C so library order is deterministic.
    for (t, p) in [("A", "/m/a.wav"), ("B", "/m/b.wav"), ("C", "/m/c.wav")] {
        ids.push(repo.save_scanned_track(meta(t, p)).await.unwrap());
    }
    (processor, ids)
}

async fn state(p: &CoreProcessor) -> serde_json::Value {
    match p.execute_query(Query::GetPlaybackState).await.unwrap() {
        QueryResponse::PlaybackState(v) => v,
        _ => panic!("expected playback state"),
    }
}

#[tokio::test]
async fn play_all_follows_library_order_and_stops_at_end() {
    let (p, ids) = setup().await;
    p.dispatch_command(Command::PlayTrackList {
        track_ids: None,
        start_track_id: None,
        shuffle: false,
    })
    .await
    .unwrap();

    let s = state(&p).await;
    assert_eq!(s["queue_length"].as_u64().unwrap(), 3);
    assert_eq!(s["current_track_id"].as_str().unwrap(), ids[0]);

    for expected in [&ids[1], &ids[2]] {
        p.dispatch_command(Command::NextTrack).await.unwrap();
        assert_eq!(state(&p).await["current_track_id"].as_str().unwrap(), expected.as_str());
    }

    // Past the last track: stop, no random refill appended.
    p.dispatch_command(Command::NextTrack).await.unwrap();
    let s = state(&p).await;
    assert_eq!(s["queue_length"].as_u64().unwrap(), 3, "no refill at end of archive list");
    assert_eq!(s["is_playing"].as_bool(), Some(false));
}

#[tokio::test]
async fn start_track_and_shuffle_keep_the_same_set() {
    let (p, ids) = setup().await;
    p.dispatch_command(Command::PlayTrackList {
        track_ids: Some(ids.clone()),
        start_track_id: Some(ids[1].clone()),
        shuffle: false,
    })
    .await
    .unwrap();
    let s = state(&p).await;
    assert_eq!(s["current_track_id"].as_str().unwrap(), ids[1]);
    assert_eq!(s["current_queue_index"].as_u64().unwrap(), 1);

    p.dispatch_command(Command::PlayTrackList {
        track_ids: None,
        start_track_id: Some(ids[2].clone()),
        shuffle: true,
    })
    .await
    .unwrap();
    let s = state(&p).await;
    assert_eq!(s["queue_length"].as_u64().unwrap(), 3);
    assert_eq!(s["current_track_id"].as_str().unwrap(), ids[2], "start track leads a shuffled list");
    assert_eq!(s["current_queue_index"].as_u64().unwrap(), 0);
}
