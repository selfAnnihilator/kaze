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

#[derive(Debug, Clone)]
pub struct LocalTrackCandidate<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub artist: &'a str,
    pub duration_secs: f64,
    pub is_lofi: bool,
}

impl<'a> From<(&'a str, &'a str, &'a str, f64)> for LocalTrackCandidate<'a> {
    fn from(t: (&'a str, &'a str, &'a str, f64)) -> Self {
        LocalTrackCandidate {
            id: t.0,
            title: t.1,
            artist: t.2,
            duration_secs: t.3,
            is_lofi: false,
        }
    }
}

pub struct FuzzyTrackMatcher;

impl FuzzyTrackMatcher {
    /// Detects if a title, artist, genre, or context indicates lofi music.
    pub fn is_lofi_indicator(s: &str) -> bool {
        let lower = s.to_lowercase();
        lower.contains("lofi")
            || lower.contains("lo-fi")
            || lower.contains("lo fi")
            || lower.contains("chillhop")
            || lower.contains("chill beat")
            || lower.contains("sleep beat")
            || lower.contains("lofi hip hop")
    }

    /// Normalizes music titles and artist strings by removing common noise:
    /// diacritics, bracketed remasters, editions, featured artists, and non-alphanumeric punctuation.
    pub fn normalize_string(s: &str) -> String {
        let mut clean = s.to_lowercase();

        // Convert common accented characters to ASCII equivalents (e.g. Pokémon -> Pokemon, Nausicaä -> Nausicaa)
        clean = clean
            .chars()
            .map(|c| match c {
                'é' | 'è' | 'ê' | 'ë' => 'e',
                'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' => 'a',
                'í' | 'ì' | 'î' | 'ï' => 'i',
                'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
                'ú' | 'ù' | 'û' | 'ü' => 'u',
                'ñ' => 'n',
                'ç' => 'c',
                other => other,
            })
            .collect();

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

    /// Checks if two artists match, accounting for collaborative credits (e.g. "Eminem" in "Eminem, Nate Dogg").
    pub fn artists_match(a1: &str, a2: &str) -> bool {
        let norm1 = Self::normalize_string(a1);
        let norm2 = Self::normalize_string(a2);

        if norm1.is_empty() || norm2.is_empty() {
            return false;
        }

        if norm1 == norm2 {
            return true;
        }

        // Split collaborations before punctuation stripping
        let split_delims = [",", "&", "/", " and ", " x ", " with ", " feat ", " ft "];
        let mut list1: Vec<String> = vec![a1.to_string()];
        for d in &split_delims {
            list1 = list1.iter().flat_map(|s| s.split(d)).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        }

        let mut list2: Vec<String> = vec![a2.to_string()];
        for d in &split_delims {
            list2 = list2.iter().flat_map(|s| s.split(d)).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        }

        let norm_list1: Vec<String> = list1.iter().map(|s| Self::normalize_string(s)).filter(|s| !s.is_empty()).collect();
        let norm_list2: Vec<String> = list2.iter().map(|s| Self::normalize_string(s)).filter(|s| !s.is_empty()).collect();

        for sub1 in &norm_list1 {
            for sub2 in &norm_list2 {
                if sub1 == sub2 && sub1.len() >= 2 {
                    return true;
                }
            }
        }

        false
    }

    /// Checks if two normalized song titles match, preventing common prefix false positives.
    pub fn titles_match(t1: &str, t2: &str) -> bool {
        let norm1 = Self::normalize_string(t1);
        let norm2 = Self::normalize_string(t2);

        if norm1.is_empty() || norm2.is_empty() {
            return false;
        }

        if norm1 == norm2 {
            return true;
        }

        // Prevent common prefix traps like "pokemon center" vs "pokemon theme"
        let words1: Vec<&str> = norm1.split_whitespace().collect();
        let words2: Vec<&str> = norm2.split_whitespace().collect();
        if words1.len() > 1 && words2.len() > 1 {
            let common = words1.iter().filter(|w| words2.contains(w)).count();
            let overlap = common as f64 / words1.len().max(words2.len()) as f64;
            if overlap < 0.8 {
                return false;
            }
            // If overlap is >= 0.8, ensure the non-matching words are not critical divergent nouns
            return true;
        }

        false
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

        let title_matches = Self::titles_match(ext_title, local_title);
        let artist_matches = Self::artists_match(ext_artist, local_artist);

        let is_preview_duration = ext_duration.map_or(true, |ed| ed <= 35.0);
        let duration_diff = if is_preview_duration {
            None
        } else {
            ext_duration.map(|ed| (ed - local_duration).abs())
        };

        if title_matches && artist_matches {
            if let Some(diff) = duration_diff {
                if diff <= 4.0 {
                    return MatchResult {
                        status: MatchStatus::ExactMatch,
                        confidence: 1.0,
                        matched_track_id: Some(local_id.to_string()),
                        explanation: format!("Exact match: Duration diff {:.1}s", diff),
                    };
                } else if diff <= 12.0 {
                    return MatchResult {
                        status: MatchStatus::LikelyMatch,
                        confidence: 0.9,
                        matched_track_id: Some(local_id.to_string()),
                        explanation: format!("Likely match: Duration diff {:.1}s", diff),
                    };
                } else {
                    return MatchResult {
                        status: MatchStatus::PossibleMatch,
                        confidence: 0.7,
                        matched_track_id: None,
                        explanation: format!("Possible alternate cut: Duration diff {:.1}s", diff),
                    };
                }
            } else {
                return MatchResult {
                    status: MatchStatus::ExactMatch,
                    confidence: 0.98,
                    matched_track_id: Some(local_id.to_string()),
                    explanation: "Exact title and artist match".to_string(),
                };
            }
        }

        // Fuzzy similarity fallback for backwards compatibility in discovery charts
        let raw_title_sim = jaro_winkler(&norm_ext_title, &norm_local_title);
        let norm_ext_artist = Self::normalize_string(ext_artist);
        let norm_local_artist = Self::normalize_string(local_artist);
        let raw_artist_sim = jaro_winkler(&norm_ext_artist, &norm_local_artist);

        if raw_title_sim >= 0.85 && raw_artist_sim >= 0.80 {
            if let Some(diff) = duration_diff {
                if diff <= 10.0 {
                    return MatchResult {
                        status: MatchStatus::LikelyMatch,
                        confidence: (raw_title_sim * 0.55 + raw_artist_sim * 0.45).clamp(0.0, 1.0),
                        matched_track_id: Some(local_id.to_string()),
                        explanation: "Likely match via high string similarity".to_string(),
                    };
                }
            }
        }

        if raw_title_sim >= 0.75 && raw_artist_sim >= 0.70 {
            return MatchResult {
                status: MatchStatus::PossibleMatch,
                confidence: (raw_title_sim * 0.6 + raw_artist_sim * 0.4).clamp(0.0, 1.0),
                matched_track_id: None,
                explanation: "Possible match via title/artist similarity".to_string(),
            };
        }

        MatchResult {
            status: MatchStatus::NotFound,
            confidence: 0.0,
            matched_track_id: None,
            explanation: "No matching track found in local library".to_string(),
        }
    }

    /// Iterates through candidates and finds the single best matching local track.
    ///
    /// Rules:
    /// - For a regular song: an exact match (both title and artist matching) is required.
    ///   If unavailable, do NOT check for closest match; return NotFound.
    /// - For a lofi song (is_lofi_context): if title + artist matches, prefer it.
    ///   Otherwise, look for a track with the same title that is a lofi song,
    ///   even if the artist does not match.
    pub fn find_best_match<'a, C, I>(
        ext_title: &str,
        ext_artist: &str,
        ext_duration: Option<f64>,
        is_lofi_context: bool,
        candidates: I,
    ) -> MatchResult
    where
        C: Into<LocalTrackCandidate<'a>>,
        I: IntoIterator<Item = C>,
    {
        let is_lofi = is_lofi_context
            || Self::is_lofi_indicator(ext_title)
            || Self::is_lofi_indicator(ext_artist);

        let mut exact_match: Option<MatchResult> = None;
        let mut lofi_title_match: Option<MatchResult> = None;

        for item in candidates {
            let cand: LocalTrackCandidate<'a> = item.into();

            let title_ok = Self::titles_match(ext_title, cand.title);
            if !title_ok {
                continue;
            }

            let artist_ok = Self::artists_match(ext_artist, cand.artist);

            if artist_ok {
                // Both title and artist match!
                let is_preview = ext_duration.map_or(true, |d| d <= 35.0);
                let dur_diff = if is_preview {
                    None
                } else {
                    ext_duration.map(|d| (d - cand.duration_secs).abs())
                };

                let dur_close = dur_diff.map_or(true, |d| d <= 12.0);
                if dur_close {
                    return MatchResult {
                        status: MatchStatus::ExactMatch,
                        confidence: 1.0,
                        matched_track_id: Some(cand.id.to_string()),
                        explanation: format!("Exact match for '{}' by '{}'", cand.title, cand.artist),
                    };
                } else if exact_match.is_none() {
                    exact_match = Some(MatchResult {
                        status: MatchStatus::ExactMatch,
                        confidence: 0.95,
                        matched_track_id: Some(cand.id.to_string()),
                        explanation: format!("Title and artist match for '{}' by '{}'", cand.title, cand.artist),
                    });
                }
            } else if is_lofi && cand.is_lofi && lofi_title_match.is_none() {
                // Lofi mode: artist did not match, but candidate has the same title and is a lofi track!
                lofi_title_match = Some(MatchResult {
                    status: MatchStatus::LikelyMatch,
                    confidence: 0.85,
                    matched_track_id: Some(cand.id.to_string()),
                    explanation: format!(
                        "Lofi track matched by title '{}' (library artist '{}', requested '{}')",
                        cand.title, cand.artist, ext_artist
                    ),
                });
            }
        }

        if let Some(exact) = exact_match {
            return exact;
        }

        if is_lofi {
            if let Some(lofi_match) = lofi_title_match {
                return lofi_match;
            }
        }

        // For regular songs (or lofi songs without a match): return NotFound instead of a closest match
        MatchResult {
            status: MatchStatus::NotFound,
            confidence: 0.0,
            matched_track_id: None,
            explanation: if is_lofi {
                "No matching lofi track found in library".to_string()
            } else {
                format!("Exact match for '{}' by '{}' not in library", ext_title, ext_artist)
            },
        }
    }
}
