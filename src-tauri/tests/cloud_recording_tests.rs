use music_player_backend::{
    cloud::{SyncManager, SyncPayload},
    database::create_in_memory_pool,
    providers::{jiosaavn::JioSaavnProvider, online_catalog::OnlineCatalog, ExternalTrackMetadata},
    recommendations::mixes::ensure_online_track,
};
use serde_json::json;
use sqlx::SqlitePool;
use std::sync::Arc;

const IDS: [&str; 3] = [
    "itunes:old",
    "legacy:second",
    "jiosaavn:exact_album_version",
];

fn metadata() -> ExternalTrackMetadata {
    serde_json::from_value(json!({
        "provider":"jiosaavn", "provider_track_id":"exact_album_version",
        "title":"Song", "artist_name":"Artist", "album_title":"Selected release",
        "duration_secs":200.0, "cover_art_url":"https://example.invalid/selected.jpg",
        "year":2024, "language":"hindi", "track_number":3, "album_id":"42"
    }))
    .unwrap()
}

async fn seeded() -> SqlitePool {
    let pool = create_in_memory_pool().await.unwrap();
    for id in IDS {
        ensure_online_track(
            &pool,
            id,
            "Song",
            Some("Artist"),
            Some("Selected release"),
            Some(200.0),
            None,
            None,
        )
        .await
        .unwrap();
        sqlx::query("INSERT INTO online_track_sources(track_id,provider_id,metadata_json,linked_at) VALUES(?,?,?,123)")
            .bind(id).bind("exact_album_version").bind(serde_json::to_string(&metadata()).unwrap()).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO track_statistics(user_id,track_id,manual_like,play_count) VALUES('default',?,1,7)")
            .bind(id).execute(&pool).await.unwrap();
    }
    sqlx::query("INSERT INTO playlists(id,user_id,name,created_at,updated_at) VALUES('saved','default','Saved',1,1)").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO playlist_tracks(playlist_id,track_id,position,added_at) VALUES('saved','itunes:old',0,1)").execute(&pool).await.unwrap();
    pool
}

fn catalog(pool: &SqlitePool) -> Arc<OnlineCatalog> {
    // Binding restored snapshots must succeed without a usable network endpoint.
    OnlineCatalog::new(
        pool.clone(),
        Arc::new(JioSaavnProvider::with_endpoint("http://127.0.0.1:1")),
    )
}

#[tokio::test]
async fn converted_aliases_and_canonical_song_keep_exact_snapshot_after_cloud_roundtrip() {
    let pool = seeded().await;
    let payload = SyncManager::prepare_local_sync_payload(&pool, "default")
        .await
        .unwrap();
    let serialized = serde_json::to_value(&payload).unwrap();
    for song in serialized["songs"].as_array().unwrap() {
        assert_eq!(song["online_source"]["provider_id"], "exact_album_version");
        assert_eq!(
            song["online_source"]["metadata"],
            serde_json::to_value(metadata()).unwrap()
        );
        assert_eq!(song["online_source"]["linked_at"], 123);
    }
    let payload: SyncPayload = serde_json::from_value(serialized).unwrap();
    let restored = create_in_memory_pool().await.unwrap();
    for _ in 0..2 {
        SyncManager::apply_remote_sync_payload(&restored, "default", &payload)
            .await
            .unwrap();
    }
    let catalog = catalog(&restored);
    for id in IDS {
        assert_eq!(
            catalog.source_id(id).await.unwrap().as_deref(),
            Some("exact_album_version")
        );
        assert_eq!(
            serde_json::to_value(catalog.bind(id).await.unwrap().unwrap()).unwrap(),
            serde_json::to_value(metadata()).unwrap()
        );
        let pair: (String, String) =
            sqlx::query_as("SELECT provider,provider_id FROM external_tracks WHERE id=?")
                .bind(id)
                .fetch_one(&restored)
                .await
                .unwrap();
        assert_eq!(
            pair,
            if id.starts_with("jiosaavn:") {
                ("jiosaavn".into(), "exact_album_version".into())
            } else {
                ("online".into(), id.into())
            }
        );
    }
    let member: String =
        sqlx::query_scalar("SELECT track_id FROM playlist_tracks WHERE playlist_id='saved'")
            .fetch_one(&restored)
            .await
            .unwrap();
    assert_eq!(member, "itunes:old");
    let stats: i64 = sqlx::query_scalar("SELECT count(*) FROM track_statistics WHERE user_id='default' AND manual_like=1 AND play_count=7").fetch_one(&restored).await.unwrap();
    assert_eq!(stats, 3);
    let again = SyncManager::prepare_local_sync_payload(&restored, "default")
        .await
        .unwrap();
    let again = serde_json::to_value(again).unwrap();
    assert_eq!(again["songs"].as_array().unwrap().len(), 3);
    for song in again["songs"].as_array().unwrap() {
        assert_eq!(
            song["online_source"]["metadata"],
            serde_json::to_value(metadata()).unwrap()
        );
        assert_eq!(song["online_source"]["linked_at"], 123);
    }
}

#[tokio::test]
async fn older_payloads_and_conflicting_remote_bindings_do_not_erase_local_selection() {
    let pool = seeded().await;
    let payload = SyncManager::prepare_local_sync_payload(&pool, "default")
        .await
        .unwrap();
    let mut old = serde_json::to_value(&payload).unwrap();
    for song in old["songs"].as_array_mut().unwrap() {
        song.as_object_mut().unwrap().remove("online_source");
    }
    let old: SyncPayload = serde_json::from_value(old).unwrap();
    SyncManager::apply_remote_sync_payload(&pool, "default", &old)
        .await
        .unwrap();
    let fresh = create_in_memory_pool().await.unwrap();
    SyncManager::apply_remote_sync_payload(&fresh, "default", &old)
        .await
        .unwrap();
    assert_eq!(catalog(&fresh).source_id("itunes:old").await.unwrap(), None);
    assert_eq!(
        catalog(&fresh).source_id(IDS[2]).await.unwrap().as_deref(),
        Some("exact_album_version")
    );
    let mut different = serde_json::to_value(payload).unwrap();
    for song in different["songs"].as_array_mut().unwrap() {
        song["online_source"]["provider_id"] = json!("different_release");
        song["online_source"]["metadata"]["provider_track_id"] = json!("different_release");
    }
    SyncManager::apply_remote_sync_payload(
        &pool,
        "default",
        &serde_json::from_value(different).unwrap(),
    )
    .await
    .unwrap();
    for id in IDS {
        assert_eq!(
            catalog(&pool).source_id(id).await.unwrap().as_deref(),
            Some("exact_album_version")
        );
    }
}

#[tokio::test]
async fn source_restore_errors_are_reported_and_song_insert_is_rolled_back() {
    let pool = seeded().await;
    let payload = SyncManager::prepare_local_sync_payload(&pool, "default")
        .await
        .unwrap();
    let fresh = create_in_memory_pool().await.unwrap();
    sqlx::query("CREATE TRIGGER reject_source BEFORE INSERT ON online_track_sources BEGIN SELECT RAISE(ABORT,'source write failed'); END").execute(&fresh).await.unwrap();
    assert!(
        SyncManager::apply_remote_sync_payload(&fresh, "default", &payload)
            .await
            .is_err()
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM external_tracks")
        .fetch_one(&fresh)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn restore_does_not_rewrite_physical_local_tracks() {
    let pool = seeded().await;
    let payload = SyncManager::prepare_local_sync_payload(&pool, "default")
        .await
        .unwrap();
    sqlx::query("UPDATE tracks SET file_path='/music/original.flac',format='flac',title='Local tags' WHERE id='itunes:old'").execute(&pool).await.unwrap();
    SyncManager::apply_remote_sync_payload(&pool, "default", &payload)
        .await
        .unwrap();
    let row: (String, String, String) =
        sqlx::query_as("SELECT file_path,format,title FROM tracks WHERE id='itunes:old'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        row,
        (
            "/music/original.flac".into(),
            "flac".into(),
            "Local tags".into()
        )
    );
}

#[tokio::test]
async fn inconsistent_source_identity_is_rejected_before_writes() {
    let pool = seeded().await;
    let payload = SyncManager::prepare_local_sync_payload(&pool, "default")
        .await
        .unwrap();
    let mut value = serde_json::to_value(payload).unwrap();
    value["songs"][0]["online_source"]["provider_id"] = json!("another_recording");
    let payload: SyncPayload = serde_json::from_value(value).unwrap();
    let fresh = create_in_memory_pool().await.unwrap();
    assert!(
        SyncManager::apply_remote_sync_payload(&fresh, "default", &payload)
            .await
            .is_err()
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM external_tracks")
        .fetch_one(&fresh)
        .await
        .unwrap();
    assert_eq!(count, 0);
}
