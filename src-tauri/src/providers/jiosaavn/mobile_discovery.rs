use super::*;
mod taste_artists;

const LANGUAGES: &[&str] = &[
    "english",
    "hindi",
    "malayalam",
    "tamil",
    "telugu",
    "kannada",
    "punjabi",
    "bengali",
    "marathi",
    "gujarati",
    "bhojpuri",
    "odia",
    "assamese",
    "urdu",
    "haryanvi",
];
const COLLECTION_WORDS: &[&str] = &[
    "top",
    "hit",
    "hits",
    "songs",
    "music",
    "trending",
    "latest",
    "best",
    "popular",
    "chart",
    "charts",
    "playlist",
    "melodies",
    "classics",
    "evergreen",
];

impl JioSaavnProvider {
    pub(crate) async fn language_charts(
        &self,
        language: &str,
        refresh: bool,
    ) -> Result<Vec<Value>> {
        if !LANGUAGES.contains(&language) {
            return Err(JioSaavnError::Unavailable("Unknown chart language".into()));
        }
        let params = [
            ("language", language),
            ("languages", language),
            ("ctx", "web6dot0"),
        ];
        let (charts, launch) = tokio::join!(
            self.request("content.getCharts", &params, refresh),
            self.request("webapi.getLaunchData", &params, refresh),
        );
        let feeds = language_feeds(charts, launch, language)?;
        let mut rows = merge_charts(&feeds);
        for row in &mut rows {
            row["language"] = language.into();
        }
        Ok(rows)
    }

    pub(crate) async fn search_playlists(&self, query: &str) -> Result<Vec<Value>> {
        let response = self
            .request(
                "search.getPlaylistResults",
                &[("q", query), ("n", "20"), ("p", "1")],
                false,
            )
            .await?;
        Ok(relevant_playlists(query, &response["results"]))
    }

    pub(crate) async fn mobile_search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<(Vec<ExternalTrackMetadata>, Vec<Value>)> {
        let (songs, playlists) =
            tokio::join!(self.search(query, limit), self.search_playlists(query));
        let playlists = playlists.unwrap_or_default();
        let mut songs = match songs {
            Err(error) if playlists.is_empty() => return Err(error),
            songs => songs.unwrap_or_default(),
        };
        if let Some(language) = broad_search_language(query) {
            self.complete_language_search(&mut songs, &playlists, &language, limit)
                .await;
        }
        Ok((songs, playlists))
    }

    async fn complete_language_search(
        &self,
        songs: &mut Vec<ExternalTrackMetadata>,
        playlists: &[Value],
        language: &str,
        limit: usize,
    ) {
        songs.retain(|track| is_language(track, language));
        if songs.len() >= limit.min(8) {
            return;
        }
        let id = playlists.first().and_then(|row| row["id"].as_str());
        let extra = match id {
            Some(id) => self.playlist(id, limit, false).await.unwrap_or_default(),
            None if songs.is_empty() => self
                .search(&format!("{language} hits"), limit)
                .await
                .unwrap_or_default(),
            None => vec![],
        };
        let mut seen: HashSet<_> = songs
            .iter()
            .map(|song| song.provider_track_id.clone())
            .collect();
        songs.extend(extra.into_iter().filter(|song| {
            is_language(song, language) && seen.insert(song.provider_track_id.clone())
        }));
        songs.truncate(limit);
    }
}

fn is_language(track: &ExternalTrackMetadata, language: &str) -> bool {
    track
        .language
        .as_deref()
        .is_some_and(|tag| tag.trim().eq_ignore_ascii_case(language))
}

fn language_feeds(
    charts: Result<Value>,
    launch: Result<Value>,
    language: &str,
) -> Result<Vec<Value>> {
    let (charts, launch) = match (charts, launch) {
        (Err(error), Err(_)) => return Err(error),
        (charts, launch) => (charts.unwrap_or_default(), launch.unwrap_or_default()),
    };
    let mut feeds = vec![];
    for (items, editorial) in [
        (&charts, false),
        (&charts["charts"], false),
        (&launch["charts"], false),
        (&launch["top_playlists"], true),
    ] {
        let entries = items.as_array().into_iter().flatten().filter(|row| {
            let title = text(row, "title")
                .or_else(|| text(row, "listname"))
                .unwrap_or_default()
                .to_lowercase();
            let words = tokens(&title);
            let named: Vec<_> = LANGUAGES
                .iter()
                .filter(|name| words.contains(**name))
                .collect();
            if let Some(tag) = text(row, "language") {
                return tag.eq_ignore_ascii_case(language);
            }
            named.iter().any(|name| **name == language) || (!editorial && named.is_empty())
        });
        feeds.push(Value::Array(entries.cloned().collect()));
    }
    Ok(feeds)
}

pub(crate) fn broad_search_language(query: &str) -> Option<String> {
    let query = query.to_lowercase();
    let words = tokens(&query);
    let language = LANGUAGES.iter().find(|name| words.contains(**name))?;
    words
        .iter()
        .all(|word| {
            LANGUAGES.contains(word)
                || COLLECTION_WORDS.contains(word)
                || word.parse::<u16>().is_ok()
        })
        .then(|| (*language).to_owned())
        .filter(|_| words.len() > 1)
}

fn tokens(text: &str) -> HashSet<&str> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect()
}

fn relevant_playlists(query: &str, items: &Value) -> Vec<Value> {
    let query = query.to_lowercase();
    let words = tokens(&query);
    let language = broad_search_language(&query);
    let mut ranked: Vec<_> = merge_charts(std::slice::from_ref(items))
        .into_iter()
        .filter_map(|row| {
            let title = row["title"].as_str()?.to_lowercase();
            let score = playlist_score(&title, &words, language.as_deref())?;
            Some((score, row))
        })
        .collect();
    ranked.sort_by_key(|(score, _)| std::cmp::Reverse(*score));
    ranked
        .into_iter()
        .take(8)
        .map(|(_, playlist)| playlist)
        .collect()
}

fn playlist_score(title: &str, words: &HashSet<&str>, language: Option<&str>) -> Option<usize> {
    let title = tokens(title);
    let overlap = title.intersection(words).count();
    let has_theme = title.iter().any(|word| COLLECTION_WORDS.contains(word));
    let relevant = match language {
        Some(language) => title.contains(language) && has_theme,
        None => overlap == words.len() && overlap > 0,
    };
    relevant.then_some(overlap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_reordered_language_playlist_names() {
        let hits = relevant_playlists(
            "malayalam top hits",
            &serde_json::json!([
                {"id":"1","title":"Top Hindi Hits"},
                {"id":"2","title":"Malayalam Melodies"},
                {"listid":"3","listname":"Top Malayalam Hits"},
                {"id":"4","title":"Malayalam News"},
            ]),
        );
        assert_eq!(
            hits.iter()
                .map(|row| row["id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["3", "2"]
        );
        assert_eq!(
            broad_search_language("Malayalam Top Hits").as_deref(),
            Some("malayalam")
        );
        assert_eq!(broad_search_language("Malayalam"), None);
        assert_eq!(
            broad_search_language("Malayalam Top Hits by an artist"),
            None
        );
    }

    #[test]
    fn filters_other_language_charts_and_keeps_editorial_hits() {
        let feeds = language_feeds(
            Ok(serde_json::json!([
                {"id":"1","title":"Trending Malayalam"}, {"id":"2","title":"Hindi Top 50"}
            ])),
            Ok(serde_json::json!({"top_playlists":[
                {"id":"3","title":"Malayalam Top Hits"}, {"id":"4","title":"General Picks"}
            ]})),
            "malayalam",
        )
        .unwrap();
        assert_eq!(
            merge_charts(&feeds)
                .iter()
                .map(|row| row["id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["1", "3"]
        );
    }
}
