use dossier_beatmap::Beatmap;
use dossier_replay::{bits, HitCounts, Keys, Mods, Replay, ReplayFrame};
use dossier_sim::score::{difficulty_multiplier, stable_mod_multiplier};
use dossier_sim::{GameState, Ruleset, ScoreTrack};

fn beatmap(body: &str) -> Beatmap {
    Beatmap::parse(&format!("osu file format v14\n\n{body}")).expect("test map should parse")
}

fn replay_with(frames: Vec<ReplayFrame>, mods: u32) -> Replay {
    Replay {
        mode: dossier_replay::GameMode::Standard,
        game_version: 20_260_101,
        beatmap_hash: String::new(),
        player: "tester".into(),
        replay_hash: String::new(),
        hits: HitCounts::default(),
        score: 0,
        max_combo: 0,
        perfect_combo: false,
        mods: Mods::new(mods),
        life_bar: String::new(),
        timestamp_ticks: 0,
        online_score_id: 0,
        target_practice_accuracy: None,
        frames,
        rng_seed: None,
        score_info: None,
    }
}

fn click(time_ms: i64, x: f32, y: f32) -> Vec<ReplayFrame> {
    vec![
        ReplayFrame {
            time_ms: time_ms - 10,
            x,
            y,
            keys: Keys(0),
        },
        ReplayFrame {
            time_ms,
            x,
            y,
            keys: Keys(Keys::K1),
        },
        ReplayFrame {
            time_ms: time_ms + 10,
            x,
            y,
            keys: Keys(0),
        },
    ]
}

#[test]
fn a_multiplier_landing_on_a_half_rounds_to_the_even_side() {
    let m = beatmap(
        "[Difficulty]\nHPDrainRate:5\nCircleSize:4\nOverallDifficulty:9.2\n\n\
         [HitObjects]\n0,0,1000,1,0\n",
    );
    let raw: f64 = (5.0 + 9.2 + 4.0 + 16.0) / 38.0 * 5.0;
    assert!((raw - 4.5).abs() < 1e-12, "the premise of this test: {raw}");
    assert_eq!(difficulty_multiplier(&m, 1000, 100.0), 4);
}

#[test]
fn the_pieces_of_a_slider_score_flat_however_long_the_combo() {
    let map = beatmap(
        "[Difficulty]\nHPDrainRate:5\nCircleSize:5\nOverallDifficulty:5\n\
         SliderMultiplier:1.0\nSliderTickRate:1\n\n\
         [TimingPoints]\n0,500,4,2,0,100,1,0\n\n\
         [HitObjects]\n\
         100,100,1000,1,0\n100,100,2000,1,0\n100,100,3000,1,0\n\
         100,100,4000,2,0,L|200:100,1,100\n",
    );

    let mut frames = Vec::new();
    for t in [1000, 2000, 3000] {
        frames.extend(click(t, 100.0, 100.0));
    }

    for t in (3990..=4600).step_by(10) {
        let progress = ((t - 4000) as f32 / 500.0).clamp(0.0, 1.0);
        frames.push(ReplayFrame {
            time_ms: t,
            x: 100.0 + 100.0 * progress,
            y: 100.0,
            keys: Keys(if (4000..=4500).contains(&t) {
                Keys::K1
            } else {
                0
            }),
        });
    }

    let replay = replay_with(frames, 0);
    let state = GameState::new(&map, &replay);
    let judge = state.judge().expect("the map should be judged");

    assert_eq!(judge.final_state().combo, 5, "{:?}", judge.events());
    assert!(
        !judge.events().iter().any(|e| e.result.is_miss()),
        "{:?}",
        judge.events()
    );

    let m = u64::from(difficulty_multiplier(
        &map,
        map.objects.len(),
        dossier_sim::score::drain_seconds(&map),
    ));
    let track = ScoreTrack::build(judge, &map, Mods::new(0), Ruleset::STABLE);
    assert_eq!(track.total(), 1260 + 60 * m, "multiplier was {m}");
}

#[test]
fn the_first_two_objects_of_a_map_are_worth_their_face_value() {
    let map = beatmap(
        "[Difficulty]\nHPDrainRate:5\nCircleSize:5\nOverallDifficulty:5\n\n\
         [HitObjects]\n100,100,1000,1,0\n100,100,2000,1,0\n100,100,3000,1,0\n",
    );
    let mut frames = Vec::new();
    for t in [1000, 2000, 3000] {
        frames.extend(click(t, 100.0, 100.0));
    }
    let replay = replay_with(frames, 0);
    let state = GameState::new(&map, &replay);
    let judge = state.judge().expect("the map should be judged");
    let track = ScoreTrack::build(judge, &map, Mods::new(0), Ruleset::STABLE);

    assert_eq!(track.at(1000.0), 300, "the first note is worth 300 flat");
    assert_eq!(track.at(2000.0), 600, "so is the second");

    let multiplier = f64::from(difficulty_multiplier(
        &map,
        map.objects.len(),
        dossier_sim::score::drain_seconds(&map),
    ));
    assert_eq!(track.at(3000.0), 900 + (300.0 / 25.0 * multiplier) as u64);
}

#[test]
fn nofail_halves_the_whole_score_and_not_just_the_combo_part() {
    let map = beatmap(
        "[Difficulty]\nHPDrainRate:5\nCircleSize:5\nOverallDifficulty:5\n\n\
         [HitObjects]\n100,100,1000,1,0\n100,100,2000,1,0\n100,100,3000,1,0\n",
    );
    let mut frames = Vec::new();
    for t in [1000, 2000, 3000] {
        frames.extend(click(t, 100.0, 100.0));
    }
    let plain = GameState::new(&map, &replay_with(frames.clone(), 0));
    let nofail = GameState::new(&map, &replay_with(frames, bits::NO_FAIL));

    let a = ScoreTrack::build(
        plain.judge().expect("judged"),
        &map,
        Mods::new(0),
        Ruleset::STABLE,
    );
    let b = ScoreTrack::build(
        nofail.judge().expect("judged"),
        &map,
        Mods::new(bits::NO_FAIL),
        Ruleset::STABLE,
    );

    assert_eq!(stable_mod_multiplier(Mods::new(bits::NO_FAIL)), 0.5);
    assert!(b.total() < a.total(), "{} against {}", b.total(), a.total());
    assert!(b.total() >= 900, "the face value is not scaled");
}

#[test]
fn lazers_score_is_capped_near_a_million_where_stables_is_not() {
    let map = beatmap(
        "[Difficulty]\nHPDrainRate:5\nCircleSize:5\nOverallDifficulty:5\n\n\
         [HitObjects]\n100,100,1000,1,0\n100,100,2000,1,0\n100,100,3000,1,0\n",
    );
    let mut frames = Vec::new();
    for t in [1000, 2000, 3000] {
        frames.extend(click(t, 100.0, 100.0));
    }
    let state = GameState::new(&map, &replay_with(frames, 0));
    let judge = state.judge().expect("judged");

    let stable = ScoreTrack::build(judge, &map, Mods::new(0), Ruleset::STABLE);
    let lazer = ScoreTrack::build(judge, &map, Mods::new(0), Ruleset::LAZER);

    assert!(stable.total() < 2_000, "{}", stable.total());

    assert_eq!(lazer.total(), 1_000_000);
}

#[test]
fn lazer_never_exceeds_a_million_on_a_clean_play() {
    let map = beatmap(
        "[Difficulty]\nHPDrainRate:5\nCircleSize:5\nOverallDifficulty:5\nSliderMultiplier:1.0\n\n\
         [HitObjects]\n100,100,1000,2,0,L|200:100,1,100\n300,200,3000,1,0\n",
    );
    let mut frames: Vec<ReplayFrame> = (990..=1600)
        .step_by(10)
        .map(|t| {
            let progress = ((t - 1000) as f32 / 100.0).clamp(0.0, 1.0);
            ReplayFrame {
                time_ms: t,
                x: 100.0 + 100.0 * progress,
                y: 100.0,
                keys: Keys(if t >= 1000 { Keys::K1 } else { 0 }),
            }
        })
        .collect();
    frames.extend(click(3000, 300.0, 200.0));

    let state = GameState::new(&map, &replay_with(frames, 0));
    let judge = state.judge().expect("judged");
    let track = ScoreTrack::build(judge, &map, Mods::new(0), Ruleset::LAZER);
    assert!(track.total() <= 1_000_000, "{}", track.total());
}

fn missed_middle() -> (String, Vec<ReplayFrame>) {
    let body = String::from(
        "[Difficulty]\nHPDrainRate:5\nCircleSize:5\nOverallDifficulty:5\n\n\
         [HitObjects]\n100,100,1000,1,0\n100,100,2000,1,0\n\
         300,200,3000,1,0\n100,100,4000,1,0\n",
    );
    let mut frames = Vec::new();
    for t in [1000, 2000, 4000] {
        frames.extend(click(t, 100.0, 100.0));
    }
    (body, frames)
}

#[test]
fn stables_score_never_goes_backwards() {
    let (body, frames) = missed_middle();
    let map = beatmap(&body);
    let replay = replay_with(frames, 0);
    let state = GameState::new(&map, &replay);
    let judge = state.judge().expect("judged");
    let track = ScoreTrack::build(judge, &map, Mods::new(0), Ruleset::STABLE);

    let mut last = 0;
    for t in (0..5000).step_by(50) {
        let now = track.at(f64::from(t));
        assert!(now >= last, "stable went backwards at {t}ms");
        last = now;
    }

    assert_eq!(track.total(), 900, "three hits at face value");

    let mut clean = Vec::new();
    for t in [1000, 2000, 4000] {
        clean.extend(click(t, 100.0, 100.0));
    }
    clean.extend(click(3000, 300.0, 200.0));
    clean.sort_by_key(|f| f.time_ms);
    let replay = replay_with(clean, 0);
    let state = GameState::new(&map, &replay);
    let track = ScoreTrack::build(
        state.judge().expect("judged"),
        &map,
        Mods::new(0),
        Ruleset::STABLE,
    );
    assert!(
        track.total() > 1200,
        "the clean play should carry a combo bonus: {}",
        track.total()
    );
}

#[test]
fn lazers_score_falls_when_a_note_is_missed() {
    let (body, frames) = missed_middle();
    let map = beatmap(&body);
    let replay = replay_with(frames, 0);
    let state = GameState::new(&map, &replay);
    let judge = state.judge().expect("judged");
    let track = ScoreTrack::build(judge, &map, Mods::new(0), Ruleset::LAZER);

    let before = track.at(2500.0);
    let after = track.at(3500.0);
    assert!(
        after < before,
        "the miss cost nothing: {before} then {after}"
    );
}

#[test]
fn lazers_combo_half_is_weighted_by_what_the_note_was_worth_at_best() {
    let mut body = String::from(
        "[Difficulty]\nHPDrainRate:5\nCircleSize:5\nOverallDifficulty:5\n\n[HitObjects]\n",
    );
    let mut frames = Vec::new();
    for n in 0..12i64 {
        let (x, y) = (80.0 + (n % 4) as f32 * 90.0, 80.0 + (n / 4) as f32 * 90.0);
        let t = 1000 + n * 400;
        body.push_str(&format!("{x},{y},{t},1,0\n"));
        frames.extend(click(t + 60, x, y));
    }

    let map = beatmap(&body);
    let mut replay = replay_with(frames, 0);
    replay.game_version = 30_000_016;
    let state = GameState::new(&map, &replay);
    let judge = state.judge().expect("judged");

    assert_eq!(
        judge.final_state().counts.count_100,
        12,
        "{:?}",
        judge.events()
    );
    assert_eq!(judge.final_state().combo, 12);

    let track = ScoreTrack::build(judge, &map, Mods::new(0), Ruleset::LAZER);
    let expected = (500_000.0 / 3.0 + 500_000.0 * (1.0f64 / 3.0).powi(5)).round() as u64;
    assert_eq!(track.total(), expected);
}
