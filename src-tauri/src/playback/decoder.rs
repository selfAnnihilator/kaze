use std::fs::File;
use std::path::Path;
use std::time::Duration;
use rodio::source::Source;
use symphonia::core::audio::{SampleBuffer, SignalSpec};
use symphonia::core::codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units;
use crate::core::error::{AppError, AppResult};

pub struct SymphoniaSource {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    buffer: SampleBuffer<i16>,
    current_frame_offset: usize,
    spec: SignalSpec,
    total_duration: Option<Duration>,
}

impl SymphoniaSource {
    pub fn new(file_path: &Path) -> AppResult<Self> {
        let file = File::open(file_path).map_err(|e| {
            AppError::Playback(format!("Failed to open file for playback: {}", e))
        })?;

        // Std File implements symphonia's MediaSource directly and provides accurate byte_len,
        // avoiding Rodio's ReadSeekSource bug which drops byte_len and panics on ISO-MP4/M4A streams.
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            if ext != "audio" {
                hint.with_extension(ext);
            }
        }

        let format_opts = FormatOptions {
            enable_gapless: true,
            ..Default::default()
        };
        let metadata_opts: MetadataOptions = Default::default();

        let mut probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .map_err(|e| AppError::Playback(format!("Audio format probe error for {}: {}", file_path.display(), e)))?;

        let stream = match probed.format.default_track() {
            Some(s) => s,
            None => probed
                .format
                .tracks()
                .iter()
                .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
                .ok_or_else(|| AppError::Playback(format!("No supported audio track in {}", file_path.display())))?,
        };

        let track_id = stream.id;
        let mut decoder = symphonia::default::get_codecs()
            .make(&stream.codec_params, &DecoderOptions::default())
            .map_err(|e| AppError::Playback(format!("Failed to initialize audio decoder: {}", e)))?;

        let total_duration = stream
            .codec_params
            .time_base
            .zip(stream.codec_params.n_frames)
            .map(|(base, frames)| {
                let time = base.calc_time(frames);
                Duration::from_secs_f64(time.seconds as f64 + time.frac)
            });

        // Decode first packet to prime the buffer and obtain audio signal spec
        let mut decode_retries = 0;
        let decoded = loop {
            let current_frame = match probed.format.next_packet() {
                Ok(packet) => packet,
                Err(Error::IoError(e)) => return Err(AppError::Playback(format!("IO error reading packet: {}", e))),
                Err(e) => return Err(AppError::Playback(format!("Packet read error: {}", e))),
            };

            if current_frame.track_id() != track_id {
                continue;
            }

            match decoder.decode(&current_frame) {
                Ok(decoded) => break decoded,
                Err(Error::DecodeError(_)) => {
                    decode_retries += 1;
                    if decode_retries > 5 {
                        return Err(AppError::Playback("Too many decode errors initializing audio stream".into()));
                    }
                    continue;
                }
                Err(e) => return Err(AppError::Playback(format!("Audio decode error: {}", e))),
            }
        };

        let spec = decoded.spec().to_owned();
        let duration = units::Duration::from(decoded.capacity() as u64);
        let mut buffer = SampleBuffer::<i16>::new(duration, spec);
        buffer.copy_interleaved_ref(decoded);

        Ok(Self {
            format: probed.format,
            decoder,
            track_id,
            buffer,
            current_frame_offset: 0,
            spec,
            total_duration,
        })
    }
}

impl Iterator for SymphoniaSource {
    type Item = i16;

    #[inline]
    fn next(&mut self) -> Option<i16> {
        if self.current_frame_offset >= self.buffer.len() {
            let mut new_buffer = None;
            for _ in 0..5 {
                let packet = self.format.next_packet().ok()?;
                if packet.track_id() != self.track_id {
                    continue;
                }
                if let Ok(decoded) = self.decoder.decode(&packet) {
                    decoded.spec().clone_into(&mut self.spec);
                    let duration = units::Duration::from(decoded.capacity() as u64);
                    let mut buffer = SampleBuffer::<i16>::new(duration, self.spec);
                    buffer.copy_interleaved_ref(decoded);
                    new_buffer = Some(buffer);
                    break;
                }
            }

            self.buffer = new_buffer?;
            self.current_frame_offset = 0;
        }

        let sample = *self.buffer.samples().get(self.current_frame_offset)?;
        self.current_frame_offset += 1;
        Some(sample)
    }
}

impl Source for SymphoniaSource {
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.buffer.samples().len().saturating_sub(self.current_frame_offset))
    }

    #[inline]
    fn channels(&self) -> u16 {
        self.spec.channels.count() as u16
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.spec.rate
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        self.total_duration
    }
}

pub enum PlayerSource {
    Symphonia(SymphoniaSource),
    Rodio(rodio::Decoder<std::io::BufReader<std::fs::File>>),
}

impl Iterator for PlayerSource {
    type Item = i16;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Symphonia(s) => s.next(),
            Self::Rodio(s) => s.next(),
        }
    }
}

impl Source for PlayerSource {
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        match self {
            Self::Symphonia(s) => s.current_frame_len(),
            Self::Rodio(s) => s.current_frame_len(),
        }
    }

    #[inline]
    fn channels(&self) -> u16 {
        match self {
            Self::Symphonia(s) => s.channels(),
            Self::Rodio(s) => s.channels(),
        }
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        match self {
            Self::Symphonia(s) => s.sample_rate(),
            Self::Rodio(s) => s.sample_rate(),
        }
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        match self {
            Self::Symphonia(s) => s.total_duration(),
            Self::Rodio(s) => s.total_duration(),
        }
    }
}
