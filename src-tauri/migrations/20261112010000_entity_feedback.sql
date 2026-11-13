-- Artist and genre level negative feedback ("not this artist", "less like this").
CREATE TABLE IF NOT EXISTS recommendation_entity_feedback (
    user_id TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('artist', 'genre')),
    key_norm TEXT NOT NULL,
    display TEXT NOT NULL,
    level TEXT NOT NULL CHECK (level IN ('block', 'less')),
    created_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, kind, key_norm)
);
