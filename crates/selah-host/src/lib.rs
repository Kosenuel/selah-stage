//! The host loop: runs the cue engine and the audio engine together.
//!
//! One thread owns the audio device, the cue engine, and the audio engine. It
//! ticks the engine, passes the audio side's reports to it, and publishes a
//! [`Snapshot`] for the UI. Everything else talks to it through [`Host`], which
//! is cheap to clone around and never blocks on audio.
//!
//! The thread is needed because some platforms require the device's stream to
//! stay on the thread that created it.

#![forbid(unsafe_code)]

mod folder;
mod host;

pub use folder::{FolderError, show_from_folder};
pub use host::{
    Command, DeviceStatus, Host, HostError, Output, Snapshot, SnapshotCue, SnapshotWarning,
};
