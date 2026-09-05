use dossier_beatmap::Beatmap;
use dossier_render::{Effects, Layout, Scene, Skin};
use dossier_replay::{bits, Mods};
use dossier_sim::GameState;

fn beatmap(body: &str) -> Beatmap {
    Beatmap::parse(&format!("osu file format v14\n\n{body}")).expect("test map should parse")
}

const ONE_CIRCLE: &str = "
[Difficulty]
CircleSize:5
ApproachRate:5

[HitObjects]
256,192,5000,1,0
";

fn drawn(map: &Beatmap, time_ms: f64) -> usize {
    drawn_with(map, time_ms, Mods::default())
}

fn drawn_with(map: &Beatmap, time_ms: f64, mods: Mods) -> usize {
    let state = GameState::from_beatmap(map, mods);
    let skin = Skin::with_combo_colours(map.combo_colours());
    let background = skin.background.to_color_u8();
    let scene = Scene::new(&state, skin);
    let layout = Layout::new(320, 240);
    scene
        .frame(time_ms, &layout)
        .pixels()
        .iter()
        .filter(|p| {
            p.red() != background.red()
                || p.green() != background.green()
                || p.blue() != background.blue()
        })
        .count()
}

#[test]
fn nothing_is_drawn_before_a_note_spawns() {
    let map = beatmap(ONE_CIRCLE);
    assert_eq!(drawn(&map, 3700.0), 0);
}

#[test]
fn a_note_is_on_screen_once_it_spawns_and_gone_after_it_resolves() {
    let map = beatmap(ONE_CIRCLE);
    assert!(drawn(&map, 4500.0) > 0, "mid-approach");
    assert!(drawn(&map, 5000.0) > 0, "due");

    assert_eq!(drawn(&map, 7000.0), 0, "long gone");
}

#[test]
fn a_note_grows_more_solid_as_it_approaches() {
    let map = beatmap(ONE_CIRCLE);
    let state = GameState::from_beatmap(&map, Mods::default());
    let scene = Scene::new(&state, Skin::with_combo_colours(map.combo_colours()));
    let layout = Layout::new(320, 240);

    let alpha_at = |t: f64| {
        let (x, y) = layout.map(dossier_beatmap::Point::CENTRE);
        let frame = scene.frame(t, &layout);
        let pixel = frame
            .pixel(x as u32, y as u32)
            .expect("the centre of the field is inside the frame");
        u32::from(pixel.red()) + u32::from(pixel.green()) + u32::from(pixel.blue())
    };
    assert!(alpha_at(3900.0) < alpha_at(5000.0));
}

#[test]
fn the_frame_matches_the_size_it_was_asked_for() {
    let map = beatmap(ONE_CIRCLE);
    let state = GameState::from_beatmap(&map, Mods::default());
    let scene = Scene::new(&state, Skin::default());
    let layout = Layout::new(640, 480);
    let frame = scene.frame(5000.0, &layout);
    assert_eq!((frame.width(), frame.height()), (640, 480));
}

#[test]
fn a_note_is_drawn_where_the_playfield_puts_it() {
    let left = beatmap("[Difficulty]\nApproachRate:5\n\n[HitObjects]\n60,192,5000,1,0\n");
    let right = beatmap("[Difficulty]\nApproachRate:5\n\n[HitObjects]\n450,192,5000,1,0\n");

    let centroid = |map: &Beatmap| {
        let state = GameState::from_beatmap(map, Mods::default());
        let skin = Skin::default();
        let background = skin.background.to_color_u8();
        let scene = Scene::new(&state, skin);
        let layout = Layout::new(320, 240);
        let frame = scene.frame(5000.0, &layout);
        let (mut sum, mut count) = (0u64, 0u64);
        for (i, p) in frame.pixels().iter().enumerate() {
            if p.red() != background.red() || p.blue() != background.blue() {
                sum += (i as u64) % 320;
                count += 1;
            }
        }
        sum as f64 / count.max(1) as f64
    };

    assert!(centroid(&left) < centroid(&right));
}

#[test]
fn the_palette_advances_on_every_new_combo() {
    let map = beatmap(
        "
[Difficulty]
ApproachRate:5

[Colours]
Combo1 : 255,0,0
Combo2 : 0,0,255

[HitObjects]
150,192,5000,5,0
350,192,5000,5,0
",
    );
    assert_eq!(map.combo_colours().len(), 2);

    let state = GameState::from_beatmap(&map, Mods::default());
    let scene = Scene::new(&state, Skin::with_combo_colours(map.combo_colours()));
    let layout = Layout::new(320, 240);
    let frame = scene.frame(5000.0, &layout);

    let sample = |x: f64| {
        let (px, py) = layout.map(dossier_beatmap::Point { x, y: 192.0 });
        let p = frame.pixel(px as u32, py as u32).expect("inside the frame");
        (p.red(), p.blue())
    };
    let (first_red, first_blue) = sample(150.0);
    let (second_red, second_blue) = sample(350.0);
    assert!(first_red > first_blue, "first combo is the red one");
    assert!(second_blue > second_red, "second combo is the blue one");
}

fn font() -> dossier_render::Font {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/fonts/TorusNotched-Bold.ttf"
    );
    let bytes = std::fs::read(path).expect("the repo ships this font");
    dossier_render::Font::from_bytes(&bytes).expect("and it parses")
}

fn replay_over(frames: Vec<dossier_replay::ReplayFrame>) -> dossier_replay::Replay {
    dossier_replay::Replay {
        mode: dossier_replay::GameMode::Standard,
        game_version: 20_260_101,
        beatmap_hash: String::new(),
        player: "tester".into(),
        replay_hash: String::new(),
        hits: Default::default(),
        score: 0,
        max_combo: 0,
        perfect_combo: false,
        mods: Mods::default(),
        life_bar: String::new(),
        timestamp_ticks: 0,
        online_score_id: 0,
        target_practice_accuracy: None,
        frames,
        rng_seed: None,
        score_info: None,
    }
}

fn corner_ink(frame: &tiny_skia::Pixmap, right: bool, bottom: bool) -> usize {
    let (w, h) = (frame.width(), frame.height());
    let xs = if right { w * 2 / 3..w } else { 0..w / 3 };
    let ys = if bottom { h * 4 / 5..h } else { 0..h / 5 };
    let mut count = 0;
    for y in ys {
        for x in xs.clone() {
            let p = frame.pixel(x, y).expect("inside the frame");
            if p.red() > 60 && p.green() > 60 && p.blue() > 60 {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn a_map_with_no_replay_shows_no_score() {
    let map = beatmap(ONE_CIRCLE);
    let state = GameState::from_beatmap(&map, Mods::default());
    let scene = Scene::new(&state, Skin::default().with_font(font()));
    let frame = scene.frame(5000.0, &Layout::new(640, 480));

    assert_eq!(corner_ink(&frame, true, false), 0, "no accuracy");
    assert_eq!(corner_ink(&frame, false, true), 0, "no combo");
}

#[test]
fn a_replay_puts_accuracy_and_combo_in_the_corners() {
    let map = beatmap(ONE_CIRCLE);
    let replay = replay_over(vec![
        dossier_replay::ReplayFrame {
            time_ms: 4990,
            x: 256.0,
            y: 192.0,
            keys: dossier_replay::Keys(0),
        },
        dossier_replay::ReplayFrame {
            time_ms: 5000,
            x: 256.0,
            y: 192.0,
            keys: dossier_replay::Keys(dossier_replay::Keys::K1),
        },
        dossier_replay::ReplayFrame {
            time_ms: 5010,
            x: 256.0,
            y: 192.0,
            keys: dossier_replay::Keys(0),
        },
    ]);
    let state = GameState::new(&map, &replay);
    let scene = Scene::new(&state, Skin::default().with_font(font()));
    let frame = scene.frame(5200.0, &Layout::new(640, 480));

    assert!(corner_ink(&frame, true, false) > 0, "accuracy, top right");
    assert!(corner_ink(&frame, false, true) > 0, "combo, bottom left");
}

#[test]
fn without_a_font_the_play_is_still_drawn() {
    let map = beatmap(ONE_CIRCLE);
    assert!(drawn(&map, 5000.0) > 0);
}

#[test]
fn numbers_measure_wider_the_more_digits_they_have() {
    let font = font();
    assert!(font.width("1", 40.0) < font.width("11", 40.0));
    assert!(font.width("999", 40.0) < font.width("1000", 40.0));
    assert!(font.digit_height(40.0) > 0.0);
}

#[test]
fn the_combo_number_restarts_at_one_in_a_new_combo() {
    let map = beatmap(
        "
[Difficulty]
CircleSize:5
ApproachRate:5

[HitObjects]
150,192,5000,5,0
250,192,5100,1,0
350,192,5200,5,0
",
    );
    let state = GameState::from_beatmap(&map, Mods::default());
    let scene = Scene::new(&state, Skin::default().with_font(font()));
    let layout = Layout::new(640, 480);
    let frame = scene.frame(4900.0, &layout);

    let ink_on = |x: f64| {
        let (cx, cy) = layout.map(dossier_beatmap::Point { x, y: 192.0 });
        let mut count = 0;
        for dy in -12i32..12 {
            for dx in -12i32..12 {
                let p = frame
                    .pixel((cx as i32 + dx) as u32, (cy as i32 + dy) as u32)
                    .expect("inside the frame");
                if p.red() > 200 && p.green() > 200 && p.blue() > 200 {
                    count += 1;
                }
            }
        }
        count
    };
    assert_eq!(ink_on(150.0), ink_on(350.0), "both are a 1");
    assert_ne!(ink_on(150.0), ink_on(250.0), "the middle one is a 2");
}

fn repeating_slider(slides: u32) -> Beatmap {
    beatmap(&format!(
        "
[Difficulty]
CircleSize:5
ApproachRate:5
SliderMultiplier:1.4
SliderTickRate:1

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
100,192,1000,2,0,L|240:192,{slides},140
"
    ))
}

fn white_ink_at(map: &Beatmap, time_ms: f64, x: f64, y: f64) -> usize {
    let state = GameState::from_beatmap(map, Mods::default());
    let scene = Scene::new(&state, Skin::default());
    let layout = Layout::new(640, 480);
    let frame = scene.frame(time_ms, &layout);
    let (cx, cy) = layout.map(dossier_beatmap::Point { x, y });

    let mut count = 0;
    for dy in -6i32..6 {
        for dx in -6i32..6 {
            let Some(p) = frame.pixel((cx as i32 + dx) as u32, (cy as i32 + dy) as u32) else {
                continue;
            };
            if p.red() > 230 && p.green() > 230 && p.blue() > 230 {
                count += 1;
            }
        }
    }
    count
}

fn ink_at(map: &Beatmap, time_ms: f64, x: f64, y: f64) -> usize {
    let state = GameState::from_beatmap(map, Mods::default());
    let scene = Scene::new(&state, Skin::default());
    let layout = Layout::new(640, 480);
    let frame = scene.frame(time_ms, &layout);
    let (cx, cy) = layout.map(dossier_beatmap::Point { x, y });

    let background = Skin::default().background;
    let level = |c: f32| (c * 255.0).round() as i32;
    let (br, bg, bb) = (
        level(background.red()),
        level(background.green()),
        level(background.blue()),
    );

    let mut count = 0;
    for dy in -6i32..6 {
        for dx in -6i32..6 {
            let Some(p) = frame.pixel((cx as i32 + dx) as u32, (cy as i32 + dy) as u32) else {
                continue;
            };
            let off = (i32::from(p.red()) - br).abs()
                + (i32::from(p.green()) - bg).abs()
                + (i32::from(p.blue()) - bb).abs();

            if off > 12 {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn a_slider_that_never_turns_gets_no_arrow() {
    let map = repeating_slider(1);
    assert_eq!(white_ink_at(&map, 1200.0, 240.0, 192.0), 0);
}

#[test]
fn a_repeating_slider_marks_the_end_it_is_heading_for() {
    let map = repeating_slider(2);

    assert!(
        white_ink_at(&map, 1200.0, 240.0, 192.0) > 0,
        "arrow at the tail"
    );
    assert_eq!(
        white_ink_at(&map, 1200.0, 100.0, 192.0),
        0,
        "and not at the head"
    );
}

#[test]
fn the_arrow_is_up_before_the_slider_even_starts() {
    let turning = white_ink_at(&repeating_slider(2), 700.0, 240.0, 192.0);
    let plain = white_ink_at(&repeating_slider(1), 700.0, 240.0, 192.0);
    assert!(
        turning > plain,
        "the arrow is up on the approach: {turning} against {plain} with no turn"
    );
}

#[test]
fn a_slider_is_whole_from_the_moment_it_appears() {
    let map = repeating_slider(1);
    assert!(
        ink_at(&map, -100.0, 240.0, 192.0) > 0,
        "the tail is there as soon as the slider is"
    );
}

#[test]
fn the_arrow_moves_to_the_other_end_after_a_turn() {
    let map = repeating_slider(3);

    assert!(
        white_ink_at(&map, 1700.0, 100.0, 192.0) > 0,
        "arrow at the head"
    );
    assert_eq!(
        white_ink_at(&map, 1700.0, 240.0, 192.0),
        0,
        "no longer at the tail"
    );
}

#[test]
fn the_last_traversal_has_nothing_left_to_point_at() {
    let map = repeating_slider(2);

    assert_eq!(white_ink_at(&map, 1700.0, 100.0, 192.0), 0);
    assert_eq!(white_ink_at(&map, 1700.0, 240.0, 192.0), 0);
}

#[test]
fn the_maps_own_combo_colour_is_what_gets_drawn() {
    let map = beatmap(
        "
[Difficulty]
ApproachRate:5

[Colours]
Combo1 : 0,255,0

[HitObjects]
256,192,5000,5,0
",
    );
    let state = GameState::from_beatmap(&map, Mods::default());
    let layout = Layout::new(320, 240);
    let at_note = |skin: Skin| {
        let frame = Scene::new(&state, skin).frame(5000.0, &layout);
        let (x, y) = layout.map(dossier_beatmap::Point::CENTRE);
        let p = frame.pixel(x as u32, y as u32).expect("inside the frame");
        (p.red(), p.green(), p.blue())
    };

    let (_, green, _) = at_note(Skin::with_combo_colours(map.combo_colours()));
    assert!(green > 200, "the map asked for green");
}

#[test]
#[ignore = "a measurement, not a check"]
fn profile_the_phases_of_a_frame() {
    use std::time::Instant;

    let map = beatmap(
        "
[Difficulty]
CircleSize:4
ApproachRate:9
SliderMultiplier:1.8

[TimingPoints]
0,300,4,2,0,60,1,0

[HitObjects]
100,100,1000,1,0
200,150,1100,2,0,L|300:150,1,180
300,200,1300,1,0
150,250,1400,1,0
250,300,1500,2,0,L|400:300,1,180
",
    );
    let state = GameState::from_beatmap(&map, Mods::default());
    let scene = Scene::new(&state, Skin::default().with_font(font()));
    let layout = Layout::new(1920, 1080);
    let mut pixmap = tiny_skia::Pixmap::new(1920, 1080).unwrap();

    let rounds = 60;
    let fill_colour = Skin::default().background;

    let mark = Instant::now();
    for _ in 0..rounds {
        pixmap.fill(fill_colour);
    }
    let filling = mark.elapsed().as_secs_f64() / f64::from(rounds) * 1000.0;

    let mark = Instant::now();
    for i in 0..rounds {
        scene.draw_into(&mut pixmap, 1000.0 + f64::from(i), &layout, None);
    }
    let whole = mark.elapsed().as_secs_f64() / f64::from(rounds) * 1000.0;

    let bare = Scene::new(&state, Skin::default());
    let mark = Instant::now();
    for i in 0..rounds {
        bare.draw_into(&mut pixmap, 1000.0 + f64::from(i), &layout, None);
    }
    let without_text = mark.elapsed().as_secs_f64() / f64::from(rounds) * 1000.0;

    println!(
        "1080p frame: {whole:.2}ms total — fill {filling:.2}ms, text {:.2}ms, shapes {:.2}ms",
        whole - without_text,
        without_text - filling
    );
}

#[test]
#[ignore = "a measurement, not a check"]
fn profile_stroking_against_filling() {
    use std::time::Instant;
    use tiny_skia::{FillRule, LineCap, LineJoin, Paint, PathBuilder, Stroke, Transform};

    let mut builder = PathBuilder::new();
    builder.move_to(200.0, 500.0);
    for i in 1..400 {
        let t = f32::from(i as u16) / 400.0;
        builder.line_to(200.0 + t * 1200.0, 500.0 + (t * 9.0).sin() * 220.0);
    }
    let path = builder.finish().unwrap();

    let stroke = Stroke {
        width: 110.0,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Default::default()
    };
    let paint = Paint {
        anti_alias: true,
        ..Default::default()
    };
    let mut pixmap = tiny_skia::Pixmap::new(1920, 1080).unwrap();
    let rounds = 100;

    let mark = Instant::now();
    for _ in 0..rounds {
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }
    let stroking = mark.elapsed().as_secs_f64() / f64::from(rounds) * 1000.0;

    let outline = tiny_skia::PathStroker::new()
        .stroke(&path, &stroke, 1.0)
        .expect("the stroker can outline this");
    let mark = Instant::now();
    for _ in 0..rounds {
        pixmap.fill_path(
            &outline,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
    let filling = mark.elapsed().as_secs_f64() / f64::from(rounds) * 1000.0;

    println!(
        "one slider body: stroke+fill {stroking:.2}ms, fill of a cached outline {filling:.2}ms \
         ({:.0}% saved)",
        (1.0 - filling / stroking) * 100.0
    );
}

#[test]
#[ignore = "a measurement, not a check"]
fn profile_filling_against_blitting() {
    use std::time::Instant;
    use tiny_skia::{
        FillRule, LineCap, LineJoin, Paint, PathBuilder, PixmapPaint, Stroke, Transform,
    };

    let mut builder = PathBuilder::new();
    builder.move_to(200.0, 500.0);
    for i in 1..400 {
        let t = f32::from(i as u16) / 400.0;
        builder.line_to(200.0 + t * 1200.0, 500.0 + (t * 9.0).sin() * 220.0);
    }
    let path = builder.finish().unwrap();
    let stroke = Stroke {
        width: 110.0,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Default::default()
    };
    let paint = Paint {
        anti_alias: true,
        ..Default::default()
    };

    let mut frame = tiny_skia::Pixmap::new(1920, 1080).unwrap();
    let rounds = 100;

    let inner = Stroke {
        width: 90.0,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Default::default()
    };
    let mark = Instant::now();
    for _ in 0..rounds {
        frame.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        frame.stroke_path(&path, &paint, &inner, Transform::identity(), None);
    }
    let both_fills = mark.elapsed().as_secs_f64() / f64::from(rounds) * 1000.0;

    let bounds = path.bounds();
    let (w, h) = (
        (bounds.width() + 120.0) as u32,
        (bounds.height() + 120.0) as u32,
    );
    let mut cached = tiny_skia::Pixmap::new(w, h).unwrap();
    let offset = Transform::from_translate(-bounds.x() + 60.0, -bounds.y() + 60.0);
    cached.stroke_path(&path, &paint, &stroke, offset, None);
    cached.stroke_path(&path, &paint, &inner, offset, None);

    let faded = PixmapPaint {
        opacity: 0.8,
        ..Default::default()
    };
    let mark = Instant::now();
    for _ in 0..rounds {
        frame.draw_pixmap(
            (bounds.x() - 60.0) as i32,
            (bounds.y() - 60.0) as i32,
            cached.as_ref(),
            &faded,
            Transform::identity(),
            None,
        );
    }
    let blitting = mark.elapsed().as_secs_f64() / f64::from(rounds) * 1000.0;

    println!(
        "one slider body: two fills {both_fills:.2}ms, blit of a cached raster {blitting:.2}ms \
         ({:.1}× faster), cache {} KiB",
        both_fills / blitting,
        (w * h * 4) / 1024
    );
    let _ = FillRule::Winding;
}

#[test]
#[ignore = "a measurement, not a check"]
fn profile_the_border_as_a_rim_instead_of_a_fill() {
    use std::time::Instant;
    use tiny_skia::{FillRule, LineCap, LineJoin, Paint, PathBuilder, Stroke, Transform};

    let mut builder = PathBuilder::new();
    builder.move_to(200.0, 500.0);
    for i in 1..400 {
        let t = f32::from(i as u16) / 400.0;
        builder.line_to(200.0 + t * 1200.0, 500.0 + (t * 9.0).sin() * 220.0);
    }
    let path = builder.finish().unwrap();
    let round = |width: f32| Stroke {
        width,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Default::default()
    };
    let paint = Paint {
        anti_alias: true,
        ..Default::default()
    };
    let mut frame = tiny_skia::Pixmap::new(1920, 1080).unwrap();
    let rounds = 100;

    let mark = Instant::now();
    for _ in 0..rounds {
        frame.stroke_path(&path, &paint, &round(110.0), Transform::identity(), None);
        frame.stroke_path(&path, &paint, &round(90.0), Transform::identity(), None);
    }
    let two_fills = mark.elapsed().as_secs_f64() / f64::from(rounds) * 1000.0;

    let body = tiny_skia::PathStroker::new()
        .stroke(&path, &round(100.0), 1.0)
        .expect("outline");
    let mark = Instant::now();
    for _ in 0..rounds {
        frame.fill_path(
            &body,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
        frame.stroke_path(&body, &paint, &round(10.0), Transform::identity(), None);
    }
    let fill_and_rim = mark.elapsed().as_secs_f64() / f64::from(rounds) * 1000.0;

    println!(
        "one slider body: two fills {two_fills:.2}ms, fill plus a rim {fill_and_rim:.2}ms \
         ({:.0}% saved)",
        (1.0 - fill_and_rim / two_fills) * 100.0
    );
}

#[test]
#[ignore = "a measurement, not a check"]
fn profile_circles_against_slider_bodies() {
    use std::time::Instant;
    use tiny_skia::{FillRule, LineCap, LineJoin, Paint, PathBuilder, Stroke, Transform};

    let mut frame = tiny_skia::Pixmap::new(1920, 1080).unwrap();
    let paint = Paint {
        anti_alias: true,
        ..Default::default()
    };
    let rounds = 200;

    let circles: Vec<_> = (0..40)
        .map(|i| {
            let angle = f32::from(i as u16) * 0.9;
            PathBuilder::from_circle(
                960.0 + angle.cos() * 400.0,
                540.0 + angle.sin() * 300.0,
                90.0 - (i % 5) as f32 * 15.0,
            )
            .unwrap()
        })
        .collect();

    let mark = Instant::now();
    for _ in 0..rounds {
        for path in &circles {
            frame.fill_path(path, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }
    let discs = mark.elapsed().as_secs_f64() / f64::from(rounds) * 1000.0;

    let mut bodies = Vec::new();
    for k in 0..2 {
        let mut builder = PathBuilder::new();
        builder.move_to(200.0, 300.0 + f32::from(k as u16) * 400.0);
        for i in 1..300 {
            let t = f32::from(i as u16) / 300.0;
            builder.line_to(
                200.0 + t * 1400.0,
                300.0 + f32::from(k as u16) * 400.0 + (t * 7.0).sin() * 150.0,
            );
        }
        bodies.push(builder.finish().unwrap());
    }
    let round = |width: f32| Stroke {
        width,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Default::default()
    };

    let mark = Instant::now();
    for _ in 0..rounds {
        for path in &bodies {
            frame.stroke_path(path, &paint, &round(110.0), Transform::identity(), None);
            frame.stroke_path(path, &paint, &round(90.0), Transform::identity(), None);
        }
    }
    let sliders = mark.elapsed().as_secs_f64() / f64::from(rounds) * 1000.0;

    println!(
        "1080p: {} circles {discs:.2}ms, 2 slider bodies {sliders:.2}ms",
        circles.len()
    );
}

#[test]
#[ignore = "a measurement, not a check"]
fn profile_a_hand_written_circle_against_the_library() {
    use std::time::Instant;
    use tiny_skia::{Color, FillRule, Paint, PathBuilder, PremultipliedColorU8, Transform};

    fn disc(pixmap: &mut tiny_skia::Pixmap, cx: f32, cy: f32, r: f32, colour: Color) {
        let (w, h) = (pixmap.width() as i32, pixmap.height() as i32);
        let (sr, sg, sb, sa) = (colour.red(), colour.green(), colour.blue(), colour.alpha());
        let pixels = pixmap.pixels_mut();

        let top = ((cy - r - 1.0).floor() as i32).max(0);
        let bottom = ((cy + r + 1.0).ceil() as i32).min(h - 1);
        for y in top..=bottom {
            let dy = y as f32 + 0.5 - cy;
            let inside = r * r - dy * dy;
            if inside <= 0.0 {
                continue;
            }
            let half = inside.sqrt();
            let left = ((cx - half - 1.0).floor() as i32).max(0);
            let right = ((cx + half + 1.0).ceil() as i32).min(w - 1);

            let solid = (half - 1.5).max(0.0);
            let row = (y * w) as usize;

            for x in left..=right {
                let dx = x as f32 + 0.5 - cx;
                let coverage = if dx.abs() <= solid {
                    1.0
                } else {
                    (r + 0.5 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0)
                };
                if coverage <= 0.0 {
                    continue;
                }
                let a = sa * coverage;
                let keep = 1.0 - a;
                let slot = row + x as usize;
                let dst = pixels[slot];
                let mix =
                    |src: f32, dst: u8| ((src * a + f32::from(dst) / 255.0 * keep) * 255.0) as u8;
                let (r8, g8, b8) = (
                    mix(sr, dst.red()),
                    mix(sg, dst.green()),
                    mix(sb, dst.blue()),
                );
                let a8 = ((a + f32::from(dst.alpha()) / 255.0 * keep) * 255.0) as u8;
                pixels[slot] =
                    PremultipliedColorU8::from_rgba(r8.min(a8), g8.min(a8), b8.min(a8), a8)
                        .unwrap_or(dst);
            }
        }
    }

    let mut frame = tiny_skia::Pixmap::new(1920, 1080).unwrap();
    let colour = Color::from_rgba8(226, 72, 72, 255);
    let paint = Paint {
        shader: tiny_skia::Shader::SolidColor(colour),
        anti_alias: true,
        ..Default::default()
    };
    let placed: Vec<_> = (0..40)
        .map(|i| {
            let angle = f32::from(i as u16) * 0.9;
            (
                960.0 + angle.cos() * 400.0,
                540.0 + angle.sin() * 300.0,
                90.0 - (i % 5) as f32 * 15.0,
            )
        })
        .collect();
    let paths: Vec<_> = placed
        .iter()
        .map(|&(x, y, r)| PathBuilder::from_circle(x, y, r).unwrap())
        .collect();
    let rounds = 200;

    let mark = Instant::now();
    for _ in 0..rounds {
        for path in &paths {
            frame.fill_path(path, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }
    let library = mark.elapsed().as_secs_f64() / f64::from(rounds) * 1000.0;

    let mark = Instant::now();
    for _ in 0..rounds {
        for &(x, y, r) in &placed {
            disc(&mut frame, x, y, r, colour);
        }
    }
    let ours = mark.elapsed().as_secs_f64() / f64::from(rounds) * 1000.0;

    println!(
        "40 circles: tiny-skia {library:.2}ms, hand-written {ours:.2}ms ({:.1}× faster)",
        library / ours
    );
}

#[test]
#[ignore = "a measurement, not a check"]
fn profile_converting_to_yuv_ourselves() {
    use std::time::Instant;

    let frame = tiny_skia::Pixmap::new(1920, 1080).unwrap();
    let (w, h) = (1920usize, 1080usize);
    let mut yuv = vec![0u8; w * h * 3 / 2];
    let rounds = 100;

    let mark = Instant::now();
    for _ in 0..rounds {
        let src = frame.data();

        let (luma, chroma) = yuv.split_at_mut(w * h);
        let (u_plane, v_plane) = chroma.split_at_mut(w * h / 4);

        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) * 4;
                let (r, g, b) = (
                    f32::from(src[i]),
                    f32::from(src[i + 1]),
                    f32::from(src[i + 2]),
                );
                luma[y * w + x] = (0.257 * r + 0.504 * g + 0.098 * b + 16.0) as u8;
                if y % 2 == 0 && x % 2 == 0 {
                    let at = (y / 2) * (w / 2) + x / 2;
                    u_plane[at] = (-0.148 * r - 0.291 * g + 0.439 * b + 128.0) as u8;
                    v_plane[at] = (0.439 * r - 0.368 * g - 0.071 * b + 128.0) as u8;
                }
            }
        }
    }
    let converting = mark.elapsed().as_secs_f64() / f64::from(rounds) * 1000.0;

    println!(
        "1080p frame: RGBA→I420 {converting:.2}ms, pipe {} KiB → {} KiB",
        w * h * 4 / 1024,
        yuv.len() / 1024
    );
}

const LONE_SLIDER: &str = "
[Difficulty]
CircleSize:4
ApproachRate:5
OverallDifficulty:5
SliderMultiplier:1.4

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
100,192,2000,2,0,L|400:192,1,300
";

#[test]
fn a_slider_does_not_change_shape_while_it_is_watched() {
    let map = beatmap(LONE_SLIDER);
    let state = GameState::from_beatmap(&map, Mods::default());
    let skin = Skin::default();
    let background = skin.background;
    let scene = Scene::new(&state, skin);
    let layout = Layout::new(640, 480);

    let reach = |t: f64| {
        let frame = scene.frame(t, &layout);
        let bg = background.to_color_u8();
        (0..frame.width())
            .filter(|&x| {
                (0..frame.height()).any(|y| {
                    frame.pixel(x, y).is_some_and(|p| {
                        p.red() != bg.red() || p.green() != bg.green() || p.blue() != bg.blue()
                    })
                })
            })
            .count() as i64
    };

    let object = &state.timeline().objects[0];
    let span = object.end_ms - object.start_ms;
    let early = reach(object.start_ms + span * 0.4);
    let late = reach(object.start_ms + span * 0.8);
    assert!(
        early > 0 && (early - late).abs() <= 4,
        "{early} against {late}"
    );
}

fn reach(frame: &tiny_skia::Pixmap, background: tiny_skia::Color) -> i64 {
    let bg = background.to_color_u8();
    (0..frame.width())
        .filter(|&x| {
            (0..frame.height()).any(|y| {
                frame.pixel(x, y).is_some_and(|p| {
                    p.red() != bg.red() || p.green() != bg.green() || p.blue() != bg.blue()
                })
            })
        })
        .count() as i64
}

#[test]
fn a_slider_retracts_behind_the_ball_when_asked() {
    let map = beatmap(LONE_SLIDER);
    let state = GameState::from_beatmap(&map, Mods::default());
    let mut skin = Skin::default();
    Effects::apply(&mut skin, "snake-out");
    let background = skin.background;
    let scene = Scene::new(&state, skin);
    let layout = Layout::new(640, 480);

    let object = &state.timeline().objects[0];
    let span = object.end_ms - object.start_ms;
    let early = reach(
        &scene.frame(object.start_ms + span * 0.4, &layout),
        background,
    );
    let late = reach(
        &scene.frame(object.start_ms + span * 0.8, &layout),
        background,
    );

    assert!(
        late + 20 < early,
        "four fifths through, most of the body is behind the ball: {late} against {early}"
    );
}

#[test]
fn a_slider_grows_out_of_its_head_when_asked() {
    let map = beatmap(LONE_SLIDER);
    let state = GameState::from_beatmap(&map, Mods::default());
    let mut skin = Skin::default();
    Effects::apply(&mut skin, "snake-in");
    let background = skin.background;
    let scene = Scene::new(&state, skin);
    let layout = Layout::new(640, 480);

    let object = &state.timeline().objects[0];
    let approach = state.timeline().objects[0].start_ms - 1200.0;
    let young = reach(&scene.frame(approach.max(0.0) + 60.0, &layout), background);
    let due = reach(&scene.frame(object.start_ms - 20.0, &layout), background);

    assert!(young > 0 && young + 20 < due, "{young} against {due}");
}

#[test]
fn neither_end_of_a_slider_moves_unless_it_is_asked_for() {
    let skin = Skin::default();
    assert!(!skin.snake_in, "growth is off by default");
    assert!(!skin.snake_out, "and so is retraction");
}

const REPEATING_SLIDER: &str = "
[Difficulty]
CircleSize:4
ApproachRate:5
OverallDifficulty:5
SliderMultiplier:1.4

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
100,192,2000,2,0,L|400:192,4,300
";

#[test]
fn a_slider_head_leaves_on_its_own_click_not_at_the_end_of_the_slider() {
    let map = beatmap(REPEATING_SLIDER);

    let frames: Vec<_> = (0..90)
        .map(|i| {
            let time_ms = 1800 + i * 20;
            dossier_replay::ReplayFrame {
                time_ms,
                x: 100.0,
                y: 192.0,
                keys: dossier_replay::Keys(u8::from(time_ms >= 2000)),
            }
        })
        .collect();
    let replay = replay_over(frames);
    let state = GameState::new(&map, &replay);
    let skin = Skin::default();
    let scene = Scene::new(&state, skin);
    let layout = Layout::new(640, 480);

    let object = &state.timeline().objects[0];
    assert!(
        object.end_ms > 5000.0,
        "the slider runs long: {}",
        object.end_ms
    );

    let radius = state.difficulty().circle_radius() * 0.98;
    let brightness = |t: f64| {
        let frame = scene.frame(t, &layout);
        let mut total = 0u32;
        for step in 0..8 {
            let angle = f64::from(step) * std::f64::consts::TAU / 8.0;
            let at = dossier_beatmap::Point {
                x: object.pos.x + radius * angle.cos(),
                y: object.pos.y + radius * angle.sin(),
            };
            let (x, y) = layout.map(at);
            let p = frame.pixel(x as u32, y as u32).expect("inside the frame");
            total += u32::from(p.red()) + u32::from(p.green()) + u32::from(p.blue());
        }
        total
    };

    let clicked = brightness(2000.0);
    let mid_slide = brightness(2600.0);
    assert!(
        mid_slide < clicked,
        "the head should be gone by mid-slide: {mid_slide} against {clicked} at the click"
    );
}

#[test]
fn the_balls_core_grows_to_fill_it_as_the_slider_runs_out() {
    let map = beatmap(LONE_SLIDER);
    let state = GameState::from_beatmap(&map, Mods::default());
    let scene = Scene::new(&state, Skin::default());
    let layout = Layout::new(640, 480);

    let object = &state.timeline().objects[0];
    let span = object.end_ms - object.start_ms;
    let offset = state.difficulty().circle_radius() * 0.6;

    let at_core_edge = |fraction: f64| {
        let t = object.start_ms + span * fraction;
        let ball = object.ball_at(t).expect("the ball is on the path");
        let probe = dossier_beatmap::Point {
            x: ball.x + offset,
            y: ball.y,
        };
        let (x, y) = layout.map(probe);
        let p = scene
            .frame(t, &layout)
            .pixel(x as u32, y as u32)
            .expect("inside");
        u32::from(p.red()) + u32::from(p.green()) + u32::from(p.blue())
    };

    let early = at_core_edge(0.15);
    let late = at_core_edge(0.9);
    assert!(
        late > early,
        "the core should have reached this point by the end: {late} against {early}"
    );
}

#[test]
fn a_tick_still_waits_for_its_own_moment() {
    let map = beatmap(LONE_SLIDER);
    let state = GameState::from_beatmap(&map, Mods::default());
    let skin = Skin::default();
    let background = skin.background;
    let scene = Scene::new(&state, skin);
    let layout = Layout::new(640, 480);

    let object = &state.timeline().objects[0];
    let far_tick = *object.tick_times().last().expect("this slider has ticks");
    let at = object.ball_at(far_tick).expect("on the path");
    let (x, y) = layout.map(at);
    let bg = background.to_color_u8();
    let lit = |t: f64| {
        let frame = scene.frame(t, &layout);
        let p = frame.pixel(x as u32, y as u32).expect("inside the frame");
        i32::from(p.red()) - i32::from(bg.red()) + i32::from(p.green()) - i32::from(bg.green())
            + i32::from(p.blue())
            - i32::from(bg.blue())
    };
    assert!(
        lit(far_tick - 2000.0) < lit(far_tick - 50.0),
        "{} against {}",
        lit(far_tick - 2000.0),
        lit(far_tick - 50.0)
    );
}

const THRICE_SLIDER: &str = "
[Difficulty]
CircleSize:4
ApproachRate:5
OverallDifficulty:5
SliderMultiplier:1.4

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
100,192,2000,2,0,L|400:192,3,300
";

const TWICE_SLIDER: &str = "
[Difficulty]
CircleSize:4
ApproachRate:5
OverallDifficulty:5
SliderMultiplier:1.4

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
100,192,2000,2,0,L|400:192,2,300
";

#[test]
fn both_ends_keep_an_arrow_while_both_still_have_a_turn_coming() {
    let ink_at_head = |source: &str| {
        let map = beatmap(source);
        let state = GameState::from_beatmap(&map, Mods::default());
        let scene = Scene::new(&state, Skin::default());
        let layout = Layout::new(640, 480);
        let object = &state.timeline().objects[0];

        let t = object.start_ms + (object.end_ms - object.start_ms) / 3.0 + 180.0;
        let frame = scene.frame(t, &layout);
        let (x, y) = layout.map(object.pos);

        let mut count = 0;
        for dy in -20i32..=20 {
            for dx in -20i32..=20 {
                let p = frame
                    .pixel((x as i32 + dx) as u32, (y as i32 + dy) as u32)
                    .expect("inside the frame");
                if p.red() > 230 && p.green() > 230 && p.blue() > 230 {
                    count += 1;
                }
            }
        }
        count
    };

    let (three, two) = (ink_at_head(THRICE_SLIDER), ink_at_head(TWICE_SLIDER));
    assert!(
        three > two,
        "three slides turn at the head as well, so that end carries an arrow too: {three} vs {two}"
    );
}

#[test]
fn the_combo_number_goes_the_instant_the_note_is_judged() {
    let map = beatmap(ONE_CIRCLE);
    let state = GameState::from_beatmap(&map, Mods::default());
    let scene = Scene::new(&state, Skin::default().with_font(font()));
    let layout = Layout::new(640, 480);
    let object = &state.timeline().objects[0];

    let (x, y) = layout.map(object.pos);
    let pale_at = |t: f64| {
        let frame = scene.frame(t, &layout);
        let mut count = 0;
        for dy in -8i32..=8 {
            for dx in -8i32..=8 {
                let p = frame
                    .pixel((x as i32 + dx) as u32, (y as i32 + dy) as u32)
                    .expect("inside the frame");
                if p.red() > 200 && p.green() > 200 && p.blue() > 200 {
                    count += 1;
                }
            }
        }
        count
    };

    let resolved = object.start_ms + state.difficulty().hit_window_50();
    assert!(
        pale_at(object.start_ms) > 0,
        "the number is up while it is a target"
    );
    assert_eq!(
        pale_at(resolved + 20.0),
        0,
        "and gone the moment it is judged"
    );
}

#[test]
fn the_arrow_takes_the_skins_colour_not_the_one_it_was_drawn_in() {
    let map = beatmap(THRICE_SLIDER);
    let state = GameState::from_beatmap(&map, Mods::default());
    let skin = Skin::default();
    let want = skin.circle_border.to_color_u8();
    let scene = Scene::new(&state, skin);
    let layout = Layout::new(640, 480);

    let object = &state.timeline().objects[0];
    let path = match &object.kind {
        dossier_sim::TimedKind::Slider { path, .. } => path,
        _ => unreachable!("this map is one slider"),
    };
    let (x, y) = layout.map(path.position_at(1.0).expect("the slider has an end"));
    let frame = scene.frame(object.start_ms + 200.0, &layout);

    let mut matched = 0;
    for dy in -14i32..=14 {
        for dx in -14i32..=14 {
            let p = frame
                .pixel((x as i32 + dx) as u32, (y as i32 + dy) as u32)
                .expect("inside the frame");
            if p.red() == want.red() && p.green() == want.green() && p.blue() == want.blue() {
                matched += 1;
            }
        }
    }
    assert!(
        matched > 40,
        "the arrow is drawn in the skin's colour: {matched} pixels"
    );
}

const BREAK_MAP_TIMED: &str = "
[Difficulty]
CircleSize:5
ApproachRate:5

[TimingPoints]
0,500,4,2,0,60,1,0

[Events]
2,3000,9000

[HitObjects]
100,100,2000,1,0
400,300,12000,1,0
";

#[test]
fn the_break_arrows_pulse_on_the_map_s_own_beat() {
    let map = beatmap(BREAK_MAP_TIMED);
    let state = GameState::from_beatmap(&map, Mods::default());
    let scene = Scene::new(&state, Skin::default());
    let layout = Layout::new(640, 480);

    let glow = |t: f64| {
        scene
            .frame(t, &layout)
            .pixels()
            .iter()
            .map(|p| u32::from(p.red()) + u32::from(p.green()) + u32::from(p.blue()))
            .sum::<u32>()
    };

    let on_beat = glow(8500.0);
    let between = glow(8980.0);
    assert!(
        on_beat > between,
        "brightest on the beat: {on_beat} against {between} just before the next"
    );
}

const BREAK_MAP: &str = "
[Difficulty]
CircleSize:5
ApproachRate:5

[Events]
2,3000,9000

[HitObjects]
100,100,2000,1,0
400,300,12000,1,0
";

#[test]
fn a_break_puts_arrows_up_before_the_map_resumes() {
    let map = beatmap(BREAK_MAP);
    assert_eq!(map.breaks, vec![(3000.0, 9000.0)], "the break parsed");

    let state = GameState::from_beatmap(&map, Mods::default());
    let skin = Skin::default();
    let background = skin.background.to_color_u8();
    let scene = Scene::new(&state, skin);
    let layout = Layout::new(640, 480);

    let ink = |t: f64| {
        scene
            .frame(t, &layout)
            .pixels()
            .iter()
            .filter(|p| {
                p.red() != background.red()
                    || p.green() != background.green()
                    || p.blue() != background.blue()
            })
            .count()
    };

    let glow = |t: f64| {
        scene
            .frame(t, &layout)
            .pixels()
            .iter()
            .map(|p| u32::from(p.red()) + u32::from(p.green()) + u32::from(p.blue()))
            .sum::<u32>()
    };

    assert_eq!(ink(4000.0), 0, "nothing yet — the break has just begun");
    assert!(ink(8800.0) > 0, "arrows before the map resumes");

    let steady: Vec<u32> = (0..12)
        .map(|i| glow(8300.0 + f64::from(i) * 25.0))
        .collect();
    assert!(
        steady.iter().all(|g| *g == steady[0]),
        "no timing, no pulse: {steady:?}"
    );

    assert!(ink(9050.0) > 0, "still on their way out just after");
    assert_eq!(ink(9300.0), 0, "and gone once the exit has run");
}

#[test]
fn the_break_arrows_sit_outside_the_field_and_inside_the_frame() {
    let map = beatmap(BREAK_MAP_TIMED);
    let state = GameState::from_beatmap(&map, Mods::default());
    let skin = Skin::default();
    let background = skin.background.to_color_u8();
    let scene = Scene::new(&state, skin);

    for (w, h) in [(640u32, 480u32), (1280, 720), (1920, 1080)] {
        let layout = Layout::new(w, h);
        let frame = scene.frame(8500.0, &layout);
        let (x0, y0) = layout.map(dossier_beatmap::Point { x: 0.0, y: 0.0 });
        let (x1, y1) = layout.map(dossier_beatmap::Point {
            x: dossier_beatmap::PLAYFIELD_WIDTH,
            y: dossier_beatmap::PLAYFIELD_HEIGHT,
        });

        let slack = layout.length(5.0);
        let (mut inside, mut outside, mut on_the_border) = (0, 0, 0);
        for y in 0..h {
            for x in 0..w {
                let p = frame.pixel(x, y).expect("inside the frame");
                if p.red() == background.red()
                    && p.green() == background.green()
                    && p.blue() == background.blue()
                {
                    continue;
                }
                let (fx, fy) = (x as f32, y as f32);
                if fx > x0 + slack && fx < x1 - slack && fy > y0 + slack && fy < y1 - slack {
                    inside += 1;
                } else {
                    outside += 1;
                }
                if x == 0 || y == 0 || x == w - 1 || y == h - 1 {
                    on_the_border += 1;
                }
            }
        }

        assert!(outside > 0, "{w}x{h}: the arrows are up");
        assert_eq!(inside, 0, "{w}x{h}: none of them intrudes on the field");
        assert_eq!(on_the_border, 0, "{w}x{h}: nor cut off by the frame");
    }
}

const LONE_SPINNER: &str = "
[Difficulty]
CircleSize:5
ApproachRate:5
OverallDifficulty:5

[HitObjects]
256,192,2000,12,0,6000
";

#[test]
fn the_spinner_ring_closes_onto_its_centre_mark() {
    let map = beatmap(LONE_SPINNER);
    let state = GameState::from_beatmap(&map, Mods::default());
    let skin = Skin::default();
    let background = skin.background.to_color_u8();
    let scene = Scene::new(&state, skin);
    let layout = Layout::new(640, 480);
    let object = &state.timeline().objects[0];

    let reach = |t: f64| {
        let frame = scene.frame(t, &layout);
        let (cx, cy) = layout.map(dossier_beatmap::Point::CENTRE);
        let mut furthest = 0.0f32;
        for y in 0..frame.height() {
            for x in 0..frame.width() {
                let p = frame.pixel(x, y).expect("inside the frame");
                if p.red() == background.red()
                    && p.green() == background.green()
                    && p.blue() == background.blue()
                {
                    continue;
                }
                let (dx, dy) = (x as f32 - cx, y as f32 - cy);
                furthest = furthest.max((dx * dx + dy * dy).sqrt());
            }
        }
        furthest
    };

    let opening = reach(object.start_ms + 50.0);
    let closing = reach(object.end_ms - 50.0);
    assert!(
        opening > closing,
        "the ring closes: {opening} then {closing}"
    );

    let mark = layout.length(20.0);
    assert!(
        (closing - mark).abs() < layout.length(6.0),
        "it lands on the mark: {closing} against a mark of {mark}"
    );

    assert!(
        ink_near_centre(&scene, &layout, object.start_ms + 50.0) > 0,
        "the centre mark is there while the ring is still wide"
    );
}

fn ink_near_centre(scene: &Scene<'_>, layout: &Layout, t: f64) -> usize {
    let background = Skin::default().background.to_color_u8();
    let frame = scene.frame(t, layout);
    let (cx, cy) = layout.map(dossier_beatmap::Point::CENTRE);
    let r = layout.length(20.0);
    let mut count = 0;
    for y in 0..frame.height() {
        for x in 0..frame.width() {
            let (dx, dy) = (x as f32 - cx, y as f32 - cy);
            if (dx * dx + dy * dy).sqrt() > r {
                continue;
            }
            let p = frame.pixel(x, y).expect("inside the frame");
            if p.red() != background.red() || p.green() != background.green() {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn no_arrow_stands_under_the_head_while_the_first_slide_runs() {
    let map = beatmap(THRICE_SLIDER);
    let state = GameState::from_beatmap(&map, Mods::default());
    let scene = Scene::new(&state, Skin::default());
    let layout = Layout::new(640, 480);
    let object = &state.timeline().objects[0];
    let span = (object.end_ms - object.start_ms) / 3.0;

    let white_at_head = |t: f64| {
        let frame = scene.frame(t, &layout);
        let (x, y) = layout.map(object.pos);
        let mut count = 0;
        for dy in -20i32..=20 {
            for dx in -20i32..=20 {
                let p = frame
                    .pixel((x as i32 + dx) as u32, (y as i32 + dy) as u32)
                    .expect("inside the frame");
                if p.red() > 230 && p.green() > 230 && p.blue() > 230 {
                    count += 1;
                }
            }
        }
        count
    };

    assert_eq!(
        white_at_head(object.start_ms + span * 0.4),
        0,
        "the head's turn is two traversals away — nothing belongs there yet"
    );

    assert!(
        white_at_head(object.start_ms + span + 180.0) > 0,
        "and it arrives once the ball sets off towards it"
    );
}

#[test]
fn hidden_takes_the_note_away_before_it_is_due() {
    let map =
        beatmap("[Difficulty]\nApproachRate:5\nCircleSize:4\n\n[HitObjects]\n256,192,2000,1,0\n");

    assert!(drawn_with(&map, 1300.0, Mods::default()) > 0);
    assert!(drawn_with(&map, 1300.0, Mods::new(bits::HIDDEN)) > 0);

    assert_eq!(
        drawn_with(&map, 1700.0, Mods::new(bits::HIDDEN)),
        0,
        "the note should be gone"
    );
    assert!(
        drawn_with(&map, 1700.0, Mods::default()) > 0,
        "and plainly visible without the mod"
    );
}

#[test]
fn hidden_draws_no_approach_circle() {
    let map =
        beatmap("[Difficulty]\nApproachRate:5\nCircleSize:4\n\n[HitObjects]\n256,192,2000,1,0\n");
    let plain = drawn_with(&map, 1280.0, Mods::default());
    let hidden = drawn_with(&map, 1280.0, Mods::new(bits::HIDDEN));
    assert!(
        plain > hidden,
        "the ring is missing from neither: {plain} against {hidden}"
    );
}

fn ball_pixel(map: &Beatmap, time_ms: f64, mods: Mods) -> (u8, u8, u8) {
    let state = GameState::from_beatmap(map, mods);
    let object = &state.timeline().objects[0];
    let ball = object
        .ball_at(time_ms)
        .expect("the slider is still running");
    let skin = Skin::with_combo_colours(map.combo_colours());
    let layout = Layout::new(320, 240);
    let frame = Scene::new(&state, skin).frame(time_ms, &layout);
    let (x, y) = layout.map(ball);
    let p = frame.pixel(x as u32, y as u32).expect("inside the frame");
    (p.red(), p.green(), p.blue())
}

#[test]
fn hidden_leaves_the_ball_and_the_arrow_alone() {
    let map = beatmap(
        "[Difficulty]\nApproachRate:5\nCircleSize:4\nSliderMultiplier:1.0\nSliderTickRate:1\n\n         [TimingPoints]\n0,500,4,2,0,100,1,0\n\n         [HitObjects]\n100,192,2000,2,0,L|300:192,2,100\n",
    );

    let plain = ball_pixel(&map, 2800.0, Mods::default());
    let hidden = ball_pixel(&map, 2800.0, Mods::new(bits::HIDDEN));
    assert_ne!(plain, (0, 0, 0), "the fixture draws no ball at all");
    assert_eq!(
        hidden, plain,
        "Hidden dimmed the ball, which it does not touch"
    );
}

const THREE_CIRCLES: &str = "
[Difficulty]
CircleSize:5
ApproachRate:5

[HitObjects]
256,192,5000,1,0
256,192,6000,1,0
256,192,7000,1,0
";

fn brightness(frame: &tiny_skia::Pixmap) -> f64 {
    let sum: u64 = frame
        .pixels()
        .iter()
        .map(|p| u64::from(p.red()) + u64::from(p.green()) + u64::from(p.blue()))
        .sum();
    sum as f64 / frame.pixels().len() as f64
}

fn failed_scene(map: &Beatmap) -> (GameState, Skin) {
    let mut replay = replay_over(vec![
        dossier_replay::ReplayFrame {
            time_ms: 5000,
            x: 256.0,
            y: 192.0,
            keys: dossier_replay::Keys(dossier_replay::Keys::K1),
        },
        dossier_replay::ReplayFrame {
            time_ms: 5040,
            x: 256.0,
            y: 192.0,
            keys: dossier_replay::Keys(0),
        },
    ]);

    replay.hits.count_300 = 1;
    let state = GameState::new(map, &replay);
    let skin = Skin::with_combo_colours(map.combo_colours()).with_font(font());
    (state, skin)
}

#[test]
fn the_play_comes_up_from_black_rather_than_cutting_in() {
    let map = beatmap(THREE_CIRCLES);
    let (state, skin) = failed_scene(&map);
    let scene = Scene::new(&state, skin);
    let layout = Layout::new(320, 240);
    let (from, _) = state.span_ms();

    let opening = brightness(&scene.frame(from + 20.0, &layout));
    let midway = brightness(&scene.frame(from + 200.0, &layout));
    let settled = brightness(&scene.frame(from + 600.0, &layout));

    assert!(
        opening < midway && midway < settled,
        "the opening should climb: {opening:.3} → {midway:.3} → {settled:.3}"
    );
}

#[test]
fn the_failed_frame_goes_black_the_instant_it_springs_back() {
    let map = beatmap(THREE_CIRCLES);
    let (state, skin) = failed_scene(&map);
    let end = state
        .ending()
        .expect("a play that stopped early has an ending")
        .time_ms;
    let scene = Scene::new(&state, skin);
    let layout = Layout::new(320, 240);

    let animation = dossier_render::FAIL_ANIMATION_MS;

    let released = brightness(&scene.frame(end + animation - 1.0, &layout));

    let gone = brightness(&scene.frame(end + animation + 1.0, &layout));

    assert!(
        released > 0.0,
        "the frame should still hold the play when it lets go"
    );

    let empty = {
        let mut blank = tiny_skia::Pixmap::new(320, 240).expect("a frame");
        blank.fill(Skin::default().background);
        brightness(&blank)
    };
    assert_eq!(
        gone, empty,
        "the frame did not clear when the movement landed"
    );
}

#[test]
fn nofail_takes_the_health_bar_and_the_warning_off() {
    let map = beatmap(THREE_CIRCLES);
    let (state, skin) = failed_scene(&map);
    let plain = brightness(&Scene::new(&state, skin.clone()).frame(5200.0, &Layout::new(320, 240)));

    let mut with_nofail = replay_over(vec![
        dossier_replay::ReplayFrame {
            time_ms: 5000,
            x: 256.0,
            y: 192.0,
            keys: dossier_replay::Keys(dossier_replay::Keys::K1),
        },
        dossier_replay::ReplayFrame {
            time_ms: 5040,
            x: 256.0,
            y: 192.0,
            keys: dossier_replay::Keys(0),
        },
    ]);
    with_nofail.hits.count_300 = 1;
    with_nofail.mods = Mods::new(dossier_replay::bits::NO_FAIL);
    let nofail_state = GameState::new(&map, &with_nofail);
    let quiet = brightness(&Scene::new(&nofail_state, skin).frame(5200.0, &Layout::new(320, 240)));

    assert!(
        quiet < plain,
        "a NoFail frame should carry less: {quiet:.3} against {plain:.3}"
    );
}

#[test]
fn the_field_is_offset_in_osu_pixels_rather_than_in_frame_pixels() {
    let map = beatmap(ONE_CIRCLE);
    let state = GameState::from_beatmap(&map, Mods::default());
    let skin = Skin::with_combo_colours(map.combo_colours());
    let scene = Scene::new(&state, skin);

    for (w, h) in [(1920u32, 1080u32), (960, 1080), (1080, 1920)] {
        let layout = Layout::new(w, h);
        let (_, centre_y) = layout.map(dossier_beatmap::Point { x: 256.0, y: 192.0 });
        let offset = centre_y - h as f32 / 2.0;
        let expected = layout.length(8.0);
        assert!(
            (offset - expected).abs() < 0.01,
            "{w}x{h}: field centre is {offset:.2}px below the frame's, wanted {expected:.2}px"
        );
        let _ = &scene;
    }
}

const ONE_SPINNER: &str = "
[Difficulty]
CircleSize:4
ApproachRate:8
OverallDifficulty:6

[HitObjects]
256,192,2000,12,0,6000
";

#[test]
fn hidden_does_not_take_the_spinner_away() {
    let map = beatmap(ONE_SPINNER);
    let skin = Skin::with_combo_colours(map.combo_colours());

    let plain = GameState::from_beatmap(&map, Mods::default());
    let hidden = GameState::from_beatmap(&map, Mods::new(dossier_replay::bits::HIDDEN));
    let layout = Layout::new(320, 240);

    let without = brightness(&Scene::new(&plain, skin.clone()).frame(5000.0, &layout));
    let with = brightness(&Scene::new(&hidden, skin).frame(5000.0, &layout));

    assert!(without > 0.0, "the spinner should be drawn at all");
    assert!(
        (with - without).abs() < without * 0.05,
        "Hidden should leave the spinner alone: {with:.3} against {without:.3}"
    );
}

#[test]
fn the_play_goes_out_the_way_it_came_in() {
    let map = beatmap(THREE_CIRCLES);
    let mut replay = replay_over(vec![
        dossier_replay::ReplayFrame {
            time_ms: 5000,
            x: 256.0,
            y: 192.0,
            keys: dossier_replay::Keys(dossier_replay::Keys::K1),
        },
        dossier_replay::ReplayFrame {
            time_ms: 5040,
            x: 256.0,
            y: 192.0,
            keys: dossier_replay::Keys(0),
        },
    ]);
    replay.hits.count_300 = 3;
    let state = GameState::new(&map, &replay);
    let skin = Skin::with_combo_colours(map.combo_colours()).with_font(font());
    let scene = Scene::new(&state, skin);
    let layout = Layout::new(320, 240);
    let (_, to) = state.span_ms();

    let settled = brightness(&scene.frame(to - 50.0, &layout));
    let going = brightness(&scene.frame(to + dossier_render::OUTRO_FADE_MS * 0.5, &layout));
    let last = brightness(&scene.frame(to + dossier_render::OUTRO_FADE_MS * 0.95, &layout));

    assert!(
        last < going && going < settled,
        "the close should dim: {settled:.3} → {going:.3} → {last:.3}"
    );
}

#[test]
fn a_failed_play_is_not_faded_out_as_well() {
    let map = beatmap(THREE_CIRCLES);
    let (state, skin) = failed_scene(&map);
    let scene = Scene::new(&state, skin);
    let layout = Layout::new(320, 240);
    let (_, to) = state.span_ms();

    let early = brightness(&scene.frame(to - 50.0, &layout));
    let late = brightness(&scene.frame(to + dossier_render::OUTRO_FADE_MS * 0.95, &layout));
    assert!(
        (late - early).abs() < early * 0.35,
        "no second fade on a failed play: {early:.3} against {late:.3}"
    );
}

#[test]
fn hidden_fades_a_slider_body_slowly_and_its_head_like_a_note() {
    let map = repeating_slider(1);
    let hidden = GameState::from_beatmap(&map, Mods::new(dossier_replay::bits::HIDDEN));
    let skin = Skin::with_combo_colours(map.combo_colours());
    let scene = Scene::new(&hidden, skin);

    let object = &hidden.timeline().objects[0];
    let at = object.start_ms - 30.0;
    let head = scene.head_alpha_for_test(0, at);
    let body = scene.alpha_for_test(0, at);
    assert_eq!(head, 0.0, "the head goes on the note's schedule");
    assert!(
        body > 0.2,
        "while the body is still on its way out: {body:.3}"
    );

    let arriving = object.start_ms - hidden.difficulty().preempt_ms() + 1.0;
    assert!(
        (scene.head_alpha_for_test(0, arriving) - scene.alpha_for_test(0, arriving)).abs() < 0.01
    );
}

fn skin_folder(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("dossier-frame-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    dir
}

fn write_element(dir: &std::path::Path, name: &str, size: u32, alpha: u8) {
    let mut pixmap = tiny_skia::Pixmap::new(size, size).expect("a canvas");
    for pixel in pixmap.pixels_mut() {
        *pixel = tiny_skia::PremultipliedColorU8::from_rgba(alpha, alpha, alpha, alpha)
            .expect("a colour");
    }
    std::fs::write(dir.join(name), pixmap.encode_png().expect("png")).expect("written");
}

fn write_panel(dir: &std::path::Path, name: &str, width: u32, height: u32, alpha: u8) {
    let mut pixmap = tiny_skia::Pixmap::new(width, height).expect("a canvas");
    for pixel in pixmap.pixels_mut() {
        *pixel = tiny_skia::PremultipliedColorU8::from_rgba(alpha, alpha, alpha, alpha)
            .expect("a colour");
    }
    std::fs::write(dir.join(name), pixmap.encode_png().expect("png")).expect("written");
}

fn one_note() -> Beatmap {
    beatmap(
        "
[Difficulty]
ApproachRate:5
CircleSize:4

[Colours]
Combo1 : 0,255,0

[HitObjects]
256,192,5000,5,0
",
    )
}

const FIELD: (u8, u8, u8) = (12, 12, 16);

fn note_pixel(skin: Skin) -> (u8, u8, u8) {
    let map = one_note();
    let state = GameState::from_beatmap(&map, Mods::default());
    let layout = Layout::new(640, 480);
    let frame = Scene::new(&state, skin).frame(5000.0, &layout);
    let (x, y) = layout.map(dossier_beatmap::Point::CENTRE);
    let p = frame.pixel(x as u32, y as u32).expect("inside the frame");
    (p.red(), p.green(), p.blue())
}

fn with_sprites(dir: &std::path::Path, map: &Beatmap) -> Skin {
    use dossier_render::elements::Element;
    use dossier_render::imported::Sprites;
    let mut skin = Skin::with_combo_colours(map.combo_colours());
    let wanted = [Element::HitCircle, Element::HitCircleOverlay];
    let sprites = Sprites::read(dir, &wanted).tint_for(&skin.combo_colours);
    skin.sprites = Some(std::sync::Arc::new(sprites));
    skin
}

#[test]
fn a_skins_own_hit_circle_is_drawn_in_place_of_ours() {
    let dir = skin_folder("drawn");
    write_element(&dir, "hitcircle.png", 128, 255);
    let (r, g, b) = note_pixel(with_sprites(&dir, &one_note()));

    assert!(
        g > 200,
        "the skin's note is there, in the map's combo colour"
    );
    assert!(
        r < 60 && b < 60,
        "not the white it was drawn in: {r},{g},{b}"
    );
}

#[test]
fn a_skin_that_turned_the_note_off_gets_an_empty_field() {
    let dir = skin_folder("silenced");
    write_element(&dir, "hitcircle.png", 128, 0);
    assert_eq!(
        note_pixel(with_sprites(&dir, &one_note())),
        FIELD,
        "the bare field, where the note would have been"
    );
}

#[test]
fn a_skin_that_says_nothing_leaves_the_note_to_us() {
    let dir = skin_folder("empty");
    assert_ne!(
        note_pixel(with_sprites(&dir, &one_note())),
        FIELD,
        "our own circle is still drawn"
    );
}

fn ink_reach(dir: &std::path::Path) -> usize {
    let map = one_note();
    let state = GameState::from_beatmap(&map, Mods::default());
    let layout = Layout::new(640, 480);
    let (cx, cy) = layout.map(dossier_beatmap::Point::CENTRE);
    let frame = Scene::new(&state, with_sprites(dir, &map)).frame(5000.0, &layout);
    (0..300)
        .take_while(|step| {
            frame
                .pixel(cx as u32 + step, cy as u32)
                .is_some_and(|p| (p.red(), p.green(), p.blue()) != FIELD)
        })
        .count()
}

#[test]
fn a_bigger_file_is_a_bigger_element_because_that_is_what_it_means() {
    let small = skin_folder("small");
    write_element(&small, "hitcircle.png", 128, 255);
    let big = skin_folder("big");
    write_element(&big, "hitcircle.png", 512, 255);

    let (near, far) = (ink_reach(&small), ink_reach(&big));
    let ratio = far as f32 / near as f32;
    assert!(
        (3.5..4.5).contains(&ratio),
        "four times the file, four times the element — got {near} against {far}"
    );
}

#[test]
fn the_high_resolution_suffix_is_what_normalises_a_size() {
    let plain = skin_folder("plain-size");
    write_element(&plain, "hitcircle.png", 128, 255);
    let double = skin_folder("double-size");
    write_element(&double, "hitcircle@2x.png", 256, 255);

    let (a, b) = (ink_reach(&plain), ink_reach(&double));
    assert!(
        a.abs_diff(b) <= 2,
        "the same element at the same size: {a} against {b}"
    );
}

#[test]
fn a_skins_digits_are_drawn_where_it_asked_for_them() {
    let dir = skin_folder("digits");
    for digit in 0..10 {
        write_element(&dir, &format!("default-{digit}.png"), 64, 255);
    }
    std::fs::write(dir.join("skin.ini"), "[Fonts]\nHitCircleOverlap: 0\n").expect("written");

    let map = one_note();
    let state = GameState::from_beatmap(&map, Mods::default());
    let layout = Layout::new(640, 480);
    let frame = Scene::new(&state, with_digits(&dir, &map)).frame(5000.0, &layout);
    let (x, y) = layout.map(dossier_beatmap::Point::CENTRE);
    let p = frame.pixel(x as u32, y as u32).expect("inside the frame");
    assert_ne!(
        (p.red(), p.green(), p.blue()),
        FIELD,
        "the skin's own figure is on the note"
    );
}

#[test]
fn an_overlap_as_wide_as_the_digit_stacks_the_figures() {
    let stacked = skin_folder("stacked");
    let spread = skin_folder("spread");
    for dir in [&stacked, &spread] {
        for digit in 0..10 {
            write_element(dir, &format!("default-{digit}.png"), 256, 255);
        }
    }
    std::fs::write(stacked.join("skin.ini"), "[Fonts]\nHitCircleOverlap: 256\n").expect("written");
    std::fs::write(spread.join("skin.ini"), "[Fonts]\nHitCircleOverlap: 0\n").expect("written");

    let map = beatmap(
        "
[Difficulty]
ApproachRate:5
CircleSize:4

[HitObjects]
256,192,4000,5,0
256,192,4200,1,0
256,192,4400,1,0
256,192,4600,1,0
256,192,4800,1,0
256,192,5000,1,0
256,192,5200,1,0
256,192,5400,1,0
256,192,5600,1,0
256,192,5800,1,0
256,192,6000,1,0
256,192,6200,1,0
",
    );
    let state = GameState::from_beatmap(&map, Mods::default());
    let layout = Layout::new(640, 480);
    let (cx, cy) = layout.map(dossier_beatmap::Point::CENTRE);

    let reach = |dir: &std::path::Path| {
        let frame = Scene::new(&state, with_digits(dir, &map)).frame(6200.0, &layout);
        (0..300)
            .take_while(|step| {
                frame
                    .pixel(cx as u32 + step, cy as u32)
                    .is_some_and(|p| (p.red(), p.green(), p.blue()) != FIELD)
            })
            .count()
    };
    assert!(
        reach(&stacked) < reach(&spread),
        "stacked {} against spread {}",
        reach(&stacked),
        reach(&spread)
    );
}

fn with_digits(dir: &std::path::Path, map: &Beatmap) -> Skin {
    use dossier_render::elements::Element;
    use dossier_render::imported::Sprites;
    let mut skin = Skin::with_combo_colours(map.combo_colours());
    let wanted: Vec<Element> = (0..10).map(Element::Digit).collect();
    let sprites = Sprites::read(dir, &wanted).tint_for(&skin.combo_colours);
    skin.sprites = Some(std::sync::Arc::new(sprites));
    skin
}

fn plain_slider() -> Beatmap {
    beatmap(
        "
[Difficulty]
CircleSize:4
ApproachRate:5
SliderMultiplier:1.4
SliderTickRate:1

[Colours]
Combo1 : 0,255,0

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
100,192,1000,2,0,L|240:192,1,140
",
    )
}

fn slider_wanted() -> Vec<dossier_render::elements::Element> {
    use dossier_render::elements::Element;
    vec![
        Element::HitCircle,
        Element::HitCircleOverlay,
        Element::SliderHead,
        Element::SliderHeadOverlay,
        Element::SliderTail,
        Element::SliderTailOverlay,
    ]
}

fn dressed(dir: &std::path::Path, map: &Beatmap) -> Skin {
    use dossier_render::imported::Sprites;
    let mut skin = Skin::with_combo_colours(map.combo_colours());
    let sprites = Sprites::read(dir, &slider_wanted()).tint_for(&skin.combo_colours);
    skin.sprites = Some(std::sync::Arc::new(sprites));
    skin
}

fn slider_frame(dir: &std::path::Path) -> tiny_skia::Pixmap {
    let map = plain_slider();
    let state = GameState::from_beatmap(&map, Mods::default());
    let layout = Layout::new(640, 480);
    Scene::new(&state, dressed(dir, &map)).frame(1000.0, &layout)
}

fn line_pixel(frame: &tiny_skia::Pixmap, x: f64) -> (u8, u8, u8) {
    let layout = Layout::new(640, 480);
    let (cx, cy) = layout.map(dossier_beatmap::Point { x, y: 192.0 });
    let p = frame
        .pixel(cx as u32, cy as u32)
        .expect("the slider is inside the frame");
    (p.red(), p.green(), p.blue())
}

const HEAD_X: f64 = 100.0;
const TAIL_X: f64 = 240.0;

const BODY_X: f64 = 170.0;

#[test]
fn a_skins_own_start_circle_is_drawn_in_place_of_the_note() {
    let note_only = skin_folder("slider-note-only");
    write_element(&note_only, "hitcircle.png", 128, 255);

    let with_start = skin_folder("slider-own-start");
    write_element(&with_start, "hitcircle.png", 128, 255);
    write_element(&with_start, "sliderstartcircle.png", 128, 60);

    let plain = line_pixel(&slider_frame(&note_only), HEAD_X);
    let own = line_pixel(&slider_frame(&with_start), HEAD_X);
    assert_ne!(plain, own, "the skin's own start circle was ignored");
}

#[test]
fn an_overlay_without_its_own_base_falls_back_to_the_notes_pair() {
    let note_only = skin_folder("slider-pair-base");
    write_element(&note_only, "hitcircle.png", 128, 255);
    write_element(&note_only, "hitcircleoverlay.png", 128, 90);

    let orphan = skin_folder("slider-pair-orphan");
    write_element(&orphan, "hitcircle.png", 128, 255);
    write_element(&orphan, "hitcircleoverlay.png", 128, 90);
    write_element(&orphan, "sliderstartcircleoverlay.png", 128, 20);

    assert_eq!(
        line_pixel(&slider_frame(&note_only), HEAD_X),
        line_pixel(&slider_frame(&orphan), HEAD_X),
        "an overlay with no base of its own changed the head"
    );
}

#[test]
fn the_end_of_a_slider_wears_the_note_when_the_skin_says_nothing() {
    let dir = skin_folder("slider-end-default");
    write_element(&dir, "hitcircle.png", 128, 255);
    let frame = slider_frame(&dir);
    assert_ne!(
        line_pixel(&frame, TAIL_X),
        line_pixel(&frame, BODY_X),
        "the end of the slider is bare body — no circle was drawn there"
    );
}

#[test]
fn an_end_circle_blanked_on_purpose_stays_blank() {
    let dir = skin_folder("slider-end-hidden");
    write_element(&dir, "hitcircle.png", 128, 255);
    write_element(&dir, "sliderendcircle.png", 1, 0);
    let frame = slider_frame(&dir);

    assert!(
        apart(line_pixel(&frame, TAIL_X), line_pixel(&frame, BODY_X)) <= 6,
        "something was drawn where the skin asked for nothing: {:?} against {:?}",
        line_pixel(&frame, TAIL_X),
        line_pixel(&frame, BODY_X)
    );
}

#[test]
fn our_own_look_still_ends_a_slider_on_its_body() {
    let map = plain_slider();
    let state = GameState::from_beatmap(&map, Mods::default());
    let layout = Layout::new(640, 480);
    let bare = Scene::new(&state, Skin::default()).frame(1000.0, &layout);
    assert!(
        apart(line_pixel(&bare, TAIL_X), line_pixel(&bare, BODY_X)) <= 6,
        "our own look grew an end circle it never had: {:?} against {:?}",
        line_pixel(&bare, TAIL_X),
        line_pixel(&bare, BODY_X)
    );
}

fn spaced_pair(new_combo: bool) -> Beatmap {
    beatmap(&format!(
        "
[Difficulty]
CircleSize:4
ApproachRate:5

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
100,192,4000,5,0
400,192,5000,{},0
",
        if new_combo { 5 } else { 1 }
    ))
}

fn trail_ink(dir: Option<&std::path::Path>, map: &Beatmap, time_ms: f64) -> usize {
    use dossier_render::elements::Element;
    use dossier_render::imported::Sprites;
    let mut skin = Skin::with_combo_colours(map.combo_colours());
    if let Some(dir) = dir {
        let wanted = [Element::HitCircle, Element::FollowPoint];
        skin.sprites = Some(std::sync::Arc::new(
            Sprites::read(dir, &wanted).tint_for(&skin.combo_colours),
        ));
    }
    let state = GameState::from_beatmap(map, Mods::default());
    let layout = Layout::new(640, 480);
    let frame = Scene::new(&state, skin).frame(time_ms, &layout);

    let (cx, cy) = layout.map(dossier_beatmap::Point { x: 250.0, y: 192.0 });
    let mut count = 0;
    for dy in -30i32..30 {
        for dx in -60i32..60 {
            let Some(p) = frame.pixel((cx as i32 + dx) as u32, (cy as i32 + dy) as u32) else {
                continue;
            };
            if p.red() > 40 || p.green() > 40 || p.blue() > 40 {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn a_trail_runs_between_two_notes_of_one_combo() {
    let dir = skin_folder("trail");
    write_element(&dir, "followpoint.png", 32, 255);
    let map = spaced_pair(false);

    assert!(
        trail_ink(Some(&dir), &map, 4600.0) > 0,
        "nothing was drawn between the two notes"
    );
}

#[test]
fn no_trail_crosses_a_new_combo() {
    let dir = skin_folder("trail-combo");
    write_element(&dir, "followpoint.png", 32, 255);
    assert_eq!(
        trail_ink(Some(&dir), &spaced_pair(true), 4600.0),
        0,
        "a trail was drawn across a combo break"
    );
}

#[test]
fn a_skin_without_the_picture_gets_no_trail() {
    let dir = skin_folder("trail-none");
    write_element(&dir, "hitcircle.png", 128, 255);
    assert_eq!(trail_ink(Some(&dir), &spaced_pair(false), 4600.0), 0);
    assert_eq!(trail_ink(None, &spaced_pair(false), 4600.0), 0, "nor ours");
}

#[test]
fn the_trail_is_gone_once_the_note_it_led_to_is_due() {
    let dir = skin_folder("trail-gone");
    write_element(&dir, "followpoint.png", 32, 255);
    let map = spaced_pair(false);
    let before = trail_ink(Some(&dir), &map, 4600.0);
    let after = trail_ink(Some(&dir), &map, 5400.0);
    assert!(after < before, "{after} against {before}");
}

#[test]
fn the_hit_flash_is_off_unless_it_is_asked_for() {
    let dir = skin_folder("flash");
    write_element(&dir, "hitcircle.png", 128, 255);
    write_element(&dir, "lighting.png", 100, 255);

    use dossier_render::elements::Element;
    use dossier_render::imported::Sprites;
    let sprites = Sprites::read(&dir, &[Element::HitCircle, Element::Lighting]);
    assert!(
        !sprites.draw_ourselves(Element::Lighting),
        "the skin's flash is read"
    );
    assert!(!Skin::default().hit_lighting, "and not drawn");
}

fn tapped() -> (Beatmap, dossier_replay::Replay) {
    let map = beatmap(
        "
[Difficulty]
ApproachRate:5
CircleSize:4

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
100,100,3000,5,0
400,300,4000,1,0
",
    );
    let tap = |t: i64, x: f32, y: f32, k: u8| dossier_replay::ReplayFrame {
        time_ms: t,
        x,
        y,
        keys: dossier_replay::Keys(k),
    };
    let replay = replay_over(vec![
        tap(2990, 100.0, 100.0, 0),
        tap(3000, 100.0, 100.0, dossier_replay::Keys::K1),
        tap(3010, 100.0, 100.0, 0),
        tap(3990, 400.0, 300.0, 0),
        tap(4000, 400.0, 300.0, dossier_replay::Keys::K2),
        tap(4010, 400.0, 300.0, 0),
    ]);
    (map, replay)
}

fn key_column(dir: Option<&std::path::Path>, time_ms: f64) -> usize {
    use dossier_render::elements::Element;
    use dossier_render::imported::Sprites;
    let (map, replay) = tapped();
    let mut skin = Skin::with_combo_colours(map.combo_colours()).with_font(font());
    if let Some(dir) = dir {
        let wanted = [
            Element::HitCircle,
            Element::InputOverlayKey,
            Element::InputOverlayBackground,
        ];
        skin.sprites = Some(std::sync::Arc::new(
            Sprites::read(dir, &wanted).tint_for(&skin.combo_colours),
        ));
    }
    let state = GameState::new(&map, &replay);
    let frame = Scene::new(&state, skin).frame(time_ms, &Layout::new(640, 480));

    let mut count = 0;
    for y in 180..300u32 {
        for x in 560..640u32 {
            let Some(p) = frame.pixel(x, y) else { continue };
            let off = (i32::from(p.red()) - i32::from(FIELD.0)).abs()
                + (i32::from(p.green()) - i32::from(FIELD.1)).abs()
                + (i32::from(p.blue()) - i32::from(FIELD.2)).abs();
            if off > 12 {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn a_skins_own_key_overlay_replaces_ours() {
    let dir = skin_folder("keys");
    write_element(&dir, "inputoverlay-key.png", 46, 255);
    write_element(&dir, "inputoverlay-background.png", 64, 160);

    let ours = key_column(None, 4200.0);
    let theirs = key_column(Some(&dir), 4200.0);
    assert!(ours > 0, "our own counters are drawn when nothing else is");
    assert_ne!(ours, theirs, "the skin's overlay was ignored");
}

#[test]
fn a_skin_with_no_overlay_keeps_our_counters() {
    let dir = skin_folder("keys-none");
    write_element(&dir, "hitcircle.png", 128, 255);
    assert_eq!(key_column(Some(&dir), 4200.0), key_column(None, 4200.0));
}

#[test]
fn a_held_key_is_lit_and_a_loose_one_is_not() {
    let dir = skin_folder("keys-lit");
    write_element(&dir, "inputoverlay-key.png", 46, 255);

    assert_ne!(
        key_column(Some(&dir), 3000.0),
        key_column(Some(&dir), 3600.0),
        "held and loose look the same"
    );
}

#[test]
fn the_skins_overlay_hangs_off_the_edge_of_the_frame() {
    use dossier_render::elements::Element;
    use dossier_render::imported::Sprites;

    let dir = skin_folder("keys-edge");
    write_element(&dir, "inputoverlay-key.png", 46, 255);
    write_element(&dir, "inputoverlay-background.png", 64, 200);

    let (map, replay) = tapped();
    let mut skin = Skin::with_combo_colours(map.combo_colours()).with_font(font());
    let wanted = [Element::InputOverlayKey, Element::InputOverlayBackground];
    skin.sprites = Some(std::sync::Arc::new(
        Sprites::read(&dir, &wanted).tint_for(&skin.combo_colours),
    ));
    let state = GameState::new(&map, &replay);
    let layout = Layout::new(640, 480);
    let frame = Scene::new(&state, skin).frame(4200.0, &layout);

    let lit = (180..300u32)
        .filter(|&y| {
            frame
                .pixel(639, y)
                .is_some_and(|p| i32::from(p.red()) - i32::from(FIELD.0) != 0)
        })
        .count();
    assert!(lit > 0, "the panel does not reach the edge of the frame");
}

fn first_key_row(dir: &std::path::Path) -> u32 {
    use dossier_render::elements::Element;
    use dossier_render::imported::Sprites;
    let (map, replay) = tapped();
    let mut skin = Skin::with_combo_colours(map.combo_colours()).with_font(font());
    let wanted = [Element::InputOverlayKey, Element::InputOverlayBackground];
    skin.sprites = Some(std::sync::Arc::new(
        Sprites::read(dir, &wanted).tint_for(&skin.combo_colours),
    ));
    let state = GameState::new(&map, &replay);
    let frame = Scene::new(&state, skin).frame(4200.0, &Layout::new(640, 480));
    (100..480u32)
        .find(|&y| (618..632u32).all(|x| frame.pixel(x, y).is_some_and(|p| p.red() > 200)))
        .expect("the skin's key is drawn somewhere below the score")
}

#[test]
fn the_keys_sit_where_they_sit_whatever_the_panel_measures() {
    let short = skin_folder("keys-panel-short");
    write_element(&short, "inputoverlay-key.png", 46, 255);
    write_panel(&short, "inputoverlay-background.png", 55, 55, 90);

    let long = skin_folder("keys-panel-long");
    write_element(&long, "inputoverlay-key.png", 46, 255);
    write_panel(&long, "inputoverlay-background.png", 300, 55, 90);

    let (a, b) = (first_key_row(&short), first_key_row(&long));
    assert_eq!(a, b, "the panel's length moved the keys: {a} against {b}");

    assert!(
        (202..=207).contains(&a),
        "the keys start at {a}, not the 204 osu! starts them at"
    );
}

fn miss_mark_width(circle_size: &str, dir: &std::path::Path) -> usize {
    use dossier_render::elements::Element;
    use dossier_render::elements::Verdict;
    use dossier_render::imported::Sprites;

    let map = beatmap(&format!(
        "
[Difficulty]
ApproachRate:5
OverallDifficulty:5
CircleSize:{circle_size}

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
256,192,3000,5,0
256,192,9000,5,0
"
    ));

    let replay = replay_over(
        (0..40)
            .map(|i| dossier_replay::ReplayFrame {
                time_ms: 2000 + i64::from(i) * 100,
                x: 20.0,
                y: 20.0,
                keys: dossier_replay::Keys(0),
            })
            .collect(),
    );

    let mut skin = Skin::with_combo_colours(map.combo_colours()).with_font(font());
    let all: Vec<Element> = Verdict::ALL.iter().copied().map(Element::Verdict).collect();
    let sprites = Sprites::read(dir, &all).tint_for(&skin.combo_colours);
    skin.sprites = Some(std::sync::Arc::new(sprites));

    let state = GameState::new(&map, &replay);
    let layout = Layout::new(640, 480);

    let frame = Scene::new(&state, skin).frame(3400.0, &layout);

    let (cx, cy) = layout.map(dossier_beatmap::Point::CENTRE);
    (0..640u32)
        .filter(|&x| {
            frame.pixel(x, cy as u32).is_some_and(|p| {
                (i32::from(p.red()) - i32::from(FIELD.0)).abs()
                    + (i32::from(p.green()) - i32::from(FIELD.1)).abs()
                    + (i32::from(p.blue()) - i32::from(FIELD.2)).abs()
                    > 12
            }) && x.abs_diff(cx as u32) < 200
        })
        .count()
}

#[test]
fn a_judgement_below_the_ceiling_keeps_the_size_the_skin_drew() {
    let dir = skin_folder("verdict-ruler");

    write_padded(&dir, "hit0.png", 200, 12);

    let roomy = miss_mark_width("2", &dir);
    let tight = miss_mark_width("6", &dir);
    assert!(roomy > 0, "the skin's mark is drawn at all");
    assert_eq!(
        roomy, tight,
        "the circle size changed the mark: {roomy} against {tight}"
    );

    let wider = skin_folder("verdict-ruler-wide");
    write_padded(&wider, "hit0.png", 200, 16);
    assert!(
        miss_mark_width("2", &wider) > roomy,
        "a wider picture was not a wider mark"
    );
}

fn overlap_samples(time_ms: f64, played: bool) -> [(u8, u8, u8); 3] {
    let map = beatmap(
        "
[Difficulty]
ApproachRate:5
OverallDifficulty:5
CircleSize:4

[Colours]
Combo1 : 255,0,0
Combo2 : 0,0,255

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
230,192,3000,5,0
280,192,3150,5,0
",
    );
    let layout = Layout::new(640, 480);

    let state = if played {
        GameState::new(
            &map,
            &replay_over(vec![
                dossier_replay::ReplayFrame {
                    time_ms: 2990,
                    x: 230.0,
                    y: 192.0,
                    keys: dossier_replay::Keys(0),
                },
                dossier_replay::ReplayFrame {
                    time_ms: 3000,
                    x: 230.0,
                    y: 192.0,
                    keys: dossier_replay::Keys(dossier_replay::Keys::K1),
                },
                dossier_replay::ReplayFrame {
                    time_ms: 3010,
                    x: 230.0,
                    y: 192.0,
                    keys: dossier_replay::Keys(0),
                },
            ]),
        )
    } else {
        GameState::from_beatmap(&map, Mods::default())
    };
    let frame = Scene::new(&state, Skin::default().with_font(font())).frame(time_ms, &layout);
    let at = |x: f64| {
        let (cx, cy) = layout.map(dossier_beatmap::Point { x, y: 192.0 });
        let p = frame.pixel(cx as u32, cy as u32).expect("inside the frame");
        (p.red(), p.green(), p.blue())
    };

    [at(215.0), at(295.0), at(255.0)]
}

fn apart(a: (u8, u8, u8), b: (u8, u8, u8)) -> i32 {
    (i32::from(a.0) - i32::from(b.0)).abs()
        + (i32::from(a.1) - i32::from(b.1)).abs()
        + (i32::from(a.2) - i32::from(b.2)).abs()
}

#[test]
fn among_notes_still_in_play_the_soonest_is_on_top() {
    let [first, second, seam] = overlap_samples(3100.0, false);
    assert!(
        apart(first, second) > 60,
        "the two notes are tellable apart"
    );
    assert!(
        apart(seam, first) < apart(seam, second),
        "the later note covered one still to be hit: {seam:?} against {first:?}"
    );
}

fn exit_over_a_later_body(slider: bool) -> (u8, u8, u8) {
    let mut objects = String::from("256,192,2000,5,0\n");
    if slider {
        objects.push_str("120,192,2200,2,0,L|400:192,1,280\n");
    }
    let map = beatmap(&format!(
        "
[Difficulty]
CircleSize:4
ApproachRate:5
OverallDifficulty:5
SliderMultiplier:0.4
SliderTickRate:1

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
{objects}"
    ));
    let replay = replay_over(vec![
        dossier_replay::ReplayFrame {
            time_ms: 1000,
            x: 60.0,
            y: 340.0,
            keys: dossier_replay::Keys(0),
        },
        dossier_replay::ReplayFrame {
            time_ms: 1990,
            x: 256.0,
            y: 192.0,
            keys: dossier_replay::Keys(0),
        },
        dossier_replay::ReplayFrame {
            time_ms: 2000,
            x: 256.0,
            y: 192.0,
            keys: dossier_replay::Keys(dossier_replay::Keys::K1),
        },
        dossier_replay::ReplayFrame {
            time_ms: 2020,
            x: 60.0,
            y: 340.0,
            keys: dossier_replay::Keys(0),
        },
    ]);
    let state = GameState::new(&map, &replay);
    let layout = Layout::new(640, 480);
    let frame = Scene::new(&state, Skin::default().with_font(font())).frame(2100.0, &layout);
    let (cx, cy) = layout.map(dossier_beatmap::Point::CENTRE);
    let p = frame.pixel(cx as u32, cy as u32).expect("inside the frame");
    (p.red(), p.green(), p.blue())
}

#[test]
fn a_notes_exit_animation_is_not_dimmed_by_a_later_sliders_body() {
    let with_slider = exit_over_a_later_body(true);
    let alone = exit_over_a_later_body(false);

    let sum = |c: (u8, u8, u8)| u32::from(c.0) + u32::from(c.1) + u32::from(c.2);
    assert!(
        sum(with_slider) >= sum(alone),
        "the exit animation was dimmed by a later slider's body: \
         {with_slider:?} against {alone:?}"
    );
}

#[test]
fn the_one_underneath_is_still_drawn() {
    for played in [false, true] {
        let [first, _, _] = overlap_samples(3100.0, played);
        assert!(
            u32::from(first.0) + u32::from(first.1) + u32::from(first.2) > 40,
            "the earlier note vanished rather than going underneath: {first:?}"
        );
    }
}

fn stacked(time_ms: f64, note: bool, slider: bool) -> (u8, u8, u8) {
    let mut objects = String::new();
    if slider {
        objects.push_str("120,192,2000,2,0,L|400:192,1,280\n");
    }
    if note {
        objects.push_str("256,192,2500,5,0\n");
    }
    let map = beatmap(&format!(
        "
[Difficulty]
CircleSize:4
ApproachRate:5
OverallDifficulty:5
SliderMultiplier:0.4
SliderTickRate:1

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
{objects}"
    ));
    let replay = replay_over(vec![
        dossier_replay::ReplayFrame {
            time_ms: 1000,
            x: 60.0,
            y: 340.0,
            keys: dossier_replay::Keys(0),
        },
        dossier_replay::ReplayFrame {
            time_ms: 2490,
            x: 256.0,
            y: 192.0,
            keys: dossier_replay::Keys(0),
        },
        dossier_replay::ReplayFrame {
            time_ms: 2500,
            x: 256.0,
            y: 192.0,
            keys: dossier_replay::Keys(dossier_replay::Keys::K1),
        },
        dossier_replay::ReplayFrame {
            time_ms: 2520,
            x: 60.0,
            y: 340.0,
            keys: dossier_replay::Keys(0),
        },
    ]);
    let state = GameState::new(&map, &replay);
    let layout = Layout::new(640, 480);

    let quiet = skin_folder("dim-probe");
    for name in ["hit300", "hit100", "hit50", "hit0"] {
        write_element(&quiet, &format!("{name}.png"), 8, 0);
    }
    let mut skin = Skin::default().with_font(font());
    let wanted: Vec<dossier_render::elements::Element> = [
        dossier_render::elements::Verdict::Three,
        dossier_render::elements::Verdict::Hundred,
        dossier_render::elements::Verdict::Fifty,
        dossier_render::elements::Verdict::Miss,
    ]
    .into_iter()
    .map(dossier_render::elements::Element::Verdict)
    .collect();
    skin.sprites = Some(std::sync::Arc::new(
        dossier_render::imported::Sprites::read(&quiet, &wanted),
    ));
    let frame = Scene::new(&state, skin).frame(time_ms, &layout);

    let (cx, cy) = layout.map(dossier_beatmap::Point {
        x: 256.0 + 28.0,
        y: 192.0,
    });
    let p = frame.pixel(cx as u32, cy as u32).expect("inside the frame");
    (p.red(), p.green(), p.blue())
}

#[test]
fn a_note_the_body_passes_over_is_dimmed_rather_than_hidden() {
    let both = stacked(2520.0, true, true);
    let body_alone = stacked(2520.0, false, true);
    let note_alone = stacked(2520.0, true, false);
    println!("СКВОЗЬ: под телом {both:?}, тело {body_alone:?}, нота {note_alone:?}");
    assert!(
        apart(both, body_alone) > 20,
        "the note under the body left no trace at all: {both:?} against {body_alone:?}"
    );

    assert!(
        apart(both, body_alone) < apart(both, note_alone),
        "the note under the body was not dimmed by it: {both:?}, \
         body {body_alone:?}, note {note_alone:?}"
    );
}

fn note_before_slider(time_ms: f64, slider: bool) -> (u8, u8, u8) {
    let mut objects = String::from("256,192,2000,5,0\n");
    if slider {
        objects.push_str("120,192,2200,2,0,L|400:192,1,280\n");
    }
    let map = beatmap(&format!(
        "
[Difficulty]
CircleSize:4
ApproachRate:5
OverallDifficulty:5
SliderMultiplier:0.4
SliderTickRate:1

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
{objects}"
    ));
    let state = GameState::from_beatmap(&map, Mods::default());
    let layout = Layout::new(640, 480);
    let frame = Scene::new(&state, Skin::default().with_font(font())).frame(time_ms, &layout);
    let (cx, cy) = layout.map(dossier_beatmap::Point { x: 256.0, y: 192.0 });
    let p = frame.pixel(cx as u32, cy as u32).expect("inside the frame");
    (p.red(), p.green(), p.blue())
}

#[test]
fn a_note_still_to_be_hit_is_above_the_body_of_a_later_slider() {
    let with_body = note_before_slider(1990.0, true);
    let alone = note_before_slider(1990.0, false);
    assert!(
        apart(with_body, alone) < 30,
        "a later slider's body covered a note still to be hit: {with_body:?} against {alone:?}"
    );
}

#[test]
fn each_mark_plays_the_animation_from_its_own_beginning() {
    let dir = skin_folder("trail-frames");
    write_element(&dir, "followpoint-0.png", 32, 0);
    for frame in 1..10 {
        write_element(&dir, &format!("followpoint-{frame}.png"), 32, 255);
    }

    let map = beatmap(
        "
[Difficulty]
CircleSize:4
ApproachRate:5

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
100,192,3000,5,0
400,192,5000,1,0
",
    );

    let silent = skin_folder("trail-frames-off");
    for frame in 0..10 {
        write_element(&silent, &format!("followpoint-{frame}.png"), 32, 0);
    }
    assert!(
        trail_ink(Some(&dir), &map, 4000.0) > trail_ink(Some(&silent), &map, 4000.0),
        "the whole trail vanished on a frame the skin drew empty"
    );
}

fn across_body(at: f32) -> (u8, u8, u8) {
    let map = beatmap(
        "
[Difficulty]
CircleSize:4
ApproachRate:5
SliderMultiplier:1.4
SliderTickRate:1

[Colours]
Combo1 : 0,120,255

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
150,120,2000,2,0,L|400:120,1,250
",
    );
    let state = GameState::from_beatmap(&map, Mods::default());
    let layout = Layout::new(1280, 720);
    let frame =
        Scene::new(&state, Skin::with_combo_colours(map.combo_colours())).frame(2000.0, &layout);

    let radius = state.difficulty().circle_radius();
    let (cx, cy) = layout.map(dossier_beatmap::Point { x: 280.0, y: 120.0 });
    let half = layout.length(radius);
    let y = cy - half * (1.0 - at);
    let p = frame
        .pixel(cx as u32, y.round() as u32)
        .expect("inside the frame");
    (p.red(), p.green(), p.blue())
}

#[test]
fn the_border_is_a_band_of_one_colour_rather_than_a_fade() {
    let inner = across_body(0.10);
    let outer = across_body(0.17);
    assert!(
        apart(inner, outer) < 12,
        "the border is not one colour across its width: {inner:?} against {outer:?}"
    );
}

fn body_alpha_across() -> [f32; 3] {
    let map = beatmap(
        "
[Difficulty]
CircleSize:4
ApproachRate:5
SliderMultiplier:0.4
SliderTickRate:1

[Colours]
Combo1 : 255,0,0

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
120,192,2000,2,0,L|400:192,1,280
",
    );
    let state = GameState::from_beatmap(&map, Mods::default());
    let layout = Layout::new(640, 480);
    let sample = |bg: u8| {
        let mut skin = Skin::with_combo_colours(map.combo_colours());
        skin.background = tiny_skia::Color::from_rgba8(bg, bg, bg, 255);
        let frame = Scene::new(&state, skin).frame(2300.0, &layout);
        let (cx, cy) = layout.map(dossier_beatmap::Point { x: 300.0, y: 192.0 });

        [0.05f32, 0.12, 0.9].map(|at| {
            let half = layout.length(state.difficulty().circle_radius());
            let y = cy - half * (1.0 - at);
            let p = frame.pixel(cx as u32, y.round() as u32).expect("in frame");
            f32::from(p.red())
        })
    };
    let dark = sample(0);
    let light = sample(255);
    [0, 1, 2].map(|i| 1.0 - (light[i] - dark[i]) / 255.0)
}

#[test]
fn a_slider_body_dims_what_it_passes_over_rather_than_covering_it() {
    let [shadow, border, track] = body_alpha_across();
    assert!(
        (track - 0.70).abs() < 0.03,
        "the track is not seven tenths opaque: {track}"
    );
    assert!(border > 0.97, "the border is solid: {border}");
    assert!(
        shadow < 0.30,
        "and the shadow is a hint rather than a wall: {shadow}"
    );
}

#[test]
fn the_ring_closing_in_is_never_dimmed_by_a_track() {
    let sample = |slider: bool| {
        let mut objects = String::from("256,192,2600,5,0\n");
        if slider {
            objects.push_str("120,192,2000,2,0,L|400:192,1,280\n");
        }
        let map = beatmap(&format!(
            "
[Difficulty]
CircleSize:4
ApproachRate:5
SliderMultiplier:0.4
SliderTickRate:1

[Colours]
Combo1 : 255,255,255

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
{objects}"
        ));
        let state = GameState::from_beatmap(&map, Mods::default());
        let layout = Layout::new(640, 480);
        let skin = Skin::with_combo_colours(map.combo_colours()).with_font(font());
        let frame = Scene::new(&state, skin).frame(2300.0, &layout);

        let radius = state.difficulty().circle_radius();
        let progress = 1.0 - (2600.0 - 2300.0) / state.difficulty().preempt_ms();
        let scale = 1.0 + 3.0 * (1.0 - progress.clamp(0.0, 1.0));
        let (cx, cy) = layout.map(dossier_beatmap::Point { x: 256.0, y: 192.0 });
        let out = layout.length(radius * scale);

        let mut best = (0u8, 0u8, 0u8);
        for step in -4i32..=4 {
            let y = cy - out + step as f32;
            if let Some(p) = frame.pixel(cx as u32, y.round().max(0.0) as u32) {
                if u32::from(p.red()) + u32::from(p.green()) + u32::from(p.blue())
                    > u32::from(best.0) + u32::from(best.1) + u32::from(best.2)
                {
                    best = (p.red(), p.green(), p.blue());
                }
            }
        }
        best
    };
    let over_body = sample(true);
    let alone = sample(false);
    assert!(
        u32::from(alone.0) + u32::from(alone.1) + u32::from(alone.2) > 60,
        "the ring is drawn at all: {alone:?}"
    );
    assert!(
        apart(over_body, alone) < 20,
        "the track darkened the ring closing in: {over_body:?} against {alone:?}"
    );
}

#[test]
fn a_skins_own_slider_tick_is_drawn_in_place_of_ours() {
    use dossier_render::elements::Element;
    use dossier_render::imported::Sprites;

    let map = beatmap(LONE_SLIDER);
    let state = GameState::from_beatmap(&map, Mods::default());
    let object = &state.timeline().objects[0];
    let far_tick = *object.tick_times().last().expect("this slider has ticks");
    let at = object.ball_at(far_tick).expect("on the path");

    let lit = |skin: Skin| {
        let background = skin.background;
        let layout = Layout::new(640, 480);
        let scene = Scene::new(&state, skin);
        let (x, y) = layout.map(at);
        let bg = background.to_color_u8();

        let frame = scene.frame(far_tick - 20.0, &layout);
        let p = frame
            .pixel(x as u32 + 52, y as u32)
            .expect("inside the frame");
        i32::from(p.red()) - i32::from(bg.red()) + i32::from(p.green()) - i32::from(bg.green())
            + i32::from(p.blue())
            - i32::from(bg.blue())
    };

    let ours = lit(Skin::default());

    let dir = skin_folder("tick");
    write_element(&dir, "sliderscorepoint.png", 512, 255);
    let mut dressed = Skin::with_combo_colours(map.combo_colours());
    let sprites =
        Sprites::read(&dir, &[Element::SliderScorePoint]).tint_for(&dressed.combo_colours);
    dressed.sprites = Some(std::sync::Arc::new(sprites));
    let theirs = lit(dressed);

    assert!(
        theirs > ours + 30,
        "the skin's own tick was not drawn: {theirs} against {ours}"
    );
}

fn scored_mark_width(circle_size: &str, dir: &std::path::Path) -> usize {
    use dossier_render::elements::Element;
    use dossier_render::elements::Verdict;
    use dossier_render::imported::Sprites;

    let map = beatmap(&format!(
        "
[Difficulty]
ApproachRate:5
OverallDifficulty:5
CircleSize:{circle_size}

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
256,192,3000,5,0
256,192,9000,5,0
"
    ));

    let mut frames = Vec::new();
    for i in 0..40 {
        let at = 2000 + i64::from(i) * 100;
        let on_note = at <= 3000;
        frames.push(dossier_replay::ReplayFrame {
            time_ms: at,
            x: if on_note { 256.0 } else { 20.0 },
            y: if on_note { 192.0 } else { 20.0 },
            keys: dossier_replay::Keys(if at == 3000 {
                dossier_replay::Keys::K1
            } else {
                0
            }),
        });
    }
    let replay = replay_over(frames);

    let mut skin = Skin::with_combo_colours(map.combo_colours()).with_font(font());
    let all: Vec<Element> = Verdict::ALL.iter().copied().map(Element::Verdict).collect();
    let sprites = Sprites::read(dir, &all).tint_for(&skin.combo_colours);
    skin.sprites = Some(std::sync::Arc::new(sprites));

    let state = GameState::new(&map, &replay);
    let layout = Layout::new(640, 480);
    let frame = Scene::new(&state, skin).frame(3400.0, &layout);

    let (cx, cy) = layout.map(dossier_beatmap::Point::CENTRE);
    (0..640u32)
        .filter(|&x| {
            frame.pixel(x, cy as u32).is_some_and(|p| {
                (i32::from(p.red()) - i32::from(FIELD.0)).abs()
                    + (i32::from(p.green()) - i32::from(FIELD.1)).abs()
                    + (i32::from(p.blue()) - i32::from(FIELD.2)).abs()
                    > 12
            }) && x.abs_diff(cx as u32) < 200
        })
        .count()
}

fn write_padded(dir: &std::path::Path, name: &str, canvas: u32, ink: u32) {
    write_ink(dir, name, canvas, ink, ink);
}

fn write_ink(dir: &std::path::Path, name: &str, canvas: u32, wide: u32, tall: u32) {
    let mut art = tiny_skia::Pixmap::new(canvas, canvas).expect("a canvas");
    let (from_x, from_y) = ((canvas - wide) / 2, (canvas - tall) / 2);
    for y in from_y..from_y + tall {
        for x in from_x..from_x + wide {
            art.pixels_mut()[(y * canvas + x) as usize] =
                tiny_skia::PremultipliedColorU8::from_rgba(255, 255, 255, 255).expect("white");
        }
    }
    std::fs::write(dir.join(name), art.encode_png().expect("png")).expect("written");
}

#[test]
fn squat_lettering_is_held_by_its_width_too() {
    let squat = skin_folder("verdict-squat");
    write_ink(&squat, "hit300.png", 200, 120, 28);

    let upright = skin_folder("verdict-upright");
    write_ink(&upright, "hit300.png", 200, 120, 80);

    let held = scored_mark_width("6", &squat);
    let tall = scored_mark_width("6", &upright);
    assert!(held > 0, "the mark is drawn at all");
    assert!(
        held.abs_diff(tall) <= 2,
        "the same picture came out two sizes: {held} squat against {tall} upright"
    );
}

#[test]
fn the_widest_mark_brings_its_siblings_down_with_it() {
    let together = skin_folder("verdict-set");
    write_ink(&together, "hit300.png", 200, 120, 28);
    write_ink(&together, "hit0.png", 200, 40, 28);

    let alone = skin_folder("verdict-set-alone");
    write_ink(&alone, "hit0.png", 200, 40, 28);

    let long = scored_mark_width("6", &together);
    let short = miss_mark_width("6", &together);
    assert!(short > 0, "the compact mark is drawn at all");
    assert!(
        long > short,
        "the set was squeezed to a common width: {long} against {short}"
    );
    assert!(
        short < miss_mark_width("6", &alone),
        "a mark whose sibling was held did not come down with it"
    );
}

#[test]
fn a_mark_past_the_ceiling_is_brought_down_to_it() {
    let dir = skin_folder("verdict-share");
    write_padded(&dir, "hit300.png", 200, 40);
    write_padded(&dir, "hit0.png", 200, 40);

    let big = skin_folder("verdict-share-big");
    write_padded(&big, "hit300.png", 200, 80);
    write_padded(&big, "hit0.png", 200, 80);

    for measure in [
        scored_mark_width as fn(&str, &std::path::Path) -> usize,
        miss_mark_width as fn(&str, &std::path::Path) -> usize,
    ] {
        let modest = measure("6", &dir);
        let huge = measure("6", &big);
        assert!(modest > 0, "the mark is drawn at all");
        assert!(
            huge <= modest + 2,
            "a mark twice as wide was not brought down: {huge} against {modest}"
        );
    }

    let small = skin_folder("verdict-share-modest");
    write_padded(&small, "hit300.png", 200, 8);
    write_padded(&small, "hit0.png", 200, 8);
    assert!(
        scored_mark_width("6", &small) < scored_mark_width("6", &dir),
        "a mark already inside the ceiling was resized anyway"
    );
}

#[test]
fn a_disjoint_trail_reaches_back_the_time_the_game_gives_it() {
    let map = beatmap(
        "
[Difficulty]
CircleSize:4
ApproachRate:5

[HitObjects]
256,192,1500,1,0
256,192,3000,1,0
",
    );

    let speed = 0.6_f32;
    let frames: Vec<_> = (0..200)
        .map(|i| dossier_replay::ReplayFrame {
            time_ms: 1000 + i64::from(i) * 5,
            x: 20.0 + speed * (i as f32) * 5.0,

            y: 60.0,
            keys: dossier_replay::Keys(0),
        })
        .collect();
    let state = GameState::new(&map, &replay_over(frames));

    let skin = Skin::default();
    let background = skin.background;
    let layout = Layout::new(640, 480);
    let scene = Scene::new(&state, skin);
    let at = 1800.0;
    let frame = scene.frame(at, &layout);

    let here = state
        .cursor_track()
        .sample(at)
        .expect("the cursor is on the field")
        .pos;
    let (cx, cy) = layout.map(here);
    let bg = background.to_color_u8();

    let reach = (0..cx as u32)
        .filter(|&x| {
            frame.pixel(x, cy as u32).is_some_and(|p| {
                (i32::from(p.red()) - i32::from(bg.red())).abs()
                    + (i32::from(p.green()) - i32::from(bg.green())).abs()
                    + (i32::from(p.blue()) - i32::from(bg.blue())).abs()
                    > 10
            })
        })
        .min()
        .map(|x| cx - x as f32);
    let reach = reach.expect("the trail is drawn at all");

    let back = |ms: f64| layout.length(f64::from(speed) * ms) as f64;
    assert!(
        reach as f64 > back(120.0),
        "the trail stops short: {reach} against {} for 120ms",
        back(120.0)
    );
    assert!(
        (reach as f64) < back(200.0),
        "the trail runs past its life: {reach} against {} for 200ms",
        back(200.0)
    );
}

#[test]
fn a_long_break_ends_on_the_skins_own_section_banner() {
    use dossier_render::elements::Element;
    use dossier_render::imported::Sprites;

    let map = beatmap(BREAK_MAP);
    let state = GameState::from_beatmap(&map, Mods::default());
    let (from, to) = state.timeline().breaks[0];
    assert!(to - from > 2880.0, "the fixture's break is long enough");
    let at = (to - 2880.0).min(to - (to - from) / 2.0);

    let dir = skin_folder("section");
    write_element(&dir, "section-pass.png", 400, 255);
    let mut skin = Skin::with_combo_colours(map.combo_colours());
    let sprites = Sprites::read(&dir, &[Element::SectionPass]).tint_for(&skin.combo_colours);
    skin.sprites = Some(std::sync::Arc::new(sprites));
    let background = skin.background;
    let layout = Layout::new(640, 480);
    let scene = Scene::new(&state, skin);

    let bg = background.to_color_u8();
    let lit = |t: f64| {
        let (x, y) = layout.map(dossier_beatmap::Point::CENTRE);
        scene
            .frame(t, &layout)
            .pixel(x as u32, y as u32)
            .map_or(0, |p| {
                (i32::from(p.red()) - i32::from(bg.red())).abs()
                    + (i32::from(p.green()) - i32::from(bg.green())).abs()
                    + (i32::from(p.blue()) - i32::from(bg.blue())).abs()
            })
    };

    assert!(lit(at - 200.0) < 20, "the banner was up before its moment");
    assert!(lit(at + 400.0) > 60, "the banner never appeared");

    assert!(lit(at + 130.0) < 20, "the blink does not blink");

    assert!(lit(at + 1600.0) < 20, "the banner outstayed its fade");
}

#[test]
fn a_short_break_gets_no_banner_at_all() {
    use dossier_render::elements::Element;
    use dossier_render::imported::Sprites;

    let map = beatmap(
        "
[Difficulty]
CircleSize:5
ApproachRate:5

[Events]
2,3000,5000

[HitObjects]
100,100,2000,1,0
400,300,9000,1,0
",
    );
    let state = GameState::from_beatmap(&map, Mods::default());
    let (from, to) = state.timeline().breaks[0];
    assert!(to - from < 2880.0, "the fixture's break is short enough");

    let dir = skin_folder("section-short");
    write_element(&dir, "section-pass.png", 400, 255);
    let mut skin = Skin::with_combo_colours(map.combo_colours());
    let sprites = Sprites::read(&dir, &[Element::SectionPass]).tint_for(&skin.combo_colours);
    skin.sprites = Some(std::sync::Arc::new(sprites));
    let background = skin.background;
    let layout = Layout::new(640, 480);
    let scene = Scene::new(&state, skin);

    let bg = background.to_color_u8();
    let (x, y) = layout.map(dossier_beatmap::Point::CENTRE);
    for step in 0..40 {
        let t = from + (to - from) * f64::from(step) / 40.0;
        let lit = scene
            .frame(t, &layout)
            .pixel(x as u32, y as u32)
            .map_or(0, |p| {
                (i32::from(p.red()) - i32::from(bg.red())).abs()
                    + (i32::from(p.green()) - i32::from(bg.green())).abs()
                    + (i32::from(p.blue()) - i32::from(bg.blue())).abs()
            });
        assert!(lit < 20, "a short break drew a banner at {t}");
    }
}

#[test]
fn a_trail_mark_is_the_size_the_skin_drew_it() {
    use dossier_render::elements::Element;
    use dossier_render::imported::Sprites;

    let map = beatmap(
        "
[Difficulty]
CircleSize:4
ApproachRate:5

[HitObjects]
256,192,1500,1,0
256,192,4000,1,0
",
    );

    let frames: Vec<_> = (0..200)
        .map(|i| dossier_replay::ReplayFrame {
            time_ms: 1000 + i64::from(i) * 5,
            x: 20.0 + 0.6 * (i as f32) * 5.0,
            y: 60.0,
            keys: dossier_replay::Keys(0),
        })
        .collect();
    let state = GameState::new(&map, &replay_over(frames));

    let dir = skin_folder("trail-size");
    write_element(&dir, "cursortrail.png", 64, 255);
    let mut skin = Skin::with_combo_colours(map.combo_colours());
    let sprites = Sprites::read(&dir, &[Element::CursorTrail]).tint_for(&skin.combo_colours);
    skin.sprites = Some(std::sync::Arc::new(sprites));
    let background = skin.background;
    let layout = Layout::new(640, 480);
    let scene = Scene::new(&state, skin);

    let at = 1800.0;
    let track = state.cursor_track();

    let oldest = track.sample(at - 140.0).expect("on the field").pos;
    let (ox, oy) = layout.map(oldest);
    let bg = background.to_color_u8();
    let frame = scene.frame(at, &layout);

    let lit = (0..480u32)
        .filter(|&y| {
            y.abs_diff(oy as u32) < 60
                && frame.pixel(ox as u32, y).is_some_and(|p| {
                    (i32::from(p.red()) - i32::from(bg.red())).abs()
                        + (i32::from(p.green()) - i32::from(bg.green())).abs()
                        + (i32::from(p.blue()) - i32::from(bg.blue())).abs()
                        > 6
                })
        })
        .count();

    let expected = 64.0 * 480.0 / 768.0;
    assert!(
        (lit as f32 - expected).abs() < expected * 0.35,
        "the mark is {lit} tall where {expected:.0} was stated"
    );
}

fn write_glyph(dir: &std::path::Path, name: &str, size: u32, colour: (u8, u8, u8)) {
    let mut pixmap = tiny_skia::Pixmap::new(size, size).expect("a canvas");
    for pixel in pixmap.pixels_mut() {
        *pixel = tiny_skia::PremultipliedColorU8::from_rgba(colour.0, colour.1, colour.2, 255)
            .expect("a colour");
    }
    std::fs::write(dir.join(name), pixmap.encode_png().expect("png")).expect("written");
}

fn combo_corner(dir: &std::path::Path, colour: (u8, u8, u8), slack: i32) -> usize {
    use dossier_render::elements::Element;
    use dossier_render::imported::Sprites;
    let (map, replay) = tapped();
    let mut skin = Skin::with_combo_colours(map.combo_colours()).with_font(font());
    let wanted: Vec<Element> = ('0'..='9')
        .chain([',', '.', '%', 'x'])
        .flat_map(|c| [Element::Score(c), Element::Combo(c)])
        .collect();
    skin.sprites = Some(std::sync::Arc::new(
        Sprites::read(dir, &wanted).tint_for(&skin.combo_colours),
    ));
    let state = GameState::new(&map, &replay);
    let frame = Scene::new(&state, skin).frame(4200.0, &Layout::new(640, 480));

    let mut count = 0;
    for y in 380..480u32 {
        for x in 0..240u32 {
            let Some(p) = frame.pixel(x, y) else { continue };
            let off = (i32::from(p.red()) - i32::from(colour.0)).abs()
                + (i32::from(p.green()) - i32::from(colour.1)).abs()
                + (i32::from(p.blue()) - i32::from(colour.2)).abs();
            if off <= slack {
                count += 1;
            }
        }
    }
    count
}

const COMBO_FACE: (u8, u8, u8) = (255, 0, 0);
const SCORE_FACE: (u8, u8, u8) = (0, 0, 255);

#[test]
fn the_counter_is_drawn_in_the_face_the_skin_named_for_it() {
    let dir = skin_folder("two-faces");
    for digit in 0..10 {
        write_glyph(&dir, &format!("score-{digit}.png"), 24, SCORE_FACE);
        write_glyph(&dir, &format!("combo-{digit}.png"), 24, COMBO_FACE);
    }
    write_glyph(&dir, "score-x.png", 24, SCORE_FACE);
    write_glyph(&dir, "combo-x.png", 24, COMBO_FACE);
    std::fs::write(
        dir.join("skin.ini"),
        "[Fonts]\nScorePrefix: score\nComboPrefix: combo\n",
    )
    .expect("written");

    let theirs = combo_corner(&dir, COMBO_FACE, 40);
    let scores = combo_corner(&dir, SCORE_FACE, 40);
    assert!(
        theirs > 100,
        "the combo face is barely there: {theirs} pixels"
    );
    assert_eq!(scores, 0, "the score face turned up in the combo's corner");
}

#[test]
fn a_glyph_the_face_has_not_got_does_not_take_the_line_with_it() {
    let dir = skin_folder("no-x");
    for digit in 0..10 {
        write_glyph(&dir, &format!("score-{digit}.png"), 24, COMBO_FACE);
    }
    std::fs::write(dir.join("skin.ini"), "[General]\nVersion: 2.5\n").expect("written");

    assert!(
        combo_corner(&dir, COMBO_FACE, 40) > 100,
        "the line fell back to our typeface over one missing sign"
    );
}

#[test]
fn a_face_the_skin_has_none_of_is_still_ours_to_draw() {
    let dir = skin_folder("no-face");
    write_element(&dir, "hitcircle.png", 128, 255);
    assert_eq!(
        combo_corner(&dir, COMBO_FACE, 40),
        0,
        "this skin has no red in it at all"
    );

    let (map, replay) = tapped();
    let mut skin = Skin::with_combo_colours(map.combo_colours()).with_font(font());
    skin.sprites = Some(std::sync::Arc::new(
        dossier_render::imported::Sprites::read(
            &dir,
            &[dossier_render::elements::Element::HitCircle],
        )
        .tint_for(&skin.combo_colours),
    ));
    let state = GameState::new(&map, &replay);
    let frame = Scene::new(&state, skin).frame(4200.0, &Layout::new(640, 480));
    let lit = (380..480u32)
        .flat_map(|y| (0..240u32).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            frame
                .pixel(x, y)
                .is_some_and(|p| (i32::from(p.red()) - i32::from(FIELD.0)).abs() > 20)
        })
        .count();
    assert!(lit > 50, "nothing was drawn in the combo's corner at all");
}

#[test]
fn a_skins_scorebar_keeps_its_own_size_and_corner() {
    let dir = skin_folder("scorebar");
    let mut picture = tiny_skia::Pixmap::new(400, 200).expect("a canvas");
    for pixel in picture.pixels_mut() {
        *pixel = tiny_skia::PremultipliedColorU8::from_rgba(0, 200, 0, 255).expect("a colour");
    }
    std::fs::write(
        dir.join("scorebar-bg.png"),
        picture.encode_png().expect("png"),
    )
    .expect("written");

    let (map, replay) = tapped();
    let mut skin = Skin::with_combo_colours(map.combo_colours()).with_font(font());
    skin.sprites = Some(std::sync::Arc::new(
        dossier_render::imported::Sprites::read(
            &dir,
            &[dossier_render::elements::Element::ScoreBarBackground],
        )
        .tint_for(&skin.combo_colours),
    ));
    let state = GameState::new(&map, &replay);
    let frame = Scene::new(&state, skin).frame(4200.0, &Layout::new(640, 480));

    let green = |x: u32, y: u32| {
        frame
            .pixel(x, y)
            .is_some_and(|p| p.green() > 120 && p.red() < 80)
    };
    assert!(green(1, 1), "it does not start in the corner");
    assert!(green(240, 118), "it is short of the size it was drawn at");
    assert!(!green(258, 118), "it runs past its own width");
    assert!(!green(240, 132), "it runs past its own height");
}

fn covered_note(above: Option<bool>) -> Skin {
    use dossier_render::elements::Element;
    use dossier_render::imported::Sprites;
    let dir = skin_folder(match above {
        Some(true) => "rim-over",
        Some(false) => "rim-under",
        None => "rim-unsaid",
    });
    write_glyph(&dir, "hitcircle.png", 128, (0, 0, 90));
    write_glyph(&dir, "hitcircleoverlay.png", 128, (0, 200, 0));
    for digit in 0..10 {
        write_glyph(&dir, &format!("default-{digit}.png"), 48, (255, 0, 0));
    }
    if let Some(above) = above {
        std::fs::write(
            dir.join("skin.ini"),
            format!(
                "[General]\nHitCircleOverlayAboveNumber: {}\n",
                u8::from(above)
            ),
        )
        .expect("written");
    }

    let map = one_note();
    let mut skin = Skin::with_combo_colours(map.combo_colours());
    let mut wanted = vec![Element::HitCircle, Element::HitCircleOverlay];
    wanted.extend((0..10).map(Element::Digit));
    skin.sprites = Some(std::sync::Arc::new(
        Sprites::read(&dir, &wanted).tint_for(&skin.combo_colours),
    ));
    skin
}

fn number_shows(skin: Skin) -> bool {
    let map = one_note();
    let state = GameState::from_beatmap(&map, Mods::default());
    let layout = Layout::new(640, 480);
    let frame = Scene::new(&state, skin).frame(5000.0, &layout);
    let (x, y) = layout.map(dossier_beatmap::Point::CENTRE);
    (0..24).any(|step| {
        frame
            .pixel(x as u32, y as u32 - step)
            .is_some_and(|p| p.red() > 120 && p.green() < 120)
    })
}

#[test]
fn a_skin_that_says_nothing_gets_its_rim_over_the_number() {
    assert!(
        !number_shows(covered_note(None)),
        "the figure came out on top of a rim that covers the note"
    );
}

#[test]
fn a_skin_can_put_its_rim_under_the_number_instead() {
    assert!(
        number_shows(covered_note(Some(false))),
        "`HitCircleOverlayAboveNumber: 0` was not honoured"
    );
    assert!(
        !number_shows(covered_note(Some(true))),
        "and 1 is the default"
    );
}

fn score_corner(dir: &std::path::Path, colour: (u8, u8, u8), slack: i32) -> usize {
    use dossier_render::elements::Element;
    use dossier_render::imported::Sprites;
    let (map, replay) = tapped();
    let mut skin = Skin::with_combo_colours(map.combo_colours()).with_font(font());
    let wanted: Vec<Element> = ('0'..='9')
        .chain([',', '.', '%', 'x'])
        .flat_map(|c| [Element::Score(c), Element::Combo(c)])
        .collect();
    skin.sprites = Some(std::sync::Arc::new(
        Sprites::read(dir, &wanted).tint_for(&skin.combo_colours),
    ));
    let state = GameState::new(&map, &replay);
    let frame = Scene::new(&state, skin).frame(4200.0, &Layout::new(640, 480));

    let mut count = 0;
    for y in 0..120u32 {
        for x in 400..640u32 {
            let Some(p) = frame.pixel(x, y) else { continue };
            let off = (i32::from(p.red()) - i32::from(colour.0)).abs()
                + (i32::from(p.green()) - i32::from(colour.1)).abs()
                + (i32::from(p.blue()) - i32::from(colour.2)).abs();
            if off <= slack {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn a_skin_that_drew_its_numbers_bigger_gets_bigger_numbers() {
    fn face(name: &str, size: u32) -> std::path::PathBuf {
        let dir = skin_folder(name);
        for digit in 0..10 {
            write_glyph(&dir, &format!("score-{digit}.png"), size, SCORE_FACE);
        }
        std::fs::write(dir.join("skin.ini"), "[Fonts]\nScorePrefix: score\n").expect("written");
        dir
    }

    let small = score_corner(&face("small-face", 20), SCORE_FACE, 40);
    let large = score_corner(&face("large-face", 40), SCORE_FACE, 40);

    assert!(
        small > 0 && large > 0,
        "both skins drew something: {small}, {large}"
    );

    assert!(
        large as f32 > small as f32 * 2.5,
        "the larger face should cover far more: {small} against {large}"
    );
}

#[test]
fn the_cursor_and_its_trail_are_read_by_the_same_ruler() {
    let dir = skin_folder("one-ruler");
    write_glyph(&dir, "cursor.png", 64, SCORE_FACE);
    let ink = |scale: f32| -> usize {
        use dossier_render::elements::Element;
        use dossier_render::imported::Sprites;
        let (map, replay) = tapped();
        let mut skin = Skin::with_combo_colours(map.combo_colours()).with_font(font());
        skin.cursor_scale = scale;
        skin.sprites = Some(std::sync::Arc::new(
            Sprites::read(&dir, &[Element::Cursor]).tint_for(&skin.combo_colours),
        ));
        let state = GameState::new(&map, &replay);
        let frame = Scene::new(&state, skin).frame(4200.0, &Layout::new(640, 480));
        let mut count = 0;
        for y in 0..480u32 {
            for x in 0..640u32 {
                if let Some(p) = frame.pixel(x, y) {
                    if p.blue() > 200 && p.red() < 60 && p.green() < 60 {
                        count += 1;
                    }
                }
            }
        }
        count
    };

    let (small, plain, large) = (ink(0.5), ink(1.0), ink(2.0));
    assert!(plain > 0, "the cursor was not drawn at all");

    assert!(
        small * 2 < plain,
        "0.5 should be far smaller: {small} against {plain}"
    );
    assert!(
        large > plain * 2,
        "2.0 should be far larger: {large} against {plain}"
    );
}

#[test]
fn a_judgement_a_skin_animated_moves() {
    use dossier_render::elements::{Element, Verdict};
    use dossier_render::imported::Sprites;

    let dir = skin_folder("moving-300");

    write_glyph(&dir, "hit300.png", 48, (255, 0, 0));
    write_glyph(&dir, "hit300-1.png", 48, (0, 255, 0));
    write_glyph(&dir, "hit300-2.png", 48, (0, 0, 255));

    std::fs::write(dir.join("skin.ini"), "[General]\nAnimationFramerate: 10\n").expect("written");

    let ink = |at: f64, colour: (u8, u8, u8)| -> usize {
        let (map, replay) = tapped();
        let mut skin = Skin::with_combo_colours(map.combo_colours()).with_font(font());

        skin.show_300 = true;
        skin.sprites = Some(std::sync::Arc::new(
            Sprites::read(&dir, &[Element::Verdict(Verdict::Three)]).tint_for(&skin.combo_colours),
        ));
        let state = GameState::new(&map, &replay);
        let frame = Scene::new(&state, skin).frame(at, &Layout::new(640, 480));
        let mut count = 0;
        for y in 0..480u32 {
            for x in 0..640u32 {
                if let Some(p) = frame.pixel(x, y) {
                    let (r, g, b) = (
                        i32::from(p.red()),
                        i32::from(p.green()),
                        i32::from(p.blue()),
                    );
                    let leads = match colour {
                        (255, 0, 0) => r > g + 20 && r > b + 20,
                        (0, 255, 0) => g > r + 20 && g > b + 20,
                        _ => b > r + 20 && b > g + 20,
                    };
                    if leads {
                        count += 1;
                    }
                }
            }
        }
        count
    };

    let green_at_first = ink(3050.0, (0, 255, 0));
    let green_later = ink(3150.0, (0, 255, 0));
    assert!(
        green_later > green_at_first * 3 && green_later > 200,
        "the strip does not advance — the still is drawn for ever: \
         {green_at_first} green on frame zero against {green_later} on frame one"
    );

    let blue_at_first = ink(3050.0, (0, 0, 255));
    let blue_past_the_end = ink(3600.0, (0, 0, 255));
    assert!(
        blue_past_the_end > blue_at_first * 3 && blue_past_the_end > 200,
        "the strip wrapped rather than holding its last frame: \
         {blue_at_first} blue at the start against {blue_past_the_end} at the end"
    );
}

#[test]
fn a_bar_file_with_something_else_in_it_draws_only_the_bar() {
    use dossier_render::elements::Element;
    use dossier_render::imported::Sprites;

    let dir = skin_folder("bar-with-an-island");

    let mut art = tiny_skia::Pixmap::new(400, 20).expect("a canvas");
    for (x, y) in (0..400u32).flat_map(|x| (0..20u32).map(move |y| (x, y))) {
        let inside = x < 200 || x >= 380;
        if inside {
            let at = (y * 400 + x) as usize;
            art.pixels_mut()[at] =
                tiny_skia::PremultipliedColorU8::from_rgba(255, 255, 255, 255).expect("a colour");
        }
    }
    std::fs::write(dir.join("scorebar-bg.png"), art.encode_png().expect("png")).expect("written");

    let (map, replay) = tapped();
    let mut skin = Skin::with_combo_colours(map.combo_colours()).with_font(font());
    skin.sprites = Some(std::sync::Arc::new(
        Sprites::read(&dir, &[Element::ScoreBarBackground]).tint_for(&skin.combo_colours),
    ));
    let state = GameState::new(&map, &replay);
    let frame = Scene::new(&state, skin).frame(4200.0, &Layout::new(640, 480));

    let lit = |x: u32| {
        (0..40u32).any(|y| {
            frame
                .pixel(x, y)
                .is_some_and(|p| p.red() > 200 && p.green() > 200)
        })
    };

    assert!(lit(20), "the bar itself was not drawn");
    assert!(
        !(235..252).any(lit),
        "the island past the gap was drawn as well"
    );
}

#[test]
fn the_follow_circle_beats_on_a_tick_and_leaves_at_the_end() {
    let map = beatmap(
        "
[Difficulty]
ApproachRate:5
CircleSize:4
SliderMultiplier:1.0
SliderTickRate:1

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
100,192,2000,2,0,L|400:192,1,300
",
    );

    let frames: Vec<_> = (0..80)
        .map(|i| dossier_replay::ReplayFrame {
            time_ms: 1900 + i64::from(i) * 25,
            x: 100.0 + 300.0 * ((i as f32 - 4.0) / 60.0).clamp(0.0, 1.0),
            y: 192.0,
            keys: dossier_replay::Keys(if i >= 4 { dossier_replay::Keys::K1 } else { 0 }),
        })
        .collect();
    let state = GameState::new(&map, &replay_over(frames));

    let ends = state.timeline().objects[0].end_ms;
    let ticks: Vec<f64> = state
        .judge()
        .expect("judged")
        .events_for(0)
        .filter(|e| e.part == dossier_sim::Part::SliderTick)
        .map(|e| e.time_ms)
        .collect();
    assert!(!ticks.is_empty(), "the fixture has no ticks to beat on");

    let skin = Skin::with_combo_colours(map.combo_colours()).with_font(font());
    let layout = Layout::new(640, 480);

    let reach = |at: f64| -> u32 {
        let frame = Scene::new(&state, skin.clone()).frame(at, &layout);
        let ball = state.timeline().objects[0]
            .ball_at(at.min(ends))
            .expect("a ball");
        let (bx, by) = layout.map(ball);

        (0..200u32)
            .rev()
            .find(|&d| {
                [by as u32 + d, (by as u32).saturating_sub(d)]
                    .iter()
                    .any(|&y| frame.pixel(bx as u32, y).is_some_and(|p| p.red() > 40))
            })
            .unwrap_or(0)
    };

    let tick = ticks[0];
    let (before, on) = (reach(tick - 60.0), reach(tick + 5.0));
    println!(
        "КОЛЬЦО: до тика {before}, на тике {on}, после конца {}",
        reach(ends + 100.0)
    );
    assert!(
        on > before,
        "the ring did not move on a tick: {before} then {on}"
    );

    let note = layout.length(state.difficulty().circle_radius());
    let (leaving, left) = (reach(ends + 100.0), reach(ends + 400.0));
    println!("КОЛЬЦО НА ВЫХОДЕ: {leaving} потом {left}, нота {note:.0}");
    assert!(
        leaving > note as u32,
        "the ring vanished the instant the ball did: {leaving} against a note of {note:.0}"
    );
    assert!(
        left <= note as u32,
        "the ring never left: still reaching {left} past a note of {note:.0}"
    );
}

fn write_glyph_sized(
    dir: &std::path::Path,
    name: &str,
    width: u32,
    height: u32,
    colour: (u8, u8, u8),
) {
    let mut pixmap = tiny_skia::Pixmap::new(width, height).expect("a canvas");
    for pixel in pixmap.pixels_mut() {
        *pixel = tiny_skia::PremultipliedColorU8::from_rgba(colour.0, colour.1, colour.2, 255)
            .expect("a colour");
    }
    std::fs::write(dir.join(name), pixmap.encode_png().expect("png")).expect("written");
}

fn dial_and_accuracy(dir: &std::path::Path) -> (Option<u32>, Option<u32>) {
    use dossier_render::elements::Element;
    use dossier_render::imported::Sprites;
    let (map, replay) = tapped();
    let mut skin = Skin::with_combo_colours(map.combo_colours()).with_font(font());
    let wanted: Vec<Element> = ('0'..='9')
        .chain([',', '.', '%', 'x'])
        .map(Element::Score)
        .collect();
    skin.sprites = Some(std::sync::Arc::new(
        Sprites::read(dir, &wanted).tint_for(&skin.combo_colours),
    ));
    let state = GameState::new(&map, &replay);
    let frame = Scene::new(&state, skin).frame(4200.0, &Layout::new(640, 480));

    let (mut dial_right, mut accuracy_left) = (None, None);

    for y in 0..120u32 {
        for x in 320..640u32 {
            let Some(p) = frame.pixel(x, y) else { continue };
            let (r, g, b) = (
                i32::from(p.red()),
                i32::from(p.green()),
                i32::from(p.blue()),
            );
            let blue = b - r.max(g) > 60;
            let grey = (r - g).abs() < 24 && (g - b).abs() < 24 && r + g + b > 120;
            if blue {
                accuracy_left = Some(accuracy_left.map_or(x, |had: u32| had.min(x)));
            } else if grey {
                dial_right = Some(dial_right.map_or(x, |had: u32| had.max(x)));
            }
        }
    }
    (dial_right, accuracy_left)
}

#[test]
fn the_dial_stays_clear_of_a_skins_own_accuracy_however_wide_it_is() {
    let dir = skin_folder("wide-percent");

    for digit in 0..10 {
        write_glyph_sized(&dir, &format!("score-{digit}.png"), 96, 24, (0, 0, 255));
    }
    for (name, glyph) in [
        ("score-percent", '%'),
        ("score-dot", '.'),
        ("score-comma", ','),
    ] {
        let _ = glyph;
        write_glyph_sized(&dir, &format!("{name}.png"), 96, 24, (0, 0, 255));
    }

    let (dial_right, accuracy_left) = dial_and_accuracy(&dir);
    let dial_right = dial_right.expect("the dial was not drawn at all");
    let accuracy_left = accuracy_left.expect("the skin's accuracy was not drawn");
    assert!(
        dial_right < accuracy_left,
        "the dial runs into the accuracy: it ends at {dial_right} and the \
         accuracy starts at {accuracy_left}"
    );
}

fn field_corners(width: u32, height: u32) -> (f32, f32, f32, f32) {
    use dossier_beatmap::{Point, PLAYFIELD_HEIGHT, PLAYFIELD_WIDTH};
    let layout = Layout::new(width, height);
    let (x0, y0) = layout.map(Point { x: 0.0, y: 0.0 });
    let (x1, y1) = layout.map(Point {
        x: PLAYFIELD_WIDTH,
        y: PLAYFIELD_HEIGHT,
    });
    (x0, y0, x1, y1)
}

#[test]
fn the_field_stays_on_screen_at_any_shape_of_frame() {
    let shapes = [
        (1920, 1080, "16:9, the ordinary one"),
        (1280, 720, "16:9, small"),
        (1440, 1080, "4:3, somebody's old monitor"),
        (
            1080,
            1920,
            "9:16, vertical — the one non-standard sizes are for",
        ),
        (1080, 1080, "square"),
        (3840, 720, "absurdly wide"),
        (256, 256, "the smallest the bot accepts"),
        (3840, 2160, "the largest"),
        (1600, 900, "16:9, an odd multiple"),
        (1366, 768, "the laptop panel that is not quite 16:9"),
    ];
    for (width, height, what) in shapes {
        let (x0, y0, x1, y1) = field_corners(width, height);
        assert!(
            x0 >= 0.0 && y0 >= 0.0,
            "{what} ({width}×{height}): the field starts off the frame at ({x0}, {y0})"
        );
        assert!(
            x1 <= width as f32 && y1 <= height as f32,
            "{what} ({width}×{height}): the field ends off the frame at ({x1}, {y1})"
        );
        assert!(
            x1 - x0 > 1.0 && y1 - y0 > 1.0,
            "{what} ({width}×{height}): the field collapsed to {}×{}",
            x1 - x0,
            y1 - y0
        );

        let (left, right) = (x0, width as f32 - x1);
        assert!(
            (left - right).abs() < 1.0,
            "{what} ({width}×{height}): the field is off-centre by {}",
            left - right
        );
    }
}

#[test]
fn a_vertical_frame_actually_renders() {
    let (map, replay) = tapped();
    let state = GameState::new(&map, &replay);
    let skin = Skin::with_combo_colours(map.combo_colours()).with_font(font());
    let frame = Scene::new(&state, skin).frame(4200.0, &Layout::new(608, 1080));

    let lit = (0..frame.height())
        .flat_map(|y| (0..frame.width()).map(move |x| (x, y)))
        .filter(|&(x, y)| frame.pixel(x, y).is_some_and(|p| p.alpha() > 0))
        .count();
    assert!(
        lit > 608 * 1080 / 2,
        "a vertical frame came out mostly empty: {lit} pixels drawn"
    );
}

fn held_slider() -> (Beatmap, dossier_replay::Replay) {
    let map = beatmap(
        "
[Difficulty]
CircleSize:5
ApproachRate:5
SliderMultiplier:1.4
SliderTickRate:1

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
100,192,1000,2,0,L|240:192,1,140
",
    );
    let held = |t: i64, x: f32, down: bool| dossier_replay::ReplayFrame {
        time_ms: t,
        x,
        y: 192.0,
        keys: dossier_replay::Keys(if down { dossier_replay::Keys::K1 } else { 0 }),
    };
    let mut frames = vec![held(990, 100.0, false)];

    for step in 0..=60 {
        let along = 100.0 + 140.0 * (step as f32 / 60.0);
        frames.push(held(1000 + step as i64 * 10, along, true));
    }
    frames.push(held(1700, 240.0, false));
    (map, replay_over(frames))
}

fn verdict_ink_at(time_ms: f64, x: f64, y: f64) -> usize {
    let (map, replay) = held_slider();
    let state = GameState::new(&map, &replay);
    let layout = Layout::new(640, 480);
    let frame = Scene::new(&state, Skin::default().with_font(font())).frame(time_ms, &layout);
    let (cx, cy) = layout.map(dossier_beatmap::Point { x, y });

    let mut count = 0;
    for dy in -34i32..34 {
        for dx in -34i32..34 {
            let Some(p) = frame.pixel((cx as i32 + dx) as u32, (cy as i32 + dy) as u32) else {
                continue;
            };

            let (r, g, b) = (
                i32::from(p.red()),
                i32::from(p.green()),
                i32::from(p.blue()),
            );
            if b > 90 && g * 5 > b * 3 && g < b && r * 2 < b {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn a_sliders_verdict_is_drawn_at_the_end_of_its_body() {
    let at_head = verdict_ink_at(1700.0, 100.0, 192.0);
    let at_tail = verdict_ink_at(1700.0, 240.0, 192.0);

    assert!(
        at_tail > 30,
        "no mark at the tail at all: {at_tail} pixels — the fixture drew nothing"
    );
    assert!(
        at_tail > at_head * 3,
        "the mark is at the head: {at_head} pixels there against {at_tail} at the tail"
    );
}

fn skinned_ink_at(dir: &std::path::Path, time_ms: f64, x: f64, y: f64) -> usize {
    use dossier_render::elements::Element;
    use dossier_render::imported::Sprites;
    let (map, replay) = held_slider();
    let mut skin = Skin::with_combo_colours(map.combo_colours()).with_font(font());
    skin.sprites = Some(std::sync::Arc::new(
        Sprites::read(
            dir,
            &[Element::Verdict(dossier_render::elements::Verdict::Three)],
        )
        .tint_for(&skin.combo_colours),
    ));
    let state = GameState::new(&map, &replay);
    let layout = Layout::new(640, 480);
    let frame = Scene::new(&state, skin).frame(time_ms, &layout);
    let (cx, cy) = layout.map(dossier_beatmap::Point { x, y });

    let mut count = 0;
    for dy in -40i32..40 {
        for dx in -40i32..40 {
            let Some(p) = frame.pixel((cx as i32 + dx) as u32, (cy as i32 + dy) as u32) else {
                continue;
            };

            if p.red() > 90 && p.blue() > 90 && p.green() < 60 {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn a_skins_own_verdict_is_drawn_at_the_end_of_a_slider_too() {
    let dir = skin_folder("verdict-at-the-end");
    let mut art = tiny_skia::Pixmap::new(64, 32).expect("a canvas");
    for pixel in art.pixels_mut() {
        *pixel = tiny_skia::PremultipliedColorU8::from_rgba(200, 0, 200, 255).expect("a colour");
    }
    std::fs::write(dir.join("hit300.png"), art.encode_png().expect("png")).expect("written");

    let at_head = skinned_ink_at(&dir, 1700.0, 100.0, 192.0);
    let at_tail = skinned_ink_at(&dir, 1700.0, 240.0, 192.0);

    assert!(
        at_tail > 100,
        "the skin's mark was not drawn at all: {at_tail}"
    );
    assert!(
        at_tail > at_head * 3,
        "the skin's mark is at the head: {at_head} there against {at_tail} at the tail"
    );
}

fn write_half_cursor(dir: &std::path::Path) {
    let mut pixmap = tiny_skia::Pixmap::new(64, 64).expect("a canvas");
    let width = pixmap.width();
    for (at, pixel) in pixmap.pixels_mut().iter_mut().enumerate() {
        let x = at as u32 % width;
        *pixel = if x < width / 2 {
            tiny_skia::PremultipliedColorU8::from_rgba(255, 255, 255, 255).expect("a colour")
        } else {
            tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 0, 0).expect("nothing")
        };
    }
    std::fs::write(dir.join("cursor.png"), pixmap.encode_png().expect("png")).expect("written");
}

fn ink_left_of_the_cursor(dir: &std::path::Path, at_ms: f64) -> u32 {
    ink_left_of_the_cursor_with(dir, at_ms, None)
}

fn ink_left_of_the_cursor_with(dir: &std::path::Path, at_ms: f64, rotate: Option<bool>) -> u32 {
    use dossier_render::elements::Element;
    use dossier_render::imported::Sprites;

    let held = dossier_beatmap::Point { x: 100.0, y: 100.0 };
    let map = beatmap(ONE_CIRCLE);
    let replay = replay_over(vec![
        dossier_replay::ReplayFrame {
            time_ms: 0,
            x: 100.0,
            y: 100.0,
            keys: dossier_replay::Keys(0),
        },
        dossier_replay::ReplayFrame {
            time_ms: 30_000,
            x: 100.0,
            y: 100.0,
            keys: dossier_replay::Keys(0),
        },
    ]);
    let state = GameState::new(&map, &replay);
    let layout = Layout::new(640, 480);

    let mut skin = Skin::with_combo_colours(map.combo_colours());
    skin.cursor_rotate = rotate;

    let sprites = Sprites::read(dir, &[Element::Cursor]).tint_for(&skin.combo_colours);
    skin.sprites = Some(std::sync::Arc::new(sprites));
    let frame = Scene::new(&state, skin).frame(at_ms, &layout);

    let (x, y) = layout.map(held);

    let mut ink = 0;
    for dx in -18..-10_i32 {
        for dy in -6..6_i32 {
            let p = frame
                .pixel((x as i32 + dx) as u32, (y as i32 + dy) as u32)
                .expect("inside the frame");
            if p.red() > 60 {
                ink += 1;
            }
        }
    }
    ink
}

#[test]
fn the_cursor_turns_the_way_the_game_turns_it() {
    let dir = skin_folder("cursor-turns");
    write_half_cursor(&dir);

    let at_rest = ink_left_of_the_cursor(&dir, 10_000.0);
    let half_a_turn = ink_left_of_the_cursor(&dir, 15_000.0);

    assert!(at_rest > 0, "the cursor was not drawn at all");
    assert!(
        half_a_turn * 4 < at_rest,
        "five seconds in, the cursor has not turned: {at_rest} against {half_a_turn}"
    );
}

#[test]
fn a_skin_that_says_not_to_turn_the_cursor_is_obeyed() {
    let dir = skin_folder("cursor-still");
    write_half_cursor(&dir);
    std::fs::write(dir.join("skin.ini"), "[General]\nCursorRotate: 0\n").expect("written");

    let at_rest = ink_left_of_the_cursor(&dir, 10_000.0);
    let later = ink_left_of_the_cursor(&dir, 15_000.0);

    assert!(at_rest > 0, "the cursor was not drawn at all");
    assert_eq!(
        at_rest, later,
        "it turned a cursor that asked to stay still"
    );
}

#[test]
fn the_render_can_turn_a_cursor_the_skin_holds_still() {
    let dir = skin_folder("cursor-forced-on");
    write_half_cursor(&dir);
    std::fs::write(dir.join("skin.ini"), "[General]\nCursorRotate: 0\n").expect("written");

    let at_rest = ink_left_of_the_cursor_with(&dir, 10_000.0, Some(true));
    let half_a_turn = ink_left_of_the_cursor_with(&dir, 15_000.0, Some(true));

    assert!(at_rest > 0, "the cursor was not drawn at all");
    assert!(
        half_a_turn * 4 < at_rest,
        "the override did not turn a cursor the skin holds still: {at_rest} against {half_a_turn}"
    );
}

#[test]
fn the_render_can_hold_still_a_cursor_the_skin_turns() {
    let dir = skin_folder("cursor-forced-off");
    write_half_cursor(&dir);

    let at_rest = ink_left_of_the_cursor_with(&dir, 10_000.0, Some(false));
    let later = ink_left_of_the_cursor_with(&dir, 15_000.0, Some(false));

    assert!(at_rest > 0, "the cursor was not drawn at all");
    assert_eq!(at_rest, later, "the override did not hold the cursor still");
}

fn write_rpm_plate(dir: &std::path::Path) {
    let mut pixmap = tiny_skia::Pixmap::new(280, 56).expect("a canvas");
    for pixel in pixmap.pixels_mut() {
        *pixel = tiny_skia::PremultipliedColorU8::from_rgba(200, 0, 200, 255).expect("a colour");
    }
    std::fs::write(
        dir.join("spinner-rpm.png"),
        pixmap.encode_png().expect("png"),
    )
    .expect("written");

    let mut digit = tiny_skia::Pixmap::new(33, 46).expect("a canvas");
    for pixel in digit.pixels_mut() {
        *pixel = tiny_skia::PremultipliedColorU8::from_rgba(255, 255, 255, 255).expect("white");
    }
    std::fs::write(dir.join("score-0.png"), digit.encode_png().expect("png")).expect("written");
}

fn plate_and_figure(dir: &std::path::Path) -> (u32, u32) {
    use dossier_render::elements::Element;
    use dossier_render::imported::Sprites;

    let map = beatmap(ONE_SPINNER);
    let replay = replay_over(vec![
        dossier_replay::ReplayFrame {
            time_ms: 0,
            x: 256.0,
            y: 120.0,
            keys: dossier_replay::Keys(0),
        },
        dossier_replay::ReplayFrame {
            time_ms: 8_000,
            x: 256.0,
            y: 120.0,
            keys: dossier_replay::Keys(0),
        },
    ]);
    let state = GameState::new(&map, &replay);
    let layout = Layout::new(640, 480);

    let mut skin = Skin::with_combo_colours(map.combo_colours()).with_font(font());

    let wanted = [Element::SpinnerRpm, Element::Score('0')];
    let sprites = Sprites::read(dir, &wanted).tint_for(&skin.combo_colours);
    skin.sprites = Some(std::sync::Arc::new(sprites));

    let frame = Scene::new(&state, skin).frame(4_000.0, &layout);

    if std::env::var("DOSSIER_PROBE").is_ok() {
        let x0 = (layout.width as f32 * 0.5) as u32;
        for y in (layout.height - 60)..layout.height {
            let mut row = String::new();
            for x in x0..(x0 + 60) {
                let p = frame.pixel(x, y).expect("inside");
                row.push(match (p.red(), p.green(), p.blue()) {
                    (12, 12, 16) => '.',
                    (r, g, bl) if r > 120 || g > 120 || bl > 120 => '#',
                    _ => '-',
                });
            }
            eprintln!("{y:>4} {row}");
        }
    }

    let is_plate =
        |p: tiny_skia::PremultipliedColorU8| p.red() > 150 && p.blue() > 150 && p.green() < 90;
    let (mut top, mut bottom) = (u32::MAX, 0);
    let (mut left, mut right) = (u32::MAX, 0);
    for y in 0..layout.height {
        for x in 0..layout.width {
            if is_plate(frame.pixel(x, y).expect("inside the frame")) {
                top = top.min(y);
                bottom = bottom.max(y);
                left = left.min(x);
                right = right.max(x);
            }
        }
    }
    if top == u32::MAX {
        return (0, 0);
    }
    let plate = bottom - top + 1;

    let figure = (top..=bottom)
        .filter(|&y| {
            (left..=right).any(|x| {
                let p = frame.pixel(x, y).expect("inside the frame");

                p.green() > 120 && p.red() > 120 && p.blue() > 120
            })
        })
        .count() as u32;
    (plate, figure)
}

#[test]
fn the_speed_figure_fills_its_plate_the_way_the_game_fills_it() {
    let dir = skin_folder("rpm-plate");
    write_rpm_plate(&dir);

    let (plate, figure) = plate_and_figure(&dir);
    assert!(plate > 0, "the plate was not drawn");
    assert!(figure > 0, "the figure was not drawn");

    let share = figure as f32 / plate as f32;

    assert!(
        (0.75..=0.92).contains(&share),
        "the figure is {figure}px in a {plate}px plate — {:.0}%; the game's own \
         fills four fifths",
        share * 100.0
    );
}

fn write_lopsided_ball(dir: &std::path::Path, name: &str, size: u32) {
    let mut pixmap = tiny_skia::Pixmap::new(size, size).expect("a canvas");
    let width = pixmap.width();
    for (at, pixel) in pixmap.pixels_mut().iter_mut().enumerate() {
        let x = at as u32 % width;
        *pixel = if x >= width / 2 {
            tiny_skia::PremultipliedColorU8::from_rgba(220, 0, 0, 255).expect("a colour")
        } else {
            tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 0, 0).expect("a colour")
        };
    }
    std::fs::write(dir.join(name), pixmap.encode_png().expect("png")).expect("written");
}

fn red_sides(frame: &tiny_skia::Pixmap, centre: (f32, f32)) -> (usize, usize) {
    let (cx, cy) = centre;
    let (mut left, mut right) = (0usize, 0usize);
    for y in (cy - 45.0).max(0.0) as u32..((cy + 45.0) as u32).min(frame.height()) {
        for x in (cx - 45.0).max(0.0) as u32..((cx + 45.0) as u32).min(frame.width()) {
            let Some(p) = frame.pixel(x, y) else { continue };
            if p.red() > 150 && p.green() < 90 && p.blue() < 90 {
                if f32::from(u16::try_from(x).unwrap_or(0)) < cx {
                    left += 1;
                } else {
                    right += 1;
                }
            }
        }
    }
    (left, right)
}

fn ball_at_two_legs(flip: bool) -> ((usize, usize), (usize, usize)) {
    use dossier_render::elements::Element;
    use dossier_render::imported::Sprites;

    let dir = skin_folder(if flip { "ball-flip" } else { "ball-plain" });
    write_lopsided_ball(&dir, "sliderb.png", 128);
    std::fs::write(
        dir.join("skin.ini"),
        if flip {
            "[General]\nSliderBallFlip: 1\n"
        } else {
            "[General]\nSliderBallFlip: 0\n"
        },
    )
    .expect("written");

    let map = beatmap(REPEATING_SLIDER);

    let frames: Vec<_> = (0..260)
        .map(|i| dossier_replay::ReplayFrame {
            time_ms: 1900 + i * 20,
            x: 250.0,
            y: 192.0,
            keys: dossier_replay::Keys(dossier_replay::Keys::K1),
        })
        .collect();
    let replay = replay_over(frames);
    let state = GameState::new(&map, &replay);

    let object = &state.timeline().objects[0];
    let slide = object.slide_duration_ms().expect("a slider");
    let (out_ms, back_ms) = (
        object.start_ms + slide * 0.25,
        object.start_ms + slide * 1.75,
    );

    let mut skin = Skin::with_combo_colours(map.combo_colours()).with_font(font());
    skin.sprites = Some(std::sync::Arc::new(
        Sprites::read(&dir, &[Element::SliderBall]).tint_for(&skin.combo_colours),
    ));
    let layout = Layout::new(640, 480);
    let where_it_is = |ms: f64| layout.map(object.ball_at(ms).expect("the ball is out"));
    let scene = Scene::new(&state, skin);
    (
        red_sides(&scene.frame(out_ms, &layout), where_it_is(out_ms)),
        red_sides(&scene.frame(back_ms, &layout), where_it_is(back_ms)),
    )
}

#[test]
fn the_slider_ball_turns_over_on_the_way_back_when_the_skin_asks() {
    let ((out_left, out_right), (back_left, back_right)) = ball_at_two_legs(true);
    assert!(
        out_right > 200 && out_left * 4 < out_right,
        "on the way out the red half should be on the right: {out_left} left, {out_right} right"
    );
    assert!(
        back_left > 200 && back_right * 4 < back_left,
        "on the way back it should have turned over: {back_left} left, {back_right} right"
    );
}

#[test]
fn a_skin_that_did_not_ask_gets_the_ball_the_same_way_round_both_ways() {
    let ((out_left, out_right), (back_left, back_right)) = ball_at_two_legs(false);
    assert!(
        out_right > 200 && back_right > 200,
        "the ball was not drawn"
    );
    assert!(
        out_left * 4 < out_right && back_left * 4 < back_right,
        "nothing asked for a flip and it turned over anyway: \
         out {out_left}/{out_right}, back {back_left}/{back_right}"
    );
}
