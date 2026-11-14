//! Building one smart mix by type.
use super::*;

impl SmartMixGenerator {
    /// Generates a smart mix according to the specified type and persists it as a smart playlist.
    pub async fn generate_mix(
        &self,
        user_id: &str,
        mix_type: &SmartMixType,
    ) -> AppResult<PlaylistRecord> {
        self.generate_mix_with(user_id, mix_type, None).await
    }

    /// `clusters` lets a caller building several Daily Mix N reuse one clustering pass.
    pub(super) async fn generate_mix_with(
        &self,
        user_id: &str,
        mix_type: &SmartMixType,
        clusters: Option<&[super::super::clusters::Cluster]>,
    ) -> AppResult<PlaylistRecord> {
        let now = Utc::now().timestamp();
        let (name, description, mix_type_str, generation_reason, track_ids) = match mix_type {
            SmartMixType::DailyCluster(index) => {
                let computed;
                let clusters = match clusters {
                    Some(clusters) => clusters,
                    None => {
                        computed = self.taste_clusters(user_id).await?;
                        &computed[..]
                    }
                };
                let cluster = clusters
                    .get((*index as usize).wrapping_sub(1))
                    .ok_or_else(|| {
                        AppError::NotFound(format!("No taste cluster for Daily Mix {index}"))
                    })?;
                let track_ids = self.generate_cluster_mix(user_id, cluster).await?;
                (
                    format!("Daily Mix {index}"),
                    Some(format!(
                        "{} and artists like them, refreshed daily",
                        cluster.artists.iter().take(2).cloned().collect::<Vec<_>>().join(" & ")
                    )),
                    format!("daily:{index}"),
                    cluster.label.clone(),
                    track_ids,
                )
            }
            SmartMixType::Daily => {
                let track_ids = self.generate_daily_mix(user_id).await?;
                (
                    "Daily Mix".to_string(),
                    Some("A personalized daily blend of your favorites, forgotten gems, and fresh picks".to_string()),
                    "daily".to_string(),
                    "Generated from your recent taste profile, featuring top affinities, forgotten favorites, and local exploration".to_string(),
                    track_ids,
                )
            }
            SmartMixType::OnRepeat => {
                let track_ids = self.generate_on_repeat_mix(user_id).await?;
                (
                    "On Repeat".to_string(),
                    Some("Your most-played and highest-completion tracks from the last 14 days".to_string()),
                    "on_repeat".to_string(),
                    "Built from tracks with frequent qualified plays and high completion rates over the past 2 weeks".to_string(),
                    track_ids,
                )
            }
            SmartMixType::ForgottenFavorites => {
                let track_ids = self.generate_forgotten_favorites_mix(user_id).await?;
                (
                    "Forgotten Favorites".to_string(),
                    Some("Beloved tracks and past favorites you haven't played recently".to_string()),
                    "forgotten_favorites".to_string(),
                    "Discovered from your history and likes that have not been played in over 30 days".to_string(),
                    track_ids,
                )
            }
            SmartMixType::Genre(genre_name) => {
                let track_ids = self.generate_genre_mix(user_id, genre_name).await?;
                (
                    format!("{} Mix", genre_name),
                    Some(format!(
                        "Curated tracks and highlights from the {} genre",
                        genre_name
                    )),
                    format!("genre:{}", genre_name.to_lowercase()),
                    format!(
                        "Personalized selection of top-ranked and discovery tracks in {}",
                        genre_name
                    ),
                    track_ids,
                )
            }
            SmartMixType::Artist(artist_name) => {
                let track_ids = self.generate_artist_mix(user_id, artist_name).await?;
                (
                    format!("{} Radio", artist_name),
                    Some(format!(
                        "Tracks by {} and artists sharing similar musical styles",
                        artist_name
                    )),
                    format!("artist:{}", artist_name.to_lowercase()),
                    format!(
                        "Curated playlist centered around {} and related artist affinities",
                        artist_name
                    ),
                    track_ids,
                )
            }
            SmartMixType::LateNight => {
                let track_ids = self.generate_late_night_mix(user_id).await?;
                (
                    "Late Night Chill".to_string(),
                    Some(
                        "Relaxing, deep-cut tracks tailored for evening and late-night listening"
                            .to_string(),
                    ),
                    "late_night".to_string(),
                    "Selection of mellow, unhurried tracks and deep cuts".to_string(),
                    track_ids,
                )
            }
            SmartMixType::Discovery => {
                let track_ids = self.generate_discovery_mix(user_id).await?;
                (
                    "Local Discoveries".to_string(),
                    Some(
                        "Unheard gems hidden in your local music library matching your taste"
                            .to_string(),
                    ),
                    "discovery".to_string(),
                    "Local tracks with 0 plays that align with your favorite artists and genres"
                        .to_string(),
                    track_ids,
                )
            }
        };

        let feedback = super::super::signals::SongSignals::load(&self.pool, user_id).await?;
        let explicit_favorites: HashSet<String> = if matches!(mix_type, SmartMixType::ForgottenFavorites) {
            sqlx::query_scalar("SELECT id FROM tracks WHERE LOWER(file_path) LIKE '%/fav/%'")
                .fetch_all(&self.pool).await?.into_iter().collect()
        } else { HashSet::new() };
        let mut seen = std::collections::HashSet::new();
        let track_ids = track_ids
            .into_iter()
            .filter(|id| !feedback.blocked_id(id))
            .filter(|id| {
                if !matches!(mix_type, SmartMixType::OnRepeat | SmartMixType::ForgottenFavorites) { return true; }
                let Some(song) = feedback.identities.get(id) else { return false; };
                let heard = feedback.signal(id, &song.title, &song.artist);
                heard.manual_like == 1 || explicit_favorites.contains(id)
                    || heard.play_count >= 2
                    || (matches!(mix_type, SmartMixType::OnRepeat) && heard.completion_count >= 1)
            })
            .filter(|id| {
                let key = feedback
                    .identities
                    .get(id)
                    .and_then(|song| super::super::signals::song_key(&song.title, &song.artist))
                    .unwrap_or_else(|| id.clone());
                seen.insert(key)
            })
            .collect::<Vec<_>>();
        // Mixes are queues too: space artists like radio and cap each artist at two
        // songs, however much it is played. Artist Radio is the one intended exception.
        let track_ids = if matches!(mix_type, SmartMixType::Artist(_)) {
            track_ids
        } else {
            let count = track_ids.len();
            let items: Vec<super::super::diversity::DiversityItem> = track_ids
                .iter()
                .enumerate()
                .map(|(rank, id)| super::super::diversity::DiversityItem {
                    // Rank order, nudged down by fading "less like this" feedback.
                    score: (count - rank) as f64 / count as f64
                        - feedback
                            .identities
                            .get(id)
                            .map_or(0.0, |song| feedback.less_penalty(&song.artist, None, now)),
                    artist: feedback
                        .identities
                        .get(id)
                        .map(|song| song.artist.clone())
                        .unwrap_or_else(|| id.clone()),
                    album: None,
                    is_online: false,
                    max_per_window: None,
                    features: None,
                })
                .collect();
            super::super::diversity::diversify(
                &items,
                count,
                &super::super::diversity::DiversityRules {
                    max_total: Some(2),
                    ..super::super::diversity::DiversityRules::queue(0.0)
                },
                &[],
            )
            .into_iter()
            .map(|index| track_ids[index].clone())
            .collect()
        };

        // Reuse existing mix ID and record for this smart mix so we update its contents in-place,
        // without creating a duplicate mix collection beside it
        let existing = self
            .playlist_repo
            .find_smart_mix_for_user(user_id, &mix_type_str, &name)
            .await?;
        let mix_id = match existing {
            Some(ref e) => e.id.clone(),
            None => format!("mix_{}", uuid::Uuid::new_v4()),
        };
        let created_at = existing.as_ref().map(|e| e.created_at).unwrap_or(now);
        let final_name = existing.as_ref().map(|e| e.name.clone()).unwrap_or(name);

        // Daily mixes expire every 24 hours (86,400s); other mixes change weekly (7 days / 604,800s)
        let expires_at = match mix_type {
            SmartMixType::Daily | SmartMixType::DailyCluster(_) => Some(now + 86400),
            _ => Some(now + (7 * 86400)),
        };

        let playlist = PlaylistRecord {
            id: mix_id.clone(),
            name: final_name,
            description,
            is_smart_mix: 1,
            mix_type: Some(mix_type_str.clone()),
            generation_reason: Some(generation_reason),
            expires_at,
            created_at,
            updated_at: now,
        };

        // Persist playlist record (updates existing in-place) scoped to user
        self.playlist_repo
            .create_playlist_with_user(&playlist, user_id)
            .await?;

        // Update the contents inside the mix (replaces tracks for this mix_id)
        self.playlist_repo.set_tracks(&mix_id, &track_ids).await?;

        // Clean up any historical duplicate entries with the same mix_type or name for this user
        let _ = self
            .playlist_repo
            .delete_duplicate_smart_mixes_for_user(user_id, &mix_type_str, &playlist.name, &mix_id)
            .await;

        Ok(playlist)
    }
}
