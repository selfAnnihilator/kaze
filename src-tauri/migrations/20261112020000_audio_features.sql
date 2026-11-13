-- Lightweight per-track audio descriptors computed locally (no network).
-- energy/brightness are 0..1; tempo_bpm is NULL when the beat is not confident.
-- A row with energy NULL marks a file that could not be analyzed by this
-- analyzer_version; bumping the version re-queues every track.
CREATE TABLE IF NOT EXISTS track_audio_features (
    track_id TEXT PRIMARY KEY NOT NULL,
    tempo_bpm REAL,
    energy REAL,
    brightness REAL,
    analyzer_version INTEGER NOT NULL,
    analyzed_at INTEGER NOT NULL
);
