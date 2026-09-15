-- Custom playlist names are unique per account, allowing every account to own
-- its own undeletable Liked Songs playlist. Smart-mix names remain global.
DROP INDEX IF EXISTS idx_playlists_unique_name;

CREATE UNIQUE INDEX IF NOT EXISTS idx_user_playlists_unique_name
ON playlists(user_id, name COLLATE BINARY)
WHERE is_smart_mix = 0;

CREATE UNIQUE INDEX IF NOT EXISTS idx_smart_playlists_unique_name
ON playlists(name COLLATE BINARY)
WHERE is_smart_mix = 1;
