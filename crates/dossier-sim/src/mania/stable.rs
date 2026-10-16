//! Experimental native stable ScoreV1 policy (NoMod, HR, DT and HRDT).
//! Source: osu!wiki Gameplay/Judgement/osu!mania. ScoreV2 is a separate, unsupported policy.
use std::collections::BTreeSet;

use super::{
    judge::{Event, Judge, OrderedEvent, Part, ResultKind},
    Error, InputTrack, KeyEvent, Note, Timeline,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StableRules {
    windows: [f64; 6],
    clock_rate: f64,
    hard_rock: bool,
}

/// Diagnostic transitions, never extra scored judgements. These report the
/// simulator's decisions, not recovered events from the original game client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceKind {
    HeadPressed,
    HeadExpired,
    HeadForcedMiss,
    ReleasedBeforeHead,
    BodyReleased,
    Regrabbed,
    ReleasedAtTail,
    Overheld,
    TailExpired,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceEvent {
    pub object_index: usize,
    pub column: u8,
    pub time_ms: f64,
    pub kind: TraceKind,
    /// The originally accepted head offset; re-grabbing does not replace it.
    pub head_offset_ms: Option<f64>,
    pub broken: bool,
    /// Original triggering input frame; None for analytical expiry.
    pub frame_index: Option<usize>,
}

impl StableRules {
    pub fn native_no_mod(od: f64) -> Result<Self, Error> {
        Self::native_with_mods(od, 1.0, false)
    }
    /// Apply DT/HR to raw windows before truncation (the official Classic
    /// ScoreV1 policy). Input and map timestamps stay in beatmap milliseconds.
    pub fn native_with_mods(od: f64, clock_rate: f64, hard_rock: bool) -> Result<Self, Error> {
        if !od.is_finite() || !(0.0..=10.0).contains(&od) {
            return Err(Error::InvalidDifficulty);
        }
        if clock_rate != 1.0 && clock_rate != 1.5 {
            return Err(Error::InvalidClockRate);
        }
        let inverted = 10.0 - f64::from(od as f32);
        let multiplier = clock_rate / if hard_rock { 1.4 } else { 1.0 };
        Ok(Self {
            windows: [
                16.0,
                34.0 + 3.0 * inverted,
                67.0 + 3.0 * inverted,
                97.0 + 3.0 * inverted,
                121.0 + 3.0 * inverted,
                158.0 + 3.0 * inverted,
            ]
            .map(|window| (window * multiplier).floor()),
            clock_rate,
            hard_rock,
        })
    }
    pub fn clock_rate(self) -> f64 {
        self.clock_rate
    }
    pub fn hard_rock(self) -> bool {
        self.hard_rock
    }
    /// Integer beatmap-time thresholds, compared against rounded hit error.
    pub fn window(self, result: ResultKind) -> f64 {
        self.windows[result as usize]
    }
    pub fn result_for(self, offset_ms: f64) -> Option<ResultKind> {
        if !offset_ms.is_finite() {
            return None;
        }
        let error = offset_ms.abs().round_ties_even();
        [
            ResultKind::Perfect,
            ResultKind::Great,
            ResultKind::Good,
            ResultKind::Ok,
            ResultKind::Meh,
            ResultKind::Miss,
        ]
        .into_iter()
        .find(|kind| error <= self.window(*kind))
    }
    /// ScoreV1 has a head constraint in addition to the combined error budget.
    pub fn hold_result_for(self, head_offset_ms: f64, tail_offset_ms: f64) -> Option<ResultKind> {
        if !head_offset_ms.is_finite() || !tail_offset_ms.is_finite() {
            return None;
        }
        let head = head_offset_ms.abs().round_ties_even();
        let combined = head + tail_offset_ms.abs().round_ties_even();
        Some(
            [
                (ResultKind::Perfect, 1.2),
                (ResultKind::Great, 1.1),
                (ResultKind::Good, 1.0),
                (ResultKind::Ok, 1.0),
            ]
            .into_iter()
            .find_map(|(kind, factor)| {
                let limit = self.window(kind) * factor;
                (head <= limit && combined <= limit * 2.0).then_some(kind)
            })
            .unwrap_or(ResultKind::Meh),
        )
    }
    fn deadline(self, time_ms: f64, hold: bool) -> f64 {
        // Stable's final tap millisecond is excluded; hold heads retain it.
        // Independent stable parity fixtures document this asymmetry (see docs).
        let window = self.window(ResultKind::Ok) - if hold { 0.0 } else { 1.0 };
        let boundary = time_ms + window + 0.5;
        if window as u32 % 2 == 0 {
            boundary.next_up()
        } else {
            boundary
        }
    }

    pub fn judge(self, timeline: &Timeline, input: &InputTrack) -> Result<Judge, Error> {
        self.judge_observing(timeline, input, &mut |_| {})
    }

    /// Runs exactly the same judge while recording its state transitions.
    /// Ordinary judging does not allocate or retain this diagnostic stream.
    pub fn judge_with_trace(
        self,
        timeline: &Timeline,
        input: &InputTrack,
    ) -> Result<(Judge, Vec<TraceEvent>), Error> {
        let mut trace = Vec::new();
        let judge = self.judge_observing(timeline, input, &mut |event| trace.push(event))?;
        trace.sort_by(|a, b| {
            a.time_ms
                .total_cmp(&b.time_ms)
                .then(a.column.cmp(&b.column))
        });
        Ok((judge, trace))
    }

    fn judge_observing(
        self,
        timeline: &Timeline,
        input: &InputTrack,
        observer: &mut impl FnMut(TraceEvent),
    ) -> Result<Judge, Error> {
        if input.columns != timeline.columns() {
            return Err(Error::InvalidColumns);
        }
        let mut edges = vec![Vec::new(); timeline.columns() as usize];
        for edge in input.events() {
            edges[edge.column as usize].push(*edge);
        }
        let mut events = Vec::new();
        let has_holds = timeline.lanes.iter().flat_map(|l| &l.notes).any(|n| n.hold);
        let mut combo = has_holds.then(|| super::stable_combo::Builder::new(timeline));
        for (column, lane) in timeline.lanes.iter().enumerate() {
            if let Some(pair) = lane.notes.windows(2).find(|pair| {
                pair[0].start_ms == pair[1].start_ms || pair[1].start_ms < pair[0].end_ms
            }) {
                return Err(Error::OverlappingNotes(pair[1].object_index));
            }
            StableLane::new(&lane.notes, self).run(&edges[column], &mut events, &mut |event| {
                if let Some(combo) = &mut combo {
                    combo.observe(event);
                }
                observer(event);
            });
        }
        let combo = combo.and_then(|builder| builder.finish(&events));
        let mut judge = Judge::from_events(events);
        judge.stable_combo = combo;
        Ok(judge)
    }
}

#[derive(Default, Clone)]
struct HoldState {
    head_offset: Option<f64>,
    broken: bool,
}
struct StableLane<'a> {
    notes: &'a [Note],
    rules: StableRules,
    heads: BTreeSet<usize>,
    tails: BTreeSet<usize>,
    states: Vec<HoldState>,
    grabbed: Option<usize>,
    input_order: Option<usize>,
}
impl<'a> StableLane<'a> {
    fn new(notes: &'a [Note], rules: StableRules) -> Self {
        Self {
            notes,
            rules,
            heads: (0..notes.len()).collect(),
            tails: notes
                .iter()
                .enumerate()
                .filter_map(|(i, n)| n.hold.then_some(i))
                .collect(),
            states: vec![HoldState::default(); notes.len()],
            grabbed: None,
            input_order: None,
        }
    }
    fn trace(&self, index: usize, at: f64, kind: TraceKind, observer: &mut impl FnMut(TraceEvent)) {
        let note = self.notes[index];
        let state = &self.states[index];
        observer(TraceEvent {
            object_index: note.object_index,
            column: note.column,
            time_ms: at,
            kind,
            head_offset_ms: state.head_offset,
            broken: state.broken,
            frame_index: self.input_order,
        });
    }
    fn expire_head(
        &mut self,
        index: usize,
        at: f64,
        out: &mut Vec<OrderedEvent>,
        kind: TraceKind,
        observer: &mut impl FnMut(TraceEvent),
    ) {
        self.heads.remove(&index);
        let note = self.notes[index];
        if note.hold {
            self.states[index].broken = true;
        } else {
            self.emit(note, at, ResultKind::Miss, None, out);
        }
        self.trace(index, at, kind, observer);
    }
    fn advance(
        &mut self,
        at: f64,
        out: &mut Vec<OrderedEvent>,
        observer: &mut impl FnMut(TraceEvent),
    ) {
        while let Some(&index) = self.heads.first() {
            let note = self.notes[index];
            let deadline = self.rules.deadline(note.start_ms, note.hold);
            if deadline > at {
                break;
            }
            self.expire_head(index, deadline, out, TraceKind::HeadExpired, observer);
        }
        while let Some(&index) = self.tails.first() {
            let note = self.notes[index];
            let deadline = self.rules.deadline(note.end_ms, true);
            if deadline > at {
                break;
            }
            self.tails.remove(&index);
            self.heads.remove(&index);
            let overheld = self.grabbed == Some(index);
            let result = if overheld {
                self.grabbed = None;
                // ScoreV1 overholding substitutes a Meh-sized release error.
                // This yields Good/Ok/Meh from the stored head error, including
                // re-grabbed holds. It is not a doubled release window.
                self.states[index]
                    .head_offset
                    .map_or(ResultKind::Meh, |head| {
                        self.rules
                            .hold_result_for(head, self.rules.window(ResultKind::Meh))
                            .unwrap()
                    })
            } else {
                ResultKind::Miss
            };
            self.trace(
                index,
                deadline,
                if overheld {
                    TraceKind::Overheld
                } else {
                    TraceKind::TailExpired
                },
                observer,
            );
            self.emit(note, deadline, result, None, out);
        }
    }
    fn run(
        mut self,
        edges: &[KeyEvent],
        out: &mut Vec<OrderedEvent>,
        observer: &mut impl FnMut(TraceEvent),
    ) {
        for edge in edges {
            let at = edge.time_ms as f64;
            self.input_order = None;
            self.advance(at, out, observer);
            self.input_order = Some(edge.frame_index);
            if !edge.pressed {
                // Non-overlapping holds allow at most one grabbed body per lane.
                if let Some(index) = self.grabbed.take() {
                    let note = self.notes[index];
                    let state = &mut self.states[index];
                    let offset = at - note.end_ms;
                    if offset < 0.0
                        && offset.abs().round_ties_even() > self.rules.window(ResultKind::Meh)
                    {
                        state.broken = true;
                        self.trace(
                            index,
                            at,
                            if at < note.start_ms {
                                TraceKind::ReleasedBeforeHead
                            } else {
                                TraceKind::BodyReleased
                            },
                            observer,
                        );
                    } else {
                        let result = if state.broken || state.head_offset.is_none() {
                            ResultKind::Meh
                        } else {
                            self.rules
                                .hold_result_for(state.head_offset.unwrap(), offset)
                                .unwrap()
                        };
                        self.tails.remove(&index);
                        self.heads.remove(&index);
                        self.trace(index, at, TraceKind::ReleasedAtTail, observer);
                        self.emit(note, at, result, Some(offset), out);
                    }
                }
                continue;
            }
            // Re-grabbing a started (or previously early-hit) hold is recovery,
            // not another head judgement. A broken body stays capped at Meh.
            let from = self
                .notes
                .partition_point(|note| note.start_ms <= at)
                .saturating_sub(1);
            let next_head = self.heads.range(from..).next().copied();
            if let Some(&index) = self.tails.range(from..).next() {
                let note = self.notes[index];
                // Once the tail has passed, a recoverable old hold must not
                // steal an early press intended for the following note.
                let following_head = next_head.is_some_and(|i| {
                    i > index && self.rules.result_for(at - self.notes[i].start_ms).is_some()
                });
                if (at <= note.end_ms || !following_head)
                    && ((at >= note.start_ms && !self.heads.contains(&index))
                        || self.states[index].head_offset.is_some())
                {
                    self.grabbed = Some(index);
                    self.trace(index, at, TraceKind::Regrabbed, observer);
                    continue;
                }
            }
            let Some(index) = next_head else {
                continue;
            };
            let note = self.notes[index];
            let offset = at - note.start_ms;
            let Some(result) = self.rules.result_for(offset) else {
                continue;
            };
            self.heads.remove(&index);
            if note.hold {
                let state = &mut self.states[index];
                state.head_offset = Some(offset);
                self.grabbed = Some(index);
                state.broken |= result == ResultKind::Miss;
            } else {
                self.states[index].head_offset = Some(offset);
                self.emit(note, at, result, Some(offset), out);
            }
            self.trace(index, at, TraceKind::HeadPressed, observer);
            if result != ResultKind::Miss {
                while let Some(&earlier) = self.heads.first() {
                    if earlier >= index {
                        break;
                    }
                    self.expire_head(earlier, at, out, TraceKind::HeadForcedMiss, observer);
                }
            }
        }
        self.input_order = None;
        self.advance(f64::INFINITY, out, observer);
    }

    fn emit(
        &self,
        note: Note,
        time_ms: f64,
        result: ResultKind,
        offset_ms: Option<f64>,
        events: &mut Vec<OrderedEvent>,
    ) {
        events.push(OrderedEvent {
            input_order: self.input_order,
            event: Event {
                object_index: note.object_index,
                column: note.column,
                time_ms,
                part: if note.hold { Part::Hold } else { Part::Tap },
                result,
                offset_ms,
            },
        });
    }
}
