//! The storyboard: its curves, its reading, and what a sprite is at a moment.
//!
//! Three things are being checked here and they fail in different ways. A
//! curve that is subtly wrong makes every storyboard slightly off and nothing
//! obviously broken; a reading that is wrong loses a sprite outright; and the
//! arithmetic that turns a list of commands into a picture is where the rules
//! nobody writes down live — what a sprite looks like before its first fade,
//! how long a flip lasts, when a loop's second turn starts.

use dossier_beatmap::storyboard::{self, Change, Layer, Origin, Switch};

// ── the curves ───────────────────────────────────────────────────────────

/// `b` to `b + c` over `d`, sampled at `at`.
fn at(kind: u8, at: f64) -> f64 {
    storyboard::ease(kind, at, 0.0, 1.0, 1.0)
}

#[test]
fn every_curve_starts_and_ends_where_it_was_told_to() {
    // Whatever happens in between, a command that says "nought to one over
    // this long" has to be at one when the time is up — otherwise a sprite
    // faded in by an elastic curve settles at the wrong opacity and stays
    // there for the rest of the map.
    for kind in 0..=34u8 {
        assert!(
            (at(kind, 1.0) - 1.0).abs() < 1e-9,
            "curve {kind} ends at {}",
            at(kind, 1.0)
        );
        // Stable's own early-out: a time of nought is the starting value
        // for every curve, before any of them is consulted.
        assert!(
            (at(kind, 0.0) - 0.0).abs() < 1e-9,
            "curve {kind} starts wrong"
        );
    }
}

#[test]
fn nothing_is_asked_of_a_curve_with_nowhere_to_go() {
    // All three early-outs, which are stable's and not tidiness: a command
    // with no change, no elapsed time or no duration answers with its
    // beginning rather than dividing by zero.
    assert!((storyboard::ease(24, 5.0, 7.0, 0.0, 10.0) - 7.0).abs() < 1e-12);
    assert!((storyboard::ease(24, 0.0, 7.0, 3.0, 10.0) - 7.0).abs() < 1e-12);
    assert!((storyboard::ease(24, 5.0, 7.0, 3.0, 0.0) - 7.0).abs() < 1e-12);
}

#[test]
fn one_and_two_are_the_old_pair_and_are_the_wrong_way_round() {
    // The two osu! had before the rest existed. `1` is quadratic *out* and
    // `2` is quadratic *in*, which reads backwards and is what storyboards
    // are written against — half a second in, an "out" curve is already
    // three quarters of the way there.
    assert!(
        (at(1, 0.5) - 0.75).abs() < 1e-12,
        "1 should be quadratic out"
    );
    assert!(
        (at(2, 0.5) - 0.25).abs() < 1e-12,
        "2 should be quadratic in"
    );
    assert!(
        (at(1, 0.5) - at(4, 0.5)).abs() < 1e-12,
        "1 and 4 are the same curve"
    );
    assert!(
        (at(2, 0.5) - at(3, 0.5)).abs() < 1e-12,
        "2 and 3 are the same curve"
    );
}

#[test]
fn linear_is_linear_and_so_is_a_number_from_the_future() {
    assert!((at(0, 0.25) - 0.25).abs() < 1e-12);
    // A storyboard naming a curve this table has never heard of is drawn
    // rather than dropped.
    assert!((at(200, 0.25) - 0.25).abs() < 1e-12);
}

#[test]
fn the_ones_that_overshoot_actually_overshoot() {
    // Back and elastic leave the range they were given and come back, and
    // a table that quietly clamped them would look like a table that
    // worked. Bounce stays inside it.
    assert!(
        (0..=100).any(|i| at(30, f64::from(i) / 100.0) > 1.0),
        "back does not overshoot"
    );
    assert!(
        (0..=100).any(|i| at(25, f64::from(i) / 100.0) > 1.0),
        "elastic does not ring"
    );
    assert!(
        (0..=100).all(|i| at(33, f64::from(i) / 100.0) <= 1.0 + 1e-12),
        "bounce should stay under its mark"
    );
}

#[test]
fn bounce_in_is_bounce_out_run_backwards() {
    // Which is how stable writes it — case 32 calls case 33 with the time
    // reversed — and the reason to check it is that the recursion is easy
    // to get subtly wrong.
    for i in 0..=20 {
        let t = f64::from(i) / 20.0;
        assert!(
            (at(32, t) - (1.0 - at(33, 1.0 - t))).abs() < 1e-12,
            "at {t}"
        );
    }
}

// ── the reading ──────────────────────────────────────────────────────────

fn read(text: &str) -> storyboard::Storyboard {
    storyboard::parse(text)
}

/// `[Events]` with the lines given, indented the way a real file indents them.
fn events(lines: &str) -> String {
    format!("[Events]\n{lines}\n")
}

#[test]
fn a_sprite_is_read_with_its_layer_its_origin_and_where_it_sits() {
    let sb = read(&events(
        r#"Sprite,Foreground,Centre,"sb\flash.png",320,240"#,
    ));
    assert_eq!(sb.sprites.len(), 1);
    let sprite = &sb.sprites[0];
    assert_eq!(sprite.layer, Layer::Foreground);
    assert_eq!(sprite.origin, Origin::Centre);
    // The separators are left as written: what opens the file has to cope with
    // both anyway, and rewriting the path here would hide which it was.
    assert_eq!(sprite.path, r"sb\flash.png");
    assert_eq!((sprite.x, sprite.y), (320.0, 240.0));
}

#[test]
fn the_numbers_mean_the_same_as_the_names() {
    // Old storyboards write the layer and the origin as numbers, and plenty of
    // new ones do too because the editor emits them.
    let named = read(&events(r#"Sprite,Overlay,BottomRight,"a.png",1,2"#));
    let numbered = read(&events(r#"4,4,8,"a.png",1,2"#));
    assert_eq!(named.sprites[0].layer, numbered.sprites[0].layer);
    assert_eq!(named.sprites[0].origin, numbered.sprites[0].origin);
    assert_eq!(numbered.sprites[0].layer, Layer::Overlay);
}

#[test]
fn an_animation_carries_its_frames_and_how_long_each_is_up() {
    let sb = read(&events(
        r#"Animation,Background,TopLeft,"f.png",0,0,12,33.33,LoopOnce"#,
    ));
    let animation = sb.sprites[0].animation.expect("an animation");
    assert_eq!(animation.frames, 12);
    assert!((animation.frame_ms - 33.33).abs() < 1e-9);
    assert!(animation.once);
    // And a plain sprite is not one, which is what stops it being asked for a
    // file name with a number in it.
    assert!(
        read(&events(r#"Sprite,Background,TopLeft,"f.png",0,0"#)).sprites[0]
            .animation
            .is_none()
    );
}

#[test]
fn the_video_line_is_picked_out_of_the_same_section() {
    let sb = read(&events(
        "0,0,\"bg.jpg\",0,0\nVideo,-1200,\"clip.mp4\",0,0\n2,1000,2000",
    ));
    let video = sb.video.expect("a video");
    assert_eq!(video.path, "clip.mp4");
    // Videos routinely start before the song does.
    assert!((video.start_ms - -1200.0).abs() < 1e-9);
    // The background and the break on either side are not sprites.
    assert!(sb.sprites.is_empty());
}

#[test]
fn commands_land_on_the_sprite_above_them() {
    let sb = read(&events(
        "Sprite,Background,TopLeft,\"a.png\",0,0\n_F,0,0,500,0,1\nSprite,Background,TopLeft,\"b.png\",0,0\n_S,0,0,500,1,2",
    ));
    assert_eq!(sb.sprites.len(), 2);
    assert_eq!(sb.sprites[0].commands.len(), 1);
    assert_eq!(sb.sprites[1].commands.len(), 1);
    assert!(matches!(
        sb.sprites[1].commands[0].change,
        Change::Scale(1.0, 2.0)
    ));
}

#[test]
fn an_underscore_indents_exactly_as_far_as_a_space() {
    // Files use one or the other and some use both in the same storyboard.
    let spaces = read(&events(
        "Sprite,Background,TopLeft,\"a.png\",0,0\n L,0,2\n  F,0,0,100,0,1",
    ));
    let bars = read(&events(
        "Sprite,Background,TopLeft,\"a.png\",0,0\n_L,0,2\n__F,0,0,100,0,1",
    ));
    assert_eq!(spaces.sprites[0].commands, bars.sprites[0].commands);
    assert_eq!(spaces.sprites[0].commands.len(), 2, "the loop ran twice");
}

#[test]
fn an_empty_end_time_is_an_instant_and_a_missing_end_value_repeats_the_start() {
    let sb = read(&events(
        "Sprite,Background,TopLeft,\"a.png\",0,0\n_F,0,1000,,0.5\n_M,0,2000,3000,10,20",
    ));
    let fade = &sb.sprites[0].commands[0];
    assert!((fade.start_ms - fade.end_ms).abs() < 1e-9, "not an instant");
    assert!(
        matches!(fade.change, Change::Fade(a, b) if (a - 0.5).abs() < 1e-6 && (b - 0.5).abs() < 1e-6)
    );
    // The same rule on a move, where it is how a sprite is parked somewhere
    // for a stretch rather than dragged.
    assert!(matches!(
        sb.sprites[0].commands[1].change,
        Change::Move(10.0, 20.0, 10.0, 20.0)
    ));
}

#[test]
fn a_loop_is_laid_out_a_turn_at_a_time() {
    // Its body is written from the loop's own start, and one turn lasts as
    // long as its longest command — so the second turn begins where the first
    // one ended and not at some fixed guess.
    let sb = read(&events(
        "Sprite,Background,TopLeft,\"a.png\",0,0\n_L,1000,3\n__F,0,0,200,0,1\n__F,0,200,400,1,0",
    ));
    let times: Vec<(f64, f64)> = sb.sprites[0]
        .commands
        .iter()
        .map(|c| (c.start_ms, c.end_ms))
        .collect();
    assert_eq!(
        times,
        vec![
            (1000.0, 1200.0),
            (1200.0, 1400.0),
            (1400.0, 1600.0),
            (1600.0, 1800.0),
            (1800.0, 2000.0),
            (2000.0, 2200.0),
        ]
    );
}

#[test]
fn a_trigger_is_skipped_with_its_whole_body() {
    // It fires on something the storyboard cannot know by itself. Skipping it
    // loses an effect; expanding it on a guess invents one.
    let sb = read(&events(
        "Sprite,Background,TopLeft,\"a.png\",0,0\n_F,0,0,100,0,1\n_T,HitSoundClap,0,10000\n__F,0,0,200,1,0\n__S,0,0,200,1,2",
    ));
    assert_eq!(
        sb.sprites[0].commands.len(),
        1,
        "the trigger's body was let through"
    );
}

#[test]
fn a_variable_is_spent_before_the_line_is_read() {
    let text =
        "[Variables]\n$p=Background,TopLeft\n$pos=320,240\n\n[Events]\nSprite,$p,\"a.png\",$pos\n";
    let sb = read(text);
    assert_eq!(sb.sprites.len(), 1);
    assert_eq!((sb.sprites[0].x, sb.sprites[0].y), (320.0, 240.0));
}

#[test]
fn the_longest_variable_name_is_spent_first() {
    // `$a` inside `$ab` would otherwise eat its front and leave a `b`.
    let text = "[Variables]\n$a=1\n$ab=2\n\n[Events]\nSprite,Background,TopLeft,\"x.png\",$ab,$a\n";
    let sb = read(text);
    assert_eq!((sb.sprites[0].x, sb.sprites[0].y), (2.0, 1.0));
}

#[test]
fn a_comment_goes_and_a_path_with_two_slashes_stays() {
    let sb = read(&events(
        "//Storyboard Layer 0 (Background)\nSprite,Background,TopLeft,\"sb//a.png\",0,0",
    ));
    assert_eq!(sb.sprites.len(), 1);
    assert_eq!(sb.sprites[0].path, "sb//a.png");
}

#[test]
fn a_line_nobody_can_read_is_reported_and_the_rest_is_kept() {
    // A storyboard is decoration. One bad line should cost that line.
    let (sb, errors) = storyboard::parse_reporting(&events(
        "Sprite,Background,TopLeft,\"a.png\",0,0\n_F,0,nonsense,500,0,1\n_S,0,0,500,1,2",
    ));
    assert_eq!(
        sb.sprites[0].commands.len(),
        1,
        "the good line was dropped too"
    );
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].line, 3);
}

#[test]
fn only_the_events_section_is_read() {
    let text = "osu file format v14\n\n[General]\nAudioFilename: a.mp3\n\n[Events]\nSprite,Background,TopLeft,\"a.png\",0,0\n _F,0,0,1,0,1\n\n[HitObjects]\n100,100,3000,5,0\n";
    assert_eq!(read(text).sprites.len(), 1);
}

// ── what a sprite is at a moment ─────────────────────────────────────────

#[test]
fn a_sprite_is_out_for_as_long_as_something_is_happening_to_it() {
    // Not for the length of the song. A storyboard with four thousand sprites
    // has perhaps thirty out at once, and this is the whole reason.
    let sb = read(&events(
        "Sprite,Background,TopLeft,\"a.png\",0,0\n_F,0,1000,2000,0,1",
    ));
    let sprite = &sb.sprites[0];
    assert_eq!(sprite.alive(), Some((1000.0, 2000.0)));
    assert!(sprite.at(999.0).is_none());
    assert!(sprite.at(1500.0).is_some());
    assert!(sprite.at(2001.0).is_none());
}

#[test]
fn before_its_first_fade_a_sprite_is_already_at_that_fades_start() {
    // The rule nobody writes down. A sprite kept alive by a long move, with a
    // fade that starts later, is *invisible* until the fade begins — not fully
    // lit and then suddenly dark. Held at the default instead, every such
    // sprite flashes on at its own start time.
    let sb = read(&events(
        "Sprite,Background,TopLeft,\"a.png\",0,0\n_M,0,0,3000,0,0,100,0\n_F,0,1000,2000,0,1",
    ));
    let sprite = &sb.sprites[0];
    assert!(
        (sprite.at(500.0).unwrap().alpha - 0.0).abs() < 1e-6,
        "lit too early"
    );
    assert!((sprite.at(1500.0).unwrap().alpha - 0.5).abs() < 1e-6);
    // And after the last one, it stays where it was left.
    assert!((sprite.at(2900.0).unwrap().alpha - 1.0).abs() < 1e-6);
}

#[test]
fn a_sprite_nobody_faded_is_simply_visible() {
    let sb = read(&events(
        "Sprite,Background,TopLeft,\"a.png\",0,0\n_M,0,0,1000,0,0,100,50",
    ));
    let drawn = sb.sprites[0].at(500.0).expect("out");
    assert!((drawn.alpha - 1.0).abs() < 1e-6);
    // And halfway through the move, halfway along it.
    assert!((drawn.x - 50.0).abs() < 1e-4 && (drawn.y - 25.0).abs() < 1e-4);
}

#[test]
fn an_instant_switch_holds_and_one_with_a_length_lets_go() {
    // `P,0,t,,H` is how a storyboard mirrors a sprite for good. Ending it the
    // moment it began — which is what its two equal times say literally —
    // would be a picture that never turns over at all.
    let held = read(&events(
        "Sprite,Background,TopLeft,\"a.png\",0,0\n_F,0,0,5000,1,1\n_P,0,1000,,H",
    ));
    assert!(
        !held.sprites[0].at(500.0).unwrap().flip.0,
        "flipped before it was told"
    );
    assert!(
        held.sprites[0].at(4000.0).unwrap().flip.0,
        "the flip did not hold"
    );

    let timed = read(&events(
        "Sprite,Background,TopLeft,\"a.png\",0,0\n_F,0,0,5000,1,1\n_P,0,1000,1500,H",
    ));
    assert!(timed.sprites[0].at(1200.0).unwrap().flip.0);
    assert!(
        !timed.sprites[0].at(2000.0).unwrap().flip.0,
        "the flip outstayed it"
    );
}

#[test]
fn additive_is_a_switch_like_the_others() {
    let sb = read(&events(
        "Sprite,Background,TopLeft,\"a.png\",0,0\n_F,0,0,5000,1,1\n_P,0,0,,A",
    ));
    assert!(sb.sprites[0].at(2000.0).unwrap().additive);
    assert!(matches!(
        sb.sprites[0].commands[1].change,
        Change::Parameter(Switch::Additive)
    ));
}

#[test]
fn an_animation_wraps_unless_it_was_told_to_stop() {
    let looping = read(&events(
        "Animation,Background,TopLeft,\"f.png\",0,0,4,100,LoopForever\n_F,0,1000,3000,1,1",
    ));
    let once = read(&events(
        "Animation,Background,TopLeft,\"f.png\",0,0,4,100,LoopOnce\n_F,0,1000,3000,1,1",
    ));
    // Counted from the sprite's own start, not from zero.
    assert_eq!(looping.sprites[0].at(1000.0).unwrap().frame, 0);
    assert_eq!(looping.sprites[0].at(1250.0).unwrap().frame, 2);
    assert_eq!(
        looping.sprites[0].at(1500.0).unwrap().frame,
        1,
        "it did not wrap"
    );
    assert_eq!(
        once.sprites[0].at(1500.0).unwrap().frame,
        3,
        "it did not stop"
    );
}

#[test]
fn colour_is_walked_channel_by_channel() {
    let sb = read(&events(
        "Sprite,Background,TopLeft,\"a.png\",0,0\n_C,0,0,1000,255,0,0,0,0,255",
    ));
    let drawn = sb.sprites[0].at(500.0).expect("out");
    assert_eq!(drawn.colour[1], 0);
    assert!(
        (i16::from(drawn.colour[0]) - 127).abs() <= 1
            && (i16::from(drawn.colour[2]) - 127).abs() <= 1,
        "halfway between red and blue is {:?}",
        drawn.colour
    );
}

#[test]
fn the_two_kinds_of_scale_both_land() {
    let flat = read(&events(
        "Sprite,Background,TopLeft,\"a.png\",0,0\n_S,0,0,1000,1,3",
    ));
    let vector = read(&events(
        "Sprite,Background,TopLeft,\"a.png\",0,0\n_V,0,0,1000,1,1,3,5",
    ));
    assert_eq!(flat.sprites[0].at(1000.0).unwrap().scale, (3.0, 3.0));
    assert_eq!(vector.sprites[0].at(1000.0).unwrap().scale, (3.0, 5.0));
}

#[test]
fn the_drawing_order_is_by_layer_and_then_by_the_file() {
    // Overlay goes over the play; the rest go under it. Inside a layer the
    // file's own order decides, so a sort that is not stable would shuffle a
    // mapper's stacking every frame.
    let sb = read(&events(
        "Sprite,Overlay,TopLeft,\"over.png\",0,0\n_F,0,0,1000,1,1\n\
         Sprite,Background,TopLeft,\"first.png\",0,0\n_F,0,0,1000,1,1\n\
         Sprite,Background,TopLeft,\"second.png\",0,0\n_F,0,0,1000,1,1",
    ));
    let out = sb.at(500.0);
    let names: Vec<&str> = out.iter().map(|d| d.path).collect();
    assert_eq!(names, vec!["first.png", "second.png", "over.png"]);
}

#[test]
fn nothing_invisible_is_handed_out_to_be_drawn() {
    let sb = read(&events(
        "Sprite,Background,TopLeft,\"a.png\",0,0\n_F,0,0,1000,0,0",
    ));
    assert!(
        sb.at(500.0).is_empty(),
        "a sprite at nought opacity was offered"
    );
}

#[test]
fn a_difficultys_own_events_are_drawn_over_the_sets() {
    // The `.osb` is read first and `[Events]` second, which is the order the
    // game draws them in.
    let mut set = read(&events(
        "Sprite,Background,TopLeft,\"set.png\",0,0\n_F,0,0,1,1,1",
    ));
    let own = read(&events(
        "Sprite,Background,TopLeft,\"own.png\",0,0\n_F,0,0,1,1,1",
    ));
    set.absorb(own);
    let names: Vec<&str> = set.at(0.5).iter().map(|d| d.path).collect();
    assert_eq!(names, vec!["set.png", "own.png"]);
}
