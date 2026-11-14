//! Candidate rows for Flow: library songs, taste-relevant online songs, and one
//! preferred release per online song.
use super::inputs::FlowInputs;
use super::*;
use crate::recommendations::session;
use crate::recommendations::signals::SongSignals;

const LOCAL_CANDIDATES: &str = "SELECT t.id, t.title, t.artist_id, a.name as artist_name,
        t.album_id, al.title as album_title, g.name as genre_name, t.year,
        t.duration_secs,
        CASE WHEN t.has_cover_art = 1 THEN '/api/cover/' || t.id ELSE NULL END as cover_art_url,
        ts.play_count, ts.completion_count, ts.skip_count, ts.last_played_at,
        ts.manual_like, 0 as is_online, NULL as preview_url
 FROM tracks t
 LEFT JOIN artists a ON a.id = t.artist_id
 LEFT JOIN albums al ON al.id = t.album_id
 LEFT JOIN genres g ON g.id = t.genre_id
 LEFT JOIN track_statistics ts ON ts.track_id = t.id AND ts.user_id = ?
 WHERE t.format != 'online' AND t.file_path NOT LIKE 'online://%'";

const ONLINE_CANDIDATES: &str = "SELECT e.id, e.title, a.id as artist_id, e.artist as artist_name,
        t.album_id, e.album as album_title, COALESCE(g.name, e.genre) as genre_name, t.year,
        COALESCE(e.duration_secs, t.duration_secs, 210.0) as duration_secs,
        e.cover_art_url, ts.play_count, ts.completion_count, ts.skip_count, ts.last_played_at,
        CASE WHEN ts.manual_like = -1 THEN -1
             WHEN liked.track_id IS NOT NULL THEN 1 ELSE ts.manual_like END as manual_like,
        1 as is_online, e.preview_url
 FROM external_tracks e
 LEFT JOIN tracks t ON t.id = e.id
 LEFT JOIN artists a ON a.normalized_name = lower(trim(e.artist))
 LEFT JOIN genres g ON g.id = t.genre_id
 LEFT JOIN track_statistics ts ON ts.track_id = e.id AND ts.user_id = ?
 LEFT JOIN (
    SELECT DISTINCT pt.track_id FROM playlist_tracks pt
    JOIN playlists p ON p.id = pt.playlist_id
    WHERE p.user_id = ? AND p.name = 'Liked Songs' AND p.is_smart_mix = 0
 ) liked ON liked.track_id = e.id
 WHERE (e.provider = 'jiosaavn' OR EXISTS (SELECT 1 FROM online_track_sources source WHERE source.track_id=e.id)) AND (t.id IS NULL OR t.format = 'online' OR t.file_path LIKE 'online://%')
   AND NOT EXISTS (
       SELECT 1 FROM tracks local WHERE local.id = e.matched_local_track_id
         AND local.format != 'online' AND local.file_path NOT LIKE 'online://%'
   )
 ORDER BY (liked.track_id IS NOT NULL OR COALESCE(ts.play_count, 0) > 0) DESC,
          COALESCE(ts.play_count, 0) DESC, e.created_at DESC, e.id
 LIMIT 2000";

impl FlowEngine {
    /// Library songs, plus (for playback Flow) online songs that fit the user's taste.
    pub(super) async fn candidate_rows(
        &self,
        user_id: &str,
        inputs: &FlowInputs,
        include_online: bool,
        session: Option<&SessionSteer>,
    ) -> AppResult<Vec<FlowCandidateRow>> {
        let mut candidates = sqlx::query_as::<_, FlowCandidateRow>(LOCAL_CANDIDATES)
            .bind(user_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
        if include_online {
            let online = sqlx::query_as::<_, FlowCandidateRow>(ONLINE_CANDIDATES)
                .bind(user_id)
                .bind(user_id)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
            candidates.extend(
                online
                    .into_iter()
                    .map(|c| with_signal(c, &inputs.signals))
                    .filter(|c| online_is_relevant(c, inputs, session)),
            );
        }
        Ok(candidates)
    }
}

/// Copies the user's listening signal (plays, skips, like, last play) onto a row.
pub(super) fn with_signal(mut c: FlowCandidateRow, signals: &SongSignals) -> FlowCandidateRow {
    let signal = signals.signal(&c.id, &c.title, artist_of(&c));
    c.play_count = Some(signal.play_count);
    c.completion_count = Some(signal.completion_count);
    c.skip_count = Some(signal.skip_count);
    c.manual_like = Some(signal.manual_like);
    c.last_played_at = signal.last_played_at;
    c
}

pub(super) fn artist_of(c: &FlowCandidateRow) -> &str {
    c.artist_name.as_deref().unwrap_or("")
}

/// An online song qualifies when it is known, or matches the user's artists,
/// similar artists, genres, languages or starters, and its style isn't blocked
/// by repeated skips in this session.
fn online_is_relevant(c: &FlowCandidateRow, inputs: &FlowInputs, session: Option<&SessionSteer>) -> bool {
    let profile = &inputs.profile;
    let known = c.manual_like == Some(1) || c.last_played_at.is_some() || c.play_count.unwrap_or(0) > 0;
    let artist_match = profile.artist_affinity(artist_of(c)) > 0.15;
    let language_match = inputs
        .signals
        .identities
        .get(&c.id)
        .and_then(|song| song.language.as_ref())
        .and_then(|language| profile.language_affinities.get(&language.to_lowercase()))
        .copied()
        .unwrap_or(0.0)
        > 0.15;
    let genre_match = c.genre_name.as_ref().is_some_and(|genre| {
        profile
            .genre_affinities
            .iter()
            .any(|(name, affinity)| name.eq_ignore_ascii_case(genre.trim()) && *affinity > 0.15)
    });
    // A style skipped repeatedly this session gets no online exploration.
    let session_blocked = !known
        && session.is_some_and(|steer| steer.genre_score(c.genre_name.as_deref()) <= session::BLOCK_GENRE_SCORE);
    // "Fans of your artists also like" reaches beyond tag overlap.
    let similar_match = inputs.similarity.boost(artist_of(c)) > SIMILAR_CANDIDATE_MIN;
    !session_blocked
        && (known || artist_match || similar_match || genre_match || language_match || inputs.starters.contains_key(&c.id))
}

/// Songs never to pick: excluded ids and dislikes, matched by title and artist.
pub(super) fn excluded_song_keys(candidates: &[FlowCandidateRow], exclude: &HashSet<&str>) -> HashSet<String> {
    candidates
        .iter()
        .filter(|c| exclude.contains(c.id.as_str()) || c.manual_like == Some(-1))
        .map(|c| FlowEngine::song_key(&c.title, artist_of(c)))
        .collect()
}

/// Keeps one online release per song: the best-ranked (original over compilation).
pub(super) fn prefer_best_release(candidates: &mut Vec<FlowCandidateRow>, signals: &SongSignals) {
    let mut preferred: HashMap<String, ((bool, u8, i64), String)> = HashMap::new();
    for candidate in candidates.iter().filter(|c| c.is_online) {
        let key = FlowEngine::song_key(&candidate.title, artist_of(candidate));
        let year = signals.identities.get(&candidate.id).and_then(|song| song.year).or(candidate.year);
        let rank = signals.release_rank(&candidate.id, &candidate.title, candidate.album_title.as_deref(), year);
        if preferred.get(&key).is_none_or(|(old, _)| rank > *old) {
            preferred.insert(key, (rank, candidate.id.clone()));
        }
    }
    candidates.retain(|candidate| {
        !candidate.is_online
            || preferred
                .get(&FlowEngine::song_key(&candidate.title, artist_of(candidate)))
                .is_none_or(|(_, id)| id == &candidate.id)
    });
}
