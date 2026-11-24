use crate::core::sync::MutexExt;
use crate::core::error::{AppError, AppResult};
use crate::playback::output::{Activity, OutputDevice};
use rodio::{Decoder, Sink};
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
    fn load_and_play_source(&mut self, _source: crate::playback::decoder::PlayerSource, file_path: Option<&Path>) -> AppResult<()> {
        if let Some(path) = file_path {
            self.load_and_play(path)
        } else {
            Err(AppError::Playback("Backend does not support direct PlayerSource without file path".into()))
        }
    }
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
    output: Option<OutputDevice>,
    sink: Option<Sink>,
    current_path: Option<PathBuf>,
    volume: f32,
    is_paused: bool,
}

impl RodioAudioBackend {
    /// Creates the backend. The output device opens on first playback, so an
    /// app that never plays never holds the device.
    pub fn new() -> Self {
        Self {
            output: None,
            sink: None,
            current_path: None,
            volume: 0.8,
            is_paused: false,
        }
    }

    /// The open device, reopening it if it failed or was never opened.
    fn output(&mut self) -> Option<&OutputDevice> {
        if self.output.as_ref().is_some_and(OutputDevice::has_failed) {
            warn!("Audio output device failed; reopening");
            self.output = None;
        }
        if self.output.is_none() {
            self.output = OutputDevice::open();
        }
        self.output.as_ref()
    }

    fn set_activity(&self, activity: Activity) {
        if let Some(output) = &self.output {
            output.set_activity(activity);
        }
    }
}

/// rodio's `Sink::try_seek` waits until the audio thread applies the seek,
/// and that thread only runs while the output device does. A paused player
/// suspends the device after `IDLE_AFTER`, so a seek then waited forever while
/// holding the player lock, freezing every later command. Wake the device for
/// the seek; reporting idle again restarts its timer.
pub(crate) fn seek_awake<T>(paused: bool, set_activity: impl Fn(Activity), seek: impl FnOnce() -> T) -> T {
    if paused {
        set_activity(Activity::Playing);
    }
    let result = seek();
    if paused {
        set_activity(Activity::Idle);
    }
    result
}

impl Default for RodioAudioBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioBackend for RodioAudioBackend {
    fn load_and_play_source(&mut self, source: crate::playback::decoder::PlayerSource, file_path: Option<&Path>) -> AppResult<()> {
        let Some(output) = self.output() else {
            return Err(AppError::Playback("Audio output device unavailable".into()));
        };
        let sink = output.new_sink();
        sink.set_volume(self.volume);
        sink.append(source);
        sink.play();

        self.sink = Some(sink);
        self.current_path = file_path.map(|p| p.to_path_buf());
        self.is_paused = false;

        info!(path = ?file_path, "Rodio playback started");
        Ok(())
    }

    fn load_and_play(&mut self, file_path: &Path) -> AppResult<()> {
        if !file_path.exists() {
            return Err(AppError::Playback(format!(
                "Audio file does not exist: {}",
                file_path.display()
            )));
        }

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

        self.load_and_play_source(source, Some(file_path))
    }

    fn pause(&mut self) -> AppResult<()> {
        if let Some(ref sink) = self.sink {
            sink.pause();
            self.is_paused = true;
            debug!("Playback paused");
        }
        self.set_activity(Activity::Idle);
        Ok(())
    }

    fn resume(&mut self) -> AppResult<()> {
        if let Some(ref sink) = self.sink {
            self.set_activity(Activity::Playing);
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
        self.set_activity(Activity::Idle);
        debug!("Playback stopped");
        Ok(())
    }

    fn seek(&mut self, position: Duration) -> AppResult<()> {
        if let Some(ref sink) = self.sink {
            seek_awake(self.is_paused, |a| self.set_activity(a), || sink.try_seek(position)).map_err(|e| {
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
        let finished = self.sink.as_ref().map(|s| s.empty()).unwrap_or(true);
        if finished {
            // The queue ran out: nothing feeds the device any more.
            self.set_activity(Activity::Idle);
        }
        finished
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
    fn load_and_play_source(&mut self, _source: crate::playback::decoder::PlayerSource, file_path: Option<&Path>) -> AppResult<()> {
        let mut path_guard = self.loaded_path.lock_or_recover();
        *path_guard = file_path.map(|p| p.to_path_buf());
        self.is_paused_flag.store(false, Ordering::SeqCst);
        self.is_finished_flag.store(false, Ordering::SeqCst);
        self.position_millis.store(0, Ordering::SeqCst);
        Ok(())
    }

    fn load_and_play(&mut self, file_path: &Path) -> AppResult<()> {
        let mut path_guard = self.loaded_path.lock_or_recover();
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
        let mut path_guard = self.loaded_path.lock_or_recover();
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
        let mut guard = self.volume_level.lock_or_recover();
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

#[cfg(test)]
#[path = "backend_seek_tests.rs"]
mod seek_tests;
