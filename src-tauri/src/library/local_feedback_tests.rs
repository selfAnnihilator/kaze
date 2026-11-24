use super::*;
use crate::database::create_in_memory_pool;

const USER: &str = "u1";
const LIKED: &str = "pl_liked_u1";

fn kaze() -> PathBuf {
    PathBuf::from("/music/Kaze")
}

async fn track(pool: &SqlitePool, id: &str, path: &str) {
    sqlx::query("INSERT INTO tracks (id,file_path,file_size,modified_timestamp,title,normalized_title,duration_secs,format,created_at,updated_at) VALUES (?,?,0,0,?,?,1.0,'flac',0,0)")
        .bind(id).bind(path).bind(id).bind(id).execute(pool).await.unwrap();
}

async fn rated(pool: &SqlitePool, id: &str, like: i64) {
    sqlx::query("INSERT INTO track_statistics (user_id,track_id,manual_like) VALUES (?,?,?)")
        .bind(USER).bind(id).bind(like).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO recommendation_feedback (user_id,song_key,track_id,title,artist,manual_like,updated_at) VALUES (?,?,?,?,'a',?,0)")
        .bind(USER).bind(format!("k:{id}")).bind(id).bind(id).bind(like).execute(pool).await.unwrap();
    if like == 1 {
        sqlx::query("INSERT INTO playlist_tracks (playlist_id,track_id,position,added_at) VALUES (?,?,(SELECT count(*) FROM playlist_tracks),0)")
            .bind(LIKED).bind(id).execute(pool).await.unwrap();
    }
}

async fn setup() -> SqlitePool {
    let pool = create_in_memory_pool().await.unwrap();
    sqlx::query("INSERT INTO playlists (id,user_id,name,is_smart_mix,created_at,updated_at) VALUES (?,?,'Liked Songs',0,0,0)")
        .bind(LIKED).bind(USER).execute(&pool).await.unwrap();
    track(&pool, "mine", "/music/fav/a.flac").await;
    track(&pool, "download", "/music/Kaze/b.m4a").await;
    track(&pool, "online:x", "online://online:x").await;
    // Another device's library file, known here only as a cloud placeholder.
    track(&pool, "theirs", "online://theirs").await;
    sqlx::query("INSERT INTO external_tracks (id,provider,provider_id,title,artist,created_at) VALUES ('theirs','local','theirs','t','a',0)")
        .execute(&pool).await.unwrap();
    for id in ["mine", "download", "online:x", "theirs"] {
        rated(&pool, id, 1).await;
    }
    pool
}

async fn liked_ids(pool: &SqlitePool) -> Vec<String> {
    sqlx::query_scalar("SELECT track_id FROM playlist_tracks WHERE playlist_id = ? ORDER BY track_id")
        .bind(LIKED).fetch_all(pool).await.unwrap()
}

async fn like_of(pool: &SqlitePool, id: &str) -> (i64, i64) {
    sqlx::query_as("SELECT s.manual_like, f.manual_like FROM track_statistics s JOIN recommendation_feedback f ON f.track_id = s.track_id WHERE s.track_id = ?")
        .bind(id).fetch_one(pool).await.unwrap()
}

#[tokio::test]
async fn only_files_from_the_users_folders_are_local() {
    let pool = setup().await;
    assert!(is_local_file(&pool, "mine", &kaze()).await.unwrap());
    assert!(is_local_file(&pool, "theirs", &kaze()).await.unwrap(), "another device's library file");
    assert!(!is_local_file(&pool, "download", &kaze()).await.unwrap(), "Kaze downloads take ratings");
    assert!(!is_local_file(&pool, "online:x", &kaze()).await.unwrap());
}

#[tokio::test]
async fn own_local_ratings_are_cleared_and_the_removal_syncs() {
    let pool = setup().await;
    sqlx::query("UPDATE track_statistics SET manual_like = -1 WHERE track_id = 'mine'").execute(&pool).await.unwrap();
    assert_eq!(clear(&pool, USER, &kaze()).await.unwrap(), 2);
    assert_eq!(like_of(&pool, "mine").await, (0, 0), "a dislike goes too");
    assert_eq!(liked_ids(&pool).await, ["download", "online:x"]);
    let tombstones = SyncManager::get_tombstones(&pool, USER).await.unwrap();
    assert_eq!(tombstones.into_iter().map(|t| t.2).collect::<Vec<_>>(), [format!("{LIKED}:mine")], "only this device's file is removed in the cloud");
}

#[tokio::test]
async fn another_devices_file_leaves_liked_songs_without_changing_its_rating() {
    let pool = setup().await;
    clear(&pool, USER, &kaze()).await.unwrap();
    assert_eq!(like_of(&pool, "theirs").await, (1, 1), "the owning device clears it, or keeps it if it's a download there");
    assert_eq!(like_of(&pool, "download").await, (1, 1));
    assert_eq!(like_of(&pool, "online:x").await, (1, 1));
}

#[tokio::test]
async fn a_second_cleanup_changes_nothing() {
    let pool = setup().await;
    clear(&pool, USER, &kaze()).await.unwrap();
    assert_eq!(clear(&pool, USER, &kaze()).await.unwrap(), 0);
}
