//! Counterfactual export-rounding audit. Witnesses are possibilities, not recovered input.
use dossier_replay::ReplayFrame;
use dossier_sim::mania::{
    judge::{Event, Judge, LazerRules, Part, ResultKind as R},
    Timeline,
};
use std::collections::{BTreeMap, BTreeSet};

pub struct Audit {
    pub status: &'static str,
    pub candidates: usize,
    pub trials: usize,
    pub rejected: usize,
    pub offsets: Vec<(usize, f64)>,
}
impl Audit {
    pub fn columns(&self) -> String {
        let witness = self
            .offsets
            .iter()
            .map(|(i, d)| format!("{i}:{d}"))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "\t{}\t{}\t{}\t{}\t{}\t{witness}",
            self.status,
            self.candidates,
            self.trials,
            self.rejected,
            self.offsets.len()
        )
    }
}

#[derive(Default)]
struct Group<'a> {
    tails: Vec<&'a Event>,
    note_lock: bool,
}

fn candidates<'a>(
    events: &'a [Event],
    frames: &[ReplayFrame],
    rules: LazerRules,
) -> Result<Vec<(usize, Vec<f64>)>, ()> {
    let mut edges = BTreeMap::new();
    let mut previous = 0u32;
    for (i, frame) in frames.iter().enumerate() {
        let mask = frame.x as u32;
        let mut changed = mask ^ previous;
        while changed != 0 {
            let column = changed.trailing_zeros() as u8;
            let pressed = mask & (1 << column) != 0;
            edges
                .entry((frame.time_ms, column, pressed))
                .and_modify(|entry| *entry = None)
                .or_insert(Some(i));
            changed &= changed - 1;
        }
        previous = mask;
    }
    let forced: BTreeSet<_> = events
        .iter()
        .filter(|e| {
            e.result == R::Miss
                && e.offset_ms.is_none()
                && matches!(e.part, Part::Head | Part::Tap)
                && e.time_ms.fract() == 0.0
        })
        .map(|e| (e.time_ms as i64, e.column))
        .collect();
    let mut groups: BTreeMap<usize, Group<'a>> = BTreeMap::new();
    for event in events {
        let Some(offset) = event.offset_ms else {
            continue;
        };
        let tail = event.part == Part::Tail
            && (event.result as usize) < R::Meh as usize
            && rules.result_for(offset / 1.5) == Some(event.result)
            && [-0.5, 0.5]
                .into_iter()
                .any(|d| rules.result_for((offset + d) / 1.5) != Some(event.result));
        let note_lock = matches!(event.part, Part::Tap | Part::Head)
            && offset == 0.0
            && forced.contains(&(event.time_ms as i64, event.column));
        if !tail && !note_lock {
            continue;
        }
        if !event.time_ms.is_finite() || event.time_ms.fract() != 0.0 {
            return Err(());
        }
        let Some(Some(frame)) = edges.get(&(event.time_ms as i64, event.column, !tail)) else {
            return Err(());
        };
        let group = groups.entry(*frame).or_default();
        if tail {
            group.tails.push(event);
        }
        group.note_lock |= note_lock;
    }
    Ok(groups
        .into_iter()
        .map(|(frame, group)| (frame, choices(&group, rules)))
        .collect())
}

fn choices(group: &Group<'_>, rules: LazerRules) -> Vec<f64> {
    let mut cuts = vec![-0.5, 0.0, 0.5];
    for event in &group.tails {
        for grade in [R::Perfect, R::Great, R::Good, R::Ok] {
            for sign in [-1.0, 1.0] {
                let cut = sign * rules.window(grade) * 1.5 - event.offset_ms.unwrap();
                if (-0.5..0.5).contains(&cut) {
                    cuts.push(cut);
                }
            }
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    // Interior representatives round back to the stored timestamp, regardless
    // of the encoder's tie rule. Boundary-only possibilities are not exhausted.
    let mut probes = vec![0.0];
    probes.extend(cuts.windows(2).map(|w| (w[0] + w[1]) / 2.0));
    let mut seen = BTreeSet::new();
    probes
        .into_iter()
        .filter(|delta| {
            let mut signature: Vec<_> = group
                .tails
                .iter()
                .map(|e| {
                    rules
                        .result_for((e.offset_ms.unwrap() + delta) / 1.5)
                        .map_or(6, |r| r as usize)
                })
                .collect();
            if group.note_lock {
                signature.push(usize::from(*delta >= 0.0));
            }
            seen.insert(signature)
        })
        .collect()
}

/// Fill unconstrained frames at equal timestamps with the closest-to-zero
/// displacement that preserves all fixed choices and original frame order.
fn ordered_offsets(frames: &[ReplayFrame], fixed: &BTreeMap<usize, f64>) -> Option<Vec<f64>> {
    let mut offsets = vec![0.0; frames.len()];
    let mut start = 0;
    while start < frames.len() {
        let end = start + frames[start..].partition_point(|f| f.time_ms == frames[start].time_ms);
        let mut lower: f64 = -0.5;
        let mut cursor = start;
        for (&index, &delta) in fixed.range(start..end) {
            if delta < lower || !delta.is_finite() || delta.abs() >= 0.5 {
                return None;
            }
            offsets[cursor..index].fill(0.0f64.clamp(lower, delta));
            offsets[index] = delta;
            lower = delta;
            cursor = index + 1;
        }
        offsets[cursor..end].fill(0.0f64.max(lower));
        start = end;
    }
    Some(offsets)
}

fn matches(judge: &Judge, counts: [u32; 6], combo: u32, score: u64) -> bool {
    [R::Perfect, R::Great, R::Good, R::Ok, R::Meh, R::Miss].map(|r| judge.final_counts().get(r))
        == counts
        && judge.final_combo().is_some_and(|c| c.maximum == combo)
        && judge
            .final_score()
            .is_some_and(|s| s.total_score_without_mods == score)
}

/// Tests a finite set of timing hypotheses against all three recorded totals.
/// Failure to find a witness is not proof that export rounding is impossible.
pub fn audit(
    timeline: &Timeline,
    frames: &[ReplayFrame],
    rules: LazerRules,
    baseline: &Judge,
    counts: [u32; 6],
    combo: u32,
    score: u64,
    limit: usize,
) -> Audit {
    let mut result = Audit {
        status: "unchanged-match",
        candidates: 0,
        trials: 0,
        rejected: 0,
        offsets: vec![],
    };
    if matches(baseline, counts, combo, score) {
        return result;
    }
    let Ok(groups) = candidates(baseline.events(), frames, rules) else {
        result.status = "indeterminate-provenance";
        return result;
    };
    result.candidates = groups.len();
    let mut digits = vec![0; groups.len()];
    loop {
        if result.trials >= limit {
            result.status = "indeterminate-limit";
            return result;
        }
        result.trials += 1;
        let fixed: BTreeMap<_, _> = groups
            .iter()
            .zip(&digits)
            .map(|((frame, choices), &choice)| (*frame, choices[choice]))
            .collect();
        if let Some(offsets) = ordered_offsets(frames, &fixed) {
            match Judge::with_frame_offsets(timeline, frames, &offsets, rules) {
                Ok(judge) => {
                    if matches(&judge, counts, combo, score) {
                        result.status = "compatible-hypothesis";
                        result.offsets = offsets
                            .into_iter()
                            .enumerate()
                            .filter(|(_, d)| *d != 0.0)
                            .collect();
                        return result;
                    }
                }
                Err(_) => result.rejected += 1,
            }
        } else {
            result.rejected += 1;
        }
        let mut position = 0;
        while position < digits.len() {
            digits[position] += 1;
            if digits[position] < groups[position].1.len() {
                break;
            }
            digits[position] = 0;
            position += 1;
        }
        if position == digits.len() {
            result.status = "no-witness-found";
            return result;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dossier_beatmap::Beatmap;
    use dossier_replay::Keys;
    use dossier_sim::mania::InputTrack;

    fn frames(values: &[(i64, u32)]) -> Vec<ReplayFrame> {
        values
            .iter()
            .map(|&(time_ms, mask)| ReplayFrame {
                time_ms,
                x: mask as f32,
                y: 0.0,
                keys: Keys(0),
            })
            .collect()
    }
    fn timeline(objects: &str) -> Timeline {
        Timeline::new(&Beatmap::parse(&format!(
            "osu file format v14\n[General]\nMode:3\n[Difficulty]\nCircleSize:4\n[HitObjects]\n{objects}"
        )).unwrap()).unwrap()
    }
    fn judge(timeline: &Timeline, frames: &[ReplayFrame], rules: LazerRules) -> Judge {
        Judge::new(timeline, &InputTrack::new(frames, 4).unwrap(), rules).unwrap()
    }

    #[test]
    fn tail_witness_is_replayed_and_matches_counts_combo_and_score() {
        let t = timeline("64,192,1000,128,0,2000:0:0:0:0:");
        let f = frames(&[(1000, 1), (2025, 0)]);
        let rules = LazerRules::native_no_mod(8.0).unwrap();
        let baseline = judge(&t, &f, rules);
        let a = audit(
            &t,
            &f,
            rules,
            &baseline,
            [2, 0, 0, 0, 0, 0],
            2,
            1_000_000,
            10,
        );
        assert_eq!(a.status, "compatible-hypothesis");
        assert_eq!(a.candidates, 1);
        assert_eq!(a.offsets, [(1, -0.375)]);
        let mut offsets = vec![0.0; f.len()];
        for (i, d) in a.offsets {
            offsets[i] = d;
        }
        let hypothetical = Judge::with_frame_offsets(&t, &f, &offsets, rules).unwrap();
        assert!(matches(&hypothetical, [2, 0, 0, 0, 0, 0], 2, 1_000_000));
        assert_eq!(f[1].time_ms, 2025);
        assert_eq!(baseline.final_counts().get(R::Great), 1);
        assert_eq!(baseline.events(), judge(&t, &f, rules).events());
        for (frame, delta) in f.iter().zip(offsets) {
            assert_eq!(
                (frame.time_ms as f64 + delta).round_ties_even(),
                frame.time_ms as f64
            );
        }
    }

    #[test]
    fn matching_counts_do_not_bypass_combo_or_score_checks() {
        let t = timeline("64,192,1000,128,0,2000:0:0:0:0:");
        let f = frames(&[(1000, 1), (2025, 0)]);
        let r = LazerRules::native_no_mod(8.0).unwrap();
        let j = judge(&t, &f, r);
        for (combo, score) in [(1, 1_000_000), (2, 999_999)] {
            let a = audit(&t, &f, r, &j, [2, 0, 0, 0, 0, 0], combo, score, 10);
            assert_eq!(a.status, "no-witness-found");
            assert!(a.offsets.is_empty());
        }
    }

    #[test]
    fn identical_chord_tails_cannot_receive_different_grades_from_one_frame() {
        let t = timeline("64,192,1000,128,0,2000:0:0:0:0:\n192,192,1000,128,0,2000:0:0:0:0:");
        let f = frames(&[(1000, 3), (2025, 0)]);
        let r = LazerRules::native_no_mod(8.0).unwrap();
        let j = judge(&t, &f, r);
        let a = audit(&t, &f, r, &j, [3, 1, 0, 0, 0, 0], 4, 999_999, 10);
        assert_eq!(a.status, "no-witness-found");
        assert_eq!(a.candidates, 1);
        assert_eq!(a.trials, 2);
    }

    #[test]
    fn equal_time_idle_frames_move_only_as_needed_to_preserve_order() {
        let f = frames(&[(100, 0), (100, 0), (100, 1), (100, 0), (101, 0)]);
        let fixed = BTreeMap::from([(2, -0.375), (3, 0.25)]);
        assert_eq!(
            ordered_offsets(&f, &fixed).unwrap(),
            [-0.375, -0.375, -0.375, 0.25, 0.0]
        );
        let backwards = BTreeMap::from([(1, 0.25), (2, -0.375)]);
        assert!(ordered_offsets(&f, &backwards).is_none());
    }

    #[test]
    fn note_lock_witness_uses_the_full_judge_not_only_a_tail_classifier() {
        let t = timeline("64,192,900,1,0\n64,192,1000,1,0");
        let f = frames(&[(1000, 1)]);
        let r = LazerRules::native_no_mod(5.0).unwrap();
        let j = judge(&t, &f, r);
        let shifted = Judge::with_frame_offsets(&t, &f, &[-0.25], r).unwrap();
        assert_eq!(shifted.events()[0].object_index, 0);
        assert_eq!(shifted.events()[0].result, R::Ok);
        assert_eq!(j.events()[0].object_index, 1);
        let a = audit(
            &t,
            &f,
            r,
            &j,
            [0, 0, 0, 1, 0, 1],
            1,
            shifted.final_score().unwrap().total_score_without_mods,
            10,
        );
        assert_eq!(a.status, "compatible-hypothesis");
        assert_eq!(a.offsets, [(0, -0.25)]);
    }

    #[test]
    fn budget_exhaustion_and_ambiguous_edges_are_unknown() {
        let t = timeline("64,192,1000,128,0,2000:0:0:0:0:");
        let f = frames(&[(1000, 1), (2025, 0), (2025, 1), (2025, 0)]);
        let r = LazerRules::native_no_mod(8.0).unwrap();
        let j = judge(&t, &f, r);
        assert_eq!(
            audit(&t, &f, r, &j, [2, 0, 0, 0, 0, 0], 2, 1_000_000, 10).status,
            "indeterminate-provenance"
        );
        let f = frames(&[(1000, 1), (2025, 0)]);
        let j = judge(&t, &f, r);
        assert_eq!(
            audit(&t, &f, r, &j, [2, 0, 0, 0, 0, 0], 2, 1_000_000, 1).status,
            "indeterminate-limit"
        );
        let perfect = frames(&[(1000, 1), (2000, 0)]);
        let j = judge(&t, &perfect, r);
        let a = audit(&t, &perfect, r, &j, [2, 0, 0, 0, 0, 0], 2, 1_000_000, 0);
        assert_eq!(a.status, "unchanged-match");
        assert_eq!(a.trials, 0);
    }
}
