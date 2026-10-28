//! Restore stable cloud song IDs without claiming a provider pair twice.
use super::models::{CloudOnlineSource, CloudSong};
use crate::{
    core::{AppError, AppResult},
    database::models::ExternalTrackRecord,
    providers::ExternalTrackMetadata,
    recommendations::mixes::ensure_online_track,
};
use sqlx::{Sqlite, SqlitePool, Transaction};

fn source_from_song(song: &CloudSong, provider_id: &str, now: i64) -> CloudOnlineSource {
    CloudOnlineSource {
        provider_id: provider_id.into(),
        linked_at: now,
        metadata: ExternalTrackMetadata {
            provider: "jiosaavn".into(),
            provider_track_id: provider_id.into(),
            title: song.title.clone(),
            artist_name: song
                .artist
                .clone()
                .unwrap_or_else(|| "Unknown Artist".into()),
            album_title: song.album.clone(),
            duration_secs: Some(song.duration_secs),
            cover_art_url: song.cover_art_url.clone(),
            artist_id: None,
            album_id: None,
            year: None,
            track_number: None,
            language: None,
        },
    }
}

async fn save_source(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
    source: &CloudOnlineSource,
) -> AppResult<()> {
    source.validate()?;
    let snapshot =
        serde_json::to_string(&source.metadata).map_err(|e| AppError::Internal(e.to_string()))?;
    sqlx::query("INSERT INTO online_track_sources(track_id,provider_id,metadata_json,linked_at) VALUES(?,?,?,?) ON CONFLICT(track_id) DO NOTHING")
        .bind(id).bind(&source.provider_id).bind(snapshot).bind(source.linked_at)
        .execute(&mut **transaction).await?;
    Ok(())
}

async fn normalize_legacy_owners(
    transaction: &mut Transaction<'_, Sqlite>,
    song: &CloudSong,
    now: i64,
) -> AppResult<()> {
    // Handle both an existing copy of this ID and a legacy owner of the incoming
    // canonical pair. Snapshot before changing either identity; never rewrite IDs.
    // Registration recognizes canonical IDs even in old payloads whose provider
    // label was omitted or rewritten to cloud/online.
    let provider_id = song.id.strip_prefix("jiosaavn:").or_else(|| {
        (song.provider.as_deref() == Some("jiosaavn"))
            .then_some(song.provider_id.as_deref())
            .flatten()
    });
    let owners: Vec<ExternalTrackRecord> = sqlx::query_as(
        "SELECT * FROM external_tracks WHERE provider='jiosaavn'
         AND id != 'jiosaavn:' || provider_id AND (id=? OR provider_id=?)",
    )
    .bind(&song.id)
    .bind(provider_id)
    .fetch_all(&mut **transaction)
    .await?;
    for owner in owners {
        let has_source: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM online_track_sources WHERE track_id=?)",
        )
        .bind(&owner.id)
        .fetch_one(&mut **transaction)
        .await?;
        if !has_source {
            if let Some(source) = song
                .online_source
                .as_ref()
                .filter(|source| owner.id == song.id && source.provider_id == owner.provider_id)
            {
                save_source(transaction, &owner.id, source).await?;
            } else {
                let (language, year): (Option<String>, Option<i64>) =
                    sqlx::query_as("SELECT language,release_year FROM external_tracks WHERE id=?")
                        .bind(&owner.id)
                        .fetch_one(&mut **transaction)
                        .await?;
                let source = CloudOnlineSource {
                    provider_id: owner.provider_id.clone(),
                    linked_at: now,
                    metadata: ExternalTrackMetadata {
                        provider: "jiosaavn".into(),
                        provider_track_id: owner.provider_id,
                        title: owner.title,
                        artist_name: owner.artist,
                        album_title: owner.album,
                        duration_secs: owner.duration_secs,
                        cover_art_url: owner.cover_art_url,
                        language,
                        year,
                        artist_id: None,
                        album_id: None,
                        track_number: None,
                    },
                };
                save_source(transaction, &owner.id, &source).await?;
            }
        }
        sqlx::query("UPDATE external_tracks SET provider='online',provider_id=id WHERE id=?")
            .bind(&owner.id)
            .execute(&mut **transaction)
            .await?;
    }
    Ok(())
}

pub(super) async fn restore_song(pool: &SqlitePool, song: &CloudSong, now: i64) -> AppResult<()> {
    let mut transaction = pool.begin().await?;
    // A cloud metadata placeholder must never rewrite a physical library file.
    let physical: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM tracks WHERE id=? AND file_path NOT LIKE 'online://%')",
    )
    .bind(&song.id)
    .fetch_one(&mut *transaction)
    .await?;
    if physical {
        return Ok(());
    }
    let mut provider = song.provider.clone().unwrap_or_else(|| "cloud".into());
    let mut provider_id = song.provider_id.clone().unwrap_or_else(|| {
        if provider == "jiosaavn" {
            song.id.strip_prefix("jiosaavn:").unwrap_or(&song.id).into()
        } else {
            song.id.clone()
        }
    });
    let source = song
        .online_source
        .clone()
        .or_else(|| (provider == "jiosaavn").then(|| source_from_song(song, &provider_id, now)));
    if let Some(source) = &source {
        source.validate()?;
    }
    normalize_legacy_owners(&mut transaction, song, now).await?;
    if provider == "jiosaavn" && song.id != format!("jiosaavn:{provider_id}") {
        provider = "online".into();
        provider_id = song.id.clone();
    }

    sqlx::query(
        "INSERT INTO external_tracks (id, provider, provider_id, title, artist, album, duration_secs, cover_art_url, preview_url, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
             provider = CASE WHEN external_tracks.provider IN ('online','cloud') AND excluded.provider = 'jiosaavn' THEN excluded.provider ELSE external_tracks.provider END,
             provider_id = CASE WHEN external_tracks.provider IN ('online','cloud') AND excluded.provider = 'jiosaavn' THEN excluded.provider_id ELSE external_tracks.provider_id END,
             title = excluded.title,
             artist = excluded.artist,
             album = excluded.album,
             duration_secs = excluded.duration_secs,
             cover_art_url = coalesce(excluded.cover_art_url, external_tracks.cover_art_url),
             preview_url = coalesce(excluded.preview_url, external_tracks.preview_url)"
    )
    .bind(&song.id)
    .bind(&provider)
    .bind(&provider_id)
    .bind(&song.title)
    .bind(song.artist.as_deref().unwrap_or("Unknown Artist"))
    .bind(song.album.as_deref().unwrap_or(""))
    .bind(song.duration_secs)
    .bind(&song.cover_art_url)
    .bind(&song.preview_url)
    .bind(song.created_at)
    .execute(&mut *transaction)
    .await?;

    if let Some(source) = &source {
        save_source(&mut transaction, &song.id, source).await?;
    }
    transaction.commit().await?;

    // The canonical provider pair is now available and every demoted alias has
    // its exact source. Registration cannot promote a non-prefixed alias again.
    ensure_online_track(
        pool,
        &song.id,
        &song.title,
        song.artist.as_deref(),
        song.album.as_deref(),
        Some(song.duration_secs),
        song.cover_art_url.as_deref(),
        song.preview_url.as_deref(),
    )
    .await?;
    Ok(())
}
