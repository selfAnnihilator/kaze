use super::*;
use crate::database::repositories::taste_setup::{catalog_in_language, StarterTracks};
use crate::recommendations::setup::language_key;
use std::time::Instant;

pub(super) fn matches_language(languages: &[String], language: Option<&str>) -> bool {
    languages.is_empty() || language.is_some_and(|name| languages.contains(&language_key(name)))
}

#[derive(PartialEq)]
pub(super) struct StarterRequest {
    pub revision: i64,
    pub languages: Vec<String>,
}

pub(super) struct StarterAttempt {
    request: StarterRequest,
    at: Instant,
}

impl DiscoveryCoordinator {
    pub(super) async fn recent_starter_attempt(
        &self,
        user: &str,
        request: &StarterRequest,
        force: bool,
        empty: bool,
    ) -> bool {
        !force
            && self
                .starter_attempts
                .lock()
                .await
                .get(user)
                .is_some_and(|attempt| {
                    attempt.request == *request
                        && attempt.at.elapsed().as_secs() < if empty { 30 } else { 900 }
                })
    }

    pub(super) async fn remember_starter_attempt(&self, user: &str, request: StarterRequest) {
        let mut attempts = self.starter_attempts.lock().await;
        if attempts.len() >= 16 && !attempts.contains_key(user) {
            let oldest = attempts
                .iter()
                .min_by_key(|(_, attempt)| attempt.at)
                .map(|(key, _)| key.clone());
            if let Some(oldest) = oldest {
                attempts.remove(&oldest);
            }
        }
        attempts.insert(
            user.into(),
            StarterAttempt {
                request,
                at: Instant::now(),
            },
        );
    }

    pub(super) async fn language_starters(&self, languages: &[String]) -> AppResult<StarterTracks> {
        let mut chosen = StarterTracks::new();
        let mut needs_tracks = Vec::new();
        for language in languages.iter().take(16) {
            let cached = catalog_in_language(self.taste_engine.pool(), language).await?;
            if cached.len() < 8 {
                needs_tracks.push(language.clone());
            }
            chosen.extend(
                cached
                    .into_iter()
                    .map(|track| (track.id, ("language".into(), language.clone()))),
            );
        }
        if let Some(providers) = &self.provider_coordinator {
            if providers.enabled().await {
                fetch_language_starters(providers, &needs_tracks, &mut chosen).await;
            }
        }
        Ok(chosen)
    }
}

async fn fetch_language_starters(
    providers: &Arc<ProviderCoordinator>,
    languages: &[String],
    chosen: &mut StarterTracks,
) {
    for batch in languages.chunks(3) {
        let mut jobs = tokio::task::JoinSet::new();
        for language in batch {
            let providers = providers.clone();
            let language = language.clone();
            jobs.spawn(async move { fetch_language(&providers, language).await });
        }
        while let Some(Ok((language, ids))) = jobs.join_next().await {
            chosen.extend(
                ids.into_iter()
                    .map(|id| (id, ("language".into(), language.clone()))),
            );
        }
    }
}

async fn fetch_language(
    providers: &ProviderCoordinator,
    language: String,
) -> (String, Vec<String>) {
    let query = format!("{language} hits");
    let tracks = providers
        .jiosaavn
        .search(&query, 35)
        .await
        .unwrap_or_default();
    let matching = tracks.into_iter().filter(|track| {
        track
            .language
            .as_deref()
            .is_some_and(|name| language_key(name) == language)
    });
    let mut ids = Vec::new();
    for track in matching.take(15) {
        if let Ok(row) = providers.persist(track).await {
            ids.push(row.id);
        }
    }
    (language, ids)
}
