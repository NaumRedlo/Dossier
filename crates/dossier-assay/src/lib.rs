pub mod aim;
pub mod flashlight;
pub mod legacy;
pub mod performance;
pub mod preprocessing;
pub mod reading;
pub mod slider;
pub mod speed;
pub mod strain;
pub mod utils;

use dossier_beatmap::Beatmap;
use dossier_replay::Mods;
use dossier_sim::{TimedKind, TimedObject, Timeline};

use crate::slider::{nested_objects, tick_distance, NestedObject};

#[derive(Debug, Clone, Default)]
pub struct Attributes {
    pub max_combo: u32,
    pub aim_difficulty: f64,
    pub speed_difficulty: f64,
    pub slider_factor: f64,
    pub aim_difficult_slider_count: f64,
    pub aim_difficult_strain_count: f64,
    pub speed_difficult_strain_count: f64,
    pub speed_note_count: f64,

    pub overall_difficulty_raw: f64,
    pub reading_difficulty: f64,
    pub reading_difficult_note_count: f64,
    pub flashlight_difficulty: f64,
    pub aim_top_weighted_slider_factor: f64,
    pub speed_top_weighted_slider_factor: f64,

    pub hit_circle_count: u32,
    pub slider_count: u32,
    pub spinner_count: u32,

    pub large_tick_count: u32,

    pub nested_score_per_object: f64,
    pub legacy_score_base_multiplier: f64,
    pub maximum_legacy_combo_score: f64,

    pub star_rating: f64,
}

pub(crate) const PERFORMANCE_NORM_EXPONENT: f64 = 1.1;
pub(crate) const PERFORMANCE_BASE_MULTIPLIER: f64 = 1.12;

pub fn hit_windows(overall_difficulty: f64, clock_rate: f64) -> (f64, f64, f64) {
    let at = |min, mid, max| {
        (dossier_beatmap::difficulty_range(overall_difficulty, min, mid, max).floor() - 0.5)
            / clock_rate
    };
    (
        at(80.0, 50.0, 20.0),
        at(140.0, 100.0, 60.0),
        at(200.0, 150.0, 100.0),
    )
}

pub fn attributes(beatmap: &Beatmap, mods: Mods) -> Attributes {
    use dossier_replay::bits;
    let objects = preprocessing::difficulty_objects(beatmap, mods);
    let relax = mods.contains(bits::RELAX);
    let touch = mods.contains(bits::TOUCH_DEVICE);
    let autopilot = mods.contains(bits::AUTOPILOT);

    let mut with = aim::Aim::of(&objects, true, relax, touch, autopilot);
    let mut without = aim::Aim::of(&objects, false, relax, touch, autopilot);
    let aim_value = with.difficulty_value();
    let aim_rating = aim::difficulty_rating(aim_value);

    let mut speed = speed::Speed::of(&objects, relax);
    let speed_value = speed.difficulty_value();

    let hidden = mods.contains(bits::HIDDEN);
    let mut reading = reading::Reading::of(&objects, hidden, relax, touch, autopilot);
    let reading_value = reading.difficulty_value();
    let reading_rating = reading::difficulty_rating(reading_value);

    let has_flashlight = mods.contains(bits::FLASHLIGHT);
    let torch = flashlight::Flashlight::of(
        &objects,
        has_flashlight,
        hidden,
        relax,
        touch,
        autopilot,
        objects.len() + 1,
    );
    let flashlight_rating = if has_flashlight {
        flashlight::difficulty_rating(torch.difficulty_value())
    } else {
        0.0
    };

    let cognition = flashlight::sum_cognition(
        speed::harmonic_to_performance(reading_rating),
        flashlight::difficulty_to_performance(flashlight_rating),
        PERFORMANCE_NORM_EXPONENT,
    );
    let base = utils::norm(
        PERFORMANCE_NORM_EXPONENT,
        &[
            aim::difficulty_to_performance(aim_rating),
            speed::harmonic_to_performance(speed::difficulty_rating(speed_value)),
            cognition,
        ],
    );
    let star_rating = (base * PERFORMANCE_BASE_MULTIPLIER).cbrt();

    let no_slider_value = without.difficulty_value();
    let aim_slider_count = without.count_top_weighted_sliders(no_slider_value);
    let aim_strain_count = without.top_weighted_strains(no_slider_value);
    let aim_top_weighted_slider_factor =
        aim_slider_count / (aim_strain_count - aim_slider_count).max(1.0);
    let speed_slider_count = speed.count_top_weighted_sliders(speed_value);
    let speed_strain_count = speed.top_weighted_strains(speed_value);
    let speed_top_weighted_slider_factor =
        speed_slider_count / (speed_strain_count - speed_slider_count).max(1.0);

    let timeline = dossier_sim::Timeline::build(beatmap, mods);
    let (mut circles, mut sliders, mut spinners) = (0, 0, 0);
    for object in &timeline.objects {
        match object.kind {
            dossier_sim::TimedKind::Circle => circles += 1,
            dossier_sim::TimedKind::Slider { .. } => sliders += 1,
            dossier_sim::TimedKind::Spinner => spinners += 1,
        }
    }

    Attributes {
        max_combo: max_combo(beatmap, mods),
        aim_difficulty: aim_rating,
        speed_difficulty: speed::difficulty_rating(speed_value),
        slider_factor: if aim_value > 0.0 {
            aim::difficulty_rating(without.difficulty_value()) / aim_rating
        } else {
            1.0
        },
        aim_difficult_slider_count: with.difficult_sliders(),
        aim_difficult_strain_count: with.top_weighted_strains(aim_value),

        speed_difficult_strain_count: speed.top_weighted_strains(speed_value),
        reading_difficult_note_count: reading.top_weighted_notes(reading_value),
        speed_note_count: speed.note_count(),
        overall_difficulty_raw: timeline.difficulty.overall_difficulty,
        reading_difficulty: reading_rating,
        flashlight_difficulty: flashlight_rating,
        star_rating,
        aim_top_weighted_slider_factor,
        speed_top_weighted_slider_factor,
        hit_circle_count: circles,
        slider_count: sliders,
        spinner_count: spinners,
        large_tick_count: timeline
            .objects
            .iter()
            .map(|object| {
                slider_parts(beatmap, object)
                    .iter()
                    .filter(|part| {
                        matches!(part.kind, slider::Nested::Tick | slider::Nested::Repeat)
                    })
                    .count() as u32
            })
            .sum(),
        nested_score_per_object: legacy::nested_score_per_object(
            beatmap,
            mods,
            circles + sliders + spinners,
        ),
        legacy_score_base_multiplier: f64::from(legacy::difficulty_peppy_stars(beatmap)),
        maximum_legacy_combo_score: legacy::maximum_combo_score(beatmap, mods),
    }
}

pub fn aim_difficulty(beatmap: &Beatmap, mods: Mods) -> (f64, f64) {
    use dossier_replay::bits;
    let objects = preprocessing::difficulty_objects(beatmap, mods);
    let relax = mods.contains(bits::RELAX);
    let touch = mods.contains(bits::TOUCH_DEVICE);
    let autopilot = mods.contains(bits::AUTOPILOT);

    let mut with = aim::Aim::of(&objects, true, relax, touch, autopilot);
    let mut without = aim::Aim::of(&objects, false, relax, touch, autopilot);
    let value = with.difficulty_value();
    let rating = aim::difficulty_rating(value);
    let slider_factor = if value > 0.0 {
        aim::difficulty_rating(without.difficulty_value()) / rating
    } else {
        1.0
    };
    (rating, slider_factor)
}

pub fn speed_difficulty(beatmap: &Beatmap, mods: Mods) -> f64 {
    let objects = preprocessing::difficulty_objects(beatmap, mods);
    let relax = mods.contains(dossier_replay::bits::RELAX);
    let mut skill = speed::Speed::of(&objects, relax);
    speed::difficulty_rating(skill.difficulty_value())
}

pub fn slider_parts(beatmap: &Beatmap, object: &TimedObject) -> Vec<NestedObject> {
    let TimedKind::Slider {
        path,
        slides,
        slide_duration_ms,
        ..
    } = &object.kind
    else {
        return Vec::new();
    };
    let velocity = if *slide_duration_ms > 0.0 {
        path.length() / slide_duration_ms
    } else {
        0.0
    };
    let beat_length = beatmap
        .timing
        .timing_point_at(object.start_ms)
        .map_or(0.0, |point| point.beat_length);
    nested_objects(
        path,
        object.start_ms,
        *slide_duration_ms,
        *slides,
        tick_distance(velocity, beat_length, beatmap.difficulty.slider_tick_rate),
        velocity,
    )
}

pub fn max_combo(beatmap: &Beatmap, mods: Mods) -> u32 {
    let timeline = Timeline::build(beatmap, mods);
    timeline
        .objects
        .iter()
        .map(|object| match &object.kind {
            TimedKind::Circle | TimedKind::Spinner => 1,

            TimedKind::Slider { .. } => slider_parts(beatmap, object).len() as u32,
        })
        .sum()
}
