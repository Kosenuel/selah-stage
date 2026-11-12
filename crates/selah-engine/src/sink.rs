//! What the engine needs from the outside world.

use selah_core::{CueId, Decibels, FadeCurve, PatchId, Seconds};

/// The master clock. In the app this is derived from the audio device.
pub trait Clock {
    fn now(&self) -> Seconds;
}

/// Everything an audio cue needs in order to start.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioPlay {
    pub file: String,
    pub level: Decibels,
    pub start: Seconds,
    pub end: Option<Seconds>,
    pub loops: Option<u32>,
    pub fade_in: Seconds,
    pub fade_out: Seconds,
    pub curve: FadeCurve,
    pub patches: Vec<PatchId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SinkError {
    /// The file is not where the show says it is.
    MissingMedia(String),
    /// The audio side could not start the cue for another reason.
    Failed(String),
}

/// The audio side. Calls must return immediately; the engine runs on the
/// control thread and must never block on audio.
pub trait AudioSink {
    fn start(&mut self, cue: CueId, play: &AudioPlay) -> Result<(), SinkError>;
    /// Stops a cue with a short ramp so nothing clicks.
    fn stop(&mut self, cue: CueId, fade: Seconds);
    fn pause(&mut self, cue: CueId);
    fn resume(&mut self, cue: CueId);
    fn fade(&mut self, cue: CueId, to: Decibels, duration: Seconds, curve: FadeCurve);
    fn stop_all(&mut self, fade: Seconds);
}
