use super::taste::{EntityAffinity, TasteProfile};
use crate::core::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};

mod languages;
pub(crate) use languages::{language_key, LanguagePreference};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecommendationSetup {
    pub user_id: String,
    pub status: String,
    pub artists: Vec<String>,
    pub genres: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<String>,
    pub revision: i64,
    pub should_prompt: bool,
}
pub async fn load(pool: &SqlitePool, user: &str) -> AppResult<RecommendationSetup> {
    use crate::database::repositories::taste_setup as storage;
    let row = storage::load(pool, user).await?;
    match row {
        Some((status, artists, genres, languages, revision)) => Ok(RecommendationSetup {
            user_id: user.into(),
            status,
            artists: serde_json::from_str(&artists)
                .map_err(|e| AppError::Database(e.to_string()))?,
            genres: serde_json::from_str(&genres).map_err(|e| AppError::Database(e.to_string()))?,
            languages: serde_json::from_str(&languages).map_err(|e| AppError::Database(e.to_string()))?,
            revision,
            should_prompt: false,
        }),
        None => Ok(RecommendationSetup {
            user_id: user.into(),
            status: "pending".into(),
            ..Default::default()
        }),
    }
}
fn clean(values: Vec<String>) -> AppResult<Vec<String>> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for value in values {
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        if value.chars().count() > 120 {
            return Err(AppError::Validation("Taste choice is too long".into()));
        }
        if seen.insert(value.to_lowercase()) {
            result.push(value.to_owned());
        }
    }
    Ok(result)
}
pub async fn save(
    pool: &SqlitePool,
    user: &str,
    artists: Vec<String>,
    genres: Vec<String>,
    skipped: bool,
) -> AppResult<()> {
    save_selection(pool, user, Selection { artists, genres, languages: None, skipped }).await
}

pub(crate) struct Selection {
    pub artists: Vec<String>,
    pub genres: Vec<String>,
    pub languages: Option<Vec<String>>,
    pub skipped: bool,
}

impl Selection {
    fn validated(mut self) -> AppResult<Self> {
        if self.skipped {
            self.artists.clear();
            self.genres.clear();
            self.languages = self.languages.map(|_| vec![]);
            return Ok(self);
        }
        self.artists = clean_artists(clean(self.artists)?);
        self.genres = clean(self.genres)?;
        self.languages = self.languages.map(clean).transpose()?;
        if self.artists.len() > 5 {
            return Err(AppError::Validation("Choose up to five artists".into()));
        }
        if self.artists.is_empty() && self.genres.is_empty() && self.languages.as_ref().is_none_or(Vec::is_empty) {
            return Err(AppError::Validation("Choose a language, artist or genre, or skip for now".into()));
        }
        Ok(self)
    }
}

fn clean_artists(artists: Vec<String>) -> Vec<String> {
    let mut artist_keys = HashSet::new();
    artists
        .into_iter()
        .filter(|a| artist_keys.insert(super::signals::artist_credits(a)))
        .collect()
}

pub(crate) async fn save_selection(pool: &SqlitePool, user: &str, selection: Selection) -> AppResult<()> {
    crate::database::repositories::taste_setup::save(pool, user, selection.validated()?).await
}

pub(crate) async fn remember_returning_listener(pool: &SqlitePool, user: &str) {
    if cfg!(target_os = "android") {
        if let Err(error) = crate::database::repositories::taste_setup::skip_missing(pool, user).await {
            tracing::warn!(%error, "Could not record returning listener's taste setup status");
        }
    }
}
pub(crate) async fn apply(
    pool: &SqlitePool,
    user: &str,
    profile: &mut TasteProfile,
    heard: impl FnOnce() -> HashSet<String>,
) -> AppResult<()> {
    let setup = load(pool, user).await?;
    if !setup.languages.is_empty() {
        profile.language_preference = LanguagePreference::new(&setup.languages, heard());
        for language in &setup.languages {
            profile.language_affinities.entry(language_key(language)).or_insert(0.35);
        }
    }
    for artist in setup.artists {
        let credits = super::signals::artist_credits(&artist);
        if profile.top_artists.iter().any(|a| {
            super::signals::artist_credits(&a.display_name)
                .iter()
                .any(|a| credits.contains(a))
        }) {
            continue;
        }
        let id: Option<String> =
            sqlx::query_scalar("SELECT id FROM artists WHERE normalized_name=?")
                .bind(artist.trim().to_lowercase())
                .fetch_optional(pool)
                .await?;
        let id = id.unwrap_or_else(|| format!("starter:{}", artist.to_lowercase()));
        profile.artist_affinities.entry(id.clone()).or_insert(0.35);
        profile.top_artists.push(EntityAffinity {
            id_or_name: id,
            display_name: artist,
            affinity: 0.35,
        });
    }
    for genre in setup.genres {
        if profile
            .genre_affinities
            .keys()
            .any(|g| g.eq_ignore_ascii_case(&genre))
        {
            continue;
        }
        profile.genre_affinities.insert(genre.clone(), 0.35);
        profile.top_genres.push(EntityAffinity {
            id_or_name: genre.clone(),
            display_name: genre,
            affinity: 0.35,
        });
    }
    profile.top_artists.sort_by(|a, b| {
        b.affinity
            .total_cmp(&a.affinity)
            .then_with(|| a.id_or_name.cmp(&b.id_or_name))
    });
    profile.top_genres.sort_by(|a, b| {
        b.affinity
            .total_cmp(&a.affinity)
            .then_with(|| a.id_or_name.cmp(&b.id_or_name))
    });
    Ok(())
}
/// Account-scoped query provenance, never saved as a song's verified genre.
pub async fn members(
    pool: &SqlitePool,
    user: &str,
    profile: &TasteProfile,
) -> AppResult<HashMap<String, (String, String)>> {
    let rows:Vec<(String,String,String)>=sqlx::query_as("SELECT track_id,source,seed_value FROM recommendation_starter_tracks WHERE user_id=? AND revision=COALESCE((SELECT revision FROM recommendation_setup WHERE user_id=?),0)").bind(user).bind(user).fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .filter(|(_, source, value)| {
            !profile.has_learned_taste
                || (source == "artist" && profile.artist_affinity(value) > 0.0)
                || (source == "genre"
                    && profile
                        .genre_affinities
                        .iter()
                        .any(|(g, a)| g.eq_ignore_ascii_case(value) && *a > 0.0))
                || (source == "language" && profile.language_preference.allows(Some(value)))
        })
        .map(|(id, source, value)| (id, (source, value)))
        .collect())
}
pub fn explanation(source: &str, value: &str) -> String {
    match source {
        "artist" => format!("From an artist you chose: {value}"),
        "genre" => format!("From your {value} starter selection"),
        "language" => format!("Matches your taste for {value} songs"),
        _ => "Popular songs to get you started".into(),
    }
}
