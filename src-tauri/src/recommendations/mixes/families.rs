//! Taste-cluster and daily mix families, and starter mixes.
use super::*;

impl SmartMixGenerator {
    pub(super) async fn taste_clusters(&self, user_id: &str) -> AppResult<Vec<super::super::clusters::Cluster>> {
        let profile = self.taste_engine.compute_taste_profile(user_id).await?;
        super::super::clusters::taste_clusters(&self.pool, user_id, &profile).await
    }

    /// ~70% songs by the cluster's own artists, ~30% by artists similar to them
    /// that are not already among the user's top artists. Rest of the pipeline
    /// (blocked filter, dedupe, artist spacing) runs in `generate_mix_with`.
    pub(super) async fn generate_cluster_mix(
        &self,
        user_id: &str,
        cluster: &super::super::clusters::Cluster,
    ) -> AppResult<Vec<String>> {
        use rand::seq::SliceRandom;
        const SIZE: usize = 30;
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT t.id, a.name FROM tracks t
             JOIN artists a ON a.id = t.artist_id
             LEFT JOIN track_statistics ts ON ts.track_id = t.id AND ts.user_id = ?
             WHERE COALESCE(ts.manual_like, 0) != -1",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        let profile = self.taste_engine.compute_taste_profile(user_id).await?;
        let known: HashSet<String> = profile
            .top_artists
            .iter()
            .filter_map(|a| super::super::signals::artist_credits(&a.display_name).into_iter().next())
            .collect();
        let seeds: Vec<(String, f64)> = cluster.artists.iter().map(|a| (a.clone(), 1.0)).collect();
        let index = super::super::similarity::SimilarityIndex::load(&self.pool, user_id, &seeds).await?;
        let similar: HashSet<String> = index
            .top_similar(8, &known)
            .into_iter()
            .filter_map(|hit| super::super::signals::artist_credits(&hit.display).into_iter().next())
            .collect();

        let mut own = Vec::new();
        let mut neighbours = Vec::new();
        for (id, artist) in rows {
            let credits = super::super::signals::artist_credits(&artist);
            if credits.iter().any(|c| cluster.norms.contains(c)) {
                own.push(id);
            } else if credits.iter().any(|c| similar.contains(c)) {
                neighbours.push(id);
            }
        }
        let mut rng = rand::thread_rng();
        own.shuffle(&mut rng);
        neighbours.shuffle(&mut rng);
        let neighbour_take = neighbours.len().min(SIZE * 3 / 10);
        let own_take = own.len().min(SIZE - neighbour_take);
        let mut picked: Vec<String> = own.into_iter().take(own_take).collect();
        // Interleave neighbours evenly so they are not all bunched at the end.
        let step = picked.len().checked_div(neighbour_take).map_or(0, |q| q.max(1) + 1);
        for (i, id) in neighbours.into_iter().take(neighbour_take).enumerate() {
            let at = ((i + 1) * step).min(picked.len());
            picked.insert(at, id);
        }
        Ok(picked)
    }

    /// Keeps the Daily family consistent: one "Daily Mix N" per taste cluster
    /// when there are at least two clusters, otherwise the single Daily Mix.
    /// Regenerates the whole family together once any member is missing or expired.
    pub(super) async fn sync_daily_family(&self, user_id: &str, existing: &[PlaylistRecord]) -> AppResult<()> {
        let now = Utc::now().timestamp();
        let family: Vec<&PlaylistRecord> = existing
            .iter()
            .filter(|m| {
                m.mix_type
                    .as_deref()
                    .is_some_and(|t| t == "daily" || t.starts_with("daily:"))
            })
            .collect();
        let stale = family.is_empty() || family.iter().any(|m| m.expires_at.is_some_and(|e| now >= e));
        if !stale {
            return Ok(());
        }
        let clusters = self.taste_clusters(user_id).await?;
        let keep_cluster_mixes = clusters.len() >= 2;
        if keep_cluster_mixes {
            for index in 1..=clusters.len() {
                self.generate_mix_with(user_id, &SmartMixType::DailyCluster(index as u8), Some(&clusters))
                    .await?;
            }
        } else if family.is_empty() && !self.has_library_or_starters(user_id).await? {
            return Ok(());
        } else {
            self.generate_mix(user_id, &SmartMixType::Daily).await?;
        }
        // Drop family members that no longer match the current clustering.
        let current = self.playlist_repo.get_smart_mixes_for_user(user_id).await?;
        for mix in current {
            let Some(kind) = mix.mix_type.as_deref() else { continue };
            let obsolete = if keep_cluster_mixes {
                kind == "daily"
                    || kind
                        .strip_prefix("daily:")
                        .and_then(|n| n.parse::<usize>().ok())
                        .is_some_and(|n| n == 0 || n > clusters.len())
            } else {
                kind.starts_with("daily:")
            };
            if obsolete {
                self.playlist_repo.delete_playlist(&mix.id).await?;
            }
        }
        Ok(())
    }

    pub(super) async fn has_library_or_starters(&self, user_id: &str) -> AppResult<bool> {
        let total_tracks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tracks")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);
        if total_tracks > 0 {
            return Ok(true);
        }
        let profile = self.taste_engine.compute_taste_profile(user_id).await?;
        Ok(!super::super::setup::members(&self.pool, user_id, &profile).await?.is_empty())
    }

    pub(super) async fn starter_mix(&self, user: &str) -> AppResult<Option<Vec<String>>> {
        let profile = self.taste_engine.compute_taste_profile(user).await?;
        if profile.has_learned_taste
            || super::super::setup::members(&self.pool, user, &profile)
                .await?
                .is_empty()
        {
            return Ok(None);
        }
        let tracks = super::super::FlowEngine::new(self.pool.clone())
            .generate_playback_tracks_after(user, 0.5, 25, &[], &[])
            .await?;
        Ok(Some(tracks.into_iter().map(|t| t.track_id).collect()))
    }
}
