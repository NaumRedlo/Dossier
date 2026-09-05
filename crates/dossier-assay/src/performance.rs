use crate::preprocessing::MIN_DELTA_TIME;
use crate::utils::{logistic, smoothstep};

use crate::Attributes;

#[derive(Debug, Clone, Default)]
pub struct Score {
    pub max_combo: u32,
    pub great: u32,
    pub ok: u32,
    pub meh: u32,
    pub miss: u32,

    pub slider_tail_hit: u32,

    pub large_tick_miss: u32,

    pub classic: bool,

    pub legacy_total_score: Option<u64>,

    pub accuracy: Option<f64>,
}

impl Score {
    pub fn total_hits(&self) -> u32 {
        self.great + self.ok + self.meh + self.miss
    }

    pub fn total_successful_hits(&self) -> u32 {
        self.great + self.ok + self.meh
    }

    pub fn total_imperfect_hits(&self) -> u32 {
        self.ok + self.meh + self.miss
    }

    pub fn lazer_accuracy(&self, slider_count: u32, large_tick_count: u32) -> f64 {
        let judged =
            300.0 * f64::from(self.great) + 100.0 * f64::from(self.ok) + 50.0 * f64::from(self.meh);
        let possible = 300.0 * f64::from(self.total_hits());
        if self.classic {
            return if possible > 0.0 {
                (judged / possible).clamp(0.0, 1.0)
            } else {
                0.0
            };
        }
        let achieved = judged
            + 150.0 * f64::from(self.slider_tail_hit.min(slider_count))
            + 30.0 * f64::from(large_tick_count.saturating_sub(self.large_tick_miss));
        let maximum =
            possible + 150.0 * f64::from(slider_count) + 30.0 * f64::from(large_tick_count);
        if maximum <= 0.0 {
            return 0.0;
        }
        (achieved / maximum).clamp(0.0, 1.0)
    }

    pub fn accuracy(&self) -> f64 {
        if let Some(given) = self.accuracy {
            return given.clamp(0.0, 1.0);
        }
        let total = self.total_hits();
        if total == 0 {
            return 0.0;
        }
        let weighted =
            300.0 * f64::from(self.great) + 100.0 * f64::from(self.ok) + 50.0 * f64::from(self.meh);
        (weighted / (300.0 * f64::from(total))).clamp(0.0, 1.0)
    }
}

pub fn combo_based_miss_count(score: &Score, attributes: &Attributes) -> f64 {
    let misses = f64::from(score.miss);
    if attributes.slider_count == 0 {
        return misses;
    }
    let max_combo = f64::from(attributes.max_combo);
    let score_combo = f64::from(score.max_combo);
    let mut count = misses;

    if score.classic {
        let likely_dropped =
            0.04 + 0.06 * attributes.aim_top_weighted_slider_factor.min(1.0).powi(2);
        let sliders = f64::from(attributes.slider_count);

        let threshold = max_combo - (4.0 + likely_dropped * sliders).min(sliders);
        if score_combo < threshold {
            count = threshold / score_combo.max(1.0);
        }

        count = count.min(f64::from(score.total_imperfect_hits()));

        let max_breaks = (attributes.slider_count as i64)
            .min(((attributes.max_combo as i64) - (score.max_combo as i64)) / 2)
            .max(0) as f64;
        if count - misses > max_breaks {
            count = misses + max_breaks;
        }
    } else {
        let dropped = f64::from(
            attributes
                .slider_count
                .saturating_sub(score.slider_tail_hit),
        );
        let threshold = max_combo - dropped;
        if score_combo < threshold {
            count = threshold / score_combo.max(1.0);
        }

        count = count.min(f64::from(score.large_tick_miss + score.miss));
    }
    count
}

pub fn estimated_slider_breaks(
    score: &Score,
    attributes: &Attributes,
    effective_misses: f64,
    top_weighted_slider_factor: f64,
) -> f64 {
    let non_miss_mistakes = f64::from(score.ok + score.meh);
    if !score.classic || non_miss_mistakes == 0.0 {
        return 0.0;
    }

    let missed_combo = 1.0 - f64::from(score.max_combo) / f64::from(attributes.max_combo);
    let mut breaks = non_miss_mistakes.min(effective_misses * top_weighted_slider_factor);

    let adjustment = (non_miss_mistakes - breaks + 4.5) / (non_miss_mistakes + 4.0);

    breaks *= smoothstep(effective_misses, 1.0, 2.0);

    breaks * adjustment * logistic(missed_combo, 0.33, 15.0, 1.0)
}

#[derive(Debug, Clone, Default)]
pub struct Effective {
    pub miss_count: f64,
    pub combo_based_miss_count: f64,
    pub aim_slider_breaks: f64,
    pub speed_slider_breaks: f64,
}

pub fn effective(score: &Score, attributes: &Attributes, mods: dossier_replay::Mods) -> Effective {
    let combo_based = combo_based_miss_count(score, attributes);

    let mut miss_count = if score.classic && score.legacy_total_score.is_some() {
        crate::legacy::score_based_miss_count(score, attributes, mods)
    } else {
        combo_based
    };

    miss_count = miss_count.max(f64::from(score.miss));
    miss_count = miss_count.min(f64::from(score.total_hits()));
    miss_count = miss_count.max(0.0);

    let (aim_breaks, speed_breaks) = if miss_count > 0.0 {
        (
            estimated_slider_breaks(
                score,
                attributes,
                miss_count,
                attributes.aim_top_weighted_slider_factor,
            ),
            estimated_slider_breaks(
                score,
                attributes,
                miss_count,
                attributes.speed_top_weighted_slider_factor,
            ),
        )
    } else {
        (0.0, 0.0)
    };

    Effective {
        miss_count,
        combo_based_miss_count: combo_based,
        aim_slider_breaks: aim_breaks,
        speed_slider_breaks: speed_breaks,
    }
}

pub fn overall_difficulty(great_hit_window: f64) -> f64 {
    (79.5 - great_hit_window / 2.0) / 6.0
}

pub const MINIMUM_DELTA_TIME: f64 = MIN_DELTA_TIME;

pub fn miss_penalty(miss_count: f64, difficult_strain_count: f64) -> f64 {
    0.93 / (miss_count / (4.0 * difficult_strain_count.max(1.0).ln()) + 1.0)
}

fn length_bonus(total_hits: u32) -> f64 {
    let hits = f64::from(total_hits);
    0.95 + 0.35 * (hits / 2000.0).min(1.0)
        + if hits > 2000.0 {
            (hits / 2000.0).log10() * 0.5
        } else {
            0.0
        }
}

pub fn aim_value(score: &Score, attributes: &Attributes, effective: &Effective) -> f64 {
    let mut difficulty = attributes.aim_difficulty;

    if attributes.slider_count > 0 && attributes.aim_difficult_slider_count > 0.0 {
        let dropped = if score.classic {
            f64::from(score.total_imperfect_hits())
                .min(f64::from(
                    attributes.max_combo.saturating_sub(score.max_combo),
                ))
                .clamp(0.0, attributes.aim_difficult_slider_count)
        } else {
            let ends = attributes
                .slider_count
                .saturating_sub(score.slider_tail_hit);
            f64::from(ends + score.large_tick_miss)
                .clamp(0.0, attributes.aim_difficult_slider_count)
        };

        let nerf = (1.0 - attributes.slider_factor)
            * (1.0 - dropped / attributes.aim_difficult_slider_count).powi(3)
            + attributes.slider_factor;
        difficulty *= nerf;
    }

    let mut value = crate::aim::difficulty_to_performance(difficulty);
    value *= length_bonus(score.total_hits());

    if effective.miss_count > 0.0 {
        let relevant = (effective.miss_count + effective.aim_slider_breaks).min(f64::from(
            score.total_imperfect_hits() + score.large_tick_miss,
        ));
        value *= miss_penalty(relevant, attributes.aim_difficult_strain_count);
    }

    value * score.accuracy()
}

pub fn accuracy_value(score: &Score, attributes: &Attributes, overall_difficulty: f64) -> f64 {
    let mut with_accuracy = attributes.hit_circle_count;
    if !score.classic {
        with_accuracy += attributes.slider_count;
    }
    if with_accuracy == 0 {
        return 0.0;
    }

    let others = f64::from(score.total_hits().saturating_sub(with_accuracy));
    let better = (((f64::from(score.great) - others) * 6.0
        + f64::from(score.ok) * 2.0
        + f64::from(score.meh))
        / (f64::from(with_accuracy) * 6.0))
        .max(0.0);

    let mut value = 1.52163f64.powf(overall_difficulty) * better.powi(24) * 2.83;

    let share = f64::from(with_accuracy) / 1000.0;
    value *= if with_accuracy < 1000 {
        share.powf(0.3)
    } else {
        share.powf(0.1)
    };
    value
}

pub fn deviation(
    great: f64,
    ok: f64,
    meh: f64,
    great_window: f64,
    ok_window: f64,
    meh_window: f64,
) -> Option<f64> {
    if great + ok + meh <= 0.0 {
        return None;
    }

    let n = (great + ok).max(1.0);
    let p = great / n;

    const Z: f64 = 2.326_347_874_04;

    let lower = p.min(
        (n * p + Z * Z / 2.0) / (n + Z * Z)
            - Z / (n + Z * Z) * (n * p * (1.0 - p) + Z * Z / 4.0).sqrt(),
    );

    let mut value;
    if lower > 0.01 {
        value = great_window / (crate::utils::SQRT2 * crate::utils::erf_inv(lower));

        let tail = (2.0 / std::f64::consts::PI).sqrt()
            * ok_window
            * (-0.5 * (ok_window / value).powi(2)).exp()
            / (value * crate::utils::erf(ok_window / (crate::utils::SQRT2 * value)));
        value *= (1.0 - tail).sqrt();
    } else {
        value = ok_window / 3.0f64.sqrt();
    }

    let meh_variance =
        (meh_window * meh_window + ok_window * meh_window + ok_window * ok_window) / 3.0;
    Some((((great + ok) * value.powi(2) + meh * meh_variance) / (great + ok + meh)).sqrt())
}

pub fn speed_deviation(
    score: &Score,
    attributes: &Attributes,
    windows: (f64, f64, f64),
) -> Option<f64> {
    if score.total_successful_hits() == 0 {
        return None;
    }
    let mut notes = attributes.speed_note_count;

    notes += (f64::from(score.total_hits()) - attributes.speed_note_count) * 0.1;

    let miss = f64::from(score.miss).min(notes);
    let meh = f64::from(score.meh).min(notes - miss);
    let ok = f64::from(score.ok).min(notes - miss - meh);
    let great = (notes - miss - meh - ok).max(0.0);

    deviation(great, ok, meh, windows.0, windows.1, windows.2)
}

pub fn speed_value(
    score: &Score,
    attributes: &Attributes,
    effective: &Effective,
    deviation: Option<f64>,
    relax: bool,
) -> f64 {
    let Some(deviation) = deviation else {
        return 0.0;
    };
    if relax {
        return 0.0;
    }

    let mut value = crate::speed::harmonic_to_performance(attributes.speed_difficulty);

    if effective.miss_count > 0.0 {
        let relevant = (effective.miss_count + effective.speed_slider_breaks).min(f64::from(
            score.total_imperfect_hits() + score.large_tick_miss,
        ));
        value *= miss_penalty(relevant, attributes.speed_difficult_strain_count);
    }

    value *= high_deviation_nerf(attributes.speed_difficulty, deviation);

    let effective_window = 20.0 * (4.0 / attributes.speed_difficulty).powf(0.35);
    let effective_accuracy = crate::utils::erf(effective_window / deviation);
    value * effective_accuracy.powi(2)
}

fn high_deviation_nerf(speed_difficulty: f64, deviation: f64) -> f64 {
    let value = crate::speed::harmonic_to_performance(speed_difficulty);
    let cutoff = 100.0 + 220.0 * (22.0 / deviation).powf(6.5);
    if value <= cutoff {
        return 1.0;
    }
    const SCALE: f64 = 50.0;
    let adjusted = SCALE * (((value - cutoff) / SCALE + 1.0).ln() + cutoff / SCALE);
    let blend = 1.0 - crate::utils::reverse_lerp(deviation, 22.0, 27.0);
    (adjusted + (value - adjusted) * blend) / value
}

pub fn reading_value(score: &Score, attributes: &Attributes, effective: &Effective) -> f64 {
    let mut value = crate::speed::harmonic_to_performance(attributes.reading_difficulty);
    if effective.miss_count > 0.0 {
        value *= miss_penalty(
            effective.miss_count + effective.aim_slider_breaks,
            attributes.reading_difficult_note_count,
        );
    }
    value * score.accuracy().powi(3)
}

pub fn flashlight_value(
    score: &Score,
    attributes: &Attributes,
    effective: &Effective,
    has_flashlight: bool,
) -> f64 {
    if !has_flashlight {
        return 0.0;
    }
    let mut value = crate::flashlight::difficulty_to_performance(attributes.flashlight_difficulty);
    if effective.miss_count > 0.0 {
        let hits = f64::from(score.total_hits()).max(1.0);
        value *= 0.97
            * (1.0 - (effective.miss_count / hits).powf(0.775))
                .powf(effective.miss_count.powf(0.875));
    }
    value *= combo_scaling(score, attributes);
    value * (0.5 + score.accuracy() / 2.0)
}

fn combo_scaling(score: &Score, attributes: &Attributes) -> f64 {
    if attributes.max_combo == 0 {
        return 1.0;
    }
    (f64::from(score.max_combo).powf(0.8) / f64::from(attributes.max_combo).powf(0.8)).min(1.0)
}

#[derive(Debug, Clone, Default)]
pub struct Performance {
    pub aim: f64,
    pub speed: f64,
    pub accuracy: f64,
    pub reading: f64,
    pub flashlight: f64,
    pub effective_miss_count: f64,
    pub speed_deviation: Option<f64>,
    pub pp: f64,
}

pub fn performance(
    score: &Score,
    attributes: &Attributes,
    mods: dossier_replay::Mods,
) -> Performance {
    use dossier_replay::bits;
    let relax = mods.contains(bits::RELAX);
    let no_fail = mods.contains(bits::NO_FAIL);
    let spun_out = mods.contains(bits::SPUN_OUT);
    let has_flashlight = mods.contains(bits::FLASHLIGHT);

    let mut effective = effective(score, attributes, mods);
    let hits = score.total_hits();

    let windows = crate::hit_windows(attributes.overall_difficulty_raw, mods.speed_multiplier());
    let difficulty = overall_difficulty(2.0 * windows.0);

    let mut multiplier = crate::PERFORMANCE_BASE_MULTIPLIER;
    if no_fail {
        multiplier *= (1.0 - 0.02 * effective.miss_count).max(0.90);
    }
    if spun_out && hits > 0 {
        multiplier *= 1.0 - (f64::from(attributes.spinner_count) / f64::from(hits)).powf(0.85);
    }
    if relax {
        let ok_multiplier = 0.75
            * if difficulty > 0.0 {
                1.0 - difficulty / 13.33
            } else {
                1.0
            }
            .max(0.0);
        let meh_multiplier = if difficulty > 0.0 {
            1.0 - (difficulty / 13.33).powi(5)
        } else {
            1.0
        }
        .max(0.0);
        effective.miss_count = (effective.miss_count
            + f64::from(score.ok) * ok_multiplier
            + f64::from(score.meh) * meh_multiplier)
            .min(f64::from(hits));
    }

    let deviation = speed_deviation(score, attributes, windows);
    let aim = aim_value(score, attributes, &effective);
    let speed = speed_value(score, attributes, &effective, deviation, relax);
    let accuracy = if relax {
        0.0
    } else {
        accuracy_value(score, attributes, difficulty)
    };
    let reading = reading_value(score, attributes, &effective);
    let flashlight = flashlight_value(score, attributes, &effective, has_flashlight);

    let cognition =
        crate::flashlight::sum_cognition(reading, flashlight, crate::PERFORMANCE_NORM_EXPONENT);
    let pp = crate::utils::norm(
        crate::PERFORMANCE_NORM_EXPONENT,
        &[aim, speed, accuracy, cognition],
    ) * multiplier;

    Performance {
        aim,
        speed,
        accuracy,
        reading,
        flashlight,
        effective_miss_count: effective.miss_count,
        speed_deviation: deviation,
        pp,
    }
}
