-- Per-device cumulative track statistics.
-- Lifetime stats = SUM of all device contributions for (user_id, track_id).
-- The "_baseline" device_id holds all pre-migration cumulative data.
CREATE TABLE IF NOT EXISTS track_device_statistics (
    user_id      TEXT NOT NULL,
    device_id    TEXT NOT NULL,
    track_id     TEXT NOT NULL,
    play_count        INTEGER NOT NULL DEFAULT 0,
    total_time_listened REAL NOT NULL DEFAULT 0.0,
    completion_count  INTEGER NOT NULL DEFAULT 0,
    skip_count        INTEGER NOT NULL DEFAULT 0,
    last_played_at    INTEGER,
    updated_at        INTEGER NOT NULL,
    PRIMARY KEY (user_id, device_id, track_id)
);

CREATE INDEX IF NOT EXISTS idx_tds_user_track
    ON track_device_statistics(user_id, track_id);

-- Migrate existing track_statistics into the baseline device bucket.
-- INSERT OR IGNORE: if a row already exists (idempotent re-run), skip it.
INSERT OR IGNORE INTO track_device_statistics
    (user_id, device_id, track_id, play_count, total_time_listened,
     completion_count, skip_count, last_played_at, updated_at)
SELECT
    user_id,
    '_baseline',
    track_id,
    play_count,
    total_time_listened,
    completion_count,
    skip_count,
    last_played_at,
    COALESCE(last_played_at, strftime('%s','now'))
FROM track_statistics;
