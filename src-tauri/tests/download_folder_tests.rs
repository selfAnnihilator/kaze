//! Kaze-owned download folder: moving Kaze's own files in, keeping library rows,
//! the Local/Downloads split, the embedded recording id and the folder watcher.
//! Every test uses temporary directories, never the real ~/Music.

use music_player_backend::core::error::AppResult;
use music_player_backend::core::event::Event;
use music_player_backend::core::EventBus;
use music_player_backend::database::models::DownloadTaskRecord;
use music_player_backend::database::repositories::{
    DownloadRepository, LibraryScope, ScannedMetadata, SqliteDownloadRepository,
    SqliteWishlistRepository, TrackRepository,
};
use music_player_backend::discovery::WishlistManager;
use music_player_backend::downloads::{write_tags, DownloadService, DownloadTags, MockDownloadProvider};
use music_player_backend::library::LibraryService;
use sqlx::sqlite::SqlitePoolOptions;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::tempdir;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/seek-tone.m4a");

async fn pool() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

struct Fixture {
    service: Arc<DownloadService>,
    repo: Arc<SqliteDownloadRepository>,
    library: Arc<LibraryService>,
    bus: Arc<EventBus>,
}

fn service(pool: &sqlx::SqlitePool, kaze: &Path) -> Fixture {
    let bus = Arc::new(EventBus::default());
    let repo = Arc::new(SqliteDownloadRepository::new(pool.clone()));
    let library = Arc::new(LibraryService::new(pool.clone(), bus.clone()));
    let service = Arc::new(DownloadService::new(
        repo.clone(),
        Arc::new(MockDownloadProvider::new()),
        bus.clone(),
        library.clone(),
        Arc::new(WishlistManager::new(Arc::new(SqliteWishlistRepository::new(pool.clone())))),
        kaze.to_path_buf(),
        true,
    ));
    Fixture { service, repo, library, bus }
}

fn completed(id: &str, source: &str, path: &Path) -> DownloadTaskRecord {
    DownloadTaskRecord {
        source_track_id: Some(source.into()),
        id: id.into(),
        provider: "jiosaavn".into(),
        provider_task_id: None,
        title: "Song".into(),
        artist: "Artist".into(),
        album: None,
        filename: path.file_name().unwrap().to_string_lossy().into(),
        destination_path: Some(path.to_string_lossy().into()),
        file_size: Some(1),
        bytes_downloaded: 1,
        status: "COMPLETED".into(),
        error_message: None,
        wishlist_id: None,
        created_at: 1,
        completed_at: Some(1),
    }
}

fn canonical(path: &Path) -> String {
    path.canonicalize().unwrap().to_string_lossy().into()
}

#[tokio::test]
async fn only_kazes_own_downloads_move_and_keep_their_library_row() -> AppResult<()> {
    let root = tempdir().unwrap();
    let old_dir = root.path().join("Downloads");
    let kaze = root.path().join("Kaze");
    std::fs::create_dir_all(&old_dir).unwrap();
    let ours = old_dir.join("Artist - Song (abc12345).m4a");
    let users_own = old_dir.join("My Own Song.m4a");
    std::fs::copy(FIXTURE, &ours).unwrap();
    std::fs::copy(FIXTURE, &users_own).unwrap();

    let pool = pool().await;
    let f = service(&pool, &kaze);
    let track_id = f.library.import_file(&ours).await?;
    f.repo.create_task(&completed("dl_1", "abc12345", &ours)).await?;

    f.service.list_downloads(None, 100).await?;

    let moved = kaze.join("Artist - Song (abc12345).m4a");
    assert!(moved.is_file(), "Kaze's download is in the Kaze folder");
    assert!(!ours.exists(), "and no longer in the old folder");
    assert!(users_own.is_file(), "the user's own file is never touched");
    let task = f.repo.get_task_by_id("dl_1").await?.unwrap();
    assert_eq!(task.destination_path.as_deref(), Some(moved.to_string_lossy().as_ref()));
    assert_eq!(task.status, "COMPLETED");
    let row = f.library.track_repo().find_by_path(&canonical(&moved)).await?.unwrap();
    assert_eq!(row.id, track_id, "same library row: history, likes and playlists survive");

    // Re-running (next launch) changes nothing.
    let again = service(&pool, &kaze);
    again.service.list_downloads(None, 100).await?;
    assert!(moved.is_file());
    assert_eq!(again.library.track_repo().find_by_path(&canonical(&moved)).await?.unwrap().id, track_id);
    Ok(())
}

#[tokio::test]
async fn an_existing_target_is_replaced_only_when_identical() -> AppResult<()> {
    let root = tempdir().unwrap();
    let old_dir = root.path().join("Downloads");
    let kaze = root.path().join("Kaze");
    std::fs::create_dir_all(&old_dir).unwrap();
    std::fs::create_dir_all(&kaze).unwrap();
    let same = old_dir.join("A - Same (same1234).m4a");
    let differs = old_dir.join("A - Differs (diff1234).m4a");
    std::fs::copy(FIXTURE, &same).unwrap();
    std::fs::copy(FIXTURE, kaze.join("A - Same (same1234).m4a")).unwrap();
    std::fs::write(&differs, b"old bytes").unwrap();
    std::fs::write(kaze.join("A - Differs (diff1234).m4a"), b"other bytes").unwrap();

    let pool = pool().await;
    let f = service(&pool, &kaze);
    f.repo.create_task(&completed("dl_same", "same1234", &same)).await?;
    f.repo.create_task(&completed("dl_diff", "diff1234", &differs)).await?;
    f.service.list_downloads(None, 100).await?;

    assert!(!same.exists(), "an identical copy already in place: the duplicate is removed");
    assert!(differs.is_file(), "a different file with that name is never overwritten");
    assert_eq!(std::fs::read(kaze.join("A - Differs (diff1234).m4a")).unwrap(), b"other bytes");
    Ok(())
}

fn scanned(path: &str) -> ScannedMetadata {
    serde_json::from_value(serde_json::json!({
        "file_path": path, "file_size": 1, "modified_timestamp": 1,
        "title": path, "title_is_fallback": false, "duration_secs": 100.0,
        "format": "m4a", "has_cover_art": false
    }))
    .unwrap()
}

#[tokio::test]
async fn local_and_downloads_sections_split_by_folder_even_with_like_wildcards() -> AppResult<()> {
    let root = tempdir().unwrap();
    // `_` would match any character in a LIKE pattern; the split must be exact.
    let kaze = root.path().join("Ka_ze");
    std::fs::create_dir_all(&kaze).unwrap();
    let kaze_canon = kaze.canonicalize().unwrap();
    let pool = pool().await;
    let tracks = music_player_backend::database::repositories::SqliteTrackRepository::new(pool.clone());
    let lookalike = kaze_canon.with_file_name("KaXze");
    for path in [
        kaze_canon.join("Downloaded (abc12345).m4a"),
        lookalike.join("Lookalike.flac"),
        root.path().join("Local.flac"),
    ] {
        tracks.save_scanned_track(scanned(&path.to_string_lossy())).await?;
    }

    let page = |scope: Option<&str>| {
        let scope = LibraryScope::parse(scope, &kaze);
        let tracks = &tracks;
        async move { tracks.library_page_scoped("default", 0, 200, &scope).await.unwrap() }
    };
    let (downloads, total) = page(Some("downloads")).await;
    assert_eq!(total, 1);
    assert!(downloads[0].file_path.ends_with("Downloaded (abc12345).m4a"));
    let (local, total) = page(Some("local")).await;
    assert_eq!(total, 2, "the lookalike folder is local, not a download");
    assert!(local.iter().all(|t| !t.file_path.contains("Downloaded")));
    assert_eq!(page(None).await.1, 3, "no scope keeps the old behaviour");
    Ok(())
}

#[tokio::test]
async fn a_renamed_download_stays_linked_through_its_embedded_id() -> AppResult<()> {
    let root = tempdir().unwrap();
    let kaze = root.path().join("Kaze");
    std::fs::create_dir_all(&kaze).unwrap();
    // No "(id)" in the name: only the tag identifies the recording.
    let renamed = kaze.join("my favourite.m4a");
    std::fs::copy(FIXTURE, &renamed).unwrap();
    write_tags(
        &renamed,
        &DownloadTags {
            title: "Song".into(),
            artist: "Artist".into(),
            album: None,
            year: None,
            source_id: Some("tagged99".into()),
        },
    )?;
    let pool = pool().await;
    let f = service(&pool, &kaze);
    let rows = f.service.list_downloads(None, 100).await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].source_track_id.as_deref(), Some("tagged99"));
    assert_eq!(rows[0].status, "COMPLETED");
    Ok(())
}

#[tokio::test]
async fn deleting_a_file_in_the_kaze_folder_updates_everything_at_once() -> AppResult<()> {
    let root = tempdir().unwrap();
    let kaze: PathBuf = root.path().join("Kaze");
    std::fs::create_dir_all(&kaze).unwrap();
    let file = kaze.join("Artist - Song (watch123).m4a");
    std::fs::copy(FIXTURE, &file).unwrap();
    let pool = pool().await;
    let f = service(&pool, &kaze);
    let task = f.service.list_downloads(None, 100).await?.remove(0);
    assert_eq!(task.status, "COMPLETED");
    let canon = canonical(&file);
    assert!(f.library.track_repo().find_by_path(&canon).await?.is_some());

    let mut events = f.bus.subscribe();
    f.service.start_watching();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    std::fs::remove_file(&file).unwrap();

    let changed = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if let Ok(Event::DownloadsChanged) = events.recv().await {
                break;
            }
        }
    })
    .await;
    assert!(changed.is_ok(), "the UI is told without any refresh or polling");
    assert_eq!(f.repo.get_task_by_id(&task.id).await?.unwrap().status, "MISSING");
    assert!(
        f.library.track_repo().find_by_path(&canon).await?.is_none(),
        "the deleted song leaves the Downloads list"
    );
    Ok(())
}

/// Older Kaze versions run every migration they know and reject a database that
/// has one they don't, so a new migration file breaks them. Adding one needs an
/// explicit compatibility decision: update this count only after making it.
#[test]
fn no_new_migrations_without_a_compatibility_decision() {
    let count = std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/migrations"))
        .unwrap()
        .filter(|e| e.as_ref().is_ok_and(|e| e.path().extension().is_some_and(|x| x == "sql")))
        .count();
    assert_eq!(count, 32, "a new migration makes older Kaze versions fail to start");
}

async fn scalar(pool: &sqlx::SqlitePool, sql: &str, bind: &str) -> i64 {
    sqlx::query_scalar(sql).bind(bind).fetch_one(pool).await.unwrap()
}

/// Gives a library row a like, a playlist entry, a play and statistics.
async fn add_user_data(pool: &sqlx::SqlitePool, track: &str) {
    for sql in [
        "INSERT OR IGNORE INTO playlists (id, name, created_at, updated_at) VALUES ('pl', 'Mix', 1, 1)",
        "INSERT INTO playlist_tracks (playlist_id, track_id, position, added_at) VALUES ('pl', ?, 0, 1)",
        "INSERT INTO playback_history (id, track_id, started_at, ended_at, seconds_listened, percentage_listened, completed, skipped, source) VALUES ('h1', ?, 1, 2, 30, 0.5, 0, 0, 'library')",
        "INSERT INTO track_statistics (user_id, track_id, play_count, last_played_at, manual_like) VALUES ('default', ?, 2, 50, 1)",
    ] {
        sqlx::query(sql).bind(track).execute(pool).await.unwrap();
    }
}

#[tokio::test]
async fn removing_a_download_deletes_the_file_and_keeps_the_song_online() -> AppResult<()> {
    let root = tempdir().unwrap();
    let kaze = root.path().join("Kaze");
    std::fs::create_dir_all(&kaze).unwrap();
    let file = kaze.join("Artist - Song (rem12345).m4a");
    std::fs::copy(FIXTURE, &file).unwrap();
    let pool = pool().await;
    let f = service(&pool, &kaze);
    f.service.list_downloads(None, 100).await?;
    let local = f.library.track_repo().find_by_path(&canonical(&file)).await?.unwrap().id;
    add_user_data(&pool, &local).await;
    // The online song already has its own plays: counts are merged, not replaced.
    music_player_backend::recommendations::mixes::ensure_online_track(
        &pool,
        &music_player_backend::recommendations::mixes::OnlineTrackInput {
            id: "jiosaavn:rem12345", title: "Song", artist: Some("Artist"), album: None,
            duration_secs: Some(100.0), cover_art_url: None, preview_url: None,
        },
    )
    .await?;
    sqlx::query("INSERT INTO track_statistics (user_id, track_id, play_count, last_played_at) VALUES ('default', 'jiosaavn:rem12345', 3, 10)")
        .execute(&pool).await.unwrap();

    let released = f.service.remove_download("jiosaavn:rem12345", Some("rem12345")).await?;

    assert_eq!(released, Some((local.clone(), "jiosaavn:rem12345".to_string())));
    assert!(!file.exists(), "the file is deleted");
    assert!(f.service.list_downloads(None, 100).await?.is_empty(), "the song is no longer downloaded");
    assert_eq!(scalar(&pool, "SELECT COUNT(*) FROM tracks WHERE id = ?", &local).await, 0);
    let online = "jiosaavn:rem12345";
    assert_eq!(scalar(&pool, "SELECT COUNT(*) FROM playlist_tracks WHERE track_id = ?", online).await, 1, "playlist entry kept");
    assert_eq!(scalar(&pool, "SELECT COUNT(*) FROM playback_history WHERE track_id = ?", online).await, 1, "history kept");
    assert_eq!(scalar(&pool, "SELECT play_count FROM track_statistics WHERE track_id = ?", online).await, 5, "plays merged");
    assert_eq!(scalar(&pool, "SELECT manual_like FROM track_statistics WHERE track_id = ?", online).await, 1, "like kept");
    assert_eq!(scalar(&pool, "SELECT last_played_at FROM track_statistics WHERE track_id = ?", online).await, 50);
    Ok(())
}

#[tokio::test]
async fn removing_works_from_the_local_row_and_creates_the_online_song() -> AppResult<()> {
    let root = tempdir().unwrap();
    let kaze = root.path().join("Kaze");
    std::fs::create_dir_all(&kaze).unwrap();
    let file = kaze.join("Artist - Song (loc12345).m4a");
    std::fs::copy(FIXTURE, &file).unwrap();
    let pool = pool().await;
    let f = service(&pool, &kaze);
    f.service.list_downloads(None, 100).await?;
    let local = f.library.track_repo().find_by_path(&canonical(&file)).await?.unwrap().id;
    add_user_data(&pool, &local).await;

    // The Downloads list only knows the local row.
    f.service.remove_download(&local, None).await?;

    assert!(!file.exists());
    let online = "jiosaavn:loc12345";
    assert_eq!(scalar(&pool, "SELECT COUNT(*) FROM tracks WHERE id = ? AND file_path LIKE 'online://%'", online).await, 1);
    assert_eq!(scalar(&pool, "SELECT COUNT(*) FROM playlist_tracks WHERE track_id = ?", online).await, 1);
    assert_eq!(scalar(&pool, "SELECT play_count FROM track_statistics WHERE track_id = ?", online).await, 2);
    Ok(())
}

#[tokio::test]
async fn a_file_outside_the_kaze_folder_is_never_deleted() -> AppResult<()> {
    let root = tempdir().unwrap();
    let kaze = root.path().join("Kaze");
    let elsewhere = root.path().join("Mine");
    std::fs::create_dir_all(&kaze).unwrap();
    std::fs::create_dir_all(&elsewhere).unwrap();
    let file = elsewhere.join("Artist - Song (out12345).m4a");
    std::fs::copy(FIXTURE, &file).unwrap();
    let pool = pool().await;
    let f = service(&pool, &kaze);
    f.repo.create_task(&completed("dl_out", "out12345", &file)).await?;

    assert!(f.service.remove_download("jiosaavn:out12345", None).await.is_err());
    assert!(file.is_file(), "the user's file is untouched");
    assert!(f.repo.get_task_by_id("dl_out").await?.is_some());
    Ok(())
}

#[tokio::test]
async fn a_download_deleted_outside_kaze_keeps_its_likes_on_the_online_song() -> AppResult<()> {
    let root = tempdir().unwrap();
    let kaze = root.path().join("Kaze");
    std::fs::create_dir_all(&kaze).unwrap();
    let file = kaze.join("Artist - Song (gone1234).m4a");
    std::fs::copy(FIXTURE, &file).unwrap();
    let pool = pool().await;
    let f = service(&pool, &kaze);
    let task = f.service.list_downloads(None, 100).await?.remove(0);
    let local = f.library.track_repo().find_by_path(&canonical(&file)).await?.unwrap().id;
    add_user_data(&pool, &local).await;

    std::fs::remove_file(&file).unwrap();
    f.service.sync_folder().await?;

    assert_eq!(f.repo.get_task_by_id(&task.id).await?.unwrap().status, "MISSING");
    let online = "jiosaavn:gone1234";
    assert_eq!(scalar(&pool, "SELECT COUNT(*) FROM playlist_tracks WHERE track_id = ?", online).await, 1);
    assert_eq!(scalar(&pool, "SELECT COUNT(*) FROM playback_history WHERE track_id = ?", online).await, 1);
    // Removing it now (file already gone) just clears the record.
    f.service.remove_download(online, None).await?;
    assert!(f.repo.get_task_by_id(&task.id).await?.is_none());
    Ok(())
}
