-- Preserve the original Kaze alias identity separately from its selected recording.
ALTER TABLE songs ADD COLUMN provider TEXT;
ALTER TABLE songs ADD COLUMN provider_id TEXT;
ALTER TABLE songs ADD COLUMN online_source_json TEXT;
