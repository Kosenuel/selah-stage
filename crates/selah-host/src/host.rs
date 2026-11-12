use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use selah_audio::{AudioEngine, AudioEvent, OfflineBackend, OutputDevice, RunningDevice};
use selah_core::{CueId, Show};
use selah_engine::{CueState, Engine, EngineConfig, Event, Warning};
use selah_persist::Autosaver;

/// Where sound goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output {
    /// The system's default output. If none can be opened the host runs
    /// silently and says why in [`DeviceStatus::error`].
    Default,
    /// No device: the clock runs at real speed and nothing is heard. For tests
    /// and machines without audio.
    Silent { channels: usize },
}

/// What the UI and other threads can ask of the host.
#[derive(Debug)]
pub enum Command {
    /// A show that has not been saved anywhere yet, with its media folder.
    Load {
        show: Box<Show>,
        media_root: PathBuf,
    },
    /// A show read from its folder. Edits to it are autosaved there.
    Open {
        show: Box<Show>,
        folder: PathBuf,
        recovered_from: Option<PathBuf>,
    },
    /// Writes the show to its folder (or its media folder, if never saved) and
    /// turns on autosave.
    SaveShow,
    SetArmed {
        cue: CueId,
        armed: bool,
    },
    Go,
    Skip,
    Panic,
    Reset,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HostError {
    #[error("The audio engine could not start.")]
    Startup,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DeviceStatus {
    pub name: String,
    pub sample_rate: u32,
    pub channels: usize,
    /// True when sound really goes to a device.
    pub audible: bool,
    /// The device went away while running.
    pub lost: bool,
    /// Why the host is not using a real device, in words for an operator.
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotCue {
    pub id: String,
    pub number: String,
    pub name: String,
    pub state: CueState,
    pub armed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotWarning {
    pub cue: String,
    pub message: String,
}

/// Everything the UI shows, as of the last loop.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Snapshot {
    pub title: String,
    pub cues: Vec<SnapshotCue>,
    pub playhead: Option<String>,
    pub warnings: Vec<SnapshotWarning>,
    /// Peak level per device channel since the previous snapshot.
    pub peaks: Vec<f32>,
    pub underruns: u32,
    pub device: DeviceStatus,
    /// The show's folder once it is saved; `None` while it exists only in memory.
    pub location: Option<String>,
    /// Why the last save or autosave failed, in words for an operator.
    pub save_problem: Option<String>,
    /// Something the operator should know about how the show was opened.
    pub notice: Option<String>,
}

/// A handle to the running host. Dropping it stops the thread.
pub struct Host {
    commands: Sender<Command>,
    snapshot: Arc<Mutex<Snapshot>>,
    thread: Option<JoinHandle<()>>,
}

const LOOP_INTERVAL: Duration = Duration::from_millis(5);
const SNAPSHOT_INTERVAL: Duration = Duration::from_millis(40);
const MAX_WARNINGS: usize = 20;
/// Autosave waits for this much quiet after a change, so a burst of edits is
/// one write.
const AUTOSAVE_QUIET: Duration = Duration::from_millis(500);

/// What the loop knows about where the show lives.
#[derive(Default)]
struct Storage {
    media_root: Option<PathBuf>,
    location: Option<PathBuf>,
    autosaver: Option<Autosaver>,
    save_problem: Option<String>,
    notice: Option<String>,
}

impl Host {
    pub fn spawn(output: Output) -> Result<Self, HostError> {
        let (commands, receiver) = mpsc::channel();
        let snapshot = Arc::new(Mutex::new(Snapshot::default()));
        let shared = snapshot.clone();
        let (ready_tx, ready_rx) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("selah-host".into())
            .spawn(move || run(output, &receiver, &shared, &ready_tx))
            .map_err(|_| HostError::Startup)?;
        ready_rx
            .recv_timeout(Duration::from_secs(10))
            .map_err(|_| HostError::Startup)?;
        Ok(Self {
            commands,
            snapshot,
            thread: Some(thread),
        })
    }

    /// Never blocks; returns `false` only if the host has stopped.
    pub fn send(&self, command: Command) -> bool {
        self.commands.send(command).is_ok()
    }

    pub fn snapshot(&self) -> Snapshot {
        self.snapshot
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        // Closing the channel ends the loop.
        let (closed, _) = mpsc::channel();
        self.commands = closed;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

enum Sound {
    Device(RunningDevice),
    Silent(OfflineBackend, Instant, f64),
}

fn run(
    output: Output,
    receiver: &mpsc::Receiver<Command>,
    shared: &Mutex<Snapshot>,
    ready: &Sender<()>,
) {
    let (sample_rate, channels, mut status, device) = open_output(output);
    let (audio, mixer) = AudioEngine::new(sample_rate, channels);
    let clock = audio.clock();
    let mut sound = match device {
        Some(device) => match device.start(mixer) {
            Ok(running) => Sound::Device(running),
            Err(error) => {
                // Starting consumed the mixer, so fall back to a fresh engine.
                tracing::warn!(%error, "audio device failed to start");
                status.error = Some(error.to_string());
                status.audible = false;
                return run_silent(sample_rate, channels, status, receiver, shared, ready);
            }
        },
        None => Sound::Silent(OfflineBackend::new(mixer, channels), Instant::now(), 0.0),
    };
    let mut engine = Engine::new(clock, audio, EngineConfig::default());
    let _ = ready.send(());
    drive(&mut engine, &mut sound, status, receiver, shared);
}

/// Used when a real device was found but refused to start.
fn run_silent(
    sample_rate: u32,
    channels: usize,
    status: DeviceStatus,
    receiver: &mpsc::Receiver<Command>,
    shared: &Mutex<Snapshot>,
    ready: &Sender<()>,
) {
    let (audio, mixer) = AudioEngine::new(sample_rate, channels);
    let clock = audio.clock();
    let mut sound = Sound::Silent(OfflineBackend::new(mixer, channels), Instant::now(), 0.0);
    let mut engine = Engine::new(clock, audio, EngineConfig::default());
    let _ = ready.send(());
    drive(&mut engine, &mut sound, status, receiver, shared);
}

fn open_output(output: Output) -> (u32, usize, DeviceStatus, Option<OutputDevice>) {
    match output {
        Output::Silent { channels } => (
            48_000,
            channels,
            DeviceStatus {
                name: "Silent".into(),
                sample_rate: 48_000,
                channels,
                ..DeviceStatus::default()
            },
            None,
        ),
        Output::Default => match OutputDevice::default_output(None) {
            Ok(device) => {
                let status = DeviceStatus {
                    name: device.name().to_owned(),
                    sample_rate: device.sample_rate(),
                    channels: device.channels(),
                    audible: true,
                    ..DeviceStatus::default()
                };
                (
                    device.sample_rate(),
                    device.channels(),
                    status,
                    Some(device),
                )
            }
            Err(error) => (
                48_000,
                2,
                DeviceStatus {
                    name: "No sound output".into(),
                    sample_rate: 48_000,
                    channels: 2,
                    error: Some(error.to_string()),
                    ..DeviceStatus::default()
                },
                None,
            ),
        },
    }
}

fn drive(
    engine: &mut Engine<selah_audio::AudioClock, AudioEngine>,
    sound: &mut Sound,
    mut status: DeviceStatus,
    receiver: &mpsc::Receiver<Command>,
    shared: &Mutex<Snapshot>,
) {
    let mut show: Option<Show> = None;
    let mut storage = Storage::default();
    let mut warnings: Vec<SnapshotWarning> = Vec::new();
    let mut last_snapshot = Instant::now() - SNAPSHOT_INTERVAL;
    loop {
        let mut commands = Vec::new();
        match receiver.recv_timeout(LOOP_INTERVAL) {
            Ok(command) => commands.push(command),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        while let Ok(command) = receiver.try_recv() {
            commands.push(command);
        }

        if let Sound::Silent(backend, started, rendered) = sound {
            // Play the clock forward at real speed, as a device would.
            let due = started.elapsed().as_secs_f64() * f64::from(engine.audio_mut().sample_rate());
            let frames = (due - *rendered).max(0.0) as usize;
            if frames > 0 {
                backend.render(frames);
                *rendered += frames as f64;
            }
        }

        for command in commands {
            match command {
                Command::Load {
                    show: loaded,
                    media_root,
                } => {
                    engine.audio_mut().set_media_root(media_root.clone());
                    engine.load(&loaded);
                    warnings.clear();
                    storage = Storage {
                        media_root: Some(media_root),
                        ..Storage::default()
                    };
                    show = Some(*loaded);
                }
                Command::Open {
                    show: opened,
                    folder,
                    recovered_from,
                } => {
                    engine.audio_mut().set_media_root(folder.clone());
                    engine.load(&opened);
                    warnings.clear();
                    for missing in selah_persist::missing_media(&opened, &folder) {
                        warnings.push(SnapshotWarning {
                            cue: cue_name(Some(&opened), missing.cue),
                            message: describe(&Warning::MissingMedia { file: missing.file }),
                        });
                    }
                    storage = Storage {
                        media_root: Some(folder.clone()),
                        autosaver: Some(Autosaver::start(folder.clone(), AUTOSAVE_QUIET)),
                        location: Some(folder),
                        notice: recovered_from.map(|_| {
                            "The show file was damaged, so the latest backup was opened. Check your recent changes.".to_owned()
                        }),
                        ..Storage::default()
                    };
                    show = Some(*opened);
                }
                Command::SaveShow => save_now(show.as_ref(), &mut storage),
                Command::SetArmed { cue, armed } => {
                    if let Some(current) = show.as_mut()
                        && let Some(target) = current.cue_mut(cue)
                    {
                        target.armed = armed;
                        engine.set_armed(cue, armed);
                        if let Some(saver) = &storage.autosaver {
                            saver.submit(current.clone());
                        }
                    }
                }
                Command::Go => engine.go(),
                Command::Skip => engine.skip(),
                Command::Panic => engine.panic(),
                Command::Reset => engine.reset(),
            }
        }

        for event in engine.audio_mut().poll() {
            if let AudioEvent::Finished(cue) = event {
                engine.audio_finished(cue);
            }
        }
        engine.tick();

        for event in engine.take_events() {
            if let Event::Warning { cue, warning } = event {
                warnings.push(SnapshotWarning {
                    cue: cue_name(show.as_ref(), cue),
                    message: describe(&warning),
                });
                if warnings.len() > MAX_WARNINGS {
                    warnings.remove(0);
                }
            }
        }

        if let Sound::Device(running) = sound
            && running.is_lost()
            && !status.lost
        {
            status.lost = true;
            status.error =
                Some("The audio device was disconnected. Reconnect it and restart.".into());
        }

        if last_snapshot.elapsed() >= SNAPSHOT_INTERVAL {
            last_snapshot = Instant::now();
            let snapshot = build_snapshot(engine, show.as_ref(), &warnings, &status, &storage);
            *shared.lock().unwrap_or_else(PoisonError::into_inner) = snapshot;
        }
    }
}

/// Saves now, to the show's folder or, for a show never saved, its media
/// folder. A first save turns autosave on.
fn save_now(show: Option<&Show>, storage: &mut Storage) {
    let (Some(show), Some(folder)) = (
        show,
        storage.location.clone().or(storage.media_root.clone()),
    ) else {
        return;
    };
    match selah_persist::save(&folder, show) {
        Ok(()) => {
            storage.save_problem = None;
            if storage.autosaver.is_none() {
                storage.autosaver = Some(Autosaver::start(folder.clone(), AUTOSAVE_QUIET));
            }
            storage.location = Some(folder);
        }
        Err(error) => storage.save_problem = Some(error.to_string()),
    }
}

fn cue_name(show: Option<&Show>, id: CueId) -> String {
    show.and_then(|s| {
        s.all_cues()
            .into_iter()
            .find(|c| c.id == id)
            .map(|c| c.name.clone())
    })
    .unwrap_or_default()
}

fn describe(warning: &Warning) -> String {
    match warning {
        Warning::MissingMedia { file } => {
            format!("Can't find {file}. Relink it or skip.")
        }
        Warning::AudioFailed { reason } => format!("Couldn't play it: {reason}"),
        Warning::BadGoTo => "A go-to cue points at a cue that can't be reached.".into(),
    }
}

fn build_snapshot(
    engine: &mut Engine<selah_audio::AudioClock, AudioEngine>,
    show: Option<&Show>,
    warnings: &[SnapshotWarning],
    status: &DeviceStatus,
    storage: &Storage,
) -> Snapshot {
    let cues = show
        .map(|s| {
            s.items
                .iter()
                .flat_map(|item| item.cues.iter())
                .map(|c| SnapshotCue {
                    id: c.id.as_uuid().to_string(),
                    number: c.number.clone(),
                    name: c.name.clone(),
                    state: engine.state(c.id),
                    armed: c.armed,
                })
                .collect()
        })
        .unwrap_or_default();
    Snapshot {
        title: show.map(|s| s.meta.title.clone()).unwrap_or_default(),
        cues,
        playhead: engine.playhead_cue().map(|c| c.as_uuid().to_string()),
        warnings: warnings.to_vec(),
        peaks: engine.audio_mut().peaks(),
        underruns: engine.audio_mut().meters().underruns(),
        device: status.clone(),
        location: storage.location.as_ref().map(|p| p.display().to_string()),
        save_problem: storage.save_problem.clone().or_else(|| {
            storage
                .autosaver
                .as_ref()
                .and_then(Autosaver::last_error)
                .map(|e| e.to_string())
        }),
        notice: storage.notice.clone(),
    }
}
