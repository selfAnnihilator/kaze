use super::*;

impl JioSaavnProvider {
    pub(crate) async fn taste_artists(
        &self,
        languages: &[String],
        artist: Option<&str>,
    ) -> Vec<String> {
        let languages: Vec<_> = LANGUAGES
            .iter()
            .copied()
            .filter(|language| {
                languages
                    .iter()
                    .any(|name| name.eq_ignore_ascii_case(language))
            })
            .collect();
        if let Some(artist) = artist {
            let tracks = self.search(artist, 25).await.unwrap_or_default();
            let artist_key = artist.to_lowercase();
            return artist_names(tracks.iter().filter(|track| {
                (languages.is_empty()
                    || languages
                        .iter()
                        .any(|language| is_language(track, language)))
                    && credits(&track.artist_name).any(|name| name.to_lowercase() == artist_key)
            }))
            .into_iter()
            .filter(|name| !name.eq_ignore_ascii_case(artist))
            .take(12)
            .collect();
        }
        if languages.is_empty() {
            return artist_names(
                self.search("popular hits", 35)
                    .await
                    .unwrap_or_default()
                    .iter(),
            );
        }
        self.artist_pools(&languages).await
    }

    async fn artist_pools(&self, languages: &[&str]) -> Vec<String> {
        let mut pools = Vec::new();
        for batch in languages.chunks(3) {
            let searches = batch.iter().map(|&language| async move {
                let tracks = self
                    .search(&format!("{language} hits"), 30)
                    .await
                    .unwrap_or_default();
                artist_names(tracks.iter().filter(|track| is_language(track, language)))
            });
            // Each batch has at most three requests.
            let mut searches = searches;
            let mut next = || {
                let query = searches.next();
                async move {
                    match query {
                        Some(future) => future.await,
                        None => Vec::new(),
                    }
                }
            };
            let (first, second, third) = tokio::join!(next(), next(), next());
            pools.extend([first, second, third]);
        }
        let mut seen = HashSet::new();
        (0..8)
            .flat_map(|rank| pools.iter().filter_map(move |pool| pool.get(rank)))
            .filter(|name| seen.insert(name.to_lowercase()))
            .take(30)
            .cloned()
            .collect()
    }
}

fn credits(artist: &str) -> impl Iterator<Item = &str> {
    artist
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
}

fn artist_names<'a>(tracks: impl Iterator<Item = &'a ExternalTrackMetadata>) -> Vec<String> {
    let mut seen = HashSet::new();
    tracks
        .flat_map(|track| credits(&track.artist_name))
        .filter(|name| {
            let key = name.to_lowercase();
            !matches!(key.as_str(), "various artists" | "unknown artist") && seen.insert(key)
        })
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artist_picks_split_collaborations_and_keep_unique_names() {
        let tracks = [
            serde_json::json!({"id":"1","title":"Song One","primary_artists":"Artist A, Artist B"}),
            serde_json::json!({"id":"2","title":"Song Two","primary_artists":"artist a, Various Artists, Artist C"}),
        ].iter().map(|song| parse_track(song).unwrap()).collect::<Vec<_>>();
        assert_eq!(
            artist_names(tracks.iter()),
            ["Artist A", "Artist B", "Artist C"]
        );
    }
}
