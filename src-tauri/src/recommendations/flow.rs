use crate::core::error::{AppError, AppResult};
use crate::recommendations::local::RecommendedTrackDetail;
use crate::recommendations::scoring::{CandidateTrack, ScoringEngine};
use crate::recommendations::taste::{TasteProfile, TasteProfileEngine};
use chrono::Utc;

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Recorded evidence only; presentation belongs to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FlowExplanation {
    pub primary: String,
    pub is_liked: bool,
    pub is_unplayed: bool,
    pub play_count: Option<i64>,
    pub days_since_played: Option<i64>,
    pub artist_affinity: Option<f64>,
    pub artist_name: Option<String>,
    pub familiarity_ratio: Option<f64>,
    pub selected_at: Option<i64>,
    pub from_existing_queue: bool,
}

/// A track selected by the Flow engine with deterministic scoring and explanation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowTrackItem {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_secs: f64,
    pub cover_art_url: Option<String>,
    pub score: f64,
    pub reason: String,
    pub reason_category: String,
    pub explanation: FlowExplanation,
}

/// Lightweight persistent state for Flow session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowSessionState {
    #[serde(default)]
    pub generation: Option<u64>,
    pub is_active: bool,
    pub familiarity_ratio: f64, // 0.0 = pure rediscover, 1.0 = pure familiar
    pub updated_at: i64,
    pub last_track_id: Option<String>,
    pub last_track_title: Option<String>,
    pub last_track_artist: Option<String>,
    pub flow_label: String,
}

/// A track in the Recent Rotation section.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecentTrackItem {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_secs: f64,
    pub cover_art_url: Option<String>,
    pub started_at: i64,
}

/// A track in the Rediscover section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RediscoverTrackItem {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_secs: f64,
    pub cover_art_url: Option<String>,
    pub last_played_at: Option<i64>,
    pub play_count: i64,
    pub is_liked: bool,
    pub reason: String,
    pub explanation: FlowExplanation,
}

/// Spotlight moment from the user's library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryMoment {
    pub album_id: String,
    pub album_title: String,
    pub artist_name: String,
    pub cover_art_path: Option<String>,
    pub total_tracks: i64,
    pub release_year: Option<i64>,
    pub subtitle: String,
}

/// Listening summary for the last 7 days.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct YourWeekStats {
    pub total_seconds: f64,
    pub track_count: u64,
    pub artist_count: u64,
    pub formatted_time: String,
}

/// Composite payload for Kaze Home screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeOverview {
    pub flow_session: Option<FlowSessionState>,
    pub recent_rotation: Vec<RecentTrackItem>,
    pub rediscover: Vec<RediscoverTrackItem>,
    pub for_you: Vec<RecommendedTrackDetail>,
    pub library_moment: Option<LibraryMoment>,
    pub your_week: Option<YourWeekStats>,
}

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct FlowCandidateRow {
    id: String,
    title: String,
    artist_id: Option<String>,
    artist_name: Option<String>,
    album_id: Option<String>,
    album_title: Option<String>,
    genre_name: Option<String>,
    year: Option<i64>,
    duration_secs: f64,
    cover_art_url: Option<String>,
    play_count: Option<i64>,
    completion_count: Option<i64>,
    skip_count: Option<i64>,
    last_played_at: Option<i64>,
    manual_like: Option<i64>,
    is_online: bool,
    preview_url: Option<String>,
}

pub struct FlowEngine {
    pool: SqlitePool,
    taste_engine: Arc<TasteProfileEngine>,
    home_picks: tokio::sync::Mutex<HashMap<String, (i64, i64, Vec<RecommendedTrackDetail>)>>,
}

impl FlowEngine {
    pub fn new(pool: SqlitePool) -> Self {
        let taste_engine = Arc::new(TasteProfileEngine::new(pool.clone()));
        Self {
            pool,
            taste_engine,
            home_picks: tokio::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Generates next queue tracks using deterministic Flow scoring.
    ///
    /// Steering: `familiarity_ratio`:
    ///   1.0 -> High familiarity (top favorites, high affinity)
    ///   0.5 -> Balanced (familiar + forgotten favorites + fresh picks)
    ///   0.0 -> High rediscovery (unplayed gems, tracks unheard for months)
    pub async fn generate_flow_tracks(
        &self,
        user_id: &str,
        familiarity_ratio: f64,
        limit: usize,
        exclude_track_ids: &[String],
    ) -> AppResult<Vec<FlowTrackItem>> {
        self.generate_flow_tracks_after(user_id, familiarity_ratio, limit, exclude_track_ids, None)
            .await
    }

    pub async fn generate_flow_tracks_after(
        &self,
        user_id: &str,
        familiarity_ratio: f64,
        limit: usize,
        exclude_track_ids: &[String],
        previous_artist: Option<&str>,
    ) -> AppResult<Vec<FlowTrackItem>> {
        self.generate_candidates(
            user_id,
            familiarity_ratio,
            limit,
            exclude_track_ids,
            previous_artist,
            false,
        )
        .await
    }

    /// Runtime Flow includes account history and taste-matched online catalog metadata.
    /// Selection performs no network/audio work; the shared player resolves a song on play.
    pub async fn generate_playback_tracks_after(
        &self,
        user_id: &str,
        familiarity_ratio: f64,
        limit: usize,
        exclude_track_ids: &[String],
        previous_artist: Option<&str>,
    ) -> AppResult<Vec<FlowTrackItem>> {
        self.generate_candidates(
            user_id,
            familiarity_ratio,
            limit,
            exclude_track_ids,
            previous_artist,
            true,
        )
        .await
    }

    async fn generate_candidates(
        &self,
        user_id: &str,
        familiarity_ratio: f64,
        limit: usize,
        exclude_track_ids: &[String],
        previous_artist: Option<&str>,
        include_online: bool,
    ) -> AppResult<Vec<FlowTrackItem>> {
        if !familiarity_ratio.is_finite() {
            return Err(AppError::Validation("Flow steering must be finite".into()));
        }
        let limit = limit.min(100);
        if limit == 0 {
            return Ok(Vec::new());
        }
        let ratio = familiarity_ratio.clamp(0.0, 1.0);
        let profile = self.taste_engine.compute_taste_profile(user_id).await?;
        let signals = super::signals::SongSignals::load(&self.pool, user_id).await?;
        let now = Utc::now().timestamp();

        let mut candidates = sqlx::query_as::<_, FlowCandidateRow>(
            "SELECT t.id, t.title, t.artist_id, a.name as artist_name,
                    t.album_id, al.title as album_title, g.name as genre_name, t.year,
                    t.duration_secs,
                    CASE WHEN t.has_cover_art = 1 THEN '/api/cover/' || t.id ELSE NULL END as cover_art_url,
                    ts.play_count, ts.completion_count, ts.skip_count, ts.last_played_at,
                    ts.manual_like, 0 as is_online, NULL as preview_url
             FROM tracks t
             LEFT JOIN artists a ON a.id = t.artist_id
             LEFT JOIN albums al ON al.id = t.album_id
             LEFT JOIN genres g ON g.id = t.genre_id
             LEFT JOIN track_statistics ts ON ts.track_id = t.id AND ts.user_id = ?
             WHERE t.format != 'online' AND t.file_path NOT LIKE 'online://%'"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        if include_online {
            let online = sqlx::query_as::<_, FlowCandidateRow>(
                "SELECT e.id, e.title, a.id as artist_id, e.artist as artist_name,
                        t.album_id, e.album as album_title, COALESCE(g.name, e.genre) as genre_name, t.year,
                        COALESCE(e.duration_secs, t.duration_secs, 210.0) as duration_secs,
                        e.cover_art_url, ts.play_count, ts.completion_count, ts.skip_count, ts.last_played_at,
                        CASE WHEN ts.manual_like = -1 THEN -1
                             WHEN liked.track_id IS NOT NULL THEN 1 ELSE ts.manual_like END as manual_like,
                        1 as is_online, e.preview_url
                 FROM external_tracks e
                 LEFT JOIN tracks t ON t.id = e.id
                 LEFT JOIN artists a ON a.normalized_name = lower(trim(e.artist))
                 LEFT JOIN genres g ON g.id = t.genre_id
                 LEFT JOIN track_statistics ts ON ts.track_id = e.id AND ts.user_id = ?
                 LEFT JOIN (
                    SELECT DISTINCT pt.track_id FROM playlist_tracks pt
                    JOIN playlists p ON p.id = pt.playlist_id
                    WHERE p.user_id = ? AND p.name = 'Liked Songs' AND p.is_smart_mix = 0
                 ) liked ON liked.track_id = e.id
                 WHERE (e.provider = 'jiosaavn' OR EXISTS (SELECT 1 FROM online_track_sources source WHERE source.track_id=e.id)) AND (t.id IS NULL OR t.format = 'online' OR t.file_path LIKE 'online://%')
                   AND NOT EXISTS (
                       SELECT 1 FROM tracks local WHERE local.id = e.matched_local_track_id
                         AND local.format != 'online' AND local.file_path NOT LIKE 'online://%'
                   )
                 ORDER BY (liked.track_id IS NOT NULL OR COALESCE(ts.play_count, 0) > 0) DESC,
                          COALESCE(ts.play_count, 0) DESC, e.created_at DESC, e.id
                 LIMIT 2000"
            ).bind(user_id).bind(user_id).fetch_all(&self.pool).await
                .map_err(|e| AppError::Database(e.to_string()))?;
            candidates.extend(
                online
                    .into_iter()
                    .map(|mut c| {
                        let signal =
                            signals.signal(&c.id, &c.title, c.artist_name.as_deref().unwrap_or(""));
                        c.play_count = Some(signal.play_count);
                        c.completion_count = Some(signal.completion_count);
                        c.skip_count = Some(signal.skip_count);
                        c.manual_like = Some(signal.manual_like);
                        c.last_played_at = signal.last_played_at;
                        c
                    })
                    .filter(|c| {
                        let known = c.manual_like == Some(1)
                            || c.last_played_at.is_some()
                            || c.play_count.unwrap_or(0) > 0;
                        let artist_match =
                            profile.artist_affinity(c.artist_name.as_deref().unwrap_or("")) > 0.15;
                        let language_match = signals
                            .identities
                            .get(&c.id)
                            .and_then(|song| song.language.as_ref())
                            .and_then(|language| {
                                profile.language_affinities.get(&language.to_lowercase())
                            })
                            .copied()
                            .unwrap_or(0.0)
                            > 0.15;
                        let genre_match = c.genre_name.as_ref().is_some_and(|genre| {
                            profile.genre_affinities.iter().any(|(name, affinity)| {
                                name.eq_ignore_ascii_case(genre.trim()) && *affinity > 0.15
                            })
                        });
                        known || artist_match || genre_match || language_match
                    }),
            );
        }

        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        let exclude_set: HashSet<&str> = exclude_track_ids.iter().map(|s| s.as_str()).collect();
        let excluded_songs: HashSet<_> = candidates
            .iter()
            .filter(|c| exclude_set.contains(c.id.as_str()) || c.manual_like == Some(-1))
            .map(|c| Self::song_key(&c.title, c.artist_name.as_deref().unwrap_or("")))
            .collect();
        let mut preferred: HashMap<String, (u8, String)> = HashMap::new();
        for candidate in &candidates {
            if !candidate.is_online
                || candidate.play_count.unwrap_or(0) > 0
                || candidate.manual_like == Some(1)
            {
                continue;
            }
            let key = Self::song_key(
                &candidate.title,
                candidate.artist_name.as_deref().unwrap_or(""),
            );
            let rank = crate::recommendations::signals::release_preference(
                &candidate.title,
                candidate.album_title.as_deref(),
            );
            if preferred.get(&key).is_none_or(|(old, _)| rank > *old) {
                preferred.insert(key, (rank, candidate.id.clone()));
            }
        }
        candidates.retain(|candidate| {
            !candidate.is_online
                || candidate.play_count.unwrap_or(0) > 0
                || candidate.manual_like == Some(1)
                || preferred
                    .get(&Self::song_key(
                        &candidate.title,
                        candidate.artist_name.as_deref().unwrap_or(""),
                    ))
                    .is_none_or(|(_, id)| id == &candidate.id)
        });

        let mut seen_songs = HashSet::new();

        struct ScoredCandidate {
            candidate: FlowCandidateRow,
            score: f64,
            reason: String,
            reason_category: String,
            explanation: FlowExplanation,
        }

        let mut scored: Vec<ScoredCandidate> = Vec::new();

        for mut c in candidates {
            if signals.blocked(&c.id, &c.title, c.artist_name.as_deref().unwrap_or("")) {
                continue;
            }
            let signal = signals.signal(&c.id, &c.title, c.artist_name.as_deref().unwrap_or(""));
            c.play_count = Some(signal.play_count);
            c.completion_count = Some(signal.completion_count);
            c.skip_count = Some(signal.skip_count);
            c.last_played_at = signal.last_played_at;
            c.manual_like = Some(signal.manual_like);

            // Disliked tracks are strictly excluded
            if c.manual_like == Some(-1) {
                continue;
            }

            // Excluded tracks (already queued or recently played in session) are skipped
            let identity = Self::song_key(&c.title, c.artist_name.as_deref().unwrap_or(""));
            if exclude_set.contains(c.id.as_str())
                || excluded_songs.contains(&identity)
                || !seen_songs.insert(identity)
            {
                continue;
            }

            let play_count = c.play_count.unwrap_or(0);
            let completion_count = c.completion_count.unwrap_or(0);
            let skip_count = c.skip_count.unwrap_or(0);
            let is_liked = c.manual_like == Some(1);

            let artist_aff = c
                .artist_id
                .as_ref()
                .and_then(|aid| profile.artist_affinities.get(aid))
                .copied()
                .unwrap_or(0.0);

            let genre_aff = c
                .genre_name
                .as_ref()
                .and_then(|g| profile.genre_affinities.get(g.trim()))
                .copied()
                .unwrap_or(0.0);

            // Reuse the existing recommendation scorer for taste, era, completion,
            // explicit feedback and recency. Flow adds only steering/rediscovery.
            let input = CandidateTrack {
                id: c.id.clone(),
                title: c.title.clone(),
                artist_id: c.artist_id.clone(),
                artist_name: c.artist_name.clone(),
                genre_name: c.genre_name.clone(),
                year: c
                    .year
                    .or_else(|| signals.identities.get(&c.id).and_then(|song| song.year)),
                language: signals
                    .identities
                    .get(&c.id)
                    .and_then(|song| song.language.clone()),
                play_count,
                completion_count,
                skip_count,
                last_played_at: c.last_played_at,
                user_feedback: if is_liked { Some("like".into()) } else { None },
            };
            let taste_score = ScoringEngine::score_track(&input, &profile, now).final_score;
            // Familiarity means known songs, rather than every unheard song by a favorite artist.
            let known_strength = if is_liked {
                1.0
            } else {
                ((play_count.max(0) as f64).ln_1p() / 6.0_f64.ln_1p()).min(1.0)
            };
            let familiarity_score = taste_score * 0.4 + known_strength * 0.6;

            // 2. Rediscovery Component (favors forgotten tracks and unplayed library gems)
            let mut rediscovery_score = 0.0;

            if let Some(last_played) = c.last_played_at {
                let age_secs = (now - last_played).max(0);
                let days = age_secs / 86400;

                if days >= 60 {
                    // Over 2 months unplayed
                    rediscovery_score += 0.50;
                    if is_liked || play_count >= 2 {
                        rediscovery_score += 0.30; // Beloved past favorite
                    }
                } else if days >= 21 {
                    // 3 to 8 weeks unplayed
                    rediscovery_score += 0.35;
                } else if days >= 7 {
                    rediscovery_score += 0.15;
                }
            } else if play_count == 0 {
                // Never played library track!
                rediscovery_score += 0.40;
                // If it aligns with known taste, boost it as a discovery pick
                if artist_aff > 0.15 || genre_aff > 0.15 {
                    rediscovery_score += 0.25;
                }
            }

            // 3. Blend based on steering slider
            let mut final_score = (familiarity_score * ratio) + (rediscovery_score * (1.0 - ratio));

            // Preserve some taste relevance at the rediscovery end.
            final_score += familiarity_score * (1.0 - ratio) * 0.20;
            if let Some(last_played) = c.last_played_at {
                let age = (now - last_played).max(0);
                let dampening = if age < 86400 {
                    0.20
                } else if age < 3 * 86400 {
                    0.50
                } else if age < 7 * 86400 {
                    0.80
                } else {
                    1.0
                };
                // Familiar score already includes the shared scorer's recency penalty.
                final_score -= rediscovery_score * (1.0 - ratio) * (1.0 - dampening);
            }
            if skip_count > 2 && skip_count > completion_count {
                final_score -= ((skip_count - completion_count) as f64 * 0.05).min(0.35);
            }
            final_score = final_score.max(0.0);
            if c.is_online
                && !is_liked
                && (final_score <= 0.0 || (play_count == 0 && skip_count > 0))
            {
                // An online exploration slot must not force repeatedly skipped songs.
                continue;
            }
            let (reason, reason_category) = Self::explain(&c, &profile, now);

            let mut explanation = Self::evidence(&c, &profile, now);
            explanation.familiarity_ratio = Some(ratio);
            explanation.selected_at = Some(now);
            scored.push(ScoredCandidate {
                candidate: c,
                score: final_score,
                reason,
                reason_category,
                explanation,
            });
        }

        // Sort descending by final score
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.candidate.id.cmp(&b.candidate.id))
        });

        // Revisit deferred artists after every choice. Only relax spacing/caps
        // when no candidate can satisfy them (e.g. a one-artist library).
        let mut selected = Vec::with_capacity(limit);
        let mut artist_counts: HashMap<String, usize> = HashMap::new();
        let mut last_artist = previous_artist.map(str::to_owned);
        let mut selected_online = 0;
        let max_per_artist = if limit <= 10 { 2 } else { 3 };
        while !scored.is_empty() && selected.len() < limit {
            let key = |item: &ScoredCandidate| {
                item.candidate
                    .artist_name
                    .clone()
                    .unwrap_or_else(|| "Unknown Artist".into())
            };
            // Reserve some exploration slots for online candidates so a large local
            // library cannot hide them through deterministic score ties. Familiar
            // still lets known online favorites compete on their listening scores.
            let online_due = include_online
                && selected_online
                    < (((selected.len() + 1) as f64 * (1.0 - ratio) * 0.5).round() as usize);
            let spaced = |item: &ScoredCandidate| {
                let artist = key(item);
                last_artist.as_ref() != Some(&artist)
                    && artist_counts.get(&artist).copied().unwrap_or(0) < max_per_artist
            };
            let index = scored
                .iter()
                .position(|item| spaced(item) && (!online_due || item.candidate.is_online))
                .or_else(|| scored.iter().position(spaced))
                .or_else(|| {
                    scored
                        .iter()
                        .position(|item| last_artist.as_ref() != Some(&key(item)))
                })
                .unwrap_or(0);
            let item = scored.remove(index);
            if item.candidate.is_online {
                selected_online += 1;
            }
            if include_online && item.candidate.is_online {
                crate::recommendations::mixes::ensure_online_track(
                    &self.pool,
                    &item.candidate.id,
                    &item.candidate.title,
                    item.candidate.artist_name.as_deref(),
                    item.candidate.album_title.as_deref(),
                    Some(item.candidate.duration_secs),
                    item.candidate.cover_art_url.as_deref(),
                    item.candidate.preview_url.as_deref(),
                )
                .await?;
            }
            let artist = key(&item);
            *artist_counts.entry(artist.clone()).or_insert(0) += 1;
            last_artist = Some(artist);
            selected.push(FlowTrackItem {
                track_id: item.candidate.id,
                title: item.candidate.title,
                artist: item
                    .candidate
                    .artist_name
                    .unwrap_or_else(|| "Unknown Artist".into()),
                album: item.candidate.album_title,
                duration_secs: item.candidate.duration_secs,
                cover_art_url: item.candidate.cover_art_url,
                score: item.score,
                reason: item.reason,
                reason_category: item.reason_category,
                explanation: item.explanation,
            });
        }

        Ok(selected)
    }

    /// Fetches the composite Kaze Home overview:
    /// - Flow Session
    /// - Recent Rotation
    /// - Rediscover
    /// - For You
    /// - Library Moment
    /// - Your Week
    pub async fn get_home_overview(&self, user_id: &str) -> AppResult<HomeOverview> {
        let now = Utc::now().timestamp();

        // 1. Flow session state from application_settings
        let flow_session = self.get_flow_session(user_id).await?;

        // 2. Recent Rotation (distinct tracks played recently)
        let recent_rotation = sqlx::query_as::<_, RecentTrackItem>(
            "SELECT h.track_id, t.title,
                    COALESCE(a.name, 'Unknown Artist') as artist,
                    al.title as album,
                    t.duration_secs,
                    CASE WHEN t.has_cover_art = 1 THEN '/api/cover/' || t.id ELSE NULL END as cover_art_url,
                    MAX(h.started_at) as started_at
             FROM playback_history h
             JOIN tracks t ON t.id = h.track_id
             LEFT JOIN artists a ON a.id = t.artist_id
             LEFT JOIN albums al ON al.id = t.album_id
             WHERE h.user_id = ? AND t.format != 'online' AND t.file_path NOT LIKE 'online://%'
             GROUP BY h.track_id
             ORDER BY started_at DESC
             LIMIT 8"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        // 3. Rediscover: Tracks previously played or liked, unplayed in > 14 days
        let cutoff_rediscover = now - (14 * 86400);
        let rediscover_rows: Vec<(
            String,
            String,
            Option<String>,
            Option<String>,
            f64,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            i64,
        )> = sqlx::query_as(
            "SELECT t.id, t.title, a.name as artist, al.title as album,
                    t.duration_secs, ts.last_played_at, ts.play_count, ts.manual_like,
                    t.has_cover_art
             FROM track_statistics ts
             JOIN tracks t ON t.id = ts.track_id
             LEFT JOIN artists a ON a.id = t.artist_id
             LEFT JOIN albums al ON al.id = t.album_id
             WHERE ts.user_id = ? AND COALESCE(ts.manual_like, 0) != -1
               AND t.format != 'online' AND t.file_path NOT LIKE 'online://%'
               AND (ts.manual_like = 1 OR ts.play_count >= 2)
               AND (ts.last_played_at IS NULL OR ts.last_played_at < ?)
             ORDER BY (COALESCE(ts.play_count, 0) * 2 + CASE WHEN ts.manual_like = 1 THEN 8 ELSE 0 END) DESC
             LIMIT 8"
        )
        .bind(user_id)
        .bind(cutoff_rediscover)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        let rediscover = rediscover_rows
            .into_iter()
            .map(
                |(id, title, artist, album, dur, last_played, play_count, like, has_art)| {
                    let is_liked = like == Some(1);
                    let days_ago = last_played
                        .map(|lp| (now - lp).max(0) / 86400)
                        .unwrap_or(30);

                    let reason = if is_liked && last_played.is_some() && days_ago >= 60 {
                        format!(
                            "Liked song · Not heard in {} months",
                            (days_ago / 30).max(2)
                        )
                    } else if is_liked {
                        "One of your liked tracks".to_string()
                    } else if last_played.is_none() {
                        "Previously played from your library".to_string()
                    } else if days_ago >= 90 {
                        format!(
                            "Played often · Unplayed for {} months",
                            (days_ago / 30).max(3)
                        )
                    } else {
                        format!("Unheard in {} days", days_ago)
                    };

                    RediscoverTrackItem {
                        cover_art_url: if has_art == 1 {
                            Some(format!("/api/cover/{}", id))
                        } else {
                            None
                        },
                        track_id: id,
                        title,
                        artist: artist.unwrap_or_else(|| "Unknown Artist".into()),
                        album,
                        duration_secs: dur,
                        last_played_at: last_played,
                        play_count: play_count.unwrap_or(0),
                        is_liked,
                        explanation: FlowExplanation {
                            primary: if is_liked { "liked" } else { "rediscovery" }.into(),
                            is_liked,
                            play_count,
                            days_since_played: last_played.map(|lp| (now - lp).max(0) / 86400),
                            selected_at: Some(now),
                            ..Default::default()
                        },
                        reason,
                    }
                },
            )
            .collect();

        // 4. Keep stable picks until listening changes or the short cache expires.
        let history_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM playback_history WHERE user_id = ?")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        let mut picks = self.home_picks.lock().await;
        if picks
            .get(user_id)
            .map(|(time, count, _)| now - time >= 60 || *count != history_count)
            .unwrap_or(true)
        {
            let for_you = self
                .generate_playback_tracks_after(user_id, 1.0, 8, &[], None)
                .await?
                .into_iter()
                .map(|t| RecommendedTrackDetail {
                    track_id: t.track_id,
                    title: t.title,
                    artist_name: Some(t.artist),
                    album_title: t.album,
                    genre_name: None,
                    duration_secs: t.duration_secs,
                    score: t.score,
                    explanation: Some(t.explanation),
                    is_discovery: t.reason_category == "library_exploration",
                    reasons: vec![crate::recommendations::scoring::RecommendationReason {
                        factor: t.reason_category,
                        description: t.reason,
                        weight: t.score,
                    }],
                })
                .collect();
            picks.insert(user_id.into(), (now, history_count, for_you));
        }
        let for_you = picks.get(user_id).unwrap().2.clone();
        drop(picks);

        // 5. Library Moment: Spotlight on an album from user's library
        // Prefer an album with tracks that was either listened to in the past or has never been finished
        let library_moment_row: Option<(String, String, Option<String>, Option<String>, Option<i64>, Option<i64>)> =
            sqlx::query_as(
                "SELECT al.id, al.title, a.name as artist, al.cover_art_path, al.total_tracks, al.release_year
                 FROM albums al
                 LEFT JOIN artists a ON a.id = al.artist_id
                 WHERE al.id IN (
                     SELECT DISTINCT album_id FROM tracks WHERE album_id IS NOT NULL
                 )
                 ORDER BY RANDOM()
                 LIMIT 1"
            )
            .fetch_optional(&self.pool)
            .await
            .unwrap_or(None);

        let library_moment =
            library_moment_row.map(|(aid, atitle, aname, art_path, ttracks, ryear)| {
                LibraryMoment {
                    album_id: aid,
                    album_title: atitle,
                    artist_name: aname.unwrap_or_else(|| "Various Artists".into()),
                    cover_art_path: art_path,
                    total_tracks: ttracks.unwrap_or(0),
                    release_year: ryear,
                    subtitle: "Revisit an album from your library".to_string(),
                }
            });

        // 6. Your Week: Last 7 days summary from playback_history
        let week_cutoff = now - (7 * 86400);
        let week_summary: Option<(Option<f64>, Option<i64>, Option<i64>)> = sqlx::query_as(
            "SELECT SUM(h.seconds_listened) as total_secs,
                    COUNT(DISTINCT h.track_id) as tracks_count,
                    COUNT(DISTINCT t.artist_id) as artists_count
             FROM playback_history h
             JOIN tracks t ON t.id = h.track_id
             WHERE h.user_id = ? AND h.started_at >= ?",
        )
        .bind(user_id)
        .bind(week_cutoff)
        .fetch_optional(&self.pool)
        .await?;

        let your_week = week_summary.map(|(s, t, a)| {
            let total_seconds = s.unwrap_or(0.0);
            let hours = (total_seconds / 3600.0).floor() as u64;
            let mins = ((total_seconds % 3600.0) / 60.0).floor() as u64;
            let formatted_time = if hours > 0 {
                format!("{}h {}m", hours, mins)
            } else {
                format!("{}m", mins)
            };

            YourWeekStats {
                total_seconds,
                track_count: t.unwrap_or(0) as u64,
                artist_count: a.unwrap_or(0) as u64,
                formatted_time,
            }
        });

        Ok(HomeOverview {
            flow_session,
            recent_rotation,
            rediscover,
            for_you,
            library_moment,
            your_week,
        })
    }

    pub async fn invalidate_home_picks(&self, user_id: &str) {
        self.home_picks.lock().await.remove(user_id);
    }

    /// Retrieves persisted Flow session state.
    pub async fn get_flow_session(&self, user_id: &str) -> AppResult<Option<FlowSessionState>> {
        let key = format!("flow_session:{}", user_id);
        let row: Option<(String,)> =
            sqlx::query_as("SELECT value FROM application_settings WHERE key = ?")
                .bind(&key)
                .fetch_optional(&self.pool)
                .await?;

        if let Some((json_str,)) = row {
            if let Ok(state) = serde_json::from_str::<FlowSessionState>(&json_str) {
                return Ok(Some(state));
            }
        }

        Ok(None)
    }

    /// Persists Flow session state.
    pub async fn save_flow_session(
        &self,
        user_id: &str,
        state: &FlowSessionState,
    ) -> AppResult<()> {
        let key = format!("flow_session:{}", user_id);
        let json_str =
            serde_json::to_string(state).map_err(|e| AppError::Database(e.to_string()))?;
        let now = Utc::now().timestamp();

        sqlx::query(
            "INSERT INTO application_settings (key, value, updated_at) VALUES (?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at"
        )
        .bind(&key)
        .bind(&json_str)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(())
    }

    /// Queries the deterministic "Why this track?" explanation for any track.
    pub async fn get_track_explanation(
        &self,
        user_id: &str,
        track_id: &str,
    ) -> AppResult<FlowExplanation> {
        let feedback = super::signals::SongSignals::load(&self.pool, user_id).await?;
        if feedback.blocked_id(track_id) {
            return Ok(FlowExplanation {
                primary: "not_interested".into(),
                ..Default::default()
            });
        }
        let candidate = sqlx::query_as::<_, FlowCandidateRow>(
            "SELECT t.id, t.title, t.artist_id, a.name as artist_name,
                    t.album_id, al.title as album_title, g.name as genre_name, t.year,
                    t.duration_secs, NULL as cover_art_url,
                    ts.play_count, ts.completion_count, ts.skip_count, ts.last_played_at, ts.manual_like,
                    (t.format = 'online' OR t.file_path LIKE 'online://%') as is_online, NULL as preview_url
             FROM tracks t LEFT JOIN artists a ON a.id = t.artist_id
             LEFT JOIN albums al ON al.id = t.album_id LEFT JOIN genres g ON g.id = t.genre_id
             LEFT JOIN track_statistics ts ON ts.track_id = t.id AND ts.user_id = ?
             WHERE t.id = ?")
            .bind(user_id).bind(track_id).fetch_optional(&self.pool).await?
            .ok_or_else(|| AppError::NotFound(format!("Track not found: {track_id}")))?;
        let profile = self.taste_engine.compute_taste_profile(user_id).await?;
        Ok(Self::evidence(&candidate, &profile, Utc::now().timestamp()))
    }

    pub async fn get_why_this_track(&self, user_id: &str, track_id: &str) -> AppResult<String> {
        let e = self.get_track_explanation(user_id, track_id).await?;
        Ok(match e.primary.as_str() {
            "not_interested" => "You marked this song Not interested".into(),
            "liked" => "One of your liked tracks".into(),
            "rediscovery" => format!(
                "You haven't played this in {} days",
                e.days_since_played.unwrap()
            ),
            "library_exploration" => "Unplayed track from your library".into(),
            "artist_affinity" => format!(
                "Matches your affinity for {}",
                e.artist_name.as_deref().unwrap_or("this artist")
            ),
            "online_discovery" => "Online discovery matching your listening taste".into(),
            "online" => "Selected from your online listening".into(),
            _ => "Selected from your library".into(),
        })
    }

    fn song_key(title: &str, artist: &str) -> String {
        let clean = |value: &str| {
            value
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
        };
        format!("{}:{}", clean(artist), clean(title))
    }

    fn evidence(c: &FlowCandidateRow, profile: &TasteProfile, now: i64) -> FlowExplanation {
        FlowExplanation {
            primary: Self::explain(c, profile, now).1,
            is_liked: c.manual_like == Some(1),
            play_count: c.play_count,
            is_unplayed: c.play_count.unwrap_or(0) == 0
                && c.last_played_at.is_none()
                && c.skip_count.unwrap_or(0) == 0,
            days_since_played: c.last_played_at.map(|lp| (now - lp).max(0) / 86400),
            artist_affinity: c
                .artist_id
                .as_ref()
                .and_then(|id| profile.artist_affinities.get(id))
                .copied()
                .filter(|value| *value > 0.3),
            artist_name: c.artist_name.clone(),
            ..Default::default()
        }
    }

    fn explain(c: &FlowCandidateRow, profile: &TasteProfile, now: i64) -> (String, String) {
        let (reason, category) = if c.manual_like == Some(1) {
            ("One of your liked tracks".into(), "liked")
        } else if let Some(days) = c
            .last_played_at
            .map(|lp| (now - lp).max(0) / 86400)
            .filter(|d| *d >= 30)
        {
            (
                format!("You haven't played this in {days} days"),
                "rediscovery",
            )
        } else if c.play_count.unwrap_or(0) == 0
            && c.last_played_at.is_none()
            && c.skip_count.unwrap_or(0) == 0
        {
            if c.is_online {
                (
                    "Online discovery matching your listening taste".into(),
                    "online_discovery",
                )
            } else {
                (
                    "Unplayed track from your library".into(),
                    "library_exploration",
                )
            }
        } else if c
            .artist_id
            .as_ref()
            .and_then(|id| profile.artist_affinities.get(id))
            .copied()
            .unwrap_or(0.0)
            > 0.3
        {
            (
                format!(
                    "Matches your affinity for {}",
                    c.artist_name.as_deref().unwrap_or("this artist")
                ),
                "artist_affinity",
            )
        } else if c.is_online {
            ("Selected from your online listening".into(), "online")
        } else {
            ("Selected from your library".into(), "library")
        };
        (reason, category.into())
    }
}
