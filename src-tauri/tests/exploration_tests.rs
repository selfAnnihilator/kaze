use music_player_backend::database::create_in_memory_pool;
use music_player_backend::database::repositories::{
    ScannedMetadata, SqliteTrackRepository, TrackRepository,
};
use music_player_backend::recommendations::exploration::{
    bias_from_counts, compute_bias, explore_bias, record_slots, stamp_listen, EXPLORE, FAMILIAR,
    MAX_BIAS, MIN_LISTENS,
};
use sqlx::SqlitePool;

const NOW: i64 = 1_800_000_000;

async fn track(repo: &SqliteTrackRepository) -> String {
    repo.save_scanned_track(ScannedMetadata {
        title_is_fallback: false,
        file_path: "/music/a.mp3".to_string(),
        file_size: 2000,
        modified_timestamp: 100,
        file_hash: None,
        title: "A".to_string(),
        artist: Some("Artist".to_string()),
        album: Some("Album".to_string()),
        album_artist: None,
        genre: Some("Rock".to_string()),
        track_number: Some(1),
        disc_number: Some(1),
        year: Some(2020),
        duration_secs: 200.0,
        bitrate: Some(128),
        sample_rate: Some(44100),
        format: "mp3".to_string(),
        has_cover_art: false,
        musicbrainz_track_id: None,
    })
    .await
    .expect("save track")
}

/// Inserts `n` Flow listens in `slot`; the first `good` complete, the next
/// `skipped` are early skips, the rest are partial.
async fn listens(pool: &SqlitePool, track_id: &str, slot: &str, n: i64, good: i64, skipped: i64) {
    for i in 0..n {
        let (completed, skip, pct) = if i < good {
            (1, 0, 1.0)
        } else if i < good + skipped {
            (0, 1, 0.05)
        } else {
            (0, 0, 0.3)
        };
        sqlx::query(
            "INSERT INTO playback_history (id, user_id, track_id, started_at, ended_at,
                seconds_listened, percentage_listened, completed, skipped, source, rec_slot)
             VALUES (?, 'default', ?, ?, ?, 10, ?, ?, ?, 'flow', ?)",
        )
        .bind(format!("{slot}-{i}"))
        .bind(track_id)
        .bind(NOW - 3600 - i)
        .bind(NOW - 3000 - i)
        .bind(pct)
        .bind(completed)
        .bind(skip)
        .bind(slot)
        .execute(pool)
        .await
        .unwrap();
    }
}

#[test]
fn pure_bias_signs_and_bounds() {
    let n = MIN_LISTENS;
    assert!(bias_from_counts((n, n - 2, 1), (n, n / 2, n / 4)) > 0.0);
    assert!(bias_from_counts((n, 2, n - 4), (n, n / 2, 2)) < 0.0);
    assert_eq!(bias_from_counts((n - 1, n - 1, 0), (n, 0, n)), 0.0);
    assert_eq!(bias_from_counts((n, n, 0), (n - 1, 0, 0)), 0.0);
    let extreme = bias_from_counts((n, n, 0), (n, 0, n));
    assert!((extreme - MAX_BIAS).abs() < 1e-9);
}

#[tokio::test]
async fn completed_explore_listens_raise_bias_and_skips_lower_it() {
    let pool = create_in_memory_pool().await.unwrap();
    let repo = SqliteTrackRepository::new(pool.clone());
    let id = track(&repo).await;

    listens(&pool, &id, EXPLORE, 30, 27, 1).await;
    listens(&pool, &id, FAMILIAR, 30, 15, 8).await;
    let up = compute_bias(&pool, "default", NOW).await.unwrap();
    assert!(up > 0.0 && up <= MAX_BIAS, "bias {up}");

    sqlx::query("DELETE FROM playback_history").execute(&pool).await.unwrap();
    listens(&pool, &id, EXPLORE, 30, 3, 24).await;
    listens(&pool, &id, FAMILIAR, 30, 15, 8).await;
    let down = compute_bias(&pool, "default", NOW).await.unwrap();
    assert!((-MAX_BIAS..0.0).contains(&down), "bias {down}");

    // Older than the window: ignored, so no evidence.
    let later = NOW + 31 * 86_400;
    assert_eq!(compute_bias(&pool, "default", later).await.unwrap(), 0.0);
}

#[tokio::test]
async fn below_minimum_listens_bias_is_zero() {
    let pool = create_in_memory_pool().await.unwrap();
    let repo = SqliteTrackRepository::new(pool.clone());
    let id = track(&repo).await;
    listens(&pool, &id, EXPLORE, 10, 10, 0).await;
    listens(&pool, &id, FAMILIAR, 40, 0, 40).await;
    assert_eq!(compute_bias(&pool, "default", NOW).await.unwrap(), 0.0);
}

#[tokio::test]
async fn slots_stamp_listens_and_bias_is_cached_daily() {
    let pool = create_in_memory_pool().await.unwrap();
    let repo = SqliteTrackRepository::new(pool.clone());
    let id = track(&repo).await;

    record_slots(&pool, "default", &[(id.clone(), EXPLORE)], NOW).await.unwrap();
    sqlx::query(
        "INSERT INTO playback_history (id, user_id, track_id, started_at, ended_at,
            seconds_listened, percentage_listened, completed, skipped, source)
         VALUES ('h1', 'default', ?, ?, ?, 10, 1.0, 1, 0, 'flow')",
    )
    .bind(&id)
    .bind(NOW - 100)
    .bind(NOW - 50)
    .execute(&pool)
    .await
    .unwrap();
    stamp_listen(&pool, "h1", "default", &id).await.unwrap();
    let slot: (Option<String>,) =
        sqlx::query_as("SELECT rec_slot FROM playback_history WHERE id = 'h1'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(slot.0.as_deref(), Some(EXPLORE));

    // Re-enqueue as familiar overwrites the assignment.
    record_slots(&pool, "default", &[(id.clone(), FAMILIAR)], NOW).await.unwrap();
    stamp_listen(&pool, "h1", "default", &id).await.unwrap();
    let slot: (Option<String>,) =
        sqlx::query_as("SELECT rec_slot FROM playback_history WHERE id = 'h1'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(slot.0.as_deref(), Some(FAMILIAR));

    // Cache: first call computes 0 (no evidence) and stores it.
    assert_eq!(explore_bias(&pool, "default", NOW).await.unwrap(), 0.0);
    listens(&pool, &id, EXPLORE, 30, 30, 0).await;
    listens(&pool, &id, FAMILIAR, 30, 0, 30).await;
    // Within a day the cached value is served.
    assert_eq!(explore_bias(&pool, "default", NOW + 60).await.unwrap(), 0.0);
    // After a day it recomputes.
    let fresh = explore_bias(&pool, "default", NOW + 86_400).await.unwrap();
    assert!((fresh - MAX_BIAS).abs() < 1e-9, "fresh {fresh}");
}
