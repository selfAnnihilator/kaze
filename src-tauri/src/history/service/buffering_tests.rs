use super::*;
use crate::database::{create_in_memory_pool, repositories::{SqliteHistoryRepository, SqliteStatsRepository}};

#[tokio::test]
async fn buffer_recovery_does_not_restart_listening_while_paused_or_for_an_old_session() {
    let pool = create_in_memory_pool().await.unwrap();
    let service = HistoryService::new(
        Arc::new(SqliteHistoryRepository::new(pool.clone())),
        Arc::new(SqliteStatsRepository::new(pool)),
        HistoryConfig::default(), Arc::new(EventBus::default()), None,
    );
    service.on_playback_started("current".into(), "song".into(), 240.0, "library".into()).await;
    let buffering = |session: &str, is_buffering| Event::PlaybackBufferingChanged {
        session_id: session.into(), track_id: "song".into(), is_buffering,
    };
    service.handle_event(buffering("current", true)).await;
    assert!(service.current_session.lock().await.as_ref().unwrap().playing_since.is_none());
    service.handle_event(Event::PlaybackPaused {
        session_id: "current".into(), track_id: "song".into(), position_secs: 42.0,
    }).await;
    service.handle_event(buffering("current", false)).await;
    assert!(service.current_session.lock().await.as_ref().unwrap().playing_since.is_none());
    service.handle_event(Event::PlaybackResumed {
        session_id: "current".into(), track_id: "song".into(), position_secs: 42.0,
    }).await;
    service.handle_event(buffering("old", true)).await;
    assert!(service.current_session.lock().await.as_ref().unwrap().playing_since.is_some());
    // No fictional track should be finalized by this event-accounting test.
    service.current_session.lock().await.take();
    service.shutdown().await;
}
