use music_player_backend::{
    config::AppConfig,
    core::command::SmartMixType,
    core::{Command, CoreProcessor, Query, QueryResponse},
    database::{
        create_in_memory_pool,
        models::ExternalTrackRecord,
        repositories::{SqliteWishlistRepository, WishlistRepository},
    },
    playback::backend::MockAudioBackend,
    recommendations::{
        mixes::ensure_online_track, signals::SongSignals, LocalRecommender, SmartMixGenerator,
        TasteProfileEngine,
    },
};

async fn song(pool: &sqlx::SqlitePool, id: &str, title: &str, artist: &str, provider: &str) {
    ensure_online_track(
        pool,
        id,
        title,
        Some(artist),
        Some("Album"),
        Some(180.0),
        None,
        None,
    )
    .await
    .unwrap();
    sqlx::query("UPDATE external_tracks SET provider='jiosaavn',provider_id=?,language='malayalam',release_year=2024 WHERE id=?").bind(provider).bind(id).execute(pool).await.unwrap();
}
async fn alias(pool: &sqlx::SqlitePool, id: &str, provider: &str) {
    ensure_online_track(
        pool,
        id,
        "Song",
        Some("Artist"),
        Some("Album"),
        Some(180.0),
        None,
        None,
    )
    .await
    .unwrap();
    let metadata = serde_json::json!({"provider":"jiosaavn","provider_track_id":provider,"title":"Song","artist_name":"Artist","album_title":"Album","duration_secs":180.0,"language":"malayalam","year":2024,"cover_art_url":"https://example.test/exact-recording.jpg"});
    sqlx::query("INSERT INTO online_track_sources(track_id,provider_id,metadata_json,linked_at) VALUES(?,?,?,0)").bind(id).bind(provider).bind(metadata.to_string()).execute(pool).await.unwrap();
}
#[tokio::test]
async fn source_aliases_share_listening_and_language_year_affinity_without_guessing_genres() {
    let pool = create_in_memory_pool().await.unwrap();
    song(&pool, "jiosaavn:actual", "Song", "Artist", "actual").await;
    alias(&pool, "itunes:saved", "actual").await;
    song(&pool, "jiosaavn:single", "Song", "Artist", "single").await;
    sqlx::query("INSERT INTO track_statistics(user_id,track_id,play_count,completion_count,skip_count,manual_like) VALUES('default','itunes:saved',7,6,1,1)").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO playback_history(id,track_id,user_id,started_at,ended_at,seconds_listened,percentage_listened,completed,skipped,source) VALUES('listen','itunes:saved','default',?,0,180,1,1,0,'online')").bind(chrono::Utc::now().timestamp()).execute(&pool).await.unwrap();
    let index = SongSignals::load(&pool, "default").await.unwrap();
    for id in ["jiosaavn:actual", "jiosaavn:single", "itunes:saved"] {
        let signal = index.signal(id, "Song", "Artist");
        assert_eq!(signal.play_count, 7);
        assert_eq!(signal.manual_like, 1);
    }
    let profile = TasteProfileEngine::new(pool.clone())
        .compute_taste_profile("default")
        .await
        .unwrap();
    assert!(profile.language_affinities["malayalam"] > 0.0);
    assert!(profile.era_affinities["2020s"] > 0.0);
    assert!(
        profile.genre_affinities.is_empty(),
        "language is not an invented genre"
    );
    assert!(
        profile.artist_affinity("Second Artist, Artist") > 0.0,
        "primary artist credits must match collaborations"
    );
    assert!(
        SongSignals::load(&pool, "other")
            .await
            .unwrap()
            .signal("jiosaavn:actual", "Song", "Artist")
            .play_count
            == 0
    );
}

#[tokio::test]
async fn not_interested_blocks_aliases_future_releases_mixes_and_queue_but_preserves_manual_access()
{
    let pool = create_in_memory_pool().await.unwrap();
    let temp = tempfile::tempdir().unwrap();
    song(&pool, "jiosaavn:actual", "Song", "Artist", "actual").await;
    alias(&pool, "itunes:saved", "actual").await;
    song(&pool, "jiosaavn:single", "Song", "Artist", "single").await;
    song(&pool, "allowed", "Other Song", "Other Artist", "other").await;
    // Physical media stays on the local route; MockAudioBackend requires no file.
    sqlx::query(
        "UPDATE tracks SET format='flac',file_path='/fixture/allowed.flac' WHERE id='allowed'",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO playlists(id,name,user_id,is_smart_mix,created_at,updated_at) VALUES('manual','My saved playlist','default',0,0,0)").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO playlist_tracks(playlist_id,track_id,position,added_at) VALUES('manual','jiosaavn:actual',0,0)").execute(&pool).await.unwrap();
    let mut config = AppConfig::default_with_dirs();
    config.cache_dir = temp.path().join("cache");
    let core =
        CoreProcessor::new_with_backend(pool.clone(), config, Box::new(MockAudioBackend::new()));
    core.dispatch_command(Command::SetMetadataPreference {
        key: "enable_jiosaavn".into(),
        enabled: false,
    })
    .await
    .unwrap();
    core.dispatch_command(Command::SetMetadataPreference {
        key: "auto_fill_metadata".into(),
        enabled: false,
    })
    .await
    .unwrap();
    core.dispatch_command(Command::LikeTrack {
        track_id: "itunes:saved".into(),
    })
    .await
    .unwrap();
    let before = TasteProfileEngine::new(pool.clone())
        .compute_taste_profile("default")
        .await
        .unwrap()
        .artist_affinity("Artist");
    core.dispatch_command(Command::PlayTrack {
        track_id: "allowed".into(),
        source: Some("library".into()),
    })
    .await
    .unwrap();
    core.dispatch_command(Command::EnqueueTrack {
        track_id: "jiosaavn:single".into(),
        play_next: false,
    })
    .await
    .unwrap();
    core.dispatch_command(Command::DislikeTrack {
        track_id: "itunes:saved".into(),
    })
    .await
    .unwrap();
    let index = SongSignals::load(&pool, "default").await.unwrap();
    for id in ["itunes:saved", "jiosaavn:actual", "jiosaavn:single"] {
        assert!(index.blocked_id(id));
    }
    assert!(!index.blocked_id("allowed"));
    assert!(index.filter(serde_json::json!({
        "album_id":"saved-album", "album_title":"Saved album", "first_track_id":"jiosaavn:actual"
    })).is_none(), "Library Moment must not directly recommend a blocked recording");
    assert!(index.filter(serde_json::json!({
        "album_id":"other-album", "album_title":"Other album", "first_track_id":"allowed"
    })).is_some(), "eligible library albums remain available");
    assert!(!SongSignals::load(&pool, "other")
        .await
        .unwrap()
        .blocked_id("jiosaavn:actual"));
    assert!(!core
        .playback_service()
        .get_playback_state()
        .await
        .queue_track_ids
        .contains(&"jiosaavn:single".to_string()));
    let after = TasteProfileEngine::new(pool.clone())
        .compute_taste_profile("default")
        .await
        .unwrap()
        .artist_affinity("Artist");
    assert!(after < before);
    song(&pool, "jiosaavn:future", "Song", "Artist", "future").await;
    song(
        &pool,
        "jiosaavn:credit-variant",
        "Song",
        "Artist, Guest",
        "credit-variant",
    )
    .await;
    assert!(SongSignals::load(&pool, "default")
        .await
        .unwrap()
        .blocked_id("jiosaavn:credit-variant"));
    assert!(SongSignals::load(&pool, "default")
        .await
        .unwrap()
        .blocked_id("jiosaavn:future"));
    let picks = LocalRecommender::new(pool.clone())
        .recommend("default", 100)
        .await
        .unwrap();
    assert!(picks
        .iter()
        .all(|song| !index.blocked_id(&song.track_id) && song.track_id != "jiosaavn:future"));
    let mix = SmartMixGenerator::new(pool.clone())
        .generate_mix("default", &SmartMixType::Daily)
        .await
        .unwrap();
    let mix_ids: Vec<String> =
        sqlx::query_scalar("SELECT track_id FROM playlist_tracks WHERE playlist_id=?")
            .bind(&mix.id)
            .fetch_all(&pool)
            .await
            .unwrap();
    assert!(mix_ids
        .iter()
        .all(|id| !index.blocked_id(id) && id != "jiosaavn:future"));
    let QueryResponse::Track(Some(_)) = core
        .execute_query(Query::GetTrackById {
            track_id: "jiosaavn:actual".into(),
        })
        .await
        .unwrap()
    else {
        panic!("explicit song access must remain available")
    };
    let saved:i64=sqlx::query_scalar("SELECT COUNT(*) FROM playlist_tracks WHERE playlist_id='manual' AND track_id='jiosaavn:actual'").fetch_one(&pool).await.unwrap();
    assert_eq!(saved, 1);
    // A newly fetched result need not have been played/materialized to accept feedback.
    SqliteWishlistRepository::new(pool.clone())
        .upsert_external_track(&ExternalTrackRecord {
            id: "jiosaavn:new".into(),
            provider: "jiosaavn".into(),
            provider_id: "new".into(),
            title: "New Song".into(),
            artist: "New Artist".into(),
            album: None,
            duration_secs: Some(190.0),
            cover_art_url: None,
            preview_url: None,
            genre: None,
            match_status: "NOT_FOUND".into(),
            matched_local_track_id: None,
            created_at: 0,
        })
        .await
        .unwrap();
    core.dispatch_command(Command::DislikeTrack {
        track_id: "jiosaavn:new".into(),
    })
    .await
    .unwrap();
    assert!(SongSignals::load(&pool, "default")
        .await
        .unwrap()
        .blocked_id("jiosaavn:new"));
    let cloud =
        music_player_backend::cloud::SyncManager::prepare_local_sync_payload(&pool, "default")
            .await
            .unwrap();
    assert!(cloud
        .song_stats
        .iter()
        .any(|stat| stat.song_id == "itunes:saved" && stat.manual_like == -1));
    let restored = create_in_memory_pool().await.unwrap();
    music_player_backend::cloud::SyncManager::apply_remote_sync_payload(
        &restored, "default", &cloud,
    )
    .await
    .unwrap();
    song(
        &restored,
        "jiosaavn:later-on-another-device",
        "Song",
        "Artist",
        "later",
    )
    .await;
    assert!(SongSignals::load(&restored, "default")
        .await
        .unwrap()
        .blocked_id("jiosaavn:later-on-another-device"));
    let mut stale = cloud.clone();
    for stat in &mut stale.song_stats {
        stat.manual_like = 0;
    }
    music_player_backend::cloud::SyncManager::apply_remote_sync_payload(
        &restored, "default", &stale,
    )
    .await
    .unwrap();
    assert!(
        SongSignals::load(&restored, "default")
            .await
            .unwrap()
            .blocked_id("jiosaavn:later-on-another-device"),
        "stale neutral sync cannot erase an imported explicit choice"
    );
    // Fresh engines/restart do not discard an explicit exclusion.
    assert!(
        music_player_backend::recommendations::FlowEngine::new(pool.clone())
            .generate_playback_tracks_after("default", 0.0, 100, &[], None)
            .await
            .unwrap()
            .iter()
            .all(|song| !index.blocked_id(&song.track_id) && song.track_id != "jiosaavn:future")
    );
    core.dispatch_command(Command::RemoveTrackFeedback {
        track_id: "jiosaavn:actual".into(),
    })
    .await
    .unwrap();
    let artwork = core
        .execute_query(Query::GetTrackCoverArt {
            track_id: "itunes:saved".into(),
        })
        .await
        .unwrap();
    assert!(
        matches!(artwork,QueryResponse::CoverArt(Some(url)) if url=="https://example.test/exact-recording.jpg")
    );
    let stale_cover = temp.path().join("unrelated.png");
    image::RgbImage::new(1, 1).save(&stale_cover).unwrap();
    sqlx::query("INSERT INTO track_enrichment(track_id,provider,provider_id,cover_art_path,state,attempted_at) VALUES ('jiosaavn:actual','jiosaavn','different-recording',?,'matched',0)").bind(stale_cover.to_string_lossy().as_ref()).execute(&pool).await.unwrap();
    let missing = core
        .execute_query(Query::GetTrackCoverArt {
            track_id: "jiosaavn:actual".into(),
        })
        .await
        .unwrap();
    assert!(
        matches!(missing, QueryResponse::CoverArt(None)),
        "missing source artwork must not borrow a different recording's enrichment cover"
    );
    let status = core
        .execute_query(Query::GetRecommendationFeedback)
        .await
        .unwrap();
    if let QueryResponse::RecommendationFeedback(ids) = status {
        assert!(!ids.contains(&"itunes:saved".to_string()));
    } else {
        panic!("feedback state missing");
    }
    let unblocked = SongSignals::load(&pool, "default").await.unwrap();
    assert!(!unblocked.blocked_id("itunes:saved"));
    assert!(!unblocked.blocked_id("jiosaavn:future"));
    use music_player_backend::database::repositories::{SqliteUserRepository, UserRepository};
    SqliteUserRepository::new(pool.clone())
        .claim_guest_data_for_user("claimed-account")
        .await
        .unwrap();
    assert!(SongSignals::load(&pool, "claimed-account")
        .await
        .unwrap()
        .blocked_id("jiosaavn:new"));
    assert!(!SongSignals::load(&pool, "default")
        .await
        .unwrap()
        .blocked_id("jiosaavn:new"));
    core.shutdown().await.unwrap();
}
