//! Native mania playback in beatmap milliseconds. Judgement and scoring are separate layers.
pub mod judge;
mod mods;
pub mod scoring;
pub mod stable;
mod stable_combo;
use dossier_beatmap::{Beatmap, ObjectKind};
use dossier_replay::{GameMode, Replay, ReplayFrame};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    WrongMode,
    UnsupportedMods,
    InvalidColumns,
    InvalidNote(usize),
    InvalidFrame(usize),
    InvalidFrameOffsets,
    InvalidFrameOffset(usize),
    InvalidView,
    InvalidDifficulty,
    InvalidClockRate,
    MismatchedClockRate,
    OverlappingNotes(usize),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "mania playback: {self:?}")
    }
}
impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Note {
    pub object_index: usize,
    pub column: u8,
    pub start_ms: f64,
    pub end_ms: f64,
    pub hold: bool,
}

#[derive(Debug, Clone)]
struct Lane {
    notes: Vec<Note>,
    // Prefix maxima allow seeking past expired notes without losing long holds.
    ends: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct Timeline {
    lanes: Vec<Lane>,
}
impl Timeline {
    pub fn new(map: &Beatmap) -> Result<Self, Error> {
        if map.mode != 3 {
            return Err(Error::WrongMode);
        }
        let columns = map.difficulty.circle_size;
        if !columns.is_finite() || columns.fract() != 0.0 || !(1.0..=18.0).contains(&columns) {
            return Err(Error::InvalidColumns);
        }
        let mut lanes = vec![
            Lane {
                notes: Vec::new(),
                ends: Vec::new()
            };
            columns as usize
        ];
        for (index, object) in map.objects.iter().enumerate() {
            let (end, hold) = match object.kind {
                ObjectKind::Circle => (object.time_ms, false),
                ObjectKind::Hold { end_time_ms } => (end_time_ms, true),
                _ => return Err(Error::InvalidNote(index)),
            };
            if !object.pos.x.is_finite()
                || !object.time_ms.is_finite()
                || !end.is_finite()
                || end < object.time_ms
            {
                return Err(Error::InvalidNote(index));
            }
            let column = (object.pos.x * columns / 512.0)
                .floor()
                .clamp(0.0, columns - 1.0) as u8;
            lanes[column as usize].notes.push(Note {
                object_index: index,
                column,
                start_ms: object.time_ms,
                end_ms: end,
                hold,
            });
        }
        for lane in &mut lanes {
            lane.notes.sort_by(|a, b| a.start_ms.total_cmp(&b.start_ms));
            let mut end = f64::NEG_INFINITY;
            for note in &lane.notes {
                end = end.max(note.end_ms);
                lane.ends.push(end);
            }
        }
        Ok(Self { lanes })
    }
    pub fn columns(&self) -> u8 {
        self.lanes.len() as u8
    }
    pub fn notes(&self, column: u8) -> Option<&[Note]> {
        self.lanes
            .get(column as usize)
            .map(|lane| lane.notes.as_slice())
    }
    /// Only geometry: positions are signed distances from the judgement line,
    /// in screen heights. Positive values approach the line; negative values passed it.
    pub fn visible_at(&self, time_ms: f64, approach_ms: f64) -> Result<Vec<VisibleNote>, Error> {
        if !time_ms.is_finite() || !approach_ms.is_finite() || approach_ms <= 0.0 {
            return Err(Error::InvalidView);
        }
        let mut visible = Vec::new();
        for lane in &self.lanes {
            let start = lane.ends.partition_point(|end| *end < time_ms);
            let end = lane
                .notes
                .partition_point(|note| note.start_ms <= time_ms + approach_ms);
            for note in &lane.notes[start.min(end)..end] {
                if note.end_ms >= time_ms {
                    visible.push(VisibleNote {
                        note: *note,
                        head: (note.start_ms - time_ms) / approach_ms,
                        tail: (note.end_ms - time_ms) / approach_ms,
                    });
                }
            }
        }
        Ok(visible)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VisibleNote {
    pub note: Note,
    pub head: f64,
    pub tail: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    pub time_ms: i64,
    pub column: u8,
    pub pressed: bool,
    /// Original decoded replay frame; simultaneous column changes share it.
    pub frame_index: usize,
}
#[derive(Debug, Clone)]
pub struct InputTrack {
    columns: u8,
    states: Vec<(i64, u32)>,
    events: Vec<KeyEvent>,
}
impl InputTrack {
    /// Stable prefixes replays with up to two skip markers at (256, -500).
    /// Their X is not a key mask. Remove only those known prefix markers;
    /// retain original timestamps/indices and reject malformed gameplay frames.
    pub fn from_replay(replay: &Replay, columns: u8) -> Result<Self, Error> {
        let stable = replay.game_version < 30_000_000;
        let mut last_time = i64::MIN;
        let mut gameplay_started = false;
        Self::from_frames(
            replay.frames.iter().enumerate().filter(|(index, frame)| {
                if stable && *index < 2 && frame.x == 256.0 && frame.y == -500.0 {
                    return false;
                }
                // Legacy pre-roll may briefly run backwards while no key is down.
                // Only redundant idle frames before the first press can be removed
                // losslessly. Never sort frames or discard a gameplay transition.
                if stable && !gameplay_started && frame.x == 0.0 && frame.time_ms < last_time {
                    return false;
                }
                gameplay_started |= frame.x != 0.0;
                last_time = frame.time_ms;
                true
            }),
            columns,
        )
    }
    /// In mania's legacy replay encoding X is the column bitmask, not `Keys`.
    pub fn new(frames: &[ReplayFrame], columns: u8) -> Result<Self, Error> {
        Self::from_frames(frames.iter().enumerate(), columns)
    }
    fn from_frames<'a>(
        frames: impl IntoIterator<Item = (usize, &'a ReplayFrame)>,
        columns: u8,
    ) -> Result<Self, Error> {
        if !(1..=18).contains(&columns) {
            return Err(Error::InvalidColumns);
        }
        let mask = (1u32 << columns) - 1;
        let mut states = Vec::new();
        let mut events = Vec::new();
        let mut previous = 0u32;
        let mut previous_time = i64::MIN;
        for (index, frame) in frames {
            let x = frame.x;
            if frame.time_ms < previous_time
                || !x.is_finite()
                || x < 0.0
                || x.fract() != 0.0
                || x > mask as f32
            {
                return Err(Error::InvalidFrame(index));
            }
            let down = x as u32;
            for column in 0..columns {
                let bit = 1u32 << column;
                if (previous ^ down) & bit != 0 {
                    events.push(KeyEvent {
                        time_ms: frame.time_ms,
                        column,
                        pressed: down & bit != 0,
                        frame_index: index,
                    });
                }
            }
            if down != previous {
                states.push((frame.time_ms, down));
            }
            previous = down;
            previous_time = frame.time_ms;
        }
        Ok(Self {
            columns,
            states,
            events,
        })
    }
    pub fn events(&self) -> &[KeyEvent] {
        &self.events
    }
    /// Last state at this timestamp wins; sampling is independent of seek direction.
    pub fn keys_at(&self, time_ms: f64) -> u32 {
        if !time_ms.is_finite() {
            return 0;
        }
        let end = self.states.partition_point(|(at, _)| *at as f64 <= time_ms);
        end.checked_sub(1).map_or(0, |index| self.states[index].1)
    }
}

#[derive(Debug, Clone)]
pub struct Playback {
    pub timeline: Timeline,
    pub input: InputTrack,
    options: mods::ReplayOptions,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub time_ms: f64,
    pub keys: u32,
    pub notes: Vec<VisibleNote>,
}
impl Playback {
    fn check_stable_rules(&self, rules: stable::StableRules) -> Result<(), Error> {
        if self.has_accuracy_challenge()
            || (!self.options.stable && self.clock_rate() != 1.0)
            || rules.hard_rock() != self.options.hard_rock
        {
            return Err(Error::UnsupportedMods);
        }
        if rules.clock_rate() != self.clock_rate() {
            return Err(Error::MismatchedClockRate);
        }
        Ok(())
    }
    /// Select the supported native stable ScoreV1 windows from replay modifiers.
    pub fn stable_rules(&self, od: f64) -> Result<stable::StableRules, Error> {
        if !self.options.stable {
            return Err(Error::UnsupportedMods);
        }
        stable::StableRules::native_with_mods(od, self.clock_rate(), self.options.hard_rock)
    }
    /// Explicit stable ScoreV1 policy; taps and combined hold judgements.
    pub fn judge_stable_with(&self, rules: stable::StableRules) -> Result<judge::Judge, Error> {
        self.check_stable_rules(rules)?;
        rules.judge(&self.timeline, &self.input)
    }
    /// Optional diagnostic path with the same modifier guard as normal judging.
    pub fn trace_stable_with(
        &self,
        rules: stable::StableRules,
    ) -> Result<(judge::Judge, Vec<stable::TraceEvent>), Error> {
        self.check_stable_rules(rules)?;
        rules.judge_with_trace(&self.timeline, &self.input)
    }
    /// Opt-in analytical lazer rules. This does not establish replay fidelity,
    /// nor choose rules for stable or older lazer replay versions automatically.
    pub fn judge_with(&self, rules: judge::LazerRules) -> Result<judge::Judge, Error> {
        if self.options.hard_rock {
            return Err(Error::UnsupportedMods);
        }
        if rules.clock_rate() != self.clock_rate() {
            return Err(Error::MismatchedClockRate);
        }
        judge::Judge::new(&self.timeline, &self.input, rules)
    }
    /// Windows in beatmap milliseconds for the selected experimental lazer policy.
    pub fn lazer_rules(&self, od: f64) -> Result<judge::LazerRules, Error> {
        if self.options.hard_rock {
            return Err(Error::UnsupportedMods);
        }
        judge::LazerRules::native_at_rate(od, self.clock_rate())
    }
    /// Playback/audio clock rate. Map and input timestamps remain unchanged.
    pub fn clock_rate(&self) -> f64 {
        self.options.clock_rate
    }
    /// Note grades are supported for AC; its failure condition is not simulated.
    pub fn has_accuracy_challenge(&self) -> bool {
        self.options.accuracy_challenge
    }
    /// Native-map geometry/input. Accepts stable NoMod/HR/DT and lazer NoMod/DT/AC.
    /// Conversion, other modifiers and AC failure handling are not implemented.
    pub fn new(map: &Beatmap, replay: &Replay) -> Result<Self, Error> {
        if replay.mode != GameMode::Mania {
            return Err(Error::WrongMode);
        }
        let options = mods::ReplayOptions::from_replay(replay)?;
        let timeline = Timeline::new(map)?;
        let input = InputTrack::from_replay(replay, timeline.columns())?;
        Ok(Self {
            timeline,
            input,
            options,
        })
    }
    pub fn at(&self, time_ms: f64, approach_ms: f64) -> Result<Snapshot, Error> {
        Ok(Snapshot {
            time_ms,
            keys: self.input.keys_at(time_ms),
            notes: self.timeline.visible_at(time_ms, approach_ms)?,
        })
    }
}
