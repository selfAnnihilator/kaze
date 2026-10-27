use crate::core::event::Event;
use crate::core::event_bus::EventBus;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use symphonia::core::io::MediaSource;
use tracing::debug;

/// Status of a progressive remote audio download stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamStatus {
    Downloading,
    Completed,
    Failed(String),
    Cancelled,
}

/// Shared thread-safe coordination state between the async HTTP downloader
/// and the synchronous Rodio/Symphonia audio decoding thread.
pub struct ProgressiveStreamState {
    pub downloaded_bytes: AtomicU64,
    pub content_length: AtomicU64,
    pub status: Mutex<StreamStatus>,
    pub condvar: Condvar,
    pub is_buffering: AtomicBool,
    pub event_bus: Option<Arc<EventBus>>,
    pub track_id: String,
    playback_session: Mutex<Option<String>>,
}

impl ProgressiveStreamState {
    pub fn new(track_id: String, event_bus: Option<Arc<EventBus>>) -> Self {
        Self {
            downloaded_bytes: AtomicU64::new(0),
            content_length: AtomicU64::new(0),
            status: Mutex::new(StreamStatus::Downloading),
            condvar: Condvar::new(),
            is_buffering: AtomicBool::new(false),
            event_bus,
            track_id,
            playback_session: Mutex::new(None),
        }
    }

    /// Updates the downloaded byte count and wakes any waiting reader threads.
    pub fn update_downloaded(&self, new_bytes: u64) {
        let _status = self.status.lock().unwrap();
        self.downloaded_bytes.store(new_bytes, Ordering::Release);
        self.set_buffering(false);
        self.condvar.notify_all();
    }

    /// Attach only after PlaybackStarted. Probing/preparation and obsolete
    /// streams must never publish transport or current-song buffering events.
    pub fn activate_playback(&self, session_id: String) {
        let _status = self.status.lock().unwrap();
        *self.playback_session.lock().unwrap() = Some(session_id);
        self.publish_buffering(self.is_buffering());
    }

    pub fn deactivate_playback(&self) {
        let _status = self.status.lock().unwrap();
        *self.playback_session.lock().unwrap() = None;
    }

    // All callers hold status, serializing the flag and its notification with
    // chunk arrival/EOF. Otherwise a late "buffering" can overtake recovery.
    fn set_buffering(&self, buffering: bool) {
        if self.is_buffering.swap(buffering, Ordering::SeqCst) != buffering {
            self.publish_buffering(buffering);
        }
    }

    fn publish_buffering(&self, is_buffering: bool) {
        if let (Some(bus), Some(session_id)) = (&self.event_bus, self.playback_session.lock().unwrap().as_ref()) {
            let _ = bus.publish(Event::PlaybackBufferingChanged {
                session_id: session_id.clone(),
                track_id: self.track_id.clone(),
                is_buffering,
            });
        }
    }

    /// Marks the download as completed successfully.
    pub fn set_completed(&self) {
        {
            let mut s = self.status.lock().unwrap();
            *s = StreamStatus::Completed;
            self.set_buffering(false);
        }
        self.condvar.notify_all();
        // A paused player has no position ticks. Tell controllers that seeking
        // became available without pretending playback resumed.
        if let Some(ref bus) = self.event_bus {
            let _ = bus.publish(Event::PlaybackCapabilitiesChanged {
                track_id: self.track_id.clone(),
            });
        }
        debug!(track_id = %self.track_id, "Progressive stream marked completed");
    }

    /// Marks the download as failed with an error description.
    pub fn set_failed(&self, err: String) {
        {
            let mut s = self.status.lock().unwrap();
            *s = StreamStatus::Failed(err);
            self.set_buffering(false);
        }
        self.condvar.notify_all();
    }

    /// Marks the download as cancelled.
    pub fn set_cancelled(&self) {
        {
            let mut s = self.status.lock().unwrap();
            *s = StreamStatus::Cancelled;
            self.set_buffering(false);
        }
        self.condvar.notify_all();
    }

    pub fn is_completed(&self) -> bool {
        matches!(*self.status.lock().unwrap(), StreamStatus::Completed)
    }

    pub fn is_cancelled(&self) -> bool {
        matches!(*self.status.lock().unwrap(), StreamStatus::Cancelled)
    }

    pub fn is_buffering(&self) -> bool {
        self.is_buffering.load(Ordering::Relaxed)
    }
}

/// Synchronous `MediaSource` reader backed by the actively growing `.part` file on disk.
///
/// Ensures bounded memory consumption: bytes are served directly from the operating system's
/// disk/page cache without allocating unbounded `Vec<u8>` in RAM.
pub struct ProgressiveStreamReader {
    file: File,
    read_pos: u64,
    state: Arc<ProgressiveStreamState>,
    #[allow(dead_code)]
    part_path: PathBuf,
}

impl ProgressiveStreamReader {
    pub fn new(part_path: &Path, state: Arc<ProgressiveStreamState>) -> std::io::Result<Self> {
        let file = File::open(part_path)?;
        Ok(Self {
            file,
            read_pos: 0,
            state,
            part_path: part_path.to_path_buf(),
        })
    }
}

impl Read for ProgressiveStreamReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        loop {
            let status_guard = self.state.status.lock().unwrap();
            let downloaded = self.state.downloaded_bytes.load(Ordering::Acquire);
            if self.read_pos < downloaded {
                self.state.set_buffering(false);
                drop(status_guard);
                let available = downloaded - self.read_pos;
                let to_read = (available.min(buf.len() as u64)) as usize;
                let n = self.file.read(&mut buf[..to_read])?;
                self.read_pos += n as u64;
                return Ok(n);
            }

            // Reader caught up to downloaded bytes: check download status
            match &*status_guard {
                StreamStatus::Completed => {
                    // Truly at end of file
                    return Ok(0);
                }
                StreamStatus::Failed(err) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        format!("Progressive stream download failed: {}", err),
                    ));
                }
                StreamStatus::Cancelled => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Interrupted,
                        "Progressive stream playback cancelled",
                    ));
                }
                StreamStatus::Downloading => {
                    // Producer is still actively downloading: wait for new chunks
                    self.state.set_buffering(true);

                    // Wait on condvar with a 500ms timeout to wake up reliably
                    let _ = self.state.condvar.wait_timeout(status_guard, Duration::from_millis(500)).unwrap();

                    // Synchronize file read offset in case of desync
                    let _ = self.file.seek(SeekFrom::Start(self.read_pos));
                }
            }
        }
    }
}

impl Seek for ProgressiveStreamReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        // Once download is fully completed, allow complete unrestricted seeking
        if self.state.is_completed() {
            let res = self.file.seek(pos)?;
            self.read_pos = res;
            return Ok(res);
        }

        match pos {
            SeekFrom::Start(target) => {
                let downloaded = self.state.downloaded_bytes.load(Ordering::Acquire);
                if target <= downloaded {
                    let res = self.file.seek(SeekFrom::Start(target))?;
                    self.read_pos = res;
                    Ok(res)
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Cannot seek past buffered range during progressive download",
                    ))
                }
            }
            SeekFrom::Current(delta) => {
                let downloaded = self.state.downloaded_bytes.load(Ordering::Acquire);
                let target = (self.read_pos as i64) + delta;
                if target >= 0 && (target as u64) <= downloaded {
                    let res = self.file.seek(SeekFrom::Start(target as u64))?;
                    self.read_pos = res;
                    Ok(res)
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Seek delta outside buffered stream range",
                    ))
                }
            }
            SeekFrom::End(_) => {
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "SeekFrom::End unsupported while streaming is in progress",
                ))
            }
        }
    }
}

impl MediaSource for ProgressiveStreamReader {
    fn is_seekable(&self) -> bool {
        // Honest seek capability: false during active streaming, true once completed!
        self.state.is_completed()
    }

    fn byte_len(&self) -> Option<u64> {
        let cl = self.state.content_length.load(Ordering::Acquire);
        if cl > 0 {
            Some(cl)
        } else {
            None
        }
    }
}
