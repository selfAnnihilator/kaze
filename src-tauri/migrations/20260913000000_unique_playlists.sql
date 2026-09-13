-- Deduplicate any existing duplicate playlists (keeping the oldest)
DELETE FROM playlist_tracks WHERE playlist_id IN (
    SELECT p2.id FROM playlists p1
    JOIN playlists p2 ON p1.name = p2.name AND p1.is_smart_mix = p2.is_smart_mix AND p1.id != p2.id AND p1.created_at <= p2.created_at
);
DELETE FROM playlists WHERE id IN (
    SELECT p2.id FROM playlists p1
    JOIN playlists p2 ON p1.name = p2.name AND p1.is_smart_mix = p2.is_smart_mix AND p1.id != p2.id AND p1.created_at <= p2.created_at
);

-- Ensure no two playlists can have the exact same name (case-sensitive COLLATE BINARY)
CREATE UNIQUE INDEX IF NOT EXISTS idx_playlists_unique_name ON playlists(name COLLATE BINARY, is_smart_mix);
