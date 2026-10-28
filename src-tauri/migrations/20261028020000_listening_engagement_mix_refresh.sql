-- Rebuild only automatic recommendations under the listening-time policy.
-- Keep playlist IDs, manual collections, history and all cumulative counters.
UPDATE playlists SET expires_at=0 WHERE is_smart_mix=1;
