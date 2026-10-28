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

// Two independent devices can legitimately save the same provider pair under
// different stable IDs. Export each device independently to retain that state.
async fn recording_device(id: &str, with_snapshot: bool) -> (SqlitePool, SyncPayload) {
    let pool = create_in_memory_pool().await.unwrap();
    ensure_online_track(
        &pool,
        id,
        "Song",
        Some("Artist"),
        Some("Selected release"),
        Some(200.0),
        Some("https://example.invalid/selected.jpg"),
        None,
    )
    .await
    .unwrap();
    sqlx::query("UPDATE external_tracks SET provider='jiosaavn',provider_id='exact_album_version' WHERE id=?")
        .bind(id).execute(&pool).await.unwrap();
    if with_snapshot {
        sqlx::query("INSERT INTO online_track_sources(track_id,provider_id,metadata_json,linked_at) VALUES(?,'exact_album_version',?,123)")
            .bind(id).bind(serde_json::to_string(&metadata()).unwrap()).execute(&pool).await.unwrap();
    }
    sqlx::query("INSERT INTO track_statistics(user_id,track_id,manual_like,play_count) VALUES('default',?,1,7)")
        .bind(id).execute(&pool).await.unwrap();
    let playlist = format!("saved:{id}");
    sqlx::query(
        "INSERT INTO playlists(id,user_id,name,created_at,updated_at) VALUES(?,'default',?,1,1)",
    )
    .bind(&playlist)
    .bind(&playlist)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO playlist_tracks(playlist_id,track_id,position,added_at) VALUES(?,?,0,1)",
    )
    .bind(playlist)
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO playback_history(id,track_id,started_at,ended_at,seconds_listened,percentage_listened,completed,skipped,source) VALUES(?,?,1,2,1,0.5,0,0,'library')")
        .bind(format!("history:{id}")).bind(id).execute(&pool).await.unwrap();
    let payload = SyncManager::prepare_local_sync_payload(&pool, "default")
        .await
        .unwrap();
    (pool, payload)
}

async fn assert_recording_aliases(pool: &SqlitePool, ids: &[&str]) {
    let catalog = catalog(pool);
    for id in ids {
        let expected = if *id == IDS[2] {
            ("jiosaavn".to_string(), "exact_album_version".to_string())
        } else {
            ("online".to_string(), id.to_string())
        };
        let pair: (String, String) =
            sqlx::query_as("SELECT provider,provider_id FROM external_tracks WHERE id=?")
                .bind(id)
                .fetch_one(pool)
                .await
                .unwrap();
        assert_eq!(pair, expected, "alias representation for {id}");
        assert_eq!(
            catalog.source_id(id).await.unwrap().as_deref(),
            Some("exact_album_version")
        );
        let bound = catalog.bind(id).await.unwrap().unwrap();
        assert_eq!(bound.provider_track_id, "exact_album_version");
        assert_eq!(bound.album_title.as_deref(), Some("Selected release"));
        assert_eq!(bound.duration_secs, Some(200.0));
        let member: String =
            sqlx::query_scalar("SELECT track_id FROM playlist_tracks WHERE playlist_id=?")
                .bind(format!("saved:{id}"))
                .fetch_one(pool)
                .await
                .unwrap();
        assert_eq!(&member, id);
        let stats: (i64, i64) = sqlx::query_as("SELECT manual_like,play_count FROM track_statistics WHERE user_id='default' AND track_id=?")
            .bind(id).fetch_one(pool).await.unwrap();
        assert_eq!(stats, (1, 7));
        ensure_online_track(
            pool,
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
    }
}

#[tokio::test]
async fn legacy_jiosaavn_identity_merges_with_canonical_on_either_device() {
    for with_snapshot in [false, true] {
        for legacy_first in [false, true] {
            let (legacy, legacy_payload) = recording_device("legacy:jio-song", with_snapshot).await;
            let (canonical, canonical_payload) = recording_device(IDS[2], with_snapshot).await;
            let (destination, incoming, existing) = if legacy_first {
                (&legacy, &canonical_payload, &legacy_payload)
            } else {
                (&canonical, &legacy_payload, &canonical_payload)
            };
            for _ in 0..2 {
                SyncManager::apply_remote_sync_payload(destination, "default", incoming)
                    .await
                    .unwrap();
                // A server can continue returning the original provider pair.
                SyncManager::apply_remote_sync_payload(destination, "default", existing)
                    .await
                    .unwrap();
                assert_recording_aliases(destination, &["legacy:jio-song", IDS[2]]).await;
            }
            let existing_id = if legacy_first {
                "legacy:jio-song"
            } else {
                IDS[2]
            };
            let history: String =
                sqlx::query_scalar("SELECT track_id FROM playback_history WHERE id=?")
                    .bind(format!("history:{existing_id}"))
                    .fetch_one(destination)
                    .await
                    .unwrap();
            assert_eq!(history, existing_id);
            let reexport = SyncManager::prepare_local_sync_payload(destination, "default")
                .await
                .unwrap();
            let reexport: SyncPayload =
                serde_json::from_str(&serde_json::to_string(&reexport).unwrap()).unwrap();
            for song in &reexport.songs {
                assert!(serde_json::to_value(song).unwrap()["online_source"].is_object());
            }
            let fresh = create_in_memory_pool().await.unwrap();
            SyncManager::apply_remote_sync_payload(&fresh, "default", &reexport)
                .await
                .unwrap();
            assert_recording_aliases(&fresh, &["legacy:jio-song", IDS[2]]).await;
        }
    }
}

#[tokio::test]
async fn multiple_legacy_provider_rows_restore_in_either_payload_order() {
    for with_snapshot in [false, true] {
        let (_, mut payload) = recording_device("legacy:first", with_snapshot).await;
        for id in ["legacy:second", IDS[2]] {
            let (_, other) = recording_device(id, with_snapshot).await;
            payload.songs.extend(other.songs);
            payload.playlists.extend(other.playlists);
            payload.playlist_songs.extend(other.playlist_songs);
            payload.song_stats.extend(other.song_stats);
        }
        for _ in 0..2 {
            payload.songs.reverse();
            let pool = create_in_memory_pool().await.unwrap();
            for _ in 0..2 {
                SyncManager::apply_remote_sync_payload(&pool, "default", &payload)
                    .await
                    .unwrap();
                assert_recording_aliases(&pool, &["legacy:first", "legacy:second", IDS[2]]).await;
            }
        }
    }
}

#[tokio::test]
async fn failed_canonical_insert_rolls_back_existing_alias_conversion_and_source() {
    for with_snapshot in [false, true] {
        let (pool, _) = recording_device("legacy:jio-song", with_snapshot).await;
        let (_, payload) = recording_device(IDS[2], true).await;
        sqlx::query("CREATE TRIGGER reject_canonical BEFORE INSERT ON external_tracks WHEN NEW.id='jiosaavn:exact_album_version' BEGIN SELECT RAISE(ABORT,'canonical insert failed'); END")
            .execute(&pool).await.unwrap();
        let error = SyncManager::apply_remote_sync_payload(&pool, "default", &payload)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("canonical insert failed"));
        let pair: (String, String) = sqlx::query_as(
            "SELECT provider,provider_id FROM external_tracks WHERE id='legacy:jio-song'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(pair, ("jiosaavn".into(), "exact_album_version".into()));
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM online_track_sources")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, i64::from(with_snapshot));
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM external_tracks")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }
}

#[tokio::test]
async fn old_canonical_cloud_labels_reconcile_legacy_pair_before_registration() {
    for provider in [None, Some("cloud"), Some("online")] {
        let (pool, _) = recording_device("legacy:jio-song", false).await;
        let (_, mut payload) = recording_device(IDS[2], false).await;
        payload.songs[0].provider = provider.map(str::to_string);
        payload.songs[0].provider_id = Some(IDS[2].into());
        for _ in 0..2 {
            SyncManager::apply_remote_sync_payload(&pool, "default", &payload)
                .await
                .unwrap();
            assert_recording_aliases(&pool, &["legacy:jio-song", IDS[2]]).await;
        }
    }
}

#[tokio::test]
async fn alias_conversion_preserves_selected_snapshot_and_imports_full_matching_snapshot() {
    // No local snapshot: use the complete incoming snapshot for the same explicit
    // source rather than saving a reduced fallback first.
    let (pool, _) = recording_device("legacy:jio-song", false).await;
    let (_, incoming) = recording_device("legacy:jio-song", true).await;
    SyncManager::apply_remote_sync_payload(&pool, "default", &incoming)
        .await
        .unwrap();
    let row: (String, String, i64) = sqlx::query_as("SELECT provider_id,metadata_json,linked_at FROM online_track_sources WHERE track_id='legacy:jio-song'").fetch_one(&pool).await.unwrap();
    assert_eq!(row.0, "exact_album_version");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&row.1).unwrap(),
        serde_json::to_value(metadata()).unwrap()
    );
    assert_eq!(row.2, 123);

    // An already saved selection remains authoritative even when its external
    // legacy pair names a different source and the incoming payload disagrees.
    let (pool, _) = recording_device("legacy:jio-song", true).await;
    let mut selected = metadata();
    selected.provider_track_id = "chosen_other_release".into();
    selected.album_title = Some("Previously chosen release".into());
    let selected_json = serde_json::to_string(&selected).unwrap();
    sqlx::query("UPDATE online_track_sources SET provider_id='chosen_other_release',metadata_json=? WHERE track_id='legacy:jio-song'")
        .bind(&selected_json).execute(&pool).await.unwrap();
    let (_, incoming) = recording_device(IDS[2], true).await;
    for _ in 0..2 {
        SyncManager::apply_remote_sync_payload(&pool, "default", &incoming)
            .await
            .unwrap();
    }
    let row: (String, String, i64) = sqlx::query_as("SELECT provider_id,metadata_json,linked_at FROM online_track_sources WHERE track_id='legacy:jio-song'").fetch_one(&pool).await.unwrap();
    assert_eq!(row, ("chosen_other_release".into(), selected_json, 123));
    let bound = catalog(&pool)
        .bind("legacy:jio-song")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bound.provider_track_id, "chosen_other_release");
    assert_eq!(
        bound.album_title.as_deref(),
        Some("Previously chosen release")
    );
}

#[tokio::test]
async fn reconciliation_write_failures_keep_the_original_identity_and_history() {
    for trigger in [
        "CREATE TRIGGER fail_source BEFORE INSERT ON online_track_sources BEGIN SELECT RAISE(ABORT,'reconciliation failed'); END",
        "CREATE TRIGGER fail_alias BEFORE UPDATE OF provider ON external_tracks WHEN OLD.id='legacy:jio-song' BEGIN SELECT RAISE(ABORT,'reconciliation failed'); END",
    ] {
        let (pool, _) = recording_device("legacy:jio-song", false).await;
        let (_, payload) = recording_device(IDS[2], true).await;
        sqlx::query(trigger).execute(&pool).await.unwrap();
        let error = SyncManager::apply_remote_sync_payload(&pool, "default", &payload).await.unwrap_err();
        assert!(error.to_string().contains("reconciliation failed"));
        let pair: (String, String) = sqlx::query_as("SELECT provider,provider_id FROM external_tracks WHERE id='legacy:jio-song'").fetch_one(&pool).await.unwrap();
        assert_eq!(pair, ("jiosaavn".into(), "exact_album_version".into()));
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM online_track_sources").fetch_one(&pool).await.unwrap();
        assert_eq!(count, 0, "source creation must roll back with alias conversion");
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM external_tracks").fetch_one(&pool).await.unwrap();
        assert_eq!(count, 1);
        let history: String = sqlx::query_scalar("SELECT track_id FROM playback_history WHERE id='history:legacy:jio-song'").fetch_one(&pool).await.unwrap();
        assert_eq!(history, "legacy:jio-song");
    }
}

#[tokio::test]
async fn reconstructed_legacy_source_keeps_local_metadata_and_uses_reconciliation_time() {
    let (pool, _) = recording_device("legacy:jio-song", false).await;
    sqlx::query(
        "UPDATE external_tracks SET language='hindi',release_year=2024 WHERE id='legacy:jio-song'",
    )
    .execute(&pool)
    .await
    .unwrap();
    let (_, payload) = recording_device(IDS[2], true).await;
    let before = chrono::Utc::now().timestamp();
    SyncManager::apply_remote_sync_payload(&pool, "default", &payload)
        .await
        .unwrap();
    let after = chrono::Utc::now().timestamp();
    let (snapshot, linked_at): (String, i64) = sqlx::query_as(
        "SELECT metadata_json,linked_at FROM online_track_sources WHERE track_id='legacy:jio-song'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let snapshot: ExternalTrackMetadata = serde_json::from_str(&snapshot).unwrap();
    assert_eq!(snapshot.provider_track_id, "exact_album_version");
    assert_eq!(snapshot.language.as_deref(), Some("hindi"));
    assert_eq!(snapshot.year, Some(2024));
    assert_eq!(snapshot.album_title.as_deref(), Some("Selected release"));
    assert_eq!(
        snapshot.cover_art_url.as_deref(),
        Some("https://example.invalid/selected.jpg")
    );
    assert!((before..=after).contains(&linked_at));
}
