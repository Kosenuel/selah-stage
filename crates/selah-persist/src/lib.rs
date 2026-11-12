//! Show files on disk: load, save, migrate, autosave, and bundle.
//!
//! Writes are atomic, and every schema change ships with a forward migration
//! and a fixture of the old version.
//!
//! Nothing is implemented yet; see the roadmap in `CLAUDE.md`.

#![forbid(unsafe_code)]
