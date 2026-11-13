//! Artist co-occurrence from the user's own behaviour: artists placed near each
//! other in hand-made playlists, or listened to back to back in one session.
//! Score = count / sqrt(n_a * n_b) (cosine over co-occurrence counts).

use super::engagement::Engagement;
use super::signals::artist_credits;
use crate::core::AppResult;
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};

/// Neighbours within this many positions (playlist) or listens (session) pair up.
pub const WINDOW: usize = 5;
/// Consecutive qualified listens further apart than this start a new session.
pub const SESSION_GAP_SECS: i64 = 30 * 60;
pub const MIN_PAIR_COUNT: u32 = 2;
pub const MAX_NEIGHBOURS: usize = 50;

#[derive(Debug, Clone, PartialEq)]
pub struct CoocEdge {
    pub source: String,
    pub target: String,
    pub target_display: String,
    pub score: f64,
}

/// One ordered sequence of artist credits (one playlist or one listening session).
pub type ArtistSequence = Vec<Vec<(String, String)>>;

/// Pure scoring over sequences of (normalized, display) credits; deterministic.
pub fn score_sequences(sequences: &[ArtistSequence]) -> Vec<CoocEdge> {
    let mut pairs: HashMap<(String, String), u32> = HashMap::new();
    let mut appearances: HashMap<String, u32> = HashMap::new();
    let mut display: HashMap<String, String> = HashMap::new();
    for sequence in sequences {
        // Each artist counts once per sequence, so one long playlist cannot dominate.
        let mut seen = HashSet::new();
        let mut seq_pairs = HashSet::new();
        for (i, credits) in sequence.iter().enumerate() {
            for (norm, shown) in credits {
                display.entry(norm.clone()).or_insert_with(|| shown.clone());
                if seen.insert(norm.clone()) {
                    *appearances.entry(norm.clone()).or_default() += 1;
                }
            }
            let end = (i + WINDOW + 1).min(sequence.len());
            for other in &sequence[i + 1..end] {
                for (a, _) in credits {
                    for (b, _) in other {
                        if a != b {
                            let key = if a < b { (a.clone(), b.clone()) } else { (b.clone(), a.clone()) };
                            seq_pairs.insert(key);
                        }
                    }
                }
            }
        }
        for key in seq_pairs {
            *pairs.entry(key).or_default() += 1;
        }
    }
    let mut out: HashMap<String, Vec<CoocEdge>> = HashMap::new();
    for ((a, b), count) in pairs {
        if count < MIN_PAIR_COUNT {
            continue;
        }
        let na = f64::from(appearances[&a]);
        let nb = f64::from(appearances[&b]);
        let score = (f64::from(count) / (na * nb).sqrt()).clamp(0.0, 1.0);
        for (s, t) in [(&a, &b), (&b, &a)] {
            out.entry(s.clone()).or_default().push(CoocEdge {
                source: s.clone(),
                target: t.clone(),
                target_display: display[t].clone(),
                score,
            });
        }
    }
    let mut edges = Vec::new();
    let mut sources: Vec<_> = out.into_iter().collect();
    sources.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, mut list) in sources {
        list.sort_by(|a, b| b.score.total_cmp(&a.score).then_with(|| a.target.cmp(&b.target)));
        list.truncate(MAX_NEIGHBOURS);
        edges.extend(list);
    }
    edges
}

/// Splits a display credit string into (normalized, display) pairs.
pub fn display_credits(artist: &str) -> Vec<(String, String)> {
    let decoded = html_escape::decode_html_entities(artist).to_string();
    let mut parts: Vec<String> = vec![decoded];
    for sep in [" feat. ", " Feat. ", " ft. ", " Ft. ", " & ", " and ", ","] {
        parts = parts
            .iter()
            .flat_map(|p| p.split(sep).map(str::to_string).collect::<Vec<_>>())
            .collect();
    }
    let mut seen = HashSet::new();
    parts
        .into_iter()
        .filter_map(|shown| {
            let shown = shown.trim().to_string();
            let norm = artist_credits(&shown).into_iter().next()?;
            seen.insert(norm.clone()).then_some((norm, shown))
        })
        .collect()
}

#[derive(sqlx::FromRow)]
struct PlaylistRow {
    playlist_id: String,
    artist: Option<String>,
}

#[derive(sqlx::FromRow)]
struct ListenRow {
    artist: Option<String>,
    started_at: i64,
    duration_secs: f64,
    seconds_listened: f64,
    skipped: i64,
}

async fn load_sequences(pool: &SqlitePool, user_id: &str) -> AppResult<Vec<ArtistSequence>> {
    let mut sequences = Vec::new();
    let rows = sqlx::query_as::<_, PlaylistRow>(
        "SELECT pt.playlist_id, COALESCE(a.name, e.artist) AS artist
         FROM playlist_tracks pt
         JOIN playlists p ON p.id = pt.playlist_id
         LEFT JOIN tracks t ON t.id = pt.track_id
         LEFT JOIN artists a ON a.id = t.artist_id
         LEFT JOIN external_tracks e ON e.id = pt.track_id
         WHERE p.user_id = ? AND COALESCE(p.is_smart_mix, 0) = 0
         ORDER BY pt.playlist_id, pt.position",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    let mut current: Option<String> = None;
    let mut sequence: ArtistSequence = Vec::new();
    for row in rows {
        if current.as_deref() != Some(row.playlist_id.as_str()) {
            if sequence.len() > 1 {
                sequences.push(std::mem::take(&mut sequence));
            }
            sequence.clear();
            current = Some(row.playlist_id);
        }
        let credits = row.artist.as_deref().map(display_credits).unwrap_or_default();
        if !credits.is_empty() {
            sequence.push(credits);
        }
    }
    if sequence.len() > 1 {
        sequences.push(sequence);
    }

    let listens = sqlx::query_as::<_, ListenRow>(
        "SELECT COALESCE(a.name, e.artist) AS artist, h.started_at,
                COALESCE(t.duration_secs, e.duration_secs, 0) AS duration_secs,
                h.seconds_listened, h.skipped
         FROM playback_history h
         LEFT JOIN tracks t ON t.id = h.track_id
         LEFT JOIN artists a ON a.id = t.artist_id
         LEFT JOIN external_tracks e ON e.id = h.track_id
         WHERE h.user_id = ?
         ORDER BY h.started_at ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    let mut session: ArtistSequence = Vec::new();
    let mut last_at: Option<i64> = None;
    for row in listens {
        let engagement =
            Engagement::from_listening(row.seconds_listened, row.duration_secs, row.skipped != 0);
        if !engagement.qualified {
            continue;
        }
        if last_at.is_some_and(|last| row.started_at - last > SESSION_GAP_SECS) && session.len() > 1 {
            sequences.push(std::mem::take(&mut session));
        } else if last_at.is_some_and(|last| row.started_at - last > SESSION_GAP_SECS) {
            session.clear();
        }
        last_at = Some(row.started_at);
        let credits = row.artist.as_deref().map(display_credits).unwrap_or_default();
        if !credits.is_empty() {
            session.push(credits);
        }
    }
    if session.len() > 1 {
        sequences.push(session);
    }
    Ok(sequences)
}

/// Rewrites this user's `source='cooc'` rows. Returns the number of edges stored.
pub async fn rebuild(pool: &SqlitePool, user_id: &str) -> AppResult<usize> {
    let edges = score_sequences(&load_sequences(pool, user_id).await?);
    let now = chrono::Utc::now().timestamp();
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM artist_similarity WHERE user_id = ? AND source = 'cooc'")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    for edge in &edges {
        sqlx::query(
            "INSERT OR REPLACE INTO artist_similarity
             (user_id, source_norm, target_norm, target_display, score, source, updated_at)
             VALUES (?, ?, ?, ?, ?, 'cooc', ?)",
        )
        .bind(user_id)
        .bind(&edge.source)
        .bind(&edge.target)
        .bind(&edge.target_display)
        .bind(edge.score)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(edges.len())
}
