//! Scoring candidate songs for Flow and playback continuation.
use super::*;

impl FlowEngine {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn generate_candidates(
        &self,
        user_id: &str,
        familiarity_ratio: f64,
        limit: usize,
        exclude_track_ids: &[String],
        recent_artists: &[String],
        include_online: bool,
        session: Option<&SessionSteer>,
        // `Some(_)` enables transition smoothing; the inner id seeds the first pick.
        transitions: Option<Option<&str>>,
    ) -> AppResult<Vec<FlowTrackItem>> {
        let session = session.filter(|steer| !steer.is_empty());
        if !familiarity_ratio.is_finite() {
            return Err(AppError::Validation("Flow steering must be finite".into()));
        }
        let limit = limit.min(100);
        if limit == 0 {
            return Ok(Vec::new());
        }
        let ratio = familiarity_ratio.clamp(0.0, 1.0);
        let profile = self.taste_engine.compute_taste_profile(user_id).await?;
        let signals = super::super::signals::SongSignals::load(&self.pool, user_id).await?;
        let starters = super::super::setup::members(&self.pool, user_id, &profile).await?;
        let similarity = super::super::similarity::SimilarityIndex::load(
            &self.pool,
            user_id,
            &super::super::similarity::seeds_from_profile(&profile),
        )
        .await
        .unwrap_or_default();
        let now = Utc::now().timestamp();
        // Learned from how explore vs familiar picks landed; playback Flow only.
        let explore_bias = if include_online {
            super::super::exploration::explore_bias(&self.pool, user_id, now)
                .await
                .unwrap_or(0.0)
        } else {
            0.0
        };
        let novelty_weight = 0.55 * (1.0 + explore_bias);

        let mut candidates = sqlx::query_as::<_, FlowCandidateRow>(
            "SELECT t.id, t.title, t.artist_id, a.name as artist_name,
                    t.album_id, al.title as album_title, g.name as genre_name, t.year,
                    t.duration_secs,
                    CASE WHEN t.has_cover_art = 1 THEN '/api/cover/' || t.id ELSE NULL END as cover_art_url,
                    ts.play_count, ts.completion_count, ts.skip_count, ts.last_played_at,
                    ts.manual_like, 0 as is_online, NULL as preview_url
             FROM tracks t
             LEFT JOIN artists a ON a.id = t.artist_id
             LEFT JOIN albums al ON al.id = t.album_id
             LEFT JOIN genres g ON g.id = t.genre_id
             LEFT JOIN track_statistics ts ON ts.track_id = t.id AND ts.user_id = ?
             WHERE t.format != 'online' AND t.file_path NOT LIKE 'online://%'"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        if include_online {
            let online = sqlx::query_as::<_, FlowCandidateRow>(
                "SELECT e.id, e.title, a.id as artist_id, e.artist as artist_name,
                        t.album_id, e.album as album_title, COALESCE(g.name, e.genre) as genre_name, t.year,
                        COALESCE(e.duration_secs, t.duration_secs, 210.0) as duration_secs,
                        e.cover_art_url, ts.play_count, ts.completion_count, ts.skip_count, ts.last_played_at,
                        CASE WHEN ts.manual_like = -1 THEN -1
                             WHEN liked.track_id IS NOT NULL THEN 1 ELSE ts.manual_like END as manual_like,
                        1 as is_online, e.preview_url
                 FROM external_tracks e
                 LEFT JOIN tracks t ON t.id = e.id
                 LEFT JOIN artists a ON a.normalized_name = lower(trim(e.artist))
                 LEFT JOIN genres g ON g.id = t.genre_id
                 LEFT JOIN track_statistics ts ON ts.track_id = e.id AND ts.user_id = ?
                 LEFT JOIN (
                    SELECT DISTINCT pt.track_id FROM playlist_tracks pt
                    JOIN playlists p ON p.id = pt.playlist_id
                    WHERE p.user_id = ? AND p.name = 'Liked Songs' AND p.is_smart_mix = 0
                 ) liked ON liked.track_id = e.id
                 WHERE (e.provider = 'jiosaavn' OR EXISTS (SELECT 1 FROM online_track_sources source WHERE source.track_id=e.id)) AND (t.id IS NULL OR t.format = 'online' OR t.file_path LIKE 'online://%')
                   AND NOT EXISTS (
                       SELECT 1 FROM tracks local WHERE local.id = e.matched_local_track_id
                         AND local.format != 'online' AND local.file_path NOT LIKE 'online://%'
                   )
                 ORDER BY (liked.track_id IS NOT NULL OR COALESCE(ts.play_count, 0) > 0) DESC,
                          COALESCE(ts.play_count, 0) DESC, e.created_at DESC, e.id
                 LIMIT 2000"
            ).bind(user_id).bind(user_id).fetch_all(&self.pool).await
                .map_err(|e| AppError::Database(e.to_string()))?;
            candidates.extend(
                online
                    .into_iter()
                    .map(|mut c| {
                        let signal =
                            signals.signal(&c.id, &c.title, c.artist_name.as_deref().unwrap_or(""));
                        c.play_count = Some(signal.play_count);
                        c.completion_count = Some(signal.completion_count);
                        c.skip_count = Some(signal.skip_count);
                        c.manual_like = Some(signal.manual_like);
                        c.last_played_at = signal.last_played_at;
                        c
                    })
                    .filter(|c| {
                        let known = c.manual_like == Some(1)
                            || c.last_played_at.is_some()
                            || c.play_count.unwrap_or(0) > 0;
                        let artist_match =
                            profile.artist_affinity(c.artist_name.as_deref().unwrap_or("")) > 0.15;
                        let language_match = signals
                            .identities
                            .get(&c.id)
                            .and_then(|song| song.language.as_ref())
                            .and_then(|language| {
                                profile.language_affinities.get(&language.to_lowercase())
                            })
                            .copied()
                            .unwrap_or(0.0)
                            > 0.15;
                        let genre_match = c.genre_name.as_ref().is_some_and(|genre| {
                            profile.genre_affinities.iter().any(|(name, affinity)| {
                                name.eq_ignore_ascii_case(genre.trim()) && *affinity > 0.15
                            })
                        });
                        // A style skipped repeatedly this session gets no online exploration.
                        let session_blocked = !known
                            && session.is_some_and(|steer| {
                                steer.genre_score(c.genre_name.as_deref())
                                    <= super::super::session::BLOCK_GENRE_SCORE
                            });
                        // "Fans of your artists also like" reaches beyond tag overlap.
                        let similar_match = similarity
                            .boost(c.artist_name.as_deref().unwrap_or(""))
                            > SIMILAR_CANDIDATE_MIN;
                        !session_blocked
                            && (known
                                || artist_match
                                || similar_match
                                || genre_match
                                || language_match
                                || starters.contains_key(&c.id))
                    }),
            );
        }

        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        let exclude_set: HashSet<&str> = exclude_track_ids.iter().map(|s| s.as_str()).collect();
        let excluded_songs: HashSet<_> = candidates
            .iter()
            .filter(|c| exclude_set.contains(c.id.as_str()) || c.manual_like == Some(-1))
            .map(|c| Self::song_key(&c.title, c.artist_name.as_deref().unwrap_or("")))
            .collect();
        let mut preferred: HashMap<String, ((bool, u8, i64), String)> = HashMap::new();
        for candidate in &candidates {
            if !candidate.is_online {
                continue;
            }
            let key = Self::song_key(
                &candidate.title,
                candidate.artist_name.as_deref().unwrap_or(""),
            );
            let rank = signals.release_rank(
                &candidate.id,
                &candidate.title,
                candidate.album_title.as_deref(),
                signals
                    .identities
                    .get(&candidate.id)
                    .and_then(|song| song.year)
                    .or(candidate.year),
            );
            if preferred.get(&key).is_none_or(|(old, _)| rank > *old) {
                preferred.insert(key, (rank, candidate.id.clone()));
            }
        }
        candidates.retain(|candidate| {
            !candidate.is_online
                || preferred
                    .get(&Self::song_key(
                        &candidate.title,
                        candidate.artist_name.as_deref().unwrap_or(""),
                    ))
                    .is_none_or(|(_, id)| id == &candidate.id)
        });

        let mut seen_songs = HashSet::new();
        let context_now = super::super::context::current(&profile.context_affinity, now);

        struct ScoredCandidate {
            candidate: FlowCandidateRow,
            score: f64,
            reason: String,
            reason_category: String,
            explanation: FlowExplanation,
        }

        let mut scored: Vec<ScoredCandidate> = Vec::new();

        for mut c in candidates {
            if signals.blocked(&c.id, &c.title, c.artist_name.as_deref().unwrap_or("")) {
                continue;
            }
            let signal = signals.signal(&c.id, &c.title, c.artist_name.as_deref().unwrap_or(""));
            c.play_count = Some(signal.play_count);
            c.completion_count = Some(signal.completion_count);
            c.skip_count = Some(signal.skip_count);
            c.last_played_at = signal.last_played_at;
            c.manual_like = Some(signal.manual_like);

            // Disliked tracks are strictly excluded
            if c.manual_like == Some(-1) {
                continue;
            }

            // Excluded tracks (already queued or recently played in session) are skipped
            let identity = Self::song_key(&c.title, c.artist_name.as_deref().unwrap_or(""));
            if exclude_set.contains(c.id.as_str())
                || excluded_songs.contains(&identity)
                || !seen_songs.insert(identity)
            {
                continue;
            }

            let play_count = c.play_count.unwrap_or(0);
            let completion_count = c.completion_count.unwrap_or(0);
            let skip_count = c.skip_count.unwrap_or(0);
            let is_liked = c.manual_like == Some(1);

            let artist_aff = c
                .artist_id
                .as_ref()
                .and_then(|aid| profile.artist_affinities.get(aid))
                .copied()
                .unwrap_or(0.0)
                .max(profile.artist_affinity(c.artist_name.as_deref().unwrap_or("")));

            let genre_aff = c
                .genre_name
                .as_ref()
                .and_then(|g| profile.genre_affinities.get(g.trim()))
                .copied()
                .unwrap_or(0.0);
            let identity = signals.identities.get(&c.id);
            let language_aff = identity
                .and_then(|song| song.language.as_ref())
                .and_then(|language| profile.language_affinities.get(&language.to_lowercase()))
                .copied()
                .unwrap_or(0.0);
            let era_aff = c
                .year
                .or_else(|| identity.and_then(|song| song.year))
                .and_then(|year| profile.era_affinities.get(&format!("{}0s", year / 10)))
                .copied()
                .unwrap_or(0.0);

            // Reuse the existing recommendation scorer for taste, era, completion,
            // explicit feedback and recency. Flow adds only steering/rediscovery.
            let input = CandidateTrack {
                id: c.id.clone(),
                title: c.title.clone(),
                artist_id: c.artist_id.clone(),
                artist_name: c.artist_name.clone(),
                genre_name: c.genre_name.clone(),
                year: c
                    .year
                    .or_else(|| signals.identities.get(&c.id).and_then(|song| song.year)),
                language: signals
                    .identities
                    .get(&c.id)
                    .and_then(|song| song.language.clone()),
                play_count,
                completion_count,
                skip_count,
                last_played_at: c.last_played_at,
                listening_engagement: signals.signal(&c.id, &c.title, c.artist_name.as_deref().unwrap_or("")).engagement(),
                user_feedback: if is_liked { Some("like".into()) } else { None },
            };
            let taste_score = ScoringEngine::score_track(&input, &profile, now).final_score;
            // Familiarity means known songs, rather than every unheard song by a favorite artist.
            let known_strength = if is_liked {
                1.0
            } else {
                ((play_count.max(0) as f64).ln_1p() / 6.0_f64.ln_1p()).min(1.0)
            };
            // Style match: how well the song fits the genres/languages/eras the
            // user enjoys, independent of who performs it.
            let style_match = ((genre_aff * 0.5 + language_aff * 0.3 + era_aff * 0.2) * 0.6
                + taste_score.min(1.0) * 0.4)
                .clamp(0.0, 1.0);
            // Novelty: never-heard songs, preferably by artists the user barely knows.
            let novelty = (1.0 - known_strength) * 0.6 + (1.0 - artist_aff) * 0.4;
            // Songs heard very recently should not dominate Familiar either.
            let recency = c.last_played_at.map_or(1.0, |last_played| {
                let age = (now - last_played).max(0);
                if age < 86400 {
                    0.20
                } else if age < 3 * 86400 {
                    0.50
                } else if age < 7 * 86400 {
                    0.80
                } else {
                    1.0
                }
            });

            // Unfamiliar (0) → new songs/artists in the user's styles.
            // Familiar (1) → known favorites and artists with broad enjoyment.
            let familiar_part = ratio * (known_strength * recency * 0.35 + artist_aff * 0.20);
            let explore_part = (1.0 - ratio) * novelty * novelty_weight;
            let mut final_score = style_match * 0.45 + familiar_part + explore_part;
            let is_explore = c.is_online || explore_part > familiar_part;
            // "Fans of your artists also like": favours unknown artists near your taste.
            let similar_hit = similarity.hit(c.artist_name.as_deref().unwrap_or(""));
            let similar_boost = similar_hit.map_or(0.0, |h| h.boost) * (1.0 - artist_aff);
            final_score += SIMILAR_WEIGHT * similar_boost;
            // Time-of-day context: styles this user favours at this hour/day type.
            let context_hit = context_now.map(
                |(part, affinity)| {
                    (
                        part,
                        super::super::context::lift(
                            affinity,
                            c.genre_name.as_deref(),
                            identity.and_then(|song| song.language.as_deref()),
                        ),
                    )
                },
            );
            let context_lift = context_hit.map_or(0.0, |(_, lift)| lift);
            final_score += super::super::context::CONTEXT_WEIGHT * context_lift;
            let context_note = context_hit
                .filter(|(_, lift)| *lift >= 0.25)
                .map(|(part, _)| part.as_str().to_string());
            // "Less like this": soft, fading artist/genre penalty.
            final_score -= signals.less_penalty(
                c.artist_name.as_deref().unwrap_or(""),
                c.genre_name.as_deref(),
                now,
            );
            let similar_seed = similar_hit
                .filter(|_| similar_boost >= 0.1)
                .map(|h| h.because_of.clone());

            // Forgotten favorites (the old "rediscover" end) peak in the middle.
            if let Some(last_played) = c.last_played_at {
                let days = (now - last_played).max(0) / 86400;
                if days >= 30 && (is_liked || play_count >= 2) {
                    final_score += 0.25 * (1.0 - (ratio - 0.5).abs() * 2.0);
                }
            }
            if skip_count > 2 && skip_count > completion_count {
                final_score -= ((skip_count - completion_count) as f64 * 0.05).min(0.35);
            }
            // Live session steering: recent skips/full listens of this style.
            let mut session_note = None;
            if let Some(steer) = session {
                let keys = StyleKeys::new(
                    c.artist_name.as_deref(),
                    c.genre_name.as_deref(),
                    identity.and_then(|song| song.language.as_deref()),
                );
                let adjustment = steer.adjustment(&keys);
                final_score += adjustment;
                if adjustment.abs() >= 0.1 {
                    let style = if keys.genre.as_deref().is_some_and(|g| {
                        steer.score(StyleKind::Genre, g).abs() >= 0.5
                    }) {
                        c.genre_name.clone()
                    } else {
                        identity.and_then(|song| song.language.clone())
                    };
                    session_note = Some((style, adjustment));
                }
            }
            final_score = final_score.max(if starters.contains_key(&c.id) {
                0.1
            } else {
                0.0
            });
            if c.is_online
                && !is_liked
                && (final_score <= 0.0 || (play_count == 0 && skip_count > 0))
            {
                // An online exploration slot must not force repeatedly skipped songs.
                continue;
            }
            let (reason, reason_category) = if let Some((source, value)) = starters.get(&c.id) {
                (
                    super::super::setup::explanation(source, value),
                    format!("starter_{source}"),
                )
            } else {
                Self::explain(&c, &profile, now)
            };

            let mut explanation = Self::evidence(&c, &profile, now);
            if reason_category.starts_with("starter_") {
                explanation.primary = reason_category.clone();
            }
            explanation.familiarity_ratio = Some(ratio);
            explanation.selected_at = Some(now);
            explanation.similar_artist = similar_seed;
            explanation.context_bucket = context_note;
            explanation.rec_slot = Some(
                if is_explore {
                    super::super::exploration::EXPLORE
                } else {
                    super::super::exploration::FAMILIAR
                }
                .to_string(),
            );
            if is_explore && explore_bias.abs() >= super::super::exploration::NOTICE_BIAS {
                explanation.explore_bias = Some((explore_bias * 100.0).round() / 100.0);
            }
            if let Some((style, adjustment)) = session_note {
                explanation.session_style = style.map(|s| s.trim().to_string());
                explanation.session_adjustment = Some((adjustment * 100.0).round() / 100.0);
            }
            scored.push(ScoredCandidate {
                candidate: c,
                score: final_score,
                reason,
                reason_category,
                explanation,
            });
        }

        // Sort descending by final score
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.candidate.id.cmp(&b.candidate.id))
        });

        // Local audio descriptors drive gentle tempo/energy transitions.
        let sounds = if transitions.is_some() {
            crate::audio_features::load_vectors(&self.pool)
                .await
                .unwrap_or_default()
        } else {
            std::collections::HashMap::new()
        };

        // Radio-style diversification shared with other surfaces: artists are
        // spaced and capped across refills (via `recent_artists`), and an online
        // share is reserved for exploration. No artist gets an exception, not even
        // the most played one: Familiar means more of your styles, not one artist.
        let items: Vec<super::super::diversity::DiversityItem> = scored
            .iter()
            .map(|item| {
                let artist = item
                    .candidate
                    .artist_name
                    .clone()
                    .unwrap_or_else(|| "Unknown Artist".into());
                super::super::diversity::DiversityItem {
                    score: item.score,
                    artist,
                    album: item.candidate.album_title.clone(),
                    is_online: item.candidate.is_online,
                    max_per_window: None,
                    features: sounds.get(&item.candidate.id).copied(),
                }
            })
            .collect();
        let online_share = if include_online {
            ((1.0 - ratio) * 0.6 + explore_bias).clamp(0.0, 0.7)
        } else {
            0.0
        };
        let mut rules = super::super::diversity::DiversityRules::queue(online_share);
        if transitions.is_some() && !sounds.is_empty() {
            rules.transition_weight = TRANSITION_WEIGHT;
        }
        let seed_sound = transitions
            .flatten()
            .and_then(|id| sounds.get(id).copied());
        let order = super::super::diversity::diversify_from(
            &items,
            limit,
            &rules,
            recent_artists,
            seed_sound,
        );
        drop(items);
        let mut slots: Vec<Option<ScoredCandidate>> = scored.into_iter().map(Some).collect();
        let mut selected = Vec::with_capacity(order.len());
        for index in order {
            let Some(item) = slots[index].take() else { continue };
            if include_online && item.candidate.is_online {
                crate::recommendations::mixes::ensure_online_track(
                    &self.pool,
                    &OnlineTrackInput {
                        id: &item.candidate.id,
                        title: &item.candidate.title,
                        artist: item.candidate.artist_name.as_deref(),
                        album: item.candidate.album_title.as_deref(),
                        duration_secs: Some(item.candidate.duration_secs),
                        cover_art_url: item.candidate.cover_art_url.as_deref(),
                        preview_url: item.candidate.preview_url.as_deref(),
                    },
                )
                .await?;
            }
            selected.push(FlowTrackItem {
                track_id: item.candidate.id,
                title: item.candidate.title,
                artist: item
                    .candidate
                    .artist_name
                    .unwrap_or_else(|| "Unknown Artist".into()),
                album: item.candidate.album_title,
                duration_secs: item.candidate.duration_secs,
                cover_art_url: item.candidate.cover_art_url,
                score: item.score,
                reason: item.reason,
                reason_category: item.reason_category,
                explanation: item.explanation,
            });
        }

        if include_online {
            let slots: Vec<(String, &'static str)> = selected
                .iter()
                .map(|t| {
                    let slot = if t.explanation.rec_slot.as_deref()
                        == Some(super::super::exploration::EXPLORE)
                    {
                        super::super::exploration::EXPLORE
                    } else {
                        super::super::exploration::FAMILIAR
                    };
                    (t.track_id.clone(), slot)
                })
                .collect();
            if let Err(error) =
                super::super::exploration::record_slots(&self.pool, user_id, &slots, now).await
            {
                tracing::warn!(%error, "Failed to record Flow exploration slots");
            }
        }

        Ok(selected)
    }
}
