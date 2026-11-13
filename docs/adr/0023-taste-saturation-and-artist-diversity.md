# Taste saturation, artist diversity and Unfamiliar → Familiar steering

Recommendations were too eager. Taste affinities were max-normalized, so two full listens of one artist made that artist the strongest possible signal (1.0). That flooded Flow, For You and local mixes with the artist's whole catalog. Local and liked songs fed style taste weakly. Flow's artist cap reset on every refill.

Decision:

| Area | Rule |
| --- | --- |
| Affinity curve | `1 - exp(-raw / K)` with K = 3 for artists and 4 for genre, language and decade, applied to recent (60%) and lifetime (40%) totals separately. |
| Per-song damping | Evidence is first summed per song. Each song then adds `min(ln(1 + net), 2)` to its artist; skips stay linear. Replays of one song cannot make the whole artist a favorite. Breadth counts distinct positively-engaged songs. |
| Priors | Liked songs add +2 to style (genre/language/decade). Own playlists add a small per-track prior. Shared local files only amplify keys that already have positive user evidence. |
| Scoring | Artist 0.20, genre 0.32, language 0.14, era 0.10. The unplayed-gem bonus needs a style match or an artist breadth of at least 3. For You uses `90 × artist`. |
| Diversity | One shared diversifier keyed on the canonical primary artist: none within 4 picks, at most 2 per 20, an album penalty, and cross-batch memory of about 15 recent artists. No artist is exempt, including the most-played one. Recommendation lists and smart mixes (except Artist Radio) have a hard cap of 2 per artist. |
| Steering | The bar runs Unfamiliar (new songs and artists in preferred styles, 60% online target) to Familiar (known favorites plus artists with breadth). Forgotten favorites get a small bonus at the midpoint. The IPC field `familiarity_ratio` is unchanged. |

Consequences: liking one song no longer implies liking the artist, and Flow sounds like a style radio. A new user's artist affinities grow more slowly. The starter-setup floor (0.35) still covers cold start.

Tests: `src-tauri/tests/taste_saturation_tests.rs`, plus `flow_tests.rs` (`two_listens_do_not_flood_flow_across_refills` and `steering_runs_unfamiliar_to_familiar`, `most_played_artist_does_not_flood_familiar_flow`, `most_played_artist_does_not_flood_smart_mixes`).
