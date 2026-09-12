use super::matcher::{FuzzyTrackMatcher, MatchResult, MatchStatus};
use crate::core::error::AppResult;
use crate::database::models::ExternalTrackRecord;
use crate::database::repositories::{TrackRepository, WishlistRepository};
use crate::providers::ProviderCoordinator;
use crate::recommendations::TasteProfileEngine;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryRecommendation {
    pub external_track_id: String,
    pub provider: String,
    pub provider_id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_secs: Option<f64>,
    pub cover_art_url: Option<String>,
    pub preview_url: Option<String>,
    pub genre: Option<String>,
    pub match_status: MatchStatus,
    pub matched_local_track_id: Option<String>,
    pub recommendation_reason: String,
    pub in_wishlist: bool,
}

pub struct DiscoveryCoordinator {
    wishlist_repo: Arc<dyn WishlistRepository>,
    track_repo: Arc<dyn TrackRepository>,
    taste_engine: Arc<TasteProfileEngine>,
    #[allow(dead_code)]
    provider_coordinator: Option<Arc<ProviderCoordinator>>,
}

/// Maps common genre names/keywords to iTunes Store genre IDs
pub fn map_genre_to_itunes_genre_id(genre: &str) -> Option<u32> {
    let g = genre.to_lowercase();
    if g.contains("hip hop") || g.contains("hip-hop") || g.contains("rap") {
        Some(18) // Hip-Hop/Rap
    } else if g.contains("pop") && !g.contains("k-pop") && !g.contains("j-pop") {
        Some(14) // Pop
    } else if g.contains("electro") || g.contains("dance") || g.contains("electronic") || g.contains("house") || g.contains("edm") {
        Some(7)  // Electronic / Dance
    } else if g.contains("alt") {
        Some(20) // Alternative
    } else if g.contains("rock") || g.contains("metal") {
        Some(21) // Rock
    } else if g.contains("r&b") || g.contains("soul") {
        Some(15) // R&B / Soul
    } else if g.contains("soundtrack") || g.contains("film") || g.contains("game") {
        Some(24) // Soundtrack
    } else if g.contains("country") {
        Some(6)  // Country
    } else if g.contains("jazz") {
        Some(11) // Jazz
    } else if g.contains("classical") {
        Some(5)  // Classical
    } else if g.contains("reggae") {
        Some(16) // Reggae
    } else {
        None
    }
}

/// Checks if a candidate genre fuzzy matches a user's library genre
pub fn genre_matches(candidate_genre: &str, user_genre: &str) -> bool {
    let cg = candidate_genre.to_lowercase();
    let ug = user_genre.to_lowercase();
    if cg.is_empty() || ug.is_empty() {
        return false;
    }
    if cg == ug || cg.contains(&ug) || ug.contains(&cg) {
        return true;
    }
    let is_rap_hiphop = (cg.contains("hip-hop") || cg.contains("hip hop") || cg.contains("rap"))
        && (ug.contains("hip-hop") || ug.contains("hip hop") || ug.contains("rap"));
    let is_electronic = (cg.contains("dance") || cg.contains("electronic") || cg.contains("electro"))
        && (ug.contains("dance") || ug.contains("electronic") || ug.contains("electro"));
    let is_rock_alt = (cg.contains("rock") || cg.contains("alternative"))
        && (ug.contains("rock") || ug.contains("alternative"));
    let is_rnb = (cg.contains("r&b") || cg.contains("soul"))
        && (ug.contains("r&b") || ug.contains("soul"));

    let is_asian = (cg.contains("asian") || cg.contains("j-pop") || cg.contains("anime") || cg.contains("k-pop") || cg.contains("japanese"))
        && (ug.contains("asian") || ug.contains("j-pop") || ug.contains("anime") || ug.contains("k-pop") || ug.contains("japanese"));
    let is_latin_brazil = (cg.contains("brazil") || cg.contains("latin") || cg.contains("samba") || cg.contains("bossa"))
        && (ug.contains("brazil") || ug.contains("latin") || ug.contains("samba") || ug.contains("bossa"));
    let is_indian = (cg.contains("bollywood") || cg.contains("indian") || cg.contains("desi") || cg.contains("hindi") || cg.contains("punjabi"))
        && (ug.contains("bollywood") || ug.contains("indian") || ug.contains("desi") || ug.contains("hindi") || ug.contains("punjabi"));
    let is_malayalam = (cg.contains("malayalam") || cg.contains("regional indian") || cg.contains("indian"))
        && (ug.contains("malayalam") || ug.contains("asian") || ug.contains("indian") || ug.contains("soundtrack"));

    is_rap_hiphop || is_electronic || is_rock_alt || is_rnb || is_asian || is_latin_brazil || is_indian || is_malayalam
}

impl DiscoveryCoordinator {
    pub fn new(
        wishlist_repo: Arc<dyn WishlistRepository>,
        track_repo: Arc<dyn TrackRepository>,
        taste_engine: Arc<TasteProfileEngine>,
        provider_coordinator: Option<Arc<ProviderCoordinator>>,
    ) -> Self {
        Self {
            wishlist_repo,
            track_repo,
            taste_engine,
            provider_coordinator,
        }
    }

    pub fn wishlist_repo(&self) -> Arc<dyn WishlistRepository> {
        self.wishlist_repo.clone()
    }

    /// Evaluates fuzzy similarity for an external track candidate against all local library tracks.
    pub async fn match_against_library(
        &self,
        ext_title: &str,
        ext_artist: &str,
        ext_duration: Option<f64>,
    ) -> AppResult<MatchResult> {
        let local_tracks = self.track_repo.list_tracks(0, 50000, None, true).await?;

        let candidates = local_tracks.iter().map(|t| {
            (
                t.id.as_str(),
                t.title.as_str(),
                t.artist_name.as_deref().unwrap_or(""),
                t.duration_secs,
            )
        });

        Ok(FuzzyTrackMatcher::find_best_match(
            ext_title,
            ext_artist,
            ext_duration,
            candidates,
        ))
    }

    /// Ingests or updates an external track candidate, running fuzzy matching
    /// against the local library to determine match status and saving to database.
    pub async fn ingest_external_track(
        &self,
        mut track: ExternalTrackRecord,
    ) -> AppResult<MatchResult> {
        let match_result = self
            .match_against_library(&track.title, &track.artist, track.duration_secs)
            .await?;

        track.match_status = match_result.status.as_str().to_string();
        track.matched_local_track_id = match_result.matched_track_id.clone();

        self.wishlist_repo.upsert_external_track(&track).await?;
        Ok(match_result)
    }

    /// Fetches trending hits and popular new music across overall charts and targeted user genres.
    /// Also extracts 30-second AAC audio preview links for instant in-app listening before download.
    pub async fn fetch_trending_and_genre_candidates(
        &self,
        genre_ids: &[u32],
        storefronts: &[String],
        regional_queries: &[String],
    ) -> AppResult<usize> {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(4))
            .build()
        {
            Ok(c) => c,
            Err(_) => return Ok(0),
        };

        let mut urls = Vec::new();

        // Query trending charts for each detected storefront (e.g. "us", "jp", "kr", "in", "br")
        for sf in storefronts {
            urls.push(format!(
                "https://itunes.apple.com/{}/rss/topsongs/limit=25/json",
                sf
            ));
        }

        // Query top 25 for each user taste genre
        for &gid in genre_ids.iter().take(3) {
            urls.push(format!(
                "https://itunes.apple.com/us/rss/topsongs/limit=25/genre={}/json",
                gid
            ));
        }

        let mut ingested_count = 0;

        for url in urls {
            let resp = match client.get(&url).send().await {
                Ok(r) if r.status().is_success() => r,
                _ => continue,
            };

            let json_val: serde_json::Value = match resp.json().await {
                Ok(v) => v,
                Err(_) => continue,
            };

            let entries = json_val
                .get("feed")
                .and_then(|f| f.get("entry"))
                .and_then(|e| e.as_array());

            if let Some(entries) = entries {
                for entry in entries {
                    let title = entry
                        .get("im:name")
                        .and_then(|n| n.get("label"))
                        .and_then(|l| l.as_str())
                        .unwrap_or("")
                        .trim();

                    let artist = entry
                        .get("im:artist")
                        .and_then(|a| a.get("label"))
                        .and_then(|l| l.as_str())
                        .unwrap_or("")
                        .trim();

                    if title.is_empty() || artist.is_empty() {
                        continue;
                    }

                    let album = entry
                        .get("im:collection")
                        .and_then(|c| c.get("im:name"))
                        .and_then(|n| n.get("label"))
                        .and_then(|l| l.as_str())
                        .map(|s| s.to_string());

                    let cover_art_url = entry
                        .get("im:image")
                        .and_then(|imgs| imgs.as_array())
                        .and_then(|arr| arr.last())
                        .and_then(|img| img.get("label"))
                        .and_then(|l| l.as_str())
                        .map(|s| s.to_string());

                    let itunes_id = entry
                        .get("id")
                        .and_then(|i| i.get("attributes"))
                        .and_then(|a| a.get("im:id"))
                        .and_then(|id| id.as_str())
                        .unwrap_or("");

                    let genre = entry
                        .get("category")
                        .and_then(|c| c.get("attributes"))
                        .and_then(|a| a.get("label"))
                        .and_then(|l| l.as_str())
                        .map(|s| s.to_string());

                    // Extract 30s AAC audio preview stream link
                    let preview_url = entry
                        .get("link")
                        .and_then(|l| l.as_array())
                        .and_then(|links| {
                            links.iter().find_map(|link| {
                                let attrs = link.get("attributes")?;
                                let rel = attrs.get("rel")?.as_str()?;
                                if rel == "enclosure" {
                                    attrs.get("href")?.as_str().map(|s| s.to_string())
                                } else {
                                    None
                                }
                            })
                        });

                    let record = ExternalTrackRecord {
                        id: format!(
                            "itunes:{}",
                            if itunes_id.is_empty() {
                                format!("{}:{}", artist, title)
                            } else {
                                itunes_id.to_string()
                            }
                        ),
                        provider: "itunes".to_string(),
                        provider_id: if itunes_id.is_empty() {
                            title.to_string()
                        } else {
                            itunes_id.to_string()
                        },
                        title: title.to_string(),
                        artist: artist.to_string(),
                        album: album.or_else(|| genre.clone()),
                        duration_secs: Some(30.0),
                        cover_art_url,
                        preview_url,
                        genre,
                        match_status: MatchStatus::NotFound.as_str().to_string(),
                        matched_local_track_id: None,
                        created_at: chrono::Utc::now().timestamp(),
                    };

                    let _ = self.ingest_external_track(record).await;
                    ingested_count += 1;
                }
            }
        }

        // Query targeted regional terms (e.g. "malayalam", "malayalam hits")
        for query in regional_queries {
            let encoded = query.replace(' ', "+");
            let url = format!(
                "https://itunes.apple.com/search?term={}&country=in&media=music&entity=song&limit=25",
                encoded
            );
            let resp = match client.get(&url).send().await {
                Ok(r) if r.status().is_success() => r,
                _ => continue,
            };

            let json_val: serde_json::Value = match resp.json().await {
                Ok(v) => v,
                Err(_) => continue,
            };

            if let Some(results) = json_val.get("results").and_then(|r| r.as_array()) {
                for item in results {
                    let title = item
                        .get("trackName")
                        .and_then(|t| t.as_str())
                        .unwrap_or("")
                        .trim();
                    let artist = item
                        .get("artistName")
                        .and_then(|a| a.as_str())
                        .unwrap_or("")
                        .trim();

                    if title.is_empty() || artist.is_empty() {
                        continue;
                    }

                    let album = item
                        .get("collectionName")
                        .and_then(|c| c.as_str())
                        .map(|s| s.to_string());
                    let cover_art_url = item
                        .get("artworkUrl100")
                        .and_then(|a| a.as_str())
                        .map(|s| s.to_string());
                    let preview_url = item
                        .get("previewUrl")
                        .and_then(|p| p.as_str())
                        .map(|s| s.to_string());
                    let track_id = item
                        .get("trackId")
                        .map(|id| id.to_string())
                        .unwrap_or_default();
                    let genre = item
                        .get("primaryGenreName")
                        .and_then(|g| g.as_str())
                        .map(|s| s.to_string());

                    let record = ExternalTrackRecord {
                        id: format!(
                            "itunes:{}",
                            if track_id.is_empty() {
                                format!("{}:{}", artist, title)
                            } else {
                                track_id.clone()
                            }
                        ),
                        provider: "itunes".to_string(),
                        provider_id: if track_id.is_empty() {
                            title.to_string()
                        } else {
                            track_id
                        },
                        title: title.to_string(),
                        artist: artist.to_string(),
                        album,
                        duration_secs: Some(30.0),
                        cover_art_url,
                        preview_url,
                        genre: genre.or_else(|| Some("Malayalam".to_string())),
                        match_status: MatchStatus::NotFound.as_str().to_string(),
                        matched_local_track_id: None,
                        created_at: chrono::Utc::now().timestamp(),
                    };

                    let _ = self.ingest_external_track(record).await;
                    ingested_count += 1;
                }
            }
        }

        Ok(ingested_count)
    }

    /// Generates external discovery recommendations tailored so the MAJORITY are trending
    /// hits that match the user's taste (top genres, listened artists, library artists).
    pub async fn get_discovery_recommendations(
        &self,
        limit: usize,
        force_refresh: bool,
    ) -> AppResult<Vec<DiscoveryRecommendation>> {
        // 1. Fetch current wishlist to cross-reference
        let wishlist_items = self.wishlist_repo.get_all(None).await?;
        let wishlist_ext_ids: HashSet<String> = wishlist_items
            .iter()
            .filter_map(|w| w.external_track_id.clone())
            .collect();
        let wishlist_keys: HashSet<String> = wishlist_items
            .iter()
            .map(|w| format!("{}:{}", w.artist.to_lowercase(), w.title.to_lowercase()))
            .collect();

        // 2. Analyze user library & listening taste to rank genres and artists
        let local_tracks = self
            .track_repo
            .list_tracks(0, 50000, None, true)
            .await
            .unwrap_or_default();

        let mut genre_counts: HashMap<String, usize> = HashMap::new();
        let mut library_artists: HashSet<String> = HashSet::new();

        for t in &local_tracks {
            if let Some(ref g) = t.genre_name {
                let clean = g.to_lowercase().trim().to_string();
                if !clean.is_empty() {
                    *genre_counts.entry(clean).or_insert(0) += 1;
                }
            }
            if let Some(ref a) = t.artist_name {
                let clean = a.to_lowercase().trim().to_string();
                if !clean.is_empty() {
                    library_artists.insert(clean);
                }
            }
        }

        let mut listened_artists: HashSet<String> = HashSet::new();
        if let Ok(profile) = self.taste_engine.compute_taste_profile().await {
            for a in profile.top_artists {
                listened_artists.insert(a.display_name.to_lowercase().trim().to_string());
            }
            for g in profile.top_genres {
                let clean = g.display_name.to_lowercase().trim().to_string();
                if !clean.is_empty() {
                    *genre_counts.entry(clean).or_insert(0) += 10;
                }
            }
        }

        let mut ranked_user_genres: Vec<(String, usize)> = genre_counts.into_iter().collect();
        ranked_user_genres.sort_by(|a, b| b.1.cmp(&a.1));

        let mut target_itunes_genre_ids = Vec::new();
        for (g, _) in &ranked_user_genres {
            if let Some(gid) = map_genre_to_itunes_genre_id(g) {
                if !target_itunes_genre_ids.contains(&gid) {
                    target_itunes_genre_ids.push(gid);
                    if target_itunes_genre_ids.len() >= 3 {
                        break;
                    }
                }
            }
        }

        // 3. Fetch candidate external tracks from database
        let mut ext_tracks = self
            .wishlist_repo
            .list_external_tracks(None, 300)
            .await?;

        // Detect regional storefronts based on user's library genres and artists
        let mut storefronts = vec!["us".to_string()];

        let has_japanese = ranked_user_genres.iter().any(|(g, _)| {
            let lower = g.to_lowercase();
            lower.contains("asian") || lower.contains("j-pop") || lower.contains("japanese") || lower.contains("anime")
        }) || library_artists.iter().any(|a| {
            a.contains("ado")
                || a.contains("yoasobi")
                || a.contains("kajiura")
                || a.contains("nemu")
                || a.chars().any(|c| {
                    (c >= '\u{3040}' && c <= '\u{30ff}') || (c >= '\u{4e00}' && c <= '\u{9fff}')
                })
        });

        let has_korean = ranked_user_genres.iter().any(|(g, _)| {
            let lower = g.to_lowercase();
            lower.contains("k-pop") || lower.contains("korean")
        }) || library_artists
            .iter()
            .any(|a| a.chars().any(|c| c >= '\u{ac00}' && c <= '\u{d7af}'));

        let has_indian = ranked_user_genres.iter().any(|(g, _)| {
            let lower = g.to_lowercase();
            lower.contains("bollywood")
                || lower.contains("indian")
                || lower.contains("hindi")
                || lower.contains("punjabi")
                || lower.contains("desi")
        }) || library_artists
            .iter()
            .any(|a| a.chars().any(|c| c >= '\u{0900}' && c <= '\u{097f}'));

        let has_brazilian = ranked_user_genres.iter().any(|(g, _)| {
            let lower = g.to_lowercase();
            lower.contains("brazil") || lower.contains("samba") || lower.contains("bossa")
        });

        let has_latin = ranked_user_genres.iter().any(|(g, _)| {
            let lower = g.to_lowercase();
            lower.contains("latin") || lower.contains("reggaeton") || lower.contains("spanish")
        });

        if has_japanese && !storefronts.contains(&"jp".to_string()) {
            storefronts.push("jp".to_string());
        }
        if has_korean && !storefronts.contains(&"kr".to_string()) {
            storefronts.push("kr".to_string());
        }
        let has_malayalam = library_artists.iter().any(|a| {
            let al = a.to_lowercase();
            al.contains("sushin")
                || al.contains("dabzee")
                || al.contains("shaan rahman")
                || al.contains("yesudas")
                || al.contains("hanumankind")
                || al.contains("ranjin raj")
                || al.contains("anirudh")
                || al.contains("chithra")
                || al.contains("malayalam")
                || a.chars().any(|c| c >= '\u{0d00}' && c <= '\u{0d7f}')
        }) || local_tracks.iter().any(|t| {
            let tl = t.title.to_lowercase();
            tl.contains("illuminati")
                || tl.contains("malare")
                || tl.contains("poomuthole")
                || tl.contains("aalolam")
                || t.title.chars().any(|c| c >= '\u{0d00}' && c <= '\u{0d7f}')
        }) || ranked_user_genres.iter().any(|(g, _)| {
            g.to_lowercase().contains("malayalam")
        });

        if (has_indian || has_malayalam) && !storefronts.contains(&"in".to_string()) {
            storefronts.push("in".to_string());
        }

        let mut regional_queries = Vec::new();
        if has_malayalam {
            regional_queries.push("malayalam".to_string());
            regional_queries.push("malayalam hits".to_string());
        }

        if (has_brazilian || has_latin) && !storefronts.contains(&"br".to_string()) {
            storefronts.push("br".to_string());
        }

        // 4. Check if pool needs refreshment (e.g. missing previews or too few candidates)
        let unique_artists: HashSet<_> = ext_tracks
            .iter()
            .map(|t| t.artist.to_lowercase().trim().to_string())
            .collect();
        let has_previews = ext_tracks.iter().any(|t| t.preview_url.is_some());

        if self.provider_coordinator.is_some()
            && (force_refresh || unique_artists.len() < 10 || ext_tracks.len() < limit || !has_previews)
        {
            let _ = self
                .fetch_trending_and_genre_candidates(
                    &target_itunes_genre_ids,
                    &storefronts,
                    &regional_queries,
                )
                .await;

            // Re-fetch external tracks after fresh ingestion
            ext_tracks = self
                .wishlist_repo
                .list_external_tracks(None, 300)
                .await?;
        }

        // 5. Score candidates with strong taste weighting (Genre + Artist + Trending)
        // High affinity for user top genres (+100, +80, +65) ensures trending music from
        // the user's taste overwhelmingly ranks at the top!
        struct ScoredCandidate {
            track: ExternalTrackRecord,
            score: f64,
            reason: String,
        }

        let mut scored_list: Vec<ScoredCandidate> = Vec::new();

        let candidate_local_tuples: Vec<(&str, &str, &str, f64)> = local_tracks
            .iter()
            .map(|t| {
                (
                    t.id.as_str(),
                    t.title.as_str(),
                    t.artist_name.as_deref().unwrap_or(""),
                    t.duration_secs,
                )
            })
            .collect();

        let now_millis = chrono::Utc::now().timestamp_millis();

        for mut track in ext_tracks {
            // Dynamically match against current local library so newly downloaded tracks
            // immediately transition to EXACT_MATCH / "In Library"!
            let match_res = FuzzyTrackMatcher::find_best_match(
                &track.title,
                &track.artist,
                track.duration_secs,
                candidate_local_tuples.iter().copied(),
            );

            let status = match_res.status;
            let status_str = status.as_str().to_string();
            if track.match_status != status_str {
                track.match_status = status_str;
                track.matched_local_track_id = match_res.matched_track_id.clone();
                let _ = self.wishlist_repo.upsert_external_track(&track).await;
            }

            let artist_lower = track.artist.to_lowercase().trim().to_string();
            let track_genre = track.genre.as_deref().unwrap_or("").to_lowercase();
            let track_album = track.album.as_deref().unwrap_or("").to_lowercase();

            let is_listened_artist = listened_artists.contains(&artist_lower);
            let is_library_artist = library_artists.contains(&artist_lower);

            let mut genre_score: f64 = 0.0;
            let mut matched_genre_display: Option<String> = None;

            for (idx, (ug, _)) in ranked_user_genres.iter().enumerate() {
                if genre_matches(&track_genre, ug) || genre_matches(&track_album, ug) {
                    genre_score = match idx {
                        0 => 100.0,
                        1 => 80.0,
                        2 => 65.0,
                        3 => 50.0,
                        _ => 35.0,
                    };
                    matched_genre_display =
                        Some(track.genre.clone().unwrap_or_else(|| ug.clone()));
                    break;
                }
            }

            if has_malayalam && (track_genre.contains("malayalam") || track_album.contains("malayalam")) {
                genre_score = genre_score.max(85.0);
            }

            let mut base_score = 15.0; // baseline chart presence
            if is_listened_artist {
                base_score += 90.0;
            } else if is_library_artist {
                base_score += 70.0;
            }
            base_score += genre_score;

            // Slight boost if audio preview is ready to play
            if track.preview_url.is_some() {
                base_score += 5.0;
            }

            // On force_refresh, apply a hash-based rotation offset so the recommendations rotate and feel fresh
            if force_refresh {
                let hash = (track.id.len() * 19 + track.title.len() * 37 + (now_millis as usize % 97)) % 50;
                base_score += (hash as f64) - 25.0;
            }

            let reason = match status {
                MatchStatus::NotFound => {
                    if is_listened_artist {
                        format!(
                            "✨ Trending release • Similar to your listening habit for {}",
                            track.artist
                        )
                    } else if is_library_artist {
                        format!("🎵 Trending hit by your library artist {}", track.artist)
                    } else if track_genre.contains("malayalam") || track_album.contains("malayalam") {
                        format!("🔥 Trending in Malayalam • Matches your library favorites")
                    } else if let Some(ref gd) = matched_genre_display {
                        format!("🔥 Trending in {} • Matches your top genre", gd)
                    } else {
                        format!("🔥 Trending Chart Hit • Popular discovery across charts")
                    }
                }
                MatchStatus::PossibleMatch => {
                    "Alternate release / version of local track".to_string()
                }
                MatchStatus::LikelyMatch => "Remastered edition of local track".to_string(),
                MatchStatus::ExactMatch => "Already in local library".to_string(),
            };

            scored_list.push(ScoredCandidate {
                track,
                score: base_score,
                reason,
            });
        }

        // Sort candidates: Unowned tracks first (NotFound > PossibleMatch > LikelyMatch > ExactMatch),
        // and within each group by taste affinity score descending!
        scored_list.sort_by(|a, b| {
            let match_order = |status: &str| match status {
                "NOT_FOUND" => 0,
                "POSSIBLE_MATCH" => 1,
                "LIKELY_MATCH" => 2,
                _ => 3,
            };
            let order_a = match_order(&a.track.match_status);
            let order_b = match_order(&b.track.match_status);
            if order_a != order_b {
                order_a.cmp(&order_b)
            } else {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
        });

        // 6. Enforce STRICT ARTIST DIVERSITY (maximum 2 tracks per artist)
        let mut artist_counts: HashMap<String, usize> = HashMap::new();
        let mut recs = Vec::new();

        for scored in scored_list {
            let artist_key = scored.track.artist.to_lowercase().trim().to_string();
            let count = artist_counts.entry(artist_key.clone()).or_insert(0);
            if *count >= 2 {
                continue; // Never allow more than 2 tracks from the same artist!
            }
            *count += 1;

            let in_wishlist = (scored.track.provider_id.as_str() != ""
                && wishlist_ext_ids.contains(&scored.track.id))
                || wishlist_keys.contains(&format!(
                    "{}:{}",
                    scored.track.artist.to_lowercase(),
                    scored.track.title.to_lowercase()
                ));

            let status = match scored.track.match_status.as_str() {
                "EXACT_MATCH" => MatchStatus::ExactMatch,
                "LIKELY_MATCH" => MatchStatus::LikelyMatch,
                "POSSIBLE_MATCH" => MatchStatus::PossibleMatch,
                _ => MatchStatus::NotFound,
            };

            recs.push(DiscoveryRecommendation {
                external_track_id: scored.track.id,
                provider: scored.track.provider,
                provider_id: scored.track.provider_id,
                title: scored.track.title,
                artist: scored.track.artist,
                album: scored.track.album,
                duration_secs: scored.track.duration_secs,
                cover_art_url: scored.track.cover_art_url,
                preview_url: scored.track.preview_url,
                genre: scored.track.genre,
                match_status: status,
                matched_local_track_id: scored.track.matched_local_track_id,
                recommendation_reason: scored.reason,
                in_wishlist,
            });

            if recs.len() >= limit {
                break;
            }
        }

        Ok(recs)
    }

    /// Searches online music providers (iTunes Music API) for any artist, song, or album worldwide.
    /// Ingests candidate results, matches them in real-time against the local library, and returns
    /// full DiscoveryRecommendation records ready for previewing, wishlisting, or downloading.
    pub async fn search_online_music(
        &self,
        query: &str,
        limit: usize,
    ) -> AppResult<Vec<DiscoveryRecommendation>> {
        let clean_query = query.trim();
        if clean_query.is_empty() {
            return Ok(Vec::new());
        }

        let lower_query = clean_query.to_lowercase();
        info!(query = %clean_query, limit = limit, "Executing case-insensitive online music search");

        // 1. Build HTTP client with 6s timeout and proper User-Agent
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(6))
            .user_agent("SoundFlow/0.1.0 (Linux; x86_64)")
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "Failed to build HTTP client for online search");
                return Ok(Vec::new());
            }
        };

        // 2. Fetch wishlist and local tracks for matching and synthesis
        let wishlist_items = self.wishlist_repo.get_all(None).await.unwrap_or_default();
        let wishlist_ext_ids: HashSet<String> = wishlist_items
            .iter()
            .filter_map(|w| w.external_track_id.clone())
            .collect();
        let wishlist_keys: HashSet<String> = wishlist_items
            .iter()
            .map(|w| format!("{}:{}", w.artist.to_lowercase().trim(), w.title.to_lowercase().trim()))
            .collect();

        let local_tracks = self
            .track_repo
            .list_tracks(0, 10000, None, true)
            .await
            .unwrap_or_default();

        let candidate_local_tuples: Vec<(&str, &str, &str, f64)> = local_tracks
            .iter()
            .map(|t| (t.id.as_str(), t.title.as_str(), t.artist_name.as_deref().unwrap_or(""), t.duration_secs))
            .collect();

        // 3. Multi-storefront iTunes querying (Global + Regional India/Asia for broad catalog coverage)
        let encoded = url_encode(clean_query);
        let search_limit = limit.max(15).min(50);
        let mut raw_items: Vec<serde_json::Value> = Vec::new();

        // Query 1: Global catalog (broad media=music, non-restricted entity)
        let url_global = format!(
            "https://itunes.apple.com/search?term={}&media=music&limit={}",
            encoded, search_limit
        );
        if let Ok(Ok(resp)) = tokio::time::timeout(std::time::Duration::from_secs(5), client.get(&url_global).send()).await {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(arr) = json_val.get("results").and_then(|r| r.as_array()) {
                            raw_items.extend(arr.iter().cloned());
                        }
                    }
                }
            }
        }

        // Query 2: Regional Indian storefront (&country=in) if fewer than 8 results found (captures Malayalam, Bollywood, Regional Asian tracks)
        if raw_items.len() < 8 {
            let url_in = format!(
                "https://itunes.apple.com/search?term={}&country=in&media=music&limit={}",
                encoded, search_limit
            );
            if let Ok(Ok(resp)) = tokio::time::timeout(std::time::Duration::from_secs(5), client.get(&url_in).send()).await {
                if resp.status().is_success() {
                    if let Ok(text) = resp.text().await {
                        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&text) {
                            if let Some(arr) = json_val.get("results").and_then(|r| r.as_array()) {
                                raw_items.extend(arr.iter().cloned());
                            }
                        }
                    }
                }
            }
        }

        // Query 3: Entity=song fallback if still 0 items
        if raw_items.is_empty() {
            let url_song = format!(
                "https://itunes.apple.com/search?term={}&entity=song&limit={}",
                encoded, search_limit
            );
            if let Ok(Ok(resp)) = tokio::time::timeout(std::time::Duration::from_secs(5), client.get(&url_song).send()).await {
                if resp.status().is_success() {
                    if let Ok(text) = resp.text().await {
                        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&text) {
                            if let Some(arr) = json_val.get("results").and_then(|r| r.as_array()) {
                                raw_items.extend(arr.iter().cloned());
                            }
                        }
                    }
                }
            }
        }

        // 4. Process, deduplicate, and score candidates
        struct ScoredResult {
            recommendation: DiscoveryRecommendation,
            score: i32,
        }

        let mut scored_results: Vec<ScoredResult> = Vec::new();
        let mut seen_keys: HashSet<String> = HashSet::new();
        let query_tokens: Vec<&str> = lower_query.split_whitespace().collect();

        for item in raw_items {
            let title = item.get("trackName")
                .or_else(|| item.get("trackCensoredName"))
                .or_else(|| item.get("collectionName"))
                .or_else(|| item.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            let artist = item.get("artistName")
                .or_else(|| item.get("collectionArtistName"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();

            if title.is_empty() || artist.is_empty() {
                continue;
            }

            let dedup_key = format!("{}:{}", artist.to_lowercase(), title.to_lowercase());
            if seen_keys.contains(&dedup_key) {
                continue;
            }
            seen_keys.insert(dedup_key.clone());

            let album = item.get("collectionName").and_then(|v| v.as_str()).map(|s| s.to_string());
            let genre = item.get("primaryGenreName").and_then(|v| v.as_str()).map(|s| s.to_string());
            let itunes_track_id = item.get("trackId").and_then(|v| v.as_i64()).map(|id| id.to_string()).unwrap_or_default();

            let cover_art_url = item.get("artworkUrl100")
                .or_else(|| item.get("artworkUrl60"))
                .and_then(|v| v.as_str())
                .map(|url| url.replace("100x100bb.jpg", "600x600bb.jpg").replace("60x60bb.jpg", "600x600bb.jpg"));

            let preview_url = item.get("previewUrl").and_then(|v| v.as_str()).map(|s| s.to_string());
            let duration_secs = item.get("trackTimeMillis")
                .and_then(|v| v.as_f64())
                .map(|ms| ms / 1000.0)
                .or(Some(30.0));

            let ext_id = format!("itunes:{}", if itunes_track_id.is_empty() { format!("{}:{}", artist, title) } else { itunes_track_id.clone() });

            // Dynamic library matching
            let match_res = FuzzyTrackMatcher::find_best_match(
                title,
                artist,
                duration_secs,
                candidate_local_tuples.iter().copied(),
            );

            let in_wishlist = wishlist_ext_ids.contains(&ext_id)
                || wishlist_keys.contains(&dedup_key);

            // Case-insensitive pinpoint relevance score
            let lower_title = title.to_lowercase();
            let lower_artist = artist.to_lowercase();
            let mut score = 0;

            if lower_title == lower_query {
                score += 1000;
            } else if lower_title.starts_with(&lower_query) {
                score += 500;
            } else if lower_title.contains(&lower_query) {
                score += 250;
            }

            if lower_artist == lower_query {
                score += 800;
            } else if lower_artist.starts_with(&lower_query) {
                score += 400;
            } else if lower_artist.contains(&lower_query) {
                score += 200;
            }

            for tok in &query_tokens {
                if lower_title.contains(tok) {
                    score += 60;
                }
                if lower_artist.contains(tok) {
                    score += 40;
                }
            }

            let reason = if score >= 500 {
                format!("Top online match for \"{}\"", clean_query)
            } else {
                format!("Online result for \"{}\"", clean_query)
            };

            scored_results.push(ScoredResult {
                recommendation: DiscoveryRecommendation {
                    external_track_id: ext_id,
                    provider: "itunes".to_string(),
                    provider_id: if itunes_track_id.is_empty() { title.to_string() } else { itunes_track_id },
                    title: title.to_string(),
                    artist: artist.to_string(),
                    album,
                    duration_secs,
                    cover_art_url,
                    preview_url,
                    genre,
                    match_status: match_res.status,
                    matched_local_track_id: match_res.matched_track_id,
                    recommendation_reason: reason,
                    in_wishlist,
                },
                score,
            });
        }

        // 5. Synthesize matching local library tracks that aren't already represented
        for lt in &local_tracks {
            let lt_title = lt.title.trim();
            let lt_artist = lt.artist_name.as_deref().unwrap_or("").trim();
            let lt_key = format!("{}:{}", lt_artist.to_lowercase(), lt_title.to_lowercase());
            if seen_keys.contains(&lt_key) {
                continue;
            }

            let lower_lt_title = lt_title.to_lowercase();
            let lower_lt_artist = lt_artist.to_lowercase();

            let matches_title = lower_lt_title.contains(&lower_query);
            let matches_artist = lower_lt_artist.contains(&lower_query);
            let matches_tokens = query_tokens.iter().all(|tok| lower_lt_title.contains(tok) || lower_lt_artist.contains(tok));

            if matches_title || matches_artist || matches_tokens {
                seen_keys.insert(lt_key);
                let mut score = 50;
                if lower_lt_title == lower_query {
                    score += 900;
                } else if lower_lt_title.starts_with(&lower_query) {
                    score += 450;
                } else if matches_title {
                    score += 200;
                }

                if lower_lt_artist == lower_query {
                    score += 700;
                } else if matches_artist {
                    score += 150;
                }

                scored_results.push(ScoredResult {
                    recommendation: DiscoveryRecommendation {
                        external_track_id: format!("local:{}", lt.id),
                        provider: "library".to_string(),
                        provider_id: lt.id.clone(),
                        title: lt_title.to_string(),
                        artist: lt_artist.to_string(),
                        album: lt.album_title.clone(),
                        duration_secs: Some(lt.duration_secs),
                        cover_art_url: None,
                        preview_url: None,
                        genre: lt.genre_name.clone(),
                        match_status: MatchStatus::ExactMatch,
                        matched_local_track_id: Some(lt.id.clone()),
                        recommendation_reason: "In your local library".to_string(),
                        in_wishlist: false,
                    },
                    score,
                });
            }
        }

        // 6. Sort descending by relevance score (pinpoint exact matches first)
        scored_results.sort_by(|a, b| b.score.cmp(&a.score));

        let out: Vec<DiscoveryRecommendation> = scored_results
            .into_iter()
            .map(|sr| sr.recommendation)
            .take(limit)
            .collect();

        info!(count = out.len(), query = %clean_query, "Online music search returning candidates");
        Ok(out)
    }
}

/// Helper RFC 3986 percent-encoding for search query parameters
fn url_encode(input: &str) -> String {
    let mut encoded = String::new();
    for b in input.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            b' ' => encoded.push('+'),
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}
