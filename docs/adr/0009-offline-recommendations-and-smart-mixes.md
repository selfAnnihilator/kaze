# ADR 0009: Offline Recommendations, Explainability Scoring, and Smart Mixes

## Context
Standard streaming music players rely on massive cloud infrastructure, collaborative filtering models across millions of users, and centralized neural recommenders to suggest music. For a local-first desktop audio player operating 100% offline on a personal music library, the recommendation system must:
1. Operate entirely locally without network access or cloud dependencies.
2. Form accurate short-term and long-term taste affinity models from sparse, personal listening data.
3. Be completely transparent: provide human-readable factor explanations for every suggested track rather than an opaque black-box score.
4. Avoid repetitive recommendation fatigue by applying temporal recency dampening.
5. Support dynamic, auto-generated smart mixes (`DailyMix`, `OnRepeat`, `ForgottenFavorites`, `GenreMix`, `ArtistMix`, `LateNight`, `Discovery`).

## Options Considered
1. **Simple Random or Shuffle Queuing**: Play random unplayed tracks or tracks by top artists. Rejected because it lacks personalization, produces jarring genre shifts, and fails to identify forgotten gems or curate focused mixes.
2. **Local Embedding Neural Network (e.g. ONNX/Wasm)**: Run heavy audio analysis embeddings locally. Rejected for core Phase 5 because of binary size bloat (hundreds of MBs of model weights), CPU/GPU resource contention, and lack of human-interpretable reasoning.
3. **Multi-Factor Transparent Heuristic Engine with Dual-Window Affinity Profiling**:
   - `TasteProfileEngine`: Computes dual-window affinities across artists, genres, and eras (60% short-term 14-day recency + 40% long-term 90-day baseline), persisting affinities in `user_preferences`.
   - `ScoringEngine`: Scores candidate tracks using weighted factors:
     - Artist Affinity (35%)
     - Genre Affinity (25%)
     - Historical Track Performance & Completion (20%)
     - Era Affinity (10%)
     - Explicit Likes (+25% boost) / Dislikes (immediate exclusion)
     - Discovery Bonus (+20%) for unplayed local tracks matching favorite styles
     - Repetition Penalty (-50% if played < 24h, -30% if played < 72h, -15% if played < 7d)
     - Forgotten Favorite Bonus (+15% if unplayed for > 30 days)
   - Explainability: Emits structured, human-readable reason strings alongside every recommendation.
   - `SmartMixGenerator`: Dynamically generates and persists curated smart playlists (`Daily`, `OnRepeat`, `ForgottenFavorites`, `Genre`, `Artist`, `LateNight`, `Discovery`) in the `playlists` and `playlist_tracks` tables.

## Decision
Adopt the **Multi-Factor Transparent Heuristic Engine with Dual-Window Affinity Profiling and Explainability Breakdown**.

## Reasoning
* **100% Local & Instantaneous**: Computations execute in sub-millisecond SQLite queries and in-memory scoring passes with zero external calls or heavy model dependencies.
* **Explainability**: Users can see exactly why a track was recommended (e.g., `"Favorite genre Rock: 85% match"`, `"Liked track boost"`, `"Repetition penalty: Played in last 24h (-50%)"`).
* **Controlled Entropy & Diversity**: Artist diversity constraints (maximum 2 tracks per artist per mix) and exploration bonuses prevent monotonous playlists and surface forgotten or unheard tracks in the user's collection.

## Consequences
* All recommendations operate seamlessly offline.
* Recommendation sessions and transparent reasons are logged in `recommendation_sessions` and `recommendations` tables.
* Smart mixes can be refreshed on-demand, scheduled, or queued into the playback engine.
