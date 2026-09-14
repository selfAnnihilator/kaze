-- Cloudflare D1 Database Schema for SoundFlow Music Player
-- Online user storage and synchronization

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY NOT NULL,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

-- Sessions store cryptographic SHA-256 hash of token with hybrid idle/absolute expiry
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT UNIQUE NOT NULL,
    device_id TEXT NOT NULL,
    device_name TEXT NOT NULL,
    client_version TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    last_used_at INTEGER NOT NULL,
    idle_expires_at INTEGER NOT NULL,
    absolute_expires_at INTEGER NOT NULL,
    revoked_at INTEGER,
    revoked_reason TEXT
);

CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_token_hash ON sessions(token_hash);
CREATE INDEX IF NOT EXISTS idx_sessions_idle ON sessions(idle_expires_at);
CREATE INDEX IF NOT EXISTS idx_sessions_absolute ON sessions(absolute_expires_at);
CREATE INDEX IF NOT EXISTS idx_sessions_revoked ON sessions(revoked_at);

-- Rate limiting tracking table for auth abuse mitigation
CREATE TABLE IF NOT EXISTS auth_rate_limits (
    key TEXT PRIMARY KEY NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 1,
    reset_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_rate_limits_reset ON auth_rate_limits(reset_at);

CREATE TABLE IF NOT EXISTS songs (
    id TEXT NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    artist TEXT NOT NULL DEFAULT '',
    album TEXT NOT NULL DEFAULT '',
    duration_secs REAL NOT NULL DEFAULT 0.0,
    cover_art_url TEXT,
    preview_url TEXT,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_songs_user ON songs(user_id);

CREATE TABLE IF NOT EXISTS playlists (
    id TEXT NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    is_smart_mix INTEGER NOT NULL DEFAULT 0,
    mix_type TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_playlists_user ON playlists(user_id);

CREATE TABLE IF NOT EXISTS playlist_songs (
    playlist_id TEXT NOT NULL,
    song_id TEXT NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    position INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (playlist_id, song_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_playlist_songs_user ON playlist_songs(user_id);

CREATE TABLE IF NOT EXISTS song_stats (
    song_id TEXT NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    play_count INTEGER NOT NULL DEFAULT 0,
    total_seconds REAL NOT NULL DEFAULT 0.0,
    completion_count INTEGER NOT NULL DEFAULT 0,
    skip_count INTEGER NOT NULL DEFAULT 0,
    manual_like INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (song_id, user_id)
);

CREATE TABLE IF NOT EXISTS user_stats (
    user_id TEXT PRIMARY KEY NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    total_seconds REAL NOT NULL DEFAULT 0.0,
    top_songs_json TEXT NOT NULL DEFAULT '[]',
    top_artists_json TEXT NOT NULL DEFAULT '[]',
    top_days_json TEXT NOT NULL DEFAULT '[]',
    yearly_archives_json TEXT NOT NULL DEFAULT '[]',
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS user_settings (
    user_id TEXT PRIMARY KEY NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    settings_json TEXT NOT NULL DEFAULT '{}',
    updated_at INTEGER NOT NULL
);
