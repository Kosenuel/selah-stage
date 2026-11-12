//! The audio engine: devices, patches, voices, fades, and meters.
//!
//! The real-time callback must not allocate, lock, log, panic, or touch files
//! or the network. Backends sit behind one trait so tests can render offline
//! into a buffer; no test may require a real audio device.
//!
//! Nothing is implemented yet; see the roadmap in `CLAUDE.md`.
