-- Verified artist-owned alternatives for automatic selection; saved sources remain immutable.
CREATE TABLE catalog_artist_editions (
    source_id TEXT PRIMARY KEY,
    preferred_source_id TEXT NOT NULL,
    checked_at INTEGER NOT NULL
);
