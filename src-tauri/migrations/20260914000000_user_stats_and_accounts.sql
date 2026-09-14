-- Migration: User accounts and Yearly stats archive
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY NOT NULL,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

ALTER TABLE playlists ADD COLUMN user_id TEXT DEFAULT 'default';
ALTER TABLE playback_history ADD COLUMN user_id TEXT DEFAULT 'default';

CREATE INDEX IF NOT EXISTS idx_history_user_started ON playback_history(user_id, started_at);
CREATE INDEX IF NOT EXISTS idx_playlists_user ON playlists(user_id);

CREATE TABLE IF NOT EXISTS yearly_stats_archive (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL DEFAULT 'default',
    year INTEGER NOT NULL,
    total_seconds REAL NOT NULL DEFAULT 0.0,
    top_songs_json TEXT NOT NULL DEFAULT '[]',
    top_artists_json TEXT NOT NULL DEFAULT '[]',
    archived_at INTEGER NOT NULL,
    UNIQUE(user_id, year)
);
CREATE INDEX IF NOT EXISTS idx_yearly_stats_user_year ON yearly_stats_archive(user_id, year);
