-- Artist-to-artist similarity: Last.fm "fans also like" (global, user_id='')
-- plus co-occurrence in the user's own playlists and listening sessions.
CREATE TABLE IF NOT EXISTS artist_similarity (
    user_id TEXT NOT NULL DEFAULT '',
    source_norm TEXT NOT NULL,
    target_norm TEXT NOT NULL,
    target_display TEXT NOT NULL,
    score REAL NOT NULL,
    source TEXT NOT NULL CHECK (source IN ('lastfm', 'cooc')),
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, source_norm, target_norm, source)
);
CREATE INDEX IF NOT EXISTS idx_artist_similarity_source ON artist_similarity(source_norm);

-- TTL and negative cache for Last.fm lookups (status: ok | missing | error).
CREATE TABLE IF NOT EXISTS lastfm_artist_fetch (
    artist_norm TEXT PRIMARY KEY,
    fetched_at INTEGER NOT NULL,
    status TEXT NOT NULL
);
