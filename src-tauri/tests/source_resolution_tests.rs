use music_player_backend::core::command::RepeatMode;
use music_player_backend::core::event::QueueItem;
use music_player_backend::playback::queue::PlaybackQueue;
use music_player_backend::playback::resolution_cache::{
    parse_url_expiry, ResolutionCache, ResolvedEntry, RESOLUTION_CACHE_MAX,
};
use music_player_backend::playback::stream::{
    PreparedPlayback, ResolvedAudioSource, StreamPlaybackManager,
};
use tempfile::tempdir;

fn sample_item(id: &str, title: &str, artist: &str) -> QueueItem {
    QueueItem {
        queue_id: format!("queue-{}", id),
        track_id: id.to_string(),
        title: title.to_string(),
        artist: artist.to_string(),
        duration_secs: 200.0,
    }
}

// ---------------------------------------------------------------------------
// 1. Resolution Cache Tests
// ---------------------------------------------------------------------------

#[test]
fn test_resolution_cache_hit_miss_and_invalidation() {
    let cache = ResolutionCache::new();
    let key = "full:v2:track1:beatles:let-it-be".to_string();

    // Miss before insert
    assert!(cache.get(&key).is_none());

    // Insert
    let entry = ResolvedEntry::new(
        "https://rr1.googlevideo.com/videoplayback?id=123".to_string(),
        243.0,
        None,
    );
    cache.insert(key.clone(), entry);

    // Hit
    let cached = cache.get(&key).expect("should hit resolution cache");
    assert_eq!(cached.url, "https://rr1.googlevideo.com/videoplayback?id=123");
    assert_eq!(cached.duration, 243.0);
    assert!(cached.is_valid());

    // Invalidation
    cache.invalidate(&key);
    assert!(cache.get(&key).is_none());
}

#[test]
fn test_resolution_cache_expiry_and_pruning() {
    let cache = ResolutionCache::new();

    // Insert an already expired entry
    let past_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .saturating_sub(100);

    let expired_entry = ResolvedEntry::new(
        "https://expired.example.com/audio.m4a".to_string(),
        180.0,
        Some(past_unix),
    );
    cache.insert("expired_key".to_string(), expired_entry);

    // .get() should evict expired entry immediately
    assert!(cache.get("expired_key").is_none());
    assert_eq!(cache.valid_len(), 0);
}

#[test]
fn test_resolution_cache_max_capacity_eviction() {
    let cache = ResolutionCache::new();

    // Fill to RESOLUTION_CACHE_MAX
    for i in 0..RESOLUTION_CACHE_MAX {
        let entry = ResolvedEntry::new(
            format!("https://example.com/song_{}.m4a", i),
            180.0,
            None,
        );
        cache.insert(format!("key_{}", i), entry);
    }

    assert_eq!(cache.valid_len(), RESOLUTION_CACHE_MAX);

    // Inserting one more must trigger eviction
    let extra = ResolvedEntry::new(
        "https://example.com/extra.m4a".to_string(),
        210.0,
        None,
    );
    cache.insert("extra_key".to_string(), extra);

    assert_eq!(cache.valid_len(), RESOLUTION_CACHE_MAX);
    assert!(cache.get("extra_key").is_some());
}

#[test]
fn test_parse_url_expiry_from_googlevideo() {
    let valid_yt = "https://rr3.googlevideo.com/videoplayback?expire=1742233445&itag=140&ratebypass=yes";
    assert_eq!(parse_url_expiry(valid_yt), Some(1742233445));

    let direct_audius = "https://api.audius.co/v1/tracks/abc123/stream";
    assert_eq!(parse_url_expiry(direct_audius), None);

    let direct_archive = "https://archive.org/download/test_item/song.mp3";
    assert_eq!(parse_url_expiry(direct_archive), None);
}

// ---------------------------------------------------------------------------
// 2. Queue peek_next Lookahead Tests
// ---------------------------------------------------------------------------

#[test]
fn test_peek_next_sequential_and_bounds() {
    let mut queue = PlaybackQueue::new();
    let t1 = sample_item("t1", "Track 1", "Artist");
    let t2 = sample_item("t2", "Track 2", "Artist");
    let t3 = sample_item("t3", "Track 3", "Artist");

    queue.set_queue(vec![t1.clone(), t2.clone(), t3.clone()], Some(0));

    // When at 0, peek_next() sees t2 without mutating current_index
    assert_eq!(queue.peek_next().map(|i| i.track_id.as_str()), Some("t2"));
    assert_eq!(queue.current_index(), Some(0));

    // Advance to t2
    assert_eq!(queue.next().map(|i| i.track_id.as_str()), Some("t2"));
    assert_eq!(queue.current_index(), Some(1));

    // When at 1, peek_next() sees t3
    assert_eq!(queue.peek_next().map(|i| i.track_id.as_str()), Some("t3"));

    // Advance to t3
    assert_eq!(queue.next().map(|i| i.track_id.as_str()), Some("t3"));
    assert_eq!(queue.current_index(), Some(2));

    // When at 2 (last), peek_next() with RepeatMode::None returns None
    assert_eq!(queue.peek_next(), None);
}

#[test]
fn test_peek_next_repeat_all_and_repeat_one() {
    let mut queue = PlaybackQueue::new();
    let t1 = sample_item("t1", "Track 1", "Artist");
    let t2 = sample_item("t2", "Track 2", "Artist");
    queue.set_queue(vec![t1.clone(), t2.clone()], Some(1));

    // Repeat Off at last item -> None
    queue.set_repeat_mode(RepeatMode::Off);
    assert_eq!(queue.peek_next(), None);

    // Repeat All at last item -> wraps to t1
    queue.set_repeat_mode(RepeatMode::All);
    assert_eq!(queue.peek_next().map(|i| i.track_id.as_str()), Some("t1"));

    // Repeat One -> always returns current item (t2)
    queue.set_repeat_mode(RepeatMode::One);
    assert_eq!(queue.peek_next().map(|i| i.track_id.as_str()), Some("t2"));
    assert_eq!(queue.current_index(), Some(1));
}


#[test]
fn test_peek_next_with_shuffle() {
    let mut queue = PlaybackQueue::new();
    let items = vec![
        sample_item("t1", "Track 1", "Artist"),
        sample_item("t2", "Track 2", "Artist"),
        sample_item("t3", "Track 3", "Artist"),
        sample_item("t4", "Track 4", "Artist"),
    ];
    queue.set_queue(items, Some(0));
    queue.set_shuffle(true);

    // Current item is still the first item in the shuffle
    let current = queue.current().unwrap().track_id.clone();
    let peeked = queue.peek_next().unwrap().track_id.clone();
    assert_ne!(current, peeked);

    // Peek multiple times must return the same item
    assert_eq!(queue.peek_next().unwrap().track_id, peeked);
    assert_eq!(queue.current().unwrap().track_id, current);

    // Calling next() advances to that exact peeked item
    let advanced = queue.next().unwrap().track_id.clone();
    assert_eq!(advanced, peeked);
}

// ---------------------------------------------------------------------------
// 3. StreamPlaybackManager Source Resolution Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_resolve_source_only_disk_cache_hit_bypasses_network() {
    let dir = tempdir().unwrap();
    let manager = StreamPlaybackManager::new(dir.path().to_path_buf(), None, None);

    let track_id = "online:test-track-123";
    let title = "Hit Song";
    let artist = "Famous Artist";

    // Write a valid decodable audio file into the disk cache
    let cache_key = format!("full:v2:{}", StreamPlaybackManager::compute_cache_key(track_id, artist, title));
    let cache_path = manager.get_cache_path(&cache_key);

    std::fs::create_dir_all(manager.cache_dir()).unwrap();
    // Generate valid 44-byte WAV header
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36u32 + 8192u32).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&2u16.to_le_bytes()); // Stereo
    wav.extend_from_slice(&44100u32.to_le_bytes()); // Sample rate
    wav.extend_from_slice(&(44100u32 * 4).to_le_bytes()); // Byte rate
    wav.extend_from_slice(&4u16.to_le_bytes()); // Block align
    wav.extend_from_slice(&16u16.to_le_bytes()); // Bits per sample
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&8192u32.to_le_bytes());
    wav.extend(vec![0u8; 8192]);
    std::fs::write(&cache_path, wav).unwrap();

    // Call resolve_source_only
    let resolved = manager
        .resolve_source_only(track_id, title, artist)
        .await
        .expect("Disk cache hit must succeed");

    match resolved {
        ResolvedAudioSource::LocalFile { path, duration } => {
            assert_eq!(path, cache_path);
            assert!(duration > 0.0);
        }
        _ => panic!("Expected LocalFile from disk cache hit"),
    }

    // Verify resolve_and_prepare_playback also takes the fast local path
    let prepared = manager
        .resolve_and_prepare_playback(track_id, title, artist)
        .await
        .expect("Must prepare playback from disk cache");

    match prepared {
        PreparedPlayback::LocalFile { path, .. } => {
            assert_eq!(path, cache_path);
        }
        _ => panic!("Expected PreparedPlayback::LocalFile from disk cache hit"),
    }
}

#[tokio::test]
async fn test_resolve_source_only_memory_cache_hit_bypasses_provider() {
    let dir = tempdir().unwrap();
    // No download service attached — a provider call would fail immediately
    let manager = StreamPlaybackManager::new(dir.path().to_path_buf(), None, None);

    let track_id = "online:memory-cached-1";
    let title = "Cached Song";
    let artist = "Cached Artist";

    let cache_key = format!("full:v2:{}", StreamPlaybackManager::compute_cache_key(track_id, artist, title));

    // Pre-populate resolution cache directly via resolve_source_only's cache key
    let test_url = "https://rr2.googlevideo.com/videoplayback?id=cached123&expire=9999999999".to_string();
    let entry = ResolvedEntry::new(test_url.clone(), 215.0, Some(9999999999));
    manager.resolution_cache().insert(cache_key.clone(), entry);

    // Call resolve_source_only — must hit in-memory resolution cache without provider
    let resolved = manager
        .resolve_source_only(track_id, title, artist)
        .await
        .expect("Memory cache hit must succeed without network/provider");

    match resolved {
        ResolvedAudioSource::RemoteStream { url, duration, from_cache } => {
            assert_eq!(url, test_url);
            assert_eq!(duration, 215.0);
            assert!(from_cache);
        }
        _ => panic!("Expected RemoteStream from memory resolution cache hit"),
    }

    // Verify resolve_source_only did not create any .part files
    let part_path = StreamPlaybackManager::get_part_path(&manager.get_cache_path(&cache_key));
    assert!(!part_path.exists());
}

#[tokio::test]
async fn test_resolve_source_only_never_creates_part_file() {
    let dir = tempdir().unwrap();
    let manager = StreamPlaybackManager::new(dir.path().to_path_buf(), None, None);

    let track_id = "online:no-part-file";
    let title = "Virtual Track";
    let artist = "Ghost Artist";

    let cache_key = format!("full:v2:{}", StreamPlaybackManager::compute_cache_key(track_id, artist, title));
    let target_path = manager.get_cache_path(&cache_key);
    let part_path = StreamPlaybackManager::get_part_path(&target_path);

    // Call resolve_source_only (without provider it will return error or local miss)
    let _ = manager.resolve_source_only(track_id, title, artist).await;

    // Must NOT create part file or target file
    assert!(!part_path.exists());
    assert!(!target_path.exists());
}
