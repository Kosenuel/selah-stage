//! The cue engine: playhead, sequencing, waits, follow modes, groups, control
//! cues, and panic.
//!
//! The engine never touches a device. It is driven through two traits, a
//! [`Clock`] and an [`AudioSink`], so it is tested with a fake clock and a
//! recording sink. It must never depend on `selah-audio`.
//!
//! Time moves only when [`Engine::tick`] is called (and inside operator
//! actions such as [`Engine::go`]); the host calls `tick` often, driven by the
//! audio clock.
//!
//! Not here yet: the undoable edit stack and live editing of a loaded show.

#![forbid(unsafe_code)]

mod engine;
mod sink;
pub mod testing;

pub use engine::{CueState, Engine, EngineConfig, Event, Warning};
pub use sink::{AudioPlay, AudioSink, Clock, SinkError};
