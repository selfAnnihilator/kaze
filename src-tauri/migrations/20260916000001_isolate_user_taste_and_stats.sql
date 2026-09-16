-- Migration: Isolate user taste profiles, statistics, preferences, and smart mixes per user account

-- 0. Clean up any orphaned historical records where parent rows were removed
DELETE FROM track_statistics WHERE track_id NOT IN (SELECT id FROM tracks);
DELETE FROM playback_history WHERE track_id NOT IN (SELECT id FROM tracks);
DELETE FROM artist_statistics WHERE artist_id NOT IN (SELECT id FROM artists);
DELETE FROM genre_statistics WHERE genre_id NOT IN (SELECT id FROM genres);

-- 1. Migrate track_statistics to composite PRIMARY KEY (user_id, track_id)
CREATE TABLE IF NOT EXISTS track_statistics_new (
    user_id TEXT NOT NULL DEFAULT 'default',
    track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    play_count INTEGER NOT NULL DEFAULT 0,
    total_time_listened REAL NOT NULL DEFAULT 0.0,
    completion_count INTEGER NOT NULL DEFAULT 0,
    skip_count INTEGER NOT NULL DEFAULT 0,
    last_played_at INTEGER,
    manual_like INTEGER NOT NULL DEFAULT 0,
    playlist_addition_count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, track_id)
);

INSERT OR IGNORE INTO track_statistics_new (
    user_id, track_id, play_count, total_time_listened,
    completion_count, skip_count, last_played_at, manual_like, playlist_addition_count
)
SELECT
    COALESCE((SELECT id FROM users ORDER BY created_at ASC LIMIT 1), 'default'),
    ts.track_id, ts.play_count, ts.total_time_listened,
    ts.completion_count, ts.skip_count, ts.last_played_at, ts.manual_like, ts.playlist_addition_count
FROM track_statistics ts
JOIN tracks t ON t.id = ts.track_id;

DROP TABLE track_statistics;
ALTER TABLE track_statistics_new RENAME TO track_statistics;

CREATE INDEX IF NOT EXISTS idx_track_stats_user_track ON track_statistics(user_id, track_id);
CREATE INDEX IF NOT EXISTS idx_track_stats_user_like ON track_statistics(user_id, manual_like);

-- 2. Migrate user_preferences to composite PRIMARY KEY (user_id, entity_type, entity_id)
CREATE TABLE IF NOT EXISTS user_preferences_new (
    user_id TEXT NOT NULL DEFAULT 'default',
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    short_term_affinity REAL NOT NULL DEFAULT 0.0,
    long_term_affinity REAL NOT NULL DEFAULT 0.0,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, entity_type, entity_id)
);

INSERT OR IGNORE INTO user_preferences_new (
    user_id, entity_type, entity_id, short_term_affinity, long_term_affinity, updated_at
)
SELECT
    COALESCE((SELECT id FROM users ORDER BY created_at ASC LIMIT 1), 'default'),
    entity_type, entity_id, short_term_affinity, long_term_affinity, updated_at
FROM user_preferences;

DROP TABLE user_preferences;
ALTER TABLE user_preferences_new RENAME TO user_preferences;

CREATE INDEX IF NOT EXISTS idx_user_preferences_user_type ON user_preferences(user_id, entity_type);

-- 3. Add user_id to recommendation_sessions
ALTER TABLE recommendation_sessions ADD COLUMN user_id TEXT NOT NULL DEFAULT 'default';
UPDATE recommendation_sessions
SET user_id = COALESCE((SELECT id FROM users ORDER BY created_at ASC LIMIT 1), 'default')
WHERE user_id = 'default';
CREATE INDEX IF NOT EXISTS idx_rec_sessions_user_type ON recommendation_sessions(user_id, session_type);

-- 4. Scope smart playlists uniquely per user instead of globally
DROP INDEX IF EXISTS idx_smart_playlists_unique_name;
CREATE UNIQUE INDEX IF NOT EXISTS idx_smart_playlists_user_name
ON playlists(user_id, name COLLATE BINARY)
WHERE is_smart_mix = 1;

-- 5. Migrate artist_statistics and genre_statistics to composite (user_id, id)
CREATE TABLE IF NOT EXISTS artist_statistics_new (
    user_id TEXT NOT NULL DEFAULT 'default',
    artist_id TEXT NOT NULL REFERENCES artists(id) ON DELETE CASCADE,
    play_count INTEGER NOT NULL DEFAULT 0,
    total_time_listened REAL NOT NULL DEFAULT 0.0,
    last_played_at INTEGER,
    affinity_score REAL NOT NULL DEFAULT 0.0,
    PRIMARY KEY (user_id, artist_id)
);

INSERT OR IGNORE INTO artist_statistics_new (
    user_id, artist_id, play_count, total_time_listened, last_played_at, affinity_score
)
SELECT
    COALESCE((SELECT id FROM users ORDER BY created_at ASC LIMIT 1), 'default'),
    ast.artist_id, ast.play_count, ast.total_time_listened, ast.last_played_at, ast.affinity_score
FROM artist_statistics ast
JOIN artists a ON a.id = ast.artist_id;

DROP TABLE artist_statistics;
ALTER TABLE artist_statistics_new RENAME TO artist_statistics;

CREATE TABLE IF NOT EXISTS genre_statistics_new (
    user_id TEXT NOT NULL DEFAULT 'default',
    genre_id TEXT NOT NULL REFERENCES genres(id) ON DELETE CASCADE,
    play_count INTEGER NOT NULL DEFAULT 0,
    total_time_listened REAL NOT NULL DEFAULT 0.0,
    last_played_at INTEGER,
    affinity_score REAL NOT NULL DEFAULT 0.0,
    PRIMARY KEY (user_id, genre_id)
);

INSERT OR IGNORE INTO genre_statistics_new (
    user_id, genre_id, play_count, total_time_listened, last_played_at, affinity_score
)
SELECT
    COALESCE((SELECT id FROM users ORDER BY created_at ASC LIMIT 1), 'default'),
    gs.genre_id, gs.play_count, gs.total_time_listened, gs.last_played_at, gs.affinity_score
FROM genre_statistics gs
JOIN genres g ON g.id = gs.genre_id;

DROP TABLE genre_statistics;
ALTER TABLE genre_statistics_new RENAME TO genre_statistics;
