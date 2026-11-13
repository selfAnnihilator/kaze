# ADR 0017: One online source for the catalog and audio

Implemented on the testing branch, 2026-10-27. This completes the source consolidation after ADR 0016.

JioSaavn now supplies online search, recommendation candidate discovery, charts, playlist imports, audio downloads and local-file metadata/artwork enrichment. The retired iTunes chart/search implementation, MusicBrainz/Spotify/Cover Art Archive adapters, video/audio download adapters, external Soulseek launching/importing and their public commands/settings are removed. Local files, saved playlists, account history, original tag columns and durable source aliases remain compatible. Schema/tag IDs are historical data, not active providers.

Charts use actual provider playlist IDs and titles returned by content.getCharts. No synthetic worldwide/country chart claims remain. Playlist import accepts JioSaavn URLs or numeric IDs, loads up to 50 available songs, and preserves each fetched source ID, album and artwork. The adapter supports both flat search records and nested playlist more_info fields, including artistMap credits, duration, rights and ordinary media; unavailable/DRM-only entries remain excluded. Language is persisted separately from genre for recommendation work.

Download search and selection use JioSaavn source IDs. Downloads resolve that exact selection, stream bounded media into a private partial file, limit active transfers, support cancellation, and publish a final file without overwriting another file. The existing download service/history/import path remains. No title-based alternate source is used. Source availability settings are honored.

Validation before the cleanup checkpoint: affected Rust suites (discovery, scoring, recommendation, Flow, downloads, source identity and playback restoration), frontend build, thirteen Node files and both interface source/settings checks passed. The opt-in catalog_playlist_and_exact_download_gate exercised provider charts, a real provider playlist URL, exact imported identities, one selected full-song download and the production decoder in disposable storage. Runtime scoring/feedback improvements are a following commit, not part of this source cleanup.

## Addendum 2026-11-13: Spotify playlist links (metadata only)

Playlist import also accepts public Spotify playlist links (`open.spotify.com/playlist/{id}`, `/intl-xx/`, `spotify:playlist:`). Kaze reads names, artists and durations from the public embed page's `__NEXT_DATA__` JSON. There is no API key, no Spotify audio and no Spotify provider identity. Each of the first 50 playable songs is searched on JioSaavn (4 concurrent searches) and accepted only by `strict_match::import_match`. That check requires all of the following:
- the title matches after stripping ` - …`, ` | …`, `(…)` and `[…]`;
- at least one artist is shared;
- the durations are within 5 seconds;
- live, remix and acoustic variant markers agree.

Unmatched songs are listed in the import modal and are not saved. JioSaavn remains the only catalog and audio source. Any other host is rejected with "Paste a JioSaavn or Spotify playlist link". If Spotify changes the embed format, the import fails with a clear "make sure it's public" error.
