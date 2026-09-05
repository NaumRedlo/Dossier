use dossier_beatmap::Beatmap;
use dossier_replay::{bits, Mods};
use dossier_sim::{TimedKind, Timeline};

use crate::slider::Nested;

const BIG_TICK_SCORE: f64 = 30.0;
const SMALL_TICK_SCORE: f64 = 10.0;

pub fn nested_score_per_object(beatmap: &Beatmap, mods: Mods, object_count: u32) -> f64 {
    if object_count == 0 {
        return 0.0;
    }
    let timeline = Timeline::build(beatmap, mods);
    let (mut big, mut small) = (0.0, 0.0);
    let mut spinner_score = 0.0;

    for object in &timeline.objects {
        match &object.kind {
            TimedKind::Slider { slides, .. } => {
                big += 2.0 + f64::from(slides.saturating_sub(1));
                small += crate::slider_parts(beatmap, object)
                    .iter()
                    .filter(|part| part.kind == Nested::Tick)
                    .count() as f64;
            }
            TimedKind::Spinner => spinner_score += spinner_score_of(object.duration_ms()),
            TimedKind::Circle => {}
        }
    }

    (big * BIG_TICK_SCORE + small * SMALL_TICK_SCORE + spinner_score) / f64::from(object_count)
}

fn spinner_score_of(duration_ms: f64) -> f64 {
    const SPIN_SCORE: f64 = 100.0;
    const BONUS_SPIN_SCORE: f64 = 1000.0;

    const MAXIMUM_ROTATIONS_PER_SECOND: f64 = 477.0 / 60.0;
    const MINIMUM_ROTATIONS_PER_SECOND: f64 = 3.0;

    let seconds = duration_ms / 1000.0;
    let total_half_spins = (seconds * MAXIMUM_ROTATIONS_PER_SECOND * 2.0) as i64;
    let required_for_completion = (seconds * MINIMUM_ROTATIONS_PER_SECOND) as i64;

    let required_before_bonus = required_for_completion + 3;

    let full_spins = total_half_spins / 2;
    let mut score = SPIN_SCORE * full_spins as f64;
    let bonus_spins = ((total_half_spins - required_before_bonus) / 2 - full_spins / 2).max(0);
    score += BONUS_SPIN_SCORE * bonus_spins as f64;
    score
}

pub fn difficulty_peppy_stars(beatmap: &Beatmap) -> i32 {
    let objects = beatmap.objects.len();
    let mut drain_length = 0i64;

    if objects > 0 {
        let timeline = Timeline::build(beatmap, Mods::new(0));
        let breaks: i64 = timeline
            .breaks
            .iter()
            .map(|(start, end)| end.round() as i64 - start.round() as i64)
            .sum();
        let first = beatmap.objects.first().map_or(0.0, |o| o.time_ms).round() as i64;
        let last = beatmap.objects.last().map_or(0.0, |o| o.time_ms).round() as i64;
        drain_length = (last - first - breaks) / 1000;
    }

    let ratio = if drain_length != 0 {
        (objects as f64 / drain_length as f64 * 8.0).clamp(0.0, 16.0)
    } else {
        16.0
    };
    let difficulty = &beatmap.difficulty;
    let sum = f64::from(difficulty.hp_drain as f32)
        + f64::from(difficulty.overall_difficulty as f32)
        + f64::from(difficulty.circle_size as f32)
        + ratio;
    (sum / 38.0 * 5.0).round() as i32
}

pub fn score_multiplier(mods: Mods) -> f64 {
    if mods.contains(bits::RELAX) || mods.contains(bits::AUTOPILOT) {
        return 0.0;
    }
    let mut multiplier = 1.0;
    if mods.contains(bits::NO_FAIL) {
        multiplier *= 0.5;
    }
    if mods.contains(bits::EASY) {
        multiplier *= 0.5;
    }
    if mods.contains(bits::HALF_TIME) {
        multiplier *= 0.3;
    }
    if mods.contains(bits::HIDDEN) {
        multiplier *= 1.06;
    }
    if mods.contains(bits::HARD_ROCK) {
        multiplier *= 1.06;
    }
    if mods.contains(bits::DOUBLE_TIME) {
        multiplier *= 1.12;
    }
    if mods.contains(bits::FLASHLIGHT) {
        multiplier *= 1.12;
    }
    if mods.contains(bits::SPUN_OUT) {
        multiplier *= 0.9;
    }
    multiplier
}

pub fn maximum_combo_score(beatmap: &Beatmap, mods: Mods) -> f64 {
    let multiplier = f64::from(difficulty_peppy_stars(beatmap));
    let timeline = Timeline::build(beatmap, mods);

    let mut combo: i64 = 0;
    let mut combo_score: i64 = 0;

    let score_at = |value: i64, combo: i64| -> i64 {
        ((combo - 1).max(0) as f64 * ((value / 25) as f64 * multiplier)) as i64
    };

    for object in &timeline.objects {
        match &object.kind {
            TimedKind::Circle => {
                combo += 1;
                combo_score += score_at(300, combo);
            }
            TimedKind::Slider { .. } => {
                for _ in crate::slider_parts(beatmap, object) {
                    combo += 1;
                }

                combo_score += score_at(300, combo);
            }
            TimedKind::Spinner => {
                combo += 1;
                combo_score += score_at(300, combo);
            }
        }
    }
    combo_score as f64
}

pub fn score_based_miss_count(
    score: &crate::performance::Score,
    attributes: &crate::Attributes,
    mods: Mods,
) -> f64 {
    let Some(total) = score.legacy_total_score else {
        return 0.0;
    };
    if attributes.max_combo == 0 {
        return 0.0;
    }

    let v1_multiplier = attributes.legacy_score_base_multiplier * score_multiplier(mods);
    let combo_per_object = relevant_combo_per_object(attributes);
    let maximum = maximum_combo_based_miss_count(score, attributes);

    let during_max_combo = score_at_combo(
        score,
        attributes,
        f64::from(score.max_combo),
        combo_per_object,
        v1_multiplier,
    );
    let remaining = total as f64 - during_max_combo;
    if remaining <= 0.0 {
        return maximum;
    }

    let remaining_combo = f64::from(attributes.max_combo.saturating_sub(score.max_combo));
    let expected = score_at_combo(
        score,
        attributes,
        remaining_combo,
        combo_per_object,
        v1_multiplier,
    );

    (expected / remaining).max(1.0).min(maximum)
}

fn score_at_combo(
    score: &crate::performance::Score,
    attributes: &crate::Attributes,
    combo: f64,
    combo_per_object: f64,
    v1_multiplier: f64,
) -> f64 {
    let total_hits = f64::from(score.total_hits());
    let objects = combo / combo_per_object - 1.0;

    let combo_score = if combo_per_object > 0.0 {
        (2.0 * (combo_per_object - 1.0) + (objects - 1.0) * combo_per_object) * objects / 2.0
    } else {
        0.0
    };
    let combo_score = combo_score * score.accuracy() * 300.0 / 25.0 * v1_multiplier;

    let hit = (total_hits - f64::from(score.miss)) * combo / f64::from(attributes.max_combo);

    let flat = (300.0 + attributes.nested_score_per_object) * score.accuracy() * hit;
    combo_score + flat
}

fn relevant_combo_per_object(attributes: &crate::Attributes) -> f64 {
    let combo_score = attributes.maximum_legacy_combo_score
        / (300.0 / 25.0 * attributes.legacy_score_base_multiplier);
    let max_combo = f64::from(attributes.max_combo);
    (max_combo - 2.0) * max_combo / (max_combo + 2.0 * (combo_score - 1.0)).max(1.0)
}

fn maximum_combo_based_miss_count(
    score: &crate::performance::Score,
    attributes: &crate::Attributes,
) -> f64 {
    let misses = f64::from(score.miss);
    if attributes.slider_count == 0 {
        return misses;
    }
    let likely_dropped = 0.04 + 0.06 * attributes.aim_top_weighted_slider_factor.min(1.0).powi(2);
    let sliders = f64::from(attributes.slider_count);
    let threshold = f64::from(attributes.max_combo) - (4.0 + likely_dropped * sliders).min(sliders);

    let mut count = 0.0;
    if f64::from(score.max_combo) < threshold {
        count = (threshold / f64::from(score.max_combo).max(1.0)).powf(2.5);
    }
    count = count.min(f64::from(score.total_imperfect_hits()));

    let max_breaks = (attributes.slider_count as i64)
        .min(((attributes.max_combo as i64) - (score.max_combo as i64)) / 2)
        .max(0) as f64;
    if count - misses > max_breaks {
        count = misses + max_breaks;
    }
    count
}
