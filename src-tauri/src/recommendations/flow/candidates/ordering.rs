//! Ordering scored candidates into a Flow queue: radio-style artist spacing, an
//! online exploration share and gentle tempo/energy transitions.
use super::scoring::ScoredCandidate;
use super::*;
use crate::recommendations::diversity::{self, DiversityItem, DiversityRules};
use crate::recommendations::exploration;

pub(super) struct OrderOptions<'a> {
    pub(super) limit: usize,
    pub(super) ratio: f64,
    pub(super) explore_bias: f64,
    pub(super) include_online: bool,
    pub(super) recent_artists: &'a [String],
    /// `Some(_)` enables transition smoothing; the inner id seeds the first pick.
    pub(super) transitions: Option<Option<&'a str>>,
}

impl FlowEngine {
    /// Sorts by score, diversifies and returns the picked songs in play order.
    /// Online picks are registered as tracks so they can be queued.
    pub(super) async fn order_and_select(
        &self,
        mut scored: Vec<ScoredCandidate>,
        options: &OrderOptions<'_>,
    ) -> AppResult<Vec<FlowTrackItem>> {
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.candidate.id.cmp(&b.candidate.id))
        });
        let order = self.diversified_order(&scored, options).await;
        let mut slots: Vec<Option<ScoredCandidate>> = scored.into_iter().map(Some).collect();
        let mut selected = Vec::with_capacity(order.len());
        for index in order {
            let Some(item) = slots[index].take() else { continue };
            if options.include_online && item.candidate.is_online {
                self.register_online(&item.candidate).await?;
            }
            selected.push(flow_item(item));
        }
        Ok(selected)
    }

    /// Radio-style diversification shared with other surfaces: artists are spaced and
    /// capped across refills (via `recent_artists`), and an online share is reserved
    /// for exploration. No artist gets an exception, not even the most played one:
    /// Familiar means more of your styles, not one artist.
    async fn diversified_order(&self, scored: &[ScoredCandidate], options: &OrderOptions<'_>) -> Vec<usize> {
        // Local audio descriptors drive gentle tempo/energy transitions.
        let sounds = if options.transitions.is_some() {
            crate::audio_features::load_vectors(&self.pool).await.unwrap_or_default()
        } else {
            HashMap::new()
        };
        let items: Vec<DiversityItem> = scored
            .iter()
            .map(|item| DiversityItem {
                score: item.score,
                artist: item.candidate.artist_name.clone().unwrap_or_else(|| "Unknown Artist".into()),
                album: item.candidate.album_title.clone(),
                is_online: item.candidate.is_online,
                max_per_window: None,
                features: sounds.get(&item.candidate.id).copied(),
            })
            .collect();
        let online_share = if options.include_online {
            ((1.0 - options.ratio) * 0.6 + options.explore_bias).clamp(0.0, 0.7)
        } else {
            0.0
        };
        let mut rules = DiversityRules::queue(online_share);
        if options.transitions.is_some() && !sounds.is_empty() {
            rules.transition_weight = TRANSITION_WEIGHT;
        }
        let seed_sound = options.transitions.flatten().and_then(|id| sounds.get(id).copied());
        diversity::diversify_from(&items, options.limit, &rules, options.recent_artists, seed_sound)
    }

    async fn register_online(&self, c: &FlowCandidateRow) -> AppResult<()> {
        crate::recommendations::mixes::ensure_online_track(
            &self.pool,
            &OnlineTrackInput {
                id: &c.id,
                title: &c.title,
                artist: c.artist_name.as_deref(),
                album: c.album_title.as_deref(),
                duration_secs: Some(c.duration_secs),
                cover_art_url: c.cover_art_url.as_deref(),
                preview_url: c.preview_url.as_deref(),
            },
        )
        .await?;
        Ok(())
    }

    /// Remembers which picks were explore vs familiar, so the explore bias can learn.
    pub(super) async fn record_flow_slots(&self, user_id: &str, selected: &[FlowTrackItem], now: i64) {
        let slots: Vec<(String, &'static str)> = selected
            .iter()
            .map(|t| {
                let explore = t.explanation.rec_slot.as_deref() == Some(exploration::EXPLORE);
                (t.track_id.clone(), if explore { exploration::EXPLORE } else { exploration::FAMILIAR })
            })
            .collect();
        if let Err(error) = exploration::record_slots(&self.pool, user_id, &slots, now).await {
            tracing::warn!(%error, "Failed to record Flow exploration slots");
        }
    }
}

fn flow_item(item: ScoredCandidate) -> FlowTrackItem {
    let c = item.candidate;
    FlowTrackItem {
        track_id: c.id,
        title: c.title,
        artist: c.artist_name.unwrap_or_else(|| "Unknown Artist".into()),
        album: c.album_title,
        duration_secs: c.duration_secs,
        cover_art_url: c.cover_art_url,
        score: item.score,
        reason: item.reason,
        reason_category: item.reason_category,
        explanation: item.explanation,
    }
}
