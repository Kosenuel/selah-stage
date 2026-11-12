//! Media preparation through the bundled ffmpeg sidecar: probing, optimising on
//! import, waveform peaks, thumbnails, and loudness.
//!
//! ffmpeg always runs as a separate process with a bounded job queue, and never
//! on the GO path.
//!
//! Nothing is implemented yet; see the roadmap in `CLAUDE.md`.

#![forbid(unsafe_code)]
