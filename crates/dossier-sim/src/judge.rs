use std::collections::HashSet;
use std::f64::consts::{PI, TAU};

use dossier_beatmap::Point;
use dossier_replay::{HitCounts, Keys, ReplayFrame};

use crate::cursor::{CursorTrack, Side};
use crate::ruleset::Ruleset;
use crate::timeline::{TimedKind, TimedObject, Timeline};

pub const FOLLOW_CIRCLE_SCALE: f64 = 2.4;

pub const TAIL_LENIENCE_MS: f64 = 36.0;

const CLICK_KEYS: u8 = Keys::M1 | Keys::M2 | Keys::K1 | Keys::K2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Judgement {
    Great,

    Ok,

    Meh,
    Miss,
}

impl Judgement {
    pub fn value(self) -> u32 {
        match self {
            Self::Great => 300,
            Self::Ok => 100,
            Self::Meh => 50,
            Self::Miss => 0,
        }
    }

    pub fn is_miss(self) -> bool {
        self == Self::Miss
    }

    fn from_flag(hit: bool) -> Self {
        if hit {
            Self::Great
        } else {
            Self::Miss
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Circle,
    SliderHead,
    SliderTick,
    SliderRepeat,
    SliderTail,

    Slider,
    Spinner,

    SpinnerSpin,

    SpinnerPoints,

    SpinnerBonus,
}

impl Part {
    pub fn counts_for_accuracy(self) -> bool {
        matches!(self, Self::Circle | Self::Slider | Self::Spinner)
    }

    pub fn adds_combo(self) -> bool {
        !matches!(
            self,
            Self::Slider | Self::SpinnerSpin | Self::SpinnerPoints | Self::SpinnerBonus
        )
    }

    pub fn is_bonus(self) -> bool {
        matches!(
            self,
            Self::SpinnerSpin | Self::SpinnerPoints | Self::SpinnerBonus
        )
    }

    pub fn breaks_combo(self) -> bool {
        !matches!(self, Self::Slider | Self::SliderTail)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verdict {
    Landed { object: usize },

    TookItEarly { object: usize },

    Refused { object: usize, blocked_by: usize },

    OutOfRange { object: usize },

    Ignored { object: usize },

    FoundNothing,
}

impl Verdict {
    pub fn object(self) -> Option<usize> {
        match self {
            Self::Landed { object }
            | Self::TookItEarly { object }
            | Self::Refused { object, .. }
            | Self::OutOfRange { object }
            | Self::Ignored { object } => Some(object),
            Self::FoundNothing => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Landed { .. } => "landed",
            Self::TookItEarly { .. } => "took a note early",
            Self::Refused { .. } => "refused by the lock",
            Self::OutOfRange { .. } => "out of range",
            Self::Ignored { .. } => "ignored, stacked",
            Self::FoundNothing => "found nothing",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PressTrace {
    pub time_ms: f64,
    pub verdict: Verdict,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Event {
    pub time_ms: f64,

    pub object_index: usize,
    pub part: Part,
    pub result: Judgement,

    pub error_ms: Option<f64>,
    pub combo_after: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScoreState {
    pub combo: u32,
    pub max_combo: u32,
    pub counts: HitCounts,
}

impl ScoreState {
    pub fn accuracy(&self) -> f64 {
        self.counts.accuracy_std()
    }
}

#[derive(Debug, Clone)]
pub struct Judge {
    events: Vec<Event>,

    shakes: Vec<(usize, f64)>,

    trace: Vec<PressTrace>,

    clicks: Vec<Press>,

    states: Vec<ScoreState>,
}

impl Judge {
    pub fn run(timeline: &Timeline, cursor: &CursorTrack, ruleset: Ruleset) -> Self {
        let Heads {
            heads,
            shakes,
            trace,
            clicks,
        } = judge_heads(timeline, cursor, ruleset);
        let mut events = Vec::new();
        for (index, object) in timeline.objects.iter().enumerate() {
            build_events(
                timeline,
                cursor,
                index,
                object,
                heads[index],
                ruleset,
                &mut events,
            );
        }

        events.sort_by(|a, b| a.time_ms.total_cmp(&b.time_ms));

        let mut state = ScoreState::default();
        let mut states = Vec::with_capacity(events.len());
        for event in &mut events {
            accrue(&mut state, event);
            event.combo_after = state.combo;
            states.push(state);
        }

        Self {
            events,
            shakes,
            trace,
            clicks,
            states,
        }
    }

    pub fn shakes(&self) -> &[(usize, f64)] {
        &self.shakes
    }

    pub(crate) fn clicks(&self) -> &[Press] {
        &self.clicks
    }

    pub fn trace(&self) -> &[PressTrace] {
        &self.trace
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    pub fn state_at(&self, time_ms: f64) -> ScoreState {
        let i = self.events.partition_point(|e| e.time_ms <= time_ms);
        if i == 0 {
            ScoreState::default()
        } else {
            self.states[i - 1]
        }
    }

    pub fn final_state(&self) -> ScoreState {
        self.states.last().copied().unwrap_or_default()
    }

    pub fn state_up_to_object(&self, objects: usize) -> ScoreState {
        let mut state = ScoreState::default();
        for event in self.events.iter().filter(|e| e.object_index < objects) {
            accrue(&mut state, event);
        }
        state
    }

    pub fn events_for(&self, object_index: usize) -> impl Iterator<Item = &Event> {
        self.events
            .iter()
            .filter(move |e| e.object_index == object_index)
    }

    pub fn errors_ms(&self) -> impl Iterator<Item = (f64, f64)> + '_ {
        self.events
            .iter()
            .filter_map(|e| e.error_ms.map(|err| (e.time_ms, err)))
    }

    pub fn unstable_rate(&self, time_ms: f64) -> Option<f64> {
        let errors: Vec<f64> = self
            .errors_ms()
            .filter(|&(at, _)| at <= time_ms)
            .map(|(_, err)| err)
            .collect();
        if errors.len() < 2 {
            return None;
        }
        let mean = errors.iter().sum::<f64>() / errors.len() as f64;
        let variance = errors
            .iter()
            .map(|err| (err - mean) * (err - mean))
            .sum::<f64>()
            / errors.len() as f64;
        Some(variance.sqrt() * 10.0)
    }
}

fn accrue(state: &mut ScoreState, event: &Event) {
    if event.result.is_miss() {
        if event.part.breaks_combo() {
            state.combo = 0;
        }
    } else if event.part.adds_combo() {
        state.combo += 1;
        state.max_combo = state.max_combo.max(state.combo);
    }
    if event.part.counts_for_accuracy() {
        match event.result {
            Judgement::Great => state.counts.count_300 += 1,
            Judgement::Ok => state.counts.count_100 += 1,
            Judgement::Meh => state.counts.count_50 += 1,
            Judgement::Miss => state.counts.count_miss += 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Head {
    Hit { time_ms: f64, error_ms: f64 },

    Missed { at_ms: Option<f64> },
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Press {
    pub time_ms: f64,
    pub pos: Point,
}

const RELAX_LEAD_MS: f64 = 12.0;

fn relax_presses(
    frames: &[ReplayFrame],
    objects: &[TimedObject],
    window_50: f64,
    radius: f64,
    lazer: bool,
) -> Vec<Press> {
    let mut out = Vec::new();
    if frames.is_empty() {
        return out;
    }

    let mut at = 0usize;

    let mut lock_until = f64::NEG_INFINITY;
    for object in objects {
        if object.is_spinner() {
            continue;
        }

        let want = (object.start_ms - RELAX_LEAD_MS).max(lock_until);

        while at + 1 < frames.len() && f64::from(frames[at].time_ms as i32) < want {
            at += 1;
        }

        let mut when = at;
        let deadline = object.start_ms + window_50;
        while when < frames.len() {
            let t = f64::from(frames[when].time_ms as i32);
            if t > deadline {
                break;
            }
            let here = Point {
                x: f64::from(frames[when].x),
                y: f64::from(frames[when].y),
            };
            if here.distance_to(object.pos) <= radius {
                break;
            }
            when += 1;
        }
        let landed = when < frames.len() && f64::from(frames[when].time_ms as i32) <= deadline;
        lock_until = if landed || lazer {
            f64::NEG_INFINITY
        } else {
            deadline + 2.0
        };
        let frame = &frames[if landed { when } else { at }];
        let now = f64::from(frame.time_ms as i32);
        if now < want {
            continue;
        }
        let pos = Point {
            x: f64::from(frame.x),
            y: f64::from(frame.y),
        };

        if lazer && !landed {
            continue;
        }
        out.push(Press { time_ms: now, pos });
    }
    out
}

pub(crate) fn presses(frames: &[ReplayFrame]) -> Vec<Press> {
    let mut out = Vec::new();
    let left = |k: u8| k & (Keys::M1 | Keys::K1) != 0;
    let right = |k: u8| k & (Keys::M2 | Keys::K2) != 0;
    let mut previous = 0u8;
    for frame in frames {
        let held = frame.keys.0 & CLICK_KEYS;
        let rising =
            u8::from(left(held) && !left(previous)) + u8::from(right(held) && !right(previous));
        for _ in 0..rising {
            out.push(Press {
                time_ms: frame.time_ms as f64,
                pos: Point {
                    x: f64::from(frame.x),
                    y: f64::from(frame.y),
                },
            });
        }
        previous = held;
    }
    out
}

fn judge_heads(timeline: &Timeline, cursor: &CursorTrack, ruleset: Ruleset) -> Heads {
    let objects = &timeline.objects;
    let mut heads = vec![Head::Missed { at_ms: None }; objects.len()];
    let mut judged = vec![false; objects.len()];
    let mut shakes = Vec::new();
    let mut trace = Vec::new();

    let window = timeline.difficulty.hit_window_50();
    let radius = timeline.difficulty.circle_radius();
    let preempt = timeline.difficulty.preempt_ms();

    let mut first = 0usize;

    let mut first_live = 0usize;

    let made = ruleset.relax.then(|| {
        relax_presses(
            cursor.frames(),
            objects,
            window,
            radius,
            ruleset.client() == crate::ruleset::Client::Lazer,
        )
    });
    let clicks = made.unwrap_or_else(|| presses(cursor.frames()));
    for press in &clicks {
        for (index, object) in objects.iter().enumerate().skip(first) {
            if object.start_ms - preempt > press.time_ms {
                break;
            }
            if !judged[index] && past_it(object, press.time_ms, window) {
                judged[index] = true;
            }
        }
        while first < objects.len() && judged[first] {
            first += 1;
        }
        while first_live < objects.len() && objects[first_live].end_ms <= press.time_ms {
            first_live += 1;
        }
        if first >= objects.len() {
            trace.push(PressTrace {
                time_ms: press.time_ms,
                verdict: Verdict::FoundNothing,
            });
            continue;
        }

        let candidates = || {
            objects
                .iter()
                .enumerate()
                .skip(first)
                .take_while(|(_, object)| object.start_ms - preempt <= press.time_ms)
                .filter(|(index, object)| {
                    !judged[*index]
                        && !object.is_spinner()
                        && press.pos.distance_to(object.pos) <= radius
                        && (!ruleset.relax || (press.time_ms - object.start_ms).abs() <= window)
                })
        };
        let target = candidates().next().map(|(index, _)| index);
        let Some(target) = target else {
            trace.push(PressTrace {
                time_ms: press.time_ms,
                verdict: Verdict::FoundNothing,
            });
            continue;
        };

        if target > 0 && objects[target - 1].stack_height > 0 && !judged[target - 1] {
            trace.push(PressTrace {
                time_ms: press.time_ms,
                verdict: Verdict::Ignored { object: target },
            });
            continue;
        }

        let swallowed = ruleset.slider_swallows_notes_beneath()
            && objects
                .iter()
                .skip(first_live)
                .take(target.saturating_sub(first_live))
                .any(|object| {
                    object.is_slider()
                        && object.start_ms - preempt <= press.time_ms
                        && object.end_ms > press.time_ms
                        && press.pos.distance_to(object.pos) <= radius
                });

        let spun = ruleset.spinner_swallows_presses()
            && objects
                .iter()
                .skip(first_live)
                .take(target.saturating_sub(first_live))
                .any(|object| {
                    object.is_spinner()
                        && object.start_ms - preempt <= press.time_ms
                        && object.end_ms > press.time_ms
                });
        let swallowed = swallowed || spun;
        if swallowed {
            trace.push(PressTrace {
                time_ms: press.time_ms,
                verdict: Verdict::Ignored { object: target },
            });
            continue;
        }

        let behind = || {
            objects
                .iter()
                .enumerate()
                .skip(first.min(first_live))
                .take_while(|(index, _)| *index < target)
                .filter(|(_, object)| ruleset.can_block(object.is_spinner()))
        };
        let locked = if ruleset.blocker_is_the_last_one() {
            behind()
                .filter(|(_, object)| object.start_ms < objects[target].start_ms)
                .last()
                .filter(|(index, object)| {
                    !judged[*index]
                        && ruleset.blocks(
                            object.end_ms,
                            object.start_ms,
                            objects[target].start_ms,
                            press.time_ms,
                        )
                })
        } else {
            behind().find(|(index, object)| {
                (!judged[*index] || (object.is_slider() && press.time_ms <= object.end_ms))
                    && ruleset.blocks(
                        object.end_ms,
                        object.start_ms,
                        objects[target].start_ms,
                        press.time_ms,
                    )
            })
        }
        .map(|(index, _)| index);

        let object = &objects[target];
        let error_ms = press.time_ms - object.start_ms;
        if locked.is_some() || error_ms.abs() >= ruleset.hittable_range_ms() {
            if press.time_ms >= object.start_ms - preempt {
                shakes.push((target, press.time_ms));
            }
            trace.push(PressTrace {
                time_ms: press.time_ms,
                verdict: match locked {
                    Some(blocked_by) => Verdict::Refused {
                        object: target,
                        blocked_by,
                    },
                    None => Verdict::OutOfRange { object: target },
                },
            });
            continue;
        }

        if ruleset.writes_off_stranded_notes() {
            for index in first..target {
                if !judged[index] && !objects[index].is_spinner() {
                    judged[index] = true;
                    heads[index] = Head::Missed {
                        at_ms: Some(press.time_ms),
                    };
                }
            }
        }
        judged[target] = true;
        trace.push(PressTrace {
            time_ms: press.time_ms,
            verdict: if error_ms > -window {
                Verdict::Landed { object: target }
            } else {
                Verdict::TookItEarly { object: target }
            },
        });
        if error_ms > -window {
            heads[target] = Head::Hit {
                time_ms: press.time_ms,
                error_ms,
            };
        }
    }

    Heads {
        heads,
        shakes,
        trace,
        clicks,
    }
}

struct Heads {
    heads: Vec<Head>,
    shakes: Vec<(usize, f64)>,
    trace: Vec<PressTrace>,
    clicks: Vec<Press>,
}

fn past_it(object: &TimedObject, time_ms: f64, window_50: f64) -> bool {
    if object.is_spinner() || object.is_slider() {
        time_ms > object.end_ms
    } else {
        time_ms - 1.0 > object.start_ms + window_50
    }
}

fn build_events(
    timeline: &Timeline,
    cursor: &CursorTrack,
    index: usize,
    object: &TimedObject,
    head: Head,
    ruleset: Ruleset,
    out: &mut Vec<Event>,
) {
    let difficulty = &timeline.difficulty;

    match &object.kind {
        TimedKind::Circle => {
            let (time_ms, result, error_ms) = match head {
                Head::Hit { time_ms, error_ms } => (
                    time_ms,
                    window_judgement(error_ms, difficulty),
                    Some(error_ms),
                ),
                Head::Missed { at_ms } => (
                    at_ms.unwrap_or(object.start_ms + difficulty.hit_window_50()),
                    Judgement::Miss,
                    None,
                ),
            };
            out.push(Event {
                time_ms,
                object_index: index,
                part: Part::Circle,
                result,
                error_ms,
                combo_after: 0,
            });
        }

        TimedKind::Slider { .. } => {
            build_slider_events(timeline, cursor, index, object, head, ruleset, out)
        }

        TimedKind::Spinner => {
            let spin = Spin {
                rate: timeline.rate(),
                ..ruleset.spin()
            };
            let turns = spinner_spin_times(cursor, object.start_ms, object.end_ms, spin);
            let rotations = spinner_half_turns(cursor, object.start_ms, object.end_ms, spin);
            let required = required_half_turns(difficulty, object.duration_ms());

            let spare = spare_spins(difficulty, object.duration_ms()) as i64;
            for (turn, at) in turns.iter().enumerate() {
                let Some(part) = spinner_turn(turn as i64 + 1, required as i64, spare, ruleset)
                else {
                    continue;
                };
                out.push(Event {
                    time_ms: *at,
                    object_index: index,
                    part,
                    result: Judgement::Great,
                    error_ms: None,
                    combo_after: 0,
                });
            }
            out.push(Event {
                time_ms: object.end_ms,
                object_index: index,
                part: Part::Spinner,
                result: spinner_judgement(rotations, required, ruleset),
                error_ms: None,
                combo_after: 0,
            });
        }
    }
}

fn build_slider_events(
    timeline: &Timeline,
    cursor: &CursorTrack,
    index: usize,
    object: &TimedObject,
    head: Head,
    ruleset: Ruleset,
    out: &mut Vec<Event>,
) {
    let difficulty = &timeline.difficulty;

    let (head_time, head_error) = match head {
        Head::Hit { time_ms, error_ms } => (time_ms, Some(error_ms)),

        Head::Missed { at_ms } => (
            at_ms.unwrap_or(object.start_ms + difficulty.hit_window_50()),
            None,
        ),
    };
    let head_hit = matches!(head, Head::Hit { .. });

    let head_time_for_tracking = match head {
        Head::Hit { time_ms, .. } => Some(time_ms),
        _ => None,
    };

    out.push(Event {
        time_ms: head_time,
        object_index: index,
        part: Part::SliderHead,
        result: Judgement::from_flag(head_hit),
        error_ms: head_error,
        combo_after: 0,
    });

    let mut parts_total = 1u32;
    let mut parts_hit = u32::from(head_hit);

    let mut parts: Vec<(f64, Part)> = object
        .tick_times()
        .into_iter()
        .map(|t| (t, Part::SliderTick))
        .chain(
            object
                .repeat_times()
                .into_iter()
                .map(|t| (t, Part::SliderRepeat)),
        )
        .collect();
    parts.sort_by(|a, b| a.0.total_cmp(&b.0));
    parts.push((tail_check_ms(object), Part::SliderTail));

    for (time_ms, part, hit) in track_slider(
        cursor,
        object,
        difficulty,
        &parts,
        head_time_for_tracking,
        ruleset.slider_is_scored_by_its_head(),
        ruleset.relax,
    ) {
        parts_total += 1;
        parts_hit += u32::from(hit);
        out.push(Event {
            time_ms: if part == Part::SliderTail {
                object.end_ms
            } else {
                time_ms
            },
            object_index: index,
            part,
            result: Judgement::from_flag(hit),
            error_ms: None,
            combo_after: 0,
        });
    }

    let result = if ruleset.slider_verdict_from_head() {
        let from_head = match head {
            Head::Hit { error_ms, .. } => window_judgement(error_ms, difficulty),
            Head::Missed { .. } => Judgement::Miss,
        };
        if ruleset.slider_verdict_also_needs_its_pieces() {
            let took_head = matches!(head, Head::Hit { .. });
            let pieces = slider_judgement(parts_hit + u32::from(took_head), parts_total + 1);
            if took_head {
                score_v2_slider(pieces, from_head)
            } else {
                pieces
            }
        } else {
            from_head
        }
    } else {
        slider_judgement(parts_hit, parts_total)
    };

    let verdict_at = match head {
        Head::Hit { time_ms, .. } if ruleset.slider_verdict_from_head() => time_ms,
        _ => object.end_ms,
    };
    out.push(Event {
        time_ms: verdict_at,
        object_index: index,
        part: Part::Slider,
        result,
        error_ms: None,
        combo_after: 0,
    });
}

fn score_v2_slider(from_pieces: Judgement, from_head: Judgement) -> Judgement {
    use Judgement::{Great, Meh, Miss, Ok};
    let at_least_ok = |j: Judgement| matches!(j, Great | Ok);
    if from_pieces == Great && from_head == Great {
        Great
    } else if at_least_ok(from_pieces) && at_least_ok(from_head) {
        Ok
    } else if from_pieces != Miss && from_head != Miss {
        Meh
    } else {
        Miss
    }
}

pub(crate) fn window_judgement(
    error_ms: f64,
    difficulty: &dossier_beatmap::Difficulty,
) -> Judgement {
    let error = error_ms.abs();
    if error < difficulty.hit_window_300() {
        Judgement::Great
    } else if error < difficulty.hit_window_100() {
        Judgement::Ok
    } else if error < difficulty.hit_window_50() {
        Judgement::Meh
    } else {
        Judgement::Miss
    }
}

fn slider_judgement(hit: u32, total: u32) -> Judgement {
    if total == 0 || hit == total {
        Judgement::Great
    } else if hit * 2 >= total {
        Judgement::Ok
    } else if hit > 0 {
        Judgement::Meh
    } else {
        Judgement::Miss
    }
}

pub fn tail_check_ms(object: &TimedObject) -> f64 {
    let half_slide = object
        .slide_duration_ms()
        .map_or(0.0, |duration| object.start_ms + duration / 2.0);
    (object.end_ms - TAIL_LENIENCE_MS)
        .max(half_slide)
        .max(object.start_ms)
}

pub fn required_half_turns(difficulty: &dossier_beatmap::Difficulty, duration_ms: f64) -> f64 {
    (difficulty.half_spins_per_second() * duration_ms / 1000.0).floor()
}

const LAZER_BONUS_GAP: i64 = 2;

fn spinner_turn(turn: i64, required: i64, spare: i64, ruleset: Ruleset) -> Option<Part> {
    if ruleset.spinner_counts_half_turns() {
        let bonus_from = required + 3;
        return Some(if turn > bonus_from && (turn - bonus_from) % 2 == 0 {
            Part::SpinnerBonus
        } else if turn > 1 && turn % 2 == 0 {
            Part::SpinnerPoints
        } else {
            Part::SpinnerSpin
        });
    }
    if turn % 2 != 0 {
        return None;
    }
    let spun = turn / 2;
    let plain_until = required / 2 + LAZER_BONUS_GAP;
    if spun <= plain_until {
        Some(Part::SpinnerPoints)
    } else if spun <= plain_until + spare {
        Some(Part::SpinnerBonus)
    } else {
        None
    }
}

pub fn spare_spins(difficulty: &dossier_beatmap::Difficulty, duration_ms: f64) -> f64 {
    let full = (difficulty.full_score_spins_per_second() * duration_ms / 1000.0).floor();
    let required = (required_half_turns(difficulty, duration_ms) / 2.0).floor();
    (full - required - LAZER_BONUS_GAP as f64).max(0.0)
}

fn spinner_judgement(half_turns: f64, required: f64, ruleset: Ruleset) -> Judgement {
    if required <= 0.0 {
        return Judgement::Great;
    }
    if !ruleset.spinner_counts_half_turns() {
        let progress = half_turns / required;
        return if progress >= 1.0 {
            Judgement::Great
        } else if progress > 0.9 {
            Judgement::Ok
        } else if progress > 0.75 {
            Judgement::Meh
        } else {
            Judgement::Miss
        };
    }
    let spun = half_turns.floor();
    if spun < (required / 4.0).floor() {
        Judgement::Miss
    } else if spun > required {
        Judgement::Great
    } else if spun > required - 2.0 {
        Judgement::Ok
    } else {
        Judgement::Meh
    }
}

fn track_slider(
    cursor: &CursorTrack,
    object: &TimedObject,
    difficulty: &dossier_beatmap::Difficulty,
    parts: &[(f64, Part)],
    head_hit_ms: Option<f64>,
    tail_window: bool,
    relax: bool,
) -> Vec<(f64, Part, bool)> {
    let radius = difficulty.circle_radius();
    let follow = radius * FOLLOW_CIRCLE_SCALE;

    let mut sliding = false;
    let mut down_button = Side::NONE;
    let mut head_seeded = false;
    let head_press_ms = head_hit_ms;
    let mut slide_start = f64::INFINITY;
    let mut judged = 0usize;
    let mut out = Vec::with_capacity(parts.len());

    let frames: HashSet<i64> = cursor.frames().iter().map(|f| f.time_ms).collect();

    let mut tail_pending: Option<f64> = None;
    let mut tail_hit = false;

    let mut instants: Vec<f64> = {
        let mut v = Vec::new();
        let mut t = object.start_ms.ceil();
        while t <= object.end_ms + TAIL_LENIENCE_MS {
            v.push(t);
            t += 1.0;
        }
        v
    };
    instants.extend(parts.iter().map(|&(t, _)| t));
    instants.sort_by(f64::total_cmp);

    for now in instants {
        let head_landing = tail_window && head_hit_ms.is_some_and(|at| now >= at) && !sliding;

        if let Some(at) = head_press_ms {
            if !head_seeded && now >= at {
                head_seeded = true;
                if let Some(b) = cursor.buttons_at(at) {
                    down_button = if b.left_edge {
                        Side::LEFT
                    } else if b.right_edge {
                        Side::RIGHT
                    } else {
                        b.down
                    };
                }
            }
        }

        let acceptable = match cursor.buttons_at(now) {
            Some(buttons) => {
                let swap = buttons.down.any()
                    && !(buttons.last == Side::BOTH && buttons.last2 == buttons.down);
                let mut ok = false;
                if buttons.down.any() {
                    if down_button == Side::NONE || (buttons.down != Side::BOTH && swap) {
                        down_button = if buttons.left_edge {
                            Side::LEFT
                        } else if buttons.right_edge {
                            Side::RIGHT
                        } else {
                            buttons.down
                        };
                        ok = true;
                    } else if buttons.down.overlaps(down_button) {
                        ok = true;
                    }
                } else {
                    down_button = Side::NONE;
                }
                ok || swap || relax
            }
            None => relax,
        };
        let allowable = match (object.ball_at(now), cursor.sample(now)) {
            (Some(ball), Some(sample)) => {
                let needed = if sliding || head_landing {
                    follow
                } else {
                    radius
                };
                acceptable && sample.pos.distance_to(ball) <= needed
            }
            _ => false,
        };

        let on_a_frame = now.fract() == 0.0 && frames.contains(&(now as i64));
        if allowable && !sliding && on_a_frame {
            sliding = true;
            slide_start = now;
        }

        if let Some(&(time_ms, part)) = parts.get(judged) {
            if time_ms <= now {
                let landed = allowable && slide_start <= time_ms;
                if tail_window && part == Part::SliderTail {
                    tail_pending = Some(time_ms);
                    tail_hit = landed;
                } else {
                    out.push((time_ms, part, landed));
                }
                judged += 1;
            }
        }
        if let Some(at) = tail_pending {
            if now > at && now <= object.end_ms {
                tail_hit |= allowable && slide_start <= now;
            }
        }

        if !allowable {
            sliding = false;
        }
    }

    if let Some(at) = tail_pending {
        out.push((at, Part::SliderTail, tail_hit));
    }

    for &(time_ms, part) in &parts[judged.min(parts.len())..] {
        out.push((time_ms, part, false));
    }
    out
}

fn button_down(keys: dossier_replay::Keys, relax: bool) -> bool {
    relax || keys.is_pressed()
}

pub(crate) fn is_tracking(
    cursor: &CursorTrack,
    object: &TimedObject,
    time_ms: f64,
    radius: f64,
    relax: bool,
) -> bool {
    let Some(ball) = object.ball_at(time_ms) else {
        return false;
    };
    let Some(sample) = cursor.sample(time_ms) else {
        return false;
    };
    button_down(sample.keys, relax) && sample.pos.distance_to(ball) <= radius
}

pub fn spinner_half_turns(cursor: &CursorTrack, start_ms: f64, end_ms: f64, spin: Spin) -> f64 {
    spinner_sweep(cursor, start_ms, end_ms, spin).0
}

pub fn spinner_rpm(cursor: &CursorTrack, start_ms: f64, time_ms: f64, spin: Spin) -> f64 {
    const WINDOW_MS: f64 = 200.0;
    let from = (time_ms - WINDOW_MS).max(start_ms);
    let span = time_ms - from;
    if span < 1.0 {
        return 0.0;
    }
    spinner_half_turns(cursor, from, time_ms, spin) / 2.0 / span * 60_000.0
}

pub(crate) fn spinner_spin_times(cursor: &CursorTrack, start_ms: f64, end_ms: f64, spin: Spin) -> Vec<f64> {
    spinner_sweep(cursor, start_ms, end_ms, spin).1
}

fn spinner_sweep(
    cursor: &CursorTrack,
    start_ms: f64,
    end_ms: f64,
    spin: Spin,
) -> (f64, Vec<f64>) {
    let (turns, _, crossings) = spinner_sweep_signed(cursor, start_ms, end_ms, spin);
    (turns, crossings)
}

const SPUN_OUT_RADIANS_PER_MS: f64 = 0.03;

const SPIN_CEILING_RADIANS_PER_MS: f64 = 0.05;

const SPIN_BASE_ACCELERATION: f64 = 8e-5;

const SPIN_SHORT_SPINNER_MS: f64 = 5_000.0;

const SPIN_FRAME_MS: f64 = 50.0 / 3.0;

const SPIN_FRAME_LENIENCE_MS: f64 = 17.333_332_697_550_457;

#[derive(Debug, Clone, Copy, Default)]
pub struct Spin {
    pub spun_out: bool,
    pub relax: bool,
    pub smoothed: bool,
    pub rate: f64,
}

pub fn spin_acceleration(duration_ms: f64) -> f64 {
    SPIN_BASE_ACCELERATION + ((SPIN_SHORT_SPINNER_MS - duration_ms) / 1_000.0 / 2_000.0).max(0.0)
}

fn smoothed_sweep(cursor: &CursorTrack, start_ms: f64, end_ms: f64, spin: Spin) -> (f64, Vec<f64>) {
    let acceleration = spin_acceleration(end_ms - start_ms);
    let centre = Point::CENTRE;
    let mut turned = 0.0f64;
    let mut velocity = 0.0f64;
    let mut smoothed = SPIN_FRAME_MS;
    let mut idle = 0u32;
    let mut observed = 0.0f64;
    let mut previous: Option<(f64, f64)> = None;
    let mut crossings = Vec::new();

    for sample in cursor.frames() {
        let at = sample.time_ms as f64;
        if at < start_ms || at > end_ms {
            continue;
        }
        let (dx, dy) = (f64::from(sample.x) - centre.x, f64::from(sample.y) - centre.y);
        if dx.hypot(dy) < 1e-9 {
            continue;
        }
        let angle = dy.atan2(dx);
        let Some((was_at, before)) = previous else {
            previous = Some((at, angle));
            continue;
        };
        previous = Some((at, angle));

        let gap = (at - was_at).max(0.0);
        let held = spin.relax || sample.keys.is_pressed();
        let mut step = angle - before;
        while step > PI {
            step -= TAU;
        }
        while step < -PI {
            step += TAU;
        }

        let rate = if spin.rate > 0.0 { spin.rate } else { 1.0 };
        let decay = 0.999f64.powf(gap);
        smoothed = decay * smoothed + (1.0 - decay) * gap;

        if step == 0.0 {
            observed = if idle < 1 { observed / 3.0 } else { 0.0 };
            idle += 1;
        } else {
            idle = 0;
            if !held {
                step = 0.0;
            }
            observed = if step.abs() < PI {
                let over = if smoothed / rate > SPIN_FRAME_LENIENCE_MS {
                    gap / rate
                } else {
                    SPIN_FRAME_MS
                };
                if over > 0.0 {
                    step / over
                } else {
                    0.0
                }
            } else {
                0.0
            };
        }

        if spin.spun_out {
            velocity = SPUN_OUT_RADIANS_PER_MS;
        } else {
            let allowance = acceleration * gap / rate;
            velocity += (observed - velocity).clamp(-allowance, allowance);
        }
        velocity = velocity.clamp(-SPIN_CEILING_RADIANS_PER_MS, SPIN_CEILING_RADIANS_PER_MS);

        let before_turn = turned;
        turned += (velocity * gap).abs().min(PI) / PI;
        let mut crossed = before_turn.floor() + 1.0;
        while crossed <= turned {
            let share = if turned > before_turn {
                (crossed - before_turn) / (turned - before_turn)
            } else {
                0.0
            };
            crossings.push(was_at + (at - was_at) * share);
            crossed += 1.0;
        }
    }
    (turned, crossings)
}

fn spun_out_sweep(start_ms: f64, end_ms: f64) -> (f64, f64, Vec<f64>) {
    let swept = (end_ms - start_ms).max(0.0) * SPUN_OUT_RADIANS_PER_MS;
    let halves = swept / PI;
    let each = PI / SPUN_OUT_RADIANS_PER_MS;
    let turns = (1..=halves as i64)
        .map(|crossed| start_ms + crossed as f64 * each)
        .collect();
    (halves, swept / TAU, turns)
}

fn spinner_sweep_signed(
    cursor: &CursorTrack,
    start_ms: f64,
    end_ms: f64,
    spin: Spin,
) -> (f64, f64, Vec<f64>) {
    if spin.spun_out && !spin.smoothed {
        return spun_out_sweep(start_ms, end_ms);
    }
    let facing = raw_facing(cursor, start_ms, end_ms);
    let (turned, crossings) = if spin.smoothed {
        smoothed_sweep(cursor, start_ms, end_ms, spin)
    } else {
        counted_sweep(cursor, start_ms, end_ms, spin)
    };
    (turned, facing, crossings)
}

fn counted_sweep(cursor: &CursorTrack, start_ms: f64, end_ms: f64, spin: Spin) -> (f64, Vec<f64>) {
    let centre = Point::CENTRE;
    let mut accumulated = 0.0f64;
    let mut at_last = 0.0f64;
    let mut widest = 0.0f64;
    let mut completed = 0.0f64;
    let mut previous: Option<f64> = None;
    let mut turned = 0.0f64;
    let mut crossings = Vec::new();
    let mut was_at = start_ms;

    for sample in cursor.frames() {
        let at = sample.time_ms as f64;
        if at < start_ms || at > end_ms {
            continue;
        }
        let (dx, dy) = (f64::from(sample.x) - centre.x, f64::from(sample.y) - centre.y);
        if dx.hypot(dy) < 1e-9 {
            continue;
        }
        let angle = dy.atan2(dx);
        let Some(before) = previous else {
            previous = Some(angle);
            was_at = at;
            continue;
        };
        previous = Some(angle);

        let mut step = angle - before;
        while step > PI {
            step -= TAU;
        }
        while step < -PI {
            step += TAU;
        }
        if !(spin.relax || sample.keys.is_pressed()) {
            step = 0.0;
        }
        if step != 0.0 {
            accumulated += step;
            widest = widest.max((accumulated - at_last).abs());
            while widest >= TAU {
                let way = (accumulated - at_last).signum();
                completed += 1.0;
                at_last += way * TAU;
                widest = (accumulated - at_last).abs();
            }
        }

        let before_turn = turned;
        turned = (completed * TAU + widest) / PI;
        let mut crossed = before_turn.floor() + 1.0;
        while crossed <= turned {
            let share = if turned > before_turn {
                (crossed - before_turn) / (turned - before_turn)
            } else {
                0.0
            };
            crossings.push(was_at + (at - was_at) * share);
            crossed += 1.0;
        }
        was_at = at;
    }
    (turned, crossings)
}

pub fn spinner_facing(cursor: &CursorTrack, start_ms: f64, end_ms: f64, spin: Spin) -> f64 {
    spinner_sweep_signed(cursor, start_ms, end_ms, spin).1
}

fn raw_facing(cursor: &CursorTrack, start_ms: f64, end_ms: f64) -> f64 {
    let centre = Point::CENTRE;
    let mut facing = 0.0;
    let mut previous: Option<f64> = None;
    for sample in cursor.frames() {
        let at = sample.time_ms as f64;
        if at < start_ms || at > end_ms {
            continue;
        }
        let (dx, dy) = (f64::from(sample.x) - centre.x, f64::from(sample.y) - centre.y);
        if dx.hypot(dy) < 1e-9 {
            continue;
        }
        let angle = dy.atan2(dx);
        if let Some(before) = previous {
            let mut step = angle - before;
            while step > PI {
                step -= TAU;
            }
            while step < -PI {
                step += TAU;
            }
            facing += step;
        }
        previous = Some(angle);
    }
    facing / TAU
}
