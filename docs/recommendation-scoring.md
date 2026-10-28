# Listening scores and For You

Current behavior, 2026-10-28. Playback stays in the shared Rust PlaybackService; recommendation updates use the same account-scoped history writer.

## What counts

Every saved playback session for the active account is considered, regardless of source: local library, downloaded files, online songs, playlists, collections, and Flow. Brief neutral previews earn no automatic preference. A session is saved when it ends, is stopped, or is replaced by another song. Paused time and seeking are not listening signals. Playlist membership alone is not a listen.

The profile reads all retained account history on this device. It does not discard sessions after 90 days. Cloud synchronization currently transfers aggregate statistics rather than raw playback sessions; this change does not reconstruct other devices' missing raw history.

## How listening becomes affinity

Each session contributes to its artist, available genre, and decade:

- Below the positive listening threshold: neutral. The threshold is `min(30 seconds, max(10 seconds, 25% of duration))`; unknown durations need 30 seconds.
- Qualified partial listen: `+0.35`; at least 75% actually heard: `+0.75`; at least 95%: `+1`.
- Intentional skip before ten seconds: `-0.75`. Playback errors and leaving a buffering stream do not imply dislike.
- Explicit like/dislike: `+2` / `-2` to artist and genre totals.

Artist and genre totals are normalized against the strongest positive total, clamped to `[0, 1]`, and blended:

- Recent listening: 60%, using the last 14 days with `exp(-0.05 × age_days)` decay.
- Lifetime listening: 40%, using all retained sessions with `max(exp(-age_days / 180), 0.1)` decay.

Decade affinity uses lifetime weighting. Genre metadata can come from the local track or its external catalog record. Missing artist/genre metadata limits what can be learned from a song; audio similarity is not analyzed.

Actual heard seconds determine these tiers, never seek position or an EOF flag alone. See [ADR 0022](adr/0022-listening-time-preference.md) for history reconciliation and usage-statistics compatibility.

## How recommendations use the scores

Discover's For You ranks candidates using up to `150 × artist_affinity` plus up to `100 × genre_affinity`. Local-library artists/genres provide smaller cold-start signals. Online placeholders belonging to another account do not become library preference signals. Scores are deterministic: refreshing does not add random ranking noise.

Manual Refresh requests a fresh selection as well as refreshed source metadata. Within each taste-ranked artist's slots, it prefers least recently shown song families whose scores are at least 85% of that artist's best candidate. This changes the selection without changing any taste score or inventing listening history. Repeated clicks rotate beyond two batches when alternatives exist. Ordinary reads retain the chosen batch while the underlying ranked pool is unchanged; changed listening/feedback/catalog ordering can supersede it. Selection memory is process-local, bounded to sixteen accounts and pruned to available candidates. Dislike, source identity, song deduplication and the two-per-artist cap still apply. Small eligible pools may legitimately repeat.

Candidates come from the existing metadata pool and catalog searches for the four strongest positive artists. Artist results are cached for 15 minutes, bounded to 64 artists, and kept alongside the pool so the latest 300 unrelated metadata records cannot evict them. Existing regional/genre candidates remain available. Failed artist lookups retain cached candidates where possible. Catalog coverage and network availability still constrain fresh discoveries.

The backend retains at most two songs per artist and reserves no For You quota for generic chart hits. The frontend continues to exclude saved, local, liked, and downloading songs. Trending and Top Charts use their independent popularity feeds.

The frontend accepts the current backend's genre/language explanations (`Matches your … listening`) as personal evidence, alongside artist/taste explanations. Those valid picks were previously dropped by an outdated explanation filter.

Home retains local recommendations. Runtime Flow combines the shared multi-factor taste scorer with recorded song familiarity and online exploration, as described below. Home's short cache checks the account's history count so a new session triggers recomputation before the ordinary 60-second expiry.

## Home Flow familiarity and online songs

The familiarity dial/bar is steering for the upcoming Flow queue:

- **Familiar (100%)** favors songs actually played repeatedly or explicitly liked, including online songs. An unheard song by a favorite artist is not treated as equally familiar.
- **Rediscover (0%)** favors less-heard songs and forgotten favorites, including online discoveries aligned with the listener's taste.
- **Middle values** blend these priorities.

Flow's familiarity component is `0.4 × shared_taste_score + 0.6 × known_strength`. A liked song has known strength 1; otherwise strength grows with plays as `min(log(1 + play_count) / log(7), 1)`. The existing rediscovery, recent-repeat, skip, and artist-spacing rules remain. Recent listening and feedback from every playback source feed the shared taste profile.

Runtime generation combines physical local tracks with a bounded pool of 2,000 external metadata candidates, prioritizing the account's known/liked songs before recent metadata. Fresh candidates need positive artist or genre affinity. Liked Songs membership is account-scoped and counts as explicit song familiarity even before the first play. Another account's history/likes do not become the current account's preferences. Exact normalized song identities are deduplicated across catalog aliases and local matches; queued, disliked, and repeatedly skipped online candidates are excluded as appropriate.

Rediscover reserves room for online candidates so equal-scoring local songs cannot hide them: the exploration target grows toward half the selected slots at 0%, and roughly one quarter at 50%. This is a target subject to availability and artist spacing, not a guaranteed source ratio. Familiar online favorites can still rank normally at 100%. A local-only eligible pool remains usable.

The online metadata pool is supplied by ordinary online search, discovery and previous listening. Flow selection itself performs no network search or audio download. Only selected online songs are registered for the shared Rust queue. When played, they use PlaybackService's existing full-song resolver/cache, backend, session, history, and statistics path. Provider availability or a cached full song is required for online audio. Preview URLs are not played.

Moving the dial/bar now replaces generated upcoming slots after the existing 250ms debounce. The current entry, position, active playback session, Flow generation, and its original explanation stay intact. Manual queue entries retain their positions when enough replacements exist. No restart or wait for the old 20-song batch is needed. The UI fetches the confirmed Flow session and playback snapshot after saving steering, so the status reflects the applied choice.

The lower-level `GetFlowQueue`/local generation preview remains read-only and local; runtime start, steering and refill use the online-capable generator. Home's separate For You list remains a local-library shelf.

### Native bar dragging

Brutalist's range input owns pointer-down/move/up while using pointer capture. WebKitGTK stops its native range thumb handling after `setPointerCapture`, so relying on native `onChange` events left the bar stuck. Explicit pointer-to-value mapping fixes that while preserving outside release, 0.05 steps, keyboard access and local preview. No Flow command is sent during movement; release commits once through the existing debounce. Blur, pointer cancellation and lost capture restore the last controlled value.

Run `python3 scripts/test-familiarity-webkit.py` against the local Vite server to exercise trusted GDK events in an offscreen WebKitGTK widget. It checks the original drag, outside release/clamping and cancellation without opening or controlling the user's music player. PyGObject must load Gtk/Gdk 3.0 and WebKit2 4.1 explicitly. The existing `node scripts/test-flow-steering.mjs` now also exercises actual mouse dragging through App, alongside both themes' keyboard checks.

## Active refresh

HistoryService emits `TasteProfileChanged { user_id }` only after its history/statistics transaction commits successfully. Explicit feedback also emits the event after saving. App coalesces bursts for 250ms and refreshes personal recommendations and Home; it does not refetch public charts for listening changes. Account identity and request sequencing prevent a slow, older response from replacing newer results. Initial account hydration also refreshes the personal feed.

Automatic personal refresh passes `force_refresh: false`; only explicit refresh requests a new batch. Both layouts retain existing cards while their refresh icon is busy, then replace them when the response arrives. Network/catalog availability determines request latency; waiting alone does not guarantee different results. Run `scripts/test-for-you-refresh-ui.mjs` for manual/background refresh isolation and the `manual_for_you_refresh_rotates_relevant_songs_without_changing_taste` Rust regression for rotation, account isolation and exclusions.

Playback position ticks do not trigger scoring, recommendation requests, or new root component state. The single backend event subscription and leaf playback-progress emitter remain in place.

## Verification

2026-10-28: all 226 Rust tests passed (nine optional/manual checks ignored), all 56 frontend tests passed, and both layouts passed manual-refresh/busy/background separation, active scoring and startup checks. Frontend/embedded native builds and all thirteen private native continuity groups passed. The current discovery integration test is included in this full regression run.

For runtime Flow and playback continuity, run `cargo test --offline --manifest-path src-tauri/Cargo.toml --test flow_online_tests --test flow_tests --test playback_tests --test playback_restore_tests --test lifecycle_tests`. Use disposable XDG data/cache locations for these tests. `node scripts/test-flow-steering.mjs` checks real-App familiarity controls, debounce, confirmed status refresh, current-track continuity and one backend subscription in both themes. The online-only Rust regression uses a valid full-song cache fixture with the mock audio backend; audible native output and live provider playback are separate checks.


- `cargo test --offline --manifest-path src-tauri/Cargo.toml --test active_scoring_tests --test recommendation_tests --test history_tests --test flow_tests --test statistics_audit_tests --test discovery_tests -- --test-threads=1`
- `node --test --test-isolation=none tests/*.test.mjs`
- `npm run build`
- Start the local preview, then run `node scripts/test-active-scoring.mjs` to exercise both presentations, committed-history refresh, debounce, independent charts, account isolation, progress ticks, and stale-response protection.

Historical verification limit (2026-10-18): an older discovery integration test failed without network access, and the then-current shared browser harness stopped at a stale `Library and more` locator. Those observations describe that earlier revision; the current checks above supersede them.
