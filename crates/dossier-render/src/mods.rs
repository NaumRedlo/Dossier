use std::sync::OnceLock;

use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, PixmapPaint, Transform};

use crate::text::{Align, Font, Label};

const LETTERS: &[u8] = include_bytes!("../../../assets/fonts/JetBrainsMono-ExtraBold.ttf");

pub const RATIO: f32 = 1.62;

const CORNER: f32 = 0.26;

const LETTER_SHARE: f32 = 0.52;

const GAP: f32 = 0.12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Easier,
    Harder,
    Automatic,
    Converted,
    Other,
}

const KNOWN: &[(&str, Kind)] = &[
    ("NM", Kind::Other),
    ("EZ", Kind::Easier),
    ("NF", Kind::Easier),
    ("HT", Kind::Easier),
    ("DC", Kind::Easier),
    ("HD", Kind::Harder),
    ("HR", Kind::Harder),
    ("SD", Kind::Harder),
    ("PF", Kind::Harder),
    ("DT", Kind::Harder),
    ("NC", Kind::Harder),
    ("FL", Kind::Harder),
    ("BL", Kind::Harder),
    ("RX", Kind::Automatic),
    ("AP", Kind::Automatic),
    ("SO", Kind::Automatic),
    ("AT", Kind::Automatic),
    ("CN", Kind::Automatic),
    ("TD", Kind::Converted),
    ("MR", Kind::Converted),
    ("V2", Kind::Converted),
    ("TP", Kind::Converted),
    ("RD", Kind::Converted),
    ("CL", Kind::Converted),
    ("DA", Kind::Converted),
];

pub fn kind_of(acronym: &str) -> Kind {
    KNOWN
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(acronym))
        .map_or(Kind::Other, |(_, kind)| *kind)
}

pub fn colour_of(kind: Kind) -> Color {
    match kind {
        Kind::Easier => Color::from_rgba8(122, 198, 92, 255),
        Kind::Harder => Color::from_rgba8(214, 78, 72, 255),
        Kind::Automatic => Color::from_rgba8(86, 148, 214, 255),
        Kind::Converted => Color::from_rgba8(150, 104, 206, 255),
        Kind::Other => Color::from_rgba8(112, 96, 100, 255),
    }
}

pub fn known(acronym: &str) -> bool {
    KNOWN
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case(acronym))
}

pub fn every() -> Vec<&'static str> {
    KNOWN.iter().map(|(name, _)| *name).collect()
}

fn letters() -> Option<&'static Font> {
    static HELD: OnceLock<Option<Font>> = OnceLock::new();
    HELD.get_or_init(|| Font::from_bytes(LETTERS).ok())
        .as_ref()
}

fn plate(into: &mut Pixmap, wide: f32, high: f32, colour: Color) {
    let round = high * CORNER;
    let mut path = PathBuilder::new();
    path.move_to(round, 0.0);
    path.line_to(wide - round, 0.0);
    path.quad_to(wide, 0.0, wide, round);
    path.line_to(wide, high - round);
    path.quad_to(wide, high, wide - round, high);
    path.line_to(round, high);
    path.quad_to(0.0, high, 0.0, high - round);
    path.line_to(0.0, round);
    path.quad_to(0.0, 0.0, round, 0.0);
    let Some(path) = path.finish() else {
        return;
    };
    let mut paint = Paint {
        anti_alias: true,
        ..Default::default()
    };
    paint.set_color(colour);
    into.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
}

pub fn icon(acronym: &str, high: u32) -> Option<Pixmap> {
    if !known(acronym) {
        return None;
    }
    let high = high.max(10);
    let wide = (high as f32 * RATIO).round().max(1.0) as u32;
    let mut out = Pixmap::new(wide, high)?;

    plate(
        &mut out,
        wide as f32,
        high as f32,
        colour_of(kind_of(acronym)),
    );

    let font = letters()?;
    let name = acronym.to_ascii_uppercase();
    let size = high as f32 * LETTER_SHARE;
    font.draw(
        &mut out,
        Label {
            text: &name,
            x: wide as f32 / 2.0,
            y: (high as f32 + font.digit_height(size)) / 2.0,
            size,
            colour: Color::from_rgba8(255, 255, 255, 255),
            align: Align::Centre,
        },
    );
    Some(out)
}

pub fn row_width(count: usize, high: u32) -> u32 {
    if count == 0 {
        return 0;
    }
    let one = high as f32 * RATIO;
    let gap = high as f32 * GAP;
    ((one + gap) * count as f32 - gap).round() as u32
}

pub fn row(acronyms: &[String], high: u32) -> Option<Pixmap> {
    let wanted: Vec<&String> = acronyms.iter().filter(|one| known(one)).collect();
    if wanted.is_empty() {
        return None;
    }
    let step = high as f32 * (RATIO + GAP);
    let mut out = Pixmap::new(row_width(wanted.len(), high), high)?;
    for (at, acronym) in wanted.iter().enumerate() {
        let Some(made) = icon(acronym, high) else {
            continue;
        };
        out.draw_pixmap(
            0,
            0,
            made.as_ref(),
            &PixmapPaint::default(),
            Transform::from_translate(step * at as f32, 0.0),
            None,
        );
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_letters_are_there_to_draw_with() {
        assert!(letters().is_some(), "the badge font did not load");
    }

    #[test]
    fn a_badge_is_a_wide_plate_with_ink_in_the_middle() {
        let made = icon("HD", 100).expect("hidden draws");
        assert_eq!((made.width(), made.height()), (162, 100));

        let corner = made.pixel(1, 1).expect("a corner");
        assert_eq!(corner.alpha(), 0, "the corner should be rounded away");

        let middle = made.pixel(81, 50).expect("the middle");
        assert!(middle.alpha() > 0, "the plate is empty");

        let white = made
            .pixels()
            .iter()
            .filter(|p| p.red() > 230 && p.green() > 230 && p.blue() > 230)
            .count();
        assert!(white > 200, "only {white} pixels of lettering");
    }

    #[test]
    fn the_kind_decides_the_colour_and_unknown_names_stay_out() {
        assert_eq!(kind_of("EZ"), Kind::Easier);
        assert_eq!(kind_of("hd"), Kind::Harder);
        assert_eq!(kind_of("RX"), Kind::Automatic);
        assert_eq!(kind_of("V2"), Kind::Converted);
        assert_eq!(kind_of("ZZ"), Kind::Other);
        assert!(!known("ZZ"));
        assert!(icon("ZZ", 40).is_none());
        assert_ne!(colour_of(Kind::Easier), colour_of(Kind::Harder));
    }

    #[test]
    fn a_row_grows_with_every_badge_and_leaves_a_gap() {
        assert_eq!(row_width(1, 100), 162);
        assert_eq!(row_width(3, 100), 162 * 3 + 12 * 2);
        let made = row(&["HD".to_owned(), "HR".to_owned()], 60).expect("a row draws");
        assert_eq!(made.width(), row_width(2, 60));
        assert!(row(&["ZZ".to_owned()], 60).is_none());
        assert!(row(&[], 60).is_none());
    }
}
