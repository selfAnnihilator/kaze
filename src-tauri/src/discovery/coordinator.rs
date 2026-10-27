use super::matcher::{FuzzyTrackMatcher, LocalTrackCandidate, MatchResult, MatchStatus};
use crate::core::error::AppResult;
use crate::database::models::ExternalTrackRecord;
use crate::database::repositories::{TrackRepository, WishlistRepository};
use crate::providers::ProviderCoordinator;
use crate::recommendations::TasteProfileEngine;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryRecommendation {
    pub external_track_id: String,
    pub provider: String,
    pub provider_id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_secs: Option<f64>,
    pub cover_art_url: Option<String>,
    pub preview_url: Option<String>,
    pub genre: Option<String>,
    pub match_status: MatchStatus,
    pub matched_local_track_id: Option<String>,
    pub recommendation_reason: String,
    pub in_wishlist: bool,
}

pub struct DiscoveryCoordinator {
    wishlist_repo: Arc<dyn WishlistRepository>,
    track_repo: Arc<dyn TrackRepository>,
    taste_engine: Arc<TasteProfileEngine>,
    #[allow(dead_code)]
    provider_coordinator: Option<Arc<ProviderCoordinator>>,
    artist_edition_cache: tokio::sync::Mutex<HashMap<String, std::time::Instant>>,
    artist_candidate_cache:
        tokio::sync::Mutex<HashMap<String, (std::time::Instant, Vec<ExternalTrackRecord>)>>,
    world_chart_cache:
        tokio::sync::RwLock<Option<(std::time::Instant, Vec<DiscoveryRecommendation>)>>,
}

/// Checks if a candidate genre fuzzy matches a user's library genre
pub fn genre_matches(candidate_genre: &str, user_genre: &str) -> bool {
    let cg = candidate_genre.to_lowercase();
    let ug = user_genre.to_lowercase();
    if cg.is_empty() || ug.is_empty() {
        return false;
    }
    if cg == ug || cg.contains(&ug) || ug.contains(&cg) {
        return true;
    }
    let is_rap_hiphop = (cg.contains("hip-hop") || cg.contains("hip hop") || cg.contains("rap"))
        && (ug.contains("hip-hop") || ug.contains("hip hop") || ug.contains("rap"));
    let is_electronic =
        (cg.contains("dance") || cg.contains("electronic") || cg.contains("electro"))
            && (ug.contains("dance") || ug.contains("electronic") || ug.contains("electro"));
    let is_rock_alt = (cg.contains("rock") || cg.contains("alternative"))
        && (ug.contains("rock") || ug.contains("alternative"));
    let is_rnb =
        (cg.contains("r&b") || cg.contains("soul")) && (ug.contains("r&b") || ug.contains("soul"));

    let is_asian = (cg.contains("asian")
        || cg.contains("j-pop")
        || cg.contains("anime")
        || cg.contains("k-pop")
        || cg.contains("japanese"))
        && (ug.contains("asian")
            || ug.contains("j-pop")
            || ug.contains("anime")
            || ug.contains("k-pop")
            || ug.contains("japanese"));
    let is_latin_brazil = (cg.contains("brazil")
        || cg.contains("latin")
        || cg.contains("samba")
        || cg.contains("bossa"))
        && (ug.contains("brazil")
            || ug.contains("latin")
            || ug.contains("samba")
            || ug.contains("bossa"));
    let is_indian = (cg.contains("bollywood")
        || cg.contains("indian")
        || cg.contains("desi")
        || cg.contains("hindi")
        || cg.contains("punjabi"))
        && (ug.contains("bollywood")
            || ug.contains("indian")
            || ug.contains("desi")
            || ug.contains("hindi")
            || ug.contains("punjabi"));
    let is_malayalam =
        (cg.contains("malayalam") || cg.contains("regional indian") || cg.contains("indian"))
            && (ug.contains("malayalam")
                || ug.contains("asian")
                || ug.contains("indian")
                || ug.contains("soundtrack"));

    is_rap_hiphop
        || is_electronic
        || is_rock_alt
        || is_rnb
        || is_asian
        || is_latin_brazil
        || is_indian
        || is_malayalam
}

impl DiscoveryCoordinator {
    pub fn new(
        wishlist_repo: Arc<dyn WishlistRepository>,
        track_repo: Arc<dyn TrackRepository>,
        taste_engine: Arc<TasteProfileEngine>,
        provider_coordinator: Option<Arc<ProviderCoordinator>>,
    ) -> Self {
        Self {
            wishlist_repo,
            track_repo,
            taste_engine,
            provider_coordinator,
            artist_edition_cache: tokio::sync::Mutex::new(HashMap::new()),
            artist_candidate_cache: tokio::sync::Mutex::new(HashMap::new()),
            world_chart_cache: tokio::sync::RwLock::new(None),
        }
    }

    pub fn wishlist_repo(&self) -> Arc<dyn WishlistRepository> {
        self.wishlist_repo.clone()
    }

    /// Evaluates fuzzy similarity for an external track candidate against all local library tracks.
    pub async fn match_against_library(
        &self,
        ext_title: &str,
        ext_artist: &str,
        ext_duration: Option<f64>,
        is_lofi_context: bool,
    ) -> AppResult<MatchResult> {
        let local_tracks = self.track_repo.list_tracks(0, 50000, None, true).await?;

        let candidates = local_tracks
            .iter()
            .filter(|t| {
                t.format != "online"
                    && !t.file_path.starts_with("online://")
                    && !t.id.starts_with("itunes:")
                    && !t.id.starts_with("online:")
            })
            .map(|t| {
                let fp = t.file_path.to_lowercase();
                let genre = t.genre_name.as_deref().unwrap_or("").to_lowercase();
                let album = t.album_title.as_deref().unwrap_or("").to_lowercase();
                let title = t.title.to_lowercase();
                let artist = t.artist_name.as_deref().unwrap_or("").to_lowercase();

                let is_lofi = fp.contains("/lofi/")
                    || fp.contains("/lo-fi/")
                    || fp.contains("lofi")
                    || fp.contains("lo-fi")
                    || genre.contains("lofi")
                    || genre.contains("lo-fi")
                    || genre.contains("chillhop")
                    || album.contains("lofi")
                    || album.contains("lo-fi")
                    || title.contains("lofi")
                    || title.contains("lo-fi")
                    || artist.contains("lofi")
                    || artist.contains("lo-fi");

                LocalTrackCandidate {
                    id: t.id.as_str(),
                    title: t.title.as_str(),
                    artist: t.artist_name.as_deref().unwrap_or(""),
                    duration_secs: t.duration_secs,
                    is_lofi,
                }
            });

        Ok(FuzzyTrackMatcher::find_best_match(
            ext_title,
            ext_artist,
            ext_duration,
            is_lofi_context,
            candidates,
        ))
    }

    /// Ingests or updates an external track candidate, running fuzzy matching
    /// against the local library to determine match status and saving to database.
    pub async fn ingest_external_track(
        &self,
        mut track: ExternalTrackRecord,
    ) -> AppResult<MatchResult> {
        let is_lofi = FuzzyTrackMatcher::is_lofi_indicator(&track.title)
            || FuzzyTrackMatcher::is_lofi_indicator(&track.artist);
        let match_result = self
            .match_against_library(&track.title, &track.artist, track.duration_secs, is_lofi)
            .await?;

        track.match_status = match_result.status.as_str().to_string();
        track.matched_local_track_id = match_result.matched_track_id.clone();

        self.wishlist_repo.upsert_external_track(&track).await?;
        Ok(match_result)
    }

    /// Fetches trending hits and popular new music across overall charts and targeted user genres.
    pub async fn charts(&self, refresh: bool) -> AppResult<Vec<serde_json::Value>> {
        let Some(providers) = &self.provider_coordinator else {
            return Ok(vec![]);
        };
        if !providers.enabled().await {
            return Ok(vec![]);
        }
        Ok(providers.jiosaavn.charts(refresh).await?)
    }
    pub async fn get_chart_songs(
        &self,
        chart_id: &str,
        limit: usize,
    ) -> AppResult<Vec<DiscoveryRecommendation>> {
        self.chart_songs(chart_id, limit, false).await
    }
    async fn chart_songs(
        &self,
        chart_id: &str,
        limit: usize,
        refresh: bool,
    ) -> AppResult<Vec<DiscoveryRecommendation>> {
        let Some(providers) = &self.provider_coordinator else {
            return Ok(vec![]);
        };
        if !providers.enabled().await {
            return Ok(vec![]);
        }
        let id = chart_id
            .strip_prefix("jiosaavn:playlist:")
            .ok_or_else(|| crate::core::AppError::Validation("Unknown JioSaavn chart".into()))?;
        let mut rows = Vec::new();
        for track in providers
            .jiosaavn
            .playlist(id, limit.min(50), refresh)
            .await?
        {
            rows.push(recommendation(
                providers.persist(track).await?,
                "Chart selection",
            ));
        }
        Ok(rows)
    }
    /// Provider chart order is separate from personal recommendation ranking.
    pub async fn get_world_trending(
        &self,
        limit: usize,
        refresh: bool,
    ) -> AppResult<Vec<DiscoveryRecommendation>> {
        if !refresh {
            if let Some((at, songs)) = &*self.world_chart_cache.read().await {
                if at.elapsed().as_secs() < 900 {
                    return Ok(songs.iter().take(limit).cloned().collect());
                }
            }
        }
        let charts = self.charts(refresh).await?;
        let mut songs = Vec::new();
        let mut seen = HashSet::new();
        for chart in charts.iter().take(3) {
            let Some(id) = chart["id"].as_str() else {
                continue;
            };
            for mut row in self
                .chart_songs(&format!("jiosaavn:playlist:{id}"), 20, refresh)
                .await
                .unwrap_or_default()
            {
                if seen.insert(row.provider_id.clone()) {
                    row.recommendation_reason =
                        chart["title"].as_str().unwrap_or("Chart selection").into();
                    songs.push(row);
                }
            }
        }
        if songs.is_empty() {
            if let Some((_, cached)) = &*self.world_chart_cache.read().await {
                return Ok(cached.iter().take(limit).cloned().collect());
            }
        } else {
            *self.world_chart_cache.write().await =
                Some((std::time::Instant::now(), songs.clone()));
        }
        Ok(songs.into_iter().take(limit).collect())
    }
    async fn fetch_catalog_candidates(&self, queries: &[String]) -> AppResult<usize> {
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
    async fn artist_candidates(&self, artist: &str, refresh: bool) -> Vec<ExternalTrackRecord> {
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
    async fn seed_artist_editions(&self, user: &str, force: bool) -> AppResult<()> {
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

    /// Ranks catalog candidates by listening affinity, with library signals for cold start.
    pub async fn get_discovery_recommendations(
        &self,
        user_id: &str,
        limit: usize,
        force_refresh: bool,
    ) -> AppResult<Vec<DiscoveryRecommendation>> {
        // 1. Fetch current wishlist to cross-reference
        let wishlist_items = self.wishlist_repo.get_all(None).await?;
        let wishlist_ext_ids: HashSet<String> = wishlist_items
            .iter()
            .filter_map(|w| w.external_track_id.clone())
            .collect();
        let wishlist_keys: HashSet<String> = wishlist_items
            .iter()
            .map(|w| format!("{}:{}", w.artist.to_lowercase(), w.title.to_lowercase()))
            .collect();

        // 2. Analyze user library & listening taste to rank genres and artists
        let local_tracks = self
            .track_repo
            .list_tracks_scoped(Some(user_id), 0, 50000, None, true)
            .await
            .unwrap_or_default();

        let mut genre_counts: HashMap<String, usize> = HashMap::new();
        let mut library_artists: HashSet<String> = HashSet::new();

        for t in local_tracks
            .iter()
            .filter(|t| t.format != "online" && !t.file_path.starts_with("online://"))
        {
            if let Some(ref g) = t.genre_name {
                let clean = g.to_lowercase().trim().to_string();
                if !clean.is_empty() {
                    *genre_counts.entry(clean).or_insert(0) += 1;
                }
            }
            if let Some(ref a) = t.artist_name {
                let clean = a.to_lowercase().trim().to_string();
                if !clean.is_empty() {
                    library_artists.insert(clean);
                }
            }
        }

        let profile = self.taste_engine.compute_taste_profile(user_id).await?;
        let listened_artists: HashMap<String, f64> = profile
            .top_artists
            .iter()
            .map(|a| (a.display_name.to_lowercase().trim().to_string(), a.affinity))
            .collect();
        let mut genre_weights: HashMap<String, f64> = genre_counts
            .into_iter()
            .map(|(genre, count)| (genre, (count as f64).min(10.0)))
            .collect();
        for g in &profile.top_genres {
            let clean = g.display_name.to_lowercase().trim().to_string();
            if !clean.is_empty() {
                // Listening outranks library size, including reduced/zero affinities.
                genre_weights.insert(clean, g.affinity * 100.0);
            }
        }
        let mut ranked_user_genres: Vec<(String, f64)> = genre_weights
            .into_iter()
            .filter(|(_, weight)| *weight > 0.0)
            .collect();
        ranked_user_genres.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        self.seed_artist_editions(user_id, force_refresh).await?;
        let mut ext_tracks = self.wishlist_repo.list_catalog_tracks(1000).await?;
        if self.provider_coordinator.is_some()
            && (force_refresh || ext_tracks.len() < limit.max(30))
        {
            let mut queries: Vec<String> = ranked_user_genres
                .iter()
                .take(3)
                .map(|(genre, _)| genre.clone())
                .collect();
            if queries.is_empty() {
                queries.push("English hits".into());
            }
            let _ = self.fetch_catalog_candidates(&queries).await;
            ext_tracks = self.wishlist_repo.list_catalog_tracks(1000).await?;
        }

        if self.provider_coordinator.is_some() {
            let mut seen: HashSet<String> = ext_tracks.iter().map(|t| t.id.clone()).collect();
            for artist in profile
                .top_artists
                .iter()
                .filter(|a| a.affinity > 0.0)
                .take(4)
            {
                for track in self
                    .artist_candidates(&artist.display_name, force_refresh)
                    .await
                {
                    if seen.insert(track.id.clone()) {
                        ext_tracks.push(track);
                    }
                }
            }
        }

        let signals =
            crate::recommendations::signals::SongSignals::load(self.taste_engine.pool(), user_id)
                .await?;
        // Prefer a single/original album over compilation duplicates before scoring.
        let mut releases: HashMap<String, ExternalTrackRecord> = HashMap::new();
        for track in ext_tracks {
            let key = crate::recommendations::signals::song_key(&track.title, &track.artist)
                .unwrap_or_else(|| track.id.clone());
            let rank = signals.release_rank(
                &track.id,
                &track.title,
                track.album.as_deref(),
                signals.identities.get(&track.id).and_then(|song| song.year),
            );
            let replace = releases.get(&key).is_none_or(|old| {
                rank > signals.release_rank(
                    &old.id,
                    &old.title,
                    old.album.as_deref(),
                    signals.identities.get(&old.id).and_then(|song| song.year),
                )
            });
            if replace {
                releases.insert(key, track);
            }
        }
        let ext_tracks: Vec<_> = releases.into_values().collect();
        // 5. Listening strength drives ordering; refreshing never adds random score noise.
        struct ScoredCandidate {
            track: ExternalTrackRecord,
            score: f64,
            reason: String,
        }

        let mut scored_list: Vec<ScoredCandidate> = Vec::new();

        let candidate_local_tuples: Vec<(&str, &str, &str, f64)> = local_tracks
            .iter()
            .filter(|t| {
                t.format != "online"
                    && !t.file_path.starts_with("online://")
                    && !t.id.starts_with("itunes:")
                    && !t.id.starts_with("online:")
            })
            .map(|t| {
                (
                    t.id.as_str(),
                    t.title.as_str(),
                    t.artist_name.as_deref().unwrap_or(""),
                    t.duration_secs,
                )
            })
            .collect();

        for mut track in ext_tracks {
            if signals.blocked(&track.id, &track.title, &track.artist) {
                continue;
            }
            let listening = signals.signal(&track.id, &track.title, &track.artist);
            if listening.skip_count >= 3 && listening.skip_count > listening.play_count {
                continue;
            }

            let is_lofi = FuzzyTrackMatcher::is_lofi_indicator(&track.title)
                || FuzzyTrackMatcher::is_lofi_indicator(&track.artist);
            // Older chart rows stored the preview length as the song length.
            if track.preview_url.is_some()
                && track.duration_secs.is_some_and(|seconds| seconds <= 35.0)
            {
                track.duration_secs = None;
            }
            let match_res = FuzzyTrackMatcher::find_best_match(
                &track.title,
                &track.artist,
                track.duration_secs,
                is_lofi,
                candidate_local_tuples.iter().copied(),
            );

            let status = match_res.status;
            let status_str = status.as_str().to_string();
            if track.match_status != status_str
                || track.matched_local_track_id != match_res.matched_track_id
            {
                track.match_status = status_str;
                track.matched_local_track_id = match_res.matched_track_id.clone();
                let _ = self.wishlist_repo.upsert_external_track(&track).await;
            }

            let artist_lower = track.artist.to_lowercase().trim().to_string();
            let track_genre = track.genre.as_deref().unwrap_or("").to_lowercase();
            let track_album = track.album.as_deref().unwrap_or("").to_lowercase();

            let artist_affinity = profile.artist_affinity(&track.artist);
            let is_listened_artist = artist_affinity > 0.0;
            let is_library_artist = library_artists.contains(&artist_lower);

            let mut genre_score: f64 = 0.0;
            let mut matched_genre_display: Option<String> = None;

            for (ug, weight) in &ranked_user_genres {
                if genre_matches(&track_genre, ug) || genre_matches(&track_album, ug) {
                    genre_score = *weight;
                    matched_genre_display = Some(track.genre.clone().unwrap_or_else(|| ug.clone()));
                    break;
                }
            }

            let language = signals
                .identities
                .get(&track.id)
                .and_then(|song| song.language.as_ref());
            let language_affinity = language
                .and_then(|language| profile.language_affinities.get(&language.to_lowercase()))
                .copied()
                .unwrap_or(0.0);
            let known = listening.manual_like == 1 || listening.play_count > 0;
            let library_signal = is_library_artist && !listened_artists.contains_key(&artist_lower);
            if !is_listened_artist
                && !library_signal
                && genre_score <= 0.0
                && language_affinity <= 0.0
                && !known
            {
                continue;
            }
            let input = crate::recommendations::scoring::CandidateTrack {
                id: track.id.clone(),
                title: track.title.clone(),
                artist_id: None,
                artist_name: Some(track.artist.clone()),
                genre_name: track.genre.clone(),
                year: signals.identities.get(&track.id).and_then(|song| song.year),
                language: language.cloned(),
                play_count: listening.play_count,
                completion_count: listening.completion_count,
                skip_count: listening.skip_count,
                last_played_at: listening.last_played_at,
                user_feedback: if listening.manual_like == 1 {
                    Some("like".into())
                } else {
                    None
                },
            };
            let recording_score = crate::recommendations::scoring::ScoringEngine::score_track(
                &input,
                &profile,
                chrono::Utc::now().timestamp(),
            )
            .final_score;
            let base_score = 150.0 * artist_affinity
                + genre_score
                + 25.0 * language_affinity
                + if library_signal { 15.0 } else { 0.0 }
                + recording_score * 50.0;

            let reason = match status {
                MatchStatus::NotFound => {
                    if is_listened_artist {
                        format!(
                            "Based on your listening habit for {} ({:.0}% affinity)",
                            track.artist,
                            artist_affinity * 100.0
                        )
                    } else if is_library_artist && !listened_artists.contains_key(&artist_lower) {
                        format!("By your library artist {}", track.artist)
                    } else if let Some(ref genre) = matched_genre_display {
                        format!("Matches your {genre} listening")
                    } else if let Some(language) = language.filter(|_| language_affinity > 0.0) {
                        format!("Matches your {language} listening")
                    } else {
                        "A recording from your listening history".into()
                    }
                }
                MatchStatus::PossibleMatch => {
                    "Alternate release / version of local track".to_string()
                }
                MatchStatus::LikelyMatch => "Remastered edition of local track".to_string(),
                MatchStatus::ExactMatch => "Already in local library".to_string(),
            };

            scored_list.push(ScoredCandidate {
                track,
                score: base_score,
                reason,
            });
        }

        let mut taste_candidates: Vec<_> = scored_list
            .into_iter()
            .filter(|candidate| candidate.score > 0.0)
            .collect();

        taste_candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.track.id.cmp(&b.track.id))
        });

        // 6. Keep artist diversity (maximum two tracks per artist).
        let mut artist_counts: HashMap<String, usize> = HashMap::new();
        let mut recs = Vec::new();
        let mut seen_songs = HashSet::new();

        let to_recommendation = |scored: &ScoredCandidate| {
            let in_wishlist = (scored.track.provider_id.as_str() != ""
                && wishlist_ext_ids.contains(&scored.track.id))
                || wishlist_keys.contains(&format!(
                    "{}:{}",
                    scored.track.artist.to_lowercase(),
                    scored.track.title.to_lowercase()
                ));

            let status = match scored.track.match_status.as_str() {
                "EXACT_MATCH" => MatchStatus::ExactMatch,
                "LIKELY_MATCH" => MatchStatus::LikelyMatch,
                "POSSIBLE_MATCH" => MatchStatus::PossibleMatch,
                _ => MatchStatus::NotFound,
            };

            DiscoveryRecommendation {
                external_track_id: scored.track.id.clone(),
                provider: scored.track.provider.clone(),
                provider_id: scored.track.provider_id.clone(),
                title: scored.track.title.clone(),
                artist: scored.track.artist.clone(),
                album: scored.track.album.clone(),
                duration_secs: scored.track.duration_secs,
                cover_art_url: scored.track.cover_art_url.clone(),
                preview_url: scored.track.preview_url.clone(),
                genre: scored.track.genre.clone(),
                match_status: status,
                matched_local_track_id: scored.track.matched_local_track_id.clone(),
                recommendation_reason: scored.reason.clone(),
                in_wishlist,
            }
        };

        // For You uses only personalized candidates, ordered by actual affinity.
        for scored in &taste_candidates {
            if recs.len() >= limit {
                break;
            }
            if !seen_songs.insert(
                crate::recommendations::signals::song_key(
                    &scored.track.title,
                    &scored.track.artist,
                )
                .unwrap_or_else(|| scored.track.id.clone()),
            ) {
                continue;
            }
            let artist_key = crate::recommendations::signals::artist_credits(&scored.track.artist)
                .into_iter()
                .next()
                .unwrap_or_default();
            let count = artist_counts.entry(artist_key).or_insert(0);
            if *count >= 2 {
                continue;
            }
            *count += 1;
            recs.push(to_recommendation(scored));
        }

        Ok(recs)
    }
}

fn recommendation(row: ExternalTrackRecord, reason: &str) -> DiscoveryRecommendation {
    DiscoveryRecommendation {
        external_track_id: row.id,
        provider: row.provider,
        provider_id: row.provider_id,
        title: row.title,
        artist: row.artist,
        album: row.album,
        duration_secs: row.duration_secs,
        cover_art_url: row.cover_art_url,
        preview_url: None,
        genre: row.genre,
        match_status: MatchStatus::NotFound,
        matched_local_track_id: None,
        recommendation_reason: reason.into(),
        in_wishlist: false,
    }
}
