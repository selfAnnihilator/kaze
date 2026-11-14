//! Picking Flow and playback-continuation songs: load the user's inputs, gather
//! candidate rows, score them, then order them into a queue.
use super::*;

mod inputs;
mod ordering;
mod rows;
mod scoring;

use crate::recommendations::context;
use ordering::OrderOptions;
use scoring::{Exclusions, ScoreContext};

/// What to pick: for whom, how familiar, how many, and what to avoid.
pub(super) struct CandidateRequest<'a> {
    pub(super) user_id: &'a str,
    pub(super) familiarity_ratio: f64,
    pub(super) limit: usize,
    pub(super) exclude_track_ids: &'a [String],
    pub(super) recent_artists: &'a [String],
    /// Playback Flow: also online songs, exploration slots and the explore bias.
    pub(super) include_online: bool,
    pub(super) session: Option<&'a SessionSteer>,
    /// `Some(_)` enables transition smoothing; the inner id seeds the first pick.
    pub(super) transitions: Option<Option<&'a str>>,
}

impl FlowEngine {
    pub(super) async fn generate_candidates(&self, request: CandidateRequest<'_>) -> AppResult<Vec<FlowTrackItem>> {
        let CandidateRequest {
            user_id,
            familiarity_ratio,
            limit,
            exclude_track_ids,
            recent_artists,
            include_online,
            session,
            transitions,
        } = request;
        let session = session.filter(|steer| !steer.is_empty());
        if !familiarity_ratio.is_finite() {
            return Err(AppError::Validation("Flow steering must be finite".into()));
        }
        let limit = limit.min(100);
        if limit == 0 {
            return Ok(Vec::new());
        }
        let ratio = familiarity_ratio.clamp(0.0, 1.0);
        let inputs = self.load_flow_inputs(user_id, include_online).await?;
        let mut candidates = self.candidate_rows(user_id, &inputs, include_online, session).await?;
        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        let exclude_ids: HashSet<&str> = exclude_track_ids.iter().map(|s| s.as_str()).collect();
        let excluded_songs = rows::excluded_song_keys(&candidates, &exclude_ids);
        rows::prefer_best_release(&mut candidates, &inputs.signals);
        let scorer = ScoreContext {
            inputs: &inputs,
            ratio,
            novelty_weight: 0.55 * (1.0 + inputs.explore_bias),
            session,
            context_now: context::current(&inputs.profile.context_affinity, inputs.now),
        };
        let scored = scorer.score_all(candidates, &Exclusions { ids: &exclude_ids, songs: &excluded_songs });

        let options = OrderOptions { limit, ratio, explore_bias: inputs.explore_bias, include_online, recent_artists, transitions };
        let selected = self.order_and_select(scored, &options).await?;
        if include_online {
            self.record_flow_slots(user_id, &selected, inputs.now).await;
        }
        Ok(selected)
    }
}
