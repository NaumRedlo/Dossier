use std::sync::OnceLock;

use tiny_skia::{Pixmap, PremultipliedColorU8};

const MASK: &[u8] = include_bytes!("../../../native/assets/letter-mask.png");
const INK_FROM: u8 = 8;

pub fn emblem() -> Option<&'static Pixmap> {
    static HELD: OnceLock<Option<Pixmap>> = OnceLock::new();
    HELD.get_or_init(drawn).as_ref()
}

fn drawn() -> Option<Pixmap> {
    let mask = Pixmap::decode_png(MASK).ok()?;
    let (wide, high) = (mask.width(), mask.height());
    let ink = |x: u32, y: u32| mask.pixel(x, y).map_or(0, |pixel| pixel.red());
    let (mut left, mut top, mut right, mut bottom) = (wide, high, 0, 0);
    for y in 0..high {
        for x in 0..wide {
            if ink(x, y) > INK_FROM {
                left = left.min(x);
                top = top.min(y);
                right = right.max(x);
                bottom = bottom.max(y);
            }
        }
    }
    if right < left || bottom < top {
        return None;
    }
    let (out_wide, out_high) = (right - left + 1, bottom - top + 1);
    let mut out = Pixmap::new(out_wide, out_high)?;
    let pixels = out.pixels_mut();
    for y in 0..out_high {
        for x in 0..out_wide {
            let alpha = ink(left + x, top + y);
            if let Some(made) = PremultipliedColorU8::from_rgba(alpha, alpha, alpha, alpha) {
                pixels[(y * out_wide + x) as usize] = made;
            }
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mark_is_the_letter_cut_close_and_white() {
        let mark = emblem().expect("the mark");
        assert!(mark.width() > 100 && mark.height() > mark.width(), "{}x{}", mark.width(), mark.height());
        let solid = |y: u32| (0..mark.width()).filter_map(|x| mark.pixel(x, y)).find(|pixel| pixel.alpha() == 255).expect("ink on this row");
        for row in [2, mark.height() / 2, mark.height() - 3] {
            let ink = solid(row);
            assert_eq!((ink.red(), ink.green(), ink.blue()), (255, 255, 255), "row {row}");
        }
        assert!((0..mark.width()).any(|x| mark.pixel(x, 0).is_some_and(|pixel| pixel.alpha() > 0)), "nothing empty is left above the letter");
    }
}
