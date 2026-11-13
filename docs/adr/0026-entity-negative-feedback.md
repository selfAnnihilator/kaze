# Artist and genre negative feedback

Kaze's only negative signal was "not this song". When a user disliked an artist, Flow and the mixes kept offering that artist's other songs, features and similar-artist picks, and a song that was merely "too much of" a style could only be blocked outright. Spotify offers "Don't play this artist" and "less like this"; Kaze needed both.

Decision:

| Area | Rule |
| --- | --- |
| Storage | `recommendation_entity_feedback(user_id, kind 'artist'\|'genre', key_norm, display, level 'block'\|'less', created_at, PK(user_id, kind, key_norm))`. Artist keys use the first credit from `signals::artist_credits`; genre keys are trimmed lowercase. |
| Commands | `BlockArtist{track_id}` blocks the track's artist. `LessLikeThis{track_id}` marks its artist and genre "less". `ClearEntityFeedback{kind, key}` removes a row. All return `CommandResponse::EntityFeedback(rows)`. `Query::GetEntityFeedback` lists rows. |
| Block | `SongSignals` loads blocked artists, and `blocked()` matches any credit, so every recommender that already respects song dislikes also respects artist blocks. The taste profile drops the artist (and its affinity and breadth), so it stops seeding similarity. |
| Less | Score penalty `0.20` for the artist plus `0.10` for the genre in Flow, mixes and local scoring. Taste affinity of matching keys × `0.5`. Both decay linearly to 0 over 90 days. |
| Precedence | Upsert never downgrades `block` to `less`. |
| Side effects | Expire smart mixes, invalidate home picks, remove recommended songs on a block, publish `EntityFeedbackChanged` and refresh taste. |
| UI | `NotInterestedButton` becomes a menu: "Not this song / Not this artist / Less like this". The notice offers Undo. Settings → "Hidden from recommendations" lists rows with Remove. Local component state only, no backdrop blur. |

Consequences: one click removes an artist everywhere, including features and similarity seeds, and stays reversible. "Less like this" is soft and temporary, so a passing mood does not permanently distort taste. `GetRecommendationFeedback` now also reports tracks hidden by an artist block, which is the intended Settings behaviour.

Tests: `src-tauri/tests/entity_feedback_tests.rs` covers penalty size, credit matching and 90-day decay; block precedence and per-user isolation; a blocked artist being absent from Flow, the Daily mix, the taste profile and `SongSignals`, with Clear restoring it; and "less" halving affinity and lowering the Flow score without blocking.
