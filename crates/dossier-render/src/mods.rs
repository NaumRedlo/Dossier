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
    HELD.get_or_init(|| Font::from_bytes(LETTERS).ok()).as_ref()
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
    into.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
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

const GAME_NAMES: &[(&str, &str)] = &[
    ("EZ", "easy"),
    ("NF", "nofail"),
    ("HT", "halftime"),
    ("HR", "hardrock"),
    ("SD", "suddendeath"),
    ("PF", "perfect"),
    ("DT", "doubletime"),
    ("NC", "nightcore"),
    ("HD", "hidden"),
    ("FL", "flashlight"),
    ("RX", "relax"),
    ("AP", "relax2"),
    ("SO", "spunout"),
    ("AT", "autoplay"),
    ("CN", "cinema"),
    ("V2", "scorev2"),
    ("TD", "touchdevice"),
    ("TP", "target"),
];

const GAME_ICON_RISE: f32 = 1.45;

pub type Icons = std::collections::HashMap<String, Pixmap>;

pub fn game_name(acronym: &str) -> Option<&'static str> {
    GAME_NAMES.iter().find(|(name, _)| name.eq_ignore_ascii_case(acronym)).map(|(_, stem)| *stem)
}

pub fn icons_in(folder: &std::path::Path) -> Icons {
    let mut held: std::collections::HashMap<String, std::path::PathBuf> = std::collections::HashMap::new();
    if let Ok(entries) = std::fs::read_dir(folder) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                held.insert(name.to_ascii_lowercase(), entry.path());
            }
        }
    }
    let mut out = Icons::new();
    for (acronym, stem) in GAME_NAMES {
        let found = [format!("selection-mod-{stem}@2x.png"), format!("selection-mod-{stem}.png")]
            .iter()
            .find_map(|name| held.get(name))
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| Pixmap::decode_png(&bytes).ok())
            .filter(|picture| picture.pixels().iter().any(|pixel| pixel.alpha() > 0));
        if let Some(picture) = found {
            out.insert((*acronym).to_owned(), picture);
        }
    }
    out
}

fn sized(acronym: &str, high: u32, icons: &Icons) -> Option<(Pixmap, f32)> {
    let Some(picture) = icons.get(&acronym.to_ascii_uppercase()) else {
        return icon(acronym, high).map(|plate| (plate, 1.0));
    };
    let scale = high as f32 * GAME_ICON_RISE / picture.height().max(1) as f32;
    Some((picture.clone(), scale))
}

pub fn row_with(acronyms: &[String], high: u32, icons: &Icons) -> Option<Pixmap> {
    let wanted: Vec<(Pixmap, f32)> = acronyms.iter().filter(|one| known(one)).filter_map(|one| sized(one, high, icons)).collect();
    if wanted.is_empty() {
        return None;
    }
    let gap = high as f32 * GAP;
    let wide: f32 = wanted.iter().map(|(picture, scale)| picture.width() as f32 * scale).sum::<f32>() + gap * (wanted.len() - 1) as f32;
    let tall = wanted.iter().map(|(picture, scale)| picture.height() as f32 * scale).fold(0.0f32, f32::max);
    let mut out = Pixmap::new(wide.ceil().max(1.0) as u32, tall.ceil().max(1.0) as u32)?;
    let mut x = 0.0f32;
    for (picture, scale) in &wanted {
        let y = (tall - picture.height() as f32 * scale) / 2.0;
        out.draw_pixmap(
            0,
            0,
            picture.as_ref(),
            &PixmapPaint { quality: tiny_skia::FilterQuality::Bilinear, ..PixmapPaint::default() },
            Transform::from_scale(*scale, *scale).post_translate(x, y),
            None,
        );
        x += picture.width() as f32 * scale + gap;
    }
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
    fn the_games_own_icon_stands_in_the_row_where_there_is_one() {
        let dir = std::env::temp_dir().join(format!("dossier-mod-icons-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut drawn = Pixmap::new(68, 66).unwrap();
        drawn.fill(Color::from_rgba8(10, 200, 30, 255));
        std::fs::write(dir.join("selection-mod-hidden.png"), drawn.encode_png().unwrap()).unwrap();
        let mut twice = Pixmap::new(136, 132).unwrap();
        twice.fill(Color::from_rgba8(200, 10, 30, 255));
        std::fs::write(dir.join("Selection-Mod-DoubleTime@2x.png"), twice.encode_png().unwrap()).unwrap();
        std::fs::write(dir.join("selection-mod-doubletime.png"), drawn.encode_png().unwrap()).unwrap();
        std::fs::write(dir.join("selection-mod-easy.png"), Pixmap::new(68, 66).unwrap().encode_png().unwrap()).unwrap();
        let icons = icons_in(&dir);
        let mut names: Vec<&str> = icons.keys().map(String::as_str).collect();
        names.sort_unstable();
        assert_eq!(names, ["DT", "HD"], "a blank picture is no icon, and the sharper file wins");
        assert_eq!(icons["DT"].width(), 136);
        let mods = vec!["HD".to_owned(), "DT".to_owned(), "CL".to_owned()];
        let strip = row_with(&mods, 20, &icons).expect("a row");
        let pixel = |x: u32, y: u32| strip.pixel(x, y).unwrap();
        assert_eq!(strip.height(), 29, "the game's icon stands taller than a plate");
        assert!(pixel(5, 14).green() > 150 && pixel(5, 14).red() < 60, "the first is the game's green picture");
        assert!(pixel(40, 14).red() > 150, "the second is the sharper red picture, drawn at the same height");
        assert!(strip.width() > 29 * 2 + 20, "a mod the game has no picture for keeps its plate");
        assert_eq!(row_with(&mods, 20, &Icons::new()).map(|plain| plain.height()), row(&mods, 20).map(|plain| plain.height()));
        assert_eq!((game_name("hd"), game_name("CL")), (Some("hidden"), None));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_letters_are_there_to_draw_with() {
        assert!(letters().is_some(), "the badge font did not load");
    }

    #[test]
    fn every_mod_wears_its_own_name() {
        for name in every() {
            let made = icon(name, 100).unwrap_or_else(|| panic!("{name} draws"));
            assert_eq!((made.width(), made.height()), (162, 100));
            let white = made
                .pixels()
                .iter()
                .filter(|p| p.red() > 230 && p.green() > 230 && p.blue() > 230)
                .count();
            assert!(white > 200, "{name}: only {white} pixels of lettering");
        }
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
