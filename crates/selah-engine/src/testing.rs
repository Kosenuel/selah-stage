//! A fake clock and a recording sink, for tests in this crate and others.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

use selah_core::{CueId, Decibels, FadeCurve, Seconds};

use crate::sink::{AudioPlay, AudioSink, Clock, SinkError};

/// A clock that only moves when told to. Clones share the same time.
#[derive(Debug, Clone, Default)]
pub struct FakeClock(Rc<Cell<Seconds>>);

impl FakeClock {
    pub fn advance(&self, seconds: Seconds) {
        self.0.set(self.0.get() + seconds);
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Seconds {
        self.0.get()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Call {
    Start(CueId),
    Stop(CueId, Seconds),
    Pause(CueId),
    Resume(CueId),
    Fade(CueId, Decibels),
    StopAll(Seconds),
}

/// Records every call. Files in `missing` fail to start. Clones share state.
#[derive(Debug, Clone, Default)]
pub struct RecordingSink {
    pub calls: Rc<RefCell<Vec<Call>>>,
    pub missing: Rc<RefCell<HashSet<String>>>,
}

impl RecordingSink {
    pub fn calls(&self) -> Vec<Call> {
        self.calls.borrow().clone()
    }
}

impl AudioSink for RecordingSink {
    fn start(&mut self, cue: CueId, play: &AudioPlay) -> Result<(), SinkError> {
        if self.missing.borrow().contains(&play.file) {
            return Err(SinkError::MissingMedia(play.file.clone()));
        }
        self.calls.borrow_mut().push(Call::Start(cue));
        Ok(())
    }
    fn stop(&mut self, cue: CueId, fade: Seconds) {
        self.calls.borrow_mut().push(Call::Stop(cue, fade));
    }
    fn pause(&mut self, cue: CueId) {
        self.calls.borrow_mut().push(Call::Pause(cue));
    }
    fn resume(&mut self, cue: CueId) {
        self.calls.borrow_mut().push(Call::Resume(cue));
    }
    fn fade(&mut self, cue: CueId, to: Decibels, _duration: Seconds, _curve: FadeCurve) {
        self.calls.borrow_mut().push(Call::Fade(cue, to));
    }
    fn stop_all(&mut self, fade: Seconds) {
        self.calls.borrow_mut().push(Call::StopAll(fade));
    }
}
