use std::collections::HashMap;
use std::sync::OnceLock;

use tiny_skia::{Color, Pixmap, PixmapPaint, Transform};

const BASE: &[u8] = include_bytes!("../../../assets/mods/mod-icon.png");
const EXTENDER: &[u8] = include_bytes!("../../../assets/mods/mod-icon-extender.png");

pub const RATIO: f32 = 135.0 / 100.0;

const GLYPH_SHARE: f32 = 0.62;

const OVERLAP: f32 = 0.72;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Easier,
    Harder,
    Automatic,
    Converted,
    Other,
}

const KNOWN: &[(&str, Kind, &[u8])] = &[
    (
        "NM",
        Kind::Other,
        include_bytes!("../../../assets/mods/mod-no-mod.png"),
    ),
    (
        "EZ",
        Kind::Easier,
        include_bytes!("../../../assets/mods/mod-easy.png"),
    ),
    (
        "NF",
        Kind::Easier,
        include_bytes!("../../../assets/mods/mod-no-fail.png"),
    ),
    (
        "HT",
        Kind::Easier,
        include_bytes!("../../../assets/mods/mod-half-time.png"),
    ),
    (
        "HD",
        Kind::Harder,
        include_bytes!("../../../assets/mods/mod-hidden.png"),
    ),
    (
        "HR",
        Kind::Harder,
        include_bytes!("../../../assets/mods/mod-hard-rock.png"),
    ),
    (
        "SD",
        Kind::Harder,
        include_bytes!("../../../assets/mods/mod-sudden-death.png"),
    ),
    (
        "PF",
        Kind::Harder,
        include_bytes!("../../../assets/mods/mod-perfect.png"),
    ),
    (
        "DT",
        Kind::Harder,
        include_bytes!("../../../assets/mods/mod-double-time.png"),
    ),
    (
        "NC",
        Kind::Harder,
        include_bytes!("../../../assets/mods/mod-nightcore.png"),
    ),
    (
        "FL",
        Kind::Harder,
        include_bytes!("../../../assets/mods/mod-flashlight.png"),
    ),
    (
        "RX",
        Kind::Automatic,
        include_bytes!("../../../assets/mods/mod-relax.png"),
    ),
    (
        "AP",
        Kind::Automatic,
        include_bytes!("../../../assets/mods/mod-autopilot.png"),
    ),
    (
        "SO",
        Kind::Automatic,
        include_bytes!("../../../assets/mods/mod-spun-out.png"),
    ),
    (
        "AT",
        Kind::Automatic,
        include_bytes!("../../../assets/mods/mod-autoplay.png"),
    ),
    (
        "CN",
        Kind::Automatic,
        include_bytes!("../../../assets/mods/mod-cinema.png"),
    ),
    (
        "TD",
        Kind::Converted,
        include_bytes!("../../../assets/mods/mod-touch-device.png"),
    ),
    (
        "MR",
        Kind::Converted,
        include_bytes!("../../../assets/mods/mod-mirror.png"),
    ),
    (
        "V2",
        Kind::Converted,
        include_bytes!("../../../assets/mods/mod-score-v2.png"),
    ),
    (
        "TP",
        Kind::Converted,
        include_bytes!("../../../assets/mods/mod-target-practice.png"),
    ),
];

pub fn kind_of(acronym: &str) -> Kind {
    KNOWN
        .iter()
        .find(|(name, _, _)| name.eq_ignore_ascii_case(acronym))
        .map_or(Kind::Other, |(_, kind, _)| *kind)
}

pub fn colour_of(kind: Kind) -> Color {
    match kind {
        Kind::Easier => Color::from_rgba8(136, 214, 76, 255),
        Kind::Harder => Color::from_rgba8(226, 86, 76, 255),
        Kind::Automatic => Color::from_rgba8(95, 159, 226, 255),
        Kind::Converted => Color::from_rgba8(160, 107, 224, 255),
        Kind::Other => Color::from_rgba8(139, 116, 119, 255),
    }
}

pub fn known(acronym: &str) -> bool {
    KNOWN
        .iter()
        .any(|(name, _, _)| name.eq_ignore_ascii_case(acronym))
}

pub fn every() -> Vec<&'static str> {
    KNOWN.iter().map(|(name, _, _)| *name).collect()
}

fn glyph_bytes(acronym: &str) -> Option<&'static [u8]> {
    KNOWN
        .iter()
        .find(|(name, _, _)| name.eq_ignore_ascii_case(acronym))
        .map(|(_, _, bytes)| *bytes)
}

fn decoded() -> &'static HashMap<&'static str, Pixmap> {
    static HELD: OnceLock<HashMap<&'static str, Pixmap>> = OnceLock::new();
    HELD.get_or_init(|| {
        let mut all = HashMap::new();
        if let Ok(base) = Pixmap::decode_png(BASE) {
            all.insert("", base);
        }
        if let Ok(wide) = Pixmap::decode_png(EXTENDER) {
            all.insert(" ", wide);
        }
        for (name, _, bytes) in KNOWN {
            if let Ok(glyph) = Pixmap::decode_png(bytes) {
                all.insert(*name, glyph);
            }
        }
        all
    })
}

fn painted(art: &Pixmap, tint: Option<Color>) -> Pixmap {
    match tint {
        Some(colour) => crate::imported::tinted(art, colour),
        None => art.clone(),
    }
}

fn stamp(into: &mut Pixmap, art: &Pixmap, left: f32, top: f32, high: f32, alpha: f32) {
    let scale = high / art.height() as f32;
    into.draw_pixmap(
        0,
        0,
        art.as_ref(),
        &PixmapPaint {
            opacity: alpha.clamp(0.0, 1.0),
            quality: tiny_skia::FilterQuality::Bilinear,
            ..Default::default()
        },
        Transform::from_translate(left, top).pre_scale(scale, scale),
        None,
    );
}

pub fn icon(acronym: &str, high: u32) -> Option<Pixmap> {
    let held = decoded();
    let base = held.get("")?;
    let glyph = held.get(
        KNOWN
            .iter()
            .find(|(name, _, _)| name.eq_ignore_ascii_case(acronym))
            .map(|(name, _, _)| *name)?,
    )?;
    let _ = glyph_bytes(acronym)?;

    let high = high.max(8);
    let wide = (high as f32 * RATIO).round() as u32;
    let mut out = Pixmap::new(wide, high)?;

    let ground = painted(base, Some(colour_of(kind_of(acronym))));
    stamp(&mut out, &ground, 0.0, 0.0, high as f32, 1.0);

    let mark = high as f32 * GLYPH_SHARE;
    let mark_wide = mark * glyph.width() as f32 / glyph.height() as f32;
    stamp(
        &mut out,
        glyph,
        (wide as f32 - mark_wide) / 2.0,
        (high as f32 - mark) / 2.0,
        mark,
        1.0,
    );
    Some(out)
}

pub fn row_width(count: usize, high: u32) -> u32 {
    if count == 0 {
        return 0;
    }
    let one = high as f32 * RATIO;
    (one + one * OVERLAP * (count - 1) as f32).round() as u32
}

pub fn row(acronyms: &[String], high: u32) -> Option<Pixmap> {
    let wanted: Vec<&String> = acronyms.iter().filter(|one| known(one)).collect();
    if wanted.is_empty() {
        return None;
    }
    let one = high as f32 * RATIO;
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
            Transform::from_translate(one * OVERLAP * at as f32, 0.0),
            None,
        );
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_named_mod_carries_a_glyph_that_decodes() {
        for name in every() {
            assert!(glyph_bytes(name).is_some(), "{name} has no picture");
            assert!(decoded().contains_key(name), "{name} did not decode");
        }
        assert!(decoded().contains_key(""), "the hexagon did not decode");
        assert!(decoded().contains_key(" "), "the extender did not decode");
    }

    #[test]
    fn an_icon_is_the_shape_of_the_hexagon_and_is_not_blank() {
        let made = icon("HD", 100).expect("hidden draws");
        assert_eq!((made.width(), made.height()), (135, 100));
        let filled = made.pixels().iter().filter(|p| p.alpha() > 0).count();
        assert!(filled > 5_000, "only {filled} pixels have ink");
    }

    #[test]
    fn the_kind_decides_the_colour_and_unknown_names_stay_grey() {
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
    fn a_row_grows_with_every_mod_and_keeps_them_overlapping() {
        let one = row_width(1, 100);
        let three = row_width(3, 100);
        assert_eq!(one, 135);
        assert!(three > one && three < one * 3, "{three}");
        let made = row(&["HD".to_owned(), "HR".to_owned()], 60).expect("a row draws");
        assert_eq!(made.width(), row_width(2, 60));
        assert!(row(&["ZZ".to_owned()], 60).is_none());
        assert!(row(&[], 60).is_none());
    }
}
