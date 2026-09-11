# Discovery & Missing Music Matching Architecture

## 1. Overview
The discovery subsystem connects the local music library with the broader musical universe. It allows users to:
1. Discover tracks and albums outside their local library tailored to their taste affinities.
2. Automatically check whether discovered tracks are already owned locally using a fuzzy matching engine.
3. Maintain a persistent download wishlist (`Wishlist`) for missing tracks.
4. Prepare missing tracks for download search via Soulseek (Phase 8).

---

## 2. Fuzzy Track Matching Engine (`discovery/matcher.rs`)

When external tracks are surfaced (e.g. via Spotify or MusicBrainz), the matcher compares them against the local library using a multi-factor confidence scoring algorithm:

### Normalization Pipeline
1. Unicode canonical decomposition (`nfkd`).
2. Case-folding to lowercase.
3. Stripping of non-alphanumeric punctuation, remaster indicators (`(2011 Remaster)`, `[Deluxe Edition]`, `feat. ...`).
4. Whitespace trimming and compression.

### Similarity Metrics
* **Title Similarity**: Jaro-Winkler similarity ($S_{\text{title}} \in [0.0, 1.0]$).
* **Artist Similarity**: Jaro-Winkler similarity ($S_{\text{artist}} \in [0.0, 1.0]$).
* **Duration Difference**: $\Delta t = |t_{\text{external}} - t_{\text{local}}|$ in seconds.

### Match Classification Thresholds

| Status | Conditions | Description |
| :--- | :--- | :--- |
| **`EXACT_MATCH`** | $S_{\text{title}} \ge 0.95$, $S_{\text{artist}} \ge 0.90$, $\Delta t \le 3.0\text{s}$ | Definitively already in local library. |
| **`LIKELY_MATCH`** | $S_{\text{title}} \ge 0.85$, $S_{\text{artist}} \ge 0.80$, $\Delta t \le 8.0\text{s}$ | High probability of being in local library (e.g. slightly different master). |
| **`POSSIBLE_MATCH`** | $S_{\text{title}} \ge 0.75$, $S_{\text{artist}} \ge 0.70$ | Plausible match (e.g. live version, acoustic edit). |
| **`NOT_FOUND`** | Confidence $< 0.70$ | Not present locally; candidate for Wishlist or download. |

---

## 3. External Track Catalog (`external_tracks` table)
External tracks discovered through recommendations or manual search are tracked in the SQLite `external_tracks` table:
* `provider`, `provider_id`: Unique external identifier.
* `title`, `artist`, `album`, `duration_secs`, `cover_art_url`.
* `match_status`: `EXACT_MATCH`, `LIKELY_MATCH`, `POSSIBLE_MATCH`, `NOT_FOUND`.
* `matched_local_track_id`: Foreign key reference to local `tracks.id` if matched.

---

## 4. Wishlist Subsystem (`discovery/wishlist.rs`)
Missing tracks can be added to the user's personal wishlist with lifecycle statuses:
* **`WANT`**: Actively sought for download.
* **`IGNORE`**: Dismissed by user; not wanted.
* **`ALREADY_OWN`**: User confirmed ownership outside the indexed library.
* **`DOWNLOADED`**: Acquired through Soulseek or manual placement and ready for library scanning.

Users can add notes, filter by status, and query wishlist items via `Query::GetWishlist`.
