use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use iced::widget::image;
use iced::{Color, Element};

use crate::billing::TINTS;
use crate::ui;

pub const SIDE: u32 = 160;
const LETTER: f32 = 0.78;

fn hsl(colour: Color) -> (f32, f32, f32) {
    let (r, g, b) = (colour.r, colour.g, colour.b);
    let (high, low) = (r.max(g).max(b), r.min(g).min(b));
    let light = (high + low) / 2.0;
    if (high - low).abs() < 1e-6 {
        return (0.0, 0.0, light);
    }
    let spread = high - low;
    let sat = if light > 0.5 { spread / (2.0 - high - low) } else { spread / (high + low) };
    let hue = if high == r { (g - b) / spread + if g < b { 6.0 } else { 0.0 } } else if high == g { (b - r) / spread + 2.0 } else { (r - g) / spread + 4.0 };
    (hue * 60.0, sat, light)
}

fn rgb(hue: f32, sat: f32, light: f32) -> Color {
    let chroma = (1.0 - (2.0 * light - 1.0).abs()) * sat;
    let part = hue.rem_euclid(360.0) / 60.0;
    let second = chroma * (1.0 - (part % 2.0 - 1.0).abs());
    let (r, g, b) = match part as u32 {
        0 => (chroma, second, 0.0),
        1 => (second, chroma, 0.0),
        2 => (0.0, chroma, second),
        3 => (0.0, second, chroma),
        4 => (second, 0.0, chroma),
        _ => (chroma, 0.0, second),
    };
    let lift = light - chroma / 2.0;
    Color::from_rgb(r + lift, g + lift, b + lift)
}

pub fn vivid(colour: Color, richer: f32, turn: f32) -> Color {
    let (hue, sat, light) = hsl(colour);
    rgb(hue + turn, (sat * richer).min(1.0), light)
}

fn lit(colour: Color, k: f32) -> Color {
    ui::mix(colour, Color::WHITE, k)
}

pub fn tint(rank: Option<usize>) -> Color {
    TINTS[rank.unwrap_or(0).min(TINTS.len() - 1)]
}

pub fn stops(colour: Color, stage: u8) -> Vec<(f32, Color)> {
    match stage {
        0 | 1 => vec![(0.0, colour), (1.0, colour)],
        2 => vec![(0.0, vivid(colour, 1.3, -18.0)), (0.5, vivid(colour, 1.2, 0.0)), (1.0, vivid(colour, 1.3, 22.0))],
        3 => vec![(0.0, vivid(colour, 1.45, -32.0)), (0.35, lit(vivid(colour, 1.3, 0.0), 0.22)), (0.55, vivid(colour, 1.3, 0.0)), (1.0, vivid(colour, 1.45, 36.0))],
        _ => vec![(0.0, vivid(colour, 1.6, -46.0)), (0.26, lit(vivid(colour, 1.4, 0.0), 0.55)), (0.48, vivid(colour, 1.45, 0.0)), (0.7, lit(vivid(colour, 1.5, 44.0), 0.4)), (1.0, vivid(colour, 1.6, 52.0))],
    }
}

pub fn along(stops: &[(f32, Color)], at: f32) -> Color {
    let at = at.clamp(0.0, 1.0);
    for pair in stops.windows(2) {
        if at <= pair[1].0 {
            let span = (pair[1].0 - pair[0].0).max(1e-6);
            return ui::mix(pair[0].1, pair[1].1, ((at - pair[0].0) / span).clamp(0.0, 1.0));
        }
    }
    stops.last().map_or(Color::WHITE, |stop| stop.1)
}

pub fn title_colour(rank: Option<usize>, stage: u8) -> Color {
    along(&stops(tint(rank), stage), 0.5)
}

pub fn pixels(rank: Option<usize>, stage: u8) -> Vec<u8> {
    let letter = ui::letter();
    let (mask, held) = (letter.mask(), letter.side as usize);
    let (mut weight, mut reach) = (0.0f64, 0.0f64);
    for y in 0..held {
        for x in 0..held {
            let a = mask[y * held + x] as f64;
            weight += a;
            reach += a * x as f64;
        }
    }
    let shift = if weight > 0.0 { (held as f64 - 1.0) / 2.0 - reach / weight } else { 0.0 } as f32;
    let side = SIDE as usize;
    let drawn = side as f32 * LETTER;
    let edge = (side as f32 - drawn) / 2.0;
    let sample = |x: usize, y: usize| -> f32 {
        let u = (x as f32 + 0.5 - edge) / drawn * held as f32 - 0.5 - shift;
        let v = (y as f32 + 0.5 - edge) / drawn * held as f32 - 0.5;
        if u < 0.0 || v < 0.0 || u > held as f32 - 1.0 || v > held as f32 - 1.0 {
            return 0.0;
        }
        let (x0, y0) = (u.floor() as usize, v.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(held - 1), (y0 + 1).min(held - 1));
        let (fx, fy) = (u - x0 as f32, v - y0 as f32);
        let at = |x: usize, y: usize| mask[y * held + x] as f32 / 255.0;
        let top = at(x0, y0) + (at(x1, y0) - at(x0, y0)) * fx;
        let bottom = at(x0, y1) + (at(x1, y1) - at(x0, y1)) * fx;
        top + (bottom - top) * fy
    };
    let cover: Vec<f32> = (0..side * side).map(|at| sample(at % side, at / side)).collect();
    let colour = tint(rank);
    let line = stops(colour, stage);
    let (dx, dy) = (0.866_f32, 0.5_f32);
    let length = drawn * (dx + dy);
    let halo: Option<Vec<f32>> = (stage >= 4).then(|| {
        let grey = ::image::GrayImage::from_raw(SIDE, SIDE, cover.iter().map(|a| (a * 255.0) as u8).collect()).expect("the cover has the size of the badge");
        ::image::imageops::blur(&grey, side as f32 * 0.05).into_raw().into_iter().map(|a| a as f32 / 255.0).collect()
    });
    let mut out = Vec::with_capacity(side * side * 4);
    for y in 0..side {
        for x in 0..side {
            let centre = side as f32 / 2.0;
            let t = ((x as f32 - centre) * dx + (y as f32 - centre) * dy) / length + 0.5;
            let face = along(&line, t);
            let a = cover[y * side + x];
            let (r, g, b, alpha) = match &halo {
                Some(halo) => {
                    let glow = along(&[(0.0, line[0].1), (1.0, line[line.len() - 1].1)], x as f32 / side as f32);
                    let around = (halo[y * side + x] * 0.75).min(1.0) * (1.0 - a);
                    let total = a + around;
                    if total <= 0.0 { (0.0, 0.0, 0.0, 0.0) } else { ((face.r * a + glow.r * around) / total, (face.g * a + glow.g * around) / total, (face.b * a + glow.b * around) / total, total.min(1.0)) }
                }
                None => (face.r, face.g, face.b, a),
            };
            out.extend_from_slice(&[(r * 255.0).round() as u8, (g * 255.0).round() as u8, (b * 255.0).round() as u8, (alpha * 255.0).round() as u8]);
        }
    }
    out
}

pub fn picture(rank: Option<usize>, stage: u8) -> image::Handle {
    static KEPT: OnceLock<Mutex<HashMap<(usize, u8), image::Handle>>> = OnceLock::new();
    let key = (rank.unwrap_or(0).min(TINTS.len() - 1), stage.clamp(1, 4));
    let mut kept = KEPT.get_or_init(|| Mutex::new(HashMap::new())).lock().expect("the badge pictures are not poisoned");
    kept.entry(key).or_insert_with(|| image::Handle::from_rgba(SIDE, SIDE, pixels(Some(key.0), key.1))).clone()
}

pub fn mark<'a, Message: 'a>(rank: Option<usize>, stage: u8, size: f32) -> Element<'a, Message> {
    image(picture(rank, stage)).width(size).height(size).filter_method(image::FilterMethod::Linear).opacity(ui::fade()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(pixels: &[u8], x: u32, y: u32) -> [u8; 4] {
        let from = ((y * SIDE + x) * 4) as usize;
        [pixels[from], pixels[from + 1], pixels[from + 2], pixels[from + 3]]
    }

    fn weight_centre(pixels: &[u8]) -> (f64, f64) {
        let (mut weight, mut xs, mut ys) = (0.0, 0.0, 0.0);
        for y in 0..SIDE {
            for x in 0..SIDE {
                let a = at(pixels, x, y)[3] as f64;
                weight += a;
                xs += a * x as f64;
                ys += a * y as f64;
            }
        }
        (xs / weight, ys / weight)
    }

    #[test]
    fn the_letter_of_a_badge_sits_by_its_centre_of_mass_and_keeps_clear_corners() {
        let plain = pixels(Some(1), 1);
        assert_eq!(plain.len(), (SIDE * SIDE * 4) as usize);
        let (x, y) = weight_centre(&plain);
        let middle = (SIDE as f64 - 1.0) / 2.0;
        assert!((x - middle).abs() < 1.0 && (y - middle).abs() < 2.5, "the weight of the letter is in the middle: {x} {y}");
        for corner in [(0, 0), (SIDE - 1, 0), (0, SIDE - 1), (SIDE - 1, SIDE - 1)] {
            assert_eq!(at(&plain, corner.0, corner.1)[3], 0);
        }
        let solid: Vec<[u8; 4]> = (0..SIDE * SIDE).map(|i| at(&plain, i % SIDE, i / SIDE)).filter(|p| p[3] == 255).collect();
        let tint = TINTS[1];
        assert!(solid.len() > 2000 && solid.iter().all(|p| (p[0] as f32 - tint.r * 255.0).abs() < 2.0 && (p[2] as f32 - tint.b * 255.0).abs() < 2.0), "the first stage is the flat colour of the tier");
    }

    #[test]
    fn later_stages_shimmer_and_only_the_last_one_glows_around_the_letter() {
        let spread = |pixels: &[u8]| {
            let solid: Vec<[u8; 4]> = (0..SIDE * SIDE).map(|i| at(pixels, i % SIDE, i / SIDE)).filter(|p| p[3] == 255).collect();
            let (low, high) = solid.iter().fold((255u8, 0u8), |(low, high), p| (low.min(p[0]), high.max(p[0])));
            high as i32 - low as i32
        };
        let lit = |pixels: &[u8]| (0..SIDE * SIDE).filter(|i| at(pixels, i % SIDE, i / SIDE)[3] > 0).count();
        let stages: Vec<Vec<u8>> = (1..=4).map(|stage| pixels(Some(2), stage)).collect();
        assert!(spread(&stages[0]) <= 2 && spread(&stages[1]) > 20 && spread(&stages[2]) > spread(&stages[1]) && spread(&stages[3]) > spread(&stages[2]), "each stage shimmers more than the one before");
        assert_eq!(lit(&stages[0]), lit(&stages[1]));
        assert_eq!(lit(&stages[1]), lit(&stages[2]));
        assert!(lit(&stages[3]) > lit(&stages[2]) + 1500, "the last stage has light around the letter");
        for rank in 0..TINTS.len() {
            let own = hsl(title_colour(Some(rank), 4)).0;
            let base = hsl(TINTS[rank]).0;
            assert!(((own - base + 540.0) % 360.0 - 180.0).abs() < 25.0, "the title keeps the hue of its tier at rank {rank}");
        }
        assert_eq!(title_colour(None, 1), TINTS[0]);
        assert_eq!(picture(Some(9), 9).id(), picture(Some(4), 4).id(), "pictures are made once and kept");
    }
}
