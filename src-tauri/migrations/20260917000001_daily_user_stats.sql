-- Migration: Add daily_user_stats table for cloud-synced daily aggregate listening history

CREATE TABLE IF NOT EXISTS daily_user_stats (
    user_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    stat_date TEXT NOT NULL,
    listening_seconds REAL NOT NULL DEFAULT 0.0,
    play_count INTEGER NOT NULL DEFAULT 0,
    completion_count INTEGER NOT NULL DEFAULT 0,
    skip_count INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, device_id, stat_date)
);

CREATE INDEX IF NOT EXISTS idx_daily_user_stats_user_date ON daily_user_stats(user_id, stat_date);
CREATE INDEX IF NOT EXISTS idx_daily_user_stats_user_updated ON daily_user_stats(user_id, updated_at);

-- Idempotent backfill from existing local playback_history
INSERT OR IGNORE INTO daily_user_stats (
    user_id,
    device_id,
    stat_date,
    listening_seconds,
    play_count,
    completion_count,
    skip_count,
    updated_at
)
SELECT
    COALESCE(h.user_id, 'default') as user_id,
    COALESCE((SELECT value FROM application_settings WHERE key = 'device_id'), 'legacy_device') as device_id,
    strftime('%Y-%m-%d', datetime(h.started_at, 'unixepoch', 'localtime')) as stat_date,
    COALESCE(SUM(h.seconds_listened), 0.0) as listening_seconds,
    COALESCE(SUM(CASE WHEN h.seconds_listened >= 30.0 OR h.completed = 1 THEN 1 ELSE 0 END), 0) as play_count,
    COALESCE(SUM(h.completed), 0) as completion_count,
    COALESCE(SUM(h.skipped), 0) as skip_count,
    COALESCE(MAX(h.ended_at), MAX(h.started_at), strftime('%s', 'now')) as updated_at
FROM playback_history h
GROUP BY user_id, stat_date;
