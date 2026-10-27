-- A durable link from Kaze's saved identity to the exact provider version displayed.
CREATE TABLE online_track_sources (
    track_id TEXT PRIMARY KEY REFERENCES external_tracks(id) ON DELETE CASCADE,
    provider_id TEXT NOT NULL,
    metadata_json TEXT NOT NULL,
    linked_at INTEGER NOT NULL
);
