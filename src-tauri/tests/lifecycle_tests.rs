use std::sync::Arc;
use music_player_backend::{config::AppConfig, core::{command::Command, processor::CoreProcessor, query::{Query, QueryResponse}}, database::init_db_pool, playback::backend::MockAudioBackend};

async fn setup() -> (tempfile::TempDir, Arc<CoreProcessor>, MockAudioBackend) {
    let dir = tempfile::tempdir().unwrap();
    let mut config = AppConfig::default_with_dirs();
    config.database_path = dir.path().join("app.db");
    config.cache_dir = dir.path().join("cache");
    let pool = init_db_pool(&config.database_path).await.unwrap();
    let backend = MockAudioBackend::new();
    let processor = Arc::new(CoreProcessor::new_with_backend(pool, config, Box::new(backend.clone())));
    processor.initialize_close_behavior().await.unwrap();
    (dir, processor, backend)
}

async fn add_track(processor: &CoreProcessor) {
    sqlx::query("INSERT INTO tracks (id, file_path, file_size, modified_timestamp, title, normalized_title, duration_secs, format, created_at, updated_at) VALUES ('track', '/test.wav', 1, 1, 'Track', 'track', 180, 'wav', 1, 1)")
        .execute(processor.db_pool()).await.unwrap();
}

#[tokio::test]
async fn close_setting_defaults_persists_and_is_loaded_before_first_query() {
    let (dir, processor, _) = setup().await;
    assert_eq!(processor.close_button_behavior(), "minimize_to_tray");
    processor.dispatch_command(Command::SetCloseButtonBehavior { behavior: "quit_app".into() }).await.unwrap();
    assert_eq!(processor.close_button_behavior(), "quit_app");
    for invalid in ["", "Quit_app", "quit_app ", "minimize", "anything"] {
        assert!(processor.dispatch_command(Command::SetCloseButtonBehavior { behavior: invalid.into() }).await.is_err());
        assert_eq!(processor.close_button_behavior(), "quit_app");
    }
    processor.shutdown().await.unwrap();
    let pool = init_db_pool(&dir.path().join("app.db")).await.unwrap();
    let mut config = AppConfig::default_with_dirs();
    config.cache_dir = dir.path().join("cache");
    let relaunched = CoreProcessor::new_with_backend(pool, config, Box::new(MockAudioBackend::new()));
    match relaunched.execute_query(Query::GetSettings).await.unwrap() {
        QueryResponse::Settings(value) => assert_eq!(value["close_button_behavior"], "quit_app"),
        _ => panic!("Settings response expected"),
    }
    assert_eq!(relaunched.close_button_behavior(), "quit_app");
    relaunched.dispatch_command(Command::SetCloseButtonBehavior { behavior: "minimize_to_tray".into() }).await.unwrap();
    assert_eq!(relaunched.close_button_behavior(), "minimize_to_tray");
    relaunched.shutdown().await.unwrap();
}

#[tokio::test]
async fn failed_setting_write_keeps_previous_behavior() {
    let (_dir, processor, _) = setup().await;
    // Simulate a storage failure while keeping the processor alive.
    sqlx::query("CREATE TRIGGER reject_close_write BEFORE INSERT ON application_settings WHEN NEW.key = 'close_button_behavior' BEGIN SELECT RAISE(ABORT, 'test storage failure'); END")
        .execute(processor.db_pool()).await.unwrap();
    assert!(processor.dispatch_command(Command::SetCloseButtonBehavior { behavior: "quit_app".into() }).await.is_err());
    assert_eq!(processor.close_button_behavior(), "minimize_to_tray");
    processor.shutdown().await.unwrap();
}

#[tokio::test]
async fn startup_and_concurrent_writes_cannot_restore_an_old_close_setting() {
    let (dir, processor, _) = setup().await;
    let (a, b) = tokio::join!(
        processor.dispatch_command(Command::SetCloseButtonBehavior { behavior: "quit_app".into() }),
        processor.dispatch_command(Command::SetCloseButtonBehavior { behavior: "minimize_to_tray".into() })
    );
    a.unwrap(); b.unwrap();
    let saved: (String,) = sqlx::query_as("SELECT value FROM application_settings WHERE key = 'close_button_behavior'")
        .fetch_one(processor.db_pool()).await.unwrap();
    assert_eq!(processor.close_button_behavior(), saved.0);
    processor.shutdown().await.unwrap();
    assert!(dir.path().join("app.db").is_file());
}

#[tokio::test]
async fn concurrent_quit_drains_the_real_session_once_without_skip_or_completion() {
    let (dir, processor, backend) = setup().await;
    add_track(&processor).await;
    let mut events = processor.event_bus().subscribe();
    processor.dispatch_command(Command::PlayTrack { track_id: "track".into(), source: None }).await.unwrap();
    processor.dispatch_command(Command::Pause).await.unwrap();
    // Shutdown must join the consumer, even if Start/Pause are still queued.
    let (a,b,c) = tokio::join!(processor.shutdown(), processor.shutdown(), processor.shutdown());
    a.unwrap(); b.unwrap(); c.unwrap();
    assert!(backend.is_finished_flag.load(std::sync::atomic::Ordering::SeqCst));
    assert!(processor.dispatch_command(Command::TogglePlayPause).await.is_err());
    assert!(processor.playback_service().resume().await.is_err());
    let stopped = std::iter::from_fn(|| events.try_recv().ok()).filter(|event| matches!(event, music_player_backend::Event::PlaybackStopped { .. })).count();
    assert_eq!(stopped, 1, "duplicate Quit must not manufacture extra stop events");
    let pool = init_db_pool(&dir.path().join("app.db")).await.unwrap();
    let rows: Vec<(i64,i64)> = sqlx::query_as("SELECT completed, skipped FROM playback_history").fetch_all(&pool).await.unwrap();
    assert_eq!(rows, vec![(0,0)]);
    let journal: (i64,) = sqlx::query_as("SELECT count(*) FROM active_playback_journal").fetch_one(&pool).await.unwrap();
    assert_eq!(journal.0, 0, "history commit must finish before closing the database");
    pool.close().await;
}

#[tokio::test]
async fn empty_quit_never_creates_a_history_session() {
    let (dir, processor, _) = setup().await;
    processor.shutdown().await.unwrap();
    let pool = init_db_pool(&dir.path().join("app.db")).await.unwrap();
    let count: (i64,) = sqlx::query_as("SELECT count(*) FROM playback_history").fetch_one(&pool).await.unwrap();
    assert_eq!(count.0, 0);
    pool.close().await;
}

#[test]
fn serialized_close_setting_is_strict_and_defaults_when_missing() {
    let mut config = serde_json::to_value(AppConfig::default_with_dirs()).unwrap();
    config.as_object_mut().unwrap().remove("close_button_behavior");
    let parsed: AppConfig = serde_json::from_value(config.clone()).unwrap();
    assert_eq!(parsed.close_button_behavior, "minimize_to_tray");
    config["close_button_behavior"] = serde_json::json!("invalid");
    assert!(serde_json::from_value::<AppConfig>(config).is_err());
}

#[tokio::test]
async fn interface_style_uses_existing_settings_and_survives_relaunch() {
    let (dir, processor, _) = setup().await;
    match processor.execute_query(Query::GetSettings).await.unwrap() {
        QueryResponse::Settings(value) => assert_eq!(value["interface_style"], "spatial"),
        _ => panic!("Settings expected"),
    }
    add_track(&processor).await;
    processor.dispatch_command(Command::PlayTrack { track_id: "track".into(), source: None }).await.unwrap();
    processor.dispatch_command(Command::Pause).await.unwrap();
    let before = serde_json::to_value(processor.playback_service().get_playback_state().await).unwrap();
    for style in ["brutalist", "spatial", "brutalist"] {
        processor.dispatch_command(Command::SetInterfaceStyle { style: style.into() }).await.unwrap();
        let after = serde_json::to_value(processor.playback_service().get_playback_state().await).unwrap();
        assert_eq!(before, after, "style must not alter playback or queue");
    }
    for invalid in ["", "Spatial", "brutalist ", "standard"] {
        assert!(processor.dispatch_command(Command::SetInterfaceStyle { style: invalid.into() }).await.is_err());
    }
    let saved: (String,) = sqlx::query_as("SELECT value FROM application_settings WHERE key = 'interface_style'")
        .fetch_one(processor.db_pool()).await.unwrap();
    assert_eq!(saved.0, "brutalist");
    processor.shutdown().await.unwrap();
    let pool = init_db_pool(&dir.path().join("app.db")).await.unwrap();
    let mut config = AppConfig::default_with_dirs();
    config.cache_dir = dir.path().join("cache");
    let reopened = CoreProcessor::new_with_backend(pool, config, Box::new(MockAudioBackend::new()));
    match reopened.execute_query(Query::GetSettings).await.unwrap() {
        QueryResponse::Settings(value) => assert_eq!(value["interface_style"], "brutalist"),
        _ => panic!("Settings expected"),
    }
    reopened.shutdown().await.unwrap();
}

#[tokio::test]
async fn interface_style_storage_failure_preserves_saved_choice() {
    let (_dir, processor, _) = setup().await;
    processor.dispatch_command(Command::SetInterfaceStyle { style: "brutalist".into() }).await.unwrap();
    sqlx::query("CREATE TRIGGER reject_style_write BEFORE INSERT ON application_settings WHEN NEW.key = 'interface_style' BEGIN SELECT RAISE(ABORT, 'test storage failure'); END")
        .execute(processor.db_pool()).await.unwrap();
    assert!(processor.dispatch_command(Command::SetInterfaceStyle { style: "spatial".into() }).await.is_err());
    match processor.execute_query(Query::GetSettings).await.unwrap() {
        QueryResponse::Settings(value) => assert_eq!(value["interface_style"], "brutalist"),
        _ => panic!("Settings expected"),
    }
    processor.shutdown().await.unwrap();
}
