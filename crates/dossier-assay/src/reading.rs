use crate::preprocessing::{DiffObject, NORMALISED_DIAMETER, NORMALISED_RADIUS};
use crate::utils::{logistic, norm, reverse_lerp, smootherstep};

const READING_WINDOW_SIZE: f64 = 3000.0;

const DISTANCE_INFLUENCE_THRESHOLD: f64 = NORMALISED_DIAMETER * 1.5;

fn radians(degrees: f64) -> f64 {
    degrees * std::f64::consts::PI / 180.0
}

fn time_nerf_factor(delta: f64) -> f64 {
    (2.0 - delta / (READING_WINDOW_SIZE / 2.0)).clamp(0.0, 1.0)
}

fn past_object_influence(objects: &[DiffObject], at: usize) -> f64 {
    let current = &objects[at];
    let mut influence = 0.0;
    for back in 0..at {
        let object = &objects[at - 1 - back];
        if current.start_time - object.start_time > READING_WINDOW_SIZE
            || object.start_time < current.start_time - current.preempt
        {
            break;
        }
        let mut difficulty = current.opacity_at(object.raw_start_time, false);

        difficulty *= smootherstep(
            object.lazy_jump_distance,
            15.0,
            DISTANCE_INFLUENCE_THRESHOLD,
        );
        difficulty *= time_nerf_factor(current.start_time - object.start_time);
        influence += difficulty;
    }
    influence
}

fn visible_object_density(objects: &[DiffObject], at: usize) -> f64 {
    let current = &objects[at];
    let mut count = 0.0;
    for ahead in objects.iter().skip(at + 1) {
        if ahead.start_time - current.start_time > READING_WINDOW_SIZE
            || current.start_time < ahead.start_time - ahead.preempt
        {
            break;
        }
        count += ahead.opacity_at(current.raw_start_time, false)
            * time_nerf_factor(ahead.start_time - current.start_time);
    }
    count
}

fn constant_angle_nerf_factor(objects: &[DiffObject], at: usize) -> f64 {
    const MINIMUM_ANGLE_RELEVANCY_TIME: f64 = 2000.0;
    const MAXIMUM_ANGLE_RELEVANCY_TIME: f64 = 200.0;

    let current = &objects[at];
    let mut count = 0.0;
    let mut index = 0usize;
    let mut gap = 0.0;

    let mut prev0 = current;
    let mut prev1: Option<&DiffObject> = None;
    let mut prev2: Option<&DiffObject> = None;

    while gap < MINIMUM_ANGLE_RELEVANCY_TIME {
        let Some(object) = at.checked_sub(index + 1).map(|i| &objects[i]) else {
            break;
        };

        let long_interval = 1.0
            - reverse_lerp(
                object.adjusted_delta_time,
                MAXIMUM_ANGLE_RELEVANCY_TIME,
                MINIMUM_ANGLE_RELEVANCY_TIME,
            );

        if let (Some(here), Some(there)) = (current.angle, object.angle) {
            let difference = (here - there).abs();
            let mut alternating = std::f64::consts::PI;

            if let (Some(one), Some(two)) = (prev1, prev2) {
                if let (Some(a0), Some(a1), Some(a2), Some(ao)) =
                    (prev0.angle, one.angle, two.angle, object.angle)
                {
                    alternating = (a1 - ao).abs() + (a2 - a0).abs();

                    let mut weight = 1.0;
                    weight *= reverse_lerp(ao.min(a0) * 180.0 / std::f64::consts::PI, 20.0, 5.0);
                    weight *= reverse_lerp(ao.max(a0) * 180.0 / std::f64::consts::PI, 60.0, 120.0);
                    alternating =
                        std::f64::consts::PI + (0.1 * alternating - std::f64::consts::PI) * weight;
                }
            }

            let stack = smootherstep(object.lazy_jump_distance, 0.0, NORMALISED_RADIUS);
            count += (3.0 * radians(30.0).min(difference.min(alternating) * stack)).cos()
                * long_interval;
        }

        gap = current.start_time - object.start_time;
        index += 1;
        prev2 = prev1;
        prev1 = Some(prev0);
        prev0 = object;
    }

    (2.0 / count).clamp(0.2, 1.0)
}

pub fn reading_parts(
    objects: &[DiffObject],
    at: usize,
    hidden: bool,
) -> (f64, f64, f64, f64, f64, f64) {
    let current = &objects[at];
    if current.is_spinner || at == 0 {
        return (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    }
    let velocity = (current.lazy_jump_distance / current.adjusted_delta_time).max(1.0);
    let density = visible_object_density(objects, at);
    let past = past_object_influence(objects, at);
    let nerf = constant_angle_nerf_factor(objects, at);
    let note_density = density_difficulty(objects.get(at + 1), velocity, nerf, past, density);
    let preempt = preempt_difficulty(velocity, nerf, current.preempt);
    let _ = hidden;
    (velocity, density, past, nerf, note_density, preempt)
}

pub fn reading_difficulty_of(objects: &[DiffObject], at: usize, hidden: bool) -> f64 {
    let current = &objects[at];
    if current.is_spinner || at == 0 {
        return 0.0;
    }

    let velocity = (current.lazy_jump_distance / current.adjusted_delta_time).max(1.0);
    let density = visible_object_density(objects, at);
    let past = past_object_influence(objects, at);
    let nerf = constant_angle_nerf_factor(objects, at);

    let note_density = density_difficulty(objects.get(at + 1), velocity, nerf, past, density);
    let hidden_difficulty = if hidden {
        hidden_difficulty(current, past, density, velocity, nerf, objects.get(at - 1))
    } else {
        0.0
    };
    let preempt = preempt_difficulty(velocity, nerf, current.preempt);

    let difficulty = norm(1.5, &[preempt, hidden_difficulty, note_density]);

    difficulty * (1.0 / (1.0 - 0.8f64.powf(current.adjusted_delta_time / 1000.0)))
}

fn density_difficulty(
    next: Option<&DiffObject>,
    velocity: f64,
    nerf: f64,
    past: f64,
    density: f64,
) -> f64 {
    const DENSITY_MULTIPLIER: f64 = 2.4;
    const DENSITY_DIFFICULTY_BASE: f64 = 2.5;

    let mut future = density.sqrt();
    if let Some(next) = next {
        future *= smootherstep(next.lazy_jump_distance, 15.0, DISTANCE_INFLUENCE_THRESHOLD);
    }
    let mut difficulty = (past + future).powf(1.7) * 0.4 * nerf * velocity;

    difficulty = (difficulty - DENSITY_DIFFICULTY_BASE).max(0.0);

    difficulty.powf(0.45) * DENSITY_MULTIPLIER
}

fn preempt_difficulty(velocity: f64, nerf: f64, preempt: f64) -> f64 {
    const PREEMPT_BALANCING_FACTOR: f64 = 140_000.0;

    const PREEMPT_STARTING_POINT: f64 = 500.0;

    let over =
        ((PREEMPT_STARTING_POINT - preempt) + (preempt - PREEMPT_STARTING_POINT).abs()) / 2.0;
    over.powf(2.5) / PREEMPT_BALANCING_FACTOR * nerf * velocity
}

fn hidden_difficulty(
    current: &DiffObject,
    past: f64,
    density: f64,
    velocity: f64,
    nerf: f64,
    previous: Option<&DiffObject>,
) -> f64 {
    const HIDDEN_MULTIPLIER: f64 = 0.28;

    let preempt_factor = current.preempt.powf(2.2) * 0.01;
    let density_factor = (density + past).powf(3.3) * 3.0;
    let mut difficulty = (preempt_factor + density_factor) * nerf * velocity * 0.01;
    difficulty = difficulty.powf(0.4) * HIDDEN_MULTIPLIER;

    if let Some(previous) = previous {
        if current.lazy_jump_distance == 0.0
            && current.opacity_at(previous.raw_start_time, true) == 0.0
            && previous.start_time > current.start_time - current.preempt
        {
            difficulty += HIDDEN_MULTIPLIER * 2500.0 / current.adjusted_delta_time.powf(1.5);
        }
    }
    difficulty
}

pub struct Reading {
    pub difficulties: Vec<f64>,

    reduced_note_count: f64,
    pub weight_sum: f64,
}

impl Reading {
    pub fn of(
        objects: &[DiffObject],
        hidden: bool,
        relax: bool,
        touch: bool,
        autopilot: bool,
    ) -> Self {
        const SKILL_MULTIPLIER: f64 = 2.5;
        const REDUCED_DIFFICULTY_DURATION: f64 = 60.0 * 1000.0;

        let mut difficulties = Vec::with_capacity(objects.len());
        let mut current_strain = 0.0f64;
        let mut reduced_until: Option<f64> = None;
        let mut reduced_note_count = 0.0;

        for at in 0..objects.len() {
            let mut difficulty = reading_difficulty_of(objects, at, hidden);
            if touch {
                difficulty = difficulty.powf(0.89);
            }
            if relax {
                difficulty *= 0.4;
            }
            if autopilot {
                difficulty *= 0.1;
            }
            difficulty *= 0.825 + objects[at].overall_difficulty().max(0.0).powf(2.2) / 1125.0;

            let decay = 0.8f64.powf(objects[at].delta_time / 1000.0);
            current_strain *= decay;
            current_strain += difficulty * (1.0 - decay) * SKILL_MULTIPLIER;

            let until =
                *reduced_until.get_or_insert(objects[at].start_time + REDUCED_DIFFICULTY_DURATION);
            if objects[at].start_time <= until {
                reduced_note_count += 1.0;
            }
            difficulties.push(current_strain);
        }

        Self {
            difficulties,
            reduced_note_count,
            weight_sum: 0.0,
        }
    }

    fn transformed(&self) -> Vec<f64> {
        const REDUCED_DIFFICULTY_BASE_LINE: f64 = 0.0;
        let mut out: Vec<f64> = self
            .difficulties
            .iter()
            .copied()
            .filter(|v| *v > 0.0)
            .collect();
        let count = self.reduced_note_count;
        let limit = (out.len() as f64).min(count) as usize;
        for (index, value) in out.iter_mut().enumerate().take(limit) {
            let at = if count > 0.0 {
                (index as f64 / count).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let scale = (1.0 + 9.0 * at).log10();
            *value *= REDUCED_DIFFICULTY_BASE_LINE + (1.0 - REDUCED_DIFFICULTY_BASE_LINE) * scale;
        }
        out
    }

    pub fn difficulty_value(&mut self) -> f64 {
        let (value, weight_sum) = crate::speed::harmonic_sum(&self.transformed(), 1.0, 0.9);
        self.weight_sum = weight_sum;
        value
    }

    pub fn top_weighted_notes(&self, difficulty_value: f64) -> f64 {
        if self.difficulties.is_empty() || self.weight_sum == 0.0 {
            return 0.0;
        }
        let consistent_top = difficulty_value / self.weight_sum;
        if consistent_top == 0.0 {
            return 0.0;
        }
        self.difficulties
            .iter()
            .map(|value| logistic(value / consistent_top, 1.15, 5.0, 1.1))
            .sum()
    }
}

pub fn difficulty_rating(difficulty_value: f64) -> f64 {
    difficulty_value.sqrt() * 0.0675
}
