//! The playback state snapshot and the remote audio cache.
use super::*;

impl PlaybackService {
    pub async fn get_playback_state(&self) -> PlaybackStateDto {
        let restored = self.restored_track.read().await.clone();
        let (mut is_paused, mut is_finished, mut pos) = {
            let backend = self.backend.lock().await;
            (
                backend.is_paused(),
                backend.is_finished(),
                backend.position().as_secs_f64(),
            )
        };

        let mut duration = *self.current_duration_secs.read().await;
        let vol = *self.volume.read().await;
        let active_session = self.active_session_id.read().await.clone();
        let q_guard = self.queue.read().await;
        let restored = restored.filter(|saved| {
            q_guard
                .current()
                .is_some_and(|item| item.queue_id == saved.queue_id)
        });
        let has_loaded_track = restored.is_some() || active_session.as_ref().is_some_and(|(_, id)| q_guard.current().is_some_and(|item| &item.queue_id == id));
        if let Some(restored) = restored {
            is_paused = true;
            is_finished = false;
            pos = restored.position_secs;
            duration = restored.duration_secs;
        }
        let is_playing = !is_paused && !is_finished && has_loaded_track;
        let has_active_track = has_loaded_track && (!is_finished || is_paused);

        let queue_track_ids: Vec<String> = if has_active_track {
            if let Some(curr) = q_guard.current_index() {
                q_guard
                    .items()
                    .iter()
                    .skip(curr + 1)
                    .map(|i| i.track_id.clone())
                    .collect()
            } else {
                q_guard.items().iter().map(|i| i.track_id.clone()).collect()
            }
        } else {
            q_guard.items().iter().map(|i| i.track_id.clone()).collect()
        };

        let can_seek = match &*self.active_stream_state.read_or_recover() {
            Some(state) => state.can_seek(),
            None => true,
        };
        let is_buffering = self.is_resolving.load(std::sync::atomic::Ordering::Relaxed)
            || match &*self.active_stream_state.read_or_recover() {
                Some(state) => !is_paused && !is_finished && state.is_buffering(),
                None => false,
            };
        let has_active_track = (has_loaded_track && (!is_finished || is_paused)) || is_buffering;

        PlaybackStateDto {
            current_track_id: if has_active_track {
                q_guard.current().map(|i| i.track_id.clone())
            } else {
                None
            },
            is_playing,
            is_paused,
            can_seek,
            is_buffering,
            position_secs: pos,
            duration_secs: duration,
            volume: vol,
            is_muted: vol == 0.0,
            repeat_mode: q_guard.repeat_mode(),
            is_shuffle: q_guard.is_shuffle(),
            queue_length: q_guard.len(),
            current_queue_index: q_guard.current_index(),
            queue_track_ids,
            user_queue_track_ids: q_guard.waiting_manual_track_ids(),
            source: self.current_source.read().await.clone(),
        }
    }

    pub async fn clear_remote_audio_cache(&self) -> AppResult<(u64, usize)> {
        let sm_opt = self.stream_manager.read_or_recover().clone();
        if let Some(sm) = sm_opt {
            let res = sm.clear_cache().await?;
            Ok((res.bytes_freed, res.files_removed))
        } else {
            Ok((0, 0))
        }
    }

    pub async fn get_remote_audio_cache_stats(
        &self,
    ) -> AppResult<crate::playback::stream::RemoteAudioCacheStats> {
        let sm_opt = self.stream_manager.read_or_recover().clone();
        if let Some(sm) = sm_opt {
            sm.get_cache_stats().await
        } else {
            Ok(crate::playback::stream::RemoteAudioCacheStats::default())
        }
    }
}
