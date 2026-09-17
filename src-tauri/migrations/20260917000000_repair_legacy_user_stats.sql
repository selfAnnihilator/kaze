-- Migration: 20260917000000_repair_legacy_user_stats.sql
-- Repair legacy unowned listening history, track statistics, artist/genre statistics, and user preferences

-- 1. Reassign legacy playback_history rows to the canonical user
UPDATE playback_history
SET user_id = COALESCE(
    (SELECT id FROM users WHERE id = '103cc229-6f41-4da2-abb9-beba57ef0367'),
    (SELECT id FROM users ORDER BY created_at ASC LIMIT 1)
)
WHERE (user_id = 'default' OR user_id IS NULL)
  AND EXISTS (SELECT 1 FROM users);

-- 2. Explicitly merge collisions in track_statistics
-- Collision 1: ANUBIS (7f4c5f7b-a597-4af9-9f94-a7a74137e119)
UPDATE track_statistics
SET
    play_count = play_count + COALESCE((SELECT play_count FROM track_statistics WHERE user_id = 'default' AND track_id = '7f4c5f7b-a597-4af9-9f94-a7a74137e119'), 0),
    total_time_listened = total_time_listened + COALESCE((SELECT total_time_listened FROM track_statistics WHERE user_id = 'default' AND track_id = '7f4c5f7b-a597-4af9-9f94-a7a74137e119'), 0.0),
    completion_count = completion_count + COALESCE((SELECT completion_count FROM track_statistics WHERE user_id = 'default' AND track_id = '7f4c5f7b-a597-4af9-9f94-a7a74137e119'), 0),
    skip_count = skip_count + COALESCE((SELECT skip_count FROM track_statistics WHERE user_id = 'default' AND track_id = '7f4c5f7b-a597-4af9-9f94-a7a74137e119'), 0),
    last_played_at = MAX(COALESCE(last_played_at, 0), COALESCE((SELECT last_played_at FROM track_statistics WHERE user_id = 'default' AND track_id = '7f4c5f7b-a597-4af9-9f94-a7a74137e119'), 0)),
    playlist_addition_count = playlist_addition_count + COALESCE((SELECT playlist_addition_count FROM track_statistics WHERE user_id = 'default' AND track_id = '7f4c5f7b-a597-4af9-9f94-a7a74137e119'), 0),
    manual_like = CASE WHEN manual_like != 0 THEN manual_like ELSE COALESCE((SELECT manual_like FROM track_statistics WHERE user_id = 'default' AND track_id = '7f4c5f7b-a597-4af9-9f94-a7a74137e119'), 0) END
WHERE user_id = COALESCE(
    (SELECT id FROM users WHERE id = '103cc229-6f41-4da2-abb9-beba57ef0367'),
    (SELECT id FROM users ORDER BY created_at ASC LIMIT 1)
) AND track_id = '7f4c5f7b-a597-4af9-9f94-a7a74137e119'
  AND EXISTS (SELECT 1 FROM track_statistics WHERE user_id = 'default' AND track_id = '7f4c5f7b-a597-4af9-9f94-a7a74137e119');

DELETE FROM track_statistics
WHERE user_id = 'default' AND track_id = '7f4c5f7b-a597-4af9-9f94-a7a74137e119'
  AND EXISTS (
      SELECT 1 FROM track_statistics
      WHERE user_id = COALESCE(
          (SELECT id FROM users WHERE id = '103cc229-6f41-4da2-abb9-beba57ef0367'),
          (SELECT id FROM users ORDER BY created_at ASC LIMIT 1)
      ) AND track_id = '7f4c5f7b-a597-4af9-9f94-a7a74137e119'
  );

-- Collision 2: HEAVENLY JUMPSTYLE (dca9bb9a-4e0c-4437-a54b-2f793280e698)
UPDATE track_statistics
SET
    play_count = play_count + COALESCE((SELECT play_count FROM track_statistics WHERE user_id = 'default' AND track_id = 'dca9bb9a-4e0c-4437-a54b-2f793280e698'), 0),
    total_time_listened = total_time_listened + COALESCE((SELECT total_time_listened FROM track_statistics WHERE user_id = 'default' AND track_id = 'dca9bb9a-4e0c-4437-a54b-2f793280e698'), 0.0),
    completion_count = completion_count + COALESCE((SELECT completion_count FROM track_statistics WHERE user_id = 'default' AND track_id = 'dca9bb9a-4e0c-4437-a54b-2f793280e698'), 0),
    skip_count = skip_count + COALESCE((SELECT skip_count FROM track_statistics WHERE user_id = 'default' AND track_id = 'dca9bb9a-4e0c-4437-a54b-2f793280e698'), 0),
    last_played_at = MAX(COALESCE(last_played_at, 0), COALESCE((SELECT last_played_at FROM track_statistics WHERE user_id = 'default' AND track_id = 'dca9bb9a-4e0c-4437-a54b-2f793280e698'), 0)),
    playlist_addition_count = playlist_addition_count + COALESCE((SELECT playlist_addition_count FROM track_statistics WHERE user_id = 'default' AND track_id = 'dca9bb9a-4e0c-4437-a54b-2f793280e698'), 0),
    manual_like = CASE WHEN manual_like != 0 THEN manual_like ELSE COALESCE((SELECT manual_like FROM track_statistics WHERE user_id = 'default' AND track_id = 'dca9bb9a-4e0c-4437-a54b-2f793280e698'), 0) END
WHERE user_id = COALESCE(
    (SELECT id FROM users WHERE id = '103cc229-6f41-4da2-abb9-beba57ef0367'),
    (SELECT id FROM users ORDER BY created_at ASC LIMIT 1)
) AND track_id = 'dca9bb9a-4e0c-4437-a54b-2f793280e698'
  AND EXISTS (SELECT 1 FROM track_statistics WHERE user_id = 'default' AND track_id = 'dca9bb9a-4e0c-4437-a54b-2f793280e698');

DELETE FROM track_statistics
WHERE user_id = 'default' AND track_id = 'dca9bb9a-4e0c-4437-a54b-2f793280e698'
  AND EXISTS (
      SELECT 1 FROM track_statistics
      WHERE user_id = COALESCE(
          (SELECT id FROM users WHERE id = '103cc229-6f41-4da2-abb9-beba57ef0367'),
          (SELECT id FROM users ORDER BY created_at ASC LIMIT 1)
      ) AND track_id = 'dca9bb9a-4e0c-4437-a54b-2f793280e698'
  );

-- Reassign remaining non-conflicting track_statistics rows
UPDATE track_statistics
SET user_id = COALESCE(
    (SELECT id FROM users WHERE id = '103cc229-6f41-4da2-abb9-beba57ef0367'),
    (SELECT id FROM users ORDER BY created_at ASC LIMIT 1)
)
WHERE (user_id = 'default' OR user_id IS NULL)
  AND EXISTS (SELECT 1 FROM users);

-- 3. Reassign non-conflicting artist_statistics
UPDATE artist_statistics
SET user_id = COALESCE(
    (SELECT id FROM users WHERE id = '103cc229-6f41-4da2-abb9-beba57ef0367'),
    (SELECT id FROM users ORDER BY created_at ASC LIMIT 1)
)
WHERE (user_id = 'default' OR user_id IS NULL)
  AND EXISTS (SELECT 1 FROM users);

-- 4. Reassign non-conflicting genre_statistics
UPDATE genre_statistics
SET user_id = COALESCE(
    (SELECT id FROM users WHERE id = '103cc229-6f41-4da2-abb9-beba57ef0367'),
    (SELECT id FROM users ORDER BY created_at ASC LIMIT 1)
)
WHERE (user_id = 'default' OR user_id IS NULL)
  AND EXISTS (SELECT 1 FROM users);

-- 5. Reassign non-conflicting user_preferences
UPDATE user_preferences
SET user_id = COALESCE(
    (SELECT id FROM users WHERE id = '103cc229-6f41-4da2-abb9-beba57ef0367'),
    (SELECT id FROM users ORDER BY created_at ASC LIMIT 1)
)
WHERE (user_id = 'default' OR user_id IS NULL)
  AND EXISTS (SELECT 1 FROM users);
