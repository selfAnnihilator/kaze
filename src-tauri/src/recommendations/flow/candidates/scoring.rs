//! Scoring one Flow candidate. The score is built in a fixed order of steps
//! (base, similar artists, time of day, "less like this", forgotten favourites,
//! skips, session steering); keep that order, floating-point sums depend on it.
use super::inputs::FlowInputs;
use super::rows::{artist_of, with_signal};
use super::*;
use crate::recommendations::context::{self, ContextAffinity, DayPart};
use crate::recommendations::signals::SongIdentity;
use crate::recommendations::{exploration, setup};

pub(super) struct ScoredCandidate {
    pub(super) candidate: FlowCandidateRow,
    pub(super) score: f64,
    pub(super) reason: String,
    pub(super) reason_category: String,
    pub(super) explanation: FlowExplanation,
}

/// Everything constant across one scoring run.
pub(super) struct ScoreContext<'a> {
    pub(super) inputs: &'a FlowInputs,
    /// Familiarity dial, 0 (unfamiliar) to 1 (familiar).
    pub(super) ratio: f64,
    pub(super) novelty_weight: f64,
    pub(super) session: Option<&'a SessionSteer>,
    pub(super) context_now: Option<(DayPart, &'a ContextAffinity)>,
}

/// Song ids and songs (title + artist) that must not be picked.
pub(super) struct Exclusions<'a> {
    pub(super) ids: &'a HashSet<&'a str>,
    pub(super) songs: &'a HashSet<String>,
}

struct Counts {
    plays: i64,
    completions: i64,
    skips: i64,
    liked: bool,
}

struct Affinities {
    artist: f64,
    genre: f64,
    language: f64,
    era: f64,
}

struct BaseScore {
    score: f64,
    is_explore: bool,
}

/// Explanation details found while adjusting the score.
#[derive(Default)]
struct Notes {
    similar_seed: Option<String>,
    context_bucket: Option<String>,
    session: Option<(Option<String>, f64)>,
}

impl ScoreContext<'_> {
    /// Scores every admissible candidate, once per song.
    pub(super) fn score_all(&self, candidates: Vec<FlowCandidateRow>, exclusions: &Exclusions) -> Vec<ScoredCandidate> {
        let mut seen_songs = HashSet::new();
        let mut scored = Vec::new();
        for c in candidates {
            if self.inputs.signals.blocked(&c.id, &c.title, artist_of(&c)) {
                continue;
            }
            let c = with_signal(c, &self.inputs.signals);
            // Disliked tracks are strictly excluded.
            if c.manual_like == Some(-1) {
                continue;
            }
            // Already queued or played in this session, or a duplicate of a song seen above.
            let identity = FlowEngine::song_key(&c.title, artist_of(&c));
            if exclusions.ids.contains(c.id.as_str()) || exclusions.songs.contains(&identity) || !seen_songs.insert(identity) {
                continue;
            }
            if let Some(item) = self.score(c) {
                scored.push(item);
            }
        }
        scored
    }

    fn score(&self, c: FlowCandidateRow) -> Option<ScoredCandidate> {
        let counts = Counts {
            plays: c.play_count.unwrap_or(0),
            completions: c.completion_count.unwrap_or(0),
            skips: c.skip_count.unwrap_or(0),
            liked: c.manual_like == Some(1),
        };
        let affinities = self.affinities(&c);
        let base = self.base_score(&c, &counts, &affinities);
        let mut score = base.score;
        let notes = self.adjust(&c, &counts, affinities.artist, &mut score);
        let starter = self.inputs.starters.get(&c.id);
        score = score.max(if starter.is_some() { 0.1 } else { 0.0 });
        if c.is_online && !counts.liked && (score <= 0.0 || (counts.plays == 0 && counts.skips > 0)) {
            // An online exploration slot must not force repeatedly skipped songs.
            return None;
        }
        let (reason, reason_category) = match starter {
            Some((source, value)) => (setup::explanation(source, value), format!("starter_{source}")),
            None => FlowEngine::explain(&c, &self.inputs.profile, self.inputs.now),
        };
        let explanation = self.explanation(&c, &reason_category, base.is_explore, notes);
        Some(ScoredCandidate { candidate: c, score, reason, reason_category, explanation })
    }

    fn identity(&self, c: &FlowCandidateRow) -> Option<&SongIdentity> {
        self.inputs.signals.identities.get(&c.id)
    }

    fn affinities(&self, c: &FlowCandidateRow) -> Affinities {
        let profile = &self.inputs.profile;
        let identity = self.identity(c);
        Affinities {
            artist: c
                .artist_id
                .as_ref()
                .and_then(|aid| profile.artist_affinities.get(aid))
                .copied()
                .unwrap_or(0.0)
                .max(profile.artist_affinity(artist_of(c))),
            genre: c.genre_name.as_ref().and_then(|g| profile.genre_affinities.get(g.trim())).copied().unwrap_or(0.0),
            language: identity
                .and_then(|song| song.language.as_ref())
                .and_then(|language| profile.language_affinities.get(&language.to_lowercase()))
                .copied()
                .unwrap_or(0.0),
            era: c
                .year
                .or_else(|| identity.and_then(|song| song.year))
                .and_then(|year| profile.era_affinities.get(&format!("{}0s", year / 10)))
                .copied()
                .unwrap_or(0.0),
        }
    }

    /// The shared recommendation scorer: taste, era, completion, feedback and recency.
    fn taste_score(&self, c: &FlowCandidateRow, counts: &Counts) -> f64 {
        let signals = &self.inputs.signals;
        let input = CandidateTrack {
            id: c.id.clone(),
            title: c.title.clone(),
            artist_id: c.artist_id.clone(),
            artist_name: c.artist_name.clone(),
            genre_name: c.genre_name.clone(),
            year: c.year.or_else(|| self.identity(c).and_then(|song| song.year)),
            language: self.identity(c).and_then(|song| song.language.clone()),
            play_count: counts.plays,
            completion_count: counts.completions,
            skip_count: counts.skips,
            last_played_at: c.last_played_at,
            listening_engagement: signals.signal(&c.id, &c.title, artist_of(c)).engagement(),
            user_feedback: if counts.liked { Some("like".into()) } else { None },
        };
        ScoringEngine::score_track(&input, &self.inputs.profile, self.inputs.now).final_score
    }

    /// Style match plus the familiar or explore share set by the dial.
    fn base_score(&self, c: &FlowCandidateRow, counts: &Counts, aff: &Affinities) -> BaseScore {
        let taste_score = self.taste_score(c, counts);
        // Familiarity means known songs, rather than every unheard song by a favorite artist.
        let known_strength = if counts.liked {
            1.0
        } else {
            ((counts.plays.max(0) as f64).ln_1p() / 6.0_f64.ln_1p()).min(1.0)
        };
        // Style match: fit to the genres/languages/eras the user enjoys, whoever performs it.
        let style_match =
            ((aff.genre * 0.5 + aff.language * 0.3 + aff.era * 0.2) * 0.6 + taste_score.min(1.0) * 0.4).clamp(0.0, 1.0);
        // Novelty: never-heard songs, preferably by artists the user barely knows.
        let novelty = (1.0 - known_strength) * 0.6 + (1.0 - aff.artist) * 0.4;
        let recency = recency_factor(c.last_played_at, self.inputs.now);
        // Unfamiliar (0) → new songs/artists in the user's styles.
        // Familiar (1) → known favorites and artists with broad enjoyment.
        let familiar_part = self.ratio * (known_strength * recency * 0.35 + aff.artist * 0.20);
        let explore_part = (1.0 - self.ratio) * novelty * self.novelty_weight;
        BaseScore {
            score: style_match * 0.45 + familiar_part + explore_part,
            is_explore: c.is_online || explore_part > familiar_part,
        }
    }

    /// Applies the adjustments in their fixed order and returns what explains them.
    fn adjust(&self, c: &FlowCandidateRow, counts: &Counts, artist_aff: f64, score: &mut f64) -> Notes {
        let inputs = self.inputs;
        let language = self.identity(c).and_then(|song| song.language.as_deref());
        // "Fans of your artists also like": favours unknown artists near your taste.
        let similar_hit = inputs.similarity.hit(artist_of(c));
        let similar_boost = similar_hit.map_or(0.0, |h| h.boost) * (1.0 - artist_aff);
        *score += SIMILAR_WEIGHT * similar_boost;
        // Time-of-day context: styles this user favours at this hour/day type.
        let context_hit = self
            .context_now
            .map(|(part, affinity)| (part, context::lift(affinity, c.genre_name.as_deref(), language)));
        *score += context::CONTEXT_WEIGHT * context_hit.map_or(0.0, |(_, lift)| lift);
        // "Less like this": soft, fading artist/genre penalty.
        *score -= inputs.signals.less_penalty(artist_of(c), c.genre_name.as_deref(), inputs.now);
        *score += self.forgotten_favourite(c, counts);
        if counts.skips > 2 && counts.skips > counts.completions {
            *score -= ((counts.skips - counts.completions) as f64 * 0.05).min(0.35);
        }
        let session = self.session_adjustment(c, language, score);
        Notes {
            similar_seed: similar_hit.filter(|_| similar_boost >= 0.1).map(|h| h.because_of.clone()),
            context_bucket: context_hit.filter(|(_, lift)| *lift >= 0.25).map(|(part, _)| part.as_str().to_string()),
            session,
        }
    }

    /// Forgotten favorites (the old "rediscover" end) peak in the middle of the dial.
    fn forgotten_favourite(&self, c: &FlowCandidateRow, counts: &Counts) -> f64 {
        let Some(last_played) = c.last_played_at else { return 0.0 };
        let days = (self.inputs.now - last_played).max(0) / 86400;
        if days >= 30 && (counts.liked || counts.plays >= 2) {
            0.25 * (1.0 - (self.ratio - 0.5).abs() * 2.0)
        } else {
            0.0
        }
    }

    /// Live session steering from recent skips/full listens of this style.
    fn session_adjustment(&self, c: &FlowCandidateRow, language: Option<&str>, score: &mut f64) -> Option<(Option<String>, f64)> {
        let steer = self.session?;
        let keys = StyleKeys::new(c.artist_name.as_deref(), c.genre_name.as_deref(), language);
        let adjustment = steer.adjustment(&keys);
        *score += adjustment;
        if adjustment.abs() < 0.1 {
            return None;
        }
        let genre_led = keys.genre.as_deref().is_some_and(|g| steer.score(StyleKind::Genre, g).abs() >= 0.5);
        let style = if genre_led { c.genre_name.clone() } else { self.identity(c).and_then(|song| song.language.clone()) };
        Some((style, adjustment))
    }

    fn explanation(&self, c: &FlowCandidateRow, reason_category: &str, is_explore: bool, notes: Notes) -> FlowExplanation {
        let inputs = self.inputs;
        let mut explanation = FlowEngine::evidence(c, &inputs.profile, inputs.now);
        if reason_category.starts_with("starter_") {
            explanation.primary = reason_category.to_string();
        }
        explanation.familiarity_ratio = Some(self.ratio);
        explanation.selected_at = Some(inputs.now);
        explanation.similar_artist = notes.similar_seed;
        explanation.context_bucket = notes.context_bucket;
        explanation.rec_slot = Some(if is_explore { exploration::EXPLORE } else { exploration::FAMILIAR }.to_string());
        if is_explore && inputs.explore_bias.abs() >= exploration::NOTICE_BIAS {
            explanation.explore_bias = Some((inputs.explore_bias * 100.0).round() / 100.0);
        }
        if let Some((style, adjustment)) = notes.session {
            explanation.session_style = style.map(|s| s.trim().to_string());
            explanation.session_adjustment = Some((adjustment * 100.0).round() / 100.0);
        }
        explanation
    }
}

/// Songs heard very recently should not dominate Familiar either.
fn recency_factor(last_played: Option<i64>, now: i64) -> f64 {
    last_played.map_or(1.0, |last_played| match (now - last_played).max(0) {
        age if age < 86400 => 0.20,
        age if age < 3 * 86400 => 0.50,
        age if age < 7 * 86400 => 0.80,
        _ => 1.0,
    })
}
