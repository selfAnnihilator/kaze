//! Starting playback while the stream is still downloading: validate the first
//! bytes, then hand a progressive reader to the backend.
use super::*;

impl StreamPlaybackManager {
    pub(super) async fn try_start_progressive(
        &self, stream_url: &str, part_path: &Path, target_path: &Path,
        track_id: &str, candidate_dur: f64, preparation: &Arc<Preparation>,
    ) -> AppResult<PreparedPlayback> {
        // Each attempt owns a unique path. An obsolete attempt cannot unlink a
        // newer request's file, even when both resolve to the same cache key.
        let part_path = part_path.with_extension(format!("{}.part", uuid::Uuid::new_v4()));
        preparation.paths.lock_or_recover().insert(part_path.clone());
        let response = tokio::time::timeout(std::time::Duration::from_secs(12),
            self.http_client.get(stream_url).header(reqwest::header::USER_AGENT, "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36").send()).await
            .map_err(|_| AppError::Playback("Audio stream connection timed out (12s)".into()))?
            .map_err(|_| AppError::Playback("Could not connect to audio source".into()))?;
        if !response.status().is_success() {
            return Err(AppError::Playback(format!("Audio stream returned HTTP {}", response.status())));
        }
        let content_type = response.headers().get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()).unwrap_or("").to_owned();
        if content_type.starts_with("text/") || content_type.contains("json") {
            return Err(AppError::Playback("Audio source returned a page instead of audio".into()));
        }
        let max_size = self.config.read().await.max_single_file_bytes;
        let length = response.content_length().unwrap_or(0);
        if length > max_size { return Err(AppError::Playback("Audio source exceeds cache file limit".into())); }
        let cleanup = PartFileCleanupGuard::new(part_path.clone());
        // Keep creation and cleanup ownership in one synchronous step: dropping
        // a Tokio file-create future can otherwise create a file after cleanup.
        let file = tokio::fs::File::from_std(std::fs::File::create(&part_path)
            .map_err(|e| AppError::Io(e.to_string()))?);
        let state = Arc::new(ProgressiveStreamState::new(track_id.into(), self.event_bus.read_or_recover().clone()));
        state.content_length.store(length, std::sync::atomic::Ordering::Release);
        // Serialize registration with cancellation, including cancellation that
        // arrived while the HTTP headers were being fetched.
        {
            let mut slot = preparation.state.lock_or_recover();
            *slot = Some(state.clone());
            if *preparation.cancelled.borrow() { state.set_cancelled(); }
        }
        let mut cancelled = preparation.cancelled.subscribe();
        let producer_state = state.clone();
        let target = target_path.to_path_buf();
        let producer_path = part_path.clone();
        let manager = self.clone_for_bg();
        // Install the producer before any synchronous decoder can request bytes.
        *preparation.producer.lock().await = Some(tokio::spawn(async move {
            let mut cleanup = cleanup;
            let mut file = file;
            let mut response = response;
            let mut total = 0u64;
            let download: AppResult<()> = async {
                loop {
                    let chunk = tokio::select! {
                        biased;
                        _ = async { cancelled.wait_for(|value| *value).await.ok(); } =>
                            return Err(AppError::Playback("Stream cancelled".into())),
                        chunk = tokio::time::timeout(std::time::Duration::from_secs(25), response.chunk()) =>
                            chunk.map_err(|_| AppError::Playback("Audio download inactive for 25 seconds".into()))?
                                .map_err(|_| AppError::Playback("Audio download interrupted or truncated".into()))?,
                    };
                    let Some(chunk) = chunk else { break };
                    total += chunk.len() as u64;
                    if total > max_size { return Err(AppError::Playback("Audio exceeds cache file limit".into())); }
                    file.write_all(&chunk).await.map_err(|e| AppError::Io(e.to_string()))?;
                    file.flush().await.map_err(|e| AppError::Io(e.to_string()))?;
                    producer_state.update_downloaded(total);
                }
                if total == 0 || (length > 0 && total != length) {
                    return Err(AppError::Playback("Audio download empty or truncated".into()));
                }
                if producer_state.is_cancelled() { return Err(AppError::Playback("Stream cancelled".into())); }
                Ok(())
            }.await;
            drop(file); // Stop writing before either finalizing or removing the partial file.
            match download {
                Ok(()) => match tokio::fs::rename(&producer_path, &target).await {
                    Ok(()) => {
                        cleanup.disarm();
                        producer_state.set_completed(); // Real EOF wakes probing readers, too.
                        Self::touch_cache_entry(&target);
                        manager.spawn_cache_eviction();
                    }
                    Err(_) => producer_state.set_failed("Could not finalize audio cache".into()),
                },
                Err(error) => producer_state.set_failed(error.to_string()),
            }
        }));
        let reader = ProgressiveStreamReader::new(&part_path, state.clone())
            .or_else(|_| ProgressiveStreamReader::new(target_path, state.clone()))
            .map_err(|e| AppError::Io(e.to_string()))?;
        let mut hint = symphonia::core::probe::Hint::new();
        if content_type.contains("webm") { hint.with_extension("webm"); }
        else if content_type.contains("ogg") || content_type.contains("opus") { hint.with_extension("ogg"); }
        let desc = format!("progressive:{track_id}");
        let mut probe = preparation.probe.lock().await;
        *probe = Some(tokio::task::spawn_blocking(move || SymphoniaSource::from_media_source(Box::new(reader), hint, &desc)));
        let decoded = probe.as_mut().unwrap().await
            .map_err(|_| AppError::Playback("Audio decoder worker failed".into()))?;
        probe.take();
        let source = match decoded {
            Ok(source) => source,
            Err(error) => {
                state.set_cancelled();
                // End this candidate's producer before a retry owns a new file.
                if let Some(task) = preparation.producer.lock().await.take() {
                    task.abort(); // Async HTTP producer only; never abort the blocking probe.
                    let _ = task.await;
                }
                return Err(error);
            }
        };
        if state.is_cancelled() { return Err(AppError::Playback("Playback preparation cancelled".into())); }
        info!(track_id, "Playback preparation decoded initial audio");
        Ok(PreparedPlayback::ProgressiveStream {
            source: PlayerSource::Symphonia(source), part_path,
            target_path: target_path.into(), duration: candidate_dur, state,
        })
    }

    pub(super) async fn is_decodable_audio(path: &Path) -> bool {
        let path = path.to_path_buf();
        tokio::task::spawn_blocking(move || SymphoniaSource::new(&path).is_ok()).await.unwrap_or(false)
    }

    pub(super) async fn full_cache_duration(&self, path: &Path, track_id: &str) -> f64 {
        use rodio::Source;
        let path = path.to_path_buf();
        let decoded = tokio::task::spawn_blocking(move || SymphoniaSource::new(&path)
            .ok()
            .and_then(|source| source.total_duration())
            .map(|duration| duration.as_secs_f64())
            .filter(|duration| *duration > 30.0)).await.ok().flatten();
        if let Some(duration) = decoded { return duration; }
        self.get_known_duration(track_id).await.filter(|duration| *duration > 30.0).unwrap_or(210.0)
    }
}
