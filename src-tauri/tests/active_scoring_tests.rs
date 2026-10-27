use music_player_backend::{
    config::HistoryConfig,
    core::{event::Event, event_bus::EventBus},
    database::{
        create_in_memory_pool,
        models::{ExternalTrackRecord, PlaybackHistoryRecord},
        repositories::{
            HistoryRepository, SqliteHistoryRepository, SqliteStatsRepository,
            SqliteTrackRepository, SqliteWishlistRepository, WishlistRepository,
        },
    },
    discovery::DiscoveryCoordinator,
    history::service::HistoryService,
    recommendations::{mixes::ensure_online_track, TasteProfileEngine},
};
use sqlx::SqlitePool;
use std::{sync::Arc, time::Duration};

async fn song(pool: &SqlitePool, id: &str, artist: &str) {
    ensure_online_track(pool, id, id, Some(artist), None, Some(240.0), None, None)
        .await
        .unwrap();
}

async fn listen(
    pool: &SqlitePool,
    id: &str,
    user: &str,
    source: &str,
    fraction: f64,
    age_days: i64,
    skipped: bool,
) {
    let now = chrono::Utc::now().timestamp() - age_days * 86400;
    SqliteHistoryRepository::new(pool.clone())
        .record_playback(&PlaybackHistoryRecord {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: user.into(),
            track_id: id.into(),
            started_at: now,
            ended_at: now + 240,
            seconds_listened: fraction.min(1.0) * 240.0,
            percentage_listened: fraction,
            completed: i64::from(fraction >= 1.0 && !skipped),
            skipped: i64::from(skipped),
            source: source.into(),
            playlist_id: None,
            recommendation_session_id: None,
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn partial_listens_all_sources_and_lifetime_history_keep_their_strength() {
    let pool = create_in_memory_pool().await.unwrap();
    for (id, artist) in [
        ("partial", "Partial Artist"),
        ("full", "Full Artist"),
        ("old", "Old Artist"),
        ("other", "Other Account"),
    ] {
        song(&pool, id, artist).await;
    }
    for source in ["library", "online", "playlist", "collection", "flow"] {
        listen(&pool, "partial", "default", source, 0.5, 0, false).await;
        listen(&pool, "full", "default", source, 1.0, 0, false).await;
    }
    listen(&pool, "old", "default", "library", 1.0, 180, false).await;
    listen(&pool, "other", "different-user", "online", 1.0, 0, false).await;
    // Online genre enrichment participates even if the tracks row has no genre ID.
    sqlx::query("UPDATE external_tracks SET genre = 'Jazz' WHERE id = 'partial'")
        .execute(&pool)
        .await
        .unwrap();
    let profile = TasteProfileEngine::new(pool)
        .compute_taste_profile("default")
        .await
        .unwrap();
    let affinity = |name: &str| {
        profile
            .top_artists
            .iter()
            .find(|a| a.display_name == name)
            .unwrap()
            .affinity
    };
    assert!(
        (affinity("Partial Artist") - 0.5).abs() < 0.01,
        "fractional listens should retain half strength"
    );
    assert!(
        affinity("Old Artist") > 0.0,
        "history older than 90 days must remain relevant"
    );
    assert_eq!(profile.total_sessions_analyzed, 11);
    assert!(profile
        .top_artists
        .iter()
        .all(|a| a.display_name != "Other Account"));
    assert!(profile.genre_affinities["Jazz"] > 0.0);
}

#[tokio::test]
async fn repeated_listening_changes_for_you_order_and_skips_are_not_positive_affinity() {
    let pool = create_in_memory_pool().await.unwrap();
    for (id, artist) in [
        ("a", "Artist A"),
        ("b", "Artist B"),
        ("skip", "Skipped Artist"),
    ] {
        song(&pool, id, artist).await;
    }
    let wishlist = Arc::new(SqliteWishlistRepository::new(pool.clone()));
    for (id, artist) in [
        ("new-a", "Artist A"),
        ("new-b", "Artist B"),
        ("new-skip", "Skipped Artist"),
        ("chart", "Unrelated Chart Artist"),
    ] {
        wishlist
            .upsert_external_track(&ExternalTrackRecord {
                id: id.into(),
                provider: "jiosaavn".into(),
                provider_id: id.into(),
                title: format!("Fresh {id}"),
                artist: artist.into(),
                album: None,
                duration_secs: Some(240.0),
                cover_art_url: None,
                preview_url: None,
                genre: None,
                match_status: "NOT_FOUND".into(),
                matched_local_track_id: None,
                created_at: chrono::Utc::now().timestamp(),
            })
            .await
            .unwrap();
    }
    let engine = Arc::new(TasteProfileEngine::new(pool.clone()));
    let coordinator = DiscoveryCoordinator::new(
        wishlist,
        Arc::new(SqliteTrackRepository::new(pool.clone())),
        engine.clone(),
        None,
    );
    listen(&pool, "a", "default", "library", 1.0, 0, false).await;
    listen(&pool, "b", "default", "online", 0.5, 0, false).await;
    listen(&pool, "skip", "default", "flow", 0.02, 0, true).await;
    assert!(
        coordinator
            .get_discovery_recommendations("different-user", 10, false)
            .await
            .unwrap()
            .is_empty(),
        "another account's online history cannot become a library taste seed"
    );
    let before = coordinator
        .get_discovery_recommendations("default", 1, false)
        .await
        .unwrap();
    assert_eq!(before[0].artist, "Artist A");
    for _ in 0..4 {
        listen(&pool, "b", "default", "collection", 1.0, 0, false).await;
    }
    let after = coordinator
        .get_discovery_recommendations("default", 10, false)
        .await
        .unwrap();
    assert_eq!(after[0].artist, "Artist B");
    assert!(after
        .iter()
        .all(|r| r.artist != "Skipped Artist" && r.artist != "Unrelated Chart Artist"));
    let refreshed = coordinator
        .get_discovery_recommendations("default", 10, true)
        .await
        .unwrap();
    assert_eq!(
        after
            .iter()
            .map(|r| &r.external_track_id)
            .collect::<Vec<_>>(),
        refreshed
            .iter()
            .map(|r| &r.external_track_id)
            .collect::<Vec<_>>(),
        "refresh must not inject random ranking noise"
    );
    let profile = engine.compute_taste_profile("default").await.unwrap();
    assert_eq!(
        profile
            .top_artists
            .iter()
            .find(|a| a.display_name == "Skipped Artist")
            .unwrap()
            .affinity,
        0.0
    );
}

#[tokio::test]
async fn recommendation_refresh_event_follows_successful_history_commit() {
    let pool = create_in_memory_pool().await.unwrap();
    song(&pool, "playing", "Native Artist").await;
    let bus = Arc::new(EventBus::default());
    let service = HistoryService::new(
        Arc::new(SqliteHistoryRepository::new(pool.clone())),
        Arc::new(SqliteStatsRepository::new(pool.clone())),
        HistoryConfig::default(),
        bus.clone(),
        None,
    );
    let mut events = bus.subscribe();
    for session in ["committed", "failed"] {
        if session == "failed" {
            sqlx::query("CREATE TRIGGER reject_history BEFORE INSERT ON playback_history BEGIN SELECT RAISE(ABORT, 'test failure'); END")
                .execute(&pool).await.unwrap();
        }
        bus.publish(Event::PlaybackStarted {
            session_id: session.into(),
            track_id: "playing".into(),
            title: "Playing".into(),
            artist: "Native Artist".into(),
            duration_secs: 240.0,
            source: "online".into(),
        })
        .unwrap();
        bus.publish(Event::TrackFinished {
            session_id: session.into(),
            track_id: "playing".into(),
            seconds_listened: 240.0,
            completed: true,
        })
        .unwrap();
        if session == "committed" {
            tokio::time::timeout(Duration::from_secs(2), async {
                loop {
                    if let Event::TasteProfileChanged { user_id } = events.recv().await.unwrap() {
                        assert_eq!(user_id, "default");
                        let count: i64 = sqlx::query_scalar(
                            "SELECT COUNT(*) FROM playback_history WHERE id='committed'",
                        )
                        .fetch_one(&pool)
                        .await
                        .unwrap();
                        assert_eq!(count, 1, "refresh event cannot precede commit");
                        break;
                    }
                }
            })
            .await
            .expect("post-commit refresh event");
        }
    }
    service.shutdown().await;
    while let Ok(event) = events.try_recv() {
        assert!(
            !matches!(event, Event::TasteProfileChanged { .. }),
            "failed history writes must not announce fresh taste"
        );
    }
}
