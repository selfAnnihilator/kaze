//! What Flow scoring reads for a user: taste, listening signals, starter songs,
//! artist similarity and the learned explore bias.
use super::*;
use crate::recommendations::signals::SongSignals;
use crate::recommendations::similarity::{self, SimilarityIndex};
use crate::recommendations::{exploration, setup};

pub(super) struct FlowInputs {
    pub(super) profile: TasteProfile,
    pub(super) signals: SongSignals,
    /// Starter songs from the recommendation setup: id → (source, seed value).
    pub(super) starters: HashMap<String, (String, String)>,
    pub(super) similarity: SimilarityIndex,
    pub(super) now: i64,
    /// Learned from how explore vs familiar picks landed; playback Flow only.
    pub(super) explore_bias: f64,
}

impl FlowEngine {
    pub(super) async fn load_flow_inputs(&self, user_id: &str, include_online: bool) -> AppResult<FlowInputs> {
        let profile = self.taste_engine.compute_taste_profile(user_id).await?;
        let signals = SongSignals::load(&self.pool, user_id).await?;
        let starters = setup::members(&self.pool, user_id, &profile).await?;
        let similarity = SimilarityIndex::load(&self.pool, user_id, &similarity::seeds_from_profile(&profile))
            .await
            .unwrap_or_default();
        let now = Utc::now().timestamp();
        let explore_bias = if include_online {
            exploration::explore_bias(&self.pool, user_id, now).await.unwrap_or(0.0)
        } else {
            0.0
        };
        Ok(FlowInputs { profile, signals, starters, similarity, now, explore_bias })
    }
}
