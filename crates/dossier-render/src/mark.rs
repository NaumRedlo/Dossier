use std::sync::OnceLock;

use tiny_skia::{Pixmap, PremultipliedColorU8};

const MASK: &[u8] = include_bytes!("../../../native/assets/letter-mask.png");
const TOP: (f32, f32, f32) = (0.886, 0.282, 0.282);
const BOTTOM: (f32, f32, f32) = (0.788, 0.204, 0.184);
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
        let share = y as f32 / (out_high - 1).max(1) as f32;
        let colour = [TOP.0 + (BOTTOM.0 - TOP.0) * share, TOP.1 + (BOTTOM.1 - TOP.1) * share, TOP.2 + (BOTTOM.2 - TOP.2) * share];
        for x in 0..out_wide {
            let alpha = f32::from(ink(left + x, top + y)) / 255.0;
            let channel = |value: f32| (value * alpha * 255.0).round() as u8;
            if let Some(made) = PremultipliedColorU8::from_rgba(channel(colour[0]), channel(colour[1]), channel(colour[2]), (alpha * 255.0).round() as u8) {
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
    fn the_mark_is_the_letter_cut_close_and_red_from_top_to_bottom() {
        let mark = emblem().expect("the mark");
        assert!(mark.width() > 100 && mark.height() > mark.width(), "{}x{}", mark.width(), mark.height());
        let solid = |y: u32| (0..mark.width()).filter_map(|x| mark.pixel(x, y)).find(|pixel| pixel.alpha() == 255).expect("ink on this row");
        let (first, last) = (solid(2), solid(mark.height() - 3));
        assert!(first.red() > 215 && first.green() < 90, "{first:?}");
        assert!(last.red() < first.red() && last.red() > 180, "the letter deepens towards its foot: {last:?}");
        assert!((0..mark.width()).any(|x| mark.pixel(x, 0).is_some_and(|pixel| pixel.alpha() > 0)), "nothing empty is left above the letter");
    }
}
