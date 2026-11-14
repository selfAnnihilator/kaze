//! Sticky selection: a manual refresh rotates among similarly relevant songs by
//! the same artist instead of re-scoring; then the shelf is diversified.
use super::inputs::WishlistIndex;
use super::scoring::ScoredCandidate;
use super::*;
use crate::recommendations::diversity::{self, DiversityItem, DiversityRules};
use crate::recommendations::signals;

/// Most users whose selection state is remembered.
const MAX_SELECTIONS: usize = 16;

fn family(candidate: &ScoredCandidate) -> String {
    signals::song_key(&candidate.track.title, &candidate.track.artist).unwrap_or_else(|| candidate.track.id.clone())
}

fn lead_artist(candidate: &ScoredCandidate) -> String {
    signals::artist_credits(&candidate.track.artist).into_iter().next().unwrap_or_default()
}

/// The user's selection state, evicting the least recently used one when full.
pub(super) fn selection_for<'a>(
    selections: &'a mut HashMap<String, ForYouSelection>,
    user_id: &str,
) -> &'a mut ForYouSelection {
    if !selections.contains_key(user_id) && selections.len() >= MAX_SELECTIONS {
        if let Some(oldest) = selections.iter().min_by_key(|(_, state)| state.touched).map(|(user, _)| user.clone()) {
            selections.remove(&oldest);
        }
    }
    selections.entry(user_id.to_string()).or_default()
}

/// Manual refresh is a new selection, not a new taste score. Rotates only among
/// similarly relevant songs by the same artist, keeping the taste-ranked artist
/// slots and the two-song diversity cap.
pub(super) fn rotate(
    candidates: &mut [ScoredCandidate],
    selection: &mut ForYouSelection,
    limit: usize,
    force_refresh: bool,
) {
    let ranked_ids: Vec<_> = candidates.iter().map(|c| c.track.id.clone()).collect();
    // New listening/feedback/catalog ordering can supersede the current batch.
    if selection.ranked_ids != ranked_ids {
        selection.current.clear();
    }
    selection.ranked_ids = ranked_ids;
    let available: HashSet<_> = candidates.iter().map(family).collect();
    selection.last_shown.retain(|key, _| available.contains(key));
    let mut artists: HashMap<String, Vec<usize>> = HashMap::new();
    for (index, candidate) in candidates.iter().enumerate() {
        artists.entry(lead_artist(candidate)).or_default().push(index);
    }
    for indices in artists.values() {
        let best = candidates[indices[0]].score;
        let eligible: Vec<_> = indices.iter().copied().filter(|&i| candidates[i].score >= best * 0.85).collect();
        if eligible.len() <= limit.min(2) {
            continue;
        }
        let mut choices: Vec<_> = eligible.iter().map(|&i| candidates[i].clone()).collect();
        choices.sort_by(|a, b| {
            selection_priority(selection, a, b, force_refresh)
                .then_with(|| b.score.total_cmp(&a.score))
                .then_with(|| a.track.id.cmp(&b.track.id))
        });
        for (index, candidate) in eligible.into_iter().zip(choices) {
            candidates[index] = candidate;
        }
    }
}

/// Refresh: least recently shown first. Otherwise: keep the current order.
fn selection_priority(
    selection: &ForYouSelection,
    a: &ScoredCandidate,
    b: &ScoredCandidate,
    force_refresh: bool,
) -> std::cmp::Ordering {
    let (ka, kb) = (family(a), family(b));
    if force_refresh {
        let shown = |key: &String| selection.last_shown.get(key).copied().unwrap_or(0);
        shown(&ka).cmp(&shown(&kb))
    } else {
        let position = |key: &String| selection.current.get(key).copied().unwrap_or(usize::MAX);
        position(&ka).cmp(&position(&kb))
    }
}

/// One card per song, ordered by rank (and sticky selection) wherever artist
/// spacing allows it (one song per artist in every eight cards).
pub(super) fn diversify_shelf(
    candidates: &[ScoredCandidate],
    wishlist: &WishlistIndex,
    limit: usize,
) -> Vec<DiscoveryRecommendation> {
    let mut seen_songs = HashSet::new();
    let unique: Vec<&ScoredCandidate> = candidates.iter().filter(|scored| seen_songs.insert(family(scored))).collect();
    let items: Vec<DiversityItem> = unique
        .iter()
        .enumerate()
        .map(|(rank, scored)| DiversityItem {
            score: -(rank as f64),
            artist: scored.track.artist.clone(),
            album: None,
            is_online: false,
            max_per_window: None,
            features: None,
        })
        .collect();
    diversity::diversify(&items, limit, &DiversityRules::shelf(), &[])
        .into_iter()
        .map(|index| to_recommendation(unique[index], wishlist))
        .collect()
}

/// Records what was shown, so the next refresh rotates past it.
pub(super) fn remember_shown(selection: &mut ForYouSelection, recs: &[DiscoveryRecommendation], force_refresh: bool) {
    if force_refresh || selection.generation == 0 {
        selection.generation = selection.generation.saturating_add(1);
    }
    selection.current.clear();
    for (index, rec) in recs.iter().enumerate() {
        let key = signals::song_key(&rec.title, &rec.artist).unwrap_or_else(|| rec.external_track_id.clone());
        selection.last_shown.insert(key.clone(), selection.generation);
        selection.current.insert(key, index);
    }
    selection.touched = Some(std::time::Instant::now());
}

fn to_recommendation(scored: &ScoredCandidate, wishlist: &WishlistIndex) -> DiscoveryRecommendation {
    let track = &scored.track;
    let match_status = match track.match_status.as_str() {
        "EXACT_MATCH" => MatchStatus::ExactMatch,
        "LIKELY_MATCH" => MatchStatus::LikelyMatch,
        "POSSIBLE_MATCH" => MatchStatus::PossibleMatch,
        _ => MatchStatus::NotFound,
    };
    DiscoveryRecommendation {
        external_track_id: track.id.clone(),
        provider: track.provider.clone(),
        provider_id: track.provider_id.clone(),
        title: track.title.clone(),
        artist: track.artist.clone(),
        album: track.album.clone(),
        duration_secs: track.duration_secs,
        cover_art_url: track.cover_art_url.clone(),
        preview_url: track.preview_url.clone(),
        genre: track.genre.clone(),
        match_status,
        matched_local_track_id: track.matched_local_track_id.clone(),
        recommendation_reason: scored.reason.clone(),
        in_wishlist: wishlist.contains(track),
    }
}
