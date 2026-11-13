//! "Fans of X also like Y": combines the user's own co-occurrence graph with
//! Last.fm listener similarity, seeded by the artists the user already loves.

use super::signals::artist_credits;
use super::taste::TasteProfile;
use crate::core::AppResult;
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};

pub const COOC_WEIGHT: f64 = 1.0;
pub const LASTFM_WEIGHT: f64 = 0.8;
pub const SEED_COUNT: usize = 15;

#[derive(Debug, Clone, PartialEq)]
pub struct SimilarHit {
    pub display: String,
    pub boost: f64,
    /// Display name of the seed artist that contributes most.
    pub because_of: String,
}

#[derive(Debug, Default, Clone)]
pub struct SimilarityIndex {
    /// target_norm -> best hit
    hits: HashMap<String, SimilarHit>,
    seeds: HashSet<String>,
}

#[derive(sqlx::FromRow)]
struct EdgeRow {
    source_norm: String,
    target_norm: String,
    target_display: String,
    score: f64,
    source: String,
}

/// (display name, affinity 0..1) of the strongest artists in a taste profile.
pub fn seeds_from_profile(profile: &TasteProfile) -> Vec<(String, f64)> {
    profile
        .top_artists
        .iter()
        .take(SEED_COUNT)
        .map(|a| (a.display_name.clone(), a.affinity.clamp(0.0, 1.0)))
        .collect()
}

impl SimilarityIndex {
    pub async fn load(pool: &SqlitePool, user_id: &str, seeds: &[(String, f64)]) -> AppResult<Self> {
        let mut seed_aff: HashMap<String, (f64, String)> = HashMap::new();
        for (display, aff) in seeds {
            for norm in artist_credits(display) {
                let entry = seed_aff.entry(norm).or_insert((0.0, display.clone()));
                if *aff > entry.0 {
                    *entry = (*aff, display.clone());
                }
            }
        }
        if seed_aff.is_empty() {
            return Ok(Self::default());
        }
        let placeholders = vec!["?"; seed_aff.len()].join(",");
        let sql = format!(
            "SELECT source_norm, target_norm, target_display, score, source FROM artist_similarity
             WHERE source_norm IN ({placeholders}) AND (user_id = '' OR user_id = ?)"
        );
        let mut query = sqlx::query_as::<_, EdgeRow>(&sql);
        let mut keys: Vec<&String> = seed_aff.keys().collect();
        keys.sort();
        for key in keys {
            query = query.bind(key);
        }
        let rows = query.bind(user_id).fetch_all(pool).await?;
        let edges = rows
            .into_iter()
            .map(|r| (r.source_norm, r.target_norm, r.target_display, r.score, r.source));
        Ok(Self::from_edges(&seed_aff, edges))
    }

    fn from_edges(
        seed_aff: &HashMap<String, (f64, String)>,
        edges: impl Iterator<Item = (String, String, String, f64, String)>,
    ) -> Self {
        let mut hits: HashMap<String, SimilarHit> = HashMap::new();
        for (source, target, display, score, kind) in edges {
            let Some((aff, seed_display)) = seed_aff.get(&source) else { continue };
            let weight = if kind == "cooc" { COOC_WEIGHT } else { LASTFM_WEIGHT };
            let boost = (score * weight * aff).clamp(0.0, 1.0);
            let better = hits.get(&target).is_none_or(|h| {
                boost > h.boost || (boost == h.boost && seed_display < &h.because_of)
            });
            if better && boost > 0.0 {
                hits.insert(
                    target,
                    SimilarHit { display, boost, because_of: seed_display.clone() },
                );
            }
        }
        Self { hits, seeds: seed_aff.keys().cloned().collect() }
    }

    /// Test/offline constructor: `edges` are (seed display, target display, score, source).
    pub fn from_pairs(seeds: &[(String, f64)], edges: &[(&str, &str, f64, &str)]) -> Self {
        let mut seed_aff: HashMap<String, (f64, String)> = HashMap::new();
        for (display, aff) in seeds {
            for norm in artist_credits(display) {
                seed_aff.insert(norm, (*aff, display.clone()));
            }
        }
        let rows: Vec<_> = edges
            .iter()
            .flat_map(|(s, t, score, kind)| {
                let targets = artist_credits(t);
                artist_credits(s).into_iter().flat_map(move |sn| {
                    targets
                        .clone()
                        .into_iter()
                        .map(move |tn| (sn.clone(), tn, t.to_string(), *score, kind.to_string()))
                })
            })
            .collect();
        Self::from_edges(&seed_aff, rows.into_iter())
    }

    pub fn is_empty(&self) -> bool {
        self.hits.is_empty()
    }

    /// Strongest similarity of any credit on `artist` to the seed set, 0..1.
    pub fn boost(&self, artist: &str) -> f64 {
        self.hit(artist).map_or(0.0, |h| h.boost)
    }

    pub fn hit(&self, artist: &str) -> Option<&SimilarHit> {
        artist_credits(artist)
            .iter()
            .filter(|norm| !self.seeds.contains(*norm))
            .filter_map(|norm| self.hits.get(norm))
            .max_by(|a, b| a.boost.total_cmp(&b.boost).then_with(|| b.display.cmp(&a.display)))
    }

    /// Top similar artists that are not seeds and not in `exclude` (normalized credits).
    pub fn top_similar(&self, n: usize, exclude: &HashSet<String>) -> Vec<SimilarHit> {
        let mut list: Vec<_> = self
            .hits
            .iter()
            .filter(|(norm, _)| !self.seeds.contains(*norm) && !exclude.contains(*norm))
            .map(|(_, h)| h.clone())
            .collect();
        list.sort_by(|a, b| b.boost.total_cmp(&a.boost).then_with(|| a.display.cmp(&b.display)));
        list.truncate(n);
        list
    }
}
