use std::path::{Path, PathBuf};

use resvg::usvg;
use tiny_skia::{Pixmap, PixmapPaint, PremultipliedColorU8, Transform};

const WIDE: u32 = 120;
const HIGH: u32 = 84;
const INK: f32 = 66.0;
const OVERSAMPLE: f32 = 8.0;
const FAINTEST: u8 = 8;

fn drawn(from: &Path) -> Result<Pixmap, String> {
    let text = std::fs::read_to_string(from).map_err(|why| format!("{}: {why}", from.display()))?;
    let tree = usvg::Tree::from_str(&text, &usvg::Options::default())
        .map_err(|why| format!("{}: {why}", from.display()))?;
    let size = tree.size();
    let scale = INK * OVERSAMPLE / size.width().max(size.height());
    let mut out = Pixmap::new(
        (size.width() * scale).ceil() as u32 + 2,
        (size.height() * scale).ceil() as u32 + 2,
    )
    .ok_or("the page came out empty")?;
    resvg::render(
        &tree,
        Transform::from_translate(1.0, 1.0).pre_scale(scale, scale),
        &mut out.as_mut(),
    );
    Ok(out)
}

fn whiten(art: &mut Pixmap) {
    for pixel in art.pixels_mut() {
        let alpha = pixel.alpha();
        *pixel = if alpha < FAINTEST {
            PremultipliedColorU8::from_rgba(0, 0, 0, 0)
        } else {
            PremultipliedColorU8::from_rgba(alpha, alpha, alpha, alpha)
        }
        .unwrap_or(*pixel);
    }
}

fn bounds(art: &Pixmap) -> Option<(u32, u32, u32, u32)> {
    let (wide, high) = (art.width(), art.height());
    let (mut left, mut top, mut right, mut bottom) = (wide, high, 0, 0);
    for y in 0..high {
        for x in 0..wide {
            if art.pixel(x, y).map_or(0, |one| one.alpha()) == 0 {
                continue;
            }
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
        }
    }
    (left <= right).then(|| (left, top, right + 1 - left, bottom + 1 - top))
}

fn centred(art: &Pixmap) -> Result<Pixmap, String> {
    let (left, top, wide, high) = bounds(art).ok_or("nothing was drawn")?;
    let mut cut = Pixmap::new(wide, high).ok_or("the ink came out empty")?;
    cut.draw_pixmap(
        0,
        0,
        art.as_ref(),
        &PixmapPaint::default(),
        Transform::from_translate(-(left as f32), -(top as f32)),
        None,
    );
    let scale = INK / wide.max(high) as f32;
    let mut out = Pixmap::new(WIDE, HIGH).ok_or("the plate came out empty")?;
    out.draw_pixmap(
        0,
        0,
        cut.as_ref(),
        &PixmapPaint {
            quality: tiny_skia::FilterQuality::Bicubic,
            ..Default::default()
        },
        Transform::from_translate(
            (WIDE as f32 - wide as f32 * scale) / 2.0,
            (HIGH as f32 - high as f32 * scale) / 2.0,
        )
        .pre_scale(scale, scale),
        None,
    );
    Ok(out)
}

fn sources(from: &Path) -> Result<Vec<PathBuf>, String> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(from)
        .map_err(|why| format!("{}: {why}", from.display()))?
        .filter_map(|one| one.ok())
        .map(|one| one.path())
        .filter(|one| one.extension().is_some_and(|kind| kind == "svg"))
        .collect();
    found.sort();
    Ok(found)
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let from = PathBuf::from(args.next().unwrap_or_else(|| "assets/mods/svg".to_owned()));
    let into = PathBuf::from(args.next().unwrap_or_else(|| "assets/mods".to_owned()));

    for one in sources(&from)? {
        let mut art = drawn(&one)?;
        whiten(&mut art);
        let plate = centred(&art)?;
        let named = into.join(one.file_stem().ok_or("a file with no name")?).with_extension("png");
        std::fs::write(&named, plate.encode_png().map_err(|why| why.to_string())?)
            .map_err(|why| format!("{}: {why}", named.display()))?;
        println!("{}", named.display());
    }
    Ok(())
}
