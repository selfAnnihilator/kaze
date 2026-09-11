use crate::core::error::{AppError, AppResult};
use notify::{Config, Event as NotifyEvent, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;
use tracing::{info, warn};

pub struct LibraryWatcher {
    watcher: RecommendedWatcher,
    watched_paths: Vec<PathBuf>,
}

impl LibraryWatcher {
    pub fn new<F>(event_handler: F) -> AppResult<Self>
    where
        F: Fn(NotifyEvent) + Send + 'static,
    {
        let (tx, rx): (std::sync::mpsc::Sender<notify::Result<NotifyEvent>>, Receiver<notify::Result<NotifyEvent>>) = channel();

        let watcher = RecommendedWatcher::new(
            tx,
            Config::default().with_poll_interval(Duration::from_secs(2)),
        )
        .map_err(|e| AppError::Library(format!("Failed to initialize directory watcher: {}", e)))?;

        // Spawn event listener thread
        std::thread::spawn(move || {
            while let Ok(res) = rx.recv() {
                match res {
                    Ok(event) => event_handler(event),
                    Err(err) => warn!("Directory watcher error: {}", err),
                }
            }
        });

        Ok(Self {
            watcher,
            watched_paths: Vec::new(),
        })
    }

    pub fn watch_directory(&mut self, path: &Path) -> AppResult<()> {
        if !path.exists() || !path.is_dir() {
            return Err(AppError::Library(format!(
                "Cannot watch non-directory path: {}",
                path.display()
            )));
        }

        self.watcher
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| AppError::Library(format!("Failed to watch directory {}: {}", path.display(), e)))?;

        self.watched_paths.push(path.to_path_buf());
        info!(path = %path.display(), "Active directory watcher registered");
        Ok(())
    }

    pub fn unwatch_directory(&mut self, path: &Path) -> AppResult<()> {
        self.watcher
            .unwatch(path)
            .map_err(|e| AppError::Library(format!("Failed to unwatch directory: {}", e)))?;

        self.watched_paths.retain(|p| p != path);
        info!(path = %path.display(), "Directory watcher unregistered");
        Ok(())
    }
}
