-- Explicit choices outlive counters, cache expiry, library rescans and stale sync snapshots.
CREATE TABLE recommendation_feedback (
    user_id TEXT NOT NULL,
    song_key TEXT NOT NULL,
    track_id TEXT NOT NULL,
    source_id TEXT,
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    manual_like INTEGER NOT NULL CHECK(manual_like IN (-1,0,1)),
    updated_at INTEGER NOT NULL,
    PRIMARY KEY(user_id,song_key)
);
