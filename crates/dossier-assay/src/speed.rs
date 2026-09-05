use crate::preprocessing::{DiffObject, MIN_DELTA_TIME};
use crate::utils::{
    bpm_to_milliseconds, logistic, milliseconds_to_bpm, reverse_lerp, smoothstep_bell_curve,
};

const MIN_SPEED_BONUS_BPM: f64 = 200.0;

const STRAIN_DECAY_BASE: f64 = 0.3;

fn strain_decay(ms: f64) -> f64 {
    STRAIN_DECAY_BASE.powf(ms / 1000.0)
}

pub fn speed_difficulty_of(objects: &[DiffObject], at: usize) -> f64 {
    let current = &objects[at];
    if current.is_spinner {
        return 0.0;
    }

    const SPEED_BALANCING_FACTOR: f64 = 40.0;

    let mut strain_time = current.adjusted_delta_time;
    let feasibility = 1.0 - current.double_tap_feasibility(objects.get(at + 1));

    strain_time /= ((strain_time / current.hit_window_great) / 0.93).clamp(0.92, 1.0);

    let mut speed_bonus = 0.0;
    if milliseconds_to_bpm(strain_time) > MIN_SPEED_BONUS_BPM {
        speed_bonus = 0.75
            * ((bpm_to_milliseconds(MIN_SPEED_BONUS_BPM) - strain_time) / SPEED_BALANCING_FACTOR)
                .powi(2);
    }

    let mut difficulty = (1.0 + speed_bonus) * 1000.0 / strain_time;

    difficulty *= 1.0 / (1.0 - strain_decay(current.adjusted_delta_time));
    difficulty * feasibility
}

#[derive(Debug, Clone, Copy)]
struct Island {
    delta: i64,
    count: i64,
    occurrences: i64,

    empty: bool,
}

impl Island {
    fn unset() -> Self {
        Self {
            delta: i64::MAX,
            count: 1,
            occurrences: 1,
            empty: true,
        }
    }

    fn new(delta: i64) -> Self {
        Self {
            delta: delta.max(MIN_DELTA_TIME as i64),
            count: 1,
            occurrences: 1,
            empty: false,
        }
    }

    fn add_delta(&mut self, delta: i64) {
        if self.empty {
            self.delta = delta.max(MIN_DELTA_TIME as i64);
            self.empty = false;
        }
        self.count += 1;
    }

    fn is_similar_polarity(&self, other: &Island, epsilon: f64) -> bool {
        if self.count <= 1 || other.count <= 1 {
            return false;
        }
        ((self.delta - other.delta).abs() as f64) < epsilon && self.count % 2 == other.count % 2
    }

    fn almost_equals(&self, other: &Island, epsilon: f64) -> bool {
        ((self.delta - other.delta).abs() as f64) < epsilon && self.count == other.count
    }
}

fn effective_difficulty(ratio: f64) -> f64 {
    const RHYTHM_RATIO_DIFFICULTY_MULTIPLIER: f64 = 26.0;
    let fraction = ratio - ratio.trunc();
    1.0 + RHYTHM_RATIO_DIFFICULTY_MULTIPLIER * smoothstep_bell_curve(fraction).min(0.5)
}

pub fn rhythm_multiplier_of(objects: &[DiffObject], at: usize) -> f64 {
    let current = &objects[at];
    if current.is_spinner {
        return 0.0;
    }

    const HISTORY_TIME_MAX: f64 = 5_000.0;
    const HISTORY_OBJECTS_MAX: usize = 32;
    const RHYTHM_OVERALL_MULTIPLIER: f64 = 0.95;

    const DELTA_MIN_VALUE: f64 = 1e-7;

    let mut complexity_sum = 0.0;
    let epsilon = current.hit_window_great * 0.3;

    let mut island = Island::unset();
    let mut previous_island = Island::unset();
    let mut islands: Vec<Island> = Vec::new();

    let mut start_difficulty = 0.0;
    let mut first_delta_switch = false;

    let historical_note_count = at.min(HISTORY_OBJECTS_MAX);

    let previous =
        |back: usize| -> Option<&DiffObject> { at.checked_sub(back + 1).map(|i| &objects[i]) };

    let mut rhythm_start = 0;
    while rhythm_start + 2 < historical_note_count
        && previous(rhythm_start)
            .is_some_and(|obj| current.start_time - obj.start_time < HISTORY_TIME_MAX)
    {
        rhythm_start += 1;
    }

    let Some(mut prev_obj) = previous(rhythm_start) else {
        return 1.0;
    };
    let mut prev_prev_obj = previous(rhythm_start + 1);

    for i in (1..=rhythm_start).rev() {
        let Some(curr_obj) = previous(i - 1) else {
            continue;
        };
        if curr_obj.is_spinner {
            continue;
        }

        let time_decay =
            (HISTORY_TIME_MAX - (current.start_time - curr_obj.start_time)) / HISTORY_TIME_MAX;
        let note_decay = (historical_note_count - i) as f64 / historical_note_count as f64;
        let historical_decay = note_decay.min(time_decay);

        let curr_delta = curr_obj.delta_time.max(DELTA_MIN_VALUE);
        let prev_delta = prev_obj.delta_time.max(DELTA_MIN_VALUE);
        let delta_difference = (prev_delta - curr_delta).abs();

        if island.empty {
            island = Island::new(curr_delta as i64);
        }

        let ratio = prev_delta.max(curr_delta) / prev_delta.min(curr_delta);

        let difference_multiplier = (2.0 - ratio / 8.0).clamp(0.0, 1.0);
        let window_penalty = ((delta_difference - epsilon) / epsilon).clamp(0.0, 1.0);

        let mut difficulty = effective_difficulty(ratio) * window_penalty * difference_multiplier;

        if prev_obj.is_slider {
            let lazy_end_delta = curr_obj.minimum_jump_time;
            let lazy_ratio = lazy_end_delta.max(curr_delta) / lazy_end_delta.min(curr_delta);
            let real_end_delta = curr_obj.last_object_end_delta_time;
            let real_ratio = real_end_delta.max(curr_delta) / real_end_delta.min(curr_delta);
            let slider_difficulty =
                effective_difficulty(lazy_ratio).min(effective_difficulty(real_ratio));
            difficulty = slider_difficulty.min(difficulty);
        }

        if delta_difference < epsilon {
            island.add_delta(curr_delta as i64);
        }

        if first_delta_switch {
            if delta_difference > epsilon {
                if curr_obj.is_slider {
                    difficulty *= 0.5;
                }
                if island.is_similar_polarity(&previous_island, epsilon) {
                    difficulty *= 0.5;
                }

                if prev_prev_obj.map_or(false, |obj| {
                    obj.delta_time.max(DELTA_MIN_VALUE) > prev_delta + epsilon
                }) && prev_delta > curr_delta + epsilon
                {
                    difficulty *= 0.125;
                }

                if previous_island.count == island.count {
                    difficulty *= 0.5;
                }
                if prev_delta > curr_delta + epsilon {
                    difficulty *= 0.65;
                }

                let mut found = false;
                for existing in islands.iter_mut() {
                    if existing.almost_equals(&island, epsilon) {
                        if previous_island.almost_equals(&island, epsilon) {
                            existing.occurrences += 1;
                        }
                        let power = logistic(island.delta as f64, 58.33, 0.24, 2.75);
                        difficulty *= (3.0 / existing.occurrences as f64)
                            .min((1.0 / existing.occurrences as f64).powf(power));
                        found = true;
                        break;
                    }
                }
                if !found && island.count > 0 {
                    islands.push(island);
                }

                difficulty *= 1.0 - prev_obj.double_tap_feasibility(Some(curr_obj)) * 0.75;

                if island.count > 1 {
                    complexity_sum += (difficulty * start_difficulty).sqrt() * historical_decay;
                } else {
                    complexity_sum += 0.7 * historical_decay;
                }

                start_difficulty = difficulty;

                if prev_delta + epsilon < curr_delta {
                    first_delta_switch = false;
                }

                previous_island = island;
                island = Island::new(curr_delta as i64);
            }
        } else if prev_delta > curr_delta + epsilon {
            first_delta_switch = true;
            if curr_obj.is_slider {
                difficulty *= 0.6;
            }
            if prev_obj.is_slider {
                difficulty *= 0.6;
            }
            start_difficulty = difficulty;
            island = Island::new(curr_delta as i64);
        }

        prev_prev_obj = Some(prev_obj);
        prev_obj = curr_obj;
    }

    complexity_sum *= reverse_lerp(island.count as f64, 22.0, 3.0);

    (4.0 + complexity_sum * RHYTHM_OVERALL_MULTIPLIER).sqrt() / 2.0
}

pub struct Speed {
    pub strains: Vec<f64>,

    pub slider_strains: Vec<f64>,

    pub weight_sum: f64,
}

const HARMONIC_SCALE: f64 = 20.0;
const DECAY_EXPONENT: f64 = 0.9;

impl Speed {
    pub fn of(objects: &[DiffObject], relax: bool) -> Self {
        const SKILL_MULTIPLIER: f64 = 1.16;

        let mut strains = Vec::with_capacity(objects.len());
        let mut slider_strains = Vec::new();
        let mut current_strain = 0.0;

        for at in 0..objects.len() {
            if relax {
                strains.push(0.0);
                continue;
            }
            let decay = strain_decay(objects[at].adjusted_delta_time);
            current_strain *= decay;
            current_strain += speed_difficulty_of(objects, at) * (1.0 - decay) * SKILL_MULTIPLIER;

            let total = current_strain * rhythm_multiplier_of(objects, at);
            if objects[at].is_slider {
                slider_strains.push(total);
            }
            strains.push(total);
        }

        Self {
            strains,
            slider_strains,
            weight_sum: 0.0,
        }
    }

    pub fn difficulty_value(&mut self) -> f64 {
        let (value, weight_sum) = harmonic_sum(&self.strains, HARMONIC_SCALE, DECAY_EXPONENT);
        self.weight_sum = weight_sum;
        value
    }
}

pub fn harmonic_sum(values: &[f64], harmonic_scale: f64, decay_exponent: f64) -> (f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0);
    }
    let mut sorted: Vec<f64> = values.iter().copied().filter(|v| *v > 0.0).collect();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));

    let (mut difficulty, mut weight_sum) = (0.0, 0.0);
    for (index, value) in sorted.iter().enumerate() {
        let index = index as f64;
        let harmonic = harmonic_scale / (1.0 + index);
        let weight = (1.0 + harmonic) / (index.powf(decay_exponent) + 1.0 + harmonic);
        weight_sum += weight;
        difficulty += value * weight;
    }
    (difficulty, weight_sum)
}

impl Speed {
    pub fn note_count(&self) -> f64 {
        let hardest = self.strains.iter().copied().fold(0.0f64, f64::max);
        if self.strains.is_empty() || hardest == 0.0 {
            return 0.0;
        }
        self.strains
            .iter()
            .map(|strain| crate::utils::logistic(strain / hardest, 0.5, 12.0, 1.0))
            .sum()
    }

    pub fn count_top_weighted_sliders(&self, difficulty_value: f64) -> f64 {
        if self.slider_strains.is_empty() || self.weight_sum == 0.0 {
            return 0.0;
        }
        let consistent_top = difficulty_value / self.weight_sum;
        if consistent_top == 0.0 {
            return 0.0;
        }
        self.slider_strains
            .iter()
            .map(|strain| crate::utils::logistic(strain / consistent_top, 0.88, 10.0, 1.1))
            .sum()
    }

    pub fn top_weighted_strains(&self, difficulty_value: f64) -> f64 {
        if self.strains.is_empty() || self.weight_sum == 0.0 {
            return 0.0;
        }
        let consistent_top = difficulty_value / self.weight_sum;
        if consistent_top == 0.0 {
            return 0.0;
        }
        self.strains
            .iter()
            .map(|strain| crate::utils::logistic(strain / consistent_top, 0.88, 10.0, 1.1))
            .sum()
    }
}

pub fn difficulty_rating(difficulty_value: f64) -> f64 {
    difficulty_value.sqrt() * 0.0675
}

pub fn harmonic_to_performance(difficulty: f64) -> f64 {
    4.0 * difficulty.powi(3)
}
