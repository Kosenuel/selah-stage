//! The real-time core. [`Mixer::render`] runs inside the audio callback:
//! **no allocation, no locks, no logging, no panics, no I/O.**
//!
//! Voices arrive through a lock-free queue as boxes built on another thread.
//! A finished voice is handed back through a second queue so that it is
//! *dropped* on a control thread, never here.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use rtrb::{Consumer, Producer};
use selah_core::{CueId, FadeCurve};

use crate::fader::{Ramp, ramp_value};

pub const MAX_VOICES: usize = 32;
pub const MAX_DEVICE_CHANNELS: usize = 32;
/// Voices carry mono or stereo; wider sources are reduced when decoded.
pub const MAX_SOURCE_CHANNELS: usize = 2;

/// How a voice's source channels reach the device channels.
pub type Matrix = [[f32; MAX_DEVICE_CHANNELS]; MAX_SOURCE_CHANNELS];

/// Quick ramp used for pause and resume, so nothing clicks.
const PAUSE_RAMP_FRAMES: u32 = 480;

/// Messages from the control side. All but `Start` are plain data.
pub enum Command {
    Start(Box<Voice>),
    Stop {
        cue: CueId,
        frames: u32,
    },
    Pause(CueId),
    Resume(CueId),
    Fade {
        cue: CueId,
        to: f32,
        frames: u32,
        curve: FadeCurve,
    },
    StopAll {
        frames: u32,
    },
}

/// Messages to the control side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceEnd {
    /// The cue played to its end.
    Finished(CueId),
    /// The cue was stopped.
    Stopped(CueId),
}

/// Everything fixed about a voice when it is created.
pub struct VoiceParams {
    pub cue: CueId,
    pub channels: usize,
    pub matrix: Matrix,
    pub fade_in_frames: u32,
    pub fade_out_frames: u32,
    /// Length in frames after trimming, when known and not looping.
    pub total_frames: Option<u64>,
    pub curve: FadeCurve,
    /// Samples that must be buffered before playback begins, so the first
    /// moments cannot underrun.
    pub prebuffer_samples: usize,
}

/// One playing cue.
pub struct Voice {
    cue: CueId,
    consumer: Consumer<f32>,
    eof: Arc<AtomicBool>,
    channels: usize,
    matrix: Matrix,
    prebuffer: usize,
    started: bool,
    level: Ramp,
    pause_gain: Ramp,
    paused: bool,
    pausing: bool,
    stopping: bool,
    played: u64,
    total_frames: Option<u64>,
    fade_out_frames: u32,
    curve: FadeCurve,
}

impl Voice {
    pub fn new(params: VoiceParams, consumer: Consumer<f32>, eof: Arc<AtomicBool>) -> Self {
        let mut level = Ramp::steady(1.0);
        if params.fade_in_frames > 0 {
            level = Ramp::steady(0.0);
            level.retarget(1.0, params.fade_in_frames, params.curve);
        }
        Self {
            cue: params.cue,
            consumer,
            eof,
            channels: params.channels.clamp(1, MAX_SOURCE_CHANNELS),
            matrix: params.matrix,
            prebuffer: params.prebuffer_samples,
            started: false,
            level,
            pause_gain: Ramp::steady(1.0),
            paused: false,
            pausing: false,
            stopping: false,
            played: 0,
            total_frames: params.total_frames,
            fade_out_frames: params.fade_out_frames,
            curve: params.curve,
        }
    }

    fn envelope(&self) -> f32 {
        match self.total_frames {
            Some(total) if self.fade_out_frames > 0 => {
                let start = total.saturating_sub(u64::from(self.fade_out_frames));
                if self.played >= start {
                    let p = (self.played - start) as f32 / self.fade_out_frames as f32;
                    return ramp_value(1.0, 0.0, p, self.curve);
                }
                1.0
            }
            _ => 1.0,
        }
    }

    /// Adds this voice's next `frames` frames into `out`. Returns how it ended,
    /// if it did.
    fn render_into(
        &mut self,
        out: &mut [f32],
        device_channels: usize,
        frames: usize,
        underruns: &AtomicU32,
    ) -> Option<VoiceEnd> {
        if !self.started {
            if self.consumer.slots() >= self.prebuffer || self.eof.load(Ordering::Acquire) {
                self.started = true;
            } else {
                return None;
            }
        }
        if self.paused {
            return None;
        }
        let used = device_channels.min(MAX_DEVICE_CHANNELS);
        for frame in 0..frames {
            // Read the flag first: if it is set, everything was already pushed.
            let finished_writing = self.eof.load(Ordering::Acquire);
            let have_sample = self.consumer.slots() >= self.channels;
            if !have_sample && finished_writing {
                return Some(VoiceEnd::Finished(self.cue));
            }
            if have_sample {
                if self.total_frames.is_some_and(|t| self.played >= t) {
                    return Some(VoiceEnd::Finished(self.cue));
                }
                let left = self.consumer.pop().unwrap_or(0.0);
                let right = if self.channels == 2 {
                    self.consumer.pop().unwrap_or(0.0)
                } else {
                    left
                };
                let gain = self.level.current() * self.pause_gain.current() * self.envelope();
                let base = frame * device_channels;
                for ch in 0..used {
                    out[base + ch] +=
                        gain * (left * self.matrix[0][ch] + right * self.matrix[1][ch]);
                }
                self.played += 1;
            } else {
                underruns.fetch_add(1, Ordering::Relaxed);
            }
            // Fades follow the device's time, so a stalled source cannot
            // stretch a stop or a pause.
            self.level.step();
            self.pause_gain.step();
            if self.stopping && self.level.is_done() {
                return Some(VoiceEnd::Stopped(self.cue));
            }
            if self.pausing && self.pause_gain.is_done() {
                self.pausing = false;
                self.paused = true;
                return None;
            }
        }
        None
    }

    fn stop(&mut self, frames: u32) {
        self.stopping = true;
        self.level.retarget(0.0, frames, FadeCurve::Linear);
    }
}

/// Peak levels per device channel and a count of underruns, shared with the
/// control side. Peak values are stored as the bits of a non-negative `f32`,
/// which order the same way as the floats.
pub struct Meters {
    peaks: [AtomicU32; MAX_DEVICE_CHANNELS],
    underruns: AtomicU32,
    frames: AtomicU64,
}

impl Meters {
    pub fn new() -> Self {
        Self {
            peaks: std::array::from_fn(|_| AtomicU32::new(0)),
            underruns: AtomicU32::new(0),
            frames: AtomicU64::new(0),
        }
    }

    /// Frames the device has played so far. This is the master clock.
    pub fn frames_rendered(&self) -> u64 {
        self.frames.load(Ordering::Relaxed)
    }

    /// Peaks since the last call, then resets them.
    pub fn take_peaks(&self, channels: usize) -> Vec<f32> {
        self.peaks
            .iter()
            .take(channels.min(MAX_DEVICE_CHANNELS))
            .map(|p| f32::from_bits(p.swap(0, Ordering::Relaxed)))
            .collect()
    }

    /// Times a voice ran out of decoded audio. Should stay at zero.
    pub fn underruns(&self) -> u32 {
        self.underruns.load(Ordering::Relaxed)
    }
}

impl Default for Meters {
    fn default() -> Self {
        Self::new()
    }
}

/// The audio-thread side of the engine.
pub struct Mixer {
    sample_rate: u32,
    commands: Consumer<Command>,
    ended: Producer<VoiceEnd>,
    garbage: Producer<Box<Voice>>,
    slots: Vec<Option<Box<Voice>>>,
    meters: Arc<Meters>,
}

impl Mixer {
    pub fn new(
        sample_rate: u32,
        commands: Consumer<Command>,
        ended: Producer<VoiceEnd>,
        garbage: Producer<Box<Voice>>,
        meters: Arc<Meters>,
    ) -> Self {
        Self {
            sample_rate,
            commands,
            ended,
            garbage,
            slots: (0..MAX_VOICES).map(|_| None).collect(),
            meters,
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Fills `out` (interleaved, `device_channels` wide) with the next block.
    pub fn render(&mut self, out: &mut [f32], device_channels: usize) {
        out.fill(0.0);
        if device_channels == 0 {
            return;
        }
        self.drain_commands();
        let frames = out.len() / device_channels;
        self.meters
            .frames
            .fetch_add(frames as u64, Ordering::Relaxed);
        for index in 0..self.slots.len() {
            let end = match self.slots[index].as_mut() {
                Some(voice) => {
                    voice.render_into(out, device_channels, frames, &self.meters.underruns)
                }
                None => continue,
            };
            if let Some(end) = end {
                let _ = self.ended.push(end);
                if let Some(done) = self.slots[index].take() {
                    self.retire(done);
                }
            }
        }
        self.record_peaks(out, device_channels);
    }

    fn record_peaks(&self, out: &[f32], device_channels: usize) {
        for ch in 0..device_channels.min(MAX_DEVICE_CHANNELS) {
            let peak = out
                .iter()
                .skip(ch)
                .step_by(device_channels)
                .fold(0.0f32, |m, s| m.max(s.abs()));
            self.meters.peaks[ch].fetch_max(peak.to_bits(), Ordering::Relaxed);
        }
    }

    fn drain_commands(&mut self) {
        while let Ok(command) = self.commands.pop() {
            match command {
                Command::Start(voice) => self.start_voice(voice),
                Command::Stop { cue, frames } => {
                    if let Some(v) = self.voice_mut(cue) {
                        v.stop(frames);
                    }
                }
                Command::Pause(cue) => {
                    if let Some(v) = self.voice_mut(cue).filter(|v| !v.paused && !v.pausing) {
                        v.pausing = true;
                        v.pause_gain
                            .retarget(0.0, PAUSE_RAMP_FRAMES, FadeCurve::Linear);
                    }
                }
                Command::Resume(cue) => {
                    if let Some(v) = self.voice_mut(cue) {
                        v.paused = false;
                        v.pausing = false;
                        v.pause_gain
                            .retarget(1.0, PAUSE_RAMP_FRAMES, FadeCurve::Linear);
                    }
                }
                Command::Fade {
                    cue,
                    to,
                    frames,
                    curve,
                } => {
                    if let Some(v) = self.voice_mut(cue).filter(|v| !v.stopping) {
                        v.level.retarget(to, frames, curve);
                    }
                }
                Command::StopAll { frames } => {
                    for v in self.slots.iter_mut().flatten() {
                        v.stop(frames);
                    }
                }
            }
        }
    }

    fn start_voice(&mut self, voice: Box<Voice>) {
        // Replace a voice already playing for the same cue.
        if let Some(old) = self.find(voice.cue).and_then(|i| self.slots[i].take()) {
            self.retire(old);
        }
        match self.slots.iter().position(Option::is_none) {
            Some(free) => self.slots[free] = Some(voice),
            None => {
                // No free voice: refuse the cue without allocating.
                let _ = self.ended.push(VoiceEnd::Stopped(voice.cue));
                self.retire(voice);
            }
        }
    }

    /// Hands a voice to the control side to be dropped there. If the queue is
    /// somehow full, leak rather than free memory on the audio thread.
    fn retire(&mut self, voice: Box<Voice>) {
        if let Err(rtrb::PushError::Full(voice)) = self.garbage.push(voice) {
            std::mem::forget(voice);
        }
    }

    fn voice_mut(&mut self, cue: CueId) -> Option<&mut Voice> {
        let index = self.find(cue)?;
        self.slots[index].as_deref_mut()
    }

    fn find(&self, cue: CueId) -> Option<usize> {
        self.slots
            .iter()
            .position(|s| s.as_ref().is_some_and(|v| v.cue == cue))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Rig {
        mixer: Mixer,
        commands: Producer<Command>,
        ended: Consumer<VoiceEnd>,
        garbage: Consumer<Box<Voice>>,
        meters: Arc<Meters>,
    }

    fn rig() -> Rig {
        let (cmd_p, cmd_c) = rtrb::RingBuffer::new(64);
        let (end_p, end_c) = rtrb::RingBuffer::new(64);
        let (gar_p, gar_c) = rtrb::RingBuffer::new(64);
        let meters = Arc::new(Meters::new());
        Rig {
            mixer: Mixer::new(48_000, cmd_c, end_p, gar_p, meters.clone()),
            commands: cmd_p,
            ended: end_c,
            garbage: gar_c,
            meters,
        }
    }

    /// Identity matrix: source channel n to device channel n.
    fn identity() -> Matrix {
        let mut m = [[0.0; MAX_DEVICE_CHANNELS]; MAX_SOURCE_CHANNELS];
        m[0][0] = 1.0;
        m[1][1] = 1.0;
        m
    }

    fn params(cue: CueId, channels: usize) -> VoiceParams {
        VoiceParams {
            cue,
            channels,
            matrix: identity(),
            fade_in_frames: 0,
            fade_out_frames: 0,
            total_frames: None,
            curve: FadeCurve::Linear,
            prebuffer_samples: 0,
        }
    }

    /// A voice whose whole source is already buffered.
    fn voice(params: VoiceParams, samples: &[f32]) -> Box<Voice> {
        let (mut p, c) = rtrb::RingBuffer::new(samples.len().max(1));
        for s in samples {
            p.push(*s).unwrap();
        }
        Box::new(Voice::new(params, c, Arc::new(AtomicBool::new(true))))
    }

    fn render(rig: &mut Rig, frames: usize) -> Vec<f32> {
        let mut out = vec![0.0; frames * 2];
        rig.mixer.render(&mut out, 2);
        out
    }

    #[test]
    fn plays_a_stereo_voice_and_reports_that_it_finished() {
        let mut rig = rig();
        let cue = CueId::new();
        let src = [0.5, -0.5, 0.25, -0.25];
        rig.commands
            .push(Command::Start(voice(params(cue, 2), &src)))
            .ok();

        let out = render(&mut rig, 4);

        assert_eq!(&out[..4], &src);
        assert_eq!(&out[4..], &[0.0; 4]);
        assert_eq!(rig.ended.pop(), Ok(VoiceEnd::Finished(cue)));
        assert!(
            rig.garbage.pop().is_ok(),
            "the voice is handed back, not dropped here"
        );
    }

    #[test]
    fn a_mono_voice_feeds_both_channels_through_its_matrix() {
        let mut rig = rig();
        let mut p = params(CueId::new(), 1);
        p.matrix[1] = [0.0; MAX_DEVICE_CHANNELS];
        p.matrix[0][1] = 1.0;
        rig.commands
            .push(Command::Start(voice(p, &[0.5, 0.25])))
            .ok();

        let out = render(&mut rig, 2);

        assert_eq!(out, [0.5, 0.5, 0.25, 0.25]);
    }

    #[test]
    fn two_voices_sum() {
        let mut rig = rig();
        for _ in 0..2 {
            rig.commands
                .push(Command::Start(voice(
                    params(CueId::new(), 2),
                    &[0.25, 0.25],
                )))
                .ok();
        }
        assert_eq!(render(&mut rig, 1), [0.5, 0.5]);
    }

    #[test]
    fn a_fade_in_is_sample_accurate() {
        let mut rig = rig();
        let mut p = params(CueId::new(), 1);
        p.fade_in_frames = 4;
        rig.commands.push(Command::Start(voice(p, &[1.0; 6]))).ok();

        let out = render(&mut rig, 6);
        let left: Vec<f32> = out.iter().step_by(2).copied().collect();

        assert_eq!(left, [0.0, 0.25, 0.5, 0.75, 1.0, 1.0]);
    }

    #[test]
    fn a_natural_fade_out_reaches_silence_at_the_end() {
        let mut rig = rig();
        let mut p = params(CueId::new(), 1);
        p.total_frames = Some(8);
        p.fade_out_frames = 4;
        rig.commands.push(Command::Start(voice(p, &[1.0; 8]))).ok();

        let out = render(&mut rig, 8);
        let left: Vec<f32> = out.iter().step_by(2).copied().collect();

        assert_eq!(left, [1.0, 1.0, 1.0, 1.0, 1.0, 0.75, 0.5, 0.25]);
    }

    #[test]
    fn stop_ramps_to_silence_then_reports_stopped() {
        let mut rig = rig();
        let cue = CueId::new();
        rig.commands
            .push(Command::Start(voice(params(cue, 1), &[1.0; 16])))
            .ok();
        rig.commands.push(Command::Stop { cue, frames: 4 }).ok();

        let out = render(&mut rig, 8);
        let left: Vec<f32> = out.iter().step_by(2).copied().collect();

        assert_eq!(&left[..5], &[1.0, 0.75, 0.5, 0.25, 0.0]);
        assert_eq!(rig.ended.pop(), Ok(VoiceEnd::Stopped(cue)));
    }

    #[test]
    fn a_fade_command_moves_the_level() {
        let mut rig = rig();
        let cue = CueId::new();
        rig.commands
            .push(Command::Start(voice(params(cue, 1), &[1.0; 8])))
            .ok();
        rig.commands
            .push(Command::Fade {
                cue,
                to: 0.5,
                frames: 4,
                curve: FadeCurve::Linear,
            })
            .ok();

        let out = render(&mut rig, 6);
        let left: Vec<f32> = out.iter().step_by(2).copied().collect();

        assert_eq!(left, [1.0, 0.875, 0.75, 0.625, 0.5, 0.5]);
    }

    #[test]
    fn pause_ramps_down_then_holds_and_resume_continues_where_it_left_off() {
        let mut rig = rig();
        let cue = CueId::new();
        let src: Vec<f32> = vec![1.0; 4000];
        rig.commands
            .push(Command::Start(voice(params(cue, 1), &src)))
            .ok();
        render(&mut rig, 100);

        rig.commands.push(Command::Pause(cue)).ok();
        render(&mut rig, 600);
        let held = render(&mut rig, 100);
        assert!(held.iter().all(|s| *s == 0.0), "paused voices are silent");

        rig.commands.push(Command::Resume(cue)).ok();
        let after = render(&mut rig, 600);
        assert!(
            after.iter().any(|s| *s > 0.9),
            "and come back to full level"
        );
    }

    #[test]
    fn nothing_plays_until_the_prebuffer_is_full() {
        let mut rig = rig();
        let (mut prod, cons) = rtrb::RingBuffer::new(64);
        let eof = Arc::new(AtomicBool::new(false));
        let mut p = params(CueId::new(), 1);
        p.prebuffer_samples = 8;
        rig.commands
            .push(Command::Start(Box::new(Voice::new(p, cons, eof))))
            .ok();

        assert_eq!(render(&mut rig, 2), [0.0; 4]);
        for _ in 0..8 {
            prod.push(1.0).unwrap();
        }
        assert_eq!(render(&mut rig, 1), [1.0, 1.0]);
    }

    #[test]
    fn running_dry_without_eof_is_an_underrun_not_an_end() {
        let mut rig = rig();
        let (mut prod, cons) = rtrb::RingBuffer::new(8);
        prod.push(1.0).unwrap();
        let eof = Arc::new(AtomicBool::new(false));
        rig.commands
            .push(Command::Start(Box::new(Voice::new(
                params(CueId::new(), 1),
                cons,
                eof,
            ))))
            .ok();

        render(&mut rig, 4);

        assert!(rig.meters.underruns() >= 3);
        assert!(rig.ended.pop().is_err());
    }

    #[test]
    fn a_stop_still_completes_on_time_when_the_source_is_starved() {
        let mut rig = rig();
        let cue = CueId::new();
        let (_prod, cons) = rtrb::RingBuffer::new(8);
        let eof = Arc::new(AtomicBool::new(false));
        rig.commands
            .push(Command::Start(Box::new(Voice::new(
                params(cue, 1),
                cons,
                eof,
            ))))
            .ok();
        rig.commands.push(Command::Stop { cue, frames: 4 }).ok();

        render(&mut rig, 8);

        assert_eq!(rig.ended.pop(), Ok(VoiceEnd::Stopped(cue)));
    }

    #[test]
    fn peaks_are_recorded_per_channel_and_reset_when_read() {
        let mut rig = rig();
        rig.commands
            .push(Command::Start(voice(params(CueId::new(), 2), &[0.5, 0.25])))
            .ok();
        render(&mut rig, 1);
        assert_eq!(rig.meters.take_peaks(2), [0.5, 0.25]);
        assert_eq!(rig.meters.take_peaks(2), [0.0, 0.0]);
    }

    #[test]
    fn a_full_mixer_refuses_extra_voices_without_failing() {
        let mut rig = rig();
        let mut last = CueId::new();
        for _ in 0..=MAX_VOICES {
            last = CueId::new();
            rig.commands
                .push(Command::Start(voice(params(last, 1), &[0.0; 4000])))
                .ok();
            render(&mut rig, 1);
        }
        assert_eq!(rig.ended.pop(), Ok(VoiceEnd::Stopped(last)));
    }
}
