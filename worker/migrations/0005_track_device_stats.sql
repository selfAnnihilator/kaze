-- Migration: Add track_device_stats table for Cloudflare D1
CREATE TABLE IF NOT EXISTS track_device_stats (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id TEXT NOT NULL,
    track_id TEXT NOT NULL,
    play_count INTEGER NOT NULL DEFAULT 0,
    total_seconds REAL NOT NULL DEFAULT 0.0,
    completion_count INTEGER NOT NULL DEFAULT 0,
    skip_count INTEGER NOT NULL DEFAULT 0,
    last_played_at INTEGER,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, device_id, track_id)
);

CREATE INDEX IF NOT EXISTS idx_track_device_stats_user ON track_device_stats(user_id);
CREATE INDEX IF NOT EXISTS idx_track_device_stats_user_track ON track_device_stats(user_id, track_id);
