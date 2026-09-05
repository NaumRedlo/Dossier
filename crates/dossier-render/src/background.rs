use tiny_skia::{Color, Pixmap, PixmapPaint, Transform};

pub(crate) fn decode(bytes: &[u8]) -> Option<Pixmap> {
    if let Ok(pixmap) = Pixmap::decode_png(bytes) {
        return Some(pixmap);
    }
    let mut decoder = jpeg_decoder::Decoder::new(bytes);
    let pixels = decoder.decode().ok()?;
    let info = decoder.info()?;
    let (width, height) = (u32::from(info.width), u32::from(info.height));
    let mut pixmap = Pixmap::new(width, height)?;
    let out = pixmap.pixels_mut();
    match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => {
            for (i, chunk) in pixels.chunks_exact(3).enumerate() {
                out[i] =
                    tiny_skia::PremultipliedColorU8::from_rgba(chunk[0], chunk[1], chunk[2], 255)?;
            }
        }
        jpeg_decoder::PixelFormat::L8 => {
            for (i, grey) in pixels.iter().enumerate() {
                out[i] = tiny_skia::PremultipliedColorU8::from_rgba(*grey, *grey, *grey, 255)?;
            }
        }

        _ => return None,
    }
    Some(pixmap)
}

pub fn prepare(
    bytes: &[u8],
    width: u32,
    height: u32,
    dim: f32,
    blur: f32,
    towards: Color,
) -> Option<Pixmap> {
    let source = decode(bytes)?;
    let mut canvas = Pixmap::new(width, height)?;

    let scale = (width as f32 / source.width() as f32).max(height as f32 / source.height() as f32);
    let transform = Transform::from_translate(
        (width as f32 - source.width() as f32 * scale) / 2.0,
        (height as f32 - source.height() as f32 * scale) / 2.0,
    )
    .pre_scale(scale, scale);
    canvas.draw_pixmap(
        0,
        0,
        source.as_ref(),
        &PixmapPaint {
            quality: tiny_skia::FilterQuality::Bilinear,
            ..Default::default()
        },
        transform,
        None,
    );

    let radius = (blur * height as f32).round() as u32;
    if radius > 0 {
        blur_box(&mut canvas, radius);
    }
    if dim > 0.0 {
        wash(&mut canvas, towards, dim.min(1.0));
    }
    Some(canvas)
}

fn blur_box(pixmap: &mut Pixmap, radius: u32) {
    for _ in 0..2 {
        blur_pass(pixmap, radius, true);
        blur_pass(pixmap, radius, false);
    }
}

fn blur_pass(pixmap: &mut Pixmap, radius: u32, horizontal: bool) {
    let (width, height) = (pixmap.width() as i64, pixmap.height() as i64);
    let radius = radius as i64;
    let source: Vec<[u8; 4]> = pixmap
        .pixels()
        .iter()
        .map(|p| [p.red(), p.green(), p.blue(), p.alpha()])
        .collect();
    let (major, minor) = if horizontal {
        (width, height)
    } else {
        (height, width)
    };
    let at = |along: i64, across: i64| -> usize {
        let (x, y) = if horizontal {
            (along, across)
        } else {
            (across, along)
        };
        (y * width + x) as usize
    };

    let out = pixmap.pixels_mut();
    for across in 0..minor {
        let mut sums = [0u32; 4];
        let mut count = 0u32;
        for along in 0..(radius + 1).min(major) {
            let p = source[at(along, across)];
            for c in 0..4 {
                sums[c] += u32::from(p[c]);
            }
            count += 1;
        }
        for along in 0..major {
            let mut pixel = [0u8; 4];
            for c in 0..4 {
                pixel[c] = (sums[c] / count.max(1)) as u8;
            }
            if let Some(colour) =
                tiny_skia::PremultipliedColorU8::from_rgba(pixel[0], pixel[1], pixel[2], pixel[3])
            {
                out[at(along, across)] = colour;
            }

            let leaving = along - radius;
            if leaving >= 0 {
                let p = source[at(leaving, across)];
                for c in 0..4 {
                    sums[c] -= u32::from(p[c]);
                }
                count -= 1;
            }
            let arriving = along + radius + 1;
            if arriving < major {
                let p = source[at(arriving, across)];
                for c in 0..4 {
                    sums[c] += u32::from(p[c]);
                }
                count += 1;
            }
        }
    }
}

fn wash(pixmap: &mut Pixmap, towards: Color, amount: f32) {
    let keep = 1.0 - amount;
    let (r, g, b) = (
        towards.red() * 255.0 * amount,
        towards.green() * 255.0 * amount,
        towards.blue() * 255.0 * amount,
    );
    for pixel in pixmap.pixels_mut() {
        let Some(washed) = tiny_skia::PremultipliedColorU8::from_rgba(
            (f32::from(pixel.red()) * keep + r) as u8,
            (f32::from(pixel.green()) * keep + g) as u8,
            (f32::from(pixel.blue()) * keep + b) as u8,
            pixel.alpha(),
        ) else {
            continue;
        };
        *pixel = washed;
    }
}
