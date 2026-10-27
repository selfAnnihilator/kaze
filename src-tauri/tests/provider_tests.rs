use music_player_backend::{core::{Event,EventBus},database::create_in_memory_pool,providers::ProviderCoordinator};
use std::sync::Arc;
#[tokio::test]
async fn only_jiosaavn_is_announced_as_an_online_provider() {
    let pool=create_in_memory_pool().await.unwrap();let bus=Arc::new(EventBus::default());let mut events=bus.subscribe();
    let temp=tempfile::tempdir().unwrap();
    ProviderCoordinator::new(pool,bus,temp.path().into()).emit_status();
    assert!(matches!(events.try_recv().unwrap(),Event::ProviderStatusChanged{provider,available:true,..} if provider=="jiosaavn"));
    assert!(events.try_recv().is_err());
}
