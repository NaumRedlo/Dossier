use std::sync::OnceLock;

use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, PixmapPaint, Stroke,
    Transform,
};

use crate::text::{Align, Font, Label};

const LETTERS: &[u8] = include_bytes!("../../../assets/fonts/JetBrainsMono-ExtraBold.ttf");

pub const RATIO: f32 = 1.62;

const CORNER: f32 = 0.26;

const LETTER_SHARE: f32 = 0.52;

const MARK_SHARE: f32 = 0.62;

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

fn ink() -> Paint<'static> {
    let mut paint = Paint {
        anti_alias: true,
        ..Default::default()
    };
    paint.set_color(Color::from_rgba8(255, 255, 255, 255));
    paint
}

fn pen(width: f32) -> Stroke {
    Stroke {
        width,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Default::default()
    }
}

fn chevrons(path: &mut PathBuilder, facing: f32, span: f32) {
    for step in [-0.42f32, 0.34] {
        path.move_to(-0.34 * facing + step * span, -0.5);
        path.line_to(0.3 * facing + step * span, 0.0);
        path.line_to(-0.34 * facing + step * span, 0.5);
    }
}

fn mark_of(acronym: &str) -> Option<(Path, Option<Path>)> {
    let mut line = PathBuilder::new();
    let mut solid = PathBuilder::new();
    match acronym {
        "EZ" | "NF" => {
            line.move_to(0.0, 0.52);
            line.cubic_to(-0.78, -0.06, -0.5, -0.62, 0.0, -0.24);
            line.cubic_to(0.5, -0.62, 0.78, -0.06, 0.0, 0.52);
            if acronym == "NF" {
                line.move_to(-0.62, 0.58);
                line.line_to(0.62, -0.58);
            }
        }
        "HT" | "DC" => {
            chevrons(&mut line, -1.0, 0.62);
            if acronym == "DC" {
                solid.push_circle(0.0, -0.6, 0.16);
            }
        }
        "DT" | "NC" => {
            chevrons(&mut line, 1.0, 0.62);
            if acronym == "NC" {
                solid.push_circle(0.0, -0.6, 0.16);
            }
        }
        "HD" => {
            line.move_to(-0.62, 0.0);
            line.quad_to(0.0, -0.56, 0.62, 0.0);
            line.quad_to(0.0, 0.56, -0.62, 0.0);
            solid.push_circle(0.0, 0.0, 0.17);
        }
        "HR" => {
            for step in [0.1f32, -0.34] {
                line.move_to(-0.46, 0.24 + step);
                line.line_to(0.0, -0.24 + step);
                line.line_to(0.46, 0.24 + step);
            }
        }
        "SD" | "PF" => {
            line.move_to(-0.46, -0.34);
            line.line_to(0.0, 0.2);
            line.line_to(0.46, -0.34);
            line.move_to(-0.46, 0.46);
            line.line_to(0.46, 0.46);
            if acronym == "PF" {
                solid.push_circle(0.0, -0.44, 0.1);
            }
        }
        "FL" => {
            line.move_to(-0.44, 0.5);
            line.line_to(-0.14, -0.5);
            line.line_to(0.14, -0.5);
            line.line_to(0.44, 0.5);
            line.close();
            line.move_to(-0.24, -0.08);
            line.line_to(0.24, -0.08);
        }
        "BL" => {
            line.move_to(-0.5, -0.52);
            line.line_to(-0.5, 0.52);
            line.move_to(0.5, -0.52);
            line.line_to(0.5, 0.52);
            line.move_to(-0.16, -0.3);
            line.line_to(-0.16, 0.3);
            line.move_to(0.16, -0.3);
            line.line_to(0.16, 0.3);
        }
        "RX" => {
            line.move_to(-0.4, 0.52);
            line.line_to(-0.4, -0.1);
            line.move_to(-0.13, 0.2);
            line.line_to(-0.13, -0.5);
            line.move_to(0.13, 0.2);
            line.line_to(0.13, -0.44);
            line.move_to(0.4, 0.2);
            line.line_to(0.4, -0.24);
            line.move_to(-0.4, 0.3);
            line.quad_to(0.0, 0.62, 0.4, 0.2);
        }
        "AP" => {
            line.move_to(-0.3, -0.52);
            line.line_to(0.34, 0.16);
            line.line_to(0.02, 0.2);
            line.line_to(0.2, 0.54);
            line.line_to(-0.02, 0.6);
            line.line_to(-0.2, 0.26);
            line.line_to(-0.42, 0.46);
            line.close();
        }
        "TD" => {
            line.move_to(-0.34, -0.56);
            line.line_to(0.34, -0.56);
            line.quad_to(0.44, -0.56, 0.44, -0.46);
            line.line_to(0.44, 0.46);
            line.quad_to(0.44, 0.56, 0.34, 0.56);
            line.line_to(-0.34, 0.56);
            line.quad_to(-0.44, 0.56, -0.44, 0.46);
            line.line_to(-0.44, -0.46);
            line.quad_to(-0.44, -0.56, -0.34, -0.56);
            line.close();
            solid.push_circle(0.0, 0.24, 0.12);
        }
        "SO" => {
            line.move_to(0.0, -0.5);
            line.quad_to(0.5, -0.5, 0.5, 0.0);
            line.quad_to(0.5, 0.5, 0.0, 0.5);
            line.quad_to(-0.5, 0.5, -0.5, 0.0);
            line.quad_to(-0.5, -0.18, -0.24, -0.2);
            line.quad_to(0.06, -0.2, 0.06, 0.06);
        }
        "AT" | "CN" => {
            if acronym == "CN" {
                line.move_to(-0.56, -0.44);
                line.line_to(0.56, -0.44);
                line.line_to(0.56, 0.44);
                line.line_to(-0.56, 0.44);
                line.close();
                solid.push_circle(-0.34, 0.0, 0.09);
                solid.push_circle(0.34, 0.0, 0.09);
            } else {
                line.move_to(-0.32, -0.5);
                line.line_to(0.44, 0.0);
                line.line_to(-0.32, 0.5);
                line.close();
            }
        }
        "MR" => {
            line.move_to(0.0, -0.62);
            line.line_to(0.0, 0.62);
            line.move_to(-0.2, -0.38);
            line.line_to(-0.62, 0.0);
            line.line_to(-0.2, 0.38);
            line.close();
            line.move_to(0.2, -0.38);
            line.line_to(0.62, 0.0);
            line.line_to(0.2, 0.38);
            line.close();
        }
        "TP" => {
            line.push_circle(0.0, 0.0, 0.5);
            line.push_circle(0.0, 0.0, 0.24);
            solid.push_circle(0.0, 0.0, 0.09);
        }
        "RD" => {
            line.move_to(-0.5, -0.5);
            line.line_to(0.5, -0.5);
            line.line_to(0.5, 0.5);
            line.line_to(-0.5, 0.5);
            line.close();
            solid.push_circle(-0.24, -0.24, 0.09);
            solid.push_circle(0.24, 0.24, 0.09);
            solid.push_circle(0.0, 0.0, 0.09);
        }
        "NM" => {
            line.push_circle(0.0, 0.0, 0.44);
            line.move_to(-0.32, 0.32);
            line.line_to(0.32, -0.32);
        }
        _ => return None,
    }
    let solid = solid.finish();
    line.finish().map(|stroked| (stroked, solid))
}

fn draw_mark(into: &mut Pixmap, acronym: &str, wide: f32, high: f32) -> bool {
    let Some((line, solid)) = mark_of(acronym) else {
        return false;
    };
    let reach = high * MARK_SHARE;
    let put = Transform::from_translate(wide / 2.0, high / 2.0).pre_scale(reach, reach);
    let paint = ink();
    into.stroke_path(&line, &paint, &pen(high * 0.115 / reach), put, None);
    if let Some(solid) = solid {
        into.fill_path(&solid, &paint, FillRule::Winding, put, None);
    }
    true
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

    let name = acronym.to_ascii_uppercase();
    if draw_mark(&mut out, &name, wide as f32, high as f32) {
        return Some(out);
    }

    let font = letters()?;
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
