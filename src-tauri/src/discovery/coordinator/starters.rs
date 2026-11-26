//! Online candidates: catalog search, artist editions and starter songs.
use super::*;
use super::starter_languages::StarterRequest;
use crate::database::repositories::taste_setup::{save_starter_tracks, StarterTracks};

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

    /// Concurrent consumers share a bounded, revision-checked account pool.
    pub async fn ensure_starter_candidates(&self, user: &str, force: bool) -> AppResult<()> {
        use crate::recommendations::setup;
        let _guard = self.starter_fetch_lock.lock().await;
        let pool = self.taste_engine.pool();
        let preferences = setup::load(pool, user).await?;
        let profile = self.taste_engine.compute_taste_profile(user).await?;
        let languages = profile.language_preference.seed_languages(&preferences.languages);
        if profile.has_learned_taste && preferences.artists.is_empty()
            && preferences.genres.is_empty() && languages.is_empty()
        {
            return Ok(());
        }
        let request = StarterRequest { revision: preferences.revision, languages };
        let current = setup::members(pool, user, &profile).await?;
        if self.recent_starter_attempt(user, &request, force, current.is_empty()).await {
            return Ok(());
        }
        let chosen = if request.languages.is_empty() {
            self.general_starters(&preferences, profile.has_learned_taste, force).await?
        } else {
            let mut chosen = self.language_starters(&request.languages).await?;
            self.add_search_starters(&preferences, &request.languages, &mut chosen).await;
            chosen
        };
        save_starter_tracks(pool, user, preferences.revision, chosen).await?;
        self.remember_starter_attempt(user, request).await;
        Ok(())
    }

    async fn general_starters(
        &self,
        preferences: &crate::recommendations::setup::RecommendationSetup,
        learned_taste: bool,
        force: bool,
    ) -> AppResult<StarterTracks> {
        let mut chosen: HashMap<String, (String, String)> = HashMap::new();
        // Offline matching uses only this account's explicit choices, not another account's history.
        for track in self.wishlist_repo.list_catalog_tracks(1000).await? {
            if let Some(artist) = preferences.artists.iter().find(|artist| artist_matches(artist, &track.artist)) {
                chosen.insert(track.id, ("artist".into(), artist.clone()));
            } else if let Some(genre) = preferences
                .genres
                .iter()
                .find(|g| genre_matches(track.genre.as_deref().unwrap_or(""), g))
            {
                chosen.insert(track.id, ("genre".into(), genre.clone()));
            }
        }
        if chosen.len() < 30 {
            self.add_search_starters(preferences, &[], &mut chosen).await;
        }
        if chosen.len() < 30 && !learned_taste {
            // Public chart provenance is safe to share; private account tastes are not.
            let songs = self.get_world_trending(40, force).await.unwrap_or_default();
            for song in songs {
                chosen
                    .entry(song.external_track_id)
                    .or_insert(("popular".into(), String::new()));
            }
        }
        Ok(chosen)
    }

    async fn add_search_starters(
        &self,
        preferences: &crate::recommendations::setup::RecommendationSetup,
        languages: &[String],
        chosen: &mut StarterTracks,
    ) {
        let Some(providers) = &self.provider_coordinator else { return; };
        if !providers.enabled().await {
            return;
        }
        let genre_budget = if languages.is_empty() { 12 } else { 3 };
        let queries: Vec<_> = preferences.artists.iter().map(|artist| ("artist", artist))
            .chain(preferences.genres.iter().take(genre_budget).map(|genre| ("genre", genre)))
            .collect();
        let languages: Arc<[String]> = Arc::from(languages);
        // Keep at most three provider searches in flight.
        for batch in queries.chunks(3) {
            let mut jobs = tokio::task::JoinSet::new();
            for &(source, value) in batch {
                let providers = providers.clone();
                let value = value.clone();
                let languages = Arc::clone(&languages);
                jobs.spawn(async move { search_starters(&providers, source, value, &languages).await });
            }
            while let Some(Ok(rows)) = jobs.join_next().await {
                chosen.extend(rows);
            }
        }
    }
}

async fn search_starters(
    providers: &ProviderCoordinator, source: &str, value: String, languages: &[String],
) -> StarterTracks {
    let tracks = providers.jiosaavn.search(&value, 25).await.unwrap_or_default();
    let mut rows = StarterTracks::new();
    let tracks = tracks.into_iter().filter(|track| {
        super::starter_languages::matches_language(languages, track.language.as_deref())
            && (source != "artist" || artist_matches(&value, &track.artist_name))
    });
    for track in tracks {
        if let Ok(row) = providers.persist(track).await {
            rows.insert(row.id, (source.into(), value.clone()));
        }
    }
    rows
}

fn artist_matches(seed: &str, artist: &str) -> bool {
    use crate::recommendations::signals::artist_credits;
    let credits = artist_credits(seed);
    artist_credits(artist).iter().any(|name| credits.contains(name))
}
