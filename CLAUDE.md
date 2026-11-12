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

Two documents define the project:

- **This file** says how the code is organised and the rules for working on it.
- **[docs/PRD.md](docs/PRD.md)** says what the product must do and why, as
  numbered requirements such as `CUE-6` and `AUD-4`. Read the PRD sections for
  your area before you start, and quote requirement IDs in commits and
  hand-offs. If you change behaviour the PRD specifies, update the PRD in the
  same change.

## Current state

- **M0 (scaffold) is done.** The workspace builds, `pnpm check` passes, and
  the app launches and shows an About panel whose details come from the Rust
  core through the generated IPC bindings.
- **M1 is in progress.** Done: `selah-core` (domain types, `validate`) and
  `selah-engine` (playhead, GO with debounce, pre/post-waits, continue and
  follow, groups together/sequence/shuffle/looped, fade, stop, pause, go-to
  and arm cues, panic with a second-press hard stop, reset, missing-media
  warnings, events). The engine is driven by the `Clock` and `AudioSink`
  traits; `selah_engine::testing` has `FakeClock` and `RecordingSink`.
  `selah-audio` is built: a real-time `Mixer` (voices, sample-accurate fades,
  pause ramps, per-channel peak meters, no allocation in the callback), a
  streaming decoder and resampler (symphonia and rubato, one worker thread per
  playing cue), patch routing, an `AudioEngine` that implements
  `selah_engine::AudioSink`, an `OfflineBackend` for tests, and a cpal
  `OutputDevice` (WASAPI on Windows). The cue engine plus audio engine are
  tested end to end with real WAV files and no device.
  `selah-host` runs both on one thread with the audio device, using the
  audio frame count as the master clock, and publishes a `Snapshot`. The shell
  exposes it as commands (`service_state`, `load_folder`, `go`, `skip`,
  `panic_stop`, `reset_show`), and the control window has a working operator
  panel: load a folder of audio files as cues, GO (Space), Skip, Reset, Panic
  (Esc), cue states, next marker, warnings, output meter. If no sound device
  can be opened the app still starts and says why. The UI polls every 100 ms;
  events and channels are not used yet.
  `selah-persist` reads and writes show folders: atomic saves (temp file,
  sync, rename), a rolling set of 20 backups in `backups/`, forward migrations
  over plain JSON, recovery from the newest readable backup when `show.json` is
  damaged, a debounced background `Autosaver`, and `missing_media`. The host
  can open a show, save it, and autosave edits (the one edit so far is arming
  and disarming a cue). The UI has Open show, Load folder, Save show, a
  per-cue Armed checkbox, and says where the show lives and whether saving
  failed.
  Still to do in M1: the undoable edit stack and the rest of live editing
  (add, move, delete, rename cues; waits, levels, fades, trims), the cue
  inspector and show mode, device settings and patch mapping, reconnecting
  after a device is lost (detected and reported, but not recovered), a preload
  so GO is instant (AUD-9), a recent-shows list, bundle export, and relinking
  missing media (it is listed but not fixable yet). Not started in audio:
  loudness measurement, a preview patch, ASIO.
  **Tried on Windows by the owner (2026-11-12):** playback, the safety keys,
  and error messages worked as described. That predates show files; the saving
  and opening flow has only been tested by automated tests.
- **What has been verified.** On Linux: the full gate, a production build,
  the dev workflow, and a launch on a virtual display. On Windows, in CI: the
  full gate and a build of the app. On the owner's Windows machine: audio
  playback and the operator panel, as of the host commit. To try a new build,
  run the "Windows build" workflow (Actions tab) and download its installer.
- **`main` exists** (created 2026-11-12 from the M0 commit). Sessions in
  restricted environments may still be limited to an assigned branch; see
  [Branches and commits](#branches-and-commits).

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
| Item | A segment of the service plan (pre-service, song, announcements, message). Holds cues. |
| Cue | One operable action: play audio, show lyrics, fade, stop, and so on. |
| Playhead | The cue that fires on the next GO. Shown as "standby". |
| GO | Fire the playhead cue and advance the playhead. Spacebar by default. |
| Live | A cue that is currently running or currently on an output. |
| Presentation cue | A cue with internal slides (lyrics, scripture, slide deck). |
| Section | A named part of a song (Verse 1, Chorus, Bridge). |
| Arrangement | The ordered list of sections used for one service. |
| Look | A reusable style for text on an output: font, sizes, colours, background. |
| Quick show | Putting a song or passage on screen that is not in the plan. |
| Layer | One compositing level of an output (see [Outputs](#outputs)). |
| Patch | A named logical audio output ("Main", "In-ears", "Stream") mapped to device channels per machine. |
| Stems | The separate audio files of a multitrack (drums, keys, click, guide). |
| Panic | Fade out and stop all audio within a short, configurable time. |
| Preflight | The check that a show is ready to run on this machine. |
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

```
apps/desktop/                 The app (pnpm package `@selah/desktop`)
  index.html, output.html     One entry page per kind of window
  src/
    app/                      Control window: shell and composition
    features/                 One folder per feature (about/, service/ so far)
    outputs/                  Output window renderers
    components/               Shared presentational pieces, no feature logic
    lib/                      bindings.ts (generated), ipc.ts, mount.tsx
    styles/                   tokens.css and index.css
    test/                     Test setup and shared test helpers
  tooling/                    The ESLint rule that enforces the layering
  src-tauri/                  The shell crate, `selah-desktop`
    src/                      lib.rs, ipc.rs, commands.rs, error.rs
    tests/                    The shell's tests (see Testing for why here)
    capabilities/             What each window may do
    icons/                    Generated from app-icon.svg (a placeholder mark)
    tauri.conf.json
crates/
  selah-core/                 Domain types, ids, validation. No I/O, no async.
  selah-engine/               Cue engine. Depends on core and on traits, not on audio.
  selah-audio/                Audio engine and backends.
  selah-media/                ffmpeg sidecar wrapper and media analysis.
  selah-library/              Song and scripture library, importers.
  selah-persist/              Show file load, save, migrate, autosave, bundle.
  selah-control/              MIDI, OSC, remote control.
  selah-host/                 The host loop: owns the device, cue engine, and
                              audio engine on one thread; publishes snapshots.
docs/PRD.md                   Product requirements
fixtures/                     Small test media and sample shows (public domain only)
scripts/check-crate-deps.mjs  Enforces the crate dependency rules
deny.toml                     Licence policy for Rust dependencies
.github/workflows/ci.yml      CI: the gate on Windows and Linux, plus licences
```

`src-tauri` must stay thin. If a command handler grows logic, move the logic
into a crate and test it there.

The shell already depends on every `selah-*` crate, and each crate already
depends on `selah-core`, so parallel sessions do not all need to edit the same
manifests to wire a crate in.

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
- Everything serialised, to the UI or to a show file, uses camelCase names
  (see [IPC contract](#ipc-contract)).

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
  Sources are reduced to mono or stereo when decoded; wider stems arrive in M4.
- Fades and stops follow the device's time, not the decoded samples, so a
  starved source cannot stretch a panic.
- A finished voice is handed back to the control thread to be dropped there;
  never drop one in `Mixer::render`. `selah-audio` depends on `selah-engine`
  for the `AudioSink` trait; the reverse is forbidden.
- Fades are sample-accurate ramps with selectable curves. A device underrun or
  a device disappearing must be reported and recovered from, not fatal.
- Meters and playback positions go to the UI on a throttled channel (about
  30 Hz), never one message per audio block.

Crates in use: `cpal` 0.18, `symphonia` 0.6 (MPL-2.0, allowed), `rubato` 5,
`rtrb` 0.4. All three audio libraries changed their APIs recently; read the
installed source, not memory. Time-stretch and pitch-shift for multitracks is
an open question; pick a permissively licensed library.

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
- An output with nothing on it is black. `outputs/OutputRoot.tsx` renders
  exactly that today.

## IPC contract

How the UI and the core talk. `apps/desktop/src-tauri/src/ipc.rs` is the single
list of everything exposed.

- **UI → core: Tauri commands**, `snake_case`, verb first (`go`, `stop_all`,
  `set_cue_level`). Every command returns `Result<T, AppError>`.
- **Core → UI: events** for state changes and **channels** for high-frequency
  streams (meters, positions). No events exist yet; `ipc.rs` says where to
  register the first one.
- **`AppError` is `{ kind, message }`.** `message` is plain language that is
  safe to show an operator. Crates define their own `thiserror` types and the
  command handler maps them. Add an `AppErrorKind` variant only when the UI
  must react differently to a new class of failure.
- **Names on the wire are camelCase.** Put `#[serde(rename_all = "camelCase")]`
  on every struct and enum that crosses IPC or is written to a show file.
- **The TypeScript side is generated** by tauri-specta into
  `apps/desktop/src/lib/bindings.ts`. After changing a command or a type it
  uses, run `pnpm bindings` and commit the file. A test in
  `src-tauri/tests/ipc_contract.rs` fails the gate when the file is stale.
  Never edit it, and never hand-write a type that mirrors a Rust type.
- **The UI calls commands through `call()`** from `lib/ipc.ts`:
  `const result = await call(commands.appInfo())`. It never throws. The result
  is `{ status: "ok", data }` or `{ status: "error", error }` with a
  well-formed `AppError`, even when the failure came from Tauri itself.
- **Tests stub commands with `mockCommands`** from the bindings. It is typed
  against the real signatures, so a changed command breaks the tests that
  depend on it at type-check time.
- Where a Rust type has an `Option` field, the bindings contain `X_Serialize`,
  `X_Deserialize`, and `X`. Use plain `X` in UI code.
- A change to a command or event updates the Rust side, the bindings, and
  every caller in the same commit.
- tauri-specta and specta are release candidates, pinned to exact versions in
  `src-tauri/Cargo.toml`. Bump them together, in their own commit.

## Show files and persistence

- A show is a folder: `show.json` plus its media. Cue `file` paths are
  relative to that folder. Export (not built yet) zips the folder into a
  single portable bundle.
- `show.json` is the `Show` type from `selah-core`, serialised as camelCase
  JSON. Backups live in `backups/` beside it and are not part of the show.
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

- Dark theme by default; a light theme redefines the same tokens. The theme is
  the `data-theme` attribute on `<html>`.
- **Every colour is a token** in `styles/tokens.css`, exposed to Tailwind in
  `styles/index.css` (`bg-surface`, `text-text-muted`, `border-border`).
  Tailwind's built-in palette is removed, so `bg-red-500` produces nothing.
  To add a colour, add a token in both themes.
- Spacing, radii, and type sizes use Tailwind's scale.
- System fonts only for the interface. Nothing is downloaded at run time.
- Every action has a keyboard shortcut and appears in a command palette.
- Show mode: large playhead and "next" display readable from two metres, a
  prominent GO, visible running cues with remaining time, and no modals.
- State is never shown by colour alone. Pair colour with an icon or label.
- Motion is short and functional. Nothing animates in a way that delays an
  operator action.
- Write UI text in plain, friendly language, in sentence case. Say what
  happened and what to do next ("Can't find intro.wav. Relink it or skip.").
- Interface text must be kept separate from code from the first real feature
  onward, so it can be translated (PRD `L10N-3`). The mechanism is not chosen
  yet; whoever builds the first M1 screen chooses it and records it here.

Front-end stack, installed: React 19, TypeScript 6 in strict mode, Vite 8,
Tailwind 4, Vitest with Testing Library, ESLint with typescript-eslint,
Prettier. Planned and to be added by the first feature that needs them:
Zustand for view state, Radix primitives for accessible components.

## Coding conventions

Rust:

- `cargo fmt` and `cargo clippy --all-targets -- -D warnings` must pass.
- No `unwrap`, `expect`, `panic!`, `todo!`, `dbg!`, or printing outside tests.
  Where an invariant makes one provably safe, allow that one line with
  `#[allow(...)]` and a comment saying why.
- Libraries use `thiserror`; the shell maps errors to `AppError`.
- `unsafe` is denied workspace-wide. Where it cannot be avoided, allow it on
  the smallest item and add a `// SAFETY:` comment.
- Crates depend downward only: everything may use `selah-core`; `selah-core`
  uses nothing of ours. `selah-engine` must not depend on `selah-audio`. Only
  the shell depends on Tauri.
- Lints live in `[workspace.lints]` in the root `Cargo.toml`; every crate opts
  in with `[lints] workspace = true`. Third-party dependencies shared by more
  than one crate are declared in `[workspace.dependencies]`.
- Use `tracing` for logs, never on the audio thread.

TypeScript:

- `strict` on, no `any`, no non-null assertions without a comment.
- Imports follow the layering below. Features do not import from other
  features; share through `components/` or `lib/`.
- Components render state and dispatch commands. Business rules belong in Rust.
- Import from other folders with the `@/` alias (`@/lib/ipc`).

| Folder under `src/` | May import from |
|---|---|
| `app/` | `app`, `features`, `components`, `lib`, `styles` |
| `features/<name>/` | itself, `components`, `lib`, `styles` |
| `outputs/` | `outputs`, `components`, `lib`, `styles` |
| `components/` | `components`, `lib`, `styles` |
| `lib/` | `lib` |
| `test/` | `test`, `components`, `lib`, `styles` (and any test file may import `test`) |

Both:

- Comments explain why, not what. No commented-out code.
- Do not add a dependency for something a few lines would do. Check a new
  dependency's licence and maintenance before adding it.

### What the tools enforce

These rules are checked by `pnpm check` or CI, so a violation fails the build
rather than waiting for a reviewer. If a rule needs to change, change the tool
and this file in the same commit.

| Rule | Enforced by |
|---|---|
| Crate dependency direction; only the shell uses Tauri | `scripts/check-crate-deps.mjs` |
| Front-end import layering | ESLint rule `selah/boundaries` (`apps/desktop/tooling/`) |
| Bindings match the Rust types; `AppError` keeps its shape | `apps/desktop/src-tauri/tests/ipc_contract.rs` |
| No `unwrap`, `panic!`, `todo!`, printing, undocumented `unsafe` | Clippy, via `[workspace.lints]` |
| Only token colours | Tailwind theme in `styles/index.css` |
| Dependency licences (Rust) | `cargo deny check licenses` in CI, policy in `deny.toml` |
| Formatting | rustfmt and Prettier |

## Branches and commits

Name every branch by its purpose:

- `feature/<feature-name>` for new behaviour (`feature/cue-follow-modes`)
- `fix/<name-of-fix>` for a defect (`fix/audio-device-loss`)
- `release/<release-name>` for release preparation (`release/0.1.0-alpha.1`)
- and the same pattern for other kinds of work: `docs/<topic>`,
  `refactor/<area>`, `test/<area>`, `ci/<change>`, `chore/<task>`

Names are lowercase, hyphen-separated, and short. One topic per branch,
branched from `main`.

If the environment a session runs in assigns it a branch and does not allow
pushing to any other, use the assigned branch and say so in the hand-off so
the owner can rename or merge it.

Commits are small and focused, in the form `area: summary`
(`audio: add equal-power fade curve`). Areas: `core`, `engine`, `audio`,
`media`, `library`, `persist`, `control`, `shell`, `ui`, `outputs`, `build`,
`ci`, `docs`, `chore`. Name the PRD requirements a commit implements in its
body.

## Testing and definition of done

- `selah-core`, `selah-engine`, `selah-persist`: thorough unit tests. The cue
  engine is tested with a fake clock; every follow mode, wait, and edge case
  (disarmed, missing media, stop during pre-wait) has a test.
- `selah-audio`: render through `OfflineBackend` and assert on the samples
  (levels, fade shapes, sample-accurate start times). No test may require a
  real audio device.
- Front end: Vitest and Testing Library, with tests beside the code as
  `*.test.ts(x)`. Stub the core with `mockCommands`. Playwright for the main
  operator flows once they exist.
- Persistence: round-trip and migration tests against fixtures. **A change to
  the `Show` format** bumps `CURRENT_SCHEMA_VERSION`, adds an entry to
  `MIGRATIONS` in `selah-persist`, and adds `fixtures/shows/v<N>/show.json`
  for the *old* version; `tests/fixtures.rs` opens every fixture. Fixtures are
  excluded from Prettier and must never be reformatted.
- Rust tests live in a `#[cfg(test)] mod tests` in the file they test, with
  one exception: **the shell crate's tests go in
  `apps/desktop/src-tauri/tests/`**. A unit-test program for that crate links
  Tauri's Windows libraries without the app's manifest, and Windows will not
  start it. `build.rs` attaches a manifest to programs built from `tests/`,
  and unit tests are switched off for the crate's library. This is one more
  reason to keep logic out of the shell and in the crates.

A change is done when:

1. `pnpm check` passes.
2. New behaviour has tests, or the hand-off note says why it cannot.
3. Anything that could only be verified on Windows with real hardware is
   listed explicitly as unverified.
4. This file, and the PRD where behaviour changed, are updated.

## Commands

Run from the repository root.

Install dependencies:

```bash
pnpm install
```

Run the app with live reload:

```bash
pnpm tauri dev
```

Run the whole gate: formatting, crate rules, clippy, Rust tests, type check,
ESLint, and front-end tests:

```bash
pnpm check
```

The same gate without the shell crate, for a Linux machine that lacks
WebKitGTK:

```bash
pnpm check:headless
```

Regenerate the TypeScript bindings after changing a command or its types:

```bash
pnpm bindings
```

Format everything:

```bash
pnpm format
```

Build the app without packaging an installer:

```bash
pnpm tauri build --debug --no-bundle
```

Narrower runs while working: `pnpm check:rust`, `pnpm check:web`,
`cargo test -p selah-engine`, and
`pnpm --filter @selah/desktop test:watch`.

### Environment notes

- The product targets **Windows 10 and 11**. Many sessions run in Linux
  containers with no audio device and no display. That is why engine logic sits
  behind traits and is testable headless.
- The Rust toolchain is pinned in `rust-toolchain.toml`; rustup downloads it on
  first use. Node 22 or newer and the pnpm version in `package.json` are
  required.
- Building the shell on Linux needs these packages:

  ```bash
  sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libayatana-appindicator3-dev libxdo-dev libssl-dev libasound2-dev
  ```

  Without them, use `pnpm check:headless` and say so in the hand-off.
- To look at the app on a Linux machine with no display, build it without
  packaging (above), then run it on a virtual display and take a screenshot.
  This needs `xvfb` and `imagemagick`:

  ```bash
  xvfb-run -a -s "-screen 0 1280x800x24" bash -c 'WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 ./target/debug/selah-desktop & sleep 12; import -window root app.png; kill $!'
  ```

- `pnpm --filter @selah/desktop dev` serves the UI alone on
  `http://localhost:1420`. In a plain browser there is no core, so commands
  fail and the UI shows its error states.
- Windows-only code (WASAPI specifics, ASIO, display enumeration) goes behind
  `cfg(windows)` with a stub for other platforms so the workspace always
  compiles everywhere.
- CI on Windows compiles the app and runs the tests. It does not launch the
  app. Never claim a Windows-only behaviour works unless it was run on
  Windows.

## Licensing and content rules

- The project is **Apache-2.0**; the text is in `LICENSE`. Crates inherit the
  licence from the workspace. Source files carry no licence header.
- The owner may offer paid additions later. To keep that possible, **add no
  GPL, AGPL, or LGPL crates**. Permissive licences and MPL-2.0 are fine. CI
  enforces this for Rust through `deny.toml`. There is no automated check for
  npm packages yet, so check the licence yourself when adding one.
- ffmpeg must be an **LGPL build** (no `--enable-gpl`, no `--enable-nonfree`),
  shipped as a separate executable and credited in the about screen. That
  means no libx264 or libx265; encode H.264 with the Windows Media Foundation
  or hardware encoders, and VP9 with libvpx.
- ASIO: the Steinberg SDK is offered under GPLv3 or Steinberg's own agreement.
  GPLv3 does not fit this project, so ASIO needs the owner to accept that
  agreement first. Do not enable it without the owner's sign-off.
- **Never commit copyrighted lyrics, licensed Bible translations, or
  commercial media**, including as test fixtures. Use public-domain hymns
  ("Amazing Grace") and public-domain translations (KJV, WEB) only.
- Licensed lyric and scripture sources are user-authorised imports. Slides
  must be able to show the church's licence number and song copyright line.
- Never commit secrets, API keys, or a user's show files.

## Roadmap and module status

Update the status column as work lands: `not started`, `in progress`, `usable`,
`done`. The PRD's requirement tables say exactly what each milestone contains,
and its release plan gives the exit criteria.

| Milestone | Scope | Status |
|---|---|---|
| M0 Scaffold | Workspace, Tauri shell, React app, generated bindings, `pnpm check`, CI | done |
| M1 Cue engine and audio | Core types (done), cue engine (done except undo), audio engine (done except preload and device recovery), show files and autosave, audio cues, fades, patches, panic, cue list and inspector, show mode | in progress |
| M2 Lyrics and outputs | Song library and import, lyrics and scripture cues, quick show, looks and text fitting, main and stage outputs, layers and clears, timers, preflight | not started |
| M3 Video and media | ffmpeg sidecar, import and optimise, video and image cues, slide decks and loops, backgrounds, lower third, cue cart | not started |
| M4 Multitracks | Track cues with stems, click and guide routing, sections, lyrics that follow tracks, pads, ASIO | not started |
| M5 Control | Phone remote, MIDI, OSC, local API, outputs as browser sources | not started |
| M6 Integrations | Service plan import, library migration, usage reports, lighting scene recall | not started |

Out of scope for now: timecode chase, edge blending and projection mapping,
MIDI Show Control, macOS and Linux releases. The PRD lists the rest.

## Working in parallel sessions

Several sessions may work at once. Stay in your lane and protect the shared
contracts.

Starting a task:

1. Read [Current state](#current-state) and the PRD sections for your area.
2. Create a branch named as in [Branches and commits](#branches-and-commits).
3. Run `pnpm install` and `pnpm check` so you know the gate is green before
   you change anything.

| Lane | Paths |
|---|---|
| Domain and persistence | `crates/selah-core`, `crates/selah-persist` |
| Cue engine | `crates/selah-engine` |
| Audio | `crates/selah-audio` |
| Media | `crates/selah-media` |
| Library | `crates/selah-library` |
| Control UI | `apps/desktop/src/app`, `features`, `components`, `styles` |
| Outputs | `apps/desktop/src/outputs` |
| Shell and IPC | `apps/desktop/src-tauri`, `apps/desktop/src/lib` |
| Tooling and CI | root config files, `scripts/`, `.github/`, `apps/desktop/tooling` |

- **Shared contracts** are `selah-core` types, the IPC commands and events, and
  the show file schema. Change them in small, separate commits, and call the
  change out in your hand-off, because other lanes build on them.
- **Generated and lock files** (`Cargo.lock`, `pnpm-lock.yaml`,
  `bindings.ts`) are never merged by hand. After a conflict, take either side,
  then regenerate with `cargo check`, `pnpm install`, or `pnpm bindings`.
- If your task needs a change in another lane, make the smallest change that
  unblocks you and mention it, rather than refactoring someone else's module.
- Do not reformat or reorganise files you are not otherwise changing.
- End each session with a short hand-off: what changed, which PRD requirements
  it covers, what was verified and how, what is unverified, and what should
  happen next.
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
- 2026-11-12: The licence is Apache-2.0, with paid additions possible later
  and no per-file headers. Reason: the owner's choice; GPL-family
  dependencies stay banned so a paid edition remains possible.
- 2026-11-12: Branches are named `feature/…`, `fix/…`, `release/…`, and so on.
  Reason: the owner's instruction.
- 2026-11-12: Everything serialised uses camelCase names. Reason: one rule for
  IPC and show files, and natural in TypeScript.
- 2026-11-12: Commands never throw in the UI; `call()` narrows every failure
  to an `AppError`. Reason: Tauri's own failures and ours look alike in
  JavaScript, and developer text must not reach an operator's screen.
- 2026-11-12: The Rust toolchain is pinned, and TypeScript is held at 6.x.
  Reason: identical lint results on every machine; typescript-eslint does not
  support TypeScript 7 yet.
- 2026-11-12: Architecture rules are enforced by tools in the gate. Reason:
  parallel sessions cannot rely on reviewing each other.
- 2026-11-12: The shell crate's tests live in `tests/`, with unit tests off
  for its library. Reason: its unit-test program could not start on Windows
  (seen in CI); the alternative, removing a Tauri feature, would have changed
  the app's own error dialog.
- 2026-11-12: A `selah-host` crate owns the device, cue engine, and audio
  engine on one thread. Reason: the device's stream must stay on one thread,
  and the loop is logic that belongs outside the shell.
- 2026-11-12: The cue engine's clock is the count of frames the audio device
  has played. Reason: waits and fades must stay tied to the sound (see
  Architecture rules).
- 2026-11-12: The UI polls `service_state` every 100 ms for now. Reason:
  simplest correct start; switch to events and channels when load or latency
  makes it worth it.
- 2026-11-12: Shows are plain JSON in a folder, written atomically with
  rolling backups, and autosaved 500 ms after the last change. Reason: a
  power cut or crash must never lose or corrupt a service (REL-7, FILE-2).
- 2026-11-12: Opening a show is validated in the shell command so errors
  reach the operator immediately; the host thread receives an already-loaded
  show. Reason: host commands are fire-and-forget.
- 2026-11-12: The app identifier is `io.github.kosenuel.selahstage`, for now.
  Reason: it is verifiably ours; changing it after a release moves users'
  data, so it must be settled before the first one.

## Open questions

Raise these with the owner before building on an assumption. PRD section 16
has the full list, with product questions and the author's leanings.

- Contributor terms (sign-off or agreement) before outside code is accepted.
- The ASIO licence route for multi-channel output in M4.
- Time-stretch and pitch-shift library for multitracks.
- How interface text is kept translatable (to be chosen in M1).
- Whether a native video player is needed, and if so libmpv or a custom one.
- Installer, code signing, update approach, and whether to ship a fixed
  webview version.
- The final app identifier and a trademark check on the name.
