use crate::core::{error::AppResult, AppError};
use crate::database::models::ExternalTrackRecord;
use crate::recommendations::setup::Selection;
use sqlx::SqlitePool;

type SetupRow = (String, String, String, String, i64);
const LANGUAGE_KEY_PREFIX: &str = "recommendation_setup.languages.";

pub(crate) async fn skip_missing(pool: &SqlitePool, user: &str) -> AppResult<()> {
    sqlx::query("INSERT INTO recommendation_setup(user_id,status,artists_json,genres_json,updated_at) VALUES(?,'skipped','[]','[]',?) ON CONFLICT(user_id) DO NOTHING")
        .bind(user).bind(chrono::Utc::now().timestamp()).execute(pool).await?;
    Ok(())
}

pub(crate) async fn catalog_in_language(
    pool: &SqlitePool,
    language: &str,
) -> AppResult<Vec<ExternalTrackRecord>> {
    Ok(sqlx::query_as(
        "SELECT e.id,e.provider,e.provider_id,e.title,e.artist,e.album,e.duration_secs,e.cover_art_url,e.preview_url,e.genre,e.match_status,e.matched_local_track_id,e.created_at
         FROM external_tracks e WHERE lower(trim(e.language))=? AND e.provider='jiosaavn'
         ORDER BY e.created_at DESC,e.id LIMIT 15",
    ).bind(language).fetch_all(pool).await?)
}

pub(crate) async fn starter_catalog(
    pool: &SqlitePool,
    user: &str,
) -> AppResult<Vec<ExternalTrackRecord>> {
    Ok(sqlx::query_as(
        "SELECT e.id,e.provider,e.provider_id,e.title,e.artist,e.album,e.duration_secs,e.cover_art_url,e.preview_url,e.genre,e.match_status,e.matched_local_track_id,e.created_at
         FROM recommendation_starter_tracks s JOIN external_tracks e ON e.id=s.track_id
         JOIN recommendation_setup setup ON setup.user_id=s.user_id AND setup.revision=s.revision
         WHERE s.user_id=?",
    ).bind(user).fetch_all(pool).await?)
}

pub(crate) async fn load(pool: &SqlitePool, user: &str) -> AppResult<Option<SetupRow>> {
    Ok(sqlx::query_as(
        "SELECT setup.status, setup.artists_json, setup.genres_json, COALESCE(settings.value,'[]'), setup.revision
         FROM recommendation_setup setup LEFT JOIN application_settings settings ON settings.key=?
         WHERE setup.user_id=?",
    )
    .bind(format!("{LANGUAGE_KEY_PREFIX}{user}"))
    .bind(user)
    .fetch_optional(pool)
    .await?)
}

pub(crate) async fn save(pool: &SqlitePool, user: &str, selection: Selection) -> AppResult<()> {
    let status = if selection.skipped {
        "skipped"
    } else {
        "completed"
    };
    let now = chrono::Utc::now().timestamp();
    let json = |values| {
        serde_json::to_string(values).map_err(|error| AppError::Internal(error.to_string()))
    };
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO recommendation_setup(user_id,status,artists_json,genres_json,revision,updated_at) VALUES(?,?,?,?,1,?) ON CONFLICT(user_id) DO UPDATE SET status=excluded.status,artists_json=excluded.artists_json,genres_json=excluded.genres_json,revision=revision+1,updated_at=excluded.updated_at")
        .bind(user).bind(status).bind(json(&selection.artists)?).bind(json(&selection.genres)?).bind(now).execute(&mut *tx).await?;
    if let Some(languages) = selection.languages {
        sqlx::query("INSERT INTO application_settings(key,value,updated_at) VALUES(?,?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=excluded.updated_at")
            .bind(format!("{LANGUAGE_KEY_PREFIX}{user}")).bind(json(&languages)?).bind(now).execute(&mut *tx).await?;
    }
    sqlx::query("DELETE FROM recommendation_starter_tracks WHERE user_id=?")
        .bind(user)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) type StarterTracks = std::collections::HashMap<String, (String, String)>;

pub(crate) async fn save_starter_tracks(
    pool: &SqlitePool,
    user: &str,
    revision: i64,
    chosen: StarterTracks,
) -> AppResult<()> {
    let now = chrono::Utc::now().timestamp();
    let mut tx = pool.begin().await?;
    if !chosen.is_empty() {
        sqlx::query("DELETE FROM recommendation_starter_tracks WHERE user_id=? AND revision=? AND COALESCE((SELECT revision FROM recommendation_setup WHERE user_id=?),0)=?")
            .bind(user).bind(revision).bind(user).bind(revision).execute(&mut *tx).await?;
    }
    // Never let a slow request from an older saved choice refill the new pool.
    let mut chosen: Vec<_> = chosen.into_iter().collect();
    chosen.sort_by(|a, b| {
        let priority = |source: &str| match source {
            "artist" => 0,
            "genre" => 1,
            _ => 2,
        };
        priority(&a.1 .0)
            .cmp(&priority(&b.1 .0))
            .then_with(|| a.0.cmp(&b.0))
    });
    for (id, (source, value)) in chosen.into_iter().take(250) {
        sqlx::query("INSERT INTO recommendation_starter_tracks(user_id,track_id,source,seed_value,revision,created_at) SELECT ?,?,?,?,?,? WHERE COALESCE((SELECT revision FROM recommendation_setup WHERE user_id=?),0)=? ON CONFLICT(user_id,track_id) DO UPDATE SET source=excluded.source,seed_value=excluded.seed_value,revision=excluded.revision,created_at=excluded.created_at")
            .bind(user).bind(id).bind(source).bind(value).bind(revision).bind(now).bind(user).bind(revision).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod returning_listener_tests {
    use super::*;

    #[tokio::test]
    async fn returning_listener_does_not_prompt_or_lose_saved_taste() {
        let pool = crate::database::create_in_memory_pool().await.unwrap();
        skip_missing(&pool, "returning").await.unwrap();
        assert_eq!(
            crate::recommendations::setup::load(&pool, "returning")
                .await
                .unwrap()
                .status,
            "skipped"
        );
        assert_eq!(
            crate::recommendations::setup::load(&pool, "new")
                .await
                .unwrap()
                .status,
            "pending"
        );
        save(
            &pool,
            "returning",
            Selection {
                artists: vec![],
                genres: vec!["Folk".into()],
                languages: Some(vec!["Malayalam".into()]),
                skipped: false,
            },
        )
        .await
        .unwrap();
        skip_missing(&pool, "returning").await.unwrap();
        let profile = crate::recommendations::setup::load(&pool, "returning")
            .await
            .unwrap();
        assert_eq!(profile.status, "completed");
        assert_eq!(profile.languages, ["Malayalam"]);
        assert_eq!(profile.genres, ["Folk"]);
    }
}
