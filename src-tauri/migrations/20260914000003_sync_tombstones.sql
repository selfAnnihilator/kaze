-- Migration: Cloud Sync Tombstones for Explicit Deletions
-- Distinguishes between "record does not exist locally" and "user explicitly deleted this record".
-- Tombstones are sent during sync to Cloudflare D1 and cleared once pushed.

CREATE TABLE IF NOT EXISTS sync_tombstones (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    entity_type TEXT NOT NULL, -- 'playlist' or 'playlist_song'
    entity_id TEXT NOT NULL,   -- playlist_id, or 'playlist_id:song_id'
    deleted_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sync_tombstones_user ON sync_tombstones(user_id);
