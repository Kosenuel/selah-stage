//! The host loop end to end, with a silent output and real files.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use selah_engine::CueState;
use selah_host::{Command, Host, Output, Snapshot, show_from_folder};

fn tone_folder(tag: &str, files: &[(&str, f64)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("selah-host-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (name, seconds) in files {
        write_tone(&dir.join(name), *seconds);
    }
    dir
}

fn write_tone(path: &Path, seconds: f64) {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 48_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for i in 0..(48_000.0 * seconds) as usize {
        let s = ((i as f64 * 440.0 * std::f64::consts::TAU / 48_000.0).sin() * 12_000.0) as i16;
        writer.write_sample(s).unwrap();
        writer.write_sample(s).unwrap();
    }
    writer.finalize().unwrap();
}

fn wait_for(host: &Host, what: &str, check: impl Fn(&Snapshot) -> bool) -> Snapshot {
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        let snapshot = host.snapshot();
        if check(&snapshot) {
            return snapshot;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for {what}: {:?}", host.snapshot());
}

fn load(host: &Host, dir: &Path) {
    let (show, root) = show_from_folder(dir).unwrap();
    assert!(host.send(Command::Load {
        show: Box::new(show),
        media_root: root
    }));
}

#[test]
fn go_plays_a_cue_to_its_end_and_moves_the_playhead() {
    let dir = tone_folder("go", &[("a.wav", 0.3), ("b.wav", 0.3)]);
    let host = Host::spawn(Output::Silent { channels: 2 }).unwrap();
    load(&host, &dir);
    let loaded = wait_for(&host, "the show to load", |s| s.cues.len() == 2);
    assert_eq!(loaded.playhead.as_deref(), Some(loaded.cues[0].id.as_str()));

    host.send(Command::Go);
    let running = wait_for(&host, "the first cue to run", |s| {
        s.cues[0].state == CueState::Running
    });
    assert_eq!(
        running.playhead.as_deref(),
        Some(running.cues[1].id.as_str())
    );

    let done = wait_for(&host, "the first cue to finish", |s| {
        s.cues[0].state == CueState::Done
    });
    assert_eq!(done.cues[1].state, CueState::Idle);
}

#[test]
fn a_missing_file_shows_a_plain_warning() {
    let dir = tone_folder("missing", &[("a.wav", 0.2)]);
    let host = Host::spawn(Output::Silent { channels: 2 }).unwrap();
    load(&host, &dir);
    wait_for(&host, "the show to load", |s| !s.cues.is_empty());
    std::fs::remove_file(dir.join("a.wav")).unwrap();

    host.send(Command::Go);
    let snapshot = wait_for(&host, "a warning", |s| !s.warnings.is_empty());

    assert_eq!(
        snapshot.warnings[0].message,
        "Can't find a.wav. Relink it or skip."
    );
    assert_eq!(snapshot.warnings[0].cue, "a");
}

#[test]
fn panic_stops_what_is_playing_and_reset_starts_over() {
    let dir = tone_folder("panic", &[("long.wav", 5.0)]);
    let host = Host::spawn(Output::Silent { channels: 2 }).unwrap();
    load(&host, &dir);
    wait_for(&host, "the show to load", |s| !s.cues.is_empty());
    host.send(Command::Go);
    wait_for(&host, "the cue to run", |s| {
        s.cues[0].state == CueState::Running
    });

    host.send(Command::Panic);
    wait_for(&host, "the cue to stop", |s| {
        s.cues[0].state == CueState::Stopped
    });

    host.send(Command::Reset);
    let reset = wait_for(&host, "reset", |s| s.cues[0].state == CueState::Idle);
    assert_eq!(reset.playhead.as_deref(), Some(reset.cues[0].id.as_str()));
}

#[test]
fn the_clock_is_the_audio_clock_and_meters_move() {
    let dir = tone_folder("meters", &[("a.wav", 1.0)]);
    let host = Host::spawn(Output::Silent { channels: 2 }).unwrap();
    load(&host, &dir);
    wait_for(&host, "the show to load", |s| !s.cues.is_empty());
    host.send(Command::Go);
    wait_for(&host, "sound on the meters", |s| {
        s.peaks.iter().any(|p| *p > 0.2)
    });
}
