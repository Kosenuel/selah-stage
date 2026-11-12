//! The cue engine: playhead, sequencing, waits, follow modes, and the transport
//! state machine.
//!
//! The engine is driven through traits (a clock and an audio sink) rather than
//! by `selah-audio` directly, so it can be tested with a fake clock and no
//! devices. It must never depend on `selah-audio`.
//!
//! Nothing is implemented yet; see the roadmap in `CLAUDE.md`.

#![forbid(unsafe_code)]
