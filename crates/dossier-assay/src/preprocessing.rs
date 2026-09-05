use dossier_beatmap::{Beatmap, Point};
use dossier_replay::Mods;
use dossier_sim::{TimedKind, TimedObject, Timeline};

use crate::slider::{Nested, NestedObject};

pub const NORMALISED_RADIUS: f64 = 50.0;

pub const NORMALISED_DIAMETER: f64 = NORMALISED_RADIUS * 2.0;

pub const MIN_DELTA_TIME: f64 = 25.0;

const ASSUMED_SLIDER_RADIUS: f64 = NORMALISED_RADIUS * 1.8;
const MAXIMUM_SLIDER_RADIUS: f64 = NORMALISED_RADIUS * 2.4;

#[derive(Debug, Clone)]
pub struct DiffObject {
    pub index: usize,
    pub start_time: f64,
    pub end_time: f64,

    pub delta_time: f64,

    pub adjusted_delta_time: f64,

    pub last_object_end_delta_time: f64,

    pub jump_distance: f64,

    pub lazy_jump_distance: f64,

    pub minimum_jump_distance: f64,
    pub minimum_jump_time: f64,

    pub travel_distance: f64,
    pub travel_time: f64,

    pub angle: Option<f64>,

    pub normalised_vector_angle: Option<f64>,

    pub lazy_end_position: Option<Point>,
    pub lazy_travel_distance: f64,
    pub lazy_travel_time: f64,

    pub hit_window_great: f64,
    pub is_slider: bool,
    pub is_spinner: bool,

    pub pos: Point,
    pub radius: f64,

    pub preempt: f64,

    pub raw_start_time: f64,
    pub raw_preempt: f64,

    pub raw_time_fade_in: f64,

    pub end_pos: Point,

    pub repeat_count: u32,
}

pub fn great_hit_window(overall_difficulty: f64) -> f64 {
    dossier_beatmap::difficulty_range(overall_difficulty, 80.0, 50.0, 20.0).floor() - 0.5
}

const PREEMPT_MIN: f64 = 450.0;

const HIDDEN_FADE_OUT_DURATION_MULTIPLIER: f64 = 0.3;
const HIDDEN_FADE_IN_DURATION_MULTIPLIER: f64 = 0.4;

impl DiffObject {
    pub fn opacity_at(&self, time: f64, hidden: bool) -> f64 {
        if time > self.raw_start_time {
            return 0.0;
        }
        let fade_in_start = self.raw_start_time - self.raw_preempt;

        let fade_in_duration = 400.0 * (self.raw_preempt / PREEMPT_MIN).min(1.0);
        let faded_in = ((time - fade_in_start) / fade_in_duration).clamp(0.0, 1.0);
        if !hidden {
            return faded_in;
        }
        let fade_out_start = fade_in_start + self.raw_time_fade_in;
        let fade_out_duration = self.raw_preempt * HIDDEN_FADE_OUT_DURATION_MULTIPLIER;
        faded_in.min(1.0 - ((time - fade_out_start) / fade_out_duration).clamp(0.0, 1.0))
    }

    pub fn small_circle_bonus(&self) -> f64 {
        (1.0 + (30.0 - self.radius) / 70.0).max(1.0)
    }

    pub fn overall_difficulty(&self) -> f64 {
        (79.5 - self.hit_window_great / 2.0) / 6.0
    }

    pub fn double_tap_feasibility(&self, next: Option<&DiffObject>) -> f64 {
        let Some(next) = next else { return 0.0 };

        let here = self.delta_time.max(1.0);
        let there = next.delta_time.max(1.0);
        let difference = (there - here).abs();

        let speed_ratio = here / here.max(difference);
        let window_ratio = (here / self.hit_window_great).min(1.0).powi(5);

        let distance_factor = crate::utils::reverse_lerp(
            self.lazy_jump_distance,
            NORMALISED_DIAMETER,
            NORMALISED_RADIUS,
        )
        .powi(2);

        1.0 - speed_ratio.powf(distance_factor * (1.0 - window_ratio))
    }
}

pub fn difficulty_objects(beatmap: &Beatmap, mods: Mods) -> Vec<DiffObject> {
    let timeline = Timeline::build(beatmap, mods);
    let clock_rate = mods.speed_multiplier();
    let radius = timeline.difficulty.circle_radius();

    let preempt = timeline.difficulty.preempt_ms();
    let hidden = mods.contains(dossier_replay::bits::HIDDEN);
    let hit_window_great =
        2.0 * great_hit_window(timeline.difficulty.overall_difficulty) / clock_rate;

    let parts: Vec<Vec<NestedObject>> = timeline
        .objects
        .iter()
        .map(|object| crate::slider_parts(beatmap, object))
        .collect();

    let mut out: Vec<DiffObject> = Vec::with_capacity(timeline.objects.len().saturating_sub(1));
    for (index, object) in timeline.objects.iter().enumerate().skip(1) {
        let last = &timeline.objects[index - 1];
        let delta_time = (object.start_ms - last.start_ms) / clock_rate;
        let adjusted_delta_time = delta_time.max(MIN_DELTA_TIME);

        let mut current = DiffObject {
            index,
            start_time: object.start_ms / clock_rate,
            end_time: object.end_ms / clock_rate,
            delta_time,
            adjusted_delta_time,
            last_object_end_delta_time: match out.last() {
                Some(previous) => {
                    (object.start_ms / clock_rate - previous.end_time).max(MIN_DELTA_TIME)
                }

                None => adjusted_delta_time,
            },
            jump_distance: 0.0,
            lazy_jump_distance: 0.0,
            minimum_jump_distance: 0.0,
            minimum_jump_time: adjusted_delta_time,
            travel_distance: 0.0,
            travel_time: 0.0,
            angle: None,
            normalised_vector_angle: None,
            lazy_end_position: None,
            lazy_travel_distance: 0.0,
            lazy_travel_time: 0.0,
            hit_window_great,
            is_slider: object.is_slider(),
            is_spinner: object.is_spinner(),
            pos: object.pos,
            radius,
            preempt: preempt / clock_rate,
            raw_start_time: object.start_ms,
            raw_preempt: preempt,
            raw_time_fade_in: if hidden && !object.is_slider() {
                preempt * HIDDEN_FADE_IN_DURATION_MULTIPLIER
            } else {
                400.0 * (preempt / PREEMPT_MIN).min(1.0)
            },
            end_pos: parts[index].last().map_or(object.pos, |part| part.pos),
            repeat_count: match &object.kind {
                TimedKind::Slider { slides, .. } => slides.saturating_sub(1),
                _ => 0,
            },
        };

        compute_slider_cursor_position(&mut current, object, &parts[index], radius);
        set_distances(&mut current, object, last, &out, &parts, clock_rate, radius);
        out.push(current);
    }
    out
}

fn compute_slider_cursor_position(
    current: &mut DiffObject,
    object: &TimedObject,
    parts: &[NestedObject],
    radius: f64,
) {
    let TimedKind::Slider {
        path,
        slide_duration_ms,
        ..
    } = &object.kind
    else {
        return;
    };
    if parts.is_empty() {
        return;
    }

    let duration = object.end_ms - object.start_ms;

    let mut tracking_end = (object.start_ms + duration + crate::slider::TAIL_LENIENCY)
        .max(object.start_ms + duration / 2.0);

    let mut ordered: Vec<NestedObject> = parts.to_vec();
    let last_tick = ordered.iter().rposition(|part| part.kind == Nested::Tick);
    if let Some(at) = last_tick {
        if ordered[at].time_ms > tracking_end {
            tracking_end = ordered[at].time_ms;

            let moved = ordered.remove(at);
            ordered.push(moved);
        }
    }

    current.lazy_travel_time = tracking_end - object.start_ms;

    let mut progress = if *slide_duration_ms > 0.0 {
        current.lazy_travel_time / slide_duration_ms
    } else {
        0.0
    };
    progress = if progress % 2.0 >= 1.0 {
        1.0 - progress % 1.0
    } else {
        progress % 1.0
    };
    let mut lazy_end = path.position_at(progress).unwrap_or(object.pos);

    let mut cursor = object.pos;
    let scaling = NORMALISED_RADIUS / radius;

    for (at, part) in ordered.iter().enumerate().skip(1) {
        let last = at == ordered.len() - 1;
        let mut movement = Point {
            x: part.pos.x - cursor.x,
            y: part.pos.y - cursor.y,
        };
        let mut length = scaling * hypot(movement);

        let mut required = ASSUMED_SLIDER_RADIUS;

        if last {
            let lazy_movement = Point {
                x: lazy_end.x - cursor.x,
                y: lazy_end.y - cursor.y,
            };
            if hypot(lazy_movement) < hypot(movement) {
                movement = lazy_movement;
            }
            length = scaling * hypot(movement);
        } else if part.kind == Nested::Repeat {
            required = NORMALISED_RADIUS;
        }

        if length > required {
            let keep = (length - required) / length;
            cursor = Point {
                x: cursor.x + movement.x * keep,
                y: cursor.y + movement.y * keep,
            };
            current.lazy_travel_distance += length * keep;
        }

        if last {
            lazy_end = cursor;
        }
    }

    current.lazy_end_position = Some(lazy_end);
}

#[allow(clippy::too_many_arguments)]
fn set_distances(
    current: &mut DiffObject,
    object: &TimedObject,
    last: &TimedObject,
    previous: &[DiffObject],
    parts: &[Vec<NestedObject>],
    clock_rate: f64,
    radius: f64,
) {
    if let TimedKind::Slider { slides, .. } = &object.kind {
        let repeats = f64::from(slides.saturating_sub(1));
        current.travel_distance = current.lazy_travel_distance * repeats.powf(0.3).max(1.0);
        current.travel_time = (current.lazy_travel_time / clock_rate).max(MIN_DELTA_TIME);
    }

    current.minimum_jump_time = current.adjusted_delta_time;

    if current.is_spinner || last.is_spinner() {
        return;
    }

    let scaling = NORMALISED_RADIUS / radius;
    let last_diff = previous.last();
    let last_last_diff = previous
        .len()
        .checked_sub(2)
        .and_then(|at| previous.get(at));

    let mut last_cursor = last_diff
        .and_then(|diff| diff.lazy_end_position)
        .unwrap_or(last.pos);

    current.jump_distance = distance(last.pos, object.pos) * scaling;
    current.lazy_jump_distance = distance(last_cursor, object.pos) * scaling;
    current.minimum_jump_distance = current.lazy_jump_distance;

    if last.is_slider() {
        if let Some(last_diff) = last_diff {
            let last_travel = (last_diff.lazy_travel_time / clock_rate).max(MIN_DELTA_TIME);
            current.minimum_jump_time =
                (current.adjusted_delta_time - last_travel).max(MIN_DELTA_TIME);

            let tail = parts[last_diff.index]
                .last()
                .map_or(last.pos, |part| part.pos);
            let tail_jump = distance(tail, object.pos) * scaling;
            current.minimum_jump_distance = (current.lazy_jump_distance
                - (MAXIMUM_SLIDER_RADIUS - ASSUMED_SLIDER_RADIUS))
                .min(tail_jump - MAXIMUM_SLIDER_RADIUS)
                .max(0.0);
        }
    }

    let Some(last_last_diff) = last_last_diff else {
        return;
    };
    if last_last_diff.is_spinner {
        return;
    }
    let Some(last_diff) = last_diff else { return };

    if last_diff.is_slider && last_diff.travel_distance > 0.0 {
        last_cursor = parts[last_diff.index]
            .first()
            .map_or(last.pos, |part| part.pos);
    }

    let last_last_cursor = last_last_diff
        .lazy_end_position
        .unwrap_or(last_last_diff.pos);

    let angle = corner(object.pos, last_cursor, last_last_cursor);
    let slider_angle = slider_angle(object.pos, last_diff, parts, last, last_last_cursor);

    let v = Point {
        x: object.pos.x - last_cursor.x,
        y: object.pos.y - last_cursor.y,
    };
    current.normalised_vector_angle = Some(v.y.abs().atan2(v.x.abs()));
    current.angle = Some(angle.min(slider_angle));
}

fn slider_angle(
    pos: Point,
    last_diff: &DiffObject,
    parts: &[Vec<NestedObject>],
    last: &TimedObject,
    fallback: Point,
) -> f64 {
    let last_cursor = last_diff.lazy_end_position.unwrap_or(last.pos);
    let mut last_last = fallback;
    if last_diff.is_slider && last_diff.travel_distance > 0.0 {
        let pieces = &parts[last_diff.index];
        if pieces.len() >= 2 {
            last_last = pieces[pieces.len() - 2].pos;
        }
    }
    corner(pos, last_cursor, last_last)
}

fn corner(current: Point, middle: Point, before: Point) -> f64 {
    let v1 = Point {
        x: before.x - middle.x,
        y: before.y - middle.y,
    };
    let v2 = Point {
        x: current.x - middle.x,
        y: current.y - middle.y,
    };
    let dot = v1.x * v2.x + v1.y * v2.y;
    let det = v1.x * v2.y - v1.y * v2.x;
    det.atan2(dot).abs()
}

fn hypot(p: Point) -> f64 {
    p.x.hypot(p.y)
}

fn distance(a: Point, b: Point) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}
