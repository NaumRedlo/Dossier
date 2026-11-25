use dossier_hud::{compose, sample, sprites, Context, Lang, Panel, Profile, Stage};
use dossier_overlay::Viewport;

fn main() {
    let out = std::env::args()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("dossier-hud-frames"));
    std::fs::create_dir_all(&out).expect("the output folder is made");
    let viewport = Viewport {
        width: 1280,
        height: 720,
        scale: 1.0,
    };
    let mut frames = sample::frames(Lang::Ru);
    let (_, menu, context) = frames[0].clone();
    let (_, select, select_context) = frames[1].clone();
    let profile = Profile {
        name: "hmosha".into(),
        pp: 10326.5,
        rank: 6546,
        country: "RU".into(),
        country_rank: 596,
        accuracy: 99.0827,
        plays: 101_118,
        level: 101.0,
    };
    frames.push((
        "board",
        menu.clone(),
        Context {
            panel: Some(Panel::Board),
            board: 1.0,
            greet: false,
            ..context.clone()
        },
    ));
    frames.push((
        "profile",
        menu,
        Context {
            profile: Some(profile),
            profile_shown: 1.0,
            greet: false,
            ..context
        },
    ));
    let mut with_max = select;
    let mut map = with_max.snapshot.beatmap.take().unwrap_or(dossier_overlay::Beatmap {
        md5: "a".repeat(32),
        title: String::new(),
        artist: String::new(),
        difficulty: String::new(),
        max_pp: None,
        max_pp_mods: 0,
        max_combo: None,
    });
    map.max_pp = Some(612.4);
    map.max_pp_mods = (1 << 3) | (1 << 6);
    map.max_combo = Some(12843);
    with_max.snapshot.beatmap = Some(map);
    frames.push(("maxpp", with_max, select_context));
    for (name, view, context) in frames {
        let stage = Stage::settled(Some(&view), &context);
        let mut frame = sample::backdrop(viewport.width, viewport.height);
        compose(
            &mut frame,
            viewport,
            &sprites(Some(&view), &context, &stage, viewport),
        );
        let file = out.join(format!("{name}.png"));
        std::fs::write(&file, sample::png(viewport.width, viewport.height, &frame))
            .expect("a frame is written");
        println!("{}", file.display());
    }
}
