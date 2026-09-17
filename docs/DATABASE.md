# Database Architecture & Schema Specification

The application uses an embedded **SQLite** database managed via **`sqlx`**. SQLite provides zero-latency, local-first persistence with ACID transactions, atomic writes, and zero external daemon requirements.

---

## 1. Migration Management

Migrations are stored as ordered SQL files within `src-tauri/migrations/` and embedded directly into the compiled binary via `sqlx::migrate!()`. Upon startup, `CoreProcessor` runs pending migrations automatically before initializing services.

---

## 2. Core Entities & Schema Definition

### 2.1 Library Structure

```sql
-- Registered root directories for library indexing
CREATE TABLE IF NOT EXISTS library_folders (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    added_at INTEGER NOT NULL,
    last_scanned_at INTEGER,
    enabled INTEGER NOT NULL DEFAULT 1
);

-- Artists
CREATE TABLE IF NOT EXISTS artists (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    normalized_name TEXT NOT NULL,
    musicbrainz_id TEXT,
    bio TEXT,
    image_url TEXT,
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_artists_normalized ON artists(normalized_name);

-- Albums
CREATE TABLE IF NOT EXISTS albums (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    normalized_title TEXT NOT NULL,
    artist_id TEXT REFERENCES artists(id) ON DELETE SET NULL,
    album_artist TEXT,
    release_year INTEGER,
    total_tracks INTEGER,
    cover_art_path TEXT,
    musicbrainz_id TEXT,
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_albums_artist ON albums(artist_id);
CREATE INDEX IF NOT EXISTS idx_albums_normalized ON albums(normalized_title);

-- Genres
CREATE TABLE IF NOT EXISTS genres (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    normalized_name TEXT NOT NULL
);

-- Tracks
CREATE TABLE IF NOT EXISTS tracks (
    id TEXT PRIMARY KEY,
    file_path TEXT NOT NULL UNIQUE,
    file_size INTEGER NOT NULL,
    modified_timestamp INTEGER NOT NULL,
    file_hash TEXT,
    title TEXT NOT NULL,
    normalized_title TEXT NOT NULL,
    artist_id TEXT REFERENCES artists(id) ON DELETE SET NULL,
    album_id TEXT REFERENCES albums(id) ON DELETE SET NULL,
    genre_id TEXT REFERENCES genres(id) ON DELETE SET NULL,
    track_number INTEGER,
    disc_number INTEGER DEFAULT 1,
    year INTEGER,
    duration_secs REAL NOT NULL,
    bitrate INTEGER,
    sample_rate INTEGER,
    format TEXT NOT NULL,
    has_cover_art INTEGER NOT NULL DEFAULT 0,
    musicbrainz_track_id TEXT,
    spotify_id TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_tracks_artist ON tracks(artist_id);
CREATE INDEX IF NOT EXISTS idx_tracks_album ON tracks(album_id);
CREATE INDEX IF NOT EXISTS idx_tracks_genre ON tracks(genre_id);
CREATE INDEX IF NOT EXISTS idx_tracks_filepath ON tracks(file_path);
CREATE INDEX IF NOT EXISTS idx_tracks_normalized ON tracks(normalized_title);

-- Full-Text Search (FTS5) for instant library query
CREATE VIRTUAL TABLE IF NOT EXISTS tracks_fts USING fts5(
    track_id UNINDEXED,
    title,
    artist,
    album,
    genre,
    content='tracks',
    content_rowid='rowid'
);
```

### 2.2 Playlists

```sql
CREATE TABLE IF NOT EXISTS playlists (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    is_smart_mix INTEGER NOT NULL DEFAULT 0,
    mix_type TEXT,                -- 'daily', 'genre', 'artist', 'on_repeat', 'rediscover'
    generation_reason TEXT,
    expires_at INTEGER,           -- For temporary/generated playlists
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS playlist_tracks (
    playlist_id TEXT NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    added_at INTEGER NOT NULL,
    PRIMARY KEY (playlist_id, position)
);
CREATE INDEX IF NOT EXISTS idx_playlist_tracks_track ON playlist_tracks(track_id);
```

### 2.3 History & Granular Statistics

```sql
-- Playback History Log
CREATE TABLE IF NOT EXISTS playback_history (
    id TEXT PRIMARY KEY,
    track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    started_at INTEGER NOT NULL,
    ended_at INTEGER NOT NULL,
    seconds_listened REAL NOT NULL,
    percentage_listened REAL NOT NULL,
    completed INTEGER NOT NULL,   -- 1 if reached end of track
    skipped INTEGER NOT NULL,     -- 1 if skipped before meaningful play threshold
    source TEXT NOT NULL,         -- 'library', 'playlist', 'smart_mix', 'search'
    playlist_id TEXT,
    recommendation_session_id TEXT
);
CREATE INDEX IF NOT EXISTS idx_history_track ON playback_history(track_id);
CREATE INDEX IF NOT EXISTS idx_history_started ON playback_history(started_at);

-- Pre-aggregated Track Statistics (Updated incrementally on meaningful plays, cloud-synced)
CREATE TABLE IF NOT EXISTS track_statistics (
    user_id TEXT NOT NULL DEFAULT 'default',
    track_id TEXT NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    play_count INTEGER NOT NULL DEFAULT 0,
    total_time_listened REAL NOT NULL DEFAULT 0.0,
    completion_count INTEGER NOT NULL DEFAULT 0,
    skip_count INTEGER NOT NULL DEFAULT 0,
    last_played_at INTEGER,
    manual_like INTEGER NOT NULL DEFAULT 0,    -- 1 for liked, -1 for disliked, 0 neutral
    playlist_addition_count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, track_id)
);

-- Compact Daily User Statistics (Aggregated by day and device, cloud-synced)
CREATE TABLE IF NOT EXISTS daily_user_stats (
    user_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    stat_date TEXT NOT NULL,                  -- Local calendar day 'YYYY-MM-DD'
    listening_seconds REAL NOT NULL DEFAULT 0.0,
    play_count INTEGER NOT NULL DEFAULT 0,
    completion_count INTEGER NOT NULL DEFAULT 0,
    skip_count INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (user_id, device_id, stat_date)
);
CREATE INDEX IF NOT EXISTS idx_daily_user_stats_date ON daily_user_stats(user_id, stat_date);
CREATE INDEX IF NOT EXISTS idx_daily_user_stats_updated ON daily_user_stats(user_id, updated_at);

-- Artist Aggregated Statistics
CREATE TABLE IF NOT EXISTS artist_statistics (
    artist_id TEXT PRIMARY KEY REFERENCES artists(id) ON DELETE CASCADE,
    play_count INTEGER NOT NULL DEFAULT 0,
    total_time_listened REAL NOT NULL DEFAULT 0.0,
    last_played_at INTEGER,
    affinity_score REAL NOT NULL DEFAULT 0.0
);

-- Genre Aggregated Statistics
CREATE TABLE IF NOT EXISTS genre_statistics (
    genre_id TEXT PRIMARY KEY REFERENCES genres(id) ON DELETE CASCADE,
    play_count INTEGER NOT NULL DEFAULT 0,
    total_time_listened REAL NOT NULL DEFAULT 0.0,
    last_played_at INTEGER,
    affinity_score REAL NOT NULL DEFAULT 0.0
);
```

#### Three-Layer Statistics Architecture
1. **`track_statistics`** (Cumulative / All-Time, Cloud-Synced):
   - Authoritative for **Lifetime Listening**, **All-Time Top Songs**, and **All-Time Top Artists**.
   - Preserves complete listening aggregates across re-installs and device migrations.
2. **`daily_user_stats`** (Date-Bucketed Aggregates, Cloud-Synced):
   - Authoritative for **Today**, **This Week**, **This Month**, **This Year**, **Daily Graph**, **Active Days**, and **Top Days**.
   - Keyed by `(user_id, device_id, stat_date)`. Total for a user/date is `SUM(listening_seconds)` across devices.
   - Snapshot upserts (`ON CONFLICT ... DO UPDATE SET ... WHERE excluded.updated_at >= updated_at`) prevent double-counting on repeated syncs.
3. **`playback_history`** (Granular Chronological Log, Local-Only):
   - Stores raw session events with precise start/end timestamps and percentage completed.
   - Strictly local; never uploaded to the cloud or synced to preserve user privacy and cloud storage bandwidth.

### 2.4 Taste Profile, Recommendations & Discovery

```sql
-- User Taste Profile Signals (Short-term & Long-term)
CREATE TABLE IF NOT EXISTS user_preferences (
    entity_type TEXT NOT NULL,     -- 'artist', 'genre', 'era', 'track'
    entity_id TEXT NOT NULL,
    short_term_affinity REAL NOT NULL DEFAULT 0.0,
    long_term_affinity REAL NOT NULL DEFAULT 0.0,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (entity_type, entity_id)
);

-- Recommendation Sessions and Recorded Recommendations
CREATE TABLE IF NOT EXISTS recommendation_sessions (
    id TEXT PRIMARY KEY,
    generated_at INTEGER NOT NULL,
    session_type TEXT NOT NULL     -- 'local_mix', 'discovery', 'home_feed'
);

CREATE TABLE IF NOT EXISTS recommendations (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES recommendation_sessions(id) ON DELETE CASCADE,
    track_id TEXT REFERENCES tracks(id) ON DELETE CASCADE,
    external_track_id TEXT,
    score REAL NOT NULL,
    reasons_json TEXT NOT NULL,    -- Detailed breakdown of contributing factors
    is_discovery INTEGER NOT NULL DEFAULT 0
);

-- External Discovery Tracks (Discovered tracks not in local library)
CREATE TABLE IF NOT EXISTS external_tracks (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,        -- 'spotify', 'musicbrainz'
    provider_id TEXT NOT NULL,
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    album TEXT,
    duration_secs REAL,
    cover_art_url TEXT,
    match_status TEXT NOT NULL DEFAULT 'NOT_FOUND', -- 'EXACT_MATCH', 'LIKELY_MATCH', 'POSSIBLE_MATCH', 'NOT_FOUND'
    matched_local_track_id TEXT REFERENCES tracks(id) ON DELETE SET NULL,
    created_at INTEGER NOT NULL,
    UNIQUE(provider, provider_id)
);

-- Download Wishlist
CREATE TABLE IF NOT EXISTS wishlist (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    artist TEXT NOT NULL,
    album TEXT,
    external_track_id TEXT REFERENCES external_tracks(id) ON DELETE SET NULL,
    status TEXT NOT NULL DEFAULT 'WANT', -- 'WANT', 'IGNORE', 'ALREADY_OWN', 'DOWNLOADED'
    notes TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Application Settings
CREATE TABLE IF NOT EXISTS application_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
```

---

## 3. Performance & Pragmatic Configuration

* **WAL Mode (Write-Ahead Logging)**: Enabled on connection pool setup (`PRAGMA journal_mode = WAL;`) for maximum concurrency between readers and writers.
* **Synchronous Normal**: `PRAGMA synchronous = NORMAL;` to maintain ACID compliance without unnecessary filesystem sync penalties.
* **Foreign Keys**: Enforced via `PRAGMA foreign_keys = ON;`.
* **Busy Timeout**: Set to 5000ms (`PRAGMA busy_timeout = 5000;`) to prevent locking issues under multi-threaded SQLite operations.
