//! Scoring one For You candidate: local-library match, style relevance and the
//! weighted taste score. Keep the order of the score terms; float sums depend on it.
use super::inputs::TasteInputs;
use super::*;
use crate::recommendations::scoring::{CandidateTrack, ScoringEngine};
use crate::recommendations::signals::{ListeningSignal, SongSignals};
use crate::recommendations::similarity::SimilarHit;

#[derive(Clone)]
pub(super) struct ScoredCandidate {
    pub(super) track: ExternalTrackRecord,
    pub(super) score: f64,
    pub(super) reason: String,
}

/// Everything constant across one scoring run.
pub(super) struct ForYouScorer<'a> {
    pub(super) taste: &'a TasteInputs,
    pub(super) signals: &'a SongSignals,
    pub(super) starters: &'a HashMap<String, (String, String)>,
}

/// Why a candidate is relevant to this user.
struct Relevance<'a> {
    artist_affinity: f64,
    is_library_artist: bool,
    library_signal: bool,
    genre_score: f64,
    matched_genre: Option<String>,
    language: Option<&'a String>,
    language_affinity: f64,
    similar_hit: Option<&'a SimilarHit>,
}

impl DiscoveryCoordinator {
    /// Scores every admissible candidate, storing changed local-match states.
    pub(super) async fn score_candidates(
        &self,
        tracks: Vec<ExternalTrackRecord>,
        scorer: &ForYouScorer<'_>,
    ) -> Vec<ScoredCandidate> {
        let local = local_match_tuples(scorer.taste);
        let mut scored = Vec::new();
        for mut track in tracks {
            if scorer.signals.blocked(&track.id, &track.title, &track.artist) {
                continue;
            }
            let listening = scorer.signals.signal(&track.id, &track.title, &track.artist);
            if listening.skip_count >= 3 && listening.skip_count > listening.play_count {
                continue;
            }
            let status = match_local(&mut track, &local);
            if let Some(changed) = status.1 {
                track.match_status = changed.0;
                track.matched_local_track_id = changed.1;
                let _ = self.wishlist_repo.upsert_external_track(&track).await;
            }
            if let Some(item) = scorer.score(track, &listening, status.0) {
                scored.push(item);
            }
        }
        scored
    }
}

type LocalTuple<'a> = (&'a str, &'a str, &'a str, f64);

fn local_match_tuples(taste: &TasteInputs) -> Vec<LocalTuple<'_>> {
    taste
        .local_tracks
        .iter()
        .filter(|t| {
            t.format != "online"
                && !t.file_path.starts_with("online://")
                && !t.id.starts_with("itunes:")
                && !t.id.starts_with("online:")
        })
        .map(|t| (t.id.as_str(), t.title.as_str(), t.artist_name.as_deref().unwrap_or(""), t.duration_secs))
        .collect()
}

/// Matches a candidate against the library. Returns the status and, when it
/// differs from what is stored, the new (status, matched id).
fn match_local(track: &mut ExternalTrackRecord, local: &[LocalTuple]) -> (MatchStatus, Option<(String, Option<String>)>) {
    let is_lofi = FuzzyTrackMatcher::is_lofi_indicator(&track.title) || FuzzyTrackMatcher::is_lofi_indicator(&track.artist);
    // Older chart rows stored the preview length as the song length.
    if track.preview_url.is_some() && track.duration_secs.is_some_and(|seconds| seconds <= 35.0) {
        track.duration_secs = None;
    }
    let found = FuzzyTrackMatcher::find_best_match(&track.title, &track.artist, track.duration_secs, is_lofi, local.iter().copied());
    let status_str = found.status.as_str().to_string();
    let changed = (track.match_status != status_str || track.matched_local_track_id != found.matched_track_id)
        .then(|| (status_str, found.matched_track_id.clone()));
    (found.status, changed)
}

impl Relevance<'_> {
    /// A listened or library artist, a liked style, or a similar artist.
    fn matches_taste(&self) -> bool {
        let style = self.genre_score > 0.0 || self.language_affinity > 0.0;
        let artist = self.artist_affinity > 0.0 || self.library_signal;
        artist || style || self.similar_hit.is_some()
    }
}

impl ForYouScorer<'_> {
    fn score(&self, track: ExternalTrackRecord, listening: &ListeningSignal, status: MatchStatus) -> Option<ScoredCandidate> {
        let relevance = self.relevance(&track);
        let known = listening.manual_like == 1 || listening.play_count > 0;
        if !(known || relevance.matches_taste() || self.starters.contains_key(&track.id)) {
            return None;
        }
        let recording_score = self.recording_score(&track, listening, relevance.language);
        let artist_affinity = relevance.artist_affinity;
        // Style (genre/language) leads; artist affinity is saturating and already
        // includes the owned-library prior, so it is weighted more gently.
        let base_score = 90.0 * artist_affinity
            + relevance.genre_score
            + 35.0 * relevance.language_affinity
            + if relevance.library_signal { 10.0 } else { 0.0 }
            + relevance.similar_hit.map_or(0.0, |h| 40.0 * h.boost * (1.0 - artist_affinity))
            + recording_score * 50.0;
        let (score, reason) = match self.starters.get(&track.id) {
            Some((source, value)) => (base_score.max(10.0), crate::recommendations::setup::explanation(source, value)),
            None => (base_score, self.reason(&track, status, &relevance)),
        };
        Some(ScoredCandidate { track, score, reason })
    }

    fn relevance<'s>(&'s self, track: &ExternalTrackRecord) -> Relevance<'s> {
        let taste = self.taste;
        let artist_lower = track.artist.to_lowercase().trim().to_string();
        let track_genre = track.genre.as_deref().unwrap_or("").to_lowercase();
        let track_album = track.album.as_deref().unwrap_or("").to_lowercase();
        let is_library_artist = taste.library_artists.contains(&artist_lower);
        let matched = taste
            .ranked_genres
            .iter()
            .find(|(genre, _)| genre_matches(&track_genre, genre) || genre_matches(&track_album, genre));
        let language = self.signals.identities.get(&track.id).and_then(|song| song.language.as_ref());
        Relevance {
            artist_affinity: taste.profile.artist_affinity(&track.artist),
            is_library_artist,
            library_signal: is_library_artist && !taste.listened_artists.contains_key(&artist_lower),
            genre_score: matched.map_or(0.0, |(_, weight)| *weight),
            matched_genre: matched.map(|(genre, _)| track.genre.clone().unwrap_or_else(|| genre.clone())),
            language,
            language_affinity: language
                .and_then(|language| taste.profile.language_affinities.get(&language.to_lowercase()))
                .copied()
                .unwrap_or(0.0),
            similar_hit: taste.similarity.hit(&track.artist).filter(|h| h.boost > 0.2),
        }
    }

    /// The shared recording scorer: taste, era, completion, feedback and recency.
    fn recording_score(&self, track: &ExternalTrackRecord, listening: &ListeningSignal, language: Option<&String>) -> f64 {
        let input = CandidateTrack {
            id: track.id.clone(),
            title: track.title.clone(),
            artist_id: None,
            artist_name: Some(track.artist.clone()),
            genre_name: track.genre.clone(),
            year: self.signals.identities.get(&track.id).and_then(|song| song.year),
            language: language.cloned(),
            play_count: listening.play_count,
            completion_count: listening.completion_count,
            skip_count: listening.skip_count,
            listening_engagement: listening.engagement(),
            last_played_at: listening.last_played_at,
            user_feedback: if listening.manual_like == 1 { Some("like".into()) } else { None },
        };
        ScoringEngine::score_track(&input, &self.taste.profile, chrono::Utc::now().timestamp()).final_score
    }

    fn reason(&self, track: &ExternalTrackRecord, status: MatchStatus, relevance: &Relevance) -> String {
        match status {
            MatchStatus::NotFound => self.taste_reason(track, relevance),
            MatchStatus::PossibleMatch => "Alternate release / version of local track".to_string(),
            MatchStatus::LikelyMatch => "Remastered edition of local track".to_string(),
            MatchStatus::ExactMatch => "Already in local library".to_string(),
        }
    }

    fn taste_reason(&self, track: &ExternalTrackRecord, relevance: &Relevance) -> String {
        if relevance.artist_affinity > 0.0 && self.taste.profile.artist_breadth(&track.artist) > 0 {
            format!(
                "Based on your listening habit for {} ({:.0}% affinity)",
                track.artist,
                relevance.artist_affinity * 100.0
            )
        } else if relevance.is_library_artist {
            format!("By your library artist {}", track.artist)
        } else if let Some(hit) = relevance.similar_hit {
            format!("Fans of {} also like {}", hit.because_of, track.artist)
        } else if let Some(genre) = &relevance.matched_genre {
            format!("Matches your {genre} listening")
        } else if let Some(language) = relevance.language.filter(|_| relevance.language_affinity > 0.0) {
            format!("Matches your {language} listening")
        } else {
            "A recording from your listening history".into()
        }
    }
}
