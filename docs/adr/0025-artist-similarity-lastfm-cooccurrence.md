# Artist similarity from Last.fm and personal co-occurrence

Recommendations treated two artists as related only when they shared genre or language tags. Tags are sparse and coarse, so Flow and Discovery could not find "fans of X also like Y" artists, which is the strongest discovery signal Spotify has. Kaze has too few users for collaborative filtering between users to work, so that was ruled out.

Decision:

| Area | Rule |
| --- | --- |
| Sources | Personal co-occurrence: the same hand-made playlist (window 5) and the same listening session (gap ≤ 30 min, qualified listens, window 5), cosine `count/sqrt(n_a·n_b)`, count ≥ 2. Plus Last.fm `artist.getSimilar`, which is computed from Last.fm's listener base. |
| Storage | `artist_similarity(user_id, source_norm, target_norm, target_display, score, source, updated_at)`. Last.fm rows are shared with `user_id=''`, and co-occurrence rows belong to each user. Keys are normalized with `signals::artist_credits`. `lastfm_artist_fetch` holds the TTL and negative cache: 30 days for ok and missing, 1 day for errors. |
| Worker | `SimilarityService` sends one Last.fm request every 1.2 s, most-listened artists first. It yields to buffering and playback start and rebuilds co-occurrence at most every 6 h after taste changes. It is disabled by the setting `enable_lastfm_similarity` or by an empty key. |
| Scoring | Seeds are the top 15 taste artists. `boost = max(score·weight·affinity)`, with weight 1.0 for co-occurrence and 0.8 for Last.fm. Seeds are excluded. Flow adds `+0.12·boost·(1−artist affinity)` and lets online candidates with `boost > 0.2` through the pre-filter. Discovery searches the top 3 similar artists and adds `+40·boost·(1−affinity)`. |
| UI | `FlowExplanation.similar_artist` → "Fans of X also like this". No new frontend state. |

API key: the read-only Last.fm API key is a constant in `src-tauri/src/providers/lastfm.rs` rather than a Settings field. It is public, rate-limited and read-only, which is the model every open-source Last.fm client follows, and asking each user to register a key would mean that almost nobody enables the feature. The shared secret is never used or stored, because Kaze makes no authenticated or write calls. An empty constant turns the feature off cleanly.

Privacy: only artist names leave the device. Kaze sends no account, scrobbles, listening history or identifiers. Co-occurrence is computed locally and never leaves the database.

Consequences: unknown artists who share fans with the user's favourites rise in Flow's unfamiliar side and in Discovery, and the reason names the seed artist. Similarity can only add, so a missing graph (offline, no key, new library) behaves exactly like before. Last.fm data skews toward Western catalogues, so personal co-occurrence weighs more and covers what Last.fm lacks.

Tests: `src-tauri/tests/artist_similarity_tests.rs` covers co-occurrence scoring and rebuild, the Last.fm parser fixtures (no network), index ordering and seed exclusion, and Flow ranking a similar unknown artist above an unrelated one.
