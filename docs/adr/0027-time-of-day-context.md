# Time-of-day listening context

Kaze scored candidates the same at 8 am and at midnight, while Spotify's mixes lean on what you play at that time. Many users have clear patterns, such as focus music in the morning or mellow or regional music late at night.

Decision:

| Area | Rule |
| --- | --- |
| Buckets | Local time via `chrono::Local`: morning 05–11, afternoon 12–16, evening 17–21, night 22–04. Each listen is counted under `part:weekday\|weekend` and under `part`. After midnight, a listen keeps the previous day's weekday/weekend type. |
| Signal | Only qualified listens count (`Engagement::qualified`), weighted by the session weight. Counts are kept per lowercase genre and language. |
| Affinity | Lift over the overall share, `max(0, share − base)/(1 − base)`, capped at 1, dropped below 0.05, rounded to 3 decimals. Styles played at every hour get nothing. |
| Minimum | A bucket needs ≥ 20 qualified listens. Smaller buckets are omitted, so a thin history has no effect. |
| Storage | `TasteProfile.context_affinity: HashMap<String, ContextAffinity>`, a serde default, so old cached profiles still load. |
| Flow | `+0.08 × max(genre lift, 0.6 × language lift)` for the bucket in effect (the specific key first, then the day-part fallback). If the lift is ≥ 0.25, `FlowExplanation.context_bucket` is set and the UI shows "Fits your evenings". This text is not shown for starter picks. |
| Late Night mix | Up to 15 tracks come from the top 3 night genres. The existing mellow query fills the mix up to 25. |

Alternatives: hour-of-day histograms would need much more data per bucket. Weekday-only buckets would split thin history further. A fixed time-to-genre map ("jazz at night") would ignore the user's own habits.

Consequences: the weight is small (0.08, below the 0.12 for similarity), so context nudges ordering without overriding taste or the familiarity dial. Buckets are recomputed with the taste profile and cost no extra queries.

Tests: `src-tauri/tests/context_taste_tests.rs` covers day-part boundaries and the after-midnight rule; lift for a style concentrated in one part versus zero for one played everywhere; buckets under 20 listens being absent; and a real taste profile learning evening jazz and morning rock from history. `tests/homeFlowUx.test.mjs` covers the explanation copy.
