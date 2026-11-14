//! What the For You feed reads: the wishlist, the library, the user's taste and
//! the catalog of online candidates (one preferred release per song).
use super::*;
use crate::database::repositories::track_repo::TrackDetail;
use crate::recommendations::signals::{self, SongSignals};
use crate::recommendations::similarity::{self, SimilarityIndex};
use crate::recommendations::TasteProfile;

/// Wishlisted songs, by catalog id and by "artist:title".
pub(super) struct WishlistIndex {
    ids: HashSet<String>,
    keys: HashSet<String>,
}

impl WishlistIndex {
    pub(super) fn contains(&self, track: &ExternalTrackRecord) -> bool {
        (!track.provider_id.is_empty() && self.ids.contains(&track.id))
            || self.keys.contains(&format!("{}:{}", track.artist.to_lowercase(), track.title.to_lowercase()))
    }
}

pub(super) struct TasteInputs {
    pub(super) local_tracks: Vec<TrackDetail>,
    /// Lower-cased artists of the user's own files.
    pub(super) library_artists: HashSet<String>,
    /// Lower-cased listened artists → affinity.
    pub(super) listened_artists: HashMap<String, f64>,
    /// Genres by weight: listening affinity, else library size.
    pub(super) ranked_genres: Vec<(String, f64)>,
    pub(super) profile: TasteProfile,
    pub(super) similarity: SimilarityIndex,
}

impl DiscoveryCoordinator {
    pub(super) async fn wishlist_index(&self) -> AppResult<WishlistIndex> {
        let items = self.wishlist_repo.get_all(None).await?;
        Ok(WishlistIndex {
            ids: items.iter().filter_map(|w| w.external_track_id.clone()).collect(),
            keys: items.iter().map(|w| format!("{}:{}", w.artist.to_lowercase(), w.title.to_lowercase())).collect(),
        })
    }

    /// The library and listening taste that rank genres and artists.
    pub(super) async fn taste_inputs(&self, user_id: &str) -> AppResult<TasteInputs> {
        let local_tracks = self
            .track_repo
            .list_tracks_scoped(Some(user_id), 0, 50000, None, true)
            .await
            .unwrap_or_default();
        let (genre_counts, library_artists) = library_genres_and_artists(&local_tracks);
        let profile = self.taste_engine.compute_taste_profile(user_id).await?;
        let listened_artists = profile
            .top_artists
            .iter()
            .map(|a| (a.display_name.to_lowercase().trim().to_string(), a.affinity))
            .collect();
        let ranked_genres = ranked_genres(genre_counts, &profile);
        let similarity = SimilarityIndex::load(self.taste_engine.pool(), user_id, &similarity::seeds_from_profile(&profile))
            .await
            .unwrap_or_default();
        Ok(TasteInputs { local_tracks, library_artists, listened_artists, ranked_genres, profile, similarity })
    }

    /// Catalog candidates: cached rows, topped up from the provider by top genres,
    /// favourite artists and similar artists.
    pub(super) async fn gather_catalog(
        &self,
        user_id: &str,
        taste: &TasteInputs,
        limit: usize,
        force_refresh: bool,
    ) -> AppResult<Vec<ExternalTrackRecord>> {
        self.seed_artist_editions(user_id, force_refresh).await?;
        let mut tracks = self.wishlist_repo.list_catalog_tracks(1000).await?;
        if self.provider_coordinator.is_none() {
            return Ok(tracks);
        }
        if force_refresh || tracks.len() < limit.max(30) {
            let mut queries: Vec<String> = taste.ranked_genres.iter().take(3).map(|(genre, _)| genre.clone()).collect();
            if queries.is_empty() {
                queries.push("English hits".into());
            }
            let _ = self.fetch_catalog_candidates(&queries).await;
            tracks = self.wishlist_repo.list_catalog_tracks(1000).await?;
        }
        self.add_artist_candidates(&mut tracks, taste, force_refresh).await;
        Ok(tracks)
    }

    /// Songs by the top listened artists, then by a few similar, unheard artists
    /// ("fans of your artists also like").
    async fn add_artist_candidates(&self, tracks: &mut Vec<ExternalTrackRecord>, taste: &TasteInputs, force_refresh: bool) {
        let mut seen: HashSet<String> = tracks.iter().map(|t| t.id.clone()).collect();
        let listened = || taste.profile.top_artists.iter().filter(|a| a.affinity > 0.0);
        let mut artists: Vec<String> = listened().take(4).map(|a| a.display_name.clone()).collect();
        let exclude: HashSet<String> = listened().flat_map(|a| signals::artist_credits(&a.display_name)).collect();
        artists.extend(taste.similarity.top_similar(3, &exclude).into_iter().map(|hit| hit.display.clone()));
        for artist in artists {
            for track in self.artist_candidates(&artist, force_refresh).await {
                if seen.insert(track.id.clone()) {
                    tracks.push(track);
                }
            }
        }
    }
}

fn library_genres_and_artists(local_tracks: &[TrackDetail]) -> (HashMap<String, usize>, HashSet<String>) {
    let mut genre_counts: HashMap<String, usize> = HashMap::new();
    let mut artists = HashSet::new();
    for t in local_tracks.iter().filter(|t| t.format != "online" && !t.file_path.starts_with("online://")) {
        if let Some(genre) = t.genre_name.as_deref().map(|g| g.to_lowercase().trim().to_string()).filter(|g| !g.is_empty()) {
            *genre_counts.entry(genre).or_insert(0) += 1;
        }
        if let Some(artist) = t.artist_name.as_deref().map(|a| a.to_lowercase().trim().to_string()).filter(|a| !a.is_empty()) {
            artists.insert(artist);
        }
    }
    (genre_counts, artists)
}

/// Listening outranks library size, including reduced/zero affinities.
fn ranked_genres(genre_counts: HashMap<String, usize>, profile: &TasteProfile) -> Vec<(String, f64)> {
    let mut weights: HashMap<String, f64> =
        genre_counts.into_iter().map(|(genre, count)| (genre, (count as f64).min(10.0))).collect();
    for g in &profile.top_genres {
        let clean = g.display_name.to_lowercase().trim().to_string();
        if !clean.is_empty() {
            weights.insert(clean, g.affinity * 100.0);
        }
    }
    let mut ranked: Vec<(String, f64)> = weights.into_iter().filter(|(_, weight)| *weight > 0.0).collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked
}

/// Prefers a single/original album over compilation duplicates before scoring.
pub(super) fn preferred_releases(tracks: Vec<ExternalTrackRecord>, signals: &SongSignals) -> Vec<ExternalTrackRecord> {
    let rank = |t: &ExternalTrackRecord| {
        signals.release_rank(&t.id, &t.title, t.album.as_deref(), signals.identities.get(&t.id).and_then(|song| song.year))
    };
    let mut releases: HashMap<String, ExternalTrackRecord> = HashMap::new();
    for track in tracks {
        let key = signals::song_key(&track.title, &track.artist).unwrap_or_else(|| track.id.clone());
        if releases.get(&key).is_none_or(|old| rank(&track) > rank(old)) {
            releases.insert(key, track);
        }
    }
    releases.into_values().collect()
}
