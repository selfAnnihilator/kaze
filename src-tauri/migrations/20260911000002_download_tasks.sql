-- Download tasks table for Soulseek and other download providers
CREATE TABLE IF NOT EXISTS download_tasks (
    id TEXT PRIMARY KEY NOT NULL,
    provider TEXT NOT NULL,
    provider_task_id TEXT,
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    album TEXT,
    filename TEXT NOT NULL,
    destination_path TEXT,
    file_size INTEGER,
    bytes_downloaded INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'QUEUED',
    error_message TEXT,
    wishlist_id TEXT REFERENCES wishlist(id) ON DELETE SET NULL,
    created_at INTEGER NOT NULL,
    completed_at INTEGER
);

CREATE INDEX IF NOT EXISTS idx_downloads_status ON download_tasks(status);
CREATE INDEX IF NOT EXISTS idx_downloads_wishlist_id ON download_tasks(wishlist_id);
