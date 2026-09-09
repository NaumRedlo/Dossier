use dossier_beatmap::Beatmap;
use dossier_replay::{bits, HitCounts, Keys, Mods, Replay, ReplayFrame};
use dossier_sim::{GameState, HealthTrack, Ruleset};

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

fn note_at(n: i64) -> (f32, f32) {
    (
        60.0 + (n % 7) as f32 * 55.0,
        60.0 + ((n / 7) % 5) as f32 * 55.0,
    )
}

fn note_time(n: i64) -> i64 {
    1000 + n * 333
}

fn stream(hp: f64, count: i64) -> String {
    let mut body = format!(
        "[Difficulty]\nHPDrainRate:{hp}\nCircleSize:5\nOverallDifficulty:5\n\n[HitObjects]\n"
    );
    for n in 0..count {
        let flags = if n % 8 == 0 { 5 } else { 1 };
        let (x, y) = note_at(n);
        body.push_str(&format!("{x},{y},{},{flags},0\n", note_time(n)));
    }
    body
}

fn played_perfectly(count: i64) -> Vec<ReplayFrame> {
    (0..count)
        .flat_map(|n| {
            let (x, y) = note_at(n);
            click(note_time(n), x, y)
        })
        .collect()
}

fn played_dropping(count: i64, skip: i64) -> Vec<ReplayFrame> {
    (0..count)
        .filter(|n| n % skip != 0)
        .flat_map(|n| {
            let (x, y) = note_at(n);
            click(note_time(n), x, y)
        })
        .collect()
}

#[test]
fn a_perfect_play_survives_at_every_hp() {
    for hp in [0.0, 2.0, 5.0, 7.0, 9.0, 10.0] {
        let map = beatmap(&stream(hp, 60));
        let replay = replay_with(played_perfectly(60), 0);
        let state = GameState::new(&map, &replay);
        let judge = state.judge().expect("judged");
        let track = HealthTrack::build(
            judge,
            state.timeline(),
            &map.breaks,
            map.format_version,
            Mods::new(0),
            Ruleset::STABLE,
        );

        assert_eq!(track.failed_at(), None, "a perfect play died at HP {hp}");

        let floor = dossier_beatmap::difficulty_range(hp, 195.0, 160.0, 60.0) / 200.0;
        let lowest = (0..21_000)
            .step_by(50)
            .map(|t| track.at(f64::from(t)))
            .fold(f32::INFINITY, f32::min);
        assert!(
            f64::from(lowest) >= floor - 0.02,
            "HP {hp} bottomed at {lowest} against a floor of {floor}"
        );
    }
}

#[test]
fn a_play_that_hits_nothing_dies() {
    let map = beatmap(&stream(5.0, 60));
    let replay = replay_with(Vec::new(), 0);
    let state = GameState::new(&map, &replay);
    let judge = state.judge().expect("judged");
    let track = HealthTrack::build(
        judge,
        state.timeline(),
        &map.breaks,
        map.format_version,
        Mods::new(0),
        Ruleset::STABLE,
    );

    let died = track.failed_at().expect("hitting nothing should be fatal");
    assert!(died < 5_000.0, "took until {died}ms to die");
}

#[test]
fn the_calibration_settles_on_a_real_drain_rate() {
    let mut last = 0.0;
    for hp in [0.0, 3.0, 5.0, 8.0, 10.0] {
        let map = beatmap(&stream(hp, 60));
        let replay = replay_with(played_perfectly(60), 0);
        let state = GameState::new(&map, &replay);
        let track = HealthTrack::build(
            state.judge().expect("judged"),
            state.timeline(),
            &map.breaks,
            map.format_version,
            Mods::new(0),
            Ruleset::STABLE,
        );
        let rate = track.drain_rate();
        assert!(rate > 0.0, "HP {hp} drains at nothing");

        assert!(
            rate > last,
            "HP {hp} drains at {rate}, no faster than {last}"
        );
        last = rate;
    }
}

#[test]
fn a_high_hp_map_punishes_the_same_misses_harder() {
    let dropped = played_dropping(60, 3);

    let mut lowest = Vec::new();
    for hp in [1.0, 9.0] {
        let map = beatmap(&stream(hp, 60));
        let replay = replay_with(dropped.clone(), 0);
        let state = GameState::new(&map, &replay);
        let track = HealthTrack::build(
            state.judge().expect("judged"),
            state.timeline(),
            &map.breaks,
            map.format_version,
            Mods::new(0),
            Ruleset::STABLE,
        );
        let floor = (0..21_000)
            .step_by(100)
            .map(|t| track.at(f64::from(t)))
            .fold(f32::INFINITY, f32::min);
        lowest.push(floor);
    }
    assert!(
        lowest[0] > lowest[1],
        "HP 1 bottomed at {} and HP 9 at {}",
        lowest[0],
        lowest[1]
    );
}

#[test]
fn nothing_drains_during_a_break() {
    let mut body = String::from(
        "[Difficulty]\nHPDrainRate:8\nCircleSize:5\nOverallDifficulty:5\n\n\
         [Events]\n2,4000,14000\n\n[HitObjects]\n",
    );
    let mut frames = Vec::new();
    for (n, t) in [1000, 2000, 3000, 15_000, 16_000, 17_000]
        .into_iter()
        .enumerate()
    {
        let (x, y) = note_at(n as i64);
        body.push_str(&format!("{x},{y},{t},1,0\n"));
        frames.extend(click(t, x, y));
    }

    let map = beatmap(&body);
    let replay = replay_with(frames, 0);
    let state = GameState::new(&map, &replay);
    let track = HealthTrack::build(
        state.judge().expect("judged"),
        state.timeline(),
        &map.breaks,
        map.format_version,
        Mods::new(0),
        Ruleset::STABLE,
    );

    let entering = track.at(4_500.0);
    let leaving = track.at(13_500.0);
    assert!(
        (entering - leaving).abs() < 1e-4,
        "the bar moved across the break: {entering} then {leaving}"
    );
}

#[test]
fn the_two_clients_do_not_share_a_model() {
    let map = beatmap(&stream(6.0, 60));
    let dropped = played_dropping(60, 4);
    let replay = replay_with(dropped, 0);
    let state = GameState::new(&map, &replay);
    let judge = state.judge().expect("judged");

    let stable = HealthTrack::build(
        judge,
        state.timeline(),
        &map.breaks,
        map.format_version,
        Mods::new(0),
        Ruleset::STABLE,
    );
    let lazer = HealthTrack::build(
        judge,
        state.timeline(),
        &map.breaks,
        map.format_version,
        Mods::new(0),
        Ruleset::LAZER,
    );

    let mut apart = 0f32;
    for t in (0..21_000).step_by(250) {
        apart = apart.max((stable.at(f64::from(t)) - lazer.at(f64::from(t))).abs());
    }
    assert!(apart > 0.02, "the two models agree everywhere: {apart}");
}

#[test]
fn the_bar_stays_inside_its_own_range() {
    for ruleset in [Ruleset::STABLE, Ruleset::LAZER] {
        let map = beatmap(&stream(4.0, 60));
        let messy = played_dropping(60, 5);
        let replay = replay_with(messy, 0);
        let state = GameState::new(&map, &replay);
        let track = HealthTrack::build(
            state.judge().expect("judged"),
            state.timeline(),
            &map.breaks,
            map.format_version,
            Mods::new(0),
            ruleset,
        );
        for t in (0..21_000).step_by(100) {
            let h = track.at(f64::from(t));
            assert!(
                (0.0..=1.0).contains(&h),
                "{ruleset:?} left the range at {t}ms: {h}"
            );
        }
    }
}

#[test]
fn a_replay_without_a_graph_still_gets_a_bar() {
    let map = beatmap(&stream(5.0, 60));
    let replay = replay_with(played_perfectly(60), 0);
    assert!(replay.life_bar.is_empty(), "the premise of this test");

    let state = GameState::new(&map, &replay);
    let bar = state.health_at(8_000.0).expect("a bar was computed");
    assert!((0.0..=1.0).contains(&bar), "{bar}");
}

#[test]
fn the_model_draws_even_when_the_replay_brought_a_graph() {
    let map = beatmap(&stream(5.0, 60));
    let mut replay = replay_with(played_perfectly(60), 0);
    replay.life_bar = "0|1,10000|0.1,20000|1".into();

    let state = GameState::new(&map, &replay);
    let bar = state.health_at(10_000.0).expect("a bar");
    assert!(
        bar > 0.5,
        "the model should not believe the graph here: {bar}"
    );

    assert!(
        state
            .recorded_health()
            .iter()
            .any(|&(at, v)| (at - 10_000.0).abs() < 1.0 && (v - 0.1).abs() < 1e-3),
        "{:?}",
        state.recorded_health()
    );
}

#[test]
fn halftime_drains_more_gently_per_millisecond() {
    let map = beatmap(&stream(7.0, 60));
    let dropped = played_dropping(60, 3);

    let mut floors = Vec::new();
    for mods in [0, bits::HALF_TIME] {
        let replay = replay_with(dropped.clone(), mods);
        let state = GameState::new(&map, &replay);
        let track = HealthTrack::build(
            state.judge().expect("judged"),
            state.timeline(),
            &map.breaks,
            map.format_version,
            Mods::new(mods),
            Ruleset::STABLE,
        );
        floors.push(
            (0..21_000)
                .step_by(100)
                .map(|t| track.at(f64::from(t)))
                .fold(f32::INFINITY, f32::min),
        );
    }
    assert!(
        floors[1] > floors[0],
        "HalfTime bottomed at {} against {} without it",
        floors[1],
        floors[0]
    );
}

#[test]
fn easy_carries_two_spare_lives() {
    let map = beatmap(&stream(5.0, 200));
    let died = |mods: u32| {
        let replay = replay_with(Vec::new(), mods);
        let state = GameState::new(&map, &replay);
        HealthTrack::build(
            state.judge().expect("judged"),
            state.timeline(),
            &map.breaks,
            map.format_version,
            Mods::new(mods),
            Ruleset::STABLE,
        )
        .failed_at()
    };

    let plain = died(0).expect("hitting nothing should be fatal");
    let easy = died(dossier_replay::bits::EASY).expect("even three lives run out");
    assert!(
        easy > plain,
        "Easy should carry the player past the first death: {easy} against {plain}"
    );
}
