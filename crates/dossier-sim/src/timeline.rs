use dossier_beatmap::{Beatmap, Difficulty, HitObject, ObjectKind, Point, SliderPath};
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

        tick_offsets_ms: Vec<f64>,
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

    pub fn tick_times(&self) -> Vec<f64> {
        let TimedKind::Slider {
            slides,
            slide_duration_ms,
            tick_offsets_ms,
            ..
        } = &self.kind
        else {
            return Vec::new();
        };

        let mut times = Vec::with_capacity(tick_offsets_ms.len() * *slides as usize);
        for slide in 0..*slides {
            let base = self.start_ms + f64::from(slide) * slide_duration_ms;
            if slide % 2 == 0 {
                times.extend(tick_offsets_ms.iter().map(|o| base + o));
            } else {
                times.extend(
                    tick_offsets_ms
                        .iter()
                        .rev()
                        .map(|o| base + slide_duration_ms - o),
                );
            }
        }
        times
    }

    pub(crate) fn translate(&mut self, dx: f64, dy: f64) {
        self.pos.x += dx;
        self.pos.y += dy;
        if let TimedKind::Slider { path, .. } = &mut self.kind {
            path.translate(dx, dy);
        }
    }

    pub fn repeat_times(&self) -> Vec<f64> {
        let TimedKind::Slider {
            slides,
            slide_duration_ms,
            ..
        } = &self.kind
        else {
            return Vec::new();
        };
        (1..*slides)
            .map(|s| self.start_ms + f64::from(s) * slide_duration_ms)
            .collect()
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
        let difficulty = tuning.stats(apply_mods(beatmap.difficulty, mods));
        let mirror = Reflect {
            across: mods.contains(bits::HARD_ROCK) || tuning.reflect.across,
            along: tuning.reflect.along,
        };
        let mut objects: Vec<TimedObject> = beatmap
            .objects
            .iter()
            .enumerate()
            .map(|(index, obj)| resolve(beatmap, &difficulty, index, obj, mirror))
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
            let path = SliderPath::new(slider.curve_type, &points, Some(slider.length));
            let slides = slider.slides.max(1);
            let slide_duration_ms = slide_duration(beatmap, difficulty, obj.time_ms, path.length());
            let tick_offsets_ms = tick_offsets(beatmap, difficulty, obj.time_ms, slide_duration_ms);
            (
                TimedKind::Slider {
                    path,
                    slides,
                    slide_duration_ms,
                    tick_offsets_ms,
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

fn slide_duration(beatmap: &Beatmap, difficulty: &Difficulty, time_ms: f64, length: f64) -> f64 {
    let beat_length = beatmap
        .timing
        .timing_point_at(time_ms)
        .map_or(0.0, |p| p.beat_length);
    let pixels_per_beat =
        difficulty.slider_multiplier * 100.0 * beatmap.timing.velocity_at(time_ms);

    if beat_length <= 0.0 || pixels_per_beat <= 0.0 || !length.is_finite() {
        return 0.0;
    }
    length / pixels_per_beat * beat_length
}

fn tick_offsets(
    beatmap: &Beatmap,
    difficulty: &Difficulty,
    time_ms: f64,
    slide_duration_ms: f64,
) -> Vec<f64> {
    let beat_length = beatmap
        .timing
        .timing_point_at(time_ms)
        .map_or(0.0, |p| p.beat_length);
    let spacing = beat_length / difficulty.slider_tick_rate.max(0.1);

    if spacing <= 0.0 || !spacing.is_finite() || slide_duration_ms <= 0.0 {
        return Vec::new();
    }

    let limit = slide_duration_ms - spacing / 8.0;
    let mut offsets = Vec::new();
    let mut t = spacing;
    while t < limit {
        offsets.push(t);
        t += spacing;

        if offsets.len() >= 10_000 {
            break;
        }
    }
    offsets
}
