use std::path::PathBuf;

use dossier_native::gallery;

fn backend() {
    static READY: std::sync::Once = std::sync::Once::new();
    READY.call_once(|| {
        if std::env::var_os("ICED_TEST_BACKEND").is_none() {
            std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
        }
    });
}

fn golden(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("golden").join(name)
}

fn pixels_of(file: &std::path::Path) -> Result<(u32, u32, Vec<u8>), String> {
    let bytes = std::fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().map_err(|e| format!("{}: {e}", file.display()))?;
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut pixels).map_err(|e| format!("{}: {e}", file.display()))?;
    pixels.truncate(info.buffer_size());
    Ok((info.width, info.height, pixels))
}

fn check_frame(shot: iced_test::simulator::Snapshot, name: &str) -> Result<(), String> {
    let review = std::env::var_os("DOSSIER_GOLDEN_REVIEW").map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join(format!("dossier-golden-{}", std::process::id())));
    let actual = gallery::write_snapshot(&shot, &review.join(name))?;
    let reference = golden(actual.file_name().unwrap().to_str().unwrap());
    compare_files(&reference, &actual).map_err(|why| format!("{name}: {why}; current frame: {}", actual.display()))
}

fn compare_files(reference: &std::path::Path, actual: &std::path::Path) -> Result<(), String> {
    let (width, height, expected) = pixels_of(reference)?;
    let (actual_width, actual_height, found) = pixels_of(actual)?;
    if (width, height) != (actual_width, actual_height) || expected.len() != found.len() {
        return Err("dimensions differ".into());
    }
    let changed = expected.chunks_exact(4).zip(found.chunks_exact(4))
        .filter(|(a, b)| a.iter().zip(b.iter()).any(|(x, y)| x.abs_diff(*y) > 32))
        .count();
    let allowed = (width as usize * height as usize * 3) / 400;
    if changed > allowed {
        Err(format!("{changed} pixels differ beyond the 0.75% renderer tolerance (limit {allowed})"))
    } else {
        Ok(())
    }
}

#[test]
fn comparison_keeps_missing_and_damaged_references_unchanged() {
    backend();
    let dir = std::env::temp_dir().join(format!("dossier-golden-readonly-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut ui = iced_test::Simulator::<dossier_native::Message>::with_size(iced::Settings::default(), iced::Size::new(12.0, 12.0), iced::widget::Space::new());
    let actual = gallery::write_snapshot(&ui.snapshot(&dossier_native::theme::theme()).unwrap(), &dir.join("actual")).unwrap();
    let reference = dir.join("reference.png");
    assert!(!reference.exists());
    assert!(compare_files(&reference, &actual).is_err());
    assert!(!reference.exists(), "missing reference must not approve itself");
    std::fs::write(&reference, b"unfinished PNG").unwrap();
    assert!(compare_files(&reference, &actual).is_err());
    assert_eq!(std::fs::read(&reference).unwrap(), b"unfinished PNG");
    std::fs::copy(&actual, &reference).unwrap();
    assert!(compare_files(&reference, &actual).is_ok());
    let mut changed = pixels_of(&reference).unwrap();
    for pixel in changed.2.chunks_exact_mut(4).take(8) { pixel[0] ^= 255; }
    let mut png = png::Encoder::new(std::fs::File::create(&reference).unwrap(), changed.0, changed.1);
    png.set_color(png::ColorType::Rgba);
    png.write_header().unwrap().write_image_data(&changed.2).unwrap();
    assert!(compare_files(&reference, &actual).is_err(), "a changed image must still fail");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn exporting_replaces_a_truncated_png_with_a_completed_frame() {
    backend();
    let dir = std::env::temp_dir().join(format!("dossier-golden-export-{}", std::process::id()));
    let mut ui = iced_test::Simulator::<dossier_native::Message>::with_size(iced::Settings::default(), iced::Size::new(12.0, 12.0), iced::widget::Space::new());
    let shot = ui.snapshot(&dossier_native::theme::theme()).unwrap();
    let actual = gallery::write_snapshot(&shot, &dir.join("frame")).unwrap();
    std::fs::write(&actual, b"unfinished PNG").unwrap();
    assert_eq!(gallery::write_snapshot(&shot, &dir.join("frame")).unwrap(), actual);
    assert_eq!(pixels_of(&actual).unwrap().0, 24);
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1, "staging directory is cleaned up");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn every_first_run_state_matches_its_approved_frame() {
    backend();
    let mut wrong = Vec::new();
    for (name, flow, size) in gallery::every_frame() {
        let shot = gallery::snapshot(&flow, size).expect("a frame");
        if let Err(why) = check_frame(shot, &name) { wrong.push(why); }
    }
    assert!(wrong.is_empty(), "frames that no longer match their approved picture:\n{}", wrong.join("\n"));
}

#[test]
fn every_frame_is_the_size_of_its_window_and_stays_clear_of_the_bottom() {
    backend();
    for (name, flow, size) in gallery::every_frame() {
        let stem = std::env::temp_dir().join(format!("dossier-check-{name}"));
        let file = gallery::write_snapshot(&gallery::snapshot(&flow, size).expect("a frame"), &stem).expect("written");
        let (width, height, pixels) = pixels_of(&file).expect("valid PNG");
        assert_eq!((width, height), ((size.width * 2.0) as u32, (size.height * 2.0) as u32), "{name}");
        let tail = &pixels[(height as usize - 8) * width as usize * 4..];
        let bright = tail.chunks(4).filter(|px| px[0] as u32 + px[1] as u32 + px[2] as u32 > 120).count();
        assert_eq!(bright, 0, "{name} reaches the bottom edge of its window");
        let _ = std::fs::remove_file(&file);
    }
}

#[test]
fn every_state_keeps_its_buttons_on_screen() {
    backend();
    use iced_test::Simulator;
    let labels = ["Continue", "Use this", "Use both", "Browse…", "Open Dossier", "Check again", "Continue anyway", "Продолжить", "Выбрать", "Выбрать оба", "Обзор…", "Открыть Dossier", "Проверить снова", "Продолжить без него"];
    let mut lost = Vec::new();
    for (name, flow, size) in gallery::every_frame() {
        let backdrop = dossier_native::ui::backdrop_handle();
        let mut ui = Simulator::with_size(dossier_native::settings(), size, gallery::frame(&flow, &backdrop));
        let shown = labels.iter().filter_map(|label| ui.find(*label).ok()).collect::<Vec<_>>();
        if shown.is_empty() {
            continue;
        }
        for target in shown {
            let bounds = target.bounds();
            if bounds.width < 1.0 || bounds.height < 1.0 || bounds.y + bounds.height > size.height {
                lost.push(format!("{name}: {:?}", bounds));
            }
        }
    }
    assert!(lost.is_empty(), "buttons squeezed to nothing or pushed off the window:\n{}", lost.join("\n"));
}

#[test]
fn every_main_screen_state_matches_its_approved_frame() {
    backend();
    let mut wrong = Vec::new();
    let only = std::env::var("DOSSIER_ONLY_FRAMES").ok();
    for (name, main, size) in gallery::every_main_frame() {
        if only.as_deref().is_some_and(|only| !only.split(',').any(|part| name.contains(part))) {
            continue;
        }
        let shot = gallery::snapshot_main(&main, size).expect("a frame");
        if let Err(why) = check_frame(shot, &name) { wrong.push(why); }
    }
    assert!(wrong.is_empty(), "main-screen frames that no longer match their approved picture:\n{}", wrong.join("\n"));
}
