-- Durable journal for the currently active playback session.
-- Written at session start and meaningful transitions; deleted on clean finalization.
-- On startup, any unfinished record indicates a crash-interrupted session.
CREATE TABLE IF NOT EXISTS active_playback_journal (
    session_id   TEXT PRIMARY KEY,
    user_id      TEXT NOT NULL,
    track_id     TEXT NOT NULL,
    device_id    TEXT NOT NULL,
    stat_date    TEXT NOT NULL,
    started_at   INTEGER NOT NULL,
    listened_secs REAL NOT NULL DEFAULT 0.0,
    duration_secs REAL NOT NULL DEFAULT 0.0,
    source       TEXT NOT NULL DEFAULT 'library',
    updated_at   INTEGER NOT NULL
);
