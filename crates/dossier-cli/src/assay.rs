use std::path::Path;

use dossier_assay::performance::Score;
use dossier_beatmap::Beatmap;
use dossier_replay::{bits, Mods};

pub fn parse_mods(text: &str) -> Result<Mods, String> {
    let cleaned: String = text
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase();
    let mut raw = 0u32;
    for pair in cleaned.as_bytes().chunks(2) {
        let name = std::str::from_utf8(pair).map_err(|_| "mods must be acronyms".to_owned())?;
        raw |= match name {
            "NF" => bits::NO_FAIL,
            "EZ" => bits::EASY,
            "TD" => bits::TOUCH_DEVICE,
            "HD" => bits::HIDDEN,
            "HR" => bits::HARD_ROCK,
            "SD" => bits::SUDDEN_DEATH,
            "DT" => bits::DOUBLE_TIME,
            "RX" => bits::RELAX,
            "HT" => bits::HALF_TIME,
            "NC" => bits::NIGHTCORE | bits::DOUBLE_TIME,
            "FL" => bits::FLASHLIGHT,
            "SO" => bits::SPUN_OUT,
            "AP" => bits::AUTOPILOT,
            "PF" => bits::PERFECT | bits::SUDDEN_DEATH,

            "V2" => bits::SCORE_V2,
            "AT" => bits::AUTOPLAY,
            "CN" => bits::CINEMA,
            "RD" => bits::RANDOM,
            "TP" => bits::TARGET,
            "MR" => bits::MIRROR,

            "CL" | "" => 0,
            other => return Err(format!("no such mod: {other}")),
        };
    }
    Ok(Mods::new(raw))
}

#[allow(clippy::too_many_arguments)]
pub fn score_from(
    attributes: &dossier_assay::Attributes,
    accuracy: Option<f64>,
    combo: Option<u32>,
    misses: u32,
    n300: Option<u32>,
    n100: Option<u32>,
    n50: Option<u32>,
    slider_ends: Option<u32>,
    large_tick_misses: u32,
    classic: bool,
    legacy_total: Option<u64>,
) -> Score {
    let objects = attributes.hit_circle_count + attributes.slider_count + attributes.spinner_count;

    let (great, ok, meh) = match (n300, n100, n50) {
        (Some(great), Some(ok), Some(meh)) => (great, ok, meh),
        _ => {
            let target = accuracy.unwrap_or(100.0) / 100.0;
            let judged = objects.saturating_sub(misses);
            let meh = n50.unwrap_or(0);

            let want = target * 300.0 * f64::from(objects);
            let have = 50.0 * f64::from(meh);
            let hundreds = ((300.0 * f64::from(judged.saturating_sub(meh)) + have - want) / 200.0)
                .round()
                .clamp(0.0, f64::from(judged.saturating_sub(meh)));
            let ok = hundreds as u32;
            (judged.saturating_sub(meh).saturating_sub(ok), ok, meh)
        }
    };

    let mut score = Score {
        max_combo: combo.unwrap_or(attributes.max_combo),
        great,
        ok,
        meh,
        miss: misses,

        slider_tail_hit: slider_ends.unwrap_or(attributes.slider_count),
        large_tick_miss: large_tick_misses,
        classic,
        legacy_total_score: legacy_total,
        accuracy: None,
    };

    score.accuracy = match accuracy {
        Some(percent) => Some((percent / 100.0).clamp(0.0, 1.0)),
        None => Some(score.accuracy()),
    };
    score
}

pub fn run(map_path: &Path, mods: Mods, play: Option<Score>) -> Result<String, String> {
    let text = std::fs::read_to_string(map_path)
        .map_err(|error| format!("could not read {}: {error}", map_path.display()))?;
    let map = Beatmap::parse(&text).map_err(|error| format!("could not parse the map: {error}"))?;
    let attributes = dossier_assay::attributes(&map, mods);

    let mut out = String::from("{\n");
    out.push_str(&format!("  \"star_rating\": {},\n", attributes.star_rating));
    out.push_str(&format!("  \"max_combo\": {},\n", attributes.max_combo));
    out.push_str(&format!(
        "  \"aim_difficulty\": {},\n",
        attributes.aim_difficulty
    ));
    out.push_str(&format!(
        "  \"speed_difficulty\": {},\n",
        attributes.speed_difficulty
    ));
    out.push_str(&format!(
        "  \"reading_difficulty\": {},\n",
        attributes.reading_difficulty
    ));
    out.push_str(&format!(
        "  \"flashlight_difficulty\": {},\n",
        attributes.flashlight_difficulty
    ));
    out.push_str(&format!(
        "  \"slider_factor\": {},\n",
        attributes.slider_factor
    ));
    out.push_str(&format!(
        "  \"hit_circle_count\": {},\n",
        attributes.hit_circle_count
    ));
    out.push_str(&format!(
        "  \"slider_count\": {},\n",
        attributes.slider_count
    ));
    out.push_str(&format!(
        "  \"spinner_count\": {}",
        attributes.spinner_count
    ));

    if let Some(play) = play {
        let performance = dossier_assay::performance::performance(&play, &attributes, mods);

        let objects =
            attributes.hit_circle_count + attributes.slider_count + attributes.spinner_count;
        let mut unbroken = play.clone();
        unbroken.great += unbroken.miss;
        unbroken.miss = 0;
        unbroken.max_combo = attributes.max_combo;
        unbroken.large_tick_miss = 0;
        unbroken.slider_tail_hit = attributes.slider_count;

        unbroken.accuracy =
            Some(unbroken.lazer_accuracy(attributes.slider_count, attributes.large_tick_count));
        let if_unbroken = dossier_assay::performance::performance(&unbroken, &attributes, mods);

        let perfect = Score {
            max_combo: attributes.max_combo,
            great: objects,
            ok: 0,
            meh: 0,
            miss: 0,
            slider_tail_hit: attributes.slider_count,
            large_tick_miss: 0,
            classic: play.classic,
            legacy_total_score: None,
            accuracy: Some(1.0),
        };

        let if_perfect = dossier_assay::performance::performance(&perfect, &attributes, mods);

        out.push_str(",\n");
        out.push_str(&format!("  \"pp\": {},\n", performance.pp));
        out.push_str(&format!("  \"pp_if_unbroken\": {},\n", if_unbroken.pp));
        out.push_str(&format!("  \"pp_if_perfect\": {},\n", if_perfect.pp));
        out.push_str(&format!("  \"accuracy\": {},\n", play.accuracy() * 100.0));
        out.push_str(&format!("  \"aim\": {},\n", performance.aim));
        out.push_str(&format!("  \"speed\": {},\n", performance.speed));
        out.push_str(&format!(
            "  \"accuracy_value\": {},\n",
            performance.accuracy
        ));
        out.push_str(&format!("  \"reading\": {},\n", performance.reading));
        out.push_str(&format!("  \"flashlight\": {},\n", performance.flashlight));
        out.push_str(&format!(
            "  \"effective_miss_count\": {}",
            performance.effective_miss_count
        ));
    }
    out.push_str("\n}\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mods_are_read_however_they_are_written() {
        for text in ["HDDT", "hddt", "HD,DT", "hd dt"] {
            let mods = parse_mods(text).expect(text);
            assert!(
                mods.contains(bits::HIDDEN) && mods.contains(bits::DOUBLE_TIME),
                "{text}"
            );
        }
    }

    #[test]
    fn classic_is_accepted_and_carries_no_bit() {
        let mods = parse_mods("HDCL").expect("HDCL");
        assert!(mods.contains(bits::HIDDEN));
        assert_eq!(parse_mods("CL").expect("CL").raw(), 0);
    }

    #[test]
    fn a_mod_nobody_has_is_refused_rather_than_ignored() {
        assert!(parse_mods("ZZ").is_err());
    }

    fn attributes() -> dossier_assay::Attributes {
        dossier_assay::Attributes {
            max_combo: 1000,
            hit_circle_count: 600,
            slider_count: 300,
            spinner_count: 4,
            ..Default::default()
        }
    }

    #[test]
    fn an_accuracy_alone_is_turned_into_the_judgements_that_produce_it() {
        let score = score_from(
            &attributes(),
            Some(98.0),
            None,
            0,
            None,
            None,
            None,
            None,
            0,
            false,
            None,
        );
        let objects = 904;
        assert_eq!(score.great + score.ok + score.meh + score.miss, objects);

        let from_counts = (300.0 * f64::from(score.great) + 100.0 * f64::from(score.ok))
            / (300.0 * f64::from(objects));
        assert!((from_counts - 0.98).abs() < 0.001, "{from_counts}");
    }

    #[test]
    fn the_accuracy_a_caller_gives_is_the_one_used() {
        let score = score_from(
            &attributes(),
            Some(99.5),
            None,
            1,
            Some(800),
            Some(100),
            Some(3),
            None,
            0,
            false,
            None,
        );
        assert!((score.accuracy() - 0.995).abs() < 1e-12);
        assert_eq!(score.great, 800, "the counts are still the caller's");
    }

    #[test]
    fn a_play_that_said_nothing_about_slider_ends_is_assumed_to_have_caught_them() {
        let score = score_from(
            &attributes(),
            Some(97.0),
            None,
            0,
            None,
            None,
            None,
            None,
            0,
            false,
            None,
        );
        assert_eq!(score.slider_tail_hit, 300);
    }
}
