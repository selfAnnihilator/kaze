//! Kaze's own download folder: watching it, moving Kaze's files into it,
//! adopting files that have no record and keeping records in step with disk.

use super::*;

impl DownloadService {
    /// Watches the Kaze folder so added, removed or renamed files show up
    /// everywhere at once. If watching fails, the focus refresh still applies.
    pub fn start_watching(self: &Arc<Self>) {
        use notify::{RecursiveMode, Watcher};
        if let Err(error) = std::fs::create_dir_all(&self.download_dir) {
            warn!(%error, "Could not create the Kaze download folder");
            return;
        }
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();
        let watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
            let relevant = result.is_ok_and(|event| {
                // Ignore in-progress partials and moves (".x.part", ".x.moving").
                event.paths.iter().any(|path| {
                    !path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with('.'))
                })
            });
            if relevant {
                let _ = tx.send(());
            }
        });
        let mut watcher = match watcher {
            Ok(watcher) => watcher,
            Err(error) => {
                warn!(%error, "Could not watch the Kaze download folder");
                return;
            }
        };
        if let Err(error) = watcher.watch(&self.download_dir, RecursiveMode::NonRecursive) {
            warn!(%error, "Could not watch the Kaze download folder");
            return;
        }
        let service = Arc::downgrade(self);
        tokio::spawn(async move {
            let _watcher = watcher; // lives as long as this task
            while rx.recv().await.is_some() {
                // Debounce bursts (a copy or move fires several events).
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                while rx.try_recv().is_ok() {}
                let Some(service) = service.upgrade() else { break };
                if let Err(error) = service.sync_folder().await {
                    warn!(%error, "Could not sync the Kaze download folder");
                }
            }
        });
    }

    /// Brings records and the library in line with the Kaze folder, then tells the UI.
    pub async fn sync_folder(&self) -> AppResult<()> {
        self.adopt_orphans().await?;
        self.reconcile_files().await?;
        let _ = self.event_bus.publish(Event::DownloadsChanged);
        Ok(())
    }

    /// Moves Kaze's own downloads (files with a download record) into the Kaze
    /// folder. Safe to re-run: files already there are skipped, and older Kaze
    /// versions that still download elsewhere get their files moved next launch.
    pub(super) async fn relocate_into_folder(&self) -> AppResult<()> {
        let dir = &self.download_dir;
        for task in self.repo.list_tasks(None, 10000).await? {
            if !matches!(task.status.as_str(), "COMPLETED" | "MISSING") {
                continue;
            }
            let Some(old) = task.destination_path.as_deref().map(PathBuf::from) else { continue };
            if !old.is_file() || in_folder(&old, dir) {
                continue;
            }
            let Some(name) = old.file_name() else { continue };
            tokio::fs::create_dir_all(dir)
                .await
                .map_err(|e| AppError::Io(e.to_string()))?;
            let target = dir.join(name);
            let old_track = old.canonicalize().unwrap_or_else(|_| old.clone());
            if let Err(error) = move_file(&old, &target).await {
                warn!(%error, from = %old.display(), "Could not move download into the Kaze folder");
                continue;
            }
            let new_track = target.canonicalize().unwrap_or_else(|_| target.clone());
            self.repo
                .relocate(
                    &task.id,
                    &target.to_string_lossy(),
                    &old_track.to_string_lossy(),
                    &new_track.to_string_lossy(),
                )
                .await?;
            info!(task_id = %task.id, to = %target.display(), "Moved download into the Kaze folder");
        }
        Ok(())
    }

    /// Completed records must point at a real file: mark vanished files MISSING
    /// (dropping their stale library row) and restore ones that came back.
    pub(super) async fn reconcile_files(&self) -> AppResult<()> {
        for status in ["COMPLETED", "MISSING"] {
            for task in self.repo.list_tasks(Some(status), 10000).await? {
                let Some(path) = task.destination_path.as_deref() else { continue };
                let present = Path::new(path).is_file();
                if status == "COMPLETED" && !present {
                    self.repo
                        .update_status(&task.id, "MISSING", None, Some(MISSING_MESSAGE), task.completed_at)
                        .await?;
                    // Keep the song's likes, playlists and history on its online version.
                    if let Err(error) = self.release(&task).await {
                        warn!(%error, task_id = %task.id, "Could not release a missing download");
                    }
                    info!(task_id = %task.id, file = %path, "Downloaded file is missing");
                } else if status == "MISSING" && present {
                    self.repo
                        .update_status(&task.id, "COMPLETED", None, None, task.completed_at)
                        .await?;
                }
            }
        }
        Ok(())
    }

    /// Downloaded files without a record (e.g. after a profile reset) become
    /// completed downloads again, and untagged library rows get clean names.
    pub(super) async fn adopt_orphans(&self) -> AppResult<()> {
        let Ok(mut entries) = tokio::fs::read_dir(&self.download_dir).await else {
            return Ok(());
        };
        let tasks = self.repo.list_tasks(None, 10000).await?;
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if !is_download_file(&path) {
                continue;
            }
            let Some(source) = source_of(&path).await else { continue };
            let path_str = path.to_string_lossy();
            let mine: Vec<_> = tasks
                .iter()
                .filter(|t| {
                    t.source_track_id.as_deref() == Some(source.as_str())
                        || t.destination_path.as_deref() == Some(&path_str)
                })
                .collect();
            if mine.iter().any(|t| {
                matches!(t.status.as_str(), "DOWNLOADING" | "QUEUED")
                    || (t.status == "COMPLETED" && file_exists(t.destination_path.as_deref()))
            }) {
                self.refresh_untagged(&path, &source).await;
                continue;
            }
            let existing = mine.first().map(|t| (*t).clone());
            if let Err(error) = self.adopt(&path, &source, existing, false).await {
                warn!(%error, file = %path.display(), "Could not adopt downloaded file");
            }
        }
        Ok(())
    }

    /// Re-imports a download whose library row still shows the raw filename.
    pub(super) async fn refresh_untagged(&self, path: &Path, source: &str) {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let row = self
            .library_service
            .track_repo()
            .find_by_path(&canonical.to_string_lossy())
            .await
            .ok()
            .flatten();
        if row.is_none_or(|r| r.title.ends_with(&format!("({source})"))) {
            self.finisher.import(path).await;
        }
    }
}

/// The recording a downloaded file belongs to: the id Kaze embedded in it,
/// else the `(id)` at the end of its name.
pub(super) async fn source_of(path: &Path) -> Option<String> {
    let owned = path.to_path_buf();
    if let Ok(Some(id)) = tokio::task::spawn_blocking(move || crate::downloads::read_source_id(&owned)).await {
        return Some(id);
    }
    let stem = path.file_stem().and_then(|s| s.to_str())?;
    parse_download_stem(stem).map(|n| n.source_id)
}

/// The canonical path of a file that may no longer exist (its folder still does).
pub(super) fn canonical_file(path: &Path) -> String {
    let resolved = match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => parent.canonicalize().map(|p| p.join(name)).ok(),
        _ => None,
    };
    resolved.unwrap_or_else(|| path.to_path_buf()).to_string_lossy().into_owned()
}

/// Whether `path` sits directly in `dir` (the Kaze folder is not recursive).
pub(super) fn in_folder(path: &Path, dir: &Path) -> bool {
    let canon = |p: &Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    path.parent().is_some_and(|parent| canon(parent) == canon(dir))
}

/// Moves a file without ever losing it: rename when possible, otherwise copy,
/// sync, verify the size and only then remove the original. An existing target
/// is accepted only if it is byte-identical.
async fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    if to.exists() {
        if same_contents(from, to).await? {
            return tokio::fs::remove_file(from).await;
        }
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "a different file already has this name",
        ));
    }
    if tokio::fs::rename(from, to).await.is_ok() {
        return Ok(());
    }
    // Different filesystems: copy beside the target, then publish atomically.
    let name = to.file_name().and_then(|n| n.to_str()).unwrap_or("download");
    let temp = to.with_file_name(format!(".{name}.moving"));
    let result = async {
        tokio::fs::copy(from, &temp).await?;
        tokio::fs::File::open(&temp).await?.sync_all().await?;
        let (a, b) = (tokio::fs::metadata(from).await?, tokio::fs::metadata(&temp).await?);
        if a.len() != b.len() {
            return Err(std::io::Error::other("copied size does not match"));
        }
        tokio::fs::rename(&temp, to).await
    }
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&temp).await;
        return result;
    }
    tokio::fs::remove_file(from).await
}

async fn same_contents(a: &Path, b: &Path) -> std::io::Result<bool> {
    if tokio::fs::metadata(a).await?.len() != tokio::fs::metadata(b).await?.len() {
        return Ok(false);
    }
    Ok(tokio::fs::read(a).await? == tokio::fs::read(b).await?)
}

pub(super) fn file_exists(path: Option<&str>) -> bool {
    path.is_some_and(|p| Path::new(p).is_file())
}

pub(super) fn is_download_file(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("m4a"))
}
