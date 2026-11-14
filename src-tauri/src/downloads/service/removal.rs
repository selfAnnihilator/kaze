//! Removing a download: the file goes, the song becomes its online recording
//! again and its likes, playlists and history move with it.

use super::*;

impl DownloadService {
    /// Removes a song from Downloads: deletes Kaze's file and turns the library
    /// row back into the online recording (likes, playlists and history move
    /// with it). `track_id` may be the online id, a saved alias or the local row.
    /// Returns the released `(local id, online id)` when a library row existed.
    pub async fn remove_download(
        &self,
        track_id: &str,
        source_id: Option<&str>,
    ) -> AppResult<Option<(String, String)>> {
        let tasks = self.tasks_for_recording(track_id, source_id).await?;
        self.ensure_owned_files(&tasks)?;
        let mut released = None;
        for task in &tasks {
            released = self.delete_and_release(task).await?.or(released);
        }
        let _ = self.event_bus.publish(Event::DownloadsChanged);
        Ok(released)
    }

    /// The saved downloads (present or missing) of the song the UI showed.
    async fn tasks_for_recording(
        &self,
        track_id: &str,
        source_id: Option<&str>,
    ) -> AppResult<Vec<DownloadTaskRecord>> {
        let source = match source_id.or_else(|| track_id.strip_prefix("jiosaavn:")) {
            Some(source) => Some(source.to_string()),
            None => self.source_of_track(track_id).await?,
        };
        let Some(source) = source else {
            return Err(AppError::Validation("This song isn't in Downloads".into()));
        };
        let tasks: Vec<_> = self
            .repo
            .list_tasks(None, 10000)
            .await?
            .into_iter()
            .filter(|t| matches!(t.status.as_str(), "COMPLETED" | "MISSING"))
            .filter(|t| t.source_track_id.as_deref() == Some(source.as_str()))
            .collect();
        if tasks.is_empty() {
            return Err(AppError::Validation("This song isn't in Downloads".into()));
        }
        Ok(tasks)
    }

    /// Only ever delete Kaze's own files: check every file before deleting any.
    fn ensure_owned_files(&self, tasks: &[DownloadTaskRecord]) -> AppResult<()> {
        let foreign = tasks
            .iter()
            .filter_map(|task| task.destination_path.as_deref().map(Path::new))
            .find(|path| path.exists() && !in_folder(path, &self.download_dir));
        match foreign {
            Some(path) => Err(AppError::Validation(format!(
                "{} is outside Kaze's download folder, so Kaze won't delete it",
                path.display()
            ))),
            None => Ok(()),
        }
    }

    /// Deletes one download's file (already gone is fine), hands its library
    /// row to the online song and forgets the record.
    async fn delete_and_release(
        &self,
        task: &DownloadTaskRecord,
    ) -> AppResult<Option<(String, String)>> {
        if let Some(path) = task.destination_path.as_deref() {
            match tokio::fs::remove_file(path).await {
                Ok(()) => info!(task_id = %task.id, file = %path, "Removed download"),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(AppError::Io(format!("Couldn't delete {path}: {e}"))),
            }
        }
        self.forget(task).await
    }

    /// Drops a download whose file is gone: its library row is handed to the
    /// online song, then the record is deleted.
    pub(super) async fn forget(&self, task: &DownloadTaskRecord) -> AppResult<Option<(String, String)>> {
        let released = self.release(task).await?;
        self.repo.delete_task(&task.id).await?;
        Ok(released)
    }

    /// The recording behind a local library row, from its file.
    pub(super) async fn source_of_track(&self, track_id: &str) -> AppResult<Option<String>> {
        let Some(track) = self.library_service.track_repo().find_by_id(track_id).await? else {
            return Ok(None);
        };
        let path = PathBuf::from(&track.file_path);
        let canonical = |p: &str| canonical_file(Path::new(p));
        let by_task = self
            .repo
            .list_tasks(None, 10000)
            .await?
            .into_iter()
            .find(|t| t.destination_path.as_deref().is_some_and(|d| canonical(d) == canonical(&track.file_path)))
            .and_then(|t| t.source_track_id);
        Ok(match by_task {
            Some(source) => Some(source),
            None => source_of(&path).await,
        })
    }

    /// Hands a download's library row back to its online recording.
    pub(super) async fn release(&self, task: &DownloadTaskRecord) -> AppResult<Option<(String, String)>> {
        let Some(path) = task.destination_path.as_deref() else { return Ok(None) };
        let paths = vec![path.to_string(), canonical_file(Path::new(path))];
        let Some(source) = task.source_track_id.clone() else {
            // No recording to fall back to: nothing to keep the data on.
            for path in &paths {
                self.library_service.track_repo().delete_by_path(path).await?;
            }
            return Ok(None);
        };
        let recording = OnlineRecording {
            source_id: source,
            title: task.title.clone(),
            artist: task.artist.clone(),
            album: task.album.clone(),
        };
        self.repo.release_to_online(&paths, &recording).await
    }
}
