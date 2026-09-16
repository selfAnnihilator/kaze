use music_player_backend::database::create_in_memory_pool;
use music_player_backend::database::repositories::playlist_repo::{PlaylistRepository, SqlitePlaylistRepository};

#[tokio::test]
async fn playlist_rejects_repeat_ids_and_duplicate_song_metadata() {
    let pool = create_in_memory_pool().await.expect("database");
    sqlx::query("INSERT INTO playlists (id, name, created_at, updated_at) VALUES ('sleep', 'Sleep', 1, 1)")
        .execute(&pool).await.expect("playlist");
    for (id, name) in [("kato", "Kato"), ("other", "Other Artist")] {
        sqlx::query("INSERT INTO artists (id, name, normalized_name, created_at) VALUES (?, ?, ?, 1)")
            .bind(id).bind(name).bind(name.to_lowercase())
            .execute(&pool).await.expect("artist");
    }
    for (id, title, artist) in [
        ("a", "Unravel", "kato"),
        ("b", "unravel", "kato"),
        ("c", "Littleroot Town", "kato"),
        ("d", "Unravel", "other"),
    ] {
        sqlx::query("INSERT INTO tracks (id, file_path, file_size, modified_timestamp, title, normalized_title, artist_id, duration_secs, format, created_at, updated_at) VALUES (?, ?, 1, 1, ?, ?, ?, 120, 'mp3', 1, 1)")
            .bind(id).bind(format!("/tmp/{id}.mp3")).bind(title).bind(title.to_lowercase()).bind(artist)
            .execute(&pool).await.expect("track");
    }

    let repo = SqlitePlaylistRepository::new(pool.clone());
    repo.add_track("sleep", "a", None).await.expect("first song");
    repo.add_track("sleep", "a", None).await.expect("repeat ID is harmless");
    repo.add_track("sleep", "b", None).await.expect("same song from another source is harmless");
    repo.add_track("sleep", "c", None).await.expect("different title");
    repo.add_track("sleep", "d", None).await.expect("same title, different artist");
    assert_eq!(repo.get_track_count("sleep").await.expect("count"), 3);

    let ids: Vec<(String,)> = sqlx::query_as("SELECT track_id FROM playlist_tracks WHERE playlist_id = 'sleep' ORDER BY position")
        .fetch_all(&pool).await.expect("ordered tracks");
    assert_eq!(ids.into_iter().map(|row| row.0).collect::<Vec<_>>(), ["a", "c", "d"]);

    repo.set_tracks("sleep", &["a".into(), "b".into(), "a".into(), "c".into()])
        .await.expect("replace playlist with duplicates");
    assert_eq!(repo.get_track_count("sleep").await.expect("count"), 2);

    sqlx::query("INSERT INTO playlist_tracks (playlist_id, track_id, position, added_at) VALUES ('sleep', 'b', 8, 1) ON CONFLICT(playlist_id, position) DO UPDATE SET track_id = excluded.track_id")
        .execute(&pool).await.expect("sync duplicate ignored");
    sqlx::query("INSERT INTO playlist_tracks (playlist_id, track_id, position, added_at) VALUES ('sleep', 'b', 3, 1) ON CONFLICT(playlist_id, position) DO UPDATE SET track_id = excluded.track_id")
        .execute(&pool).await.expect("sync update duplicate ignored");
    assert_eq!(repo.get_track_count("sleep").await.expect("count"), 2);
    let remaining: Vec<(String,)> = sqlx::query_as("SELECT track_id FROM playlist_tracks WHERE playlist_id = 'sleep' ORDER BY position")
        .fetch_all(&pool).await.expect("remaining tracks");
    assert_eq!(remaining.into_iter().map(|row| row.0).collect::<Vec<_>>(), ["a", "c"]);
}
