use dossier_native::first_run::qr_for;
use dossier_native::ui;
use iced::widget::container;
use iced::{Length, Size};
use iced_test::Simulator;

const LINK: &str = "https://t.me/OneNineEightFourGlobalBot?start=pair-K7QNM4XZ";

fn decode(file: &std::path::Path) -> Option<String> {
    let bytes = std::fs::read(file).ok()?;
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().ok()?;
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut pixels).ok()?;
    let (w, h) = (info.width as usize, info.height as usize);
    let mut prepared = rqrr::PreparedImage::prepare_from_greyscale(w, h, |x, y| {
        let at = (y * w + x) * 4;
        ((pixels[at] as u32 + pixels[at + 1] as u32 + pixels[at + 2] as u32) / 3) as u8
    });
    let grids = prepared.detect_grids();
    grids.first().and_then(|grid| grid.decode().ok()).map(|(_, text)| text)
}

fn read_back(code: &ui::Qr, name: &str) -> Option<String> {
    let side = ui::QR_SIDE + 40.0;
    let stem = std::env::temp_dir().join(format!("dossier-qr-{name}"));
    let _ = std::fs::remove_file(dossier_native::gallery::written_as(&stem));
    let element: iced::Element<'_, ()> = container(ui::qr::<()>(code)).center(Length::Fill).into();
    let mut sim = Simulator::with_size(dossier_native::settings(), Size::new(side, side), element);
    let shot = sim.snapshot(&dossier_native::theme::theme()).expect("a frame");
    shot.matches_image(&stem).expect("written");
    decode(&dossier_native::gallery::written_as(&stem))
}

#[test]
fn the_plainest_drawing_reads_back() {
    let mut code = qr_for(LINK).expect("a code");
    code.heart = 0.0;
    code.module_round = 0.0;
    code.finder_round = 0.0;
    assert_eq!(read_back(&code, "plain").as_deref(), Some(LINK));
}

#[test]
fn the_drawn_code_still_reads_as_the_link() {
    let code = qr_for(LINK).expect("a code");
    let mut plain = code.clone();
    plain.heart = 0.0;
    let mut square = code.clone();
    square.module_round = 0.0;
    let mut sharp = code.clone();
    sharp.finder_round = 0.0;
    let mut mild = code.clone();
    mild.finder_round = 1.0;
    let mut milder = code.clone();
    milder.finder_round = 0.6;
    let mut bigger_heart = code.clone();
    bigger_heart.finder_round = 1.0;
    bigger_heart.heart = 0.3;
    let tried = [
        ("as drawn", read_back(&code, "drawn")),
        ("without the mark", read_back(&plain, "no-heart")),
        ("square modules", read_back(&square, "square")),
        ("sharp finders", read_back(&sharp, "sharp")),
        ("finders rounded by one cell", read_back(&mild, "mild")),
        ("finders rounded by 0.6", read_back(&milder, "milder")),
        ("one cell and a bigger mark", read_back(&bigger_heart, "bigger")),
    ];
    for (name, read) in &tried {
        eprintln!("{name}: {read:?}");
    }
    assert_eq!(tried[0].1.as_deref(), Some(LINK));
}
