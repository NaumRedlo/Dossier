use dossier_assay::performance::Score;
use dossier_beatmap::Beatmap;
use dossier_replay::{bits, Mods};

struct Play {
    map: Beatmap,
    label: String,
    mods: Mods,
    score: Score,
    expected: serde_json::Value,
}

fn mods_of(key: &str) -> Option<Mods> {
    if key == "NM" {
        return Some(Mods::new(0));
    }
    let mut raw = 0u32;
    for pair in key.as_bytes().chunks(2) {
        raw |= match std::str::from_utf8(pair).ok()? {
            "CL" => 0,
            "EZ" => bits::EASY,
            "HD" => bits::HIDDEN,
            "HR" => bits::HARD_ROCK,
            "DT" => bits::DOUBLE_TIME,
            "HT" => bits::HALF_TIME,
            "FL" => bits::FLASHLIGHT,
            "NC" => bits::NIGHTCORE | bits::DOUBLE_TIME,
            _ => return None,
        };
    }
    Some(Mods::new(raw))
}

fn plays() -> Vec<Play> {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let text = std::fs::read_to_string(dir.join("scores.json"))
        .expect("corpus/scores.json — build it with scripts/pp_scores.py");
    let corpus: serde_json::Value = serde_json::from_str(&text).expect("valid json");

    corpus["scores"]
        .as_array()
        .expect("scores")
        .iter()
        .filter_map(|entry| {
            let id = entry["beatmap_id"].as_u64()?;
            let key = entry["mods"].as_str()?;
            let mods = mods_of(key)?;
            let map = Beatmap::parse(
                &std::fs::read_to_string(dir.join("maps").join(format!("{id}.osu"))).ok()?,
            )
            .ok()?;
            let stats = &entry["statistics"];
            let get = |name: &str| stats[name].as_u64().unwrap_or(0) as u32;
            Some(Play {
                map,
                label: format!("{id} {key} {}", entry["play"].as_str().unwrap_or("?")),
                mods,
                score: Score {
                    max_combo: entry["combo"].as_u64().unwrap_or(0) as u32,
                    great: get("great"),
                    ok: get("ok"),
                    meh: get("meh"),
                    miss: get("miss"),
                    slider_tail_hit: get("slider_tail_hit"),
                    large_tick_miss: get("large_tick_miss"),
                    classic: key.contains("CL"),
                    legacy_total_score: entry["legacy_total_score"]
                        .as_u64()
                        .filter(|total| *total > 0),

                    accuracy: entry["accuracy"].as_f64().map(|percent| percent / 100.0),
                },
                expected: entry["performance"].clone(),
            })
        })
        .collect()
}

#[test]
fn the_score_corpus_is_there_and_covers_the_awkward_plays() {
    let plays = plays();
    assert!(plays.len() >= 200, "only {} plays", plays.len());

    assert!(
        plays.iter().any(|p| p.score.miss > 10),
        "no ruinous play to test the clamps"
    );
    assert!(
        plays
            .iter()
            .any(|p| p.score.miss == 0 && p.score.max_combo > 0),
        "no unbroken play"
    );
}

#[test]
fn the_effective_miss_count_is_the_one_ppy_reports() {
    let mut worst = (0.0f64, String::from("nothing"));
    let mut checked = 0;
    for play in plays() {
        let Some(theirs) = play.expected["effective_miss_count"].as_f64() else {
            continue;
        };
        let attributes = dossier_assay::attributes(&play.map, play.mods);
        let ours =
            dossier_assay::performance::effective(&play.score, &attributes, play.mods).miss_count;
        checked += 1;
        let off = (ours - theirs).abs() / theirs.abs().max(1.0);
        if off > worst.0 {
            worst = (
                off,
                format!("{}: наш {ours:.4}, ppy {theirs:.4}", play.label),
            );
        }
    }
    assert!(checked >= 200, "only {checked} plays");
    assert!(
        worst.0 < 0.005,
        "худшее расхождение {:.2}% на {checked} плеях — {}",
        worst.0 * 100.0,
        worst.1
    );
}

#[test]
fn a_lazer_score_has_no_slider_breaks_to_estimate() {
    for play in plays().into_iter().filter(|play| !play.score.classic) {
        let attributes = dossier_assay::attributes(&play.map, play.mods);
        let effective = dossier_assay::performance::effective(&play.score, &attributes, play.mods);
        assert_eq!(effective.aim_slider_breaks, 0.0, "{}", play.label);
        assert_eq!(effective.speed_slider_breaks, 0.0, "{}", play.label);
        assert_eq!(
            play.expected["aim_estimated_slider_breaks"].as_f64(),
            Some(0.0)
        );
    }
}

fn worst_component(
    field: &str,
    ours: impl Fn(&Play, &dossier_assay::Attributes) -> f64,
) -> (usize, f64, String) {
    let mut worst = (0.0f64, String::from("nothing"));
    let mut checked = 0;
    for play in plays() {
        let Some(theirs) = play.expected[field].as_f64() else {
            continue;
        };
        let attributes = dossier_assay::attributes(&play.map, play.mods);
        let mine = ours(&play, &attributes);
        checked += 1;
        let off = (mine - theirs).abs() / theirs.abs().max(1.0);
        if off > worst.0 {
            worst = (
                off,
                format!("{}: наш {mine:.4}, ppy {theirs:.4}", play.label),
            );
        }
    }
    (checked, worst.0, worst.1)
}

fn great_window(play: &Play) -> f64 {
    2.0 * windows(play).0
}

fn windows(play: &Play) -> (f64, f64, f64) {
    let difficulty = dossier_sim::Timeline::build(&play.map, play.mods).difficulty;
    let rate = play.mods.speed_multiplier();
    let at = |min, mid, max| {
        (dossier_beatmap::difficulty_range(difficulty.overall_difficulty, min, mid, max).floor()
            - 0.5)
            / rate
    };
    (
        at(80.0, 50.0, 20.0),
        at(140.0, 100.0, 60.0),
        at(200.0, 150.0, 100.0),
    )
}

#[test]
fn the_aim_component_is_the_one_ppy_reports() {
    let (checked, off, what) = worst_component("aim", |play, attributes| {
        let effective = dossier_assay::performance::effective(&play.score, attributes, play.mods);
        dossier_assay::performance::aim_value(&play.score, attributes, &effective)
    });
    assert!(checked >= 200, "only {checked} plays");
    assert!(
        off < 0.001,
        "худшее расхождение {:.2}% на {checked} плеях — {what}",
        off * 100.0
    );
}

#[test]
fn the_accuracy_component_is_the_one_ppy_reports() {
    let (checked, off, what) = worst_component("accuracy", |play, attributes| {
        dossier_assay::performance::accuracy_value(
            &play.score,
            attributes,
            dossier_assay::performance::overall_difficulty(great_window(play)),
        )
    });
    assert!(checked >= 200, "only {checked} plays");
    assert!(
        off < 0.001,
        "худшее расхождение {:.2}% на {checked} плеях — {what}",
        off * 100.0
    );
}

#[test]
fn the_speed_deviation_is_the_one_ppy_reports() {
    let (checked, off, what) = worst_component("speed_deviation", |play, attributes| {
        dossier_assay::performance::speed_deviation(&play.score, attributes, windows(play))
            .unwrap_or(0.0)
    });
    assert!(checked >= 200, "only {checked} plays");
    assert!(
        off < 0.001,
        "худшее расхождение {:.2}% на {checked} плеях — {what}",
        off * 100.0
    );
}

#[test]
fn the_speed_component_is_the_one_ppy_reports() {
    let (checked, off, what) = worst_component("speed", |play, attributes| {
        let effective = dossier_assay::performance::effective(&play.score, attributes, play.mods);
        let deviation =
            dossier_assay::performance::speed_deviation(&play.score, attributes, windows(play));
        dossier_assay::performance::speed_value(
            &play.score,
            attributes,
            &effective,
            deviation,
            false,
        )
    });
    assert!(checked >= 200, "only {checked} plays");
    assert!(
        off < 0.001,
        "худшее расхождение {:.2}% на {checked} плеях — {what}",
        off * 100.0
    );
}

#[test]
fn the_whole_thing_is_the_pp_ppy_reports() {
    let (checked, off, what) = worst_component("pp", |play, attributes| {
        dossier_assay::performance::performance(&play.score, attributes, play.mods).pp
    });
    assert!(checked >= 200, "only {checked} plays");
    assert!(
        off < 0.001,
        "худшее расхождение {:.2}% на {checked} плеях — {what}",
        off * 100.0
    );
}

#[test]
fn the_reading_component_is_the_one_ppy_reports() {
    let (checked, off, what) = worst_component("reading", |play, attributes| {
        let effective = dossier_assay::performance::effective(&play.score, attributes, play.mods);
        dossier_assay::performance::reading_value(&play.score, attributes, &effective)
    });
    assert!(checked >= 200, "only {checked} plays");
    assert!(
        off < 0.15,
        "худшее расхождение {:.2}% на {checked} плеях — {what}",
        off * 100.0
    );
}

#[test]
fn a_classic_score_is_read_out_of_its_total() {
    let classic: Vec<_> = plays()
        .into_iter()
        .filter(|play| play.score.classic)
        .collect();
    assert!(classic.len() >= 100, "only {} classic plays", classic.len());
    assert!(
        classic
            .iter()
            .all(|play| play.score.legacy_total_score.is_some()),
        "a classic play with no total to read"
    );

    let mut worst = (0.0f64, String::from("nothing"));
    for play in &classic {
        let Some(theirs) = play.expected["effective_miss_count"].as_f64() else {
            continue;
        };
        let attributes = dossier_assay::attributes(&play.map, play.mods);
        let ours =
            dossier_assay::performance::effective(&play.score, &attributes, play.mods).miss_count;
        let off = (ours - theirs).abs() / theirs.abs().max(1.0);
        if off > worst.0 {
            worst = (
                off,
                format!("{}: наш {ours:.4}, ppy {theirs:.4}", play.label),
            );
        }
    }
    assert!(
        worst.0 < 0.005,
        "худшее расхождение {:.2}% — {}",
        worst.0 * 100.0,
        worst.1
    );
}
