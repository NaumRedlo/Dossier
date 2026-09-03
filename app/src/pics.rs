//! The pictures the viewer needs out of a skin.
//!
//! The window draws the play on a canvas, so it cannot ask the renderer for a
//! frame — it needs the art itself. These are the few elements that decide what
//! a play *looks* like: the note, its overlay, the ring that closes in, the
//! cursor and the ten digits.
//!
//! They come back as data URLs. A skin's files are on this machine and the
//! window could be pointed at them, except that it is served from `tauri://`
//! and a local path is not reachable from there — and opening one up would be
//! opening the whole disk to a page.

use std::path::Path;

use dossier_render::elements::Element;
use dossier_render::imported::Sprites;

#[derive(Default, serde::Serialize)]
pub struct Pictures {
    pub circle: Option<Picture>,
    pub overlay: Option<Picture>,
    pub approach: Option<Picture>,
    pub cursor: Option<Picture>,
    /// The ten combo digits, in order. Empty unless every one of them is there:
    /// a number drawn half from the skin and half from the engine is worse than
    /// one drawn wholly from either.
    pub digits: Vec<Picture>,
    /// The skin's own combo colours, when it states any.
    pub colours: Vec<String>,
}

#[derive(serde::Serialize)]
pub struct Picture {
    /// `data:image/png;base64,…`
    pub src: String,
    /// File pixels per osu! pixel: 2 for an `@2x` file, 1 otherwise.
    pub scale: f32,
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Base64, written out rather than depended on: it is fifteen lines and the
/// alternative is a crate in the tree for one call.
fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[((n >> (18 - i * 6)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn picture(sprites: &Sprites, element: Element) -> Option<Picture> {
    let sprite = sprites.get(element)?;
    let png = sprite.pixmap.encode_png().ok()?;
    Some(Picture {
        src: format!("data:image/png;base64,{}", base64(&png)),
        scale: sprite.scale,
    })
}

/// Read what the viewer draws with out of one skin folder.
pub fn of(folder: &Path) -> Pictures {
    let wanted: Vec<Element> = [
        Element::HitCircle,
        Element::HitCircleOverlay,
        Element::ApproachCircle,
        Element::Cursor,
    ]
    .into_iter()
    .chain((0..10).map(Element::Digit))
    .collect();
    let sprites = Sprites::read(folder, &wanted);

    let digits: Vec<Picture> = (0..10)
        .filter_map(|n| picture(&sprites, Element::Digit(n)))
        .collect();
    Pictures {
        circle: picture(&sprites, Element::HitCircle),
        overlay: picture(&sprites, Element::HitCircleOverlay),
        approach: picture(&sprites, Element::ApproachCircle),
        cursor: picture(&sprites, Element::Cursor),
        digits: if digits.len() == 10 {
            digits
        } else {
            Vec::new()
        },
        colours: sprites
            .ini()
            .combo_colours
            .iter()
            .map(|c| {
                let (r, g, b) = (c.red(), c.green(), c.blue());
                format!(
                    "#{:02x}{:02x}{:02x}",
                    (r * 255.0) as u8,
                    (g * 255.0) as u8,
                    (b * 255.0) as u8
                )
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_says_what_everybody_elses_says() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn a_folder_that_is_not_a_skin_gives_nothing_rather_than_failing() {
        let said = of(Path::new("/definitely/not/here"));
        assert!(said.circle.is_none());
        assert!(said.digits.is_empty());
    }
}
