//! The audio engine: devices, patches, voices, fades, and meters.
//!
//! The real-time callback must not allocate, lock, log, panic, or touch files
//! or the network. Everything it needs is built beforehand on other threads
//! and handed over through lock-free queues.
//!
//! Layout:
//! - [`fader`]: fade curves and decibel conversion (pure maths).
//! - [`mixer`]: the real-time core that turns voices into device samples.
//! - [`decode`]: reading and resampling files, on worker threads.
//! - [`AudioEngine`]: the control-side handle. It implements
//!   `selah_engine::AudioSink`, so the cue engine drives it directly.
//! - [`OfflineBackend`] renders into a buffer (tests); [`OutputDevice`] plays
//!   through a real device.
//!
//! Each file is decoded by its own worker thread into a ring buffer that the
//! mixer reads, so GO never waits on the disk.

#![forbid(unsafe_code)]

pub mod decode;
pub mod fader;
pub mod mixer;

mod backend;
mod engine;
mod routing;

pub use backend::{DeviceError, OfflineBackend, OutputDevice, RunningDevice};
pub use engine::{AudioEngine, AudioEvent};
pub use mixer::{MAX_DEVICE_CHANNELS, MAX_VOICES, Meters, Mixer};
