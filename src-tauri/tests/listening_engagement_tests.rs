use music_player_backend::{
    database::{
        create_in_memory_pool,
        models::PlaybackHistoryRecord,
        repositories::{
            HistoryRepository, ScannedMetadata, SqliteHistoryRepository, SqliteTrackRepository,
            TrackRepository,
        },
    },
    recommendations::TasteProfileEngine,
};

async fn track(pool: &sqlx::SqlitePool, name: &str, duration: f64) -> String {
    SqliteTrackRepository::new(pool.clone())
        .save_scanned_track(ScannedMetadata {
            file_path: format!("/fixture/{name}.flac"),
            file_size: 100,
            modified_timestamp: 1,
            file_hash: None,
            title: name.into(),
            title_is_fallback: false,
            artist: Some(name.into()),
            album: Some("Album".into()),
            album_artist: None,
            genre: Some("Rock".into()),
            year: Some(2024),
            track_number: None,
            disc_number: None,
            duration_secs: duration,
            bitrate: None,
            sample_rate: None,
            format: "flac".into(),
            has_cover_art: false,
            musicbrainz_track_id: None,
        })
        .await
        .unwrap()
}
async fn listen(
    pool: &sqlx::SqlitePool,
    id: &str,
    session: &str,
    seconds: f64,
    completed: bool,
    skipped: bool,
) {
    let now = chrono::Utc::now().timestamp();
    SqliteHistoryRepository::new(pool.clone())
        .record_playback(&PlaybackHistoryRecord {
            id: session.into(),
            track_id: id.into(),
            user_id: "default".into(),
            started_at: now,
            ended_at: now,
            seconds_listened: seconds,
            percentage_listened: if completed { 1.0 } else { seconds / 240.0 },
            completed: completed.into(),
            skipped: skipped.into(),
            source: "library".into(),
            playlist_id: None,
            recommendation_session_id: None,
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn brief_previews_are_neutral_and_cannot_become_positive_from_completion_flags() {
    let pool = create_in_memory_pool().await.unwrap();
    for (name, seconds, complete) in [
        ("Preview", 5.0, false),
        ("Neutral", 20.0, false),
        ("SeekToEnd", 5.0, true),
    ] {
        let id = track(&pool, name, 240.0).await;
        listen(&pool, &id, name, seconds, complete, false).await;
    }
    let profile = TasteProfileEngine::new(pool)
        .compute_taste_profile("default")
        .await
        .unwrap();
    for name in ["Preview", "Neutral", "SeekToEnd"] {
        assert_eq!(
            profile.artist_affinity(name),
            0.0,
            "{name} must not earn taste without enough audible listening"
        );
    }
}

#[test]
fn listening_tiers_have_real_time_floors_and_safe_boundaries() {
    use music_player_backend::recommendations::engagement::Engagement as E;
    assert_eq!(E::from_listening(5.0, 240.0, false).weight, 0.0);
    assert_eq!(E::from_listening(9.999, 240.0, true).weight, -0.75);
    assert_eq!(E::from_listening(10.0, 240.0, true).weight, 0.0);
    assert_eq!(E::from_listening(29.999, 240.0, true).weight, 0.0);
    assert_eq!(E::from_listening(30.0, 240.0, true).weight, 0.35);
    assert_eq!(E::from_listening(180.0, 240.0, false).weight, 0.75);
    assert_eq!(E::from_listening(240.0, 240.0, false).weight, 1.0);
    assert!(!E::from_listening(5.0, 20.0, false).qualified);
    assert!(E::from_listening(10.0, 20.0, false).qualified);
    assert!(!E::from_listening(14.9, 60.0, false).qualified);
    assert!(E::from_listening(15.0, 60.0, false).qualified);
    assert!(!E::from_listening(29.9, 0.0, false).qualified);
    assert_eq!(E::from_listening(30.0, 0.0, false).weight, 0.35);
    assert_eq!(E::from_listening(f64::NAN, 240.0, false).weight, 0.0);
    assert_eq!(E::from_listening(f64::INFINITY, 240.0, false).weight, 0.0);
    assert_eq!(E::from_listening(1000.0, 240.0, false).seconds, 240.0);
}

#[tokio::test]
async fn longer_listens_gain_more_taste_and_a_quick_skip_reduces_preference() {
    let pool = create_in_memory_pool().await.unwrap();
    for (name, seconds) in [("Baseline", 30.0), ("Strong", 180.0), ("Full", 240.0)] {
        let id = track(&pool, name, 240.0).await;
        listen(&pool, &id, name, seconds, seconds == 240.0, false).await;
    }
    let engine = TasteProfileEngine::new(pool.clone());
    let before = engine.compute_taste_profile("default").await.unwrap();
    assert!(before.artist_affinity("Baseline") > 0.0);
    assert!(before.artist_affinity("Strong") > before.artist_affinity("Baseline"));
    assert!(before.artist_affinity("Full") > before.artist_affinity("Strong"));
    let full: String = sqlx::query_scalar("SELECT id FROM tracks WHERE title='Full'")
        .fetch_one(&pool)
        .await
        .unwrap();
    listen(&pool, &full, "early-skip", 5.0, false, true).await;
    let after = engine.compute_taste_profile("default").await.unwrap();
    assert!(after.artist_affinity("Full") < before.artist_affinity("Full"));
    assert_eq!(
        engine
            .compute_taste_profile("another-user")
            .await
            .unwrap()
            .artist_affinity("Full"),
        0.0
    );
}

#[tokio::test]
async fn brief_sessions_do_not_add_preference_and_false_completions_are_reclassified_on_read() {
    use music_player_backend::recommendations::signals::SongSignals;
    let pool = create_in_memory_pool().await.unwrap();
    let id = track(&pool, "Song", 240.0).await;
    sqlx::query("INSERT INTO track_statistics(user_id,track_id,play_count,completion_count,skip_count,total_time_listened,last_played_at) VALUES('default',?,1,1,0,5,1)").bind(&id).execute(&pool).await.unwrap();
    listen(&pool, &id, "old-false-end", 5.0, true, false).await;
    let signal = SongSignals::load(&pool, "default")
        .await
        .unwrap()
        .signal(&id, "Song", "Song");
    assert_eq!(
        (
            signal.play_count,
            signal.completion_count,
            signal.skip_count
        ),
        (0, 0, 0)
    );
    assert_eq!(signal.engagement(), None);
    listen(&pool, &id, "earned", 30.0, false, false).await;
    let before = SongSignals::load(&pool, "default")
        .await
        .unwrap()
        .signal(&id, "Song", "Song");
    assert_eq!(before.play_count, 1);
    assert_eq!(before.engagement(), Some(0.35));
    for i in 0..3 {
        listen(&pool, &id, &format!("preview{i}"), 20.0, false, false).await;
    }
    let after = SongSignals::load(&pool, "default")
        .await
        .unwrap()
        .signal(&id, "Song", "Song");
    assert_eq!(after.play_count, before.play_count);
    assert_eq!(after.engagement(), before.engagement());
    assert_eq!(after.last_played_at, before.last_played_at);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT play_count FROM track_statistics WHERE track_id=?")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        1,
        "recommendation correction must not rewrite imported lifetime/device counters"
    );
}

#[test]
fn recommendation_performance_rewards_qualified_time_and_respects_dislikes() {
    use music_player_backend::recommendations::{CandidateTrack, ScoringEngine, TasteProfile};
    let score = |weight, plays, skips, feedback| {
        ScoringEngine::score_track(
            &CandidateTrack {
                id: "song".into(),
                title: "Song".into(),
                artist_id: None,
                artist_name: Some("Artist".into()),
                genre_name: None,
                year: None,
                language: None,
                play_count: plays,
                completion_count: 0,
                skip_count: skips,
                listening_engagement: weight,
                last_played_at: None,
                user_feedback: feedback,
            },
            &TasteProfile::default(),
            chrono::Utc::now().timestamp(),
        )
        .final_score
    };
    let neutral = score(None, 0, 0, None);
    assert!(score(Some(0.35), 1, 0, None) > neutral);
    assert!(score(Some(0.75), 1, 0, None) > score(Some(0.35), 1, 0, None));
    assert!(score(Some(1.0), 1, 0, None) > score(Some(0.75), 1, 0, None));
    assert!(score(Some(1.0), 2, 0, None) > score(Some(1.0), 1, 0, None));
    assert!(score(Some(-0.75), 0, 1, None) < neutral);
    assert!(score(Some(1.0), 2, 0, Some("like".into())) > score(Some(1.0), 1, 0, None));
    assert_eq!(score(Some(1.0), 10, 0, Some("dislike".into())), 0.0);
}

#[tokio::test]
async fn eof_after_seek_and_playback_failure_do_not_manufacture_listens_or_skip_penalties() {
    use music_player_backend::{
        config::HistoryConfig,
        core::{Event, EventBus},
        database::repositories::SqliteStatsRepository,
        history::HistoryService,
    };
    use std::sync::Arc;
    let pool = create_in_memory_pool().await.unwrap();
    let id = track(&pool, "Actual Audio", 240.0).await;
    let bus = Arc::new(EventBus::default());
    let mut events = bus.subscribe();
    let service = HistoryService::new_with_pool(
        Arc::new(SqliteHistoryRepository::new(pool.clone())),
        Arc::new(SqliteStatsRepository::new(pool.clone())),
        HistoryConfig::default(),
        bus.clone(),
        None,
        Some(pool.clone()),
    );
    for session in ["seek-end", "failed-source"] {
        bus.publish(Event::PlaybackStarted {
            session_id: session.into(),
            track_id: id.clone(),
            title: "Actual Audio".into(),
            artist: "Artist".into(),
            duration_secs: 240.0,
            source: "library".into(),
        })
        .unwrap();
        if session == "seek-end" {
            bus.publish(Event::PlaybackSeeked {
                position_secs: 239.9,
            })
            .unwrap();
            bus.publish(Event::TrackFinished {
                session_id: session.into(),
                track_id: id.clone(),
                seconds_listened: 0.0,
                completed: true,
            })
            .unwrap();
        } else {
            bus.publish(Event::PlaybackError {
                session_id: session.into(),
                message: "source interrupted".into(),
            })
            .unwrap();
            bus.publish(Event::PlaybackStopped {
                session_id: session.into(),
            })
            .unwrap();
        }
    }
    service.shutdown().await;
    let rows: Vec<(f64, i64, i64)> =
        sqlx::query_as("SELECT seconds_listened,completed,skipped FROM playback_history")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(rows.len(), 2);
    for (seconds, completed, skipped) in rows {
        assert!(seconds < 10.0);
        assert_eq!((completed, skipped), (0, 0));
    }
    while let Ok(event) = events.try_recv() {
        assert!(
            !matches!(event, Event::TasteProfileChanged { .. }),
            "neutral sessions must not trigger taste refreshes"
        );
    }
}

#[tokio::test]
async fn repeated_previews_cannot_fill_on_repeat_or_hide_genuinely_repeated_songs() {
    use music_player_backend::{core::command::SmartMixType, recommendations::SmartMixGenerator};
    let pool = create_in_memory_pool().await.unwrap();
    // More ineligible previews than the mix's limit, each with more attempts
    // than the real repeated song. Qualification must happen before LIMIT.
    for i in 0..26 {
        let id = track(&pool, &format!("Preview {i}"), 240.0).await;
        for j in 0..3 {
            listen(&pool, &id, &format!("preview-{i}-{j}"), 20.0, false, false).await;
        }
    }
    let genuine = track(&pool, "Genuine Repeat", 240.0).await;
    for i in 0..2 {
        listen(
            &pool,
            &genuine,
            &format!("qualified-{i}"),
            30.0,
            false,
            false,
        )
        .await;
    }
    let mix = SmartMixGenerator::new(pool.clone())
        .generate_mix("default", &SmartMixType::OnRepeat)
        .await
        .unwrap();
    let ids: Vec<String> = sqlx::query_scalar(
        "SELECT track_id FROM playlist_tracks WHERE playlist_id=? ORDER BY position",
    )
    .bind(mix.id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(ids, vec![genuine]);
}

#[tokio::test]
async fn permissive_legacy_play_counters_cannot_turn_short_previews_into_familiarity() {
    use music_player_backend::recommendations::signals::SongSignals;
    let pool = create_in_memory_pool().await.unwrap();
    let id = track(&pool, "Old short-play setting", 240.0).await;
    sqlx::query("INSERT INTO track_statistics(user_id,track_id,play_count,completion_count,skip_count,total_time_listened) VALUES('default',?,1,0,0,20)").bind(&id).execute(&pool).await.unwrap();
    listen(&pool, &id, "legacy-short", 20.0, false, false).await;
    let heard = SongSignals::load(&pool, "default").await.unwrap().signal(
        &id,
        "Old short-play setting",
        "Old short-play setting",
    );
    assert_eq!(heard.play_count, 0);
    assert_eq!(heard.engagement(), None);
}

#[tokio::test]
async fn ranked_scores_ignore_neutral_preview_time_but_usage_totals_remain_true() {
    use music_player_backend::{
        config::RankingWeightsConfig,
        database::repositories::{SqliteStatsRepository, StatsRepository},
    };
    let pool = create_in_memory_pool().await.unwrap();
    let id = track(&pool, "Ranked Song", 240.0).await;
    let now = chrono::Utc::now().timestamp();
    let history = SqliteHistoryRepository::new(pool.clone());
    let entry = |sid: &str, seconds: f64| PlaybackHistoryRecord {
        id: sid.into(),
        user_id: "default".into(),
        track_id: id.clone(),
        started_at: now,
        ended_at: now,
        seconds_listened: seconds,
        percentage_listened: seconds / 240.0,
        completed: 0,
        skipped: 0,
        source: "library".into(),
        playlist_id: None,
        recommendation_session_id: None,
    };
    let repo = SqliteStatsRepository::new(pool.clone());
    let weights = RankingWeightsConfig::default();
    history
        .record_session(&entry("neutral", 20.0), "2026-10-28", false)
        .await
        .unwrap();
    let first = repo
        .get_all_time_ranked_tracks("default", &weights, 10)
        .await
        .unwrap();
    assert_eq!(first[0].score, 0.0);
    assert_eq!(first[0].total_seconds, 20.0);
    history
        .record_session(&entry("qualified", 30.0), "2026-10-28", true)
        .await
        .unwrap();
    let earned = repo
        .get_all_time_ranked_tracks("default", &weights, 10)
        .await
        .unwrap()[0]
        .score;
    assert!(earned > 0.0);
    history
        .record_session(&entry("another-preview", 20.0), "2026-10-28", false)
        .await
        .unwrap();
    let after = repo
        .get_all_time_ranked_tracks("default", &weights, 10)
        .await
        .unwrap();
    assert_eq!(
        after[0].score, earned,
        "extra preview seconds cannot add scoring points"
    );
    assert_eq!(after[0].total_seconds, 70.0);
    assert_eq!(after[0].play_count, 1);
    let windowed = repo
        .get_ranked_tracks_scoped(Some("default"), Some(now - 1), Some(now + 1), &weights, 10)
        .await
        .unwrap();
    assert_eq!(windowed[0].score, earned);
    assert_eq!(windowed[0].total_seconds, 70.0);
    let artist = repo
        .get_all_time_ranked_artists("default", &weights, 10)
        .await
        .unwrap();
    assert_eq!(artist[0].score, earned);
    assert_eq!(artist[0].total_seconds, 70.0);
    let windowed_artist = repo
        .get_ranked_artists_scoped(Some("default"), Some(now - 1), Some(now + 1), &weights, 10)
        .await
        .unwrap();
    assert_eq!(windowed_artist[0].score, earned);
    let albums = repo
        .get_ranked_albums_for_user(Some("default"), Some(now - 1), &weights, 10)
        .await
        .unwrap();
    assert_eq!(albums[0].score, earned);
    let genres = repo
        .get_ranked_genres_for_user(Some("default"), Some(now - 1), &weights, 10)
        .await
        .unwrap();
    assert_eq!(genres[0].score, earned);
}

#[tokio::test]
async fn new_policy_expires_only_automatic_mix_caches() {
    let pool = create_in_memory_pool().await.unwrap();
    sqlx::query("INSERT INTO playlists(id,name,user_id,is_smart_mix,expires_at,created_at,updated_at) VALUES('auto','Auto','default',1,9999999999,1,1),('manual','Manual','default',0,9999999999,1,1)").execute(&pool).await.unwrap();
    sqlx::raw_sql(include_str!(
        "../migrations/20261028020000_listening_engagement_mix_refresh.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT id,expires_at FROM playlists WHERE id IN ('auto','manual') ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        rows,
        vec![("auto".into(), 0), ("manual".into(), 9999999999)]
    );
}

#[tokio::test]
async fn leaving_a_buffering_song_is_neutral_instead_of_a_dislike() {
    use music_player_backend::{
        config::HistoryConfig,
        core::{Event, EventBus},
        database::repositories::SqliteStatsRepository,
        history::HistoryService,
    };
    use std::sync::Arc;
    let pool = create_in_memory_pool().await.unwrap();
    let id = track(&pool, "Stalled Audio", 240.0).await;
    let bus = Arc::new(EventBus::default());
    let service = HistoryService::new_with_pool(
        Arc::new(SqliteHistoryRepository::new(pool.clone())),
        Arc::new(SqliteStatsRepository::new(pool.clone())),
        HistoryConfig::default(),
        bus.clone(),
        None,
        Some(pool.clone()),
    );
    bus.publish(Event::PlaybackStarted {
        session_id: "buffered".into(),
        track_id: id.clone(),
        title: "Stalled Audio".into(),
        artist: "Artist".into(),
        duration_secs: 240.0,
        source: "online".into(),
    })
    .unwrap();
    bus.publish(Event::PlaybackBufferingChanged {
        session_id: "buffered".into(),
        track_id: id,
        is_buffering: true,
    })
    .unwrap();
    bus.publish(Event::PlaybackStopped {
        session_id: "buffered".into(),
    })
    .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM playback_history WHERE id='buffered'",
            )
            .fetch_one(&pool)
            .await
            .unwrap()
                == 1
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    service.shutdown().await;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT skipped FROM playback_history WHERE id='buffered'")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}
