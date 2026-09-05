use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

#[derive(Clone, Debug, PartialEq)]
pub struct Expectation {
    pub replay_md5: String,

    pub beatmap_md5: String,

    pub beatmap_id: Option<u32>,

    pub error: u32,

    pub combo: i64,

    pub score: Option<f64>,

    pub name: String,
}

impl Expectation {
    pub fn worse_than(&self, error: u32, combo: i64, score: Option<f64>) -> Option<String> {
        if error > self.error {
            return Some(format!("count error {} → {error}", self.error));
        }
        if combo.abs() > self.combo.abs() {
            return Some(format!("combo {:+} → {combo:+}", self.combo));
        }

        match (self.score, score) {
            (Some(was), Some(now)) if now.abs() > was.abs() + 0.01 => {
                Some(format!("score {was:+.2}% → {now:+.2}%"))
            }

            (Some(was), None) => Some(format!("score {was:+.2}% → not comparable")),
            _ => None,
        }
    }
}

pub fn read(path: &Path) -> Result<BTreeMap<String, Expectation>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut rows = BTreeMap::new();
    for (number, line) in text.lines().enumerate() {
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let [replay_md5, beatmap_md5, beatmap_id, error, combo, score, name] = fields[..] else {
            return Err(format!(
                "{}:{}: expected 7 tab-separated fields, found {}",
                path.display(),
                number + 1,
                fields.len()
            ));
        };
        let at = |what: &str| format!("{}:{}: bad {what}", path.display(), number + 1);
        rows.insert(
            replay_md5.to_owned(),
            Expectation {
                replay_md5: replay_md5.to_owned(),
                beatmap_md5: beatmap_md5.to_owned(),
                beatmap_id: match beatmap_id {
                    "-" => None,
                    id => Some(id.parse().map_err(|_| at("beatmap id"))?),
                },
                error: error.parse().map_err(|_| at("count error"))?,
                combo: combo.parse().map_err(|_| at("combo"))?,
                score: match score {
                    "-" => None,
                    off => Some(off.parse().map_err(|_| at("score"))?),
                },
                name: name.to_owned(),
            },
        );
    }
    Ok(rows)
}

pub fn after_run(
    was: &BTreeMap<String, Expectation>,
    measured: Vec<Expectation>,
    on_disk: &std::collections::BTreeSet<String>,
    prune: bool,
) -> (BTreeMap<String, Expectation>, usize) {
    let mut rows = was.clone();
    for mut fresh in measured {
        fresh.beatmap_id = fresh
            .beatmap_id
            .or_else(|| was.get(&fresh.replay_md5).and_then(|old| old.beatmap_id));
        rows.insert(fresh.replay_md5.clone(), fresh);
    }
    let dropped = if prune {
        let before = rows.len();
        rows.retain(|md5, _| on_disk.contains(md5));
        before - rows.len()
    } else {
        0
    };
    (rows, dropped)
}

pub fn write(path: &Path, rows: &BTreeMap<String, Expectation>) -> Result<(), String> {
    let mut out = String::from(
        "# The corpus: which replays it is made of, and what each one does.\n\
         #\n\
         # Written by `dossier corpus --expect <this file> --update-expect`, and\n\
         # the beatmap ids by `tools/fetch-maps.py --manifest <this file>`. Do not\n\
         # edit by hand — a row that disagrees with what the engine measures is\n\
         # worse than no row at all.\n\
         #\n\
         # replay_md5\tbeatmap_md5\tbeatmap_id\terror\tcombo\tscore\tname\n",
    );
    for row in rows.values() {
        let id = row
            .beatmap_id
            .map_or_else(|| "-".to_owned(), |id| id.to_string());
        let score = row
            .score
            .map_or_else(|| "-".to_owned(), |off| format!("{off:.2}"));
        let _ = writeln!(
            out,
            "{}\t{}\t{id}\t{}\t{}\t{score}\t{}",
            row.replay_md5, row.beatmap_md5, row.error, row.combo, row.name
        );
    }

    let temporary = path.with_extension("tsv.part");
    std::fs::write(&temporary, out).map_err(|e| format!("{}: {e}", temporary.display()))?;
    std::fs::rename(&temporary, path).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> Expectation {
        Expectation {
            replay_md5: "a".repeat(32),
            beatmap_md5: "b".repeat(32),
            beatmap_id: Some(12345),
            error: 4,
            combo: -1,
            score: Some(0.12),
            name: "somebody — a map".to_owned(),
        }
    }

    #[test]
    fn a_run_that_matches_is_not_worse() {
        assert_eq!(row().worse_than(4, -1, Some(0.12)), None);
    }

    #[test]
    fn improving_is_never_a_failure() {
        assert_eq!(row().worse_than(0, 0, Some(0.0)), None);
    }

    #[test]
    fn more_count_error_is_worse() {
        assert!(row().worse_than(5, -1, Some(0.12)).is_some());
    }

    #[test]
    fn combo_is_judged_by_distance_not_sign() {
        assert_eq!(row().worse_than(4, 1, Some(0.12)), None);
        assert!(row().worse_than(4, -2, Some(0.12)).is_some());
    }

    #[test]
    fn the_score_gets_a_hundredth_of_slack() {
        assert_eq!(row().worse_than(4, -1, Some(0.125)), None);
        assert!(row().worse_than(4, -1, Some(0.30)).is_some());
    }

    #[test]
    fn losing_the_ability_to_compare_a_score_is_a_regression() {
        assert!(row().worse_than(4, -1, None).is_some());
    }

    fn was() -> BTreeMap<String, Expectation> {
        let elsewhere = Expectation {
            replay_md5: "e".repeat(32),
            name: "a replay on the other machine".to_owned(),
            ..row()
        };
        BTreeMap::from([
            (row().replay_md5, row()),
            (elsewhere.replay_md5.clone(), elsewhere),
        ])
    }

    fn on_disk(hashes: &[&str]) -> std::collections::BTreeSet<String> {
        hashes.iter().map(|h| (*h).to_owned()).collect()
    }

    #[test]
    fn a_partial_run_keeps_the_rows_it_did_not_see() {
        let measured = vec![Expectation { error: 0, ..row() }];
        let (rows, dropped) = after_run(&was(), measured, &on_disk(&[&"a".repeat(32)]), false);
        assert_eq!(rows.len(), 2);
        assert_eq!(dropped, 0);
        assert_eq!(rows[&"e".repeat(32)].name, "a replay on the other machine");

        assert_eq!(rows[&"a".repeat(32)].error, 0);
    }

    #[test]
    fn a_replay_the_file_never_heard_of_is_added() {
        let arrival = Expectation {
            replay_md5: "f".repeat(32),
            beatmap_id: None,
            ..row()
        };
        let (rows, _) = after_run(&was(), vec![arrival], &on_disk(&[&"f".repeat(32)]), false);
        assert_eq!(rows.len(), 3);
        assert!(rows.contains_key(&"f".repeat(32)));
    }

    #[test]
    fn a_pinned_beatmap_id_survives_a_re_measurement() {
        let measured = vec![Expectation {
            beatmap_id: None,
            ..row()
        }];
        let (rows, _) = after_run(&was(), measured, &on_disk(&[&"a".repeat(32)]), false);
        assert_eq!(rows[&"a".repeat(32)].beatmap_id, Some(12345));
    }

    #[test]
    fn pruning_drops_what_is_not_on_the_disk() {
        let measured = vec![row()];
        let (rows, dropped) = after_run(&was(), measured, &on_disk(&[&"a".repeat(32)]), true);
        assert_eq!(dropped, 1);
        assert_eq!(rows.len(), 1);
        assert!(!rows.contains_key(&"e".repeat(32)));
    }

    #[test]
    fn pruning_spares_a_replay_that_is_here_and_could_not_be_judged() {
        let (rows, dropped) = after_run(
            &was(),
            Vec::new(),
            &on_disk(&["a".repeat(32).as_str(), "e".repeat(32).as_str()]),
            true,
        );
        assert_eq!(dropped, 0);
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn a_row_survives_being_written_and_read_back() {
        let directory =
            std::env::temp_dir().join(format!("dossier-manifest-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("a scratch directory");
        let path = directory.join("corpus.tsv");
        let mut rows = BTreeMap::new();
        rows.insert(row().replay_md5.clone(), row());

        let bare = Expectation {
            replay_md5: "c".repeat(32),
            beatmap_id: None,
            score: None,
            name: String::new(),
            ..row()
        };
        rows.insert(bare.replay_md5.clone(), bare);

        write(&path, &rows).expect("it writes");
        assert_eq!(read(&path).expect("it reads back"), rows);
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn a_malformed_row_is_named_rather_than_skipped() {
        let directory = std::env::temp_dir().join(format!("dossier-bad-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("a scratch directory");
        let path = directory.join("corpus.tsv");
        std::fs::write(&path, "# fine\nnot\tenough\tfields\n").expect("it writes");
        let error = read(&path).expect_err("three fields is not seven");
        assert!(error.contains(":2:"), "{error}");
        let _ = std::fs::remove_dir_all(&directory);
    }
}
