//! The host loop end to end, with a silent output and real files.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use selah_core::CueId;
use selah_engine::CueState;
use selah_host::{Command, Host, Output, Snapshot, show_from_folder};
use selah_persist::{SHOW_FILE, load as load_show};

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

fn open(host: &Host, dir: &Path) {
    let loaded = load_show(dir).unwrap();
    assert!(host.send(Command::Open {
        show: Box::new(loaded.show),
        folder: dir.to_path_buf(),
        recovered_from: loaded.recovered_from,
    }));
}

fn cue_id(snapshot: &Snapshot, index: usize) -> CueId {
    CueId::from_uuid(snapshot.cues[index].id.parse().unwrap())
}

#[test]
fn a_loaded_folder_can_be_saved_and_then_opened_as_a_show() {
    let dir = tone_folder("save", &[("a.wav", 0.2), ("b.wav", 0.2)]);
    let host = Host::spawn(Output::Silent { channels: 2 }).unwrap();
    load(&host, &dir);
    let loaded = wait_for(&host, "the show to load", |s| s.cues.len() == 2);
    assert_eq!(loaded.location, None, "not saved yet");

    host.send(Command::SaveShow);
    let saved = wait_for(&host, "the save", |s| s.location.is_some());
    assert_eq!(saved.save_problem, None);
    assert!(dir.join(SHOW_FILE).is_file());

    let second = Host::spawn(Output::Silent { channels: 2 }).unwrap();
    open(&second, &dir);
    let reopened = wait_for(&second, "the show to open", |s| s.cues.len() == 2);
    assert_eq!(reopened.cues[0].name, "a");
    assert_eq!(reopened.cues[0].id, loaded.cues[0].id, "ids survive a save");
}

#[test]
fn an_edit_to_an_opened_show_is_autosaved() {
    let dir = tone_folder("autosave", &[("a.wav", 0.2)]);
    let host = Host::spawn(Output::Silent { channels: 2 }).unwrap();
    load(&host, &dir);
    wait_for(&host, "the show to load", |s| !s.cues.is_empty());
    host.send(Command::SaveShow);
    let saved = wait_for(&host, "the save", |s| s.location.is_some());

    host.send(Command::SetArmed {
        cue: cue_id(&saved, 0),
        armed: false,
    });

    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let on_disk = load_show(&dir).unwrap().show;
        if !on_disk.all_cues()[0].armed {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "the change never reached the disk"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let after = wait_for(&host, "the snapshot", |s| !s.cues[0].armed);
    assert_eq!(after.save_problem, None);
}

#[test]
fn a_disarmed_cue_is_skipped_but_the_playhead_moves_on() {
    let dir = tone_folder("disarm", &[("a.wav", 0.5), ("b.wav", 0.5)]);
    let host = Host::spawn(Output::Silent { channels: 2 }).unwrap();
    load(&host, &dir);
    let loaded = wait_for(&host, "the show to load", |s| s.cues.len() == 2);

    host.send(Command::SetArmed {
        cue: cue_id(&loaded, 0),
        armed: false,
    });
    host.send(Command::Go);

    let after = wait_for(&host, "the skip", |s| s.cues[0].state == CueState::Done);
    assert_eq!(after.playhead.as_deref(), Some(after.cues[1].id.as_str()));
}

#[test]
fn opening_a_show_with_a_missing_file_warns_about_it() {
    let dir = tone_folder("openmissing", &[("a.wav", 0.2), ("b.wav", 0.2)]);
    let host = Host::spawn(Output::Silent { channels: 2 }).unwrap();
    load(&host, &dir);
    wait_for(&host, "the show to load", |s| s.cues.len() == 2);
    host.send(Command::SaveShow);
    wait_for(&host, "the save", |s| s.location.is_some());
    std::fs::remove_file(dir.join("b.wav")).unwrap();

    let second = Host::spawn(Output::Silent { channels: 2 }).unwrap();
    open(&second, &dir);
    let snapshot = wait_for(&second, "the warning", |s| !s.warnings.is_empty());

    assert_eq!(snapshot.warnings[0].cue, "b");
    assert_eq!(
        snapshot.warnings[0].message,
        "Can't find b.wav. Relink it or skip."
    );
}

#[test]
fn a_recovered_show_says_so() {
    let dir = tone_folder("recover", &[("a.wav", 0.2)]);
    let host = Host::spawn(Output::Silent { channels: 2 }).unwrap();
    load(&host, &dir);
    let loaded = wait_for(&host, "the show to load", |s| !s.cues.is_empty());
    host.send(Command::SaveShow);
    wait_for(&host, "the save", |s| s.location.is_some());
    host.send(Command::SetArmed {
        cue: cue_id(&loaded, 0),
        armed: false,
    });
    // Wait for the autosave to make a backup, then damage the main file.
    let deadline = Instant::now() + Duration::from_secs(5);
    while !dir.join("backups").is_dir() {
        assert!(Instant::now() < deadline, "no backup appeared");
        std::thread::sleep(Duration::from_millis(20));
    }
    drop(host);
    std::fs::write(dir.join(SHOW_FILE), b"{ broken").unwrap();

    let second = Host::spawn(Output::Silent { channels: 2 }).unwrap();
    open(&second, &dir);
    let snapshot = wait_for(&second, "the notice", |s| s.notice.is_some());

    assert!(snapshot.notice.unwrap().contains("backup"));
}
