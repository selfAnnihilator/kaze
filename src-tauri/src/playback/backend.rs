use crate::core::error::{AppError, AppResult};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Abstraction trait for audio output backends.
pub trait AudioBackend: Send + Sync {
    fn load_and_play(&mut self, file_path: &Path) -> AppResult<()>;
    fn pause(&mut self) -> AppResult<()>;
    fn resume(&mut self) -> AppResult<()>;
    fn stop(&mut self) -> AppResult<()>;
    fn seek(&mut self, position: Duration) -> AppResult<()>;
    fn set_volume(&mut self, volume: f32) -> AppResult<()>;
    fn position(&self) -> Duration;
    fn is_paused(&self) -> bool;
    fn is_finished(&self) -> bool;
}

/// Concrete audio backend powered by rodio and cpal.
pub struct RodioAudioBackend {
    stream_handle: Option<OutputStreamHandle>,
    sink: Option<Sink>,
    current_path: Option<PathBuf>,
    volume: f32,
    is_paused: bool,
}

impl RodioAudioBackend {
    pub fn try_new() -> AppResult<Self> {
        match OutputStream::try_default() {
            Ok((stream, handle)) => {
                // Keep the audio output stream active for the process lifetime
                std::mem::forget(stream);
                Ok(Self {
                    stream_handle: Some(handle),
                    sink: None,
                    current_path: None,
                    volume: 0.8,
                    is_paused: false,
                })
            }
            Err(err) => {
                warn!("Audio device initialization failed: {}. Playback engine fallback active.", err);
                Ok(Self {
                    stream_handle: None,
                    sink: None,
                    current_path: None,
                    volume: 0.8,
                    is_paused: false,
                })
            }
        }
    }
}

impl AudioBackend for RodioAudioBackend {
    fn load_and_play(&mut self, file_path: &Path) -> AppResult<()> {
        if !file_path.exists() {
            return Err(AppError::Playback(format!(
                "Audio file does not exist: {}",
                file_path.display()
            )));
        }

        // Recreate sink to clear any previous buffer completely
        if let Some(ref handle) = self.stream_handle {
            let sink = Sink::try_new(handle).map_err(|e| {
                AppError::Playback(format!("Failed to initialize audio sink: {}", e))
            })?;
            sink.set_volume(self.volume);

            let source = match crate::playback::decoder::SymphoniaSource::new(file_path) {
                Ok(symphonia_source) => {
                    crate::playback::decoder::PlayerSource::Symphonia(symphonia_source)
                }
                Err(err) => {
                    debug!(error = %err, path = %file_path.display(), "SymphoniaSource failed, attempting rodio::Decoder fallback");
                    let file = File::open(file_path).map_err(|e| {
                        AppError::Playback(format!("Failed to open file for playback: {}", e))
                    })?;
                    let rodio_decoder = Decoder::new(BufReader::new(file)).map_err(|e| {
                        AppError::Playback(format!("Audio decoder failure for {}: {}", file_path.display(), e))
                    })?;
                    crate::playback::decoder::PlayerSource::Rodio(rodio_decoder)
                }
            };

            sink.append(source);
            sink.play();

            self.sink = Some(sink);
            self.current_path = Some(file_path.to_path_buf());
            self.is_paused = false;

            info!(path = %file_path.display(), "Rodio playback started");
            Ok(())
        } else {
            Err(AppError::Playback("Audio output device unavailable".into()))
        }
    }

    fn pause(&mut self) -> AppResult<()> {
        if let Some(ref sink) = self.sink {
            sink.pause();
            self.is_paused = true;
            debug!("Playback paused");
        }
        Ok(())
    }

    fn resume(&mut self) -> AppResult<()> {
        if let Some(ref sink) = self.sink {
            sink.play();
            self.is_paused = false;
            debug!("Playback resumed");
        }
        Ok(())
    }

    fn stop(&mut self) -> AppResult<()> {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self.current_path = None;
        self.is_paused = false;
        debug!("Playback stopped");
        Ok(())
    }

    fn seek(&mut self, position: Duration) -> AppResult<()> {
        if let Some(ref sink) = self.sink {
            sink.try_seek(position).map_err(|e| {
                AppError::Playback(format!("Audio seek error: {}", e))
            })?;
            debug!(seek_secs = position.as_secs_f64(), "Audio seek performed");
        }
        Ok(())
    }

    fn set_volume(&mut self, volume: f32) -> AppResult<()> {
        let clamped = volume.clamp(0.0, 1.0);
        self.volume = clamped;
        if let Some(ref sink) = self.sink {
            sink.set_volume(clamped);
        }
        Ok(())
    }

    fn position(&self) -> Duration {
        self.sink
            .as_ref()
            .map(|s| s.get_pos())
            .unwrap_or(Duration::ZERO)
    }

    fn is_paused(&self) -> bool {
        self.is_paused
    }

    fn is_finished(&self) -> bool {
        self.sink
            .as_ref()
            .map(|s| s.empty())
            .unwrap_or(true)
    }
}

/// Headless in-memory audio backend for testing and environments without soundcards.
#[derive(Default, Clone)]
pub struct MockAudioBackend {
    pub loaded_path: Arc<std::sync::Mutex<Option<PathBuf>>>,
    pub is_paused_flag: Arc<AtomicBool>,
    pub is_finished_flag: Arc<AtomicBool>,
    pub position_millis: Arc<AtomicU64>,
    pub volume_level: Arc<std::sync::Mutex<f32>>,
}

impl MockAudioBackend {
    pub fn new() -> Self {
        Self {
            loaded_path: Arc::new(std::sync::Mutex::new(None)),
            is_paused_flag: Arc::new(AtomicBool::new(false)),
            is_finished_flag: Arc::new(AtomicBool::new(true)),
            position_millis: Arc::new(AtomicU64::new(0)),
            volume_level: Arc::new(std::sync::Mutex::new(0.8)),
        }
    }

    pub fn set_finished(&self, finished: bool) {
        self.is_finished_flag.store(finished, Ordering::SeqCst);
    }

    pub fn advance_position(&self, delta: Duration) {
        self.position_millis.fetch_add(delta.as_millis() as u64, Ordering::SeqCst);
    }
}

impl AudioBackend for MockAudioBackend {
    fn load_and_play(&mut self, file_path: &Path) -> AppResult<()> {
        let mut path_guard = self.loaded_path.lock().unwrap();
        *path_guard = Some(file_path.to_path_buf());
        self.is_paused_flag.store(false, Ordering::SeqCst);
        self.is_finished_flag.store(false, Ordering::SeqCst);
        self.position_millis.store(0, Ordering::SeqCst);
        Ok(())
    }

    fn pause(&mut self) -> AppResult<()> {
        self.is_paused_flag.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn resume(&mut self) -> AppResult<()> {
        self.is_paused_flag.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn stop(&mut self) -> AppResult<()> {
        let mut path_guard = self.loaded_path.lock().unwrap();
        *path_guard = None;
        self.is_paused_flag.store(false, Ordering::SeqCst);
        self.is_finished_flag.store(true, Ordering::SeqCst);
        self.position_millis.store(0, Ordering::SeqCst);
        Ok(())
    }

    fn seek(&mut self, position: Duration) -> AppResult<()> {
        self.position_millis.store(position.as_millis() as u64, Ordering::SeqCst);
        Ok(())
    }

    fn set_volume(&mut self, volume: f32) -> AppResult<()> {
        let mut guard = self.volume_level.lock().unwrap();
        *guard = volume.clamp(0.0, 1.0);
        Ok(())
    }

    fn position(&self) -> Duration {
        let millis = self.position_millis.load(Ordering::SeqCst);
        Duration::from_millis(millis)
    }

    fn is_paused(&self) -> bool {
        self.is_paused_flag.load(Ordering::SeqCst)
    }

    fn is_finished(&self) -> bool {
        self.is_finished_flag.load(Ordering::SeqCst)
    }
}
