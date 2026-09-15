use std::path::PathBuf;

use dossier_native::gallery;

fn golden(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("golden").join(name)
}

fn pixels_of(file: &std::path::Path) -> (u32, u32, Vec<u8>) {
    let bytes = std::fs::read(file).expect("the frame on disk");
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().expect("a png");
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut pixels).expect("a frame");
    pixels.truncate(info.buffer_size());
    (info.width, info.height, pixels)
}

#[test]
fn every_first_run_state_matches_its_approved_frame() {
    let mut wrong = Vec::new();
    for (name, flow, size) in gallery::every_frame() {
        let shot = gallery::snapshot(&flow, size).expect("a frame");
        let stem = golden(&name);
        if !shot.matches_image(&stem).expect("a comparison") {
            wrong.push(name);
        }
    }
    assert!(wrong.is_empty(), "frames that no longer match their approved picture:\n{}", wrong.join("\n"));
}

#[test]
fn every_frame_is_the_size_of_its_window_and_stays_clear_of_the_bottom() {
    for (name, flow, size) in gallery::every_frame() {
        let stem = std::env::temp_dir().join(format!("dossier-check-{name}"));
        let _ = std::fs::remove_file(gallery::written_as(&stem));
        gallery::snapshot(&flow, size).expect("a frame").matches_image(&stem).expect("written");
        let file = gallery::written_as(&stem);
        let (width, height, pixels) = pixels_of(&file);
        assert_eq!((width, height), ((size.width * 2.0) as u32, (size.height * 2.0) as u32), "{name}");
        let tail = &pixels[(height as usize - 8) * width as usize * 4..];
        let bright = tail.chunks(4).filter(|px| px[0] as u32 + px[1] as u32 + px[2] as u32 > 120).count();
        assert_eq!(bright, 0, "{name} reaches the bottom edge of its window");
        let _ = std::fs::remove_file(&file);
    }
}
