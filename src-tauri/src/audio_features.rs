//! Local audio descriptors for smoother Flow transitions.
//!
//! Each local track is decoded once in the background (middle 60 s, mono,
//! decimated to ~11 kHz) and reduced to three numbers:
//! - `energy`: RMS loudness mapped from -40..0 dBFS to 0..1
//! - `brightness`: zero-crossing rate, a cheap stand-in for spectral centroid
//! - `tempo_bpm`: onset-flux autocorrelation over 60–180 BPM, `None` when the
//!   pulse is not confident (ambient, speech, sustained tones)
//!
//! No network, no new dependencies, nothing on the audio thread.

use crate::core::{AppError, AppResult, Event, EventBus};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;
use tokio::sync::Notify;

/// Bump to re-analyze every track after an algorithm change.
pub const ANALYZER_VERSION: i64 = 1;
const ANALYSIS_SECS: f64 = 60.0;
const TARGET_RATE: u32 = 11_025;
const FRAME: usize = 256;
const HOP: usize = 128;
const MIN_BPM: f64 = 60.0;
const MAX_BPM: f64 = 180.0;
/// Normalized autocorrelation peak below which the tempo is reported as unknown.
const TEMPO_CONFIDENCE: f64 = 0.15;
const TICK_SPACING: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioFeatures {
    pub tempo_bpm: Option<f64>,
    pub energy: f64,
    pub brightness: f64,
}

impl AudioFeatures {
    /// Compact vector for the diversifier: `[tempo or NaN, energy, brightness]`.
    pub fn vector(&self) -> [f32; 3] {
        [
            self.tempo_bpm.map_or(f32::NAN, |t| t as f32),
            self.energy as f32,
            self.brightness as f32,
        ]
    }
}

/// Distance between two feature vectors in 0..1. Tempo compares on a log scale
/// and forgives half/double time, since a 70 BPM ballad and a 140 BPM track
/// with the same pulse sit fine next to each other.
pub fn transition_distance(a: &[f32; 3], b: &[f32; 3]) -> f64 {
    let energy = (a[1] - b[1]).abs() as f64;
    let brightness = (a[2] - b[2]).abs() as f64;
    if a[0].is_finite() && b[0].is_finite() && a[0] > 0.0 && b[0] > 0.0 {
        let octave = (a[0] as f64 / b[0] as f64).log2();
        let tempo = [octave, octave - 1.0, octave + 1.0]
            .into_iter()
            .map(f64::abs)
            .fold(f64::INFINITY, f64::min);
        // ~15% tempo difference counts as fully different.
        let tempo = (tempo / 0.2).min(1.0);
        (0.5 * tempo + 0.35 * energy + 0.15 * brightness).clamp(0.0, 1.0)
    } else {
        (0.7 * energy + 0.3 * brightness).clamp(0.0, 1.0)
    }
}

/// Decodes up to 60 s from the middle of `path` and analyzes it.
pub fn analyze_file(path: &Path) -> AppResult<AudioFeatures> {
    let (samples, rate) = decode_middle_mono(path)?;
    analyze_samples(&samples, rate)
        .ok_or_else(|| AppError::Playback(format!("Too little audio in {}", path.display())))
}

fn decode_middle_mono(path: &Path) -> AppResult<(Vec<f32>, u32)> {
    let desc = path.display().to_string();
    let file = File::open(path)
        .map_err(|e| AppError::Playback(format!("Cannot open {desc}: {e}")))?;
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| AppError::Playback(format!("Audio format probe error for {desc}: {e}")))?;
    let track = probed
        .format
        .default_track()
        .filter(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .or_else(|| {
            probed
                .format
                .tracks()
                .iter()
                .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        })
        .ok_or_else(|| AppError::Playback(format!("No supported audio track in {desc}")))?;
    let track_id = track.id;
    let params = track.codec_params.clone();
    let mut decoder = symphonia::default::get_codecs()
        .make(&params, &DecoderOptions::default())
        .map_err(|e| AppError::Playback(format!("Failed to initialize decoder: {e}")))?;

    let duration = params
        .time_base
        .zip(params.n_frames)
        .map(|(base, frames)| {
            let t = base.calc_time(frames);
            t.seconds as f64 + t.frac
        });
    if let Some(total) = duration.filter(|total| *total > ANALYSIS_SECS + 10.0) {
        let start = (total - ANALYSIS_SECS) / 2.0;
        let seek = probed.format.seek(
            SeekMode::Coarse,
            SeekTo::Time {
                time: Time::new(start as u64, start.fract()),
                track_id: Some(track_id),
            },
        );
        if seek.is_ok() {
            decoder.reset();
        }
    }

    let mut mono = Vec::new();
    let mut rate = params.sample_rate.unwrap_or(44_100);
    let mut needed = (ANALYSIS_SECS * rate as f64) as usize;
    let mut buffer: Option<SampleBuffer<f32>> = None;
    let mut errors = 0;
    while mono.len() < needed {
        let packet = match probed.format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(_)) => break, // end of stream
            Err(SymphoniaError::ResetRequired) => break,
            Err(e) => return Err(AppError::Playback(format!("Packet read error: {e}"))),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(SymphoniaError::DecodeError(_)) if errors < 32 => {
                errors += 1;
                continue;
            }
            Err(e) => return Err(AppError::Playback(format!("Decode error: {e}"))),
        };
        let spec = *decoded.spec();
        if rate != spec.rate {
            rate = spec.rate;
            needed = (ANALYSIS_SECS * rate as f64) as usize;
        }
        let channels = spec.channels.count().max(1);
        let capacity = decoded.capacity() as u64;
        if buffer.as_ref().is_none_or(|b| b.capacity() < decoded.capacity() * channels) {
            buffer = Some(SampleBuffer::new(capacity, spec));
        }
        let buf = buffer.as_mut().expect("sample buffer allocated");
        buf.copy_interleaved_ref(decoded);
        for frame in buf.samples().chunks_exact(channels) {
            mono.push(frame.iter().sum::<f32>() / channels as f32);
        }
    }
    mono.truncate(needed);
    Ok((mono, rate))
}

/// Analyzes mono samples at `rate` Hz. `None` when shorter than ~5 s.
pub fn analyze_samples(samples: &[f32], rate: u32) -> Option<AudioFeatures> {
    if rate == 0 {
        return None;
    }
    let factor = (rate as f64 / TARGET_RATE as f64).round().max(1.0) as usize;
    let low_rate = rate as f64 / factor as f64;
    let low: Vec<f32> = samples
        .chunks_exact(factor)
        .map(|chunk| chunk.iter().sum::<f32>() / factor as f32)
        .collect();
    if (low.len() as f64) < low_rate * 5.0 {
        return None;
    }

    let mean_square = low.iter().map(|s| (*s as f64).powi(2)).sum::<f64>() / low.len() as f64;
    let db = 10.0 * mean_square.max(1e-12).log10();
    let energy = ((db + 40.0) / 40.0).clamp(0.0, 1.0);

    let crossings = low
        .windows(2)
        .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
        .count();
    let zcr = crossings as f64 / low.len() as f64;
    let brightness = (zcr / 0.25).clamp(0.0, 1.0);

    Some(AudioFeatures {
        tempo_bpm: estimate_tempo(&low, low_rate),
        energy: round3(energy),
        brightness: round3(brightness),
    })
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn estimate_tempo(samples: &[f32], rate: f64) -> Option<f64> {
    // Onset strength: positive change in log frame energy.
    let log_energy: Vec<f64> = samples
        .windows(FRAME)
        .step_by(HOP)
        .map(|frame| {
            let e = frame.iter().map(|s| (*s as f64).powi(2)).sum::<f64>() / FRAME as f64;
            (e + 1e-9).ln()
        })
        .collect();
    if log_energy.len() < 64 {
        return None;
    }
    let mut onset: Vec<f64> = log_energy
        .windows(2)
        .map(|w| (w[1] - w[0]).max(0.0))
        .collect();
    let mean = onset.iter().sum::<f64>() / onset.len() as f64;
    for value in &mut onset {
        *value -= mean;
    }
    let zero_lag: f64 = onset.iter().map(|v| v * v).sum();
    if zero_lag <= 1e-9 {
        return None;
    }
    let fps = rate / HOP as f64;
    let min_lag = (60.0 * fps / MAX_BPM).floor().max(1.0) as usize;
    let max_lag = (60.0 * fps / MIN_BPM).ceil() as usize;
    if max_lag + 2 >= onset.len() {
        return None;
    }
    let autocorr = |lag: usize| -> f64 {
        onset[..onset.len() - lag]
            .iter()
            .zip(&onset[lag..])
            .map(|(a, b)| a * b)
            .sum::<f64>()
            / zero_lag
    };
    let values: Vec<f64> = (min_lag - 1..=max_lag + 1).map(autocorr).collect();
    // Mild log-normal preference around 120 BPM breaks octave ties.
    let weighted = |i: usize| -> f64 {
        let lag = (min_lag - 1 + i) as f64;
        let bpm = 60.0 * fps / lag;
        values[i] * (-0.5 * (bpm / 120.0).log2().powi(2)).exp()
    };
    let best = (1..values.len() - 1).max_by(|&a, &b| weighted(a).total_cmp(&weighted(b)))?;
    let peak = values[best];
    if peak < TEMPO_CONFIDENCE {
        return None;
    }
    // Parabolic interpolation for sub-frame lag precision.
    let (left, right) = (values[best - 1], values[best + 1]);
    let curvature = left - 2.0 * peak + right;
    let offset = if curvature.abs() > 1e-12 {
        (0.5 * (left - right) / curvature).clamp(-0.5, 0.5)
    } else {
        0.0
    };
    let lag = (min_lag - 1 + best) as f64 + offset;
    let bpm = 60.0 * fps / lag;
    (MIN_BPM * 0.95..=MAX_BPM * 1.05)
        .contains(&bpm)
        .then(|| (bpm * 10.0).round() / 10.0)
}

/// All analyzed feature vectors, keyed by track id.
pub async fn load_vectors(pool: &SqlitePool) -> AppResult<HashMap<String, [f32; 3]>> {
    let rows: Vec<(String, Option<f64>, f64, f64)> = sqlx::query_as(
        "SELECT track_id, tempo_bpm, energy, brightness FROM track_audio_features
         WHERE energy IS NOT NULL AND brightness IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(id, tempo, energy, brightness)| {
            let features = AudioFeatures {
                tempo_bpm: tempo,
                energy,
                brightness,
            };
            (id, features.vector())
        })
        .collect())
}

/// Background analyzer: one local track per tick, yielding to playback start.
pub struct AudioFeaturesService {
    pool: SqlitePool,
    bus: Arc<EventBus>,
    wake: Notify,
}

impl AudioFeaturesService {
    pub fn new(pool: SqlitePool, bus: Arc<EventBus>) -> Self {
        Self {
            pool,
            bus,
            wake: Notify::new(),
        }
    }

    pub fn wake(&self) {
        self.wake.notify_one();
    }

    pub async fn run(self: Arc<Self>) {
        let mut events = self.bus.subscribe();
        let mut busy_until = tokio::time::Instant::now() + Duration::from_secs(45);
        loop {
            let work = async {
                tokio::time::sleep_until(busy_until).await;
                self.tick().await
            };
            tokio::select! {
                result = work => match result {
                    Ok(true) => { tokio::time::sleep(TICK_SPACING).await; continue; }
                    Ok(false) => (),
                    Err(e) => tracing::warn!(error = %e, "Audio feature worker will retry"),
                },
                event = relevant_event(&mut events) => {
                    match event {
                        Ok(Event::PlaybackBuffering { .. } | Event::OnlinePlaybackRequested { .. }) => busy_until = tokio::time::Instant::now() + Duration::from_secs(20),
                        Ok(Event::PlaybackStarted { .. } | Event::PlaybackError { .. }) => busy_until = tokio::time::Instant::now() + Duration::from_secs(5),
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        _ => (),
                    }
                    continue;
                }
                _ = self.wake.notified() => continue,
            }
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(1800)) => (),
                _ = self.wake.notified() => (),
                event = relevant_event(&mut events) => match event {
                    Ok(Event::LibraryScanCompleted { .. }) => (),
                    Err(_) if self.pool.is_closed() => break,
                    _ => continue,
                },
            }
        }
    }

    /// Analyzes the next pending local track. `Ok(true)` means more may be pending.
    pub async fn tick(&self) -> AppResult<bool> {
        let next: Option<(String, String)> = sqlx::query_as(
            "SELECT t.id, t.file_path FROM tracks t
             LEFT JOIN track_audio_features f ON f.track_id = t.id
             WHERE (f.track_id IS NULL OR f.analyzer_version < ?)
               AND t.file_path NOT LIKE '%://%'
             ORDER BY t.id LIMIT 1",
        )
        .bind(ANALYZER_VERSION)
        .fetch_optional(&self.pool)
        .await?;
        let Some((track_id, file_path)) = next else {
            return Ok(false);
        };
        let path = std::path::PathBuf::from(&file_path);
        let analysis = tokio::task::spawn_blocking(move || analyze_file(&path))
            .await
            .map_err(|e| AppError::Playback(format!("Audio analysis task failed: {e}")))?;
        let features = match analysis {
            Ok(features) => Some(features),
            Err(e) => {
                tracing::debug!(track_id = %track_id, error = %e, "Audio features unavailable");
                None
            }
        };
        store(&self.pool, &track_id, features.as_ref()).await?;
        Ok(true)
    }
}

/// Persists features (or an "unanalyzable" marker) for the current analyzer version.
pub async fn store(
    pool: &SqlitePool,
    track_id: &str,
    features: Option<&AudioFeatures>,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO track_audio_features
         (track_id, tempo_bpm, energy, brightness, analyzer_version, analyzed_at)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(track_id) DO UPDATE SET
           tempo_bpm = excluded.tempo_bpm, energy = excluded.energy,
           brightness = excluded.brightness, analyzer_version = excluded.analyzer_version,
           analyzed_at = excluded.analyzed_at",
    )
    .bind(track_id)
    .bind(features.and_then(|f| f.tempo_bpm))
    .bind(features.map(|f| f.energy))
    .bind(features.map(|f| f.brightness))
    .bind(ANALYZER_VERSION)
    .bind(chrono::Utc::now().timestamp())
    .execute(pool)
    .await?;
    Ok(())
}

async fn relevant_event(
    events: &mut tokio::sync::broadcast::Receiver<Event>,
) -> std::result::Result<Event, tokio::sync::broadcast::error::RecvError> {
    loop {
        match events.recv().await {
            Ok(
                e @ (Event::PlaybackBuffering { .. }
                | Event::OnlinePlaybackRequested { .. }
                | Event::PlaybackStarted { .. }
                | Event::PlaybackError { .. }
                | Event::LibraryScanCompleted { .. }),
            ) => return Ok(e),
            Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(e) => return Err(e),
        }
    }
}
