//! Time-of-day / weekday listening context. Each listen falls into a day part
//! (morning 5–12, afternoon 12–17, evening 17–22, night 22–5) crossed with
//! weekday/weekend. A bucket only speaks once it has enough qualified listens,
//! and its affinity is the *lift* of a style over the user's overall share,
//! so styles played equally at all hours get no context boost.
use chrono::{Datelike, Local, TimeZone, Timelike, Weekday};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Minimum qualified listens before a bucket influences anything.
pub const MIN_BUCKET_LISTENS: usize = 20;
/// Flow weight of the context lift.
pub const CONTEXT_WEIGHT: f64 = 0.08;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DayPart {
    Morning,
    Afternoon,
    Evening,
    Night,
}

impl DayPart {
    pub fn from_hour(hour: u32) -> Self {
        match hour {
            5..=11 => Self::Morning,
            12..=16 => Self::Afternoon,
            17..=21 => Self::Evening,
            _ => Self::Night,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Morning => "morning",
            Self::Afternoon => "afternoon",
            Self::Evening => "evening",
            Self::Night => "night",
        }
    }
}

/// Bucket keys for a local timestamp: the specific `part:weekday|weekend` key
/// first, then the day-part-only fallback.
pub fn bucket_keys(ts: i64) -> Option<(String, String)> {
    let local = Local.timestamp_opt(ts, 0).single()?;
    let part = DayPart::from_hour(local.hour());
    // Night after midnight still belongs to the previous evening's day type.
    let day = if part == DayPart::Night && local.hour() < 5 {
        local.weekday().pred()
    } else {
        local.weekday()
    };
    let kind = if matches!(day, Weekday::Sat | Weekday::Sun) {
        "weekend"
    } else {
        "weekday"
    };
    Some((
        format!("{}:{kind}", part.as_str()),
        part.as_str().to_string(),
    ))
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ContextAffinity {
    /// lowercase genre -> lift in [0, 1]
    #[serde(default)]
    pub genres: HashMap<String, f64>,
    /// lowercase language -> lift in [0, 1]
    #[serde(default)]
    pub languages: HashMap<String, f64>,
    #[serde(default)]
    pub listens: usize,
}

#[derive(Default)]
struct BucketAcc {
    genres: HashMap<String, f64>,
    languages: HashMap<String, f64>,
    genre_total: f64,
    language_total: f64,
    listens: usize,
}

/// Accumulates engagement-weighted style counts per bucket during the taste pass.
#[derive(Default)]
pub struct ContextAccumulator {
    overall: BucketAcc,
    buckets: HashMap<String, BucketAcc>,
}

impl ContextAccumulator {
    pub fn add(&mut self, ts: i64, weight: f64, genre: Option<&str>, language: Option<&str>) {
        let Some((specific, part)) = bucket_keys(ts) else {
            return;
        };
        let genre = genre.map(|g| g.trim().to_lowercase()).filter(|g| !g.is_empty());
        let language = language
            .map(|l| l.trim().to_lowercase())
            .filter(|l| !l.is_empty());
        let add_to = |acc: &mut BucketAcc| {
            acc.listens += 1;
            if let Some(g) = &genre {
                *acc.genres.entry(g.clone()).or_default() += weight;
                acc.genre_total += weight;
            }
            if let Some(l) = &language {
                *acc.languages.entry(l.clone()).or_default() += weight;
                acc.language_total += weight;
            }
        };
        add_to(&mut self.overall);
        add_to(self.buckets.entry(specific).or_default());
        add_to(self.buckets.entry(part).or_default());
    }

    pub fn finish(self) -> HashMap<String, ContextAffinity> {
        let lift = |counts: &HashMap<String, f64>,
                    total: f64,
                    overall: &HashMap<String, f64>,
                    overall_total: f64| {
            counts
                .iter()
                .filter_map(|(key, &value)| {
                    if total <= 0.0 || overall_total <= 0.0 {
                        return None;
                    }
                    let share = value / total;
                    let base = overall.get(key).copied().unwrap_or(0.0) / overall_total;
                    if base >= 1.0 {
                        return None;
                    }
                    let lift = ((share - base).max(0.0) / (1.0 - base)).min(1.0);
                    (lift >= 0.05).then(|| (key.clone(), (lift * 1000.0).round() / 1000.0))
                })
                .collect::<HashMap<_, _>>()
        };
        self.buckets
            .into_iter()
            .filter(|(_, acc)| acc.listens >= MIN_BUCKET_LISTENS)
            .map(|(key, acc)| {
                let affinity = ContextAffinity {
                    genres: lift(
                        &acc.genres,
                        acc.genre_total,
                        &self.overall.genres,
                        self.overall.genre_total,
                    ),
                    languages: lift(
                        &acc.languages,
                        acc.language_total,
                        &self.overall.languages,
                        self.overall.language_total,
                    ),
                    listens: acc.listens,
                };
                (key, affinity)
            })
            .collect()
    }
}

/// The context bucket that applies at `ts` and its affinity, preferring the
/// specific weekday/weekend bucket over the day-part fallback.
pub fn current(
    map: &HashMap<String, ContextAffinity>,
    ts: i64,
) -> Option<(DayPart, &ContextAffinity)> {
    let (specific, part) = bucket_keys(ts)?;
    let affinity = map.get(&specific).or_else(|| map.get(&part))?;
    let day_part = match part.as_str() {
        "morning" => DayPart::Morning,
        "afternoon" => DayPart::Afternoon,
        "evening" => DayPart::Evening,
        _ => DayPart::Night,
    };
    Some((day_part, affinity))
}

/// Context lift of a candidate's style for the bucket in effect (0 when none).
pub fn lift(affinity: &ContextAffinity, genre: Option<&str>, language: Option<&str>) -> f64 {
    let genre = genre
        .map(|g| g.trim().to_lowercase())
        .and_then(|g| affinity.genres.get(&g).copied())
        .unwrap_or(0.0);
    let language = language
        .map(|l| l.trim().to_lowercase())
        .and_then(|l| affinity.languages.get(&l).copied())
        .unwrap_or(0.0);
    genre.max(language * 0.6)
}
