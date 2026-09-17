use music_player_backend::{
    cloud::{CloudDailyStat, SyncManager, SyncPayload},
    config::{HistoryConfig, RankingWeightsConfig},
    core::{event::Event, event_bus::EventBus},
    database::{create_in_memory_pool, models::PlaybackHistoryRecord, repositories::{HistoryRepository, SqliteHistoryRepository, SqliteStatsRepository, StatsRepository, UserProfile}},
    history::service::HistoryService,
};
use std::sync::Arc;
use tokio::sync::RwLock;

fn entry(id: &str) -> PlaybackHistoryRecord {
    PlaybackHistoryRecord {
        id: id.into(), user_id: "A".into(), track_id: "song".into(),
        started_at: 1789689480, ended_at: 1789690080, seconds_listened: 600.,
        percentage_listened: 1., completed: 1, skipped: 0, source: "library".into(),
        playlist_id: None, recommendation_session_id: None,
    }
}
async fn pool() -> sqlx::SqlitePool {
    let p = create_in_memory_pool().await.unwrap();
    music_player_backend::recommendations::mixes::ensure_online_track(
        &p,"song","Song",Some("Artist"),None,Some(600.),None,None).await.unwrap();
    p
}
#[tokio::test]
async fn atomic_session_retry_and_midnight_start_day() {
    let p = pool().await;
    let repo = SqliteHistoryRepository::new(p.clone());
    let mut e = entry("session");
    e.started_at = chrono::DateTime::parse_from_rfc3339("2026-09-16T23:58:00+05:30").unwrap().timestamp();
    e.ended_at = e.started_at + 600;
    for _ in 0..2 { repo.record_session(&e, "2026-09-16", true).await.unwrap(); }
    let h: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM playback_history").fetch_one(&p).await.unwrap();
    assert_eq!(h, 1);
    let t: (f64,i64,i64) = sqlx::query_as("SELECT total_time_listened,play_count,completion_count FROM track_statistics WHERE user_id='A'").fetch_one(&p).await.unwrap();
    assert_eq!(t, (600.,1,1));
    let d: (String,f64) = sqlx::query_as("SELECT stat_date,listening_seconds FROM daily_user_stats").fetch_one(&p).await.unwrap();
    assert_eq!(d, ("2026-09-16".into(),600.));
}
#[tokio::test]
async fn failed_daily_write_rolls_back_history_and_lifetime() {
    let p = pool().await;
    sqlx::query("CREATE TRIGGER fail_daily BEFORE INSERT ON daily_user_stats BEGIN SELECT RAISE(ABORT, 'injected failure'); END").execute(&p).await.unwrap();
    let repo = SqliteHistoryRepository::new(p.clone());
    assert!(repo.record_session(&entry("failure"), "2026-09-17", true).await.is_err());
    for table in ["playback_history","track_statistics","daily_user_stats"] {
        let n: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table} WHERE user_id='A'")).fetch_one(&p).await.unwrap();
        assert_eq!(n,0,"{table}");
    }
    sqlx::query("DROP TRIGGER fail_daily").execute(&p).await.unwrap();
    repo.record_session(&entry("failure"), "2026-09-17", true).await.unwrap();
}
#[tokio::test]
async fn duplicate_finish_stop_and_account_switch_preserve_owner() {
    let p = pool().await;
    let bus = Arc::new(EventBus::default());
    let user = Arc::new(RwLock::new(Some(UserProfile::new("A".into(),"Alice".into(),0))));
    let _service = HistoryService::new(Arc::new(SqliteHistoryRepository::new(p.clone())), Arc::new(SqliteStatsRepository::new(p.clone())), HistoryConfig::default(), bus.clone(), Some(user.clone()));
    let sid1 = "session-1".to_string();
    bus.publish(Event::PlaybackStarted { session_id:sid1.clone(),track_id:"song".into(),title:"Song".into(),artist:"Artist".into(),duration_secs:600.,source:"library".into() }).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    *user.write().await = Some(UserProfile::new("B".into(),"Bob".into(),0));
    // First TrackFinished matches; second is a duplicate (session already taken — ignored).
    bus.publish(Event::TrackFinished {session_id:sid1.clone(),track_id:"song".into(),seconds_listened:600.,completed:true}).unwrap();
    bus.publish(Event::TrackFinished {session_id:sid1.clone(),track_id:"song".into(),seconds_listened:600.,completed:true}).unwrap();
    bus.publish(Event::PlaybackStopped { session_id: String::new() }).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    for table in ["playback_history","track_statistics","daily_user_stats"] {
        let owners: Vec<String> = sqlx::query_scalar(&format!("SELECT user_id FROM {table}")).fetch_all(&p).await.unwrap();
        assert_eq!(owners,vec!["A"],"{table}");
    }
    *user.write().await = None;
    let sid2 = "session-2".to_string();
    bus.publish(Event::PlaybackStarted {session_id:sid2.clone(),track_id:"song".into(),title:"Song".into(),artist:"Artist".into(),duration_secs:600.,source:"library".into()}).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    *user.write().await = Some(UserProfile::new("B".into(),"Bob".into(),0));
    bus.publish(Event::TrackFinished {session_id:sid2.clone(),track_id:"song".into(),seconds_listened:600.,completed:true}).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let guests: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM daily_user_stats WHERE user_id='default'").fetch_one(&p).await.unwrap();
    assert_eq!(guests,1);
    *user.write().await = None;
    let sid3 = "session-3".to_string();
    bus.publish(Event::PlaybackStarted {session_id:sid3.clone(),track_id:"song".into(),title:"Song".into(),artist:"Artist".into(),duration_secs:600.,source:"library".into()}).unwrap();
    bus.publish(Event::PlaybackPositionChanged {session_id:sid3.clone(),position_secs:5.,duration_secs:600.}).unwrap();
    bus.publish(Event::PlaybackStopped { session_id: sid3.clone() }).unwrap();
    bus.publish(Event::PlaybackStopped { session_id: String::new() }).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let skips: i64 = sqlx::query_scalar("SELECT skip_count FROM track_statistics WHERE user_id='default'").fetch_one(&p).await.unwrap();
    assert_eq!(skips,1);

}
fn snapshot(device: &str, seconds: f64, updated: i64) -> SyncPayload {
    SyncPayload { daily_stats: vec![CloudDailyStat {user_id:"A".into(),device_id:device.into(),stat_date:"2026-09-17".into(),listening_seconds:seconds,play_count:1,completion_count:1,skip_count:0,updated_at:updated}], ..Default::default() }
}
#[tokio::test]
async fn stale_equal_and_future_clocks_never_regress_daily_snapshots() {
    let p = pool().await;
    for payload in [snapshot("device-a",3600.,100),snapshot("device-a",4200.,50),snapshot("device-a",3600.,100),snapshot("device-a",10.,9999999999),snapshot("device-b",1000.,50),snapshot("device-b",1000.,50)] {
        SyncManager::apply_remote_sync_payload(&p,"A",&payload).await.unwrap();
    }
    let sum: f64 = sqlx::query_scalar("SELECT SUM(listening_seconds) FROM daily_user_stats").fetch_one(&p).await.unwrap();
    assert_eq!(sum,5200.);
}
#[tokio::test]
async fn offline_accumulation_restore_and_payload_privacy() {
    let p = pool().await;
    let repo = SqliteHistoryRepository::new(p.clone());
    for id in ["one","two","three"] { repo.record_session(&entry(id),"2026-09-17",true).await.unwrap(); }
    let payload = SyncManager::prepare_local_sync_payload(&p,"A").await.unwrap();
    assert_eq!(payload.daily_stats.len(),1);
    assert_eq!(payload.daily_stats[0].listening_seconds,1800.);
    let json = serde_json::to_value(&payload).unwrap();
    for key in ["playback_history","session_ids","playback_ticks","source"] { assert!(json.get(key).is_none()); }
    let fresh = create_in_memory_pool().await.unwrap();
    for _ in 0..2 { SyncManager::apply_remote_sync_payload(&fresh,"A",&payload).await.unwrap(); }
    let stats = SqliteStatsRepository::new(fresh.clone()).get_stats_overview("A",0,0,Some(2026),Some(9),&RankingWeightsConfig::default()).await.unwrap();
    assert_eq!(stats.lifetime_seconds,1800.);
    assert_eq!(stats.monthly_seconds,1800.);
    assert_eq!(stats.total_year_seconds,1800.);
    assert_eq!(stats.top_songs.len(),1);
    assert_eq!(stats.top_artists.len(),1);
    assert!(stats.history_started_at.is_some());
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM playback_history").fetch_one(&fresh).await.unwrap();
    assert_eq!(n,0);
    let new_device = music_player_backend::cloud::device::get_or_create_device_id(&fresh).await.unwrap();
    assert_ne!(new_device,payload.daily_stats[0].device_id);
}
#[tokio::test]
async fn old_year_uses_daily_rows_instead_of_stale_archive() {
    let p = pool().await;
    SqliteHistoryRepository::new(p.clone()).record_session(&entry("old"),"2025-12-31",true).await.unwrap();
    let repo = SqliteStatsRepository::new(p.clone());
    let first = repo.get_stats_overview("A",0,0,Some(2025),Some(12),&RankingWeightsConfig::default()).await.unwrap();
    assert_eq!(first.total_year_seconds,600.);
    repo.record_daily_playback("A",Some("other"),"2025-12-31",100.,true,false,false).await.unwrap();
    let next = repo.get_stats_overview("A",0,0,Some(2025),Some(12),&RankingWeightsConfig::default()).await.unwrap();
    assert_eq!(next.total_year_seconds,700.);
    assert_eq!(next.top_songs.len(),1);
}
#[tokio::test]
async fn concurrent_device_initialization_is_stable() {
    let p = pool().await;
    let (a,b) = tokio::join!(music_player_backend::cloud::device::get_or_create_device_id(&p),music_player_backend::cloud::device::get_or_create_device_id(&p));
    assert_eq!(a.unwrap(),b.unwrap());
}

#[tokio::test]
async fn seven_days_includes_today_and_six_prior_dates_only() {
    let p = pool().await;
    let repo = SqliteStatsRepository::new(p);
    let today = chrono::Local::now().date_naive();
    for offset in 0..8 {
        let day = (today - chrono::Duration::days(offset)).format("%Y-%m-%d").to_string();
        repo.record_daily_playback("A",Some("device"),&day,10.,false,false,false).await.unwrap();
    }
    let stats=repo.get_stats_overview("A",0,0,None,None,&RankingWeightsConfig::default()).await.unwrap();
    assert_eq!(stats.weekly_seconds,70.);
    assert_eq!(stats.daily_seconds,10.);
}

#[tokio::test]
async fn registration_does_not_claim_guest_history_or_aggregates() {
    use music_player_backend::database::repositories::{UserRepository, SqliteUserRepository};
    let p=pool().await;
    let mut e=entry("guest");e.user_id="default".into();
    SqliteHistoryRepository::new(p.clone()).record_session(&e,"2026-09-17",true).await.unwrap();
    SqliteUserRepository::new(p.clone()).create_user("new_user","password").await.unwrap();
    for table in ["playback_history","track_statistics","daily_user_stats"] {
        let owner: String=sqlx::query_scalar(&format!("SELECT user_id FROM {table}")).fetch_one(&p).await.unwrap();
        assert_eq!(owner,"default");
    }
}

#[tokio::test]
async fn fresh_install_restore_multi_device_lifetime() {
    use music_player_backend::cloud::CloudTrackDeviceStat;
    let p = pool().await;
    let payload = SyncPayload {
        track_device_stats: vec![
            CloudTrackDeviceStat {
                user_id: "A".into(),
                device_id: "_baseline".into(),
                track_id: "song".into(),
                play_count: 5,
                total_seconds: 100.0,
                completion_count: 5,
                skip_count: 0,
                last_played_at: Some(1700000000),
                updated_at: 1700000000,
            },
            CloudTrackDeviceStat {
                user_id: "A".into(),
                device_id: "device_A".into(),
                track_id: "song".into(),
                play_count: 1,
                total_seconds: 30.0,
                completion_count: 1,
                skip_count: 0,
                last_played_at: Some(1700000010),
                updated_at: 1700000010,
            },
            CloudTrackDeviceStat {
                user_id: "A".into(),
                device_id: "device_B".into(),
                track_id: "song".into(),
                play_count: 2,
                total_seconds: 40.0,
                completion_count: 2,
                skip_count: 0,
                last_played_at: Some(1700000020),
                updated_at: 1700000020,
            },
        ],
        ..Default::default()
    };

    SyncManager::apply_remote_sync_payload(&p, "A", &payload).await.unwrap();

    let lifetime_from_device_stats: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_time_listened), 0.0) FROM track_device_statistics WHERE user_id = 'A'"
    ).fetch_one(&p).await.unwrap();
    assert_eq!(lifetime_from_device_stats, 170.0, "track_device_statistics sum must be 170 (100 + 30 + 40)");

    let lifetime_from_track_stats: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_time_listened), 0.0) FROM track_statistics WHERE user_id = 'A'"
    ).fetch_one(&p).await.unwrap();
    assert_eq!(lifetime_from_track_stats, 170.0, "track_statistics synchronized lifetime must be 170");

    let repo = SqliteStatsRepository::new(p.clone());
    let stats = repo.get_stats_overview("A", 0, 0, None, None, &RankingWeightsConfig::default()).await.unwrap();
    assert_eq!(stats.lifetime_seconds, 170.0, "Stats overview lifetime_seconds must be 170");

    // Pulling again must not duplicate baseline or inflate total
    SyncManager::apply_remote_sync_payload(&p, "A", &payload).await.unwrap();
    let stats2 = repo.get_stats_overview("A", 0, 0, None, None, &RankingWeightsConfig::default()).await.unwrap();
    assert_eq!(stats2.lifetime_seconds, 170.0, "Repeated pull must not duplicate baseline or inflate lifetime");
}

#[tokio::test]
#[ignore = "read-only local production audit; requires this workstation's database"]
async fn read_only_live_stats_overview() {
    let options=sqlx::sqlite::SqliteConnectOptions::new()
        .filename("/home/abhi/.local/share/music-player/music_player.db").read_only(true);
    let p=sqlx::sqlite::SqlitePoolOptions::new().max_connections(1).connect_with(options).await.unwrap();
    let stats=SqliteStatsRepository::new(p).get_stats_overview(
        "103cc229-6f41-4da2-abb9-beba57ef0367",0,0,Some(2026),Some(9),&RankingWeightsConfig::default()).await.unwrap();
    println!("{}",serde_json::to_string_pretty(&stats).unwrap());
}
