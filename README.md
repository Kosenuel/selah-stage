# Selah Stage

Selah Stage is a free, open-source Windows app that runs a whole church
service from one cue list: audio, lyrics, scripture, slides, video, backing
tracks, and stage displays. It is built for volunteer operators and for
worship that does not always follow the plan.

**Status: pre-alpha.** The project foundation is in place and checks itself
on Windows and Linux. There are no show features yet. See the
[release plan](docs/PRD.md#13-release-plan) for what comes next.

## Documents

- [docs/PRD.md](docs/PRD.md): what the product must do and why.
- [CLAUDE.md](CLAUDE.md): architecture, conventions, and how to work on the
  code. Written for AI coding sessions and just as useful for people.

## Develop

You need:

- [Rust](https://rustup.rs). The exact toolchain is pinned in
  `rust-toolchain.toml` and installs itself on first use.
- [Node.js](https://nodejs.org) 22 or newer, and pnpm (`corepack enable`).
- On Windows: the Microsoft C++ Build Tools and the WebView2 runtime
  (already present on Windows 11).
- On Linux (development only): the libraries listed under "Environment
  notes" in `CLAUDE.md`.

Install dependencies:

```bash
pnpm install
```

Run the app with live reload:

```bash
pnpm tauri dev
```

Run every check (formatting, lints, type checks, and tests). A change is not
done until this passes:

```bash
pnpm check
```

## Licence

[Apache-2.0](LICENSE).
