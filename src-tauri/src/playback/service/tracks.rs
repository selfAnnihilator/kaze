//! Finding a track to play or queue, registering online songs known only from discovery.
use super::*;
use crate::database::repositories::track_repo::TrackDetail;

impl PlaybackService {
    /// The library track, or an online song from `external_tracks` registered on the fly.
    pub(super) async fn find_or_register_track(&self, track_id: &str) -> AppResult<TrackDetail> {
        let track = match self.track_repo.find_by_id(track_id).await? {
            Some(t) => t,
            None => {
                // If not in track_repo, check external_tracks for online / discovery track
                if let Some(pool) = &self.pool {
                    type ExtRow = (String, String, Option<String>, Option<f64>, Option<String>, Option<String>);
                    let ext: Option<ExtRow> = sqlx::query_as(
                        "SELECT title, artist, album, duration_secs, cover_art_url, preview_url FROM external_tracks WHERE id = ?"
                    )
                    .bind(track_id)
                    .fetch_optional(pool)
                    .await
                    .unwrap_or(None);

                    if let Some((title, artist, album, duration_secs, cover_art_url, preview_url)) =
                        ext
                    {
                        let _ = crate::recommendations::mixes::ensure_online_track(
                            pool,
                            &OnlineTrackInput {
                                id: track_id,
                                title: &title,
                                artist: Some(&artist),
                                album: album.as_deref(),
                                duration_secs,
                                cover_art_url: cover_art_url.as_deref(),
                                preview_url: preview_url.as_deref(),
                            },
                        )
                        .await;
                        self.track_repo.find_by_id(track_id).await?.ok_or_else(|| {
                            AppError::NotFound(format!(
                                "Track not found after registration: {}",
                                track_id
                            ))
                        })?
                    } else {
                        return Err(AppError::NotFound(format!("Track not found: {}", track_id)));
                    }
                } else {
                    return Err(AppError::NotFound(format!("Track not found: {}", track_id)));
                }
            }
        };
        Ok(track)
    }
}
