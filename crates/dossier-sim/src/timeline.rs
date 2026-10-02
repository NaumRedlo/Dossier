use dossier_beatmap::{Flattening, Beatmap, Difficulty, HitObject, ObjectKind, Point, SliderPath};
use dossier_replay::{bits, Mods};

#[derive(Debug, Clone)]
pub struct TimedObject {
    pub index: usize,
    pub pos: Point,
    pub start_ms: f64,

    pub end_ms: f64,
    pub new_combo: bool,

    pub stack_height: i32,
    pub kind: TimedKind,
}

#[derive(Debug, Clone)]
pub enum TimedKind {
    Circle,
    Slider {
        path: SliderPath,
        slides: u32,

        slide_duration_ms: f64,

        tick_times_ms: Vec<f64>,

        turn_times_ms: Vec<f64>,
    },
    Spinner,
}

impl TimedObject {
    pub fn duration_ms(&self) -> f64 {
        self.end_ms - self.start_ms
    }

    pub fn is_slider(&self) -> bool {
        matches!(self.kind, TimedKind::Slider { .. })
    }

    pub fn is_spinner(&self) -> bool {
        matches!(self.kind, TimedKind::Spinner)
    }

    pub fn slide_duration_ms(&self) -> Option<f64> {
        match &self.kind {
            TimedKind::Slider {
                slide_duration_ms, ..
            } => Some(*slide_duration_ms),
            _ => None,
        }
    }

    pub fn ball_at(&self, time_ms: f64) -> Option<Point> {
        let TimedKind::Slider {
            path,
            slides,
            slide_duration_ms,
            ..
        } = &self.kind
        else {
            return None;
        };
        if time_ms < self.start_ms || time_ms > self.end_ms || *slide_duration_ms <= 0.0 {
            return None;
        }
        let progress = (time_ms - self.start_ms) / slide_duration_ms;
        path.position_at_slide(progress, *slides)
    }

    pub fn ball_on_whole_ms(&self, time_ms: f64) -> Option<Point> {
        let TimedKind::Slider {
            path,
            slides,
            slide_duration_ms,
            ..
        } = &self.kind
        else {
            return None;
        };
        if time_ms < self.start_ms || time_ms > self.end_ms || *slide_duration_ms <= 0.0 {
            return None;
        }
        path.position_on_whole_ms(self.start_ms, *slide_duration_ms, *slides, time_ms)
    }

    pub fn tick_times(&self) -> Vec<f64> {
        let TimedKind::Slider { tick_times_ms, .. } = &self.kind else {
            return Vec::new();
        };
        tick_times_ms.iter().map(|after| self.start_ms + after).collect()
    }

    pub(crate) fn translate(&mut self, dx: f64, dy: f64) {
        self.pos.x += dx;
        self.pos.y += dy;
        if let TimedKind::Slider { path, .. } = &mut self.kind {
            path.translate(dx, dy);
        }
    }

    pub fn repeat_times(&self) -> Vec<f64> {
        let TimedKind::Slider { turn_times_ms, .. } = &self.kind else {
            return Vec::new();
        };
        let turns = turn_times_ms.len().saturating_sub(1);
        turn_times_ms[..turns].iter().map(|after| self.start_ms + after).collect()
    }

    pub fn last_point_ms(&self) -> f64 {
        match &self.kind {
            TimedKind::Slider { turn_times_ms, .. } => {
                turn_times_ms.last().map_or(self.end_ms, |after| self.start_ms + after)
            }
            _ => self.end_ms,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Timeline {
    pub objects: Vec<TimedObject>,

    pub difficulty: Difficulty,
    pub mods: Mods,

    pub tuning: Tuning,

    pub breaks: Vec<(f64, f64)>,

    pub timing: dossier_beatmap::Timing,
}

impl Timeline {
    pub fn build(beatmap: &Beatmap, mods: Mods) -> Self {
        Self::tuned(beatmap, mods, Tuning::default())
    }

    pub fn rate(&self) -> f64 {
        self.tuning.rate.unwrap_or_else(|| self.mods.speed_multiplier())
    }

    pub fn tuned(beatmap: &Beatmap, mods: Mods, tuning: Tuning) -> Self {
        Self::for_client(beatmap, mods, tuning, crate::Client::Stable)
    }

    pub fn for_client(beatmap: &Beatmap, mods: Mods, tuning: Tuning, client: crate::Client) -> Self {
        let difficulty = tuning.stats(apply_mods(beatmap.difficulty.in_single_precision(), mods));
        let mirror = Reflect {
            across: mods.contains(bits::HARD_ROCK) || tuning.reflect.across,
            along: tuning.reflect.along,
        };
        let mut objects: Vec<TimedObject> = beatmap
            .objects
            .iter()
            .enumerate()
            .map(|(index, obj)| resolve(beatmap, &difficulty, index, obj, mirror, client))
            .collect();

        crate::stacking::apply(
            &mut objects,
            &difficulty,
            beatmap.stack_leniency,
            beatmap.format_version,
        );

        Self {
            objects,
            difficulty,
            mods,
            tuning,
            breaks: beatmap.breaks.clone(),
            timing: beatmap.timing.clone(),
        }
    }

    pub fn visible_at(&self, time_ms: f64) -> impl Iterator<Item = &TimedObject> {
        let preempt = self.difficulty.preempt_ms();
        self.objects
            .iter()
            .filter(move |o| time_ms >= o.start_ms - preempt && time_ms <= o.end_ms)
    }

    pub fn approach_progress(&self, object: &TimedObject, time_ms: f64) -> f64 {
        let preempt = self.difficulty.preempt_ms();
        if preempt <= 0.0 {
            return 1.0;
        }
        (time_ms - (object.start_ms - preempt)) / preempt
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Reflect {
    pub across: bool,
    pub along: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Tuning {
    pub rate: Option<f64>,
    pub reflect: Reflect,
    pub circle_size: Option<f64>,
    pub approach_rate: Option<f64>,
    pub overall_difficulty: Option<f64>,
    pub drain_rate: Option<f64>,
}

impl Tuning {
    #[must_use]
    pub fn of_replay(replay: &dossier_replay::Replay) -> Self {
        let mods = replay.lazer_mods();
        let stat = |acronym: &str, name: &str| -> Option<f64> {
            mods.iter()
                .find(|m| m.acronym == acronym)
                .and_then(|m| match m.settings.get(name) {
                    Some(dossier_replay::Setting::Number(n)) => Some(*n),
                    _ => None,
                })
        };
        let reflection = mods
            .iter()
            .find(|m| m.acronym == "MR")
            .map(|m| m.number("reflection", 0.0) as i64);
        Self {
            reflect: Reflect {
                across: matches!(reflection, Some(1 | 2)),
                along: matches!(reflection, Some(0 | 2)),
            },
            rate: ["DT", "NC", "HT", "DC"]
                .iter()
                .find_map(|m| stat(m, "speed_change")),
            circle_size: stat("DA", "circle_size"),
            approach_rate: stat("DA", "approach_rate"),
            overall_difficulty: stat("DA", "overall_difficulty"),
            drain_rate: stat("DA", "drain_rate"),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    #[must_use]
    pub fn stats(&self, difficulty: Difficulty) -> Difficulty {
        Difficulty {
            hp_drain: self.drain_rate.unwrap_or(difficulty.hp_drain),
            circle_size: self.circle_size.unwrap_or(difficulty.circle_size),
            overall_difficulty: self
                .overall_difficulty
                .unwrap_or(difficulty.overall_difficulty),
            approach_rate: self.approach_rate.unwrap_or(difficulty.approach_rate),
            ..difficulty
        }
    }
}

fn apply_mods(difficulty: Difficulty, mods: Mods) -> Difficulty {
    if mods.contains(bits::EASY) {
        difficulty.easy()
    } else if mods.contains(bits::HARD_ROCK) {
        difficulty.hard_rock()
    } else {
        difficulty
    }
}

fn resolve(
    beatmap: &Beatmap,
    difficulty: &Difficulty,
    index: usize,
    obj: &HitObject,
    mirror: Reflect,
    client: crate::Client,
) -> TimedObject {
    let flip = |p: Point| {
        let p = if mirror.across { p.mirrored() } else { p };
        if mirror.along {
            p.flipped()
        } else {
            p
        }
    };

    let (kind, end_ms) = match &obj.kind {
        ObjectKind::Circle => (TimedKind::Circle, obj.time_ms),
        ObjectKind::Spinner { end_time_ms } => (TimedKind::Spinner, *end_time_ms),
        ObjectKind::Slider(slider) => {
            let points: Vec<Point> = slider.points.iter().map(|p| flip(*p)).collect();
            let how = match client {
                crate::Client::Stable => Flattening::Stable { format_version: beatmap.format_version },
                crate::Client::Lazer => Flattening::Lazer,
            };
            let path = SliderPath::flattened(slider.curve_type, &points, Some(slider.length), how);
            let slides = slider.slides.max(1);
            let (slide_duration_ms, tick_times_ms, turn_times_ms) =
                time_slider(beatmap, difficulty, obj.time_ms, &path, slides, slider.length, client);
            (
                TimedKind::Slider {
                    path,
                    slides,
                    slide_duration_ms,
                    tick_times_ms,
                    turn_times_ms,
                },
                obj.time_ms + slide_duration_ms * f64::from(slides),
            )
        }
    };

    TimedObject {
        index,
        pos: flip(obj.pos),
        start_ms: obj.time_ms,
        end_ms,
        new_combo: obj.new_combo,

        stack_height: 0,
        kind,
    }
}

const MOST_TICKS: usize = 10_000;
const NO_TICK_THIS_CLOSE_TO_THE_END_MS: f64 = 10.0;
const TICKS_FOLLOW_VELOCITY_FROM_VERSION: u32 = 8;

fn time_slider(
    beatmap: &Beatmap,
    difficulty: &Difficulty,
    time_ms: f64,
    path: &SliderPath,
    slides: u32,
    stated_length: f64,
    client: crate::Client,
) -> (f64, Vec<f64>, Vec<f64>) {
    let nothing = (0.0, Vec::new(), vec![0.0; slides as usize]);
    let Some((beat_length, multiplier)) = beatmap.timing.slider_beat_at(time_ms) else {
        return nothing;
    };
    let rate = difficulty.slider_tick_rate.max(0.1);
    let scoring = 100.0 * difficulty.slider_multiplier / rate;
    let beat = beat_length * f64::from(multiplier);
    if !(beat > 0.0) || !(scoring > 0.0) || !path.length().is_finite() {
        return nothing;
    }
    let velocity = scoring * rate * (1000.0 / beat);
    let stable = client == crate::Client::Stable;
    let slide = if stable { path.duration_ms(velocity) } else { path.length() / velocity * 1000.0 };
    if !(slide > 0.0) {
        return nothing;
    }
    let evenly: Vec<f64> = (1..=slides).map(|turn| f64::from(turn) * slide).collect();

    let follows = !stable || beatmap.format_version >= TICKS_FOLLOW_VELOCITY_FROM_VERSION;
    let apart = if follows { scoring / f64::from(multiplier) } else { scoring };
    let apart = if stated_length > 0.0 { apart.min(stated_length) } else { apart };
    if !(apart > 0.0) || !apart.is_finite() {
        return (slide, Vec::new(), evenly);
    }
    let too_close = NO_TICK_THIS_CLOSE_TO_THE_END_MS / 1000.0 * velocity;
    let reached = |travelled: f64| f64::from(travelled as f32) / velocity * 1000.0;
    let mut ticks = Vec::new();

    if !stable {
        let long = path.length();
        for index in 0..slides {
            let begins = f64::from(index) * slide;
            let mut along = apart;
            while along <= long && along < long - too_close && ticks.len() < MOST_TICKS {
                let share = along / long;
                let share = if index % 2 == 1 { 1.0 - share } else { share };
                ticks.push(begins + share * slide);
                along += apart;
            }
        }
        ticks.sort_by(f64::total_cmp);
        return (slide, ticks, evenly);
    }

    let pieces = path.piece_lengths();
    let mut turns = Vec::with_capacity(slides as usize);
    let (mut travelled, mut carried) = (0.0f64, 0.0f64);
    for index in 0..slides {
        let mut left = path.length();
        let mut stopped = false;
        let forward = index % 2 == 0;
        for at in 0..pieces.len() {
            carried += if forward { pieces[at] } else { pieces[pieces.len() - 1 - at] };
            while carried >= apart && !stopped {
                travelled += apart;
                carried -= apart;
                left -= apart;
                stopped = left <= too_close || ticks.len() >= MOST_TICKS;
                if stopped {
                    break;
                }
                ticks.push(reached(travelled));
            }
        }
        travelled += carried;
        turns.push(reached(travelled));
        if stopped {
            carried = 0.0;
        } else {
            travelled -= apart - carried;
            carried = apart - carried;
        }
    }
    (slide, ticks, turns)
}
