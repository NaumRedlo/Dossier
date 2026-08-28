//! Drawing the storyboard.
//!
//! The reading and the arithmetic are `dossier_beatmap::storyboard`; this is
//! the half that has pictures in it. A [`Show`] is a parsed storyboard with
//! every picture it names decoded once, and drawing a frame is asking it what
//! is out and copying those.
//!
//! ## The space it is stated in
//!
//! A storyboard is authored on 640×480, and the playfield is the 512×384 in
//! the middle of that — so one storyboard unit is exactly one osu!pixel and
//! the layout's own scale is the whole conversion. 480 of them make the frame's
//! height, which is what puts a full-screen background sprite edge to edge.
//!
//! Widescreen falls out of it rather than being a case: the middle of the
//! storyboard is the middle of the frame, so a wider frame simply shows more of
//! the space either side, which is what the game does and what mappers who
//! draw past x=640 are drawing for.
//!
//! ## What is slow here
//!
//! A tinted sprite is built rather than blended — tiny-skia can carry an
//! opacity through a blit but not a colour, so a sprite under a `C` command is
//! multiplied into a scratch copy first. White sprites, which is nearly all of
//! them, skip that entirely.

use std::collections::HashMap;

use dossier_beatmap::storyboard::{Drawn, Layer, Sprite, Storyboard};
use tiny_skia::{Pixmap, PixmapPaint, Transform};

use crate::background::decode;
use crate::layout::Layout;

/// A storyboard with its pictures.
pub struct Show {
    board: Storyboard,
    /// Keyed by the path as the file wrote it, put through `key`.
    pictures: HashMap<String, Pixmap>,
}

impl Show {
    /// Decode every picture the storyboard names.
    ///
    /// `bytes_of` is handed a path exactly as the storyboard wrote it —
    /// backslashes and all — and answers `None` for anything the map does not
    /// have. A missing picture costs that sprite and nothing else: storyboards
    /// routinely name files that were never shipped.
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

    /// The map's video, when it named one.
    #[must_use]
    pub fn video(&self) -> Option<&dossier_beatmap::storyboard::Video> {
        self.board.video.as_ref()
    }

    /// Draw everything that is out at `time_ms` and belongs `over` the play, or
    /// under it.
    ///
    /// Two passes rather than one because that is where they go: the storyboard
    /// is mostly scenery behind the notes, and its `Overlay` layer is the part
    /// a mapper put in front of them on purpose.
    pub fn draw(&self, pixmap: &mut Pixmap, time_ms: f64, layout: &Layout, over: bool) {
        for drawn in self.board.at(time_ms) {
            // `Fail` is the branch a replay never takes. `Pass` is the one it
            // always does, so it is drawn with the rest.
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

        // The middle of the storyboard is the middle of the frame; everything
        // else follows from one unit being one osu!pixel.
        let x = layout.width as f32 / 2.0 + (drawn.x - 320.0) * unit;
        let y = layout.height as f32 / 2.0 + (drawn.y - 240.0) * unit;

        // A mirrored sprite is a negative scale, and because the step back to
        // the origin happens after it, it turns over about its origin — which
        // is where the game turns it.
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

/// Multiply a picture by a colour, keeping it premultiplied.
fn tint(picture: &Pixmap, colour: [u8; 3]) -> Option<Pixmap> {
    let mut out = picture.clone();
    let share = |v: u8, by: u8| ((u16::from(v) * u16::from(by)) / 255) as u8;
    for pixel in out.pixels_mut() {
        // Premultiplied in, premultiplied out: scaling the three colour
        // channels by a factor at most one cannot take any of them past the
        // alpha they are already multiplied by.
        *pixel = tiny_skia::PremultipliedColorU8::from_rgba(
            share(pixel.red(), colour[0]),
            share(pixel.green(), colour[1]),
            share(pixel.blue(), colour[2]),
            pixel.alpha(),
        )?;
    }
    Some(out)
}

/// Every file a sprite can ask for: one, or one per frame of an animation.
fn wanted(sprite: &Sprite) -> Vec<String> {
    match sprite.animation {
        None => vec![sprite.path.clone()],
        Some(animation) => (0..animation.frames)
            .map(|frame| frame_path(&sprite.path, frame))
            .collect(),
    }
}

/// `sb/f.png` frame 3 is `sb/f3.png`, which is how osu! names them.
///
/// A still sprite is its own file and never comes here: frame nought of an
/// animation is `…0.png`, and handing a still sprite the same treatment would
/// look for a file the map does not have.
fn frame_path(path: &str, frame: u32) -> String {
    match path.rfind('.') {
        Some(dot) => format!("{}{}{}", &path[..dot], frame, &path[dot..]),
        None => format!("{path}{frame}"),
    }
}

/// One spelling for a path, so `SB\Flash.PNG` and `sb/flash.png` are the same
/// picture — which they are on the case-insensitive filesystem most maps were
/// authored on.
fn key(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}
