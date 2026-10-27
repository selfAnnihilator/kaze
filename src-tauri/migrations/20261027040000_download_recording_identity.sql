-- Preserve the recording behind each download independently of its title/filename.
ALTER TABLE download_tasks ADD COLUMN source_track_id TEXT;
CREATE INDEX idx_download_recording ON download_tasks(provider, source_track_id);
UPDATE download_tasks SET source_track_id=(SELECT provider_id FROM external_tracks e
 WHERE e.provider='jiosaavn' AND substr(download_tasks.filename, -length(' (' || e.provider_id || ').m4a'))=' (' || e.provider_id || ').m4a' LIMIT 1)
 WHERE provider='jiosaavn';
