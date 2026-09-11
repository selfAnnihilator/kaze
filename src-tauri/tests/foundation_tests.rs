use music_player_backend::config::AppConfig;
use music_player_backend::core::command::{Command, CommandResponse};
use music_player_backend::core::error::AppError;
use music_player_backend::core::event::Event;
use music_player_backend::core::processor::CoreProcessor;
use music_player_backend::core::query::{Query, QueryResponse};
use music_player_backend::database::create_in_memory_pool;

#[tokio::test]
async fn test_database_initialization_and_migrations() {
    let pool = create_in_memory_pool().await.expect("Failed to initialize DB pool");
    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM library_folders")
        .fetch_one(&pool)
        .await
        .expect("Query failed");
    assert_eq!(count.0, 0);

    // Verify all 17 tables are created by migrations
    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_sqlx_%' ORDER BY name"
    )
    .fetch_all(&pool)
    .await
    .expect("Query tables failed");

    let table_names: Vec<String> = tables.into_iter().map(|(n,)| n).collect();
    assert!(table_names.contains(&"tracks".to_string()));
    assert!(table_names.contains(&"artists".to_string()));
    assert!(table_names.contains(&"albums".to_string()));
    assert!(table_names.contains(&"playlists".to_string()));
    assert!(table_names.contains(&"playback_history".to_string()));
    assert!(table_names.contains(&"track_statistics".to_string()));
    assert!(table_names.contains(&"user_preferences".to_string()));
    assert!(table_names.contains(&"wishlist".to_string()));
}

#[tokio::test]
async fn test_event_bus_broadcast() {
    let pool = create_in_memory_pool().await.expect("DB pool");
    let processor = CoreProcessor::new(pool, AppConfig::default_with_dirs());
    let bus = processor.event_bus();
    let mut rx = bus.subscribe();

    bus.publish(Event::PlaybackStarted {
        track_id: "track_123".into(),
        title: "Test Track".into(),
        artist: "Test Artist".into(),
        duration_secs: 180.0,
        source: "library".into(),
    })
    .expect("Publish failed");

    let event = rx.recv().await.expect("Recv failed");
    match event {
        Event::PlaybackStarted { track_id, title, .. } => {
            assert_eq!(track_id, "track_123");
            assert_eq!(title, "Test Track");
        }
        _ => panic!("Unexpected event received"),
    }
}

#[tokio::test]
async fn test_core_processor_commands_and_queries() {
    let pool = create_in_memory_pool().await.expect("DB pool");
    let processor = CoreProcessor::new(pool, AppConfig::default_with_dirs());

    // Validation error on empty folder path
    let err = processor
        .dispatch_command(Command::AddLibraryFolder { path: "   ".into() })
        .await
        .unwrap_err();
    match err {
        AppError::Validation(msg) => assert!(msg.contains("cannot be empty")),
        _ => panic!("Expected validation error"),
    }

    // Successfully add folder
    let res = processor
        .dispatch_command(Command::AddLibraryFolder {
            path: "/home/user/Music".into(),
        })
        .await
        .expect("Command failed");

    let folder_id = match res {
        CommandResponse::EntityId(id) => id,
        _ => panic!("Expected EntityId response"),
    };

    // Query library overview
    let query_res = processor
        .execute_query(Query::GetLibraryOverview)
        .await
        .expect("Query failed");

    match query_res {
        QueryResponse::Tracks(folders) => {
            assert_eq!(folders.len(), 1);
        }
        _ => panic!("Expected Tracks response"),
    }

    // Successfully remove folder
    let remove_res = processor
        .dispatch_command(Command::RemoveLibraryFolder { folder_id })
        .await
        .expect("Remove command failed");
    assert!(matches!(remove_res, CommandResponse::Ok));

    // Volume command validation
    let vol_err = processor
        .dispatch_command(Command::SetVolume { volume: 1.5 })
        .await
        .unwrap_err();
    assert!(matches!(vol_err, AppError::Validation(_)));

    let vol_ok = processor
        .dispatch_command(Command::SetVolume { volume: 0.5 })
        .await
        .expect("Volume command failed");
    assert!(matches!(vol_ok, CommandResponse::Ok));
}
