use dossier_beatmap::{Beatmap, Difficulty};
use dossier_replay::{bits, Mods};

use crate::judge::{window_judgement, Event, Judge, Judgement, Part};
use crate::ruleset::{Client, Ruleset};

fn takes_combo_multiplier(part: Part) -> bool {
    matches!(part, Part::Circle | Part::Slider | Part::Spinner)
}

fn stable_base_value(part: Part, result: Judgement) -> u32 {
    match part {
        Part::SliderTick => 10,

        Part::SpinnerSpin => 0,
        Part::SpinnerPoints => 100,
        Part::SpinnerBonus => 1100,
        Part::SliderRepeat | Part::SliderTail | Part::SliderHead => 30,
        Part::Circle | Part::Slider | Part::Spinner => match result {
            Judgement::Great => 300,
            Judgement::Ok => 100,
            Judgement::Meh => 50,
            Judgement::Miss => 0,
        },
    }
}

fn v2_value(part: Part, result: Judgement) -> u32 {
    if result.is_miss() {
        return 0;
    }
    match part {
        Part::SliderTick => 10,
        Part::SpinnerSpin => 0,
        Part::SpinnerPoints => 100,

        Part::SpinnerBonus => 500,
        Part::SliderHead | Part::SliderRepeat | Part::SliderTail => 30,
        Part::Circle | Part::Slider | Part::Spinner => result.value(),
    }
}

fn round_half_to_even(x: f64) -> f64 {
    let rounded = x.round();
    if (x - x.trunc()).abs() == 0.5 && rounded % 2.0 != 0.0 {
        rounded - x.signum()
    } else {
        rounded
    }
}

pub fn difficulty_multiplier(beatmap: &Beatmap, object_count: usize, drain_seconds: f64) -> u32 {
    let d = &beatmap.difficulty;
    let density = if drain_seconds > 0.0 {
        (object_count as f64 / drain_seconds * 8.0).clamp(0.0, 16.0)
    } else {
        0.0
    };
    let raw = (d.hp_drain + d.overall_difficulty + d.circle_size + density) / 38.0 * 5.0;
    round_half_to_even(raw) as u32
}

pub fn stable_mod_multiplier(mods: Mods) -> f64 {
    if mods.contains(bits::RELAX) || mods.contains(bits::AUTOPILOT) {
        return 0.0;
    }

    let v2 = mods.contains(bits::SCORE_V2);
    let mut m = 1.0;
    for (bit, factor) in [
        (bits::NO_FAIL, if v2 { 1.0 } else { 0.5 }),
        (bits::EASY, 0.5),
        (bits::HALF_TIME, 0.3),
        (bits::HARD_ROCK, if v2 { 1.10 } else { 1.06 }),
        (bits::HIDDEN, 1.06),
        (bits::FLASHLIGHT, 1.12),
        (bits::SPUN_OUT, 0.9),
    ] {
        if mods.contains(bit) {
            m *= factor;
        }
    }

    if mods.contains(bits::DOUBLE_TIME) || mods.contains(bits::NIGHTCORE) {
        m *= if v2 { 1.20 } else { 1.12 };
    }
    m
}

pub fn drain_seconds(beatmap: &Beatmap) -> f64 {
    let (Some(first), Some(last)) = (beatmap.objects.first(), beatmap.objects.last()) else {
        return 0.0;
    };
    let span = last.time_ms - first.time_ms;
    let breaks: f64 = beatmap
        .breaks
        .iter()
        .map(|&(from, to)| (to - from).max(0.0))
        .sum();
    ((span - breaks) / 1000.0).max(0.0)
}

pub fn stable_halves(judge: &Judge) -> (f64, f64) {
    let mut flat = 0f64;
    let mut combo_units = 0f64;
    let mut combo = 0u32;
    for event in judge.events() {
        let value = f64::from(stable_base_value(event.part, event.result));
        if value > 0.0 {
            flat += value;
            if takes_combo_multiplier(event.part) {
                combo_units += value * f64::from(combo.saturating_sub(1)) / 25.0;
            }
        }
        combo = event.combo_after;
    }
    (flat, combo_units)
}

fn lazer_max_value(part: Part) -> f64 {
    match part {
        Part::Circle | Part::Spinner | Part::SliderHead => 300.0,
        Part::SliderTail => 150.0,
        Part::SliderTick | Part::SliderRepeat => 30.0,
        Part::Slider => 0.0,

        Part::SpinnerSpin | Part::SpinnerPoints | Part::SpinnerBonus => 0.0,
    }
}

fn lazer_value(event: &Event, difficulty: &Difficulty) -> f64 {
    match event.part {
        Part::Slider => 0.0,

        Part::SpinnerSpin | Part::SpinnerPoints | Part::SpinnerBonus => 0.0,
        Part::SliderHead => match event.error_ms {
            Some(error) => tiered(window_judgement(error, difficulty)),
            None => 0.0,
        },

        Part::SliderTail | Part::SliderTick | Part::SliderRepeat => {
            if event.result.is_miss() {
                0.0
            } else {
                lazer_max_value(event.part)
            }
        }
        Part::Circle | Part::Spinner => tiered(event.result),
    }
}

fn tiered(result: Judgement) -> f64 {
    match result {
        Judgement::Great => 300.0,
        Judgement::Ok => 100.0,
        Judgement::Meh => 50.0,
        Judgement::Miss => 0.0,
    }
}

const COMBO_EXPONENT: f64 = 0.5;

#[derive(Debug, Clone, Default)]
pub struct ScoreTrack {
    points: Vec<(f64, u64)>,
    ruleset: Option<Ruleset>,
    comparable: bool,
}

impl ScoreTrack {
    pub fn build(judge: &Judge, beatmap: &Beatmap, mods: Mods, ruleset: Ruleset) -> Self {
        Self::build_with(judge, beatmap, mods, &mods.as_lazer_mods(), ruleset)
    }

    pub fn build_with(
        judge: &Judge,
        beatmap: &Beatmap,
        mods: Mods,
        lazer_mods: &[dossier_replay::LazerMod],
        ruleset: Ruleset,
    ) -> Self {
        Self::build_for(judge, beatmap, mods, lazer_mods, None, usize::MAX, ruleset)
    }

    pub fn build_for(
        judge: &Judge,
        beatmap: &Beatmap,
        mods: Mods,
        lazer_mods: &[dossier_replay::LazerMod],
        recorded_multiplier: Option<f64>,
        played: usize,
        ruleset: Ruleset,
    ) -> Self {
        let score_v2 = mods.contains(dossier_replay::bits::SCORE_V2);
        let mut track = match ruleset.client() {
            Client::Stable if score_v2 => Self::stable_v2(judge, mods, played),
            Client::Stable => Self::stable(judge, beatmap, mods, played),
            Client::Lazer => Self::lazer(
                judge,
                beatmap,
                lazer_mods,
                recorded_multiplier,
                played,
                ruleset,
            ),
        };
        track.ruleset = Some(ruleset);
        track
    }

    fn stable_v2(judge: &Judge, mods: Mods, played: usize) -> Self {
        let (mut combo_part_max, mut max_hits) = (0.0f64, 0u32);
        let mut perfect_combo = 0u32;
        for event in judge.events() {
            let value = f64::from(v2_value(event.part, Judgement::Great));
            if event.part.adds_combo() {
                perfect_combo += 1;
            }
            combo_part_max += value * (1.0 + f64::from(perfect_combo) / 10.0);
            if event.part.counts_for_accuracy() {
                max_hits += 1;
            }
        }
        if combo_part_max <= 0.0 || max_hits == 0 {
            return Self {
                points: Vec::new(),
                ruleset: None,
                comparable: true,
            };
        }

        let multiplier = stable_mod_multiplier(mods);
        let (mut combo_part, mut raw_score, mut hits) = (0.0f64, 0u32, 0u32);
        let mut bonus = 0.0f64;
        let mut points = Vec::with_capacity(judge.events().len());
        for event in judge.events() {
            if event.object_index >= played {
                break;
            }
            if event.part.is_bonus() {
                bonus += f64::from(v2_value(event.part, event.result));
            } else {
                combo_part += f64::from(v2_value(event.part, event.result))
                    * (1.0 + f64::from(event.combo_after) / 10.0);
            }
            if event.part.counts_for_accuracy() {
                raw_score += event.result.value();
                hits += 1;
            }

            let accuracy = if hits > 0 {
                raw_score as f32 / (hits * 300) as f32
            } else {
                1.0
            };
            let total = (combo_part / combo_part_max * 700_000.0
                + f64::from(accuracy).powi(10) * f64::from(hits) / f64::from(max_hits) * 300_000.0
                + bonus)
                * multiplier;
            points.push((event.time_ms, total.round().max(0.0) as u64));
        }
        Self {
            points,
            ruleset: None,
            comparable: true,
        }
    }

    fn stable(judge: &Judge, beatmap: &Beatmap, mods: Mods, played: usize) -> Self {
        let multiplier = f64::from(difficulty_multiplier(
            beatmap,
            beatmap.objects.len(),
            drain_seconds(beatmap),
        )) * stable_mod_multiplier(mods);

        let mut total = 0u64;
        let mut combo = 0u32;
        let mut points = Vec::with_capacity(judge.events().len());
        for event in judge.events() {
            if event.object_index >= played {
                break;
            }
            let value = f64::from(stable_base_value(event.part, event.result));
            if value > 0.0 {
                total += value as u64;
                if takes_combo_multiplier(event.part) {
                    let carried = f64::from(combo.saturating_sub(1));
                    total += (carried * (value / 25.0 * multiplier)) as u64;
                }
            }
            combo = event.combo_after;
            points.push((event.time_ms, total));
        }
        Self {
            points,
            ruleset: None,
            comparable: true,
        }
    }

    fn lazer(
        judge: &Judge,
        beatmap: &Beatmap,
        mods: &[dossier_replay::LazerMod],
        recorded_multiplier: Option<f64>,
        played: usize,
        ruleset: Ruleset,
    ) -> Self {
        let difficulty = &beatmap.difficulty;
        let multiplier = recorded_multiplier.unwrap_or_else(|| {
            crate::multiplier::lazer_multiplier(ruleset.multipliers(), mods, difficulty)
        });

        let mut best_combo = 0u32;
        let mut max_combo_portion = 0f64;
        let mut judgements = 0f64;
        for event in judge.events() {
            let max = lazer_max_value(event.part);
            if max == 0.0 {
                continue;
            }
            if event.part.adds_combo() {
                best_combo += 1;
            }
            max_combo_portion += max * f64::from(best_combo).powf(COMBO_EXPONENT);
            judgements += 1.0;
        }

        let mut points = Vec::with_capacity(judge.events().len());
        let mut combo_portion = 0f64;
        let mut base = 0f64;
        let mut reached_base = 0f64;
        let mut made = 0f64;
        for event in judge.events() {
            if event.object_index >= played {
                break;
            }
            let max = lazer_max_value(event.part);
            if max > 0.0 {
                combo_portion += max * f64::from(event.combo_after).powf(COMBO_EXPONENT);
                base += lazer_value(event, difficulty);
                reached_base += max;
                made += 1.0;
            }
            let accuracy = if reached_base > 0.0 {
                base / reached_base
            } else {
                1.0
            };
            let combo_progress = if max_combo_portion > 0.0 {
                combo_portion / max_combo_portion
            } else {
                1.0
            };
            let accuracy_progress = if judgements > 0.0 {
                made / judgements
            } else {
                1.0
            };
            let total = (500_000.0 * accuracy * combo_progress
                + 500_000.0 * accuracy.powi(5) * accuracy_progress)
                * multiplier;
            points.push((event.time_ms, total.round() as u64));
        }
        Self {
            points,
            ruleset: None,
            comparable: true,
        }
    }

    pub fn reached(&self, score: u64) -> f64 {
        let i = self.points.partition_point(|(_, total)| *total < score);
        self.points
            .get(i)
            .map_or(f64::NEG_INFINITY, |(time_ms, _)| *time_ms)
    }

    pub fn at(&self, time_ms: f64) -> u64 {
        let i = self.points.partition_point(|(t, _)| *t <= time_ms);
        if i == 0 {
            0
        } else {
            self.points[i - 1].1
        }
    }

    pub fn total(&self) -> u64 {
        self.points.last().map_or(0, |(_, v)| *v)
    }

    pub fn comparable(&self) -> bool {
        self.comparable
    }

    pub fn ruleset(&self) -> Option<Ruleset> {
        self.ruleset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(body: &str) -> Beatmap {
        Beatmap::parse(&format!("osu file format v14\n\n{body}")).expect("parses")
    }

    #[test]
    fn halves_round_to_the_even_neighbour_in_both_directions() {
        assert_eq!(round_half_to_even(4.5), 4.0);
        assert_eq!(round_half_to_even(5.5), 6.0);
        assert_eq!(round_half_to_even(2.5), 2.0);
        assert_eq!(round_half_to_even(3.5), 4.0);
        assert_eq!(round_half_to_even(4.4), 4.0);
        assert_eq!(round_half_to_even(4.6), 5.0);
    }

    #[test]
    fn the_difficulty_multiplier_follows_the_documented_formula() {
        let m = map("[Difficulty]\nHPDrainRate:5\nCircleSize:5\nOverallDifficulty:5\n\n[HitObjects]\n0,0,1000,1,0\n0,0,2000,1,0\n");

        assert_eq!(difficulty_multiplier(&m, 2, 1.0), 4);

        assert_eq!(difficulty_multiplier(&m, 0, 1.0), 2);
    }

    #[test]
    fn density_is_clamped_so_a_stream_map_is_not_worth_double() {
        let m = map("[Difficulty]\nHPDrainRate:5\nCircleSize:5\nOverallDifficulty:5\n\n[HitObjects]\n0,0,1000,1,0\n");
        assert_eq!(
            difficulty_multiplier(&m, 100, 1.0),
            difficulty_multiplier(&m, 10_000, 1.0)
        );
    }

    #[test]
    fn breaks_come_out_of_the_drain_length() {
        let plain = map("[HitObjects]\n0,0,1000,1,0\n0,0,11000,1,0\n");
        assert!((drain_seconds(&plain) - 10.0).abs() < 1e-9);

        let with_break =
            map("[Events]\n2,3000,7000\n\n[HitObjects]\n0,0,1000,1,0\n0,0,11000,1,0\n");
        assert!(
            (drain_seconds(&with_break) - 6.0).abs() < 1e-9,
            "{}",
            drain_seconds(&with_break)
        );
    }

    #[test]
    fn the_mods_scale_stables_score() {
        assert!((stable_mod_multiplier(Mods::new(0)) - 1.0).abs() < 1e-9);
        assert!((stable_mod_multiplier(Mods::new(bits::NO_FAIL)) - 0.5).abs() < 1e-9);

        let hdhr = stable_mod_multiplier(Mods::new(bits::HIDDEN | bits::HARD_ROCK));
        assert!((hdhr - 1.06 * 1.06).abs() < 1e-9, "{hdhr}");

        let nc = stable_mod_multiplier(Mods::new(bits::NIGHTCORE | bits::DOUBLE_TIME));
        assert!((nc - 1.12).abs() < 1e-9, "{nc}");

        assert_eq!(
            stable_mod_multiplier(Mods::new(bits::RELAX | bits::HIDDEN)),
            0.0
        );
    }

    #[test]
    fn the_slider_summary_is_not_paid_twice_in_lazer() {
        assert_eq!(lazer_max_value(Part::Slider), 0.0);
    }
}
