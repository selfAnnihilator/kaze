# ADR 0015: JioSaavn playback and reversible local-library enrichment

Date: 2026-10-26. Status: experimental, on `codex/jiosaavn-playback-experiment`.

## Decision

Use a Rust adapter for JioSaavn's unofficial web endpoints as the default online search source. The search UI exposes Alternative sources for the existing iTunes/download-network search. Home scoring, Flow generation and chart feeds retain their existing logic.

Catalogue identity is `provider=jiosaavn`, `provider_id=<original song ID>` and a stable Kaze ID `jiosaavn:<song ID>`. Search results are persisted before selection. Registration and cloud ingestion preserve established identities, including non-prefixed legacy external IDs. Exact JioSaavn selections bypass frontend local metadata aliases and backend filename-based substitutions. Existing recommendation/chart identities may resolve through JioSaavn only after a unique conservative match; otherwise they retain the previous resolver. Validated associations include their source metadata signature.

All playback remains in `StreamPlaybackManager` and `PlaybackService`, with the existing queue, cancellation generation, history writer, statistics and event stream. Provider-specific resolution/audio cache keys use `full:v3:jiosaavn:<song ID>`. Failed media URLs are refreshed once by exact ID. Pre-resolution fetches metadata only. Completed cache entries remain playable with the provider disabled; uncached playback reports the disabled source. There is no browser audio path, JioSaavn login, Python runtime, separate server or permanent JioSaavn downloader.

## Library provenance

Migration `20261026000000_provider_enrichment.sql` adds source snapshots, enrichment evidence and provider associations without replacing track IDs or account-owned data. `ScannedMetadata.title_is_fallback` distinguishes a filename from an embedded title. Original scanned values are stored separately before applying provider fills. Only absent title/artist/album/year/track-number fields are filled; measured duration and technical properties remain authoritative. Audio files are never rewritten.

Matching retains recording labels and requires normalized title, the complete artist string and duration within three seconds. Missing artists require title, album and duration. Multiple distinct candidate IDs, missing evidence or conflicting versions are rejected. JioSaavn is tried first, followed by enabled MusicBrainz/Spotify sources. The old unsafe first-result enrichment path now delegates to this same conservative mechanism.

Prepared metadata is persisted as `applying`, merged through the canonical track repository, then marked `matched`. Interrupted merges resume without another provider request. Rescans overlay only remaining gaps and invalidate fills when recording identity changes. Removal restores the latest source snapshots and turns automatic filling off until the user re-enables it or requests a manual fill.

Track changes and source snapshots commit together. Background application compares the latest source snapshot in its update, so an older network result cannot replace a newer rescan's embedded tags. A failed comparison leaves the prepared merge retryable.

Validated artwork is stored in the local enrichment cache, behind embedded and folder artwork. Existing usable album artwork is preserved. The normal artwork loader serves it to rows, queues, playlists and player surfaces; MPRIS uses the cached path. An album may use enriched track artwork only with confirmed provider album identity. Local artwork paths and media URLs are not added to cloud song records.

## Scheduling and performance

One cancellable worker processes library gaps after startup/scans or manual requests. Failed/no-match attempts survive restart with finite retry windows. Playback buffering interrupts enrichment requests. Work resumes after playback startup settles. Metadata notifications are batched, and unsuccessful attempts do not publish visual updates. Playback progress remains in the existing leaf subscription; source choice and settings contain no playback timers.

Online input/search retains a 200 ms debounce. Existing result virtualization and memoized shell components remain. Artwork cache invalidation notifies artwork leaves so the same mounted track can gain or lose art without adding playback ticks to App.

## Provenance and verification

The reference implementation is [cyberboysumanjay/JioSaavnAPI](https://github.com/cyberboysumanjay/JioSaavnAPI/tree/67ecd4d92186a72f65603d2947bd05a57e148f90), pinned to `67ecd4d92186a72f65603d2947bd05a57e148f90`. Endpoint selection and the ordinary media URL envelope are adapted into Rust. Its MIT copyright/license is preserved in [JioSaavnAPI-LICENSE](../third-party/JioSaavnAPI-LICENSE). The separate DRM media field and preview URL are not used for full-track playback.

See [experiment guide](../jiosaavn-experiment.md) for acceptance evidence, repeatable isolated testing and limitations. The original requested scope is retained in [the supplied plan](../jiosaavn-plan.md).
