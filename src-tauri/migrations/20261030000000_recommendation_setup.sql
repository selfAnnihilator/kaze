-- Explicit starter tastes are not listening statistics or computed preferences.
CREATE TABLE recommendation_setup (
    user_id TEXT PRIMARY KEY NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('completed','skipped')),
    artists_json TEXT NOT NULL,
    genres_json TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1,
    updated_at INTEGER NOT NULL
);
CREATE TABLE recommendation_starter_tracks (
    user_id TEXT NOT NULL,
    track_id TEXT NOT NULL,
    source TEXT NOT NULL,
    seed_value TEXT NOT NULL,
    revision INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY(user_id, track_id)
);
