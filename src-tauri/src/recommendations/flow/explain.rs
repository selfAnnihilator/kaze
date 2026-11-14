//! Why a song was picked: recorded evidence for the frontend.
use super::*;

impl FlowEngine {
    /// Queries the deterministic "Why this track?" explanation for any track.
    pub async fn get_track_explanation(
        &self,
        user_id: &str,
        track_id: &str,
    ) -> AppResult<FlowExplanation> {
        let feedback = super::super::signals::SongSignals::load(&self.pool, user_id).await?;
        if feedback.blocked_id(track_id) {
            return Ok(FlowExplanation {
                primary: "not_interested".into(),
                ..Default::default()
            });
        }
        let candidate = sqlx::query_as::<_, FlowCandidateRow>(
            "SELECT t.id, t.title, t.artist_id, a.name as artist_name,
                    t.album_id, al.title as album_title, g.name as genre_name, t.year,
                    t.duration_secs, NULL as cover_art_url,
                    ts.play_count, ts.completion_count, ts.skip_count, ts.last_played_at, ts.manual_like,
                    (t.format = 'online' OR t.file_path LIKE 'online://%') as is_online, NULL as preview_url
             FROM tracks t LEFT JOIN artists a ON a.id = t.artist_id
             LEFT JOIN albums al ON al.id = t.album_id LEFT JOIN genres g ON g.id = t.genre_id
             LEFT JOIN track_statistics ts ON ts.track_id = t.id AND ts.user_id = ?
             WHERE t.id = ?")
            .bind(user_id).bind(track_id).fetch_optional(&self.pool).await?
            .ok_or_else(|| AppError::NotFound(format!("Track not found: {track_id}")))?;
        let profile = self.taste_engine.compute_taste_profile(user_id).await?;
        let mut evidence = Self::evidence(&candidate, &profile, Utc::now().timestamp());
        if let Some((source, _)) = super::super::setup::members(&self.pool, user_id, &profile)
            .await?
            .get(track_id)
        {
            evidence.primary = format!("starter_{source}");
            evidence.artist_affinity = None;
        }
        Ok(evidence)
    }

    pub async fn get_why_this_track(&self, user_id: &str, track_id: &str) -> AppResult<String> {
        let e = self.get_track_explanation(user_id, track_id).await?;
        Ok(match e.primary.as_str() {
            "starter_artist" => "From an artist you chose".into(),
            "starter_genre" => "From your genre choices".into(),
            "starter_popular" => "Popular songs to get you started".into(),
            "not_interested" => "You marked this song Not interested".into(),
            "liked" => "One of your liked tracks".into(),
            "rediscovery" => format!(
                "You haven't played this in {} days",
                e.days_since_played.unwrap()
            ),
            "library_exploration" => "Unplayed track from your library".into(),
            "artist_affinity" => format!(
                "Matches your affinity for {}",
                e.artist_name.as_deref().unwrap_or("this artist")
            ),
            "online_discovery" => "Online discovery matching your listening taste".into(),
            "online" => "Selected from your online listening".into(),
            _ => "Selected from your library".into(),
        })
    }

    pub(super) fn song_key(title: &str, artist: &str) -> String {
        let clean = |value: &str| {
            value
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
        };
        format!("{}:{}", clean(artist), clean(title))
    }

    pub(super) fn evidence(c: &FlowCandidateRow, profile: &TasteProfile, now: i64) -> FlowExplanation {
        FlowExplanation {
            primary: Self::explain(c, profile, now).1,
            is_liked: c.manual_like == Some(1),
            play_count: c.play_count,
            is_unplayed: c.play_count.unwrap_or(0) == 0
                && c.last_played_at.is_none()
                && c.skip_count.unwrap_or(0) == 0,
            days_since_played: c.last_played_at.map(|lp| (now - lp).max(0) / 86400),
            artist_affinity: c
                .artist_id
                .as_ref()
                .and_then(|id| profile.artist_affinities.get(id))
                .copied()
                .filter(|value| *value > 0.3),
            artist_name: c.artist_name.clone(),
            ..Default::default()
        }
    }

    pub(super) fn explain(c: &FlowCandidateRow, profile: &TasteProfile, now: i64) -> (String, String) {
        let (reason, category) = if c.manual_like == Some(1) {
            ("One of your liked tracks".into(), "liked")
        } else if let Some(days) = c
            .last_played_at
            .map(|lp| (now - lp).max(0) / 86400)
            .filter(|d| *d >= 30)
        {
            (
                format!("You haven't played this in {days} days"),
                "rediscovery",
            )
        } else if c.play_count.unwrap_or(0) == 0
            && c.last_played_at.is_none()
            && c.skip_count.unwrap_or(0) == 0
        {
            if c.is_online {
                (
                    "Online discovery matching your listening taste".into(),
                    "online_discovery",
                )
            } else {
                (
                    "Unplayed track from your library".into(),
                    "library_exploration",
                )
            }
        } else if c
            .artist_id
            .as_ref()
            .and_then(|id| profile.artist_affinities.get(id))
            .copied()
            .unwrap_or(0.0)
            > 0.3
        {
            (
                format!(
                    "Matches your affinity for {}",
                    c.artist_name.as_deref().unwrap_or("this artist")
                ),
                "artist_affinity",
            )
        } else if c.is_online {
            ("Selected from your online listening".into(), "online")
        } else {
            ("Selected from your library".into(), "library")
        };
        (reason, category.into())
    }
}
