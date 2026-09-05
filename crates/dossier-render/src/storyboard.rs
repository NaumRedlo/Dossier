use std::collections::HashMap;

use dossier_beatmap::storyboard::{Drawn, Layer, Sprite, Storyboard};
use tiny_skia::{Pixmap, PixmapPaint, Transform};

use crate::background::decode;
use crate::layout::Layout;

pub struct Show {
    board: Storyboard,

    pictures: HashMap<String, Pixmap>,
}

impl Show {
    #[must_use]
    pub fn load(board: Storyboard, mut bytes_of: impl FnMut(&str) -> Option<Vec<u8>>) -> Self {
        let mut pictures: HashMap<String, Pixmap> = HashMap::new();
        for sprite in &board.sprites {
            for path in wanted(sprite) {
                let name = key(&path);
                if pictures.contains_key(&name) {
                    continue;
                }
                if let Some(picture) = bytes_of(&path).as_deref().and_then(decode) {
                    pictures.insert(name, picture);
                }
            }
        }
        Self { board, pictures }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pictures.is_empty()
    }

    #[must_use]
    pub fn video(&self) -> Option<&dossier_beatmap::storyboard::Video> {
        self.board.video.as_ref()
    }

    pub fn draw(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout, over: bool) {
        for drawn in self.board.at(time_ms) {
            if drawn.layer == Layer::Fail {
                continue;
            }
            if (drawn.layer == Layer::Overlay) != over {
                continue;
            }
            self.paint(pixmap, &drawn, layout);
        }
    }

    fn paint(&self, pixmap: &mut Pixmap, drawn: &Drawn<'_>, layout: &Layout) {
        let name = if drawn.animated {
            key(&frame_path(drawn.path, drawn.frame))
        } else {
            key(drawn.path)
        };
        let Some(picture) = self.pictures.get(&name) else {
            return;
        };
        let unit = layout.scale() as f32;
        let (across, down) = drawn.origin.fractions();
        let (w, h) = (picture.width() as f32, picture.height() as f32);

        let x = layout.width as f32 / 2.0 + (drawn.x - 320.0) * unit;
        let y = layout.height as f32 / 2.0 + (drawn.y - 240.0) * unit;

        let sx = drawn.scale.0 * unit * if drawn.flip.0 { -1.0 } else { 1.0 };
        let sy = drawn.scale.1 * unit * if drawn.flip.1 { -1.0 } else { 1.0 };
        if sx == 0.0 || sy == 0.0 {
            return;
        }
        let transform = Transform::from_translate(x, y)
            .pre_rotate(drawn.rotation.to_degrees())
            .pre_scale(sx, sy)
            .pre_translate(-across * w, -down * h);

        let paint = PixmapPaint {
            opacity: drawn.alpha.clamp(0.0, 1.0),
            quality: tiny_skia::FilterQuality::Bilinear,
            blend_mode: if drawn.additive {
                tiny_skia::BlendMode::Plus
            } else {
                tiny_skia::BlendMode::SourceOver
            },
        };

        if drawn.colour == [255, 255, 255] {
            pixmap.draw_pixmap(0, 0, picture.as_ref(), &paint, transform, None);
        } else if let Some(tinted) = tint(picture, drawn.colour) {
            pixmap.draw_pixmap(0, 0, tinted.as_ref(), &paint, transform, None);
        }
    }
}

fn tint(picture: &Pixmap, colour: [u8; 3]) -> Option<Pixmap> {
    let mut out = picture.clone();
    let share = |v: u8, by: u8| ((u16::from(v) * u16::from(by)) / 255) as u8;
    for pixel in out.pixels_mut() {
        *pixel = tiny_skia::PremultipliedColorU8::from_rgba(
            share(pixel.red(), colour[0]),
            share(pixel.green(), colour[1]),
            share(pixel.blue(), colour[2]),
            pixel.alpha(),
        )?;
    }
    Some(out)
}

fn wanted(sprite: &Sprite) -> Vec<String> {
    match sprite.animation {
        None => vec![sprite.path.clone()],
        Some(animation) => (0..animation.frames)
            .map(|frame| frame_path(&sprite.path, frame))
            .collect(),
    }
}

fn frame_path(path: &str, frame: u32) -> String {
    match path.rfind('.') {
        Some(dot) => format!("{}{}{}", &path[..dot], frame, &path[dot..]),
        None => format!("{path}{frame}"),
    }
}

fn key(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}
