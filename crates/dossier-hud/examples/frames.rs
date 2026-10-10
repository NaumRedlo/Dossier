use dossier_hud::{compose, sample, sprites, Lang, Stage};
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
    for (name, view, context) in sample::frames(Lang::Ru) {
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
