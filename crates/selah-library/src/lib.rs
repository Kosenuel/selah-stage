//! The song and scripture library: storage, search, and importers.
//!
//! Songs are copied by value into a show when added, so the library can change
//! without altering a service that has already run.
//!
//! Nothing is implemented yet; see the roadmap in `CLAUDE.md`.

#![forbid(unsafe_code)]
