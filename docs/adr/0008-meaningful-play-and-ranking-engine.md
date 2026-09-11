# ADR 0008: Meaningful Play Thresholds, Event-Driven History Logging, and Multi-Factor Ranking Engine

## Context
A primary differentiator for an intelligent local music player is accurate listening history tracking, user preference feedback (likes/dislikes), and personalized ranking. Simple naive play-count metrics suffer from severe skew: accidental skips, preview taps, or fast-forwarding inflate play counts, while long tracks are undercounted. Furthermore, rankings must support dynamic time windows (today, 7 days, 30 days, 6 months, 1 year, all-time) and blend multiple signals (recency, completion rate, explicit user feedback) rather than raw play counts alone.

## Options Considered
1. **Frontend-Driven Play Logging**: Frontend decides when a track has been "listened to" and sends an IPC command `LogPlay`. Rejected because the frontend is thin and untrusted; UI reloads, background play, or frontend crashes would lead to missed or corrupted history records.
2. **Synchronous DB Logging in Audio Loop**: Write to SQLite every time a track ends or pauses directly inside `PlaybackService`. Rejected because database I/O inside playback state transitions introduces jitter and potential audio stuttering.
3. **Event-Driven History Service with Independent Ranking Engine**:
   - `PlaybackService` emits domain events (`PlaybackStarted`, `PlaybackPaused`, `PlaybackResumed`, `PlaybackStopped`, `PlaybackCompleted`, `PlaybackPositionChanged`) over `EventBus`.
   - `HistoryService` subscribes asynchronously to `EventBus`, accumulates session state (`accumulated_duration_ms`, `seek_count`, completion flag), and flushes history sessions to SQLite upon track change or stop.
   - Meaningful play qualification is strictly defined as: `listened_seconds >= 30.0` OR `completion_percentage >= 50.0%` OR `completed == true`.
   - `RankingEngine` computes multi-factor scores with time-decay recency weighting, completion rate weighting, and explicit preference weighting (+30% boost for liked tracks, excluded/downweighted for disliked tracks).
   - SQL queries compute aggregated rankings across `Track`, `Artist`, `Album`, and `Genre` dimensions across rolling time windows.

## Decision
Adopt the **Event-Driven History Service** with **Meaningful Play Thresholds** and the **Multi-Factor Ranking Engine**.

## Reasoning
* **Decoupled Architecture**: `PlaybackService` remains purely focused on audio playback; `HistoryService` runs concurrently in the background.
* **Accuracy**: Accidental clicks under 30 seconds (or <50% for short audio) are logged as sessions but marked `is_meaningful = false`, preventing corrupted statistics.
* **Rich Analytics**: Multi-factor scoring provides realistic top rankings across tracks, artists, albums, and genres without requiring external services like Last.fm or Spotify.
* **Offline-First**: All history, stats, and rankings run on local SQLite tables (`listening_history`, `track_stats`, `user_feedback`) with zero external network dependencies.

## Consequences
* History sessions are durably preserved for long-term taste profiling and recommendation generation (Phase 5).
* User feedback (likes/dislikes) immediately updates preferences and propagates into ranking queries.
