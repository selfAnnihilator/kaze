//! Track info lookup and selection helpers for MPRIS.
use super::*;

/// Fetches enriched track info from database when a track starts playing.
pub(super) async fn fetch_track_info(
    processor: &Arc<CoreProcessor>,
    track_id: &str,
    title: &str,
    artist: &str,
    duration_secs: f64,
) -> MprisTrackInfo {
    let mut info = MprisTrackInfo {
        track_id: track_id.to_string(),
        title: title.to_string(),
        artist: artist.to_string(),
        album: None,
        album_artist: None,
        duration_secs,
        cover_art: None,
        track_number: None,
        url: None,
    };

    // Query tracks table
    type TrackRow = (String, Option<String>, Option<String>, Option<String>, Option<String>, Option<i64>, Option<String>, f64);
    let track_row: Option<TrackRow> = sqlx::query_as(
        "SELECT 
            t.title,
            a.name AS artist_name,
            al.title AS album_title,
            COALESCE(al.cover_art_path, (SELECT e.cover_art_path FROM track_enrichment e JOIN track_source_metadata s ON s.track_id=e.track_id WHERE e.track_id=t.id AND e.state='matched' AND json_extract(s.metadata_json, '$.has_cover_art') = 0)),
            ext.cover_art_url,
            t.track_number,
            t.file_path,
            t.duration_secs
         FROM tracks t
         LEFT JOIN artists a ON a.id = t.artist_id
         LEFT JOIN albums al ON al.id = t.album_id
         LEFT JOIN external_tracks ext ON ext.id = t.id
         WHERE t.id = ?
         LIMIT 1",
    )
    .bind(track_id)
    .fetch_optional(processor.db_pool())
    .await
    .ok()
    .flatten();

    if let Some((
        t_title,
        a_name,
        album_title,
        cover_art_path,
        cover_art_url,
        track_number,
        file_path,
        d_secs,
    )) = track_row
    {
        if info.title.is_empty() {
            info.title = t_title;
        }
        if info.artist.is_empty() {
            info.artist = a_name.unwrap_or_else(|| "Unknown Artist".to_string());
        }
        if info.duration_secs <= 0.0 {
            info.duration_secs = d_secs;
        }
        info.album = album_title;
        info.cover_art = cover_art_path.or(cover_art_url);
        info.track_number = track_number.map(|n| n as i32);
        info.url = file_path;
        return info;
    }

    // Fallback: query external_tracks table
    type ExtRow = (Option<String>, Option<String>, Option<String>, Option<String>, Option<f64>);
    let ext_row: Option<ExtRow> = sqlx::query_as(
        "SELECT title, artist, album, cover_art_url, duration_secs FROM external_tracks WHERE id = ? LIMIT 1",
    )
    .bind(track_id)
    .fetch_optional(processor.db_pool())
    .await
    .ok()
    .flatten();

    if let Some((e_title, e_artist, album, cover_art_url, e_dur)) = ext_row {
        if info.title.is_empty() {
            if let Some(t) = e_title {
                info.title = t;
            }
        }
        if info.artist.is_empty() {
            if let Some(a) = e_artist {
                info.artist = a;
            }
        }
        if info.duration_secs <= 0.0 {
            if let Some(d) = e_dur {
                info.duration_secs = d;
            }
        }
        info.album = album;
        info.cover_art = cover_art_url;
    }

    info
}

pub(super) fn can_seek(state: &PlaybackStateDto) -> bool {
    state.can_seek
        && state.current_track_id.is_some()
        && state.duration_secs > 0.0
        && (state.is_playing || state.is_paused)
}

/// A stopped queue still has a real selection that Play will start. History and
/// arbitrary library entries are not current tracks and must not become metadata.
pub(super) fn selected_track_id(state: &PlaybackStateDto) -> Option<&str> {
    state.current_track_id.as_deref().or_else(|| {
        state
            .queue_track_ids
            .get(state.current_queue_index.unwrap_or(0))
            .map(String::as_str)
    })
}
