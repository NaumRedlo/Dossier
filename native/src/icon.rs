use std::io::Cursor;
use std::path::Path;

use image::{imageops, GrayImage, ImageFormat, Luma, Rgba, RgbaImage};

use crate::theme::ACCENT;
use crate::ui::ACCENT_DEEP;

const MASTER: u32 = 1024;
const ROUNDNESS: f32 = 5.0;
const LETTER: f32 = 0.6;
const ICO: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];
const ICNS: [(&[u8; 4], u32); 11] = [
    (b"icp4", 16),
    (b"icp5", 32),
    (b"icp6", 64),
    (b"ic07", 128),
    (b"ic08", 256),
    (b"ic09", 512),
    (b"ic10", 1024),
    (b"ic11", 32),
    (b"ic12", 64),
    (b"ic13", 256),
    (b"ic14", 512),
];
const GROUND: [(f32, [f32; 3]); 5] = [
    (0.0, [0x4a as f32, 0x14 as f32, 0x1b as f32]),
    (0.30, [0x2e as f32, 0x0b as f32, 0x12 as f32]),
    (0.55, [0x1b as f32, 0x08 as f32, 0x0d as f32]),
    (0.78, [0x10 as f32, 0x06 as f32, 0x09 as f32]),
    (1.0, [0x09 as f32, 0x04 as f32, 0x05 as f32]),
];

fn ground(t: f32) -> [f32; 3] {
    for pair in GROUND.windows(2) {
        let (a, ca) = pair[0];
        let (b, cb) = pair[1];
        if t >= a && t <= b {
            let k = if b > a { (t - a) / (b - a) } else { 0.0 };
            return [ca[0] + (cb[0] - ca[0]) * k, ca[1] + (cb[1] - ca[1]) * k, ca[2] + (cb[2] - ca[2]) * k];
        }
    }
    GROUND[4].1
}

fn shape(u: f32, v: f32) -> f32 {
    u.abs().powf(ROUNDNESS) + v.abs().powf(ROUNDNESS)
}

fn coverage(x: u32, y: u32, centre: f32, half: f32) -> f32 {
    let at = |px: f32, py: f32| shape((px - centre) / half, (py - centre) / half);
    let middle = at(x as f32 + 0.5, y as f32 + 0.5);
    if middle < 0.9 {
        return 1.0;
    }
    if middle > 1.12 {
        return 0.0;
    }
    let mut inside = 0;
    for sy in 0..4 {
        for sx in 0..4 {
            if at(x as f32 + (sx as f32 + 0.5) / 4.0, y as f32 + (sy as f32 + 0.5) / 4.0) <= 1.0 {
                inside += 1;
            }
        }
    }
    inside as f32 / 16.0
}

fn letter_alpha(nx: f32, ny: f32) -> f32 {
    let letter = crate::ui::letter();
    let side = letter.side as usize;
    let mask = letter.mask();
    let fx = nx * side as f32 - 0.5;
    let fy = ny * side as f32 - 0.5;
    if fx < -1.0 || fy < -1.0 || fx > side as f32 || fy > side as f32 {
        return 0.0;
    }
    let pick = |x: isize, y: isize| -> f32 {
        if x < 0 || y < 0 || x as usize >= side || y as usize >= side {
            0.0
        } else {
            mask[y as usize * side + x as usize] as f32 / 255.0
        }
    };
    let (x0, y0) = (fx.floor() as isize, fy.floor() as isize);
    let (kx, ky) = (fx - fx.floor(), fy - fy.floor());
    let top = pick(x0, y0) * (1.0 - kx) + pick(x0 + 1, y0) * kx;
    let bottom = pick(x0, y0 + 1) * (1.0 - kx) + pick(x0 + 1, y0 + 1) * kx;
    top * (1.0 - ky) + bottom * ky
}

pub fn master(margin: f32) -> RgbaImage {
    let side = MASTER as f32;
    let tile = side * (1.0 - 2.0 * margin);
    let (left, centre, half) = (side * margin, side / 2.0, tile / 2.0);
    let drawn = tile * LETTER;
    let letter_left = centre - drawn / 2.0;
    let letter_top = centre - drawn / 2.0;
    let small = 256u32;
    let mut glow = GrayImage::new(small, small);
    for y in 0..small {
        for x in 0..small {
            let px = (x as f32 + 0.5) * side / small as f32;
            let py = (y as f32 + 0.5) * side / small as f32;
            let a = letter_alpha((px - letter_left) / drawn, (py - letter_top) / drawn);
            glow.put_pixel(x, y, Luma([(a * 255.0) as u8]));
        }
    }
    let glow = imageops::resize(&imageops::blur(&glow, 9.0), MASTER, MASTER, imageops::FilterType::Triangle);
    let mut out = RgbaImage::new(MASTER, MASTER);
    for y in 0..MASTER {
        for x in 0..MASTER {
            let cover = coverage(x, y, centre, half);
            if cover <= 0.0 {
                continue;
            }
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let nx = (px - left) / tile;
            let ny = (py - left) / tile;
            let dx = (nx - 0.5) / 1.1;
            let dy = (ny + 0.12) / 1.02;
            let mut colour = ground((dx * dx + dy * dy).sqrt().min(1.0));
            let edge = shape((px - centre) / half, (py - centre) / half);
            if edge > 0.93 {
                let rim = ((edge - 0.93) / 0.06).clamp(0.0, 1.0) * 0.16 * (1.0 - ny).clamp(0.0, 1.0).powf(1.4);
                for c in &mut colour {
                    *c += (255.0 - *c) * rim;
                }
            }
            let halo = glow.get_pixel(x, y)[0] as f32 / 255.0 * 0.55;
            let red = [ACCENT.r * 255.0, ACCENT.g * 255.0, ACCENT.b * 255.0];
            for (c, r) in colour.iter_mut().zip(red) {
                *c += (r - *c) * halo * 0.6;
            }
            let ink = letter_alpha((px - letter_left) / drawn, (py - letter_top) / drawn);
            if ink > 0.0 {
                let k = ((py - letter_top) / drawn).clamp(0.0, 1.0);
                let tint = [
                    (ACCENT.r + (ACCENT_DEEP.r - ACCENT.r) * k) * 255.0,
                    (ACCENT.g + (ACCENT_DEEP.g - ACCENT.g) * k) * 255.0,
                    (ACCENT.b + (ACCENT_DEEP.b - ACCENT.b) * k) * 255.0,
                ];
                for (c, t) in colour.iter_mut().zip(tint) {
                    *c += (t - *c) * ink;
                }
            }
            out.put_pixel(x, y, Rgba([colour[0].round() as u8, colour[1].round() as u8, colour[2].round() as u8, (cover * 255.0).round() as u8]));
        }
    }
    out
}

fn sized(master: &RgbaImage, side: u32) -> RgbaImage {
    if side == master.width() {
        master.clone()
    } else {
        imageops::resize(master, side, side, imageops::FilterType::Lanczos3)
    }
}

fn png(picture: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut out = Cursor::new(Vec::new());
    picture.write_to(&mut out, ImageFormat::Png).map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}

pub fn ico(master: &RgbaImage) -> Result<Vec<u8>, String> {
    let pictures: Vec<(u32, Vec<u8>)> = ICO.iter().map(|&side| png(&sized(master, side)).map(|bytes| (side, bytes))).collect::<Result<_, _>>()?;
    let mut out = Vec::new();
    out.extend(0u16.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend((pictures.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * pictures.len() as u32;
    for (side, bytes) in &pictures {
        let edge = if *side >= 256 { 0 } else { *side as u8 };
        out.extend([edge, edge, 0, 0]);
        out.extend(1u16.to_le_bytes());
        out.extend(32u16.to_le_bytes());
        out.extend((bytes.len() as u32).to_le_bytes());
        out.extend(offset.to_le_bytes());
        offset += bytes.len() as u32;
    }
    for (_, bytes) in &pictures {
        out.extend(bytes);
    }
    Ok(out)
}

pub fn icns(master: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    for (kind, side) in ICNS {
        let bytes = png(&sized(master, side))?;
        body.extend(kind);
        body.extend((8 + bytes.len() as u32).to_be_bytes());
        body.extend(bytes);
    }
    let mut out = Vec::with_capacity(8 + body.len());
    out.extend(b"icns");
    out.extend((8 + body.len() as u32).to_be_bytes());
    out.extend(body);
    Ok(out)
}

pub fn write(dir: &Path) -> Result<usize, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let rounded = master(100.0 / 1024.0);
    let flat = master(0.03);
    let files: [(&str, Vec<u8>); 4] = [
        ("dossier.icns", icns(&rounded)?),
        ("dossier.ico", ico(&flat)?),
        ("dossier-256.png", png(&sized(&flat, 256))?),
        ("dossier-512.png", png(&sized(&flat, 512))?),
    ];
    for (name, bytes) in &files {
        std::fs::write(dir.join(name), bytes).map_err(|e| format!("{name}: {e}"))?;
    }
    Ok(files.len())
}

pub fn window() -> Option<iced::window::Icon> {
    let picture = image::load_from_memory(include_bytes!("../assets/icon/dossier-256.png")).ok()?.to_rgba8();
    let (width, height) = picture.dimensions();
    iced::window::icon::from_rgba(picture.into_raw(), width, height).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_wears_the_mark() {
        assert!(window().is_some());
    }

    #[test]
    fn the_ico_holds_every_size_as_a_png() {
        let bytes = include_bytes!("../assets/icon/dossier.ico");
        let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
        assert_eq!(&bytes[..4], &[0, 0, 1, 0]);
        assert_eq!(count, ICO.len());
        for at in 0..count {
            let entry = &bytes[6 + 16 * at..22 + 16 * at];
            let size = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as usize;
            let offset = u32::from_le_bytes(entry[12..16].try_into().unwrap()) as usize;
            assert!(offset + size <= bytes.len());
            assert_eq!(&bytes[offset..offset + 4], b"\x89PNG");
        }
    }

    #[test]
    fn the_icns_is_as_long_as_it_says() {
        let bytes = include_bytes!("../assets/icon/dossier.icns");
        assert_eq!(&bytes[..4], b"icns");
        assert_eq!(u32::from_be_bytes(bytes[4..8].try_into().unwrap()) as usize, bytes.len());
        let mut at = 8;
        let mut kinds = Vec::new();
        while at < bytes.len() {
            kinds.push(bytes[at..at + 4].to_vec());
            at += u32::from_be_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        }
        assert_eq!(at, bytes.len());
        assert!(kinds.contains(&b"ic10".to_vec()), "the 1024 picture is inside");
    }
}
