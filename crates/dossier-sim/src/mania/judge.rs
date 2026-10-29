use std::collections::BTreeSet;

use super::scoring::{Accumulator, LazerScore};
use super::{Error, InputTrack, Note, Timeline};
use dossier_replay::ReplayFrame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum ResultKind {
    Perfect,
    Great,
    Good,
    Ok,
    Meh,
    Miss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Tap,
    Hold,
    Head,
    Tail,
    HoldBreak,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Event {
    pub object_index: usize,
    pub column: u8,
    pub time_ms: f64,
    pub part: Part,
    pub result: ResultKind,
    pub offset_ms: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LazerRules {
    windows: [f64; 6],
    clock_rate: f64,
}
impl LazerRules {
    pub const UPSTREAM: &'static str = "bdfabf93c77483bb471dfde8c900fa9176a9d531";
    pub fn native_no_mod(od: f64) -> Result<Self, Error> {
        Self::native_at_rate(od, 1.0)
    }
    pub fn native_at_rate(od: f64, clock_rate: f64) -> Result<Self, Error> {
        if !clock_rate.is_finite() || !(0.5..=2.0).contains(&clock_rate) {
            return Err(Error::InvalidClockRate);
        }
        if !od.is_finite() || !(0.0..=10.0).contains(&od) {
            return Err(Error::InvalidDifficulty);
        }
        let od = f64::from(od as f32);
        let factor = (od - 5.0) / 5.0;
        Ok(Self {
            clock_rate,
            windows: [
                (22.4, 19.4, 13.9),
                (64.0, 49.0, 34.0),
                (97.0, 82.0, 67.0),
                (127.0, 112.0, 97.0),
                (151.0, 136.0, 121.0),
                (188.0, 173.0, 158.0),
            ]
            .map(|(min, mid, max)| mid + if od > 5.0 { max - mid } else { mid - min } * factor)
            .map(|ms| (ms * clock_rate).floor() + 0.5),
        })
    }
    pub fn clock_rate(self) -> f64 {
        self.clock_rate
    }
    pub fn window(self, result: ResultKind) -> f64 {
        self.windows[result as usize]
    }
    pub fn result_for(self, offset_ms: f64) -> Option<ResultKind> {
        if !offset_ms.is_finite() {
            return None;
        }
        [
            ResultKind::Perfect,
            ResultKind::Great,
            ResultKind::Good,
            ResultKind::Ok,
            ResultKind::Meh,
            ResultKind::Miss,
        ]
        .into_iter()
        .find(|kind| offset_ms.abs() <= self.window(*kind))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    values: [u32; 6],
    pub hold_breaks: u32,
}
impl Counts {
    pub fn get(self, kind: ResultKind) -> u32 {
        self.values[kind as usize]
    }
    pub fn total(self) -> u32 {
        self.values.iter().sum()
    }
    fn apply(&mut self, event: &Event) {
        if event.part == Part::HoldBreak {
            self.hold_breaks += 1;
        } else {
            self.values[event.result as usize] += 1;
        }
    }
}

#[derive(Debug, Clone)]
pub struct Judge {
    events: Vec<Event>,
    prefixes: Vec<Counts>,
    combo_prefixes: Option<Vec<Combo>>,
    pub(super) stable_combo: Option<super::stable_combo::StableCombo>,
    score_prefixes: Option<Vec<LazerScore>>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Combo {
    pub current: u32,
    pub maximum: u32,
}

impl Combo {
    fn apply(&mut self, event: &Event) {
        if event.part == Part::HoldBreak || event.result == ResultKind::Miss {
            self.current = 0;
        } else {
            self.current += 1;
            self.maximum = self.maximum.max(self.current);
        }
    }
}
impl Judge {
    pub fn new(timeline: &Timeline, input: &InputTrack, rules: LazerRules) -> Result<Self, Error> {
        if input.columns != timeline.columns() {
            return Err(Error::InvalidColumns);
        }
        let mut edges = vec![Vec::new(); timeline.columns() as usize];
        for (order, edge) in input.events().iter().enumerate() {
            edges[edge.column as usize].push((edge.time_ms as f64, edge.pressed, order));
        }
        Self::judge_edges(timeline, rules, &edges)
    }

    pub fn with_frame_offsets(
        timeline: &Timeline,
        frames: &[ReplayFrame],
        offsets: &[f64],
        rules: LazerRules,
    ) -> Result<Self, Error> {
        if offsets.len() != frames.len() {
            return Err(Error::InvalidFrameOffsets);
        }
        InputTrack::new(frames, timeline.columns())?;
        let mut edges = vec![Vec::new(); timeline.columns() as usize];
        let mut previous_mask = 0u32;
        let mut previous_time = f64::NEG_INFINITY;
        let mut order = 0;
        for (index, (frame, offset)) in frames.iter().zip(offsets).enumerate() {
            let time = frame.time_ms as f64 + offset;
            if !offset.is_finite()
                || offset.abs() > 0.5
                || frame.time_ms.unsigned_abs() >= (1u64 << 51)
                || time < previous_time
            {
                return Err(Error::InvalidFrameOffset(index));
            }
            let mask = frame.x as u32;
            for column in 0..timeline.columns() {
                let bit = 1u32 << column;
                if (previous_mask ^ mask) & bit != 0 {
                    edges[column as usize].push((time, mask & bit != 0, order));
                    order += 1;
                }
            }
            previous_mask = mask;
            previous_time = time;
        }
        Self::judge_edges(timeline, rules, &edges)
    }

    fn judge_edges(
        timeline: &Timeline,
        rules: LazerRules,
        edges: &[Vec<(f64, bool, usize)>],
    ) -> Result<Self, Error> {
        let mut events = Vec::new();
        for (column, lane) in timeline.lanes.iter().enumerate() {
            for pair in lane.notes.windows(2) {
                let duplicate_taps =
                    pair[1].start_ms == pair[0].start_ms && !pair[0].hold && !pair[1].hold;
                if !duplicate_taps
                    && (pair[1].start_ms <= pair[0].start_ms || pair[1].start_ms < pair[0].end_ms)
                {
                    return Err(Error::OverlappingNotes(pair[1].object_index));
                }
            }
            LaneJudge::new(&lane.notes, rules).run(&edges[column], &mut events);
        }
        events.sort_by(|a, b| {
            a.event
                .time_ms
                .total_cmp(&b.event.time_ms)
                .then(a.input_order.cmp(&b.input_order))
                .then(a.event.column.cmp(&b.event.column))
        });
        let mut judge = Self::from_chronological_events(
            events.into_iter().map(|ordered| ordered.event).collect(),
        );
        let mut combo = Combo::default();
        let total = timeline
            .lanes
            .iter()
            .flat_map(|l| &l.notes)
            .map(|n| if n.hold { 2 } else { 1 })
            .sum();
        let mut scoring = Accumulator::new(total);
        let mut combos = Vec::with_capacity(judge.events.len());
        let mut scores = Vec::with_capacity(judge.events.len());
        for event in &judge.events {
            combo.apply(event);
            combos.push(combo);
            scores.push(scoring.apply(event, combo));
        }
        judge.combo_prefixes = Some(combos);
        judge.score_prefixes = Some(scores);
        Ok(judge)
    }
    pub(super) fn from_events(mut events: Vec<OrderedEvent>) -> Self {
        events.sort_by(|a, b| {
            a.event
                .time_ms
                .total_cmp(&b.event.time_ms)
                .then(a.input_order.cmp(&b.input_order))
                .then(a.event.object_index.cmp(&b.event.object_index))
        });
        let mut judge =
            Self::from_chronological_events(events.into_iter().map(|e| e.event).collect());
        if judge.events.iter().all(|event| event.part == Part::Tap) {
            let mut combo = Combo::default();
            judge.combo_prefixes = Some(
                judge
                    .events
                    .iter()
                    .map(|event| {
                        combo.apply(event);
                        combo
                    })
                    .collect(),
            );
        }
        judge
    }
    fn from_chronological_events(events: Vec<Event>) -> Self {
        let mut count = Counts::default();
        let prefixes = events
            .iter()
            .map(|event| {
                count.apply(event);
                count
            })
            .collect();
        Self {
            events,
            prefixes,
            combo_prefixes: None,
            stable_combo: None,
            score_prefixes: None,
        }
    }
    pub fn events(&self) -> &[Event] {
        &self.events
    }
    pub fn counts_at(&self, time_ms: f64) -> Counts {
        if time_ms.is_nan() {
            return Counts::default();
        }
        let end = self
            .events
            .partition_point(|event| event.time_ms <= time_ms);
        end.checked_sub(1)
            .map_or(Counts::default(), |index| self.prefixes[index])
    }
    pub fn final_counts(&self) -> Counts {
        self.prefixes.last().copied().unwrap_or_default()
    }

    pub fn combo_at(&self, time_ms: f64) -> Option<Combo> {
        if let Some(combo) = &self.stable_combo {
            return Some(combo.at(time_ms));
        }
        let prefixes = self.combo_prefixes.as_ref()?;
        if time_ms.is_nan() {
            return Some(Combo::default());
        }
        let end = self
            .events
            .partition_point(|event| event.time_ms <= time_ms);
        Some(end.checked_sub(1).map_or(Combo::default(), |i| prefixes[i]))
    }

    pub fn final_combo(&self) -> Option<Combo> {
        self.combo_at(f64::INFINITY)
    }

    pub fn score_at(&self, time_ms: f64) -> Option<LazerScore> {
        let prefixes = self.score_prefixes.as_ref()?;
        if time_ms.is_nan() {
            return Some(LazerScore::default());
        }
        let end = self.events.partition_point(|e| e.time_ms <= time_ms);
        Some(
            end.checked_sub(1)
                .map_or(LazerScore::default(), |i| prefixes[i]),
        )
    }

    pub fn final_score(&self) -> Option<LazerScore> {
        self.score_prefixes
            .as_ref()
            .map(|p| p.last().copied().unwrap_or_default())
    }
}

#[derive(Default, Clone)]
struct State {
    head: Option<ResultKind>,
    tail: bool,
    holding: bool,
    broken: bool,
}
pub(super) struct OrderedEvent {
    pub(super) event: Event,
    pub(super) input_order: Option<usize>,
}
struct LaneJudge<'a> {
    notes: &'a [Note],
    states: Vec<State>,
    pending: BTreeSet<usize>,
    holds: BTreeSet<usize>,
    tails: BTreeSet<usize>,
    deadlines: Vec<(f64, usize, Part)>,
    deadline: usize,
    rules: LazerRules,
    input_order: Option<usize>,
}
impl<'a> LaneJudge<'a> {
    fn new(notes: &'a [Note], rules: LazerRules) -> Self {
        let mut deadlines = Vec::new();
        for (i, note) in notes.iter().enumerate() {
            deadlines.push((
                (note.start_ms + rules.window(ResultKind::Meh)).next_up(),
                i,
                Part::Head,
            ));
            if note.hold {
                deadlines.push((
                    (note.end_ms + rules.window(ResultKind::Meh) * 1.5).next_up(),
                    i,
                    Part::Tail,
                ));
            }
        }
        deadlines.sort_by(|a, b| a.0.total_cmp(&b.0));
        Self {
            notes,
            states: vec![State::default(); notes.len()],
            pending: (0..notes.len()).collect(),
            holds: BTreeSet::new(),
            tails: notes
                .iter()
                .enumerate()
                .filter_map(|(i, n)| n.hold.then_some(i))
                .collect(),
            deadlines,
            deadline: 0,
            rules,
            input_order: None,
        }
    }
    fn emit(
        &self,
        index: usize,
        at: f64,
        part: Part,
        result: ResultKind,
        offset: Option<f64>,
        out: &mut Vec<OrderedEvent>,
    ) {
        let note = self.notes[index];
        out.push(OrderedEvent {
            input_order: self.input_order,
            event: Event {
                object_index: note.object_index,
                column: note.column,
                time_ms: at,
                part,
                result,
                offset_ms: offset,
            },
        });
    }
    fn head(
        &mut self,
        index: usize,
        at: f64,
        result: ResultKind,
        offset: Option<f64>,
        out: &mut Vec<OrderedEvent>,
    ) {
        if self.states[index].head.is_some() {
            return;
        }
        self.states[index].head = Some(result);
        self.pending.remove(&index);
        self.emit(
            index,
            at,
            if self.notes[index].hold {
                Part::Head
            } else {
                Part::Tap
            },
            result,
            offset,
            out,
        );
    }
    fn broken(&mut self, index: usize, at: f64, out: &mut Vec<OrderedEvent>) {
        if !self.states[index].broken {
            self.states[index].broken = true;
            self.emit(index, at, Part::HoldBreak, ResultKind::Miss, None, out);
        }
    }
    fn tail(
        &mut self,
        index: usize,
        at: f64,
        mut result: ResultKind,
        offset: Option<f64>,
        out: &mut Vec<OrderedEvent>,
    ) {
        if self.states[index].tail {
            return;
        }
        if result != ResultKind::Miss
            && (self.states[index].head.is_none()
                || self.states[index].head == Some(ResultKind::Miss)
                || self.states[index].broken)
        {
            result = ResultKind::Meh;
        }
        self.emit(index, at, Part::Tail, result, offset, out);
        if result == ResultKind::Miss {
            self.broken(index, at, out);
        }
        self.states[index].tail = true;
        self.states[index].holding = false;
        self.holds.remove(&index);
        self.tails.remove(&index);
    }
    fn expire(&mut self, at: f64, out: &mut Vec<OrderedEvent>) {
        while self.deadline < self.deadlines.len() && self.deadlines[self.deadline].0 <= at {
            let (when, index, part) = self.deadlines[self.deadline];
            self.deadline += 1;
            if part == Part::Head {
                self.head(index, when, ResultKind::Miss, None, out);
            } else {
                self.tail(index, when, ResultKind::Miss, None, out);
            }
        }
    }
    fn force_earlier(&mut self, target: usize, at: f64, out: &mut Vec<OrderedEvent>) {
        while let Some(&index) = self.pending.first() {
            if index >= target || self.notes[index].end_ms >= self.notes[target].start_ms {
                break;
            }
            self.head(index, at, ResultKind::Miss, None, out);
            if self.notes[index].hold {
                self.tail(index, at, ResultKind::Miss, None, out);
            }
        }
        let earlier: Vec<_> = self
            .tails
            .iter()
            .copied()
            .take_while(|i| self.notes[*i].end_ms < self.notes[target].start_ms)
            .collect();
        for index in earlier {
            self.tail(index, at, ResultKind::Miss, None, out);
        }
    }
    fn press(&mut self, at: f64, out: &mut Vec<OrderedEvent>) {
        let started = self.notes.partition_point(|note| note.start_ms <= at);
        let from = started.saturating_sub(1);
        let candidate = self.pending.range(from..).next().copied();
        for index in from..candidate.unwrap_or(self.notes.len()) {
            let note = self.notes[index];
            if at < note.start_ms - self.rules.window(ResultKind::Miss) {
                break;
            }
            if note.hold
                && !self.states[index].tail
                && at <= note.end_ms + self.rules.window(ResultKind::Meh)
            {
                self.states[index].holding = true;
                self.holds.insert(index);
            }
        }
        let Some(index) = candidate else {
            return;
        };
        let note = self.notes[index];
        let offset = at - note.start_ms;
        let Some(result) = self.rules.result_for(offset) else {
            return;
        };
        if note.hold {
            self.states[index].holding = true;
            self.holds.insert(index);
        }
        self.head(index, at, result, Some(offset), out);
        if result != ResultKind::Miss {
            self.force_earlier(index, at, out);
        }
    }
    fn release(&mut self, at: f64, out: &mut Vec<OrderedEvent>) {
        let held: Vec<_> = self
            .holds
            .iter()
            .copied()
            .filter(|i| self.states[*i].holding)
            .collect();
        for index in held {
            let offset = at - self.notes[index].end_ms;
            if let Some(result) = self.rules.result_for(offset / 1.5) {
                self.tail(index, at, result, Some(offset), out);
            } else {
                self.broken(index, at, out);
            }
            self.states[index].holding = false;
        }
    }
    fn run(mut self, edges: &[(f64, bool, usize)], out: &mut Vec<OrderedEvent>) {
        for &(at, pressed, order) in edges {
            self.input_order = None;
            self.expire(at, out);
            self.input_order = Some(order);
            if pressed {
                self.press(at, out);
            } else {
                self.release(at, out);
            }
        }
        self.input_order = None;
        self.expire(f64::INFINITY, out);
    }
}
