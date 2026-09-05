use dossier_beatmap::Beatmap;

fn difficulty(body: &str) -> dossier_beatmap::Difficulty {
    Beatmap::parse(&format!("osu file format v14\n\n[Difficulty]\n{body}\n"))
        .expect("test map should parse")
        .difficulty
}

#[test]
fn approach_rate_sets_preempt_by_the_documented_table() {
    for (ar, preempt) in [
        (0.0, 1800.0),
        (2.0, 1560.0),
        (5.0, 1200.0),
        (8.0, 750.0),
        (9.0, 600.0),
        (9.6, 510.0),
        (10.0, 450.0),
    ] {
        let d = difficulty(&format!("ApproachRate:{ar}"));
        assert!(
            (d.preempt_ms() - preempt).abs() < 1e-9,
            "AR {ar}: {} against {preempt}",
            d.preempt_ms()
        );
    }
}

#[test]
fn an_approach_rate_past_ten_keeps_going() {
    let d = difficulty("ApproachRate:11");
    assert!((d.preempt_ms() - 300.0).abs() < 1e-9, "{}", d.preempt_ms());
}

#[test]
fn the_fade_in_is_two_thirds_of_preempt() {
    for (ar, fade) in [(0.0, 1200.0), (5.0, 800.0), (10.0, 300.0)] {
        let d = difficulty(&format!("ApproachRate:{ar}"));
        assert!(
            (d.fade_in_ms() - fade).abs() < 1e-9,
            "AR {ar}: {} against {fade}",
            d.fade_in_ms()
        );
    }
}

#[test]
fn overall_difficulty_sets_the_windows_by_the_documented_table() {
    for (od, windows) in [
        (0.0, (80.0, 140.0, 200.0)),
        (3.5, (59.0, 112.0, 165.0)),
        (4.25, (54.0, 106.0, 157.0)),
        (6.5, (41.0, 88.0, 135.0)),
        (8.5, (29.0, 72.0, 115.0)),
        (9.2, (24.0, 66.0, 108.0)),
        (9.3, (24.0, 65.0, 107.0)),
        (10.0, (20.0, 60.0, 100.0)),
    ] {
        let d = difficulty(&format!("OverallDifficulty:{od}"));
        assert_eq!(
            (d.hit_window_300(), d.hit_window_100(), d.hit_window_50()),
            windows,
            "OD {od}"
        );
    }
}

#[test]
fn od_nine_point_three_gives_a_hundred_and_seven() {
    let d = difficulty("OverallDifficulty:9.3");
    assert_eq!(d.hit_window_50(), 107.0);
    assert_eq!(78276.0 + d.hit_window_50(), 78383.0);
}

#[test]
fn circle_size_sets_the_radius() {
    for (cs, plain) in [
        (0.0, 54.4),
        (2.0, 45.44),
        (4.0, 36.48),
        (5.0, 32.0),
        (7.0, 23.04),
        (10.0, 9.6),
    ] {
        let d = difficulty(&format!("CircleSize:{cs}"));
        let radius = plain * 1.00041;
        assert!(
            (d.circle_radius() - radius).abs() < 1e-9,
            "CS {cs}: {} against {radius}",
            d.circle_radius()
        );
    }
}

#[test]
fn the_rounding_allowance_is_under_a_pixel_and_never_zero() {
    for cs in [0.0, 4.0, 5.0, 10.0] {
        let d = difficulty(&format!("CircleSize:{cs}"));
        let plain = 54.4 - 4.48 * cs;
        let allowance = d.circle_radius() - plain;
        assert!(allowance > 0.0, "CS {cs} lost the allowance");
        assert!(allowance < 1.0, "CS {cs}: {allowance} is more than a pixel");
    }
}

#[test]
fn hard_rock_scales_every_stat_and_caps_at_ten() {
    let d =
        difficulty("HPDrainRate:5\nCircleSize:4\nOverallDifficulty:6\nApproachRate:7").hard_rock();
    assert_eq!(d.hp_drain, 7.0);
    assert!((d.circle_size - 5.2).abs() < 1e-9, "{}", d.circle_size);
    assert!((d.overall_difficulty - 8.4).abs() < 1e-9);
    assert!((d.approach_rate - 9.8).abs() < 1e-9);

    let capped =
        difficulty("HPDrainRate:9\nCircleSize:9\nOverallDifficulty:9\nApproachRate:9").hard_rock();
    assert_eq!(capped.hp_drain, 10.0);
    assert_eq!(capped.overall_difficulty, 10.0);
    assert_eq!(capped.approach_rate, 10.0);
    assert!(
        (capped.circle_size - 10.0).abs() < 1e-9,
        "CS 9 × 1.3 is 11.7, capped: {}",
        capped.circle_size
    );
}

#[test]
fn easy_halves_every_stat() {
    let d = difficulty("HPDrainRate:5\nCircleSize:4\nOverallDifficulty:7\nApproachRate:9").easy();
    assert_eq!(
        (
            d.hp_drain,
            d.circle_size,
            d.overall_difficulty,
            d.approach_rate
        ),
        (2.5, 2.0, 3.5, 4.5)
    );
}

#[test]
fn neither_mod_touches_the_slider_settings() {
    let d = difficulty("SliderMultiplier:1.8\nSliderTickRate:2");
    for scaled in [d.hard_rock(), d.easy()] {
        assert_eq!(scaled.slider_multiplier, 1.8);
        assert_eq!(scaled.slider_tick_rate, 2.0);
    }
}
