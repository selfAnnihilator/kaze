use super::*;

impl CoreProcessor {
    pub(super) async fn mobile_language_charts(
        &self,
        language: &str,
        refresh: bool,
    ) -> AppResult<QueryResponse> {
        if !self.provider_coordinator.enabled().await {
            return Ok(QueryResponse::Charts(vec![]));
        }
        let rows = self
            .provider_coordinator
            .jiosaavn
            .language_charts(language, refresh)
            .await?;
        Ok(QueryResponse::Charts(rows))
    }

    pub(super) async fn search_mobile_catalog(
        &self,
        query: &str,
        limit: usize,
    ) -> AppResult<QueryResponse> {
        let query = query.trim();
        if !self.provider_coordinator.enabled().await {
            return Err(AppError::Validation(
                "Enable online music in Settings to search.".into(),
            ));
        }
        let (tracks, playlists) = self
            .provider_coordinator
            .jiosaavn
            .mobile_search(query, limit)
            .await?;
        let mut songs = Vec::with_capacity(tracks.len());
        for track in tracks {
            let row = self.provider_coordinator.persist(track).await?;
            songs.push(
                serde_json::to_value(crate::discovery::coordinator::recommendation(
                    row,
                    "Search result",
                ))
                .map_err(|error| AppError::Internal(error.to_string()))?,
            );
        }
        let songs = self
            .present_recommendations(
                serde_json::Value::Array(songs),
                &self.current_user_id().await,
            )
            .await?;
        Ok(QueryResponse::MobileCatalogSearch(
            serde_json::json!({ "songs": songs.as_array(), "playlists": playlists }),
        ))
    }

    pub(super) async fn mobile_taste_artists(
        &self,
        languages: &[String],
        artist: Option<&str>,
    ) -> AppResult<QueryResponse> {
        let user = self.current_user_id().await;
        let mut suggestions = Vec::new();
        if let Some(artist) = artist {
            let seeds = vec![(artist.to_owned(), 1.0)];
            let index = crate::recommendations::similarity::SimilarityIndex::load(
                &self.db_pool,
                &user,
                &seeds,
            )
            .await?;
            suggestions.extend(
                index
                    .top_similar(8, &Default::default())
                    .into_iter()
                    .map(|hit| hit.display.clone()),
            );
        }
        if self.provider_coordinator.enabled().await {
            let provider = &self.provider_coordinator.jiosaavn;
            suggestions.extend(provider.taste_artists(languages, artist).await);
        }
        Ok(QueryResponse::TasteArtists(suggestions))
    }
}
