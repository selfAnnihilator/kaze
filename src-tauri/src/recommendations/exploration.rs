//! Auto-tuned exploration: compares how the user responds to Flow's
//! exploration picks versus its familiar picks and nudges the explore share.
//! The familiarity dial stays authoritative; the bias is a small, bounded shift.
use crate::core::AppResult;
use sqlx::SqlitePool;

pub const EXPLORE: &str = "explore";
pub const FAMILIAR: &str = "familiar";
/// Window of history the tuner learns from.
pub const WINDOW_SECS: i64 = 30 * 86_400;
/// Minimum listens of each slot before the tuner says anything.
pub const MIN_LISTENS: i64 = 30;
pub const MAX_BIAS: f64 = 0.12;
/// Bias at or above which an exploration pick says "Exploring a bit more".
pub const NOTICE_BIAS: f64 = 0.03;
const CACHE_SECS: i64 = 86_400;

fn cache_key(user_id: &str) -> String {
    format!("flow_explore_bias:{user_id}")
}

/// Remembers which slot each Flow pick filled, so the finished listen can be
/// attributed. Re-enqueueing a track overwrites its slot.
pub async fn record_slots(
    pool: &SqlitePool,
    user_id: &str,
    slots: &[(String, &'static str)],
    now: i64,
) -> AppResult<()> {
    if slots.is_empty() {
        return Ok(());
    }
    let mut tx = pool.begin().await?;
    for (track_id, slot) in slots {
        sqlx::query(
            "INSERT INTO flow_slot_assignments (user_id, track_id, slot, assigned_at)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(user_id, track_id) DO UPDATE SET
                 slot = excluded.slot, assigned_at = excluded.assigned_at",
        )
        .bind(user_id)
        .bind(track_id)
        .bind(slot)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Copies the pick's slot onto a finished Flow listen.
pub async fn stamp_listen(
    pool: &SqlitePool,
    history_id: &str,
    user_id: &str,
    track_id: &str,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE playback_history SET rec_slot = (
             SELECT slot FROM flow_slot_assignments WHERE user_id = ? AND track_id = ?
         ) WHERE id = ?",
    )
    .bind(user_id)
    .bind(track_id)
    .bind(history_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Success rate per slot: qualified (non-skipped, meaningful) listens count +1,
/// early skips count −1, everything else 0; mapped to 0..1.
fn success(listens: i64, completed: i64, skipped: i64) -> f64 {
    if listens <= 0 {
        return 0.5;
    }
    let net = (completed - skipped) as f64 / listens as f64;
    ((net + 1.0) / 2.0).clamp(0.0, 1.0)
}

/// Pure tuner: positive when exploration picks land better than familiar ones.
pub fn bias_from_counts(explore: (i64, i64, i64), familiar: (i64, i64, i64)) -> f64 {
    if explore.0 < MIN_LISTENS || familiar.0 < MIN_LISTENS {
        return 0.0;
    }
    let s_explore = success(explore.0, explore.1, explore.2);
    let s_familiar = success(familiar.0, familiar.1, familiar.2);
    ((s_explore - s_familiar) * 0.5).clamp(-MAX_BIAS, MAX_BIAS)
}

/// Computes the bias from the last 30 days of Flow listens (uncached).
pub async fn compute_bias(pool: &SqlitePool, user_id: &str, now: i64) -> AppResult<f64> {
    let rows: Vec<(String, i64, i64, i64)> = sqlx::query_as(
        "SELECT rec_slot, COUNT(*),
                COALESCE(SUM(CASE WHEN skipped = 0 AND (completed = 1 OR percentage_listened >= 0.5) THEN 1 ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN skipped = 1 THEN 1 ELSE 0 END), 0)
         FROM playback_history
         WHERE COALESCE(user_id, 'default') = ? AND rec_slot IS NOT NULL AND started_at >= ?
         GROUP BY rec_slot",
    )
    .bind(user_id)
    .bind(now - WINDOW_SECS)
    .fetch_all(pool)
    .await?;
    let counts = |slot: &str| {
        rows.iter()
            .find(|r| r.0 == slot)
            .map_or((0, 0, 0), |r| (r.1, r.2, r.3))
    };
    Ok(bias_from_counts(counts(EXPLORE), counts(FAMILIAR)))
}

/// Cached daily in `flow_explore_bias:{user}` as "bias@computed_at".
pub async fn explore_bias(pool: &SqlitePool, user_id: &str, now: i64) -> AppResult<f64> {
    let key = cache_key(user_id);
    let cached: Option<(String,)> =
        sqlx::query_as("SELECT value FROM application_settings WHERE key = ?")
            .bind(&key)
            .fetch_optional(pool)
            .await?;
    if let Some((value,)) = cached {
        if let Some((bias, at)) = value.split_once('@') {
            if let (Ok(bias), Ok(at)) = (bias.parse::<f64>(), at.parse::<i64>()) {
                if now - at < CACHE_SECS && now >= at && bias.is_finite() {
                    return Ok(bias.clamp(-MAX_BIAS, MAX_BIAS));
                }
            }
        }
    }
    let bias = compute_bias(pool, user_id, now).await?;
    sqlx::query(
        "INSERT INTO application_settings (key, value, updated_at) VALUES (?, ?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(&key)
    .bind(format!("{bias}@{now}"))
    .bind(now)
    .execute(pool)
    .await?;
    Ok(bias)
}
