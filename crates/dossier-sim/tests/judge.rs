use std::f64::consts::TAU;

use dossier_beatmap::Beatmap;
use dossier_replay::{HitCounts, Keys, Mods, Replay, ReplayFrame};
use dossier_sim::{GameState, Judgement, Part, Verdict};

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

fn frame(time_ms: i64, x: f32, y: f32, keys: u8) -> ReplayFrame {
    ReplayFrame {
        time_ms,
        x,
        y,
        keys: Keys(keys),
    }
}

fn click(time_ms: i64, x: f32, y: f32) -> Vec<ReplayFrame> {
    vec![
        frame(time_ms - 10, x, y, 0),
        frame(time_ms, x, y, Keys::K1),
        frame(time_ms + 10, x, y, 0),
    ]
}

fn frames_over(
    from: i64,
    to: i64,
    pos: impl Fn(i64) -> (f32, f32),
    held: impl Fn(i64) -> bool,
) -> Vec<ReplayFrame> {
    (from..=to)
        .step_by(10)
        .map(|t| {
            let (x, y) = pos(t);
            frame(t, x, y, if held(t) { Keys::K1 } else { 0 })
        })
        .collect()
}

fn judged(map: &Beatmap, replay: &Replay) -> HitCounts {
    GameState::new(map, replay)
        .judge()
        .unwrap()
        .final_state()
        .counts
}

const ONE_CIRCLE: &str = "
[Difficulty]
CircleSize:5
OverallDifficulty:5

[HitObjects]
100,100,1000,1,0
";

#[test]
fn a_click_on_time_is_a_three_hundred() {
    let map = beatmap(ONE_CIRCLE);
    let state = GameState::new(&map, &replay_with(click(1000, 100.0, 100.0), 0));
    let score = state.judge().unwrap().final_state();

    assert_eq!(score.counts.count_300, 1);
    assert_eq!(score.combo, 1);
    assert!((score.accuracy() - 100.0).abs() < 1e-9);
}

#[test]
fn the_windows_step_down_as_the_click_drifts() {
    let map = beatmap(ONE_CIRCLE);
    let at = |t: i64| {
        let state = GameState::new(&map, &replay_with(click(t, 100.0, 100.0), 0));
        state.judge().unwrap().events()[0].result
    };

    assert_eq!(at(1049), Judgement::Great, "one under the 300 window");
    assert_eq!(at(1050), Judgement::Ok, "exactly on it is not inside it");
    assert_eq!(at(1099), Judgement::Ok);
    assert_eq!(at(1100), Judgement::Meh, "same rule at the next edge");
    assert_eq!(at(1149), Judgement::Meh, "the last hittable millisecond");
}

#[test]
fn a_click_exactly_on_the_fifty_window_does_not_land_at_all() {
    let map = beatmap(ONE_CIRCLE);
    assert_eq!(
        judged(&map, &replay_with(click(1149, 100.0, 100.0), 0)).count_50,
        1
    );
    assert_eq!(
        judged(&map, &replay_with(click(1150, 100.0, 100.0), 0)).count_miss,
        1
    );
}

#[test]
fn the_error_is_signed_so_early_and_late_are_distinguishable() {
    let map = beatmap(ONE_CIRCLE);
    let early = GameState::new(&map, &replay_with(click(970, 100.0, 100.0), 0));
    let late = GameState::new(&map, &replay_with(click(1030, 100.0, 100.0), 0));

    assert_eq!(early.judge().unwrap().events()[0].error_ms, Some(-30.0));
    assert_eq!(late.judge().unwrap().events()[0].error_ms, Some(30.0));
}

#[test]
fn a_click_past_the_window_never_lands() {
    let map = beatmap(ONE_CIRCLE);
    let counts = judged(&map, &replay_with(click(1151, 100.0, 100.0), 0));
    assert_eq!(counts.count_miss, 1);
}

#[test]
fn an_early_click_on_the_note_takes_it_with_it() {
    let map = beatmap(ONE_CIRCLE);
    let mut frames = click(800, 100.0, 100.0);
    frames.extend(click(1000, 100.0, 100.0));
    let counts = judged(&map, &replay_with(frames, 0));

    assert_eq!(counts.count_miss, 1, "the early click took it");
    assert_eq!(counts.count_300, 0, "so the click on time found nothing");
}

#[test]
fn an_early_click_that_misses_the_circle_takes_nothing() {
    let map = beatmap(ONE_CIRCLE);
    let mut frames = click(800, 200.0, 100.0);
    frames.extend(click(1000, 100.0, 100.0));
    assert_eq!(judged(&map, &replay_with(frames, 0)).count_300, 1);
}

#[test]
fn a_click_off_the_circle_does_not_count() {
    let map = beatmap(ONE_CIRCLE);

    let counts = judged(&map, &replay_with(click(1000, 140.0, 100.0), 0));
    assert_eq!(counts.count_miss, 1);
    assert_eq!(counts.count_300, 0);
}

#[test]
fn an_unclicked_circle_misses() {
    let map = beatmap(ONE_CIRCLE);
    let frames = frames_over(0, 3000, |_| (100.0, 100.0), |_| false);
    let counts = judged(&map, &replay_with(frames, 0));
    assert_eq!(counts.count_miss, 1);
}

#[test]
fn a_miss_is_recorded_when_its_window_shuts_not_when_it_was_due() {
    let map = beatmap(ONE_CIRCLE);
    let state = GameState::new(&map, &replay_with(Vec::new(), 0));
    let judge = state.judge().unwrap();
    assert_eq!(judge.events()[0].time_ms, 1150.0);

    assert_eq!(judge.state_at(1100.0).counts.count_miss, 0);
    assert_eq!(judge.state_at(1150.0).counts.count_miss, 1);
}

#[test]
fn a_click_cannot_reach_past_an_object_that_is_still_live() {
    let map = beatmap(
        "
[Difficulty]
CircleSize:5
OverallDifficulty:5

[HitObjects]
100,100,1000,1,0
300,300,1050,1,0
",
    );
    let counts = judged(&map, &replay_with(click(1050, 300.0, 300.0), 0));
    assert_eq!(counts.count_miss, 2, "notelock swallowed the click");
    assert_eq!(counts.count_300, 0);
}

#[test]
fn holding_the_button_hits_one_object_not_every_object_under_it() {
    let map = beatmap(
        "
[Difficulty]
CircleSize:5
OverallDifficulty:5

[HitObjects]
100,100,1000,1,0
100,100,1200,1,0
",
    );
    let frames = frames_over(900, 1400, |_| (100.0, 100.0), |t| t >= 1000);
    let counts = judged(&map, &replay_with(frames, 0));
    assert_eq!(counts.count_300, 1, "only the rising edge is a click");
    assert_eq!(counts.count_miss, 1);
}

#[test]
fn a_keyboard_press_setting_two_bits_is_still_one_click() {
    let map = beatmap(
        "
[Difficulty]
CircleSize:5
OverallDifficulty:5

[HitObjects]
100,100,1000,1,0
100,100,1005,1,0
",
    );
    let frames = vec![
        frame(990, 100.0, 100.0, 0),
        frame(1000, 100.0, 100.0, Keys::K1 | Keys::M1),
        frame(1010, 100.0, 100.0, 0),
    ];
    let counts = judged(&map, &replay_with(frames, 0));
    assert_eq!(counts.count_300, 1);
    assert_eq!(counts.count_miss, 1);
}

#[test]
fn releasing_and_pressing_again_is_two_clicks() {
    let map = beatmap(
        "
[Difficulty]
CircleSize:5
OverallDifficulty:5

[HitObjects]
100,100,1000,1,0
100,100,1200,1,0
",
    );
    let mut frames = click(1000, 100.0, 100.0);
    frames.extend(click(1200, 100.0, 100.0));
    assert_eq!(judged(&map, &replay_with(frames, 0)).count_300, 2);
}

#[test]
fn a_miss_resets_the_combo_but_not_the_peak() {
    let map = beatmap(
        "
[Difficulty]
CircleSize:5
OverallDifficulty:5

[HitObjects]
100,100,1000,1,0
100,100,1400,1,0
100,100,1800,1,0
100,100,2200,1,0
",
    );
    let mut frames = click(1000, 100.0, 100.0);
    frames.extend(click(1400, 100.0, 100.0));

    frames.extend(click(2200, 100.0, 100.0));

    let state = GameState::new(&map, &replay_with(frames, 0));
    let score = state.judge().unwrap().final_state();
    assert_eq!(score.max_combo, 2);
    assert_eq!(score.combo, 1);
    assert_eq!(score.counts.count_miss, 1);
}

#[test]
fn accuracy_weighs_the_judgements() {
    let map = beatmap(
        "
[Difficulty]
CircleSize:5
OverallDifficulty:5

[HitObjects]
100,100,1000,1,0
100,100,1400,1,0
",
    );
    let mut frames = click(1000, 100.0, 100.0);
    frames.extend(click(1480, 100.0, 100.0));
    let score = GameState::new(&map, &replay_with(frames, 0))
        .judge()
        .unwrap()
        .final_state();

    assert_eq!(score.counts.count_300, 1);
    assert_eq!(score.counts.count_100, 1);

    assert!((score.accuracy() - 400.0 / 600.0 * 100.0).abs() < 1e-9);
}

#[test]
fn the_score_can_be_read_at_any_instant() {
    let map = beatmap(
        "
[Difficulty]
CircleSize:5
OverallDifficulty:5

[HitObjects]
100,100,1000,1,0
100,100,1400,1,0
",
    );
    let mut frames = click(1000, 100.0, 100.0);
    frames.extend(click(1400, 100.0, 100.0));
    let state = GameState::new(&map, &replay_with(frames, 0));

    assert_eq!(state.update(500.0).score.unwrap().combo, 0);
    assert_eq!(state.update(1000.0).score.unwrap().combo, 1);
    assert_eq!(state.update(1399.0).score.unwrap().combo, 1);
    assert_eq!(state.update(1400.0).score.unwrap().combo, 2);
}

#[test]
fn a_map_with_no_replay_reports_no_score_rather_than_a_wall_of_misses() {
    let map = beatmap(ONE_CIRCLE);
    let state = GameState::from_beatmap(&map, Mods::default());
    assert!(state.judge().is_none());
    assert!(state.update(1000.0).score.is_none());
}

const SHORT_SLIDER: &str = "
[Difficulty]
CircleSize:5
OverallDifficulty:5
SliderMultiplier:1.4
SliderTickRate:1

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
0,0,1000,2,0,L|140:0,1,140
";

fn ball_x(t: i64, start: f64, duration: f64) -> f32 {
    let progress = ((t as f64 - start) / duration).clamp(0.0, 1.0);
    (140.0 * progress) as f32
}

#[test]
fn a_fully_tracked_slider_is_a_three_hundred() {
    let map = beatmap(SHORT_SLIDER);
    let frames = frames_over(
        900,
        1600,
        |t| (ball_x(t, 1000.0, 500.0), 0.0),
        |t| t >= 1000,
    );
    let state = GameState::new(&map, &replay_with(frames, 0));
    let score = state.judge().unwrap().final_state();

    assert_eq!(score.counts.count_300, 1);

    assert_eq!(score.max_combo, 2);
}

#[test]
fn dropping_the_tail_costs_the_slider_a_hundred() {
    let map = beatmap(SHORT_SLIDER);
    let frames = frames_over(
        900,
        1600,
        |t| (ball_x(t, 1000.0, 500.0), 0.0),
        |t| (1000..1200).contains(&t),
    );
    let state = GameState::new(&map, &replay_with(frames, 0));
    let score = state.judge().unwrap().final_state();

    assert_eq!(score.counts.count_100, 1);

    assert_eq!(score.combo, 1, "the head still counts");
    assert_eq!(score.max_combo, 1);
}

#[test]
fn a_dropped_tick_breaks_the_combo_but_a_dropped_tail_does_not() {
    let map = beatmap(TICKED_SLIDER);
    let ball = |t: i64| {
        let progress = ((t as f64 - 1000.0) / 1000.0).clamp(0.0, 1.0);
        ((280.0 * progress) as f32, 0.0)
    };

    let lost_tick = frames_over(900, 2100, ball, |t| t >= 1000 && !(1450..1550).contains(&t));
    let lost_tail = frames_over(900, 2100, ball, |t| (1000..1900).contains(&t));

    let after_tick = GameState::new(&map, &replay_with(lost_tick, 0))
        .judge()
        .unwrap()
        .final_state();
    let after_tail = GameState::new(&map, &replay_with(lost_tail, 0))
        .judge()
        .unwrap()
        .final_state();

    assert_eq!(after_tick.counts.count_100, 1);
    assert_eq!(after_tick.combo, 1, "combo restarted at the tail");

    assert_eq!(after_tail.counts.count_100, 1);
    assert_eq!(after_tail.combo, 2, "head and tick, uninterrupted");
}

#[test]
fn letting_go_a_hair_early_still_keeps_the_tail() {
    let map = beatmap(SHORT_SLIDER);

    let kept = frames_over(
        900,
        1600,
        |t| (ball_x(t, 1000.0, 500.0), 0.0),
        |t| (1000..1470).contains(&t),
    );
    let dropped = frames_over(
        900,
        1600,
        |t| (ball_x(t, 1000.0, 500.0), 0.0),
        |t| (1000..1450).contains(&t),
    );

    assert_eq!(judged(&map, &replay_with(kept, 0)).count_300, 1);
    assert_eq!(judged(&map, &replay_with(dropped, 0)).count_100, 1);
}

#[test]
fn a_short_slide_gets_half_its_length_of_grace_not_a_flat_36ms() {
    let map = beatmap(
        "
[Difficulty]
CircleSize:5
OverallDifficulty:5
SliderMultiplier:1.4
SliderTickRate:1

[TimingPoints]
0,250,4,2,0,60,1,0

[HitObjects]
0,0,1000,2,0,L|35:0,1,35
",
    );
    let state = GameState::from_beatmap(&map, Mods::default());
    let slider = &state.timeline().objects[0];
    assert!((slider.end_ms - 1062.5).abs() < 1e-6, "{}", slider.end_ms);

    let ball = |t: i64| {
        let progress = ((t as f64 - 1000.0) / 62.5).clamp(0.0, 1.0);
        ((35.0 * progress) as f32, 0.0)
    };

    let frames = frames_over(900, 1200, ball, |t| (1000..1040).contains(&t));
    assert_eq!(judged(&map, &replay_with(frames, 0)).count_300, 1);

    let early = frames_over(900, 1200, ball, |t| (1000..1030).contains(&t));
    assert_eq!(judged(&map, &replay_with(early, 0)).count_100, 1);
}

#[test]
fn a_missed_head_still_lets_the_body_score() {
    let map = beatmap(SHORT_SLIDER);

    let frames = frames_over(500, 1600, |t| (ball_x(t, 1000.0, 500.0), 0.0), |_| true);
    let counts = judged(&map, &replay_with(frames, 0));

    assert_eq!(counts.count_100, 1, "half the parts landed");
    assert_eq!(counts.count_miss, 0, "the slider itself is not a miss");
}

#[test]
fn a_slider_nobody_touched_is_a_miss() {
    let map = beatmap(SHORT_SLIDER);
    let frames = frames_over(900, 1600, |_| (300.0, 300.0), |_| false);
    assert_eq!(judged(&map, &replay_with(frames, 0)).count_miss, 1);
}

#[test]
fn straying_outside_the_follow_circle_drops_tracking() {
    let map = beatmap(SHORT_SLIDER);

    let frames = frames_over(900, 1600, |_| (0.0, 0.0), |t| t >= 1000);
    assert_eq!(judged(&map, &replay_with(frames, 0)).count_100, 1);
}

const TICKED_SLIDER: &str = "
[Difficulty]
CircleSize:5
OverallDifficulty:5
SliderMultiplier:1.4
SliderTickRate:1

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
0,0,1000,2,0,L|280:0,1,280
";

#[test]
fn ticks_land_on_the_beat() {
    let map = beatmap(TICKED_SLIDER);
    let state = GameState::from_beatmap(&map, Mods::default());
    let slider = &state.timeline().objects[0];
    assert_eq!(slider.tick_times(), vec![1500.0]);
    assert!(slider.repeat_times().is_empty());
}

#[test]
fn dropping_a_tick_breaks_combo_and_downgrades_the_slider() {
    let map = beatmap(TICKED_SLIDER);
    let duration = 1000.0;
    let frames = frames_over(
        900,
        2100,
        |t| {
            let progress = ((t as f64 - 1000.0) / duration).clamp(0.0, 1.0);
            ((280.0 * progress) as f32, 0.0)
        },
        |t| t >= 1000 && !(1450..1550).contains(&t),
    );
    let state = GameState::new(&map, &replay_with(frames, 0));
    let judge = state.judge().unwrap();
    let score = judge.final_state();

    let tick = judge
        .events()
        .iter()
        .find(|e| e.part == Part::SliderTick)
        .expect("the slider has a tick");
    assert_eq!(tick.result, Judgement::Miss);
    assert_eq!(tick.combo_after, 0, "a dropped tick resets combo");

    assert_eq!(score.counts.count_100, 1);
    assert_eq!(score.max_combo, 1);
}

#[test]
fn the_full_combo_counts_every_part_of_every_object() {
    let map = beatmap(&format!("{}\n100,100,4000,1,0\n", TICKED_SLIDER.trim_end()));
    let state = GameState::from_beatmap(&map, Mods::default());
    assert_eq!(state.timeline().objects.len(), 2);
    assert_eq!(state.max_possible_combo(), 4);
}

#[test]
fn a_reversed_slide_meets_its_ticks_in_the_opposite_order() {
    let map = beatmap(
        "
[Difficulty]
SliderMultiplier:1.4
SliderTickRate:2

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
0,0,1000,2,0,L|140:0,2,140
",
    );
    let state = GameState::from_beatmap(&map, Mods::default());
    let slider = &state.timeline().objects[0];

    assert_eq!(slider.tick_times(), vec![1250.0, 1750.0]);
    assert_eq!(slider.repeat_times(), vec![1500.0]);
    assert_eq!(slider.end_ms, 2000.0);
}

const SPINNER: &str = "
[Difficulty]
OverallDifficulty:5

[HitObjects]
256,192,1000,12,0,2000
";

const SIXTY_A_SECOND_MS: i64 = 17;

fn spin_frames(from: i64, to: i64, turns: f64) -> Vec<ReplayFrame> {
    let span = (to - from) as f64;
    (from..=to)
        .step_by(SIXTY_A_SECOND_MS as usize)
        .map(|t| {
            let angle = (t - from) as f64 / span * turns * TAU;
            frame(
                t,
                (256.0 + 100.0 * angle.cos()) as f32,
                (192.0 + 100.0 * angle.sin()) as f32,
                Keys::K1,
            )
        })
        .collect()
}

const LONG_SPINNER: &str = "
[Difficulty]
OverallDifficulty:5

[HitObjects]
256,192,1000,12,0,5000
";

#[test]
fn the_requirement_is_counted_in_half_turns() {
    let od5 = dossier_beatmap::Difficulty::default();
    assert!((od5.half_spins_per_second() - 5.0).abs() < 1e-9);
    assert!(
        (od5.half_spins_per_second() * 60.0 / 2.0 - 150.0).abs() < 1e-9,
        "a hundred and fifty turns a minute at OD5"
    );

    let od10 = dossier_beatmap::Difficulty {
        overall_difficulty: 10.0,
        ..Default::default()
    };
    assert!((od10.half_spins_per_second() - 7.5).abs() < 1e-9);
    assert!((od10.half_spins_per_second() * 60.0 / 2.0 - 225.0).abs() < 1e-9);
}

#[test]
fn a_spinner_pays_a_hundred_a_turn_and_a_thousand_past_the_clear() {
    let map = beatmap(LONG_SPINNER);
    let required = dossier_sim::required_half_turns(&map.difficulty, 4_000.0);
    assert_eq!(required, 20.0, "OD5 over four seconds asks for twenty halves");

    let parts = |turns: f64| {
        let state = GameState::new(&map, &replay_with(spin_frames(1000, 5000, turns), 0));
        let judge = state.judge().unwrap();
        let counted = |wanted: dossier_sim::Part| {
            judge.events().iter().filter(|e| e.part == wanted).count()
        };
        (
            counted(dossier_sim::Part::SpinnerPoints),
            counted(dossier_sim::Part::SpinnerBonus),
        )
    };

    assert_eq!(parts(8.0), (8, 0), "eight turns, a hundred on each, no bonus");
    assert_eq!(
        parts(11.0),
        (11, 0),
        "the clear is at ten turns and the bonus still waits"
    );
    assert_eq!(
        parts(15.0),
        (15, 3),
        "the bonus starts once the halves pass the requirement by three"
    );
}

#[test]
fn a_completed_spinner_is_a_three_hundred() {
    let map = beatmap(SPINNER);
    let counts = judged(&map, &replay_with(spin_frames(1000, 2000, 4.0), 0));
    assert_eq!(counts.count_300, 1);
}

#[test]
fn a_spinner_nobody_span_is_a_miss() {
    let map = beatmap(SPINNER);
    let frames = frames_over(900, 2100, |_| (256.0, 100.0), |_| false);
    assert_eq!(judged(&map, &replay_with(frames, 0)).count_miss, 1);
}

#[test]
fn a_nearly_finished_spinner_scores_partially() {
    let map = beatmap(LONG_SPINNER);

    let counts = judged(&map, &replay_with(spin_frames(1000, 5000, 10.0), 0));
    assert_eq!(counts.count_100, 1);

    let counts = judged(&map, &replay_with(spin_frames(1000, 5000, 8.0), 0));
    assert_eq!(counts.count_50, 1);
}

#[test]
fn an_ordinary_spin_rate_clears_an_ordinary_spinner() {
    let map = beatmap(LONG_SPINNER);
    let turns = 200.0 / 60.0 * 4.0;
    assert_eq!(
        judged(&map, &replay_with(spin_frames(1000, 5000, turns), 0)).count_300,
        1
    );
}

#[test]
fn a_spinner_wants_a_button_held_unless_the_player_is_relaxed() {
    let map = beatmap(SPINNER);
    let loose: Vec<ReplayFrame> = spin_frames(1000, 2000, 6.0)
        .into_iter()
        .map(|f| ReplayFrame {
            keys: Keys(0),
            ..f
        })
        .collect();

    assert_eq!(
        judged(&map, &replay_with(loose.clone(), 0)).count_miss,
        1,
        "osu! only turns the spinner while a button is down"
    );
    assert_eq!(
        judged(&map, &replay_with(loose, dossier_replay::bits::RELAX)).count_300,
        1,
        "Relax turns it without one"
    );
}

#[test]
fn a_click_during_a_spinner_does_not_reach_the_object_behind_it() {
    let map = beatmap(
        "
[Difficulty]
CircleSize:5
OverallDifficulty:5

[HitObjects]
256,192,1000,12,0,2000
100,100,2500,1,0
",
    );
    let mut frames = spin_frames(1000, 2000, 6.0);
    frames.extend(click(1500, 100.0, 100.0));
    frames.extend(click(2500, 100.0, 100.0));
    frames.sort_by_key(|f| f.time_ms);

    let counts = judged(&map, &replay_with(frames, 0));
    assert_eq!(counts.count_300, 2, "spinner and circle both landed");
    assert_eq!(counts.count_miss, 0);
}

#[test]
fn hard_rock_mirrors_the_playfield() {
    let map = beatmap("[HitObjects]\n100,100,1000,1,0\n");
    let plain = GameState::from_beatmap(&map, Mods::default());
    let hr = GameState::from_beatmap(&map, Mods::new(dossier_replay::bits::HARD_ROCK));

    assert_eq!(plain.timeline().objects[0].pos.y, 100.0);
    assert_eq!(hr.timeline().objects[0].pos.y, 284.0, "384 - 100");
    assert_eq!(hr.timeline().objects[0].pos.x, 100.0, "x is untouched");
}

#[test]
fn a_hard_rock_replay_is_judged_against_mirrored_positions() {
    let map = beatmap(
        "
[Difficulty]
CircleSize:5
OverallDifficulty:5

[HitObjects]
100,100,1000,1,0
",
    );
    let hr = dossier_replay::bits::HARD_ROCK;

    let mirrored = replay_with(click(1000, 100.0, 284.0), hr);
    let authored = replay_with(click(1000, 100.0, 100.0), hr);

    assert_eq!(judged(&map, &mirrored).count_300, 1);
    assert_eq!(judged(&map, &authored).count_miss, 1);
}

#[test]
fn hard_rock_tightens_the_windows_it_is_judged_with() {
    let map = beatmap(
        "[Difficulty]\nCircleSize:5\nOverallDifficulty:5\n\n[HitObjects]\n100,100,1000,1,0\n",
    );

    let hr = dossier_replay::bits::HARD_ROCK;
    let state = GameState::new(&map, &replay_with(click(1045, 100.0, 284.0), hr));
    assert_eq!(state.judge().unwrap().events()[0].result, Judgement::Ok);

    let plain = GameState::new(&map, &replay_with(click(1045, 100.0, 100.0), 0));
    assert_eq!(plain.judge().unwrap().events()[0].result, Judgement::Great);
}

#[test]
fn a_head_missed_on_a_short_slider_breaks_after_the_tail_lands() {
    let map = beatmap(
        "
[Difficulty]
ApproachRate:5
OverallDifficulty:5
CircleSize:5
SliderMultiplier:1.4
SliderTickRate:1

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
100,100,1000,1,0
300,100,1500,2,0,L|335:100,1,35
",
    );

    let replay = replay_with(
        frames_over(
            990,
            1700,
            |t| {
                if t < 1200 {
                    (100.0, 100.0)
                } else {
                    (317.0, 100.0)
                }
            },
            |t| t >= 1000,
        ),
        0,
    );

    let state = GameState::new(&map, &replay);
    let judge = state.judge().unwrap();

    let head = judge
        .events_for(1)
        .find(|e| e.part == Part::SliderHead)
        .expect("the slider has a head");
    let tail = judge
        .events_for(1)
        .find(|e| e.part == Part::SliderTail)
        .expect("the slider has a tail");
    assert!(head.result.is_miss(), "no press ever reached the head");
    assert!(!tail.result.is_miss(), "the slider was tracked throughout");
    assert!(
        tail.time_ms < head.time_ms,
        "the tail happens at {}ms and the head's window shuts at {}ms",
        tail.time_ms,
        head.time_ms
    );

    assert_eq!(judge.final_state().max_combo, 2);
}

#[test]
fn verification_compares_our_totals_with_the_replays_own() {
    let map = beatmap(
        "
[Difficulty]
CircleSize:5
OverallDifficulty:5

[HitObjects]
100,100,1000,1,0
100,100,1400,1,0
",
    );
    let mut frames = click(1000, 100.0, 100.0);
    frames.extend(click(1400, 100.0, 100.0));

    let mut replay = replay_with(frames, 0);
    replay.hits.count_300 = 2;
    replay.max_combo = 2;

    let state = GameState::new(&map, &replay);
    let check = state.verify(&replay).expect("a replay was supplied");
    assert!(check.is_exact(), "{check:?}");

    replay.hits.count_geki = 42;
    replay.hits.count_katu = 7;
    let with_awards = GameState::new(&map, &replay).verify(&replay).unwrap();
    assert!(with_awards.is_exact(), "{with_awards:?}");

    replay.max_combo = 3;
    let off = GameState::new(&map, &replay).verify(&replay).unwrap();
    assert!(off.counts_match());
    assert!(!off.combo_matches());
}

#[test]
fn a_play_that_ended_early_is_compared_over_the_part_that_happened() {
    let map = beatmap(
        "
[Difficulty]
CircleSize:5
OverallDifficulty:5

[HitObjects]
100,100,1000,1,0
100,100,1400,1,0
100,100,1800,1,0
100,100,2200,1,0
",
    );
    let mut frames = click(1000, 100.0, 100.0);
    frames.extend(click(1400, 100.0, 100.0));

    let mut replay = replay_with(frames, 0);
    replay.hits.count_300 = 2;
    replay.max_combo = 2;

    let check = GameState::new(&map, &replay)
        .verify(&replay)
        .expect("a replay was supplied");
    assert!(!check.finished(), "{check:?}");
    assert_eq!((check.judged, check.objects), (2, 4));
    assert!(check.is_exact(), "{check:?}");
    assert_eq!(check.ours.count_miss, 0, "{check:?}");
}

#[test]
fn a_header_with_no_counts_at_all_is_not_read_as_a_play_that_ended_early() {
    let map = beatmap(
        "
[Difficulty]
CircleSize:5
OverallDifficulty:5

[HitObjects]
100,100,1000,1,0
100,100,1400,1,0
",
    );
    let replay = replay_with(click(1000, 100.0, 100.0), 0);

    let check = GameState::new(&map, &replay)
        .verify(&replay)
        .expect("a replay was supplied");
    assert!(check.finished(), "{check:?}");
    assert_eq!(check.ours.count_300, 1);
    assert_eq!(
        check.ours.count_miss, 1,
        "the unclicked note is still a miss"
    );
}

const TRACKED_SLIDER: &str = "
[Difficulty]
ApproachRate:5
OverallDifficulty:5
CircleSize:4
SliderMultiplier:1.4
SliderTickRate:1

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
0,0,1000,2,0,L|280:0,1,280
";

#[test]
fn the_follow_circle_only_opens_once_a_slide_has_started() {
    let map = beatmap(TRACKED_SLIDER);
    let radius = map.difficulty.circle_radius();
    assert!(
        60.0 > radius && 60.0 < radius * 2.4,
        "the test sits between the two radii"
    );

    let mut frames = Vec::new();
    for step in 0..=20 {
        let t = 1000 + step * 50;
        let x = (step as f32) * 14.0;
        frames.push(frame(t, x, 60.0, 1));
    }
    let counts = judged(&map, &replay_with(frames, 0));

    assert_eq!(counts.count_300, 0, "no slide was ever established");
}

#[test]
fn a_slide_that_starts_inside_the_circle_keeps_the_wider_tolerance() {
    let map = beatmap(TRACKED_SLIDER);
    let mut frames = vec![frame(1000, 0.0, 0.0, 1)];
    for step in 1..=20 {
        let t = 1000 + step * 50;
        let x = (step as f32) * 14.0;
        frames.push(frame(t, x, 60.0, 1));
    }
    let counts = judged(&map, &replay_with(frames, 0));

    assert_eq!(counts.count_300, 1, "the slide carried the whole slider");
}

#[test]
fn a_click_before_the_window_opens_is_recorded_as_a_shake() {
    let map = beatmap(ONE_CIRCLE);

    let frames = frames_over(
        500,
        1100,
        |_| (100.0, 100.0),
        |t| (600..=620).contains(&t) || (1000..=1020).contains(&t),
    );
    let state = GameState::new(&map, &replay_with(frames, 0));
    let judge = state.judge().expect("a replay was attached");

    let shakes = judge.shakes();
    assert_eq!(shakes.len(), 1, "{shakes:?}");
    assert_eq!(shakes[0].0, 0, "aimed at the first note");
    assert!(
        (shakes[0].1 - 600.0).abs() < 1.0,
        "at the moment of the click"
    );

    assert_eq!(judge.final_state().counts.count_300, 1);
}

#[test]
fn a_click_on_a_note_that_has_not_appeared_shakes_nothing() {
    let map = beatmap(
        "
[Difficulty]
CircleSize:5
OverallDifficulty:5
ApproachRate:5

[HitObjects]
100,100,3000,1,0
",
    );
    let frames = frames_over(400, 600, |_| (100.0, 100.0), |t| (500..=520).contains(&t));
    let state = GameState::new(&map, &replay_with(frames, 0));
    assert!(state.judge().expect("attached").shakes().is_empty());
}

#[test]
fn a_click_far_out_on_a_visible_note_shakes_it() {
    let map = beatmap(ONE_CIRCLE);
    let frames = frames_over(0, 200, |_| (100.0, 100.0), |t| (100..=120).contains(&t));
    let state = GameState::new(&map, &replay_with(frames, 0));
    let judge = state.judge().expect("attached");

    assert_eq!(judge.shakes().len(), 1, "{:?}", judge.shakes());

    assert_eq!(judge.final_state().counts.count_miss, 1);
}

const TWO_CIRCLES: &str = "
[Difficulty]
CircleSize:5
OverallDifficulty:5
ApproachRate:5

[HitObjects]
100,100,1000,1,0
300,100,1100,1,0
";

#[test]
fn an_unjudged_earlier_note_still_blocks_a_later_one() {
    let map = beatmap(TWO_CIRCLES);
    let state = GameState::new(&map, &replay_with(click(1100, 300.0, 100.0), 0));
    let counts = state.judge().expect("attached").final_state().counts;

    assert_eq!(counts.count_300, 0, "the click was refused");
    assert_eq!(counts.count_miss, 2, "and both notes ran out");
}

const STACK: &str = "
[General]
StackLeniency: 0.7

[Difficulty]
CircleSize:5
OverallDifficulty:5
ApproachRate:5

[HitObjects]
100,100,1000,1,0
100,100,1100,1,0
";

#[test]
fn a_click_inside_both_notes_of_a_stack_takes_the_earlier_one() {
    let map = beatmap(STACK);
    let state = GameState::new(&map, &replay_with(click(1000, 100.0, 100.0), 0));
    let judge = state.judge().expect("attached");

    assert_eq!(
        judge.trace()[0].verdict,
        Verdict::Landed { object: 0 },
        "{:?}",
        judge.trace()
    );
    let counts = judge.final_state().counts;
    assert_eq!((counts.count_300, counts.count_miss), (1, 1));
}

#[test]
fn a_click_only_on_the_later_note_of_a_stack_passes_through_untouched() {
    let map = beatmap(STACK);
    let state = GameState::new(&map, &replay_with(click(1100, 122.0, 122.0), 0));
    let judge = state.judge().expect("attached");

    assert_eq!(
        judge.trace()[0].verdict,
        Verdict::Ignored { object: 1 },
        "{:?}",
        judge.trace()
    );
    assert!(judge.shakes().is_empty(), "ignored, not shaken");
    assert_eq!(
        judge.final_state().counts.count_miss,
        2,
        "the click did nothing, so both notes ran out"
    );
}

#[test]
fn once_the_stacks_front_is_judged_the_click_reaches_the_note_behind() {
    let map = beatmap(STACK);
    let mut frames = click(1000, 96.8, 96.8);
    frames.extend(click(1100, 122.0, 122.0));
    let counts = judged(&map, &replay_with(frames, 0));

    assert_eq!(counts.count_300, 2, "both notes were clicked");
    assert_eq!(counts.count_miss, 0);
}

#[test]
fn a_note_under_a_travelling_slider_never_sees_the_click() {
    let map = beatmap(
        "
[General]
StackLeniency: 0.7

[Difficulty]
CircleSize:5
OverallDifficulty:5
ApproachRate:5
SliderMultiplier:1.4

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
100,100,1000,2,0,L|240:100,1,140
100,100,1200,1,0
",
    );
    let mut frames = click(1000, 100.0, 100.0);
    frames.extend(click(1200, 100.0, 100.0));
    let state = GameState::new(&map, &replay_with(frames, 0));
    let judge = state.judge().expect("attached");

    assert_eq!(
        judge.trace()[1].verdict,
        Verdict::Ignored { object: 1 },
        "{:?}",
        judge.trace()
    );
}

#[test]
fn a_note_after_the_slider_has_finished_is_clickable_again() {
    let map = beatmap(
        "
[General]
StackLeniency: 0.7

[Difficulty]
CircleSize:5
OverallDifficulty:5
ApproachRate:5
SliderMultiplier:1.4

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
100,100,1000,2,0,L|240:100,1,140
240,100,1700,1,0
",
    );
    let mut frames: Vec<ReplayFrame> = (990..=1500)
        .step_by(10)
        .map(|t| {
            let x = if t < 1000 {
                100.0
            } else {
                100.0 + (t - 1000) as f32 * 0.28
            };
            frame(t, x, 100.0, if t >= 1000 { Keys::K1 } else { 0 })
        })
        .collect();
    frames.extend(click(1700, 243.2, 103.2));
    let counts = judged(&map, &replay_with(frames, 0));

    assert_eq!(counts.count_300, 2, "the slider and the note after it");
    assert_eq!(counts.count_miss, 0);
}

#[test]
fn two_notes_at_the_very_same_moment_do_not_block_each_other() {
    let map = beatmap(
        "
[General]
StackLeniency: 0.7

[Difficulty]
CircleSize:5
OverallDifficulty:5
ApproachRate:5

[HitObjects]
100,100,1000,1,0
100,100,1000,1,0
",
    );
    let mut frames = click(1000, 96.8, 96.8);
    frames.extend(click(1010, 100.0, 100.0));
    let counts = judged(&map, &replay_with(frames, 0));

    assert_eq!(counts.count_300, 2, "both were reachable");
    assert_eq!(counts.count_miss, 0);
}

#[test]
fn every_press_is_accounted_for() {
    let map = beatmap(TWO_CIRCLES);
    let frames = frames_over(
        900,
        1400,
        |_| (100.0, 100.0),
        |t| (950..=970).contains(&t) || (1000..=1020).contains(&t) || (1300..=1320).contains(&t),
    );
    let state = GameState::new(&map, &replay_with(frames, 0));
    let summary = state.press_verdicts();

    assert_eq!(summary.total(), 3, "one per click: {summary:?}");
    assert_eq!(
        summary.total(),
        state.press_count(),
        "and the same number the click reader found"
    );
}

#[test]
fn a_run_of_refusals_is_reported_with_where_it_began() {
    let map = beatmap(TWO_CIRCLES);

    let frames = frames_over(1000, 1100, |_| (300.0, 100.0), |t| (t / 10) % 2 == 0);
    let state = GameState::new(&map, &replay_with(frames, 0));
    let summary = state.press_verdicts();

    assert!(summary.refused >= 4, "{summary:?}");
    assert_eq!(summary.landed, 0);
    let (at, count) = *summary
        .refusal_runs
        .first()
        .expect("a run of refusals is reported");
    assert!(count >= 4, "{summary:?}");
    assert!((1000.0..=1100.0).contains(&at), "at {at}");
}

#[test]
fn a_map_with_no_replay_has_nothing_to_account_for() {
    let map = beatmap(TWO_CIRCLES);
    let state = GameState::from_beatmap(&map, Mods::default());
    assert_eq!(state.press_verdicts().total(), 0);
}

#[test]
fn the_header_version_says_which_ruleset_to_read_the_replay_with() {
    use dossier_sim::Ruleset;
    assert_eq!(Ruleset::of_replay_version(20_260_412), Ruleset::STABLE);
    assert_eq!(Ruleset::of_replay_version(20_231_121), Ruleset::STABLE);
    assert_eq!(Ruleset::of_replay_version(30_000_016), Ruleset::LAZER);
    assert_eq!(Ruleset::of_replay_version(30_000_018), Ruleset::LAZER);
}

const TRAILING_STREAM: &str = "
[Difficulty]
CircleSize:4
OverallDifficulty:6.5
ApproachRate:9

[HitObjects]
100,100,1000,1,0
140,100,1080,1,0
180,100,1160,1,0
220,100,1240,1,0
";

fn trailing_clicks() -> Vec<ReplayFrame> {
    let mut frames = Vec::new();
    for (at, x) in [(1040, 145.0), (1120, 185.0), (1200, 225.0)] {
        frames.push(frame(at - 10, x, 100.0, 0));
        frames.push(frame(at, x, 100.0, Keys::K1));
        frames.push(frame(at + 10, x, 100.0, 0));
    }
    frames
}

#[test]
fn stable_locks_the_stream_and_lazer_lets_it_through() {
    let map = beatmap(TRAILING_STREAM);

    let mut stable = replay_with(trailing_clicks(), 0);
    stable.game_version = 20_260_412;
    let stable_counts = judged(&map, &stable);

    let mut lazer = replay_with(trailing_clicks(), 0);
    lazer.game_version = 30_000_018;
    let lazer_counts = judged(&map, &lazer);

    assert_eq!(
        (stable_counts.count_300, stable_counts.count_miss),
        (0, 4),
        "stable strands the first note and the lock never lets the player back          in: {stable_counts:?}"
    );
    assert_eq!(
        (lazer_counts.count_300, lazer_counts.count_miss),
        (3, 1),
        "lazer writes the stranded note off and carries on: {lazer_counts:?}"
    );
}

#[test]
fn lazer_writes_off_a_stranded_note_at_the_click_not_at_its_window() {
    let map = beatmap(TRAILING_STREAM);
    let mut lazer = replay_with(trailing_clicks(), 0);
    lazer.game_version = 30_000_018;

    let state = GameState::new(&map, &lazer);
    let judge = state.judge().expect("attached");
    let miss = judge
        .events()
        .iter()
        .find(|e| e.result == Judgement::Miss)
        .expect("one note was stranded");
    let object = &state.timeline().objects[miss.object_index];

    assert!(
        miss.time_ms < object.start_ms + state.difficulty().hit_window_50(),
        "written off at {:.0}ms, before its window shut at {:.0}ms",
        miss.time_ms,
        object.start_ms + state.difficulty().hit_window_50()
    );
}

#[test]
fn a_slider_swallows_clicks_from_the_moment_it_spawns() {
    let map = beatmap(
        "
[Difficulty]
ApproachRate:5
OverallDifficulty:5
CircleSize:5
SliderMultiplier:1.4

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
100,100,1000,2,0,L|240:100,1,140
130,100,1300,1,0
",
    );

    let mut frames = click(980, 100.0, 100.0);
    frames.extend(click(990, 115.0, 100.0));
    let state = GameState::new(&map, &replay_with(frames, 0));
    let judge = state.judge().expect("attached");

    assert_eq!(
        judge.trace()[1].verdict,
        Verdict::Ignored { object: 1 },
        "the slider is on screen and unjudged, so it swallows the press: {:?}",
        judge.trace()
    );

    let circle = judge
        .events()
        .iter()
        .find(|e| e.object_index == 1)
        .expect("the circle is judged");
    assert_eq!(
        circle.time_ms, 1450.0,
        "eaten at the press instead of running its window out: {circle:?}"
    );
}

#[test]
fn a_note_keeps_blocking_for_two_milliseconds_after_its_window_shuts() {
    let map = beatmap(
        "[Difficulty]\nHPDrainRate:5\nCircleSize:5\nOverallDifficulty:5\n\n\
         [HitObjects]\n100,100,1000,1,0\n400,300,1200,1,0\n",
    );
    let landed = |at: i64| {
        judged(&map, &replay_with(click(at, 400.0, 300.0), 0)).count_300
            + judged(&map, &replay_with(click(at, 400.0, 300.0), 0)).count_100
            + judged(&map, &replay_with(click(at, 400.0, 300.0), 0)).count_50
    };

    assert_eq!(landed(1151), 0, "the blocker was freed a millisecond early");

    assert_eq!(landed(1152), 1, "the blocker was never freed at all");
}

#[test]
fn a_click_on_a_note_whose_window_has_shut_spends_it_there_and_then() {
    let map = beatmap(ONE_CIRCLE);
    let state = GameState::new(&map, &replay_with(click(1151, 100.0, 100.0), 0));
    let judge = state.judge().expect("judged");
    let event = judge.events()[0];

    assert_eq!(event.result, Judgement::Miss);
    assert_eq!(event.time_ms, 1151.0, "dated to the click");
    assert_eq!(event.error_ms, Some(151.0), "and it knows how late it was");
}

#[test]
fn a_slider_is_worth_its_head_in_lazer_and_its_pieces_in_stable() {
    let map = beatmap(
        "[Difficulty]\nHPDrainRate:5\nCircleSize:5\nOverallDifficulty:5\n\
         SliderMultiplier:1.0\nSliderTickRate:1\n\n\
         [TimingPoints]\n0,500,4,2,0,100,1,0\n\n\
         [HitObjects]\n100,100,1000,2,0,L|200:100,1,100\n",
    );

    let mut frames = Vec::new();
    for t in (1050..=1600).step_by(10) {
        let progress = ((t - 1060) as f32 / 500.0).clamp(0.0, 1.0);
        frames.push(frame(
            t,
            100.0 + 100.0 * progress,
            100.0,
            if (1060..=1560).contains(&t) {
                Keys::K1
            } else {
                0
            },
        ));
    }

    let stable = replay_with(frames.clone(), 0);
    let mut lazer = replay_with(frames, 0);
    lazer.game_version = 30_000_016;

    let verdict = |replay: &dossier_replay::Replay| {
        let state = GameState::new(&map, replay);
        let judge = state.judge().expect("judged");
        let slider = judge
            .events()
            .iter()
            .find(|e| e.part == Part::Slider)
            .expect("the slider was judged");

        assert!(
            !judge
                .events()
                .iter()
                .any(|e| e.part != Part::Slider && e.result.is_miss()),
            "{:?}",
            judge.events()
        );
        slider.result
    };

    assert_eq!(verdict(&stable), Judgement::Great, "stable keeps it whole");
    assert_eq!(verdict(&lazer), Judgement::Ok, "lazer scores the head");
}

#[test]
fn landing_a_late_head_starts_the_slide_in_lazer_but_not_in_stable() {
    let map = beatmap(
        "[Difficulty]\nHPDrainRate:5\nCircleSize:5\nOverallDifficulty:5\n\
         SliderMultiplier:2.0\nSliderTickRate:1\n\n\
         [TimingPoints]\n0,200,4,2,0,100,1,0\n\n\
         [HitObjects]\n100,100,1000,2,0,L|300:100,1,200\n",
    );

    let mut frames = Vec::new();
    for t in (1040..=1260).step_by(5) {
        let x = 100.0 + (t as f32 - 1050.0).max(0.0);
        frames.push(frame(
            t,
            x,
            100.0,
            if (1050..=1250).contains(&t) {
                Keys::K1
            } else {
                0
            },
        ));
    }

    let tail_landed = |version: i32| {
        let mut replay = replay_with(frames.clone(), 0);
        replay.game_version = version;
        let state = GameState::new(&map, &replay);
        let judge = state.judge().expect("judged");

        let head = judge
            .events()
            .iter()
            .find(|e| e.part == Part::SliderHead)
            .expect("a head");
        assert!(!head.result.is_miss(), "the fixture must land the head");
        !judge
            .events()
            .iter()
            .find(|e| e.part == Part::SliderTail)
            .expect("a tail")
            .result
            .is_miss()
    };

    assert!(tail_landed(30_000_016), "lazer should carry the slide over");
    assert!(
        !tail_landed(20_260_101),
        "stable has no such rule, and giving it one costs the corpus half its \
         exact replays"
    );
}

fn score_v2_slider(late_by: i64, hold_until: i64, mods: u32) -> HitCounts {
    let map = beatmap(SHORT_SLIDER);
    let frames = frames_over(
        900,
        1600,
        |t| (ball_x(t, 1000.0, 500.0), 0.0),
        |t| (1000 + late_by..hold_until).contains(&t),
    );
    judged(&map, &replay_with(frames, mods))
}

#[test]
fn score_v2_makes_a_stable_slider_worth_what_its_head_was_worth() {
    let plain = score_v2_slider(60, 1600, 0);
    assert_eq!((plain.count_300, plain.count_100), (1, 0), "{plain:?}");

    let v2 = score_v2_slider(60, 1600, dossier_replay::bits::SCORE_V2);
    assert_eq!((v2.count_300, v2.count_100), (0, 1), "{v2:?}");
}

#[test]
fn score_v2_still_wants_the_pieces_after_the_head_is_in() {
    let dropped = score_v2_slider(0, 1200, dossier_replay::bits::SCORE_V2);
    assert_eq!(
        (dropped.count_300, dropped.count_100),
        (0, 1),
        "{dropped:?}"
    );

    let whole = score_v2_slider(0, 1600, dossier_replay::bits::SCORE_V2);
    assert_eq!((whole.count_300, whole.count_100), (1, 0), "{whole:?}");
}

#[test]
fn the_unstable_rate_is_ten_times_the_spread_of_the_errors() {
    let map = beatmap(
        "
[Difficulty]
CircleSize:5
OverallDifficulty:5

[HitObjects]
100,100,1000,1,0
200,100,2000,1,0
300,100,3000,1,0
",
    );
    let mut frames = Vec::new();
    for (at, x, offset) in [
        (1000i64, 100.0f32, -20i64),
        (2000, 200.0, 0),
        (3000, 300.0, 20),
    ] {
        let click = at + offset;
        frames.push(frame(click - 5, x, 100.0, 0));
        frames.push(frame(click, x, 100.0, Keys::K1));
        frames.push(frame(click + 5, x, 100.0, 0));
    }
    let state = GameState::new(&map, &replay_with(frames, 0));
    let judge = state.judge().expect("a replay was judged");

    let rate = judge
        .unstable_rate(f64::MAX)
        .expect("three hits have a spread");
    let expected = (800.0f64 / 3.0).sqrt() * 10.0;
    assert!(
        (rate - expected).abs() < 1.0,
        "{rate:.1} against {expected:.1}"
    );
}

#[test]
fn the_unstable_rate_waits_for_a_second_hit() {
    let map = beatmap(
        "
[Difficulty]
CircleSize:5
OverallDifficulty:5

[HitObjects]
100,100,1000,1,0
200,100,2000,1,0
",
    );
    let mut frames = Vec::new();
    for (at, x) in [(1000i64, 100.0f32), (2000, 200.0)] {
        frames.push(frame(at - 5, x, 100.0, 0));
        frames.push(frame(at, x, 100.0, Keys::K1));
        frames.push(frame(at + 5, x, 100.0, 0));
    }
    let state = GameState::new(&map, &replay_with(frames, 0));
    let judge = state.judge().expect("a replay was judged");

    assert!(
        judge.unstable_rate(1500.0).is_none(),
        "one hit is not a spread"
    );
    assert!(judge.unstable_rate(2500.0).is_some(), "two are");
}

#[test]
fn a_live_spinner_takes_a_press_wherever_the_cursor_is() {
    let map = beatmap(
        "[Difficulty]\nHPDrainRate:5\nCircleSize:5\nOverallDifficulty:5\nApproachRate:5\n\n\
         [HitObjects]\n256,192,1000,12,0,2300\n400,300,2200,1,0\n",
    );

    let mut stable = replay_with(click(2150, 400.0, 300.0), 0);
    stable.game_version = 20_260_412;
    let stable_counts = judged(&map, &stable);

    let mut lazer = replay_with(click(2150, 400.0, 300.0), 0);
    lazer.game_version = 30_000_018;
    let lazer_counts = judged(&map, &lazer);

    let stable_landed = stable_counts.count_300 + stable_counts.count_100 + stable_counts.count_50;
    let lazer_landed = lazer_counts.count_300 + lazer_counts.count_100 + lazer_counts.count_50;
    assert_eq!(
        stable_landed, 0,
        "the spinner should have swallowed it: {stable_counts:?}"
    );
    assert!(
        lazer_landed > 0,
        "lazer has no such rule and the click reaches the circle: {lazer_counts:?}"
    );
}

#[test]
fn a_relax_replay_is_clicked_for_rather_than_read() {
    let map = beatmap(TWO_CIRCLES);
    let frames = vec![
        dossier_replay::ReplayFrame {
            time_ms: 900,
            x: 100.0,
            y: 100.0,
            keys: dossier_replay::Keys(0),
        },
        dossier_replay::ReplayFrame {
            time_ms: 1000,
            x: 100.0,
            y: 100.0,
            keys: dossier_replay::Keys(0),
        },
        dossier_replay::ReplayFrame {
            time_ms: 1100,
            x: 300.0,
            y: 100.0,
            keys: dossier_replay::Keys(0),
        },
        dossier_replay::ReplayFrame {
            time_ms: 1200,
            x: 300.0,
            y: 100.0,
            keys: dossier_replay::Keys(0),
        },
    ];

    let mut plain = replay_with(frames.clone(), 0);
    plain.game_version = 20_260_412;
    let plain_counts = judged(&map, &plain);
    assert_eq!(
        plain_counts.count_miss, 2,
        "nobody pressed anything: {plain_counts:?}"
    );

    let mut relaxed = replay_with(frames, dossier_replay::bits::RELAX);
    relaxed.game_version = 20_260_412;
    let relax_counts = judged(&map, &relaxed);
    assert_eq!(
        relax_counts.count_miss, 0,
        "the game should have clicked for them: {relax_counts:?}"
    );
}

#[test]
fn a_relax_slider_is_held_as_well_as_clicked() {
    let map = beatmap(
        "[Difficulty]\nHPDrainRate:5\nCircleSize:5\nOverallDifficulty:5\nApproachRate:5\n\
         SliderMultiplier:1\nSliderTickRate:4\n\n[TimingPoints]\n0,500,4,2,0,60,1,0\n\n\
         [HitObjects]\n100,100,1000,2,0,L|300:100,1,200\n",
    );
    let mut frames = Vec::new();
    for step in 0..90 {
        let t = 900 + step * 25;
        let travel = ((t - 1000).max(0) as f64 / 1000.0).min(1.0);
        frames.push(dossier_replay::ReplayFrame {
            time_ms: t,
            x: (100.0 + 200.0 * travel) as f32,
            y: 100.0,
            keys: dossier_replay::Keys(0),
        });
    }

    let mut relaxed = replay_with(frames, dossier_replay::bits::RELAX);
    relaxed.game_version = 20_260_412;
    let counts = judged(&map, &relaxed);

    assert_eq!(
        (counts.count_50, counts.count_miss),
        (0, 0),
        "the slider was followed and should have been held throughout: {counts:?}"
    );
}

const THREE_AHEAD: &str = "
[Difficulty]
CircleSize:5
OverallDifficulty:5
ApproachRate:5

[HitObjects]
100,100,1000,1,0
200,100,1100,1,0
300,100,1200,1,0
";

#[test]
fn lazer_asks_the_last_note_behind_the_target_not_the_first() {
    let map = beatmap(THREE_AHEAD);
    let mut replay = replay_with(click(950, 300.0, 100.0), 0);
    replay.game_version = 30_000_018;
    let state = GameState::new(&map, &replay);
    let judge = state.judge().expect("attached");

    assert_eq!(
        judge.trace()[0].verdict,
        Verdict::Refused {
            object: 2,
            blocked_by: 1,
        },
        "the note immediately behind the target, not the one before it: {:?}",
        judge.trace()
    );
}

const SPINNER_BETWEEN: &str = "
[Difficulty]
CircleSize:5
OverallDifficulty:5
ApproachRate:5

[HitObjects]
100,100,1000,1,0
256,192,1050,12,0,1150
300,100,1200,1,0
";

#[test]
fn a_spinner_cannot_be_what_blocks_a_note_under_lazer() {
    let map = beatmap(SPINNER_BETWEEN);
    let mut replay = replay_with(click(1020, 300.0, 100.0), 0);
    replay.game_version = 30_000_018;
    let state = GameState::new(&map, &replay);
    let judge = state.judge().expect("attached");

    assert!(
        !matches!(judge.trace()[0].verdict, Verdict::Refused { .. }),
        "a spinner is not a blocking object: {:?}",
        judge.trace()
    );
}

#[test]
fn a_missed_note_does_not_take_the_stream_behind_it_under_relax() {
    let map = beatmap(
        "
[Difficulty]
CircleSize:5
ApproachRate:9
OverallDifficulty:5

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
20,20,1000,1,0
256,192,1120,1,0
256,192,1240,1,0
",
    );

    let frames: Vec<ReplayFrame> = (0..40)
        .map(|i| frame(900 + i * 20, 256.0, 192.0, 0))
        .collect();

    let mut relaxed = replay_with(frames, dossier_replay::bits::RELAX);
    relaxed.game_version = 20_260_412;
    let counts = judged(&map, &relaxed);
    assert_eq!(
        counts.count_miss, 1,
        "the unreachable circle took the two behind it: {counts:?}"
    );
    assert_eq!(counts.count_300, 2, "{counts:?}");
}

#[test]
fn spun_out_turns_the_spinner_for_the_player() {
    let map = beatmap(SPINNER);
    let still = frames_over(900, 2100, |_| (256.0, 100.0), |_| false);

    assert_eq!(
        judged(&map, &replay_with(still.clone(), 0)).count_miss,
        1,
        "a spinner nobody span is a miss"
    );
    assert_eq!(
        judged(&map, &replay_with(still, dossier_replay::bits::SPUN_OUT))
            .count_300,
        1,
        "with Spun Out the game turns it instead"
    );
}

#[test]
fn spun_out_turns_at_the_rate_the_game_turns_at() {
    let map = beatmap(LONG_SPINNER);
    let still = frames_over(900, 5100, |_| (256.0, 100.0), |_| false);
    let state = GameState::new(
        &map,
        &replay_with(still, dossier_replay::bits::SPUN_OUT),
    );
    let judge = state.judge().unwrap();
    let spun = judge
        .events()
        .iter()
        .filter(|e| {
            matches!(
                e.part,
                dossier_sim::Part::SpinnerSpin
                    | dossier_sim::Part::SpinnerPoints
                    | dossier_sim::Part::SpinnerBonus
            )
        })
        .count();

    let wanted = (4_000.0 * 0.03 / std::f64::consts::PI) as usize;
    assert_eq!(spun, wanted, "four seconds at three hundredths of a radian");
}

#[test]
fn easy_wins_over_hard_rock_the_way_the_game_settles_it() {
    let map = beatmap(ONE_CIRCLE);
    let both = dossier_replay::bits::EASY | dossier_replay::bits::HARD_ROCK;
    let state = GameState::new(&map, &replay_with(Vec::new(), both));
    let only_easy = GameState::new(
        &map,
        &replay_with(Vec::new(), dossier_replay::bits::EASY),
    );
    assert_eq!(
        state.difficulty().overall_difficulty,
        only_easy.difficulty().overall_difficulty,
        "the game strips Hard Rock when Easy is on, not the other way about"
    );
}

#[test]
fn a_bonus_spin_is_worth_the_thousand_the_game_shows() {
    use dossier_sim::score::stable_base_value;

    assert_eq!(
        stable_base_value(dossier_sim::Part::SpinnerBonus, dossier_sim::Judgement::Great),
        1_000,
        "osu! writes 1000 times the bonus spin on screen and pays exactly that"
    );
    assert_eq!(
        stable_base_value(dossier_sim::Part::SpinnerPoints, dossier_sim::Judgement::Great),
        100,
        "and a hundred for a turn below the threshold"
    );
    assert_eq!(
        stable_base_value(dossier_sim::Part::SpinnerSpin, dossier_sim::Judgement::Great),
        0,
        "the odd half turn is worth nothing"
    );
}

#[test]
fn a_spinner_winds_up_faster_the_shorter_it_is() {
    let long = dossier_sim::spin_acceleration(10_000.0);
    let short = dossier_sim::spin_acceleration(1_000.0);
    assert!((long - 8e-5).abs() < 1e-12, "past five seconds it is flat");
    assert!(
        short > long * 20.0,
        "a one-second spinner has to get going: {short} against {long}"
    );
}

#[test]
fn the_spinner_will_not_be_turned_faster_than_the_game_allows() {
    let map = beatmap(LONG_SPINNER);
    let absurd = spin_frames(1000, 5000, 200.0);
    let state = GameState::new(&map, &replay_with(absurd, 0));
    let counted =
        dossier_sim::spinner_half_turns(state.cursor_track(), 1_000.0, 5_000.0, state.spin());

    let ceiling = 4_000.0 * 0.05 / std::f64::consts::PI;
    assert!(
        counted <= ceiling + 0.5,
        "the game caps at a twentieth of a radian a millisecond: {counted} against {ceiling:.1}"
    );
}

#[test]
fn a_spinner_nobody_holds_turns_nothing() {
    let map = beatmap(LONG_SPINNER);
    let loose = frames_over(
        1_000,
        5_000,
        |t| {
            let angle = (t - 1_000) as f64 / 4_000.0 * 30.0 * std::f64::consts::TAU;
            (
                (256.0 + 100.0 * angle.cos()) as f32,
                (192.0 + 100.0 * angle.sin()) as f32,
            )
        },
        |_| false,
    );
    let state = GameState::new(&map, &replay_with(loose, 0));
    let counted =
        dossier_sim::spinner_half_turns(state.cursor_track(), 1_000.0, 5_000.0, state.spin());
    assert!(
        counted < 1.0,
        "the game only turns the spinner while a button is down: {counted}"
    );
}

#[test]
fn the_wind_up_is_measured_against_real_seconds() {
    let map = beatmap(LONG_SPINNER);
    let spun = spin_frames(1000, 5000, 20.0);

    let turned = |mods: u32| {
        let state = GameState::new(&map, &replay_with(spun.clone(), mods));
        dossier_sim::spinner_half_turns(state.cursor_track(), 1_000.0, 5_000.0, state.spin())
    };

    let plain = turned(0);
    let doubled = turned(dossier_replay::bits::DOUBLE_TIME);
    assert!(
        doubled < plain,
        "the wind-up is measured against real seconds, so the same map-time frames \
         accelerate slower on a doubled clock: {doubled} against {plain}"
    );
    assert!(
        doubled > plain * 0.99,
        "but only the wind-up differs, not the turning: {doubled} against {plain}"
    );
}

#[test]
fn lazer_will_not_be_cheesed_by_a_wobbling_cursor() {
    let map = beatmap(LONG_SPINNER);
    let wobble = (1_000..=5_000)
        .step_by(17)
        .map(|t| {
            let swing = if (t / 34) % 2 == 0 { 2.9 } else { -2.9 };
            frame(
                t,
                (256.0 + 100.0 * f64::cos(swing) as f32),
                (192.0 + 100.0 * f64::sin(swing) as f32),
                Keys::K1,
            )
        })
        .collect();

    let mut replay = replay_with(wobble, 0);
    replay.game_version = 30_000_016;
    let state = GameState::new(&map, &replay);
    let counted =
        dossier_sim::spinner_half_turns(state.cursor_track(), 1_000.0, 5_000.0, state.spin());

    assert!(
        counted < 2.0,
        "lazer counts whole turns in one direction, not every wobble: {counted}"
    );
}
