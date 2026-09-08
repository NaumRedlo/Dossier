use tiny_skia::{Color, Pixmap, PremultipliedColorU8};

#[derive(Clone)]
pub struct Font {
    faces: Vec<std::sync::Arc<fontdue::Font>>,
}

impl std::fmt::Debug for Font {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Font(..)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Centre,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub struct Label<'a> {
    pub text: &'a str,
    pub x: f32,

    pub y: f32,
    pub size: f32,
    pub colour: Color,
    pub align: Align,
}

impl Font {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let face = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())?;
        Ok(Self {
            faces: vec![std::sync::Arc::new(face)],
        })
    }

    pub fn behind(mut self, other: &Self) -> Self {
        self.faces.extend(other.faces.iter().cloned());
        self
    }

    pub fn faces(&self) -> usize {
        self.faces.len()
    }

    fn face_for(&self, ch: char) -> &fontdue::Font {
        self.faces
            .iter()
            .find(|face| face.lookup_glyph_index(ch) != 0)
            .unwrap_or(&self.faces[0])
    }

    pub fn width(&self, text: &str, size: f32) -> f32 {
        text.chars()
            .map(|c| self.face_for(c).metrics(c, size).advance_width)
            .sum()
    }

    pub fn draw(&self, pixmap: &mut Pixmap, label: Label<'_>) {
        let Label {
            text,
            x,
            y,
            size,
            colour,
            align,
        } = label;

        let mut pen = match align {
            Align::Left => x,
            Align::Centre => x - self.width(text, size) / 2.0,
            Align::Right => x - self.width(text, size),
        };

        for ch in text.chars() {
            let (metrics, coverage) = self.face_for(ch).rasterize(ch, size);

            let left = (pen + metrics.xmin as f32).round() as i32;
            let top = (y - (metrics.height as i32 + metrics.ymin) as f32).round() as i32;
            blit(pixmap, &coverage, metrics.width, left, top, colour);
            pen += metrics.advance_width;
        }
    }

    pub fn digit_height(&self, size: f32) -> f32 {
        self.face_for('0').metrics('0', size).height as f32
    }
}

fn blit(pixmap: &mut Pixmap, coverage: &[u8], width: usize, left: i32, top: i32, colour: Color) {
    if width == 0 || coverage.is_empty() {
        return;
    }
    let (frame_w, frame_h) = (pixmap.width() as i32, pixmap.height() as i32);
    let height = coverage.len() / width;
    let (sr, sg, sb, sa) = (colour.red(), colour.green(), colour.blue(), colour.alpha());
    let pixels = pixmap.pixels_mut();

    for row in 0..height {
        let y = top + row as i32;
        if y < 0 || y >= frame_h {
            continue;
        }
        for column in 0..width {
            let x = left + column as i32;
            if x < 0 || x >= frame_w {
                continue;
            }
            let alpha = f32::from(coverage[row * width + column]) / 255.0 * sa;
            if alpha <= 0.0 {
                continue;
            }
            let index = (y * frame_w + x) as usize;
            let dst = pixels[index];

            let keep = 1.0 - alpha;
            let mix = |src: f32, dst: u8| {
                ((src * alpha + f32::from(dst) / 255.0 * keep) * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8
            };
            let (r, g, b) = (
                mix(sr, dst.red()),
                mix(sg, dst.green()),
                mix(sb, dst.blue()),
            );
            let a = ((alpha + f32::from(dst.alpha()) / 255.0 * keep) * 255.0)
                .round()
                .clamp(0.0, 255.0) as u8;

            pixels[index] =
                PremultipliedColorU8::from_rgba(r.min(a), g.min(a), b.min(a), a).unwrap_or(dst);
        }
    }
}
