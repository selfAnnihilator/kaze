-- Automatic sync triggers for tracks_fts full-text search index

CREATE TRIGGER IF NOT EXISTS tracks_ai AFTER INSERT ON tracks BEGIN
  INSERT INTO tracks_fts(rowid, track_id, title, artist, album, genre)
  VALUES (
    new.rowid,
    new.id,
    new.title,
    coalesce((SELECT name FROM artists WHERE id = new.artist_id), 'Unknown Artist'),
    coalesce((SELECT title FROM albums WHERE id = new.album_id), 'Unknown Album'),
    coalesce((SELECT name FROM genres WHERE id = new.genre_id), '')
  );
END;

CREATE TRIGGER IF NOT EXISTS tracks_ad AFTER DELETE ON tracks BEGIN
  INSERT INTO tracks_fts(tracks_fts, rowid, track_id, title, artist, album, genre)
  VALUES (
    'delete',
    old.rowid,
    old.id,
    old.title,
    coalesce((SELECT name FROM artists WHERE id = old.artist_id), 'Unknown Artist'),
    coalesce((SELECT title FROM albums WHERE id = old.album_id), 'Unknown Album'),
    coalesce((SELECT name FROM genres WHERE id = old.genre_id), '')
  );
END;

CREATE TRIGGER IF NOT EXISTS tracks_au AFTER UPDATE ON tracks BEGIN
  INSERT INTO tracks_fts(tracks_fts, rowid, track_id, title, artist, album, genre)
  VALUES (
    'delete',
    old.rowid,
    old.id,
    old.title,
    coalesce((SELECT name FROM artists WHERE id = old.artist_id), 'Unknown Artist'),
    coalesce((SELECT title FROM albums WHERE id = old.album_id), 'Unknown Album'),
    coalesce((SELECT name FROM genres WHERE id = old.genre_id), '')
  );
  INSERT INTO tracks_fts(rowid, track_id, title, artist, album, genre)
  VALUES (
    new.rowid,
    new.id,
    new.title,
    coalesce((SELECT name FROM artists WHERE id = new.artist_id), 'Unknown Artist'),
    coalesce((SELECT title FROM albums WHERE id = new.album_id), 'Unknown Album'),
    coalesce((SELECT name FROM genres WHERE id = new.genre_id), '')
  );
END;
