//! Processor handlers for playlists, smart mixes, Liked Songs and song feedback.

use super::routing::misrouted;
use super::*;

impl CoreProcessor {
    pub(super) async fn collections_command(&self, cmd: Command) -> AppResult<CommandResponse> {
        match cmd {
            Command::LikeTrack { track_id } => {
                self.set_song_feedback(&track_id, 1).await?;
                Ok(CommandResponse::Ok)
            }
            Command::DislikeTrack { track_id } => {
                self.set_song_feedback(&track_id, -1).await?;
                Ok(CommandResponse::Ok)
            }
            Command::RemoveTrackFeedback { track_id } => {
                self.set_song_feedback(&track_id, 0).await?;
                Ok(CommandResponse::Ok)
            }
            Command::EnsureLikedSongsPlaylist => {
                let user_id = self.current_user_id().await;
                let id = self
                    .playlist_repo
                    .ensure_liked_songs_playlist(&user_id)
                    .await?;
                Ok(CommandResponse::EntityId(id))
            }
            Command::CreatePlaylist { name, description } => {
                let trimmed_name = name.trim().to_string();
                if trimmed_name.is_empty() {
                    return Err(AppError::Validation("Playlist name cannot be empty".into()));
                }

                let current_user_guard = self.current_user.read().await;
                let user_id = current_user_guard
                    .as_ref()
                    .map(|u| u.id.clone())
                    .unwrap_or_else(|| "default".to_string());
                drop(current_user_guard);

                if self
                    .playlist_repo
                    .find_by_name_and_user(&trimmed_name, &user_id)
                    .await?
                    .is_some()
                {
                    return Err(AppError::Validation(format!(
                        "A playlist named '{}' already exists",
                        trimmed_name
                    )));
                }

                let now = chrono::Utc::now().timestamp();
                let id = format!("pl_{}", uuid::Uuid::new_v4());
                let record = PlaylistRecord {
                    id: id.clone(),
                    name: trimmed_name,
                    description,
                    is_smart_mix: 0,
                    mix_type: None,
                    generation_reason: None,
                    expires_at: None,
                    created_at: now,
                    updated_at: now,
                };
                self.playlist_repo
                    .create_playlist_with_user(&record, &user_id)
                    .await?;

                // Background sync if user is logged in
                self.trigger_background_sync().await;

                Ok(CommandResponse::EntityId(id))
            }
            Command::DeletePlaylist { playlist_id } => {
                let user_id = {
                    let current_user_guard = self.current_user.read().await;
                    current_user_guard.as_ref().map(|u| u.id.clone())
                };
                let Some(ref uid) = user_id else {
                    return Err(AppError::Validation("Sign in to delete playlists.".into()));
                };
                let owned: Option<(String,)> = sqlx::query_as(
                    "SELECT name FROM playlists WHERE id = ? AND user_id = ? AND is_smart_mix = 0",
                )
                .bind(&playlist_id)
                .bind(uid)
                .fetch_optional(&self.db_pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
                let Some((playlist_name,)) = owned else {
                    return Err(AppError::Validation(
                        "Playlist not found or not owned by this account.".into(),
                    ));
                };
                if playlist_name == "Liked Songs" {
                    return Err(AppError::Validation(
                        "The Liked Songs playlist cannot be deleted.".into(),
                    ));
                }
                if let Some(ref uid) = user_id {
                    let _ =
                        SyncManager::record_tombstone(&self.db_pool, uid, "playlist", &playlist_id)
                            .await;
                }
                self.playlist_repo.delete_playlist(&playlist_id).await?;
                self.trigger_background_sync().await;
                Ok(CommandResponse::Ok)
            }
            Command::RenamePlaylist { playlist_id, name } => {
                let trimmed = name.trim().to_string();
                if trimmed.is_empty() {
                    return Err(AppError::Validation("Playlist name cannot be empty".into()));
                }
                let user_id = {
                    let current_user_guard = self.current_user.read().await;
                    current_user_guard.as_ref().map(|u| u.id.clone())
                };
                let Some(uid) = user_id else {
                    return Err(AppError::Validation("Sign in to rename playlists.".into()));
                };
                let owned: Option<(String,)> = sqlx::query_as(
                    "SELECT name FROM playlists WHERE id = ? AND user_id = ? AND is_smart_mix = 0",
                )
                .bind(&playlist_id)
                .bind(&uid)
                .fetch_optional(&self.db_pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
                let Some((playlist_name,)) = owned else {
                    return Err(AppError::Validation(
                        "Playlist not found or not owned by this account.".into(),
                    ));
                };
                if playlist_name == "Liked Songs" {
                    return Err(AppError::Validation(
                        "The Liked Songs playlist cannot be renamed.".into(),
                    ));
                }
                if let Some(existing) = self
                    .playlist_repo
                    .find_by_name_and_user(&trimmed, &uid)
                    .await?
                {
                    if existing.id != playlist_id {
                        return Err(AppError::Validation(format!(
                            "A playlist named '{}' already exists",
                            trimmed
                        )));
                    }
                }
                self.playlist_repo
                    .rename_playlist(&playlist_id, &trimmed)
                    .await?;
                self.trigger_background_sync().await;
                Ok(CommandResponse::Ok)
            }
            Command::AddTrackToPlaylist {
                playlist_id,
                track_id,
                title,
                artist,
                album,
                duration_secs,
                cover_art_url,
                preview_url,
            } => {
                let track_opt = self
                    .library_service
                    .track_repo()
                    .find_by_id(&track_id)
                    .await?;
                let needs_online_ensure = match &track_opt {
                    None => true,
                    Some(t) => {
                        t.title.starts_with("itunes:")
                            || t.title.starts_with("online:")
                            || t.format == "online"
                    }
                };

                if needs_online_ensure {
                    let is_clean_title = match &title {
                        Some(t) => {
                            !t.trim().is_empty()
                                && !t.starts_with("itunes:")
                                && !t.starts_with("online:")
                        }
                        None => false,
                    };

                    let (t_title, t_artist, t_album, t_dur, t_cover, t_prev) = if is_clean_title {
                        (
                            title.unwrap_or_default(),
                            artist,
                            album,
                            duration_secs,
                            cover_art_url,
                            preview_url,
                        )
                    } else {
                        type ExtRow = (String, String, Option<String>, Option<f64>, Option<String>, Option<String>);
                        let ext_opt: Option<ExtRow> = sqlx::query_as(
                            "SELECT title, artist, album, duration_secs, cover_art_url, preview_url FROM external_tracks WHERE id = ?"
                        )
                        .bind(&track_id)
                        .fetch_optional(&self.db_pool)
                        .await
                        .ok()
                        .flatten();

                        if let Some((e_title, e_artist, e_album, e_dur, e_cov, e_prev)) = ext_opt {
                            (e_title, Some(e_artist), e_album, e_dur, e_cov, e_prev)
                        } else {
                            (
                                title.unwrap_or_else(|| track_id.clone()),
                                artist,
                                album,
                                duration_secs,
                                cover_art_url,
                                preview_url,
                            )
                        }
                    };

                    let _ = crate::recommendations::mixes::ensure_online_track(
                        &self.db_pool,
                        &OnlineTrackInput {
                            id: &track_id,
                            title: if t_title.is_empty() {
                            &track_id
                        } else {
                            &t_title
                        },
                            artist: t_artist.as_deref(),
                            album: t_album.as_deref(),
                            duration_secs: t_dur,
                            cover_art_url: t_cover.as_deref(),
                            preview_url: t_prev.as_deref(),
                        },
                    )
                    .await;
                }
                self.playlist_repo
                    .add_track(&playlist_id, &track_id, None)
                    .await?;
                self.trigger_background_sync().await;
                Ok(CommandResponse::Ok)
            }
            Command::RemoveTrackFromPlaylist {
                playlist_id,
                track_id,
            } => {
                let user_id = {
                    let current_user_guard = self.current_user.read().await;
                    current_user_guard.as_ref().map(|u| u.id.clone())
                };
                if let Some(ref uid) = user_id {
                    let entity_id = format!("{}:{}", playlist_id, track_id);
                    let _ = SyncManager::record_tombstone(
                        &self.db_pool,
                        uid,
                        "playlist_song",
                        &entity_id,
                    )
                    .await;
                }
                self.playlist_repo
                    .remove_track(&playlist_id, &track_id)
                    .await?;
                self.trigger_background_sync().await;
                Ok(CommandResponse::Ok)
            }
            other => Err(misrouted(&other)),
        }
    }

    pub(super) async fn collections_query(&self, query: Query) -> AppResult<QueryResponse> {
        match query {
            Query::GetSmartMixes | Query::RefreshSmartMixes => {
                if matches!(query, Query::RefreshSmartMixes) {
                    sqlx::query(
                        "UPDATE playlists SET expires_at=0 WHERE user_id=? AND is_smart_mix=1",
                    )
                    .bind(self.current_user_id().await)
                    .execute(&self.db_pool)
                    .await?;
                }
                let user_id = self.current_user_id().await;
                self.discovery_coordinator
                    .ensure_starter_candidates(&user_id, false)
                    .await?;
                let mixes = self
                    .smart_mix_generator
                    .ensure_default_mixes(&user_id)
                    .await?;
                let mut val: Vec<serde_json::Value> = Vec::new();
                for m in mixes {
                    let count = self.playlist_repo.get_track_count(&m.id).await.unwrap_or(0);
                    if let Ok(mut v) = serde_json::to_value(&m) {
                        if let Some(obj) = v.as_object_mut() {
                            obj.insert("track_count".to_string(), serde_json::json!(count));
                        }
                        val.push(v);
                    }
                }
                Ok(QueryResponse::SmartMixes(val))
            }
            Query::GetPlaylists => {
                let is_logged_in = self.current_user.read().await.is_some();
                let user_id = self.current_user_id().await;
                self.discovery_coordinator
                    .ensure_starter_candidates(&user_id, false)
                    .await?;
                let _ = self
                    .smart_mix_generator
                    .ensure_default_mixes(&user_id)
                    .await;
                if is_logged_in {
                    self.playlist_repo
                        .ensure_liked_songs_playlist(&user_id)
                        .await?;
                }
                let raw_playlists = self.playlist_repo.get_user_playlists(&user_id).await?;
                let playlists: Vec<_> = if is_logged_in {
                    raw_playlists
                } else {
                    raw_playlists
                        .into_iter()
                        .filter(|p| p.is_smart_mix == 1)
                        .collect()
                };
                let mut val: Vec<serde_json::Value> = Vec::new();
                for p in playlists {
                    let count = self.playlist_repo.get_track_count(&p.id).await.unwrap_or(0);

                    // Query the first track in the playlist to use its thumbnail/cover
                    type FirstSongRow = (String, Option<String>, Option<String>, i64, Option<String>);
                    let first_song: Option<FirstSongRow> = sqlx::query_as(
                        "SELECT pt.track_id, 
                                ext.cover_art_url, 
                                al.cover_art_path, 
                                COALESCE(t.has_cover_art, 0),
                                t.file_path
                         FROM playlist_tracks pt
                         LEFT JOIN tracks t ON t.id = pt.track_id
                         LEFT JOIN albums al ON al.id = t.album_id
                         LEFT JOIN external_tracks ext ON ext.id = pt.track_id
                         WHERE pt.playlist_id = ?
                         ORDER BY pt.position ASC
                         LIMIT 1",
                    )
                    .bind(&p.id)
                    .fetch_optional(&self.db_pool)
                    .await
                    .ok()
                    .flatten();

                    if let Ok(mut v) = serde_json::to_value(&p) {
                        if let Some(obj) = v.as_object_mut() {
                            obj.insert("track_count".to_string(), serde_json::json!(count));

                            if let Some((
                                track_id,
                                ext_cover,
                                album_cover,
                                _has_cover,
                                _file_path_opt,
                            )) = first_song
                            {
                                obj.insert(
                                    "first_track_id".to_string(),
                                    serde_json::json!(&track_id),
                                );

                                let mut resolved_cover = ext_cover.or(album_cover);

                                if resolved_cover.is_none() {
                                    resolved_cover = self.load_track_artwork(&track_id).await?;
                                }

                                if let Some(cover) = resolved_cover {
                                    obj.insert(
                                        "cover_art_url".to_string(),
                                        serde_json::json!(cover),
                                    );
                                }
                            }
                        }
                        val.push(v);
                    }
                }
                Ok(QueryResponse::Playlists(val))
            }
            Query::GetPlaylistTracks { playlist_id } => {
                let _ =
                    crate::recommendations::mixes::repair_legacy_online_tracks(&self.db_pool).await;
                let tracks = self.playlist_repo.get_playlist_tracks(&playlist_id).await?;
                let val: Vec<serde_json::Value> = tracks
                    .into_iter()
                    .filter_map(|t| serde_json::to_value(t).ok())
                    .collect();
                let smart: bool =
                    sqlx::query_scalar("SELECT is_smart_mix=1 FROM playlists WHERE id=?")
                        .bind(&playlist_id)
                        .fetch_optional(&self.db_pool)
                        .await?
                        .unwrap_or(false);
                let value = if smart {
                    self.present_recommendations(
                        serde_json::Value::Array(val),
                        &self.current_user_id().await,
                    )
                    .await?
                } else {
                    self.online_catalog
                        .present(serde_json::Value::Array(val))
                        .await?
                };
                Ok(QueryResponse::PlaylistTracks(
                    value.as_array().cloned().unwrap_or_default(),
                ))
            }
            Query::GetTrackPlaylistMemberships => {
                let user_id = self.current_user_id().await;
                let memberships = self
                    .playlist_repo
                    .get_track_playlist_memberships_for_user(&user_id)
                    .await?;
                let val = serde_json::to_value(&memberships)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                Ok(QueryResponse::TrackPlaylistMemberships(val))
            }
            other => Err(misrouted(&other)),
        }
    }
}
