//! Songs from the user's own folders take no like or dislike; Kaze downloads do.
use music_player_backend::config::AppConfig;
use music_player_backend::core::command::Command;
use music_player_backend::core::processor::CoreProcessor;
use music_player_backend::database::create_in_memory_pool;
use music_player_backend::database::repositories::ScannedMetadata;
use music_player_backend::playback::backend::MockAudioBackend;

async fn song(processor: &CoreProcessor, path: &str) -> String {
    processor
        .library_service()
        .track_repo()
        .save_scanned_track(ScannedMetadata {
            title_is_fallback: false,
            file_path: path.into(),
            file_size: 1000,
            modified_timestamp: 1,
            file_hash: None,
            title: path.rsplit('/').next().unwrap().into(),
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
            format: "flac".into(),
            has_cover_art: false,
            musicbrainz_track_id: None,
        })
        .await
        .unwrap()
}

async fn processor() -> CoreProcessor {
    let mut config = AppConfig::default_with_dirs();
    config.downloads.download_dir = Some("/music/Kaze".into());
    CoreProcessor::new_with_backend(create_in_memory_pool().await.unwrap(), config, Box::new(MockAudioBackend::new()))
}

#[tokio::test]
async fn a_local_file_cannot_be_liked_or_disliked() {
    let processor = processor().await;
    let local = song(&processor, "/music/fav/mine.flac").await;
    for command in [Command::LikeTrack { track_id: local.clone() }, Command::DislikeTrack { track_id: local.clone() }] {
        let error = processor.dispatch_command(command).await.expect_err("local files take no rating");
        assert!(error.to_string().contains("own folders"), "{error}");
    }
    // Clearing is always allowed (nothing to clear, but no error).
    processor.dispatch_command(Command::RemoveTrackFeedback { track_id: local }).await.unwrap();
}

#[tokio::test]
async fn a_kaze_download_can_be_liked() {
    let processor = processor().await;
    let download = song(&processor, "/music/Kaze/download.m4a").await;
    processor.dispatch_command(Command::LikeTrack { track_id: download }).await.unwrap();
}
