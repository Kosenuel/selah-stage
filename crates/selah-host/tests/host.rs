//! The host loop end to end, with a silent output and real files.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use selah_core::{ControlKind, CueChange, CueId, Edit, EditError, GroupMode, NewAudio};
use selah_engine::CueState;
use selah_host::{Command, EditFailure, Host, Output, Snapshot, show_from_folder};
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

fn arm(host: &Host, cue: CueId, armed: bool) {
    host.edit(Edit::Change {
        id: cue,
        change: CueChange::Armed(armed),
    })
    .unwrap();
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

    arm(&host, cue_id(&saved, 0), false);

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

    arm(&host, cue_id(&loaded, 0), false);
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
    arm(&host, cue_id(&loaded, 0), false);
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

fn loaded_host(tag: &str, files: &[(&str, f64)]) -> (Host, PathBuf, Snapshot) {
    let dir = tone_folder(tag, files);
    let host = Host::spawn(Output::Silent { channels: 2 }).unwrap();
    load(&host, &dir);
    let snapshot = wait_for(&host, "the show to load", |s| s.cues.len() == files.len());
    (host, dir, snapshot)
}

#[test]
fn adding_a_cue_shows_up_and_becomes_playable() {
    let (host, dir, loaded) = loaded_host("add", &[("a.wav", 0.2)]);
    write_tone(&dir.join("b.wav"), 0.2);

    let outcome = host
        .edit(Edit::AddAudio {
            files: vec![NewAudio {
                file: "b.wav".into(),
                name: "b".into(),
            }],
            before: None,
        })
        .unwrap();

    assert_eq!(outcome.created.len(), 1);
    let after = wait_for(&host, "the new cue", |s| s.cues.len() == 2);
    assert_eq!(after.cues[1].name, "b");
    assert_eq!(after.cues[1].audio.as_ref().unwrap().file, "b.wav");
    assert_eq!(after.cues[0].id, loaded.cues[0].id);

    host.send(Command::Go);
    // A second press inside the double-press window would be ignored.
    std::thread::sleep(Duration::from_millis(450));
    host.send(Command::Go);
    wait_for(&host, "both cues to have run", |s| {
        s.cues
            .iter()
            .all(|c| matches!(c.state, CueState::Running | CueState::Done))
    });
}

#[test]
fn undo_and_redo_restore_the_cue_list() {
    let (host, _dir, loaded) = loaded_host("undo", &[("a.wav", 0.2), ("b.wav", 0.2)]);
    assert!(!loaded.can_undo);

    host.edit(Edit::Delete {
        ids: vec![cue_id(&loaded, 0)],
    })
    .unwrap();
    let deleted = wait_for(&host, "the delete", |s| s.cues.len() == 1);
    assert!(deleted.can_undo);

    host.undo().unwrap();
    let restored = wait_for(&host, "the undo", |s| s.cues.len() == 2);
    assert_eq!(
        restored.cues[0].id, loaded.cues[0].id,
        "the same cue comes back"
    );
    assert!(restored.can_redo);

    host.redo().unwrap();
    wait_for(&host, "the redo", |s| s.cues.len() == 1);
}

#[test]
fn bad_values_are_refused_with_a_plain_message() {
    let (host, _dir, loaded) = loaded_host("invalid", &[("a.wav", 0.2)]);

    let result = host.edit(Edit::Change {
        id: cue_id(&loaded, 0),
        change: CueChange::PreWait(-3.0),
    });

    assert!(matches!(
        result,
        Err(EditFailure::Invalid(EditError::InvalidValue(_)))
    ));
    assert_eq!(host.snapshot().cues[0].pre_wait, 0.0);
}

#[test]
fn show_mode_locks_out_edits_and_edit_mode_allows_them_again() {
    let (host, _dir, loaded) = loaded_host("locked", &[("a.wav", 0.2)]);
    let edit = |host: &Host| {
        host.edit(Edit::Change {
            id: cue_id(&loaded, 0),
            change: CueChange::Notes("hello".into()),
        })
    };

    host.send(Command::SetEditing(false));
    wait_for(&host, "show mode", |s| !s.editing);
    assert_eq!(edit(&host), Err(EditFailure::Locked));
    assert_eq!(host.undo(), Err(EditFailure::Locked));

    host.send(Command::SetEditing(true));
    assert!(edit(&host).is_ok());
    wait_for(&host, "the note", |s| s.cues[0].notes == "hello");
}

#[test]
fn editing_with_no_show_open_says_so() {
    let host = Host::spawn(Output::Silent { channels: 2 }).unwrap();
    assert_eq!(host.edit(Edit::Renumber), Err(EditFailure::NoShow));
}

#[test]
fn an_edit_during_playback_does_not_stop_the_sound() {
    let (host, _dir, loaded) = loaded_host("live", &[("long.wav", 5.0), ("b.wav", 0.2)]);
    host.send(Command::Go);
    wait_for(&host, "the cue to run", |s| {
        s.cues[0].state == CueState::Running
    });

    host.edit(Edit::Change {
        id: cue_id(&loaded, 1),
        change: CueChange::Name("renamed".into()),
    })
    .unwrap();

    let after = wait_for(&host, "the rename", |s| s.cues[1].name == "renamed");
    assert_eq!(after.cues[0].state, CueState::Running);
}

#[test]
fn edits_to_a_saved_show_are_autosaved_and_undo_is_saved_too() {
    let (host, dir, loaded) = loaded_host("editsave", &[("a.wav", 0.2)]);
    host.send(Command::SaveShow);
    wait_for(&host, "the save", |s| s.location.is_some());

    host.edit(Edit::Change {
        id: cue_id(&loaded, 0),
        change: CueChange::Name("Opening".into()),
    })
    .unwrap();
    wait_until("the rename on disk", || {
        load_show(&dir).unwrap().show.all_cues()[0].name == "Opening"
    });

    host.undo().unwrap();
    wait_until("the undo on disk", || {
        load_show(&dir).unwrap().show.all_cues()[0].name == "a"
    });
}

fn wait_until(what: &str, check: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !check() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn a_group_runs_its_children_and_the_snapshot_shows_them_nested() {
    let (host, _dir, loaded) =
        loaded_host("group", &[("a.wav", 0.6), ("b.wav", 0.6), ("c.wav", 0.2)]);

    let group = host
        .edit(Edit::Group {
            ids: vec![cue_id(&loaded, 0), cue_id(&loaded, 1)],
        })
        .unwrap()
        .created[0];
    let grouped = wait_for(&host, "the group", |s| s.cues.len() == 2);
    assert_eq!(
        grouped.cues[0].group.as_ref().unwrap().mode,
        GroupMode::Together
    );
    assert_eq!(grouped.cues[0].children.len(), 2);
    assert_eq!(grouped.cues[0].children[0].name, "a");

    host.send(Command::Go);
    let running = wait_for(&host, "both children to play", |s| {
        s.cues[0]
            .children
            .iter()
            .all(|c| c.state == CueState::Running)
    });
    assert_eq!(running.cues[0].state, CueState::Running);
    wait_for(&host, "the group to finish", |s| {
        s.cues[0].state == CueState::Done
    });

    host.edit(Edit::Change {
        id: group,
        change: CueChange::GroupMode(GroupMode::Sequence),
    })
    .unwrap();
    wait_for(&host, "sequence mode", |s| {
        s.cues[0]
            .group
            .as_ref()
            .is_some_and(|g| g.mode == GroupMode::Sequence)
    });
}

#[test]
fn a_fade_and_stop_cue_work_on_what_is_playing() {
    let (host, _dir, loaded) = loaded_host("control", &[("long.wav", 10.0)]);
    let target = cue_id(&loaded, 0);
    let fade = host
        .edit(Edit::AddControl {
            kind: ControlKind::Fade,
            target,
            before: None,
        })
        .unwrap()
        .created[0];
    host.edit(Edit::Change {
        id: fade,
        change: CueChange::FadeDuration(0.2),
    })
    .unwrap();
    let with_fade = wait_for(&host, "the fade cue", |s| s.cues.len() == 2);
    let detail = with_fade.cues[1].control.as_ref().unwrap();
    assert_eq!(detail.kind, ControlKind::Fade);
    assert_eq!(detail.target_name.as_deref(), Some("long"));
    assert!(detail.stop_when_done);

    host.send(Command::Go);
    wait_for(&host, "the audio to play", |s| {
        s.cues[0].state == CueState::Running
    });
    std::thread::sleep(Duration::from_millis(450));
    host.send(Command::Go);

    wait_for(&host, "the fade to stop the audio", |s| {
        s.cues[0].state == CueState::Stopped && s.cues[1].state == CueState::Done
    });
}

#[test]
fn a_control_cue_whose_target_was_deleted_says_so_and_does_nothing() {
    let (host, _dir, loaded) = loaded_host("dangling", &[("a.wav", 0.2), ("b.wav", 0.2)]);
    let (a, b) = (cue_id(&loaded, 0), cue_id(&loaded, 1));
    host.edit(Edit::AddControl {
        kind: ControlKind::Stop,
        target: a,
        before: None,
    })
    .unwrap();
    wait_for(&host, "the stop cue", |s| s.cues.len() == 3);

    host.edit(Edit::Delete { ids: vec![a] }).unwrap();

    let after = wait_for(&host, "the delete", |s| s.cues.len() == 2);
    assert_eq!(after.cues[1].control.as_ref().unwrap().target_name, None);
    host.send(Command::Skip);
    host.send(Command::Go);
    wait_for(&host, "the stop cue to finish quietly", |s| {
        s.cues[1].state == CueState::Done
    });
    assert_eq!(cue_id(&after, 0), b);
}
