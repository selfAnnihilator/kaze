# JioSaavn online playback and offline-library enrichment for Kaze

## Summary

Add JioSaavn as Kaze’s primary source for online search and playback, with its integration implemented inside the Rust backend. Users retain Kaze’s existing account experience; no JioSaavn login, Python installation, or separate server is required.

Use the same provider to fill missing artwork and metadata for local songs. Preserve existing tags, audio files, and independent offline playback. Keep current recommendation logic and chart feeds.

## Provider and playback integration

- Adapt the relevant search, song-detail, album-detail, and media-resolution behavior from [JioSaavnAPI](https://github.com/cyberboysumanjay/JioSaavnAPI) into a dedicated Rust provider. Pin the reference revision and preserve applicable license attribution.
- **First acceptance gate:** verify live search, song details, returned artwork, full-track decoding, and seeking through Kaze’s Rust player. Test several tracks and languages. If upstream requests or full playback fail, report the exact blocker before integrating the UI; do not treat previews as successful full-track playback.
- Expose provider operations for search, metadata lookup, and resolving playable media by JioSaavn song ID. Normalize upstream responses into Kaze’s existing metadata types, with explicit errors for unavailable content, rate limits, malformed responses, and network failures.
- Extend `SearchOnlineMusic` with an optional provider selection, defaulting to JioSaavn. Keep existing providers available through an explicit alternative-source choice. Preserve current Home recommendations, Flow candidate generation, and chart feeds.
- Persist `provider = jiosaavn` and the original song ID throughout search, queueing, likes, playlists, history, and cloud synchronization. Fix online-track registration so it cannot replace an established provider with generic `online` identity. Existing saved tracks remain readable.
- Resolve selected JioSaavn results by their exact song ID. For existing recommendation/chart items, attempt a strict JioSaavn match; retain the existing resolver when no unambiguous match exists. Do not silently substitute a different recording.
- Feed resolved media into the existing `StreamPlaybackManager` and `PlaybackService`. Preserve one Rust audio engine, queue, state machine, history path, and event stream; introduce no browser audio player.
- Include provider identity and song ID in resolution/audio cache keys. Refresh an expired or rejected media URL once, then return a useful error. Keep temporary URLs out of cloud records.
- Preserve current bounded audio caching, cancellation, pre-resolution, and active-track protection. Pre-resolution must not start audio downloads. Permanent JioSaavn downloads are outside this version.

## Offline metadata and artwork

- Extend the existing metadata coordinator with JioSaavn as the first enrichment source, followed by existing enabled metadata providers.
- Run enrichment asynchronously after library scans and through a manual “Fill missing artwork and details” action. Process existing library gaps in a resumable background queue; never delay scanning results or playback.
- Fill missing title, artist, album, year, and track number only when the provider actually supplies them. Treat a filename-derived title as a fallback, not an embedded tag. Keep measured duration, codec, bitrate, and sample rate authoritative.
- Automatically accept only a unique candidate with matching normalized title and artist, compatible recording/version labels, and duration within three seconds. If the artist is missing, require matching title, album, and duration. With insufficient evidence or multiple candidates, leave the song unchanged.
- Preserve distinctions such as live, remix, acoustic, cover, and remastered recordings. Do not reuse the existing matcher’s unconditional removal of parenthetical labels for enrichment.
- Store enrichment separately with provider ID, match evidence, and field provenance; merge it into Kaze’s canonical library records only where source tags are absent. On rescan, new embedded tags take precedence and remaining enrichment survives. Do not modify audio-file tags.
- Cache validated artwork locally and use the existing artwork-loading path. Preserve embedded and existing usable artwork. Associate downloaded artwork with the matched track; promote it to album artwork only when album identity is confirmed.
- Ensure enriched artwork and metadata appear consistently in library rows, album views, queues, both interface styles, and desktop media controls, including after an offline restart.
- Add settings for JioSaavn availability and automatic gap filling, enabled by default for this feature. Include retry and removal of provider-filled data without removing original tags.

## Reliability and verification

- Use bounded background work, request cancellation, finite timeouts, and backoff for temporary failures. Cache search/detail results and unsuccessful enrichment attempts to avoid repeated requests on every scan. Prioritize user playback over enrichment.
- Keep search debounce at 200 ms, virtualize long results, and retain leaf-level playback progress updates. Enrichment must not introduce continuous root-component rendering.
- Add backward-compatible SQLite migrations for provider matching and enrichment provenance. Preserve existing track IDs, playlists, history, account ownership, and sync data; do not synchronize machine-local artwork paths.
- Test provider parsing against representative fixtures, missing fields, HTML entities, string-valued numbers, unavailable media, and changed response shapes.
- Test mixed local/online queues, pause/resume, seeking, next/previous, rapid selection cancellation, URL refresh, cache reuse, and exactly-once history/statistics recording.
- Test correct enrichment, ambiguous matches, different recording versions, missing artist/title, rescans, embedded-art precedence, removal of enrichment, and offline cache persistence.
- Run the existing frontend and Rust checks plus native playback tests using disposable application data. Verify both Spatial and Brutalist interfaces, MPRIS, and cloud round-tripping of JioSaavn identity. Release acceptance requires live full-track playback through the actual Rust decoder, not just valid URLs.

## Assumptions and delivery

- The integration uses JioSaavn’s catalogue and adds no Spotify account or playlist import.
- The linked API is unofficial. Live compatibility and regional availability remain unverified; the acceptance gate determines whether this approach can proceed.
- Deliver provider verification first, then online playback, then offline enrichment and native regression testing. Public deployment or a release is a separate step after acceptance.
- Update Kaze’s architecture documentation and canonical wiki with the verified implementation and limitations at completion. Knowledge capture for this planning turn is deferred because Plan Mode prohibits file edits.
