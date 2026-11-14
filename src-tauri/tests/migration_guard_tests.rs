//! ADR 0031: a profile migrated by a newer Kaze still opens, and an existing profile is
//! backed up before this build migrates it. Temporary directories only.
use music_player_backend::database::init_db_pool;
use music_player_backend::database::migration_guard::backup_path;
use tempfile::tempdir;

/// A migration whose SQL is idempotent (CREATE … IF NOT EXISTS), so it can be re-applied.
const IDEMPOTENT: i64 = 20261112000000;

#[tokio::test]
async fn a_profile_from_a_newer_kaze_still_opens() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("kaze.db");
    let pool = init_db_pool(&db).await.unwrap();
    sqlx::query(
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
         VALUES (99990101000000, 'from a newer kaze', 1, x'00', 0)",
    )
    .execute(&pool)
    .await
    .unwrap();
    pool.close().await;

    let reopened = init_db_pool(&db).await.expect("unknown applied migrations are tolerated");
    let tracks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tracks").fetch_one(&reopened).await.unwrap();
    assert_eq!(tracks, 0);
}

#[tokio::test]
async fn an_existing_profile_is_backed_up_before_new_migrations() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("kaze.db");
    let pool = init_db_pool(&db).await.unwrap();
    // Pretend an older Kaze made this profile: one of our migrations is not applied yet.
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version = ?").bind(IDEMPOTENT).execute(&pool).await.unwrap();
    pool.close().await;

    let reopened = init_db_pool(&db).await.unwrap();
    let backup = backup_path(&db);
    assert!(backup.exists(), "backup written before migrating");

    let copy = init_copy(&backup).await;
    let applied: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE version = ?")
        .bind(IDEMPOTENT)
        .fetch_one(&copy)
        .await
        .unwrap();
    assert_eq!(applied, 0, "the backup is the database as it was before migrating");
    let now: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE version = ?")
        .bind(IDEMPOTENT)
        .fetch_one(&reopened)
        .await
        .unwrap();
    assert_eq!(now, 1, "the live database is migrated");
}

#[tokio::test]
async fn new_or_current_profiles_are_not_backed_up() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("kaze.db");
    init_db_pool(&db).await.unwrap().close().await;
    init_db_pool(&db).await.unwrap().close().await;
    assert!(!backup_path(&db).exists(), "nothing pending, no copy");
}

/// Opens a backup read-only, without running migrations on it.
async fn init_copy(path: &std::path::Path) -> sqlx::SqlitePool {
    use std::str::FromStr;
    let options = sqlx::sqlite::SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))
        .unwrap()
        .read_only(true);
    sqlx::SqlitePool::connect_with(options).await.unwrap()
}
