# ADR 0010: External Metadata Providers, Rate Limiting, and Local Artwork Caching

## Context
A local-first music player must operate 100% offline using locally embedded audio tags. However, personal music libraries frequently have incomplete or missing tags (e.g. missing track numbers, release dates, artist biographies, high-resolution album artwork). To enhance local data without introducing mandatory cloud accounts or subscriptions, the player needs a modular external provider architecture connecting to free, open services (MusicBrainz, Cover Art Archive) and optional discovery services (Spotify).

External integrations introduce several challenges:
1. Rate limiting: MusicBrainz enforces a strict 1 request per second rule; exceeding this triggers IP throttling or blocking.
2. Network failure & offline behavior: The player must never hang, crash, or delay audio playback when the network is unavailable.
3. Disk caching: Album artwork must be downloaded once and cached permanently on local disk to prevent duplicate HTTP bandwidth usage.
4. Optional authentication: Spotify integration must support the Client Credentials flow and gracefully report unavailable when unconfigured.

## Options Considered
1. **Direct Synchronous HTTP Calls in UI or Audio Thread**: Rejected because blocking network calls freeze the event loop, cause UI stutters, and violate the offline-first design.
2. **Third-Party Commercial Metadata SDK (e.g. Gracenote/Last.fm paid)**: Rejected because it violates the project's zero-cost, subscription-free philosophy.
3. **Modular Async `MetadataProvider` Trait with Coordinator and Local Caching**:
   - Define a pure `MetadataProvider` trait (`search_track`, `search_artist`, `search_album`, `fetch_cover_art`).
   - Implement `MusicBrainzProvider` with an async leaky-bucket throttler enforcing 1 req/sec.
   - Implement `CoverArtArchiveProvider` with local filesystem disk caching (`cache_dir/artwork/{mbid}.jpg`).
   - Implement `SpotifyProvider` with Client Credentials flow, token caching, and graceful offline degradation.
   - Implement `ProviderCoordinator` to orchestrate multi-provider enrichment, persist IDs into SQLite (`tracks.musicbrainz_track_id`, `tracks.spotify_id`, `albums.cover_art_path`), and broadcast `Event::ProviderStatusChanged`.

## Decision
Adopt the **Modular Async `MetadataProvider` Trait with Provider Coordinator, Leaky-Bucket Rate Limiting, and Local Disk Caching**.

## Reasoning
* **Respects Free Services**: Strictly complies with the MusicBrainz rate limit policy (1 req/s) using an async throttler.
* **Resilient & Offline-First**: All providers degrade gracefully. If offline, unconfigured, or failing, requests return empty/None cleanly without throwing errors or interrupting local playback.
* **Bandwidth Efficient**: Artwork is stored locally on disk and served from cache on all subsequent queries.
* **Zero Cost**: Built entirely on FOSS endpoints (MusicBrainz, Cover Art Archive) and standard developer credentials.

## Consequences
* Tracks missing tags or cover art can be enriched on-demand or in the background via `Command::TriggerMetadataRefresh`.
* External IDs (MBIDs, Spotify IDs) enable fuzzy matching and external discovery in Phase 7.
