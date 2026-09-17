-- Migration: Add daily_user_stats table for Cloudflare D1
CREATE TABLE IF NOT EXISTS daily_user_stats (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id TEXT NOT NULL,
    stat_date TEXT NOT NULL,
    listening_seconds REAL NOT NULL DEFAULT 0.0,
    play_count INTEGER NOT NULL DEFAULT 0,
    completion_count INTEGER NOT NULL DEFAULT 0,
    skip_count INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, device_id, stat_date)
);

CREATE INDEX IF NOT EXISTS idx_daily_user_stats_user ON daily_user_stats(user_id);
CREATE INDEX IF NOT EXISTS idx_daily_user_stats_user_date ON daily_user_stats(user_id, stat_date);
