use music_player_backend::{
    cloud::SyncManager,
    config::RankingWeightsConfig,
    database::{
        create_in_memory_pool,
        models::PlaybackHistoryRecord,
        repositories::{
            HistoryRepository, ScannedMetadata, SqliteHistoryRepository, SqliteStatsRepository,
            SqliteTrackRepository, StatsRepository, TrackRepository,
        },
    },
};

fn meta(title: &str, artist: Option<&str>, genre: Option<&str>) -> ScannedMetadata {
    ScannedMetadata {
        title_is_fallback: false,
        file_path: format!("/m/{title}.mp3"),
        file_size: 1000,
        modified_timestamp: 1,
        file_hash: None,
        title: title.into(),
        artist: artist.map(Into::into),
        album: None,
        album_artist: None,
        genre: genre.map(Into::into),
        track_number: None,
        disc_number: None,
        year: None,
        duration_secs: 200.0,
        bitrate: None,
        sample_rate: None,
        format: "mp3".into(),
        has_cover_art: false,
        musicbrainz_track_id: None,
    }
}

fn session(id: &str, track_id: &str, secs: f64) -> PlaybackHistoryRecord {
    PlaybackHistoryRecord {
        id: id.into(),
        user_id: "A".into(),
        track_id: track_id.into(),
        started_at: 1_789_689_480,
        ended_at: 1_789_689_480 + secs as i64,
        seconds_listened: secs,
        percentage_listened: 1.0,
        completed: 1,
        skipped: 0,
        source: "library".into(),
        playlist_id: None,
        recommendation_session_id: None,
    }
}

#[tokio::test]
async fn genre_counts_even_without_artist_and_replay_is_idempotent() {
    let pool = create_in_memory_pool().await.unwrap();
    let id = SqliteTrackRepository::new(pool.clone())
        .save_scanned_track(meta("NoArtist", None, Some("Jazz")))
        .await
        .unwrap();
    let history = SqliteHistoryRepository::new(pool.clone());
    for _ in 0..2 {
        history.record_session(&session("s1", &id, 120.0), "2026-09-17", true).await.unwrap();
    }
    let (plays, secs): (i64, f64) = sqlx::query_as(
        "SELECT play_count, total_time_listened FROM genre_statistics WHERE user_id = 'A'",
    )
    .fetch_one(&pool)
    .await
    .expect("genre stats row must exist without an artist");
    assert_eq!((plays, secs), (1, 120.0), "same session id counts once");
    let artists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM artist_statistics")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(artists, 0);
}

#[tokio::test]
async fn overview_caps_to_five_ranked_and_merges_same_artist() {
    let pool = create_in_memory_pool().await.unwrap();
    let repo = SqliteTrackRepository::new(pool.clone());
    let history = SqliteHistoryRepository::new(pool.clone());
    // 7 distinct artists/genres; artist/genre i is played i+1 times.
    for i in 0..7 {
        let id = repo
            .save_scanned_track(meta(&format!("T{i}"), Some(&format!("Artist{i}")), Some(&format!("Genre{i}"))))
            .await
            .unwrap();
        for n in 0..=i {
            history
                .record_session(&session(&format!("s{i}-{n}"), &id, 100.0), "2026-09-17", true)
                .await
                .unwrap();
        }
    }
    let stats = SqliteStatsRepository::new(pool.clone())
        .get_stats_overview("A", 0, 0, Some(2026), Some(9), &RankingWeightsConfig::default())
        .await
        .unwrap();
    assert_eq!(stats.top_artists.len(), 5);
    assert_eq!(stats.top_genres.len(), 5);
    assert_eq!(stats.top_artists[0].name, "Artist6");
    assert_eq!(stats.top_genres[0].name, "Genre6");
    let names: Vec<_> = stats.top_artists.iter().map(|a| a.name.to_lowercase()).collect();
    let mut dedup = names.clone();
    dedup.dedup();
    assert_eq!(names, dedup);
}

#[tokio::test]
async fn sync_restore_never_lowers_existing_track_totals() {
    let source = create_in_memory_pool().await.unwrap();
    let id = SqliteTrackRepository::new(source.clone())
        .save_scanned_track(meta("Song", Some("Artist"), Some("Rock")))
        .await
        .unwrap();
    let history = SqliteHistoryRepository::new(source.clone());
    for n in 0..3 {
        history.record_session(&session(&format!("r{n}"), &id, 100.0), "2026-09-17", true).await.unwrap();
    }
    let payload = SyncManager::prepare_local_sync_payload(&source, "A").await.unwrap();

    // Target already holds a larger legacy total without any per-device rows.
    let target = create_in_memory_pool().await.unwrap();
    SqliteTrackRepository::new(target.clone())
        .save_scanned_track(meta("Song", Some("Artist"), Some("Rock")))
        .await
        .unwrap();
    let tid: String = sqlx::query_scalar("SELECT id FROM tracks LIMIT 1").fetch_one(&target).await.unwrap();
    sqlx::query(
        "INSERT INTO track_statistics (user_id, track_id, play_count, total_time_listened, completion_count, skip_count)
         VALUES ('A', ?, 10, 1000.0, 10, 0)",
    )
    .bind(&tid)
    .execute(&target)
    .await
    .unwrap();
    // Payload references source track id; align it with the target's id.
    let mut payload = payload;
    for d in payload.track_device_stats.iter_mut() {
        d.track_id = tid.clone();
    }
    SyncManager::apply_remote_sync_payload(&target, "A", &payload).await.unwrap();
    SyncManager::apply_remote_sync_payload(&target, "A", &payload).await.unwrap();
    let (plays, secs): (i64, f64) = sqlx::query_as(
        "SELECT play_count, total_time_listened FROM track_statistics WHERE user_id='A' AND track_id=?",
    )
    .bind(&tid)
    .fetch_one(&target)
    .await
    .unwrap();
    assert_eq!((plays, secs), (10, 1000.0), "sync must not lower totals or double count on replay");
}
