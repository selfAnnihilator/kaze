//! Audio output device that stops mixing when nothing plays.
//!
//! rodio's `OutputStream` keeps the device running for as long as it exists,
//! so a paused player kept a real-time thread mixing silence forever. Here a
//! dedicated thread owns the cpal stream (which is not `Send`) and pauses it
//! once playback has been idle for [`IDLE_AFTER`]. Sinks keep their queued
//! sources and position while the stream is paused, so resuming is instant.

use rodio::cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rodio::cpal::{self, FromSample, SizedSample};
use rodio::dynamic_mixer::{self, DynamicMixerController};
use rodio::Sink;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// How long the device keeps running after playback pauses or stops.
pub const IDLE_AFTER: Duration = Duration::from_secs(60);

/// What the playback backend tells the device thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    Playing,
    Idle,
}

/// A stream that can be paused and resumed (the cpal stream, or a test fake).
pub trait Pausable {
    fn resume(&self);
    fn suspend(&self);
}

impl Pausable for cpal::Stream {
    fn resume(&self) {
        if let Err(err) = self.play() {
            warn!("Audio output resume failed: {}", err);
        }
    }

    fn suspend(&self) {
        if let Err(err) = self.pause() {
            warn!("Audio output pause failed: {}", err);
        }
    }
}

/// Runs until the sender is dropped. The stream is suspended only after
/// `idle_after` passes with no `Playing` message, so quick pause/resume
/// cycles never touch the device. Repeated `Idle` messages keep the first
/// deadline.
pub fn run_activity_loop(rx: &Receiver<Activity>, stream: &impl Pausable, idle_after: Duration) {
    let mut running = true;
    let mut deadline: Option<Instant> = None;
    loop {
        let message = match deadline {
            Some(at) => match rx.recv_timeout(at.saturating_duration_since(Instant::now())) {
                Ok(message) => message,
                Err(RecvTimeoutError::Timeout) => {
                    stream.suspend();
                    running = false;
                    deadline = None;
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => return,
            },
            None => match rx.recv() {
                Ok(message) => message,
                Err(_) => return,
            },
        };
        match message {
            Activity::Playing => {
                deadline = None;
                if !running {
                    stream.resume();
                    running = true;
                }
            }
            Activity::Idle if running && deadline.is_none() => {
                deadline = Some(Instant::now() + idle_after);
            }
            Activity::Idle => {}
        }
    }
}

/// The open output device. Dropping it ends the thread and closes the device.
pub struct OutputDevice {
    mixer: Arc<DynamicMixerController<f32>>,
    activity: Sender<Activity>,
    /// The last activity sent, so callers can report it on every tick.
    idle: AtomicBool,
    failed: Arc<AtomicBool>,
}

impl OutputDevice {
    /// Opens the default output device (or any working one).
    pub fn open() -> Option<Self> {
        let (ready_tx, ready_rx) = mpsc::channel();
        let (activity, activity_rx) = mpsc::channel();
        let failed = Arc::new(AtomicBool::new(false));
        let thread_failed = failed.clone();
        let spawned = std::thread::Builder::new()
            .name("kaze-audio-output".into())
            .spawn(move || match build_stream(thread_failed) {
                Ok((mixer, stream)) => {
                    let _ = ready_tx.send(Some(mixer));
                    run_activity_loop(&activity_rx, &stream, IDLE_AFTER);
                }
                Err(err) => {
                    warn!("Audio device initialization failed: {}", err);
                    let _ = ready_tx.send(None);
                }
            });
        if let Err(err) = spawned {
            warn!("Audio output thread failed to start: {}", err);
            return None;
        }
        let mixer = ready_rx.recv().ok().flatten()?;
        info!("Audio output device opened");
        Some(Self { mixer, activity, idle: AtomicBool::new(false), failed })
    }

    /// A new sink playing into this device.
    pub fn new_sink(&self) -> Sink {
        let (sink, queue) = Sink::new_idle();
        self.mixer.add(queue);
        self.set_activity(Activity::Playing);
        sink
    }

    pub fn set_activity(&self, activity: Activity) {
        let idle = activity == Activity::Idle;
        if self.idle.swap(idle, Ordering::Relaxed) != idle {
            let _ = self.activity.send(activity);
        }
    }

    /// The device reported an error (for example, it was unplugged).
    pub fn has_failed(&self) -> bool {
        self.failed.load(Ordering::Relaxed)
    }
}

type Mixer = Arc<DynamicMixerController<f32>>;

fn build_stream(failed: Arc<AtomicBool>) -> Result<(Mixer, cpal::Stream), String> {
    let host = cpal::default_host();
    let mut devices: Vec<cpal::Device> = host.default_output_device().into_iter().collect();
    if let Ok(others) = host.output_devices() {
        devices.extend(others);
    }
    let mut last_error = "no output device".to_string();
    for device in devices {
        match build_on_device(&device, failed.clone()) {
            Ok(opened) => return Ok(opened),
            Err(err) => last_error = err,
        }
    }
    Err(last_error)
}

fn build_on_device(device: &cpal::Device, failed: Arc<AtomicBool>) -> Result<(Mixer, cpal::Stream), String> {
    let format = device.default_output_config().map_err(|e| e.to_string())?;
    let (mixer, source) = dynamic_mixer::mixer::<f32>(format.channels(), format.sample_rate().0);
    let config = format.config();
    let stream = match format.sample_format() {
        cpal::SampleFormat::F32 => build_typed::<f32>(device, &config, source, failed),
        cpal::SampleFormat::F64 => build_typed::<f64>(device, &config, source, failed),
        cpal::SampleFormat::I16 => build_typed::<i16>(device, &config, source, failed),
        cpal::SampleFormat::I32 => build_typed::<i32>(device, &config, source, failed),
        cpal::SampleFormat::U16 => build_typed::<u16>(device, &config, source, failed),
        cpal::SampleFormat::U8 => build_typed::<u8>(device, &config, source, failed),
        other => return Err(format!("unsupported sample format {other:?}")),
    }
    .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok((mixer, stream))
}

fn build_typed<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut source: dynamic_mixer::DynamicMixer<f32>,
    failed: Arc<AtomicBool>,
) -> Result<cpal::Stream, cpal::BuildStreamError> {
    device.build_output_stream::<T, _, _>(
        config,
        move |data: &mut [T], _| {
            for sample in data.iter_mut() {
                *sample = T::from_sample(source.next().unwrap_or(0.0));
            }
        },
        move |err| {
            warn!("Audio output stream error: {}", err);
            if matches!(err, cpal::StreamError::DeviceNotAvailable) {
                failed.store(true, Ordering::Relaxed);
            }
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeStream(Mutex<Vec<&'static str>>);

    impl Pausable for FakeStream {
        fn resume(&self) {
            self.0.lock().unwrap().push("resume");
        }
        fn suspend(&self) {
            self.0.lock().unwrap().push("suspend");
        }
    }

    fn run(script: impl FnOnce(&Sender<Activity>) + Send + 'static) -> Vec<&'static str> {
        let (tx, rx) = mpsc::channel();
        let stream = FakeStream::default();
        let driver = std::thread::spawn(move || script(&tx));
        run_activity_loop(&rx, &stream, Duration::from_millis(40));
        driver.join().unwrap();
        stream.0.into_inner().unwrap()
    }

    #[test]
    fn idle_device_is_suspended_after_the_delay_and_resumed_on_play() {
        let calls = run(|tx| {
            tx.send(Activity::Idle).unwrap();
            std::thread::sleep(Duration::from_millis(120));
            tx.send(Activity::Playing).unwrap();
            std::thread::sleep(Duration::from_millis(10));
        });
        assert_eq!(calls, ["suspend", "resume"]);
    }

    #[test]
    fn quick_pause_and_resume_never_touches_the_device() {
        let calls = run(|tx| {
            tx.send(Activity::Idle).unwrap();
            std::thread::sleep(Duration::from_millis(5));
            tx.send(Activity::Playing).unwrap();
            std::thread::sleep(Duration::from_millis(80));
        });
        assert!(calls.is_empty(), "{calls:?}");
    }

    #[test]
    fn repeated_idle_reports_keep_the_first_deadline() {
        let calls = run(|tx| {
            for _ in 0..10 {
                tx.send(Activity::Idle).unwrap();
                std::thread::sleep(Duration::from_millis(15));
            }
        });
        assert_eq!(calls, ["suspend"]);
    }
}
