use std::collections::HashMap;

use selah_core::{Cue, CueId, CueKind, Follow, GroupMode, Seconds, Show};

use crate::sink::{AudioPlay, AudioSink, Clock, SinkError};

/// Where a cue is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CueState {
    Idle,
    PreWait,
    Running,
    Paused,
    Done,
    Stopped,
}

impl CueState {
    fn is_active(self) -> bool {
        matches!(self, Self::PreWait | Self::Running | Self::Paused)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Warning {
    /// A cue was skipped because its media is missing.
    MissingMedia { file: String },
    /// A cue was skipped because the audio side failed to start it.
    AudioFailed { reason: String },
    /// A go-to cue pointed somewhere the playhead cannot go.
    BadGoTo,
}

/// What the UI needs to hear about. Drain with [`Engine::take_events`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    State { cue: CueId, state: CueState },
    Playhead(Option<CueId>),
    Warning { cue: CueId, warning: Warning },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineConfig {
    /// A second GO inside this window is ignored (CUE-7).
    pub go_debounce: Seconds,
    /// How long panic takes to fade everything out (CUE-6).
    pub panic_fade: Seconds,
    /// The short ramp used when a cue is stopped, so nothing clicks (AUD-3).
    pub stop_fade: Seconds,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            go_debounce: 0.3,
            panic_fade: 1.5,
            stop_fade: 0.05,
        }
    }
}

struct Node {
    cue: Cue,
    parent: Option<CueId>,
    /// The next top-level cue, used by follow modes.
    next: Option<CueId>,
}

struct Runtime {
    state: CueState,
    /// When a pre-wait ends or a timed cue (a fade) completes.
    deadline: Option<Seconds>,
    paused_from: CueState,
    paused_left: Option<Seconds>,
    /// A group's children in firing order, and how far a sequence has got.
    queue: Vec<CueId>,
    pos: usize,
}

impl Runtime {
    fn new(state: CueState) -> Self {
        Self {
            state,
            deadline: None,
            paused_from: state,
            paused_left: None,
            queue: Vec::new(),
            pos: 0,
        }
    }
}

enum Pending {
    /// Start `target` because `from` started or finished.
    Fire { from: CueId, target: CueId },
    /// Run a looping group again.
    Restart(CueId),
}

pub struct Engine<C: Clock, A: AudioSink> {
    clock: C,
    audio: A,
    config: EngineConfig,
    nodes: HashMap<CueId, Node>,
    /// Top-level cues in plan order.
    order: Vec<CueId>,
    rt: HashMap<CueId, Runtime>,
    playhead: Option<usize>,
    last_go: Option<Seconds>,
    panic_until: Option<Seconds>,
    pending: Vec<(Seconds, Pending)>,
    events: Vec<Event>,
    rng: u64,
}

/// Stops a runaway chain of instant follows (for example a go-to loop).
const MAX_CHAIN: usize = 10_000;

impl<C: Clock, A: AudioSink> Engine<C, A> {
    pub fn new(clock: C, audio: A, config: EngineConfig) -> Self {
        Self {
            clock,
            audio,
            config,
            nodes: HashMap::new(),
            order: Vec::new(),
            rt: HashMap::new(),
            playhead: None,
            last_go: None,
            panic_until: None,
            pending: Vec::new(),
            events: Vec::new(),
            rng: 0x2545_F491_4F6C_DD1D,
        }
    }

    /// Replaces the loaded show, stopping everything that was running.
    pub fn load(&mut self, show: &Show) {
        self.audio.stop_all(0.0);
        self.nodes.clear();
        self.order.clear();
        self.rt.clear();
        self.pending.clear();
        let top: Vec<&Cue> = show
            .items
            .iter()
            .flat_map(|item| item.cues.iter())
            .collect();
        for (i, cue) in top.iter().enumerate() {
            self.order.push(cue.id);
            let next = top.get(i + 1).map(|c| c.id);
            self.index(cue, None, next);
        }
        self.playhead = if self.order.is_empty() { None } else { Some(0) };
        self.events.push(Event::Playhead(self.playhead_cue()));
    }

    fn index(&mut self, cue: &Cue, parent: Option<CueId>, next: Option<CueId>) {
        self.nodes.insert(
            cue.id,
            Node {
                cue: cue.clone(),
                parent,
                next,
            },
        );
        if let CueKind::Group { children, .. } = &cue.kind {
            for child in children {
                self.index(child, Some(cue.id), None);
            }
        }
    }

    // ---- What the UI reads --------------------------------------------

    pub fn playhead_cue(&self) -> Option<CueId> {
        self.playhead.and_then(|i| self.order.get(i).copied())
    }

    pub fn state(&self, cue: CueId) -> CueState {
        self.rt.get(&cue).map_or(CueState::Idle, |r| r.state)
    }

    /// Cues that are waiting, running, or paused, in no particular order.
    pub fn active(&self) -> Vec<CueId> {
        self.rt
            .iter()
            .filter(|(_, r)| r.state.is_active())
            .map(|(id, _)| *id)
            .collect()
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    // ---- Operator actions ---------------------------------------------

    /// GO: fires the playhead cue and moves the playhead on. A second GO inside
    /// the debounce window is ignored.
    pub fn go(&mut self) {
        let now = self.clock.now();
        if self
            .last_go
            .is_some_and(|t| now - t < self.config.go_debounce)
        {
            return;
        }
        self.last_go = Some(now);
        let Some(index) = self.playhead else { return };
        let Some(&id) = self.order.get(index) else {
            return;
        };
        // Move first, so a go-to cue can override where the playhead lands.
        self.set_playhead_index(if index + 1 < self.order.len() {
            Some(index + 1)
        } else {
            None
        });
        self.fire(id);
        self.drain();
    }

    /// Moves the playhead without firing anything.
    pub fn set_playhead(&mut self, cue: CueId) -> bool {
        match self.order.iter().position(|&c| c == cue) {
            Some(i) => {
                self.set_playhead_index(Some(i));
                true
            }
            None => false,
        }
    }

    /// Skips the playhead cue without firing it.
    pub fn skip(&mut self) {
        if let Some(i) = self.playhead {
            self.set_playhead_index((i + 1 < self.order.len()).then_some(i + 1));
        }
    }

    pub fn set_armed(&mut self, cue: CueId, armed: bool) {
        if let Some(node) = self.nodes.get_mut(&cue) {
            node.cue.armed = armed;
        }
    }

    pub fn pause(&mut self, cue: CueId) {
        let now = self.clock.now();
        let Some(node) = self.nodes.get(&cue) else {
            return;
        };
        let children = child_ids(&node.cue);
        let is_audio = matches!(node.cue.kind, CueKind::Audio { .. });
        if let Some(r) = self
            .rt
            .get_mut(&cue)
            .filter(|r| matches!(r.state, CueState::PreWait | CueState::Running))
        {
            r.paused_from = r.state;
            r.paused_left = r.deadline.map(|d| (d - now).max(0.0));
            r.state = CueState::Paused;
            if is_audio && r.paused_from == CueState::Running {
                self.audio.pause(cue);
            }
            self.events.push(Event::State {
                cue,
                state: CueState::Paused,
            });
        }
        for child in children {
            self.pause(child);
        }
    }

    pub fn resume(&mut self, cue: CueId) {
        let now = self.clock.now();
        let Some(node) = self.nodes.get(&cue) else {
            return;
        };
        let children = child_ids(&node.cue);
        let is_audio = matches!(node.cue.kind, CueKind::Audio { .. });
        if let Some(r) = self
            .rt
            .get_mut(&cue)
            .filter(|r| r.state == CueState::Paused)
        {
            r.state = r.paused_from;
            r.deadline = r.paused_left.take().map(|left| now + left);
            if is_audio && r.state == CueState::Running {
                self.audio.resume(cue);
            }
            self.events.push(Event::State {
                cue,
                state: r.state,
            });
        }
        for child in children {
            self.resume(child);
        }
    }

    /// Stops a cue (and anything inside it) with a short ramp.
    pub fn stop(&mut self, cue: CueId) {
        self.stop_with(cue, self.config.stop_fade);
    }

    /// Panic: fade everything out and stop it. Pressed again while the fade
    /// is running, it stops everything at once (CUE-6).
    pub fn panic(&mut self) {
        let now = self.clock.now();
        let fading = self.panic_until.is_some_and(|until| now < until);
        let fade = if fading { 0.0 } else { self.config.panic_fade };
        self.audio.stop_all(fade);
        self.panic_until = (!fading).then_some(now + fade);
        let active = self.active();
        for id in active {
            self.set_state(id, CueState::Stopped);
        }
        self.pending.clear();
    }

    /// Returns every cue to idle and the playhead to the top, ready for the
    /// next service of the day (PLAN-5).
    pub fn reset(&mut self) {
        self.audio.stop_all(0.0);
        self.rt.clear();
        self.pending.clear();
        self.panic_until = None;
        self.last_go = None;
        self.set_playhead_index(if self.order.is_empty() { None } else { Some(0) });
    }

    /// The audio side reports that a cue ran to its end.
    pub fn audio_finished(&mut self, cue: CueId) {
        if self.state(cue) == CueState::Running {
            self.finish(cue);
            self.drain();
        }
    }

    /// Advances time: ends waits, completes fades, and fires due follows.
    pub fn tick(&mut self) {
        let now = self.clock.now();
        let mut due: Vec<(Seconds, CueId)> = self
            .rt
            .iter()
            .filter(|(_, r)| matches!(r.state, CueState::PreWait | CueState::Running))
            .filter_map(|(id, r)| r.deadline.filter(|d| *d <= now).map(|d| (d, *id)))
            .collect();
        due.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        for (_, id) in due {
            match self.state(id) {
                CueState::PreWait => self.begin(id),
                CueState::Running => self.complete_timed(id),
                _ => {}
            }
        }
        self.drain();
    }

    // ---- Internals -----------------------------------------------------

    fn set_playhead_index(&mut self, index: Option<usize>) {
        if self.playhead != index {
            self.playhead = index;
            self.events.push(Event::Playhead(self.playhead_cue()));
        }
    }

    fn set_state(&mut self, id: CueId, state: CueState) {
        let entry = self.rt.entry(id).or_insert_with(|| Runtime::new(state));
        if entry.state != state {
            entry.state = state;
            self.events.push(Event::State { cue: id, state });
        }
    }

    fn warn(&mut self, cue: CueId, warning: Warning) {
        self.events.push(Event::Warning { cue, warning });
    }

    /// Starts a cue: its pre-wait first, then its action.
    fn fire(&mut self, id: CueId) {
        let now = self.clock.now();
        let Some(node) = self.nodes.get(&id) else {
            return;
        };
        let (armed, pre_wait) = (node.cue.armed, node.cue.pre_wait);
        self.rt.insert(id, Runtime::new(CueState::Idle));
        if !armed {
            // A disarmed cue does nothing, but the show carries on.
            self.set_state(id, CueState::Done);
            return;
        }
        if pre_wait > 0.0 {
            self.set_state(id, CueState::PreWait);
            if let Some(r) = self.rt.get_mut(&id) {
                r.deadline = Some(now + pre_wait);
            }
        } else {
            self.begin(id);
        }
    }

    /// Runs a cue's action once its pre-wait is over.
    fn begin(&mut self, id: CueId) {
        let now = self.clock.now();
        let Some(node) = self.nodes.get(&id) else {
            return;
        };
        let cue = node.cue.clone();
        self.set_state(id, CueState::Running);
        if let Some(r) = self.rt.get_mut(&id) {
            r.deadline = None;
        }
        match &cue.kind {
            CueKind::Audio {
                file,
                level,
                start,
                end,
                loops,
                fade_in,
                fade_out,
                curve,
                patches,
            } => {
                let play = AudioPlay {
                    file: file.clone(),
                    level: *level,
                    start: *start,
                    end: *end,
                    loops: *loops,
                    fade_in: *fade_in,
                    fade_out: *fade_out,
                    curve: *curve,
                    patches: patches.clone(),
                };
                if let Err(error) = self.audio.start(id, &play) {
                    // Never block: warn, skip, and let the show carry on.
                    let warning = match error {
                        SinkError::MissingMedia(file) => Warning::MissingMedia { file },
                        SinkError::Failed(reason) => Warning::AudioFailed { reason },
                    };
                    self.warn(id, warning);
                    self.finish(id);
                }
            }
            CueKind::Fade {
                target,
                to,
                duration,
                curve,
                ..
            } => {
                self.audio.fade(*target, *to, *duration, *curve);
                if *duration > 0.0 {
                    if let Some(r) = self.rt.get_mut(&id) {
                        r.deadline = Some(now + duration);
                    }
                } else {
                    self.complete_timed(id);
                }
            }
            CueKind::Stop { target } => {
                self.stop(*target);
                self.finish(id);
            }
            CueKind::Pause { target } => {
                self.pause(*target);
                self.finish(id);
            }
            CueKind::GoTo { target } => {
                if !self.set_playhead(*target) {
                    self.warn(id, Warning::BadGoTo);
                }
                self.finish(id);
            }
            CueKind::Arm { target, armed } => {
                self.set_armed(*target, *armed);
                self.finish(id);
            }
            CueKind::Group { .. } => self.start_group(id),
            CueKind::Memo => self.finish(id),
        }
        if cue.follow == Follow::Continue {
            self.schedule_next(id, now + cue.post_wait);
        }
    }

    fn complete_timed(&mut self, id: CueId) {
        let stop = match self.nodes.get(&id).map(|n| &n.cue.kind) {
            Some(CueKind::Fade {
                target,
                stop_when_done: true,
                ..
            }) => Some(*target),
            _ => None,
        };
        if let Some(target) = stop {
            self.stop(target);
        }
        self.finish(id);
    }

    /// A cue ran to completion on its own.
    fn finish(&mut self, id: CueId) {
        let now = self.clock.now();
        self.set_state(id, CueState::Done);
        let Some(node) = self.nodes.get(&id) else {
            return;
        };
        let (parent, follow, post_wait) = (node.parent, node.cue.follow, node.cue.post_wait);
        match parent {
            Some(group) => self.group_child_finished(group, id),
            None if follow == Follow::Follow => self.schedule_next(id, now + post_wait),
            None => {}
        }
    }

    fn schedule_next(&mut self, from: CueId, due: Seconds) {
        if let Some(target) = self.nodes.get(&from).and_then(|n| n.next) {
            self.pending.push((due, Pending::Fire { from, target }));
        }
    }

    fn start_group(&mut self, id: CueId) {
        let Some(CueKind::Group { mode, children, .. }) =
            self.nodes.get(&id).map(|n| n.cue.kind.clone())
        else {
            return;
        };
        let mut queue: Vec<CueId> = children.iter().map(|c| c.id).collect();
        for child in &queue {
            self.reset_subtree(*child);
        }
        if mode == GroupMode::Shuffle {
            self.shuffle(&mut queue);
        }
        if queue.is_empty() {
            self.finish(id);
            return;
        }
        if let Some(r) = self.rt.get_mut(&id) {
            r.queue.clone_from(&queue);
            r.pos = 0;
        }
        match mode {
            GroupMode::Together => {
                for child in queue {
                    self.fire(child);
                }
            }
            GroupMode::Sequence | GroupMode::Shuffle => self.fire(queue[0]),
        }
    }

    fn reset_subtree(&mut self, id: CueId) {
        self.rt.remove(&id);
        if let Some(node) = self.nodes.get(&id) {
            for child in child_ids(&node.cue) {
                self.reset_subtree(child);
            }
        }
    }

    fn group_child_finished(&mut self, group: CueId, _child: CueId) {
        let Some(mode) = self.nodes.get(&group).and_then(|n| match &n.cue.kind {
            CueKind::Group { mode, .. } => Some(*mode),
            _ => None,
        }) else {
            return;
        };
        let Some(r) = self.rt.get_mut(&group) else {
            return;
        };
        if r.state != CueState::Running {
            return;
        }
        let all_over = match mode {
            GroupMode::Together => r.queue.clone(),
            _ => Vec::new(),
        };
        let finished = if mode == GroupMode::Together {
            all_over.iter().all(|c| {
                matches!(
                    self.rt.get(c).map(|r| r.state),
                    Some(CueState::Done | CueState::Stopped)
                )
            })
        } else {
            r.pos += 1;
            if let Some(&next) = r.queue.get(r.pos) {
                self.fire(next);
                false
            } else {
                true
            }
        };
        if finished {
            self.group_done(group);
        }
    }

    fn group_done(&mut self, group: CueId) {
        let looped = matches!(
            self.nodes.get(&group).map(|n| &n.cue.kind),
            Some(CueKind::Group { looped: true, .. })
        );
        if looped {
            // Restart on the next tick, so a loop of instant cues cannot spin.
            let due = self.clock.now() + 0.01;
            self.pending.push((due, Pending::Restart(group)));
        } else {
            self.finish(group);
        }
    }

    fn stop_with(&mut self, id: CueId, fade: Seconds) {
        let Some(node) = self.nodes.get(&id) else {
            return;
        };
        let (children, is_audio) = (
            child_ids(&node.cue),
            matches!(node.cue.kind, CueKind::Audio { .. }),
        );
        if self.state(id).is_active() {
            if is_audio {
                self.audio.stop(id, fade);
            }
            self.set_state(id, CueState::Stopped);
        }
        self.pending.retain(|(_, p)| match p {
            Pending::Fire { from, .. } => *from != id,
            Pending::Restart(g) => *g != id,
        });
        for child in children {
            self.stop_with(child, fade);
        }
    }

    /// Fires everything that has come due, including chains of instant cues.
    fn drain(&mut self) {
        for _ in 0..MAX_CHAIN {
            let now = self.clock.now();
            let Some(i) = self
                .pending
                .iter()
                .enumerate()
                .filter(|(_, (due, _))| *due <= now)
                .min_by(|a, b| a.1.0.total_cmp(&b.1.0))
                .map(|(i, _)| i)
            else {
                return;
            };
            match self.pending.remove(i).1 {
                Pending::Fire { target, .. } => {
                    if !self.state(target).is_active() {
                        self.fire(target);
                    }
                }
                Pending::Restart(group) => {
                    if self.state(group) == CueState::Running {
                        self.start_group(group);
                    }
                }
            }
        }
    }

    fn shuffle(&mut self, items: &mut [CueId]) {
        for i in (1..items.len()).rev() {
            self.rng ^= self.rng << 13;
            self.rng ^= self.rng >> 7;
            self.rng ^= self.rng << 17;
            let j = (self.rng % (i as u64 + 1)) as usize;
            items.swap(i, j);
        }
    }
}

fn child_ids(cue: &Cue) -> Vec<CueId> {
    match &cue.kind {
        CueKind::Group { children, .. } => children.iter().map(|c| c.id).collect(),
        _ => Vec::new(),
    }
}
