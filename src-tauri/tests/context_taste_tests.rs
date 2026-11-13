use chrono::{Duration, Local, TimeZone};
use music_player_backend::config::AppConfig;
use music_player_backend::core::CoreProcessor;
use music_player_backend::database::create_in_memory_pool;
use music_player_backend::database::models::PlaybackHistoryRecord;
use music_player_backend::database::repositories::{
    HistoryRepository, ScannedMetadata, SqliteHistoryRepository,
};
use music_player_backend::playback::backend::MockAudioBackend;
use music_player_backend::recommendations::context::{
    self, bucket_keys, ContextAccumulator, DayPart, MIN_BUCKET_LISTENS,
};
use music_player_backend::recommendations::TasteProfileEngine;
use uuid::Uuid;


async fn setup_processor() -> CoreProcessor {
    let pool = create_in_memory_pool().await.expect("create in-memory pool");
    sqlx::query("INSERT OR REPLACE INTO application_settings(key,value,updated_at) VALUES('enable_jiosaavn','false',0)")
        .execute(&pool)
        .await
        .unwrap();
    CoreProcessor::new_with_backend(
        pool,
        AppConfig::default_with_dirs(),
        Box::new(MockAudioBackend::new()),
    )
}

async fn add_track(processor: &CoreProcessor, title: &str, artist: &str, genre: &str) -> String {
    processor
        .library_service()
        .track_repo()
        .save_scanned_track(ScannedMetadata {
            title_is_fallback: false,
            file_path: format!(
                "/music/{}_{}.flac",
                artist.to_lowercase().replace(' ', "_"),
                title.to_lowercase().replace(' ', "_")
            ),
            file_size: 2000,
            modified_timestamp: 100,
            file_hash: None,
            title: title.to_string(),
            artist: Some(artist.to_string()),
            album: Some(format!("{artist} album")),
            album_artist: None,
            genre: Some(genre.to_string()),
            track_number: Some(1),
            disc_number: Some(1),
            year: Some(2010),
            duration_secs: 240.0,
            bitrate: Some(1000),
            sample_rate: Some(44100),
            format: "flac".to_string(),
            has_cover_art: true,
            musicbrainz_track_id: None,
        })
        .await
        .expect("save track")
}

async fn play(processor: &CoreProcessor, track_id: &str, started_at: i64) {
    SqliteHistoryRepository::new(processor.db_pool().clone())
        .record_playback(&PlaybackHistoryRecord {
            id: Uuid::new_v4().to_string(),
            user_id: "default".into(),
            track_id: track_id.to_string(),
            started_at,
            ended_at: started_at + 240,
            seconds_listened: 240.0,
            percentage_listened: 1.0,
            completed: 1,
            skipped: 0,
            source: "library".into(),
            playlist_id: None,
            recommendation_session_id: None,
        })
        .await
        .unwrap();
}

/// Local timestamp `days_ago` days back at `hour`:00.
fn at(days_ago: i64, hour: u32) -> i64 {
    let date = Local::now().date_naive() - Duration::days(days_ago);
    Local
        .from_local_datetime(&date.and_hms_opt(hour, 0, 0).unwrap())
        .earliest()
        .unwrap()
        .timestamp()
}

#[test]
fn buckets_split_day_parts_and_keep_after_midnight_with_previous_day() {
    assert_eq!(DayPart::from_hour(7), DayPart::Morning);
    assert_eq!(DayPart::from_hour(13), DayPart::Afternoon);
    assert_eq!(DayPart::from_hour(19), DayPart::Evening);
    assert_eq!(DayPart::from_hour(23), DayPart::Night);
    assert_eq!(DayPart::from_hour(2), DayPart::Night);
    let (specific, part) = bucket_keys(at(3, 19)).unwrap();
    assert!(specific.starts_with("evening:"));
    assert_eq!(part, "evening");
    // 01:00 belongs to the previous day's night, so its day type matches 23:00 the night before.
    let late = bucket_keys(at(3, 23)).unwrap();
    let after_midnight = bucket_keys(at(2, 1)).unwrap();
    assert_eq!(late, after_midnight);
}

#[test]
fn context_lift_rewards_styles_concentrated_in_one_day_part() {
    let mut acc = ContextAccumulator::default();
    for day in 1..=25 {
        acc.add(at(day, 19), 1.0, Some("Jazz"), Some("english"));
        acc.add(at(day, 8), 1.0, Some("Rock"), Some("english"));
    }
    let map = acc.finish();
    let evening = &map["evening"];
    let morning = &map["morning"];
    assert!(evening.genres["jazz"] > 0.9, "{evening:?}");
    assert!(!evening.genres.contains_key("rock"));
    assert!(!morning.genres.contains_key("jazz"));
    // English is played at every hour, so it carries no context lift.
    assert!(!evening.languages.contains_key("english"));
    let (part, current) = context::current(&map, at(1, 20)).unwrap();
    assert_eq!(part, DayPart::Evening);
    assert!(context::lift(current, Some("jazz "), None) > context::lift(current, Some("Rock"), None));
}

#[test]
fn sparse_buckets_have_no_effect() {
    let mut acc = ContextAccumulator::default();
    for day in 1..MIN_BUCKET_LISTENS as i64 {
        acc.add(at(day, 19), 1.0, Some("Jazz"), None);
    }
    for day in 1..=30 {
        acc.add(at(day, 8), 1.0, Some("Rock"), None);
    }
    let map = acc.finish();
    assert!(!map.contains_key("evening"));
    assert!(context::current(&map, at(1, 20)).is_none());
    assert!(map.contains_key("morning"));
}

#[tokio::test]
async fn taste_profile_learns_evening_jazz_from_history() {
    let processor = setup_processor().await;
    let jazz = add_track(&processor, "Blue Hour", "Night Trio", "Jazz").await;
    let rock = add_track(&processor, "Sunrise Riff", "Morning Band", "Rock").await;
    for day in 1..=22 {
        play(&processor, &jazz, at(day, 20)).await;
        play(&processor, &rock, at(day, 9)).await;
    }
    let profile = TasteProfileEngine::new(processor.db_pool().clone())
        .compute_taste_profile("default")
        .await
        .unwrap();
    let (_, evening) = context::current(&profile.context_affinity, at(0, 20)).unwrap();
    let (_, morning) = context::current(&profile.context_affinity, at(0, 9)).unwrap();
    assert!(context::lift(evening, Some("Jazz"), None) > 0.5);
    assert_eq!(context::lift(morning, Some("Jazz"), None), 0.0);
    assert!(context::lift(morning, Some("Rock"), None) > 0.5);
}

#[tokio::test]
async fn too_little_history_leaves_context_empty() {
    let processor = setup_processor().await;
    let jazz = add_track(&processor, "Blue Hour", "Night Trio", "Jazz").await;
    for day in 1..=10 {
        play(&processor, &jazz, at(day, 20)).await;
    }
    let profile = TasteProfileEngine::new(processor.db_pool().clone())
        .compute_taste_profile("default")
        .await
        .unwrap();
    assert!(profile.context_affinity.is_empty());
}
