use dossier_beatmap::Beatmap;
use dossier_replay::{
    bits, GameMode, HitCounts, Keys, LazerMod, Mods, Replay, ReplayFrame, ScoreInfo, Setting,
};
use dossier_sim::mania::{
    judge::{LazerRules, ResultKind},
    Error, InputTrack, Playback, Timeline,
};

fn map(columns: u8, objects: &str) -> Beatmap {
    Beatmap::parse(&format!("osu file format v14\n[General]\nMode:3\n[Difficulty]\nCircleSize:{columns}\n[HitObjects]\n{objects}")).unwrap()
}
fn frame(at: i64, down: f32) -> ReplayFrame {
    ReplayFrame {
        time_ms: at,
        x: down,
        y: 0.0,
        keys: Keys(31),
    }
}

#[test]
fn stable_skip_markers_and_idle_preroll_are_not_gameplay_keys() {
    let mut r = replay();
    r.game_version = 20251022;
    let marker = |time_ms| ReplayFrame {
        time_ms,
        x: 256.0,
        y: -500.0,
        keys: Keys(0),
    };
    r.frames = vec![
        marker(0),
        marker(2000),
        frame(8, 0.0),
        frame(43, 0.0),
        frame(31, 0.0),
        frame(42, 0.0),
        frame(45, 2.0),
        frame(60, 0.0),
    ];
    let track = InputTrack::from_replay(&r, 4).unwrap();
    assert_eq!(track.events().len(), 2);
    assert_eq!(track.events()[0].time_ms, 45);
    assert_eq!(track.events()[0].column, 1);
    assert_eq!(track.keys_at(44.0), 0);
    assert_eq!(track.keys_at(45.0), 2);
    assert_eq!(r.frames.len(), 8); // Normalization must not rewrite replay evidence.
    assert!(matches!(
        InputTrack::new(&r.frames, 4),
        Err(Error::InvalidFrame(0))
    ));
    r.game_version = 30000019;
    assert!(matches!(
        InputTrack::from_replay(&r, 4),
        Err(Error::InvalidFrame(0))
    ));
}

#[test]
fn stable_normalization_keeps_real_ninth_key_and_rejects_gameplay_rollbacks() {
    let mut r = replay();
    r.game_version = 20251022;
    r.frames = vec![frame(100, 256.0), frame(200, 0.0)];
    let track = InputTrack::from_replay(&r, 9).unwrap();
    assert_eq!(track.keys_at(100.0), 256);
    assert_eq!(track.events()[0].column, 8);
    r.frames = vec![frame(100, 0.0), frame(99, 1.0)];
    assert!(matches!(
        InputTrack::from_replay(&r, 4),
        Err(Error::InvalidFrame(1))
    ));
    r.frames = vec![frame(100, 1.0), frame(110, 0.0), frame(109, 0.0)];
    assert!(matches!(
        InputTrack::from_replay(&r, 4),
        Err(Error::InvalidFrame(2))
    ));
    r.frames = vec![
        ReplayFrame {
            time_ms: 0,
            x: 256.0,
            y: -500.0,
            keys: Keys(0),
        },
        frame(100, 0.0),
        frame(101, 16.0),
    ];
    assert!(matches!(
        InputTrack::from_replay(&r, 4),
        Err(Error::InvalidFrame(2))
    ));
}

#[test]
fn stable_rules_reject_lazer_dt_and_ac_playback() {
    let m = map(4, "64,192,1000,1,0");
    let mut r = replay();
    let rules = dossier_sim::mania::stable::StableRules::native_no_mod(5.0).unwrap();
    for modifier in ["DT", "AC"] {
        r.score_info = Some(ScoreInfo {
            mods: vec![LazerMod::plain(modifier)],
            ..Default::default()
        });
        assert!(matches!(
            Playback::new(&m, &r).unwrap().judge_stable_with(rules),
            Err(Error::UnsupportedMods)
        ));
        assert!(matches!(
            Playback::new(&m, &r).unwrap().trace_stable_with(rules),
            Err(Error::UnsupportedMods)
        ));
    }
}

#[test]
fn stable_modifier_selection_preserves_columns_and_timestamps() {
    let m = map(4, "64,192,1000,1,0\n448,192,2000,128,0,3000:0:0:0:0:");
    let mut r = replay();
    r.game_version = 20251022;
    r.frames = vec![
        frame(1017, 1.0),
        frame(1020, 0.0),
        frame(2017, 8.0),
        frame(3017, 0.0),
    ];
    let baseline = Playback::new(&m, &r).unwrap();
    for (mods, rate, hr, tap, hold) in [
        (0, 1.0, false, ResultKind::Great, ResultKind::Perfect),
        (
            bits::HARD_ROCK,
            1.0,
            true,
            ResultKind::Great,
            ResultKind::Great,
        ),
        (
            bits::DOUBLE_TIME,
            1.5,
            false,
            ResultKind::Perfect,
            ResultKind::Perfect,
        ),
        (
            bits::DOUBLE_TIME | bits::HARD_ROCK,
            1.5,
            true,
            ResultKind::Perfect,
            ResultKind::Perfect,
        ),
    ] {
        r.mods = Mods::new(mods);
        let playback = Playback::new(&m, &r).unwrap();
        let rules = playback.stable_rules(8.0).unwrap();
        assert_eq!(rules.clock_rate(), rate);
        assert_eq!(rules.hard_rock(), hr);
        assert_eq!(playback.input.events(), baseline.input.events());
        assert_eq!(
            playback.at(2500.0, 1000.0).unwrap(),
            baseline.at(2500.0, 1000.0).unwrap()
        );
        let judge = playback.judge_stable_with(rules).unwrap();
        assert_eq!(judge.events()[0].result, tap);
        assert_eq!(judge.events()[1].result, hold);
        assert_eq!(judge.events()[1].time_ms, 3017.0);
        assert_eq!(
            judge.events(),
            playback.trace_stable_with(rules).unwrap().0.events()
        );
        let wrong_rate = dossier_sim::mania::stable::StableRules::native_with_mods(
            8.0,
            if rate == 1.0 { 1.5 } else { 1.0 },
            hr,
        )
        .unwrap();
        assert!(matches!(
            playback.judge_stable_with(wrong_rate),
            Err(Error::MismatchedClockRate)
        ));
        assert!(matches!(
            playback.trace_stable_with(wrong_rate),
            Err(Error::MismatchedClockRate)
        ));
        let wrong_hr =
            dossier_sim::mania::stable::StableRules::native_with_mods(8.0, rate, !hr).unwrap();
        assert!(matches!(
            playback.judge_stable_with(wrong_hr),
            Err(Error::UnsupportedMods)
        ));
        assert!(matches!(
            playback.trace_stable_with(wrong_hr),
            Err(Error::UnsupportedMods)
        ));
        if hr {
            assert!(matches!(
                playback.lazer_rules(8.0),
                Err(Error::UnsupportedMods)
            ));
            assert!(matches!(
                playback.judge_with(LazerRules::native_at_rate(8.0, rate).unwrap()),
                Err(Error::UnsupportedMods)
            ));
        }
    }
}

#[test]
fn stable_mod_allowlist_does_not_accept_other_bits_or_lazer_settings() {
    let m = map(4, "64,192,1000,1,0");
    let mut r = replay();
    r.game_version = 20251022;
    for i in 0..32 {
        let bit = 1u32 << i;
        if bit & (bits::DOUBLE_TIME | bits::HARD_ROCK) != 0 {
            continue;
        }
        r.mods = Mods::new(bit | bits::DOUBLE_TIME | bits::HARD_ROCK);
        assert!(
            matches!(Playback::new(&m, &r), Err(Error::UnsupportedMods)),
            "bit {i}"
        );
    }
    r.mods = Mods::new(bits::DOUBLE_TIME);
    r.score_info = Some(ScoreInfo {
        mods: vec![LazerMod::plain("DT")],
        ..Default::default()
    });
    assert!(matches!(Playback::new(&m, &r), Err(Error::UnsupportedMods)));
    assert!(matches!(
        Playback::new(&m, &replay()).unwrap().stable_rules(8.0),
        Err(Error::UnsupportedMods)
    ));
}

#[test]
fn native_columns_chords_and_right_edge_are_preserved() {
    for columns in [4, 7, 9, 18] {
        let m = map(columns, "0,192,1000,1,0\n512,192,1000,1,0");
        let timeline = Timeline::new(&m).unwrap();
        assert_eq!(timeline.columns(), columns);
        assert_eq!(timeline.notes(0).unwrap()[0].column, 0);
        assert_eq!(timeline.notes(columns - 1).unwrap()[0].column, columns - 1);
        assert_eq!(timeline.visible_at(1000.0, 800.0).unwrap().len(), 2);
        assert!(timeline.visible_at(1001.0, 800.0).unwrap().is_empty());
    }
}

#[test]
fn a_long_hold_survives_seeking_past_shorter_notes() {
    let m = map(
        4,
        "64,192,1000,128,0,10000:2:3:4:75:\n64,192,2000,1,0\n64,192,500,1,0",
    );
    assert_eq!(m.objects[1].end_time_ms(), 10000.0);
    let sample = m.objects[1].hit_sample;
    assert_eq!(
        (
            sample.normal_set,
            sample.addition_set,
            sample.index,
            sample.volume
        ),
        (2, 3, 4, 75)
    );
    let timeline = Timeline::new(&m).unwrap();
    let shown = timeline.visible_at(5000.0, 1000.0).unwrap();
    assert_eq!(shown.len(), 1);
    assert!(shown[0].note.hold);
    assert_eq!((shown[0].head, shown[0].tail), (-4.0, 5.0));
    assert!(timeline.visible_at(10001.0, 1000.0).unwrap().is_empty());
    assert_eq!(timeline.visible_at(5000.0, 1000.0).unwrap(), shown);
}

#[test]
fn column_input_uses_x_and_preserves_release_and_repress_at_the_same_time() {
    let input = InputTrack::new(
        &[
            frame(100, 257.0),
            frame(110, 257.0),
            frame(200, 0.0),
            frame(200, 256.0),
        ],
        9,
    )
    .unwrap();
    assert_eq!(input.events().len(), 5);
    assert_eq!(input.events()[1].column, 8);
    assert!(!input.events()[3].pressed);
    assert!(input.events()[4].pressed);
    assert_eq!(input.keys_at(99.0), 0);
    assert_eq!(input.keys_at(200.0), 256);
    assert_eq!(input.keys_at(150.0), 257);
    assert_eq!(input.keys_at(-100.0), 0);
}

#[test]
fn invalid_input_is_rejected_instead_of_producing_a_false_simulation() {
    for x in [-1.0, 1.5, 16.0, f32::NAN, f32::INFINITY] {
        assert!(matches!(
            InputTrack::new(&[frame(0, x)], 4),
            Err(Error::InvalidFrame(0))
        ));
    }
    assert!(InputTrack::new(&[frame(2, 0.0), frame(1, 1.0)], 4).is_err());
    let mut m = map(4, "64,192,1000,1,0");
    m.mode = 0;
    assert!(matches!(Timeline::new(&m), Err(Error::WrongMode)));
    m.mode = 3;
    m.difficulty.circle_size = 4.5;
    assert!(matches!(Timeline::new(&m), Err(Error::InvalidColumns)));
    let timeline = Timeline::new(&map(4, "")).unwrap();
    assert_eq!(timeline.visible_at(0.0, 0.0), Err(Error::InvalidView));
    assert_eq!(
        timeline.visible_at(f64::NAN, 1000.0),
        Err(Error::InvalidView)
    );
}

#[test]
fn malformed_holds_do_not_turn_into_taps() {
    for tail in ["900:0:0:0:0:", "NaN:0:0:0:0:", "bad", ""] {
        assert!(Beatmap::parse(&format!(
            "osu file format v14\n[General]\nMode:3\n[HitObjects]\n64,192,1000,128,0,{tail}"
        ))
        .is_err());
    }
}

#[test]
fn playback_combines_geometry_with_input_and_refuses_unimplemented_mods() {
    let m = map(4, "64,192,1000,128,0,2000:0:0:0:0:");
    let mut replay = replay();
    replay.game_version = 20260101;
    let playback = Playback::new(&m, &replay).unwrap();
    let snapshot = playback.at(1500.0, 1000.0).unwrap();
    assert_eq!(snapshot.keys, 1);
    assert_eq!(snapshot.notes.len(), 1);
    assert_eq!(
        (snapshot.notes[0].head, snapshot.notes[0].tail),
        (-0.5, 0.5)
    );
    assert_eq!(playback.at(2001.0, 1000.0).unwrap().keys, 0);
    assert_eq!(playback.at(1500.0, 1000.0).unwrap(), snapshot);
    replay.mods = Mods::new(bits::HALF_TIME);
    assert!(matches!(
        Playback::new(&m, &replay),
        Err(Error::UnsupportedMods)
    ));
    replay.mods = Mods::new(0);
    replay.mode = GameMode::Standard;
    assert!(matches!(Playback::new(&m, &replay), Err(Error::WrongMode)));
}

fn replay() -> Replay {
    Replay {
        mode: GameMode::Mania,
        game_version: 30_000_019,
        beatmap_hash: String::new(),
        player: "test".into(),
        replay_hash: String::new(),
        hits: HitCounts::default(),
        score: 0,
        max_combo: 0,
        perfect_combo: false,
        mods: Mods::new(0),
        life_bar: String::new(),
        timestamp_ticks: 0,
        online_score_id: 0,
        target_practice_accuracy: None,
        frames: vec![frame(1000, 1.0), frame(2000, 0.0)],
        rng_seed: None,
        score_info: None,
    }
}

#[test]
fn dt_scales_windows_once_without_scaling_geometry_or_inputs() {
    let m = map(4, "64,192,1000,128,0,2000:0:0:0:0:");
    let mut replay = replay();
    replay.frames = vec![frame(1025, 1.0), frame(2025, 0.0)];
    replay.mods = Mods::new(bits::DOUBLE_TIME);
    replay.score_info = Some(ScoreInfo {
        mods: vec![LazerMod::plain("DT")],
        ..Default::default()
    });
    let playback = Playback::new(&m, &replay).unwrap();
    assert_eq!(playback.clock_rate(), 1.5);
    assert_eq!(playback.input.events()[0].time_ms, 1025);
    let snapshot = playback.at(1500.0, 1000.0).unwrap();
    assert_eq!(
        (snapshot.notes[0].head, snapshot.notes[0].tail),
        (-0.5, 0.5)
    );
    let j = playback
        .judge_with(playback.lazer_rules(5.0).unwrap())
        .unwrap();
    assert_eq!(j.final_counts().get(ResultKind::Perfect), 2);
    assert_eq!(j.events()[0].time_ms, 1025.0);
    assert!(matches!(
        playback.judge_with(LazerRules::native_no_mod(5.0).unwrap()),
        Err(Error::MismatchedClockRate)
    ));

    replay.score_info.as_mut().unwrap().mods[0]
        .settings
        .insert("speed_change".into(), Setting::Number(1.25));
    let custom = Playback::new(&m, &replay).unwrap();
    assert_eq!(custom.clock_rate(), 1.25);
    assert_eq!(
        custom
            .judge_with(custom.lazer_rules(5.0).unwrap())
            .unwrap()
            .final_counts()
            .get(ResultKind::Great),
        1
    );
}

#[test]
fn accuracy_challenge_preserves_note_grades_and_exposes_missing_failure_policy() {
    let m = map(4, "64,192,1000,128,0,2000:0:0:0:0:");
    let mut replay = replay();
    let plain = Playback::new(&m, &replay).unwrap();
    let mut ac = LazerMod::plain("AC");
    ac.settings
        .insert("minimum_accuracy".into(), Setting::Number(0.99));
    ac.settings.insert("restart".into(), Setting::Bool(true));
    replay.score_info = Some(ScoreInfo {
        mods: vec![ac],
        ..Default::default()
    });
    let playback = Playback::new(&m, &replay).unwrap();
    assert!(playback.has_accuracy_challenge());
    assert!(!plain.has_accuracy_challenge());
    assert_eq!(
        playback
            .judge_with(playback.lazer_rules(5.0).unwrap())
            .unwrap()
            .events(),
        plain
            .judge_with(plain.lazer_rules(5.0).unwrap())
            .unwrap()
            .events()
    );
}

#[test]
fn unsupported_or_malformed_mods_never_silently_become_nomod() {
    let m = map(4, "64,192,1000,1,0");
    let mut replay = replay();
    for bit in [
        bits::HARD_ROCK,
        bits::HALF_TIME,
        bits::MIRROR,
        bits::DOUBLE_TIME | bits::HIDDEN,
    ] {
        replay.mods = Mods::new(bit);
        assert!(matches!(
            Playback::new(&m, &replay),
            Err(Error::UnsupportedMods)
        ));
    }
    replay.mods = Mods::new(0);
    for mods in [
        vec![LazerMod::plain("CL")],
        vec![LazerMod::plain("DT"), LazerMod::plain("DT")],
        vec![LazerMod::plain("AC"), LazerMod::plain("AC")],
    ] {
        replay.score_info = Some(ScoreInfo {
            mods,
            ..Default::default()
        });
        assert!(matches!(
            Playback::new(&m, &replay),
            Err(Error::UnsupportedMods)
        ));
    }
    for value in [
        Setting::Number(0.0),
        Setting::Number(2.1),
        Setting::Number(f64::NAN),
        Setting::Text("1.5".into()),
    ] {
        let mut dt = LazerMod::plain("DT");
        dt.settings.insert("speed_change".into(), value);
        replay.score_info = Some(ScoreInfo {
            mods: vec![dt],
            ..Default::default()
        });
        assert!(matches!(
            Playback::new(&m, &replay),
            Err(Error::UnsupportedMods)
        ));
    }
    replay.mods = Mods::new(bits::DOUBLE_TIME);
    replay.score_info = Some(ScoreInfo {
        mods: vec![LazerMod::plain("AC")],
        ..Default::default()
    });
    assert!(matches!(
        Playback::new(&m, &replay),
        Err(Error::UnsupportedMods)
    ));
}
