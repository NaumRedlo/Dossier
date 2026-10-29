#[path = "mania_corpus/frame_rounding.rs"]
mod frame_rounding;
#[path = "mania_corpus/stable_audit.rs"]
mod stable_audit;
#[path = "mania_corpus/timing_probe.rs"]
mod timing_probe;
use dossier_beatmap::Beatmap;
use dossier_replay::{GameMode, Replay};
use dossier_sim::mania::{
    judge::{Event, Judge, LazerRules, Part, ResultKind},
    Error, Playback, Timeline,
};
use md5::{Digest, Md5};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
};

fn tail_rounding_audit(
    events: &[Event],
    rules: LazerRules,
    expected: [u32; 6],
    observed: [u32; 6],
) -> (u32, bool) {
    let mut upgrades = [0i64; 5];
    let mut downgrades = [0i64; 5];
    let mut candidates = 0;
    for event in events {
        if event.part != Part::Tail || event.result as usize >= ResultKind::Meh as usize {
            continue;
        }
        let Some(offset) = event.offset_ms else {
            continue;
        };
        if rules.result_for(offset / 1.5) != Some(event.result) {
            continue;
        }
        let mut alternative = None;
        for offset in [offset - 0.5, offset + 0.5] {
            if let Some(result) = rules.result_for(offset / 1.5) {
                if result != event.result && (result as usize) < ResultKind::Meh as usize {
                    alternative = Some(result);
                }
            }
        }
        if let Some(other) = alternative {
            let from = event.result as usize;
            let to = other as usize;
            if from.abs_diff(to) != 1 {
                continue;
            }
            if to < from {
                upgrades[to] += 1;
            } else {
                downgrades[from] += 1;
            }
            candidates += 1;
        }
    }
    let mut balance = 0i64;
    let mut compatible = expected.iter().sum::<u32>() == observed.iter().sum::<u32>();
    for boundary in 0..5 {
        balance += i64::from(expected[boundary]) - i64::from(observed[boundary]);
        compatible &= balance <= upgrades[boundary] && -balance <= downgrades[boundary];
    }
    (candidates, compatible)
}
fn walk(path: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            walk(&entry.path(), files)?;
        } else if kind.is_file() {
            files.push(entry.path());
        }
    }
    Ok(())
}
fn judge_replay(
    playback: &Playback,
    replay: &Replay,
    od: f64,
) -> Result<(Judge, Option<LazerRules>), Error> {
    if replay.game_version < 30_000_000 {
        playback
            .judge_stable_with(playback.stable_rules(od)?)
            .map(|judge| (judge, None))
    } else {
        let rules = playback.lazer_rules(od)?;
        playback.judge_with(rules).map(|judge| (judge, Some(rules)))
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let usage = "usage: mania-corpus <corpus-directory> [--audit-rounding | --audit-stable | --trace <replay-file-md5> [--probe-frame <zero-based-index> <offset-ms>]]";
    let root = args.next().ok_or(usage)?;
    let mut audit_rounding = false;
    let mut audit_stable = false;
    let trace = match args.next() {
        None => None,
        Some(flag) if flag == "--audit-rounding" => {
            audit_rounding = true;
            None
        }
        Some(flag) if flag == "--audit-stable" => {
            audit_stable = true;
            None
        }
        Some(flag) if flag == "--trace" => {
            let hash = args.next().ok_or(usage)?.into_string().map_err(|_| usage)?;
            if hash.len() != 32 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(usage.into());
            }
            Some(hash.to_ascii_lowercase())
        }
        _ => return Err(usage.into()),
    };
    let probe = match args.next() {
        None => None,
        Some(flag) if flag == "--probe-frame" && trace.is_some() => {
            let index: usize = args
                .next()
                .ok_or(usage)?
                .into_string()
                .map_err(|_| usage)?
                .parse()?;
            let delta: f64 = args
                .next()
                .ok_or(usage)?
                .into_string()
                .map_err(|_| usage)?
                .parse()?;
            if !delta.is_finite() || delta.abs() > 0.5 {
                return Err("probe offset must be finite and within ±0.5 ms".into());
            }
            Some((index, delta))
        }
        _ => return Err(usage.into()),
    };
    if args.next().is_some() {
        return Err(usage.into());
    }
    let mut traced = false;
    let mut files = Vec::new();
    walk(Path::new(&root), &mut files)?;
    files.sort();
    let mut maps = BTreeMap::new();
    for path in &files {
        if path.extension().is_some_and(|ext| ext == "osu") {
            let bytes = fs::read(path)?;
            maps.insert(format!("{:x}", Md5::digest(&bytes)), path.clone());
        }
    }
    if trace.is_none() {
        let extra = if audit_rounding {
            "\ttiming_probe\tprobe_candidates\tprobe_trials\tprobe_rejected\tprobe_shifted_frames\tprobe_offsets"
        } else if audit_stable {
            stable_audit::HEADER
        } else {
            ""
        };
        println!("replay_md5\tmap_md5\tversion\tmods\tstatus\tcount_error\texpected\tobserved\tfile\ttail_rounding_candidates\ttail_rounding_compatible\tpolicy\ttail_frame_rounding\texpected_max_combo\tobserved_max_combo\tmax_combo_error\taccuracy_from_header_counts\tobserved_accuracy\texpected_score_without_mods\tobserved_score_without_mods\tscore_error{extra}");
    }
    for path in &files {
        if !path.extension().is_some_and(|ext| ext == "osr") {
            continue;
        }
        let bytes = fs::read(path)?;
        if bytes.first() != Some(&3) {
            continue;
        }
        let hash = format!("{:x}", Md5::digest(&bytes));
        if trace.as_ref().is_some_and(|wanted| wanted != &hash) {
            continue;
        }
        let name = path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .replace(['\t', '\r', '\n'], " ");
        let replay = match Replay::parse(&bytes) {
            Ok(replay) => replay,
            Err(error) => {
                if trace.is_some() {
                    return Err(error.into());
                }
                let extra = if audit_rounding {
                    "\tnot-applicable\t\t\t\t\t"
                } else if audit_stable {
                    stable_audit::NOT_MEASURED
                } else {
                    ""
                };
                println!("{hash}\t\t\t\tparse-error:{error}\t\t\t\t{name}\t\t\t\t\t\t\t\t\t\t\t\t{extra}");
                continue;
            }
        };
        assert_eq!(replay.mode, GameMode::Mania);
        let policy = if replay.game_version < 30_000_000 {
            "stable-scorev1-experimental"
        } else {
            "lazer-native-experimental"
        };
        let expected = [
            replay.hits.count_geki,
            replay.hits.count_300,
            replay.hits.count_katu,
            replay.hits.count_100,
            replay.hits.count_50,
            replay.hits.count_miss,
        ]
        .map(u32::from);
        let mods = replay
            .lazer_mods()
            .iter()
            .map(|m| m.acronym.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let mut observed = String::new();
        let mut error = String::new();
        let mut rounding_candidates = String::new();
        let mut rounding_compatible = String::new();
        let mut frame_rounding = String::new();
        let mut observed_combo = String::new();
        let mut combo_error = String::new();
        let mut header_accuracy = String::new();
        let mut observed_accuracy = String::new();
        let expected_score = replay
            .score_info
            .as_ref()
            .and_then(|s| s.total_score_without_mods);
        let expected_score_text = expected_score.map(|s| s.to_string()).unwrap_or_default();
        let mut observed_score = String::new();
        let mut score_error = String::new();
        let mut stable_report = if audit_stable {
            if replay.game_version < 30_000_000 {
                stable_audit::NOT_MEASURED
            } else {
                stable_audit::NOT_APPLICABLE
            }
            .to_string()
        } else {
            String::new()
        };
        let mut rounding_probe = if audit_rounding {
            "\tnot-applicable\t\t\t\t\t".to_string()
        } else {
            String::new()
        };
        let status = if let Some(map_path) = maps.get(&replay.beatmap_hash) {
            match fs::read_to_string(map_path)
                .map_err(|e| e.to_string())
                .and_then(|s| Beatmap::parse(&s).map_err(|e| e.to_string()))
            {
                Err(err) => format!("map-error:{err}"),
                Ok(map) => {
                    if trace.is_some() {
                        let playback = Playback::new(&map, &replay).inspect_err(|error| {
                            if let Error::InvalidFrame(index) = error {
                                let start = index.saturating_sub(2);
                                let end = (index + 3).min(replay.frames.len());
                                eprintln!("Invalid input at original frame {index}; context {start}..{end}: {:?}", &replay.frames[start..end]);
                            }
                        })?;
                        let (judge, rules, transitions) = if replay.game_version < 30_000_000 {
                            if probe.is_some() {
                                return Err(
                                    "frame probes support only the explicit lazer policy".into()
                                );
                            }
                            let (judge, transitions) = playback.trace_stable_with(
                                playback.stable_rules(map.difficulty.overall_difficulty)?,
                            )?;
                            (judge, None, transitions)
                        } else {
                            let (judge, rules) = judge_replay(
                                &playback,
                                &replay,
                                map.difficulty.overall_difficulty,
                            )?;
                            (judge, rules, Vec::new())
                        };
                        if let Some((index, delta)) = probe {
                            let frame = replay
                                .frames
                                .get(index)
                                .ok_or("probe frame index out of range")?;
                            let mut offsets = vec![0.0; replay.frames.len()];
                            offsets[index] = delta;
                            let adjusted = Judge::with_frame_offsets(
                                &playback.timeline,
                                &replay.frames,
                                &offsets,
                                rules.ok_or("probe requires lazer rules")?,
                            )?;
                            eprintln!("HYPOTHETICAL frame probe: original index={index}, time={} ms, mask={}, offset={delta} ms. Original trace on stdout is unchanged.", frame.time_ms, frame.x);
                            eprintln!(
                                "baseline: {:?}\nhypothesis: {:?}",
                                judge.final_counts(),
                                adjusted.final_counts()
                            );
                            eprintln!(
                                "baseline combo: {:?}\nhypothesis combo: {:?}",
                                judge.final_combo(),
                                adjusted.final_combo()
                            );
                            eprintln!(
                                "baseline score: {:?}\nhypothesis score: {:?}",
                                judge.final_score(),
                                adjusted.final_score()
                            );
                            let originals: BTreeMap<_, _> = judge
                                .events()
                                .iter()
                                .map(|event| ((event.object_index, event.part as u8), event))
                                .collect();
                            let alternatives: BTreeMap<_, _> = adjusted
                                .events()
                                .iter()
                                .map(|event| ((event.object_index, event.part as u8), event))
                                .collect();
                            let keys: std::collections::BTreeSet<_> =
                                originals.keys().chain(alternatives.keys()).collect();
                            let mut changed = 0;
                            for key in keys {
                                let old = originals.get(key);
                                let new = alternatives.get(key);
                                if old != new {
                                    changed += 1;
                                    eprintln!("probe object {}: {old:?} -> {new:?}", key.0);
                                }
                            }
                            eprintln!("probe changed events: {changed}");
                        }
                        let windows = if let Some(rules) = rules {
                            format!("{rules:?}")
                        } else {
                            format!(
                                "{:?}",
                                playback.stable_rules(map.difficulty.overall_difficulty)?
                            )
                        };
                        eprintln!(
                            "{name}\npolicy: {policy}\nexpected: {expected:?}\nlegacy mod bits: {}\nlazer mods: {:?}\nwindows: {windows}",
                            replay.mods.0,
                            replay.lazer_mods()
                        );
                        eprintln!(
                            "expected max combo: {}\nobserved combo: {:?}",
                            replay.max_combo,
                            judge.final_combo()
                        );
                        eprintln!(
                            "expected score without mods: {expected_score:?}\nobserved score: {:?}",
                            judge.final_score()
                        );
                        println!("kind\ttime_ms\tcolumn\tobject\tstart_ms\tend_ms\tpart\tresult\toffset_ms\ttransition\thead_offset_ms\tbroken");
                        for edge in playback.input.events() {
                            println!(
                                "{}\t{}\t{}\t\t\t\t\t\t\t\t\t",
                                if edge.pressed { "press" } else { "release" },
                                edge.time_ms,
                                edge.column + 1
                            );
                        }
                        for event in judge.events() {
                            let object = &map.objects[event.object_index];
                            println!(
                                "result\t{}\t{}\t{}\t{}\t{}\t{:?}\t{:?}\t{}\t\t\t",
                                event.time_ms,
                                event.column + 1,
                                event.object_index,
                                object.time_ms,
                                object.end_time_ms(),
                                event.part,
                                event.result,
                                event.offset_ms.map(|v| v.to_string()).unwrap_or_default()
                            );
                        }
                        for event in transitions {
                            let object = &map.objects[event.object_index];
                            println!(
                                "state\t{}\t{}\t{}\t{}\t{}\t\t\t\t{:?}\t{}\t{}",
                                event.time_ms,
                                event.column + 1,
                                event.object_index,
                                object.time_ms,
                                object.end_time_ms(),
                                event.kind,
                                event
                                    .head_offset_ms
                                    .map(|v| v.to_string())
                                    .unwrap_or_default(),
                                event.broken,
                            );
                        }
                        traced = true;
                        break;
                    }
                    {
                        let mut stable_transitions = Vec::new();
                        let judged = Playback::new(&map, &replay).and_then(|playback| {
                            if audit_stable && replay.game_version < 30_000_000 {
                                playback
                                    .trace_stable_with(
                                        playback.stable_rules(map.difficulty.overall_difficulty)?,
                                    )
                                    .map(|(judge, transitions)| {
                                        stable_transitions = transitions;
                                        (judge, None)
                                    })
                            } else {
                                judge_replay(&playback, &replay, map.difficulty.overall_difficulty)
                            }
                        });
                        match judged {
                            Err(err) => format!("unsupported:{err}"),
                            Ok((judge, rules)) => {
                                if audit_stable && replay.game_version < 30_000_000 {
                                    stable_report = stable_audit::columns(
                                        judge.events(),
                                        &stable_transitions,
                                        map.objects.len(),
                                        replay.mods.0,
                                        replay.score,
                                    );
                                }
                                let counts = judge.final_counts();
                                let ours = [
                                    ResultKind::Perfect,
                                    ResultKind::Great,
                                    ResultKind::Good,
                                    ResultKind::Ok,
                                    ResultKind::Meh,
                                    ResultKind::Miss,
                                ]
                                .map(|kind| counts.get(kind));
                                let diff: u32 =
                                    ours.iter().zip(expected).map(|(a, b)| a.abs_diff(b)).sum();
                                error = diff.to_string();
                                observed = format!("{ours:?}");
                                if let Some(combo) = judge.final_combo() {
                                    observed_combo = combo.maximum.to_string();
                                    combo_error = combo
                                        .maximum
                                        .abs_diff(u32::from(replay.max_combo))
                                        .to_string();
                                }
                                if let Some(rules) = rules {
                                    let score = judge.final_score().expect("lazer score policy");
                                    let total: u64 = expected.iter().map(|v| u64::from(*v)).sum();
                                    let weighted: u64 = expected
                                        .iter()
                                        .zip([305u64, 300, 200, 100, 50, 0])
                                        .map(|(n, w)| u64::from(*n) * w)
                                        .sum();
                                    header_accuracy = format!(
                                        "{:.12}",
                                        if total == 0 {
                                            1.0
                                        } else {
                                            weighted as f64 / (305 * total) as f64
                                        }
                                    );
                                    observed_accuracy = format!("{:.12}", score.accuracy);
                                    observed_score = score.total_score_without_mods.to_string();
                                    if let Some(expected) = expected_score {
                                        score_error = score
                                            .total_score_without_mods
                                            .abs_diff(expected as u64)
                                            .to_string();
                                    }
                                    if audit_rounding {
                                        rounding_probe =
                                            if let Some(expected_score) = expected_score {
                                                let audit = timing_probe::audit(
                                                    &Timeline::new(&map)?,
                                                    &replay.frames,
                                                    rules,
                                                    &judge,
                                                    expected,
                                                    u32::from(replay.max_combo),
                                                    expected_score as u64,
                                                    16_384,
                                                );
                                                eprintln!(
                                                    "{hash}: {} ({} trials)",
                                                    audit.status, audit.trials
                                                );
                                                audit.columns()
                                            } else {
                                                "\tmissing-score-reference\t\t\t\t\t".to_string()
                                            };
                                    }
                                    let (candidates, compatible) =
                                        tail_rounding_audit(judge.events(), rules, expected, ours);
                                    rounding_candidates = candidates.to_string();
                                    rounding_compatible = compatible.to_string();
                                    frame_rounding = if compatible {
                                        frame_rounding::audit(
                                            judge.events(),
                                            rules,
                                            replay.frames.iter().map(|f| (f.time_ms, f.x as u32)),
                                            expected,
                                            ours,
                                        )
                                    } else {
                                        frame_rounding::Compatibility::Incompatible
                                    }
                                    .label()
                                    .to_string();
                                }
                                if diff == 0 {
                                    "counts-match-experimental".into()
                                } else {
                                    "counts-differ-experimental".into()
                                }
                            }
                        }
                    }
                }
            }
        } else {
            "missing-map".into()
        };
        if trace.is_some() {
            return Err(status.into());
        }
        println!(
            "{hash}\t{}\t{}\t{}:{mods}\t{status}\t{error}\t{expected:?}\t{observed}\t{name}\t{rounding_candidates}\t{rounding_compatible}\t{policy}\t{frame_rounding}\t{}\t{observed_combo}\t{combo_error}\t{header_accuracy}\t{observed_accuracy}\t{expected_score_text}\t{observed_score}\t{score_error}{rounding_probe}{stable_report}",
            replay.beatmap_hash, replay.game_version, replay.mods.0, replay.max_combo
        );
    }
    if trace.is_some() && !traced {
        return Err(
            "requested replay could not be traced (missing input, map, or parse error)".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ResultKind as R;

    fn release(offset: f64, result: R) -> Event {
        Event {
            object_index: 0,
            column: 0,
            time_ms: 2000.0 + offset,
            part: Part::Tail,
            result,
            offset_ms: Some(offset),
        }
    }

    #[test]
    fn rounding_capacity_cannot_be_reused_or_cross_unrelated_buckets() {
        let rules = LazerRules::native_no_mod(8.0).unwrap();
        let events = [
            release(25.0, R::Great),
            release(-25.0, R::Great),
            release(30.0, R::Great),
        ];
        assert_eq!(
            tail_rounding_audit(&events, rules, [2, 1, 0, 0, 0, 0], [0, 3, 0, 0, 0, 0]),
            (2, true)
        );
        assert_eq!(
            tail_rounding_audit(&events, rules, [3, 0, 0, 0, 0, 0], [0, 3, 0, 0, 0, 0]),
            (2, false)
        );
        assert!(!tail_rounding_audit(&events, rules, [0, 2, 1, 0, 0, 0], [0, 3, 0, 0, 0, 0]).1);
        assert!(!tail_rounding_audit(&events, rules, [0, 3, 0, 0, 0, 1], [0, 3, 0, 0, 0, 0]).1);
    }

    #[test]
    fn capped_tails_and_taps_are_not_tail_rounding_candidates() {
        let rules = LazerRules::native_no_mod(8.0).unwrap();
        let mut tap = release(25.0, R::Great);
        tap.part = Part::Tap;
        let events = [release(25.0, R::Meh), tap];
        assert_eq!(
            tail_rounding_audit(&events, rules, [1, 0, 0, 0, 1, 0], [0, 1, 0, 0, 1, 0]),
            (0, false)
        );
    }

    #[test]
    fn rounded_release_can_also_downgrade_a_grade() {
        let rules = LazerRules::native_no_mod(5.0).unwrap();
        assert_eq!(
            tail_rounding_audit(
                &[release(29.0, R::Perfect)],
                rules,
                [0, 1, 0, 0, 0, 0],
                [1, 0, 0, 0, 0, 0]
            ),
            (1, true)
        );
    }
}
