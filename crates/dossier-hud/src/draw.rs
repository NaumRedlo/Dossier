use std::sync::OnceLock;

use tiny_skia::{
    FillRule, FilterQuality, Paint, PathBuilder, Pixmap, PixmapPaint, Stroke, Transform,
};

pub type Rgba = [f32; 4];

pub const fn rgb(hex: u32, alpha: f32) -> Rgba {
    [
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
        alpha,
    ]
}

pub fn mix(from: Rgba, to: Rgba, share: f32) -> Rgba {
    let share = share.clamp(0.0, 1.0);
    [
        from[0] + (to[0] - from[0]) * share,
        from[1] + (to[1] - from[1]) * share,
        from[2] + (to[2] - from[2]) * share,
        from[3] + (to[3] - from[3]) * share,
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Face {
    Sans,
    Semi,
    Mono,
    MonoBold,
}

struct Faces {
    sans: fontdue::Font,
    semi: fontdue::Font,
    mono: fontdue::Font,
    mono_bold: fontdue::Font,
}

fn faces() -> &'static Faces {
    static HELD: OnceLock<Faces> = OnceLock::new();
    HELD.get_or_init(|| {
        let load = |bytes: &[u8]| {
            fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
                .expect("a bundled font is readable")
        };
        Faces {
            sans: load(include_bytes!(
                "../../../assets/fonts/Commissioner-Regular.ttf"
            )),
            semi: load(include_bytes!(
                "../../../assets/fonts/Commissioner-SemiBold.ttf"
            )),
            mono: load(include_bytes!(
                "../../../assets/fonts/IBMPlexMono-Regular.ttf"
            )),
            mono_bold: load(include_bytes!("../../../assets/fonts/IBMPlexMono-Bold.ttf")),
        }
    })
}

fn font(face: Face) -> &'static fontdue::Font {
    let held = faces();
    match face {
        Face::Sans => &held.sans,
        Face::Semi => &held.semi,
        Face::Mono => &held.mono,
        Face::MonoBold => &held.mono_bold,
    }
}

pub fn text_width(text: &str, face: Face, size: f32) -> f32 {
    let font = font(face);
    text.chars()
        .map(|sign| font.metrics(sign, size).advance_width)
        .sum()
}

pub fn line_height(size: f32) -> f32 {
    (size * 1.35).round()
}

pub fn fitted(text: &str, face: Face, size: f32, most: f32) -> String {
    if text_width(text, face, size) <= most {
        return text.to_owned();
    }
    let mut kept = String::new();
    for sign in text.chars() {
        let mut tried = kept.clone();
        tried.push(sign);
        tried.push('…');
        if text_width(&tried, face, size) > most {
            break;
        }
        kept.push(sign);
    }
    let mut cut = kept.trim_end().to_owned();
    cut.push('…');
    cut
}

fn letter() -> &'static (u32, u32, Vec<u8>) {
    static HELD: OnceLock<(u32, u32, Vec<u8>)> = OnceLock::new();
    HELD.get_or_init(|| {
        let decoder = png::Decoder::new(std::io::Cursor::new(
            &include_bytes!("../../../native/assets/letter-mask.png")[..],
        ));
        let mut reader = decoder.read_info().expect("the letter mask is a PNG");
        let mut pixels = vec![0; reader.output_buffer_size()];
        let info = reader
            .next_frame(&mut pixels)
            .expect("the letter mask decodes");
        let step = info.color_type.samples();
        let grey: Vec<u8> = pixels[..info.buffer_size()]
            .chunks_exact(step)
            .map(|pixel| pixel[0])
            .collect();
        let (wide, high) = (info.width as usize, info.height as usize);
        let lit = |x: usize, y: usize| grey[y * wide + x] > 8;
        let left = (0..wide)
            .find(|x| (0..high).any(|y| lit(*x, y)))
            .unwrap_or(0);
        let right = (0..wide)
            .rfind(|x| (0..high).any(|y| lit(*x, y)))
            .unwrap_or(wide - 1);
        let top = (0..high)
            .find(|y| (0..wide).any(|x| lit(x, *y)))
            .unwrap_or(0);
        let bottom = (0..high)
            .rfind(|y| (0..wide).any(|x| lit(x, *y)))
            .unwrap_or(high - 1);
        let (cut_wide, cut_high) = (right - left + 1, bottom - top + 1);
        let mut cut = Vec::with_capacity(cut_wide * cut_high);
        for y in top..=bottom {
            cut.extend_from_slice(&grey[y * wide + left..y * wide + right + 1]);
        }
        (cut_wide as u32, cut_high as u32, cut)
    })
}

pub struct Canvas {
    pub pixmap: Pixmap,
    pub scale: f32,
    alpha: f32,
}

fn paint(colour: Rgba, alpha: f32) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(
        tiny_skia::Color::from_rgba(
            colour[0].clamp(0.0, 1.0),
            colour[1].clamp(0.0, 1.0),
            colour[2].clamp(0.0, 1.0),
            (colour[3] * alpha).clamp(0.0, 1.0),
        )
        .unwrap_or(tiny_skia::Color::TRANSPARENT),
    );
    paint.anti_alias = true;
    paint
}

fn rounded(x: f32, y: f32, wide: f32, high: f32, radius: f32) -> Option<tiny_skia::Path> {
    if wide <= 0.0 || high <= 0.0 {
        return None;
    }
    let r = radius.min(wide / 2.0).min(high / 2.0).max(0.0);
    let k = r * 0.552_284_8;
    let mut path = PathBuilder::new();
    path.move_to(x + r, y);
    path.line_to(x + wide - r, y);
    path.cubic_to(x + wide - r + k, y, x + wide, y + r - k, x + wide, y + r);
    path.line_to(x + wide, y + high - r);
    path.cubic_to(
        x + wide,
        y + high - r + k,
        x + wide - r + k,
        y + high,
        x + wide - r,
        y + high,
    );
    path.line_to(x + r, y + high);
    path.cubic_to(x + r - k, y + high, x, y + high - r + k, x, y + high - r);
    path.line_to(x, y + r);
    path.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    path.close();
    path.finish()
}

impl Canvas {
    pub fn new(wide: f32, high: f32, scale: f32) -> Option<Canvas> {
        let pixmap = Pixmap::new(
            (wide * scale).ceil().max(1.0) as u32,
            (high * scale).ceil().max(1.0) as u32,
        )?;
        Some(Canvas {
            pixmap,
            scale,
            alpha: 1.0,
        })
    }

    pub fn plate(
        &mut self,
        x: f32,
        y: f32,
        wide: f32,
        high: f32,
        radius: f32,
        fill: Rgba,
        edge: Option<Rgba>,
    ) {
        let s = self.scale;
        if let Some(path) = rounded(x * s, y * s, wide * s, high * s, radius * s) {
            self.pixmap.fill_path(
                &path,
                &paint(fill, self.alpha),
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
        if let Some(edge) = edge {
            let line = s.max(1.0);
            if let Some(path) = rounded(
                x * s + line / 2.0,
                y * s + line / 2.0,
                wide * s - line,
                high * s - line,
                (radius * s - line / 2.0).max(0.0),
            ) {
                let stroke = Stroke {
                    width: line,
                    ..Stroke::default()
                };
                self.pixmap.stroke_path(
                    &path,
                    &paint(edge, self.alpha),
                    &stroke,
                    Transform::identity(),
                    None,
                );
            }
        }
    }

    pub fn disc(&mut self, x: f32, y: f32, radius: f32, fill: Rgba) {
        self.plate(
            x - radius,
            y - radius,
            radius * 2.0,
            radius * 2.0,
            radius,
            fill,
            None,
        );
    }

    pub fn line(&mut self, points: &[(f32, f32)], wide: f32, colour: Rgba) {
        let s = self.scale;
        let mut path = PathBuilder::new();
        for (at, (x, y)) in points.iter().enumerate() {
            if at == 0 {
                path.move_to(x * s, y * s);
            } else {
                path.line_to(x * s, y * s);
            }
        }
        if let Some(path) = path.finish() {
            let stroke = Stroke {
                width: wide * s,
                line_cap: tiny_skia::LineCap::Round,
                line_join: tiny_skia::LineJoin::Round,
                ..Stroke::default()
            };
            self.pixmap.stroke_path(
                &path,
                &paint(colour, self.alpha),
                &stroke,
                Transform::identity(),
                None,
            );
        }
    }

    pub fn shape(&mut self, points: &[(f32, f32)], colour: Rgba) {
        let s = self.scale;
        let mut path = PathBuilder::new();
        for (at, (x, y)) in points.iter().enumerate() {
            if at == 0 {
                path.move_to(x * s, y * s);
            } else {
                path.line_to(x * s, y * s);
            }
        }
        path.close();
        if let Some(path) = path.finish() {
            self.pixmap.fill_path(
                &path,
                &paint(colour, self.alpha),
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
    }

    fn blend(&mut self, x: i32, y: i32, cover: f32, colour: Rgba) {
        let (wide, high) = (self.pixmap.width() as i32, self.pixmap.height() as i32);
        if x < 0 || y < 0 || x >= wide || y >= high || cover <= 0.0 {
            return;
        }
        let a = (colour[3] * self.alpha * cover).clamp(0.0, 1.0);
        let at = ((y * wide + x) * 4) as usize;
        let data = self.pixmap.data_mut();
        for channel in 0..3 {
            let was = f32::from(data[at + channel]) / 255.0;
            data[at + channel] = ((colour[channel] * a + was * (1.0 - a)) * 255.0).round() as u8;
        }
        let was = f32::from(data[at + 3]) / 255.0;
        data[at + 3] = ((a + was * (1.0 - a)) * 255.0).round() as u8;
    }

    pub fn text(&mut self, x: f32, y: f32, text: &str, face: Face, size: f32, colour: Rgba) {
        let font = font(face);
        let px = size * self.scale;
        let high = line_height(size) * self.scale;
        let (ascent, descent) = font
            .horizontal_line_metrics(px)
            .map_or((px * 0.8, -px * 0.2), |line| (line.ascent, line.descent));
        let baseline = (y * self.scale + (high - (ascent - descent)) / 2.0 + ascent).round();
        let mut pen = x * self.scale;
        for sign in text.chars() {
            let (metrics, cover) = font.rasterize(sign, px);
            let left = (pen + metrics.xmin as f32).round() as i32;
            let top = baseline as i32 - metrics.ymin - metrics.height as i32;
            for row in 0..metrics.height {
                for column in 0..metrics.width {
                    let seen = cover[row * metrics.width + column];
                    if seen > 0 {
                        self.blend(
                            left + column as i32,
                            top + row as i32,
                            f32::from(seen) / 255.0,
                            colour,
                        );
                    }
                }
            }
            pen += metrics.advance_width;
        }
    }

    pub fn logo(&mut self, x: f32, y: f32, side: f32, colour: Rgba) {
        let (wide, high, mask) = letter();
        let box_px = side * self.scale;
        let fit = box_px / (*wide.max(high)) as f32;
        let (drawn_wide, drawn_high) = (*wide as f32 * fit, *high as f32 * fit);
        let left = x * self.scale + (box_px - drawn_wide) / 2.0;
        let top = y * self.scale + (box_px - drawn_high) / 2.0;
        let (first_x, first_y) = (left.floor() as i32, top.floor() as i32);
        let (last_x, last_y) = (
            (left + drawn_wide).ceil() as i32,
            (top + drawn_high).ceil() as i32,
        );
        let taps = (1.0 / fit).ceil().clamp(1.0, 8.0) as usize;
        for py in first_y..last_y {
            for px in first_x..last_x {
                let mut sum = 0.0;
                for sy in 0..taps {
                    for sx in 0..taps {
                        let u = (px as f32 + (sx as f32 + 0.5) / taps as f32 - left) / fit;
                        let v = (py as f32 + (sy as f32 + 0.5) / taps as f32 - top) / fit;
                        if u >= 0.0 && v >= 0.0 && (u as u32) < *wide && (v as u32) < *high {
                            sum += f32::from(mask[(v as u32 * wide + u as u32) as usize]) / 255.0;
                        }
                    }
                }
                self.blend(px, py, sum / (taps * taps) as f32, colour);
            }
        }
    }

    pub fn layer(
        &mut self,
        x: f32,
        y: f32,
        wide: f32,
        high: f32,
        alpha: f32,
        draw: impl FnOnce(&mut Canvas),
    ) {
        if alpha <= 0.003 {
            return;
        }
        let Some(mut inner) = Canvas::new(wide, high, self.scale) else {
            return;
        };
        draw(&mut inner);
        self.pixmap.draw_pixmap(
            (x * self.scale).round() as i32,
            (y * self.scale).round() as i32,
            inner.pixmap.as_ref(),
            &PixmapPaint {
                opacity: (alpha * self.alpha).clamp(0.0, 1.0),
                blend_mode: tiny_skia::BlendMode::SourceOver,
                quality: FilterQuality::Nearest,
            },
            Transform::identity(),
            None,
        );
    }
}
