use std::collections::HashMap;

use iced::advanced::image::Id;
use iced::widget::image::Handle;

const KEPT: usize = 256;

thread_local! {
    static CROPS: std::cell::RefCell<HashMap<(Id, u32, u32), Handle>> = std::cell::RefCell::new(HashMap::new());
}

pub fn region(source: (u32, u32), wide: f32, high: f32) -> (u32, u32, u32, u32) {
    let (width, height) = source;
    let target = wide / high;
    let have = width as f32 / height as f32;
    let (cut_wide, cut_high) = if target >= have {
        (width, ((width as f32 / target).round() as u32).clamp(1, height))
    } else {
        (((height as f32 * target).round() as u32).clamp(1, width), height)
    };
    ((width - cut_wide) / 2, (height - cut_high) / 2, cut_wide, cut_high)
}

pub fn coverage(x: f32, y: f32, wide: f32, high: f32, round: f32) -> f32 {
    let round = round.min(wide / 2.0).min(high / 2.0);
    let dx = (x - wide / 2.0).abs() - (wide / 2.0 - round);
    let dy = (y - high / 2.0).abs() - (high / 2.0 - round);
    let outside = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt();
    let inside = dx.max(dy).min(0.0);
    (0.5 - (outside + inside - round)).clamp(0.0, 1.0)
}

fn rounded(pixels: &mut [u8], width: u32, height: u32, wide: f32, high: f32, round: f32) {
    for row in 0..height {
        for column in 0..width {
            let x = (column as f32 + 0.5) / width as f32 * wide;
            let y = (row as f32 + 0.5) / height as f32 * high;
            let seen = coverage(x, y, wide, high, round);
            if seen < 1.0 {
                let at = ((row * width + column) * 4 + 3) as usize;
                pixels[at] = (f32::from(pixels[at]) * seen).round() as u8;
            }
        }
    }
}

const CALM: f32 = 0.34;
const DEEPEST: f32 = 0.55;
const DIMMED: u32 = 2_000_000_000;

pub fn luminance(pixels: &[u8]) -> f32 {
    let mut sum = 0.0f64;
    let mut seen = 0u32;
    for pixel in pixels.chunks_exact(4).step_by(7) {
        sum += f64::from(pixel[0]) * 0.2126 + f64::from(pixel[1]) * 0.7152 + f64::from(pixel[2]) * 0.0722;
        seen += 1;
    }
    if seen == 0 { 0.0 } else { (sum / f64::from(seen) / 255.0) as f32 }
}

pub fn calm(luminance: f32) -> f32 {
    if luminance <= CALM { 1.0 } else { (CALM / luminance).clamp(DEEPEST, 1.0) }
}

fn dim(pixels: &mut [u8]) {
    let k = calm(luminance(pixels));
    if k < 1.0 {
        for pixel in pixels.chunks_exact_mut(4) {
            for channel in &mut pixel[..3] {
                *channel = (f32::from(*channel) * k).round() as u8;
            }
        }
    }
}

const SUNK_KEPT: usize = 6;
const SUNK_LEVEL: f32 = 0.4;

thread_local! {
    static SUNK: std::cell::RefCell<Vec<((Id, u32, u32, u8, [u8; 3]), Handle)>> = const { std::cell::RefCell::new(Vec::new()) };
}

pub fn sink(pixels: &mut [u8], width: u32, height: u32, shade: f32, ground: [u8; 3]) {
    for row in 0..height {
        let down = (row as f32 + 0.5) / height.max(1) as f32;
        let fall = ((down - SUNK_LEVEL) / (1.0 - SUNK_LEVEL)).clamp(0.0, 1.0);
        let cover = if row + 1 == height { 1.0 } else { shade + (1.0 - shade) * fall * fall * (3.0 - 2.0 * fall) };
        let from = (row * width * 4) as usize;
        for pixel in pixels[from..from + (width * 4) as usize].chunks_exact_mut(4) {
            for (channel, base) in pixel[..3].iter_mut().zip(ground) {
                *channel = (f32::from(*channel) * (1.0 - cover) + f32::from(base) * cover).round() as u8;
            }
            pixel[3] = 255;
        }
    }
}

pub fn sunk(handle: &Handle, wide: f32, high: f32, dim: u8, ground: [u8; 3]) -> Handle {
    let Handle::Rgba { id, width, height, pixels } = handle else {
        return handle.clone();
    };
    if wide < 1.0 || high < 1.0 {
        return handle.clone();
    }
    let key = (*id, (wide / 16.0).round() as u32, high.round() as u32, dim, ground);
    if let Some(kept) = SUNK.with(|kept| kept.borrow().iter().find(|(held, _)| *held == key).map(|(_, made)| made.clone())) {
        return kept;
    }
    let (left, top, cut_wide, cut_high) = region((*width, *height), wide, high);
    let mut cut = Vec::with_capacity((cut_wide * cut_high * 4) as usize);
    for row in top..top + cut_high {
        let from = ((row * width + left) * 4) as usize;
        cut.extend_from_slice(&pixels[from..from + (cut_wide * 4) as usize]);
    }
    sink(&mut cut, cut_wide, cut_high, f32::from(dim.min(100)) / 100.0, ground);
    let made = Handle::from_rgba(cut_wide, cut_high, cut);
    SUNK.with(|kept| {
        let mut kept = kept.borrow_mut();
        if kept.len() >= SUNK_KEPT {
            kept.remove(0);
        }
        kept.push((key, made.clone()));
    });
    made
}

pub fn fitted(handle: &Handle, wide: f32, high: f32, round: f32) -> Handle {
    cut(handle, wide, high, round, false)
}

pub fn shaded(handle: &Handle, wide: f32, high: f32, round: f32) -> Handle {
    cut(handle, wide, high, round, true)
}

fn cut(handle: &Handle, wide: f32, high: f32, round: f32, calmed: bool) -> Handle {
    let Handle::Rgba { id, width, height, pixels } = handle else {
        return handle.clone();
    };
    if wide < 1.0 || high < 1.0 {
        return handle.clone();
    }
    let key = (*id, (wide * 10.0).round() as u32, (high * 10.0).round() as u32 + (round.max(0.0) * 10.0).round() as u32 * 100_000 + if calmed { DIMMED } else { 0 });
    if let Some(kept) = CROPS.with(|crops| crops.borrow().get(&key).cloned()) {
        return kept;
    }
    let (left, top, cut_wide, cut_high) = region((*width, *height), wide, high);
    let made = if cut_wide == *width && cut_high == *height && round <= 0.0 && !calmed {
        handle.clone()
    } else {
        let mut cut = Vec::with_capacity((cut_wide * cut_high * 4) as usize);
        for row in top..top + cut_high {
            let from = ((row * width + left) * 4) as usize;
            cut.extend_from_slice(&pixels[from..from + (cut_wide * 4) as usize]);
        }
        if calmed {
            dim(&mut cut);
        }
        if round > 0.0 {
            rounded(&mut cut, cut_wide, cut_high, wide, high, round);
        }
        Handle::from_rgba(cut_wide, cut_high, cut)
    };
    CROPS.with(|crops| {
        let mut crops = crops.borrow_mut();
        if crops.len() >= KEPT {
            crops.clear();
        }
        crops.insert(key, made.clone());
    });
    made
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sunk_picture_is_dimmed_at_the_top_and_ends_in_the_ground_colour() {
        let (width, height) = (4u32, 100u32);
        let mut pixels = vec![255u8; (width * height * 4) as usize];
        sink(&mut pixels, width, height, 0.6, [13, 5, 8]);
        assert_eq!(&pixels[..4], &[110, 105, 107, 255], "the top keeps the chosen share of the picture");
        let middle = (50 * width * 4) as usize;
        assert!(pixels[middle] < 110 && pixels[middle] > 13, "below the level line it goes down");
        let last = ((height - 1) * width * 4) as usize;
        assert_eq!(&pixels[last..last + 4], &[13, 5, 8, 255], "the last row is the ground itself, so no edge is left");
    }

    #[test]
    fn a_wider_target_keeps_the_whole_width_and_cuts_rows_from_both_ends() {
        assert_eq!(region((176, 100), 368.0, 150.0), (0, 14, 176, 72));
    }

    #[test]
    fn a_taller_target_keeps_the_whole_height_and_cuts_columns_from_both_sides() {
        assert_eq!(region((176, 100), 68.0, 92.0), (51, 0, 74, 100));
    }

    #[test]
    fn a_matching_shape_cuts_nothing() {
        assert_eq!(region((160, 90), 320.0, 180.0), (0, 0, 160, 90));
    }

    #[test]
    fn the_cut_takes_the_middle_pixels() {
        let (width, height) = (8u32, 4u32);
        let mut pixels = Vec::new();
        for _y in 0..height {
            for x in 0..width {
                pixels.extend_from_slice(&[x as u8, 0, 0, 255]);
            }
        }
        let handle = Handle::from_rgba(width, height, pixels);
        let cut = fitted(&handle, 4.0, 4.0, 0.0);
        let Handle::Rgba { width: cut_wide, height: cut_high, pixels, .. } = cut else { panic!("a raw handle") };
        assert_eq!((cut_wide, cut_high), (4, 4));
        assert_eq!(pixels[0], 2, "the first column kept is the third of eight");
        assert_eq!(pixels[(3 * 4) as usize], 5, "and the last is the sixth");
        let again = fitted(&handle, 4.0, 4.0, 0.0);
        assert_eq!(again.id(), cut_id(&handle), "the same cut is reused");
    }

    fn cut_id(handle: &Handle) -> Id {
        fitted(handle, 4.0, 4.0, 0.0).id()
    }

    #[test]
    fn a_handle_that_is_not_raw_pixels_is_left_alone() {
        let handle = Handle::from_bytes(vec![1u8, 2, 3]);
        assert_eq!(fitted(&handle, 10.0, 5.0, 4.0).id(), handle.id());
    }

    #[test]
    fn the_corners_fade_to_nothing_and_the_middle_stays_solid() {
        assert_eq!(coverage(50.0, 30.0, 100.0, 60.0, 12.0), 1.0);
        assert_eq!(coverage(12.0, 0.5, 100.0, 60.0, 12.0), 1.0, "the straight edge is whole");
        assert_eq!(coverage(0.2, 0.2, 100.0, 60.0, 12.0), 0.0, "the very corner is gone");
        let half = coverage(12.0 - 12.0 * std::f32::consts::FRAC_1_SQRT_2, 12.0 - 12.0 * std::f32::consts::FRAC_1_SQRT_2, 100.0, 60.0, 12.0);
        assert!((half - 0.5).abs() < 0.01, "the arc itself is half covered: {half}");
        assert_eq!(coverage(0.0, 30.0, 100.0, 60.0, 0.0), 0.5, "no rounding keeps square edges");
    }

    #[test]
    fn a_rounded_cut_has_transparent_corners_and_an_opaque_centre() {
        let (width, height) = (40u32, 20u32);
        let handle = Handle::from_rgba(width, height, vec![255u8; (width * height * 4) as usize]);
        let Handle::Rgba { width: cut_wide, height: cut_high, pixels, .. } = fitted(&handle, 80.0, 40.0, 10.0) else { panic!("a raw handle") };
        assert_eq!((cut_wide, cut_high), (width, height));
        let alpha = |x: u32, y: u32| pixels[((y * cut_wide + x) * 4 + 3) as usize];
        assert_eq!(alpha(0, 0), 0);
        assert_eq!(alpha(cut_wide - 1, cut_high - 1), 0);
        assert_eq!(alpha(20, 10), 255);
        assert_eq!(alpha(20, 0), 255);
        assert_eq!(alpha(0, 10), 255);
    }

    fn solid(level: u8) -> Handle {
        Handle::from_rgba(4, 4, [level, level, level, 255].repeat(16))
    }

    fn first_level(handle: &Handle) -> u8 {
        let Handle::Rgba { pixels, .. } = handle else { panic!("a raw handle") };
        pixels[0]
    }

    #[test]
    fn a_dark_cover_is_left_alone_and_a_bright_one_is_darkened_but_never_below_the_floor() {
        assert_eq!(first_level(&shaded(&solid(40), 4.0, 4.0, 0.0)), 40);
        let bright = first_level(&shaded(&solid(255), 4.0, 4.0, 0.0));
        assert_eq!(bright, (255.0 * DEEPEST).round() as u8, "the brightest is dimmed to the floor");
        let middle = first_level(&shaded(&solid(150), 4.0, 4.0, 0.0));
        assert!(middle < 150 && middle > (150.0 * DEEPEST) as u8, "a half-bright cover is dimmed a little: {middle}");
    }

    #[test]
    fn the_darker_a_cover_is_left_the_less_it_is_dimmed() {
        assert_eq!(calm(0.1), 1.0);
        assert_eq!(calm(CALM), 1.0);
        assert!(calm(0.5) > calm(0.8));
        assert_eq!(calm(1.0), DEEPEST);
    }

    #[test]
    fn a_dimmed_cut_and_a_plain_cut_are_kept_apart() {
        let handle = solid(250);
        assert_ne!(first_level(&fitted(&handle, 4.0, 4.0, 0.0)), first_level(&shaded(&handle, 4.0, 4.0, 0.0)));
    }
}
