//! Shared artist/album diversification for Flow, local recommendations and For You.
//!
//! Candidates arrive sorted by relevance. The picker walks them greedily, but each
//! step re-scores the remaining items against what was just picked (and against
//! recently played artists carried across refills), so one artist cannot dominate
//! the queue even when all of its songs score well. Rules relax only when no
//! candidate can satisfy them (for example, a library with a single artist).

use std::collections::HashSet;

/// One rankable candidate. `score` must be on the same scale for every item.
#[derive(Debug, Clone)]
pub struct DiversityItem {
    pub score: f64,
    pub artist: String,
    pub album: Option<String>,
    pub is_online: bool,
    /// Per-item override of `DiversityRules::max_per_window` (e.g. a broadly loved
    /// artist in Familiar mode). `None` uses the rule default.
    pub max_per_window: Option<usize>,
    /// Audio descriptors `[tempo or NaN, energy, brightness]` (see
    /// `crate::audio_features`). `None` for online or not-yet-analyzed tracks.
    pub features: Option<[f32; 3]>,
}

#[derive(Debug, Clone, Copy)]
pub struct DiversityRules {
    /// No artist may reappear within this many most recent entries.
    pub min_artist_gap: usize,
    /// Sliding window (in entries) used for the per-artist cap and repeat penalty.
    pub window: usize,
    /// Maximum entries per artist inside `window`.
    pub max_per_window: usize,
    /// Score subtracted per earlier occurrence of the artist inside `window`.
    pub repeat_penalty: f64,
    /// Same album within this many picks receives `album_penalty`.
    pub album_gap: usize,
    pub album_penalty: f64,
    /// Target share of online picks (0 disables the reservation).
    pub online_share: f64,
    /// Hard cap on picks per artist in this whole call (not relaxed). `None` = no cap.
    pub max_total: Option<usize>,
    /// Score cost per unit of tempo/energy/brightness distance from the previous
    /// pick. 0 disables transition smoothing.
    pub transition_weight: f64,
}

impl DiversityRules {
    /// Radio-style queue: artists spaced by four songs, two per twenty.
    pub fn queue(online_share: f64) -> Self {
        Self {
            min_artist_gap: 4,
            window: 20,
            max_per_window: 2,
            repeat_penalty: 0.15,
            album_gap: 6,
            album_penalty: 0.2,
            online_share: online_share.clamp(0.0, 1.0),
            max_total: None,
            transition_weight: 0.0,
        }
    }

    /// Home shelf: one song per artist in every eight cards, two per shelf at most.
    pub fn shelf() -> Self {
        Self {
            min_artist_gap: 7,
            window: 8,
            max_per_window: 1,
            repeat_penalty: 0.0,
            album_gap: 0,
            album_penalty: 0.0,
            online_share: 0.0,
            max_total: Some(2),
            transition_weight: 0.0,
        }
    }
}

/// Normalized credit set for an artist string. "A feat. B" and "A" share key "a".
pub fn artist_keys(artist: &str) -> Vec<String> {
    let credits = super::signals::artist_credits(artist);
    if credits.is_empty() {
        let fallback = artist.trim().to_lowercase();
        vec![if fallback.is_empty() {
            "unknown artist".into()
        } else {
            fallback
        }]
    } else {
        credits
    }
}

fn overlaps(a: &[String], b: &[String]) -> bool {
    a.iter().any(|key| b.contains(key))
}

/// Returns indices into `items` in pick order (at most `limit`).
/// `recent_artists` is ordered oldest → newest (e.g. the last songs played/queued).
pub fn diversify(
    items: &[DiversityItem],
    limit: usize,
    rules: &DiversityRules,
    recent_artists: &[String],
) -> Vec<usize> {
    diversify_from(items, limit, rules, recent_artists, None)
}

/// Cost of placing `next` after `prev`: 0 without a previous sound, a neutral
/// 0.5 for unanalyzed candidates (so they are neither favored nor buried).
fn transition_cost(prev: Option<&[f32; 3]>, next: Option<&[f32; 3]>) -> f64 {
    match (prev, next) {
        (None, _) => 0.0,
        (Some(_), None) => 0.5,
        (Some(a), Some(b)) => crate::audio_features::transition_distance(a, b),
    }
}

/// Like [`diversify`], with `seed` as the sound of the track playing before the
/// first pick (used when `rules.transition_weight > 0`).
pub fn diversify_from(
    items: &[DiversityItem],
    limit: usize,
    rules: &DiversityRules,
    recent_artists: &[String],
    seed: Option<[f32; 3]>,
) -> Vec<usize> {
    let mut prev_sound = seed;
    let keys: Vec<Vec<String>> = items.iter().map(|item| artist_keys(&item.artist)).collect();
    let albums: Vec<Option<String>> = items
        .iter()
        .zip(&keys)
        .map(|(item, artist)| {
            item.album
                .as_deref()
                .map(|album| album.trim().to_lowercase())
                .filter(|album| !album.is_empty())
                .map(|album| format!("{album}|{}", artist.join("|")))
        })
        .collect();

    let mut picked_keys: Vec<Vec<String>> = Vec::new();
    let mut history: Vec<Vec<String>> = recent_artists
        .iter()
        .map(|artist| artist_keys(artist))
        .collect();
    let mut picked_albums: Vec<Option<String>> = Vec::new();
    let mut remaining: Vec<usize> = (0..items.len()).collect();
    let mut order = Vec::with_capacity(limit.min(items.len()));
    let mut online_picked = 0usize;
    let mut seen = HashSet::new();

    while order.len() < limit && !remaining.is_empty() {
        let online_due = rules.online_share > 0.0
            && online_picked < ((order.len() + 1) as f64 * rules.online_share).round() as usize;
        let gap_start = history.len().saturating_sub(rules.min_artist_gap);
        let window_start = history.len().saturating_sub(rules.window);
        let album_start = picked_albums.len().saturating_sub(rules.album_gap);

        // (tier, adjusted score, position in remaining). Lower tier wins.
        let mut best: Option<(u8, f64, usize)> = None;
        for (position, &index) in remaining.iter().enumerate() {
            let item = &items[index];
            let artist = &keys[index];
            if let Some(max_total) = rules.max_total {
                let total = picked_keys
                    .iter()
                    .filter(|entry| overlaps(entry, artist))
                    .count();
                if total >= max_total {
                    continue;
                }
            }
            let gap_ok = !history[gap_start..]
                .iter()
                .any(|entry| overlaps(entry, artist));
            let not_last = history.last().is_none_or(|entry| !overlaps(entry, artist));
            let count = history[window_start..]
                .iter()
                .filter(|entry| overlaps(entry, artist))
                .count();
            let cap = item.max_per_window.unwrap_or(rules.max_per_window).max(1);
            let cap_ok = count < cap;
            let album_recent = rules.album_gap > 0
                && albums[index].as_ref().is_some_and(|album| {
                    picked_albums[album_start..]
                        .iter()
                        .any(|picked| picked.as_ref() == Some(album))
                });
            let tier = match (gap_ok && cap_ok, online_due && item.is_online) {
                (true, true) => 0,
                (true, false) if !online_due => 0,
                (true, false) => 1,
                _ if cap_ok && not_last => 2,
                _ if not_last => 3,
                _ => 4,
            };
            let adjusted = item.score
                - rules.repeat_penalty * count as f64
                - if album_recent { rules.album_penalty } else { 0.0 }
                - if rules.transition_weight > 0.0 {
                    rules.transition_weight
                        * transition_cost(prev_sound.as_ref(), item.features.as_ref())
                } else {
                    0.0
                };
            let better = match best {
                None => true,
                Some((best_tier, best_score, _)) => {
                    tier < best_tier || (tier == best_tier && adjusted > best_score)
                }
            };
            if better {
                best = Some((tier, adjusted, position));
            }
        }
        let Some((_, _, position)) = best else { break };
        let index = remaining.remove(position);
        if !seen.insert(index) {
            continue;
        }
        if items[index].is_online {
            online_picked += 1;
        }
        history.push(keys[index].clone());
        picked_keys.push(keys[index].clone());
        picked_albums.push(albums[index].clone());
        // Unanalyzed picks keep the last known sound as the reference.
        if items[index].features.is_some() {
            prev_sound = items[index].features;
        }
        order.push(index);
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(artist: &str, score: f64) -> DiversityItem {
        DiversityItem {
            score,
            artist: artist.into(),
            album: None,
            is_online: false,
            max_per_window: None,
            features: None,
        }
    }

    fn artists(items: &[DiversityItem], order: &[usize]) -> Vec<String> {
        order.iter().map(|&i| items[i].artist.clone()).collect()
    }

    #[test]
    fn spaces_dominant_artist_and_caps_per_window() {
        let mut items: Vec<_> = (0..10).map(|i| item("Hot", 2.0 - i as f64 * 0.01)).collect();
        for (n, name) in ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"].iter().enumerate() {
            items.push(item(name, 1.0 - n as f64 * 0.01));
        }
        let order = diversify(&items, 12, &DiversityRules::queue(0.0), &[]);
        let picked = artists(&items, &order);
        assert_eq!(picked.iter().filter(|a| *a == "Hot").count(), 2);
        let positions: Vec<_> = picked
            .iter()
            .enumerate()
            .filter(|(_, a)| *a == "Hot")
            .map(|(i, _)| i)
            .collect();
        assert!(positions[1] - positions[0] > 4, "{picked:?}");
    }

    #[test]
    fn featured_credits_share_artist_key() {
        let items = vec![
            item("Hot", 3.0),
            item("Hot feat. Guest", 2.9),
            item("Other", 1.0),
        ];
        let order = diversify(&items, 2, &DiversityRules::queue(0.0), &[]);
        assert_eq!(artists(&items, &order), vec!["Hot", "Other"]);
    }

    #[test]
    fn recent_artists_carry_across_batches() {
        let items = vec![item("Hot", 3.0), item("Other", 1.0), item("Third", 0.5)];
        let order = diversify(&items, 1, &DiversityRules::queue(0.0), &["Hot".into()]);
        assert_eq!(artists(&items, &order), vec!["Other"]);
    }

    #[test]
    fn relaxes_when_only_one_artist_exists() {
        let items: Vec<_> = (0..5).map(|i| item("Solo", 1.0 - i as f64 * 0.1)).collect();
        assert_eq!(diversify(&items, 5, &DiversityRules::queue(0.0), &[]).len(), 5);
    }

    #[test]
    fn reserves_online_share() {
        let mut items: Vec<_> = (0..10)
            .map(|i| item(&format!("Local {i}"), 2.0 - i as f64 * 0.01))
            .collect();
        for i in 0..10 {
            let mut online = item(&format!("Web {i}"), 0.5);
            online.is_online = true;
            items.push(online);
        }
        let order = diversify(&items, 10, &DiversityRules::queue(0.6), &[]);
        let online = order.iter().filter(|&&i| items[i].is_online).count();
        assert_eq!(online, 6);
    }

    #[test]
    fn shelf_allows_one_per_eight() {
        let items = vec![item("A", 3.0), item("A", 2.9), item("B", 1.0), item("C", 0.9)];
        let order = diversify(&items, 3, &DiversityRules::shelf(), &[]);
        assert_eq!(artists(&items, &order), vec!["A", "B", "C"]);
    }

    #[test]
    fn shelf_hard_caps_artist_total() {
        let mut items: Vec<_> = (0..8).map(|i| item("Fav", 3.0 - i as f64 * 0.1)).collect();
        items.push(item("Other", 0.5));
        let order = diversify(&items, 8, &DiversityRules::shelf(), &[]);
        let picked = artists(&items, &order);
        assert_eq!(picked.iter().filter(|a| *a == "Fav").count(), 2, "{picked:?}");
        assert_eq!(picked.len(), 3);
    }
}
