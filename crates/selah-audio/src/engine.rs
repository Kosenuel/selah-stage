//! The control-side handle to the audio engine.
//!
//! It implements [`AudioSink`], so the cue engine drives it directly. Nothing
//! here runs in the audio callback; everything that must is in [`crate::mixer`].

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use rtrb::{Consumer, Producer, RingBuffer};
use selah_core::{CueId, Decibels, FadeCurve, PatchId, Seconds};
use selah_engine::{AudioPlay, AudioSink, Clock, SinkError};

use crate::decode::{DecodeError, Decoder, StreamPlan, run_stream};
use crate::fader::db_to_linear;
use crate::mixer::{Command, MAX_VOICES, Meters, Mixer, Voice, VoiceEnd, VoiceParams};
use crate::routing::build_matrix;

/// What the audio side tells the host. Forward `Finished` to the cue engine's
/// `audio_finished`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioEvent {
    /// A cue played to its end.
    Finished(CueId),
    /// A cue was stopped, or refused because every voice was busy.
    Stopped(CueId),
}

/// How much decoded audio each voice may hold ahead of playback.
const BUFFER_SECONDS: usize = 2;
/// How much must be buffered before a voice starts sounding.
const PREBUFFER_SECONDS: f64 = 0.1;

/// Time counted in played audio frames. Hand it to the cue engine so that
/// waits and fades are tied to the sound, never to the system clock.
#[derive(Clone)]
pub struct AudioClock {
    meters: Arc<Meters>,
    sample_rate: u32,
}

impl Clock for AudioClock {
    fn now(&self) -> Seconds {
        self.meters.frames_rendered() as f64 / f64::from(self.sample_rate)
    }
}

pub struct AudioEngine {
    sample_rate: u32,
    device_channels: usize,
    commands: Producer<Command>,
    ended: Consumer<VoiceEnd>,
    garbage: Consumer<Box<Voice>>,
    meters: Arc<Meters>,
    patch_map: HashMap<PatchId, Vec<u16>>,
    default_outputs: Vec<u16>,
    media_root: PathBuf,
    /// Cancel flags of the decode threads, by cue.
    ///
    /// Known gap: if a cue is restarted in the same audio block in which its
    /// previous run ends, that end can be attributed to the new run.
    workers: HashMap<CueId, Arc<AtomicBool>>,
}

impl AudioEngine {
    /// Creates the handle and the mixer for a device with the given rate and
    /// channel count. Hand the mixer to a backend.
    pub fn new(sample_rate: u32, device_channels: usize) -> (Self, Mixer) {
        let (commands, command_rx) = RingBuffer::new(256);
        let (ended_tx, ended) = RingBuffer::new(256);
        let (garbage_tx, garbage) = RingBuffer::new(MAX_VOICES * 4);
        let meters = Arc::new(Meters::new());
        let mixer = Mixer::new(
            sample_rate,
            command_rx,
            ended_tx,
            garbage_tx,
            meters.clone(),
        );
        let engine = Self {
            sample_rate,
            device_channels,
            commands,
            ended,
            garbage,
            meters,
            patch_map: HashMap::new(),
            default_outputs: default_outputs(device_channels),
            media_root: PathBuf::new(),
            workers: HashMap::new(),
        };
        (engine, mixer)
    }

    /// For a new device after the old one was lost or changed. Everything that
    /// was playing is cancelled; the host should stop the cue engine's cues.
    pub fn reattach(&mut self, sample_rate: u32, device_channels: usize) -> Mixer {
        for cancel in self.workers.values() {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        let patch_map = std::mem::take(&mut self.patch_map);
        let media_root = std::mem::take(&mut self.media_root);
        let (mut fresh, mixer) = Self::new(sample_rate, device_channels);
        fresh.patch_map = patch_map;
        fresh.media_root = media_root;
        *self = fresh;
        mixer
    }

    /// Folder that cue file paths are relative to (the show folder).
    pub fn set_media_root(&mut self, root: impl Into<PathBuf>) {
        self.media_root = root.into();
    }

    /// Which device channels each patch uses on this machine.
    pub fn set_patch_map(&mut self, map: HashMap<PatchId, Vec<u16>>) {
        self.patch_map = map;
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn device_channels(&self) -> usize {
        self.device_channels
    }

    /// The master clock: time as counted by the frames the device has played.
    pub fn clock(&self) -> AudioClock {
        AudioClock {
            meters: self.meters.clone(),
            sample_rate: self.sample_rate,
        }
    }

    pub fn meters(&self) -> Arc<Meters> {
        self.meters.clone()
    }

    /// Peak level per device channel since the last call.
    pub fn peaks(&self) -> Vec<f32> {
        self.meters.take_peaks(self.device_channels)
    }

    /// Collects what the audio thread has reported and frees finished voices.
    /// Call this regularly from the control thread.
    pub fn poll(&mut self) -> Vec<AudioEvent> {
        let mut events = Vec::new();
        while let Ok(end) = self.ended.pop() {
            let (cue, event) = match end {
                VoiceEnd::Finished(cue) => (cue, AudioEvent::Finished(cue)),
                VoiceEnd::Stopped(cue) => (cue, AudioEvent::Stopped(cue)),
            };
            if let Some(cancel) = self.workers.remove(&cue) {
                cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            events.push(event);
        }
        // Voices are dropped here, off the audio thread.
        while self.garbage.pop().is_ok() {}
        events
    }

    fn frames(&self, seconds: Seconds) -> u32 {
        (seconds.max(0.0) * f64::from(self.sample_rate))
            .round()
            .min(f64::from(u32::MAX)) as u32
    }

    fn send(&mut self, command: Command) -> bool {
        self.commands.push(command).is_ok()
    }
}

fn default_outputs(device_channels: usize) -> Vec<u16> {
    if device_channels >= 2 {
        vec![0, 1]
    } else {
        vec![0]
    }
}

impl AudioSink for AudioEngine {
    fn start(&mut self, cue: CueId, play: &AudioPlay) -> Result<(), SinkError> {
        let path = self.media_root.join(&play.file);
        let decoder = Decoder::open(&path).map_err(|error| match error {
            DecodeError::NotFound(_) => SinkError::MissingMedia(play.file.clone()),
            other => SinkError::Failed(other.to_string()),
        })?;
        let info = decoder.info();
        if let Some(previous) = self.workers.remove(&cue) {
            previous.store(true, std::sync::atomic::Ordering::Relaxed);
        }

        let plays = match play.loops {
            None => Some(1),
            Some(0) => None,
            Some(n) => Some(n),
        };
        let region_seconds = play
            .end
            .or_else(|| {
                info.total_frames
                    .map(|f| f as f64 / f64::from(info.sample_rate))
            })
            .map(|end| (end - play.start).max(0.0));
        let total_frames = match (plays, region_seconds) {
            (Some(1), Some(seconds)) => {
                Some((seconds * f64::from(self.sample_rate)).round() as u64)
            }
            _ => None,
        };

        let patch_lists: Vec<&[u16]> = play
            .patches
            .iter()
            .filter_map(|p| self.patch_map.get(p))
            .map(Vec::as_slice)
            .collect();
        let outs: Vec<&[u16]> = if patch_lists.is_empty() {
            vec![self.default_outputs.as_slice()]
        } else {
            patch_lists
        };
        let matrix = build_matrix(
            info.channels,
            &outs,
            db_to_linear(play.level),
            self.device_channels,
        );

        let capacity = self.sample_rate as usize * info.channels * BUFFER_SECONDS;
        let (producer, consumer) = RingBuffer::new(capacity);
        let prebuffer = ((f64::from(self.sample_rate) * PREBUFFER_SECONDS) as usize
            * info.channels)
            .min(capacity / 2);
        let eof = Arc::new(AtomicBool::new(false));
        let cancel = Arc::new(AtomicBool::new(false));
        let voice = Voice::new(
            VoiceParams {
                cue,
                channels: info.channels,
                matrix,
                fade_in_frames: self.frames(play.fade_in),
                fade_out_frames: self.frames(play.fade_out),
                total_frames,
                curve: play.curve,
                prebuffer_samples: prebuffer,
            },
            consumer,
            eof.clone(),
        );
        let stream_plan = StreamPlan {
            path,
            start: play.start,
            end: play.end,
            plays,
            device_rate: self.sample_rate,
        };
        let worker_cancel = cancel.clone();
        std::thread::Builder::new()
            .name("selah-decode".into())
            .spawn(move || run_stream(decoder, &stream_plan, producer, &eof, &worker_cancel))
            .map_err(|e| SinkError::Failed(e.to_string()))?;

        if self.send(Command::Start(Box::new(voice))) {
            self.workers.insert(cue, cancel);
            Ok(())
        } else {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            Err(SinkError::Failed("the audio engine is busy".into()))
        }
    }

    fn stop(&mut self, cue: CueId, fade: Seconds) {
        let frames = self.frames(fade);
        self.send(Command::Stop { cue, frames });
    }

    fn pause(&mut self, cue: CueId) {
        self.send(Command::Pause(cue));
    }

    fn resume(&mut self, cue: CueId) {
        self.send(Command::Resume(cue));
    }

    fn fade(&mut self, cue: CueId, to: Decibels, duration: Seconds, curve: FadeCurve) {
        let frames = self.frames(duration);
        self.send(Command::Fade {
            cue,
            to: db_to_linear(to),
            frames,
            curve,
        });
    }

    fn stop_all(&mut self, fade: Seconds) {
        let frames = self.frames(fade);
        self.send(Command::StopAll { frames });
    }
}
