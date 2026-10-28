use super::taste::TasteProfile;
use serde::{Deserialize, Serialize};

/// Represents an individual factor contributing to a recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationReason {
    pub factor: String,
    pub description: String,
    pub weight: f64,
}

/// Evaluation result for candidate track scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackRecommendationScore {
    pub track_id: String,
    pub final_score: f64,
    pub reasons: Vec<RecommendationReason>,
    pub is_discovery: bool,
}

/// Metadata input required to score a candidate track.
#[derive(Debug, Clone)]
pub struct CandidateTrack {
    pub id: String,
    pub title: String,
    pub artist_id: Option<String>,
    pub artist_name: Option<String>,
    pub genre_name: Option<String>,
    pub year: Option<i64>,
    pub language: Option<String>,
    pub play_count: i64,
    pub completion_count: i64,
    pub skip_count: i64,
    pub listening_engagement: Option<f64>,
    pub last_played_at: Option<i64>,
    pub user_feedback: Option<String>, // "like", "dislike", or None
}

pub struct ScoringEngine;

impl ScoringEngine {
    /// Computes the personalized recommendation score and explanation reasons for a candidate track.
    pub fn score_track(
        candidate: &CandidateTrack,
        profile: &TasteProfile,
        current_time: i64,
    ) -> TrackRecommendationScore {
        let mut reasons = Vec::new();

        // 1. Check for dislike: immediately drops to 0
        if let Some(fb) = &candidate.user_feedback {
            if fb == "dislike" {
                reasons.push(RecommendationReason {
                    factor: "dislike".to_string(),
                    description: "Disliked track (excluded from recommendations)".to_string(),
                    weight: -1.0,
                });
                return TrackRecommendationScore {
                    track_id: candidate.id.clone(),
                    final_score: 0.0,
                    reasons,
                    is_discovery: false,
                };
            }
        }

        let mut base_score = 0.0;

        // 2. Artist Affinity (Weight: 35%)
        let artist_score = candidate
            .artist_id
            .as_ref()
            .and_then(|id| profile.artist_affinities.get(id))
            .copied()
            .unwrap_or(0.0)
            .max(profile.artist_affinity(candidate.artist_name.as_deref().unwrap_or("")));
        if artist_score > 0.1 {
            reasons.push(RecommendationReason {
                factor: "artist_affinity".into(),
                description: format!(
                    "Listening affinity for {}: {:.0}%",
                    candidate.artist_name.as_deref().unwrap_or("Artist"),
                    artist_score * 100.0
                ),
                weight: artist_score * 0.35,
            });
        }
        if let Some(language) = candidate.language.as_ref() {
            let affinity = profile
                .language_affinities
                .get(&language.to_lowercase())
                .copied()
                .unwrap_or(0.0);
            if affinity > 0.0 {
                base_score += affinity * 0.10;
                reasons.push(RecommendationReason {
                    factor: "language_affinity".into(),
                    description: format!("Matches your {language} listening"),
                    weight: affinity * 0.10,
                });
            }
        }
        base_score += artist_score * 0.35;

        // 3. Genre Affinity (Weight: 25%)
        let mut genre_score = 0.0;
        if let Some(genre) = &candidate.genre_name {
            if let Some(&aff) = profile.genre_affinities.get(genre.trim()) {
                genre_score = aff;
                if aff > 0.1 {
                    reasons.push(RecommendationReason {
                        factor: "genre_affinity".to_string(),
                        description: format!("Favorite genre {}: {:.0}% match", genre, aff * 100.0),
                        weight: aff * 0.25,
                    });
                }
            }
        }
        base_score += genre_score * 0.25;

        // 4. Era Affinity (Weight: 10%)
        if let Some(year) = candidate.year {
            let decade = format!("{}0s", (year / 10));
            if let Some(&aff) = profile.era_affinities.get(&decade) {
                base_score += aff * 0.10;
                if aff > 0.2 {
                    reasons.push(RecommendationReason {
                        factor: "era_affinity".to_string(),
                        description: format!("Favorite era {}: {:.0}% match", decade, aff * 100.0),
                        weight: aff * 0.10,
                    });
                }
            }
        }

        // 5. Track Performance & Completion (Weight: 20%)
        let mut perf_score = 0.5;
        if let Some(engagement) = candidate.listening_engagement {
            perf_score = (0.5 + engagement * 0.5).clamp(0.0, 1.0);
            if engagement < 0.0 {
                base_score += engagement * 0.20;
                reasons.push(RecommendationReason {
                    factor: "early_skip".into(),
                    description: "Quickly skipped listens reduce preference".into(),
                    weight: engagement * 0.20,
                });
            } else if engagement > 0.0 {
                reasons.push(RecommendationReason {
                    factor: "listening_time".into(),
                    description: "Preference earned through meaningful listening time".into(),
                    weight: engagement * 0.10,
                });
            }
        } else if candidate.play_count > 0 {
            // Aggregate-only cloud/legacy stats have no per-session time evidence.
            perf_score =
                (candidate.completion_count as f64 / candidate.play_count as f64).clamp(0.0, 1.0);
        }
        base_score += perf_score * 0.20;

        // Repeated qualified listens strengthen a song, not just its artist.
        if candidate.play_count >= 2
            && candidate
                .listening_engagement
                .is_some_and(|weight| weight > 0.0)
        {
            let repeat =
                (((candidate.play_count - 1) as f64).ln_1p() / 6.0_f64.ln()).min(1.0) * 0.10;
            base_score += repeat;
            reasons.push(RecommendationReason {
                factor: "meaningful_repeats".into(),
                description: "Repeated meaningful listens".into(),
                weight: repeat,
            });
        }

        // 6. Explicit Like Boost (+0.25)
        if let Some(fb) = &candidate.user_feedback {
            if fb == "like" {
                base_score += 0.25;
                reasons.push(RecommendationReason {
                    factor: "explicit_like".to_string(),
                    description: "Liked track boost (+25%)".to_string(),
                    weight: 0.25,
                });
            }
        }

        // 7. Discovery Bonus for Unplayed Gems (Weight: +0.20)
        let mut is_discovery = false;
        if candidate.play_count == 0 && candidate.skip_count == 0 {
            if artist_score > 0.25 || genre_score > 0.25 {
                base_score += 0.20;
                is_discovery = true;
                reasons.push(RecommendationReason {
                    factor: "exploration_pick".to_string(),
                    description: "Discovery pick: Unplayed local track matching your musical taste"
                        .to_string(),
                    weight: 0.20,
                });
            }
        }

        // 8. Repetition Penalty / Recency Dampening
        if let Some(last_played) = candidate.last_played_at {
            let age_secs = current_time - last_played;
            if age_secs < 86400 {
                // Played within 24h: 50% penalty
                base_score *= 0.50;
                reasons.push(RecommendationReason {
                    factor: "repetition_penalty".to_string(),
                    description: "Repetition penalty: Played in the last 24 hours (-50%)"
                        .to_string(),
                    weight: -0.50,
                });
            } else if age_secs < (3 * 86400) {
                // Played within 72h: 30% penalty
                base_score *= 0.70;
                reasons.push(RecommendationReason {
                    factor: "repetition_penalty".to_string(),
                    description: "Repetition penalty: Played in the last 3 days (-30%)".to_string(),
                    weight: -0.30,
                });
            } else if age_secs < (7 * 86400) {
                // Played within 7d: 15% penalty
                base_score *= 0.85;
                reasons.push(RecommendationReason {
                    factor: "repetition_penalty".to_string(),
                    description: "Repetition penalty: Played this week (-15%)".to_string(),
                    weight: -0.15,
                });
            } else if age_secs >= (30 * 86400) && candidate.play_count >= 2 {
                // Forgotten favorite bonus: unplayed for over 30 days
                base_score += 0.15;
                reasons.push(RecommendationReason {
                    factor: "forgotten_favorite".to_string(),
                    description: "Forgotten favorite: Not listened to in over 30 days (+15%)"
                        .to_string(),
                    weight: 0.15,
                });
            }
        }

        // Ensure every recommended track has a human-readable explanation
        if reasons.is_empty() && base_score > 0.0 {
            reasons.push(RecommendationReason {
                factor: "library_exploration".to_string(),
                description: "Library catalog exploration: Unheard track ready for discovery"
                    .to_string(),
                weight: base_score,
            });
        }

        TrackRecommendationScore {
            track_id: candidate.id.clone(),
            final_score: base_score.clamp(0.0, 2.0),
            reasons,
            is_discovery,
        }
    }
}
