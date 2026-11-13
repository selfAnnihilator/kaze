use crate::core::error::{AppError, AppResult};
use crate::database::repositories::{RecommendationRepository, SqliteRecommendationRepository};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::collections::HashMap;

/// Represents the user's computed musical affinities across artists, genres, and eras.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TasteProfile {
    /// Artist ID -> Blended affinity [0.0, 1.0]
    pub artist_affinities: HashMap<String, f64>,
    /// Genre name -> Blended affinity [0.0, 1.0]
    pub genre_affinities: HashMap<String, f64>,
    /// Era (e.g., "1970s", "1980s", "1990s", "2000s", "2010s", "2020s") -> Affinity [0.0, 1.0]
    pub era_affinities: HashMap<String, f64>,
    #[serde(default)]
    pub language_affinities: HashMap<String, f64>,
    /// Top artists with their names and affinities
    pub top_artists: Vec<EntityAffinity>,
    /// Top genres with their names and affinities
    pub top_genres: Vec<EntityAffinity>,
    /// Total sessions analyzed
    pub total_sessions_analyzed: usize,
    /// Timestamp when profile was computed
    pub computed_at: i64,
    #[serde(default)]
    pub has_learned_taste: bool,
    /// Artist ID -> number of distinct songs with positive engagement.
    #[serde(default)]
    pub artist_breadth: HashMap<String, usize>,
    /// Context bucket (`evening:weekday`, `evening`, …) -> style lift for that time.
    #[serde(default)]
    pub context_affinity: HashMap<String, super::context::ContextAffinity>,
}

impl TasteProfile {
    pub fn artist_affinity(&self, artist: &str) -> f64 {
        let credits = super::signals::artist_credits(artist);
        self.top_artists
            .iter()
            .filter(|known| {
                super::signals::artist_credits(&known.display_name)
                    .iter()
                    .any(|name| credits.contains(name))
            })
            .map(|known| known.affinity)
            .fold(0.0, f64::max)
    }

    /// Number of distinct songs by this artist the user engaged with positively.
    /// Liking one song is not the same as liking the artist; breadth tells them apart.
    pub fn artist_breadth(&self, artist: &str) -> usize {
        let credits = super::signals::artist_credits(artist);
        self.top_artists
            .iter()
            .filter(|known| {
                super::signals::artist_credits(&known.display_name)
                    .iter()
                    .any(|name| credits.contains(name))
            })
            .filter_map(|known| self.artist_breadth.get(&known.id_or_name).copied())
            .max()
            .unwrap_or(0)
    }
}

/// Saturation constants: raw evidence needed to reach ~63% affinity.
const ARTIST_K: f64 = 3.0;
const GENRE_K: f64 = 4.0;
const STYLE_K: f64 = 4.0;
const PLAYLIST_TRACK_BONUS: f64 = 0.5;

/// Absolute, confidence-aware curve: a couple of listens stay modest, broad
/// sustained engagement approaches 1.0. Negative evidence maps to 0.
pub(crate) fn saturate(raw: f64, k: f64) -> f64 {
    if raw <= 0.0 {
        0.0
    } else {
        (1.0 - (-raw / k).exp()).clamp(0.0, 1.0)
    }
}

/// Per-song contribution to the artist: log-damped when positive, so replays
/// matter less than variety; skips stay linear. One song alone can never
/// carry an artist past ~0.5 affinity, however often it is replayed.
const MAX_TRACK_ARTIST_RAW: f64 = 2.0;
fn damp_track(net: f64) -> f64 {
    if net > 0.0 {
        net.ln_1p().min(MAX_TRACK_ARTIST_RAW)
    } else {
        net
    }
}

/// Adds shared-library priors only to keys that already carry positive
/// user-specific evidence (listening, likes, playlists).
fn amplify_known(
    short: &mut HashMap<String, f64>,
    long: &mut HashMap<String, f64>,
    priors: HashMap<String, f64>,
) {
    for (key, prior) in priors {
        let known = short.get(&key).is_some_and(|v| *v > 0.0)
            || long.get(&key).is_some_and(|v| *v > 0.0);
        if known {
            *short.entry(key.clone()).or_default() += prior;
            *long.entry(key).or_default() += prior;
        }
    }
}

/// Small, log-saturating prior for owned / saved songs.
fn library_prior(count: f64) -> f64 {
    (0.6 * count.ln_1p()).min(1.5)
}

fn decade_of(year: Option<i64>) -> Option<String> {
    year.filter(|year| *year > 1900 && *year <= 2030)
        .map(|year| format!("{}0s", year / 10))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityAffinity {
    pub id_or_name: String,
    pub display_name: String,
    pub affinity: f64,
}

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct HistorySessionRow {
    track_id: String,
    artist_id: Option<String>,
    artist_name: Option<String>,
    genre_name: Option<String>,
    year: Option<i64>,
    started_at: i64,
    duration_secs: f64,
    seconds_listened: f64,
    skipped: i64,
}

#[derive(Debug, FromRow)]
#[allow(dead_code)]
struct FeedbackRow {
    track_id: String,
    artist_id: Option<String>,
    artist_name: Option<String>,
    genre_name: Option<String>,
    manual_like: i64,
}

#[derive(Debug, FromRow)]
struct PriorRow {
    track_id: String,
    artist_id: Option<String>,
    artist_name: Option<String>,
    genre_name: Option<String>,
    from_playlist: i64,
}

pub struct TasteProfileEngine {
    pool: SqlitePool,
    rec_repo: SqliteRecommendationRepository,
}

impl TasteProfileEngine {
    pub fn new(pool: SqlitePool) -> Self {
        let rec_repo = SqliteRecommendationRepository::new(pool.clone());
        Self { pool, rec_repo }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Blend recent listening (60%) with lifetime history (40%), scoped to one user.
    pub async fn compute_taste_profile(&self, user_id: &str) -> AppResult<TasteProfile> {
        let now = Utc::now().timestamp();
        let signals = super::signals::SongSignals::load(&self.pool, user_id).await?;
        let short_term_cutoff = now - (14 * 86400); // 14 days

        // Query listening history joined with track, artist, and genre for this specific user
        let rows = sqlx::query_as::<_, HistorySessionRow>(
            "SELECT h.track_id, t.artist_id, a.name as artist_name, COALESCE(g.name, e.genre) as genre_name,
                    t.year, h.started_at, COALESCE(t.duration_secs,0) as duration_secs, h.seconds_listened, h.skipped
             FROM playback_history h
             JOIN tracks t ON t.id = h.track_id
             LEFT JOIN artists a ON a.id = t.artist_id
             LEFT JOIN genres g ON g.id = t.genre_id
             LEFT JOIN external_tracks e ON e.id = t.id
             WHERE h.user_id = ?
             ORDER BY h.started_at ASC"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Query user explicit feedback (likes / dislikes) from track_statistics for this specific user
        let feedback_rows = sqlx::query_as::<_, FeedbackRow>(
            "SELECT ts.track_id, t.artist_id, a.name as artist_name, COALESCE(g.name, e.genre) as genre_name, ts.manual_like
             FROM track_statistics ts
             JOIN tracks t ON t.id = ts.track_id
             LEFT JOIN artists a ON a.id = t.artist_id
             LEFT JOIN genres g ON g.id = t.genre_id
             LEFT JOIN external_tracks e ON e.id = t.id
             WHERE ts.user_id = ?
             UNION
             SELECT t.id,t.artist_id,a.name,COALESCE(g.name,e.genre),1
             FROM playlist_tracks pt JOIN playlists p ON p.id=pt.playlist_id JOIN tracks t ON t.id=pt.track_id
             LEFT JOIN artists a ON a.id=t.artist_id LEFT JOIN genres g ON g.id=t.genre_id LEFT JOIN external_tracks e ON e.id=t.id
             WHERE p.user_id=? AND p.name='Liked Songs' AND p.is_smart_mix=0
               AND NOT EXISTS(SELECT 1 FROM track_statistics ts WHERE ts.track_id=t.id AND ts.user_id=p.user_id AND ts.manual_like!=0)"
        )
        .bind(user_id).bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        // Listening is aggregated per (artist, song) first. An artist's raw score
        // is the sum of log-damped per-song scores, so replaying one song grows
        // the artist slowly while enjoying many distinct songs grows it quickly.
        let mut track_short: HashMap<(String, String), f64> = HashMap::new();
        let mut track_long: HashMap<(String, String), f64> = HashMap::new();
        let mut artist_names: HashMap<String, String> = HashMap::new();

        let mut genre_short_raw: HashMap<String, f64> = HashMap::new();
        let mut genre_long_raw: HashMap<String, f64> = HashMap::new();

        let mut era_raw: HashMap<String, f64> = HashMap::new();
        let mut language_short: HashMap<String, f64> = HashMap::new();
        let mut language_long: HashMap<String, f64> = HashMap::new();

        let track_key = |track_id: &str, artist: Option<&str>| -> String {
            signals
                .identities
                .get(track_id)
                .and_then(|song| super::signals::song_key(&song.title, artist.unwrap_or("")))
                .unwrap_or_else(|| track_id.to_string())
        };

        let mut context = super::context::ContextAccumulator::default();
        for row in &rows {
            let engagement = super::engagement::Engagement::from_listening(
                row.seconds_listened,
                row.duration_secs,
                row.skipped == 1,
            );
            let session_weight = engagement.weight;
            if session_weight == 0.0 {
                continue;
            }
            if engagement.qualified {
                context.add(
                    row.started_at,
                    session_weight,
                    row.genre_name.as_deref(),
                    signals
                        .identities
                        .get(&row.track_id)
                        .and_then(|song| song.language.as_deref()),
                );
            }

            let age_days = ((now - row.started_at).max(0) as f64) / 86400.0;
            let decayed_score = session_weight * (-0.05 * age_days).exp();
            // Keep older listening relevant without letting it dominate recent changes.
            let lifetime_score = session_weight * (-age_days / 180.0).exp().max(0.1);
            let is_short = row.started_at >= short_term_cutoff;

            if let Some(language) = signals
                .identities
                .get(&row.track_id)
                .and_then(|song| song.language.as_ref())
            {
                let language = language.trim().to_lowercase();
                *language_long.entry(language.clone()).or_default() += lifetime_score;
                if is_short {
                    *language_short.entry(language).or_default() += decayed_score;
                }
            }
            if let Some(artist_id) = &row.artist_id {
                if let Some(name) = &row.artist_name {
                    artist_names.insert(artist_id.clone(), name.clone());
                }
                let key = (
                    artist_id.clone(),
                    track_key(&row.track_id, row.artist_name.as_deref()),
                );
                *track_long.entry(key.clone()).or_default() += lifetime_score;
                if is_short {
                    *track_short.entry(key).or_default() += decayed_score;
                }
            }

            if let Some(genre) = &row.genre_name {
                let clean_genre = genre.trim().to_string();
                if !clean_genre.is_empty() {
                    *genre_long_raw.entry(clean_genre.clone()).or_insert(0.0) += lifetime_score;
                    if is_short {
                        *genre_short_raw.entry(clean_genre).or_insert(0.0) += decayed_score;
                    }
                }
            }

            if let Some(decade) = decade_of(row.year.or_else(|| {
                signals
                    .identities
                    .get(&row.track_id)
                    .and_then(|song| song.year)
            })) {
                *era_raw.entry(decade).or_insert(0.0) += lifetime_score;
            }
        }

        // Explicit feedback: a liked song is a strong *style* signal (genre,
        // language, era) but only one song's worth of artist signal.
        let mut feedback_seen = std::collections::HashSet::new();
        let mut feedback_rows = feedback_rows;
        feedback_rows.sort_by_key(|row| row.manual_like);
        for fb in feedback_rows {
            let title = signals
                .identities
                .get(&fb.track_id)
                .map(|song| song.title.as_str())
                .unwrap_or("");
            let key = super::signals::song_key(title, fb.artist_name.as_deref().unwrap_or(""))
                .unwrap_or_else(|| fb.track_id.clone());
            if !feedback_seen.insert(key) {
                continue;
            }
            let feedback = signals
                .signal(&fb.track_id, title, fb.artist_name.as_deref().unwrap_or(""))
                .manual_like;
            if feedback == 0 {
                continue;
            }
            let bonus = if feedback > 0 { 2.0 } else { -2.0 };
            if let Some(language) = signals
                .identities
                .get(&fb.track_id)
                .and_then(|song| song.language.as_ref())
            {
                let language = language.trim().to_lowercase();
                *language_short.entry(language.clone()).or_default() += bonus;
                *language_long.entry(language).or_default() += bonus;
            }
            if let Some(artist_id) = fb.artist_id {
                let key = (artist_id.clone(), track_key(&fb.track_id, fb.artist_name.as_deref()));
                if let Some(name) = fb.artist_name {
                    artist_names.insert(artist_id, name);
                }
                *track_short.entry(key.clone()).or_default() += bonus;
                *track_long.entry(key).or_default() += bonus;
            }
            if let Some(genre) = fb.genre_name {
                let clean_genre = genre.trim().to_string();
                if !clean_genre.is_empty() {
                    *genre_short_raw.entry(clean_genre.clone()).or_insert(0.0) += bonus;
                    *genre_long_raw.entry(clean_genre).or_insert(0.0) += bonus;
                }
            }
            if bonus > 0.0 {
                if let Some(decade) = decade_of(
                    signals
                        .identities
                        .get(&fb.track_id)
                        .and_then(|song| song.year),
                ) {
                    *era_raw.entry(decade).or_insert(0.0) += bonus;
                }
            }
        }

        // Learned-taste gating (first-use setup, starter tracks) only looks at
        // actual listening and feedback, never at the passive library priors.
        let has_learned_taste = track_long.values().chain(track_short.values()).any(|v| *v > 0.0)
            || genre_long_raw.values().chain(genre_short_raw.values()).any(|v| *v > 0.0)
            || language_long.values().chain(language_short.values()).any(|v| *v > 0.0);

        // Passive priors. Playlists are per-user and count directly. The local
        // library is shared by every profile on the machine, so owning files only
        // amplifies artists/genres/languages this user already has positive
        // evidence for; it never creates taste on its own.
        let mut artist_prior: HashMap<String, f64> = HashMap::new();
        let mut genre_prior_count: HashMap<String, f64> = HashMap::new();
        let mut language_prior_count: HashMap<String, f64> = HashMap::new();
        let mut artist_owned_count: HashMap<String, f64> = HashMap::new();
        let mut genre_playlist_count: HashMap<String, f64> = HashMap::new();
        let mut language_playlist_count: HashMap<String, f64> = HashMap::new();
        let library_rows = sqlx::query_as::<_, PriorRow>(
            "SELECT t.id as track_id, t.artist_id, a.name as artist_name, g.name as genre_name, 0 as from_playlist
             FROM tracks t
             LEFT JOIN artists a ON a.id = t.artist_id
             LEFT JOIN genres g ON g.id = t.genre_id
             WHERE t.format != 'online' AND t.file_path NOT LIKE 'online://%'
             UNION ALL
             SELECT DISTINCT t.id, t.artist_id, a.name, COALESCE(g.name, e.genre), 1
             FROM playlist_tracks pt
             JOIN playlists p ON p.id = pt.playlist_id
             JOIN tracks t ON t.id = pt.track_id
             LEFT JOIN artists a ON a.id = t.artist_id
             LEFT JOIN genres g ON g.id = t.genre_id
             LEFT JOIN external_tracks e ON e.id = t.id
             WHERE p.user_id = ? AND p.is_smart_mix = 0 AND p.name != 'Liked Songs'",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;
        let mut playlist_seen = std::collections::HashSet::new();
        for row in &library_rows {
            if row.from_playlist != 0 && !playlist_seen.insert(row.track_id.clone()) {
                continue;
            }
            let (genre_counts, language_counts, weight) = if row.from_playlist != 0 {
                (&mut genre_playlist_count, &mut language_playlist_count, 1.0)
            } else {
                (&mut genre_prior_count, &mut language_prior_count, 0.5)
            };
            if let Some(genre) = row.genre_name.as_deref().map(str::trim).filter(|g| !g.is_empty()) {
                *genre_counts.entry(genre.to_string()).or_default() += weight;
            }
            if let Some(language) = signals
                .identities
                .get(&row.track_id)
                .and_then(|song| song.language.as_ref())
            {
                *language_counts
                    .entry(language.trim().to_lowercase())
                    .or_default() += weight;
            }
            let Some(artist_id) = &row.artist_id else { continue };
            if let Some(name) = &row.artist_name {
                artist_names
                    .entry(artist_id.clone())
                    .or_insert_with(|| name.clone());
            }
            if row.from_playlist != 0 {
                // Playlist membership is a light per-song signal.
                let key = (artist_id.clone(), track_key(&row.track_id, row.artist_name.as_deref()));
                *track_long.entry(key.clone()).or_default() += PLAYLIST_TRACK_BONUS;
                *track_short.entry(key).or_default() += PLAYLIST_TRACK_BONUS;
            } else {
                *artist_owned_count.entry(artist_id.clone()).or_default() += 1.0;
            }
        }
        for (artist_id, count) in artist_owned_count {
            *artist_prior.entry(artist_id).or_default() += library_prior(count);
        }

        // Fold songs into per-artist raw scores and breadth.
        let mut artist_short_raw: HashMap<String, f64> = HashMap::new();
        let mut artist_long_raw: HashMap<String, f64> = HashMap::new();
        let mut artist_breadth: HashMap<String, usize> = HashMap::new();
        for ((artist_id, _), net) in &track_short {
            *artist_short_raw.entry(artist_id.clone()).or_default() += damp_track(*net);
        }
        for ((artist_id, _), net) in &track_long {
            *artist_long_raw.entry(artist_id.clone()).or_default() += damp_track(*net);
            if *net > 0.0 {
                *artist_breadth.entry(artist_id.clone()).or_default() += 1;
            }
        }
        // Playlist style signal (user-scoped): applied first so it can qualify
        // a genre/language for the shared-library amplification below.
        for (genre, count) in genre_playlist_count {
            let prior = library_prior(count);
            *genre_short_raw.entry(genre.clone()).or_default() += prior;
            *genre_long_raw.entry(genre).or_default() += prior;
        }
        for (language, count) in language_playlist_count {
            let prior = library_prior(count);
            *language_short.entry(language.clone()).or_default() += prior;
            *language_long.entry(language).or_default() += prior;
        }
        amplify_known(&mut artist_short_raw, &mut artist_long_raw, artist_prior);
        amplify_known(
            &mut genre_short_raw,
            &mut genre_long_raw,
            genre_prior_count
                .into_iter()
                .map(|(key, count)| (key, library_prior(count)))
                .collect(),
        );
        amplify_known(
            &mut language_short,
            &mut language_long,
            language_prior_count
                .into_iter()
                .map(|(key, count)| (key, library_prior(count)))
                .collect(),
        );

        let mut all_artist_keys = artist_long_raw.keys().cloned().collect::<Vec<_>>();
        for k in artist_short_raw.keys() {
            if !artist_long_raw.contains_key(k) {
                all_artist_keys.push(k.clone());
            }
        }
        all_artist_keys.sort();

        let mut artist_affinities = HashMap::new();
        let mut top_artists = Vec::new();
        for artist_id in all_artist_keys {
            let short_norm = saturate(
                artist_short_raw.get(&artist_id).copied().unwrap_or(0.0),
                ARTIST_K,
            );
            let long_norm = saturate(
                artist_long_raw.get(&artist_id).copied().unwrap_or(0.0),
                ARTIST_K,
            );
            let blended = (short_norm * 0.6 + long_norm * 0.4).clamp(0.0, 1.0);
            // Zero-affinity (skipped) artists stay visible in top_artists but never
            // boost scoring through artist_affinities.
            if blended > 0.0 {
                artist_affinities.insert(artist_id.clone(), blended);
            }
            let display_name = artist_names
                .get(&artist_id)
                .cloned()
                .unwrap_or_else(|| "Unknown Artist".to_string());
            top_artists.push(EntityAffinity {
                id_or_name: artist_id.clone(),
                display_name,
                affinity: blended,
            });
            let _ = self
                .rec_repo
                .upsert_preference(user_id, "artist", &artist_id, short_norm, long_norm)
                .await;
        }
        top_artists.sort_by(|a, b| {
            b.affinity
                .total_cmp(&a.affinity)
                .then_with(|| a.id_or_name.cmp(&b.id_or_name))
        });

        let mut all_genre_keys = genre_long_raw.keys().cloned().collect::<Vec<_>>();
        for k in genre_short_raw.keys() {
            if !genre_long_raw.contains_key(k) {
                all_genre_keys.push(k.clone());
            }
        }
        all_genre_keys.sort();

        let mut genre_affinities = HashMap::new();
        let mut top_genres = Vec::new();
        for genre in all_genre_keys {
            let short_norm = saturate(genre_short_raw.get(&genre).copied().unwrap_or(0.0), GENRE_K);
            let long_norm = saturate(genre_long_raw.get(&genre).copied().unwrap_or(0.0), GENRE_K);
            let blended = (short_norm * 0.6 + long_norm * 0.4).clamp(0.0, 1.0);
            let _ = self
                .rec_repo
                .upsert_preference(user_id, "genre", &genre, short_norm, long_norm)
                .await;
            if blended <= 0.0 {
                continue;
            }
            genre_affinities.insert(genre.clone(), blended);
            top_genres.push(EntityAffinity {
                id_or_name: genre.clone(),
                display_name: genre.clone(),
                affinity: blended,
            });
        }
        top_genres.sort_by(|a, b| {
            b.affinity
                .total_cmp(&a.affinity)
                .then_with(|| a.id_or_name.cmp(&b.id_or_name))
        });

        let mut era_affinities = HashMap::new();
        for (decade, score) in era_raw {
            let norm = saturate(score, STYLE_K);
            let _ = self
                .rec_repo
                .upsert_preference(user_id, "era", &decade, norm, norm)
                .await;
            if norm > 0.0 {
                era_affinities.insert(decade, norm);
            }
        }

        let language_affinities: HashMap<String, f64> = language_long
            .keys()
            .chain(language_short.keys())
            .map(|language| {
                (
                    language.clone(),
                    saturate(language_short.get(language).copied().unwrap_or(0.0), STYLE_K) * 0.6
                        + saturate(language_long.get(language).copied().unwrap_or(0.0), STYLE_K)
                            * 0.4,
                )
            })
            .filter(|(_, affinity)| *affinity > 0.0)
            .collect();
        let mut profile = TasteProfile {
            has_learned_taste,
            language_affinities,
            artist_affinities,
            artist_breadth,
            genre_affinities,
            era_affinities,
            top_artists,
            top_genres,
            total_sessions_analyzed: rows.len(),
            computed_at: now,
            context_affinity: context.finish(),
        };
        super::setup::apply(&self.pool, user_id, &mut profile).await?;
        super::entity_feedback::EntityFeedbackSet::load(&self.pool, user_id)
            .await?
            .apply_to_profile(&mut profile, chrono::Utc::now().timestamp());
        Ok(profile)
    }
}
