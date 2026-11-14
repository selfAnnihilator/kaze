//! Processor handlers for transport, queue and now-playing data.

use super::routing::misrouted;
use super::*;

impl CoreProcessor {
    pub(super) async fn playback_command(&self, cmd: Command) -> AppResult<CommandResponse> {
        match cmd {
            Command::PlayTrack { track_id, source } => {
                self.playback_service.play_track(&track_id, source).await?;
                Ok(CommandResponse::Ok)
            }
            Command::SetArchiveShuffle { enabled } => {
                self.playback_service.set_archive_shuffle(enabled).await;
                Ok(CommandResponse::Ok)
            }
            Command::PlayTrackList { track_ids, start_track_id, shuffle, scope } => {
                let scope = LibraryScope::parse(scope.as_deref(), self.download_service.download_dir());
                self.playback_service
                    .play_track_list(track_ids, start_track_id, shuffle, &scope)
                    .await?;
                Ok(CommandResponse::Ok)
            }
            Command::PlayOnlineTrack {
                track_id,
                title,
                artist,
                album,
                duration_secs,
                cover_art_url,
                preview_url,
                source,
            } => {
                self.playback_service
                    .play_online_track(
                        &OnlineTrackInput {
                            id: &track_id,
                            title: &title,
                            artist: Some(&artist),
                            album: album.as_deref(),
                            duration_secs,
                            cover_art_url: cover_art_url.as_deref(),
                            preview_url: preview_url.as_deref(),
                        },
                        source,
                    )
                    .await?;
                Ok(CommandResponse::Ok)
            }
            Command::PlayQueueIndex { index } => {
                self.playback_service.play_queue_index(index).await?;
                Ok(CommandResponse::Ok)
            }
            Command::Pause => {
                self.playback_service.pause().await?;
                Ok(CommandResponse::Ok)
            }
            Command::Resume => {
                self.playback_service.resume().await?;
                Ok(CommandResponse::Ok)
            }
            Command::TogglePlayPause => {
                let state = self.playback_service.get_playback_state().await;
                if state.is_playing {
                    self.playback_service.pause().await?;
                } else if state.is_paused {
                    self.playback_service.resume().await?;
                } else if state.queue_length > 0 {
                    let idx = state.current_queue_index.unwrap_or(0);
                    self.playback_service.play_queue_index(idx).await?;
                } else {
                    return Err(AppError::Playback("Select a song to play".into()));
                }
                Ok(CommandResponse::Ok)
            }
            Command::Stop => {
                self.playback_service.stop().await?;
                Ok(CommandResponse::Ok)
            }
            Command::NextTrack => {
                self.playback_service.next().await?;
                Ok(CommandResponse::Ok)
            }
            Command::PreviousTrack => {
                self.playback_service.previous().await?;
                Ok(CommandResponse::Ok)
            }
            Command::Seek { position_secs } => {
                self.playback_service.seek(position_secs).await?;
                Ok(CommandResponse::Ok)
            }
            Command::SetVolume { volume } => {
                if !(0.0..=1.0).contains(&volume) {
                    return Err(AppError::Validation(
                        "Volume must be between 0.0 and 1.0".into(),
                    ));
                }
                self.playback_service.set_volume(volume).await?;
                Ok(CommandResponse::Ok)
            }
            Command::ToggleMute => {
                let state = self.playback_service.get_playback_state().await;
                let new_vol = if state.volume > 0.0 { 0.0 } else { 0.8 };
                self.playback_service.set_volume(new_vol).await?;
                Ok(CommandResponse::Ok)
            }
            Command::SetRepeatMode { mode } => {
                self.playback_service.set_repeat_mode(mode).await;
                Ok(CommandResponse::Ok)
            }
            Command::SetShuffle { enabled } => {
                self.playback_service.set_shuffle(enabled).await;
                Ok(CommandResponse::Ok)
            }
            Command::EnqueueTrack {
                track_id,
                play_next,
            } => {
                if self
                    .playback_service
                    .enqueue_track(&track_id, play_next)
                    .await?
                {
                    Ok(CommandResponse::Ok)
                } else {
                    Err(AppError::Validation("Track is already in the queue".into()))
                }
            }
            Command::EnqueueOnlineTrack {
                track_id,
                title,
                artist,
                album,
                duration_secs,
                cover_art_url,
                preview_url,
                play_next,
            } => {
                crate::recommendations::mixes::ensure_online_track(
                    &self.db_pool,
                    &OnlineTrackInput {
                        id: &track_id,
                        title: &title,
                        artist: Some(&artist),
                        album: album.as_deref(),
                        duration_secs,
                        cover_art_url: cover_art_url.as_deref(),
                        preview_url: preview_url.as_deref(),
                    },
                )
                .await?;
                if self
                    .playback_service
                    .enqueue_track(&track_id, play_next)
                    .await?
                {
                    Ok(CommandResponse::Ok)
                } else {
                    Err(AppError::Validation("Track is already in the queue".into()))
                }
            }
            Command::DequeueTrack { track_id } => {
                self.playback_service.dequeue_track(&track_id).await;
                Ok(CommandResponse::Ok)
            }
            Command::ClearQueue => {
                self.playback_service.clear_queue().await;
                Ok(CommandResponse::Ok)
            }
            Command::ClearRemoteAudioCache => {
                let (bytes_freed, files_removed) =
                    self.playback_service.clear_remote_audio_cache().await?;
                Ok(CommandResponse::RemoteAudioCacheCleared {
                    bytes_freed,
                    files_removed,
                })
            }
            other => Err(misrouted(&other)),
        }
    }

    pub(super) async fn playback_query(&self, query: Query) -> AppResult<QueryResponse> {
        match query {
            Query::GetPlaybackState => {
                self.playback_service.initialize_saved_playback().await;
                let state = self.playback_service.get_playback_state().await;
                let mut val =
                    serde_json::to_value(&state).map_err(|e| AppError::Internal(e.to_string()))?;
                if let Some(track_id) = &state.current_track_id {
                    let user_id = self.current_user_id().await;
                    if let Ok(Some(track)) = self
                        .library_service
                        .track_repo()
                        .find_by_id_scoped(track_id, Some(&user_id))
                        .await
                    {
                        if let Ok(track_val) = serde_json::to_value(track) {
                            if let Some(obj) = val.as_object_mut() {
                                obj.insert("current_track".to_string(), track_val);
                            }
                        }
                    }
                }
                Ok(QueryResponse::PlaybackState(
                    self.online_catalog.present(val).await?,
                ))
            }
            Query::GetTrackCoverArt { track_id } => Ok(QueryResponse::CoverArt(
                self.load_track_artwork(&track_id).await?,
            )),
            Query::GetTrackLyrics {
                track_id,
                artist,
                title,
                duration_secs,
            } => {
                // 1. If local track_id provided, attempt to read embedded lyrics from file tags
                if let Some(ref tid) = track_id {
                    if let Ok(Some(track)) = self.library_service.track_repo().find_by_id(tid).await
                    {
                        let path = std::path::Path::new(&track.file_path);
                        if path.exists() {
                            if let Ok(probe) = lofty::probe::Probe::open(path) {
                                if let Ok(probe) = probe.guess_file_type() {
                                    if let Ok(tagged_file) = probe.read() {
                                        if let Some(tag) = tagged_file
                                            .primary_tag()
                                            .or_else(|| tagged_file.first_tag())
                                        {
                                            if let Some(lyrics_val) =
                                                tag.get_string(&lofty::tag::ItemKey::Lyrics)
                                            {
                                                let is_synced = lyrics_val.contains('[')
                                                    && lyrics_val.contains(']');
                                                let payload = serde_json::json!({
                                                    "trackName": title,
                                                    "artistName": artist,
                                                    "plainLyrics": if is_synced { None } else { Some(lyrics_val) },
                                                    "syncedLyrics": if is_synced { Some(lyrics_val) } else { None },
                                                });
                                                return Ok(QueryResponse::Lyrics(Some(payload)));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 2. Query LRCLIB API for time-synced or plain lyrics
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(6))
                    .build()
                    .unwrap_or_default();

                let mut params = vec![
                    ("artist_name", artist.clone()),
                    ("track_name", title.clone()),
                ];
                let dur_str;
                if let Some(d) = duration_secs {
                    if d > 0.0 {
                        dur_str = format!("{}", d.round() as u64);
                        params.push(("duration", dur_str));
                    }
                }

                match client
                    .get("https://lrclib.net/api/get")
                    .query(&params)
                    .send()
                    .await
                {
                    Ok(resp) if resp.status().is_success() => {
                        if let Ok(json) = resp.json::<serde_json::Value>().await {
                            let plain = json
                                .get("plainLyrics")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            let synced = json
                                .get("syncedLyrics")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            let payload = serde_json::json!({
                                "trackName": title,
                                "artistName": artist,
                                "plainLyrics": plain,
                                "syncedLyrics": synced,
                            });
                            return Ok(QueryResponse::Lyrics(Some(payload)));
                        }
                    }
                    _ => {}
                }

                Ok(QueryResponse::Lyrics(None))
            }
            Query::GetRemoteAudioCacheStats => {
                let stats = self.playback_service.get_remote_audio_cache_stats().await?;
                Ok(QueryResponse::RemoteAudioCacheStats {
                    total_size_bytes: stats.total_size_bytes,
                    file_count: stats.file_count,
                    max_size_bytes: stats.max_size_bytes,
                    partial_file_count: stats.partial_file_count,
                })
            }
            other => Err(misrouted(&other)),
        }
    }
}
