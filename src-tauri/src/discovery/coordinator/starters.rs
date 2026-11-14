//! Online candidates: catalog search, artist editions and starter songs.
use super::*;

impl DiscoveryCoordinator {
    pub(super) async fn fetch_catalog_candidates(&self, queries: &[String]) -> AppResult<usize> {
        let Some(providers) = &self.provider_coordinator else {
            return Ok(0);
        };
        if !providers.enabled().await {
            return Ok(0);
        }
        let mut count = 0;
        for query in queries.iter().take(3) {
            for track in providers
                .jiosaavn
                .search(query, 25)
                .await
                .unwrap_or_default()
            {
                providers.persist(track).await?;
                count += 1;
            }
        }
        Ok(count)
    }

    pub(super) async fn artist_candidates(&self, artist: &str, refresh: bool) -> Vec<ExternalTrackRecord> {
        let key = artist.trim().to_lowercase();
        let Some(providers) = &self.provider_coordinator else {
            return vec![];
        };
        if !providers.enabled().await {
            return vec![];
        }
        if !refresh {
            if let Some((at, tracks)) = self.artist_candidate_cache.lock().await.get(&key) {
                if at.elapsed().as_secs() < 900 {
                    return tracks.clone();
                }
            }
        }
        let mut rows = Vec::new();
        for track in providers
            .jiosaavn
            .search(artist, 50)
            .await
            .unwrap_or_default()
        {
            if track
                .artist_name
                .to_lowercase()
                .split(',')
                .any(|name| name.trim() == key)
            {
                if let Ok(row) = providers.persist(track).await {
                    rows.push(row);
                }
            }
        }
        let mut cache = self.artist_candidate_cache.lock().await;
        if rows.is_empty() {
            return cache
                .get(&key)
                .map(|(_, rows)| rows.clone())
                .unwrap_or_default();
        }
        cache.retain(|_, (at, _)| at.elapsed().as_secs() < 900);
        if cache.len() >= 64 {
            cache.clear();
        }
        cache.insert(key, (std::time::Instant::now(), rows.clone()));
        rows
    }

    pub async fn search_online_music(
        &self,
        query: &str,
        limit: usize,
    ) -> AppResult<Vec<DiscoveryRecommendation>> {
        let Some(providers) = &self.provider_coordinator else {
            return Ok(vec![]);
        };
        if !providers.enabled().await {
            return Ok(vec![]);
        }
        let mut results = Vec::new();
        for track in providers.jiosaavn.search(query, limit.min(50)).await? {
            results.push(recommendation(
                providers.persist(track).await?,
                "Search result",
            ));
        }
        Ok(results)
    }

    pub(super) async fn seed_artist_editions(&self, user: &str, force: bool) -> AppResult<()> {
        let Some(providers) = &self.provider_coordinator else {
            return Ok(());
        };
        if !providers.enabled().await {
            return Ok(());
        }
        let snapshots:Vec<String>=sqlx::query_scalar("SELECT s.metadata_json FROM online_track_sources s JOIN (
            SELECT track_id,MAX(at) AS at FROM (
                SELECT track_id,updated_at AS at FROM active_playback_journal WHERE user_id=?
                UNION ALL SELECT track_id,COALESCE(last_played_at,0) AS at FROM track_statistics WHERE user_id=? AND (play_count>0 OR manual_like=1)
            ) GROUP BY track_id
        ) recent ON recent.track_id=s.track_id ORDER BY recent.at DESC LIMIT 6")
            .bind(user).bind(user).fetch_all(self.taste_engine.pool()).await?;
        let mut seen = HashSet::new();
        let mut tasks = tokio::task::JoinSet::new();
        for snapshot in snapshots {
            let Ok(source) =
                serde_json::from_str::<crate::providers::ExternalTrackMetadata>(&snapshot)
            else {
                continue;
            };
            if !seen.insert(crate::recommendations::signals::song_key(
                &source.title,
                &source.artist_name,
            )) {
                continue;
            }
            if self
                .artist_edition_cache
                .lock()
                .await
                .get(&source.provider_track_id)
                .is_some_and(|at| !force && at.elapsed().as_secs() < 3600)
            {
                continue;
            }
            let provider = providers.jiosaavn.clone();
            tasks.spawn(async move {
                let result = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    provider.artist_edition(&source),
                )
                .await;
                (
                    source.provider_track_id,
                    result.ok().and_then(Result::ok).flatten(),
                )
            });
            if tasks.len() >= 3 {
                break;
            }
        }
        while let Some(Ok((id, edition))) = tasks.join_next().await {
            if let Some(track) = edition {
                let preferred = track.provider_track_id.clone();
                providers.persist(track).await?;
                sqlx::query("INSERT INTO catalog_artist_editions(source_id,preferred_source_id,checked_at) VALUES(?,?,?) ON CONFLICT(source_id) DO UPDATE SET preferred_source_id=excluded.preferred_source_id,checked_at=excluded.checked_at")
                    .bind(&id).bind(preferred).bind(chrono::Utc::now().timestamp()).execute(self.taste_engine.pool()).await?;
            }
            let mut cache = self.artist_edition_cache.lock().await;
            if cache.len() >= 128 {
                cache.retain(|_, at| at.elapsed().as_secs() < 3600);
                if cache.len() >= 128 {
                    cache.clear();
                }
            }
            cache.insert(id, std::time::Instant::now());
        }
        Ok(())
    }

    /// Metadata preparation happens before recommendation reads / the playback operation lock.
    /// Concurrent consumers share a bounded, revision-checked account pool.
    pub async fn ensure_starter_candidates(&self, user: &str, force: bool) -> AppResult<()> {
        use crate::recommendations::setup;
        let profile = self.taste_engine.compute_taste_profile(user).await?;
        let preferences = setup::load(self.taste_engine.pool(), user).await?;
        if profile.has_learned_taste
            && preferences.artists.is_empty()
            && preferences.genres.is_empty()
        {
            return Ok(());
        }
        let _guard = self.starter_fetch_lock.lock().await;
        let preferences = setup::load(self.taste_engine.pool(), user).await?;
        let current = setup::members(self.taste_engine.pool(), user, &profile).await?;
        {
            let attempts = self.starter_attempts.lock().await;
            if !force
                && attempts.get(user).is_some_and(|(revision, time)| {
                    *revision == preferences.revision
                        && time.elapsed().as_secs() < if current.is_empty() { 30 } else { 900 }
                })
            {
                return Ok(());
            }
        }
        let mut chosen: HashMap<String, (String, String)> = HashMap::new();
        // Offline matching uses only this account's explicit choices, not another account's history.
        for track in self.wishlist_repo.list_catalog_tracks(1000).await? {
            if let Some(artist) = preferences.artists.iter().find(|a| {
                let credits = crate::recommendations::signals::artist_credits(a);
                crate::recommendations::signals::artist_credits(&track.artist)
                    .iter()
                    .any(|a| credits.contains(a))
            }) {
                chosen.insert(track.id, ("artist".into(), artist.clone()));
            } else if let Some(genre) = preferences
                .genres
                .iter()
                .find(|g| genre_matches(track.genre.as_deref().unwrap_or(""), g))
            {
                chosen.insert(track.id, ("genre".into(), genre.clone()));
            }
        }
        if let Some(providers) = &self.provider_coordinator {
            if providers.enabled().await && chosen.len() < 30 {
                let queries: Vec<_> = preferences
                    .artists
                    .iter()
                    .map(|a| ("artist".to_owned(), a.clone()))
                    .chain(
                        preferences
                            .genres
                            .iter()
                            .map(|g| ("genre".to_owned(), g.clone())),
                    )
                    .collect();
                // At most ten short searches, three in flight; audio is never fetched here.
                for batch in queries.chunks(3) {
                    let mut jobs = tokio::task::JoinSet::new();
                    for (source, value) in batch {
                        let providers = providers.clone();
                        let source = source.clone();
                        let value = value.clone();
                        jobs.spawn(async move {
                            let tracks = providers
                                .jiosaavn
                                .search(&value, 25)
                                .await
                                .unwrap_or_default();
                            (source, value, tracks)
                        });
                    }
                    while let Some(Ok((source, value, tracks))) = jobs.join_next().await {
                        for track in tracks {
                            if source == "artist" {
                                let credits =
                                    crate::recommendations::signals::artist_credits(&value);
                                if !crate::recommendations::signals::artist_credits(
                                    &track.artist_name,
                                )
                                .iter()
                                .any(|a| credits.contains(a))
                                {
                                    continue;
                                }
                            }
                            if let Ok(row) = providers.persist(track).await {
                                chosen.insert(row.id, (source.clone(), value.clone()));
                            }
                        }
                    }
                }
            }
        }
        if chosen.len() < 30 && !profile.has_learned_taste {
            // Public chart provenance is safe to share; private account tastes are not.
            let songs = self.get_world_trending(40, force).await.unwrap_or_default();
            for song in songs {
                chosen
                    .entry(song.external_track_id)
                    .or_insert(("popular".into(), String::new()));
            }
        }
        let pool = self.taste_engine.pool();
        let now = chrono::Utc::now().timestamp();
        let mut tx = pool.begin().await?;
        if !chosen.is_empty() {
            sqlx::query("DELETE FROM recommendation_starter_tracks WHERE user_id=? AND revision=? AND COALESCE((SELECT revision FROM recommendation_setup WHERE user_id=?),0)=?")
                .bind(user).bind(preferences.revision).bind(user).bind(preferences.revision).execute(&mut *tx).await?;
        }
        // Never let a slow request from an older saved choice refill the new pool.
        let mut chosen: Vec<_> = chosen.into_iter().collect();
        chosen.sort_by(|a, b| {
            let priority = |source: &str| match source {
                "artist" => 0,
                "genre" => 1,
                _ => 2,
            };
            priority(&a.1 .0)
                .cmp(&priority(&b.1 .0))
                .then_with(|| a.0.cmp(&b.0))
        });
        for (id, (source, value)) in chosen.into_iter().take(250) {
            sqlx::query("INSERT INTO recommendation_starter_tracks(user_id,track_id,source,seed_value,revision,created_at) SELECT ?,?,?,?,?,? WHERE COALESCE((SELECT revision FROM recommendation_setup WHERE user_id=?),0)=? ON CONFLICT(user_id,track_id) DO UPDATE SET source=excluded.source,seed_value=excluded.seed_value,revision=excluded.revision,created_at=excluded.created_at")
                .bind(user).bind(id).bind(source).bind(value).bind(preferences.revision).bind(now).bind(user).bind(preferences.revision).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        let mut attempts = self.starter_attempts.lock().await;
        if attempts.len() >= 16 && !attempts.contains_key(user) {
            if let Some(oldest) = attempts
                .iter()
                .min_by_key(|(_, (_, time))| *time)
                .map(|(key, _)| key.clone())
            {
                attempts.remove(&oldest);
            }
        }
        attempts.insert(
            user.into(),
            (preferences.revision, std::time::Instant::now()),
        );
        Ok(())
    }
}
