# Listening scores and For You

Current behavior, 2026-10-30. Playback stays in the shared Rust PlaybackService; recommendation updates use the same account-scoped history writer.

## What counts

Every saved playback session for the active account is considered, regardless of source: local library, downloaded files, online songs, playlists, collections, and Flow. Brief neutral previews earn no automatic preference. A session is saved when it ends, is stopped, or is replaced by another song. Paused time and seeking are not listening signals. Playlist membership alone is not a listen.

The profile reads all retained account history on this device. It does not discard sessions after 90 days. Cloud synchronization currently transfers aggregate statistics rather than raw playback sessions; this change does not reconstruct other devices' missing raw history.

## How listening becomes affinity

Each session contributes to its artist, available genre, and decade:

- Below the positive listening threshold: neutral. The threshold is `min(30 seconds, max(10 seconds, 25% of duration))`; unknown durations need 30 seconds.
- Qualified partial listen: `+0.35`; at least 75% actually heard: `+0.75`; at least 95%: `+1`.
- Intentional skip before ten seconds: `-0.75`. Playback errors and leaving a buffering stream do not imply dislike.
- Explicit like/dislike: `+2` / `-2` to artist and genre totals.

Totals are turned into affinities with an absolute saturating curve, `affinity = 1 - exp(-raw / K)`, rather than normalization against the strongest total. As a result, a couple of listens is a hint, not a verdict. `K` is 3 for artists and 4 for genre, language and decade. Two full listens of an artist give about 0.3–0.4; around five distinctly liked songs are needed to pass 0.75 ([ADR 0023](adr/0023-taste-saturation-and-artist-diversity.md)).

Artist evidence is first aggregated per song. Each song then adds `min(ln(1 + net), 2)`; skips stay linear. Replaying one song ten times therefore keeps the artist below about 0.6, while liking many different songs by the artist builds strong affinity. The profile records each artist's *breadth*: the number of distinct positively-engaged songs.

Priors:

- Liked songs add `+2` as a style signal (genre/language/decade); their artist contribution goes through the per-song damping.
- The account's own playlists add a small per-track prior to artist, genre and language.
- Local files are shared across profiles. They only *amplify* artists, genres and languages that already have positive evidence from this account, adding `min(0.6 × ln(1 + owned), 1.5)`. Owning files never creates taste on its own.

Each curve is applied separately to recent and lifetime totals, which are then blended:

- Recent listening: 60%, using the last 14 days with `exp(-0.05 × age_days)` decay.
- Lifetime listening: 40%, using all retained sessions with `max(exp(-age_days / 180), 0.1)` decay.

Decade affinity uses lifetime weighting. Genre metadata can come from the local track or its external catalog record. Missing artist/genre metadata limits what can be learned from a song; audio similarity is not analyzed.

Actual heard seconds determine these tiers, never seek position or an EOF flag alone. See [ADR 0022](adr/0022-listening-time-preference.md) for history reconciliation and usage-statistics compatibility.

## How recommendations use the scores

Discover's For You ranks candidates using up to `90 × artist_affinity` plus genre/language terms (style leads). The shared scorer weights artist 0.20, genre 0.32, language 0.14, era 0.10, and performance 0.20. The unplayed-gem bonus requires a style match (genre or language affinity above 0.35) or an artist breadth of at least 3, so one liked song cannot promote the artist's whole catalog. Local-library artists/genres provide smaller cold-start signals. Online placeholders belonging to another account do not become library preference signals. Scores are deterministic: refreshing does not add random ranking noise.

Manual Refresh requests a fresh selection as well as refreshed source metadata. Within each taste-ranked artist's slots, it prefers least recently shown song families whose scores are at least 85% of that artist's best candidate. This changes the selection without changing any taste score or inventing listening history. Repeated clicks rotate beyond two batches when alternatives exist. Ordinary reads retain the chosen batch while the underlying ranked pool is unchanged; changed listening/feedback/catalog ordering can supersede it. Selection memory is process-local, bounded to sixteen accounts and pruned to available candidates. Dislike, source identity, song deduplication and the two-per-artist cap still apply. Small eligible pools may legitimately repeat.

Candidates come from the existing metadata pool and catalog searches for the four strongest positive artists. Artist results are cached for 15 minutes, bounded to 64 artists, and kept alongside the pool so the latest 300 unrelated metadata records cannot evict them. Existing regional/genre candidates remain available. Failed artist lookups retain cached candidates where possible. Catalog coverage and network availability still constrain fresh discoveries.

The backend retains at most two songs per artist and reserves no For You quota for generic chart hits. The frontend continues to exclude saved, local, liked, and downloading songs. Trending and Top Charts use their independent popularity feeds.

The frontend accepts the current backend's genre/language explanations (`Matches your … listening`) as personal evidence, alongside artist/taste explanations. Those valid picks were previously dropped by an outdated explanation filter.

Home retains local recommendations. Runtime Flow combines the shared multi-factor taste scorer with recorded song familiarity and online exploration, as described below. Home's short cache checks the account's history count so a new session triggers recomputation before the ordinary 60-second expiry.

## Home Flow familiarity and online songs

The familiarity bar steers the upcoming Flow queue from **Unfamiliar** to **Familiar**:

- **Familiar (100%)** favors songs actually played repeatedly or explicitly liked, plus style-matched songs by artists with real breadth. It does not repeat the same artist back to back, and even the most-played artist gets no extra quota: Familiar means more of your styles, not more of one artist.
- **Unfamiliar (0%)** favors never-heard songs, preferably by new artists, that still match the listener's preferred styles (genre/language/decade). This includes online discoveries.
- **Middle values** blend these. A small forgotten-favorite bonus peaks at 50%.

Steering score: `style_match × 0.45 + ratio × (known_strength × 0.35 + artist_affinity × 0.20) + (1 − ratio) × novelty × 0.55`. Here `novelty = (1 − known_strength) × 0.6 + (1 − artist_affinity) × 0.4`. A liked song has known strength 1; otherwise strength grows with plays as `min(log(1 + play_count) / log(7), 1)`. Recency dampening and skip penalties still apply.

### Artist diversity (radio-style)

Flow, local recommendations and For You share one diversifier (`recommendations/diversity.rs`):

- An artist key is the canonical primary credit, so feat./&/x variants count as the same artist.
- No artist repeats within the last four picks, and there are at most two per twenty in a queue window. These soft rules are relaxed only when no alternative exists.
- Same-album repeats are penalized.
- Flow refills carry the recently queued/played artists (about the last 15), so the caps do not reset every batch.
- No artist is exempt, however much it is played; there is no favorite-artist allowance at the Familiar end.
- Recommendation lists (local and shelves) also have a hard cap of two songs per artist for the whole list, which is never relaxed.
- Smart mixes (Daily, On Repeat, Forgotten Favorites, Late Night, Discovery, genre mixes) pass through the same diversifier with the hard cap of two per artist. Artist Radio is the only intended exception.

Runtime generation combines physical local tracks with a bounded pool of 2,000 external metadata candidates, prioritizing the account's known/liked songs before recent metadata. Fresh candidates need positive artist or genre affinity. Liked Songs membership is account-scoped and counts as explicit song familiarity even before the first play. Another account's history/likes do not become the current account's preferences. Exact normalized song identities are deduplicated across catalog aliases and local matches; queued, disliked, and repeatedly skipped online candidates are excluded as appropriate.

Unfamiliar reserves room for online candidates so equal-scoring local songs cannot hide them. The exploration target is `(1 − ratio) × 0.6` of the selected slots: 60% at 0% and 30% at 50%. This is a target subject to availability and artist spacing, not a guaranteed source ratio. Familiar online favorites can still rank normally at 100%. A local-only eligible pool remains usable.

The online metadata pool is supplied by ordinary online search, discovery and previous listening. Flow selection itself performs no network search or audio download. Only selected online songs are registered for the shared Rust queue. When played, they use PlaybackService's existing full-song resolver/cache, backend, session, history, and statistics path. Provider availability or a cached full song is required for online audio. Preview URLs are not played.

Moving the dial/bar now replaces generated upcoming slots after the existing 250ms debounce. The current entry, position, active playback session, Flow generation, and its original explanation stay intact. Manual queue entries retain their positions when enough replacements exist. No restart or wait for the old 20-song batch is needed. The UI fetches the confirmed Flow session and playback snapshot after saving steering, so the status reflects the applied choice.

The lower-level `GetFlowQueue`/local generation preview remains read-only and local; runtime start, steering and refill use the online-capable generator. Home's For You uses the same eligible local and online candidate pool, including starter selections before listening history exists.

### Session steering (live skips and full listens)

Long-term taste learns from history and moves slowly. A running Flow session also reacts to what happens *now* (`recommendations/session.rs`):

- HistoryService publishes `Event::ListenOutcome` (early skip / qualified / near-complete) when each play session finalizes. PlaybackService listens only while the current source is Flow and the outcome belongs to the active Flow account.
- Each outcome is keyed by the track's canonical artist(s), genre and identity language. An early skip adds −1, a near-complete listen +1, a qualified partial +0.4; short non-skip stops are ignored. All session scores decay ×0.85 per recorded outcome, so evidence fades after about 15 plays.
- Triggers: the last 2 skips share a genre, language or artist → steer away from it. Any 3 consecutive skips → steer away from all of them. 3 consecutive near-complete listens sharing a genre or language → steer toward it. After a trigger, the next 2 outcomes cannot trigger again.
- A trigger re-ranks only generated upcoming slots (15 picks). The current track, playback position and manual queue entries stay in place; Flow generation checks discard stale re-ranks.
- Score shift: `0.18·tanh(g/2) + 0.08·tanh(l/2) + 0.12·tanh(min artist/2)`. Negative totals count double (a skip is a direct "not now"). The result is clamped to [−0.30, +0.15]. It is applied after the skip penalty and before the starter floor. The asymmetric cap means full listens can only nudge a style up, and the shared diversifier still holds every artist to two per twenty.
- A genre with session score ≤ −1.5 gets no online exploration slots for the rest of the decay window.
- Steering and refills also use the session state. It lives in memory in the active Flow, is reset by every `StartFlow`, and disappears when Flow stops or another source takes over. Nothing is persisted.
- Explanations carry `session_style`/`session_adjustment` when |shift| ≥ 0.1. The UI only shows the positive note ("You've been finishing Jazz this session").

### Similar artists ("fans of X also like Y")

Genre and language overlap alone cannot say that two artists share a fan base. Kaze keeps an artist graph in `artist_similarity` with two sources (ADR 0025):

- **Co-occurrence (`source='cooc'`, per user)**. `recommendations/cooccurrence.rs` builds it from the user's own behaviour. Two artists pair up when they sit within 5 positions in a hand-made (non-smart) playlist. They also pair up when listened to within 5 qualified listens of each other in one session, where sessions split on gaps of more than 30 minutes. Each artist counts once per playlist or session. Pairs seen in fewer than 2 sequences are dropped. Score = `count / sqrt(n_a·n_b)`, with at most 50 neighbours per artist. A rebuild rewrites the user's rows. The worker rebuilds after `TasteProfileChanged`, at most every 6 hours.
- **Last.fm (`source='lastfm'`, shared, `user_id=''`)**. `providers/similarity_worker.rs` calls `artist.getSimilar` for one artist every 1.2 s. It works through the most-listened artists first and then the rest of the library. Results are cached in `lastfm_artist_fetch`:
  - `ok` and `missing` results are kept for 30 days.
  - errors are retried after 1 day.

  The worker pauses on buffering and playback start, and waits 30 s after startup. Only artist names are sent. It is off when the setting `enable_lastfm_similarity` is false or when no API key is compiled into `providers/lastfm.rs`.

`SimilarityIndex` (`recommendations/similarity.rs`) seeds from the top 15 taste artists. A candidate artist's boost is `max(score · weight · seed affinity)`, where the weight is 1.0 for co-occurrence and 0.8 for Last.fm. Seeds themselves never get a boost.

- **Flow**: `+0.12 · boost · (1 − artist affinity)`, applied after the base score, so it lifts unknown artists only. An online candidate passes the pre-filter when `boost > 0.2`, even without a genre match. When the effective boost is ≥ 0.1, the explanation carries `similar_artist` and the UI shows "Fans of X also like this".
- **Discovery**: the 3 strongest similar artists that are not already liked become extra artist searches. A track whose artist has `boost > 0.2` passes the relevance filter and scores `+40 · boost · (1 − artist affinity)`, with the reason "Fans of X also like Y".
- If loading the index fails, it falls back to an empty index, so recommendations never fail because of similarity.

### Not this artist / less like this

The not-interested control on a song opens a menu with three choices (ADR 0026):

- **Not this song**: the existing per-track dislike.
- **Not this artist** (`BlockArtist`): stores `recommendation_entity_feedback(kind='artist', level='block')` for the track's first artist credit. `SongSignals::blocked()` then drops every track that credits that artist, including features, from Flow, mixes, the local recommender, discovery and `filter(json)`. The taste profile removes the artist, so it no longer seeds similarity.
- **Less like this** (`LessLikeThis`): stores `level='less'` for the artist and for the track's genre. Candidates lose `0.20` (artist) plus `0.10` (genre) in Flow, mixes and local scoring. The taste profile scales the matching artist and genre affinities by `0.5`. Both effects fade linearly to nothing over 90 days, counted from `created_at`.

A block is never downgraded to "less" by a later click. Every change expires the smart mixes, invalidates home picks and emits `EntityFeedbackChanged`. The notice offers Undo (`ClearEntityFeedback`), and Settings → "Hidden from recommendations" lists every row with a Remove button.

### Time-of-day context

The taste pass also learns *when* styles are played (ADR 0027):

- Each qualified listen is bucketed by local time: morning 05–11, afternoon 12–16, evening 17–21, night 22–04. Each listen is counted twice, once as part × weekday/weekend (for example `evening:weekend`) and once as the day part alone. Listens after midnight keep the previous day's weekday/weekend type.
- A bucket stores, for each genre and language, its *lift* over the user's overall share: `max(0, share − base) / (1 − base)`, capped at 1 and dropped below 0.05. A style played equally at every hour therefore gets no context lift.
- A bucket only counts once it has at least 20 qualified listens. It is stored in `TasteProfile.context_affinity`.
- **Flow** looks up the current bucket (the specific key first, then the day part) and adds `0.08 × max(genre lift, 0.6 × language lift)`. When the lift is ≥ 0.25, the explanation says "Fits your evenings/mornings/afternoons/late nights".
- **Late Night mix**: up to 15 tracks come from the top 3 night-lift genres. The existing mellow selection fills the rest.

### Daily Mix 1..N (taste clusters)

Instead of one Daily Mix, Kaze builds one per taste cluster, the way Spotify does (ADR 0028). `recommendations/clusters.rs` takes the 40 strongest taste artists and links each pair by `max(similarity, 0.5 if they share a primary genre)`. Similarity is the best `artist_similarity` score, with co-occurrence weighted 1.0 and Last.fm 0.8. The primary genre is the artist's most common library genre. Clusters form by greedy average-linkage merging while the average link is at least 0.25. Clusters with 3 or more artists are kept, at most 4 of them, ordered by total affinity. The same input always gives the same clusters.

- With at least 2 clusters, `ensure_default_mixes` creates `mix_type = daily:N` named "Daily Mix N". The reason line reads "A, B, C and more". About 70% of the tracks are by the cluster's own artists; about 30% are by the top 8 similar artists that are not already among the user's top artists. The normal blocked filter, dedupe and artist spacing still apply. Each mix expires after 24 h.
- The Daily family (`daily` and `daily:*`) is regenerated as a whole when any member is missing or expired. Leftover `daily:K` mixes beyond the current cluster count, and the single `daily`, are deleted.
- With fewer than 2 clusters (cold start, thin or single-style history), the single Daily Mix stays and all `daily:*` mixes are removed.

### Smooth transitions (audio features)

Flow orders its picks so that tempo and energy do not jump between songs (ADR 0029). A background worker (`src-tauri/src/audio_features.rs`) analyzes local files one at a time, with no new dependencies:

- **Decoding:** Symphonia decodes 60 s from the middle of the track, mixes it down to mono and decimates it to about 11 kHz.
- **Energy:** RMS loudness in dB, mapping −40 dB to 0 and 0 dB to 1.
- **Brightness:** the zero-crossing rate, a cheap stand-in for spectral centroid.
- **Tempo:** the onset-flux envelope is autocorrelated over 60–180 BPM, with a soft preference for tempos near 120 BPM. The tempo is stored only when its confidence is at least 0.15; otherwise it is NULL.

Results go in `track_audio_features`. Tracks that cannot be decoded get a NULL-energy marker, so they are not retried until `ANALYZER_VERSION` changes. Online tracks are skipped. The worker starts 45 s after launch. It pauses while playback is starting or buffering and waits 2 s between tracks. It wakes after a library scan.

**Distance between two songs.** The tempo term is octave-tolerant: it takes the smallest of |log2 ratio| and |log2 ratio ± 1|, divides by 0.2, and caps the result at 1, so 120 → 240 BPM counts as a match. The distance is `0.5·tempo + 0.35·energy + 0.15·brightness`. When either song has no tempo, it is `0.7·energy + 0.3·brightness` instead.

**How Flow uses it.**

- **Transition cost:** while picking, the diversifier subtracts `0.10 × distance(previous pick, candidate)` from each candidate's score.
- **Missing features:** a candidate with no features (an online or not-yet-analyzed song) costs a neutral 0.5, so online songs are neither favored nor buried.
- **Seed:** the transition chain starts from a seed.
  - A queue refill seeds from the last queued song.
  - Rerank and steering seed from the current song.
  - Starting Flow seeds from the current song.
- **Other callers:** mixes, Home shelves and Discovery keep a weight of 0, so they are unchanged.

### Auto-tuned exploration

Flow learns whether new finds work for you and adjusts its explore share slightly (ADR 0030). The familiarity dial stays authoritative. The learned bias is only a small nudge on top of it.

- **Slots:** each time playback Flow enqueues picks, every pick is recorded in `flow_slot_assignments` as `explore` or `familiar`.
  - A pick is `explore` when it is an online song, or when its novelty term `(1−ratio)·novelty·w` outweighs its familiar term `ratio·(known·recency·0.35 + affinity·0.20)`.
  - The Home preview never records slots.
- **Attribution:** when a Flow listen (`source='flow'`) is finalized, `playback_history.rec_slot` is stamped with that track's current slot.
- **Success per slot:** computed over the last 30 days, where a qualified listen is not skipped and is either completed or ≥ 50 % heard. Early skips count −1. `success = ((qualified − skips)/listens + 1)/2`.
- **Bias:** `clamp((success_explore − success_familiar) × 0.5, −0.12, +0.12)`. The bias is 0 until both slots have at least 30 listens.
  - It is cached once a day in the settings key `flow_explore_bias:{user}`.
- **Applied:**
  - The novelty weight becomes `0.55 × (1 + bias)`.
  - The online share becomes `clamp((1−ratio)·0.6 + bias, 0, 0.7)`, and only when online songs are allowed.
- **Explanation:** an explore pick with a bias ≥ 0.03 shows "Exploring a bit more — you've liked new finds." A negative bias is not announced.

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

## First-use tastes and starter songs

After folder onboarding, accounts without positive learned taste signals get a skippable artist-and-genre setup. Existing accounts with useful listening or likes do not get prompted. Completion/skip and up to five artist and five genre choices are stored per account on this device in `recommendation_setup`, separate from listening history, computed `user_preferences` and song feedback. Revisit them through Settings → Music taste. These explicit choices are not cloud-synchronized. Setup reads carry their account identity and saves reject a mismatching expected account.

Missing artist/genre affinities receive a 0.35 starter prior; an existing learned entry, including zero, takes precedence. No listens, likes, playtime or aggregate statistics are manufactured. New `has_learned_taste` profile evidence is computed before adding these priors. Starter explanations explicitly identify chosen artists, genre-query provenance or popular songs; they do not claim prior listening.

The catalog coordinator prepares an account- and preference-revision-scoped starter pool before Home, Discovery, playlists/mixes and runtime Flow need it. Up to ten metadata searches run three at a time, alongside the existing public editorial fallback when needed. Artist-query candidates must match normalized artist credits. Genre-query provenance is stored separately instead of inventing verified song genres. Pools are bounded to 250 IDs and in-memory attempt tracking to 16 accounts, coalescing simultaneous reads; normal reads reuse a prepared pool for 15 minutes and empty failures for 30 seconds. Explicit refresh retries immediately. Revision checks prevent a slow response for older choices from repopulating the new pool.

Skipping or insufficient matching candidates uses public cached/live chart songs, not another account's learned preferences. Cached matching metadata and editorial feeds remain usable offline. Without usable local/cached music and without catalog access, the app shows honest retry/connection guidance. Feedback exclusions, source identity, duplicate-release handling and artist diversity still apply. As positive learned taste develops, unchosen generic popular candidates stop bypassing normal taste eligibility.

Daily and Discovery mixes can use starter songs. Recent history, On Repeat and Forgotten Favorites retain their real-history/feedback requirements. Metadata preparation fetches no audio. Start Flow reserves its generation token before preparation and takes its playback-operation lock afterward; only selected songs enter the existing Rust resolver/player.

`GetRecommendationSetup` and `SaveRecommendationSetup` are additive typed IPC interfaces. The SQLite migration adds setup and starter-membership tables without rewriting existing history or affinities. Saving choices invalidates Home picks and generated mixes and emits the existing account-scoped taste-change event.

Verify with `cargo test --offline --locked --test recommendation_setup_tests`, the existing Flow/playback/feedback groups, `node scripts/test-first-use-ui.mjs` and `KAZE_TEST_URL=http://127.0.0.1:5194 python3 scripts/test-first-use-webkit.py` against a local Vite server. Test catalogs use isolated loopback fixtures and synthetic cached WAV audio. Keep local-only Flow tests explicitly offline; enabling automatic starter preparation otherwise makes a new account eligible for online music.
