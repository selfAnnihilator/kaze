//! The For You discovery feed: gather the user's taste and catalog candidates,
//! score them, rotate the sticky selection, then diversify the shelf.
use super::*;

mod inputs;
mod rotation;
mod scoring;

use crate::recommendations::{setup, signals::SongSignals};
use scoring::ForYouScorer;

impl DiscoveryCoordinator {
    /// Ranks catalog candidates by listening affinity, with library signals for cold start.
    pub async fn get_discovery_recommendations(
        &self,
        user_id: &str,
        limit: usize,
        force_refresh: bool,
    ) -> AppResult<Vec<DiscoveryRecommendation>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        self.ensure_starter_candidates(user_id, force_refresh).await?;
        let wishlist = self.wishlist_index().await?;
        let taste = self.taste_inputs(user_id).await?;
        let catalog = self.gather_catalog(user_id, &taste, limit, force_refresh).await?;
        let pool = self.taste_engine.pool();
        let signals = SongSignals::load(pool, user_id).await?;
        let starters = setup::members(pool, user_id, &taste.profile).await?;
        let catalog = inputs::preferred_releases(catalog, &signals);

        // Listening strength drives ordering; refreshing never adds random score noise.
        let scorer = ForYouScorer { taste: &taste, signals: &signals, starters: &starters };
        let mut candidates: Vec<_> =
            self.score_candidates(catalog, &scorer).await.into_iter().filter(|c| c.score > 0.0).collect();
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.track.id.cmp(&b.track.id))
        });

        let mut selections = self.for_you_selections.lock().await;
        let selection = rotation::selection_for(&mut selections, user_id);
        rotation::rotate(&mut candidates, selection, limit, force_refresh);
        let recs = rotation::diversify_shelf(&candidates, &wishlist, limit);
        rotation::remember_shown(selection, &recs, force_refresh);
        Ok(recs)
    }
}
