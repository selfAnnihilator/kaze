//! The For You discovery feed.
use super::*;

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
        self.ensure_starter_candidates(user_id, force_refresh)
            .await?;
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

        let similarity = crate::recommendations::similarity::SimilarityIndex::load(
            self.taste_engine.pool(),
            user_id,
            &crate::recommendations::similarity::seeds_from_profile(&profile),
        )
        .await
        .unwrap_or_default();
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
            // "Fans of your artists also like": search a few similar, unheard artists.
            let exclude: HashSet<String> = profile
                .top_artists
                .iter()
                .filter(|a| a.affinity > 0.0)
                .flat_map(|a| crate::recommendations::signals::artist_credits(&a.display_name))
                .collect();
            for hit in similarity.top_similar(3, &exclude) {
                for track in self.artist_candidates(&hit.display, force_refresh).await {
                    if seen.insert(track.id.clone()) {
                        ext_tracks.push(track);
                    }
                }
            }
        }

        let signals =
            crate::recommendations::signals::SongSignals::load(self.taste_engine.pool(), user_id)
                .await?;
        let starters =
            crate::recommendations::setup::members(self.taste_engine.pool(), user_id, &profile)
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
        #[derive(Clone)]
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
            let similar_hit = similarity.hit(&track.artist).filter(|h| h.boost > 0.2);
            let library_signal = is_library_artist && !listened_artists.contains_key(&artist_lower);
            if !is_listened_artist
                && !library_signal
                && genre_score <= 0.0
                && language_affinity <= 0.0
                && !known
                && similar_hit.is_none()
                && !starters.contains_key(&track.id)
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
                listening_engagement: listening.engagement(),
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
            // Style (genre/language) leads; artist affinity is saturating and already
            // includes the owned-library prior, so it is weighted more gently.
            let base_score = 90.0 * artist_affinity
                + genre_score
                + 35.0 * language_affinity
                + if library_signal { 10.0 } else { 0.0 }
                + similar_hit.map_or(0.0, |h| 40.0 * h.boost * (1.0 - artist_affinity))
                + recording_score * 50.0;

            let reason = match status {
                MatchStatus::NotFound => {
                    if is_listened_artist && profile.artist_breadth(&track.artist) > 0 {
                        format!(
                            "Based on your listening habit for {} ({:.0}% affinity)",
                            track.artist,
                            artist_affinity * 100.0
                        )
                    } else if is_library_artist {
                        format!("By your library artist {}", track.artist)
                    } else if let Some(hit) = similar_hit {
                        format!("Fans of {} also like {}", hit.because_of, track.artist)
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

            let (base_score, reason) = if let Some((source, value)) = starters.get(&track.id) {
                (
                    base_score.max(10.0),
                    crate::recommendations::setup::explanation(source, value),
                )
            } else {
                (base_score, reason)
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

        // Manual refresh is a new selection, not a new taste score. Rotate only
        // among similarly relevant songs by the same artist, retaining the global
        // taste-ranked artist slots and the existing two-song diversity cap.
        let family = |candidate: &ScoredCandidate| {
            crate::recommendations::signals::song_key(
                &candidate.track.title,
                &candidate.track.artist,
            )
            .unwrap_or_else(|| candidate.track.id.clone())
        };
        let artist = |candidate: &ScoredCandidate| {
            crate::recommendations::signals::artist_credits(&candidate.track.artist)
                .into_iter()
                .next()
                .unwrap_or_default()
        };
        let ranked_ids: Vec<_> = taste_candidates
            .iter()
            .map(|c| c.track.id.clone())
            .collect();
        let mut selections = self.for_you_selections.lock().await;
        if !selections.contains_key(user_id) && selections.len() >= 16 {
            if let Some(oldest) = selections
                .iter()
                .min_by_key(|(_, state)| state.touched)
                .map(|(user, _)| user.clone())
            {
                selections.remove(&oldest);
            }
        }
        let selection = selections.entry(user_id.to_string()).or_default();
        // New listening/feedback/catalog ordering can supersede the current batch.
        if selection.ranked_ids != ranked_ids {
            selection.current.clear();
        }
        selection.ranked_ids = ranked_ids;
        let available: HashSet<_> = taste_candidates.iter().map(&family).collect();
        selection
            .last_shown
            .retain(|key, _| available.contains(key));
        let mut artists: HashMap<String, Vec<usize>> = HashMap::new();
        for (index, candidate) in taste_candidates.iter().enumerate() {
            artists.entry(artist(candidate)).or_default().push(index);
        }
        for indices in artists.values() {
            let best = taste_candidates[indices[0]].score;
            let eligible: Vec<_> = indices
                .iter()
                .copied()
                .filter(|&i| taste_candidates[i].score >= best * 0.85)
                .collect();
            if eligible.len() <= limit.min(2) {
                continue;
            }
            let mut choices: Vec<_> = eligible
                .iter()
                .map(|&i| taste_candidates[i].clone())
                .collect();
            choices.sort_by(|a, b| {
                let ka = family(a);
                let kb = family(b);
                let priority = if force_refresh {
                    selection
                        .last_shown
                        .get(&ka)
                        .copied()
                        .unwrap_or(0)
                        .cmp(&selection.last_shown.get(&kb).copied().unwrap_or(0))
                } else {
                    selection
                        .current
                        .get(&ka)
                        .copied()
                        .unwrap_or(usize::MAX)
                        .cmp(&selection.current.get(&kb).copied().unwrap_or(usize::MAX))
                };
                priority
                    .then_with(|| b.score.total_cmp(&a.score))
                    .then_with(|| a.track.id.cmp(&b.track.id))
            });
            for (index, candidate) in eligible.into_iter().zip(choices) {
                taste_candidates[index] = candidate;
            }
        }

        // 6. Keep artist diversity (one song per artist in every eight cards).
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

        // For You uses only personalized candidates, ordered by actual affinity
        // (and sticky selection). Rank is the diversifier score so that order holds
        // wherever artist spacing allows it.
        let unique: Vec<&ScoredCandidate> = taste_candidates
            .iter()
            .filter(|scored| {
                seen_songs.insert(
                    crate::recommendations::signals::song_key(
                        &scored.track.title,
                        &scored.track.artist,
                    )
                    .unwrap_or_else(|| scored.track.id.clone()),
                )
            })
            .collect();
        let items: Vec<crate::recommendations::diversity::DiversityItem> = unique
            .iter()
            .enumerate()
            .map(|(rank, scored)| crate::recommendations::diversity::DiversityItem {
                score: -(rank as f64),
                artist: scored.track.artist.clone(),
                album: None,
                is_online: false,
                max_per_window: None,
                features: None,
            })
            .collect();
        let recs: Vec<DiscoveryRecommendation> = crate::recommendations::diversity::diversify(
            &items,
            limit,
            &crate::recommendations::diversity::DiversityRules::shelf(),
            &[],
        )
        .into_iter()
        .map(|index| to_recommendation(unique[index]))
        .collect();

        if force_refresh || selection.generation == 0 {
            selection.generation = selection.generation.saturating_add(1);
        }
        selection.current.clear();
        for (index, rec) in recs.iter().enumerate() {
            let key = crate::recommendations::signals::song_key(&rec.title, &rec.artist)
                .unwrap_or_else(|| rec.external_track_id.clone());
            selection.last_shown.insert(key.clone(), selection.generation);
            selection.current.insert(key, index);
        }
        selection.touched = Some(std::time::Instant::now());
        Ok(recs)
    }
}
