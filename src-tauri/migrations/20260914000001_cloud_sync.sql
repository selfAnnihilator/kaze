-- Migration: Cloud Sync and Session Metadata Cache
-- Note: Sensitive bearer tokens are stored in secure OS credentials (keyring), NOT in SQLite.
CREATE TABLE IF NOT EXISTS cloud_sessions (
    user_id TEXT PRIMARY KEY NOT NULL,
    username TEXT NOT NULL,
    expires_at INTEGER NOT NULL,
    worker_url TEXT NOT NULL,
    synced_at INTEGER,
    created_at INTEGER NOT NULL
);
