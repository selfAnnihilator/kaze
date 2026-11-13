//! Taste affinity must saturate: a couple of listens is a hint, not a verdict.
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

async fn track(pool: &sqlx::SqlitePool, title: &str, artist: &str, genre: &str) -> String {
    SqliteTrackRepository::new(pool.clone())
        .save_scanned_track(ScannedMetadata {
            file_path: format!("/fixture/{artist}-{title}.flac"),
            file_size: 100,
            modified_timestamp: 1,
            file_hash: None,
            title: title.into(),
            title_is_fallback: false,
            artist: Some(artist.into()),
            album: Some(format!("{artist} Album")),
            album_artist: None,
            genre: Some(genre.into()),
            year: Some(2020),
            track_number: None,
            disc_number: None,
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

async fn full_listen(pool: &sqlx::SqlitePool, id: &str, session: &str) {
    let now = chrono::Utc::now().timestamp();
    SqliteHistoryRepository::new(pool.clone())
        .record_playback(&PlaybackHistoryRecord {
            id: session.into(),
            track_id: id.into(),
            user_id: "default".into(),
            started_at: now,
            ended_at: now,
            seconds_listened: 240.0,
            percentage_listened: 1.0,
            completed: true.into(),
            skipped: false.into(),
            source: "library".into(),
            playlist_id: None,
            recommendation_session_id: None,
        })
        .await
        .unwrap();
}

async fn like(pool: &sqlx::SqlitePool, id: &str) {
    sqlx::query(
        "INSERT INTO track_statistics (user_id, track_id, manual_like) VALUES ('default', ?, 1)
         ON CONFLICT(user_id, track_id) DO UPDATE SET manual_like = 1",
    )
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn two_full_listens_are_a_hint_not_max_affinity() {
    let pool = create_in_memory_pool().await.unwrap();
    for n in 0..2 {
        let id = track(&pool, &format!("Song {n}"), "Spammy", "Rock").await;
        full_listen(&pool, &id, &format!("s{n}")).await;
    }
    let profile = TasteProfileEngine::new(pool)
        .compute_taste_profile("default")
        .await
        .unwrap();
    let aff = profile.artist_affinity("Spammy");
    assert!(aff > 0.0 && aff < 0.5, "two listens gave {aff}");
}

#[tokio::test]
async fn replaying_one_song_does_not_make_artist_a_favorite() {
    let pool = create_in_memory_pool().await.unwrap();
    let id = track(&pool, "Only Hit", "OneHit", "Pop").await;
    for n in 0..10 {
        full_listen(&pool, &id, &format!("r{n}")).await;
    }
    let profile = TasteProfileEngine::new(pool)
        .compute_taste_profile("default")
        .await
        .unwrap();
    let aff = profile.artist_affinity("OneHit");
    assert!(aff > 0.0 && aff < 0.6, "one song x10 gave artist {aff}");
    assert_eq!(profile.artist_breadth("OneHit"), 1);
}

#[tokio::test]
async fn liking_many_distinct_songs_builds_strong_artist_affinity() {
    let pool = create_in_memory_pool().await.unwrap();
    for n in 0..6 {
        let id = track(&pool, &format!("Deep Cut {n}"), "Loved", "Jazz").await;
        full_listen(&pool, &id, &format!("l{n}")).await;
        like(&pool, &id).await;
    }
    let profile = TasteProfileEngine::new(pool)
        .compute_taste_profile("default")
        .await
        .unwrap();
    let aff = profile.artist_affinity("Loved");
    assert!(aff > 0.75, "six liked songs gave {aff}");
    assert!(profile.artist_breadth("Loved") >= 6);
    // Liked songs are a style signal too.
    let jazz = profile.genre_affinities.get("Jazz").copied().unwrap_or(0.0);
    assert!(jazz > 0.5, "liked jazz songs gave genre {jazz}");
}

#[tokio::test]
async fn shared_local_library_alone_creates_no_taste() {
    let pool = create_in_memory_pool().await.unwrap();
    for n in 0..8 {
        track(&pool, &format!("Owned {n}"), "Shelf", "Metal").await;
    }
    let profile = TasteProfileEngine::new(pool)
        .compute_taste_profile("default")
        .await
        .unwrap();
    assert_eq!(profile.artist_affinity("Shelf"), 0.0);
}

#[tokio::test]
async fn owning_more_songs_amplifies_an_artist_you_already_play() {
    async fn affinity_with_owned(extra: usize) -> f64 {
        let pool = create_in_memory_pool().await.unwrap();
        let id = track(&pool, "Played", "Known", "Folk").await;
        full_listen(&pool, &id, "k0").await;
        for n in 0..extra {
            track(&pool, &format!("Owned {n}"), "Known", "Folk").await;
        }
        TasteProfileEngine::new(pool)
            .compute_taste_profile("default")
            .await
            .unwrap()
            .artist_affinity("Known")
    }
    let lone = affinity_with_owned(0).await;
    let owned = affinity_with_owned(6).await;
    assert!(owned > lone, "owned {owned} should exceed lone {lone}");
}
