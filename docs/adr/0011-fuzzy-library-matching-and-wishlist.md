# ADR 0011: Fuzzy Track Matching and Missing Music Wishlist Subsystem

## Context
When discovering music outside the user's local collection (via Spotify, MusicBrainz, or genre/artist exploration), the system must determine whether a candidate track is already owned in the local library or is truly missing.

Comparing external tracks against a local library presents several real-world challenges:
1. **Metadata Inconsistencies**: Local and external metadata frequently differ in formatting, featuring credits (`feat. Drake` vs `ft. Drake`), parenthetical notes (`(2011 Remaster)`, `[Deluxe Edition]`, `(Live at Wembley)`), and leading articles (`The Beatles` vs `Beatles`).
2. **Audio Duration Deltas**: Different masterings, edits (radio edit vs album version), and track lead-in silences cause audio duration differences of several seconds even for identical recordings.
3. **Wishlist Lifecycle**: Missing music identified by the user needs persistent tracking with clear lifecycle states (`WANT`, `IGNORE`, `ALREADY_OWN`, `DOWNLOADED`) to feed automated acquisition via Soulseek (Phase 8).
4. **Relational Integrity**: Foreign keys between `wishlist` and `external_tracks` must be respected without throwing errors when users add manual or arbitrary entries.

## Options Considered
1. **Exact String Matching (SQL `WHERE title = ? AND artist = ?`)**:
   - Rejected because minor differences like "(Remastered)" or "The" cause false negatives, leading to redundant recommendations and duplicate downloads.
2. **External Cloud Fingerprinting (AcoustID / Shazam APIs)**:
   - Rejected because it requires uploading audio fingerprints to third-party servers, introduces network latency, and cannot match external metadata before audio is downloaded.
3. **Multi-Factor Fuzzy Matching Pipeline + Wishlist State Machine**:
   - Implement `FuzzyTrackMatcher` with a multi-stage string normalization pipeline (lowercasing, parenthetical noise stripping, featured artist truncation, punctuation stripping, and leading article removal).
   - Use Jaro-Winkler string similarity ($S \in [0.0, 1.0]$) for title and artist combined with duration delta ($\Delta t = |t_{\text{ext}} - t_{\text{local}}|$) tolerances:
     - `EXACT_MATCH`: $S_{\text{title}} \ge 0.95$, $S_{\text{artist}} \ge 0.90$, $\Delta t \le 4.0\text{s}$.
     - `LIKELY_MATCH`: $S_{\text{title}} \ge 0.85$, $S_{\text{artist}} \ge 0.80$, $\Delta t \le 10.0\text{s}$.
     - `POSSIBLE_MATCH`: $S_{\text{title}} \ge 0.75$, $S_{\text{artist}} \ge 0.70$.
     - `NOT_FOUND`: otherwise.
   - Implement `WishlistManager` managing items in SQLite with foreign key validation, status transitions (`WANT`, `IGNORE`, `ALREADY_OWN`, `DOWNLOADED`), and status filtering.
   - Implement `DiscoveryCoordinator` linking external discovery recommendations with taste affinities and local library status.

## Decision
Adopt the **Multi-Factor Fuzzy Matching Pipeline + Wishlist State Machine** (`FuzzyTrackMatcher`, `WishlistManager`, `DiscoveryCoordinator`).

## Reasoning
* **High Precision & Recall**: Stripping noise and using Jaro-Winkler metrics correctly associates tracks despite remaster tags or alternate artist representations.
* **Duration Awareness**: Audio duration comparison distinguishes between original cuts, live takes, and alternate versions.
* **Local-First & Offline**: Fuzzy matching runs completely in-memory against local SQLite tables without requiring internet connectivity.
* **Seamless Downloader Transition**: Prepares clean metadata queries for Phase 8 (Soulseek integration).

## Consequences
* External tracks can be evaluated against the entire local library in milliseconds.
* The application surfaces actionable recommendations for unowned tracks matching the user's taste profile.
* Users can manage a structured download wishlist that directly feeds the Phase 8 Soulseek acquisition subsystem.
