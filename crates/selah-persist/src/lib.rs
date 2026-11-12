//! Show files on disk: load, save, migrate, back up, and autosave.
//!
//! A show is a folder holding `show.json` and its media. Writes are atomic: a
//! temporary file is written and synced, then renamed over the old one, so a
//! power cut leaves either the old show or the new one, never half of either.
//! Each save keeps the previous version in `backups/`, and a show whose file is
//! damaged is recovered from the newest backup that still reads.
//!
//! Every change to the saved format needs an entry in [`migrate`] and a
//! fixture of the old version in `fixtures/shows/`.

#![forbid(unsafe_code)]

mod autosave;
mod error;
mod files;
mod media;
pub mod migrate;

pub use autosave::Autosaver;
pub use error::PersistError;
pub use files::{BACKUPS_DIR, BACKUPS_KEPT, Loaded, SHOW_FILE, create, is_show_folder, load, save};
pub use media::{MEDIA_DIR, MissingMedia, import_media, missing_media};
