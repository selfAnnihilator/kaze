//! In-session Flow steering.
//!
//! Long-term taste learns slowly from history. This module reacts to what is
//! happening *right now* inside one Flow session: a couple of quick skips of the
//! same style steer Flow away from it, several full listens steer toward it.
//! State is in-memory only and decays with every outcome, so it fades on its own
//! and is discarded when Flow stops or restarts.

use std::collections::HashMap;

/// Kinds of style a session outcome is keyed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StyleKind {
    Artist,
    Genre,
    Language,
}

/// Normalized style keys of one track.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleKeys {
    /// Canonical artist credits (see `signals::artist_credits`).
    pub artists: Vec<String>,
    pub genre: Option<String>,
    pub language: Option<String>,
}

fn norm(value: Option<&str>) -> Option<String> {
    value
        .map(|v| v.trim().to_lowercase())
        .filter(|v| !v.is_empty() && v != "unknown")
}

impl StyleKeys {
    pub fn new(artist: Option<&str>, genre: Option<&str>, language: Option<&str>) -> Self {
        Self {
            artists: artist
                .map(super::signals::artist_credits)
                .unwrap_or_default()
                .into_iter()
                .filter(|a| a != "unknown artist")
                .collect(),
            genre: norm(genre),
            language: norm(language),
        }
    }

    fn keys(&self) -> Vec<(StyleKind, String)> {
        let mut keys: Vec<_> = self
            .artists
            .iter()
            .map(|a| (StyleKind::Artist, a.clone()))
            .collect();
        if let Some(g) = &self.genre {
            keys.push((StyleKind::Genre, g.clone()));
        }
        if let Some(l) = &self.language {
            keys.push((StyleKind::Language, l.clone()));
        }
        keys
    }

    /// Shared key with priority genre > language > artist.
    fn shared(group: &[StyleKeys], kinds: &[StyleKind]) -> Option<(StyleKind, String)> {
        let first = group.first()?;
        for kind in kinds {
            let candidates: Vec<String> = match kind {
                StyleKind::Genre => first.genre.iter().cloned().collect(),
                StyleKind::Language => first.language.iter().cloned().collect(),
                StyleKind::Artist => first.artists.clone(),
            };
            for key in candidates {
                let all = group.iter().all(|keys| match kind {
                    StyleKind::Genre => keys.genre.as_deref() == Some(key.as_str()),
                    StyleKind::Language => keys.language.as_deref() == Some(key.as_str()),
                    StyleKind::Artist => keys.artists.contains(&key),
                });
                if all {
                    return Some((*kind, key));
                }
            }
        }
        None
    }
}

/// Listening outcome of one finished play.
#[derive(Clone, Copy, Debug, Default)]
pub struct SessionOutcome {
    pub early_skip: bool,
    pub qualified: bool,
    pub near_complete: bool,
}

/// What a recorded outcome asks Flow to do with its upcoming queue.
#[derive(Clone, Debug, PartialEq)]
pub enum SteerTrigger {
    None,
    /// Steer away. `None` key = several unrelated skips in a row.
    Away(Option<(StyleKind, String)>),
    Toward((StyleKind, String)),
}

impl SteerTrigger {
    pub fn fired(&self) -> bool {
        !matches!(self, SteerTrigger::None)
    }
}

pub const DECAY: f64 = 0.85;
const MAX_KEYS: usize = 64;
const COOLDOWN: u32 = 2;
const STREAK: usize = 3;
/// Genre session score at or below which online exploration skips that genre.
pub const BLOCK_GENRE_SCORE: f64 = -1.5;

#[derive(Clone, Debug, Default)]
pub struct SessionSteer {
    scores: HashMap<(StyleKind, String), f64>,
    skip_streak: Vec<StyleKeys>,
    full_streak: Vec<StyleKeys>,
    /// Outcomes since the last trigger; `None` until a trigger fired.
    since_trigger: Option<u32>,
}

impl SessionSteer {
    pub fn is_empty(&self) -> bool {
        self.scores.is_empty()
    }

    pub fn score(&self, kind: StyleKind, key: &str) -> f64 {
        self.scores
            .get(&(kind, key.to_string()))
            .copied()
            .unwrap_or(0.0)
    }

    pub fn genre_score(&self, genre: Option<&str>) -> f64 {
        norm(genre).map_or(0.0, |g| self.score(StyleKind::Genre, &g))
    }

    pub fn record(&mut self, keys: &StyleKeys, outcome: SessionOutcome) -> SteerTrigger {
        let delta = if outcome.early_skip {
            -1.0
        } else if outcome.near_complete {
            1.0
        } else if outcome.qualified {
            0.4
        } else {
            // Short, non-skip stops (e.g. stopped playback) are not evidence.
            return SteerTrigger::None;
        };

        for value in self.scores.values_mut() {
            *value *= DECAY;
        }
        self.scores.retain(|_, value| value.abs() >= 0.01);
        for key in keys.keys() {
            *self.scores.entry(key).or_insert(0.0) += delta;
        }
        self.cap();

        if outcome.early_skip {
            self.full_streak.clear();
            self.skip_streak.push(keys.clone());
        } else if outcome.near_complete {
            self.skip_streak.clear();
            self.full_streak.push(keys.clone());
        }
        for streak in [&mut self.skip_streak, &mut self.full_streak] {
            if streak.len() > STREAK {
                streak.drain(..streak.len() - STREAK);
            }
        }

        if let Some(count) = self.since_trigger.as_mut() {
            *count += 1;
        }
        if self.since_trigger.is_some_and(|count| count <= COOLDOWN) {
            return SteerTrigger::None;
        }

        let trigger = self.detect();
        if trigger.fired() {
            self.since_trigger = Some(0);
            self.skip_streak.clear();
            self.full_streak.clear();
        }
        trigger
    }

    fn detect(&self) -> SteerTrigger {
        let all = [StyleKind::Genre, StyleKind::Language, StyleKind::Artist];
        if self.skip_streak.len() >= 2 {
            let last_two = &self.skip_streak[self.skip_streak.len() - 2..];
            if let Some(key) = StyleKeys::shared(last_two, &all) {
                return SteerTrigger::Away(Some(key));
            }
            if self.skip_streak.len() >= STREAK {
                return SteerTrigger::Away(None);
            }
        }
        if self.full_streak.len() >= STREAK {
            if let Some(key) =
                StyleKeys::shared(&self.full_streak, &[StyleKind::Genre, StyleKind::Language])
            {
                return SteerTrigger::Toward(key);
            }
        }
        SteerTrigger::None
    }

    fn cap(&mut self) {
        if self.scores.len() <= MAX_KEYS {
            return;
        }
        let mut entries: Vec<_> = self
            .scores
            .iter()
            .map(|(k, v)| (k.clone(), v.abs()))
            .collect();
        entries.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        for (key, _) in entries.into_iter().take(self.scores.len() - MAX_KEYS) {
            self.scores.remove(&key);
        }
    }

    /// Additive Flow score adjustment for a candidate. Bounded and asymmetric:
    /// a session can push a style down hard, but only nudge one up, so a run of
    /// full listens never re-floods a single artist.
    pub fn adjustment(&self, keys: &StyleKeys) -> f64 {
        if self.scores.is_empty() {
            return 0.0;
        }
        let t = |x: f64| (x / 2.0).tanh();
        let genre = keys
            .genre
            .as_deref()
            .map_or(0.0, |g| self.score(StyleKind::Genre, g));
        let language = keys
            .language
            .as_deref()
            .map_or(0.0, |l| self.score(StyleKind::Language, l));
        let artist = keys
            .artists
            .iter()
            .map(|a| self.score(StyleKind::Artist, a))
            .fold(None, |acc: Option<f64>, v| {
                Some(acc.map_or(v, |m| m.min(v)))
            })
            .unwrap_or(0.0);
        let raw = 0.18 * t(genre) + 0.08 * t(language) + 0.12 * t(artist);
        // A skip is a direct "not now"; it counts double so two quick skips
        // outweigh a moderate long-term style lead.
        let raw = if raw < 0.0 { raw * 2.0 } else { raw };
        raw.clamp(-0.30, 0.15)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SKIP: SessionOutcome = SessionOutcome {
        early_skip: true,
        qualified: false,
        near_complete: false,
    };
    const FULL: SessionOutcome = SessionOutcome {
        early_skip: false,
        qualified: true,
        near_complete: true,
    };
    const NEUTRAL: SessionOutcome = SessionOutcome {
        early_skip: false,
        qualified: true,
        near_complete: false,
    };

    fn keys(artist: &str, genre: &str, lang: &str) -> StyleKeys {
        StyleKeys::new(Some(artist), Some(genre), Some(lang))
    }

    #[test]
    fn two_skips_same_genre_steer_away() {
        let mut s = SessionSteer::default();
        assert_eq!(s.record(&keys("A", "Rock", "en"), SKIP), SteerTrigger::None);
        assert_eq!(
            s.record(&keys("B", " rock ", "hi"), SKIP),
            SteerTrigger::Away(Some((StyleKind::Genre, "rock".into())))
        );
        assert!(s.adjustment(&keys("C", "Rock", "fr")) < -0.1);
        assert!(s.genre_score(Some("Rock")) <= BLOCK_GENRE_SCORE);
    }

    #[test]
    fn unrelated_skips_need_three() {
        let mut s = SessionSteer::default();
        assert!(!s.record(&keys("A", "Rock", "en"), SKIP).fired());
        assert!(!s.record(&keys("B", "Jazz", "hi"), SKIP).fired());
        assert_eq!(
            s.record(&keys("C", "Pop", "fr"), SKIP),
            SteerTrigger::Away(None)
        );
    }

    #[test]
    fn three_full_listens_steer_toward() {
        let mut s = SessionSteer::default();
        assert!(!s.record(&keys("A", "Jazz", "en"), FULL).fired());
        assert!(!s.record(&keys("B", "Jazz", "hi"), FULL).fired());
        assert_eq!(
            s.record(&keys("C", "Jazz", "fr"), FULL),
            SteerTrigger::Toward((StyleKind::Genre, "jazz".into()))
        );
        assert!(s.adjustment(&keys("D", "Jazz", "de")) > 0.1);
    }

    #[test]
    fn skip_breaks_full_streak() {
        let mut s = SessionSteer::default();
        s.record(&keys("A", "Jazz", "en"), FULL);
        s.record(&keys("B", "Jazz", "en"), FULL);
        s.record(&keys("C", "Pop", "hi"), SKIP);
        assert!(!s.record(&keys("D", "Jazz", "en"), FULL).fired());
    }

    #[test]
    fn cooldown_holds_for_two_outcomes() {
        let mut s = SessionSteer::default();
        s.record(&keys("A", "Rock", "en"), SKIP);
        assert!(s.record(&keys("B", "Rock", "en"), SKIP).fired());
        assert!(!s.record(&keys("C", "Metal", "de"), SKIP).fired());
        assert!(!s.record(&keys("D", "Metal", "de"), SKIP).fired());
        assert!(s.record(&keys("E", "Metal", "de"), SKIP).fired());
    }

    #[test]
    fn evidence_decays_back_to_neutral() {
        let mut s = SessionSteer::default();
        s.record(&keys("A", "Rock", "en"), SKIP);
        s.record(&keys("B", "Rock", "en"), SKIP);
        for i in 0..16 {
            s.record(&keys(&format!("x{i}"), &format!("g{i}"), "zz"), NEUTRAL);
        }
        assert!(s.adjustment(&keys("C", "Rock", "en")).abs() < 0.05);
    }

    #[test]
    fn adjustment_is_bounded() {
        let mut s = SessionSteer::default();
        for _ in 0..30 {
            s.record(&keys("A", "Rock", "en"), SKIP);
        }
        assert!(s.adjustment(&keys("A", "Rock", "en")) >= -0.30);
        let mut s = SessionSteer::default();
        for _ in 0..30 {
            s.record(&keys("A", "Jazz", "en"), FULL);
        }
        assert!(s.adjustment(&keys("A", "Jazz", "en")) <= 0.15);
    }

    #[test]
    fn map_is_capped() {
        let mut s = SessionSteer::default();
        for i in 0..100 {
            s.record(
                &keys(&format!("a{i}"), &format!("g{i}"), &format!("l{i}")),
                NEUTRAL,
            );
        }
        assert!(s.scores.len() <= MAX_KEYS);
    }
}
