use music_player_backend::{
    config::AppConfig,
    core::{command::Command, processor::CoreProcessor},
    database::{create_in_memory_pool, repositories::SqliteWishlistRepository},
    discovery::DiscoveryCoordinator,
    playback::backend::MockAudioBackend,
    providers::{ExternalTrackMetadata, ProviderCoordinator},
    recommendations::{
        mixes::{ensure_online_track, OnlineTrackInput},
        setup,
    },
};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn selected_languages_seed_recommendations_and_qualified_plays_broaden_them() {
    let temp = tempfile::tempdir().unwrap();
    let core = fixture(temp.path()).await;
    let pool = core.db_pool();
    let coordinator = DiscoveryCoordinator::new(
        Arc::new(SqliteWishlistRepository::new(pool.clone())),
        core.library_service().track_repo(),
        core.taste_engine(),
        None,
    );
    let provider = ProviderCoordinator::new(pool.clone(), core.event_bus());
    let hindi = add_song(&provider, "hindi-starter", Some("hindi")).await;
    let malayalam = add_song(&provider, "malayalam-starter", Some("Malayalam")).await;
    let english = add_song(&provider, "english-starter", Some("english")).await;
    let english_example = add_song(&provider, "english-example", Some("english")).await;
    add_song(&provider, "unknown-language", None).await;
    save_taste(&core, &[], &["Hindi", "Malayalam"]).await;

    let initial = suggested_ids(&coordinator).await;
    assert_eq!(
        initial,
        [hindi.clone(), malayalam.clone()].into_iter().collect()
    );
    let flow = core
        .flow_engine()
        .generate_playback_tracks_after("default", 0.5, 20, &[], &[])
        .await
        .unwrap();
    assert_eq!(
        flow.into_iter()
            .map(|track| track.track_id)
            .collect::<std::collections::HashSet<_>>(),
        initial
    );

    record_listen(&core, &english, 5.0).await;
    assert_eq!(suggested_ids(&coordinator).await, initial);

    record_listen(&core, &english, 160.0).await;
    let broadened = suggested_ids(&coordinator).await;
    assert!(initial.is_subset(&broadened));
    assert!(broadened.contains(&english_example));
}

#[tokio::test]
async fn all_genres_round_trip_and_legacy_saves_preserve_language_choices() {
    let temp = tempfile::tempdir().unwrap();
    let core = fixture(temp.path()).await;
    let genres = [
        "Pop",
        "Rock",
        "Hip-Hop",
        "R&B",
        "Electronic",
        "Indie",
        "Jazz",
        "Classical",
        "Country",
        "Folk",
        "Metal",
        "Devotional",
    ];
    save_taste(&core, &genres, &[" Hindi ", "hindi", "Malayalam"]).await;
    let saved = setup::load(core.db_pool(), "default").await.unwrap();
    assert_eq!(saved.genres, genres);
    assert_eq!(saved.languages, ["Hindi", "Malayalam"]);
    assert!(setup::load(core.db_pool(), "other")
        .await
        .unwrap()
        .languages
        .is_empty());

    setup::save(
        core.db_pool(),
        "default",
        vec![],
        vec!["Jazz".into()],
        false,
    )
    .await
    .unwrap();
    assert_eq!(
        setup::load(core.db_pool(), "default")
            .await
            .unwrap()
            .languages,
        saved.languages
    );
    save_taste(&core, &["Jazz"], &[]).await;
    assert!(setup::load(core.db_pool(), "default")
        .await
        .unwrap()
        .languages
        .is_empty());
}

async fn fixture(path: &std::path::Path) -> CoreProcessor {
    let mut config = AppConfig::default_with_dirs();
    config.cache_dir = path.join("cache");
    config.database_path = path.join("unused.sqlite");
    config.downloads.download_dir = Some(path.join("downloads"));
    CoreProcessor::new_with_backend(
        create_in_memory_pool().await.unwrap(),
        config,
        Box::new(MockAudioBackend::new()),
    )
}

async fn save_taste(core: &CoreProcessor, genres: &[&str], languages: &[&str]) {
    core.dispatch_command(serde_json::from_value::<Command>(json!({
        "command": "SaveRecommendationSetup",
        "payload": {"expected_user_id": "default", "artists": [], "genres": genres, "languages": languages, "skipped": false}
    })).unwrap()).await.unwrap();
}

async fn add_song(provider: &ProviderCoordinator, id: &str, language: Option<&str>) -> String {
    provider
        .persist(ExternalTrackMetadata {
            provider: "jiosaavn".into(),
            provider_track_id: id.into(),
            title: id.into(),
            artist_name: format!("Artist {id}"),
            artist_id: None,
            album_title: None,
            album_id: None,
            duration_secs: Some(180.0),
            year: None,
            track_number: None,
            cover_art_url: None,
            language: language.map(str::to_owned),
        })
        .await
        .unwrap()
        .id
}

async fn suggested_ids(coordinator: &DiscoveryCoordinator) -> std::collections::HashSet<String> {
    coordinator
        .get_discovery_recommendations("default", 20, false)
        .await
        .unwrap()
        .into_iter()
        .map(|track| track.external_track_id)
        .collect()
}

async fn record_listen(core: &CoreProcessor, id: &str, seconds: f64) {
    let pool = core.db_pool();
    ensure_online_track(
        pool,
        &OnlineTrackInput {
            id,
            title: "english-starter",
            artist: Some("Artist english-starter"),
            album: None,
            duration_secs: Some(180.0),
            cover_art_url: None,
            preview_url: None,
        },
    )
    .await
    .unwrap();
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO playback_history(id,track_id,user_id,started_at,ended_at,seconds_listened,percentage_listened,completed,skipped,source)
         VALUES(?,?,'default',?,?,?,?,0,?,'online')",
    ).bind(format!("listen-{seconds}")).bind(id).bind(now - seconds as i64).bind(now)
        .bind(seconds).bind(seconds / 180.0 * 100.0).bind(seconds < 10.0)
        .execute(pool).await.unwrap();
}
