//! Reading audio files and converting them to the device's sample rate.
//!
//! This runs on worker threads, one per playing cue, so it may allocate and
//! block. It writes into a ring buffer that the real-time mixer reads.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use rtrb::Producer;
use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Indexing, Resampler};
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::Time;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    /// The file is not where the show says it is.
    #[error("can't find {0}")]
    NotFound(String),
    /// The file exists but cannot be read as audio.
    #[error("can't read {0} as audio")]
    Unsupported(String),
    #[error("resampling failed: {0}")]
    Resample(String),
}

/// What a file contains, after reducing it to mono or stereo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Info {
    pub sample_rate: u32,
    /// 1 or 2. Wider files are reduced to their first two channels.
    pub channels: usize,
    /// Frames in the file at its own sample rate, when the container says.
    pub total_frames: Option<u64>,
}

/// An open audio file.
pub struct Decoder {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track_id: u32,
    info: Info,
    interleaved: Vec<f32>,
    /// Frames to discard because a seek landed before the requested point.
    skip_frames: u64,
}

impl Decoder {
    pub fn open(path: &Path) -> Result<Self, DecodeError> {
        let shown = path.display().to_string();
        let file = File::open(path).map_err(|_| DecodeError::NotFound(shown.clone()))?;
        let stream = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(extension);
        }
        let format = symphonia::default::get_probe()
            .probe(
                &hint,
                stream,
                FormatOptions::default(),
                MetadataOptions::default(),
            )
            .map_err(|_| DecodeError::Unsupported(shown.clone()))?;
        let unsupported = || DecodeError::Unsupported(shown.clone());
        let track = format
            .default_track(TrackType::Audio)
            .ok_or_else(unsupported)?;
        let params = track
            .codec_params
            .as_ref()
            .and_then(|p| p.audio())
            .ok_or_else(unsupported)?;
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(params, &AudioDecoderOptions::default())
            .map_err(|_| unsupported())?;
        let sample_rate = params.sample_rate.ok_or_else(unsupported)?;
        let channels = params
            .channels
            .as_ref()
            .map_or(2, |c| c.count())
            .clamp(1, 2);
        let info = Info {
            sample_rate,
            channels,
            total_frames: track.num_frames,
        };
        let track_id = track.id;
        Ok(Self {
            format,
            decoder,
            track_id,
            info,
            interleaved: Vec::new(),
            skip_frames: 0,
        })
    }

    pub fn info(&self) -> Info {
        self.info
    }

    /// Moves to `seconds` from the start of the file.
    pub fn seek(&mut self, seconds: f64) -> Result<(), DecodeError> {
        let time = Time::try_from_secs_f64(seconds.max(0.0))
            .ok_or_else(|| DecodeError::Unsupported("seek position".into()))?;
        let landed = self
            .format
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time,
                    track_id: Some(self.track_id),
                },
            )
            .map_err(|_| DecodeError::Unsupported("seeking".into()))?;
        self.decoder.reset();
        // Containers often land on a packet boundary before the target. The
        // track's ticks are sample frames for the formats we support, so the
        // gap is the number of frames to throw away.
        self.skip_frames = landed
            .required_ts
            .get()
            .saturating_sub(landed.actual_ts.get())
            .max(0) as u64;
        Ok(())
    }

    /// Appends the next packet's frames to `out` (interleaved, 1 or 2
    /// channels). Returns `false` at the end of the file.
    pub fn read(&mut self, out: &mut Vec<f32>) -> Result<bool, DecodeError> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(Some(packet)) => packet,
                Ok(None) => return Ok(false),
                Err(SymphoniaError::IoError(_)) => return Ok(false),
                Err(_) => return Err(DecodeError::Unsupported("stream".into())),
            };
            if packet.track_id != self.track_id {
                continue;
            }
            let decoded = match self.decoder.decode(&packet) {
                Ok(decoded) => decoded,
                // A damaged packet costs a moment of audio, not the cue.
                Err(SymphoniaError::DecodeError(_) | SymphoniaError::IoError(_)) => continue,
                Err(_) => return Err(DecodeError::Unsupported("stream".into())),
            };
            let source_channels = decoded.spec().channels().count().max(1);
            self.interleaved.clear();
            decoded.copy_to_vec_interleaved::<f32>(&mut self.interleaved);
            let wanted = self.info.channels;
            let base = out.len();
            if source_channels == wanted {
                out.extend_from_slice(&self.interleaved);
            } else {
                for frame in self.interleaved.chunks_exact(source_channels) {
                    out.extend_from_slice(&frame[..wanted]);
                }
            }
            if self.skip_frames > 0 {
                let got = ((out.len() - base) / wanted) as u64;
                let drop = self.skip_frames.min(got);
                out.drain(base..base + drop as usize * wanted);
                self.skip_frames -= drop;
            }
            return Ok(true);
        }
    }
}

/// Converts a stream from one sample rate to another, a chunk at a time.
pub struct StreamResampler {
    inner: Option<Inner>,
}

struct Inner {
    resampler: Fft<f32>,
    channels: usize,
    pending: Vec<f32>,
    scratch: Vec<f32>,
    /// Output frames still to drop: the resampler's start-up delay.
    skip: usize,
    ratio: f64,
    in_frames: u64,
    out_frames: u64,
}

const RESAMPLE_CHUNK: usize = 1024;

impl StreamResampler {
    /// Passes audio through untouched when the rates already match.
    pub fn new(from: u32, to: u32, channels: usize) -> Result<Self, DecodeError> {
        if from == to {
            return Ok(Self { inner: None });
        }
        let resampler = Fft::<f32>::new(
            from as usize,
            to as usize,
            RESAMPLE_CHUNK,
            channels,
            FixedSync::Input,
        )
        .map_err(|e| DecodeError::Resample(e.to_string()))?;
        let skip = resampler.output_delay();
        Ok(Self {
            inner: Some(Inner {
                resampler,
                channels,
                pending: Vec::new(),
                scratch: Vec::new(),
                skip,
                ratio: f64::from(to) / f64::from(from),
                in_frames: 0,
                out_frames: 0,
            }),
        })
    }

    pub fn push(&mut self, input: &[f32], out: &mut Vec<f32>) -> Result<(), DecodeError> {
        let Some(inner) = self.inner.as_mut() else {
            out.extend_from_slice(input);
            return Ok(());
        };
        inner.in_frames += (input.len() / inner.channels) as u64;
        inner.pending.extend_from_slice(input);
        while inner.pending.len() / inner.channels >= inner.resampler.input_frames_next() {
            inner.run_chunk(None, out)?;
        }
        Ok(())
    }

    /// Flushes what is left, so the output has exactly the expected length.
    pub fn finish(&mut self, out: &mut Vec<f32>) -> Result<(), DecodeError> {
        let Some(inner) = self.inner.as_mut() else {
            return Ok(());
        };
        let remaining = inner.pending.len() / inner.channels;
        if remaining > 0 {
            inner.run_chunk(Some(remaining), out)?;
        }
        let expected = (inner.ratio * inner.in_frames as f64).round() as u64;
        // Silence pushes the last real samples out through the filter.
        let mut guard = 0;
        while inner.out_frames < expected && guard < 16 {
            inner.run_chunk(Some(0), out)?;
            guard += 1;
        }
        // Drop anything beyond the exact length.
        let excess = inner.out_frames.saturating_sub(expected) as usize;
        out.truncate(out.len().saturating_sub(excess * inner.channels));
        inner.out_frames = expected.min(inner.out_frames);
        Ok(())
    }
}

impl Inner {
    fn run_chunk(&mut self, partial: Option<usize>, out: &mut Vec<f32>) -> Result<(), DecodeError> {
        let channels = self.channels;
        let need = self.resampler.input_frames_next();
        let produce = self.resampler.output_frames_next();
        self.pending
            .resize(self.pending.len().max(need * channels), 0.0);
        self.scratch.clear();
        self.scratch.resize(produce * channels, 0.0);
        let mut indexing = Indexing::new();
        if let Some(valid) = partial {
            indexing = indexing.partial_len(valid);
        }
        let consumed_frames = partial.unwrap_or(need);
        let (_, written) = {
            let input = InterleavedSlice::new(&self.pending[..need * channels], channels, need)
                .map_err(|e| DecodeError::Resample(e.to_string()))?;
            let mut output = InterleavedSlice::new_mut(&mut self.scratch, channels, produce)
                .map_err(|e| DecodeError::Resample(e.to_string()))?;
            self.resampler
                .process_into_buffer(&input, &mut output, Some(&indexing))
                .map_err(|e| DecodeError::Resample(e.to_string()))?
        };
        self.pending
            .drain(..(consumed_frames * channels).min(self.pending.len()));
        let mut produced = &self.scratch[..written * channels];
        let drop_frames = self.skip.min(produced.len() / channels);
        self.skip -= drop_frames;
        produced = &produced[drop_frames * channels..];
        out.extend_from_slice(produced);
        self.out_frames += (produced.len() / channels) as u64;
        Ok(())
    }
}

/// How to play a file: which part, how often, and at what rate.
#[derive(Debug, Clone, PartialEq)]
pub struct StreamPlan {
    pub path: PathBuf,
    pub start: f64,
    pub end: Option<f64>,
    /// Total plays; `None` means forever.
    pub plays: Option<u32>,
    pub device_rate: u32,
}

/// Decodes `plan` into `producer` until the end, a cancel, or an error. Sets
/// `eof` when everything has been written, whether it ended normally or not.
pub fn run_stream(
    mut decoder: Decoder,
    plan: &StreamPlan,
    mut producer: Producer<f32>,
    eof: &AtomicBool,
    cancel: &AtomicBool,
) {
    if let Err(error) = stream(&mut decoder, plan, &mut producer, cancel) {
        tracing::warn!(%error, file = %plan.path.display(), "stopped decoding");
    }
    eof.store(true, Ordering::Release);
}

fn stream(
    decoder: &mut Decoder,
    plan: &StreamPlan,
    producer: &mut Producer<f32>,
    cancel: &AtomicBool,
) -> Result<(), DecodeError> {
    let info = decoder.info();
    let channels = info.channels;
    let limit_frames = plan
        .end
        .map(|end| ((end - plan.start).max(0.0) * f64::from(info.sample_rate)).round() as u64);
    let mut plays_done = 0u32;
    let mut raw = Vec::new();
    let mut converted = Vec::new();
    loop {
        if plays_done > 0 || plan.start > 0.0 {
            decoder.seek(plan.start)?;
        }
        let mut resampler = StreamResampler::new(info.sample_rate, plan.device_rate, channels)?;
        let mut read_frames = 0u64;
        loop {
            raw.clear();
            if !decoder.read(&mut raw)? {
                break;
            }
            if let Some(limit) = limit_frames {
                let keep = limit.saturating_sub(read_frames) as usize;
                raw.truncate(keep * channels);
            }
            read_frames += (raw.len() / channels) as u64;
            converted.clear();
            resampler.push(&raw, &mut converted)?;
            if !write_all(producer, &converted, cancel) {
                return Ok(());
            }
            if limit_frames.is_some_and(|limit| read_frames >= limit) {
                break;
            }
        }
        converted.clear();
        resampler.finish(&mut converted)?;
        if !write_all(producer, &converted, cancel) {
            return Ok(());
        }
        plays_done += 1;
        if plan.plays.is_some_and(|plays| plays_done >= plays) || read_frames == 0 {
            return Ok(());
        }
    }
}

/// Writes every sample, waiting for room. Returns `false` if cancelled.
fn write_all(producer: &mut Producer<f32>, data: &[f32], cancel: &AtomicBool) -> bool {
    let mut written = 0;
    while written < data.len() {
        if cancel.load(Ordering::Relaxed) {
            return false;
        }
        let room = producer.slots();
        if room == 0 {
            std::thread::sleep(Duration::from_millis(2));
            continue;
        }
        let n = room.min(data.len() - written);
        for sample in &data[written..written + n] {
            let _ = producer.push(*sample);
        }
        written += n;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_wav(dir: &Path, name: &str, rate: u32, channels: u16, samples: &[i16]) -> PathBuf {
        let path = dir.join(name);
        let spec = hound::WavSpec {
            channels,
            sample_rate: rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        for s in samples {
            writer.write_sample(*s).unwrap();
        }
        writer.finalize().unwrap();
        path
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("selah-audio-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sine(rate: u32, seconds: f64, hz: f64) -> Vec<i16> {
        (0..(f64::from(rate) * seconds) as usize)
            .map(|i| {
                let t = i as f64 / f64::from(rate);
                ((t * hz * std::f64::consts::TAU).sin() * 16_000.0) as i16
            })
            .collect()
    }

    fn decode_all(path: &Path) -> (Info, Vec<f32>) {
        let mut decoder = Decoder::open(path).unwrap();
        let mut out = Vec::new();
        while decoder.read(&mut out).unwrap() {}
        (decoder.info(), out)
    }

    fn collect(plan: &StreamPlan) -> Vec<f32> {
        let decoder = Decoder::open(&plan.path).unwrap();
        let (producer, mut consumer) = rtrb::RingBuffer::new(4_000_000);
        let eof = AtomicBool::new(false);
        run_stream(decoder, plan, producer, &eof, &AtomicBool::new(false));
        assert!(eof.load(Ordering::Acquire));
        let mut out = Vec::new();
        while let Ok(s) = consumer.pop() {
            out.push(s);
        }
        out
    }

    fn plan(path: PathBuf, rate: u32) -> StreamPlan {
        StreamPlan {
            path,
            start: 0.0,
            end: None,
            plays: Some(1),
            device_rate: rate,
        }
    }

    #[test]
    fn reads_a_wav_file() {
        let dir = temp_dir("read");
        let path = write_wav(&dir, "a.wav", 44_100, 1, &sine(44_100, 0.5, 440.0));
        let (info, samples) = decode_all(&path);
        assert_eq!(info.sample_rate, 44_100);
        assert_eq!(info.channels, 1);
        assert_eq!(samples.len(), 22_050);
        assert!(samples.iter().any(|s| s.abs() > 0.4));
    }

    #[test]
    fn a_missing_file_and_a_non_audio_file_are_told_apart() {
        let dir = temp_dir("errors");
        assert!(matches!(
            Decoder::open(&dir.join("nope.wav")),
            Err(DecodeError::NotFound(_))
        ));
        let text = dir.join("notes.wav");
        std::fs::write(&text, "this is not audio at all").unwrap();
        assert!(matches!(
            Decoder::open(&text),
            Err(DecodeError::Unsupported(_))
        ));
    }

    #[test]
    fn the_same_rate_passes_straight_through() {
        let dir = temp_dir("same");
        let src = sine(48_000, 0.25, 440.0);
        let path = write_wav(&dir, "a.wav", 48_000, 1, &src);
        let out = collect(&plan(path, 48_000));
        assert_eq!(out.len(), src.len());
    }

    #[test]
    fn resampling_gives_exactly_the_expected_length_and_keeps_the_pitch() {
        let dir = temp_dir("resample");
        let path = write_wav(&dir, "a.wav", 44_100, 1, &sine(44_100, 1.0, 1000.0));
        let out = collect(&plan(path, 48_000));
        assert_eq!(out.len(), 48_000);
        // Count rising zero crossings in the middle: about 1000 per second.
        let middle = &out[12_000..36_000];
        let crossings = middle
            .windows(2)
            .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
            .count();
        assert!((crossings as i32 - 500).abs() <= 2, "{crossings}");
    }

    #[test]
    fn start_and_end_trim_the_region() {
        let dir = temp_dir("trim");
        let src: Vec<i16> = (0..48_000).map(|i| (i / 48) as i16).collect();
        let path = write_wav(&dir, "ramp.wav", 48_000, 1, &src);
        let mut p = plan(path, 48_000);
        p.start = 0.25;
        p.end = Some(0.5);
        let out = collect(&p);
        assert_eq!(out.len(), 12_000);
        let first = (out[0] * 32768.0).round() as i32;
        assert!(
            (first - 250).abs() <= 2,
            "starts near the 0.25 s mark: {first}"
        );
    }

    #[test]
    fn loops_repeat_the_region() {
        let dir = temp_dir("loops");
        let path = write_wav(&dir, "a.wav", 48_000, 1, &sine(48_000, 0.1, 440.0));
        let mut p = plan(path, 48_000);
        p.plays = Some(3);
        assert_eq!(collect(&p).len(), 3 * 4_800);
    }

    #[test]
    fn wide_files_are_reduced_to_two_channels() {
        let dir = temp_dir("wide");
        let frames = 100;
        let samples: Vec<i16> = (0..frames).flat_map(|_| [1000, 2000, 3000, 4000]).collect();
        let path = write_wav(&dir, "four.wav", 48_000, 4, &samples);
        let (info, out) = decode_all(&path);
        assert_eq!(info.channels, 2);
        assert_eq!(out.len(), frames * 2);
        assert!((out[0] * 32768.0 - 1000.0).abs() < 2.0);
        assert!((out[1] * 32768.0 - 2000.0).abs() < 2.0);
    }

    #[test]
    fn cancelling_stops_the_stream_and_still_sets_eof() {
        let dir = temp_dir("cancel");
        let path = write_wav(&dir, "a.wav", 48_000, 1, &sine(48_000, 2.0, 440.0));
        let decoder = Decoder::open(&path).unwrap();
        let (producer, _consumer) = rtrb::RingBuffer::new(1_000);
        let (eof, cancel) = (AtomicBool::new(false), AtomicBool::new(true));
        run_stream(decoder, &plan(path, 48_000), producer, &eof, &cancel);
        assert!(eof.load(Ordering::Acquire));
    }
}
