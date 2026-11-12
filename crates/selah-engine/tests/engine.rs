//! Behaviour of the cue engine, driven by a fake clock and a recording sink.

use selah_core::{Cue, CueId, CueKind, FadeCurve, Follow, GroupMode, Item, ItemKind, Show};
use selah_engine::testing::{Call, FakeClock, RecordingSink};
use selah_engine::{CueState, Engine, EngineConfig, Event, Warning};

struct Rig {
    engine: Engine<FakeClock, RecordingSink>,
    clock: FakeClock,
    sink: RecordingSink,
}

impl Rig {
    fn new(cues: Vec<Cue>) -> Self {
        let (clock, sink) = (FakeClock::default(), RecordingSink::default());
        let mut engine = Engine::new(clock.clone(), sink.clone(), EngineConfig::default());
        let mut show = Show::new("test");
        let mut item = Item::new("item", ItemKind::General);
        item.cues = cues;
        show.items.push(item);
        engine.load(&show);
        sink.calls.borrow_mut().clear();
        Self {
            engine,
            clock,
            sink,
        }
    }

    /// Moves time on and ticks, the way the host does.
    fn advance(&mut self, seconds: f64) {
        self.clock.advance(seconds);
        self.engine.tick();
    }

    fn state(&self, cue: &Cue) -> CueState {
        self.engine.state(cue.id)
    }
}

fn audio(name: &str) -> Cue {
    Cue::new(
        name,
        CueKind::Audio {
            file: format!("{name}.wav"),
            level: 0.0,
            start: 0.0,
            end: None,
            loops: None,
            fade_in: 0.0,
            fade_out: 0.0,
            curve: FadeCurve::EqualPower,
            patches: Vec::new(),
        },
    )
}

fn memo(name: &str) -> Cue {
    Cue::new(name, CueKind::Memo)
}

fn group(name: &str, mode: GroupMode, children: Vec<Cue>, looped: bool) -> Cue {
    Cue::new(
        name,
        CueKind::Group {
            mode,
            children,
            looped,
        },
    )
}

#[test]
fn go_fires_the_playhead_cue_and_advances() {
    let (a, b) = (audio("a"), audio("b"));
    let mut rig = Rig::new(vec![a.clone(), b.clone()]);
    assert_eq!(rig.engine.playhead_cue(), Some(a.id));

    rig.engine.go();

    assert_eq!(rig.state(&a), CueState::Running);
    assert_eq!(rig.state(&b), CueState::Idle);
    assert_eq!(rig.engine.playhead_cue(), Some(b.id));
    assert_eq!(rig.sink.calls(), [Call::Start(a.id)]);
}

#[test]
fn the_playhead_stops_at_the_end_and_go_does_nothing_there() {
    let mut rig = Rig::new(vec![memo("only")]);
    rig.engine.go();
    assert_eq!(rig.engine.playhead_cue(), None);
    rig.advance(1.0);
    rig.engine.go();
    assert!(rig.sink.calls().is_empty());
}

#[test]
fn a_second_go_inside_the_window_is_ignored() {
    let mut rig = Rig::new(vec![audio("a"), audio("b")]);
    rig.engine.go();
    rig.advance(0.1);
    rig.engine.go();
    assert_eq!(rig.sink.calls().len(), 1);
    rig.advance(0.5);
    rig.engine.go();
    assert_eq!(rig.sink.calls().len(), 2);
}

#[test]
fn a_disarmed_cue_does_nothing_but_the_playhead_still_advances() {
    let mut a = audio("a");
    a.armed = false;
    let b = audio("b");
    let mut rig = Rig::new(vec![a.clone(), b.clone()]);

    rig.engine.go();

    assert!(rig.sink.calls().is_empty());
    assert_eq!(rig.state(&a), CueState::Done);
    assert_eq!(rig.engine.playhead_cue(), Some(b.id));
}

#[test]
fn missing_media_warns_skips_and_never_blocks() {
    let (a, b) = (audio("a"), audio("b"));
    let mut rig = Rig::new(vec![a.clone(), b.clone()]);
    rig.sink.missing.borrow_mut().insert("a.wav".into());

    rig.engine.go();

    assert_eq!(rig.state(&a), CueState::Done);
    assert_eq!(rig.engine.playhead_cue(), Some(b.id));
    assert!(rig.engine.take_events().contains(&Event::Warning {
        cue: a.id,
        warning: Warning::MissingMedia {
            file: "a.wav".into()
        },
    }));
}

#[test]
fn a_pre_wait_delays_the_action() {
    let mut a = audio("a");
    a.pre_wait = 2.0;
    let mut rig = Rig::new(vec![a.clone()]);

    rig.engine.go();
    assert_eq!(rig.state(&a), CueState::PreWait);
    rig.advance(1.9);
    assert!(rig.sink.calls().is_empty());
    rig.advance(0.2);
    assert_eq!(rig.state(&a), CueState::Running);
    assert_eq!(rig.sink.calls(), [Call::Start(a.id)]);
}

#[test]
fn continue_starts_the_next_cue_after_the_post_wait() {
    let mut a = audio("a");
    a.follow = Follow::Continue;
    a.post_wait = 1.0;
    let b = audio("b");
    let mut rig = Rig::new(vec![a.clone(), b.clone()]);

    rig.engine.go();
    assert_eq!(rig.state(&b), CueState::Idle);
    rig.advance(1.0);
    assert_eq!(rig.state(&b), CueState::Running);
    assert_eq!(rig.state(&a), CueState::Running);
}

#[test]
fn continue_with_no_wait_starts_the_next_cue_at_once() {
    let mut a = memo("a");
    a.follow = Follow::Continue;
    let b = audio("b");
    let mut rig = Rig::new(vec![a, b.clone()]);

    rig.engine.go();

    assert_eq!(rig.state(&b), CueState::Running);
}

#[test]
fn follow_starts_the_next_cue_when_this_one_finishes() {
    let mut a = audio("a");
    a.follow = Follow::Follow;
    let b = audio("b");
    let mut rig = Rig::new(vec![a.clone(), b.clone()]);

    rig.engine.go();
    rig.advance(30.0);
    assert_eq!(rig.state(&b), CueState::Idle);

    rig.engine.audio_finished(a.id);

    assert_eq!(rig.state(&a), CueState::Done);
    assert_eq!(rig.state(&b), CueState::Running);
}

#[test]
fn a_fade_cue_fades_its_target_and_can_stop_it() {
    let a = audio("a");
    let fade = Cue::new(
        "fade",
        CueKind::Fade {
            target: a.id,
            to: -60.0,
            duration: 2.0,
            curve: FadeCurve::Linear,
            stop_when_done: true,
        },
    );
    let mut rig = Rig::new(vec![a.clone(), fade.clone()]);
    rig.engine.go();
    rig.advance(1.0);
    rig.engine.go();

    assert_eq!(rig.state(&fade), CueState::Running);
    assert!(rig.sink.calls().contains(&Call::Fade(a.id, -60.0)));
    rig.advance(1.9);
    assert_eq!(rig.state(&a), CueState::Running);
    rig.advance(0.2);
    assert_eq!(rig.state(&fade), CueState::Done);
    assert_eq!(rig.state(&a), CueState::Stopped);
}

#[test]
fn a_stop_cue_stops_its_target_with_a_short_ramp() {
    let a = audio("a");
    let stop = Cue::new("stop", CueKind::Stop { target: a.id });
    let mut rig = Rig::new(vec![a.clone(), stop]);
    rig.engine.go();
    rig.advance(1.0);
    rig.engine.go();

    assert_eq!(rig.state(&a), CueState::Stopped);
    assert!(rig.sink.calls().contains(&Call::Stop(a.id, 0.05)));
}

#[test]
fn pause_and_resume_hold_a_cue_and_its_remaining_wait() {
    let mut a = audio("a");
    a.pre_wait = 4.0;
    let mut rig = Rig::new(vec![a.clone()]);
    rig.engine.go();
    rig.advance(1.0);

    rig.engine.pause(a.id);
    assert_eq!(rig.state(&a), CueState::Paused);
    rig.advance(100.0);
    assert_eq!(rig.state(&a), CueState::Paused);

    rig.engine.resume(a.id);
    rig.advance(2.9);
    assert_eq!(rig.state(&a), CueState::PreWait);
    rig.advance(0.2);
    assert_eq!(rig.state(&a), CueState::Running);
}

#[test]
fn pausing_a_running_audio_cue_pauses_the_audio() {
    let a = audio("a");
    let mut rig = Rig::new(vec![a.clone()]);
    rig.engine.go();
    rig.engine.pause(a.id);
    rig.engine.resume(a.id);
    assert_eq!(
        rig.sink.calls(),
        [Call::Start(a.id), Call::Pause(a.id), Call::Resume(a.id)]
    );
}

#[test]
fn a_go_to_cue_moves_the_playhead_without_firing() {
    let (a, c) = (memo("a"), audio("c"));
    let jump = Cue::new("jump", CueKind::GoTo { target: a.id });
    let mut rig = Rig::new(vec![a.clone(), jump, c]);
    rig.engine.go();
    rig.advance(1.0);
    rig.engine.go();
    assert_eq!(rig.engine.playhead_cue(), Some(a.id));
}

#[test]
fn an_arm_cue_changes_another_cues_armed_state() {
    let b = audio("b");
    let disarm = Cue::new(
        "disarm",
        CueKind::Arm {
            target: b.id,
            armed: false,
        },
    );
    let mut rig = Rig::new(vec![disarm, b.clone()]);
    rig.engine.go();
    rig.advance(1.0);
    rig.engine.go();
    assert!(rig.sink.calls().is_empty());
    assert_eq!(rig.state(&b), CueState::Done);
}

#[test]
fn a_together_group_runs_children_at_once_and_ends_when_all_are_done() {
    let (x, y) = (audio("x"), audio("y"));
    let g = group("g", GroupMode::Together, vec![x.clone(), y.clone()], false);
    let mut rig = Rig::new(vec![g.clone()]);

    rig.engine.go();
    assert_eq!(rig.state(&x), CueState::Running);
    assert_eq!(rig.state(&y), CueState::Running);
    rig.engine.audio_finished(x.id);
    assert_eq!(rig.state(&g), CueState::Running);
    rig.engine.audio_finished(y.id);
    assert_eq!(rig.state(&g), CueState::Done);
}

#[test]
fn a_sequence_group_runs_children_one_after_another() {
    let (x, y) = (audio("x"), audio("y"));
    let g = group("g", GroupMode::Sequence, vec![x.clone(), y.clone()], false);
    let mut rig = Rig::new(vec![g.clone()]);

    rig.engine.go();
    assert_eq!(rig.state(&x), CueState::Running);
    assert_eq!(rig.state(&y), CueState::Idle);
    rig.engine.audio_finished(x.id);
    assert_eq!(rig.state(&y), CueState::Running);
    rig.engine.audio_finished(y.id);
    assert_eq!(rig.state(&g), CueState::Done);
}

#[test]
fn a_shuffle_group_plays_every_child_exactly_once() {
    let kids: Vec<Cue> = (0..5).map(|i| audio(&format!("k{i}"))).collect();
    let g = group("g", GroupMode::Shuffle, kids.clone(), false);
    let mut rig = Rig::new(vec![g.clone()]);
    rig.engine.go();
    for _ in 0..kids.len() {
        let running: Vec<CueId> = kids
            .iter()
            .filter(|k| rig.state(k) == CueState::Running)
            .map(|k| k.id)
            .collect();
        assert_eq!(running.len(), 1);
        rig.engine.audio_finished(running[0]);
    }
    assert_eq!(rig.state(&g), CueState::Done);
    let started: Vec<_> = rig
        .sink
        .calls()
        .into_iter()
        .filter(|c| matches!(c, Call::Start(_)))
        .collect();
    assert_eq!(started.len(), 5);
}

#[test]
fn a_looped_group_of_instant_cues_does_not_spin() {
    let g = group("g", GroupMode::Sequence, vec![memo("m")], true);
    let mut rig = Rig::new(vec![g.clone()]);
    rig.engine.go();
    assert_eq!(rig.state(&g), CueState::Running);
    rig.advance(0.02);
    assert_eq!(rig.state(&g), CueState::Running);
}

#[test]
fn stopping_a_group_stops_its_children() {
    let x = audio("x");
    let g = group("g", GroupMode::Together, vec![x.clone()], false);
    let mut rig = Rig::new(vec![g.clone()]);
    rig.engine.go();
    rig.engine.stop(g.id);
    assert_eq!(rig.state(&g), CueState::Stopped);
    assert_eq!(rig.state(&x), CueState::Stopped);
}

#[test]
fn panic_fades_everything_and_a_second_press_stops_at_once() {
    let (a, b) = (audio("a"), audio("b"));
    let mut rig = Rig::new(vec![a.clone(), b.clone()]);
    rig.engine.go();
    rig.advance(1.0);
    rig.engine.go();

    rig.engine.panic();
    assert_eq!(rig.state(&a), CueState::Stopped);
    assert_eq!(rig.state(&b), CueState::Stopped);
    rig.advance(0.5);
    rig.engine.panic();

    let stops: Vec<_> = rig
        .sink
        .calls()
        .into_iter()
        .filter(|c| matches!(c, Call::StopAll(_)))
        .collect();
    assert_eq!(stops, [Call::StopAll(1.5), Call::StopAll(0.0)]);
}

#[test]
fn panic_cancels_pending_follows() {
    let mut a = memo("a");
    a.follow = Follow::Continue;
    a.post_wait = 5.0;
    let b = audio("b");
    let mut rig = Rig::new(vec![a, b.clone()]);
    rig.engine.go();
    rig.engine.panic();
    rig.advance(10.0);
    assert_eq!(rig.state(&b), CueState::Idle);
}

#[test]
fn reset_returns_everything_to_idle_and_the_playhead_to_the_top() {
    let (a, b) = (audio("a"), audio("b"));
    let mut rig = Rig::new(vec![a.clone(), b.clone()]);
    rig.engine.go();
    rig.advance(1.0);
    rig.engine.go();

    rig.engine.reset();

    assert_eq!(rig.state(&a), CueState::Idle);
    assert_eq!(rig.state(&b), CueState::Idle);
    assert_eq!(rig.engine.playhead_cue(), Some(a.id));
    rig.engine.go();
    assert_eq!(rig.state(&a), CueState::Running);
}

#[test]
fn skip_and_set_playhead_move_without_firing() {
    let (a, b, c) = (audio("a"), audio("b"), audio("c"));
    let mut rig = Rig::new(vec![a.clone(), b.clone(), c.clone()]);
    rig.engine.skip();
    assert_eq!(rig.engine.playhead_cue(), Some(b.id));
    assert!(rig.engine.set_playhead(c.id));
    assert_eq!(rig.engine.playhead_cue(), Some(c.id));
    assert!(!rig.engine.set_playhead(CueId::new()));
    assert!(rig.sink.calls().is_empty());
}
