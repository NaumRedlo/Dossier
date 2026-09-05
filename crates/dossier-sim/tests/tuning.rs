use std::collections::BTreeMap;

use dossier_beatmap::Beatmap;
use dossier_replay::{LazerMod, Mods, Replay, ReplayFrame, ScoreInfo, Setting};
use dossier_sim::{GameState, Tuning};

fn beatmap(body: &str) -> Beatmap {
    Beatmap::parse(&format!("osu file format v14\n\n{body}")).expect("a map")
}

const OD_EIGHT: &str = "
[Difficulty]
HPDrainRate:5
CircleSize:4
OverallDifficulty:8
ApproachRate:9

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
100,100,1000,1,0
";

fn a_mod(acronym: &str, settings: &[(&str, Setting)]) -> LazerMod {
    LazerMod {
        acronym: acronym.to_owned(),
        settings: settings
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect(),
    }
}

fn replay_with(mods: Vec<LazerMod>) -> Replay {
    Replay {
        mode: dossier_replay::GameMode::Standard,
        game_version: 30_000_018,
        beatmap_hash: String::new(),
        player: "tester".into(),
        replay_hash: String::new(),
        hits: Default::default(),
        score: 0,
        max_combo: 0,
        perfect_combo: false,
        mods: Mods::new(0),
        life_bar: String::new(),
        timestamp_ticks: 0,
        online_score_id: 0,
        target_practice_accuracy: None,
        frames: vec![ReplayFrame {
            time_ms: 0,
            x: 0.0,
            y: 0.0,
            keys: dossier_replay::Keys(0),
        }],
        rng_seed: None,
        score_info: Some(ScoreInfo {
            client_version: Some("2026.417.0-lazer".into()),
            rank: None,
            mods,
            statistics: BTreeMap::new(),
            maximum_statistics: BTreeMap::new(),
            total_score_without_mods: None,
        }),
    }
}

#[test]
fn a_stable_replay_has_nothing_to_say_about_any_of_this() {
    let mut plain = replay_with(Vec::new());
    plain.score_info = None;
    plain.game_version = 20_260_101;
    assert!(Tuning::of_replay(&plain).is_empty());
}

#[test]
fn difficulty_adjust_is_read_off_the_replay() {
    let replay = replay_with(vec![a_mod(
        "DA",
        &[
            ("overall_difficulty", Setting::Number(11.0)),
            ("approach_rate", Setting::Number(10.0)),
            ("extended_limits", Setting::Bool(true)),
        ],
    )]);
    let tuning = Tuning::of_replay(&replay);
    assert_eq!(tuning.overall_difficulty, Some(11.0));
    assert_eq!(tuning.approach_rate, Some(10.0));

    assert_eq!(tuning.circle_size, None);
    assert_eq!(tuning.drain_rate, None);
}

#[test]
fn the_stats_a_map_is_played_at_are_the_ones_the_player_dialled_in() {
    let replay = replay_with(vec![a_mod(
        "DA",
        &[("overall_difficulty", Setting::Number(11.0))],
    )]);
    let state = GameState::new(&beatmap(OD_EIGHT), &replay);
    let difficulty = &state.timeline().difficulty;
    assert!((difficulty.overall_difficulty - 11.0).abs() < 1e-9);

    assert!((difficulty.approach_rate - 9.0).abs() < 1e-9);
    assert!((difficulty.circle_size - 4.0).abs() < 1e-9);
}

#[test]
fn a_window_past_ten_is_kept_rather_than_clamped() {
    let played = |od: f64| {
        let replay = replay_with(vec![a_mod(
            "DA",
            &[("overall_difficulty", Setting::Number(od))],
        )]);
        GameState::new(&beatmap(OD_EIGHT), &replay)
            .timeline()
            .difficulty
            .hit_window_300()
    };
    assert!((played(8.0) - 32.0).abs() < 1e-6, "the map's own");
    assert!((played(10.0) - 20.0).abs() < 1e-6);
    assert!((played(11.0) - 14.0).abs() < 1e-6, "clamped at ten");
}

#[test]
fn a_rate_the_player_dialled_in_is_the_rate_it_is_drawn_at() {
    let replay = replay_with(vec![a_mod(
        "DT",
        &[("speed_change", Setting::Number(1.15))],
    )]);
    let state = GameState::new(&beatmap(OD_EIGHT), &replay);
    assert!((state.playback_rate() - 1.15).abs() < 1e-9);
}

#[test]
fn an_ordinary_double_time_is_still_one_and_a_half() {
    let replay = replay_with(vec![a_mod("DT", &[])]);
    let mut replay = replay;
    replay.mods = Mods::new(dossier_replay::bits::DOUBLE_TIME);
    let state = GameState::new(&beatmap(OD_EIGHT), &replay);
    assert!((state.playback_rate() - 1.5).abs() < 1e-9);
}

#[test]
fn half_time_is_read_from_its_own_setting_too() {
    let replay = replay_with(vec![a_mod("DC", &[("speed_change", Setting::Number(0.9))])]);
    let state = GameState::new(&beatmap(OD_EIGHT), &replay);
    assert!((state.playback_rate() - 0.9).abs() < 1e-9);
}

#[test]
fn previewing_a_map_under_mods_does_not_borrow_a_replays_settings() {
    let replay = replay_with(vec![a_mod(
        "DA",
        &[("overall_difficulty", Setting::Number(11.0))],
    )]);
    let state = GameState::with_mods(&beatmap(OD_EIGHT), &replay, Mods::new(0));
    assert!((state.timeline().difficulty.overall_difficulty - 8.0).abs() < 1e-9);
}
