use super::taste::{EntityAffinity, TasteProfile};
use crate::core::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecommendationSetup {
    pub user_id: String,
    pub status: String,
    pub artists: Vec<String>,
    pub genres: Vec<String>,
    pub revision: i64,
    pub should_prompt: bool,
}
pub async fn load(pool: &SqlitePool, user: &str) -> AppResult<RecommendationSetup> {
    let row: Option<(String, String, String, i64)> = sqlx::query_as(
        "SELECT status,artists_json,genres_json,revision FROM recommendation_setup WHERE user_id=?",
    )
    .bind(user)
    .fetch_optional(pool)
    .await?;
    match row {
        Some((status, artists, genres, revision)) => Ok(RecommendationSetup {
            user_id: user.into(),
            status,
            artists: serde_json::from_str(&artists)
                .map_err(|e| AppError::Database(e.to_string()))?,
            genres: serde_json::from_str(&genres).map_err(|e| AppError::Database(e.to_string()))?,
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
    if values.len() > 5 {
        return Err(AppError::Validation(
            "Choose up to five artists and five genres".into(),
        ));
    }
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
    let (artists, genres) = if skipped {
        (vec![], vec![])
    } else {
        (clean(artists)?, clean(genres)?)
    };
    let mut artist_keys = HashSet::new();
    let artists: Vec<_> = artists
        .into_iter()
        .filter(|a| artist_keys.insert(super::signals::artist_credits(a)))
        .collect();
    if !skipped && artists.is_empty() && genres.is_empty() {
        return Err(AppError::Validation(
            "Choose an artist or genre, or skip for now".into(),
        ));
    }
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO recommendation_setup(user_id,status,artists_json,genres_json,revision,updated_at) VALUES(?,?,?,?,1,?) ON CONFLICT(user_id) DO UPDATE SET status=excluded.status,artists_json=excluded.artists_json,genres_json=excluded.genres_json,revision=revision+1,updated_at=excluded.updated_at")
        .bind(user).bind(if skipped{"skipped"}else{"completed"}).bind(serde_json::to_string(&artists).unwrap()).bind(serde_json::to_string(&genres).unwrap()).bind(chrono::Utc::now().timestamp()).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM recommendation_starter_tracks WHERE user_id=?")
        .bind(user)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}
pub async fn apply(pool: &SqlitePool, user: &str, profile: &mut TasteProfile) -> AppResult<()> {
    let setup = load(pool, user).await?;
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
        })
        .map(|(id, source, value)| (id, (source, value)))
        .collect())
}
pub fn explanation(source: &str, value: &str) -> String {
    match source {
        "artist" => format!("From an artist you chose: {value}"),
        "genre" => format!("From your {value} starter selection"),
        _ => "Popular songs to get you started".into(),
    }
}
