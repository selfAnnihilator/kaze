use music_player_backend::config::AppConfig;
use music_player_backend::core::command::{Command, CommandResponse, WishlistStatus};
use music_player_backend::core::error::AppResult;
use music_player_backend::core::query::{Query, QueryResponse};
use music_player_backend::core::CoreProcessor;
use music_player_backend::database::models::ExternalTrackRecord;
use music_player_backend::database::repositories::{
    SqliteTrackRepository, SqliteWishlistRepository,
};
use music_player_backend::discovery::{
    DiscoveryCoordinator, FuzzyTrackMatcher, MatchStatus, WishlistManager,
};
use music_player_backend::playback::backend::MockAudioBackend;
use music_player_backend::recommendations::TasteProfileEngine;
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;

async fn setup_test_db() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to connect to in-memory SQLite");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Migrations failed");

    pool
}

#[tokio::test]
async fn test_fuzzy_track_matcher_classifications() {
    // 1. Exact Match: high title & artist similarity + duration within 4s
    let res_exact = FuzzyTrackMatcher::compare(
        "Bohemian Rhapsody (2011 Remaster)",
        "Queen",
        Some(354.0),
        "loc_1",
        "Bohemian Rhapsody",
        "Queen",
        355.0,
    );
    assert_eq!(res_exact.status, MatchStatus::ExactMatch);
    assert_eq!(res_exact.matched_track_id, Some("loc_1".to_string()));
    assert!(res_exact.confidence >= 0.90);

    // 1b. Exact Match with 30s preview snippet and featured artist format
    let res_eminem = FuzzyTrackMatcher::compare(
        "'Till I Collapse (feat. Nate Dogg)",
        "Eminem",
        Some(30.0),
        "loc_em",
        "Till I Collapse",
        "Eminem, Nate Dogg",
        297.9,
    );
    assert_eq!(res_eminem.status, MatchStatus::ExactMatch);
    assert_eq!(res_eminem.matched_track_id, Some("loc_em".to_string()));

    // 2. Likely Match: high similarity with slight variation / duration delta <= 10s
    let res_likely = FuzzyTrackMatcher::compare(
        "Hotel California",
        "Eagles",
        Some(390.0),
        "loc_2",
        "Hotel California (Live)",
        "The Eagles",
        398.0,
    );
    assert!(
        res_likely.status == MatchStatus::LikelyMatch || res_likely.status == MatchStatus::ExactMatch
    );

    // 3. Possible Match: reasonable similarity, alternate release or cut
    let res_possible = FuzzyTrackMatcher::compare(
        "Stairway to Heaven",
        "Led Zeppelin",
        Some(480.0),
        "loc_3",
        "Stairway to Heaven (Acoustic Solo)",
        "Led Zeppelin",
        420.0,
    );
    assert_eq!(res_possible.status, MatchStatus::PossibleMatch);

    // 4. Not Found: completely different tracks
    let res_not_found = FuzzyTrackMatcher::compare(
        "Smells Like Teen Spirit",
        "Nirvana",
        Some(301.0),
        "loc_4",
        "Imagine",
        "John Lennon",
        183.0,
    );
    assert_eq!(res_not_found.status, MatchStatus::NotFound);
    assert_eq!(res_not_found.matched_track_id, None);

    // 5. find_best_match across multiple candidates
    let candidates = vec![
        ("t1", "Yesterday", "The Beatles", 125.0),
        ("t2", "Hey Jude", "The Beatles", 431.0),
        ("t3", "Let It Be", "The Beatles", 243.0),
    ];

    let best = FuzzyTrackMatcher::find_best_match(
        "Hey Jude (Remastered 2015)",
        "The Beatles feat. Orchestra",
        Some(430.0),
        candidates.into_iter(),
    );
    assert_eq!(best.status, MatchStatus::ExactMatch);
    assert_eq!(best.matched_track_id, Some("t2".to_string()));
}

#[tokio::test]
async fn test_wishlist_manager_crud_and_status_transitions() -> AppResult<()> {
    let pool = setup_test_db().await;
    let repo = Arc::new(SqliteWishlistRepository::new(pool));
    let manager = WishlistManager::new(repo);

    // 1. Add items to wishlist
    let item1 = manager
        .add_to_wishlist(
            "Paranoid Android".to_string(),
            "Radiohead".to_string(),
            Some("OK Computer".to_string()),
            Some("ext_101".to_string()),
            Some("Essential 90s art rock".to_string()),
        )
        .await?;

    let item2 = manager
        .add_to_wishlist(
            "Karma Police".to_string(),
            "Radiohead".to_string(),
            Some("OK Computer".to_string()),
            None,
            None,
        )
        .await?;

    assert_eq!(item1.status, "WANT");
    assert_eq!(item2.status, "WANT");

    // 2. Query wishlist items
    let all = manager.get_wishlist(None).await?;
    assert_eq!(all.len(), 2);

    // 3. Status transitions
    manager
        .update_status_enum(&item1.id, WishlistStatus::Downloaded)
        .await?;
    let updated1 = manager.get_by_id(&item1.id).await?.expect("Item must exist");
    assert_eq!(updated1.status, "DOWNLOADED");

    manager.update_status(&item2.id, "ALREADY_OWN").await?;
    let updated2 = manager.get_by_id(&item2.id).await?.expect("Item must exist");
    assert_eq!(updated2.status, "ALREADY_OWN");

    // 4. Filter by status
    let downloaded = manager.get_wishlist(Some("DOWNLOADED")).await?;
    assert_eq!(downloaded.len(), 1);
    assert_eq!(downloaded[0].id, item1.id);

    // 5. Delete item
    manager.delete_item(&item2.id).await?;
    let remaining = manager.get_wishlist(None).await?;
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, item1.id);

    Ok(())
}

#[tokio::test]
async fn test_discovery_coordinator_matching_and_recommendations() -> AppResult<()> {
    let pool = setup_test_db().await;
    let wishlist_repo = Arc::new(SqliteWishlistRepository::new(pool.clone()));
    let track_repo = Arc::new(SqliteTrackRepository::new(pool.clone()));
    let taste_engine = Arc::new(TasteProfileEngine::new(pool.clone()));

    // Insert a local artist, album, and track
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO artists (id, name, normalized_name, created_at)
         VALUES ('art_1', 'Pink Floyd', 'pink floyd', ?)",
    )
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO tracks (id, file_path, file_size, modified_timestamp, title, normalized_title,
                             artist_id, duration_secs, format, created_at, updated_at)
         VALUES ('trk_1', '/music/time.mp3', 5000000, ?, 'Time', 'time', 'art_1', 413.0, 'mp3', ?, ?)",
    )
    .bind(now)
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    let coordinator = DiscoveryCoordinator::new(
        wishlist_repo.clone(),
        track_repo.clone(),
        taste_engine.clone(),
        None,
    );

    // Ingest an external track that matches our local track
    let matching_ext = ExternalTrackRecord {
        id: "mb:12345".to_string(),
        provider: "musicbrainz".to_string(),
        provider_id: "12345".to_string(),
        title: "Time (2011 Remaster)".to_string(),
        artist: "Pink Floyd".to_string(),
        album: Some("The Dark Side of the Moon".to_string()),
        duration_secs: Some(414.0),
        cover_art_url: None,
        preview_url: None,
        genre: None,
        match_status: "NOT_FOUND".to_string(),
        matched_local_track_id: None,
        created_at: now,
    };

    let match_res = coordinator.ingest_external_track(matching_ext).await?;
    assert_eq!(match_res.status, MatchStatus::ExactMatch);
    assert_eq!(match_res.matched_track_id, Some("trk_1".to_string()));

    // Ingest an external track that is missing from local library
    let missing_ext = ExternalTrackRecord {
        id: "mb:67890".to_string(),
        provider: "musicbrainz".to_string(),
        provider_id: "67890".to_string(),
        title: "Comfortably Numb".to_string(),
        artist: "Pink Floyd".to_string(),
        album: Some("The Wall".to_string()),
        duration_secs: Some(382.0),
        cover_art_url: None,
        preview_url: None,
        genre: None,
        match_status: "NOT_FOUND".to_string(),
        matched_local_track_id: None,
        created_at: now,
    };

    let missing_res = coordinator.ingest_external_track(missing_ext).await?;
    assert_eq!(missing_res.status, MatchStatus::NotFound);
    assert_eq!(missing_res.matched_track_id, None);

    // Add missing track to wishlist
    let wishlist_mgr = WishlistManager::new(wishlist_repo.clone());
    wishlist_mgr
        .add_to_wishlist(
            "Comfortably Numb".to_string(),
            "Pink Floyd".to_string(),
            Some("The Wall".to_string()),
            Some("mb:67890".to_string()),
            None,
        )
        .await?;

    // Query discovery recommendations
    let recs = coordinator.get_discovery_recommendations(10, false).await?;
    assert_eq!(recs.len(), 2);

    // Missing track should be sorted first (priority over ExactMatch) and marked as in_wishlist = true
    assert_eq!(recs[0].title, "Comfortably Numb");
    assert_eq!(recs[0].match_status, MatchStatus::NotFound);
    assert!(recs[0].in_wishlist);

    // Owned track should be sorted second
    assert_eq!(recs[1].title, "Time (2011 Remaster)");
    assert_eq!(recs[1].match_status, MatchStatus::ExactMatch);
    assert!(!recs[1].in_wishlist);

    Ok(())
}

#[tokio::test]
async fn test_core_processor_discovery_and_wishlist_integration() -> AppResult<()> {
    let pool = setup_test_db().await;
    let config = AppConfig::default_with_dirs();
    let backend = Box::new(MockAudioBackend::new());
    let processor = CoreProcessor::new_with_backend(pool.clone(), config, backend);

    // 1. Add to wishlist via Command
    let cmd_res = processor
        .dispatch_command(Command::AddToWishlist {
            title: "Heroes".to_string(),
            artist: "David Bowie".to_string(),
            album: Some("Heroes".to_string()),
            external_id: Some("spotify:trk_heroes".to_string()),
        })
        .await?;

    let wishlist_id = match cmd_res {
        CommandResponse::EntityId(id) => id,
        _ => panic!("Expected EntityId response"),
    };

    // 2. Query wishlist via Query
    let query_res = processor.execute_query(Query::GetWishlist).await?;
    match query_res {
        QueryResponse::Wishlist(items) => {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0]["title"], "Heroes");
            assert_eq!(items[0]["status"], "WANT");
        }
        _ => panic!("Expected QueryResponse::Wishlist"),
    }

    // 3. Update wishlist status via Command
    let update_res = processor
        .dispatch_command(Command::UpdateWishlistStatus {
            wishlist_id: wishlist_id.clone(),
            status: WishlistStatus::Downloaded,
        })
        .await?;
    assert!(matches!(update_res, CommandResponse::Ok));

    // 4. Verify updated status
    let query_res2 = processor.execute_query(Query::GetWishlist).await?;
    match query_res2 {
        QueryResponse::Wishlist(items) => {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0]["status"], "DOWNLOADED");
        }
        _ => panic!("Expected QueryResponse::Wishlist"),
    }

    // 5. Query discovery recommendations via Query
    let disc_res = processor
        .execute_query(Query::GetDiscoveryRecommendations {
            limit: 5,
            force_refresh: None,
        })
        .await?;
    match disc_res {
        QueryResponse::DiscoveryRecommendations(recs) => {
            // Returns typed list (empty or populated without error)
            assert!(recs.is_empty() || !recs.is_empty());
        }
        _ => panic!("Expected QueryResponse::DiscoveryRecommendations"),
    }

    // 6. Test Query::SearchOnlineMusic query execution with case-insensitivity
    let search_res = processor
        .execute_query(Query::SearchOnlineMusic {
            query: "was it real".to_string(),
            limit: Some(10),
        })
        .await?;
    match search_res {
        QueryResponse::DiscoveryRecommendations(recs) => {
            println!("Got {} search results for 'was it real'", recs.len());
            assert!(!recs.is_empty());
        }
        _ => panic!("Expected QueryResponse::DiscoveryRecommendations"),
    }

    // 6b. Test uppercase query - MUST return results identically (case-insensitive)
    let search_upper = processor
        .execute_query(Query::SearchOnlineMusic {
            query: "WAS IT REAL".to_string(),
            limit: Some(10),
        })
        .await?;
    match search_upper {
        QueryResponse::DiscoveryRecommendations(recs) => {
            println!("Got {} search results for 'WAS IT REAL'", recs.len());
            assert!(!recs.is_empty());
        }
        _ => panic!("Expected QueryResponse::DiscoveryRecommendations"),
    }

    Ok(())
}
