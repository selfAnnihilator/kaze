use music_player_backend::{
    config::AppConfig,
    core::{command::Command, event::Event, processor::CoreProcessor},
    database::{
        create_in_memory_pool,
        models::ExternalTrackRecord,
        repositories::{ScannedMetadata, SqliteWishlistRepository, WishlistRepository},
    },
    playback::backend::MockAudioBackend,
    recommendations::mixes::ensure_online_track,
};
use sha2::{Digest, Sha256};

async fn setup() -> (CoreProcessor, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let mut config = AppConfig::default_with_dirs();
    config.cache_dir = dir.path().join("cache");
    let processor = CoreProcessor::new_with_backend(
        create_in_memory_pool().await.unwrap(),
        config,
        Box::new(MockAudioBackend::new()),
    );
    (processor, dir)
}

async fn local(p: &CoreProcessor, title: &str, artist: &str) -> String {
    p.library_service()
        .track_repo()
        .save_scanned_track(ScannedMetadata {
            file_path: format!("/fixture/{artist}-{title}.flac"),
            file_size: 1,
            modified_timestamp: 0,
            file_hash: None,
            title: title.into(),
            artist: Some(artist.into()),
            album: None,
            album_artist: None,
            genre: Some("Rock".into()),
            track_number: None,
            disc_number: None,
            year: Some(2020),
            duration_secs: 240.0,
            bitrate: None,
            sample_rate: None,
            format: "flac".into(),
            has_cover_art: false,
            musicbrainz_track_id: None,
        })
        .await
        .unwrap()
}

async fn stats(p: &CoreProcessor, user: &str, id: &str, plays: i64, like: i64, days: i64) {
    sqlx::query("INSERT INTO track_statistics (user_id, track_id, play_count, completion_count, manual_like, last_played_at)
        VALUES (?, ?, ?, ?, ?, ?)")
        .bind(user).bind(id).bind(plays).bind(plays).bind(like)
        .bind(chrono::Utc::now().timestamp() - days * 86400)
        .execute(p.db_pool()).await.unwrap();
}

async fn external(p: &CoreProcessor, id: &str, title: &str, artist: &str, genre: &str) {
    SqliteWishlistRepository::new(p.db_pool().clone())
        .upsert_external_track(&ExternalTrackRecord {
            id: id.into(),
            provider: "itunes".into(),
            provider_id: id.into(),
            title: title.into(),
            artist: artist.into(),
            album: None,
            genre: Some(genre.into()),
            duration_secs: Some(240.0),
            cover_art_url: None,
            preview_url: None,
            match_status: "NOT_FOUND".into(),
            matched_local_track_id: None,
            created_at: chrono::Utc::now().timestamp(),
        })
        .await
        .unwrap();
}

async fn cache_song(dir: &tempfile::TempDir, id: &str, title: &str, artist: &str) {
    let folder = dir.path().join("cache/remote-audio");
    tokio::fs::create_dir_all(&folder).await.unwrap();
    let hash = format!(
        "{:x}",
        Sha256::digest(
            format!(
                "full:v2:{id}:{}:{}",
                artist.to_lowercase(),
                title.to_lowercase()
            )
            .as_bytes()
        )
    );
    let samples = vec![0_u8; 16000];
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36_u32 + samples.len() as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&8000_u32.to_le_bytes());
    bytes.extend_from_slice(&16000_u32.to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(samples.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&samples);
    tokio::fs::write(folder.join(format!("{hash}.audio")), bytes)
        .await
        .unwrap();
}

#[tokio::test]
async fn runtime_flow_includes_taste_matched_online_songs_with_account_scoped_familiarity() {
    let (p, _dir) = setup().await;
    let favorite = local(&p, "Favorite", "Favorite Artist").await;
    stats(&p, "default", &favorite, 20, 1, 10).await;
    external(&p, "itunes:fresh", "Fresh Song", "Favorite Artist", "Rock").await;
    external(&p, "itunes:genre", "Genre Discovery", "New Artist", "Rock").await;
    external(
        &p,
        "itunes:unrelated",
        "Unrelated",
        "Unrelated Artist",
        "Noise",
    )
    .await;
    ensure_online_track(
        p.db_pool(),
        "online:other",
        "Other account song",
        Some("Other account artist"),
        None,
        Some(240.0),
        None,
        None,
    )
    .await
    .unwrap();
    stats(&p, "other-account", "online:other", 99, 1, 0).await;
    ensure_online_track(
        p.db_pool(),
        "online:skipped",
        "Skipped online",
        Some("Favorite Artist"),
        None,
        Some(240.0),
        None,
        None,
    )
    .await
    .unwrap();
    stats(&p, "default", "online:skipped", 0, 0, 0).await;
    sqlx::query("UPDATE track_statistics SET skip_count=5 WHERE track_id='online:skipped'")
        .execute(p.db_pool())
        .await
        .unwrap();
    let engine = p.flow_engine();
    let preview = engine
        .generate_flow_tracks("default", 0.0, 10, &[])
        .await
        .unwrap();
    assert!(
        preview.iter().all(|t| t.track_id == favorite),
        "local read-only preview must not register catalog tracks"
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tracks WHERE id='itunes:fresh'")
        .fetch_one(p.db_pool())
        .await
        .unwrap();
    assert_eq!(count, 0);
    let familiar = engine
        .generate_playback_tracks_after("default", 1.0, 10, &[], None)
        .await
        .unwrap();
    let discover = engine
        .generate_playback_tracks_after("default", 0.0, 10, &[], None)
        .await
        .unwrap();
    assert_eq!(
        familiar[0].track_id, favorite,
        "Familiar must prefer played/liked songs over unheard favorite-artist songs"
    );
    assert!(discover[0].track_id.starts_with("itunes:"));
    assert!(discover.iter().any(|t| t.track_id == "itunes:fresh"));
    assert!(discover.iter().any(|t| t.track_id == "itunes:genre"));
    assert!(discover
        .iter()
        .all(|t| t.track_id != "itunes:unrelated" && t.track_id != "online:other"));
    assert!(
        discover.iter().all(|t| t.track_id != "online:skipped"),
        "online slots cannot force skipped songs"
    );
    let fresh = discover
        .iter()
        .find(|t| t.track_id == "itunes:fresh")
        .unwrap();
    assert_eq!(fresh.reason_category, "online_discovery");
    assert!(fresh.explanation.is_unplayed);
    assert!(!fresh.reason.contains("library"));
    let history: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM playback_history")
        .fetch_one(p.db_pool())
        .await
        .unwrap();
    assert_eq!(history, 0, "preparing a Flow is not a listen");
    external(&p, "itunes:alias", "Fresh Song", "Favorite Artist", "Rock").await;
    let duplicates = engine
        .generate_playback_tracks_after("default", 0.0, 10, &[], None)
        .await
        .unwrap();
    assert_eq!(
        duplicates
            .iter()
            .filter(|t| t.title == "Fresh Song")
            .count(),
        1,
        "catalog aliases cannot duplicate a song"
    );
    let excluded = engine
        .generate_playback_tracks_after("default", 0.0, 10, &["itunes:fresh".into()], None)
        .await
        .unwrap();
    assert!(
        excluded.iter().all(|t| t.title != "Fresh Song"),
        "excluding a queued song must also exclude its provider alias"
    );
    p.shutdown().await.unwrap();
}

#[tokio::test]
async fn steering_rebuilds_generated_upcoming_tracks_without_restarting_current_or_removing_manual_entries(
) {
    let (p, _dir) = setup().await;
    let current = local(&p, "Current", "Current Artist").await;
    let favorite = local(&p, "Favorite", "Favorite Artist").await;
    let forgotten = local(&p, "Forgotten", "Forgotten Artist").await;
    stats(&p, "default", &favorite, 20, 1, 10).await;
    stats(&p, "default", &forgotten, 3, 0, 100).await;
    p.dispatch_command(Command::PlayTrack {
        track_id: current.clone(),
        source: Some("library".into()),
    })
    .await
    .unwrap();
    p.dispatch_command(Command::StartFlow {
        familiarity_ratio: Some(1.0),
    })
    .await
    .unwrap();
    let playback = p.playback_service();
    let initial = playback.get_playback_state().await;
    assert_eq!(initial.queue_track_ids[0], favorite);
    let manual = local(&p, "Manual", "Manual Artist").await;
    playback.enqueue_track(&manual, true).await.unwrap();
    let manual_tail = local(&p, "Manual tail", "Manual tail artist").await;
    playback.enqueue_track(&manual_tail, false).await.unwrap();
    p.dispatch_command(Command::Seek {
        position_secs: 25.0,
    })
    .await
    .unwrap();
    let before = playback.get_playback_state().await;
    let original_evidence =
        serde_json::to_value(playback.flow_reason("default", &current).await).unwrap();
    let generation = playback
        .flow_session(&p.flow_engine(), "default")
        .await
        .unwrap()
        .unwrap()
        .generation;
    let mut events = p.event_bus().subscribe();
    p.dispatch_command(Command::SetFlowSteering {
        familiarity_ratio: 0.0,
    })
    .await
    .unwrap();
    let after = playback.get_playback_state().await;
    assert_eq!(after.current_track_id, before.current_track_id);
    assert_eq!(after.position_secs, before.position_secs);
    assert_eq!(after.is_playing, before.is_playing);
    assert_eq!(after.queue_track_ids[0], manual);
    assert_eq!(
        after
            .queue_track_ids
            .iter()
            .position(|id| id == &manual_tail),
        before
            .queue_track_ids
            .iter()
            .position(|id| id == &manual_tail),
        "a manually appended track must keep its position rather than jump ahead of Flow songs"
    );
    assert_eq!(
        after.queue_track_ids[1], forgotten,
        "new steering must apply before the old batch runs out"
    );
    assert_eq!(
        playback
            .flow_session(&p.flow_engine(), "default")
            .await
            .unwrap()
            .unwrap()
            .generation,
        generation
    );
    assert_eq!(
        serde_json::to_value(playback.flow_reason("default", &current).await).unwrap(),
        original_evidence
    );
    while let Ok(event) = events.try_recv() {
        assert!(
            !matches!(
                event,
                Event::PlaybackStarted { .. } | Event::PlaybackStopped { .. }
            ),
            "steering must not create playback/history sessions"
        );
    }
    p.shutdown().await.unwrap();
}

#[tokio::test]
async fn online_only_flow_plays_full_cached_audio_through_the_shared_rust_history_path() {
    let (p, dir) = setup().await;
    ensure_online_track(
        p.db_pool(),
        "online:favorite",
        "Online Favorite",
        Some("Online Artist"),
        None,
        Some(240.0),
        None,
        None,
    )
    .await
    .unwrap();
    stats(&p, "default", "online:favorite", 8, 1, 10).await;
    cache_song(&dir, "online:favorite", "Online Favorite", "Online Artist").await;
    let mut events = p.event_bus().subscribe();
    p.dispatch_command(Command::StartFlow {
        familiarity_ratio: Some(1.0),
    })
    .await
    .unwrap();
    assert_eq!(
        p.playback_service()
            .get_playback_state()
            .await
            .current_track_id
            .as_deref(),
        Some("online:favorite")
    );
    let mut started = 0;
    while let Ok(event) = events.try_recv() {
        if let Event::PlaybackStarted {
            track_id, source, ..
        } = event
        {
            assert_eq!(track_id, "online:favorite");
            assert_eq!(source, "flow");
            started += 1;
        }
    }
    assert_eq!(started, 1);
    p.playback_service().shutdown().await.unwrap();
    p.history_service().shutdown().await;
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT track_id, source FROM playback_history")
            .fetch_all(p.db_pool())
            .await
            .unwrap();
    assert_eq!(rows, vec![("online:favorite".into(), "flow".into())]);
    p.shutdown().await.unwrap();
}

#[tokio::test]
async fn online_liked_songs_are_familiar_without_claiming_another_accounts_playlist() {
    use music_player_backend::database::repositories::{
        PlaylistRepository, SqlitePlaylistRepository,
    };
    let (p, _dir) = setup().await;
    ensure_online_track(
        p.db_pool(),
        "online:liked",
        "Liked online",
        Some("Liked artist"),
        None,
        Some(240.0),
        None,
        None,
    )
    .await
    .unwrap();
    let repo = SqlitePlaylistRepository::new(p.db_pool().clone());
    let liked = repo.ensure_liked_songs_playlist("listener").await.unwrap();
    repo.add_track(&liked, "online:liked", None).await.unwrap();
    let engine = p.flow_engine();
    let own = engine
        .generate_playback_tracks_after("listener", 1.0, 10, &[], None)
        .await
        .unwrap();
    assert_eq!(own.len(), 1);
    assert!(own[0].explanation.is_liked);
    assert_eq!(own[0].reason_category, "liked");
    assert!(engine
        .generate_playback_tracks_after("other", 1.0, 10, &[], None)
        .await
        .unwrap()
        .is_empty());
    p.shutdown().await.unwrap();
}

#[tokio::test]
async fn rediscover_keeps_room_for_online_songs_even_with_many_equal_scoring_local_candidates() {
    let (p, _dir) = setup().await;
    let favorite = local(&p, "Favorite", "Favorite Artist").await;
    stats(&p, "default", &favorite, 20, 1, 10).await;
    for i in 0..30 {
        local(&p, &format!("Local {i}"), &format!("Local Artist {i}")).await;
    }
    for i in 0..3 {
        external(
            &p,
            &format!("itunes:fresh-{i}"),
            &format!("Discovery {i}"),
            &format!("New Artist {i}"),
            "Rock",
        )
        .await;
    }
    let engine = p.flow_engine();
    let familiar = engine
        .generate_playback_tracks_after("default", 1.0, 12, &[], None)
        .await
        .unwrap();
    let explore = engine
        .generate_playback_tracks_after("default", 0.0, 12, &[], None)
        .await
        .unwrap();
    let online_count = |tracks: &[music_player_backend::recommendations::FlowTrackItem]| {
        tracks
            .iter()
            .filter(|t| t.track_id.starts_with("itunes:"))
            .count()
    };
    assert_eq!(
        online_count(&explore),
        3,
        "valid online discoveries cannot be hidden by local-catalog score ties"
    );
    assert!(online_count(&explore) > online_count(&familiar));
    p.shutdown().await.unwrap();
}
