# Selah Stage

Selah Stage is a Windows desktop app that runs a whole church service from one
cue list: audio, lyrics, scripture, slides, video, backing tracks, and stage
displays. Think "show control software, redesigned for worship teams and
volunteer operators". It is an original product; do not copy another product's
name, artwork, file formats' branding, or interface layout.

This file is the shared brief for every session that works on the repo. Read it
fully before changing anything, and keep it true: when you finish work, update
[Current state](#current-state), the [module status](#roadmap-and-module-status)
table, and the [decision log](#decision-log). Delete statements that stop being
true rather than leaving them to mislead the next session.

## Current state

- **Nothing is scaffolded yet.** The repo holds only this file. Everything below
  marked "target" is the agreed design, not existing code.
- The first job is milestone M0 (scaffold). Whoever does it must make the
  [commands](#commands) section real and correct it if anything differs.
- Stack and architecture were agreed with the owner on 2026-11-12. The product
  will be open source, with paid offerings possible later. The specific licence
  is not chosen yet and there is no `LICENSE` file; see
  [Licensing](#licensing-and-content-rules).

## Who it is for

- **Volunteer operators** who run the service once or twice a month. They need
  big targets, obvious state, and mistakes that are easy to undo.
- **Worship leaders and bands** who need lyrics that follow them when they go
  off-plan, plus click, guide, and stems in their in-ears.
- **Tech leads** who build the service during the week on a modest PC, often
  with unreliable internet.

## Product principles

Use these to settle design arguments, in this order.

1. **The service never stops.** A crash, a missing file, or a slow disk must not
   interrupt audio or blank a screen. GO always does something sensible.
2. **Volunteer-proof.** Show mode has no modal dialogs, no destructive action
   without undo, and nothing that needs a manual to understand.
3. **Follow the room.** The operator can jump to any song section, repeat a
   chorus, or clear the screen in one keypress.
4. **Offline first.** Every core feature works with no network. Integrations
   are imports and syncs, never dependencies.
5. **Fast on modest hardware.** Target a mid-range laptop with integrated
   graphics driving two external displays.
6. **Beautiful by default.** Lyrics and slides should look good with zero
   design effort from the user.

## Glossary

Use these words in code, UI, and commits. Do not invent synonyms.

| Term | Meaning |
|---|---|
| Show | The saved document for one service: plan, cues, settings, media references. |
| Item | A segment of the service plan (pre-service, song, announcements, sermon). Holds cues. |
| Cue | One operable action: play audio, show lyrics, fade, stop, and so on. |
| Playhead | The cue that fires on the next GO. Shown as "standby". |
| GO | Fire the playhead cue and advance the playhead. Spacebar by default. |
| Live | A cue that is currently running or currently on an output. |
| Presentation cue | A cue with internal slides (lyrics, scripture, slide deck). |
| Section | A named part of a song (Verse 1, Chorus, Bridge). |
| Arrangement | The ordered list of sections used for one service. |
| Layer | One compositing level of an output (see [Outputs](#outputs)). |
| Patch | A named logical audio output ("Main", "In-ears", "Stream") mapped to device channels per machine. |
| Stems | The separate audio files of a multitrack (drums, keys, click, guide). |
| Panic | Fade out and stop all audio within a short, configurable time. |
| Edit mode / Show mode | Edit mode allows changes. Show mode locks structure and exposes only operation. |

## Architecture

Tauri 2 desktop app. A Rust core owns all state and all real-time work. A React
and TypeScript front end renders the control interface and the output windows.

```
┌──────────────────────── Tauri process (Rust) ────────────────────────┐
│  selah-engine  ── cue sequencing, playhead, transport state machine   │
│       │                                                               │
│  selah-audio   ── devices, patches, voices, fades, meters (RT thread) │
│  selah-media   ── ffmpeg sidecar: probe, transcode, peaks, loudness   │
│  selah-library ── songs, scripture, SQLite, importers                 │
│  selah-persist ── show files, autosave, migrations                    │
│  selah-control ── MIDI, OSC, remote server (later milestones)         │
│  selah-core    ── domain types shared by all of the above (no I/O)    │
└───────────────▲──────────────────────────────┬───────────────────────┘
        commands│                              │events / channels
┌───────────────┴──────────────┐  ┌────────────▼───────────────────────┐
│ Control window (React)       │  │ Output windows (React, one per     │
│ plan, cue list, inspector    │  │ display): main, stage, lower third │
└──────────────────────────────┘  └────────────────────────────────────┘
```

Rules that follow from this shape:

- **Rust is the source of truth.** The UI never holds authoritative show or
  transport state. It sends commands and renders what the core reports.
- **The audio device clock is the master clock.** Fades, waits, video sync, and
  timers derive from sample position. Never sync anything to wall-clock time or
  to JavaScript timers.
- **Output windows are dumb renderers.** They receive a complete render state
  and draw it. They contain no show logic, so they can be reloaded at any time
  without affecting the service.
- **A UI failure must not stop the engine.** If a webview crashes or hangs,
  audio continues and the window is recreated.

## Repository layout

Target layout, created in M0. Keep to it; propose changes in the decision log.

```
apps/desktop/
  src/                  React control UI and output renderers
    app/                shell, window bootstrapping, routing
    features/           service-plan, cue-list, inspector, library, settings, …
    outputs/            main, stage, lower-third (separate entry points)
    components/         design-system primitives, no feature logic
    lib/                generated IPC bindings, hooks, utilities
    styles/             design tokens
  src-tauri/            thin shell: command handlers, window management, wiring
crates/
  selah-core/           domain types, ids, validation. No I/O, no async.
  selah-engine/         cue engine. Depends on core and on traits, not on audio.
  selah-audio/          audio engine and backends.
  selah-media/          ffmpeg sidecar wrapper and media analysis.
  selah-library/        song and scripture library, importers.
  selah-persist/        show file load, save, migrate, autosave, bundle.
  selah-control/        MIDI, OSC, remote control.
fixtures/               small test media and sample shows (public domain only)
```

`src-tauri` must stay thin. If a command handler grows logic, move the logic
into a crate and test it there.

## Domain model

Sketch of the core types (`selah-core`). Field names are indicative; the Rust
definitions become authoritative once they exist.

```
Show   { schema_version, meta, items: [Item], patches: [Patch], outputs, settings }
Item   { id, title, kind, notes, cues: [Cue] }
Cue    { id, number, name, notes, color, armed,
         pre_wait, post_wait, follow: None | Continue | Follow,
         kind: CueKind }

CueKind =
  Audio | Video | Image                    media playback
  Lyrics | Scripture | Slides | Text       presentation cues with internal slides
  Fade | Stop | Pause | GoTo | Arm         control cues that target other cues
  Group                                    children fire together, in sequence, or at random
  Timer                                    countdown or count-up shown on outputs
  Track | Pad                              multitrack playback, ambient pads (M4)
  Midi | Osc | Light                       outbound control (M5+)
  Memo                                     does nothing; a note in the list
```

Conventions:

- Ids are UUIDs (v7) and never change. Cue **numbers** are user-facing labels,
  editable and not unique-enforced beyond a warning.
- Shows reference audio **patches**, not device channels, so a show built on one
  machine opens on another. Patches map to devices in per-machine settings.
- Levels are stored and shown in dB. Convert to linear gain only inside DSP
  code. Mute is an explicit flag, not a magic number.
- Time is sample frames (`u64`) inside the audio engine and seconds (`f64`) at
  every boundary (files, IPC, UI).
- Songs live in the library and are **copied by value into the show** when
  added, so editing the library later never changes a service that already ran.

## Cue engine semantics

- Cue states: `Idle → PreWait → Running → Done`, with `Paused` reachable from
  `PreWait` and `Running`, and `Stopped` from anywhere.
- `follow: Continue` starts the next cue when this one starts (after
  `post_wait`). `follow: Follow` starts the next cue when this one finishes.
- GO on a disarmed cue skips its action but still advances the playhead.
- GO on a cue with missing media logs a visible warning, skips the action, and
  advances. It never blocks.
- **Two levels of navigation.** GO moves between cues. While a presentation cue
  is live, next and previous (arrow keys) move between its slides, and section
  hotkeys jump within the song, without moving the playhead. This is the core
  interaction of the product; do not collapse it into one level.
- All edit operations go through an undoable command stack in the core.
- The engine is driven by a clock trait and an audio trait so it can be tested
  with a fake clock and no devices.

## Audio engine rules

The real-time callback is the most fragile code in the project.

- In the audio callback: **no allocation, no locks, no file or network I/O, no
  logging, no panics.** Communicate through lock-free queues only.
- Decoding and disk reads happen on worker threads that fill ring buffers.
  Audio cues near the playhead are pre-rolled so GO is immediate.
- Backends sit behind one trait. `OfflineBackend` renders into a buffer for
  tests. `CpalBackend` drives real devices. WASAPI comes first; ASIO is a later,
  feature-flagged addition (see [Licensing](#licensing-and-content-rules)).
- Signal flow: cue channels → per-cue matrix → patches → device channels.
- Fades are sample-accurate ramps with selectable curves. A device underrun or
  a device disappearing must be reported and recovered from, not fatal.
- Meters and playback positions go to the UI on a throttled channel (about
  30 Hz), never one message per audio block.

Suggested crates (confirm current versions when adding): `cpal`, `symphonia`,
`rubato`, `rtrb`. Time-stretch and pitch-shift for multitracks is an open
question; pick a permissively licensed library.

## Video and media

Decision: **playback in the output webview, with ffmpeg as a bundled sidecar
for everything the webview cannot do.**

- The webview plays H.264, VP9, and AV1 natively with hardware decoding.
- On import, `selah-media` probes each file with ffprobe. Anything the webview
  cannot play (ProRes, HAP, DNxHD, odd containers) is transcoded once into the
  show's media folder ("optimise on import"). Video with transparency becomes
  VP9 WebM with alpha.
- ffmpeg also produces waveform peak files, thumbnails, and loudness
  measurements, and extracts audio from video.
- **Video audio never plays through the webview.** The audio track is extracted
  and played by the audio engine so it follows patches and fades. The video
  element plays muted and is slaved to the audio clock: nudge `playbackRate`
  for small drift, seek for large drift.
- ffmpeg runs as a **separate process** with a bounded job queue. It is never
  invoked on the GO path; all processing happens at import or in the
  background.
- Video output sits behind a `VideoOutput` interface in the front end. If
  webview playback proves insufficient (frame-accurate sync, NDI, many layers),
  a native player (libmpv, or ffmpeg decoding to a GPU surface) can replace it
  without touching the cue engine. Do not start that work without a decision
  log entry.

## Outputs

Each output (main, stage, lower third) is a borderless fullscreen window on a
chosen display. Every output composites the same layers, bottom to top:

1. **Background**: looping motion or still behind text.
2. **Media**: slides, images, and video cues.
3. **Text**: lyrics and scripture.
4. **Overlay**: logo, timers, messages.

Clear actions operate per layer and are always one keypress in show mode:
Clear Text, Clear Media, Clear All, To Logo, Black.

- The **stage display** shows current and next slide text, clock, timers, the
  next cue name, and operator messages. Its layout is configurable.
- The **lower third** output renders text on a transparent or chroma background
  for the livestream mixer.
- Text rendering must auto-fit: no clipped or overflowing lyrics, ever. Two-
  language lyrics (primary and translation line) are supported by the model
  from the start.

## IPC contract

- UI → core: Tauri **commands**, `snake_case`, verb first (`go`, `stop_all`,
  `set_cue_level`). Commands return `Result<T, AppError>` with a typed,
  serialisable error.
- Core → UI: **events** for state changes, **channels** for high-frequency
  streams (meters, positions).
- TypeScript bindings are **generated from the Rust types** (tauri-specta or
  equivalent). Never hand-write a type that mirrors a Rust type, and commit the
  generated file.
- A change to a command or event must update the Rust side, the generated
  bindings, and every caller in the same commit.

## Show files and persistence

- A show is a folder: `show.json` plus `media/`. Export zips the folder into a
  single portable bundle.
- `show.json` carries `schema_version`. Every schema change ships with a
  forward migration and a fixture of the old version in `fixtures/`. Never
  break the ability to open an older show.
- Media is referenced by path relative to the show folder, with a content hash
  for relinking.
- Writes are atomic (write a temporary file, then rename). Autosave runs on
  every committed edit and keeps a rolling set of backups.
- The song and scripture library is SQLite in the app data folder. Machine
  settings (devices, patches-to-channels, displays) are stored there too, not
  in the show.

## Interface and design

- Dark theme by default; a light theme is supported through the same tokens.
  All colours, spacing, radii, and type sizes come from tokens in `styles/`.
  No hard-coded colours in components.
- Every action has a keyboard shortcut and appears in a command palette.
- Show mode: large playhead and "next" display readable from two metres, a
  prominent GO, visible running cues with remaining time, and no modals.
- State is never shown by colour alone. Pair colour with an icon or label.
- Motion is short and functional. Nothing animates in a way that delays an
  operator action.
- Write UI text in plain, friendly language, in sentence case. Say what
  happened and what to do next ("Can't find intro.wav. Relink it or skip.").

Front-end stack (target): React, TypeScript in strict mode, Vite, Zustand for
view state, Tailwind for styling, Radix primitives for accessible components,
pnpm for packages.

## Coding conventions

Rust:

- `cargo fmt` and `cargo clippy --all-targets -- -D warnings` must pass.
- No `unwrap`, `expect`, or `panic!` outside tests and provable invariants.
  Libraries use `thiserror`; the app shell maps errors to `AppError`.
- `unsafe` needs a `// SAFETY:` comment and a reason it cannot be avoided.
- Crates depend downward only: everything may use `selah-core`; `selah-core`
  uses nothing of ours. `selah-engine` must not depend on `selah-audio`
  directly, only on traits.
- Use `tracing` for logs, never on the audio thread.

TypeScript:

- `strict` on, no `any`, no non-null assertions without a comment.
- Features do not import from other features; share through `components/` or
  `lib/`.
- Components render state and dispatch commands. Business rules belong in Rust.

Both:

- Comments explain why, not what. No commented-out code.
- Small, focused commits in the form `area: summary`
  (`audio: add equal-power fade curve`).
- Do not add a dependency for something a few lines would do. Check a new
  dependency's licence and maintenance before adding it.

## Testing and definition of done

- `selah-core`, `selah-engine`, `selah-persist`: thorough unit tests. The cue
  engine is tested with a fake clock; every follow mode, wait, and edge case
  (disarmed, missing media, stop during pre-wait) has a test.
- `selah-audio`: render through `OfflineBackend` and assert on the samples
  (levels, fade shapes, sample-accurate start times). No test may require a
  real audio device.
- Front end: Vitest for logic and components; Playwright for the main operator
  flows once they exist.
- Persistence: round-trip and migration tests against fixtures.

A change is done when:

1. Formatting, lint, type check, and all tests pass (`pnpm check`).
2. New behaviour has tests, or the hand-off note says why it cannot.
3. Anything that could only be verified on Windows with real hardware is
   listed explicitly as unverified.
4. This file is updated where the change affects it.

## Commands

Target commands; M0 makes them real. Fix this section if they end up different.

```bash
pnpm install
```

```bash
pnpm tauri dev
```

```bash
pnpm check
```

`pnpm check` is the single gate: Rust format, clippy, and tests, plus
TypeScript type check, lint, and tests.

### Environment notes

- The product targets **Windows 10 and 11**. Many sessions run in Linux
  containers with no audio device and no display. That is why engine logic sits
  behind traits and is testable headless.
- Building the Tauri shell on Linux needs the WebKitGTK and ALSA development
  packages. If they are missing, build and test the crates on their own
  (`cargo test -p selah-engine`) and say so in the hand-off.
- Windows-only code (WASAPI specifics, ASIO, display enumeration) goes behind
  `cfg(windows)` with a stub for other platforms so the workspace always
  compiles everywhere.
- Never claim a Windows-only behaviour works unless it was run on Windows.

## Licensing and content rules

- The product will be **open source**, and the owner may charge for it or for
  a paid edition in future. The specific licence is not chosen yet. Do not add
  a `LICENSE` file or licence headers until the owner picks one.
- To keep every future option open (including a paid or dual-licensed
  edition), **add no GPL or AGPL dependencies**. MIT, Apache-2.0, BSD,
  MPL-2.0, and dynamically used LGPL are fine. Revisit this rule only if the
  owner chooses a GPL-family licence and records it in the decision log.
- ffmpeg must be an **LGPL build** (no `--enable-gpl`, no `--enable-nonfree`),
  shipped as a separate executable and credited in the about screen. That
  means no libx264 or libx265; encode H.264 with the Windows Media Foundation
  or hardware encoders, and VP9 with libvpx.
- ASIO needs the Steinberg SDK and its licence terms. Do not enable it without
  the owner's sign-off.
- **Never commit copyrighted lyrics, licensed Bible translations, or
  commercial media**, including as test fixtures. Use public-domain hymns
  ("Amazing Grace") and public-domain translations (KJV, WEB) only.
- Licensed lyric and scripture sources are user-authorised imports. Slides
  must be able to show the church's licence number and song copyright line.
- Never commit secrets, API keys, or a user's show files.

## Roadmap and module status

Update the status column as work lands: `not started`, `in progress`, `usable`,
`done`.

| Milestone | Scope | Status |
|---|---|---|
| M0 Scaffold | Cargo workspace, Tauri shell, React app, generated bindings, `pnpm check`, CI | not started |
| M1 Cue engine and audio | Core types, engine, show files and autosave, audio cues, fades, patches, panic, cue list and inspector UI, show mode | not started |
| M2 Lyrics and outputs | Song library, lyrics and scripture cues, slide navigation, main and stage outputs, layers and clear actions, text auto-fit | not started |
| M3 Video and media | ffmpeg sidecar, import and optimise, video and image cues, backgrounds, slide deck import, lower-third output | not started |
| M4 Multitracks | Track cues with stems, click and guide routing, section markers, loop and jump, pads | not started |
| M5 Control | Phone remote, MIDI and OSC in and out, foot pedal, Stream Deck | not started |
| M6 Integrations | Service plan import, lyric import formats, lighting scene recall | not started |

Out of scope for now: timecode chase, edge blending and projection mapping,
MIDI Show Control, macOS and Linux releases.

## Working in parallel sessions

Several sessions may work at once. Stay in your lane and protect the shared
contracts.

| Lane | Paths |
|---|---|
| Domain and persistence | `crates/selah-core`, `crates/selah-persist` |
| Cue engine | `crates/selah-engine` |
| Audio | `crates/selah-audio` |
| Media | `crates/selah-media` |
| Library | `crates/selah-library` |
| Control UI | `apps/desktop/src/features`, `components`, `styles` |
| Outputs | `apps/desktop/src/outputs` |
| Shell and IPC | `apps/desktop/src-tauri`, `apps/desktop/src/lib` |

- **Shared contracts** are `selah-core` types, the IPC commands and events, and
  the show file schema. Change them in small, separate commits, and call the
  change out in your hand-off, because other lanes build on them.
- If your task needs a change in another lane, make the smallest change that
  unblocks you and mention it, rather than refactoring someone else's module.
- Do not reformat or reorganise files you are not otherwise changing.
- Before starting, pull the latest default branch and re-read
  [Current state](#current-state). Before finishing, run `pnpm check`.
- End each session with a short hand-off: what changed, what was verified and
  how, what is unverified, and what should happen next.
- If you find a problem outside your task, report it; do not fix it silently
  in an unrelated change.

## Decision log

Append new entries at the bottom. One line each: date, decision, reason.

- 2026-11-12: Audience is churches and worship teams; the service plan is the
  top-level structure. Reason: shapes every priority below.
- 2026-11-12: Tauri 2 with a Rust core and a React/TypeScript front end.
  Reason: low-latency native audio with fast, good-looking web rendering for
  lyrics and slides.
- 2026-11-12: Output windows are webviews; video plays in the webview with an
  ffmpeg sidecar for probing, transcoding, and analysis. Reason: full codec
  coverage at import time without building a native video pipeline first.
- 2026-11-12: Video audio is extracted and played by the audio engine, with
  video slaved to the audio clock. Reason: webview audio cannot be routed to
  patches or faded with other cues.
- 2026-11-12: Shows reference logical patches, not device channels. Reason:
  shows must be portable between machines.
- 2026-11-12: Two-level navigation (GO between cues, next/previous within a
  presentation cue). Reason: lyrics must follow the worship leader without
  disturbing the cue list.
- 2026-11-12: The product is open source, possibly with paid offerings later;
  licence not yet chosen. Reason: owner's decision. GPL and AGPL dependencies
  stay banned so a paid edition remains possible.

## Open questions

Raise these with the owner before building on an assumption.

- Which open-source licence, and whether outside contributors must sign a
  contributor agreement (needed if a paid or dual-licensed edition is wanted).
- Time-stretch and pitch-shift library for multitracks.
- Which lyric and service-planning integrations matter most, and their terms.
- Whether a native video player is needed, and if so libmpv or a custom one.
- Installer, code signing, and auto-update approach.
