use dossier_sim::mania::judge::{Event, LazerRules, Part, ResultKind as R};
use std::collections::{BTreeMap, BTreeSet};

type Change = [i64; 6];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compatibility {
    Compatible,
    Incompatible,
    AmbiguousRelease,
    StateLimit,
}
impl Compatibility {
    pub fn label(self) -> &'static str {
        match self {
            Self::Compatible => "compatible",
            Self::Incompatible => "incompatible",
            Self::AmbiguousRelease => "indeterminate-release",
            Self::StateLimit => "indeterminate-limit",
        }
    }
}

pub fn audit(
    events: &[Event],
    rules: LazerRules,
    frames: impl IntoIterator<Item = (i64, u32)>,
    expected: [u32; 6],
    observed: [u32; 6],
) -> Compatibility {
    let target = std::array::from_fn(|i| i64::from(expected[i]) - i64::from(observed[i]));
    if target == [0; 6] {
        return Compatibility::Compatible;
    }
    if target.iter().sum::<i64>() != 0 {
        return Compatibility::Incompatible;
    }
    let mut releases = BTreeMap::new();
    let mut previous = 0u32;
    for (index, (time, mask)) in frames.into_iter().enumerate() {
        let mut up = previous & !mask;
        while up != 0 {
            let column = up.trailing_zeros() as u8;
            releases
                .entry((time, column))
                .and_modify(|frame| *frame = None)
                .or_insert(Some(index));
            up &= up - 1;
        }
        previous = mask;
    }
    let mut groups: BTreeMap<usize, Vec<&Event>> = BTreeMap::new();
    for event in events {
        let Some(offset) = event.offset_ms else {
            continue;
        };
        if event.part != Part::Tail
            || event.result as usize >= R::Meh as usize
            || rules.result_for(offset / 1.5) != Some(event.result)
        {
            continue;
        }
        if [-0.5, 0.5]
            .into_iter()
            .all(|d| rules.result_for((offset + d) / 1.5) == Some(event.result))
        {
            continue;
        }
        if !event.time_ms.is_finite() || event.time_ms.fract() != 0.0 {
            return Compatibility::AmbiguousRelease;
        }
        let Some(Some(frame)) = releases.get(&(event.time_ms as i64, event.column)) else {
            return Compatibility::AmbiguousRelease;
        };
        groups.entry(*frame).or_default().push(event);
    }
    let choices: Vec<_> = groups.values().map(|group| options(group, rules)).collect();
    reachable(&choices, target, 50_000)
}

fn options(group: &[&Event], rules: LazerRules) -> Vec<Change> {
    let mut cuts = vec![-0.5, 0.0, 0.5];
    for event in group {
        for grade in [R::Perfect, R::Great, R::Good, R::Ok] {
            for sign in [-1.0, 1.0] {
                let cut = sign * rules.window(grade) * 1.5 - event.offset_ms.unwrap();
                if (-0.5..=0.5).contains(&cut) {
                    cuts.push(cut);
                }
            }
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    let mut probes = cuts.clone();
    probes.extend(cuts.windows(2).map(|w| (w[0] + w[1]) / 2.0));
    let mut choices = BTreeSet::new();
    for delta in probes {
        let mut change = [0; 6];
        let mut valid = true;
        for event in group {
            let Some(result) = rules.result_for((event.offset_ms.unwrap() + delta) / 1.5) else {
                valid = false;
                break;
            };
            if result as usize >= R::Meh as usize {
                valid = false;
                break;
            }
            change[event.result as usize] -= 1;
            change[result as usize] += 1;
        }
        if valid {
            choices.insert(change);
        }
    }
    choices.into_iter().collect()
}

fn reachable(choices: &[Vec<Change>], target: Change, limit: usize) -> Compatibility {
    let mut lower = vec![[0; 6]; choices.len() + 1];
    let mut upper = lower.clone();
    for i in (0..choices.len()).rev() {
        if choices[i].is_empty() {
            return Compatibility::Incompatible;
        }
        for k in 0..6 {
            lower[i][k] = lower[i + 1][k] + choices[i].iter().map(|v| v[k]).min().unwrap();
            upper[i][k] = upper[i + 1][k] + choices[i].iter().map(|v| v[k]).max().unwrap();
        }
    }
    let viable = |state: Change, remaining: usize| {
        (0..6)
            .all(|k| (lower[remaining][k]..=upper[remaining][k]).contains(&(target[k] - state[k])))
    };
    if !viable([0; 6], 0) {
        return Compatibility::Incompatible;
    }
    let mut states = BTreeSet::from([[0; 6]]);
    let mut expansions = 0usize;
    for (i, group) in choices.iter().enumerate() {
        let mut next = BTreeSet::new();
        for state in states {
            for option in group {
                expansions += 1;
                if expansions > limit.saturating_mul(40) {
                    return Compatibility::StateLimit;
                }
                let candidate = std::array::from_fn(|k| state[k] + option[k]);
                if viable(candidate, i + 1) {
                    next.insert(candidate);
                    if next.len() > limit {
                        return Compatibility::StateLimit;
                    }
                }
            }
        }
        if next.is_empty() {
            return Compatibility::Incompatible;
        }
        states = next;
    }
    if states.contains(&target) {
        Compatibility::Compatible
    } else {
        Compatibility::Incompatible
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Compatibility as C;

    fn tail(column: u8, offset: f64, result: R) -> Event {
        Event {
            object_index: usize::from(column),
            column,
            time_ms: 2000.0,
            part: Part::Tail,
            result,
            offset_ms: Some(offset),
        }
    }

    #[test]
    fn one_frame_cannot_move_early_and_late_tails_in_opposite_directions() {
        let rules = LazerRules::native_no_mod(8.0).unwrap();
        let events = [tail(0, 25.0, R::Great), tail(1, -25.0, R::Great)];
        let expected = [2, 0, 0, 0, 0, 0];
        let observed = [0, 2, 0, 0, 0, 0];
        assert!(crate::tail_rounding_audit(&events, rules, expected, observed).1);
        assert_eq!(
            audit(&events, rules, [(1900, 3), (2000, 0)], expected, observed),
            C::Incompatible
        );
        assert_eq!(
            audit(
                &events,
                rules,
                [(1900, 3), (2000, 0)],
                [1, 1, 0, 0, 0, 0],
                observed
            ),
            C::Compatible
        );
    }

    #[test]
    fn distinct_frames_at_the_same_rounded_time_are_not_merged() {
        let rules = LazerRules::native_no_mod(8.0).unwrap();
        let events = [tail(0, -25.0, R::Great), tail(1, 25.0, R::Great)];
        assert_eq!(
            audit(
                &events,
                rules,
                [(1900, 3), (2000, 1), (2000, 0)],
                [2, 0, 0, 0, 0, 0],
                [0, 2, 0, 0, 0, 0]
            ),
            C::Compatible
        );
    }

    #[test]
    fn identical_chord_tails_cannot_split_into_different_grades() {
        let rules = LazerRules::native_no_mod(8.0).unwrap();
        let events = [tail(0, 25.0, R::Great), tail(1, 25.0, R::Great)];
        assert_eq!(
            audit(
                &events,
                rules,
                [(1900, 3), (2000, 0)],
                [1, 1, 0, 0, 0, 0],
                [0, 2, 0, 0, 0, 0]
            ),
            C::Incompatible
        );
        assert_eq!(
            audit(
                &events,
                rules,
                [(1900, 3), (2000, 0)],
                [2, 0, 0, 0, 0, 0],
                [0, 2, 0, 0, 0, 0]
            ),
            C::Compatible
        );
    }

    #[test]
    fn repeated_release_in_one_column_is_unknown_not_false_evidence() {
        let rules = LazerRules::native_no_mod(8.0).unwrap();
        let events = [tail(0, 25.0, R::Great)];
        for frames in [vec![(1900, 1), (2000, 0), (2000, 1), (2000, 0)], vec![]] {
            assert_eq!(
                audit(
                    &events,
                    rules,
                    frames,
                    [1, 0, 0, 0, 0, 0],
                    [0, 1, 0, 0, 0, 0]
                ),
                C::AmbiguousRelease
            );
        }
    }

    #[test]
    fn downgrades_and_interior_intervals_are_included() {
        let rules = LazerRules::native_no_mod(5.0).unwrap();
        let event = tail(0, 29.0, R::Perfect);
        assert_eq!(
            audit(
                &[event],
                rules,
                [(1900, 1), (2000, 0)],
                [0, 1, 0, 0, 0, 0],
                [1, 0, 0, 0, 0, 0]
            ),
            C::Compatible
        );
        let rules = LazerRules::native_no_mod(8.0).unwrap();
        let a = tail(0, 24.5, R::Perfect);
        let b = tail(1, -24.5, R::Perfect);
        let opts = options(&[&a, &b], rules);
        assert!(opts.contains(&[0; 6]));
        assert!(opts.contains(&[-1, 1, 0, 0, 0, 0]));
        assert_eq!(opts.len(), 2);
    }

    #[test]
    fn taps_capped_tails_and_count_changes_are_outside_the_model() {
        let rules = LazerRules::native_no_mod(8.0).unwrap();
        let mut tap = tail(0, 25.0, R::Great);
        tap.part = Part::Tap;
        let events = [tap, tail(1, 25.0, R::Meh)];
        assert_eq!(
            audit(
                &events,
                rules,
                [(1900, 3), (2000, 0)],
                [1, 0, 0, 0, 1, 0],
                [0, 1, 0, 0, 1, 0]
            ),
            C::Incompatible
        );
        assert_eq!(
            audit(&[], rules, [], [1, 0, 0, 0, 0, 0], [0; 6]),
            C::Incompatible
        );
    }

    #[test]
    fn search_allows_cancellation_and_reports_resource_limits_as_unknown() {
        let up = [1, -1, 0, 0, 0, 0];
        let down = [-1, 1, 0, 0, 0, 0];
        let choices = vec![vec![[0; 6], up], vec![[0; 6], down]];
        assert_eq!(reachable(&choices, [0; 6], 10), C::Compatible);
        assert_eq!(reachable(&choices, [0; 6], 1), C::StateLimit);
        assert_eq!(
            reachable(&[vec![up], vec![down]], [0; 6], 10),
            C::Compatible
        );
        assert_eq!(
            reachable(&choices, [0, 0, 1, -1, 0, 0], 10),
            C::Incompatible
        );
        assert_eq!(reachable(&vec![vec![[0; 6]]; 41], [0; 6], 1), C::StateLimit);
    }

    #[test]
    fn pruned_search_agrees_with_unpruned_enumeration() {
        let choices = vec![
            vec![[0; 6], [2, -2, 0, 0, 0, 0]],
            vec![[0; 6], [-1, 1, 0, 0, 0, 0]],
            vec![[0; 6], [0, 1, -1, 0, 0, 0]],
            vec![[0; 6], [1, -2, 1, 0, 0, 0]],
        ];
        let mut brute = BTreeSet::from([[0; 6]]);
        for group in &choices {
            brute = brute
                .iter()
                .flat_map(|state| {
                    group
                        .iter()
                        .map(move |option| std::array::from_fn(|k| state[k] + option[k]))
                })
                .collect();
        }
        for perfect in -3..=4 {
            for great in -4..=4 {
                let target = [perfect, great, -perfect - great, 0, 0, 0];
                assert_eq!(
                    reachable(&choices, target, 1000) == C::Compatible,
                    brute.contains(&target),
                    "{target:?}"
                );
            }
        }
    }
}
