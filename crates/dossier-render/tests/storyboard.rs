//! Where a storyboard sprite lands, and which side of the notes it goes.
//!
//! The arithmetic that turns commands into a sprite is tested in the beatmap
//! crate, where it lives. What is checked here is the other half: that one
//! storyboard unit is one osu!pixel, that the middle of the storyboard is the
//! middle of the frame, and that the layers end up on the right side of the
//! play.

use dossier_beatmap::{storyboard, Beatmap};
use dossier_render::storyboard::Show;
use dossier_render::{Layout, Scene, Skin};
use dossier_sim::GameState;

fn beatmap(body: &str) -> Beatmap {
    Beatmap::parse(&format!("osu file format v14\n\n{body}")).expect("a map")
}

/// A map with one note, in a corner and out of the way: the storyboard is
/// checked in the middle of the frame, and a hit circle sitting there would be
/// counted along with it.
const ONE_NOTE: &str = "
[Difficulty]
CircleSize:4
ApproachRate:5

[HitObjects]
20,20,60000,1,0
";

/// A moment inside the play. Before it the render is still coming up out of
/// black — which took three failing tests to notice, and is right: a frame
/// nobody has faded in yet has nothing on it, storyboard included.
const WHEN: f64 = 59_000.0;

fn replay() -> dossier_replay::Replay {
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
        mods: dossier_replay::Mods::default(),
        life_bar: String::new(),
        timestamp_ticks: 0,
        online_score_id: 0,
        target_practice_accuracy: None,
        frames: vec![dossier_replay::ReplayFrame {
            time_ms: 0,
            x: 20.0,
            y: 20.0,
            keys: dossier_replay::Keys(0),
        }],
        rng_seed: None,
        score_info: None,
    }
}

/// A solid square of one colour, as a PNG.
fn square(side: u32, colour: (u8, u8, u8)) -> Vec<u8> {
    let mut pixmap = tiny_skia::Pixmap::new(side, side).expect("a canvas");
    for pixel in pixmap.pixels_mut() {
        *pixel = tiny_skia::PremultipliedColorU8::from_rgba(colour.0, colour.1, colour.2, 255)
            .expect("a colour");
    }
    pixmap.encode_png().expect("png")
}

fn show(events: &str) -> Show {
    let board = storyboard::parse(&format!("[Events]\n{events}\n"));
    Show::load(board, |_| Some(square(100, (255, 0, 0))))
}

fn frame_with(events: &str, time_ms: f64) -> tiny_skia::Pixmap {
    let map = beatmap(ONE_NOTE);
    let replay = replay();
    let state = GameState::new(&map, &replay);
    let skin = Skin::with_combo_colours(map.combo_colours());
    Scene::new(&state, skin)
        .bare()
        .with_storyboard(show(events))
        .frame(time_ms, &Layout::new(640, 480))
}

fn is_red(frame: &tiny_skia::Pixmap, x: u32, y: u32) -> bool {
    frame
        .pixel(x, y)
        .is_some_and(|p| p.red() > 150 && p.green() < 90 && p.blue() < 90)
}

#[test]
fn the_middle_of_the_storyboard_is_the_middle_of_the_frame() {
    // 320,240 is the centre of the 640×480 a storyboard is authored on, and a
    // centred sprite put there covers the middle of the picture whatever the
    // frame's own size is.
    let frame = frame_with(
        "Sprite,Background,Centre,\"a.png\",320,240\n_F,0,0,120000,1,1",
        WHEN,
    );
    assert!(is_red(&frame, 320, 240), "nothing in the middle");
    // A hundred wide at this size, so its edges are fifty out and not more.
    assert!(is_red(&frame, 320 + 45, 240 + 45));
    assert!(
        !is_red(&frame, 320 + 60, 240),
        "it spilled past its own width"
    );
}

#[test]
fn one_storyboard_unit_is_one_osu_pixel() {
    // Which is the whole conversion: the playfield is the 512×384 in the
    // middle of the same 640×480. At this frame size the scale is exactly one,
    // so a hundred-wide picture is a hundred pixels and any other rule for the
    // storyboard's space would show up here immediately.
    let frame = frame_with(
        "Sprite,Background,TopLeft,\"a.png\",320,240\n_F,0,0,120000,1,1",
        WHEN,
    );
    // Drawn from its top-left corner: 320,240 to 420,340.
    assert!(is_red(&frame, 322, 242) && is_red(&frame, 418, 338));
    assert!(
        !is_red(&frame, 318, 238),
        "it reached above and left of its corner"
    );
    assert!(!is_red(&frame, 422, 342), "it reached past a hundred");
}

#[test]
fn a_sprite_at_the_corner_of_the_space_is_at_the_corner_of_the_frame() {
    let frame = frame_with(
        "Sprite,Background,TopLeft,\"a.png\",0,0\n_F,0,0,120000,1,1",
        WHEN,
    );
    assert!(
        is_red(&frame, 1, 1),
        "the storyboard's origin is not the frame's"
    );
}

#[test]
fn scale_and_the_origin_work_together() {
    // Doubled about its middle, so it reaches a hundred either way rather than
    // two hundred one way.
    let frame = frame_with(
        "Sprite,Background,Centre,\"a.png\",320,240\n_F,0,0,120000,1,1\n_S,0,0,120000,2,2",
        WHEN,
    );
    assert!(is_red(&frame, 320 + 90, 240 + 90), "it did not grow");
    assert!(!is_red(&frame, 320 + 110, 240), "it grew the wrong way");
}

#[test]
fn a_sprite_nobody_shipped_costs_that_sprite_and_no_more() {
    // Storyboards routinely name files that never made it into the archive.
    let board = storyboard::parse(
        "[Events]\nSprite,Background,Centre,\"missing.png\",320,240\n_F,0,0,120000,1,1\n",
    );
    let show = Show::load(board, |_| None);
    assert!(show.is_empty());
    // And drawing it is a frame, not a panic.
    let map = beatmap(ONE_NOTE);
    let replay = replay();
    let state = GameState::new(&map, &replay);
    let skin = Skin::with_combo_colours(map.combo_colours());
    let _ = Scene::new(&state, skin)
        .bare()
        .with_storyboard(show)
        .frame(WHEN, &Layout::new(640, 480));
}

#[test]
fn the_fail_layer_is_never_drawn() {
    // A replay is a play that happened, and this is the branch where it did
    // not. Drawing it would put a mapper's failure scenery over every render.
    let frame = frame_with(
        "Sprite,Fail,Centre,\"a.png\",320,240\n_F,0,0,120000,1,1",
        WHEN,
    );
    assert!(!is_red(&frame, 320, 240), "the fail layer was drawn");
    // While `Pass` — the branch a replay always takes — is.
    let passing = frame_with(
        "Sprite,Pass,Centre,\"a.png\",320,240\n_F,0,0,120000,1,1",
        WHEN,
    );
    assert!(is_red(&passing, 320, 240), "the pass layer was not drawn");
}

#[test]
fn a_sprite_that_is_not_out_yet_is_not_drawn() {
    let frame = frame_with(
        "Sprite,Background,Centre,\"a.png\",320,240\n_F,0,119000,120000,1,1",
        WHEN,
    );
    assert!(!is_red(&frame, 320, 240));
}
