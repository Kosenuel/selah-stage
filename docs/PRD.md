# Selah Stage: product requirements

| | |
|---|---|
| Status | Draft 1, for the owner's review |
| Date | 2026-11-12 |
| Covers | Everything up to and including version 1.0, plus what is deliberately left for later |

This document says **what** Selah Stage must do and **why**. `CLAUDE.md` says
**how** the code is organised and the rules for working on it. Where the two
disagree about behaviour, this document wins; where they disagree about code
structure, `CLAUDE.md` wins. Fix the disagreement in the same change.

Every requirement has a stable ID such as `CUE-6`. Use the ID in commits,
pull requests, and hand-off notes. Never renumber; retire an ID by marking it
withdrawn.

Items marked **Proposal** are the author's recommendation and have not been
confirmed by the owner. Everything in [Open questions](#16-open-questions)
needs an owner decision.

## Contents

1. [Summary](#1-summary)
2. [The problem](#2-the-problem)
3. [Vision and position](#3-vision-and-position)
4. [Goals, non-goals, and measures](#4-goals-non-goals-and-measures)
5. [Users](#5-users)
6. [Scenarios](#6-scenarios)
7. [Landscape](#7-landscape)
8. [Product principles](#8-product-principles)
9. [Functional requirements](#9-functional-requirements)
10. [Quality requirements](#10-quality-requirements)
11. [Experience requirements](#11-experience-requirements)
12. [Content and licensing](#12-content-and-licensing)
13. [Release plan](#13-release-plan)
14. [Sustainability](#14-sustainability)
15. [Risks](#15-risks)
16. [Open questions](#16-open-questions)
17. [Out of scope](#17-out-of-scope)
- [Appendix A: cue types](#appendix-a-cue-types)
- [Appendix B: sources checked](#appendix-b-sources-checked)

## 1. Summary

Selah Stage is a free, open-source Windows app that runs a whole church
service from one cue list. The operator presses one key, GO, to move through
the service: walk-in music, countdown, songs with lyrics and backing tracks,
scripture, announcements, the message, and the close.

Today a church on Windows typically needs a presentation app for lyrics and
slides, a media player for music and video, a separate multitrack player for
click and stems, and something for timers. Volunteers switch between them
under pressure. Selah Stage replaces that set with one tool built around the
order of service, designed so that a volunteer who operates once a month can
run it safely, and so that lyrics follow the worship leader when the room
goes off-plan.

"Selah" is a word from the Psalms, commonly read as a pause.

## 2. The problem

**Too many tools.** Lyrics, audio, video, tracks, and timers live in separate
apps with separate windows, shortcuts, and failure modes. The operator's
attention is split exactly when it is most needed.

**Tools built for experts, run by volunteers.** Operators rotate weekly and
get minutes of training. Show-control and music-production tools assume a
professional who uses them daily. Mistakes are public: the wrong lyric, a
click track through the main speakers, a blank screen.

**Worship does not follow the plan.** Leaders repeat a chorus, skip a verse,
or add a song. Tools built around a fixed sequence make the operator scramble
to catch up.

**Windows is under-served.** The best-known cue-based show controller runs
only on macOS. The popular purpose-built multitrack players run on Apple
devices. Churches that own Windows PCs either buy new hardware or make do
with general-purpose music software.

**Budgets and connectivity are limited.** Many churches cannot justify
several subscriptions, run on donated PCs, and have no dependable internet in
the hall.

## 3. Vision and position

> One list, one key, one calm operator.

Selah Stage is for churches and worship teams who run services on Windows
with volunteer operators. It combines three things that are normally
separate:

- the **cue list and GO button** of theatre show control,
- the **lyrics, scripture, and stage display** of worship presentation
  software, and
- the **stems, click, and section control** of a multitrack player.

Its difference is not any single feature. It is that these share one plan, one
clock, and one set of safety rules, so that lyrics can follow tracks, a
single clear key always works, and nothing depends on a second app staying in
step.

## 4. Goals, non-goals, and measures

### Goals

| ID | Goal |
|---|---|
| G1 | One app runs a typical service from doors-open to dismissal. |
| G2 | A volunteer can operate a prepared service after a 15-minute orientation. |
| G3 | Selah Stage is never the reason a service stops. |
| G4 | Preparing a typical service takes under 20 minutes. |
| G5 | It runs well on the computers churches already own, with no internet. |
| G6 | Everything needed to run a service is free and open source. |

### Non-goals

- It is not a mixing desk. Microphones and the band are mixed elsewhere.
- It is not a video switcher, recorder, or streaming encoder.
- It is not a church management or service-planning system. It imports
  plans; it does not replace the planning tool.
- It is not a lighting console. At most it recalls scenes.
- It is not a content store. Churches bring their own songs, tracks, and
  media, under their own licences.
- It is not a general theatre show controller. Theatre use is welcome but
  does not drive priorities.

### Measures

How we will know the goals are met. Targets are **Proposals** until the owner
confirms them.

| Goal | Measure | Target |
|---|---|---|
| G1 | Partner churches that run a full service with no other playback or presentation app open | 3 of 3 by the end of beta |
| G2 | First-time volunteers who complete the operating script (run cues, advance lyrics, jump to a chorus, clear the screen, recover from a wrong press) unaided after 15 minutes | 9 of 10 |
| G3 | Service-stopping defects in consecutive field services before 1.0 | 0 in 50 (REL-1) |
| G3 | Continuous playback without dropout | 8 hours (REL-2) |
| G4 | Median time for a trained user to build a five-song service from a template and an existing library | 20 minutes or less |
| G5 | Performance budgets on reference hardware | All of section 10.2 |
| G6 | Requirements marked Must that need payment or an account | None |

Usage cannot be measured by default because the app sends nothing home
(SEC-1). Adoption will be judged from downloads, opt-in reports, and direct
contact with churches. See open question 7.

## 5. Users

### 5.1 People

**Volunteer operator.** Sits at the computer during the service, perhaps once
or twice a month. May have been shown the software for ten minutes. Needs to
know what is live, what happens next, and how to undo a wrong press. Fears
being the person who put the wrong thing on screen.

**Tech lead.** Often the only technically confident person, sometimes paid,
usually not. Builds the service during the week, trains volunteers, and is
phoned when something breaks. Needs fast preparation, files that move
between the office PC and the booth PC, and confidence the booth will work
without them.

**Worship leader.** Chooses songs, keys, and arrangements, and changes them
live. Needs the lyrics and the tracks to follow, and a way to signal or make
changes from the platform.

**Band member.** Hears click, guide, and stems in in-ear monitors and glances
at a stage display. Needs to see the current and next lyric and where the
song is going.

**Speaker.** Brings slides and scripture, sometimes minutes before the
service. Needs their slides to advance when they click, a timer they can see,
and a way to be told something quietly.

**Livestream operator.** Sometimes the same person as the operator. Needs
lyrics as a lower third and clean audio for the stream.

### 5.2 Environment

These assumptions shape the requirements. Where a church differs, the
defaults should still work.

- One Windows PC, often several years old, with integrated graphics and
  8 GB of memory.
- One operator screen and one to three outputs: a projector or TV, sometimes
  a stage screen, sometimes a feed to a video mixer.
- Sound goes to a separate mixing desk, from the headphone jack or a USB
  interface. Teams that use tracks have a multi-channel interface.
- No dependable internet in the hall.
- One operator. Occasionally a second person on the stream.
- Services follow a familiar shape each week; songs and the message change.
- Songs are sung under a church copyright licence; the scheme varies by
  country.
- More than one language may be in use in a single service.

## 6. Scenarios

### 6.1 A Sunday, start to finish

1. **Arrival.** The operator opens the app. Last night's show is at the top of
   the recent list. They open it and run preflight: all media found, both
   screens connected, audio device present. They switch to show mode.
2. **Doors open.** They press GO. Walk-in music starts as a shuffled group,
   an announcement loop runs on the main screen, and a countdown to 10:00
   appears in the corner.
3. **Service starts.** At zero the countdown cue follows into the opening
   video. Its sound comes through the same output as the music, which fades
   under it.
4. **Worship.** GO starts the first song: lyrics on the main screen and the
   stage display, and, for teams with tracks, stems to the room and click to
   the band's ears. The operator presses the right arrow for each slide, or
   lets the lyrics follow the track.
5. **Off-plan.** The leader goes back to the chorus. The operator presses C.
   The chorus appears; if tracks are running they jump on the next bar.
6. **Between songs.** A pad holds the key while the leader speaks. The next
   GO starts the next song and fades the pad.
7. **Welcome and notices.** Slides and a short video. The operator sends
   "2 minutes" to the stage display.
8. **Message.** The speaker's slides advance from their clicker. They quote a
   passage that is not in the plan; the operator types the reference into
   quick show and it is on screen in a few seconds.
9. **Close.** A response song, a closing slide, walk-out music.
10. **Second service.** "Reset show" returns everything to the top.

### 6.2 Mid-week preparation

The tech lead duplicates last Sunday's show, drops in this week's songs from
the library, picks arrangements, pastes in a new song from the leader's
email, imports the speaker's PDF, and attaches the tracks. Preflight flags
that one video still needs optimising and finishes it in the background. They
export a bundle to a USB stick for the booth PC.

### 6.3 When the room goes off-plan

| What happens | What the operator does |
|---|---|
| The leader repeats a chorus | Presses the section key; "next" carries on from there |
| The leader adds a song | Opens quick show, types part of the title, presses Enter |
| The speaker quotes an unplanned verse | Opens quick show, types the reference |
| The wrong slide is up | Presses previous, or clear text |
| A lyric has a typo | Fixes it on the live slide; the screen updates |
| Something needs to stop now | Presses panic |
| The speaker is running long | Sends a preset message to the stage display |

### 6.4 When things go wrong

| What happens | What the app does |
|---|---|
| A media file is missing | Skips the cue with a visible warning; GO still advances |
| The projector cable is pulled | Parks that output; nothing lands on the operator's screen; restores on reconnect |
| The audio interface is unplugged | Keeps running, warns, and resumes when it returns |
| An output window crashes | Recreates it and shows the same content within 2 seconds |
| The app crashes | Reopens the show with the playhead and screen content restored |
| Power is lost during a save | The previous good save still opens |
| The operator presses GO twice by accident | Ignores the second press |

## 7. Landscape

Understood as of November 2026 and not legal or purchasing advice. Verify
before quoting any of it publicly. Product names are used only to describe
the market.

| Category | Examples | Strengths | Gaps for our users |
|---|---|---|---|
| Theatre show control | QLab (macOS only); Windows tools such as Show Cue System | The cue list, GO, waits, follows, fades, precise audio routing | No lyrics, scripture, or song library; built for trained operators; the best-known tool is not on Windows |
| Worship presentation | ProPresenter, EasyWorship, Proclaim, MediaShout; open source: OpenLP, FreeShow, Quelea | Lyrics, scripture, slides, stage display, planning imports | Audio is basic or absent; no stems, click, or in-ear routing; a playlist of presentations rather than one cue list for everything |
| Multitrack players | Ableton Live; MultiTracks Playback; Loop Community Prime | Stems, click and guide routing, section jumps | Ableton is deep, general-purpose music software with a steep learning curve; the purpose-built players are confirmed for Apple devices only, and no Windows version was found (Appendix B) |

Churches on Windows therefore commonly run a presentation app, a media
player, and a music program side by side, sometimes linked by MIDI that only
the tech lead understands.

Notes for contributors:

- Several open-source presentation apps are under GPL licences. Their code
  must not be copied into this Apache-2.0 project.
- Do not copy another product's interface layout, artwork, or names. Learn
  from the category, then design for our principles.

## 8. Product principles

In priority order. The full wording is in `CLAUDE.md`.

1. **The service never stops.**
2. **Volunteer-proof.**
3. **Follow the room.**
4. **Offline first.**
5. **Fast on modest hardware.**
6. **Beautiful by default.**

When two requirements conflict, the one serving the higher principle wins.

## 9. Functional requirements

### 9.1 How to read these

**Priority**

- **Must**: the milestone is not finished without it.
- **Should**: planned for the milestone; may slip one milestone without
  blocking it.
- **Could**: wanted; built only if time allows.

**Milestone** is where the requirement is first delivered: `M1` to `M6`,
`1.0` for release hardening, or `Later` for after 1.0. Milestones are defined
in [Release plan](#13-release-plan).

Terms such as show, item, cue, playhead, patch, and layer are defined in the
glossary in `CLAUDE.md`.

### 9.2 Service plan

| ID | Requirement | Priority | Milestone |
|---|---|---|---|
| PLAN-1 | A show holds an ordered list of items. Each item has a title, a kind (song, scripture, message, announcement, media, general), optional notes, and an optional planned length, and contains cues. | Must | M1 |
| PLAN-2 | Items and cues can be added, renamed, reordered by drag or keyboard, duplicated, moved between items, and deleted. | Must | M1 |
| PLAN-3 | Items collapse and expand. The list keeps the playhead in view. | Must | M1 |
| PLAN-4 | Notes on items and cues are visible in show mode. | Must | M1 |
| PLAN-5 | "Reset show" returns every cue to idle, stops timers, and puts the playhead at the top, ready for the next service of the day. | Must | M1 |
| PLAN-6 | A new show can start from a template or from a copy of an earlier show, keeping its structure, looks, and patches. | Must | M2 |
| PLAN-7 | Any cue or item can be found by name or number from anywhere in the app. | Should | M1 |
| PLAN-8 | Starter templates for common service shapes ship with the app. | Should | M2 |
| PLAN-9 | Planned and actual time per item, with a running over or under total. | Could | M5 |
| PLAN-10 | A printable run sheet. | Could | Later |

### 9.3 Cues and transport

| ID | Requirement | Priority | Milestone |
|---|---|---|---|
| CUE-1 | GO fires the playhead cue and advances the playhead. Spacebar by default, and a large on-screen button in show mode. | Must | M1 |
| CUE-2 | The standby cue and the one after it are always visible, with their names and notes. | Must | M1 |
| CUE-3 | Every cue has a pre-wait, a post-wait, and a follow mode: none, continue (start the next cue when this one starts), or follow (start it when this one ends). | Must | M1 |
| CUE-4 | Group cues fire their children together, in sequence, or shuffled, with optional looping. | Must | M1 |
| CUE-5 | Control cues: fade to a level (optionally stopping at the end), stop, pause and resume, go to a cue, arm and disarm. Each targets one or more cues. | Must | M1 |
| CUE-6 | Panic fades out and stops all audio and video within a configurable time (default 1.5 seconds). A second press stops at once. | Must | M1 |
| CUE-7 | A second GO within a configurable window (default 300 ms) is ignored. | Must | M1 |
| CUE-8 | The operator can move the playhead without firing, skip a cue, and disarm a cue. A disarmed cue does nothing when fired, but the playhead still advances. | Must | M1 |
| CUE-9 | A running panel lists every live cue with elapsed and remaining time, and offers pause, fade out, and stop for each. | Must | M1 |
| CUE-10 | A cue whose media is missing is skipped with a visible warning. GO never blocks and never opens a dialog. | Must | M1 |
| CUE-11 | Every edit can be undone and redone for the whole session. | Must | M1 |
| CUE-12 | Cut, copy, paste, and duplicate work on one or many selected cues. | Must | M1 |
| CUE-13 | Cue numbers are assigned automatically, can be edited, and can be renumbered in bulk. | Should | M1 |
| CUE-14 | Default settings per cue type (fade time, patch, look) apply to new cues. | Should | M1 |
| CUE-15 | A cue can have its own hotkey that fires it wherever the playhead is. | Should | M3 |
| CUE-16 | A cue cart: a grid of buttons for sounds and clips fired on demand. | Should | M3 |
| CUE-17 | A cue can be scheduled to start at a clock time. | Should | M3 |
| CUE-18 | A timeline view of overlapping cues and fades. | Could | Later |

### 9.4 Audio

| ID | Requirement | Priority | Milestone |
|---|---|---|---|
| AUD-1 | Plays WAV, AIFF, FLAC, MP3, AAC (M4A), and Ogg Vorbis at any common sample rate and bit depth, converted to the device's rate. | Must | M1 |
| AUD-2 | Per cue: level, start and end trim, a loop count or endless loop, and fade in and fade out with a choice of curve. | Must | M1 |
| AUD-3 | Every start, stop, and level change is ramped. Nothing clicks or pops. | Must | M1 |
| AUD-4 | Patches are named logical outputs. A cue routes to one or more patches, each with its own level. Each machine maps patches to its device channels. | Must | M1 |
| AUD-5 | Output through WASAPI, shared by default with exclusive mode optional, with a choice of device, sample rate, and buffer size. | Must | M1 |
| AUD-6 | If the audio device disappears the app keeps running, warns clearly, and resumes when the device returns or the operator chooses another. | Must | M1 |
| AUD-7 | A level meter per patch with a clip indicator, and a master mute. | Must | M1 |
| AUD-8 | Setup offers a test tone and channel identification for each patch. | Must | M1 |
| AUD-9 | Cues near the playhead are loaded in advance so that GO is immediate (PERF-1). | Must | M1 |
| AUD-10 | The inspector shows the waveform with draggable trim points and audition. | Should | M1 |
| AUD-11 | A preview patch lets the operator audition a cue in headphones without it reaching the room. | Should | M3 |
| AUD-12 | Loudness is measured on import. One action normalises a cue to a target level without changing the file. | Should | M3 |
| AUD-13 | ASIO output for multi-channel interfaces. Depends on open question 3; AUD-15 is the fallback. | Must | M4 |
| AUD-14 | A cue can lower the level of other cues while it plays. | Could | M3 |
| AUD-15 | Output to more than one audio device at once. | Could | Later |

### 9.5 Songs and lyrics

| ID | Requirement | Priority | Milestone |
|---|---|---|---|
| SONG-1 | A song has a title, authors, a copyright line, a licence song number, a default key and tempo, tags, and languages. | Must | M2 |
| SONG-2 | Lyrics are stored as typed sections (verse, chorus, pre-chorus, bridge, tag, intro, outro, instrumental, custom), each a list of lines. Any line can carry a translation. | Must | M2 |
| SONG-3 | A song has one or more named arrangements. A show uses its own copy, so changing it never alters the library unless the operator saves it back. | Must | M2 |
| SONG-4 | Slides are made from sections by a maximum number of lines, and manual breaks are honoured. | Must | M2 |
| SONG-5 | While a lyrics cue is live the operator can go to the next or previous slide, jump to any section by click or hotkey, and repeat a section. After a jump, "next" continues from that point in the arrangement. | Must | M2 |
| SONG-6 | Quick show: find any song by title, first line, lyric text, or licence number and put it on screen without it being in the plan. | Must | M2 |
| SONG-7 | A typo on the live slide can be fixed in show mode. The outputs update at once, and the app offers to save the fix to the library afterwards. | Must | M2 |
| SONG-8 | A copyright slide or line shows the title, authors, copyright, and the church's licence number, placed according to a setting. | Must | M2 |
| SONG-9 | Songs import by pasting text (sections detected from labels such as "Verse 1" and "Chorus"), from plain text files, and from OpenLyrics files. | Must | M2 |
| SONG-10 | Import warns about likely duplicates. | Should | M2 |
| SONG-11 | ChordPro import, keeping the chords for later use. | Should | M2 |
| SONG-12 | A song exports as text or OpenLyrics. | Should | M2 |
| SONG-13 | Import of SongSelect files the church has downloaded under its own licence. | Should | M6 |
| SONG-14 | A log of the songs used in each service, exportable for licence reporting. | Should | M6 |
| SONG-15 | Chords on the stage display, with transposition. | Could | Later |

The starting rule for SONG-5: a jump lands on the next occurrence of that
section in the arrangement, or on the most recent one if none lies ahead.
Usability testing decides the final rule.

### 9.6 Looks and text

A look is a reusable style for text on an output.

| ID | Requirement | Priority | Milestone |
|---|---|---|---|
| LOOK-1 | A look sets font, size range, colour, alignment, line spacing, letter case, outline and shadow, text area, and default background. | Must | M2 |
| LOOK-2 | Looks apply to a whole show and can be overridden per item and per cue. | Must | M2 |
| LOOK-3 | Text always fits. It scales between the look's largest and smallest size, and the editor warns when a slide fits only at the smallest. Text is never clipped on an output. | Must | M2 |
| LOOK-4 | At least six finished looks ship with the app, with openly licensed fonts covering Latin (including tone-marked and accented letters), Greek, and Cyrillic. Other scripts use installed fonts, with a warning when a character cannot be drawn. | Must | M2 |
| LOOK-5 | Backgrounds (colour, gradient, image; looping video from M3) persist across slides and songs until changed. Nothing flashes between slides. | Must | M2 |
| LOOK-6 | Slides change by cut or dissolve, with the duration set by the look. Text and background change independently. | Must | M2 |
| LOOK-7 | Each output renders the same slide with its own look, so main, stage, and lower third can be styled differently. | Must | M2 |
| LOOK-8 | A readability aid (a scrim or text backing) keeps lyrics legible on busy backgrounds. | Should | M2 |
| LOOK-9 | Translation lines render in a secondary style with the primary line. | Should | M2 |
| LOOK-10 | A look editor with a live preview for every output. | Should | M3 |

### 9.7 Scripture

| ID | Requirement | Priority | Milestone |
|---|---|---|---|
| SCR-1 | Public-domain translations, at least KJV and WEB, are built in and work offline. | Must | M2 |
| SCR-2 | References can be typed freely ("jn 3:16-18", "1 cor 13:4-7, 13", "ps 23"), with book names completed as the operator types. | Must | M2 |
| SCR-3 | A passage becomes slides that fit the look, each showing its reference and translation. | Must | M2 |
| SCR-4 | Quick show works for scripture: any passage on screen in seconds, then verse by verse, continuing past the chosen range on demand. | Must | M2 |
| SCR-5 | Scripture can be searched by phrase. | Should | M2 |
| SCR-6 | Translations the church has rights to can be imported from open formats (OSIS, USFM, Zefania). | Should | M3 |
| SCR-7 | Two translations or two languages shown together. | Could | M6 |
| SCR-8 | Licensed translations through an online provider, where its terms allow storing them for offline use. | Could | Later |

### 9.8 Slides, images, and video

| ID | Requirement | Priority | Milestone |
|---|---|---|---|
| MED-1 | Image cues (PNG, JPEG, WebP) with fit modes, shown on the media layer or as a background. | Must | M3 |
| MED-2 | Video cues play any file ffmpeg can read. Files the app cannot play directly are optimised once, on import. | Must | M3 |
| MED-3 | Per video cue: trim, loop, level, fade in and out, and what happens at the end (hold the last frame, clear, or loop). | Must | M3 |
| MED-4 | Video sound plays through the audio engine and its patches, in sync with the picture (PERF-6). | Must | M3 |
| MED-5 | Import runs in the background with visible progress. It never delays GO and it resumes after a restart. | Must | M3 |
| MED-6 | PDF slide decks import as presentation cues, one slide per page, drawn at output resolution. | Must | M3 |
| MED-7 | A slide loop cycles images, slides, and clips with a duration for each. | Must | M3 |
| MED-8 | Looping video backgrounds behind lyrics and scripture. | Must | M3 |
| MED-9 | Built-in text slides (title, body, image) for announcements, styled by looks. | Should | M3 |
| MED-10 | PowerPoint decks import through PowerPoint or LibreOffice when one is installed. Otherwise the app explains how to export a PDF. Animations and embedded video are not kept. | Should | M3 |
| MED-11 | Video with transparency, for lower-third graphics. | Should | M3 |
| MED-12 | A media browser with thumbnails, search, and tags, covering the show's media and a shared folder. | Should | M3 |
| MED-13 | Live video from a camera or capture device as a cue. | Could | Later |

### 9.9 Outputs

| ID | Requirement | Priority | Milestone |
|---|---|---|---|
| OUT-1 | Output roles are main, stage, and lower third. Each is assigned to a display or switched off. | Must | M2 |
| OUT-2 | Setup shows an identifying label on every display and remembers the assignment on that machine. | Must | M2 |
| OUT-3 | If a display disconnects, its output is parked and never appears on the operator's screen. It returns when the display does. | Must | M2 |
| OUT-4 | Every output composites four layers, bottom to top: background, media, text, overlay. | Must | M2 |
| OUT-5 | Clear actions take one keypress in show mode: clear text, clear media, clear all, to logo, black. | Must | M2 |
| OUT-6 | The control window shows a live preview of every output and of the next slide. | Must | M2 |
| OUT-7 | An output window that fails is recreated automatically and shows the current content again (REL-4). | Must | M2 |
| OUT-8 | The lower-third output draws on a transparent or solid key colour for a video mixer. | Must | M3 |
| OUT-9 | Custom canvas sizes and shapes (LED walls, ultrawide screens), with safe margins. | Should | M3 |
| OUT-10 | An alert overlay on the main output, such as a nursery number, that clears itself. | Should | M3 |
| OUT-11 | Any output is available at a local web address, for use as a browser source in streaming software. | Should | M5 |
| OUT-12 | One role can be mirrored to several displays. | Could | M3 |
| OUT-13 | NDI output. | Could | Later |

### 9.10 Stage display

| ID | Requirement | Priority | Milestone |
|---|---|---|---|
| STG-1 | Shows the current slide text, the next slide text, and the section names. | Must | M2 |
| STG-2 | Shows a clock, active timers, and the next cue or item. | Must | M2 |
| STG-3 | Shows operator messages: free text or presets, with an attention flash, clearing themselves after a set time. | Must | M2 |
| STG-4 | Ready-made layouts for a band, a speaker, and minimal use. | Must | M2 |
| STG-5 | Text is legible from six metres on a floor monitor and is never cut off. | Must | M2 |
| STG-6 | The speaker layout shows the current and next slide images, a speaker timer, and the clock. | Must | M3 |
| STG-7 | A layout editor to arrange what the stage display shows. | Should | M3 |
| STG-8 | The stage display opens in any browser on the local network, such as a tablet on a music stand. | Should | M5 |

### 9.11 Timers and messages

| ID | Requirement | Priority | Milestone |
|---|---|---|---|
| TMR-1 | A timer counts down to a clock time, counts down a duration, or counts up. Each has a name. | Must | M2 |
| TMR-2 | Timers appear on the stage display and, as an overlay, on the main output. | Must | M2 |
| TMR-3 | Timer cues start, stop, and reset timers. The operator can add or remove a minute while one runs. | Must | M2 |
| TMR-4 | A timer that overruns changes colour and label and keeps counting. | Must | M2 |
| TMR-5 | A countdown to a clock time reaches zero within 100 ms of the system clock. | Must | M2 |
| TMR-6 | An item timer compares elapsed time with the item's planned length. | Should | M3 |

### 9.12 Multitracks and pads

| ID | Requirement | Priority | Milestone |
|---|---|---|---|
| TRK-1 | A track cue plays a set of stems in sample-accurate sync, imported from a folder or a zip file. | Must | M4 |
| TRK-2 | Per stem: level, mute, solo, and patch. The mix is saved with the song. | Must | M4 |
| TRK-3 | Click and guide stems are recognised by name, routed to the in-ears patch, and kept out of the main patch unless the operator deliberately overrides it. | Must | M4 |
| TRK-4 | Section markers sit on the track's timeline with a tempo and time-signature map. Markers are imported when the files carry them and can be edited by hand or by tapping along. | Must | M4 |
| TRK-5 | Live arrangement: jump to a section, repeat the current one, skip the next, or end after this one. Changes land on a musical boundary, a bar or a section end. | Must | M4 |
| TRK-6 | A lyrics cue linked to a track cue follows its sections automatically, including jumps and repeats. The operator can always override. | Must | M4 |
| TRK-7 | Pads: an ambient pad per key from the church's own files, with crossfades between keys and an automatic fade under a starting track. | Must | M4 |
| TRK-8 | Stems play from disk with bounded memory use (PERF-8). | Must | M4 |
| TRK-9 | Slide timing inside a section can be captured by tapping during rehearsal and replayed. | Should | M4 |
| TRK-10 | A built-in click made from the tempo map, with a count-in, when there is no click stem. | Should | M4 |
| TRK-11 | Key and tempo changes, rendered in the background before the service rather than live. | Should | M4 |
| TRK-12 | Section control from the platform by pedal, MIDI, or phone. | Should | M5 |
| TRK-13 | A set of original pads shipped with the app. | Could | Later |

### 9.13 Control

| ID | Requirement | Priority | Milestone |
|---|---|---|---|
| CTL-1 | Every action has a keyboard shortcut and appears in a command palette. A default map ships (section 11.2). | Must | M1 |
| CTL-2 | Standard presentation clickers move to the next and previous slide with no setup. | Must | M2 |
| CTL-3 | A remote for phones and tablets, served by the app on the local network with nothing to install. Devices pair by QR code and PIN. | Must | M5 |
| CTL-4 | Remote roles limit what a device may do: operator, worship leader, speaker, viewer. | Must | M5 |
| CTL-5 | MIDI input can be mapped to any action, with a learn mode. | Must | M5 |
| CTL-6 | All network control is off by default, limited to the local network, shows who is connected, and can be revoked. | Must | M5 |
| CTL-7 | Shortcuts can be changed, with warnings about conflicts. | Should | M5 |
| CTL-8 | MIDI output cues: notes, control changes, and program changes. | Should | M5 |
| CTL-9 | OSC input and output with a documented address list. | Should | M5 |
| CTL-10 | The local interface the remote uses is documented, so control surfaces such as a Stream Deck can be supported through existing tools. | Should | M5 |
| CTL-11 | Cues that control streaming software: start, stop, and change scene. | Could | M6 |
| CTL-12 | Lighting scene recall, by sending a stored sACN or Art-Net snapshot or by triggering a console over MIDI or OSC. | Could | M6 |

### 9.14 Integrations

| ID | Requirement | Priority | Milestone |
|---|---|---|---|
| INT-1 | Import a service plan from Planning Center Services: items, order, songs matched to the library, lengths, and notes. Importing again shows what changed and never silently overwrites local edits. | Should | M6 |
| INT-2 | Song libraries migrate from other presentation apps where their formats allow. | Could | M6 |
| INT-3 | Other planning tools, according to demand. | Could | Later |

### 9.15 Files and library

| ID | Requirement | Priority | Milestone |
|---|---|---|---|
| FILE-1 | A show is a folder holding a readable show file and its media. | Must | M1 |
| FILE-2 | Every committed edit is saved automatically, and rolling backups are kept. | Must | M1 |
| FILE-3 | Older shows always open. Every format change ships with a migration. | Must | M1 |
| FILE-4 | Media can be copied into the show or referenced where it is. "Consolidate" copies everything in. | Must | M1 |
| FILE-5 | Missing media is listed when a show opens and can be relinked by searching a folder. | Must | M1 |
| FILE-6 | The start screen lists recent shows. | Must | M1 |
| FILE-7 | A show exports to and imports from a single bundle file, checked for completeness on import. | Must | M2 |
| FILE-8 | Preflight checks that media is present and playable, patches are mapped, displays are connected, fonts are available, disk space is sufficient, and the licence number is set. Each finding offers a fix. | Must | M2 |
| FILE-9 | The library of songs, looks, templates, and scripture backs up to and restores from one file. | Must | M2 |
| FILE-10 | Machine settings are kept apart from shows and can be exported and imported. | Should | M2 |
| FILE-11 | The library can be shared between machines through a folder the church already synchronises. | Could | Later |

### 9.16 Safety and recovery

| ID | Requirement | Priority | Milestone |
|---|---|---|---|
| SAFE-1 | Show mode locks the structure of the show. It allows only operating actions: GO, navigation, clears, fixing live text, trimming levels, adjusting timers, and sending messages. | Must | M1 |
| SAFE-2 | No dialog that takes focus ever appears in show mode. Problems appear as banners. | Must | M1 |
| SAFE-3 | GO, panic, and the clear keys work whichever part of the window has focus, and typing in a field never fires a cue. | Must | M1 |
| SAFE-4 | Only one copy of the app runs. Launching it again brings the first to the front. | Must | M1 |
| SAFE-5 | In show mode the computer is kept awake and its displays on. | Must | M1 |
| SAFE-6 | After a crash the app reopens the show with the playhead and screen content restored. Audio does not restart by itself. | Must | M2 |
| SAFE-7 | A timestamped log records every GO, clear, warning, and error, and can be read after the service. | Must | M2 |
| SAFE-8 | A diagnostics file with logs and system details, and no lyrics or media, can be exported for a bug report. | Should | M2 |
| SAFE-9 | Preflight warns about conditions outside the app that threaten a service: a pending restart, low battery, low disk space. | Should | M3 |
| SAFE-10 | Rehearsal mode runs the show with outputs hidden or muted. | Could | M5 |
| SAFE-11 | A second machine mirrors the first and can take over. | Could | Later |

### 9.17 Setup and updates

| ID | Requirement | Priority | Milestone |
|---|---|---|---|
| SET-1 | First-run setup covers displays, the audio device and patches, the licence number, and a default look. It can be skipped. | Must | M2 |
| SET-2 | A sample show made from public-domain content demonstrates the main features. | Should | M2 |
| SET-3 | A five-minute interactive tour teaches a new volunteer to operate. | Should | M3 |
| SET-4 | The app installs for the current user without administrator rights and on a machine with no internet. | Must | 1.0 |
| SET-5 | Updates are looked for only when online, are never installed during show mode, and are never required for the app to keep working. | Must | 1.0 |
| SET-6 | Releases are signed, so Windows does not warn when installing. | Must | 1.0 |
| SET-7 | Uninstalling leaves libraries and shows in place unless the user chooses otherwise. | Should | 1.0 |

## 10. Quality requirements

All quality requirements are Must for 1.0 unless marked otherwise. Each
milestone's exit criteria name the ones that apply earlier.

### 10.1 Reliability

| ID | Requirement |
|---|---|
| REL-1 | No service-stopping defect in 50 consecutive field services before 1.0. |
| REL-2 | Eight hours of continuous playback with no audio dropout or drift on reference hardware. |
| REL-3 | A fault in the interface, such as a crashed or frozen window, never interrupts audio. |
| REL-4 | A failed output window is back, showing the right content, within 2 seconds. |
| REL-5 | After an app crash the operator can carry on within 10 seconds of relaunching. |
| REL-6 | Fault tests pass before every release: audio device unplugged, display unplugged, disk full, media deleted, media corrupt, and the computer trying to sleep. |
| REL-7 | An interrupted save never leaves a show that cannot be opened. |

### 10.2 Performance

Measured on **reference hardware**: a 2019-class laptop with a four-core
processor (Intel Core i5-8250U or similar), 8 GB of memory, integrated
graphics, a solid-state disk, Windows 10 22H2, and two external 1080p
displays.

| ID | Requirement |
|---|---|
| PERF-1 | From GO to sound for a loaded audio cue: under 20 ms plus the device buffer. |
| PERF-2 | From a keypress to the new slide on an output: under 50 ms. |
| PERF-3 | The interface holds 60 frames per second during playback, and dissolves drop no frames. |
| PERF-4 | From launch to an operable window: under 5 seconds. Opening a 300-cue show: under 2 seconds. |
| PERF-5 | 16 stereo audio cues play at once without dropouts at a 10 ms buffer. |
| PERF-6 | Video picture and sound stay within 40 ms of each other for the length of any clip. |
| PERF-7 | 1080p H.264 video plays on two outputs at once without dropped frames. |
| PERF-8 | A 24-stem track plays using under 300 MB of memory for audio. |
| PERF-9 | Idle processor use is under 5%, and total memory is under 1.5 GB in a typical service. |

### 10.3 Platform

| ID | Requirement |
|---|---|
| PLAT-1 | Runs on Windows 10 (22H2) and Windows 11, 64-bit Intel and AMD. |
| PLAT-2 | Every Must requirement works with no network connection, except those in Control and Integrations that are network features by nature. |
| PLAT-3 | No account, sign-in, or activation is needed. |
| PLAT-4 | Windows on Arm. (Could) |
| PLAT-5 | macOS and Linux are not release targets. The code must keep building on Linux, where much of the development happens. |

### 10.4 Privacy and security

| ID | Requirement |
|---|---|
| SEC-1 | Nothing leaves the machine unless the user turns on a feature that needs it. There is no analytics by default. |
| SEC-2 | Crash and usage reports are opt-in and never include lyrics, media, or file names. |
| SEC-3 | Network services listen only when enabled, only on the local network, and only to paired devices. |
| SEC-4 | Sign-in details for integrations are kept in the Windows credential store, never in a show. |
| SEC-5 | Opening a show or importing a file from someone else cannot run code. |
| SEC-6 | The licences of everything shipped with the app are listed inside it. |

### 10.5 Accessibility

| ID | Requirement |
|---|---|
| A11Y-1 | Every operating action is available from the keyboard. |
| A11Y-2 | State is never shown by colour alone. |
| A11Y-3 | Text and controls in the control window meet WCAG 2.2 AA contrast in both themes. |
| A11Y-4 | In show mode the standby cue, GO, and what is live are readable from two metres. |
| A11Y-5 | The control window respects the system's reduced-motion and text-size settings. |
| A11Y-6 | Controls are named for screen readers. (Should) |

### 10.6 Languages and scripts

| ID | Requirement | Priority | Milestone |
|---|---|---|---|
| L10N-1 | Lyrics and scripture render correctly in any script the available fonts support, including right-to-left scripts and combining marks. | Must | M2 |
| L10N-2 | Search ignores case, accents, tone marks, and punctuation. | Must | M2 |
| L10N-3 | Interface text is kept separate from code from the first feature onward, so translations can be added. | Must | M1 |
| L10N-4 | Dates and times follow the system's regional settings. | Should | M2 |
| L10N-5 | Translations of the interface, starting with languages partner churches ask for. | Could | Later |

## 11. Experience requirements

### 11.1 The control window

The control window has two modes. The sketch below shows the information
show mode must carry. It illustrates content and priority, not a visual
design.

```
┌──────────────────────────────────────────────────────────────────────────┐
│ Sunday 10:00 service        SHOW MODE         10:02:41        1 warning  │
├───────────────────────────┬─────────────────────────────┬────────────────┤
│ PLAN                      │ LIVE                        │ OUTPUTS        │
│ ▸ Pre-service       done  │ Amazing Grace               │ ┌────────────┐ │
│ ▾ Worship                 │ Verse 2 · slide 3 of 9      │ │ Main       │ │
│   3 Amazing Grace   live  │ ┌─────────────────────────┐ │ └────────────┘ │
│   4 Pad in G     running  │ │ the slide now on screen │ │ ┌────────────┐ │
│ ▶ 5 Be Thou My Vision     │ └─────────────────────────┘ │ │ Stage      │ │
│   6 Fade pad              │ Next: Chorus                │ └────────────┘ │
│ ▸ Welcome                 │ [V1] [V2] [C] [V3] [V4]     │ RUNNING        │
│ ▸ Message                 │ Clear: text media all logo  │ Pad in G  2:14 │
├───────────────────────────┴─────────────────────────────┴────────────────┤
│ [          GO          ]   Next: 5 Be Thou My Vision            [ Panic ] │
└──────────────────────────────────────────────────────────────────────────┘
```

| ID | Requirement |
|---|---|
| UX-1 | Show mode always answers three questions at a glance: what is live, what happens on the next GO, and what is on each screen. |
| UX-2 | Edit mode adds an inspector for the selected cue and tools for building the plan. Switching modes never moves or resizes what is common to both. |
| UX-3 | The mode in force is unmistakable, and changing it is a deliberate action. |
| UX-4 | Warnings accumulate in one place and never cover the plan, the live panel, or GO. |
| UX-5 | Every destructive action can be undone, and the interface says so when it happens. |
| UX-6 | The app is dark by default, with a light theme for bright rooms. |
| UX-7 | Nothing animates in a way that delays an operator action. |
| UX-8 | Lyrics and slides look finished with no design work: a new song on a default look is fit to project. |

### 11.2 Default keys

A **Proposal**, to be settled by usability testing. Letter and number keys act
only in show mode while a lyrics cue is live and no text field has focus.

| Action | Key |
|---|---|
| GO | Space |
| Panic | Esc (press again to stop at once) |
| Move the playhead down or up | Down, Up |
| Next or previous slide | Right, Left; Page Down, Page Up |
| Jump to verse 1 to 9 | 1 to 9 |
| Jump to chorus, pre-chorus, bridge, tag, intro, outro | C, P, B, T, I, O |
| Clear text, clear media, clear all | F1, F2, F3 |
| To logo, black | F4, F5 |
| Quick show and command palette | Ctrl+K |
| Undo, redo | Ctrl+Z, Ctrl+Y |
| Switch between edit and show mode | Ctrl+E |

Known tension: some presentation clickers send Esc, B, or a full stop from
their extra buttons, which would trigger panic or a section jump. See open
question 11.

### 11.3 Voice

- Plain words, sentence case, no blame. Say what happened and what to do
  next: "Can't find intro.wav. Relink it or skip."
- A small taught vocabulary (cue, GO, patch, look) used consistently. No
  other jargon.
- Neutral across traditions. "Service", "message", and "song" rather than
  terms tied to one denomination, and nothing that assumes a style of
  worship.

## 12. Content and licensing

- **The app ships no copyrighted songs or licensed Bible text.** Samples use
  public-domain hymns and public-domain translations only.
- **Churches bring their own content** under their own licences. The app
  helps them comply: it shows the licence number and copyright line
  (SONG-8) and keeps a usage log (SONG-14).
- **Imports read files the user provides.** The app does not fetch lyrics
  from websites, and that will not be added.
- **Tracks, pads, and backgrounds** are the church's own. Anything the
  project ships must be original or released for unrestricted reuse.
- **Fonts** shipped with the app use open font licences.
- **Code** is Apache-2.0. Dependencies must be permissively licensed; the
  rules, including those for ffmpeg and ASIO, are in `CLAUDE.md`.
- **Other products' names** appear only descriptively. Their logos and
  artwork are never used.

## 13. Release plan

Milestones build on each other, and each ends with something a church can
use. The order is deliberate: reliability of audio and cues first, then the
Sunday essentials, then breadth.

| Milestone | Outcome | Exit criteria |
|---|---|---|
| **M0 Scaffold** | The project builds, tests, and checks itself. | `pnpm check` passes on Windows and Linux in CI. |
| **M1 Cue engine and audio** (alpha 1) | A dependable cue-based audio player for walk-in music, stings, and playback. | All M1 Musts. PERF-1 and PERF-5 met. REL-2 passes for audio. The AUD-6 fault test passes. A tester builds and runs a 30-cue show from the written guide alone. |
| **M2 Lyrics and outputs** (alpha 2) | A church can run a real Sunday: songs, scripture, timers, main and stage screens. | All M2 Musts. PERF-2 met. Fault tests for OUT-3 and REL-4 pass. One partner church runs a full service on it, with its usual tools on standby. A first usability round with new volunteers is done. |
| **M3 Video and media** (beta) | It replaces the presentation app and media player for a typical service, including video, slide decks, and the lower third. | All M3 Musts. PERF-6 and PERF-7 met. Three partner churches use it weekly for four weeks with no open service-stopping defect. |
| **M4 Multitracks** | Bands play to stems with click and guide in their ears, and lyrics follow the tracks. | All M4 Musts. PERF-8 met. TRK-3 is proven by test. A band plays a full set to tracks in rehearsal and in a service. |
| **M5 Control** | The platform and the booth share control: phone remote, MIDI, pedals. | All M5 Musts. The network features pass a security review against SEC-3. The remote works on current iOS and Android browsers. |
| **1.0** | A release churches can install and trust. | Every Must through M5 and every `1.0` requirement. REL-1 met. All of section 10 met. A user guide exists. |
| **M6 Integrations** | Less retyping: plans and libraries come in from other tools. | Delivered in releases after 1.0, in order of demand. |

**Proposal:** 1.0 does not wait for M6. See open question 12.

## 14. Sustainability

Everything in this section is a **Proposal**. Nothing here is decided.

The owner has said the project is open source and that charging for
something may be considered later.

**Suggested guardrails**

- Everything needed to run a service stays free and Apache-2.0 (G6).
- Reliability and safety are never paid features.
- No advertising, and no sale of data.

**Options that fit those guardrails**

| Option | What is paid for | Note |
|---|---|---|
| Donations and sponsorship | Nothing; goodwill | Simple; unpredictable |
| Paid support and onboarding | People's time | Scales with staff, not with users |
| Hosted library sync and backup | A service that costs money to run | Needs accounts and infrastructure; conflicts with nothing above |
| Original content packs | Pads, backgrounds, looks made for the project | Needs content creators |
| Managed deployment for multi-site churches | Central configuration and updates | Only relevant at scale |

To keep paid or dual-licensed editions possible, the project needs a
decision on contributor terms before it accepts outside code (open
question 2).

## 15. Risks

| Risk | Why it matters | Response |
|---|---|---|
| Audio behaves differently across Windows devices and drivers | A dropout in a service breaks principle 1 | Test on a range of real interfaces; recover from device loss (AUD-6); soak tests (REL-2); conservative defaults |
| Video in a webview cannot hold sync or frame rate on modest hardware | Videos are central from M3 | Audio clock as master; budgets PERF-6 and PERF-7; a native player can replace the webview behind the same interface |
| Text fitting and non-Latin scripts are harder than they look | Clipped or broken lyrics are the most visible failure | Fit is a Must from M2 (LOOK-3); test with real songs in several scripts; warn rather than guess |
| Development happens on Linux, the product ships on Windows | Windows-only faults go unseen | Windows CI on every change; a real-hardware checklist before each release; never claim untested Windows behaviour |
| The scope spans three product categories | Breadth before depth yields three weak products | Strict milestone order; each milestone must be usable in a service before the next begins |
| A content licensing mistake | Legal exposure for churches and the project | Ship no copyrighted content; import only user files; rules in section 12 |
| Third-party terms (planning APIs, lyric file formats, NDI, ASIO) | A feature may be unbuildable or unshippable | Review terms before building; keep every integration optional |
| Churches will not move a live service to unproven software | No field use means no evidence of reliability | Partner churches; run beside existing tools first; publish what was tested |
| A small team, possibly one maintainer | Stalls and lost knowledge | `CLAUDE.md`, tests as documentation, small independent modules |
| The binding generator is a release candidate | An upstream change could break the build | Versions pinned exactly; all use is confined to one module |
| The system webview updates itself | Behaviour can change mid-season | Consider shipping a fixed webview version (open question 4); test against the current one in CI |
| Codec patents and the ffmpeg build | Distribution risk | Use encoders provided by Windows and the graphics hardware; LGPL build only; legal check before the first public release |
| The remote opens a network port | An attack surface in a public building | Off by default, local network only, pairing, roles, and a review before release |

## 16. Open questions

Each needs a decision from the owner. The author's lean is given where there
is one.

1. **Pilot churches.** Which churches will test alphas and the beta, and who
   speaks for them? The exit criteria from M2 depend on this.
2. **Contributor terms.** A sign-off (DCO) or a contributor agreement (CLA)?
   Lean: decide before the first outside contribution; a CLA keeps a paid
   edition possible, a DCO is lighter.
3. **ASIO.** Steinberg now offers the SDK under either its own agreement or
   GPLv3. GPLv3 does not fit an Apache-2.0 product that may have a paid
   edition, so this needs the proprietary agreement or a different route to
   multi-channel output.
4. **Installer, signing, and updates.** Lean: a per-user installer, a
   code-signing certificate before beta, and updates that are offered, never
   forced. Also: ship a fixed webview version or use the one Windows
   provides?
5. **Name and identifier.** A trademark check on "Selah Stage" before beta.
   The app identifier is currently `io.github.kosenuel.selahstage`; changing
   it after release moves users' data.
6. **Which integrations first,** and are their terms acceptable?
7. **Opt-in usage reports.** Would an anonymous, opt-in "this install was
   used this week" signal be acceptable, to measure adoption?
8. **Time-stretch and pitch-shift library** for TRK-11, under a permissive
   licence.
9. **Shipped content.** Which fonts, and who makes original looks,
   backgrounds, and pads?
10. **Native video.** Decide after M3 measurements whether the webview is
    enough.
11. **Panic key and clickers.** Keep Esc for panic, or move it, given that
    some clickers send Esc?
12. **What gates 1.0.** Lean: M1 to M5 and hardening; M6 follows.
13. **Regions.** Which countries and languages come first? This affects
    licensing schemes, bundled translations, and fonts.

## 17. Out of scope

Not planned for 1.0. Raise a proposal before building any of these.

- Timecode chase and MIDI Show Control
- Projection mapping and edge blending
- Lighting programming beyond scene recall
- Live audio inputs and mixing
- Video switching, recording, and streaming
- Native mobile apps (the browser remote covers phones and tablets)
- Cloud accounts and hosted sync
- A plugin system
- macOS and Linux releases
- A store for songs, tracks, or media
- Fetching lyrics from the internet

## Appendix A: cue types

| Cue type | What it does | Milestone |
|---|---|---|
| Audio | Plays a sound file | M1 |
| Fade | Changes the level of other cues over time; can stop them | M1 |
| Stop, Pause | Stops or pauses other cues | M1 |
| Go to | Moves the playhead | M1 |
| Arm | Arms or disarms other cues | M1 |
| Group | Holds cues that fire together, in sequence, or shuffled | M1 |
| Memo | A note in the list; does nothing | M1 |
| Lyrics | Shows a song, slide by slide | M2 |
| Scripture | Shows a passage, slide by slide | M2 |
| Timer | Starts, stops, or resets a timer | M2 |
| Image | Shows a picture | M3 |
| Video | Plays a video with sound | M3 |
| Slides | Shows an imported deck or a slide loop | M3 |
| Text | Shows built-in announcement slides | M3 |
| Track | Plays stems with sections | M4 |
| Pad | Plays an ambient pad in a key | M4 |
| MIDI, OSC | Sends a message to other equipment | M5 |
| Light | Recalls a lighting scene | M6 |

## Appendix B: sources checked

Checked on 2026-11-12 for section 7 and open question 3. Other statements in
section 7 are from general knowledge and should be verified before public
use.

- MultiTracks Playback, platforms: <https://multitracks.com/playback> and
  <https://apps.apple.com/us/app/playback/id751755884>
- Loop Community Prime, platforms:
  <https://support.loopcommunity.com/article/2485-what-is-prime> and
  <https://apps.apple.com/app/id1289813088>
- ASIO licensing: Steinberg press release of 2025-10-15,
  <https://ocl-steinberg-live.steinberg.net/_storage/asset/808575/storage/master/Press%20Release%20-%202025-10-15%20-%20OBS%20Partnership-%20EN.pdf>,
  and coverage at <https://heise.de/-10963451>

An Android version of each multitrack player is mentioned by third-party
listings only and was not confirmed. No Windows version of either was found.
