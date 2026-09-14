-- Migration: Enhanced Session Management Metadata
-- Stores non-secret session metadata locally in SQLite.
-- Note: Sensitive raw bearer tokens are stored in secure OS credentials (keyring), NEVER in SQLite.

CREATE TABLE IF NOT EXISTS cloud_sessions_v2 (
    user_id TEXT PRIMARY KEY NOT NULL,
    username TEXT NOT NULL,
    session_id TEXT NOT NULL DEFAULT '',
    device_id TEXT NOT NULL DEFAULT '',
    device_name TEXT NOT NULL DEFAULT '',
    idle_expires_at INTEGER NOT NULL DEFAULT 0,
    absolute_expires_at INTEGER NOT NULL DEFAULT 0,
    last_cloud_validation_at INTEGER,
    worker_url TEXT NOT NULL,
    synced_at INTEGER,
    authenticated_before INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL
);

INSERT OR IGNORE INTO cloud_sessions_v2 (user_id, username, idle_expires_at, absolute_expires_at, worker_url, synced_at, created_at)
SELECT user_id, username, expires_at, expires_at + 60 * 86400, worker_url, synced_at, created_at
FROM cloud_sessions;

DROP TABLE IF EXISTS cloud_sessions;
ALTER TABLE cloud_sessions_v2 RENAME TO cloud_sessions;
