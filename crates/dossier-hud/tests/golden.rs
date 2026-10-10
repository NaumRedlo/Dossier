use dossier_hud::{compose, sample, sprites, Lang, Stage};
use dossier_overlay::Viewport;

fn decoded(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut pixels).unwrap();
    pixels.truncate(info.buffer_size());
    (info.width, info.height, pixels)
}

#[test]
fn every_sample_frame_matches_its_approved_picture() {
    let viewport = Viewport {
        width: 1280,
        height: 720,
        scale: 1.0,
    };
    let golden = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let review = std::env::var_os("DOSSIER_HUD_REVIEW").map(std::path::PathBuf::from);
    let mut wrong = Vec::new();
    for (name, view, context) in sample::frames(Lang::Ru) {
        let stage = Stage::settled(Some(&view), &context);
        let mut frame = sample::backdrop(viewport.width, viewport.height);
        compose(
            &mut frame,
            viewport,
            &sprites(Some(&view), &context, &stage, viewport),
        );
        if let Some(dir) = &review {
            std::fs::create_dir_all(dir).unwrap();
            std::fs::write(
                dir.join(format!("{name}.png")),
                sample::png(viewport.width, viewport.height, &frame),
            )
            .unwrap();
        }
        let Ok(bytes) = std::fs::read(golden.join(format!("{name}.png"))) else {
            wrong.push(format!("{name}: no approved picture"));
            continue;
        };
        let (wide, high, approved) = decoded(&bytes);
        if (wide, high) != (viewport.width, viewport.height) {
            wrong.push(format!("{name}: approved picture is {wide}x{high}"));
            continue;
        }
        let far = approved
            .chunks_exact(4)
            .zip(frame.chunks_exact(4))
            .filter(|(a, b)| (0..3).any(|c| a[c].abs_diff(b[c]) > 3))
            .count();
        if far > 400 {
            wrong.push(format!("{name}: {far} pixels differ"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
