//! Taste clusters: groups of the user's top artists that belong together, by
//! artist similarity (Last.fm + own co-occurrence) or a shared primary genre.
//! Each cluster with enough artists becomes its own "Daily Mix N".
use super::signals::artist_credits;
use super::taste::TasteProfile;
use crate::core::AppResult;
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};

/// How many of the strongest artists take part in clustering.
pub const CLUSTER_NODES: usize = 40;
/// Minimum average linkage for two clusters to merge.
pub const MERGE_THRESHOLD: f64 = 0.25;
/// Linkage contributed by two artists sharing a primary genre.
pub const SHARED_GENRE_LINK: f64 = 0.5;
pub const MIN_CLUSTER_ARTISTS: usize = 3;
pub const MAX_CLUSTERS: usize = 4;

#[derive(Debug, Clone, PartialEq)]
pub struct ClusterNode {
    pub display: String,
    pub norm: String,
    pub affinity: f64,
    /// Lowercase primary genre, when known.
    pub genre: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Cluster {
    /// Display names, strongest first.
    pub artists: Vec<String>,
    /// Normalized artist keys of the members.
    pub norms: HashSet<String>,
    /// Lowercase primary genres of the members, most common first.
    pub genres: Vec<String>,
    /// "A, B, C and more".
    pub label: String,
    pub weight: f64,
}

fn pair_key(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    }
}

/// Pure clustering over `nodes` (any order) with symmetric pair similarity
/// keyed by normalized names. Deterministic for the same input.
pub fn cluster_nodes(
    nodes: &[ClusterNode],
    similarity: &HashMap<(String, String), f64>,
) -> Vec<Cluster> {
    let mut nodes: Vec<&ClusterNode> = nodes.iter().collect();
    nodes.sort_by(|a, b| {
        b.affinity
            .total_cmp(&a.affinity)
            .then_with(|| a.norm.cmp(&b.norm))
    });
    nodes.dedup_by(|a, b| a.norm == b.norm);
    nodes.truncate(CLUSTER_NODES);
    let n = nodes.len();
    let link = |i: usize, j: usize| -> f64 {
        let sim = similarity
            .get(&pair_key(&nodes[i].norm, &nodes[j].norm))
            .copied()
            .unwrap_or(0.0);
        let genre = match (&nodes[i].genre, &nodes[j].genre) {
            (Some(a), Some(b)) if a == b => SHARED_GENRE_LINK,
            _ => 0.0,
        };
        sim.max(genre)
    };
    let mut links = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            let value = link(i, j);
            links[i][j] = value;
            links[j][i] = value;
        }
    }

    // Greedy agglomeration by average linkage; ties go to the lowest indices.
    let mut groups: Vec<Vec<usize>> = (0..n).map(|i| vec![i]).collect();
    loop {
        let mut best: Option<(f64, usize, usize)> = None;
        for a in 0..groups.len() {
            for b in (a + 1)..groups.len() {
                let total: f64 = groups[a]
                    .iter()
                    .flat_map(|&i| groups[b].iter().map(move |&j| (i, j)))
                    .map(|(i, j)| links[i][j])
                    .sum();
                let average = total / (groups[a].len() * groups[b].len()) as f64;
                if average >= MERGE_THRESHOLD && best.map_or(true, |(score, _, _)| average > score)
                {
                    best = Some((average, a, b));
                }
            }
        }
        let Some((_, a, b)) = best else { break };
        let merged = groups.remove(b);
        groups[a].extend(merged);
        groups[a].sort_unstable();
    }

    let mut clusters: Vec<Cluster> = groups
        .into_iter()
        .filter(|group| group.len() >= MIN_CLUSTER_ARTISTS)
        .map(|group| {
            let members: Vec<&ClusterNode> = group.iter().map(|&i| nodes[i]).collect();
            let mut genre_counts: HashMap<&str, usize> = HashMap::new();
            for node in &members {
                if let Some(genre) = &node.genre {
                    *genre_counts.entry(genre.as_str()).or_default() += 1;
                }
            }
            let mut genres: Vec<(&str, usize)> = genre_counts.into_iter().collect();
            genres.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
            let artists: Vec<String> = members.iter().map(|m| m.display.clone()).collect();
            let label = format!("{} and more", artists.iter().take(3).cloned().collect::<Vec<_>>().join(", "));
            Cluster {
                norms: members.iter().map(|m| m.norm.clone()).collect(),
                genres: genres.into_iter().map(|(g, _)| g.to_string()).collect(),
                weight: members.iter().map(|m| m.affinity).sum(),
                artists,
                label,
            }
        })
        .collect();
    clusters.sort_by(|a, b| {
        b.weight
            .total_cmp(&a.weight)
            .then_with(|| a.artists.cmp(&b.artists))
    });
    clusters.truncate(MAX_CLUSTERS);
    clusters
}

/// Builds clustering input from the taste profile and the library, then clusters.
pub async fn taste_clusters(
    pool: &SqlitePool,
    user_id: &str,
    profile: &TasteProfile,
) -> AppResult<Vec<Cluster>> {
    let mut nodes: Vec<ClusterNode> = Vec::new();
    let mut seen = HashSet::new();
    for artist in profile.top_artists.iter().filter(|a| a.affinity > 0.0) {
        let Some(norm) = artist_credits(&artist.display_name).into_iter().next() else {
            continue;
        };
        if seen.insert(norm.clone()) {
            nodes.push(ClusterNode {
                display: artist.display_name.clone(),
                norm,
                affinity: artist.affinity,
                genre: None,
            });
        }
        if nodes.len() >= CLUSTER_NODES {
            break;
        }
    }
    if nodes.len() < MIN_CLUSTER_ARTISTS * 2 {
        return Ok(Vec::new());
    }

    // Primary genre per artist = most frequent genre over their library tracks.
    let rows: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT a.name, LOWER(g.name), COUNT(*) FROM tracks t
         JOIN artists a ON a.id = t.artist_id
         JOIN genres g ON g.id = t.genre_id
         GROUP BY a.id, LOWER(g.name)",
    )
    .fetch_all(pool)
    .await?;
    let mut genre_counts: HashMap<String, HashMap<String, i64>> = HashMap::new();
    for (artist, genre, count) in rows {
        let genre = genre.trim().to_string();
        if genre.is_empty() || genre == "unknown" {
            continue;
        }
        if let Some(norm) = artist_credits(&artist).into_iter().next() {
            *genre_counts.entry(norm).or_default().entry(genre).or_default() += count;
        }
    }
    for node in &mut nodes {
        node.genre = genre_counts.get(&node.norm).and_then(|counts| {
            counts
                .iter()
                .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
                .map(|(genre, _)| genre.clone())
        });
    }

    let norms: Vec<&String> = nodes.iter().map(|n| &n.norm).collect();
    let placeholders = vec!["?"; norms.len()].join(",");
    let sql = format!(
        "SELECT source_norm, target_norm, score, source FROM artist_similarity
         WHERE source_norm IN ({placeholders}) AND target_norm IN ({placeholders})
           AND (user_id = '' OR user_id = ?)"
    );
    let mut query = sqlx::query_as::<_, (String, String, f64, String)>(&sql);
    for norm in norms.iter().chain(norms.iter()) {
        query = query.bind(*norm);
    }
    let edges = query.bind(user_id).fetch_all(pool).await?;
    let mut similarity: HashMap<(String, String), f64> = HashMap::new();
    for (source, target, score, kind) in edges {
        let weight = if kind == "cooc" {
            super::similarity::COOC_WEIGHT
        } else {
            super::similarity::LASTFM_WEIGHT
        };
        let value = (score * weight).clamp(0.0, 1.0);
        let entry = similarity.entry(pair_key(&source, &target)).or_default();
        *entry = entry.max(value);
    }
    Ok(cluster_nodes(&nodes, &similarity))
}
