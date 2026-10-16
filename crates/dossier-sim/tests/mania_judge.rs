use dossier_beatmap::Beatmap;
use dossier_replay::{Keys, ReplayFrame};
use dossier_sim::mania::{
    judge::{Combo, Judge, LazerRules, Part, ResultKind as R},
    Error, InputTrack, Timeline,
};

fn timeline(objects: &str) -> Timeline {
    Timeline::new(&Beatmap::parse(&format!("osu file format v14\n[General]\nMode:3\n[Difficulty]\nCircleSize:4\n[HitObjects]\n{objects}")).unwrap()).unwrap()
}
fn input(edges: &[(i64, u32)]) -> InputTrack {
    InputTrack::new(&frames(edges), 4).unwrap()
}
fn frames(edges: &[(i64, u32)]) -> Vec<ReplayFrame> {
    edges
        .iter()
        .map(|&(time_ms, keys)| ReplayFrame {
            time_ms,
            x: keys as f32,
            y: 0.0,
            keys: Keys(0),
        })
        .collect()
}
fn judge(objects: &str, edges: &[(i64, u32)]) -> Judge {
    Judge::new(
        &timeline(objects),
        &input(edges),
        LazerRules::native_no_mod(5.0).unwrap(),
    )
    .unwrap()
}
const TAP: &str = "64,192,1000,1,0";
const HOLD: &str = "64,192,1000,128,0,2000:0:0:0:0:";

#[test]
fn equal_time_cross_column_inputs_keep_their_replay_order_for_score() {
    let map = timeline("64,192,1000,1,0\n64,192,2000,1,0\n448,192,2930,1,0\n64,192,2900,1,0");
    let frames = frames(&[
        (1000, 1),
        (1010, 0),
        (2000, 1),
        (2010, 0),
        (3000, 8),
        (3000, 9),
    ]);
    let rules = LazerRules::native_no_mod(5.0).unwrap();
    let j = Judge::new(&map, &InputTrack::new(&frames, 4).unwrap(), rules).unwrap();
    assert_eq!(
        j.events()
            .iter()
            .map(|e| (e.column, e.result))
            .collect::<Vec<_>>(),
        [(0, R::Perfect), (0, R::Perfect), (3, R::Good), (0, R::Ok)]
    );
    let zero = Judge::with_frame_offsets(&map, &frames, &[0.0; 6], rules).unwrap();
    assert_eq!(zero.events(), j.events());
    assert_eq!(zero.final_score(), j.final_score());
    // Separating the last two frames without crossing a grade boundary must
    // not change their combo weights or final score.
    let separated =
        Judge::with_frame_offsets(&map, &frames, &[0.0, 0.0, 0.0, 0.0, 0.0, 0.25], rules).unwrap();
    assert_eq!(separated.final_score(), j.final_score());
    let reversed = Judge::new(
        &map,
        &input(&[
            (1000, 1),
            (1010, 0),
            (2000, 1),
            (2010, 0),
            (3000, 1),
            (3000, 9),
        ]),
        rules,
    )
    .unwrap();
    assert_eq!(reversed.final_counts(), j.final_counts());
    assert_eq!(reversed.final_combo(), j.final_combo());
    assert_ne!(reversed.final_score(), j.final_score());
    assert_eq!(j.combo_at(3000.0).unwrap().current, 4);
    assert_eq!(j.combo_at(2999.0).unwrap().current, 2);
}

#[test]
fn cross_column_press_before_equal_time_hold_break_keeps_earned_combo() {
    let notes = "64,192,1000,128,0,2000:0:0:0:0:\n448,192,1500,1,0";
    let j = judge(notes, &[(1000, 1), (1500, 9), (1500, 8)]);
    assert_eq!(
        j.combo_at(1500.0),
        Some(Combo {
            current: 0,
            maximum: 2
        })
    );
    assert_eq!(j.final_combo().unwrap().maximum, 2);
    let reversed = judge(notes, &[(1000, 1), (1500, 0), (1500, 8)]);
    assert_eq!(reversed.final_counts(), j.final_counts());
    assert_eq!(
        reversed.combo_at(1500.0),
        Some(Combo {
            current: 1,
            maximum: 1
        })
    );
}

#[test]
fn forced_results_stay_with_their_input_before_the_next_equal_time_frame() {
    let j = judge(
        "448,192,900,1,0\n448,192,1000,1,0\n64,192,1000,1,0",
        &[(1000, 8), (1000, 9)],
    );
    assert_eq!(
        j.events()
            .iter()
            .map(|e| (e.column, e.result))
            .collect::<Vec<_>>(),
        [(3, R::Perfect), (3, R::Miss), (0, R::Perfect)]
    );
    assert_eq!(
        j.final_combo(),
        Some(Combo {
            current: 1,
            maximum: 1
        })
    );
}

#[test]
fn lazer_score_separates_perfect_from_great_accuracy() {
    let perfect = judge(TAP, &[(1000, 1)]).final_score().unwrap();
    assert_eq!(perfect.accuracy, 1.0);
    assert_eq!(perfect.total_score_without_mods, 1_000_000);
    let great = judge(TAP, &[(1030, 1)]).final_score().unwrap();
    assert_eq!(great.accuracy, 300.0 / 305.0);
    assert_eq!(great.total_score_without_mods, 946049);
    let mixed = judge(
        "64,192,1000,1,0\n64,192,2000,1,0",
        &[(1000, 1), (1010, 0), (2030, 1)],
    )
    .final_score()
    .unwrap();
    assert_eq!(mixed.accuracy, 605.0 / 610.0);
    assert_eq!(mixed.total_score_without_mods, 972583);
}

#[test]
fn perfect_run_beyond_combo_weight_cap_reaches_one_million() {
    let notes = (0..500)
        .map(|i| format!("64,192,{},1,0", 1000 + i * 200))
        .collect::<Vec<_>>()
        .join("\n");
    let edges = (0..500)
        .flat_map(|i| [(1000 + i * 200, 1), (1010 + i * 200, 0)])
        .collect::<Vec<_>>();
    let j = judge(&notes, &edges);
    let score = j.final_score().unwrap();
    assert_eq!(score.accuracy, 1.0);
    assert_eq!(score.total_score_without_mods, 1_000_000);
    assert_eq!(j.final_combo().unwrap().maximum, 500);
    assert!(
        j.score_at(1000.0 + 399.0 * 200.0)
            .unwrap()
            .total_score_without_mods
            < 1_000_000
    );
}

#[test]
fn score_and_accuracy_seek_without_future_results() {
    let j = judge(
        "64,192,1000,1,0\n64,192,2000,1,0",
        &[(1000, 1), (1010, 0), (2000, 1)],
    );
    assert_eq!(j.final_score().unwrap().total_score_without_mods, 1_000_000);
    let halfway = j.score_at(1000.0).unwrap();
    assert_eq!(halfway.accuracy, 1.0);
    assert_eq!(halfway.total_score_without_mods, 500_000);
    let initial = j.score_at(999.0).unwrap();
    assert_eq!(initial.accuracy, 1.0);
    assert_eq!(initial.total_score_without_mods, 0);
    assert_eq!(j.score_at(f64::NAN), Some(initial));
    assert_eq!(j.score_at(f64::NEG_INFINITY), Some(initial));
    assert_eq!(j.score_at(f64::INFINITY), j.final_score());
    assert_eq!(judge("", &[]).final_score(), Some(initial));
}

#[test]
fn body_break_changes_future_weighting_not_current_accuracy_or_score() {
    let j = judge(HOLD, &[(1000, 1), (1500, 0), (1700, 1), (2000, 0)]);
    assert_eq!(j.score_at(1000.0), j.score_at(1500.0));
    assert_eq!(j.score_at(1500.0), j.score_at(1700.0));
    let final_score = j.final_score().unwrap();
    assert_eq!(final_score.accuracy, 355.0 / 610.0);
    assert_eq!(final_score.total_score_without_mods, 240811);
    assert_eq!(j.final_counts().total(), 2);
    assert_eq!(j.final_counts().hold_breaks, 1);
}

#[test]
fn missed_and_unreleased_notes_have_finite_scores_after_eof() {
    let missed = judge(HOLD, &[]).final_score().unwrap();
    assert_eq!(missed.accuracy, 0.0);
    assert_eq!(missed.total_score_without_mods, 0);
    let held = judge(HOLD, &[(1000, 1)]);
    assert_eq!(held.score_at(1000.0).unwrap().accuracy, 1.0);
    assert_eq!(held.final_score().unwrap().accuracy, 0.5);
    // Head earns half the combo portion; accuracy portion is 850000 * .5^3.
    assert_eq!(held.final_score().unwrap().total_score_without_mods, 181250);
}

#[test]
fn identical_counts_and_max_combo_do_not_imply_identical_score() {
    let notes =
        "64,192,1000,1,0\n64,192,2000,1,0\n64,192,3000,1,0\n64,192,4000,1,0\n64,192,5000,1,0";
    let a = judge(
        notes,
        &[
            (1000, 1),
            (1010, 0),
            (2000, 1),
            (2010, 0),
            (3070, 1),
            (3110, 0),
            (4100, 1),
            (4110, 0),
            (5000, 1),
        ],
    );
    let b = judge(
        notes,
        &[
            (1000, 1),
            (1010, 0),
            (2000, 1),
            (2010, 0),
            (3100, 1),
            (3110, 0),
            (4070, 1),
            (4110, 0),
            (5000, 1),
        ],
    );
    assert_eq!(a.final_counts(), b.final_counts());
    assert_eq!(a.final_combo(), b.final_combo());
    assert_eq!(
        a.final_score().unwrap().accuracy,
        b.final_score().unwrap().accuracy
    );
    assert!(
        a.final_score().unwrap().total_score_without_mods
            < b.final_score().unwrap().total_score_without_mods
    );
}

#[test]
fn combo_resets_on_miss_and_seeks_without_borrowing_future_maximum() {
    let j = judge("64,192,1000,1,0\n64,192,1100,1,0\n64,192,1200,1,0\n64,192,1400,1,0\n64,192,1500,1,0\n64,192,1600,1,0",
        &[(1000,1),(1010,0),(1100,1),(1110,0),(1400,1),(1410,0),(1500,1),(1510,0),(1600,1)]);
    assert_eq!(
        j.final_combo(),
        Some(Combo {
            current: 3,
            maximum: 3
        })
    );
    assert_eq!(
        j.combo_at(1350.0),
        Some(Combo {
            current: 0,
            maximum: 2
        })
    );
    assert_eq!(
        j.combo_at(1100.0),
        Some(Combo {
            current: 2,
            maximum: 2
        })
    );
    assert_eq!(
        j.combo_at(1000.0),
        Some(Combo {
            current: 1,
            maximum: 1
        })
    );
    assert_eq!(j.combo_at(999.0), Some(Combo::default()));
}

#[test]
fn hold_break_resets_combo_without_adding_a_basic_miss() {
    let j = judge(
        "64,192,1000,128,0,2000:0:0:0:0:\n192,192,1600,1,0",
        &[
            (1000, 1),
            (1500, 0),
            (1600, 2),
            (1610, 0),
            (1700, 1),
            (2000, 0),
        ],
    );
    assert_eq!(j.combo_at(1499.0).unwrap().current, 1);
    assert_eq!(j.combo_at(1500.0).unwrap().current, 0);
    assert_eq!(j.combo_at(1700.0).unwrap().current, 1); // re-grab adds no hit
    assert_eq!(
        j.final_combo(),
        Some(Combo {
            current: 2,
            maximum: 2
        })
    );
    assert_eq!(j.final_counts().get(R::Meh), 1);
    assert_eq!(j.final_counts().get(R::Miss), 0);
    assert_eq!(j.final_counts().hold_breaks, 1);
}

#[test]
fn successful_chord_heads_and_tails_each_increase_combo_once() {
    let j = judge(
        "64,192,1000,128,0,2000:0:0:0:0:\n192,192,1000,128,0,2000:0:0:0:0:",
        &[(1000, 3), (1100, 3), (1900, 3), (2000, 0)],
    );
    assert_eq!(
        j.combo_at(1000.0),
        Some(Combo {
            current: 2,
            maximum: 2
        })
    );
    assert_eq!(
        j.combo_at(1999.0),
        Some(Combo {
            current: 2,
            maximum: 2
        })
    );
    assert_eq!(
        j.final_combo(),
        Some(Combo {
            current: 4,
            maximum: 4
        })
    );
}

#[test]
fn automatic_tail_miss_after_eof_resets_current_but_keeps_maximum() {
    let j = judge(HOLD, &[(1000, 1)]);
    assert_eq!(
        j.combo_at(2000.0),
        Some(Combo {
            current: 1,
            maximum: 1
        })
    );
    assert_eq!(
        j.final_combo(),
        Some(Combo {
            current: 0,
            maximum: 1
        })
    );
    assert_eq!(j.combo_at(f64::INFINITY), j.final_combo());
    assert_eq!(j.combo_at(f64::NEG_INFINITY), Some(Combo::default()));
    assert_eq!(j.combo_at(f64::NAN), Some(Combo::default()));
    assert_eq!(judge("", &[]).final_combo(), Some(Combo::default()));
}

#[test]
fn stable_score_is_unavailable_and_hold_combo_counts_ticks_while_held() {
    use dossier_sim::mania::stable::StableRules;
    let judged = |objects: &str| {
        StableRules::native_no_mod(5.0)
            .unwrap()
            .judge(&timeline(objects), &input(&[(1000, 1), (2000, 0)]))
            .unwrap()
    };
    let combo = |current, maximum| Some(Combo { current, maximum });
    for objects in [
        "",
        TAP,
        HOLD,
        "64,192,1000,1,0\n64,192,2000,128,0,3000:0:0:0:0:",
    ] {
        let j = judged(objects);
        assert_eq!(j.combo_at(0.0), combo(0, 0));
        assert_eq!(j.score_at(1000.0), None);
        assert_eq!(j.final_score(), None);
    }
    let hold = judged(HOLD);
    assert_eq!(hold.combo_at(1000.0), combo(0, 0));
    assert_eq!(hold.combo_at(1550.0), combo(5, 5));
    assert_eq!(hold.final_combo(), combo(11, 11));
    let missed_head = judged("64,192,1000,1,0\n64,192,2000,128,0,3000:0:0:0:0:");
    assert_eq!(missed_head.combo_at(1000.0), combo(1, 1));
    assert_eq!(missed_head.final_combo(), combo(0, 1));
}

#[test]
fn fractional_frame_probe_changes_note_lock_without_editing_map_or_replay() {
    let map = timeline("64,192,94312,1,0\n64,192,94404,1,0");
    let frames = frames(&[(94404, 1), (94420, 0)]);
    let rules = LazerRules::native_no_mod(5.0).unwrap();
    let original = Judge::new(&map, &InputTrack::new(&frames, 4).unwrap(), rules).unwrap();
    let zero = Judge::with_frame_offsets(&map, &frames, &[0.0, 0.0], rules).unwrap();
    assert_eq!(zero.events(), original.events());
    assert_eq!(zero.final_combo(), original.final_combo());
    assert_eq!(zero.final_score(), original.final_score());
    let before = Judge::with_frame_offsets(&map, &frames, &[-0.25, 0.0], rules).unwrap();
    assert_eq!(before.events()[0].object_index, 0);
    assert_eq!(before.events()[0].time_ms, 94403.75);
    assert_eq!(before.events()[0].result, R::Ok);
    assert_eq!(before.final_counts().get(R::Miss), 1);
    assert_eq!(original.events()[0].object_index, 1);
    assert_eq!(original.final_counts().get(R::Perfect), 1);
    assert_eq!(frames[0].time_ms, 94404);
    assert_eq!(
        map.visible_at(94312.0, 1000.0).unwrap()[0].note.start_ms,
        94312.0
    );
}

#[test]
fn frame_probe_moves_all_chord_edges_together_and_retains_hold_state() {
    let map = timeline("64,192,1000,128,0,2000:0:0:0:0:\n192,192,1000,128,0,2000:0:0:0:0:");
    let frames = frames(&[(1000, 3), (2025, 0)]);
    let rules = LazerRules::native_no_mod(8.0).unwrap();
    let original = Judge::new(&map, &InputTrack::new(&frames, 4).unwrap(), rules).unwrap();
    let adjusted = Judge::with_frame_offsets(&map, &frames, &[0.0, -0.3], rules).unwrap();
    assert_eq!(original.final_counts().get(R::Great), 2);
    assert_eq!(adjusted.final_counts().get(R::Perfect), 4);
    let tails: Vec<_> = adjusted
        .events()
        .iter()
        .filter(|e| e.part == Part::Tail)
        .collect();
    assert_eq!(tails.len(), 2);
    assert!(tails.iter().all(|e| e.time_ms == 2024.7));
    assert_eq!(adjusted.final_counts().hold_breaks, 0);
}

#[test]
fn zero_probe_preserves_regrabs_and_equal_time_release_repress() {
    let map = timeline(HOLD);
    let frames = frames(&[(900, 1), (920, 0), (920, 1), (2000, 0)]);
    let rules = LazerRules::native_no_mod(5.0).unwrap();
    let ordinary = Judge::new(&map, &InputTrack::new(&frames, 4).unwrap(), rules).unwrap();
    let probe = Judge::with_frame_offsets(&map, &frames, &[0.0; 4], rules).unwrap();
    assert_eq!(probe.events(), ordinary.events());
    assert_eq!(probe.final_combo(), ordinary.final_combo());
    assert_eq!(probe.final_score(), ordinary.final_score());
    assert_eq!(probe.final_counts().hold_breaks, 1);
}

#[test]
fn frame_probe_rejects_invalid_offsets_and_crossing_even_idle_frames() {
    let map = timeline(TAP);
    let rules = LazerRules::native_no_mod(5.0).unwrap();
    let frames = frames(&[(1000, 1), (1000, 1), (1000, 0)]);
    assert_eq!(
        Judge::with_frame_offsets(&map, &frames, &[], rules).unwrap_err(),
        Error::InvalidFrameOffsets
    );
    for offset in [f64::NAN, f64::INFINITY, -0.501, 0.501] {
        assert_eq!(
            Judge::with_frame_offsets(&map, &frames, &[offset, 0.0, 0.0], rules).unwrap_err(),
            Error::InvalidFrameOffset(0)
        );
    }
    assert_eq!(
        Judge::with_frame_offsets(&map, &frames, &[0.25, 0.0, 0.0], rules).unwrap_err(),
        Error::InvalidFrameOffset(1)
    );
    assert!(Judge::with_frame_offsets(&map, &frames, &[-0.5, 0.0, 0.5], rules).is_ok());
}

#[test]
fn frame_probe_keeps_input_and_overlap_validation() {
    let map = timeline(TAP);
    let rules = LazerRules::native_no_mod(5.0).unwrap();
    let mut bad = frames(&[(1000, 1)]);
    bad[0].x = 1.5;
    assert_eq!(
        Judge::with_frame_offsets(&map, &bad, &[0.0], rules).unwrap_err(),
        Error::InvalidFrame(0)
    );
    let backwards = frames(&[(1000, 1), (999, 0)]);
    assert_eq!(
        Judge::with_frame_offsets(&map, &backwards, &[0.0; 2], rules).unwrap_err(),
        Error::InvalidFrame(1)
    );
    let huge = frames(&[(1i64 << 51, 1)]);
    assert_eq!(
        Judge::with_frame_offsets(&map, &huge, &[0.25], rules).unwrap_err(),
        Error::InvalidFrameOffset(0)
    );
    let overlap = timeline("64,192,1000,128,0,2000:0:0:0:0:\n64,192,1500,1,0");
    assert_eq!(
        Judge::with_frame_offsets(&overlap, &frames(&[(1000, 1)]), &[0.0], rules).unwrap_err(),
        Error::OverlappingNotes(1)
    );
}

#[test]
fn a_judged_hold_can_be_regrabbed_before_its_head_time() {
    // Both an early hit and an early miss consume the head, but neither should
    // prevent a subsequent early re-grab from judging the tail on release.
    for (press, head) in [(900, R::Ok), (830, R::Miss)] {
        let j = judge(HOLD, &[(press, 1), (press + 20, 0), (980, 1), (2000, 0)]);
        assert_eq!(j.events()[0].result, head);
        let tail = j.events().iter().find(|e| e.part == Part::Tail).unwrap();
        assert_eq!(tail.time_ms, 2000.0);
        assert_eq!(tail.result, R::Meh);
        assert_eq!(tail.offset_ms, Some(0.0));
        assert_eq!(j.final_counts().hold_breaks, 1);
    }
}

#[test]
fn note_lock_boundary_is_not_rounded_to_integral_milliseconds() {
    // Rounding an input onto the next head can switch which note it hits.
    // Moving the head by 0.25 ms expresses that boundary with integral replay
    // input: the old note is still hittable until the precise next start.
    let before = judge("64,192,1000,1,0\n64,192,1092.25,1,0", &[(1092, 1)]);
    assert_eq!(before.final_counts().get(R::Ok), 1);
    assert_eq!(before.final_counts().get(R::Miss), 1);
    assert_eq!(before.events()[0].object_index, 0);
    let at = judge("64,192,1000,1,0\n64,192,1092,1,0", &[(1092, 1)]);
    assert_eq!(at.final_counts().get(R::Perfect), 1);
    assert_eq!(at.final_counts().get(R::Miss), 1);
    assert_eq!(at.events()[0].object_index, 1);
}

#[test]
fn windows_match_reference_and_include_both_boundaries() {
    for (od, expected) in [
        (0.0, [22.5, 64.5, 97.5, 127.5, 151.5, 188.5]),
        (5.0, [19.5, 49.5, 82.5, 112.5, 136.5, 173.5]),
        (10.0, [13.5, 34.5, 67.5, 97.5, 121.5, 158.5]),
    ] {
        let rules = LazerRules::native_no_mod(od).unwrap();
        for (kind, limit) in [R::Perfect, R::Great, R::Good, R::Ok, R::Meh, R::Miss]
            .into_iter()
            .zip(expected)
        {
            assert_eq!(rules.window(kind), limit);
            assert_eq!(rules.result_for(limit), Some(kind));
            assert_eq!(rules.result_for(-limit), Some(kind));
            assert_ne!(rules.result_for(limit + 0.001), Some(kind));
        }
    }
    for od in [-1.0, 11.0, f64::NAN, f64::INFINITY] {
        assert_eq!(LazerRules::native_no_mod(od), Err(Error::InvalidDifficulty));
    }
}

#[test]
fn too_early_input_does_not_consume_a_note_and_holding_does_not_repeat_hits() {
    let j = judge(TAP, &[(500, 1), (501, 0), (1000, 1), (1100, 1)]);
    assert_eq!(j.final_counts().total(), 1);
    assert_eq!(j.final_counts().get(R::Perfect), 1);
    let held = judge(TAP, &[(500, 1), (1100, 1)]);
    assert_eq!(held.final_counts().get(R::Miss), 1);
}

#[test]
fn automatic_miss_starts_after_successful_window_and_never_leaks_into_the_past() {
    let j = judge(TAP, &[]);
    assert_eq!(j.counts_at(1136.5).total(), 0);
    assert_eq!(j.counts_at(1136.501).get(R::Miss), 1);
    assert_eq!(j.counts_at(900.0).total(), 0);
    assert_eq!(j.counts_at(f64::NAN).total(), 0);
}

#[test]
fn an_early_miss_has_an_input_offset_but_a_late_expiration_does_not() {
    let early = judge(TAP, &[(830, 1)]);
    assert_eq!(early.events()[0].result, R::Miss);
    assert_eq!(early.events()[0].offset_ms, Some(-170.0));
    let late = judge(TAP, &[(1170, 1)]);
    assert_eq!(late.events()[0].offset_ms, None);
}

#[test]
fn chords_are_independent_and_next_note_locks_the_previous_head() {
    let j = judge("64,192,1000,1,0\n192,192,1000,1,0", &[(1000, 3)]);
    assert_eq!(j.final_counts().get(R::Perfect), 2);
    let j = judge("64,192,1000,1,0\n64,192,1100,1,0", &[(1100, 1)]);
    assert_eq!(j.final_counts().get(R::Perfect), 1);
    assert_eq!(j.final_counts().get(R::Miss), 1);
    assert!(j.events().iter().all(|e| e.time_ms == 1100.0));
}

#[test]
fn successful_hold_has_two_results_and_no_body_break() {
    let j = judge(HOLD, &[(1000, 1), (2000, 0)]);
    assert_eq!(j.final_counts().get(R::Perfect), 2);
    assert_eq!(j.final_counts().hold_breaks, 0);
    assert_eq!(j.events()[0].part, Part::Head);
    assert_eq!(j.events()[1].part, Part::Tail);
    assert_eq!(j.counts_at(1500.0).total(), 1);
}

#[test]
fn release_windows_are_wider_than_head_windows() {
    let j = judge(HOLD, &[(1000, 1), (2029, 0)]);
    assert_eq!(j.final_counts().get(R::Perfect), 2);
    let j = judge(HOLD, &[(1000, 1), (2030, 0)]);
    assert_eq!(j.final_counts().get(R::Great), 1);
}

#[test]
fn early_release_is_one_break_and_regrab_caps_the_tail() {
    let j = judge(
        HOLD,
        &[
            (1000, 1),
            (1400, 0),
            (1500, 1),
            (1600, 0),
            (1700, 1),
            (2000, 0),
        ],
    );
    assert_eq!(j.final_counts().get(R::Perfect), 1);
    assert_eq!(j.final_counts().get(R::Meh), 1);
    assert_eq!(j.final_counts().hold_breaks, 1);
    assert_eq!(j.counts_at(1300.0).hold_breaks, 0);
    assert_eq!(j.counts_at(1400.0).hold_breaks, 1);
}

#[test]
fn missing_head_allows_regrab_but_not_full_tail_credit() {
    let j = judge(HOLD, &[(1500, 1), (2000, 0)]);
    assert_eq!(j.final_counts().get(R::Miss), 1);
    assert_eq!(j.final_counts().get(R::Meh), 1);
}

#[test]
fn never_releasing_is_not_an_automatic_perfect_tail() {
    let j = judge(HOLD, &[(1000, 1)]);
    assert_eq!(j.counts_at(2204.75).total(), 1);
    assert_eq!(j.counts_at(2204.751).get(R::Miss), 1);
    assert_eq!(j.final_counts().hold_breaks, 1);
}

#[test]
fn release_and_repress_at_one_time_remain_separate_edges() {
    let j = judge(
        "64,192,1000,1,0\n64,192,1100,1,0",
        &[(1000, 1), (1100, 0), (1100, 1)],
    );
    assert_eq!(j.final_counts().get(R::Perfect), 2);
}

#[test]
fn overlapping_notes_and_mismatched_column_counts_are_explicitly_unsupported() {
    let rules = LazerRules::native_no_mod(5.0).unwrap();
    let map = timeline("64,192,1000,128,0,2000:0:0:0:0:\n64,192,1500,1,0");
    assert!(matches!(
        Judge::new(&map, &input(&[]), rules),
        Err(Error::OverlappingNotes(_))
    ));
    for objects in [
        "64,192,1000,1,0\n64,192,1000,128,0,2000:0:0:0:0:",
        "64,192,1000,128,0,2000:0:0:0:0:\n64,192,1000,128,0,2500:0:0:0:0:",
    ] {
        assert!(matches!(
            Judge::new(&timeline(objects), &input(&[]), rules),
            Err(Error::OverlappingNotes(_))
        ));
    }
    assert!(matches!(
        Judge::new(&timeline(TAP), &InputTrack::new(&[], 7).unwrap(), rules),
        Err(Error::InvalidColumns)
    ));
}

#[test]
fn duplicate_taps_are_distinct_and_can_both_be_hit_early() {
    let objects = "64,192,1000,1,0\n64,192,1000,1,0";
    let early = judge(objects, &[(980, 1), (985, 0), (990, 1)]);
    assert_eq!(early.final_counts().total(), 2);
    assert_eq!(early.final_counts().get(R::Great), 1);
    assert_eq!(early.final_counts().get(R::Perfect), 1);
    assert_eq!(early.events()[0].object_index, 0);
    assert_eq!(early.events()[1].object_index, 1);

    let at = judge(objects, &[(1000, 1), (1000, 0), (1000, 1)]);
    assert_eq!(at.final_counts().get(R::Perfect), 1);
    assert_eq!(at.final_counts().get(R::Miss), 1);
    assert_eq!(at.events()[0].object_index, 1);
    // The earlier duplicate cannot be force-missed at the same start time;
    // it expires on its normal deadline, even if the last duplicate was hit.
    assert_eq!(at.counts_at(1000.0).total(), 1);
    assert_eq!(at.events()[1].object_index, 0);
    assert!(at.events()[1].time_ms > 1136.5);
    assert_eq!(judge(objects, &[]).final_counts().get(R::Miss), 2);
}

#[test]
fn rate_windows_scale_before_flooring_and_keep_inclusive_boundaries() {
    let dt = LazerRules::native_at_rate(5.0, 1.5).unwrap();
    for (kind, expected) in [R::Perfect, R::Great, R::Good, R::Ok, R::Meh, R::Miss]
        .into_iter()
        .zip([29.5, 73.5, 123.5, 168.5, 204.5, 259.5])
    {
        assert_eq!(dt.window(kind), expected);
        assert_eq!(dt.result_for(expected), Some(kind));
        assert_eq!(dt.result_for(-expected), Some(kind));
        assert_ne!(dt.result_for(expected + 0.001), Some(kind));
    }
    assert_eq!(
        LazerRules::native_at_rate(5.0, 1.25)
            .unwrap()
            .window(R::Perfect),
        24.5
    );
    assert_eq!(
        LazerRules::native_at_rate(5.0, 1.0).unwrap(),
        LazerRules::native_no_mod(5.0).unwrap()
    );
    for rate in [0.0, -1.0, 0.49, 2.01, f64::NAN, f64::INFINITY] {
        assert_eq!(
            LazerRules::native_at_rate(5.0, rate),
            Err(Error::InvalidClockRate)
        );
    }
}

#[test]
fn od_nine_preserves_reference_interpolation_before_flooring() {
    let rules = LazerRules::native_no_mod(9.0).unwrap();
    assert_eq!(rules.window(R::Perfect), 15.5);
    assert_eq!(rules.result_for(15.0), Some(R::Perfect));
    assert_eq!(rules.result_for(16.0), Some(R::Great));
}
