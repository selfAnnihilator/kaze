-- Shared editorial data; personal feedback is filtered on each read.
CREATE TABLE discovery_feed_cache (
    feed_key TEXT PRIMARY KEY,
    payload TEXT NOT NULL,
    fetched_at INTEGER NOT NULL
);
