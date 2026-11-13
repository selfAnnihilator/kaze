//! Phase D: taste clusters -> Daily Mix 1..N.
use music_player_backend::config::AppConfig;
use music_player_backend::core::CoreProcessor;
use music_player_backend::database::create_in_memory_pool;
use music_player_backend::database::models::{PlaybackHistoryRecord, PlaylistRecord};
use music_player_backend::database::repositories::{
    HistoryRepository, PlaylistRepository, ScannedMetadata, SqliteHistoryRepository, SqlitePlaylistRepository,
};
use music_player_backend::playback::backend::MockAudioBackend;
use music_player_backend::recommendations::clusters::{cluster_nodes, ClusterNode};
use music_player_backend::recommendations::SmartMixGenerator;
use std::collections::HashMap;
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

fn node(name: &str, affinity: f64, genre: &str) -> ClusterNode {
    ClusterNode {
        display: name.to_string(),
        norm: name.to_lowercase(),
        affinity,
        genre: Some(genre.to_string()),
    }
}

#[test]
fn disjoint_genre_groups_form_two_clusters() {
    let nodes = vec![
        node("R1", 0.9, "rock"),
        node("R2", 0.8, "rock"),
        node("R3", 0.7, "rock"),
        node("J1", 0.6, "jazz"),
        node("J2", 0.5, "jazz"),
        node("J3", 0.4, "jazz"),
        node("Solo", 0.3, "polka"),
    ];
    let clusters = cluster_nodes(&nodes, &HashMap::new());
    assert_eq!(clusters.len(), 2, "{clusters:?}");
    assert_eq!(clusters[0].artists, vec!["R1", "R2", "R3"]);
    assert_eq!(clusters[0].genres, vec!["rock"]);
    assert_eq!(clusters[0].label, "R1, R2, R3 and more");
    assert_eq!(clusters[1].artists, vec!["J1", "J2", "J3"]);
    // Deterministic regardless of input order.
    let mut reversed = nodes.clone();
    reversed.reverse();
    assert_eq!(cluster_nodes(&reversed, &HashMap::new()), clusters);
}

#[test]
fn similarity_links_artists_across_genres() {
    let nodes = vec![
        node("A", 0.9, "x"),
        node("B", 0.8, "y"),
        node("C", 0.7, "z"),
    ];
    let mut sim = HashMap::new();
    sim.insert(("a".to_string(), "b".to_string()), 0.9);
    sim.insert(("a".to_string(), "c".to_string()), 0.8);
    sim.insert(("b".to_string(), "c".to_string()), 0.7);
    let clusters = cluster_nodes(&nodes, &sim);
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].artists.len(), 3);
}

async fn seed_two_tastes(processor: &CoreProcessor) {
    let now = chrono::Utc::now().timestamp();
    let groups = [
        ("Rock", ["Stone Riders", "Iron Valley", "Red Amps", "Loud Hills"]),
        ("Jazz", ["Blue Quartet", "Smoky Keys", "Late Horns", "Soft Brush"]),
    ];
    let mut offset = 0;
    for (genre, artists) in groups {
        for artist in artists {
            for n in 0..4 {
                let id = add_track(processor, &format!("{artist} song {n}"), artist, genre).await;
                for k in 0..3 {
                    offset += 1;
                    play(processor, &id, now - 86400 * (k + 1) - offset * 300).await;
                }
            }
        }
    }
}

async fn daily_family(processor: &CoreProcessor) -> Vec<PlaylistRecord> {
    let mut mixes: Vec<PlaylistRecord> = SqlitePlaylistRepository::new(processor.db_pool().clone())
        .get_smart_mixes_for_user("default")
        .await
        .unwrap()
        .into_iter()
        .filter(|m| {
            m.mix_type
                .as_deref()
                .is_some_and(|t| t == "daily" || t.starts_with("daily:"))
        })
        .collect();
    mixes.sort_by(|a, b| a.mix_type.cmp(&b.mix_type));
    mixes
}

#[tokio::test]
async fn two_taste_clusters_produce_daily_mix_1_and_2_and_drop_stale() {
    let processor = setup_processor().await;
    seed_two_tastes(&processor).await;
    let pool = processor.db_pool().clone();
    // A stale, expired third mix from an earlier clustering.
    SqlitePlaylistRepository::new(pool.clone())
        .create_playlist_with_user(
            &PlaylistRecord {
                id: Uuid::new_v4().to_string(),
                name: "Daily Mix 3".into(),
                description: None,
                is_smart_mix: 1,
                mix_type: Some("daily:3".into()),
                generation_reason: None,
                expires_at: Some(1),
                created_at: 1,
                updated_at: 1,
            },
            "default",
        )
        .await
        .unwrap();

    SmartMixGenerator::new(pool.clone())
        .ensure_default_mixes("default")
        .await
        .unwrap();

    let family = daily_family(&processor).await;
    let kinds: Vec<_> = family.iter().filter_map(|m| m.mix_type.clone()).collect();
    assert_eq!(kinds, vec!["daily:1", "daily:2"], "{family:?}");
    assert_eq!(family[0].name, "Daily Mix 1");
    assert!(family[0].generation_reason.as_deref().unwrap().ends_with("and more"));
    assert!(family.iter().all(|m| m.expires_at.is_some_and(|e| e > chrono::Utc::now().timestamp())));

    // Each mix stays within its own taste: rock mix has no jazz and vice versa.
    for mix in &family {
        let genres: Vec<String> = sqlx::query_scalar(
            "SELECT DISTINCT g.name FROM playlist_tracks pt
             JOIN tracks t ON t.id = pt.track_id
             JOIN genres g ON g.id = t.genre_id
             WHERE pt.playlist_id = ?",
        )
        .bind(&mix.id)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(genres.len(), 1, "{} mixed genres: {genres:?}", mix.name);
    }
}

#[tokio::test]
async fn cold_start_keeps_single_daily_mix() {
    let processor = setup_processor().await;
    add_track(&processor, "Only song", "Lonely Artist", "Rock").await;
    SmartMixGenerator::new(processor.db_pool().clone())
        .ensure_default_mixes("default")
        .await
        .unwrap();
    let kinds: Vec<_> = daily_family(&processor)
        .await
        .into_iter()
        .filter_map(|m| m.mix_type)
        .collect();
    assert_eq!(kinds, vec!["daily"]);
}
