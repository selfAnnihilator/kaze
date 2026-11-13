use music_player_backend::config::AppConfig;
use music_player_backend::core::processor::CoreProcessor;
use music_player_backend::database::create_in_memory_pool;
use music_player_backend::database::models::PlaybackHistoryRecord;
use music_player_backend::database::repositories::{
    HistoryRepository, ScannedMetadata, SqliteHistoryRepository,
};
use music_player_backend::playback::backend::MockAudioBackend;
use music_player_backend::providers::lastfm::{parse_similar, LastFmError};
use music_player_backend::recommendations::cooccurrence::{self, display_credits, score_sequences};
use music_player_backend::recommendations::signals::artist_credits;
use music_player_backend::recommendations::similarity::SimilarityIndex;
use std::collections::HashSet;
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

fn seq(names: &[&str]) -> Vec<Vec<(String, String)>> {
    names.iter().map(|n| display_credits(n)).collect()
}

#[test]
fn cooccurrence_scores_repeated_neighbours_and_drops_one_offs() {
    let sequences = vec![
        seq(&["Alpha", "Beta", "Gamma"]),
        seq(&["Alpha", "Beta"]),
        seq(&["Delta", "Alpha"]),
    ];
    let edges = score_sequences(&sequences);
    let alpha = artist_credits("Alpha")[0].clone();
    let beta = artist_credits("Beta")[0].clone();
    let ab = edges
        .iter()
        .find(|e| e.source == alpha && e.target == beta)
        .expect("alpha-beta edge");
    // count 2, alpha appears in 3 sequences, beta in 2 -> 2 / sqrt(6)
    assert!((ab.score - 2.0 / 6f64.sqrt()).abs() < 1e-9);
    assert!(edges.iter().any(|e| e.source == beta && e.target == alpha));
    // Pairs seen only once are dropped.
    let delta = artist_credits("Delta")[0].clone();
    assert!(!edges.iter().any(|e| e.source == delta || e.target == delta));
    // Deterministic.
    assert_eq!(edges, score_sequences(&sequences));
}

#[test]
fn display_credits_split_features() {
    let credits = display_credits("Pritam feat. Arijit Singh & Shreya Ghoshal");
    let shown: Vec<_> = credits.iter().map(|(_, d)| d.as_str()).collect();
    assert_eq!(shown, vec!["Pritam", "Arijit Singh", "Shreya Ghoshal"]);
}

#[test]
fn lastfm_parser_handles_string_match_errors_and_single_object() {
    let body = serde_json::json!({
        "similarartists": {"artist": [
            {"name": "Beta", "match": "0.91"},
            {"name": "Gamma", "match": 0.4},
            {"name": "", "match": "1"},
            {"name": "Zero", "match": "0"}
        ], "@attr": {"artist": "Alpha"}}
    });
    let list = parse_similar(&body).unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].name, "Beta");
    assert!((list[0].score - 0.91).abs() < 1e-9);

    let single = serde_json::json!({"similarartists": {"artist": {"name": "Solo", "match": "0.5"}}});
    assert_eq!(parse_similar(&single).unwrap().len(), 1);

    let missing = serde_json::json!({"error": 6, "message": "The artist you supplied could not be found"});
    assert!(matches!(parse_similar(&missing), Err(LastFmError::NotFound)));
    let rate = serde_json::json!({"error": 29, "message": "Rate limit exceeded"});
    assert!(matches!(parse_similar(&rate), Err(LastFmError::Transient(_))));
    assert!(parse_similar(&serde_json::json!({})).unwrap().is_empty());
}

#[test]
fn similarity_index_orders_by_affinity_and_excludes_seeds() {
    let seeds = vec![("Alpha".to_string(), 1.0), ("Beta".to_string(), 0.4)];
    let index = SimilarityIndex::from_pairs(
        &seeds,
        &[
            ("Alpha", "Close", 0.9, "lastfm"),
            ("Beta", "Far", 0.9, "lastfm"),
            ("Alpha", "Beta", 1.0, "cooc"),
            ("Beta", "Close", 0.2, "cooc"),
        ],
    );
    assert!(index.boost("Close") > index.boost("Far"));
    assert_eq!(index.hit("Close").unwrap().because_of, "Alpha");
    // Seeds are never "similar" to themselves.
    assert_eq!(index.boost("Beta"), 0.0);
    // Co-occurrence is weighted above Last.fm.
    let cooc = SimilarityIndex::from_pairs(&seeds[..1], &[("Alpha", "X", 0.5, "cooc")]);
    let lastfm = SimilarityIndex::from_pairs(&seeds[..1], &[("Alpha", "X", 0.5, "lastfm")]);
    assert!(cooc.boost("X") > lastfm.boost("X"));
    // Feature credits match too.
    assert!(index.boost("Someone feat. Close") > 0.0);

    let top = index.top_similar(5, &HashSet::new());
    assert_eq!(top.first().map(|h| h.display.as_str()), Some("Close"));
    let excluded: HashSet<String> = artist_credits("Close").into_iter().collect();
    assert!(index.top_similar(5, &excluded).iter().all(|h| h.display != "Close"));
}

#[tokio::test]
async fn cooccurrence_rebuild_stores_edges_from_sessions_and_playlists() {
    let processor = setup_processor().await;
    let pool = processor.db_pool().clone();
    let a = add_track(&processor, "A1", "Alpha", "Rock").await;
    let b = add_track(&processor, "B1", "Beta", "Rock").await;
    let c = add_track(&processor, "C1", "Gamma", "Rock").await;
    let base = chrono::Utc::now().timestamp() - 10 * 86_400;
    // Two sessions where Alpha and Beta are played back to back, far apart in time.
    for day in 0..2 {
        let t = base + day * 86_400;
        play(&processor, &a, t).await;
        play(&processor, &b, t + 300).await;
    }
    // One playlist pairing Beta with Gamma; with one session it is still one co-occurrence.
    let now = chrono::Utc::now().timestamp();
    sqlx::query("INSERT INTO playlists(id,name,is_smart_mix,created_at,updated_at,user_id) VALUES('p1','Mine',0,?,?,'default')")
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .unwrap();
    for (pos, id) in [&b, &c].iter().enumerate() {
        sqlx::query("INSERT INTO playlist_tracks(playlist_id,track_id,position,added_at) VALUES('p1',?,?,?)")
            .bind(id)
            .bind(pos as i64)
            .bind(now)
            .execute(&pool)
            .await
            .unwrap();
    }

    let stored = cooccurrence::rebuild(&pool, "default").await.unwrap();
    assert_eq!(stored, 2, "alpha<->beta both directions; beta-gamma seen once");
    let rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM artist_similarity WHERE user_id='default' AND source='cooc'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(rows, 2);
    // Rebuild replaces, not appends.
    cooccurrence::rebuild(&pool, "default").await.unwrap();
    let rows_again: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM artist_similarity WHERE user_id='default' AND source='cooc'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(rows_again, 2);
}

#[tokio::test]
async fn flow_ranks_similar_unknown_artist_above_unrelated_one() {
    let processor = setup_processor().await;
    let pool = processor.db_pool().clone();
    let now = chrono::Utc::now().timestamp();

    let loved = add_track(&processor, "Loved Song", "Seed Artist", "Indie").await;
    for i in 0..8 {
        play(&processor, &loved, now - 3 * 86_400 - i * 3_600).await;
    }
    for i in 0..3 {
        add_track(&processor, &format!("Similar {i}"), "Kindred Band", "Indie").await;
        add_track(&processor, &format!("Other {i}"), "Stranger Band", "Indie").await;
    }
    let source = artist_credits("Seed Artist")[0].clone();
    let target = artist_credits("Kindred Band")[0].clone();
    sqlx::query(
        "INSERT INTO artist_similarity(user_id,source_norm,target_norm,target_display,score,source,updated_at)
         VALUES('',?,?,'Kindred Band',0.95,'lastfm',?)",
    )
    .bind(&source)
    .bind(&target)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let tracks = processor
        .flow_engine()
        .generate_flow_tracks("default", 0.3, 20, &[])
        .await
        .unwrap();
    let best_score = |artist: &str| {
        tracks
            .iter()
            .filter(|t| t.artist == artist)
            .map(|t| t.score)
            .fold(f64::NEG_INFINITY, f64::max)
    };
    let kindred = best_score("Kindred Band");
    let stranger = best_score("Stranger Band");
    assert!(kindred.is_finite(), "similar artist should appear: {tracks:?}");
    assert!(kindred > stranger, "kindred {kindred} vs stranger {stranger}");
    let explained = tracks
        .iter()
        .find(|t| t.artist == "Kindred Band")
        .and_then(|t| t.explanation.similar_artist.clone());
    assert_eq!(explained.as_deref(), Some("Seed Artist"));
}
