-- Migration: Add Cloudinary avatar metadata fields to users
ALTER TABLE users ADD COLUMN avatar_public_id TEXT;
ALTER TABLE users ADD COLUMN avatar_url TEXT;
ALTER TABLE users ADD COLUMN avatar_version INTEGER;
