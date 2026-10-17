//! Diagnostic ScoreV1 formula, not a production stable score implementation.
//! Reference: osu!wiki Gameplay/Score/ScoreV1/osu!mania (retrieved 2026-10-16).
use dossier_sim::mania::{
    judge::{Event, Part, ResultKind as R},
    stable::{TraceEvent, TraceKind},
};
use std::collections::{BTreeMap, BTreeSet};

pub const HEADER: &str = "\tstable_score_model\texpected_stable_score\tmodel_stable_score\tstable_score_error\tprehead_release_count\tprehead_release_objects\tstable_equal_time_order\tstable_order_min_score\tstable_order_max_score\tstable_order_score_relation";
pub const NOT_APPLICABLE: &str = "\tnot-applicable\t\t\t\t\t\tnot-applicable\t\t\t";
pub const NOT_MEASURED: &str = "\tnot-measured\t\t\t\t\t\tnot-measured\t\t\t";

fn valid_events(events: &[Event], objects: usize) -> bool {
    if events.len() != objects {
        return false;
    }
    let mut seen = BTreeSet::new();
    let mut previous = f64::NEG_INFINITY;
    for e in events {
        if !matches!(e.part, Part::Tap | Part::Hold)
            || !e.time_ms.is_finite()
            || e.time_ms < previous
            || e.object_index >= objects
            || !seen.insert(e.object_index)
        {
            return false;
        }
        previous = e.time_ms;
    }
    true
}

fn award(bonus: u8, result: R, scale: f64) -> (u8, f64) {
    let (value, bonus_value, change) = match result {
        R::Perfect => (320.0, 32.0, 2.0),
        R::Great => (300.0, 32.0, 1.0),
        R::Good => (200.0, 16.0, -8.0),
        R::Ok => (100.0, 8.0, -24.0),
        R::Meh => (50.0, 4.0, -44.0),
        R::Miss => (0.0, 0.0, f64::NEG_INFINITY),
    };
    let bonus = (f64::from(bonus) + change).clamp(0.0, 100.0);
    (
        bonus as u8,
        scale * (value / 320.0 + bonus_value * bonus.sqrt() / 320.0),
    )
}

/// Models the wiki formula on our current event order. No expected score enters
/// this calculation; scheduler, body-break effects and mod scoring are unverified.
fn score_no_mod(events: &[Event], objects: usize) -> Option<u64> {
    if !valid_events(events, objects) {
        return None;
    }
    if objects == 0 {
        return Some(0);
    }
    let scale = 500_000.0 / objects as f64;
    let mut bonus = 100;
    let mut score = 0.0;
    for e in events {
        let (next, points) = award(bonus, e.result, scale);
        bonus = next;
        score += points;
    }
    Some(score.round_ties_even() as u64)
}

#[derive(Clone, Copy)]
struct Bounds {
    min: f64,
    max: f64,
}

fn merge<K: Ord>(states: &mut BTreeMap<K, Bounds>, key: K, value: Bounds) {
    states
        .entry(key)
        .and_modify(|old| {
            old.min = old.min.min(value.min);
            old.max = old.max.max(value.max);
        })
        .or_insert(value);
}

/// Optimistic score bounds over all equal-time grade permutations. Frame and
/// column constraints are deliberately relaxed. Being inside the bounds is not
/// evidence of a reachable score or the original client order.
fn order_bounds(
    events: &[Event],
    objects: usize,
    mut budget: usize,
) -> Result<(u64, u64), &'static str> {
    if !valid_events(events, objects) {
        return Err("invalid-event-stream");
    }
    if objects == 0 {
        return Ok((0, 0));
    }
    let grades = [R::Perfect, R::Great, R::Good, R::Ok, R::Meh, R::Miss];
    let scale = 500_000.0 / objects as f64;
    let mut scores = BTreeMap::from([(100, Bounds { min: 0.0, max: 0.0 })]);
    let mut rest = events;
    while let Some(first) = rest.first() {
        let end = rest.partition_point(|e| e.time_ms == first.time_ms);
        let (group, tail) = rest.split_at(end);
        rest = tail;
        let mut counts = [0usize; 6];
        for e in group {
            counts[e.result as usize] += 1;
        }
        let mut states: BTreeMap<_, _> =
            scores.into_iter().map(|(b, s)| ((counts, b), s)).collect();
        for _ in 0..group.len() {
            let mut next = BTreeMap::new();
            for ((remaining, bonus), score) in states {
                for (i, &grade) in grades.iter().enumerate() {
                    if remaining[i] == 0 {
                        continue;
                    }
                    budget = budget.checked_sub(1).ok_or("indeterminate-limit")?;
                    let (bonus, points) = award(bonus, grade, scale);
                    let mut remaining = remaining;
                    remaining[i] -= 1;
                    merge(
                        &mut next,
                        (remaining, bonus),
                        Bounds {
                            min: score.min + points,
                            max: score.max + points,
                        },
                    );
                    if next.len() > 50_000 {
                        return Err("indeterminate-limit");
                    }
                }
            }
            states = next;
        }
        scores = states.into_iter().map(|((_, b), s)| (b, s)).collect();
    }
    let min = scores.values().map(|s| s.min).fold(f64::INFINITY, f64::min);
    let max = scores
        .values()
        .map(|s| s.max)
        .fold(f64::NEG_INFINITY, f64::max);
    Ok((min.round_ties_even() as u64, max.round_ties_even() as u64))
}

pub fn columns(
    events: &[Event],
    transitions: &[TraceEvent],
    objects: usize,
    legacy_mods: u32,
    recorded_score: i32,
) -> String {
    let prehead: Vec<_> = transitions
        .iter()
        .filter(|e| e.kind == TraceKind::ReleasedBeforeHead)
        .map(|e| e.object_index)
        .collect();
    let ids = prehead
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let model = if legacy_mods == 0 {
        score_no_mod(events, objects)
    } else {
        None
    };
    let policy = if legacy_mods != 0 {
        "unsupported-mods"
    } else if model.is_some() {
        "wiki-nomod-experimental"
    } else {
        "invalid-event-stream"
    };
    let score = model.map(|s| s.to_string()).unwrap_or_default();
    let error = model
        .zip(u64::try_from(recorded_score).ok())
        .map(|(s, expected)| s.abs_diff(expected).to_string())
        .unwrap_or_default();
    let order = if legacy_mods != 0 {
        "unsupported-mods\t\t\t".to_owned()
    } else {
        match order_bounds(events, objects, 2_000_000) {
            Ok((min, max)) => {
                let relation = match u64::try_from(recorded_score) {
                    Ok(expected) if expected < min => "below-bounds",
                    Ok(expected) if expected > max => "above-bounds",
                    Ok(_) => "within-bounds",
                    Err(_) => "",
                };
                format!("bounded\t{min}\t{max}\t{relation}")
            }
            Err(status) => format!("{status}\t\t\t"),
        }
    };
    format!(
        "\t{policy}\t{recorded_score}\t{score}\t{error}\t{}\t{ids}\t{order}",
        prehead.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn events(grades: &[R]) -> Vec<Event> {
        grades
            .iter()
            .enumerate()
            .map(|(i, &result)| Event {
                object_index: i,
                column: 0,
                time_ms: i as f64,
                part: Part::Tap,
                result,
                offset_ms: Some(0.0),
            })
            .collect()
    }
    #[test]
    fn formula_uses_each_grade_and_updates_bonus_before_awarding_points() {
        for (grade, score) in [
            (R::Perfect, 1_000_000),
            (R::Great, 968_750),
            (R::Good, 552_292),
            (R::Ok, 265_222),
            (R::Meh, 124_896),
            (R::Miss, 0),
        ] {
            assert_eq!(score_no_mod(&events(&[grade]), 1), Some(score));
        }
        assert_eq!(
            score_no_mod(&events(&[R::Perfect, R::Great]), 2),
            Some(984_375)
        );
        assert_eq!(
            score_no_mod(&events(&[R::Miss, R::Perfect]), 2),
            Some(285_355)
        );
        assert_eq!(score_no_mod(&[], 0), Some(0));
    }
    #[test]
    fn same_counts_can_have_different_score_and_equal_time_order_is_retained() {
        let mut a = events(&[R::Meh, R::Good, R::Perfect, R::Great]);
        let mut b = events(&[R::Meh, R::Good, R::Great, R::Perfect]);
        for e in a.iter_mut().chain(b.iter_mut()) {
            e.time_ms = 1000.0;
        }
        assert_eq!(score_no_mod(&a, 4), Some(572_494));
        assert_eq!(score_no_mod(&b, 4), Some(571_606));
    }
    #[test]
    fn combined_hold_is_scored_once_and_other_event_models_are_rejected() {
        let mut e = events(&[R::Perfect]);
        e[0].part = Part::Hold;
        assert_eq!(score_no_mod(&e, 1), Some(1_000_000));
        for part in [Part::Head, Part::Tail, Part::HoldBreak] {
            e[0].part = part;
            assert_eq!(score_no_mod(&e, 1), None);
        }
    }
    #[test]
    fn malformed_or_partial_stream_is_not_given_a_plausible_score() {
        let original = events(&[R::Perfect, R::Perfect]);
        assert_eq!(score_no_mod(&original, 3), None);
        for bad in 0..5 {
            let mut e = original.clone();
            match bad {
                0 => e[1].object_index = 0,
                1 => e[1].object_index = 2,
                2 => e[1].time_ms = -1.0,
                3 => e[1].time_ms = f64::NAN,
                _ => e[1].time_ms = f64::INFINITY,
            }
            assert_eq!(score_no_mod(&e, 2), None);
        }
    }
    #[test]
    fn reference_is_only_compared_and_mods_are_not_silently_ignored() {
        let e = events(&[R::Perfect]);
        assert_eq!(
            columns(&e, &[], 1, 0, 0),
            "\twiki-nomod-experimental\t0\t1000000\t1000000\t0\t\tbounded\t1000000\t1000000\tbelow-bounds"
        );
        assert_eq!(
            columns(&e, &[], 1, 80, 893345),
            "\tunsupported-mods\t893345\t\t\t0\t\tunsupported-mods\t\t\t"
        );
        assert_eq!(
            columns(&e, &[], 1, 0, -1),
            "\twiki-nomod-experimental\t-1\t1000000\t\t0\t\tbounded\t1000000\t1000000\t"
        );
    }
    #[test]
    fn prehead_diagnostics_retain_repeated_releases_and_original_object_ids() {
        let trace = |kind| TraceEvent {
            object_index: 9,
            column: 1,
            time_ms: 1000.,
            kind,
            head_offset_ms: Some(-18.),
            broken: true,
            frame_index: Some(0),
        };
        let s = columns(
            &[],
            &[
                trace(TraceKind::ReleasedBeforeHead),
                trace(TraceKind::BodyReleased),
                trace(TraceKind::ReleasedBeforeHead),
            ],
            0,
            0,
            0,
        );
        assert!(s.contains("\t2\t9,9\tbounded\t0\t0\twithin-bounds"));
        assert_eq!(s.split('\t').count(), HEADER.split('\t').count());
        assert_eq!(
            NOT_APPLICABLE.split('\t').count(),
            HEADER.split('\t').count()
        );
        assert_eq!(NOT_MEASURED.split('\t').count(), HEADER.split('\t').count());
    }

    fn permutations(grades: &mut [R], at: usize, visit: &mut impl FnMut(&[R])) {
        if at == grades.len() {
            visit(grades);
            return;
        }
        for i in at..grades.len() {
            grades.swap(at, i);
            permutations(grades, at + 1, visit);
            grades.swap(at, i);
        }
    }

    #[test]
    fn order_bounds_match_exhaustive_permutations_across_multiple_groups() {
        let mut scores = BTreeSet::new();
        let mut a = [R::Perfect, R::Meh, R::Good, R::Great];
        let mut b = [R::Good, R::Perfect, R::Miss, R::Perfect];
        let build = |a: &[R], b: &[R]| {
            let mut es = events(&[a, b].concat());
            for e in &mut es {
                e.time_ms = if e.object_index < a.len() {
                    1000.
                } else {
                    2000.
                };
            }
            es
        };
        let bounds = order_bounds(&build(&a, &b), 8, 100_000).unwrap();
        permutations(&mut a, 0, &mut |a| {
            permutations(&mut b, 0, &mut |b| {
                scores.insert(score_no_mod(&build(a, b), 8).unwrap());
            });
        });
        assert_eq!(bounds, (*scores.first().unwrap(), *scores.last().unwrap()));
        let unreachable = (bounds.0..=bounds.1).find(|s| !scores.contains(s)).unwrap();
        assert!(columns(&build(&a, &b), &[], 8, 0, unreachable as i32).ends_with("within-bounds"));
    }

    #[test]
    fn order_bounds_never_reorder_distinct_timestamps() {
        let e = events(&[R::Meh, R::Good, R::Perfect, R::Great]);
        let score = score_no_mod(&e, 4).unwrap();
        assert_eq!(order_bounds(&e, 4, 4), Ok((score, score)));
        assert_eq!(order_bounds(&e, 4, 3), Err("indeterminate-limit"));
        assert_eq!(order_bounds(&[], 0, 0), Ok((0, 0)));
        assert_eq!(order_bounds(&e, 5, 100), Err("invalid-event-stream"));
        let mut bad = e.clone();
        bad[1].time_ms = f64::NAN;
        assert_eq!(order_bounds(&bad, 4, 100), Err("invalid-event-stream"));
        let mut big = events(&[R::Meh, R::Good, R::Perfect, R::Great]);
        for e in &mut big {
            e.time_ms = 0.;
        }
        assert_eq!(order_bounds(&big, 4, 4), Err("indeterminate-limit"));
    }
}
