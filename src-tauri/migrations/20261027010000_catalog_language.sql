-- Language is distinct from genre; JioSaavn supplies it for songs and playlists.
ALTER TABLE external_tracks ADD COLUMN language TEXT;
