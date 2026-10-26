-- Source tags remain independent of reversible provider fills. Track IDs are unchanged.
CREATE TABLE track_source_metadata (
    track_id TEXT PRIMARY KEY NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    metadata_json TEXT NOT NULL
);
CREATE TABLE track_enrichment (
    track_id TEXT PRIMARY KEY NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    provider_id TEXT,
    metadata_json TEXT,
    evidence_json TEXT,
    cover_art_path TEXT,
    state TEXT NOT NULL DEFAULT 'pending',
    retry_after INTEGER NOT NULL DEFAULT 0,
    attempted_at INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX enrichment_retry ON track_enrichment(state, retry_after);
CREATE TABLE provider_track_matches (
    external_track_id TEXT PRIMARY KEY NOT NULL REFERENCES external_tracks(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    evidence_json TEXT NOT NULL
);
