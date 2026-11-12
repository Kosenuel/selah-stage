//! Domain types shared by every other crate: shows, items, cues, ids, and validation.
//!
//! This crate is the bottom of the dependency graph. It must stay free of I/O,
//! async, and dependencies on any other `selah-*` crate, so that everything
//! above it can be tested without devices, files, or a runtime.
//!
//! Everything here serialises with camelCase names; see "IPC contract" in
//! `CLAUDE.md`. Time is in seconds (`f64`) and levels in dB at this boundary.

#![forbid(unsafe_code)]

mod cue;
mod edit;
mod id;
mod show;
mod validate;

pub use cue::{Cue, CueKind, Decibels, FadeCurve, Follow, GroupMode, Seconds};
pub use edit::{ControlKind, CueChange, Edit, EditError, Editor, NewAudio, Outcome};
pub use id::{CueId, ItemId, PatchId};
pub use show::{CURRENT_SCHEMA_VERSION, Item, ItemKind, Patch, Show, ShowMeta};
pub use validate::{Issue, IssueKind, validate};
