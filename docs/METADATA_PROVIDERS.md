# External Metadata Providers Architecture

## 1. Overview
While the music player is strictly local-first and functions 100% offline, it can optionally connect to external metadata and discovery providers over the internet to:
1. Enrich local tracks missing release dates, track numbers, artist bios, or genres.
2. Fetch high-resolution cover artwork for albums without embedded images.
3. Obtain canonical IDs (MusicBrainz MBID, Spotify ID) for cross-referencing.
4. Discover related artists and external tracks for recommendation expansion (Phase 7).

All external providers are modular, free of paid subscriptions, and degrade gracefully when offline or unconfigured.

---

## 2. Provider Abstraction (`providers/mod.rs`)

All metadata backends implement the `MetadataProvider` trait:

```rust
#[async_trait]
pub trait MetadataProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn is_available(&self) -> bool;

    async fn search_track(
        &self,
        title: &str,
        artist: &str,
        album: Option<&str>,
    ) -> AppResult<Vec<ExternalTrackMetadata>>;

    async fn search_artist(
        &self,
        name: &str,
    ) -> AppResult<Option<ExternalArtistMetadata>>;

    async fn search_album(
        &self,
        title: &str,
        artist: &str,
    ) -> AppResult<Option<ExternalAlbumMetadata>>;

    async fn fetch_cover_art(
        &self,
        release_id: &str,
    ) -> AppResult<Option<Vec<u8>>>;
}
```

---

## 3. Supported Providers

### 3.1 MusicBrainz Provider (`providers/musicbrainz.rs`)
* **Endpoint**: `https://musicbrainz.org/ws/2/`
* **Data**: Recordings, Releases, Artists, Tags/Genres.
* **FOSS & Free**: Operates without API keys.
* **Rate Limiting**: Strictly complies with the MusicBrainz policy of 1 request per second via an async mutex-protected leaky-bucket throttler.
* **User-Agent**: `MusicPlayer/0.1.0 ( https://github.com/example/music_player )`.

### 3.2 Cover Art Archive Provider (`providers/cover_art_archive.rs`)
* **Endpoint**: `https://coverartarchive.org/`
* **Data**: Album front cover artwork mapped to MusicBrainz release MBIDs.
* **Disk Caching**: Images are downloaded once and cached locally in `{data_dir}/artwork/{release_id}.jpg`.
* **Zero Authentication**: Completely open and free.

### 3.3 Spotify Web API Provider (`providers/spotify.rs`)
* **Endpoint**: `https://api.spotify.com/v1/`
* **Auth**: OAuth 2.0 Client Credentials flow (`https://accounts.spotify.com/api/token`).
* **Optional Credentials**: Users may provide their own free Spotify developer `client_id` and `client_secret` in application settings.
* **Token Caching**: Automatically caches access tokens and refreshes them upon expiration.
* **Graceful Degradation**: If unconfigured or unauthorized, provider reports `is_available() = false` without logging errors or blocking local playback.

---

## 4. Provider Coordinator (`providers/coordinator.rs`)
The `ProviderCoordinator` manages provider lifecycles, caching, and background enrichment:
1. **Fallback Strategy**: Searches MusicBrainz first for canonical metadata, then uses Cover Art Archive for artwork, and optionally queries Spotify if enabled.
2. **Metadata Enrichment**: Matches local tracks missing tags and persists `musicbrainz_track_id`, `spotify_id`, and `cover_art_path` to SQLite.
3. **Event Notification**: Publishes `Event::ProviderStatusChanged` whenever network status or credentials change.
