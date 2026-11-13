//! Artist- and genre-level negative feedback: "not this artist" blocks every
//! credit everywhere; "less like this" is a soft penalty that fades over 90 days.
use super::signals::artist_credits;
use super::taste::TasteProfile;
use crate::core::AppResult;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::collections::{HashMap, HashSet};

pub const LESS_ARTIST_PENALTY: f64 = 0.20;
pub const LESS_GENRE_PENALTY: f64 = 0.10;
pub const LESS_DECAY_DAYS: f64 = 90.0;
pub const LESS_AFFINITY_FACTOR: f64 = 0.5;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct EntityFeedbackRow {
    pub kind: String,
    pub key: String,
    pub display: String,
    pub level: String,
    pub created_at: i64,
}

pub fn genre_norm(genre: &str) -> String {
    genre.trim().to_lowercase()
}

pub async fn load_rows(pool: &SqlitePool, user: &str) -> AppResult<Vec<EntityFeedbackRow>> {
    Ok(sqlx::query_as::<_, EntityFeedbackRow>(
        "SELECT kind, key_norm AS key, display, level, created_at
         FROM recommendation_entity_feedback WHERE user_id = ?
         ORDER BY created_at DESC, kind ASC, key_norm ASC",
    )
    .bind(user)
    .fetch_all(pool)
    .await?)
}

pub async fn upsert(
    pool: &SqlitePool,
    user: &str,
    kind: &str,
    key: &str,
    display: &str,
    level: &str,
    now: i64,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO recommendation_entity_feedback(user_id, kind, key_norm, display, level, created_at)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(user_id, kind, key_norm) DO UPDATE SET
           display = excluded.display,
           -- A block is never downgraded to 'less' by a later soft signal.
           level = CASE WHEN recommendation_entity_feedback.level = 'block' THEN 'block' ELSE excluded.level END,
           created_at = excluded.created_at",
    )
    .bind(user)
    .bind(kind)
    .bind(key)
    .bind(display)
    .bind(level)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn clear(pool: &SqlitePool, user: &str, kind: &str, key: &str) -> AppResult<bool> {
    let done = sqlx::query(
        "DELETE FROM recommendation_entity_feedback WHERE user_id = ? AND kind = ? AND key_norm = ?",
    )
    .bind(user)
    .bind(kind)
    .bind(key)
    .execute(pool)
    .await?;
    Ok(done.rows_affected() > 0)
}

#[derive(Debug, Default, Clone)]
pub struct EntityFeedbackSet {
    pub blocked_artists: HashSet<String>,
    /// normalized artist credit -> created_at
    pub less_artists: HashMap<String, i64>,
    /// lowercase genre -> created_at
    pub less_genres: HashMap<String, i64>,
}

impl EntityFeedbackSet {
    pub async fn load(pool: &SqlitePool, user: &str) -> AppResult<Self> {
        Ok(Self::from_rows(&load_rows(pool, user).await?))
    }

    pub fn from_rows(rows: &[EntityFeedbackRow]) -> Self {
        let mut set = Self::default();
        for row in rows {
            match (row.kind.as_str(), row.level.as_str()) {
                ("artist", "block") => {
                    set.blocked_artists.insert(row.key.clone());
                }
                ("artist", "less") => {
                    set.less_artists.insert(row.key.clone(), row.created_at);
                }
                ("genre", "less") => {
                    set.less_genres.insert(row.key.clone(), row.created_at);
                }
                _ => (),
            }
        }
        set
    }

    pub fn is_empty(&self) -> bool {
        self.blocked_artists.is_empty() && self.less_artists.is_empty() && self.less_genres.is_empty()
    }

    pub fn is_blocked_artist(&self, artist: &str) -> bool {
        !self.blocked_artists.is_empty()
            && artist_credits(artist)
                .iter()
                .any(|credit| self.blocked_artists.contains(credit))
    }

    fn decay(created_at: i64, now: i64) -> f64 {
        let age_days = (now - created_at).max(0) as f64 / 86_400.0;
        (1.0 - age_days / LESS_DECAY_DAYS).max(0.0)
    }

    /// Score penalty for a candidate; 0 when nothing applies.
    pub fn less_penalty(&self, artist: &str, genre: Option<&str>, now: i64) -> f64 {
        if self.less_artists.is_empty() && self.less_genres.is_empty() {
            return 0.0;
        }
        let artist_part = artist_credits(artist)
            .iter()
            .filter_map(|credit| self.less_artists.get(credit))
            .map(|&at| LESS_ARTIST_PENALTY * Self::decay(at, now))
            .fold(0.0, f64::max);
        let genre_part = genre
            .map(genre_norm)
            .and_then(|g| self.less_genres.get(&g).copied())
            .map(|at| LESS_GENRE_PENALTY * Self::decay(at, now))
            .unwrap_or(0.0);
        artist_part + genre_part
    }

    /// Fold entity feedback into a computed taste profile: blocked artists vanish
    /// (and so stop seeding similarity), "less" keys keep half their affinity.
    pub fn apply_to_profile(&self, profile: &mut TasteProfile, now: i64) {
        if self.is_empty() {
            return;
        }
        let mut removed_ids = Vec::new();
        for artist in profile.top_artists.iter_mut() {
            let credits = artist_credits(&artist.display_name);
            if credits.iter().any(|c| self.blocked_artists.contains(c)) {
                removed_ids.push(artist.id_or_name.clone());
                continue;
            }
            let fade = credits
                .iter()
                .filter_map(|c| self.less_artists.get(c))
                .map(|&at| Self::decay(at, now))
                .fold(0.0, f64::max);
            if fade > 0.0 {
                let factor = 1.0 - (1.0 - LESS_AFFINITY_FACTOR) * fade;
                artist.affinity *= factor;
                if let Some(value) = profile.artist_affinities.get_mut(&artist.id_or_name) {
                    *value *= factor;
                }
            }
        }
        profile
            .top_artists
            .retain(|artist| !removed_ids.contains(&artist.id_or_name));
        for id in &removed_ids {
            profile.artist_affinities.remove(id);
            profile.artist_breadth.remove(id);
        }
        for (genre, value) in profile.genre_affinities.iter_mut() {
            if let Some(&at) = self.less_genres.get(&genre_norm(genre)) {
                *value *= 1.0 - (1.0 - LESS_AFFINITY_FACTOR) * Self::decay(at, now);
            }
        }
        for genre in profile.top_genres.iter_mut() {
            if let Some(&at) = self.less_genres.get(&genre_norm(&genre.id_or_name)) {
                genre.affinity *= 1.0 - (1.0 - LESS_AFFINITY_FACTOR) * Self::decay(at, now);
            }
        }
        let order = |a: &super::taste::EntityAffinity, b: &super::taste::EntityAffinity| {
            b.affinity
                .partial_cmp(&a.affinity)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id_or_name.cmp(&b.id_or_name))
        };
        profile.top_artists.sort_by(order);
        profile.top_genres.sort_by(order);
    }
}
