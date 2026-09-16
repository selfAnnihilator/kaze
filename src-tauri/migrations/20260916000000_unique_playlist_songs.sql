-- Keep the earliest occurrence of each title/artist within a playlist.
-- A single song may have multiple track IDs after imports from different sources.
DELETE FROM playlist_tracks
WHERE rowid IN (
    SELECT row_id FROM (
        SELECT rowid AS row_id,
               ROW_NUMBER() OVER (
                   PARTITION BY playlist_id, track_id ORDER BY position, rowid
               ) AS occurrence
        FROM playlist_tracks
    ) WHERE occurrence > 1
);

DELETE FROM playlist_tracks
WHERE rowid IN (
    SELECT row_id FROM (
        SELECT pt.rowid AS row_id,
               ROW_NUMBER() OVER (
                   PARTITION BY pt.playlist_id,
                                LOWER(TRIM(COALESCE(ext.title, t.title))),
                                LOWER(TRIM(COALESCE(a.name, ext.artist, 'Unknown Artist')))
                   ORDER BY pt.position, pt.rowid
               ) AS occurrence
        FROM playlist_tracks pt
        JOIN tracks t ON t.id = pt.track_id
        LEFT JOIN artists a ON a.id = t.artist_id
        LEFT JOIN external_tracks ext ON ext.id = t.id
    ) WHERE occurrence > 1
);

-- Also enforce the basic identity at the storage layer.
CREATE UNIQUE INDEX IF NOT EXISTS idx_playlist_tracks_unique_song_id
    ON playlist_tracks(playlist_id, track_id);

-- Reject a second song with the same title and artist, even when its track ID differs.
CREATE TRIGGER IF NOT EXISTS playlist_song_unique_insert
BEFORE INSERT ON playlist_tracks
WHEN EXISTS (
    SELECT 1
    FROM tracks incoming
    LEFT JOIN artists incoming_artist ON incoming_artist.id = incoming.artist_id
    LEFT JOIN external_tracks incoming_ext ON incoming_ext.id = incoming.id
    JOIN playlist_tracks existing ON existing.playlist_id = NEW.playlist_id
    JOIN tracks current ON current.id = existing.track_id
    LEFT JOIN artists current_artist ON current_artist.id = current.artist_id
    LEFT JOIN external_tracks current_ext ON current_ext.id = current.id
    WHERE incoming.id = NEW.track_id
      AND LOWER(TRIM(COALESCE(incoming_ext.title, incoming.title))) = LOWER(TRIM(COALESCE(current_ext.title, current.title)))
      AND LOWER(TRIM(COALESCE(incoming_artist.name, incoming_ext.artist, 'Unknown Artist'))) =
          LOWER(TRIM(COALESCE(current_artist.name, current_ext.artist, 'Unknown Artist')))
)
BEGIN
    SELECT RAISE(IGNORE);
END;

CREATE TRIGGER IF NOT EXISTS playlist_song_unique_update
BEFORE UPDATE OF track_id ON playlist_tracks
WHEN EXISTS (
    SELECT 1
    FROM tracks incoming
    LEFT JOIN artists incoming_artist ON incoming_artist.id = incoming.artist_id
    LEFT JOIN external_tracks incoming_ext ON incoming_ext.id = incoming.id
    JOIN playlist_tracks existing ON existing.playlist_id = NEW.playlist_id AND existing.rowid != OLD.rowid
    JOIN tracks current ON current.id = existing.track_id
    LEFT JOIN artists current_artist ON current_artist.id = current.artist_id
    LEFT JOIN external_tracks current_ext ON current_ext.id = current.id
    WHERE incoming.id = NEW.track_id
      AND LOWER(TRIM(COALESCE(incoming_ext.title, incoming.title))) = LOWER(TRIM(COALESCE(current_ext.title, current.title)))
      AND LOWER(TRIM(COALESCE(incoming_artist.name, incoming_ext.artist, 'Unknown Artist'))) =
          LOWER(TRIM(COALESCE(current_artist.name, current_ext.artist, 'Unknown Artist')))
)
BEGIN
    SELECT RAISE(IGNORE);
END;
