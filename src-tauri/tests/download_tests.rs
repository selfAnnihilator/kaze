use music_player_backend::config::AppConfig;
use music_player_backend::core::command::{Command, CommandResponse};
use music_player_backend::core::error::AppResult;
use music_player_backend::core::event::Event;
use music_player_backend::core::query::{Query, QueryResponse};
use music_player_backend::core::CoreProcessor;
use music_player_backend::database::repositories::{
    DownloadRepository, SqliteDownloadRepository, SqliteWishlistRepository,
};
use music_player_backend::discovery::WishlistManager;
use music_player_backend::downloads::{
    DownloadProvider, DownloadSearchResult, DownloadService, DownloadStatus, MockDownloadProvider,
};
use music_player_backend::library::LibraryService;
use music_player_backend::playback::backend::MockAudioBackend;
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;
use tempfile::tempdir;

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
async fn test_download_provider_search_and_availability() -> AppResult<()> {
    let provider = MockDownloadProvider::new();

    // 1. Available check
    assert!(provider.is_available());

    // 2. Default search
    let results = provider.search("Pink Floyd Time").await?;
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].format, "flac");
    assert_eq!(results[1].format, "mp3");

    // 3. Unavailable check
    provider.set_available(false);
    assert!(!provider.is_available());
    let empty_res = provider.search("Pink Floyd Time").await?;
    assert!(empty_res.is_empty());

    // 4. Canned results
    provider.set_available(true);
    provider
        .set_canned_results(vec![DownloadSearchResult {
            id: "canned_1".to_string(),
            provider: "mock_soulseek".to_string(),
            username: "lossless_curator".to_string(),
            filename: "FLAC/Comfortably Numb.flac".to_string(),
            file_size: 42_000_000,
            bitrate: Some(1024),
            sample_rate: Some(96000),
            format: "flac".to_string(),
            slots_free: true,
            speed_bps: 5_000_000,
        }])
        .await;

    let canned = provider.search("Comfortably Numb").await?;
    assert_eq!(canned.len(), 1);
    assert_eq!(canned[0].id, "canned_1");

    Ok(())
}

#[tokio::test]
async fn test_core_processor_search_soulseek() -> AppResult<()> {
    let pool = setup_test_db().await;
    let config = AppConfig::default_with_dirs();
    let processor = CoreProcessor::new(pool, config);
    let res = processor.dispatch_command(Command::SearchSoulseek {
        artist: "John Michael Howell".to_string(),
        title: "Pinky Up".to_string(),
        album: None,
    }).await?;
    match res {
        CommandResponse::SearchResults(results) => {
            println!("Got {} results!", results.len());
            assert!(!results.is_empty());
        }
        _ => panic!("Unexpected response"),
    }
    Ok(())
}

#[tokio::test]
async fn test_download_service_start_download_and_progress_flow() -> AppResult<()> {
    let pool = setup_test_db().await;
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let download_dir = temp_dir.path().to_path_buf();

    let event_bus = Arc::new(music_player_backend::core::EventBus::default());
    let mut rx = event_bus.subscribe();

    let download_repo = Arc::new(SqliteDownloadRepository::new(pool.clone()));
    let mock_provider = Arc::new(MockDownloadProvider::new());
    let library_service = Arc::new(LibraryService::new(pool.clone(), event_bus.clone()));
    let wishlist_repo = Arc::new(SqliteWishlistRepository::new(pool.clone()));
    let wishlist_mgr = Arc::new(WishlistManager::new(wishlist_repo));

    let service = DownloadService::new(
        download_repo.clone(),
        mock_provider.clone(),
        event_bus.clone(),
        library_service,
        wishlist_mgr,
        download_dir,
        false, // auto_import disabled for unit test
    );

    // 1. Search to populate search cache
    let results = service.search("Radiohead Creep").await?;
    assert!(!results.is_empty());
    let search_id = &results[0].id;

    // 2. Start download
    let task = service.start_download(search_id, None).await?;
    assert_eq!(task.status, "DOWNLOADING");

    // Verify DownloadQueued event was emitted
    let ev = rx.recv().await.expect("Expected event");
    match ev {
        Event::DownloadQueued { task_id, .. } => {
            assert_eq!(task_id, task.id);
        }
        other => panic!("Unexpected event: {:?}", other),
    }

    // 3. Poll progress
    let progress = service
        .poll_task(&task.id)
        .await?
        .expect("Progress must exist");
    assert_eq!(progress.status, DownloadStatus::Downloading);

    // 4. Complete job on provider and poll again
    let pt_id = task.provider_task_id.as_deref().unwrap();
    mock_provider.complete_job(pt_id).await;

    let comp_progress = service
        .poll_task(&task.id)
        .await?
        .expect("Progress must exist");
    assert_eq!(comp_progress.status, DownloadStatus::Completed);

    // Verify database record updated to COMPLETED
    let db_task = download_repo
        .get_task_by_id(&task.id)
        .await?
        .expect("Task must exist");
    assert_eq!(db_task.status, "COMPLETED");
    assert!(db_task.completed_at.is_some());

    Ok(())
}

#[tokio::test]
async fn test_download_service_wishlist_integration_and_auto_import() -> AppResult<()> {
    let pool = setup_test_db().await;
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let download_dir = temp_dir.path().to_path_buf();

    let event_bus = Arc::new(music_player_backend::core::EventBus::default());
    let download_repo = Arc::new(SqliteDownloadRepository::new(pool.clone()));
    let mock_provider = Arc::new(MockDownloadProvider::new());
    let library_service = Arc::new(LibraryService::new(pool.clone(), event_bus.clone()));
    let wishlist_repo = Arc::new(SqliteWishlistRepository::new(pool.clone()));
    let wishlist_mgr = Arc::new(WishlistManager::new(wishlist_repo));

    let service = DownloadService::new(
        download_repo,
        mock_provider.clone(),
        event_bus,
        library_service,
        wishlist_mgr.clone(),
        download_dir,
        false,
    );

    // 1. Add item to wishlist
    let wl_item = wishlist_mgr
        .add_to_wishlist(
            "Starman".to_string(),
            "David Bowie".to_string(),
            None,
            None,
            None,
        )
        .await?;
    assert_eq!(wl_item.status, "WANT");

    // 2. Search for wishlist item
    let results = service.search_wishlist_item(&wl_item.id).await?;
    assert!(!results.is_empty());

    // 3. Start download linked to wishlist
    let task = service
        .start_download(&results[0].id, Some(wl_item.id.clone()))
        .await?;
    assert_eq!(task.wishlist_id, Some(wl_item.id.clone()));

    // 4. Complete job and verify status update on wishlist
    let pt_id = task.provider_task_id.as_deref().unwrap();
    mock_provider.complete_job(pt_id).await;
    service.poll_task(&task.id).await?;

    // Check wishlist item transitioned to DOWNLOADED
    let updated_wl = wishlist_mgr
        .get_by_id(&wl_item.id)
        .await?
        .expect("Wishlist item must exist");
    assert_eq!(updated_wl.status, "DOWNLOADED");

    Ok(())
}

#[tokio::test]
async fn test_download_service_cancel_flow() -> AppResult<()> {
    let pool = setup_test_db().await;
    let temp_dir = tempdir().expect("Failed to create tempdir");
    let download_dir = temp_dir.path().to_path_buf();

    let event_bus = Arc::new(music_player_backend::core::EventBus::default());
    let download_repo = Arc::new(SqliteDownloadRepository::new(pool.clone()));
    let mock_provider = Arc::new(MockDownloadProvider::new());
    let library_service = Arc::new(LibraryService::new(pool.clone(), event_bus.clone()));
    let wishlist_repo = Arc::new(SqliteWishlistRepository::new(pool.clone()));
    let wishlist_mgr = Arc::new(WishlistManager::new(wishlist_repo));

    let service = DownloadService::new(
        download_repo.clone(),
        mock_provider,
        event_bus,
        library_service,
        wishlist_mgr,
        download_dir,
        false,
    );

    let results = service.search("The Beatles Let It Be").await?;
    let task = service.start_download(&results[0].id, None).await?;

    service.cancel_download(&task.id).await?;

    let db_task = download_repo
        .get_task_by_id(&task.id)
        .await?
        .expect("Task must exist");
    assert_eq!(db_task.status, "CANCELLED");

    Ok(())
}

#[tokio::test]
async fn test_core_processor_download_commands_and_queries() -> AppResult<()> {
    let pool = setup_test_db().await;
    let config = AppConfig::default_with_dirs();
    let backend = Box::new(MockAudioBackend::new());
    let mock_provider: Arc<dyn music_player_backend::downloads::DownloadProvider> =
        Arc::new(MockDownloadProvider::new());

    let processor =
        CoreProcessor::new_with_services(pool.clone(), config, backend, Some(mock_provider));

    // 1. Search via Command
    let search_res = processor
        .dispatch_command(Command::SearchSoulseek {
            artist: "Led Zeppelin".to_string(),
            title: "Kashmir".to_string(),
            album: None,
        })
        .await?;

    let search_results = match search_res {
        CommandResponse::SearchResults(items) => items,
        other => panic!("Expected SearchResults response, got: {:?}", other),
    };
    assert_eq!(search_results.len(), 2);
    let first_id = search_results[0]["id"].as_str().unwrap();

    // 2. Start download via Command
    let dl_res = processor
        .dispatch_command(Command::StartDownload {
            search_result_id: first_id.to_string(),
            wishlist_id: None,
        })
        .await?;

    let task_id = match dl_res {
        CommandResponse::DownloadStarted { task_id } => task_id,
        other => panic!("Expected DownloadStarted, got: {:?}", other),
    };

    // 3. Query active downloads via Query
    let query_res = processor
        .execute_query(Query::GetDownloads {
            status_filter: None,
            limit: 10,
        })
        .await?;

    match query_res {
        QueryResponse::Downloads(tasks) => {
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0]["id"], task_id);
            assert_eq!(tasks[0]["status"], "DOWNLOADING");
        }
        other => panic!("Expected Downloads response, got: {:?}", other),
    }

    // 4. Cancel download via Command
    let cancel_res = processor
        .dispatch_command(Command::CancelDownload {
            task_id: task_id.clone(),
        })
        .await?;
    assert!(matches!(cancel_res, CommandResponse::Ok));

    // 5. Verify cancelled status
    let query_res2 = processor
        .execute_query(Query::GetDownloads {
            status_filter: Some("CANCELLED".to_string()),
            limit: 10,
        })
        .await?;

    match query_res2 {
        QueryResponse::Downloads(tasks) => {
            assert_eq!(tasks.len(), 1);
            assert_eq!(tasks[0]["status"], "CANCELLED");
        }
        other => panic!("Expected Downloads response, got: {:?}", other),
    }

    Ok(())
}
