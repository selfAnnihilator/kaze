//! Keeps profiles usable across Kaze versions (ADR 0031).
//!
//! - Migrations are additive, so a build may open a profile migrated by a newer build:
//!   unknown applied migrations are ignored instead of refusing to start.
//! - Before this build applies new migrations to an existing profile, it copies the
//!   database to `<file>.bak-before-<version>`, so an older Kaze can be restored.
use crate::core::error::{AppError, AppResult};
use sqlx::migrate::Migrator;
use sqlx::SqlitePool;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::info;

/// The app's migrations, tolerant of migrations applied by newer versions.
pub fn migrator() -> Migrator {
    let mut migrator = sqlx::migrate!("./migrations");
    migrator.set_ignore_missing(true);
    migrator
}

/// Versions this build would apply. Empty for a brand-new database (nothing to back up).
pub async fn pending_migrations(pool: &SqlitePool, migrator: &Migrator) -> AppResult<Vec<i64>> {
    let has_table: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations')",
    )
    .fetch_one(pool)
    .await?;
    if !has_table {
        return Ok(Vec::new());
    }
    let applied: HashSet<i64> = sqlx::query_scalar("SELECT version FROM _sqlx_migrations WHERE success = 1")
        .fetch_all(pool)
        .await?
        .into_iter()
        .collect();
    if applied.is_empty() {
        return Ok(Vec::new());
    }
    Ok(migrator
        .iter()
        .filter(|m| !m.migration_type.is_down_migration() && !applied.contains(&m.version))
        .map(|m| m.version)
        .collect())
}

/// Where the pre-migration copy of `db_path` goes for this build.
pub fn backup_path(db_path: &Path) -> PathBuf {
    let name = db_path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "kaze.db".into());
    db_path.with_file_name(format!("{name}.bak-before-{}", env!("CARGO_PKG_VERSION")))
}

/// Copies the database before new migrations run. Returns the backup path when one was written.
pub async fn backup_before_migrate(pool: &SqlitePool, db_path: &Path, migrator: &Migrator) -> AppResult<Option<PathBuf>> {
    let pending = pending_migrations(pool, migrator).await?;
    if pending.is_empty() {
        return Ok(None);
    }
    let target = backup_path(db_path);
    if target.exists() {
        return Ok(None);
    }
    // VACUUM INTO writes a consistent copy, including pages still in the WAL.
    let literal = target.to_string_lossy().replace('\'', "''");
    sqlx::query(&format!("VACUUM INTO '{literal}'"))
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("Backup before migration failed: {e}")))?;
    info!(backup = %target.display(), pending = ?pending, "Backed up the database before migrating");
    Ok(Some(target))
}
