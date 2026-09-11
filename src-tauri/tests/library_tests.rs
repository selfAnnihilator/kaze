use music_player_backend::config::AppConfig;
use music_player_backend::core::command::{Command, CommandResponse};
use music_player_backend::core::processor::CoreProcessor;
use music_player_backend::core::query::{Query, QueryResponse};
use music_player_backend::database::create_in_memory_pool;
use music_player_backend::library::scanner::{is_supported_audio_format, is_within_boundary};
use std::fs::{create_dir_all, write};
use std::path::Path;

/// Helper creating a minimal valid PCM WAV audio file readable by lofty.
fn create_valid_test_wav(path: &Path) {
    let mut data = Vec::new();
    let pcm_bytes = 44100 * 2 * 2; // 1 second of 16-bit stereo at 44.1kHz
    let chunk_size = 36u32 + pcm_bytes as u32;

    // RIFF chunk header
    data.extend_from_slice(b"RIFF");
    data.extend_from_slice(&chunk_size.to_le_bytes());
    data.extend_from_slice(b"WAVE");

    // fmt subchunk
    data.extend_from_slice(b"fmt ");
    data.extend_from_slice(&16u32.to_le_bytes()); // Subchunk1Size (16 for PCM)
    data.extend_from_slice(&1u16.to_le_bytes());  // AudioFormat (1 for PCM)
    data.extend_from_slice(&2u16.to_le_bytes());  // NumChannels (2)
    data.extend_from_slice(&44100u32.to_le_bytes()); // SampleRate
    data.extend_from_slice(&(44100u32 * 4).to_le_bytes()); // ByteRate
    data.extend_from_slice(&4u16.to_le_bytes());  // BlockAlign
    data.extend_from_slice(&16u16.to_le_bytes()); // BitsPerSample

    // data subchunk
    data.extend_from_slice(b"data");
    data.extend_from_slice(&(pcm_bytes as u32).to_le_bytes());
    data.resize(data.len() + pcm_bytes, 0u8);

    write(path, data).expect("Failed to write mock WAV");
}

#[tokio::test]
async fn test_onboarding_and_default_music_dir() {
    let pool = create_in_memory_pool().await.expect("DB pool");
    let processor = CoreProcessor::new(pool, AppConfig::default_with_dirs());

    // 1. Initial query: onboarding not completed
    let query_res = processor
        .execute_query(Query::GetOnboardingStatus)
        .await
        .expect("Query failed");

    match query_res {
        QueryResponse::OnboardingStatus {
            completed,
            default_music_dir,
            configured_folders,
        } => {
            assert!(!completed, "Onboarding should initially be false");
            assert!(!default_music_dir.is_empty(), "Default music dir should be populated");
            assert_eq!(configured_folders.len(), 0, "No folders should be configured yet");
        }
        _ => panic!("Expected OnboardingStatus response"),
    }

    // 2. Complete onboarding with a custom directory
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let temp_dir_str = temp_dir.path().to_string_lossy().to_string();

    let cmd_res = processor
        .dispatch_command(Command::CompleteOnboarding {
            music_folders: vec![temp_dir_str.clone()],
            start_scan: false,
        })
        .await
        .expect("Onboarding command failed");

    match cmd_res {
        CommandResponse::OnboardingCompleted { configured_folders } => {
            assert_eq!(configured_folders, 1);
        }
        _ => panic!("Expected OnboardingCompleted response"),
    }

    // 3. Second query: onboarding is now completed and folder is registered
    let query_res2 = processor
        .execute_query(Query::GetOnboardingStatus)
        .await
        .expect("Query failed");

    match query_res2 {
        QueryResponse::OnboardingStatus {
            completed,
            configured_folders,
            ..
        } => {
            assert!(completed, "Onboarding should now be true");
            assert_eq!(configured_folders.len(), 1);
        }
        _ => panic!("Expected OnboardingStatus response"),
    }
}

#[test]
fn test_strict_boundary_containment() {
    let temp_root = tempfile::tempdir().expect("root tempdir");
    let root_path = temp_root.path();

    let child_sub = root_path.join("SubFolder").join("DeepSub");
    create_dir_all(&child_sub).expect("create child sub");

    let valid_child_file = child_sub.join("test_song.flac");
    write(&valid_child_file, b"dummy content").expect("write child file");

    let outside_temp = tempfile::tempdir().expect("outside tempdir");
    let outside_file = outside_temp.path().join("external_song.mp3");
    write(&outside_file, b"outside content").expect("write outside file");

    // Child is inside boundary
    assert!(is_within_boundary(root_path, &child_sub));
    assert!(is_within_boundary(root_path, &valid_child_file));

    // External file is strictly outside boundary
    assert!(!is_within_boundary(root_path, &outside_file));
    assert!(!is_within_boundary(root_path, outside_temp.path()));
}

#[test]
fn test_supported_audio_formats() {
    assert!(is_supported_audio_format(Path::new("song.mp3")));
    assert!(is_supported_audio_format(Path::new("track.flac")));
    assert!(is_supported_audio_format(Path::new("audio.OGG")));
    assert!(is_supported_audio_format(Path::new("recording.opus")));
    assert!(is_supported_audio_format(Path::new("music.m4a")));
    assert!(is_supported_audio_format(Path::new("sound.wav")));
    assert!(!is_supported_audio_format(Path::new("photo.jpg")));
    assert!(!is_supported_audio_format(Path::new("document.pdf")));
    assert!(!is_supported_audio_format(Path::new("executable.exe")));
}

#[tokio::test]
async fn test_scanner_incremental_and_search() {
    let pool = create_in_memory_pool().await.expect("DB pool");
    let processor = CoreProcessor::new(pool, AppConfig::default_with_dirs());
    let library_service = processor.library_service();

    // Create a mock music library directory structure
    let root_dir = tempfile::tempdir().expect("root tempdir");
    let album_sub = root_dir.path().join("Deftones").join("White Pony");
    create_dir_all(&album_sub).expect("create album dir");

    let track1_path = album_sub.join("01_Change.wav");
    let track2_path = album_sub.join("02_Digital_Bath.wav");
    create_valid_test_wav(&track1_path);
    create_valid_test_wav(&track2_path);

    // Register folder
    let folder_str = root_dir.path().to_string_lossy().to_string();
    let folder_rec = library_service
        .add_folder(&folder_str)
        .await
        .expect("add folder failed");

    // 1. First scan (non-incremental)
    let summaries = library_service
        .scan_library(Some(folder_rec.id.clone()), false)
        .await
        .expect("scan failed");

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].added_tracks, 2);
    assert_eq!(summaries[0].updated_tracks, 0);
    assert_eq!(summaries[0].unchanged_tracks, 0);

    // Verify tracks exist in DB
    let track_count = library_service.track_repo().get_track_count().await.expect("count failed");
    assert_eq!(track_count, 2);

    // 2. Second scan (incremental): should skip unchanged tracks!
    let summaries_inc = library_service
        .scan_library(Some(folder_rec.id.clone()), true)
        .await
        .expect("incremental scan failed");

    assert_eq!(summaries_inc[0].added_tracks, 0);
    assert_eq!(summaries_inc[0].unchanged_tracks, 2);

    // 3. FTS5 Search verification
    let search_res = library_service
        .search("Change", 10)
        .await
        .expect("search failed");
    assert_eq!(search_res.len(), 1);
    assert!(search_res[0].title.contains("Change"));

    let search_res2 = library_service
        .search("Digital", 10)
        .await
        .expect("search failed");
    assert_eq!(search_res2.len(), 1);
    assert!(search_res2[0].title.contains("Digital_Bath"));

    // 4. Pruning deleted file verification
    std::fs::remove_file(&track2_path).expect("delete track 2");

    let summaries_prune = library_service
        .scan_library(Some(folder_rec.id.clone()), false)
        .await
        .expect("prune scan failed");

    assert_eq!(summaries_prune[0].removed_tracks, 1);
    let count_after_prune = library_service.track_repo().get_track_count().await.expect("count");
    assert_eq!(count_after_prune, 1);
}
