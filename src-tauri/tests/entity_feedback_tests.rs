use music_player_backend::config::AppConfig;
use music_player_backend::core::command::{CommandResponse, SmartMixType};
use music_player_backend::core::{Command, CoreProcessor, Query, QueryResponse};
use music_player_backend::database::create_in_memory_pool;
use music_player_backend::database::models::PlaybackHistoryRecord;
use music_player_backend::database::repositories::{
    HistoryRepository, ScannedMetadata, SqliteHistoryRepository,
};
use music_player_backend::playback::backend::MockAudioBackend;
use music_player_backend::recommendations::entity_feedback::{
    self, EntityFeedbackRow, EntityFeedbackSet, LESS_ARTIST_PENALTY, LESS_GENRE_PENALTY,
};
use music_player_backend::recommendations::signals::{artist_credits, SongSignals};
use music_player_backend::recommendations::{SmartMixGenerator, TasteProfileEngine};
use uuid::Uuid;

const DAY: i64 = 86_400;

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

/// Two artists the user loves, plus a few unknowns, so every recommender has material.
async fn library(processor: &CoreProcessor) -> (Vec<String>, Vec<String>) {
    let now = chrono::Utc::now().timestamp();
    let mut loud = Vec::new();
    let mut calm = Vec::new();
    for i in 0..5 {
        let a = add_track(processor, &format!("Loud {i}"), "Loud Band", "Metal").await;
        let b = add_track(processor, &format!("Calm {i}"), "Calm Singer", "Folk").await;
        for n in 0..4 {
            play(processor, &a, now - 2 * DAY - (i * 10 + n) * 3_600).await;
            play(processor, &b, now - 2 * DAY - (i * 10 + n) * 3_600 - 600).await;
        }
        loud.push(a);
        calm.push(b);
        add_track(processor, &format!("Fresh {i}"), "Fresh Face", "Folk").await;
    }
    (loud, calm)
}

fn row(kind: &str, key: &str, level: &str, created_at: i64) -> EntityFeedbackRow {
    EntityFeedbackRow {
        kind: kind.into(),
        key: key.into(),
        display: key.into(),
        level: level.into(),
        created_at,
    }
}

#[test]
fn less_penalty_applies_to_credits_and_fades_over_ninety_days() {
    let now = 1_000 * DAY;
    let artist = artist_credits("Loud Band")[0].clone();
    let set = EntityFeedbackSet::from_rows(&[
        row("artist", &artist, "less", now),
        row("genre", "metal", "less", now),
    ]);
    let fresh = set.less_penalty("Loud Band", Some("Metal"), now);
    assert!((fresh - (LESS_ARTIST_PENALTY + LESS_GENRE_PENALTY)).abs() < 1e-9);
    // Feature credits are matched too.
    assert!(set.less_penalty("Someone feat. Loud Band", None, now) > 0.0);
    let half = set.less_penalty("Loud Band", Some("Metal"), now + 45 * DAY);
    assert!((half - fresh / 2.0).abs() < 1e-9, "linear decay: {half}");
    assert_eq!(set.less_penalty("Loud Band", Some("Metal"), now + 90 * DAY), 0.0);
    assert_eq!(set.less_penalty("Loud Band", Some("Metal"), now + 400 * DAY), 0.0);
    assert_eq!(set.less_penalty("Other", Some("Folk"), now), 0.0);
    assert!(!set.is_blocked_artist("Loud Band"), "less never blocks");
}

#[tokio::test]
async fn upsert_never_downgrades_a_block() {
    let pool = create_in_memory_pool().await.unwrap();
    entity_feedback::upsert(&pool, "default", "artist", "x", "X", "block", 1).await.unwrap();
    entity_feedback::upsert(&pool, "default", "artist", "x", "X", "less", 2).await.unwrap();
    let rows = entity_feedback::load_rows(&pool, "default").await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].level, "block");
    // Another user is untouched.
    assert!(entity_feedback::load_rows(&pool, "other").await.unwrap().is_empty());
    assert!(entity_feedback::clear(&pool, "default", "artist", "x").await.unwrap());
    assert!(!entity_feedback::clear(&pool, "default", "artist", "x").await.unwrap());
}

#[tokio::test]
async fn blocked_artist_disappears_everywhere_and_clear_restores() {
    let processor = setup_processor().await;
    let pool = processor.db_pool().clone();
    let (loud, _) = library(&processor).await;

    let before = TasteProfileEngine::new(pool.clone())
        .compute_taste_profile("default")
        .await
        .unwrap();
    assert!(before.top_artists.iter().any(|a| a.display_name == "Loud Band"));

    let CommandResponse::EntityFeedback(rows) = processor
        .dispatch_command(Command::BlockArtist { track_id: loud[0].clone() })
        .await
        .unwrap()
    else {
        panic!("expected entity feedback rows")
    };
    assert_eq!(rows.len(), 1, "block only touches the artist: {rows:?}");
    assert_eq!((rows[0].kind.as_str(), rows[0].level.as_str()), ("artist", "block"));

    let signals = SongSignals::load(&pool, "default").await.unwrap();
    assert!(loud.iter().all(|id| signals.blocked_id(id)));
    let filtered = signals
        .filter(serde_json::json!([{"track_id": loud[1], "title": "Loud 1", "artist": "Loud Band"}]))
        .unwrap();
    assert_eq!(filtered, serde_json::json!([]));

    let flow = processor
        .flow_engine()
        .generate_flow_tracks("default", 0.5, 20, &[])
        .await
        .unwrap();
    assert!(!flow.is_empty());
    assert!(flow.iter().all(|t| t.artist != "Loud Band"), "{flow:?}");

    let mix = SmartMixGenerator::new(pool.clone())
        .generate_mix("default", &SmartMixType::Daily)
        .await
        .unwrap();
    let mix_ids: Vec<String> =
        sqlx::query_scalar("SELECT track_id FROM playlist_tracks WHERE playlist_id=?")
            .bind(&mix.id)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(mix_ids.iter().all(|id| !loud.contains(id)));

    let profile = TasteProfileEngine::new(pool.clone())
        .compute_taste_profile("default")
        .await
        .unwrap();
    assert!(profile.top_artists.iter().all(|a| a.display_name != "Loud Band"));

    let QueryResponse::EntityFeedback(listed) =
        processor.execute_query(Query::GetEntityFeedback).await.unwrap()
    else {
        panic!("expected entity feedback query")
    };
    assert_eq!(listed, rows);

    processor
        .dispatch_command(Command::ClearEntityFeedback {
            kind: "artist".into(),
            key: rows[0].key.clone(),
        })
        .await
        .unwrap();
    let signals = SongSignals::load(&pool, "default").await.unwrap();
    assert!(loud.iter().all(|id| !signals.blocked_id(id)));
    let restored = TasteProfileEngine::new(pool.clone())
        .compute_taste_profile("default")
        .await
        .unwrap();
    assert!(restored.top_artists.iter().any(|a| a.display_name == "Loud Band"));
}

#[tokio::test]
async fn less_like_this_halves_affinity_and_lowers_flow_score() {
    let processor = setup_processor().await;
    let pool = processor.db_pool().clone();
    let (loud, _) = library(&processor).await;
    let engine = TasteProfileEngine::new(pool.clone());
    let affinity = |p: &music_player_backend::recommendations::taste::TasteProfile| {
        p.top_artists
            .iter()
            .find(|a| a.display_name == "Loud Band")
            .map(|a| a.affinity)
            .unwrap_or(0.0)
    };
    let before = affinity(&engine.compute_taste_profile("default").await.unwrap());
    let best = |tracks: &[music_player_backend::recommendations::flow::FlowTrackItem]| {
        tracks
            .iter()
            .filter(|t| t.artist == "Loud Band")
            .map(|t| t.score)
            .fold(f64::NEG_INFINITY, f64::max)
    };
    let flow_before = best(
        &processor
            .flow_engine()
            .generate_flow_tracks("default", 0.8, 30, &[])
            .await
            .unwrap(),
    );

    let CommandResponse::EntityFeedback(rows) = processor
        .dispatch_command(Command::LessLikeThis { track_id: loud[0].clone() })
        .await
        .unwrap()
    else {
        panic!("expected entity feedback rows")
    };
    let kinds: Vec<_> = rows.iter().map(|r| (r.kind.as_str(), r.level.as_str())).collect();
    assert!(kinds.contains(&("artist", "less")) && kinds.contains(&("genre", "less")), "{rows:?}");

    let after = affinity(&engine.compute_taste_profile("default").await.unwrap());
    assert!(before > 0.0 && (after - before * 0.5).abs() < 1e-6, "{before} -> {after}");
    // Less never blocks.
    let signals = SongSignals::load(&pool, "default").await.unwrap();
    assert!(loud.iter().all(|id| !signals.blocked_id(id)));

    let flow_after = best(
        &processor
            .flow_engine()
            .generate_flow_tracks("default", 0.8, 30, &[])
            .await
            .unwrap(),
    );
    assert!(
        flow_before.is_finite() && (!flow_after.is_finite() || flow_after < flow_before),
        "flow score {flow_before} -> {flow_after}"
    );
}
