# Taste-cluster Daily Mixes

Kaze had a single Daily Mix that blended every taste. A listener with rock and jazz got a mix that alternated between them. Spotify splits this into Daily Mix 1..6, each staying in one corner of the user's taste.

Decision:

| Area | Rule |
| --- | --- |
| Nodes | The 40 strongest taste-profile artists with positive affinity, keyed by the normalized first artist credit. Clustering is skipped when there are fewer than 6. |
| Links | `max(artist similarity, 0.5 for a shared primary genre)`. Similarity comes from `artist_similarity` (ADR 0025): co-occurrence ×1.0, Last.fm ×0.8, best per pair. The primary genre is the artist's most common library genre, ignoring empty and "unknown". |
| Clustering | Greedy average-linkage agglomeration while the best average link is ≥ 0.25. Ties go to the lowest indices, so the result is deterministic. Clusters with ≥ 3 artists are kept, at most 4, ordered by summed affinity. |
| Mixes | `SmartMixType::DailyCluster(n)` produces `mix_type daily:n`, named "Daily Mix n", with the reason "A, B, C and more". About 70% of tracks come from cluster artists and about 30% from the top 8 similar artists outside the user's top artists. The standard blocked filter, dedupe and diversify steps apply. Mixes expire after 24 h. |
| Lifecycle | `ensure_default_mixes` treats `daily` and `daily:*` as one family. When any member is missing or expired, the whole family is regenerated from a single clustering pass, and members that no longer match are deleted. With fewer than 2 clusters, the single Daily Mix is kept. |

Alternatives: k-means over genre vectors would need a fixed k and gives unstable labels. Clustering by genre alone ignores "fans also like" links between genres. Generating one mix per top genre already exists as the genre mixes and does not group artists.

Consequences: the cost is one extra `artist_similarity` query over at most 40×40 pairs per day. No frontend change is needed, because `DiscoveryView` renders smart mixes generically. Cluster membership shifts as taste changes, so "Daily Mix 2" is not guaranteed to keep the same style from day to day.

Tests: `src-tauri/tests/daily_cluster_tests.rs` covers two disjoint genre groups producing 2 deterministic clusters, similarity linking artists across genres, a history of rock plus jazz persisting `daily:1` and `daily:2` with one genre each while deleting an expired `daily:3`, and a cold start keeping the single `daily`.
