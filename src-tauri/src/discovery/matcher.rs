use serde::{Deserialize, Serialize};
use strsim::jaro_winkler;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MatchStatus {
    ExactMatch,
    LikelyMatch,
    PossibleMatch,
    NotFound,
}

impl MatchStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            MatchStatus::ExactMatch => "EXACT_MATCH",
            MatchStatus::LikelyMatch => "LIKELY_MATCH",
            MatchStatus::PossibleMatch => "POSSIBLE_MATCH",
            MatchStatus::NotFound => "NOT_FOUND",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub status: MatchStatus,
    pub confidence: f64,
    pub matched_track_id: Option<String>,
    pub explanation: String,
}

pub struct FuzzyTrackMatcher;

impl FuzzyTrackMatcher {
    /// Normalizes music titles and artist strings by removing common noise:
    /// bracketed remasters, editions, featured artists, and non-alphanumeric punctuation.
    pub fn normalize_string(s: &str) -> String {
        let mut clean = s.to_lowercase();

        // Strip common parenthetical noise like (Remastered 2011), [Deluxe Edition], (feat. Drake)
        if let Some(pos) = clean.find('(') {
            clean.truncate(pos);
        }
        if let Some(pos) = clean.find('[') {
            clean.truncate(pos);
        }
        // Strip common featured artist prefixes
        if let Some(pos) = clean.find(" feat.") {
            clean.truncate(pos);
        } else if let Some(pos) = clean.find(" ft.") {
            clean.truncate(pos);
        } else if let Some(pos) = clean.find(" feat ") {
            clean.truncate(pos);
        }

        // Keep alphanumeric and spaces only
        let res = clean
            .chars()
            .map(|c| if c.is_alphanumeric() || c.is_whitespace() { c } else { ' ' })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        // Strip leading articles ("the ", "a ", "an ") for standard music matching
        if let Some(stripped) = res.strip_prefix("the ") {
            stripped.to_string()
        } else if let Some(stripped) = res.strip_prefix("a ") {
            stripped.to_string()
        } else if let Some(stripped) = res.strip_prefix("an ") {
            stripped.to_string()
        } else {
            res
        }
    }

    /// Evaluates fuzzy similarity between an external candidate and a local library track.
    pub fn compare(
        ext_title: &str,
        ext_artist: &str,
        ext_duration: Option<f64>,
        local_id: &str,
        local_title: &str,
        local_artist: &str,
        local_duration: f64,
    ) -> MatchResult {
        let norm_ext_title = Self::normalize_string(ext_title);
        let norm_local_title = Self::normalize_string(local_title);

        let norm_ext_artist = Self::normalize_string(ext_artist);
        let norm_local_artist = Self::normalize_string(local_artist);

        let title_sim = jaro_winkler(&norm_ext_title, &norm_local_title);
        let artist_sim = jaro_winkler(&norm_ext_artist, &norm_local_artist);

        let duration_diff = ext_duration.map(|ed| (ed - local_duration).abs());

        // Evaluation criteria:
        // 1. EXACT_MATCH: very high title & artist similarity + close duration (<= 4s)
        if title_sim >= 0.95 && artist_sim >= 0.90 {
            if let Some(diff) = duration_diff {
                if diff <= 4.0 {
                    return MatchResult {
                        status: MatchStatus::ExactMatch,
                        confidence: (title_sim * 0.5 + artist_sim * 0.4 + 0.1).clamp(0.0, 1.0),
                        matched_track_id: Some(local_id.to_string()),
                        explanation: format!(
                            "Exact match: Title {:.0}%, Artist {:.0}%, Duration diff {:.1}s",
                            title_sim * 100.0,
                            artist_sim * 100.0,
                            diff
                        ),
                    };
                }
            } else {
                // If duration is unknown, high string similarity qualifies
                return MatchResult {
                    status: MatchStatus::ExactMatch,
                    confidence: (title_sim * 0.6 + artist_sim * 0.4).clamp(0.0, 1.0),
                    matched_track_id: Some(local_id.to_string()),
                    explanation: format!(
                        "Exact string match: Title {:.0}%, Artist {:.0}%",
                        title_sim * 100.0,
                        artist_sim * 100.0
                    ),
                };
            }
        }

        // 2. LIKELY_MATCH: high similarity + moderate duration tolerance (<= 10s)
        if title_sim >= 0.85 && artist_sim >= 0.80 {
            let dur_ok = duration_diff.map_or(true, |d| d <= 10.0);
            if dur_ok {
                return MatchResult {
                    status: MatchStatus::LikelyMatch,
                    confidence: (title_sim * 0.55 + artist_sim * 0.45).clamp(0.0, 1.0),
                    matched_track_id: Some(local_id.to_string()),
                    explanation: format!(
                        "Likely match: Title {:.0}%, Artist {:.0}%",
                        title_sim * 100.0,
                        artist_sim * 100.0
                    ),
                };
            }
        }

        // 3. POSSIBLE_MATCH: reasonable similarity, candidate might be a live or alternate version
        if title_sim >= 0.75 && artist_sim >= 0.70 {
            return MatchResult {
                status: MatchStatus::PossibleMatch,
                confidence: (title_sim * 0.6 + artist_sim * 0.4).clamp(0.0, 1.0),
                matched_track_id: Some(local_id.to_string()),
                explanation: format!(
                    "Possible match (alternate cut/live): Title {:.0}%, Artist {:.0}%",
                    title_sim * 100.0,
                    artist_sim * 100.0
                ),
            };
        }

        // 4. NOT_FOUND
        MatchResult {
            status: MatchStatus::NotFound,
            confidence: 0.0,
            matched_track_id: None,
            explanation: "No matching track found in local library".to_string(),
        }
    }

    /// Iterates through candidates and finds the single best matching local track.
    pub fn find_best_match<'a, I>(
        ext_title: &str,
        ext_artist: &str,
        ext_duration: Option<f64>,
        candidates: I,
    ) -> MatchResult
    where
        I: IntoIterator<Item = (&'a str, &'a str, &'a str, f64)>,
    {
        let mut best_match: Option<MatchResult> = None;

        for (local_id, local_title, local_artist, local_dur) in candidates {
            let res = Self::compare(
                ext_title,
                ext_artist,
                ext_duration,
                local_id,
                local_title,
                local_artist,
                local_dur,
            );

            if res.status == MatchStatus::ExactMatch {
                return res;
            }

            if res.status != MatchStatus::NotFound {
                if let Some(ref current_best) = best_match {
                    if res.confidence > current_best.confidence {
                        best_match = Some(res);
                    }
                } else {
                    best_match = Some(res);
                }
            }
        }

        best_match.unwrap_or_else(|| MatchResult {
            status: MatchStatus::NotFound,
            confidence: 0.0,
            matched_track_id: None,
            explanation: "No matching track found in local library".to_string(),
        })
    }
}
