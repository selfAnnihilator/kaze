//! Registering online songs (and their artist/album) as local track rows.
use super::*;

/// Metadata needed to register an online track (and its artist/album) locally.
#[derive(Debug, Clone, Copy)]
pub struct OnlineTrackInput<'a> {
    pub id: &'a str,
    pub title: &'a str,
    pub artist: Option<&'a str>,
    pub album: Option<&'a str>,
    pub duration_secs: Option<f64>,
    pub cover_art_url: Option<&'a str>,
    pub preview_url: Option<&'a str>,
}

pub async fn ensure_online_track(
    pool: &SqlitePool,
    track: &OnlineTrackInput<'_>) -> AppResult<()> {
    let OnlineTrackInput {
        id,
        title,
        artist,
        album,
        duration_secs,
        cover_art_url,
        preview_url,
    } = *track;
    let now = chrono::Utc::now().timestamp();

    // 1. Ensure artist exists if given
    let artist_id = if let Some(a_name) = artist {
        let clean = a_name.trim();
        if !clean.is_empty() {
            let normalized = clean.to_lowercase();
            if let Some((existing_id,)) =
                sqlx::query_as::<_, (String,)>("SELECT id FROM artists WHERE normalized_name = ?")
                    .bind(&normalized)
                    .fetch_optional(pool)
                    .await
                    .unwrap_or(None)
            {
                Some(existing_id)
            } else {
                let a_id = uuid::Uuid::new_v4().to_string();
                let _ = sqlx::query(
                    "INSERT INTO artists (id, name, normalized_name, created_at)
                     VALUES (?, ?, ?, ?)",
                )
                .bind(&a_id)
                .bind(clean)
                .bind(&normalized)
                .bind(now)
                .execute(pool)
                .await;
                Some(a_id)
            }
        } else {
            None
        }
    } else {
        None
    };

    // 2. Ensure album exists if given
    let album_id = if let Some(al_title) = album {
        let clean = al_title.trim();
        if !clean.is_empty() {
            let normalized = clean.to_lowercase();
            if let Some((existing_id,)) = sqlx::query_as::<_, (String,)>(
                "SELECT id FROM albums WHERE normalized_title = ? AND (artist_id IS ? OR artist_id = ?)"
            )
            .bind(&normalized)
            .bind(&artist_id)
            .bind(&artist_id)
            .fetch_optional(pool)
            .await
            .unwrap_or(None) {
                Some(existing_id)
            } else {
                let al_id = uuid::Uuid::new_v4().to_string();
                let _ = sqlx::query(
                    "INSERT INTO albums (id, title, normalized_title, artist_id, created_at)
                     VALUES (?, ?, ?, ?, ?)"
                )
                .bind(&al_id)
                .bind(clean)
                .bind(&normalized)
                .bind(&artist_id)
                .bind(now)
                .execute(pool)
                .await;
                Some(al_id)
            }
        } else {
            None
        }
    } else {
        None
    };

    // 3. Ensure track in external_tracks table
    let _ = sqlx::query(
        "INSERT INTO external_tracks (id, provider, provider_id, title, artist, album, duration_secs, cover_art_url, preview_url, match_status, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'NOT_FOUND', ?)
         ON CONFLICT(id) DO UPDATE SET
             title = excluded.title,
             artist = excluded.artist,
             provider = CASE WHEN external_tracks.provider IN ('online', 'cloud') AND excluded.provider = 'jiosaavn' THEN excluded.provider ELSE external_tracks.provider END,
             provider_id = CASE WHEN external_tracks.provider IN ('online', 'cloud') AND excluded.provider = 'jiosaavn' THEN excluded.provider_id ELSE external_tracks.provider_id END,
             album = COALESCE(excluded.album, external_tracks.album),
             duration_secs = COALESCE(excluded.duration_secs, external_tracks.duration_secs),
             cover_art_url = COALESCE(excluded.cover_art_url, external_tracks.cover_art_url),
             preview_url = CASE WHEN external_tracks.provider = 'jiosaavn' OR excluded.provider = 'jiosaavn' THEN NULL ELSE COALESCE(excluded.preview_url, external_tracks.preview_url) END"
    )
    .bind(id)
    .bind(if id.starts_with("jiosaavn:") { "jiosaavn" } else { "online" })
    .bind(id.strip_prefix("jiosaavn:").unwrap_or(id))
    .bind(title)
    .bind(artist.unwrap_or("Unknown Artist"))
    .bind(album)
    .bind(duration_secs)
    .bind(cover_art_url)
    .bind(preview_url)
    .bind(now)
    .execute(pool)
    .await?;

    // 4. Ensure or update track in tracks table
    let fake_path = format!("online://{}", id);
    let dur = match duration_secs {
        Some(d) if d > 0.0 => d,
        _ => 210.0,
    };
    let has_cov = if cover_art_url.is_some() { 1 } else { 0 };

    let _ = sqlx::query(
        "INSERT INTO tracks (id, file_path, file_size, modified_timestamp, title, normalized_title, artist_id, album_id, duration_secs, format, has_cover_art, created_at, updated_at)
         VALUES (?, ?, 0, ?, ?, ?, ?, ?, ?, 'online', ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
             title = excluded.title,
             normalized_title = excluded.normalized_title,
             artist_id = COALESCE(excluded.artist_id, tracks.artist_id),
             album_id = COALESCE(excluded.album_id, tracks.album_id),
             duration_secs = CASE WHEN excluded.duration_secs > 0 THEN excluded.duration_secs ELSE tracks.duration_secs END,
             format = 'online',
             has_cover_art = excluded.has_cover_art,
             updated_at = excluded.updated_at"
    )
    .bind(id)
    .bind(&fake_path)
    .bind(now)
    .bind(title)
    .bind(title.to_lowercase())
    .bind(&artist_id)
    .bind(&album_id)
    .bind(dur)
    .bind(has_cov)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn repair_legacy_online_tracks(pool: &SqlitePool) -> AppResult<()> {
    type LegacyOnlineRow = (String, String, String, Option<String>, Option<f64>, Option<String>, Option<String>);
    let rows: Vec<LegacyOnlineRow> =
        sqlx::query_as(
            "SELECT ext.id, ext.title, ext.artist, ext.album, ext.duration_secs, ext.cover_art_url, ext.preview_url
             FROM external_tracks ext
             JOIN tracks t ON t.id = ext.id
             WHERE t.title LIKE 'itunes:%' OR t.title LIKE 'online:%' OR t.artist_id IS NULL OR t.duration_secs = 0.0"
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    for (id, title, artist, album, duration_secs, cover_art_url, preview_url) in rows {
        let _ = ensure_online_track(
            pool,
            &OnlineTrackInput {
                id: &id,
                title: &title,
                artist: Some(&artist),
                album: album.as_deref(),
                duration_secs,
                cover_art_url: cover_art_url.as_deref(),
                preview_url: preview_url.as_deref(),
            },
        )
        .await;
    }

    // Ensure external tracks incorrectly matched against online track placeholders are reset
    let _ = sqlx::query(
        "UPDATE external_tracks SET match_status = 'NOT_FOUND', matched_local_track_id = NULL
         WHERE matched_local_track_id LIKE 'itunes:%' OR matched_local_track_id LIKE 'online:%'",
    )
    .execute(pool)
    .await;

    Ok(())
}
