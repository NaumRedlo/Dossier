use dossier_beatmap::{Beatmap, BeatmapError, CurveType, ObjectKind};

fn map(body: &str) -> String {
    format!("osu file format v14\n\n{body}")
}

#[test]
fn reads_metadata_and_difficulty() {
    let text = map("
[General]
AudioFilename: audio.mp3
Mode: 0
StackLeniency: 0.5

[Metadata]
Title:Blue Zenith
Artist:xi
Creator:Asphyxia
Version:FOUR DIMENSIONS
BeatmapID:658127
BeatmapSetID:292301

[Difficulty]
HPDrainRate:5
CircleSize:4
OverallDifficulty:9
ApproachRate:9.5
SliderMultiplier:1.8
SliderTickRate:1
");
    let m = Beatmap::parse(&text).unwrap();

    assert_eq!(m.format_version, 14);
    assert_eq!(m.metadata.title, "Blue Zenith");
    assert_eq!(m.metadata.version, "FOUR DIMENSIONS");
    assert_eq!(m.metadata.beatmap_id, Some(658_127));
    assert_eq!(m.audio_filename, "audio.mp3");
    assert_eq!(m.stack_leniency, 0.5);
    assert_eq!(m.difficulty.circle_size, 4.0);
    assert_eq!(m.difficulty.approach_rate, 9.5);
}

#[test]
fn a_file_without_the_format_header_is_rejected() {
    assert!(matches!(
        Beatmap::parse("[Metadata]\nTitle:whatever\n"),
        Err(BeatmapError::MissingHeader)
    ));
}

#[test]
fn comments_and_blank_lines_are_ignored() {
    let text = map("
// leading comment
[Metadata]

Title:Song   // trailing comment
");
    assert_eq!(Beatmap::parse(&text).unwrap().metadata.title, "Song");
}

#[test]
fn crlf_line_endings_parse() {
    let text = "osu file format v14\r\n\r\n[Metadata]\r\nTitle:Song\r\n";
    assert_eq!(Beatmap::parse(text).unwrap().metadata.title, "Song");
}

#[test]
fn missing_approach_rate_falls_back_to_overall_difficulty() {
    let text = "osu file format v7\n\n[Difficulty]\nOverallDifficulty:8\n";
    let m = Beatmap::parse(text).unwrap();
    assert_eq!(m.difficulty.approach_rate, 8.0);
}

#[test]
fn an_authored_approach_rate_wins_over_the_fallback() {
    let text = map("[Difficulty]\nOverallDifficulty:8\nApproachRate:4\n");
    assert_eq!(Beatmap::parse(&text).unwrap().difficulty.approach_rate, 4.0);
}

#[test]
fn derives_preempt_and_hit_windows() {
    let text = map("[Difficulty]\nApproachRate:5\nOverallDifficulty:5\nCircleSize:5\n");
    let d = Beatmap::parse(&text).unwrap().difficulty;

    assert_eq!(d.preempt_ms(), 1200.0);
    assert_eq!(d.hit_window_300(), 50.0);
    assert_eq!(d.hit_window_100(), 100.0);
    assert_eq!(d.hit_window_50(), 150.0);

    assert!((d.circle_radius() - 32.0 * 1.00041).abs() < 1e-9);

    let hard = map("[Difficulty]\nApproachRate:10\nOverallDifficulty:10\n");
    let d = Beatmap::parse(&hard).unwrap().difficulty;
    assert_eq!(d.preempt_ms(), 450.0);
    assert_eq!(d.hit_window_300(), 20.0);

    let easy = map("[Difficulty]\nApproachRate:0\nOverallDifficulty:0\n");
    let d = Beatmap::parse(&easy).unwrap().difficulty;
    assert_eq!(d.preempt_ms(), 1800.0);
    assert_eq!(d.hit_window_300(), 80.0);
}

#[test]
fn hit_windows_are_whole_milliseconds() {
    let text = map("[Difficulty]\nApproachRate:9\nOverallDifficulty:9.2\n");
    let d = Beatmap::parse(&text).unwrap().difficulty;

    assert_eq!(
        d.hit_window_300(),
        24.0,
        "24.8 truncates, it does not round"
    );
    assert_eq!(d.hit_window_100(), 66.0);
    assert_eq!(d.hit_window_50(), 108.0);
}

#[test]
fn splits_red_and_green_timing_lines() {
    let text = map("
[TimingPoints]
0,500,4,2,0,60,1,0
1000,-50,4,2,0,60,0,0
2000,-200,4,2,0,60,0,1
");
    let t = Beatmap::parse(&text).unwrap().timing;

    assert_eq!(t.uninherited.len(), 1);
    assert_eq!(t.inherited.len(), 2);
    assert_eq!(t.uninherited[0].bpm(), 120.0);
    assert_eq!(t.inherited[0].velocity, 2.0);
    assert_eq!(t.inherited[1].velocity, 0.5);
    assert!(t.inherited[1].kiai);
}

#[test]
fn a_green_line_asking_for_more_than_the_game_allows_gets_what_it_allows() {
    let text = map("
[TimingPoints]
0,500,4,2,0,60,1,0
1000,-10000,4,2,0,60,0,0
2000,-5,4,2,0,60,0,0
3000,-100,4,2,0,60,0,0
");
    let t = Beatmap::parse(&text).unwrap().timing;

    assert_eq!(t.inherited[0].velocity, 0.1, "0.01 is below the floor");
    assert_eq!(t.inherited[1].velocity, 10.0, "20 is above the ceiling");
    assert_eq!(
        t.inherited[2].velocity, 1.0,
        "and an ordinary one is untouched"
    );
}

#[test]
fn timing_lookups_take_the_latest_point_at_or_before_a_time() {
    let text = map("
[TimingPoints]
0,500,4,2,0,60,1,0
10000,250,4,2,0,60,1,0
5000,-50,4,2,0,60,0,0
");
    let t = Beatmap::parse(&text).unwrap().timing;

    assert_eq!(t.bpm_at(0.0), 120.0);
    assert_eq!(t.bpm_at(9_999.0), 120.0);
    assert_eq!(t.bpm_at(10_000.0), 240.0);
    assert_eq!(t.velocity_at(4_999.0), 1.0);
    assert_eq!(t.velocity_at(5_000.0), 2.0);
}

#[test]
fn a_red_line_resets_the_slider_velocity() {
    let text = map("
[TimingPoints]
0,500,4,2,0,60,1,0
1000,-166.6667,4,2,0,60,0,0
2000,400,4,2,0,60,1,0
");
    let t = Beatmap::parse(&text).unwrap().timing;

    assert!((t.velocity_at(1_500.0) - 0.6).abs() < 1e-6);
    assert_eq!(t.velocity_at(2_000.0), 1.0, "the red line resets it");
    assert_eq!(t.velocity_at(9_999.0), 1.0, "and it stays reset");
}

#[test]
fn a_green_line_on_the_same_beat_as_a_red_one_still_applies() {
    let text = map("
[TimingPoints]
0,500,4,2,0,60,1,0
2000,400,4,2,0,60,1,0
2000,-50,4,2,0,60,0,0
");
    let t = Beatmap::parse(&text).unwrap().timing;

    assert_eq!(t.velocity_at(2_000.0), 2.0);
}

#[test]
fn objects_and_timing_points_are_sorted_even_if_the_file_is_not() {
    let text = map("
[TimingPoints]
10000,250,4,2,0,60,1,0
0,500,4,2,0,60,1,0

[HitObjects]
500,200,3000,1,0
256,192,1000,1,0
");
    let m = Beatmap::parse(&text).unwrap();
    assert_eq!(m.timing.uninherited[0].time_ms, 0.0);
    assert_eq!(m.objects[0].time_ms, 1000.0);
}

#[test]
fn parses_the_three_object_kinds() {
    let text = map("
[HitObjects]
256,192,1000,1,0
100,100,2000,2,0,B|200:200|300:100,1,140
0,0,3000,12,0,5000
");
    let objects = Beatmap::parse(&text).unwrap().objects;
    assert_eq!(objects.len(), 3);

    assert!(objects[0].is_circle());
    assert_eq!(objects[0].pos.x, 256.0);

    assert!(objects[1].is_slider());
    let ObjectKind::Slider(slider) = &objects[1].kind else {
        panic!("expected a slider");
    };
    assert_eq!(slider.curve_type, CurveType::Bezier);

    assert_eq!(slider.points.len(), 3);
    assert_eq!(slider.points[0].x, 100.0);
    assert_eq!(slider.points[2].y, 100.0);
    assert_eq!(slider.slides, 1);
    assert_eq!(slider.length, 140.0);

    assert!(objects[2].is_spinner());
    assert_eq!(objects[2].end_time_ms(), 5000.0);
    assert!(objects[2].new_combo);
}

#[test]
fn recognises_every_curve_type() {
    for (letter, expected) in [
        ("B", CurveType::Bezier),
        ("C", CurveType::Catmull),
        ("L", CurveType::Linear),
        ("P", CurveType::PerfectCircle),
    ] {
        let text = map(&format!(
            "[HitObjects]\n0,0,1000,2,0,{letter}|100:100,1,70\n"
        ));
        let objects = Beatmap::parse(&text).unwrap().objects;
        let ObjectKind::Slider(s) = &objects[0].kind else {
            panic!("expected a slider");
        };
        assert_eq!(s.curve_type, expected, "curve {letter}");
    }
}

#[test]
fn repeat_sliders_report_their_slide_count() {
    let text = map("[HitObjects]\n0,0,1000,2,0,L|100:0,3,100\n");
    let objects = Beatmap::parse(&text).unwrap().objects;
    let ObjectKind::Slider(s) = &objects[0].kind else {
        panic!("expected a slider");
    };
    assert_eq!(s.slides, 3);
}

#[test]
fn new_combo_flag_is_read_off_the_type_field() {
    let text = map("
[HitObjects]
0,0,1000,1,0
0,0,2000,5,0
");
    let objects = Beatmap::parse(&text).unwrap().objects;
    assert!(!objects[0].new_combo);
    assert!(objects[1].new_combo);
}

#[test]
fn drain_time_spans_first_object_to_last_end() {
    let text = map("
[HitObjects]
0,0,1000,1,0
0,0,3000,12,0,5000
");
    assert_eq!(Beatmap::parse(&text).unwrap().drain_time_ms(), 4000.0);
}

#[test]
fn picks_up_the_background_but_not_a_video() {
    let text = map("
[Events]
//Background and Video events
1,0,\"intro.mp4\",0,0
0,0,\"bg.jpg\",0,0
");
    assert_eq!(
        Beatmap::parse(&text).unwrap().background.as_deref(),
        Some("bg.jpg")
    );
}

#[test]
fn a_map_without_events_has_no_background() {
    assert_eq!(
        Beatmap::parse(&map("[HitObjects]\n")).unwrap().background,
        None
    );
}

#[test]
fn malformed_records_name_their_line() {
    let text = map("[HitObjects]\n256,192,1000,1,0\nnot-an-object\n");
    let Err(BeatmapError::BadHitObject { line, .. }) = Beatmap::parse(&text) else {
        panic!("expected a hit-object error");
    };
    assert_eq!(line, 5);

    let text = map("[TimingPoints]\nbroken\n");
    assert!(matches!(
        Beatmap::parse(&text),
        Err(BeatmapError::BadTimingPoint { .. })
    ));
}

#[test]
fn an_empty_map_parses_to_empty_rather_than_failing() {
    let m = Beatmap::parse("osu file format v14\n").unwrap();
    assert_eq!(m.object_count(), 0);
    assert_eq!(m.drain_time_ms(), 0.0);
    assert_eq!(m.timing.bpm_at(0.0), 0.0);
    assert_eq!(m.timing.velocity_at(0.0), 1.0);
}

#[test]
fn breaks_are_read_from_the_events_section() {
    let text = map("
[Events]
0,0,\"bg.jpg\",0,0
2,12000,18000
Break,30000,34000
1,0,\"clip.mp4\"
");
    let m = Beatmap::parse(&text).unwrap();
    assert_eq!(m.background.as_deref(), Some("bg.jpg"));
    assert_eq!(m.breaks, vec![(12_000.0, 18_000.0), (30_000.0, 34_000.0)]);
}

#[test]
fn a_break_that_ends_before_it_starts_is_not_a_break() {
    let text = map("[Events]\n2,9000,9000\n2,5000,1000\n");
    assert!(Beatmap::parse(&text).unwrap().breaks.is_empty());
}

#[test]
fn kiai_spans_read_both_kinds_of_timing_point() {
    let text = map("
[TimingPoints]
0,500,4,2,0,60,1,0
10000,500,4,2,0,60,1,1
20000,-100,4,2,0,60,0,0
");
    let m = Beatmap::parse(&text).unwrap();
    assert_eq!(m.timing.kiai_spans(), vec![(10_000.0, 20_000.0)]);
}

#[test]
fn a_kiai_the_map_never_ends_runs_to_infinity() {
    let text = map("[TimingPoints]\n0,500,4,2,0,60,1,0\n8000,-100,4,2,0,60,0,1\n");
    let spans = Beatmap::parse(&text).unwrap().timing.kiai_spans();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].0, 8_000.0);
    assert!(spans[0].1.is_infinite());
}

#[test]
fn kiai_turned_off_where_it_began_is_not_a_section() {
    let text = map("
[TimingPoints]
0,500,4,2,0,60,1,0
5000,500,4,2,0,60,1,1
5000,-100,4,2,0,60,0,0
");
    assert!(Beatmap::parse(&text)
        .unwrap()
        .timing
        .kiai_spans()
        .is_empty());
}

#[test]
fn a_map_with_no_kiai_has_no_spans() {
    let text = map("[TimingPoints]\n0,500,4,2,0,60,1,0\n");
    assert!(Beatmap::parse(&text)
        .unwrap()
        .timing
        .kiai_spans()
        .is_empty());
}
