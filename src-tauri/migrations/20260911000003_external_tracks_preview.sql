-- Add preview_url and genre to external_tracks for discovery previews and genre-based taste matching
ALTER TABLE external_tracks ADD COLUMN preview_url TEXT;
ALTER TABLE external_tracks ADD COLUMN genre TEXT;
