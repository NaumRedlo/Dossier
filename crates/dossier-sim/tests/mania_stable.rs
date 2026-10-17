use dossier_beatmap::Beatmap;
use dossier_replay::{Keys, ReplayFrame};
use dossier_sim::mania::{
    judge::{Combo, Judge, Part, ResultKind as R},
    stable::{StableRules, TraceKind as T},
    Error, InputTrack, Timeline,
};

fn timeline(objects: &str) -> Timeline {
    Timeline::new(&Beatmap::parse(&format!("osu file format v14\n[General]\nMode:3\n[Difficulty]\nCircleSize:4\n[HitObjects]\n{objects}")).unwrap()).unwrap()
}
fn input(edges: &[(i64, u32)]) -> InputTrack {
    InputTrack::new(
        &edges
            .iter()
            .map(|&(time_ms, keys)| ReplayFrame {
                time_ms,
                x: keys as f32,
                y: 0.0,
                keys: Keys(0),
            })
            .collect::<Vec<_>>(),
        4,
    )
    .unwrap()
}
fn judge(objects: &str, edges: &[(i64, u32)]) -> Judge {
    StableRules::native_no_mod(5.0)
        .unwrap()
        .judge(&timeline(objects), &input(edges))
        .unwrap()
}
const TAP: &str = "64,192,1000,1,0";
const HOLD: &str = "64,192,1000,128,0,2000:0:0:0:0:";

#[test]
fn tap_combo_counts_every_successful_grade_and_seeks_across_misses() {
    let objects = (1..=9)
        .map(|i| format!("64,192,{},1,0", i * 1000))
        .collect::<Vec<_>>()
        .join("\n");
    let edges: Vec<_> = [1000, 1970, 2940, 3900, 4870, 7000, 8000]
        .into_iter()
        .flat_map(|time| [(time, 1), (time + 1, 0)])
        .collect();
    let j = judge(&objects, &edges);
    assert_eq!(
        j.events().iter().map(|e| e.result).collect::<Vec<_>>(),
        [
            R::Perfect,
            R::Great,
            R::Good,
            R::Ok,
            R::Meh,
            R::Miss,
            R::Perfect,
            R::Perfect,
            R::Miss
        ]
    );
    assert_eq!(
        j.final_combo(),
        Some(Combo {
            current: 0,
            maximum: 5
        })
    );
    assert_eq!(
        j.combo_at(8000.),
        Some(Combo {
            current: 2,
            maximum: 5
        })
    );
    assert_eq!(
        j.combo_at(6500.),
        Some(Combo {
            current: 0,
            maximum: 5
        })
    );
    assert_eq!(
        j.combo_at(5000.),
        Some(Combo {
            current: 5,
            maximum: 5
        })
    );
    assert_eq!(
        j.combo_at(1970.),
        Some(Combo {
            current: 2,
            maximum: 2
        })
    );
    assert_eq!(
        j.combo_at(1969.),
        Some(Combo {
            current: 1,
            maximum: 1
        })
    );
    for time in [999., f64::NEG_INFINITY, f64::NAN] {
        assert_eq!(j.combo_at(time), Some(Combo::default()));
    }
    assert_eq!(j.combo_at(f64::INFINITY), j.final_combo());
    assert_eq!(j.final_score(), None);
}

#[test]
fn tap_combo_handles_empty_missed_and_modded_chord_maps() {
    for (rate, hr) in [(1., false), (1., true), (1.5, false), (1.5, true)] {
        let rules = StableRules::native_with_mods(8., rate, hr).unwrap();
        for objects in ["", TAP] {
            let j = rules.judge(&timeline(objects), &input(&[])).unwrap();
            assert_eq!(j.final_combo(), Some(Combo::default()));
            assert_eq!(j.combo_at(f64::NAN), Some(Combo::default()));
        }
        let chord = timeline("448,192,1000,1,0\n64,192,1000,1,0");
        let j = rules.judge(&chord, &input(&[(1000, 9)])).unwrap();
        assert_eq!(j.combo_at(999.), Some(Combo::default()));
        assert_eq!(
            j.final_combo(),
            Some(Combo {
                current: 2,
                maximum: 2
            })
        );
    }
}

#[test]
fn equal_time_tap_and_tail_results_retain_original_object_order() {
    let objects = "448,192,1000,128,0,1200:0:0:0:0:\n64,192,1200,1,0";
    let edges = [(1040, 8), (1216, 1)];
    let rules = StableRules::native_no_mod(5.).unwrap();
    let j = rules.judge(&timeline(objects), &input(&edges)).unwrap();
    let (traced, _) = rules
        .judge_with_trace(&timeline(objects), &input(&edges))
        .unwrap();
    assert_eq!(j.events(), traced.events());
    assert_eq!(
        j.events()
            .iter()
            .map(|e| (e.time_ms, e.object_index, e.result))
            .collect::<Vec<_>>(),
        [(1216., 0, R::Great), (1216., 1, R::Perfect)]
    );
    assert_eq!(j.counts_at(1215.).total(), 0);
    assert_eq!(j.counts_at(1216.).total(), 2);
    assert_eq!(
        j.combo_at(1140.),
        Some(Combo {
            current: 1,
            maximum: 1
        })
    );
    assert_eq!(
        j.final_combo(),
        Some(Combo {
            current: 3,
            maximum: 3
        })
    );

    // Changing columns must not change the model's order of these objects.
    let mirrored = judge(
        "64,192,1000,128,0,1200:0:0:0:0:\n448,192,1200,1,0",
        &[(1040, 1), (1216, 8)],
    );
    for (a, b) in j.events().iter().zip(mirrored.events()) {
        assert_eq!(
            (a.object_index, a.time_ms, a.result),
            (b.object_index, b.time_ms, b.result)
        );
    }
}

#[test]
fn equal_time_tap_success_precedes_later_object_early_miss() {
    let j = judge("448,192,1000,1,0\n64,192,1150,1,0", &[(1000, 9)]);
    assert_eq!(
        j.events()
            .iter()
            .map(|e| (e.object_index, e.result))
            .collect::<Vec<_>>(),
        [(0, R::Perfect), (1, R::Miss)]
    );
    assert_eq!(j.combo_at(999.), Some(Combo::default()));
    assert_eq!(
        j.combo_at(1000.),
        Some(Combo {
            current: 0,
            maximum: 1
        })
    );
    assert_eq!(j.final_combo(), j.combo_at(1000.));
}

#[test]
fn equal_time_frames_remain_distinct_from_a_single_chord_frame() {
    let objects = "448,192,1000,1,0\n64,192,1150,1,0";
    let track = input(&[(1000, 1), (1000, 9)]);
    assert_eq!(
        track
            .events()
            .iter()
            .map(|e| e.frame_index)
            .collect::<Vec<_>>(),
        [0, 1]
    );
    let j = StableRules::native_no_mod(5.)
        .unwrap()
        .judge(&timeline(objects), &track)
        .unwrap();
    assert_eq!(
        j.events()
            .iter()
            .map(|e| (e.object_index, e.result))
            .collect::<Vec<_>>(),
        [(1, R::Miss), (0, R::Perfect)]
    );
    assert_eq!(
        j.final_combo(),
        Some(Combo {
            current: 1,
            maximum: 1
        })
    );
    let chord = judge(objects, &[(1000, 9)]);
    assert_eq!(chord.final_counts(), j.final_counts());
    assert_eq!(
        chord.final_combo(),
        Some(Combo {
            current: 0,
            maximum: 1
        })
    );
    assert_eq!(
        input(&[(999, 0), (1000, 9)])
            .events()
            .iter()
            .map(|e| e.frame_index)
            .collect::<Vec<_>>(),
        [1, 1]
    );
}

#[test]
fn forced_miss_keeps_its_input_frame_after_an_earlier_equal_time_press() {
    let j = judge(
        "448,192,1000,1,0\n64,192,1010,1,0\n64,192,1100,1,0",
        &[(1100, 8), (1100, 9)],
    );
    assert_eq!(
        j.events()
            .iter()
            .map(|e| (e.object_index, e.result))
            .collect::<Vec<_>>(),
        [(0, R::Ok), (1, R::Miss), (2, R::Perfect)]
    );
    assert_eq!(j.events()[1].offset_ms, None);
    assert_eq!(
        j.final_combo(),
        Some(Combo {
            current: 1,
            maximum: 1
        })
    );
}

#[test]
fn stable_rate_and_hr_scale_raw_windows_before_flooring() {
    // Native Classic ScoreV1 windows at OD8, in beatmap milliseconds.
    for (rate, hr, expected) in [
        (1.0, true, [11., 28., 52., 73., 90., 117.]),
        (1.5, false, [24., 60., 109., 154., 190., 246.]),
        (1.5, true, [17., 42., 78., 110., 136., 175.]),
    ] {
        let rules = StableRules::native_with_mods(8.0, rate, hr).unwrap();
        assert_eq!(rules.clock_rate(), rate);
        assert_eq!(rules.hard_rock(), hr);
        for (kind, limit) in [R::Perfect, R::Great, R::Good, R::Ok, R::Meh, R::Miss]
            .into_iter()
            .zip(expected)
        {
            assert_eq!(rules.window(kind), limit);
            for sign in [-1.0, 1.0] {
                assert_eq!(rules.result_for(sign * (limit + 0.49)), Some(kind));
                assert_ne!(rules.result_for(sign * (limit + 0.51)), Some(kind));
            }
        }
    }
    let rules = StableRules::native_with_mods(8.0, 1.5, true).unwrap();
    assert_eq!(rules.result_for(17.5), Some(R::Great));
    assert_eq!(rules.result_for(42.5), Some(R::Great));
    assert_eq!(rules.result_for(42.5001), Some(R::Good));
    for rate in [0.0, 0.75, 1.25, 2.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            StableRules::native_with_mods(8.0, rate, true),
            Err(Error::InvalidClockRate)
        );
    }
}

#[test]
fn modded_tap_and_hold_deadlines_use_scaled_map_time_once() {
    for (rate, hr, ok) in [(1.0, true, 73), (1.5, false, 154), (1.5, true, 110)] {
        let rules = StableRules::native_with_mods(8.0, rate, hr).unwrap();
        for (offset, expected) in [(ok - 1, R::Ok), (ok, R::Miss)] {
            let j = rules
                .judge(&timeline(TAP), &input(&[(1000 + offset, 1)]))
                .unwrap();
            assert_eq!(j.events()[0].result, expected);
        }
        for (offset, expected) in [(ok, R::Ok), (ok + 1, R::Meh)] {
            let j = rules
                .judge(&timeline(HOLD), &input(&[(1000 + offset, 1), (2000, 0)]))
                .unwrap();
            assert_eq!(j.events()[0].result, expected);
        }
        let overheld = rules.judge(&timeline(HOLD), &input(&[(1000, 1)])).unwrap();
        assert_eq!(overheld.events()[0].result, R::Good);
        assert_eq!(overheld.counts_at(2000.0 + ok as f64 + 0.49).total(), 0);
        assert_eq!(overheld.counts_at(2000.0 + ok as f64 + 0.51).total(), 1);
    }
}

#[test]
fn hrdt_holds_combine_rounded_map_errors_and_use_scaled_body_lenience() {
    let rules = StableRules::native_with_mods(8.0, 1.5, true).unwrap();
    for (head, tail, expected) in [
        (20., 20., R::Perfect),
        (21., 0., R::Great),
        (20., 21., R::Great),
        (46., 46., R::Great),
        (47., 0., R::Good),
        (46., 47., R::Good),
    ] {
        assert_eq!(rules.hold_result_for(head, tail), Some(expected));
    }
    for (release, transition) in [(1863, T::BodyReleased), (1864, T::ReleasedAtTail)] {
        let (_, trace) = rules
            .judge_with_trace(&timeline(HOLD), &input(&[(1000, 1), (release, 0)]))
            .unwrap();
        assert_eq!(trace[1].kind, transition);
    }
}

#[test]
fn trace_retains_the_accepted_head_across_a_pre_head_release_and_regrab() {
    // Minimized DEUX EX MAXHINA input. This asserts our diagnostic account,
    // not an unverified original-client grade for the disputed hold.
    let notes = timeline(HOLD);
    let keys = input(&[(982, 1), (996, 0), (1009, 1), (2045, 0)]);
    let rules = StableRules::native_no_mod(5.0).unwrap();
    let (judged, trace) = rules.judge_with_trace(&notes, &keys).unwrap();
    assert_eq!(
        judged.events(),
        rules.judge(&notes, &keys).unwrap().events()
    );
    assert_eq!(
        trace.iter().map(|e| e.kind).collect::<Vec<_>>(),
        vec![
            T::HeadPressed,
            T::ReleasedBeforeHead,
            T::Regrabbed,
            T::ReleasedAtTail,
        ]
    );
    assert!(trace.iter().all(|e| e.head_offset_ms == Some(-18.0)));
    assert!(!trace[0].broken);
    assert!(trace[1..].iter().all(|e| e.broken));
    assert_eq!(judged.final_counts().total(), 1);
}

#[test]
fn tracing_preserves_order_for_equal_time_release_and_repress() {
    let (j, trace) = StableRules::native_no_mod(5.0)
        .unwrap()
        .judge_with_trace(&timeline(HOLD), &input(&[(1000, 1), (1500, 0), (1500, 1)]))
        .unwrap();
    assert_eq!(
        trace.iter().map(|e| e.kind).collect::<Vec<_>>(),
        vec![T::HeadPressed, T::BodyReleased, T::Regrabbed, T::Overheld,]
    );
    assert_eq!(trace[1].time_ms, trace[2].time_ms);
    assert!(trace
        .windows(2)
        .all(|pair| pair[0].time_ms <= pair[1].time_ms));
    assert_eq!(j.final_counts().total(), 1);
    assert_eq!(j.final_counts().hold_breaks, 0);
}

#[test]
fn diagnostic_timeouts_and_forced_misses_are_distinguished() {
    let rules = StableRules::native_no_mod(5.0).unwrap();
    let (_, trace) = rules
        .judge_with_trace(&timeline(HOLD), &input(&[]))
        .unwrap();
    assert_eq!(
        trace.iter().map(|e| e.kind).collect::<Vec<_>>(),
        vec![T::HeadExpired, T::TailExpired]
    );
    assert!(trace.iter().all(|e| e.head_offset_ms.is_none()));
    let (_, trace) = rules
        .judge_with_trace(
            &timeline("64,192,1000,1,0\n64,192,1050,1,0"),
            &input(&[(1050, 1)]),
        )
        .unwrap();
    assert_eq!(
        trace.iter().map(|e| e.kind).collect::<Vec<_>>(),
        vec![T::HeadPressed, T::HeadForcedMiss]
    );
    assert_eq!(trace[0].object_index, 1);
    assert_eq!(trace[1].object_index, 0);
}

#[test]
fn hold_combines_errors_but_also_limits_the_head_independently() {
    let rules = StableRules::native_no_mod(5.0).unwrap();
    for (head, tail, expected) in [
        (19.0, 19.0, R::Perfect),
        (20.0, 0.0, R::Great), // Combined error alone would incorrectly give Perfect.
        (19.0, 20.0, R::Great),
        (53.0, 54.0, R::Great),
        (54.0, 0.0, R::Good),
        (53.0, 55.0, R::Good),
        (82.0, 82.0, R::Good),
        (83.0, 0.0, R::Ok),
        (82.0, 83.0, R::Ok),
        (112.0, 112.0, R::Ok),
        (113.0, 0.0, R::Meh),
        (112.0, 113.0, R::Meh),
    ] {
        for sign in [-1.0, 1.0] {
            assert_eq!(rules.hold_result_for(head * sign, tail), Some(expected));
            assert_eq!(rules.hold_result_for(head, tail * sign), Some(expected));
        }
    }
    assert_eq!(rules.hold_result_for(f64::NAN, 0.0), None);
    assert_eq!(rules.hold_result_for(0.0, f64::INFINITY), None);
}

#[test]
fn stable_hold_has_one_result_at_release_and_seekable_counts() {
    let j = judge(HOLD, &[(1000, 1), (2000, 0)]);
    assert_eq!(j.events().len(), 1);
    assert_eq!(j.events()[0].part, Part::Hold);
    assert_eq!(j.events()[0].offset_ms, Some(0.0));
    assert_eq!(j.counts_at(2000.0).get(R::Perfect), 1);
    assert_eq!(j.counts_at(1999.0).total(), 0);
    assert_eq!(j.final_counts().hold_breaks, 0);
}

#[test]
fn overholding_finishes_automatically_before_the_late_release() {
    // King For A Day [Normal], object 56: OD7, head +19, release +109.
    // The whole replay has no misses; a tap-style tail deadline adds two.
    let rules = StableRules::native_no_mod(7.0).unwrap();
    let j = rules
        .judge(&timeline(HOLD), &input(&[(1019, 1), (2109, 0)]))
        .unwrap();
    assert_eq!(j.final_counts().get(R::Good), 1);
    assert_eq!(j.events()[0].offset_ms, None);
    assert!(j.events()[0].time_ms < 2109.0);
}

#[test]
fn body_breaks_and_missing_heads_can_recover_only_to_meh() {
    for edges in [
        vec![(1000, 1), (1500, 0), (1600, 1), (2000, 0)],
        vec![(1500, 1), (2000, 0)],
        vec![(2030, 1), (2040, 0)],
    ] {
        let j = judge(HOLD, &edges);
        assert_eq!(j.final_counts().get(R::Meh), 1);
        assert_eq!(j.final_counts().total(), 1);
    }
    assert_eq!(
        judge(HOLD, &[(1000, 1), (1500, 0)])
            .final_counts()
            .get(R::Miss),
        1
    );
}

#[test]
fn unreleased_and_unplayed_holds_finish_once_with_different_results() {
    for (edges, expected) in [(vec![], R::Miss), (vec![(1000, 1)], R::Good)] {
        let j = judge(HOLD, &edges);
        assert_eq!(j.final_counts().get(expected), 1);
        assert_eq!(j.final_counts().total(), 1);
        assert_eq!(j.counts_at(2000.0).total(), 0);
        assert_eq!(j.events()[0].offset_ms, None);
        assert_eq!(j.events()[0].part, Part::Hold);
    }
}

#[test]
fn a_tap_at_the_tail_time_is_not_stolen_by_a_broken_hold() {
    let j = judge(
        &format!("{HOLD}\n64,192,2000,1,0"),
        &[(1000, 1), (1500, 0), (2000, 1), (2010, 0)],
    );
    assert_eq!(j.final_counts().get(R::Perfect), 1);
    assert_eq!(j.final_counts().get(R::Miss), 1);
    assert_eq!(j.final_counts().total(), 2);
}

#[test]
fn expired_body_does_not_steal_an_early_press_on_the_next_head() {
    let j = judge(
        &format!("{HOLD}\n64,192,2100,128,0,2500:0:0:0:0:"),
        &[(2076, 1), (2502, 0)],
    );
    assert_eq!(j.final_counts().get(R::Miss), 1);
    assert_eq!(j.final_counts().get(R::Great), 1);
    assert_eq!(
        j.events()
            .iter()
            .find(|e| e.object_index == 1)
            .unwrap()
            .time_ms,
        2502.0
    );
}

#[test]
fn early_tail_release_and_equal_time_chords_are_distinct_objects() {
    let j = judge(
        &format!("{HOLD}\n192,192,1000,128,0,2000:0:0:0:0:"),
        &[(1000, 3), (1980, 0)],
    );
    assert_eq!(j.final_counts().get(R::Perfect), 2);
    assert_eq!(j.events()[0].offset_ms, Some(-20.0));
    assert_ne!(j.events()[0].object_index, j.events()[1].object_index);
}

#[test]
fn stable_scorev1_has_constant_perfect_and_truncated_other_windows() {
    for (od, expected) in [
        (0.0, [16.0, 64.0, 97.0, 127.0, 151.0, 188.0]),
        (5.0, [16.0, 49.0, 82.0, 112.0, 136.0, 173.0]),
        (8.5, [16.0, 38.0, 71.0, 101.0, 125.0, 162.0]),
        (10.0, [16.0, 34.0, 67.0, 97.0, 121.0, 158.0]),
    ] {
        let rules = StableRules::native_no_mod(od).unwrap();
        for (kind, limit) in [R::Perfect, R::Great, R::Good, R::Ok, R::Meh, R::Miss]
            .into_iter()
            .zip(expected)
        {
            assert_eq!(rules.window(kind), limit);
            assert_eq!(rules.result_for(limit), Some(kind));
            assert_eq!(rules.result_for(-limit), Some(kind));
            assert_ne!(rules.result_for(limit + 1.0), Some(kind));
        }
    }
    for od in [-1.0, 11.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            StableRules::native_no_mod(od),
            Err(Error::InvalidDifficulty)
        );
    }
}

#[test]
fn hit_errors_use_ties_to_even_but_do_not_round_map_timestamps() {
    let rules = StableRules::native_no_mod(5.0).unwrap();
    assert_eq!(rules.result_for(16.5), Some(R::Perfect));
    assert_eq!(rules.result_for(-16.5), Some(R::Perfect));
    assert_eq!(rules.result_for(16.5001), Some(R::Great));
    assert_eq!(rules.result_for(49.5), Some(R::Good));
    assert_eq!(rules.result_for(f64::NAN), None);
    let j = judge("64,192,1000.75,1,0", &[(1017, 1)]);
    assert_eq!(j.events()[0].offset_ms, Some(16.25));
    assert_eq!(j.events()[0].result, R::Perfect);
}

#[test]
fn early_meh_is_possible_but_late_meh_is_a_miss() {
    assert_eq!(judge(TAP, &[(870, 1)]).events()[0].result, R::Meh);
    let late = judge(TAP, &[(1130, 1)]);
    assert_eq!(late.events()[0].result, R::Miss);
    assert_eq!(late.events()[0].offset_ms, None);
    assert_eq!(late.counts_at(1111.4999).total(), 0);
    assert_eq!(late.counts_at(1111.5).get(R::Miss), 1);
    let rules = StableRules::native_no_mod(8.5).unwrap();
    let odd = rules.judge(&timeline(TAP), &input(&[])).unwrap();
    assert_eq!(odd.counts_at(1100.5).total(), 0);
    assert_eq!(odd.counts_at(1100.5001).get(R::Miss), 1);
}

#[test]
fn final_tap_millisecond_is_excluded_but_hold_head_retains_it() {
    // Independent stable parity fixture at OD8: +102 hits, +103 misses.
    // A hold head at +103 is still valid, so it must use a separate deadline.
    let rules = StableRules::native_no_mod(8.0).unwrap();
    for (offset, expected) in [(102, R::Ok), (103, R::Miss), (104, R::Miss)] {
        let j = rules
            .judge(&timeline(TAP), &input(&[(1000 + offset, 1)]))
            .unwrap();
        assert_eq!(j.final_counts().get(expected), 1);
    }
    let j = rules
        .judge(&timeline(HOLD), &input(&[(1103, 1), (2000, 0)]))
        .unwrap();
    assert_eq!(j.final_counts().get(R::Ok), 1);
    let j = rules
        .judge(&timeline(HOLD), &input(&[(1104, 1), (2000, 0)]))
        .unwrap();
    assert_eq!(j.final_counts().get(R::Meh), 1);
}

#[test]
fn expired_tap_does_not_consume_the_early_press_for_the_following_note() {
    // ischezayu [Insane], column 1, objects 662 and 665. Keeping the first
    // note alive at +101 ms incorrectly consumes the -49 ms second-note hit.
    let rules = StableRules::native_no_mod(8.5).unwrap();
    let j = rules
        .judge(
            &timeline("64,192,51891,1,0\n64,192,52041,1,0"),
            &input(&[(51992, 1), (52020, 0), (52134, 1)]),
        )
        .unwrap();
    assert_eq!(j.events().len(), 2);
    assert_eq!(j.events()[0].object_index, 0);
    assert_eq!(j.events()[0].result, R::Miss);
    assert_eq!(j.events()[1].object_index, 1);
    assert_eq!(j.events()[1].result, R::Good);
    assert_eq!(j.events()[1].offset_ms, Some(-49.0));
}

#[test]
fn overhold_grade_depends_on_head_error_and_od_not_eventual_release_time() {
    // Boundaries independently documented by Interlude's stable parity tests.
    // At OD8, 200 ends at 19 ms; 100 ends at 79 ms.
    let rules = StableRules::native_no_mod(8.0).unwrap();
    for (head, expected) in [(19, R::Good), (20, R::Ok), (79, R::Ok), (80, R::Meh)] {
        for sign in [-1, 1] {
            let baseline = rules
                .judge(&timeline(HOLD), &input(&[(1000 + head * sign, 1)]))
                .unwrap();
            assert_eq!(baseline.final_counts().get(expected), 1);
            for release in [2104, 2500, 100_000] {
                let j = rules
                    .judge(
                        &timeline(HOLD),
                        &input(&[(1000 + head * sign, 1), (release, 0)]),
                    )
                    .unwrap();
                assert_eq!(j.events(), baseline.events());
            }
        }
    }
    // This is not just a fixed cap based on the initial Perfect window.
    assert_eq!(judge(HOLD, &[(1028, 1)]).final_counts().get(R::Good), 1);
    assert_eq!(judge(HOLD, &[(1029, 1)]).final_counts().get(R::Ok), 1);
}

#[test]
fn recovered_hold_uses_overhold_policy_only_while_the_key_is_down() {
    let held = judge(HOLD, &[(1000, 1), (1500, 0), (1600, 1)]);
    assert_eq!(held.final_counts().get(R::Good), 1);
    let released = judge(HOLD, &[(1000, 1), (1500, 0), (1600, 1), (2000, 0)]);
    assert_eq!(released.final_counts().get(R::Meh), 1);
    let dropped = judge(HOLD, &[(1000, 1), (1500, 0)]);
    assert_eq!(dropped.final_counts().get(R::Miss), 1);
}

#[test]
fn too_early_press_and_a_held_key_do_not_hit_later_notes() {
    assert_eq!(
        judge(TAP, &[(800, 1), (1200, 1)])
            .final_counts()
            .get(R::Miss),
        1
    );
    let recovered = judge(TAP, &[(800, 1), (900, 0), (1000, 1)]);
    assert_eq!(recovered.final_counts().get(R::Perfect), 1);
    let early_miss = judge(TAP, &[(830, 1), (900, 0), (1000, 1)]);
    assert_eq!(early_miss.final_counts().total(), 1);
    assert_eq!(early_miss.events()[0].offset_ms, Some(-170.0));
    assert_eq!(early_miss.events()[0].result, R::Miss);
}

#[test]
fn chords_and_equal_time_represses_are_counted_once_and_seek_backwards() {
    let j = judge(
        "64,192,1000,1,0\n192,192,1000,1,0\n64,192,1100,1,0",
        &[(1000, 3), (1100, 2), (1100, 3)],
    );
    assert_eq!(j.final_counts().get(R::Perfect), 3);
    assert!(j.events().iter().all(|e| e.part == Part::Tap));
    assert_eq!(j.counts_at(1100.0).total(), 3);
    assert_eq!(j.counts_at(1000.0).total(), 2);
    assert_eq!(j.counts_at(999.0).total(), 0);
    assert_eq!(j.counts_at(f64::NAN).total(), 0);
}

#[test]
fn overlaps_duplicates_and_incompatible_columns_are_rejected() {
    let rules = StableRules::native_no_mod(5.0).unwrap();
    assert!(matches!(
        rules.judge(
            &timeline("64,192,1000,128,0,2000:0:0:0:0:\n64,192,1500,1,0"),
            &input(&[])
        ),
        Err(Error::OverlappingNotes(1))
    ));
    assert!(matches!(
        rules.judge(&timeline("64,192,1000,1,0\n64,192,1000,1,0"), &input(&[])),
        Err(Error::OverlappingNotes(1))
    ));
    assert!(matches!(
        rules.judge(&timeline(TAP), &InputTrack::new(&[], 7).unwrap()),
        Err(Error::InvalidColumns)
    ));
}

#[test]
fn hold_combo_ticks_follow_grab_and_stop_at_release_or_map_tail() {
    let combo = |n| {
        Some(Combo {
            current: n,
            maximum: n,
        })
    };
    for (edges, final_count) in [
        (vec![(1000, 1), (2000, 0)], 11),
        (vec![(1040, 1), (2040, 0)], 10),
        (vec![(1000, 1), (1950, 0)], 10),
        (vec![(1000, 1)], 11),
    ] {
        let j = judge(HOLD, &edges);
        let head = edges[0].0 as f64;
        assert_eq!(j.combo_at(head + 99.999), combo(0));
        assert_eq!(j.combo_at(head + 100.), combo(1));
        assert_eq!(j.final_combo(), combo(final_count));
        assert_eq!(j.combo_at(f64::INFINITY), j.final_combo());
        assert_eq!(j.combo_at(f64::NAN), combo(0));
        assert_eq!(j.combo_at(f64::NEG_INFINITY), combo(0));
    }
    let overheld = judge(HOLD, &[(1000, 1)]);
    assert_eq!(overheld.combo_at(2000.), combo(10));
    assert_eq!(overheld.combo_at(2100.), combo(10));
    assert_eq!(overheld.counts_at(2100.).total(), 0);
}

#[test]
fn hold_combo_breaks_reset_and_regrabs_restart_the_tick_phase() {
    let j = judge(HOLD, &[(1000, 1), (1400, 0), (1550, 1), (2000, 0)]);
    for (time, current, maximum) in [
        (1399., 3, 3),
        (1400., 0, 4),
        (1550., 0, 4),
        (1649., 0, 4),
        (1650., 1, 4),
        (1950., 4, 4),
        (2000., 5, 5),
    ]
    .into_iter()
    .rev()
    {
        assert_eq!(
            j.combo_at(time),
            Some(Combo { current, maximum }),
            "at {time}"
        );
    }
    assert_eq!(j.events().len(), 1);
    assert_eq!(j.events()[0].result, R::Meh);
    let recovered = judge(HOLD, &[(1500, 1), (2000, 0)]);
    assert_eq!(
        recovered.final_combo(),
        Some(Combo {
            current: 6,
            maximum: 6
        })
    );
    let prehead = judge(HOLD, &[(900, 1), (950, 0), (980, 1), (2000, 0)]);
    assert_eq!(prehead.combo_at(1079.), Some(Combo::default()));
    assert_eq!(
        prehead.final_combo(),
        Some(Combo {
            current: 11,
            maximum: 11
        })
    );
}

#[test]
fn expired_previous_hold_does_not_close_the_next_hold_interval() {
    let j = judge(
        "64,192,1000,128,0,2000:0:0:0:0:\n64,192,2050,128,0,3000:0:0:0:0:",
        &[(2050, 1), (3000, 0)],
    );
    assert_eq!(j.events()[0].result, R::Miss);
    assert_eq!(
        j.combo_at(2250.),
        Some(Combo {
            current: 2,
            maximum: 2
        })
    );
    assert_eq!(
        j.final_combo(),
        Some(Combo {
            current: 10,
            maximum: 10
        })
    );
}

#[test]
fn compact_hold_combo_matches_an_explicit_two_lane_tick_stream() {
    let objects = "64,192,1000,128,0,2000:0:0:0:0:\n192,192,1000,128,0,2000:0:0:0:0:";
    let edges = [(1000, 3), (1350, 2), (1450, 3), (2000, 0)];
    let rules = StableRules::native_no_mod(5.).unwrap();
    let j = rules.judge(&timeline(objects), &input(&edges)).unwrap();
    let (traced, _) = rules
        .judge_with_trace(&timeline(objects), &input(&edges))
        .unwrap();
    // Small independent event-by-event reference, including simultaneous ticks,
    // a body reset and two combined tail judgements.
    let mut actions: Vec<(i64, bool)> = (1100..=2000).step_by(100).map(|t| (t, true)).collect();
    actions.extend([1100, 1200, 1300, 1550, 1650, 1750, 1850, 1950].map(|t| (t, true)));
    actions.extend([(1350, false), (2000, true), (2000, true)]);
    actions.sort_by_key(|a| a.0);
    for time in (900..=2300).rev() {
        let mut expected = Combo::default();
        for &(_, add) in actions.iter().filter(|a| a.0 <= time) {
            expected.current = if add { expected.current + 1 } else { 0 };
            expected.maximum = expected.maximum.max(expected.current);
        }
        assert_eq!(j.combo_at(time as f64), Some(expected), "at {time}");
        assert_eq!(traced.combo_at(time as f64), Some(expected));
    }
}

#[test]
fn hold_combo_does_not_expand_billions_of_ticks_or_wrap_its_counter() {
    let j = judge("64,192,1000,128,0,100000001000:0:0:0:0:", &[(1000, 1)]);
    assert_eq!(
        j.final_combo(),
        Some(Combo {
            current: 1_000_000_001,
            maximum: 1_000_000_001
        })
    );
    let unrepresentable = judge("64,192,1000,128,0,1000000001000:0:0:0:0:", &[(1000, 1)]);
    assert_eq!(unrepresentable.final_combo(), None);
    assert_eq!(unrepresentable.combo_at(1100.), None);
    assert_eq!(unrepresentable.events().len(), 1);
}

#[test]
fn mixed_lane_input_preserves_object_and_combo_invariants() {
    let objects = [64, 192, 320, 448]
        .into_iter()
        .flat_map(|x| {
            [
                format!("{x},192,1000,128,0,1200:0:0:0:0:"),
                format!("{x},192,1250,1,0"),
                format!("{x},192,1300,128,0,1800:0:0:0:0:"),
                format!("{x},192,1900,1,0"),
            ]
        })
        .collect::<Vec<_>>()
        .join("\n");
    let notes = timeline(&objects);
    let rules = StableRules::native_no_mod(5.).unwrap();
    for seed in 1..=512u64 {
        let mut rng = seed;
        let edges = (700..=2300)
            .step_by(25)
            .map(|t| {
                rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                (t, (rng >> 32) as u32 & 15)
            })
            .collect::<Vec<_>>();
        let keys = input(&edges);
        let j = rules.judge(&notes, &keys).unwrap();
        let (traced, _) = rules.judge_with_trace(&notes, &keys).unwrap();
        assert_eq!(j.events(), traced.events(), "seed {seed}");
        let ids = j
            .events()
            .iter()
            .map(|e| e.object_index)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(ids.len(), 16, "seed {seed}");
        assert_eq!(j.final_counts().total(), 16);
        let mut previous_max = 0;
        let mut previous_count = 0;
        for time in (700..=2300).step_by(5) {
            let combo = j.combo_at(time as f64).unwrap();
            assert!(combo.current <= combo.maximum, "seed {seed}, time {time}");
            assert!(combo.maximum >= previous_max, "seed {seed}, time {time}");
            let count = j.counts_at(time as f64).total();
            assert!(count >= previous_count && count <= 16);
            previous_max = combo.maximum;
            previous_count = count;
        }
        assert_eq!(j.combo_at(2300.), j.final_combo());
        assert_eq!(j.combo_at(700.), Some(Combo::default()));
        assert_eq!(j.final_combo(), traced.final_combo());
    }
}
