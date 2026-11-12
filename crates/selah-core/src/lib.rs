//! Domain types shared by every other crate: shows, items, cues, ids, and validation.
//!
//! This crate is the bottom of the dependency graph. It must stay free of I/O,
//! async, and dependencies on any other `selah-*` crate, so that everything
//! above it can be tested without devices, files, or a runtime.
//!
//! Nothing is implemented yet; see the roadmap in `CLAUDE.md`.

#![forbid(unsafe_code)]
