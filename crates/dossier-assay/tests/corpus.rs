use std::collections::BTreeMap;
use std::path::PathBuf;

use dossier_beatmap::Beatmap;
use dossier_replay::{bits, Mods};

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus")
}

fn mods_of(key: &str) -> Option<Mods> {
    let mut raw = 0u32;
    if key == "NM" {
        return Some(Mods::new(0));
    }
    for pair in key.as_bytes().chunks(2) {
        raw |= match std::str::from_utf8(pair).ok()? {
            "NF" => bits::NO_FAIL,
            "EZ" => bits::EASY,
            "TD" => bits::TOUCH_DEVICE,
            "HD" => bits::HIDDEN,
            "HR" => bits::HARD_ROCK,
            "DT" => bits::DOUBLE_TIME,
            "HT" => bits::HALF_TIME,
            "NC" => bits::NIGHTCORE | bits::DOUBLE_TIME,
            "FL" => bits::FLASHLIGHT,
            _ => return None,
        };
    }
    Some(Mods::new(raw))
}

struct Case {
    map: Beatmap,
    title: String,

    expected: BTreeMap<String, serde_json::Value>,
}

fn cases() -> Vec<Case> {
    let dir = corpus_dir();
    let text = std::fs::read_to_string(dir.join("expected.json"))
        .expect("corpus/expected.json — build it with scripts/pp_corpus.py");
    let corpus: serde_json::Value = serde_json::from_str(&text).expect("valid json");

    corpus["maps"]
        .as_array()
        .expect("a list of maps")
        .iter()
        .map(|entry| {
            let id = entry["beatmap_id"].as_u64().expect("an id");
            let path = dir.join("maps").join(format!("{id}.osu"));
            let map = Beatmap::parse(&std::fs::read_to_string(&path).expect("the map"))
                .unwrap_or_else(|e| panic!("{id} did not parse: {e}"));
            let expected = entry["attributes"]
                .as_object()
                .expect("attributes per mod set")
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            Case {
                map,
                title: format!(
                    "{} [{}] ({id})",
                    entry["title"].as_str().unwrap_or("?"),
                    entry["version"].as_str().unwrap_or("?")
                ),
                expected,
            }
        })
        .collect()
}

#[test]
fn the_corpus_is_there_and_is_worth_checking_against() {
    let cases = cases();
    assert!(cases.len() >= 5, "only {} maps in the corpus", cases.len());
    let pairs: usize = cases.iter().map(|c| c.expected.len()).sum();
    assert!(
        pairs >= 100,
        "only {pairs} map-and-mods pairs to check against"
    );
}

#[test]
fn the_greatest_combo_a_map_allows_is_the_one_ppy_reports() {
    let mut checked = 0;
    let mut wrong = Vec::new();
    for case in cases() {
        for (key, attrs) in &case.expected {
            let Some(mods) = mods_of(key) else { continue };
            let Some(theirs) = attrs["max_combo"].as_u64() else {
                continue;
            };
            let ours = u64::from(dossier_assay::max_combo(&case.map, mods));
            checked += 1;
            if ours != theirs {
                wrong.push(format!(
                    "  {} {key}: наш {ours}, ppy {theirs} (разница {})",
                    case.title,
                    ours as i64 - theirs as i64
                ));
            }
        }
    }
    assert!(checked > 0, "nothing was checked");
    assert!(
        wrong.is_empty(),
        "{} of {checked} disagree with ppy:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}

#[test]
fn the_pressing_difficulty_is_the_one_ppy_reports() {
    let mut checked = 0;
    let mut worst: Option<(String, f64, f64)> = None;
    for case in cases() {
        for (key, attrs) in &case.expected {
            let Some(mods) = mods_of(key) else { continue };
            let Some(theirs) = attrs["speed_difficulty"].as_f64() else {
                continue;
            };
            let ours = dossier_assay::speed_difficulty(&case.map, mods);
            checked += 1;
            let off = if theirs > 0.0 {
                (ours - theirs).abs() / theirs
            } else {
                (ours - theirs).abs()
            };
            if worst.as_ref().is_none_or(|(_, _, w)| off > *w) {
                worst = Some((
                    format!("{} {key}: наш {ours:.4}, ppy {theirs:.4}", case.title),
                    ours,
                    off,
                ));
            }
        }
    }
    assert!(checked > 0, "nothing was checked");
    let (what, _, off) = worst.expect("something to report");

    assert!(
        off < 0.001,
        "худшее расхождение {:.2}% на {checked} парах — {what}",
        off * 100.0
    );
}

fn worst_against_ppy(field: &str, ours: impl Fn(&Beatmap, Mods) -> f64) -> (usize, f64, String) {
    let mut checked = 0;
    let mut worst = (0.0, String::from("nothing"));
    for case in cases() {
        for (key, attrs) in &case.expected {
            let Some(mods) = mods_of(key) else { continue };
            let Some(theirs) = attrs[field].as_f64() else {
                continue;
            };
            let mine = ours(&case.map, mods);
            checked += 1;
            let scale = theirs.abs().max(1e-9);
            let off = (mine - theirs).abs() / scale;
            if off > worst.0 {
                worst = (
                    off,
                    format!("{} {key}: наш {mine:.6}, ppy {theirs:.6}", case.title),
                );
            }
        }
    }
    (checked, worst.0, worst.1)
}

#[test]
fn the_aiming_difficulty_is_the_one_ppy_reports() {
    let (checked, off, what) = worst_against_ppy("aim_difficulty", |map, mods| {
        dossier_assay::aim_difficulty(map, mods).0
    });
    assert!(checked >= 150, "only {checked} pairs");
    assert!(
        off < 0.001,
        "худшее расхождение {:.2}% на {checked} парах — {what}",
        off * 100.0
    );
}

#[test]
fn the_slider_factor_is_the_one_ppy_reports() {
    let (checked, off, what) = worst_against_ppy("slider_factor", |map, mods| {
        dossier_assay::aim_difficulty(map, mods).1
    });
    assert!(checked >= 150, "only {checked} pairs");
    assert!(
        off < 0.005,
        "худшее расхождение {:.2}% на {checked} парах — {what}",
        off * 100.0
    );
}

#[test]
fn the_counts_of_difficult_things_are_the_ones_ppy_reports() {
    for (field, get) in [
        (
            "aim_difficult_slider_count",
            (|a: &dossier_assay::Attributes| a.aim_difficult_slider_count) as fn(&_) -> f64,
        ),
        ("aim_difficult_strain_count", |a| {
            a.aim_difficult_strain_count
        }),
        ("speed_difficult_strain_count", |a| {
            a.speed_difficult_strain_count
        }),
    ] {
        let (checked, off, what) = worst_against_ppy(field, |map, mods| {
            get(&dossier_assay::attributes(map, mods))
        });
        assert!(checked >= 150, "{field}: only {checked} pairs");
        assert!(
            off < 0.005,
            "{field}: худшее расхождение {:.2}% — {what}",
            off * 100.0
        );
    }
}

#[test]
fn the_reading_difficulty_is_close_to_the_one_ppy_reports() {
    let (checked, off, what) = worst_against_ppy("reading_difficulty", |map, mods| {
        dossier_assay::attributes(map, mods).reading_difficulty
    });
    assert!(checked >= 150, "only {checked} pairs");
    assert!(
        off < 0.04,
        "худшее расхождение {:.2}% на {checked} парах — {what}",
        off * 100.0
    );
}

#[test]
fn the_count_of_hard_to_read_notes_is_close_too() {
    let (checked, off, what) = worst_against_ppy("reading_difficult_note_count", |map, mods| {
        dossier_assay::attributes(map, mods).reading_difficult_note_count
    });
    assert!(checked >= 150, "only {checked} pairs");
    assert!(
        off < 0.03,
        "худшее расхождение {:.2}% на {checked} парах — {what}",
        off * 100.0
    );
}

#[test]
fn the_flashlight_difficulty_is_close_to_the_one_ppy_reports() {
    let (checked, off, what) = worst_against_ppy("flashlight_difficulty", |map, mods| {
        dossier_assay::attributes(map, mods).flashlight_difficulty
    });
    assert!(
        checked >= 20,
        "only {checked} pairs — is the corpus missing the field?"
    );
    assert!(
        off < 0.03,
        "худшее расхождение {:.2}% на {checked} парах — {what}",
        off * 100.0
    );
}

#[test]
fn the_star_rating_is_close_to_the_one_ppy_reports() {
    let (checked, off, what) = worst_against_ppy("star_rating", |map, mods| {
        dossier_assay::attributes(map, mods).star_rating
    });
    assert!(checked >= 150, "only {checked} pairs");
    assert!(
        off < 0.015,
        "худшее расхождение {:.2}% на {checked} парах — {what}",
        off * 100.0
    );
}
