# Local Recommendation Engine & Smart Mix Architecture

## 1. Overview
The recommendation engine is a **100% offline, local-first intelligence system** embedded in the Rust backend. It analyzes local listening history, track completion rates, explicit feedback (likes/dislikes), and temporal patterns to generate personalized taste profiles, local recommendations, and dynamic smart mixes.

All computations are transparent: every recommended track includes an explainability breakdown specifying exactly why it was recommended.

---

## 2. Taste Profile Engine (`recommendations/taste.rs`)
The taste profile captures user musical affinities across three dimensions:
1. **Artists**: Short-term vs long-term affinity based on play counts, total listening time, completions, and likes.
2. **Genres**: Derived from track and artist genre associations.
3. **Eras / Decades**: Inferred from release years (e.g. 70s, 80s, 90s, 00s, 10s, 20s).

### Dual-Window Affinity
* **Short-Term Affinity (Last 14 Days)**: Reflects immediate obsessions and current listening phases. Uses an exponential half-life decay ($\lambda \approx 7\text{ days}$).
* **Long-Term Affinity (All-Time / Last 90 Days)**: Reflects persistent preferences and baseline taste.
* **Blended Weighting**:
  $$\text{Affinity} = 0.6 \times \text{ShortTerm} + 0.4 \times \text{LongTerm}$$

---

## 3. Transparent Scoring Algorithm (`recommendations/scoring.rs`)

Candidate tracks are scored across 5 weighted factors:

| Factor | Weight | Description |
| :--- | :--- | :--- |
| **Artist Affinity** | 35% | Affinity score of the track's artist in user profile |
| **Genre Affinity** | 25% | Affinity score of the track's genre |
| **Track Performance** | 20% | Historical completion rate + like boost (+0.3), dislike penalty (-1.0) |
| **Repetition Penalty** | -15% to -50% | Penalty for tracks played recently (<24h: -50%, <72h: -30%, <7d: -15%) |
| **Exploration Bonus** | +15% | Controlled entropy bonus for unplayed or rarely played tracks from high-affinity genres |

### Repetition Avoidance & Entropy Control
To prevent recommendation fatigue:
* Tracks played within the last 24 hours receive a 50% score penalty.
* Disliked tracks are entirely excluded.
* A max-track-per-artist limit (e.g. 2 tracks per artist per mix) ensures diverse playlists.

### Human-Readable Factor Explanations
Every recommendation includes a structured list of reasons:
- `"Top Artist: Radiohead (Affinity: 0.92)"`
- `"Favorite Genre: Alternative Rock"`
- `"Liked Track (+30% boost)"`
- `"Discovery: Unplayed track from a favorite artist"`
- `"Repetition Avoidance: -30% (Played 2 days ago)"`

---

## 4. Smart Mix Generators (`recommendations/mixes.rs`)

Smart mixes are dynamic, auto-generated playlists persisted in the SQLite `playlists` table with `is_smart_mix = 1`:

1. **Daily Mix (`SmartMixType::Daily`)**:
   - 25 tracks.
   - Composition: 60% high-affinity tracks, 20% forgotten favorites, 20% exploration picks.
2. **On Repeat (`SmartMixType::OnRepeat`)**:
   - 20 tracks.
   - Most played, highest-completion tracks from the last 14 days.
3. **Forgotten Favorites (`SmartMixType::ForgottenFavorites`)**:
   - 25 tracks.
   - Tracks with high historical play counts or explicit likes, but unplayed for over 30 days.
4. **Genre Mix (`SmartMixType::Genre(name)`)**:
   - 25 tracks.
   - Curated journey through a specific genre, ordering by affinity with exploration tracks interspersed.
5. **Artist Mix (`SmartMixType::Artist(name)`)**:
   - 25 tracks.
   - Blends tracks by the target artist with tracks by artists who share similar genres.

---

## 5. Storage & Persistence
- **Playlists & Tracks**: Stored in `playlists` and `playlist_tracks`.
- **Taste Affinities**: Stored in `user_preferences` (`entity_type`, `entity_id`, `short_term_affinity`, `long_term_affinity`).
- **Recommendation Sessions**: Stored in `recommendation_sessions` and `recommendations` with JSON-encoded reasons.
