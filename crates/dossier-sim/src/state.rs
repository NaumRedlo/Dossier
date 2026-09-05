use dossier_beatmap::{Beatmap, Difficulty, Point};
use dossier_replay::{HitCounts, Mods, Replay};

use crate::cursor::{Cursor, CursorTrack};
use crate::judge::{Event, Judge, Judgement, Part, ScoreState, Verdict};
use crate::ruleset::Ruleset;
use crate::timeline::{TimedObject, Timeline};

#[derive(Debug, Clone, Copy)]
pub struct ActiveObject<'a> {
    pub object: &'a TimedObject,

    pub approach: f64,

    pub ball: Option<Point>,
}

#[derive(Debug, Clone)]
pub struct Snapshot<'a> {
    pub time_ms: f64,

    pub cursor: Option<Cursor>,
    pub objects: Vec<ActiveObject<'a>>,

    pub score: Option<ScoreState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Verification {
    pub ours: HitCounts,
    pub theirs: HitCounts,
    pub our_max_combo: u32,
    pub their_max_combo: u32,

    pub objects: usize,

    pub judged: usize,
}

impl Verification {
    pub fn finished(&self) -> bool {
        self.judged >= self.objects
    }

    pub fn counts_match(&self) -> bool {
        let ours = self.ours;
        let theirs = self.theirs;
        ours.count_300 == theirs.count_300
            && ours.count_100 == theirs.count_100
            && ours.count_50 == theirs.count_50
            && ours.count_miss == theirs.count_miss
    }

    pub fn combo_matches(&self) -> bool {
        self.our_max_combo == self.their_max_combo
    }

    pub fn is_exact(&self) -> bool {
        self.counts_match() && self.combo_matches()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MissContext {
    pub object_index: usize,
    pub kind: &'static str,
    pub time_ms: f64,

    pub press_dt_ms: Option<f64>,

    pub press_distance_px: Option<f64>,

    pub radius_px: f64,

    pub spin_rotations: Option<f64>,
    pub spin_required: Option<f64>,
}

impl MissContext {
    pub fn looks_like_a_geometry_error(&self) -> bool {
        matches!(
            (self.press_dt_ms, self.press_distance_px),
            (Some(dt), Some(distance))
                if dt.abs() <= 100.0 && distance > self.radius_px && distance < self.radius_px * 2.0
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PressDetail {
    pub time_ms: f64,
    pub verdict: Verdict,

    pub object_index: Option<usize>,
    pub object_ms: Option<f64>,

    pub error_ms: Option<f64>,

    pub distance_px: Option<f64>,
    pub radius_px: f64,

    pub blocked_by: Option<usize>,

    pub nearly: Option<NearMiss>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NearMiss {
    pub index: usize,

    pub error_ms: f64,
    pub distance_px: f64,
}

const NEAR_PRESS_WINDOW_MS: f64 = 400.0;

fn kind_name(object: &TimedObject) -> &'static str {
    if object.is_spinner() {
        "spinner"
    } else if object.is_slider() {
        "slider"
    } else {
        "circle"
    }
}

fn nearest_press(presses: &[crate::judge::Press], time_ms: f64) -> Option<&crate::judge::Press> {
    presses
        .iter()
        .filter(|p| (p.time_ms - time_ms).abs() <= NEAR_PRESS_WINDOW_MS)
        .min_by(|a, b| {
            (a.time_ms - time_ms)
                .abs()
                .total_cmp(&(b.time_ms - time_ms).abs())
        })
}

#[derive(Debug, Clone)]
pub struct GameState {
    timeline: Timeline,
    cursor: CursorTrack,
    judge: Option<Judge>,

    relax: bool,

    lazer: bool,

    played: usize,

    ending: Option<PlayEnd>,

    score: Option<crate::ScoreTrack>,

    modelled: Option<crate::HealthTrack>,

    health: Vec<(f64, f32)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayEnd {
    pub time_ms: f64,

    pub score: ScoreState,
}

fn objects_played(replay: &Replay, objects: usize) -> usize {
    match replay.hits.total_hits() as usize {
        0 => objects,
        judged => judged.min(objects),
    }
}

fn play_end(
    judge: &Judge,
    played: usize,
    objects: usize,
    bar_emptied_ms: Option<f64>,
) -> Option<PlayEnd> {
    if played >= objects {
        return None;
    }
    let last_judged = judge
        .events()
        .iter()
        .filter(|event| event.object_index < played)
        .map(|event| event.time_ms)
        .fold(f64::NEG_INFINITY, f64::max);
    if !last_judged.is_finite() {
        return None;
    }
    let time_ms = bar_emptied_ms.map_or(last_judged, |at| at.min(last_judged));
    Some(PlayEnd {
        time_ms,

        score: judge.state_up_to_object(played),
    })
}

const LEAD_IN_MS: f64 = 800.0;

impl GameState {
    pub fn new(beatmap: &Beatmap, replay: &Replay) -> Self {
        Self::tuned(
            beatmap,
            replay,
            replay.mods,
            crate::Tuning::of_replay(replay),
        )
    }

    pub fn with_mods(beatmap: &Beatmap, replay: &Replay, mods: Mods) -> Self {
        Self::tuned(beatmap, replay, mods, crate::Tuning::default())
    }

    pub fn tuned(beatmap: &Beatmap, replay: &Replay, mods: Mods, tuning: crate::Tuning) -> Self {
        let timeline = Timeline::tuned(beatmap, mods, tuning);
        let cursor = CursorTrack::new(replay.frames.clone());

        let judge = Judge::run(&timeline, &cursor, Ruleset::of_replay(replay));
        let mut health = dossier_replay::life_points(&replay.life_bar);
        let played = objects_played(replay, timeline.objects.len());
        let ruleset = Ruleset::of_replay(replay);

        let recorded_multiplier = replay.score_info.as_ref().and_then(|info| {
            info.total_score_without_mods
                .filter(|before| *before > 0)
                .map(|before| f64::from(replay.score) / before as f64)
        });
        let score = crate::ScoreTrack::build_for(
            &judge,
            beatmap,
            mods,
            replay.lazer_mods(),
            recorded_multiplier,
            played,
            ruleset,
        );

        let modelled = crate::HealthTrack::build(
            &judge,
            &timeline,
            &beatmap.breaks,
            beatmap.format_version,
            mods,
            ruleset,
        );
        let ending = play_end(&judge, played, timeline.objects.len(), modelled.failed_at());

        if let Some(end) = ending {
            health.retain(|&(at, _)| at < end.time_ms);
            health.push((end.time_ms, 0.0));
        }
        let modelled = Some(modelled);
        Self {
            timeline,
            cursor,
            judge: Some(judge),
            relax: ruleset.relax,
            lazer: ruleset.client() == crate::ruleset::Client::Lazer,
            played,
            ending,
            health,
            modelled,
            score: Some(score),
        }
    }

    pub fn from_beatmap(beatmap: &Beatmap, mods: Mods) -> Self {
        let timeline = Timeline::build(beatmap, mods);
        let played = timeline.objects.len();
        Self {
            timeline,
            cursor: CursorTrack::new(Vec::new()),
            judge: None,

            relax: false,
            lazer: false,
            played,
            ending: None,
            health: Vec::new(),
            modelled: None,
            score: None,
        }
    }

    pub fn objects_played(&self) -> usize {
        self.played
    }

    pub fn ending(&self) -> Option<PlayEnd> {
        self.ending
    }

    pub fn score_track(&self) -> Option<&crate::ScoreTrack> {
        self.score.as_ref()
    }

    pub fn score_at(&self, time_ms: f64) -> Option<u64> {
        self.score.as_ref().map(|track| track.at(time_ms))
    }

    pub fn health_at(&self, time_ms: f64) -> Option<f32> {
        self.modelled.as_ref().map(|track| track.at(time_ms))
    }

    pub fn recorded_health(&self) -> &[(f64, f32)] {
        &self.health
    }

    #[allow(dead_code)]
    fn recorded_health_at(&self, time_ms: f64) -> Option<f32> {
        if self.health.is_empty() {
            return None;
        }
        let i = self.health.partition_point(|(t, _)| *t <= time_ms);
        if i == 0 {
            return Some(self.health[0].1);
        }
        let (t0, v0) = self.health[i - 1];
        let Some(&(t1, v1)) = self.health.get(i) else {
            return Some(v0);
        };
        let span = t1 - t0;
        if span <= 0.0 {
            return Some(v1);
        }
        let f = ((time_ms - t0) / span).clamp(0.0, 1.0) as f32;
        Some(v0 + (v1 - v0) * f)
    }

    pub fn in_break(&self, time_ms: f64) -> bool {
        self.timeline
            .breaks
            .iter()
            .any(|&(from, to)| time_ms >= from && time_ms <= to)
    }

    pub fn press_detail(&self) -> Vec<PressDetail> {
        let Some(judge) = &self.judge else {
            return Vec::new();
        };

        let presses = judge.clicks();
        let radius_px = self.timeline.difficulty.circle_radius();

        judge
            .trace()
            .iter()
            .zip(presses)
            .map(|(entry, press)| {
                let object = entry
                    .verdict
                    .object()
                    .and_then(|index| self.timeline.objects.get(index));
                PressDetail {
                    time_ms: entry.time_ms,
                    verdict: entry.verdict,
                    object_index: entry.verdict.object(),
                    object_ms: object.map(|o| o.start_ms),
                    error_ms: object.map(|o| entry.time_ms - o.start_ms),
                    distance_px: object.map(|o| {
                        let (dx, dy) = (press.pos.x - o.pos.x, press.pos.y - o.pos.y);
                        (dx * dx + dy * dy).sqrt()
                    }),
                    radius_px,
                    blocked_by: match entry.verdict {
                        Verdict::Refused { blocked_by, .. } => Some(blocked_by),
                        _ => None,
                    },

                    nearly: object
                        .is_none()
                        .then(|| {
                            self.timeline
                                .objects
                                .iter()
                                .enumerate()
                                .filter(|(index, _)| {
                                    judge
                                        .events_for(*index)
                                        .find(|e| e.part.counts_for_accuracy())
                                        .is_some_and(|e| e.result == Judgement::Miss)
                                })
                                .filter(|(_, o)| {
                                    (entry.time_ms - o.start_ms).abs() <= NEAR_PRESS_WINDOW_MS
                                })
                                .min_by(|(_, a), (_, b)| {
                                    (entry.time_ms - a.start_ms)
                                        .abs()
                                        .total_cmp(&(entry.time_ms - b.start_ms).abs())
                                })
                                .map(|(index, o)| {
                                    let (dx, dy) = (press.pos.x - o.pos.x, press.pos.y - o.pos.y);
                                    NearMiss {
                                        index,
                                        error_ms: entry.time_ms - o.start_ms,
                                        distance_px: (dx * dx + dy * dy).sqrt(),
                                    }
                                })
                        })
                        .flatten(),
                }
            })
            .collect()
    }

    fn played_events<'a>(&'a self, judge: &'a Judge) -> impl Iterator<Item = &'a Event> {
        judge
            .events()
            .iter()
            .filter(move |event| event.object_index < self.played)
    }

    pub fn timeline(&self) -> &Timeline {
        &self.timeline
    }

    pub fn is_lazer(&self) -> bool {
        self.lazer
    }

    pub fn cursor_track(&self) -> &CursorTrack {
        &self.cursor
    }

    pub fn judge(&self) -> Option<&Judge> {
        self.judge.as_ref()
    }

    pub fn mods(&self) -> Mods {
        self.timeline.mods
    }

    pub fn difficulty(&self) -> &Difficulty {
        &self.timeline.difficulty
    }

    pub fn playback_rate(&self) -> f64 {
        self.timeline
            .tuning
            .rate
            .unwrap_or_else(|| self.timeline.mods.speed_multiplier())
    }

    pub fn update(&self, time_ms: f64) -> Snapshot<'_> {
        let objects = self
            .timeline
            .visible_at(time_ms)
            .map(|object| ActiveObject {
                object,
                approach: self.timeline.approach_progress(object, time_ms),
                ball: object.ball_at(time_ms),
            })
            .collect();

        let score = self.judge.as_ref().map(|judge| match self.ending {
            Some(end) if time_ms >= end.time_ms => end.score,
            _ => judge.state_at(time_ms),
        });

        Snapshot {
            time_ms,
            cursor: self.cursor.sample(time_ms),
            objects,
            score,
        }
    }

    pub fn explain_misses(&self) -> Vec<MissContext> {
        let Some(judge) = &self.judge else {
            return Vec::new();
        };
        let presses = crate::judge::presses(self.cursor.frames());
        let radius = self.timeline.difficulty.circle_radius();

        self.played_events(judge)
            .filter(|e| e.part.counts_for_accuracy() && e.result.is_miss())
            .map(|event| {
                let object = &self.timeline.objects[event.object_index];
                let spinning = object.is_spinner();
                let nearest = if spinning {
                    None
                } else {
                    nearest_press(&presses, object.start_ms)
                };
                MissContext {
                    object_index: event.object_index,
                    kind: kind_name(object),
                    time_ms: object.start_ms,
                    press_dt_ms: nearest.map(|p| p.time_ms - object.start_ms),
                    press_distance_px: nearest.map(|p| p.pos.distance_to(object.pos)),
                    radius_px: radius,
                    spin_rotations: spinning.then(|| {
                        crate::judge::spinner_rotations(
                            &self.cursor,
                            object.start_ms,
                            object.end_ms,
                        )
                    }),
                    spin_required: spinning.then(|| {
                        crate::judge::required_spins(
                            &self.timeline.difficulty,
                            object.duration_ms(),
                        )
                    }),
                }
            })
            .collect()
    }

    pub fn press_count(&self) -> usize {
        crate::judge::presses(self.cursor.frames()).len()
    }

    pub fn max_possible_combo(&self) -> u32 {
        self.timeline
            .objects
            .iter()
            .map(|object| {
                if object.is_slider() {
                    2 + object.tick_times().len() as u32 + object.repeat_times().len() as u32
                } else {
                    1
                }
            })
            .sum()
    }

    pub fn lenient_tails(&self) -> usize {
        let radius = self.timeline.difficulty.circle_radius() * crate::judge::FOLLOW_CIRCLE_SCALE;
        self.timeline
            .objects
            .iter()
            .filter(|object| object.is_slider())
            .filter(|object| {
                let check = crate::judge::tail_check_ms(object);
                crate::judge::is_tracking(&self.cursor, object, check, radius, self.relax)
                    && !crate::judge::is_tracking(
                        &self.cursor,
                        object,
                        object.end_ms,
                        radius,
                        self.relax,
                    )
            })
            .count()
    }

    pub fn tails_near_the_rim(&self) -> usize {
        let inner = self.timeline.difficulty.circle_radius() * 2.0;
        let outer = self.timeline.difficulty.circle_radius() * crate::judge::FOLLOW_CIRCLE_SCALE;
        self.timeline
            .objects
            .iter()
            .filter(|object| object.is_slider())
            .filter(|object| {
                let check = crate::judge::tail_check_ms(object);
                crate::judge::is_tracking(&self.cursor, object, check, outer, self.relax)
                    && !crate::judge::is_tracking(&self.cursor, object, check, inner, self.relax)
            })
            .count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Suspect {
    pub object_index: usize,
    pub kind: &'static str,
    pub time_ms: f64,

    pub ours: Option<Judgement>,
    pub press_dt_ms: Option<f64>,
    pub press_distance_px: Option<f64>,
    pub radius_px: f64,
}

const REFUSAL_RUN: usize = 4;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PressSummary {
    pub landed: usize,
    pub took_a_note_early: usize,
    pub refused: usize,
    pub out_of_range: usize,
    pub ignored: usize,
    pub found_nothing: usize,

    pub refusal_runs: Vec<(f64, usize)>,
}

impl PressSummary {
    pub fn total(&self) -> usize {
        self.landed
            + self.took_a_note_early
            + self.refused
            + self.out_of_range
            + self.ignored
            + self.found_nothing
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComboChain {
    pub length: u32,

    pub ended_at_ms: f64,
    pub object_index: usize,

    pub part: Option<Part>,
}

impl GameState {
    pub fn combo_chains(&self) -> Vec<ComboChain> {
        let Some(judge) = &self.judge else {
            return Vec::new();
        };
        let mut chains = Vec::new();
        let mut length = 0;
        for event in self.played_events(judge) {
            if event.result == Judgement::Miss {
                if event.part.breaks_combo() {
                    chains.push(ComboChain {
                        length,
                        ended_at_ms: event.time_ms,
                        object_index: event.object_index,
                        part: Some(event.part),
                    });
                    length = 0;
                }
            } else if event.part.adds_combo() {
                length += 1;
            }
        }

        if length > 0 {
            chains.push(ComboChain {
                length,
                ended_at_ms: f64::INFINITY,
                object_index: usize::MAX,
                part: None,
            });
        }
        chains.sort_by_key(|chain| std::cmp::Reverse(chain.length));
        chains
    }

    pub fn combo_break_suspects(&self, their_max: u32) -> Vec<Suspect> {
        let Some(judge) = &self.judge else {
            return Vec::new();
        };
        let Some(longest) = self
            .combo_chains()
            .into_iter()
            .find(|c| c.length > their_max)
        else {
            return Vec::new();
        };

        let mut run: Vec<(usize, f64)> = Vec::new();
        let mut longest_run = Vec::new();
        for event in self.played_events(judge) {
            if event.result == Judgement::Miss && event.part.breaks_combo() {
                if event.object_index == longest.object_index {
                    longest_run = std::mem::take(&mut run);
                    break;
                }
                run.clear();
            } else if event.part.adds_combo() {
                run.push((event.object_index, event.time_ms));
            }
        }

        if longest.object_index == usize::MAX {
            longest_run = run;
        }

        let presses = crate::judge::presses(self.cursor.frames());
        let radius = self.timeline.difficulty.circle_radius();
        let mut wanted = [longest.length - their_max, their_max];
        wanted.sort_unstable();
        wanted
            .iter()
            .filter_map(|&at| longest_run.get(at.saturating_sub(1) as usize).copied())
            .map(|(index, _)| {
                let object = &self.timeline.objects[index];
                let nearest = nearest_press(&presses, object.start_ms);
                Suspect {
                    object_index: index,
                    kind: kind_name(object),
                    time_ms: object.start_ms,
                    ours: judge
                        .events_for(index)
                        .find(|e| e.part.counts_for_accuracy())
                        .map(|e| e.result),
                    press_dt_ms: nearest.map(|p| p.time_ms - object.start_ms),
                    press_distance_px: nearest.map(|p| p.pos.distance_to(object.pos)),
                    radius_px: radius,
                }
            })
            .collect()
    }

    pub fn press_verdicts(&self) -> PressSummary {
        let Some(judge) = &self.judge else {
            return PressSummary::default();
        };
        let mut summary = PressSummary::default();
        let mut run: Option<(f64, usize)> = None;

        for entry in judge.trace() {
            match entry.verdict {
                Verdict::Landed { .. } => summary.landed += 1,
                Verdict::TookItEarly { .. } => summary.took_a_note_early += 1,
                Verdict::Refused { .. } => summary.refused += 1,
                Verdict::OutOfRange { .. } => summary.out_of_range += 1,
                Verdict::Ignored { .. } => summary.ignored += 1,
                Verdict::FoundNothing => summary.found_nothing += 1,
            }

            match (&mut run, entry.verdict) {
                (None, Verdict::Refused { .. }) => run = Some((entry.time_ms, 1)),
                (Some((_, count)), Verdict::Refused { .. }) => *count += 1,
                (Some((at, count)), _) => {
                    if *count >= REFUSAL_RUN {
                        summary.refusal_runs.push((*at, *count));
                    }
                    run = None;
                }
                (None, _) => {}
            }
        }
        if let Some((at, count)) = run {
            if count >= REFUSAL_RUN {
                summary.refusal_runs.push((at, count));
            }
        }
        summary
    }

    pub fn verify(&self, replay: &Replay) -> Option<Verification> {
        let judge = self.judge.as_ref()?;
        let objects = self.timeline.objects.len();
        let judged = self.played.min(objects);
        let state = if judged < objects {
            judge.state_up_to_object(judged)
        } else {
            judge.final_state()
        };
        Some(Verification {
            ours: state.counts,
            theirs: replay.hits,
            our_max_combo: state.max_combo,
            their_max_combo: u32::from(replay.max_combo),
            objects,
            judged,
        })
    }

    pub fn span_ms(&self) -> (f64, f64) {
        let preempt = self.timeline.difficulty.preempt_ms();
        let map = match (self.timeline.objects.first(), self.timeline.objects.last()) {
            (Some(first), Some(last)) => (first.start_ms - preempt, last.end_ms),
            _ => (0.0, 0.0),
        };

        let (from, to) = match self.cursor.span_ms() {
            Some((_, cursor_to)) => (map.0 - LEAD_IN_MS, map.1.max(cursor_to)),
            None => (map.0 - LEAD_IN_MS, map.1),
        };
        match self.ending {
            Some(end) => (from, to.min(end.time_ms)),
            None => (from, to),
        }
    }
}
