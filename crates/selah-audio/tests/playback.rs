//! The cue engine driving the audio engine end to end, with real files and the
//! offline backend (no audio device).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use selah_audio::{AudioEngine, AudioEvent, OfflineBackend};
use selah_core::{Cue, CueKind, FadeCurve, Item, ItemKind, PatchId, Show};
use selah_engine::testing::FakeClock;
use selah_engine::{CueState, Engine, EngineConfig, Event, Warning};

const RATE: u32 = 48_000;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("selah-playback-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A stereo 440 Hz tone at half scale.
fn write_tone(dir: &Path, name: &str, seconds: f64) {
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(dir.join(name), spec).unwrap();
    for i in 0..(f64::from(RATE) * seconds) as usize {
        let t = i as f64 / f64::from(RATE);
        let s = ((t * 440.0 * std::f64::consts::TAU).sin() * 16_000.0) as i16;
        writer.write_sample(s).unwrap();
        writer.write_sample(s).unwrap();
    }
    writer.finalize().unwrap();
}

fn audio_cue(file: &str, patches: Vec<PatchId>, loops: Option<u32>) -> Cue {
    Cue::new(
        file,
        CueKind::Audio {
            file: file.into(),
            level: 0.0,
            start: 0.0,
            end: None,
            loops,
            fade_in: 0.0,
            fade_out: 0.0,
            curve: FadeCurve::EqualPower,
            patches,
        },
    )
}

struct Rig {
    engine: Engine<FakeClock, AudioEngine>,
    clock: FakeClock,
    backend: OfflineBackend,
    channels: usize,
}

impl Rig {
    fn new(
        dir: &Path,
        cues: Vec<Cue>,
        channels: usize,
        patch_map: HashMap<PatchId, Vec<u16>>,
    ) -> Self {
        let (mut audio, mixer) = AudioEngine::new(RATE, channels);
        audio.set_media_root(dir);
        audio.set_patch_map(patch_map);
        let clock = FakeClock::default();
        let mut engine = Engine::new(clock.clone(), audio, EngineConfig::default());
        let mut show = Show::new("test");
        let mut item = Item::new("item", ItemKind::General);
        item.cues = cues;
        show.items.push(item);
        engine.load(&show);
        Self {
            engine,
            clock,
            backend: OfflineBackend::new(mixer, channels),
            channels,
        }
    }

    /// Renders `frames` and forwards what the audio side reports, as the host
    /// would. Returns the samples.
    fn pump(&mut self, frames: usize) -> Vec<f32> {
        let out = self.backend.render(frames);
        self.clock.advance(frames as f64 / f64::from(RATE));
        self.engine.tick();
        out
    }

    /// What the host does each cycle: pass the audio side's reports to the
    /// cue engine.
    fn forward_audio_events(&mut self) {
        for event in self.engine.audio_mut().poll() {
            if let AudioEvent::Finished(id) = event {
                self.engine.audio_finished(id);
            }
        }
    }

    fn peak(samples: &[f32], channels: usize, channel: usize) -> f32 {
        samples
            .iter()
            .skip(channel)
            .step_by(channels)
            .fold(0.0, |m, s| m.max(s.abs()))
    }
}

/// Keeps rendering until `done` is true, or fails after a few real seconds.
fn pump_until(rig: &mut Rig, mut done: impl FnMut(&mut Rig, &[f32]) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        let out = rig.pump(480);
        if done(rig, &out) {
            return;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!("timed out waiting for the audio engine");
}

#[test]
fn a_cue_plays_through_the_device_and_finishes() {
    let dir = temp_dir("play");
    write_tone(&dir, "tone.wav", 0.4);
    let cue = audio_cue("tone.wav", vec![], None);
    let mut rig = Rig::new(&dir, vec![cue.clone()], 2, HashMap::new());

    rig.engine.go();
    let mut heard = false;
    pump_until(&mut rig, |rig, out| {
        heard |= Rig::peak(out, 2, 0) > 0.4;
        rig.forward_audio_events();
        rig.engine.state(cue.id) == CueState::Done
    });

    assert!(heard, "the tone should have reached the output");
    assert_eq!(rig.engine.state(cue.id), CueState::Done);
}

#[test]
fn a_missing_file_warns_and_the_show_carries_on() {
    let dir = temp_dir("missing");
    let (a, b) = (
        audio_cue("gone.wav", vec![], None),
        audio_cue("also-gone.wav", vec![], None),
    );
    let mut rig = Rig::new(&dir, vec![a.clone(), b.clone()], 2, HashMap::new());

    rig.engine.go();

    assert_eq!(rig.engine.state(a.id), CueState::Done);
    assert_eq!(rig.engine.playhead_cue(), Some(b.id));
    assert!(rig.engine.take_events().iter().any(|e| matches!(
        e,
        Event::Warning { cue, warning: Warning::MissingMedia { file } } if *cue == a.id && file == "gone.wav"
    )));
}

#[test]
fn patches_route_a_cue_to_the_right_device_channels() {
    let dir = temp_dir("route");
    write_tone(&dir, "tone.wav", 0.4);
    let patch = PatchId::new();
    let cue = audio_cue("tone.wav", vec![patch], None);
    let mut map = HashMap::new();
    map.insert(patch, vec![2, 3]);
    let mut rig = Rig::new(&dir, vec![cue], 4, map);

    rig.engine.go();
    let (mut low, mut high) = (0.0f32, 0.0f32);
    pump_until(&mut rig, |rig, out| {
        low = low
            .max(Rig::peak(out, rig.channels, 0))
            .max(Rig::peak(out, rig.channels, 1));
        high = high
            .max(Rig::peak(out, rig.channels, 2))
            .max(Rig::peak(out, rig.channels, 3));
        high > 0.4
    });

    assert!(high > 0.4);
    assert_eq!(low, 0.0, "nothing leaks to channels the patch does not use");
}

#[test]
fn panic_fades_to_silence_over_the_panic_time() {
    let dir = temp_dir("panic");
    write_tone(&dir, "tone.wav", 3.0);
    let cue = audio_cue("tone.wav", vec![], Some(0));
    let mut rig = Rig::new(&dir, vec![cue.clone()], 2, HashMap::new());
    rig.engine.go();
    pump_until(&mut rig, |_, out| Rig::peak(out, 2, 0) > 0.4);

    rig.engine.panic();
    let first = rig.pump(2400);
    assert!(Rig::peak(&first, 2, 0) > 0.1, "it fades, it does not cut");
    // 1.5 s is 72,000 frames; render comfortably past it.
    let mut tail = Vec::new();
    for _ in 0..40 {
        tail = rig.pump(2400);
    }
    assert_eq!(Rig::peak(&tail, 2, 0), 0.0);
    assert_eq!(rig.engine.state(cue.id), CueState::Stopped);
}

#[test]
fn a_looping_cue_keeps_playing_past_the_length_of_its_file() {
    let dir = temp_dir("loop");
    write_tone(&dir, "short.wav", 0.1);
    let cue = audio_cue("short.wav", vec![], Some(0));
    let mut rig = Rig::new(&dir, vec![cue.clone()], 2, HashMap::new());
    rig.engine.go();

    let mut frames_heard = 0usize;
    pump_until(&mut rig, |_, out| {
        frames_heard += out.chunks(2).filter(|f| f[0].abs() > 0.01).count();
        frames_heard > 3 * (RATE as usize / 10)
    });

    assert_eq!(rig.engine.state(cue.id), CueState::Running);
}

#[test]
fn restarting_a_cue_does_not_double_it() {
    let dir = temp_dir("restart");
    write_tone(&dir, "tone.wav", 1.0);
    let cue = audio_cue("tone.wav", vec![], None);
    let mut rig = Rig::new(&dir, vec![cue.clone()], 2, HashMap::new());
    rig.engine.go();
    pump_until(&mut rig, |_, out| Rig::peak(out, 2, 0) > 0.4);
    // The engine will not fire an active cue twice; peaks stay at one voice.
    rig.clock.advance(1.0);
    rig.engine.reset();
    rig.engine.go();
    let mut max = 0.0f32;
    pump_until(&mut rig, |_, out| {
        max = max.max(Rig::peak(out, 2, 0));
        max > 0.4
    });
    assert!(max < 0.6, "one tone at half scale, not two summed: {max}");
}
