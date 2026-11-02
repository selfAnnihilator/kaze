use music_player_backend::{cloud::{AuthSessionState, CloudSessionMetadata, SessionExpiredReason, SyncManager}, config::AppConfig, core::CoreProcessor, database::create_in_memory_pool, playback::backend::MockAudioBackend};

#[tokio::test]
#[ignore = "requires explicit disposable XDG data and an unavailable private credential bus"]
async fn expired_sessions_clear_local_identity_and_synthetic_token_offline() {
    assert_eq!(std::env::var("KAZE_PRIVATE_AUTH_TEST").as_deref(), Ok("1"));
    let data = std::env::var("XDG_DATA_HOME").unwrap();
    assert!(std::path::Path::new(&data).starts_with(std::env::temp_dir()));
    for absolute in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let pool = create_in_memory_pool().await.unwrap();
        sqlx::query("INSERT INTO application_settings(key,value,updated_at) VALUES('cloud_sync_url','http://127.0.0.1:9',1),('enable_jiosaavn','false',1)").execute(&pool).await.unwrap();
        let user = format!("audit-{}", uuid::Uuid::new_v4());
        let now = chrono::Utc::now().timestamp();
        SyncManager::save_session(&pool, &CloudSessionMetadata {
            user_id:user.clone(),username:"audit".into(),session_id:"synthetic".into(),device_id:"audit".into(),device_name:"audit".into(),
            idle_expires_at:if absolute {now+100} else {now-1},absolute_expires_at:if absolute {now-1} else {now+100},
            last_cloud_validation_at:None,worker_url:"http://127.0.0.1:9".into(),synced_at:None,authenticated_before:1,created_at:now-200,
        }).await.unwrap();
        let location = directories::ProjectDirs::from("org","intelligentmusicplayer","music-player").unwrap();
        std::fs::create_dir_all(location.data_dir()).unwrap();
        let token = location.data_dir().join(format!(".session_token_{user}"));
        std::fs::write(&token,b"synthetic-audit-token").unwrap();
        #[cfg(unix)] {use std::os::unix::fs::PermissionsExt;std::fs::set_permissions(&token,std::fs::Permissions::from_mode(0o600)).unwrap();}
        let mut config = AppConfig::default_with_dirs();config.cache_dir=dir.path().join("cache");
        let processor=CoreProcessor::new_with_backend(pool.clone(),config,Box::new(MockAudioBackend::new()));
        tokio::time::timeout(std::time::Duration::from_secs(10),async {
            loop {
                let state=processor.session_state.read().await.clone();
                if matches!(state,AuthSessionState::SessionExpired {..}) {break;}
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        }).await.expect("expired session validation did not finish");
        let expected=if absolute {SessionExpiredReason::AbsoluteTimeout} else {SessionExpiredReason::IdleTimeout};
        assert_eq!(*processor.session_state.read().await,AuthSessionState::SessionExpired {reason:expected});
        assert!(processor.current_user.read().await.is_none());
        assert!(SyncManager::get_active_session(&pool).await.unwrap().is_none());assert!(!token.exists());
        processor.shutdown().await.unwrap();
    }
}
