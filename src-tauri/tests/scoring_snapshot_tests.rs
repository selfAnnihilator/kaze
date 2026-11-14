//! Frozen outputs of the two largest scorers: Flow candidates and the For You feed.
//! They guard refactors: any change to scores, order, reasons or slots shows up here.
//! If a change is intended, update the snapshot in the same commit and say why.
use music_player_backend::{
    config::AppConfig,
    core::processor::CoreProcessor,
    database::{
        create_in_memory_pool,
        models::{ExternalTrackRecord, PlaybackHistoryRecord},
        repositories::{
            HistoryRepository, ScannedMetadata, SqliteHistoryRepository, SqliteTrackRepository,
            SqliteWishlistRepository, WishlistRepository,
        },
    },
    discovery::DiscoveryCoordinator,
    playback::backend::MockAudioBackend,
    recommendations::{FlowTrackItem, TasteProfileEngine},
};
use sqlx::SqlitePool;
use std::sync::Arc;

const DAY: i64 = 86400;

async fn local(pool: &CoreProcessor, title: &str, artist: &str, genre: &str, year: i64) -> String {
    pool.library_service()
        .track_repo()
        .save_scanned_track(ScannedMetadata {
            title_is_fallback: false,
            file_path: format!("/fixture/{artist}-{title}.flac"),
            file_size: 1,
            modified_timestamp: 0,
            file_hash: None,
            title: title.into(),
            artist: Some(artist.into()),
            album: Some(format!("{artist} album")),
            album_artist: None,
            genre: Some(genre.into()),
            track_number: None,
            disc_number: None,
            year: Some(year as u32),
            duration_secs: 240.0,
            bitrate: None,
            sample_rate: None,
            format: "flac".into(),
            has_cover_art: false,
            musicbrainz_track_id: None,
        })
        .await
        .unwrap()
}

async fn external(pool: &SqlitePool, id: &str, artist: &str, genre: Option<&str>) {
    SqliteWishlistRepository::new(pool.clone())
        .upsert_external_track(&ExternalTrackRecord {
            id: id.into(),
            provider: "jiosaavn".into(),
            provider_id: id.into(),
            title: format!("Song {id}"),
            artist: artist.into(),
            album: None,
            genre: genre.map(Into::into),
            duration_secs: Some(240.0),
            cover_art_url: None,
            preview_url: None,
            match_status: "NOT_FOUND".into(),
            matched_local_track_id: None,
            created_at: 1_700_000_000,
        })
        .await
        .unwrap();
}

async fn listen(pool: &SqlitePool, id: &str, fraction: f64, age_days: i64, skipped: bool) {
    let at = chrono::Utc::now().timestamp() - age_days * DAY;
    SqliteHistoryRepository::new(pool.clone())
        .record_playback(&PlaybackHistoryRecord {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: "default".into(),
            track_id: id.into(),
            started_at: at,
            ended_at: at + 240,
            seconds_listened: fraction.min(1.0) * 240.0,
            percentage_listened: fraction,
            completed: i64::from(fraction >= 1.0 && !skipped),
            skipped: i64::from(skipped),
            source: "library".into(),
            playlist_id: None,
            recommendation_session_id: None,
        })
        .await
        .unwrap();
}

/// A library with clear favourites, a forgotten favourite, a skipped song, a disliked one
/// and online songs that match (or don't match) the taste.
async fn seeded_processor() -> CoreProcessor {
    let mut config = AppConfig::default_with_dirs();
    config.cache_dir = std::env::temp_dir().join("kaze-scoring-snapshot");
    let pool = create_in_memory_pool().await.unwrap();
    sqlx::query("INSERT OR REPLACE INTO application_settings(key,value,updated_at) VALUES('enable_jiosaavn','false',0)")
        .execute(&pool).await.unwrap();
    let p = CoreProcessor::new_with_backend(pool.clone(), config, Box::new(MockAudioBackend::new()));
    let songs = [
        ("Anthem", "Alpha", "Rock", 2015, 12, 2, false),
        ("Ballad", "Alpha", "Rock", 2016, 6, 20, false),
        ("Chorus", "Beta", "Pop", 2019, 3, 1, false),
        ("Drift", "Gamma", "Jazz", 1995, 1, 40, false),
        ("Echo", "Delta", "Rock", 2021, 0, 0, false),
        ("Fable", "Epsilon", "Pop", 2005, 0, 0, false),
        ("Gale", "Beta", "Pop", 2018, 2, 5, true),
        ("Haze", "Zeta", "Metal", 1988, 0, 0, false),
    ];
    for (title, artist, genre, year, plays, age, skipped) in songs {
        let id = local(&p, title, artist, genre, year).await;
        for _ in 0..plays {
            listen(&pool, &id, 1.0, age, false).await;
        }
        if skipped {
            for _ in 0..4 {
                listen(&pool, &id, 0.1, age, true).await;
            }
        }
        if title == "Haze" {
            sqlx::query("INSERT INTO track_statistics(user_id,track_id,manual_like) VALUES('default',?,-1)")
                .bind(&id).execute(&pool).await.unwrap();
        }
    }
    for (id, artist, genre) in [
        ("on-alpha", "Alpha", Some("Rock")),
        ("on-rock", "Newcomer", Some("Rock")),
        ("on-pop", "Popper", Some("Pop")),
        ("on-noise", "Stranger", Some("Noise")),
        ("on-beta", "Beta", None),
        ("on-alpha-2", "Alpha", Some("Rock")),
        ("on-alpha-3", "Alpha", Some("Rock")),
        ("on-alpha-4", "Alpha", None),
        ("on-alpha-5", "Alpha", Some("Pop")),
    ] {
        external(&pool, id, artist, genre).await;
    }
    p
}

fn render_flow(rows: &[FlowTrackItem]) -> String {
    rows.iter()
        .map(|t| {
            format!(
                "{}|{:.3}|{}|{}",
                t.title,
                t.score,
                t.reason_category,
                t.explanation.rec_slot.as_deref().unwrap_or("-")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[tokio::test]
async fn flow_candidate_scoring_snapshot() {
    let p = seeded_processor().await;
    let engine = p.flow_engine();
    let mut out = Vec::new();
    for ratio in [0.0, 0.5, 1.0] {
        let preview = engine.generate_flow_tracks("default", ratio, 10, &[]).await.unwrap();
        out.push(format!("preview {ratio}\n{}", render_flow(&preview)));
        let playback = engine.generate_playback_tracks_after("default", ratio, 10, &[], &[]).await.unwrap();
        out.push(format!("playback {ratio}\n{}", render_flow(&playback)));
    }
    let actual = out.join("\n");
    assert_eq!(actual, FLOW_SNAPSHOT.trim(), "\n--- actual ---\n{actual}\n");
}

#[tokio::test]
async fn for_you_feed_snapshot() {
    let p = seeded_processor().await;
    let pool = p.db_pool().clone();
    let coordinator = DiscoveryCoordinator::new(
        Arc::new(SqliteWishlistRepository::new(pool.clone())),
        Arc::new(SqliteTrackRepository::new(pool.clone())),
        Arc::new(TasteProfileEngine::new(pool.clone())),
        None,
    );
    let mut out = Vec::new();
    for refresh in [false, true, true] {
        let rows = coordinator.get_discovery_recommendations("default", 20, refresh).await.unwrap();
        out.push(
            rows.iter()
                .map(|r| format!("{}|{}", r.external_track_id, r.recommendation_reason))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
    let actual = out.join("\n--\n");
    assert_eq!(actual, FOR_YOU_SNAPSHOT.trim(), "\n--- actual ---\n{actual}\n");
}

const FLOW_SNAPSHOT: &str = r#"
preview 0
Echo|0.789|library_exploration|explore
Fable|0.744|library_exploration|explore
Gale|0.482|artist_affinity|explore
Drift|0.471|rediscovery|explore
Ballad|0.409|artist_affinity|explore
Chorus|0.392|artist_affinity|explore
Anthem|0.363|artist_affinity|explore
playback 0
Song on-rock|0.789|online_discovery|explore
Echo|0.789|library_exploration|explore
Song on-pop|0.744|online_discovery|explore
Fable|0.744|library_exploration|explore
Song on-alpha|0.667|online_discovery|explore
Song on-beta|0.451|online_discovery|explore
Drift|0.471|rediscovery|explore
Song on-alpha-2|0.667|online_discovery|explore
Gale|0.482|artist_affinity|explore
Song on-alpha-3|0.667|online_discovery|explore
preview 0.5
Ballad|0.613|artist_affinity|familiar
Echo|0.514|library_exploration|explore
Gale|0.513|artist_affinity|familiar
Fable|0.469|library_exploration|explore
Drift|0.357|rediscovery|explore
Anthem|0.480|artist_affinity|familiar
Chorus|0.392|artist_affinity|familiar
playback 0.5
Ballad|0.613|artist_affinity|familiar
Song on-rock|0.514|online_discovery|explore
Echo|0.514|library_exploration|explore
Gale|0.513|artist_affinity|familiar
Song on-pop|0.469|online_discovery|explore
Fable|0.469|library_exploration|explore
Song on-alpha|0.531|online_discovery|explore
Drift|0.357|rediscovery|explore
Song on-beta|0.309|online_discovery|explore
Song on-alpha-2|0.531|online_discovery|explore
preview 1
Ballad|0.817|artist_affinity|familiar
Gale|0.543|artist_affinity|familiar
Drift|0.243|rediscovery|familiar
Echo|0.239|library_exploration|familiar
Fable|0.194|library_exploration|familiar
Anthem|0.597|artist_affinity|familiar
Chorus|0.393|artist_affinity|familiar
playback 1
Ballad|0.817|artist_affinity|familiar
Gale|0.543|artist_affinity|familiar
Drift|0.243|rediscovery|familiar
Echo|0.239|library_exploration|familiar
Song on-rock|0.239|online_discovery|explore
Anthem|0.597|artist_affinity|familiar
Fable|0.194|library_exploration|familiar
Song on-pop|0.194|online_discovery|explore
Chorus|0.393|artist_affinity|familiar
Song on-alpha|0.396|online_discovery|explore
"#;

const FOR_YOU_SNAPSHOT: &str = r#"
on-alpha|Based on your listening habit for Alpha (66% affinity)
on-rock|Matches your Rock listening
on-pop|Matches your Pop listening
on-beta|Based on your listening habit for Beta (63% affinity)
on-alpha-2|Based on your listening habit for Alpha (66% affinity)
--
on-alpha-3|Based on your listening habit for Alpha (66% affinity)
on-rock|Matches your Rock listening
on-pop|Matches your Pop listening
on-beta|Based on your listening habit for Beta (63% affinity)
on-alpha-5|Based on your listening habit for Alpha (66% affinity)
--
on-alpha|Based on your listening habit for Alpha (66% affinity)
on-rock|Matches your Rock listening
on-pop|Matches your Pop listening
on-beta|Based on your listening habit for Beta (63% affinity)
on-alpha-2|Based on your listening habit for Alpha (66% affinity)
"#;
