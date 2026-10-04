use std::cell::RefCell;
use std::collections::HashMap;

use iced::widget::image::Handle;
use iced::widget::{container, image, text};
use iced::{Background, Border, Color, Element};

use crate::theme;
use crate::ui;

const DARK: [u8; 3] = [34, 34, 34];
const PLAIN: [u8; 3] = [100, 100, 120];
const GLYPH_SHARE: f32 = 0.78;

const GLYPHS: &[(&str, &[u8])] = &[
    ("AD", include_bytes!("../assets/mods/AD.png")),
    ("AL", include_bytes!("../assets/mods/AL.png")),
    ("AP", include_bytes!("../assets/mods/AP.png")),
    ("AS", include_bytes!("../assets/mods/AS.png")),
    ("AT", include_bytes!("../assets/mods/AT.png")),
    ("BL", include_bytes!("../assets/mods/BL.png")),
    ("BM", include_bytes!("../assets/mods/BM.png")),
    ("BR", include_bytes!("../assets/mods/BR.png")),
    ("BU", include_bytes!("../assets/mods/BU.png")),
    ("CL", include_bytes!("../assets/mods/CL.png")),
    ("CN", include_bytes!("../assets/mods/CN.png")),
    ("CO", include_bytes!("../assets/mods/CO.png")),
    ("DA", include_bytes!("../assets/mods/DA.png")),
    ("DC", include_bytes!("../assets/mods/DC.png")),
    ("DF", include_bytes!("../assets/mods/DF.png")),
    ("DP", include_bytes!("../assets/mods/DP.png")),
    ("DT", include_bytes!("../assets/mods/DT.png")),
    ("EZ", include_bytes!("../assets/mods/EZ.png")),
    ("FI", include_bytes!("../assets/mods/FI.png")),
    ("FL", include_bytes!("../assets/mods/FL.png")),
    ("FR", include_bytes!("../assets/mods/FR.png")),
    ("GR", include_bytes!("../assets/mods/GR.png")),
    ("HD", include_bytes!("../assets/mods/HD.png")),
    ("HR", include_bytes!("../assets/mods/HR.png")),
    ("HT", include_bytes!("../assets/mods/HT.png")),
    ("MG", include_bytes!("../assets/mods/MG.png")),
    ("MR", include_bytes!("../assets/mods/MR.png")),
    ("MU", include_bytes!("../assets/mods/MU.png")),
    ("NC", include_bytes!("../assets/mods/NC.png")),
    ("NF", include_bytes!("../assets/mods/NF.png")),
    ("NM", include_bytes!("../assets/mods/NM.png")),
    ("NS", include_bytes!("../assets/mods/NS.png")),
    ("PF", include_bytes!("../assets/mods/PF.png")),
    ("RD", include_bytes!("../assets/mods/RD.png")),
    ("RP", include_bytes!("../assets/mods/RP.png")),
    ("RX", include_bytes!("../assets/mods/RX.png")),
    ("SD", include_bytes!("../assets/mods/SD.png")),
    ("SG", include_bytes!("../assets/mods/SG.png")),
    ("SI", include_bytes!("../assets/mods/SI.png")),
    ("SO", include_bytes!("../assets/mods/SO.png")),
    ("ST", include_bytes!("../assets/mods/ST.png")),
    ("SV2", include_bytes!("../assets/mods/SV2.png")),
    ("SY", include_bytes!("../assets/mods/SY.png")),
    ("TC", include_bytes!("../assets/mods/TC.png")),
    ("TD", include_bytes!("../assets/mods/TD.png")),
    ("TP", include_bytes!("../assets/mods/TP.png")),
    ("TR", include_bytes!("../assets/mods/TR.png")),
    ("WD", include_bytes!("../assets/mods/WD.png")),
    ("WG", include_bytes!("../assets/mods/WG.png")),
    ("WU", include_bytes!("../assets/mods/WU.png")),
];

thread_local! {
    static KEPT: RefCell<HashMap<String, Option<Handle>>> = RefCell::new(HashMap::new());
}

pub fn colour_of(acronym: &str) -> [u8; 3] {
    let kind = |list: &[&str]| list.contains(&acronym.to_ascii_uppercase().as_str());
    if kind(&["DC", "EZ", "HT", "NF"]) {
        [178, 255, 102]
    } else if kind(&["AC", "BL", "DT", "FI", "FL", "HD", "HR", "NC", "PF", "SD", "ST", "TC"]) {
        [255, 102, 102]
    } else if kind(&["AL", "CL", "CO", "DA", "MR", "RD", "SG", "TP"]) {
        [140, 102, 255]
    } else if kind(&["AP", "AT", "CN", "RX", "SO"]) {
        [102, 204, 255]
    } else if kind(&["AD", "AS", "BM", "BR", "BU", "DF", "DP", "FR", "GR", "MG", "MU", "NS", "RP", "SI", "SY", "TR", "WD", "WG", "WU"]) {
        [255, 102, 171]
    } else if kind(&["SV2", "V2", "TD"]) {
        [255, 204, 34]
    } else {
        PLAIN
    }
}

pub fn ink_of(colour: [u8; 3]) -> [u8; 3] {
    let light = 0.299 * f32::from(colour[0]) + 0.587 * f32::from(colour[1]) + 0.114 * f32::from(colour[2]);
    if light > 120.0 { DARK } else { [255, 255, 255] }
}

pub fn shown(acronym: &str) -> bool {
    !matches!(acronym.to_ascii_uppercase().as_str(), "CL" | "NM" | "")
}

pub fn has_glyph(acronym: &str) -> bool {
    let upper = acronym.to_ascii_uppercase();
    GLYPHS.iter().any(|(name, _)| *name == upper)
}

fn glyph(acronym: &str) -> Option<Handle> {
    let upper = acronym.to_ascii_uppercase();
    if let Some(known) = KEPT.with(|kept| kept.borrow().get(&upper).cloned()) {
        return known;
    }
    let ink = ink_of(colour_of(&upper));
    let made = GLYPHS.iter().find(|(name, _)| *name == upper).and_then(|(_, bytes)| ::image::load_from_memory(bytes).ok()).map(|picture| {
        let mut rgba = picture.to_rgba8();
        for pixel in rgba.pixels_mut() {
            pixel.0[0] = ink[0];
            pixel.0[1] = ink[1];
            pixel.0[2] = ink[2];
        }
        Handle::from_rgba(rgba.width(), rgba.height(), rgba.into_raw())
    });
    KEPT.with(|kept| kept.borrow_mut().insert(upper, made.clone()));
    made
}

pub fn badge<'a, Message: 'a>(acronym: &str, size: f32, strength: f32) -> Element<'a, Message> {
    let k = ui::fade() * strength;
    let fill = colour_of(acronym);
    let ink = ink_of(fill);
    let ink_colour = Color::from_rgb8(ink[0], ink[1], ink[2]);
    let inside: Element<'a, Message> = match glyph(acronym) {
        Some(handle) => image(handle).width(size * GLYPH_SHARE).height(size * GLYPH_SHARE).opacity(k).into(),
        None => text(acronym.to_ascii_uppercase()).font(theme::MONO_BOLD).size(size * 0.4).wrapping(text::Wrapping::None).color(Color { a: k, ..ink_colour }).into(),
    };
    container(inside)
        .width(size)
        .height(size)
        .center_x(size)
        .center_y(size)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color { a: k, ..Color::from_rgb8(fill[0], fill[1], fill[2]) })),
            border: Border { color: Color::from_rgba(0.0, 0.0, 0.0, 80.0 / 255.0 * k), width: 1.0, radius: (size / 2.0).into() },
            ..container::Style::default()
        })
        .into()
}

pub fn chip<'a, Message: 'a>(acronym: &str) -> Element<'a, Message> {
    let k = ui::fade();
    container(text(acronym.to_owned()).font(theme::MONO_BOLD).size(11.0).color(ui::faded(theme::MUTED)))
        .padding([4, 9])
        .style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.02 * k))),
            border: Border { radius: 8.0.into(), ..Border::default() },
            ..container::Style::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mod_has_the_colour_of_what_it_does_to_the_game_as_in_the_bot() {
        assert_eq!(colour_of("HD"), [255, 102, 102]);
        assert_eq!(colour_of("dt"), [255, 102, 102]);
        assert_eq!(colour_of("EZ"), [178, 255, 102]);
        assert_eq!(colour_of("AT"), [102, 204, 255]);
        assert_eq!(colour_of("RD"), [140, 102, 255]);
        assert_eq!(colour_of("WU"), [255, 102, 171]);
        assert_eq!(colour_of("SV2"), [255, 204, 34]);
        assert_eq!(colour_of("NM"), PLAIN, "a mod the table does not know keeps the plain colour");
    }

    #[test]
    fn the_ink_is_dark_on_a_light_disc_and_white_on_a_dark_one() {
        for acronym in ["HD", "EZ", "AT", "RD", "WU", "SV2"] {
            assert_eq!(ink_of(colour_of(acronym)), DARK, "{acronym}");
        }
        assert_eq!(ink_of(PLAIN), [255, 255, 255]);
    }

    #[test]
    fn the_classic_and_the_no_mod_marks_are_not_shown_in_a_list() {
        assert!(!shown("CL"));
        assert!(!shown("nm"));
        assert!(!shown(""));
        assert!(shown("HD"));
        assert!(shown("SV2"));
    }

    #[test]
    fn every_glyph_is_a_picture_that_can_be_read_and_the_ink_goes_into_it() {
        for (name, _) in GLYPHS {
            assert!(glyph(name).is_some(), "{name} reads");
            assert!(has_glyph(name));
        }
        assert!(!has_glyph("FM"));
        assert!(!has_glyph("TB"));
        assert!(glyph("FM").is_none());
        let Some(Handle::Rgba { pixels, .. }) = glyph("HD") else { panic!("raw pixels") };
        assert!(pixels.chunks_exact(4).filter(|pixel| pixel[3] > 200).all(|pixel| pixel[..3] == DARK), "an opaque pixel carries the ink");
    }

    #[test]
    fn a_badge_is_drawn_with_and_without_a_glyph() {
        let _ = badge::<()>("HD", 24.0, 1.0);
        let _ = badge::<()>("FM", 24.0, 0.5);
    }
}
