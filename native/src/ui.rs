use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::widget::{button, column, container, image, row, text, Space};
use iced::{mouse, Color, ContentFit, Element, Length, Point, Rectangle, Renderer, Size, Theme};

use crate::theme::{self, ACCENT, FAINT, INK, MUTED};

thread_local! {
    static PRESSED: std::cell::Cell<Option<(Rectangle, Point)>> = const { std::cell::Cell::new(None) };
    static FADE: std::cell::Cell<f32> = const { std::cell::Cell::new(1.0) };
    static TEXT_SURFACE: std::cell::Cell<Color> = const { std::cell::Cell::new(theme::GROUND) };
}

pub fn on_surface<T>(surface: Color, build: impl FnOnce() -> T) -> T {
    struct Restore(Color);
    impl Drop for Restore {
        fn drop(&mut self) { TEXT_SURFACE.with(|slot| slot.set(self.0)); }
    }
    let _restore = Restore(TEXT_SURFACE.with(|slot| slot.replace(surface)));
    build()
}

fn text_colour(colour: Color) -> Color {
    TEXT_SURFACE.with(|slot| theme::secondary_on(colour, slot.get()))
}

pub fn scene_surface(handle: &image::Handle, size: Size) -> Color {
    type Key = (iced::advanced::image::Id, u32);
    thread_local! { static CACHE: std::cell::RefCell<Vec<(Key, Color)>> = const { std::cell::RefCell::new(Vec::new()) }; }
    let ratio = (size.width / size.height.max(1.0)).clamp(0.1, 10.0);
    let key = (handle.id(), (ratio * 256.0).round() as u32);
    CACHE.with(|cache| {
        if let Some((_, colour)) = cache.borrow().iter().find(|(seen, _)| *seen == key) { return *colour; }
        let decoded = match handle {
            image::Handle::Bytes(_, bytes) => ::image::load_from_memory(bytes).ok().map(|image| image.to_rgba8()),
            image::Handle::Path(_, path) => ::image::open(path).ok().map(|image| image.to_rgba8()),
            _ => None,
        };
        let raw = match handle {
            image::Handle::Rgba { width, height, pixels, .. } => Some((*width, *height, pixels.as_ref())),
            _ => decoded.as_ref().map(|image| (image.width(), image.height(), image.as_raw().as_slice())),
        };
        let colour = raw.filter(|(w, h, bytes)| *w > 0 && *h > 0 && bytes.len() as u64 >= u64::from(*w) * u64::from(*h) * 4).map(|(w, h, pixels)| {
            let cw = (h as f32 * ratio).min(w as f32);
            let ch = (w as f32 / ratio).min(h as f32);
            let mut samples = Vec::with_capacity(256);
            for y in 0..8 {
                for x in 0..32 {
                    let px = ((w as f32 - cw) * 0.5 + cw * (x as f32 + 0.5) / 32.0) as u32;
                    let py = ((h as f32 - ch) * 0.5 + ch * (0.5 + 0.5 * (y as f32 + 0.5) / 8.0)) as u32;
                    let i = ((py.min(h - 1) * w + px.min(w - 1)) * 4) as usize;
                    let ink = Color::from_rgb(pixels[i] as f32 / 255.0, pixels[i + 1] as f32 / 255.0, pixels[i + 2] as f32 / 255.0);
                    samples.push(mix(theme::GROUND, ink, pixels[i + 3] as f32 / 255.0));
                }
            }
            samples.sort_by(|a, b| theme::luminance(*a).total_cmp(&theme::luminance(*b)));
            mix(samples[243], theme::GROUND, 0.66)
        }).unwrap_or(theme::GROUND);
        let mut cache = cache.borrow_mut();
        if cache.len() >= 16 { cache.remove(0); }
        cache.push((key, colour));
        colour
    })
}

#[cfg(test)]
mod adaptive_text {
    use super::*;

    #[test]
    fn scene_crop_and_transparency_determine_text_contrast() {
        let dark = image::Handle::from_rgba(4, 4, [0, 0, 0, 255].repeat(16));
        let bright = image::Handle::from_rgba(4, 4, [255, 255, 255, 255].repeat(16));
        let transparent = image::Handle::from_rgba(4, 4, [255, 255, 255, 0].repeat(16));
        let size = Size::new(980.0, 720.0);
        let on_dark = on_surface(scene_surface(&dark, size), || faded(FAINT));
        let on_light = on_surface(scene_surface(&bright, size), || faded(FAINT));
        assert!(theme::luminance(on_light) > theme::luminance(on_dark));
        assert_eq!(scene_surface(&transparent, size), theme::GROUND);
        assert_eq!(faded(FAINT), FAINT, "surface scope restores the ordinary panel palette");
        let pixels = (0..400).flat_map(|i| if i % 100 < 25 || i % 100 >= 75 { [255, 255, 255, 255] } else { [0, 0, 0, 255] }).collect::<Vec<_>>();
        let sides = image::Handle::from_rgba(100, 4, pixels);
        assert!(theme::luminance(scene_surface(&sides, size)) < 0.01, "cover crop excludes bright offscreen borders");
    }

    #[test]
    fn surface_scopes_nest_without_affecting_status_colours_or_opacity() {
        on_surface(Color::WHITE, || {
            let outside = faded(FAINT);
            on_surface(theme::GROUND, || assert_eq!(faded(FAINT), FAINT));
            assert_eq!(faded(FAINT), outside);
            assert_eq!(faded(ACCENT), ACCENT);
            fading(0.25, || assert_eq!(faded(FAINT).a, 0.25));
        });
        assert_eq!(faded(FAINT), FAINT);
    }
}

pub fn fading<T>(k: f32, build: impl FnOnce() -> T) -> T {
    let before = FADE.with(|f| f.replace(k.clamp(0.0, 1.0)));
    let made = build();
    FADE.with(|f| f.set(before));
    made
}

pub fn fade() -> f32 {
    FADE.with(|f| f.get())
}

pub fn elapsed(last: &mut Option<std::time::Instant>, now: std::time::Instant) -> f32 {
    let dt = last.map_or(1.0 / 60.0, |was| now.saturating_duration_since(was).as_secs_f32()).min(1.0 / 30.0);
    *last = Some(now);
    dt
}

pub fn toward(value: f32, target: f32, per_sixtieth: f32, dt: f32) -> f32 {
    target + (value - target) * (1.0 - per_sixtieth).powf(dt * 60.0)
}

#[cfg(test)]
mod easing {
    use super::toward;

    #[test]
    fn easing_is_the_same_at_sixty_and_a_hundred_and_twenty_frames() {
        let at_sixty = (0..30).fold(0.0, |v, _| toward(v, 1.0, 0.28, 1.0 / 60.0));
        let at_twice = (0..60).fold(0.0, |v, _| toward(v, 1.0, 0.28, 1.0 / 120.0));
        assert!((at_sixty - at_twice).abs() < 1e-4, "{at_sixty} against {at_twice}");
        let one_frame = toward(0.0, 1.0, 0.28, 1.0 / 60.0);
        assert!((one_frame - 0.28).abs() < 1e-6, "at sixty frames it moves as it always did");
    }
}

pub fn dim(colour: Color, k: f32) -> Color {
    Color { a: colour.a * k.clamp(0.0, 1.0), ..colour }
}

pub fn faded(colour: Color) -> Color {
    let colour = text_colour(colour);
    Color {
        a: colour.a * fade(),
        ..colour
    }
}

pub fn keys_for(text: &str, mac: bool) -> String {
    if !mac {
        return text.to_owned();
    }
    text.replace("Ctrl Shift ", "⇧⌘").replace("Ctrl ", "⌘").replace("Delete", "⌫")
}

pub fn keys(text: &str) -> String {
    keys_for(text, cfg!(target_os = "macos"))
}

pub fn mix(from: Color, to: Color, k: f32) -> Color {
    let k = k.clamp(0.0, 1.0);
    Color {
        r: from.r + (to.r - from.r) * k,
        g: from.g + (to.g - from.g) * k,
        b: from.b + (to.b - from.b) * k,
        a: from.a + (to.a - from.a) * k,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    Dot,
    Tick,
    Cross,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sign {
    pub glyph: Glyph,
    pub grown: f32,
    pub breath: f32,
    pub alpha: f32,
}

impl Sign {
    pub fn settled(glyph: Glyph) -> Sign {
        Sign {
            glyph,
            grown: 1.0,
            breath: 0.0,
            alpha: fade(),
        }
    }

    pub fn growing(glyph: Glyph, grown: f32) -> Sign {
        Sign { grown: grown.clamp(0.0, 1.0), ..Sign::settled(glyph) }
    }

    pub fn breathing(breath: f32) -> Sign {
        Sign { breath: breath.clamp(0.0, 1.0), ..Sign::settled(Glyph::Dot) }
    }
}

const GLYPH: f32 = 14.0;

impl<Message> canvas::Program<Message> for Sign {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let centre = Point::new(GLYPH / 2.0, GLYPH / 2.0);
        let ink = Color { a: ACCENT.a * self.alpha, ..ACCENT };
        let halo = Color { a: theme::ACCENT_SOFT.a * self.alpha * (1.0 + self.breath * 0.6), ..theme::ACCENT_SOFT };
        match self.glyph {
            Glyph::Dot => {
                frame.fill(&Path::circle(centre, 6.0 + 2.0 * self.grown + 2.5 * self.breath), halo);
                frame.fill(&Path::circle(centre, 4.0 * self.grown), ink);
            }
            Glyph::Tick => {
                let reach = self.grown;
                let path = Path::new(|b| {
                    b.move_to(Point::new(3.0, 7.5));
                    let first = (reach * 2.0).min(1.0);
                    b.line_to(Point::new(3.0 + 3.0 * first, 7.5 + 3.0 * first));
                    if reach > 0.5 {
                        let second = (reach - 0.5) * 2.0;
                        b.line_to(Point::new(6.0 + 6.0 * second, 10.5 - 7.0 * second));
                    }
                });
                frame.stroke(&path, stroke(ink, 1.8));
            }
            Glyph::Cross => {
                let path = Path::new(|b| {
                    b.move_to(Point::new(3.5, 3.5));
                    b.line_to(Point::new(3.5 + 7.0 * self.grown, 3.5 + 7.0 * self.grown));
                    b.move_to(Point::new(10.5, 3.5));
                    b.line_to(Point::new(10.5 - 7.0 * self.grown, 3.5 + 7.0 * self.grown));
                });
                frame.stroke(&path, stroke(ink, 1.8));
            }
            Glyph::None => {}
        }
        vec![frame.into_geometry()]
    }
}

fn stroke<'a>(color: Color, width: f32) -> Stroke<'a> {
    Stroke {
        style: canvas::Style::Solid(color),
        width,
        line_cap: canvas::LineCap::Round,
        line_join: canvas::LineJoin::Round,
        ..Stroke::default()
    }
}

pub fn glyph<'a, Message: 'a>(which: Glyph) -> Element<'a, Message> {
    sign(Sign::settled(which))
}

pub fn sign<'a, Message: 'a>(which: Sign) -> Element<'a, Message> {
    Canvas::new(which).width(GLYPH).height(GLYPH).into()
}

const LETTER_MASK: &[u8] = include_bytes!("../assets/letter-mask.png");
pub const ACCENT_DEEP: Color = Color::from_rgb(0.788, 0.204, 0.184);

pub struct Letter {
    pub side: u32,
    mask: Vec<u8>,
}

impl Letter {
    fn read() -> Letter {
        let decoder = png::Decoder::new(std::io::Cursor::new(LETTER_MASK));
        let mut reader = decoder.read_info().expect("the letter mask is a png");
        let mut raw = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut raw).expect("the letter mask decodes");
        let (w, h) = (info.width as usize, info.height as usize);
        let channels = raw.len() / (w * h);
        let value = |x: usize, y: usize| raw[(y * w + x) * channels];
        let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
        for y in 0..h {
            for x in 0..w {
                if value(x, y) > 8 {
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x);
                    y1 = y1.max(y);
                }
            }
        }
        let tall = (y1 - y0 + 1).max(x1 - x0 + 1);
        let side = tall + tall / 8;
        let left = (x0 + x1 + 1) / 2;
        let top = (y0 + y1 + 1) / 2;
        let mut mask = vec![0u8; side * side];
        for y in 0..side {
            for x in 0..side {
                let sx = (left + x) as isize - (side / 2) as isize;
                let sy = (top + y) as isize - (side / 2) as isize;
                if sx >= 0 && sy >= 0 && (sx as usize) < w && (sy as usize) < h {
                    mask[y * side + x] = value(sx as usize, sy as usize);
                }
            }
        }
        Letter { side: side as u32, mask }
    }

    pub fn mask(&self) -> &[u8] {
        &self.mask
    }

    pub fn tinted(&self, top: Color, bottom: Color) -> image::Handle {
        let side = self.side as usize;
        let mut pixels = Vec::with_capacity(side * side * 4);
        for y in 0..side {
            let k = y as f32 / (side - 1).max(1) as f32;
            let colour = mix(top, bottom, k);
            for x in 0..side {
                let a = self.mask[y * side + x];
                pixels.extend_from_slice(&[
                    (colour.r * 255.0) as u8,
                    (colour.g * 255.0) as u8,
                    (colour.b * 255.0) as u8,
                    a,
                ]);
            }
        }
        image::Handle::from_rgba(self.side, self.side, pixels)
    }
}

pub fn letter() -> &'static Letter {
    static LETTER: std::sync::OnceLock<Letter> = std::sync::OnceLock::new();
    LETTER.get_or_init(Letter::read)
}

pub fn letter_red() -> &'static image::Handle {
    static HANDLE: std::sync::OnceLock<image::Handle> = std::sync::OnceLock::new();
    HANDLE.get_or_init(|| letter().tinted(ACCENT, ACCENT_DEEP))
}

pub fn letter_ink() -> &'static image::Handle {
    static HANDLE: std::sync::OnceLock<image::Handle> = std::sync::OnceLock::new();
    HANDLE.get_or_init(|| letter().tinted(QR_INK, QR_INK))
}

pub const EMBLEM: f32 = 36.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Emblem {
    pub alpha: f32,
}

impl<Message> canvas::Program<Message> for Emblem {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let side = bounds.width.min(bounds.height);
        let drawn = side * 0.86;
        let corner = Point::new((side - drawn) / 2.0, (side - drawn) / 2.0);
        frame.draw_image(
            Rectangle::new(corner, Size::new(drawn, drawn)),
            canvas::Image::new(letter_red()).filter_method(image::FilterMethod::Linear).opacity(self.alpha),
        );
        vec![frame.into_geometry()]
    }
}

pub const QR_SIDE: f32 = 156.0;
const QR_INK: Color = Color::from_rgb(0.078, 0.027, 0.039);

#[derive(Debug, Clone, PartialEq)]
pub struct Qr {
    cells: Vec<Vec<bool>>,
    pub heart: f32,
    pub module_round: f32,
    pub finder_round: f32,
    pub quiet: f32,
}

impl Qr {
    pub fn new(cells: Vec<Vec<bool>>) -> Qr {
        Qr {
            cells,
            heart: 0.25,
            module_round: 0.32,
            finder_round: 0.6,
            quiet: 4.0,
        }
    }

    fn n(&self) -> usize {
        self.cells.len()
    }

    fn in_finder(&self, x: usize, y: usize) -> bool {
        let n = self.n();
        (x < 7 && y < 7) || (x >= n - 7 && y < 7) || (x < 7 && y >= n - 7)
    }

    fn heart_cells(&self) -> f32 {
        if self.heart <= 0.0 {
            0.0
        } else {
            (self.n() as f32 * self.heart).max(7.0)
        }
    }

    fn in_heart(&self, x: usize, y: usize) -> bool {
        let n = self.n() as f32;
        let hole = self.heart_cells() / 2.0;
        let (cx, cy) = (n / 2.0, n / 2.0);
        let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
        dx * dx + dy * dy < hole * hole
    }
}

impl<Message> canvas::Program<Message> for Qr {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let side = bounds.width.min(bounds.height);
        frame.fill(
            &Path::rounded_rectangle(Point::ORIGIN, Size::new(side, side), 12.0.into()),
            Color::WHITE,
        );
        let n = self.n();
        if n == 0 {
            return vec![frame.into_geometry()];
        }
        let cell = side / (n as f32 + self.quiet * 2.0);
        let pad = cell * self.quiet;
        let at = |i: usize| pad + i as f32 * cell;
        for y in 0..n {
            for x in 0..n {
                if !self.cells[y][x] || self.in_finder(x, y) || self.in_heart(x, y) {
                    continue;
                }
                frame.fill(
                    &Path::rounded_rectangle(
                        Point::new(at(x), at(y)),
                        Size::new(cell, cell),
                        (cell * self.module_round).into(),
                    ),
                    QR_INK,
                );
            }
        }
        for (fx, fy) in [(0, 0), (n - 7, 0), (0, n - 7)] {
            let outer = Rectangle::new(Point::new(at(fx) + cell / 2.0, at(fy) + cell / 2.0), Size::new(cell * 6.0, cell * 6.0));
            frame.stroke(
                &Path::rounded_rectangle(outer.position(), outer.size(), (cell * self.finder_round).into()),
                Stroke {
                    style: canvas::Style::Solid(QR_INK),
                    width: cell,
                    ..Stroke::default()
                },
            );
            frame.fill(
                &Path::rounded_rectangle(Point::new(at(fx + 2), at(fy + 2)), Size::new(cell * 3.0, cell * 3.0), (cell * self.finder_round * 0.45).into()),
                QR_INK,
            );
        }
        let heart = self.heart_cells() * cell;
        if heart > 0.0 {
            let centre = Point::new(side / 2.0, side / 2.0);
            frame.fill(&Path::circle(centre, heart / 2.0), Color::WHITE);
            let drawn = heart * 0.56;
            frame.draw_image(
                Rectangle::new(Point::new(centre.x - drawn / 2.0, centre.y - drawn / 2.0), Size::new(drawn, drawn)),
                canvas::Image::new(letter_ink()).filter_method(image::FilterMethod::Linear),
            );
        }
        vec![frame.into_geometry()]
    }
}

pub fn qr<'a, Message: 'a>(code: &Qr) -> Element<'a, Message> {
    Canvas::new(code.clone()).width(QR_SIDE).height(QR_SIDE).into()
}

static AUTO_SCALE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(100);

pub fn auto_scale() -> u32 {
    AUTO_SCALE.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn set_auto_scale(percent: u32) {
    AUTO_SCALE.store(percent, std::sync::atomic::Ordering::Relaxed);
}

pub fn auto_scale_for(logical_height: f32) -> u32 {
    let k = (logical_height / 1080.0).max(0.1).powf(0.7);
    ((k * 20.0).round() * 5.0).clamp(80.0, 160.0) as u32
}

pub fn viewport_at(size: Size, before: f32, after: f32) -> Size {
    Size::new(size.width * before / after, size.height * before / after)
}

pub fn scale_of(chosen: u32) -> f32 {
    let percent = if chosen == 0 { auto_scale() } else { chosen };
    percent as f32 / 100.0
}

pub fn refit(viewport: Size, before: f32, after: f32, least: Size) -> Option<Size> {
    if after <= 0.0 || (before - after).abs() < 0.001 {
        return None;
    }
    let seen = Size::new(viewport.width * before / after, viewport.height * before / after);
    (seen.width < least.width || seen.height < least.height).then(|| Size::new(seen.width.max(least.width), seen.height.max(least.height)))
}

pub fn brand<'a, Message: 'a>() -> Element<'a, Message> {
    brand_scaled(1.0)
}

pub fn brand_scaled<'a, Message: 'a>(scale: f32) -> Element<'a, Message> {
    let alpha = fade();
    row![
        Canvas::new(Emblem { alpha }).width(EMBLEM * scale).height(EMBLEM * scale),
        container(Space::new().width(1.0).height(22.0 * scale)).style(move |theme| {
            let mut style = theme::rule_high(theme);
            if let Some(iced::Background::Color(c)) = style.background {
                style.background = Some(iced::Background::Color(Color { a: c.a * alpha, ..c }));
            }
            style
        }),
        text("Dossier").font(theme::SANS_SEMI).size(20.0 * scale).color(faded(INK)),
    ]
    .spacing(14.0 * scale)
    .align_y(iced::Center)
    .into()
}

pub struct SceneShade {
    pub alpha: f32,
    pub rest: f32,
    pub curve: fn(f32) -> f32,
}

impl SceneShade {
    pub fn at(&self, t: f32) -> f32 {
        let edge = 0.5 + 0.28 * self.rest.clamp(0.0, 1.0);
        let sample = if t <= 0.5 { t } else { 0.5 + 0.5 * ((t - edge) / (1.0 - edge)).clamp(0.0, 1.0) };
        (self.curve)(sample) * self.alpha
    }
}

impl<Message> canvas::Program<Message> for SceneShade {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let edge = 0.5 + 0.28 * self.rest.clamp(0.0, 1.0);
        let mut shade = canvas::gradient::Linear::new(Point::ORIGIN, Point::new(0.0, bounds.height));
        for t in [0.0, 0.14, 0.3, 0.5, edge, edge + (1.0 - edge) * 0.45, edge + (1.0 - edge) * 0.7, 1.0] {
            shade = shade.add_stop(t, Color { a: self.at(t), ..theme::GROUND });
        }
        frame.fill(&Path::rectangle(Point::ORIGIN, bounds.size()),
            canvas::Fill { style: canvas::Style::Gradient(shade.into()), ..canvas::Fill::default() });
        vec![frame.into_geometry()]
    }
}

const BACKDROP: (u32, u32) = (490, 360);

pub fn backdrop_pixels() -> Vec<u8> {
    let (w, h) = BACKDROP;
    let stops: [(f32, [f32; 3]); 5] = [
        (0.0, [0x3a as f32, 0x10 as f32, 0x15 as f32]),
        (0.30, [0x26 as f32, 0x09 as f32, 0x0f as f32]),
        (0.55, [0x17 as f32, 0x07 as f32, 0x0b as f32]),
        (0.78, [0x0d as f32, 0x05 as f32, 0x08 as f32]),
        (1.0, [0x07 as f32, 0x03 as f32, 0x04 as f32]),
    ];
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let dx = (x as f32 / w as f32 - 0.5) / 1.15;
            let dy = (y as f32 / h as f32 + 0.18) / 0.95;
            let t = (dx * dx + dy * dy).sqrt().min(1.0);
            let mut colour = stops[4].1;
            for pair in stops.windows(2) {
                let (a, ca) = pair[0];
                let (b, cb) = pair[1];
                if t >= a && t <= b {
                    let k = if b > a { (t - a) / (b - a) } else { 0.0 };
                    colour = [
                        ca[0] + (cb[0] - ca[0]) * k,
                        ca[1] + (cb[1] - ca[1]) * k,
                        ca[2] + (cb[2] - ca[2]) * k,
                    ];
                    break;
                }
            }
            out.extend_from_slice(&[colour[0] as u8, colour[1] as u8, colour[2] as u8, 255]);
        }
    }
    out
}

pub fn backdrop<'a, Message: 'a>(handle: &image::Handle) -> Element<'a, Message> {
    image(handle.clone())
        .content_fit(ContentFit::Cover)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

pub fn backdrop_handle() -> image::Handle {
    image::Handle::from_rgba(BACKDROP.0, BACKDROP.1, backdrop_pixels())
}

pub fn headline<'a, Message: 'a>(which: Sign, words: String, count: String) -> Element<'a, Message> {
    row![
        sign(which),
        text(words).font(theme::SANS_SEMI).size(theme::LEAD).color(INK),
        text(count).font(theme::SANS).size(theme::LEAD).color(MUTED),
    ]
    .spacing(8)
    .align_y(iced::Center)
    .into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mood {
    Done,
    Now,
    Todo,
    Failed,
}

#[derive(Debug, Clone)]
pub struct Line {
    pub mood: Mood,
    pub name: String,
    pub detail: String,
    pub note: Option<(String, Option<String>)>,
    pub action: Option<String>,
    pub settled: f32,
    pub breath: f32,
}

impl Line {
    pub fn new(mood: Mood, name: impl Into<String>) -> Line {
        Line {
            mood,
            name: name.into(),
            detail: String::new(),
            note: None,
            action: None,
            settled: 1.0,
            breath: 0.0,
        }
    }

    pub fn action(mut self, label: impl Into<String>) -> Line {
        self.action = Some(label.into());
        self
    }

    pub fn settling(mut self, settled: f32) -> Line {
        self.settled = settled.clamp(0.0, 1.0);
        self
    }

    pub fn breathing(mut self, breath: f32) -> Line {
        self.breath = breath.clamp(0.0, 1.0);
        self
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Line {
        self.detail = detail.into();
        self
    }

    pub fn note(mut self, reason: impl Into<String>, link: Option<String>) -> Line {
        self.note = Some((reason.into(), link));
        self
    }
}

pub fn ledger<'a, Message: Clone + 'a>(lines: &[Line], on_link: Option<Message>) -> Element<'a, Message> {
    ledger_with(lines, on_link, None)
}

pub fn ledger_with<'a, Message: Clone + 'a>(
    lines: &[Line],
    on_link: Option<Message>,
    on_action: Option<Message>,
) -> Element<'a, Message> {
    let mut rows = column![].spacing(4);
    for line in lines {
        let (which, colour, face, before) = match line.mood {
            Mood::Done => (Glyph::Tick, MUTED, theme::MONO, INK),
            Mood::Now => (Glyph::Dot, INK, theme::MONO_BOLD, FAINT),
            Mood::Todo => (Glyph::None, FAINT, theme::MONO, FAINT),
            Mood::Failed => (Glyph::Cross, ACCENT, theme::MONO_BOLD, INK),
        };
        let colour = faded(mix(before, colour, line.settled));
        let detail_colour = faded(match line.mood {
            Mood::Now => MUTED,
            Mood::Failed => ACCENT,
            _ => FAINT,
        });
        let sign_now = if line.mood == Mood::Now && line.breath > 0.0 {
            Sign::breathing(line.breath)
        } else {
            Sign::growing(which, line.settled)
        };
        let mut words = row![text(line.name.clone()).font(face).size(theme::BODY).color(colour)].spacing(14);
        if !line.detail.is_empty() {
            words = words.push(text(line.detail.clone()).font(theme::MONO).size(theme::BODY).color(detail_colour));
        }
        rows = rows.push(
            container(row![sign(sign_now), words].spacing(12).align_y(iced::Center)).height(24.0),
        );
        if let Some((reason, link)) = &line.note {
            let mut note = row![text(reason.clone()).font(theme::SANS).size(theme::CAPTION).color(faded(MUTED))].spacing(6);
            if let Some(link) = link {
                let go = button(text(link.clone()).font(theme::SANS).size(theme::CAPTION).color(faded(ACCENT)))
                    .padding([6, 0])
                    .style(theme::link);
                note = note.push(match on_link.clone() {
                    Some(message) => go.on_press(message),
                    None => go,
                });
            }
            rows = rows.push(container(note.align_y(iced::Center)).padding(iced::Padding::ZERO.left(26.0)));
        }
        if let Some(label) = &line.action {
            rows = rows.push(
                container(row![primary(label.clone(), on_action.clone())]).padding(iced::Padding::ZERO.left(26.0).top(4.0).bottom(4.0)),
            );
        }
    }
    rows.into()
}

fn halves<'a, Message: 'a>(top: Element<'a, Message>, bottom: Option<Element<'a, Message>>) -> iced::widget::Column<'a, Message> {
    let mut inside = column![container(top).padding([20, 24])];
    if let Some(bottom) = bottom {
        inside = inside.push(container(Space::new().height(1.0)).width(Length::Fill).style(theme::rule));
        inside = inside.push(container(bottom).padding(24));
    }
    inside
}

pub fn card<'a, Message: 'a>(top: Element<'a, Message>, bottom: Option<Element<'a, Message>>) -> Element<'a, Message> {
    container(halves(top, bottom)).width(Length::Fill).style(theme::card).into()
}

pub fn sheet<'a, Message: 'a>(top: Element<'a, Message>, bottom: Option<Element<'a, Message>>) -> Element<'a, Message> {
    container(halves(top, bottom)).width(Length::Fill).style(box_faded(theme::sheet)).into()
}

pub fn switch<'a, Message: Clone + 'a>(sides: Vec<(String, Option<String>, bool, Message)>) -> Element<'a, Message> {
    let k = fade();
    let mut inside = row![].spacing(2);
    for (label, count, on, press) in sides {
        let mut words = row![text(label).font(theme::SANS_SEMI).size(13.0).wrapping(text::Wrapping::None)].spacing(7).align_y(iced::Center);
        if let Some(count) = count {
            let badge = container(text(count).font(theme::MONO_BOLD).size(10.0).color(faded(Color::WHITE)))
                .padding([1, 6])
                .style(move |_| container::Style {
                    background: Some(iced::Background::Color(Color { a: k, ..theme::ACCENT })),
                    border: iced::Border { radius: 8.0.into(), ..iced::Border::default() },
                    ..container::Style::default()
                });
            words = words.push(badge);
        }
        inside = inside.push(
            button(container(words).center_y(32.0))
                .padding([0, 16])
                .style(button_faded(move |_: &Theme, status: button::Status| {
                    let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: match (on, lit) {
                            (true, _) => Some(iced::Background::Color(Color::from_rgb(0.231, 0.09, 0.102))),
                            (false, true) => Some(iced::Background::Color(Color::from_rgb(0.102, 0.071, 0.078))),
                            (false, false) => None,
                        },
                        text_color: if on { INK } else { MUTED },
                        border: iced::Border { radius: 8.0.into(), ..iced::Border::default() },
                        shadow: iced::Shadow::default(),
                        snap: true,
                    }
                }))
                .on_press(press),
        );
    }
    container(inside)
        .padding(3)
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(Color { a: k, ..Color::from_rgb(0.035, 0.016, 0.022) })),
            border: iced::Border { color: Color { a: k, ..Color::from_rgb(0.118, 0.094, 0.102) }, width: 1.0, radius: 10.0.into() },
            ..container::Style::default()
        })
        .into()
}

pub fn title<'a, Message: 'a>(words: String) -> Element<'a, Message> {
    text(words).font(theme::SANS_SEMI).size(theme::LEAD).color(faded(INK)).into()
}

pub fn heading<'a, Message: 'a>(name: String, words: String) -> Element<'a, Message> {
    column![title(name), why(words)].spacing(4).into()
}

pub fn gap<'a, Message: 'a>(height: f32) -> Element<'a, Message> {
    Space::new().height(height).into()
}

pub fn why<'a, Message: 'a>(words: String) -> Element<'a, Message> {
    text(words).font(theme::SANS).size(theme::BODY).color(faded(MUTED)).into()
}

pub fn cap<'a, Message: 'a>(words: String) -> Element<'a, Message> {
    text(words).font(theme::SANS).size(theme::CAPTION).color(faded(MUTED)).into()
}

pub fn faint<'a, Message: 'a>(words: String) -> Element<'a, Message> {
    text(words).font(theme::SANS).size(theme::CAPTION).color(faded(FAINT)).into()
}

pub fn body<'a, Message: 'a>(words: String, colour: Color) -> Element<'a, Message> {
    text(words).font(theme::SANS).size(theme::BODY).color(faded(colour)).into()
}

pub fn mono_small<'a, Message: 'a>(words: String, colour: Color) -> Element<'a, Message> {
    text(words).font(theme::MONO).size(11.0).color(faded(colour)).into()
}

pub fn mono<'a, Message: 'a>(words: String, colour: Color) -> Element<'a, Message> {
    text(words).font(theme::MONO).size(theme::BODY).color(faded(colour)).into()
}

fn dim_background(background: iced::Background, k: f32) -> iced::Background {
    match background {
        iced::Background::Color(colour) => iced::Background::Color(Color { a: colour.a * k, ..colour }),
        iced::Background::Gradient(iced::Gradient::Linear(mut linear)) => {
            for stop in linear.stops.iter_mut().flatten() {
                stop.color.a *= k;
            }
            iced::Background::Gradient(iced::Gradient::Linear(linear))
        }
    }
}

fn dimmed(style: impl Fn(&Theme, button::Status) -> button::Style + 'static, k: f32) -> impl Fn(&Theme, button::Status) -> button::Style {
    let surface = TEXT_SURFACE.with(|slot| slot.get());
    move |theme, status| {
        let mut made = style(theme, status);
        let background = match made.background {
            Some(iced::Background::Color(colour)) => mix(surface, colour, colour.a),
            _ => surface,
        };
        made.text_color = theme::secondary_on(made.text_color, background);
        made.text_color = Color { a: made.text_color.a * k, ..made.text_color };
        made.background = made.background.map(|background| dim_background(background, k));
        made.border.color = Color { a: made.border.color.a * k, ..made.border.color };
        made.shadow.color = Color { a: made.shadow.color.a * k, ..made.shadow.color };
        made
    }
}

fn dimmed_box(style: impl Fn(&Theme) -> container::Style + 'static, k: f32) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let mut made = style(theme);
        made.background = made.background.map(|background| dim_background(background, k));
        made.border.color = Color { a: made.border.color.a * k, ..made.border.color };
        made.shadow.color = Color { a: made.shadow.color.a * k, ..made.shadow.color };
        made.text_color = made.text_color.map(|colour| Color { a: colour.a * k, ..colour });
        made
    }
}

pub fn primary<'a, Message: Clone + 'a>(words: String, on: Option<Message>) -> Element<'a, Message> {
    let made = button(container(text(words).font(theme::SANS_SEMI).size(theme::BODY)).center_y(theme::CONTROL_HEIGHT - 2.0))
        .padding([0, 12])
        .style(dimmed(theme::primary, fade()));
    match on {
        Some(message) => made.on_press(message).into(),
        None => made.into(),
    }
}

pub fn quiet<'a, Message: Clone + 'a>(words: String, on: Option<Message>) -> Element<'a, Message> {
    let made = button(container(text(words).font(theme::SANS_SEMI).size(theme::BODY)).center_y(theme::CONTROL_HEIGHT - 2.0))
        .padding([0, 12])
        .style(dimmed(theme::quiet, fade()));
    match on {
        Some(message) => made.on_press(message).into(),
        None => made.into(),
    }
}

pub fn small_button<'a, Message: Clone + 'a>(words: String, on: Message) -> Element<'a, Message> {
    button(container(text(words).font(theme::SANS_SEMI).size(theme::CAPTION)).center_y(24.0))
        .padding([0, 10])
        .style(dimmed(theme::small, fade()))
        .on_press(on)
        .into()
}

pub fn link<'a, Message: Clone + 'a>(words: String, on: Message) -> Element<'a, Message> {
    button(text(words).font(theme::SANS).size(theme::CAPTION))
        .padding([6, 0])
        .style(dimmed(theme::link, fade()))
        .on_press(on)
        .into()
}

pub fn tag<'a, Message: 'a>(words: String) -> Element<'a, Message> {
    container(text(words).font(theme::MONO_BOLD).size(theme::CAPTION).color(faded(MUTED)))
        .padding([0, 6])
        .style(dimmed_box(theme::tag, fade()))
        .into()
}

pub fn tile<'a, Message: 'a>(value: String, label: String) -> Element<'a, Message> {
    container(
        column![
            text(value).font(theme::SANS_SEMI).size(theme::TITLE).color(faded(INK)),
            text(label).font(theme::SANS).size(theme::CAPTION).color(faded(MUTED)),
        ]
        .spacing(2),
    )
    .padding([12, 16])
    .width(Length::Fill)
    .style(dimmed_box(theme::tile, fade()))
    .into()
}

pub fn well<'a, Message: 'a>(inside: Element<'a, Message>) -> Element<'a, Message> {
    container(inside)
        .padding([0, 12])
        .height(theme::CONTROL_HEIGHT)
        .width(Length::Fill)
        .center_y(theme::CONTROL_HEIGHT)
        .style(dimmed_box(theme::well, fade()))
        .into()
}

pub fn well_box<'a, Message: 'a>(inside: Element<'a, Message>, height: f32) -> Element<'a, Message> {
    container(inside)
        .padding([0, 12])
        .height(height)
        .width(Length::Fill)
        .center_y(height)
        .style(dimmed_box(theme::well, fade()))
        .into()
}

pub fn rising<'a, Message: 'a>(k: f32, inside: Element<'a, Message>) -> Element<'a, Message> {
    container(inside)
        .padding(iced::Padding::ZERO.top((1.0 - k.clamp(0.0, 1.0)) * 10.0))
        .width(Length::Fill)
        .into()
}

pub fn grow<'a, Message: 'a>() -> Element<'a, Message> {
    Space::new().width(Length::Fill).into()
}

pub fn grow_tall<'a, Message: 'a>() -> Element<'a, Message> {
    Space::new().height(Length::Fill).into()
}

pub fn sized(w: f32, h: f32) -> Size {
    Size::new(w, h)
}

pub struct Hatch {
    pub stroke: f32,
    pub step: f32,
    pub colour: Color,
}

impl<Message> canvas::Program<Message> for Hatch {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let reach = bounds.width + bounds.height;
        let mut x = -bounds.height;
        while x < reach {
            let line = Path::line(Point::new(x, bounds.height), Point::new(x + bounds.height, 0.0));
            frame.stroke(
                &line,
                Stroke::default().with_width(self.stroke).with_color(self.colour),
            );
            x += self.step;
        }
        vec![frame.into_geometry()]
    }
}

pub fn hatch<'a, Message: 'a>() -> Element<'a, Message> {
    Canvas::new(Hatch { stroke: 8.0, step: 18.0, colour: Color::from_rgba(1.0, 1.0, 1.0, 0.03 * fade().clamp(0.0, 1.0)) }).width(Length::Fill).height(Length::Fill).into()
}

pub fn band_hatch<'a, Message: 'a>(colour: Color) -> Element<'a, Message> {
    Canvas::new(Hatch { stroke: 6.0, step: 17.0, colour: faded(colour) }).width(Length::Fill).height(Length::Fill).into()
}

pub fn fine_hatch<'a, Message: 'a>() -> Element<'a, Message> {
    Canvas::new(Hatch { stroke: 1.5, step: 5.0, colour: Color::from_rgba(1.0, 1.0, 1.0, 0.03 * fade().clamp(0.0, 1.0)) }).width(Length::Fill).height(Length::Fill).into()
}

pub fn in_thread<T>(work: impl FnOnce() -> T + Send + 'static) -> iced::Task<T>
where
    T: Send + 'static,
{
    iced::Task::run(
        iced::stream::channel(1, async move |out: iced::futures::channel::mpsc::Sender<T>| {
            std::thread::spawn(move || {
                let mut out = out;
                let made = work();
                let mut made = Some(made);
                while let Some(item) = made.take() {
                    match out.try_send(item) {
                        Ok(()) => {}
                        Err(e) if e.is_full() => {
                            made = Some(e.into_inner());
                            std::thread::sleep(std::time::Duration::from_millis(10));
                        }
                        Err(_) => {}
                    }
                }
            });
        }),
        |made| made,
    )
}

pub fn streamed<T>(work: impl FnOnce(&mut dyn FnMut(T) -> bool) + Send + 'static) -> iced::Task<T>
where
    T: Send + 'static,
{
    iced::Task::run(
        iced::stream::channel(32, async move |out: iced::futures::channel::mpsc::Sender<T>| {
            std::thread::spawn(move || {
                let mut out = out;
                let mut push = |item: T| {
                    let mut item = Some(item);
                    while let Some(inner) = item.take() {
                        match out.try_send(inner) {
                            Ok(()) => return true,
                            Err(e) if e.is_full() => {
                                item = Some(e.into_inner());
                                std::thread::sleep(std::time::Duration::from_millis(5));
                            }
                            Err(_) => return false,
                        }
                    }
                    true
                };
                work(&mut push);
            });
        }),
        |made| made,
    )
}

pub struct Veil {
    background: iced::Background,
}

impl Veil {
    pub fn new(background: impl Into<iced::Background>) -> Veil {
        Veil { background: background.into() }
    }
}

impl<Message> iced::advanced::Widget<Message, Theme, Renderer> for Veil {
    fn size(&self) -> Size<Length> {
        Size { width: Length::Fill, height: Length::Fill }
    }

    fn layout(
        &mut self,
        _: &mut iced::advanced::widget::Tree,
        _: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        iced::advanced::layout::atomic(limits, Length::Fill, Length::Fill)
    }

    fn draw(
        &self,
        _: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        _: &Theme,
        _: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        _: mouse::Cursor,
        _: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        let bounds = layout.bounds();
        let background = self.background.clone();
        renderer.with_layer(bounds, |renderer| {
            renderer.fill_quad(
                iced::advanced::renderer::Quad { bounds, ..iced::advanced::renderer::Quad::default() },
                background,
            );
        });
    }
}

impl<'a, Message: 'a> From<Veil> for Element<'a, Message> {
    fn from(veil: Veil) -> Element<'a, Message> {
        Element::new(veil)
    }
}

pub fn thin_scroll(_: &Theme, status: iced::widget::scrollable::Status) -> iced::widget::scrollable::Style {
    let held = matches!(
        status,
        iced::widget::scrollable::Status::Hovered { is_vertical_scrollbar_hovered: true, .. }
            | iced::widget::scrollable::Status::Dragged { is_vertical_scrollbar_dragged: true, .. }
    );
    let k = fade();
    let rail = iced::widget::scrollable::Rail {
        background: Some(iced::Background::Color(dim(Color::from_rgba(1.0, 1.0, 1.0, 0.04), k))),
        border: iced::Border { radius: 4.0.into(), ..iced::Border::default() },
        scroller: iced::widget::scrollable::Scroller {
            background: iced::Background::Color(dim(Color::from_rgba(1.0, 1.0, 1.0, if held { 0.38 } else { 0.2 }), k)),
            border: iced::Border { radius: 4.0.into(), ..iced::Border::default() },
        },
    };
    iced::widget::scrollable::Style {
        container: container::Style::default(),
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: iced::widget::scrollable::AutoScroll {
            background: iced::Background::Color(Color::TRANSPARENT),
            border: iced::Border::default(),
            shadow: iced::Shadow::default(),
            icon: Color::TRANSPARENT,
        },
    }
}

pub fn veil<'a, Message: 'a>(colour: Color) -> Element<'a, Message> {
    Veil::new(faded(colour)).into()
}

pub fn box_faded(style: impl Fn(&Theme) -> container::Style + 'static) -> impl Fn(&Theme) -> container::Style {
    dimmed_box(style, fade())
}

pub fn box_at(style: impl Fn(&Theme) -> container::Style + 'static, k: f32) -> impl Fn(&Theme) -> container::Style {
    dimmed_box(style, k)
}

pub fn button_faded(style: impl Fn(&Theme, button::Status) -> button::Style + 'static) -> impl Fn(&Theme, button::Status) -> button::Style {
    dimmed(style, fade())
}

pub struct Trail {
    pub points: std::sync::Arc<Vec<(f64, f32, f32)>>,
    pub at_ms: f64,
    pub window_ms: f64,
}

const PLAYFIELD: (f32, f32) = (512.0, 384.0);

impl<Message> canvas::Program<Message> for Trail {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let scale = (bounds.height * 0.8 / PLAYFIELD.1).min(bounds.width * 0.8 / PLAYFIELD.0);
        let origin = Point::new(
            (bounds.width - PLAYFIELD.0 * scale) / 2.0,
            (bounds.height - PLAYFIELD.1 * scale) / 2.0 + 8.0 * scale,
        );
        let place = |x: f32, y: f32| Point::new(origin.x + x * scale, origin.y + y * scale);
        let from = self.at_ms - self.window_ms;
        let start = self.points.partition_point(|(t, _, _)| *t < from);
        let end = self.points.partition_point(|(t, _, _)| *t <= self.at_ms);
        if end > start + 1 {
            let slice = &self.points[start..end];
            let mut previous = place(slice[0].1, slice[0].2);
            for (t, x, y) in &slice[1..] {
                let here = place(*x, *y);
                let fresh = ((t - from) / self.window_ms).clamp(0.0, 1.0) as f32;
                let alpha = 0.85 * fresh * fresh;
                let width = 1.0 + 2.2 * fresh;
                let segment = Path::line(previous, here);
                frame.stroke(
                    &segment,
                    Stroke::default()
                        .with_width(width * 3.0)
                        .with_color(Color { a: alpha * 0.12, ..ACCENT })
                        .with_line_cap(canvas::LineCap::Round),
                );
                frame.stroke(
                    &segment,
                    Stroke::default().with_width(width).with_color(Color { a: alpha, ..ACCENT }).with_line_cap(canvas::LineCap::Round),
                );
                previous = here;
            }
        }
        if let Some((_, x, y)) = self.points.get(end.saturating_sub(1)).filter(|_| end > 0) {
            let here = place(*x, *y);
            frame.fill(&Path::circle(here, 14.0), Color { a: 0.12, ..ACCENT });
            frame.fill(&Path::circle(here, 4.5), ACCENT);
            frame.stroke(&Path::circle(here, 10.0), Stroke::default().with_width(1.5).with_color(Color { a: 0.45, ..ACCENT }));
        }
        vec![frame.into_geometry()]
    }
}

pub fn hatched_picture(width: u32, height: u32, dim: impl Fn(f32) -> f32) -> image::Handle {
    let ground = [theme::GROUND.r * 255.0, theme::GROUND.g * 255.0, theme::GROUND.b * 255.0];
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        let a = dim(y as f32 / (height.max(2) - 1) as f32);
        for x in 0..width {
            let diagonal = (x as i64 + y as i64).rem_euclid(18);
            let stripe = if diagonal < 8 { 0.03 } else { 0.0 };
            let at = ((y * width + x) * 4) as usize;
            for c in 0..3 {
                let lit = ground[c] * (1.0 - stripe) + 255.0 * stripe;
                rgba[at + c] = (lit * (1.0 - a) + ground[c] * a).round().clamp(0.0, 255.0) as u8;
            }
            rgba[at + 3] = 255;
        }
    }
    image::Handle::from_rgba(width, height, rgba)
}

pub struct Scrub<'a, Message> {
    pub start: f32,
    pub len: f32,
    pub alpha: f32,
    pub on: Box<dyn Fn(f32) -> Message + 'a>,
}

#[derive(Debug, Default)]
pub struct ScrubState {
    grabbed: Option<f32>,
}

impl<Message> canvas::Program<Message> for Scrub<'_, Message> {
    type State = ScrubState;

    fn update(
        &self,
        state: &mut ScrubState,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let fraction_at = |x: f32| ((x - bounds.x) / bounds.width.max(1.0)).clamp(0.0, 1.0);
        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let at = cursor.position_in(bounds)?;
                let here = fraction_at(bounds.x + at.x);
                let grab = if here >= self.start && here <= self.start + self.len { here - self.start } else { self.len / 2.0 };
                state.grabbed = Some(grab);
                Some(canvas::Action::publish((self.on)((here - grab).clamp(0.0, 1.0 - self.len))).and_capture())
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let grab = state.grabbed?;
                let here = fraction_at(position.x);
                Some(canvas::Action::publish((self.on)((here - grab).clamp(0.0, 1.0 - self.len))).and_capture())
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.grabbed.take()?;
                Some(canvas::Action::capture())
            }
            _ => None,
        }
    }

    fn draw(&self, state: &ScrubState, renderer: &Renderer, _: &Theme, bounds: Rectangle, cursor: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let y = bounds.height / 2.0;
        let lit = state.grabbed.is_some() || cursor.is_over(bounds);
        let track = Path::line(Point::new(0.0, y), Point::new(bounds.width, y));
        frame.stroke(&track, Stroke::default().with_width(2.0).with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.06 * self.alpha)));
        if self.len < 1.0 {
            let x0 = bounds.width * self.start;
            let x1 = bounds.width * (self.start + self.len);
            let thumb = Path::line(Point::new(x0, y), Point::new(x1, y));
            let colour = if lit { MUTED } else { Color { a: 0.55, ..MUTED } };
            frame.stroke(&thumb, Stroke::default().with_width(2.0).with_color(Color { a: colour.a * self.alpha, ..colour }).with_line_cap(canvas::LineCap::Round));
        }
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(&self, state: &ScrubState, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if state.grabbed.is_some() {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}

pub struct Bar {
    pub fraction: f32,
    pub alpha: f32,
}

impl<Message> canvas::Program<Message> for Bar {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let width = bounds.width * self.fraction.clamp(0.0, 1.0);
        if width > 0.5 {
            let r = theme::CONTROL_RADIUS;
            let right = (width - (bounds.width - r)).clamp(0.0, r);
            let face = Path::rounded_rectangle(
                Point::ORIGIN,
                Size::new(width, bounds.height),
                iced::border::Radius { top_left: r, top_right: right, bottom_right: right, bottom_left: r },
            );
            frame.fill(&face, Color { a: 0.16 * self.alpha, ..ACCENT });
            let line = bottom_slice(bounds.size(), 1.0, r - 1.0, 2.0, width);
            frame.fill(&line, Color { a: ACCENT.a * self.alpha, ..ACCENT });
        }
        vec![frame.into_geometry()]
    }
}

pub fn progress<'a, Message: Clone + 'a>(words: String, fraction: f32, on: Option<Message>) -> Element<'a, Message> {
    let label = container(text(words).font(theme::SANS_SEMI).size(theme::BODY).color(faded(INK)))
        .center_x(Length::Fill)
        .center_y(Length::Fill);
    let bar = Canvas::new(Bar { fraction, alpha: fade() }).width(Length::Fill).height(Length::Fill);
    let face = iced::widget::stack![bar, label].width(theme::PROGRESS_WIDTH).height(theme::CONTROL_HEIGHT - 2.0);
    let made = button(face).padding(0).style(dimmed(theme::progressing, fade()));
    match on {
        Some(message) => made.on_press(message).into(),
        None => made.into(),
    }
}

pub struct Sensed<'a, Message> {
    content: Element<'a, Message>,
    on_enter: Box<dyn Fn(Rectangle) -> Message + 'a>,
    on_exit: Message,
    lift: f32,
    scale: f32,
    key: usize,
    hit_padding: iced::Padding,
    viewport_x: Option<f32>,
}

#[derive(Debug, Default)]
struct SensedState {
    inside: bool,
    key: usize,
    bounds: Option<Rectangle>,
}

pub fn sensed<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    on_enter: impl Fn(Rectangle) -> Message + 'a,
    on_exit: Message,
) -> Sensed<'a, Message> {
    Sensed { content: content.into(), on_enter: Box::new(on_enter), on_exit, lift: 0.0, scale: 1.0, key: 0, hit_padding: iced::Padding::ZERO, viewport_x: None }
}

impl<Message> Sensed<'_, Message> {
    pub fn keyed(mut self, key: usize) -> Self {
        self.key = key;
        self
    }

    pub fn risen(mut self, lift: f32, scale: f32) -> Self {
        self.lift = lift;
        self.scale = scale;
        self
    }

    pub fn hit_padding(mut self, padding: iced::Padding) -> Self {
        self.hit_padding = padding;
        self
    }

    pub fn horizontal_viewport(mut self, x: f32) -> Self {
        self.viewport_x = Some(x);
        self
    }
}

fn sensed_hit_bounds(bounds: Rectangle, padding: iced::Padding, viewport: Rectangle) -> Rectangle {
    Rectangle {
        x: bounds.x - padding.left,
        y: bounds.y - padding.top,
        width: bounds.width + padding.left + padding.right,
        height: bounds.height + padding.top + padding.bottom,
    }
    .intersection(&viewport)
    .unwrap_or(Rectangle::new(bounds.position(), Size::ZERO))
}

fn sensed_content_cursor(cursor: mouse::Cursor, bounds: Rectangle, hit_bounds: Rectangle) -> mouse::Cursor {
    match cursor.position().filter(|point| hit_bounds.contains(*point)) {
        Some(point) if !bounds.contains(point) => mouse::Cursor::Available(Point::new(
            point.x.clamp(bounds.x, bounds.x + (bounds.width - 0.01).max(0.0)),
            point.y.clamp(bounds.y, bounds.y + (bounds.height - 0.01).max(0.0)),
        )),
        _ => cursor,
    }
}

fn about(bounds: Rectangle, anchor: Point, lift: f32, scale: f32) -> iced::Transformation {
    let (cx, cy) = (bounds.x + bounds.width * anchor.x, bounds.y + bounds.height * anchor.y);
    iced::Transformation::translate(cx, cy - lift) * iced::Transformation::scale(scale) * iced::Transformation::translate(-cx, -cy)
}

fn sensed_changed(inside: bool, was_inside: bool, bounds_changed: bool, redraw: bool) -> bool {
    inside != was_inside || (inside && bounds_changed && redraw)
}

impl<Message: Clone> iced::advanced::Widget<Message, Theme, Renderer> for Sensed<'_, Message> {
    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        iced::advanced::widget::tree::Tag::of::<SensedState>()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        iced::advanced::widget::tree::State::new(SensedState::default())
    }

    fn children(&self) -> Vec<iced::advanced::widget::Tree> {
        vec![iced::advanced::widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut iced::advanced::widget::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        event: &iced::Event,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let seen = sensed_hit_bounds(bounds, self.hit_padding, *viewport);
        let content_cursor = sensed_content_cursor(cursor, bounds, seen);
        self.content
            .as_widget_mut()
            .update(&mut tree.children[0], event, layout, content_cursor, renderer, clipboard, shell, viewport);
        let state = tree.state.downcast_mut::<SensedState>();
        match event {
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) | iced::Event::Mouse(mouse::Event::CursorLeft) => {},
            iced::Event::Window(iced::window::Event::RedrawRequested(_)) => {}
            _ => return,
        }
        if state.key != self.key {
            state.key = self.key;
            state.inside = false;
            state.bounds = None;
        }
        let inside = !matches!(event, iced::Event::Mouse(mouse::Event::CursorLeft)) && cursor.is_over(seen);
        let shift = iced::Vector::new(self.viewport_x.map_or(0.0, |x| viewport.x - x), 0.0);
        let window_bounds = bounds - shift;
        let bounds_changed = state.bounds != Some(window_bounds);
        let redraw = matches!(event, iced::Event::Window(iced::window::Event::RedrawRequested(_)));
        if sensed_changed(inside, state.inside, bounds_changed, redraw) {
            state.inside = inside;
            state.bounds = Some(window_bounds);
            if inside {
                shell.publish((self.on_enter)(window_bounds));
            } else {
                shell.publish(self.on_exit.clone());
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let bounds = layout.bounds();
        let seen = sensed_hit_bounds(bounds, self.hit_padding, *viewport);
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, sensed_content_cursor(cursor, bounds, seen), viewport, renderer)
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let cursor = sensed_content_cursor(cursor, bounds, sensed_hit_bounds(bounds, self.hit_padding, *viewport));
        if self.lift == 0.0 && self.scale == 1.0 {
            self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
            return;
        }
        use iced::advanced::Renderer as _;
        let transformation = about(layout.bounds(), Point::new(0.5, 0.5), self.lift, self.scale);
        renderer.with_transformation(transformation, |renderer| {
            self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

impl<'a, Message: Clone + 'a> From<Sensed<'a, Message>> for Element<'a, Message> {
    fn from(sensed: Sensed<'a, Message>) -> Element<'a, Message> {
        Element::new(sensed)
    }
}

pub struct Grown<'a, Message> {
    content: Element<'a, Message>,
    anchor: Point,
    lift: f32,
    shift: f32,
    scale: f32,
}

pub fn grown<'a, Message: 'a>(content: impl Into<Element<'a, Message>>, anchor: Point, lift: f32, scale: f32) -> Grown<'a, Message> {
    Grown { content: content.into(), anchor, lift, shift: 0.0, scale }
}

impl<Message> Grown<'_, Message> {
    pub fn shifted(mut self, shift: f32) -> Self {
        self.shift = shift;
        self
    }
}

impl<Message> iced::advanced::Widget<Message, Theme, Renderer> for Grown<'_, Message> {
    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<iced::advanced::widget::Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut iced::advanced::widget::Tree) {
        self.content.as_widget().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        self.content.as_widget_mut().operate(tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        event: &iced::Event,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(tree, event, layout, cursor, renderer, clipboard, shell, viewport);
    }

    fn mouse_interaction(
        &self,
        tree: &iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        if self.lift == 0.0 && self.shift == 0.0 && (self.scale - 1.0).abs() < 0.001 {
            self.content.as_widget().draw(tree, renderer, theme, style, layout, cursor, viewport);
            return;
        }
        use iced::advanced::Renderer as _;
        let transformation = iced::Transformation::translate(self.shift, 0.0) * about(layout.bounds(), self.anchor, self.lift, self.scale);
        renderer.with_transformation(transformation, |renderer| {
            self.content.as_widget().draw(tree, renderer, theme, style, layout, cursor, viewport);
        });
    }
}

impl<'a, Message: 'a> From<Grown<'a, Message>> for Element<'a, Message> {
    fn from(grown: Grown<'a, Message>) -> Element<'a, Message> {
        Element::new(grown)
    }
}

pub struct Springy<'a, Message> {
    content: Element<'a, Message>,
    reach: f32,
}

#[derive(Debug, Default)]
struct SpringState {
    hover: f32,
    press: f32,
    over: bool,
    held: bool,
    last: Option<std::time::Instant>,
}

pub fn springy<'a, Message: 'a>(content: impl Into<Element<'a, Message>>, reach: f32) -> Springy<'a, Message> {
    Springy { content: content.into(), reach }
}

impl<Message> iced::advanced::Widget<Message, Theme, Renderer> for Springy<'_, Message> {
    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        iced::advanced::widget::tree::Tag::of::<SpringState>()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        iced::advanced::widget::tree::State::new(SpringState::default())
    }

    fn children(&self) -> Vec<iced::advanced::widget::Tree> {
        vec![iced::advanced::widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut iced::advanced::widget::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        event: &iced::Event,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);
        let state = tree.state.downcast_mut::<SpringState>();
        let over = cursor.is_over(layout.bounds());
        match event {
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) if over != state.over => {
                state.over = over;
                shell.request_redraw();
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if over => {
                state.held = true;
                shell.request_redraw();
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.held => {
                state.held = false;
                shell.request_redraw();
            }
            iced::Event::Window(iced::window::Event::RedrawRequested(now)) => {
                let (hover, press) = (if state.over { 1.0 } else { 0.0 }, if state.held { 1.0 } else { 0.0 });
                let settled = (state.hover - hover).abs() < 0.002 && (state.press - press).abs() < 0.002;
                if settled {
                    state.hover = hover;
                    state.press = press;
                    state.last = None;
                } else {
                    let dt = elapsed(&mut state.last, *now);
                    state.hover = toward(state.hover, hover, 0.28, dt);
                    state.press = toward(state.press, press, 0.4, dt);
                    shell.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<SpringState>();
        let scale = 1.0 + self.reach * state.hover - self.reach * 1.4 * state.press;
        if (scale - 1.0).abs() < 0.001 {
            self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
            return;
        }
        use iced::advanced::Renderer as _;
        renderer.with_transformation(about(layout.bounds(), Point::new(0.5, 0.5), 0.0, scale), |renderer| {
            self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
        });
    }
}

impl<'a, Message: 'a> From<Springy<'a, Message>> for Element<'a, Message> {
    fn from(springy: Springy<'a, Message>) -> Element<'a, Message> {
        Element::new(springy)
    }
}

pub fn trailing<'a, Message: 'a>(line: Element<'a, Message>, wide: f32, high: f32, under: Color) -> Element<'a, Message> {
    let k = fade();
    let veil = container(Space::new().width(FADE_TAIL).height(high)).style(move |_: &Theme| {
        let tail = iced::gradient::Linear::new(std::f32::consts::FRAC_PI_2)
            .add_stop(0.0, Color { a: 0.0, ..under })
            .add_stop(0.7, Color { a: under.a * 0.85 * k, ..under })
            .add_stop(1.0, Color { a: under.a * k, ..under });
        container::Style { background: Some(iced::Background::Gradient(tail.into())), ..container::Style::default() }
    });
    iced::widget::stack![
        container(line).width(wide).height(high).clip(true).align_y(iced::alignment::Vertical::Center),
        container(veil).width(wide).height(high).align_x(iced::alignment::Horizontal::Right)
    ]
    .width(wide)
    .height(high)
    .into()
}

pub fn scroll_fades<'a, Message: 'a>(content: Element<'a, Message>, width: f32, height: f32) -> Element<'a, Message> {
    iced::widget::stack![
        content,
        Canvas::new(ScrollEdgeFade { alpha: fade() }).width(width).height(height),
    ]
    .width(width)
    .height(height)
    .into()
}

struct ScrollEdgeFade {
    alpha: f32,
}

impl<Message> canvas::Program<Message> for ScrollEdgeFade {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let edge = 22.0_f32.min(bounds.height / 2.0);
        if edge > 0.0 {
            let bottom = canvas::gradient::Linear::new(Point::new(0.0, bounds.height - edge), Point::new(0.0, bounds.height))
                .add_stop(0.0, Color { a: 0.0, ..theme::GROUND })
                .add_stop(1.0, Color { a: 0.92 * self.alpha, ..theme::GROUND });
            frame.fill(&Path::rectangle(Point::new(0.0, bounds.height - edge), Size::new(bounds.width, edge)),
                canvas::Fill { style: canvas::Style::Gradient(bottom.into()), ..canvas::Fill::default() });
        }
        vec![frame.into_geometry()]
    }
}

pub struct SideFade {
    pub alpha: f32,
    pub left: f32,
    pub right: f32,
}

pub const SIDE_FADE: f32 = 64.0;

impl<Message> canvas::Program<Message> for SideFade {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let edge = SIDE_FADE.min(bounds.width / 3.0);
        let dark = |a: f32| Color { a: 0.94 * self.alpha * a, ..theme::GROUND };
        if self.left > 0.001 {
            let shade = canvas::gradient::Linear::new(Point::new(0.0, 0.0), Point::new(edge, 0.0)).add_stop(0.0, dark(self.left)).add_stop(1.0, dark(0.0));
            frame.fill(&Path::rectangle(Point::ORIGIN, Size::new(edge, bounds.height)), canvas::Fill { style: canvas::Style::Gradient(shade.into()), ..canvas::Fill::default() });
        }
        if self.right > 0.001 {
            let shade = canvas::gradient::Linear::new(Point::new(bounds.width - edge, 0.0), Point::new(bounds.width, 0.0)).add_stop(0.0, dark(0.0)).add_stop(1.0, dark(self.right));
            frame.fill(&Path::rectangle(Point::new(bounds.width - edge, 0.0), Size::new(edge, bounds.height)), canvas::Fill { style: canvas::Style::Gradient(shade.into()), ..canvas::Fill::default() });
        }
        vec![frame.into_geometry()]
    }
}

pub fn side_fades<'a, Message: 'a>(content: Element<'a, Message>, left: f32, right: f32) -> Element<'a, Message> {
    iced::widget::stack![content, Canvas::new(SideFade { alpha: fade(), left: left.clamp(0.0, 1.0), right: right.clamp(0.0, 1.0) }).width(Length::Fill).height(Length::Fill)].into()
}

const FADE_TAIL: f32 = 34.0;

pub fn shortened(words: String, at_most: usize) -> String {
    if words.chars().count() <= at_most {
        return words;
    }
    let mut cut: String = words.chars().take(at_most - 1).collect();
    while cut.ends_with(' ') {
        cut.pop();
    }
    cut.push('…');
    cut
}

pub struct Skin {
    pub at: f32,
    pub alpha: f32,
}

impl<Message> canvas::Program<Message> for Skin {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let style = theme::bubble(theme);
        let mut frame = Frame::new(renderer, bounds.size());
        let radius = style.border.radius.top_left;
        let card = Size::new(bounds.width, bounds.height - CARET_H);
        for (grow, alpha) in [(6.0, 0.05), (3.0, 0.08), (1.0, 0.12)] {
            let shade = Path::rounded_rectangle(Point::new(-grow, -grow + 4.0), Size::new(card.width + 2.0 * grow, card.height + 2.0 * grow), (radius + grow).into());
            frame.fill(&shade, Color::from_rgba(0.0, 0.0, 0.0, alpha * self.alpha));
        }
        let ground = match style.background {
            Some(iced::Background::Color(colour)) => colour,
            _ => Color::BLACK,
        };
        let body = Path::new(|b| {
            let r = radius;
            let (w, h) = (card.width, card.height);
            b.move_to(Point::new(r, 0.0));
            b.line_to(Point::new(w - r, 0.0));
            b.arc_to(Point::new(w, 0.0), Point::new(w, r), r);
            b.line_to(Point::new(w, h - r));
            b.arc_to(Point::new(w, h), Point::new(w - r, h), r);
            b.line_to(Point::new(self.at + CARET_H, h));
            b.line_to(Point::new(self.at, h + CARET_H));
            b.line_to(Point::new(self.at - CARET_H, h));
            b.line_to(Point::new(r, h));
            b.arc_to(Point::new(0.0, h), Point::new(0.0, h - r), r);
            b.line_to(Point::new(0.0, r));
            b.arc_to(Point::new(0.0, 0.0), Point::new(r, 0.0), r);
            b.close();
        });
        frame.fill(&body, Color { a: ground.a * self.alpha, ..ground });
        frame.stroke(&body, Stroke::default().with_width(1.0).with_color(Color { a: style.border.color.a * self.alpha, ..style.border.color }));
        vec![frame.into_geometry()]
    }
}

const CARET_H: f32 = 8.0;

fn bottom_slice(size: Size, edge: f32, radius: f32, thick: f32, until: f32) -> Path {
    let (w, h) = (size.width, size.height);
    let top = h - edge - thick;
    let centre_y = h - edge - radius;
    let left_at = |y: f32| {
        let dy = (y - centre_y).max(0.0).min(radius);
        edge + radius - (radius * radius - dy * dy).max(0.0).sqrt()
    };
    let right_at = |y: f32| (w - left_at(y)).min(until);
    let steps = 6;
    Path::new(|b| {
        b.move_to(Point::new(left_at(top), top));
        b.line_to(Point::new(right_at(top), top));
        for i in 1..=steps {
            let y = top + thick * i as f32 / steps as f32;
            b.line_to(Point::new(right_at(y), y));
        }
        for i in (0..steps).rev() {
            let y = top + thick * i as f32 / steps as f32;
            b.line_to(Point::new(left_at(y), y));
        }
        b.close();
    })
}

pub struct PlayMark;

impl<Message> canvas::Program<Message> for PlayMark {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let (w, h) = (bounds.width, bounds.height);
        let shade = Path::new(|b| {
            b.move_to(Point::new(w * 0.22, 0.0));
            b.line_to(Point::new(w * 1.02, h * 0.5));
            b.line_to(Point::new(w * 0.22, h));
            b.close();
        });
        frame.fill(&shade, faded(Color::from_rgba(0.0, 0.0, 0.0, 0.35)));
        let mark = Path::new(|b| {
            b.move_to(Point::new(w * 0.2, 0.0));
            b.line_to(Point::new(w, h * 0.5));
            b.line_to(Point::new(w * 0.2, h));
            b.close();
        });
        frame.fill(&mark, faded(Color { a: 0.92, ..INK }));
        vec![frame.into_geometry()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Play,
    Pause,
    Again,
    Back,
    Ahead,
    Earlier,
    Later,
    StepBack,
    StepAhead,
    Loud,
    Soft,
    Hushed,
    Over,
    Grow,
    Shrink,
    Mini,
    Close,
}

impl Control {
    fn drawing(self) -> &'static [u8] {
        match self {
            Control::Play => include_bytes!("../assets/player/play.svg"),
            Control::Pause => include_bytes!("../assets/player/pause.svg"),
            Control::Again => include_bytes!("../assets/player/rotate-ccw.svg"),
            Control::Back => include_bytes!("../assets/player/rewind.svg"),
            Control::Ahead => include_bytes!("../assets/player/fast-forward.svg"),
            Control::Earlier => include_bytes!("../assets/player/skip-back.svg"),
            Control::Later => include_bytes!("../assets/player/skip-forward.svg"),
            Control::StepBack => include_bytes!("../assets/player/step-back.svg"),
            Control::StepAhead => include_bytes!("../assets/player/step-forward.svg"),
            Control::Loud => include_bytes!("../assets/player/volume-2.svg"),
            Control::Soft => include_bytes!("../assets/player/volume-1.svg"),
            Control::Hushed => include_bytes!("../assets/player/volume-x.svg"),
            Control::Over => include_bytes!("../assets/player/repeat.svg"),
            Control::Grow => include_bytes!("../assets/player/maximize.svg"),
            Control::Shrink => include_bytes!("../assets/player/minimize.svg"),
            Control::Mini => include_bytes!("../assets/player/picture-in-picture.svg"),
            Control::Close => include_bytes!("../assets/player/x.svg"),
        }
    }
}

pub fn control<'a, Message: 'a>(kind: Control, side: f32, colour: Color) -> Element<'a, Message> {
    iced::widget::svg(drawn(kind.drawing()))
        .width(side)
        .height(side)
        .opacity(fade())
        .style(move |_: &Theme, _| iced::widget::svg::Style { color: Some(colour) })
        .into()
}

pub fn control_button<'a, Message: Clone + 'a>(kind: Control, side: f32, on: Option<Message>, lit: bool) -> Element<'a, Message> {
    let room = side + 14.0;
    let colour = match (&on, lit) {
        (None, _) => Color { a: 0.35, ..FAINT },
        (Some(_), true) => ACCENT,
        (Some(_), false) => INK,
    };
    let made = button(
        container(control(kind, side, colour))
            .width(room)
            .height(room)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
    )
        .padding(0)
        .style(dimmed(theme::glyph, fade()));
    match on {
        Some(message) => springy(made.on_press(message), 0.09).into(),
        None => made.into(),
    }
}

pub struct Seek<'a, Message> {
    pub played: f32,
    pub alpha: f32,
    pub at: Box<dyn Fn(f32) -> String + 'a>,
    pub on_move: Box<dyn Fn(f32) -> Message + 'a>,
    pub on_drop: Box<dyn Fn(f32) -> Message + 'a>,
}

#[derive(Debug, Default)]
pub struct SeekState {
    grabbed: bool,
    over: Option<f32>,
    knob: f32,
    last: Option<std::time::Instant>,
}

const SEEK_BUBBLE: f32 = 20.0;

impl<Message> canvas::Program<Message> for Seek<'_, Message> {
    type State = SeekState;

    fn update(&self, state: &mut SeekState, event: &iced::Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Message>> {
        let fraction_at = |x: f32| ((x - bounds.x - 7.0) / (bounds.width - 14.0).max(1.0)).clamp(0.0, 1.0);
        if let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            let want = if state.grabbed { 1.0 } else if cursor.is_over(bounds) { 0.6 } else { 0.0 };
            if (want - state.knob).abs() < 0.003 {
                state.knob = want;
                state.last = None;
                return None;
            }
            let dt = elapsed(&mut state.last, *now);
            state.knob = toward(state.knob, want, 0.3, dt);
            return Some(canvas::Action::request_redraw());
        }
        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let at = cursor.position_in(bounds)?;
                if at.y < SEEK_BUBBLE {
                    return None;
                }
                state.grabbed = true;
                state.over = Some(fraction_at(bounds.x + at.x));
                Some(canvas::Action::publish((self.on_move)(fraction_at(bounds.x + at.x))).and_capture())
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if state.grabbed {
                    state.over = Some(fraction_at(position.x));
                    return Some(canvas::Action::publish((self.on_move)(fraction_at(position.x))).and_capture());
                }
                let was = state.over;
                state.over = cursor.position_in(bounds).map(|at| fraction_at(bounds.x + at.x));
                if was.is_some() || state.over.is_some() {
                    Some(canvas::Action::request_redraw())
                } else {
                    None
                }
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.grabbed => {
                state.grabbed = false;
                let at = state.over.unwrap_or(self.played);
                Some(canvas::Action::publish((self.on_drop)(at)).and_capture())
            }
            _ => None,
        }
    }

    fn draw(&self, state: &SeekState, renderer: &Renderer, _: &Theme, bounds: Rectangle, cursor: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let y = SEEK_BUBBLE + (bounds.height - SEEK_BUBBLE) / 2.0;
        let lit = state.grabbed || cursor.is_over(bounds);
        let thick = 3.0 + 2.0 * (state.knob / 0.6).min(1.0);
        let x0 = 7.0;
        let x1 = (bounds.width - 7.0).max(x0);
        let track = Path::line(Point::new(x0, y), Point::new(x1, y));
        frame.stroke(
            &track,
            Stroke::default().with_width(thick).with_color(dim(Color::from_rgba(1.0, 1.0, 1.0, 0.16), self.alpha)).with_line_cap(canvas::LineCap::Round),
        );
        let x = x0 + (x1 - x0) * self.played.clamp(0.0, 1.0);
        if let Some(over) = state.over.filter(|_| lit) {
            let ahead = x0 + (x1 - x0) * over;
            if ahead > x {
                frame.stroke(
                    &Path::line(Point::new(x, y), Point::new(ahead, y)),
                    Stroke::default().with_width(thick).with_color(dim(Color { a: 0.3, ..INK }, self.alpha)).with_line_cap(canvas::LineCap::Round),
                );
            }
        }
        if x > x0 + 0.5 {
            let done = Path::line(Point::new(x0, y), Point::new(x, y));
            frame.stroke(&done, Stroke::default().with_width(thick).with_color(dim(ACCENT, self.alpha)).with_line_cap(canvas::LineCap::Round));
        }
        let radius = 4.5 + 2.5 * state.knob;
        frame.fill(&Path::circle(Point::new(x, y), radius), dim(INK, self.alpha));
        if let Some(over) = state.over.filter(|_| lit) {
            let words = (self.at)(over);
            let wide = words.chars().count() as f32 * 6.7 + 14.0;
            let mid = (bounds.width * over).clamp(wide / 2.0, bounds.width - wide / 2.0);
            let box_at = Rectangle { x: mid - wide / 2.0, y: 0.0, width: wide, height: SEEK_BUBBLE - 6.0 };
            frame.fill(
                &Path::rounded_rectangle(Point::new(box_at.x, box_at.y), box_at.size(), 6.0.into()),
                dim(Color::from_rgba(0.047, 0.02, 0.027, 0.96), self.alpha),
            );
            frame.fill_text(canvas::Text {
                content: words,
                position: Point::new(mid, box_at.y + box_at.height / 2.0),
                color: dim(INK, self.alpha),
                size: 11.0.into(),
                font: theme::MONO,
                align_x: iced::alignment::Horizontal::Center.into(),
                align_y: iced::alignment::Vertical::Center,
                ..canvas::Text::default()
            });
        }
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(&self, state: &SeekState, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if state.grabbed || cursor.is_over(bounds) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

pub struct Speed<'a, Message> {
    pub at: f32,
    pub stops: Vec<f32>,
    pub words: String,
    pub alpha: f32,
    pub on: Box<dyn Fn(f32) -> Message + 'a>,
}

#[derive(Debug, Default)]
pub struct SpeedState {
    shown: Option<f32>,
    grabbed: Option<(f32, f32)>,
    moved: bool,
    glow: f32,
    last: Option<std::time::Instant>,
}

const SPEED_TEXT: f32 = 50.0;
const SPEED_STEP: f32 = 15.0;

impl<Message> Speed<'_, Message> {
    fn index(&self) -> f32 {
        self.stops.iter().position(|stop| (stop - self.at).abs() < 0.001).unwrap_or(0) as f32
    }

    fn stop(&self, index: f32) -> f32 {
        let last = self.stops.len().saturating_sub(1) as f32;
        self.stops.get(index.round().clamp(0.0, last) as usize).copied().unwrap_or(self.at)
    }
}

impl<Message> canvas::Program<Message> for Speed<'_, Message> {
    type State = SpeedState;

    fn update(&self, state: &mut SpeedState, event: &iced::Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Message>> {
        match event {
            iced::Event::Window(iced::window::Event::RedrawRequested(at)) => {
                let want = self.index();
                let now = state.shown.unwrap_or(want);
                let lit = if state.grabbed.is_some() { 1.0 } else if cursor.is_over(bounds) { 0.6 } else { 0.0 };
                let still = (want - now).abs() < 0.004 && (lit - state.glow).abs() < 0.004;
                if still {
                    state.shown = Some(want);
                    state.glow = lit;
                    state.last = None;
                } else {
                    let dt = elapsed(&mut state.last, *at);
                    state.shown = Some(toward(now, want, 0.26, dt));
                    state.glow = toward(state.glow, lit, 0.3, dt);
                }
                (!still).then(canvas::Action::request_redraw)
            }
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let at = cursor.position_in(bounds)?;
                state.grabbed = Some((at.x, self.index()));
                state.moved = false;
                Some(canvas::Action::capture())
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let (from_x, from) = state.grabbed?;
                let travel = position.x - bounds.x - from_x;
                if travel.abs() > 3.0 {
                    state.moved = true;
                }
                let next = self.stop(from - travel / SPEED_STEP);
                if (next - self.at).abs() > 0.001 {
                    return Some(canvas::Action::publish((self.on)(next)).and_capture());
                }
                Some(canvas::Action::capture())
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let (from_x, from) = state.grabbed.take()?;
                if state.moved {
                    return Some(canvas::Action::capture());
                }
                let middle = SPEED_TEXT + (bounds.width - SPEED_TEXT - 6.0) / 2.0;
                let next = self.stop(if from_x < middle { from - 1.0 } else { from + 1.0 });
                Some(canvas::Action::publish((self.on)(next)).and_capture())
            }
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) if cursor.is_over(bounds) => {
                let up = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 30.0,
                };
                if up.abs() < 0.5 {
                    return Some(canvas::Action::capture());
                }
                let next = self.stop(self.index() + up.signum());
                Some(canvas::Action::publish((self.on)(next)).and_capture())
            }
            _ => None,
        }
    }

    fn draw(&self, state: &SpeedState, renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let (w, h) = (bounds.width, bounds.height);
        let glow = state.glow;
        frame.fill_text(canvas::Text {
            content: self.words.clone(),
            position: Point::new(12.0, h / 2.0),
            color: dim(mix(MUTED, INK, 0.5 + 0.5 * glow), self.alpha),
            size: 11.0.into(),
            font: theme::MONO,
            align_x: iced::alignment::Horizontal::Left.into(),
            align_y: iced::alignment::Vertical::Center,
            ..canvas::Text::default()
        });
        let (left, right) = (SPEED_TEXT, w - 6.0);
        let middle = (left + right) / 2.0;
        let reach = (right - left) / 2.0;
        let here = state.shown.unwrap_or_else(|| self.index());
        let last = self.stops.len().saturating_sub(1) as i32;
        for half in 0..=(last * 2) {
            let at = half as f32 / 2.0;
            let x = middle + (at - here) * SPEED_STEP;
            let away = ((x - middle).abs() / reach).clamp(0.0, 1.0);
            if away >= 1.0 {
                continue;
            }
            let whole = half % 2 == 0;
            let tall = if whole { 8.0 } else { 4.0 };
            let shade = Color::from_rgba(1.0, 1.0, 1.0, (if whole { 0.6 } else { 0.32 }) * (1.0 - away * away));
            frame.stroke(
                &Path::line(Point::new(x, h / 2.0 - tall / 2.0), Point::new(x, h / 2.0 + tall / 2.0)),
                Stroke::default().with_width(1.2).with_color(dim(shade, self.alpha)).with_line_cap(canvas::LineCap::Round),
            );
        }
        frame.stroke(
            &Path::line(Point::new(middle, h / 2.0 - 7.0), Point::new(middle, h / 2.0 + 7.0)),
            Stroke::default().with_width(2.0).with_color(dim(ACCENT, self.alpha)).with_line_cap(canvas::LineCap::Round),
        );
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(&self, state: &SpeedState, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        match (state.grabbed.is_some() && state.moved, cursor.is_over(bounds)) {
            (true, _) => mouse::Interaction::Grabbing,
            (false, true) => mouse::Interaction::Pointer,
            _ => mouse::Interaction::default(),
        }
    }
}


pub struct Level<'a, Message> {
    pub at: f32,
    pub alpha: f32,
    pub on: Box<dyn Fn(f32) -> Message + 'a>,
}

#[derive(Debug, Default)]
pub struct LevelState {
    grabbed: bool,
    knob: f32,
    last: Option<std::time::Instant>,
}

impl<Message> canvas::Program<Message> for Level<'_, Message> {
    type State = LevelState;

    fn update(&self, state: &mut LevelState, event: &iced::Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Message>> {
        let fraction_at = |x: f32| ((x - bounds.x - 6.0) / (bounds.width - 12.0).max(1.0)).clamp(0.0, 1.0);
        if let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            let want = if state.grabbed { 1.0 } else if cursor.is_over(bounds) { 0.6 } else { 0.0 };
            if (want - state.knob).abs() < 0.003 {
                state.knob = want;
                state.last = None;
                return None;
            }
            let dt = elapsed(&mut state.last, *now);
            state.knob = toward(state.knob, want, 0.3, dt);
            return Some(canvas::Action::request_redraw());
        }
        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let at = cursor.position_in(bounds)?;
                state.grabbed = true;
                Some(canvas::Action::publish((self.on)(fraction_at(bounds.x + at.x))).and_capture())
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) if state.grabbed => {
                Some(canvas::Action::publish((self.on)(fraction_at(position.x))).and_capture())
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.grabbed => {
                state.grabbed = false;
                Some(canvas::Action::capture())
            }
            _ => None,
        }
    }

    fn draw(&self, state: &LevelState, renderer: &Renderer, _: &Theme, bounds: Rectangle, cursor: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let y = bounds.height / 2.0;
        let lit = state.grabbed || cursor.is_over(bounds);
        let x0 = 6.0;
        let x1 = (bounds.width - 6.0).max(x0);
        frame.stroke(
            &Path::line(Point::new(x0, y), Point::new(x1, y)),
            Stroke::default().with_width(3.0).with_color(dim(Color::from_rgba(1.0, 1.0, 1.0, 0.16), self.alpha)).with_line_cap(canvas::LineCap::Round),
        );
        let x = x0 + (x1 - x0) * self.at.clamp(0.0, 1.0);
        if x > x0 + 0.5 {
            frame.stroke(
                &Path::line(Point::new(x0, y), Point::new(x, y)),
                Stroke::default().with_width(3.0).with_color(dim(if lit { INK } else { MUTED }, self.alpha)).with_line_cap(canvas::LineCap::Round),
            );
        }
        frame.fill(&Path::circle(Point::new(x, y), 4.0 + 2.0 * state.knob), dim(INK, self.alpha));
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(&self, state: &LevelState, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if state.grabbed || cursor.is_over(bounds) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

pub struct Halo {
    pub alpha: f32,
}

impl<Message> canvas::Program<Message> for Halo {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let side = bounds.width.min(bounds.height);
        let centre = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        frame.fill(&Path::circle(centre, side / 2.0), dim(Color::from_rgba(0.04, 0.02, 0.03, 0.55), self.alpha));
        frame.stroke(
            &Path::circle(centre, side / 2.0 - 0.75),
            Stroke::default().with_width(1.5).with_color(dim(Color::from_rgba(1.0, 1.0, 1.0, 0.18), self.alpha)),
        );
        vec![frame.into_geometry()]
    }
}

pub fn halo<'a, Message: 'a>(kind: Control, side: f32, k: f32) -> Element<'a, Message> {
    let alpha = fade() * k;
    iced::widget::stack![
        Canvas::new(Halo { alpha }).width(side).height(side),
        container(
            iced::widget::svg(drawn(kind.drawing()))
                .width(side * 0.42)
                .height(side * 0.42)
                .opacity(alpha)
                .style(move |_: &Theme, _| iced::widget::svg::Style { color: Some(Color { a: 0.95, ..INK }) })
        )
        .width(side)
        .height(side)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
    ]
    .width(side)
    .height(side)
    .into()
}

pub struct Disc {
    pub letter: String,
    pub hatched: bool,
    pub alpha: f32,
}

pub fn disc<'a, Message: 'a>(letter: &str, hatched: bool, side: f32) -> Element<'a, Message> {
    Canvas::new(Disc { letter: letter.to_owned(), hatched, alpha: fade() })
        .width(side)
        .height(side)
        .into()
}

impl Disc {
    fn dimmed(&self, colour: Color) -> Color {
        Color { a: colour.a * self.alpha.clamp(0.0, 1.0), ..colour }
    }
}

impl<Message> canvas::Program<Message> for Disc {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let side = bounds.width.min(bounds.height);
        let centre = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let disc = Path::circle(centre, side / 2.0);
        if self.hatched {
            frame.fill(&disc, self.dimmed(Color::from_rgba(0.08, 0.035, 0.047, 0.9)));
            for step in 0..((side * 3.0 / 5.0) as i32) {
                let x = -side + step as f32 * 5.0;
                let a = Point::new(x, side);
                let b = Point::new(x + side, 0.0);
                if let Some((p, q)) = clip_to_circle(a, b, centre, side / 2.0 - 1.0) {
                    frame.stroke(&Path::line(p, q), Stroke::default().with_width(1.5).with_color(self.dimmed(Color { a: 0.55, ..MUTED })));
                }
            }
            frame.stroke(&disc, Stroke::default().with_width(1.0).with_color(self.dimmed(Color::from_rgba(1.0, 1.0, 1.0, 0.1))));
        } else {
            frame.fill(&disc, self.dimmed(ACCENT));
            frame.stroke(&disc, Stroke::default().with_width(1.0).with_color(self.dimmed(Color::from_rgba(1.0, 1.0, 1.0, 0.12))));
            if !self.letter.is_empty() {
                frame.fill_text(canvas::Text {
                    content: self.letter.clone(),
                    position: centre,
                    color: self.dimmed(INK),
                    size: (side * 0.5).into(),
                    font: theme::SANS_SEMI,
                    align_x: iced::alignment::Horizontal::Center.into(),
                    align_y: iced::alignment::Vertical::Center,
                    ..canvas::Text::default()
                });
            }
        }
        vec![frame.into_geometry()]
    }
}

fn clip_to_circle(a: Point, b: Point, centre: Point, radius: f32) -> Option<(Point, Point)> {
    let d = Point::new(b.x - a.x, b.y - a.y);
    let f = Point::new(a.x - centre.x, a.y - centre.y);
    let qa = d.x * d.x + d.y * d.y;
    let qb = 2.0 * (f.x * d.x + f.y * d.y);
    let qc = f.x * f.x + f.y * f.y - radius * radius;
    let disc = qb * qb - 4.0 * qa * qc;
    if disc <= 0.0 || qa == 0.0 {
        return None;
    }
    let root = disc.sqrt();
    let t0 = ((-qb - root) / (2.0 * qa)).max(0.0);
    let t1 = ((-qb + root) / (2.0 * qa)).min(1.0);
    if t1 <= t0 {
        return None;
    }
    Some((Point::new(a.x + d.x * t0, a.y + d.y * t0), Point::new(a.x + d.x * t1, a.y + d.y * t1)))
}

pub struct Ring {
    pub alpha: f32,
}

pub fn ring<'a, Message: 'a>(k: f32, side: f32) -> Element<'a, Message> {
    Canvas::new(Ring { alpha: k * fade() }).width(side).height(side).into()
}

impl<Message> canvas::Program<Message> for Ring {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        if self.alpha > 0.01 {
            let side = bounds.width.min(bounds.height);
            let centre = Point::new(bounds.width / 2.0, bounds.height / 2.0);
            let ring = Path::circle(centre, side / 2.0 - 1.0);
            frame.stroke(&ring, Stroke::default().with_width(2.0).with_color(faded(Color { a: 0.55 * self.alpha, ..ACCENT })));
        }
        vec![frame.into_geometry()]
    }
}

pub struct Dot {
    pub alpha: f32,
}

pub fn dot<'a, Message: 'a>(side: f32) -> Element<'a, Message> {
    Canvas::new(Dot { alpha: fade() }).width(side).height(side).into()
}

impl<Message> canvas::Program<Message> for Dot {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let centre = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        frame.fill(&Path::circle(centre, bounds.width / 2.0 + 3.0), dim(Color { a: 0.16, ..ACCENT }, self.alpha));
        frame.fill(&Path::circle(centre, bounds.width / 2.0 - 1.0), dim(ACCENT, self.alpha));
        vec![frame.into_geometry()]
    }
}

pub struct Thread {
    pub fraction: f32,
    pub alpha: f32,
}

pub fn thread<'a, Message: 'a>(fraction: f32) -> Element<'a, Message> {
    Canvas::new(Thread { fraction, alpha: fade() }).width(Length::Fill).height(3.0).into()
}

impl<Message> canvas::Program<Message> for Thread {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let y = bounds.height / 2.0;
        let cap = canvas::LineCap::Round;
        frame.stroke(&Path::line(Point::new(1.0, y), Point::new(bounds.width - 1.0, y)), Stroke::default().with_width(2.0).with_line_cap(cap).with_color(dim(Color::from_rgba(1.0, 1.0, 1.0, 0.08), self.alpha)));
        let x = (bounds.width * self.fraction.clamp(0.0, 1.0)).max(1.0);
        if x > 1.5 {
            frame.stroke(&Path::line(Point::new(1.0, y), Point::new(x, y)), Stroke::default().with_width(2.0).with_line_cap(cap).with_color(dim(ACCENT, self.alpha)));
        }
        vec![frame.into_geometry()]
    }
}

pub struct Cross {
    pub colour: Color,
}

impl<Message> canvas::Program<Message> for Cross {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let c = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let r = bounds.width.min(bounds.height) * 0.19;
        let stroke = Stroke::default().with_width(1.6).with_color(faded(self.colour)).with_line_cap(canvas::LineCap::Round);
        frame.stroke(&Path::line(Point::new(c.x - r, c.y - r), Point::new(c.x + r, c.y + r)), stroke);
        frame.stroke(&Path::line(Point::new(c.x + r, c.y - r), Point::new(c.x - r, c.y + r)), stroke);
        vec![frame.into_geometry()]
    }
}

pub struct Steps<'a, Message> {
    pub label: String,
    pub value: String,
    pub at: f32,
    pub shown: f32,
    pub snap: f32,
    pub alpha: f32,
    pub stops: Vec<f32>,
    pub on: Box<dyn Fn(f32) -> Message + 'a>,
    pub release: Option<Box<dyn Fn() -> Message + 'a>>,
}

#[derive(Debug, Default)]
pub struct StepsState {
    grabbed: bool,
    initialized: bool,
    label_side: f32,
    value_side: f32,
    last: Option<std::time::Instant>,
}

const STEP_INSET: f32 = 5.0;
const HANDLE_ROOM: f32 = 6.0;
const STEP_SNAP: f32 = 0.035;

const STOP_CLEARANCE: f32 = 4.0;
const STOP_FADE: f32 = 6.0;

impl<Message> canvas::Program<Message> for Steps<'_, Message> {
    type State = StepsState;

    fn update(&self, state: &mut StepsState, event: &iced::Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Message>> {
        if let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            let (label_wide, value_wide) = (self.label.chars().count() as f32 * 6.8 + 4.0, self.value.chars().count() as f32 * 6.7 + 2.0);
            let x = STEP_INSET + self.shown.clamp(0.0, 1.0) * (bounds.width - 2.0 * STEP_INSET);
            let free_left = x - HANDLE_ROOM - 10.0;
            let want_label = match state.label_side > 0.5 {
                true => (free_left > label_wide + 22.0).then_some(0.0).unwrap_or(1.0),
                false => (free_left < label_wide + 4.0).then_some(1.0).unwrap_or(0.0),
            };
            let free_right = bounds.width - 8.0 - (x + HANDLE_ROOM + 10.0);
            let want_value = match state.value_side > 0.5 {
                true => (free_right > value_wide + 22.0).then_some(0.0).unwrap_or(1.0),
                false => (free_right < value_wide + 4.0).then_some(1.0).unwrap_or(0.0),
            };
            if !state.initialized {
                state.label_side = want_label;
                state.value_side = want_value;
                state.initialized = true;
                return None;
            }
            let mut moving = false;
            let dt = elapsed(&mut state.last, *now);
            for (side, want) in [(&mut state.label_side, want_label), (&mut state.value_side, want_value)] {
                if (want - *side).abs() > 0.002 {
                    *side = toward(*side, want, 0.3, dt);
                    moving = true;
                } else {
                    *side = want;
                }
            }
            if !moving {
                state.last = None;
            }
            return moving.then(canvas::Action::request_redraw);
        }
        let fraction_at = |x: f32| ((x - bounds.x - STEP_INSET) / (bounds.width - 2.0 * STEP_INSET).max(1.0)).clamp(0.0, 1.0);
        let snapped = |f: f32| {
            let mut best = f;
            let mut near = STEP_SNAP;
            for stop in [0.0, 1.0].into_iter().chain(self.stops.iter().copied()) {
                if (stop - f).abs() < near {
                    near = (stop - f).abs();
                    best = stop;
                }
            }
            best
        };
        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let at = cursor.position_in(bounds)?;
                state.grabbed = true;
                let f = snapped(fraction_at(bounds.x + at.x));
                Some(canvas::Action::publish((self.on)(f)).and_capture())
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) if state.grabbed => {
                let f = snapped(fraction_at(position.x));
                Some(canvas::Action::publish((self.on)(f)).and_capture())
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.grabbed => {
                state.grabbed = false;
                match &self.release {
                    Some(release) => Some(canvas::Action::publish(release()).and_capture()),
                    None => Some(canvas::Action::capture()),
                }
            }
            _ => None,
        }
    }

    fn draw(&self, state: &StepsState, renderer: &Renderer, _: &Theme, bounds: Rectangle, cursor: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let (w, h) = (bounds.width, bounds.height);
        let at = self.shown.clamp(0.0, 1.0);
        let x = STEP_INSET + at * (w - 2.0 * STEP_INSET);
        let lit = state.grabbed || cursor.is_over(bounds);
        frame.fill(
            &Path::rounded_rectangle(Point::ORIGIN, Size::new(w, h), 7.0.into()),
            dim(Color::from_rgba(1.0, 1.0, 1.0, if lit { 0.055 } else { 0.04 }), self.alpha),
        );
        if at > 0.004 && x > STEP_INSET + 2.0 {
            frame.fill(
                &Path::rounded_rectangle(Point::ORIGIN, Size::new(x, h), 7.0.into()),
                dim(Color::from_rgba(0.886, 0.282, 0.282, if lit { 0.2 } else { 0.15 }), self.alpha),
            );
        }
        let on_stop = if state.grabbed { 0.0 } else { self.snap.clamp(0.0, 1.0) };
        let inset = 4.0 - 3.0 * on_stop;
        let wide = 8.0 + on_stop + if state.grabbed { 2.5 } else { 0.0 };
        let label_wide = self.label.chars().count() as f32 * 6.8 + 4.0;
        let label_x = 10.0 + (w - 20.0 - label_wide) * state.label_side;
        let value_wide = self.value.chars().count() as f32 * 6.7 + 2.0;
        let right_side = x + wide / 2.0 + 10.0;
        let left_side = x - wide / 2.0 - 10.0 - value_wide;
        let value_x = (right_side + (left_side - right_side) * state.value_side)
            .clamp(10.0, (w - value_wide - 10.0).max(10.0));
        let clear_of = |sx: f32, from: f32, span: f32| ((from - STOP_CLEARANCE - sx).max(sx - from - span - STOP_CLEARANCE) / STOP_FADE).clamp(0.0, 1.0);
        for stop in &self.stops {
            let sx = STEP_INSET + stop * (w - 2.0 * STEP_INSET);
            let near = 1.0 - ((sx - x).abs() / 26.0).clamp(0.0, 1.0);
            let dot = 5.0;
            let width = dot + (wide - dot) * near;
            let height = dot + (h - 2.0 * inset - dot) * near;
            let alpha = (0.2 - 0.14 * near) * (1.0 - near * 0.5) * clear_of(sx, label_x, label_wide) * clear_of(sx, value_x, value_wide);
            if alpha > 0.004 {
                frame.fill(
                    &Path::rounded_rectangle(Point::new(sx - width / 2.0, (h - height) / 2.0), Size::new(width, height), (width / 2.0).into()),
                    dim(Color::from_rgba(1.0, 1.0, 1.0, alpha), self.alpha),
                );
            }
        }
        frame.fill(
            &Path::rounded_rectangle(Point::new(x - wide / 2.0, inset), Size::new(wide, h - 2.0 * inset), (wide / 2.0).into()),
            dim(Color { a: 0.92, ..INK }, self.alpha),
        );
        frame.fill_text(canvas::Text {
            content: self.label.clone(),
            position: Point::new(label_x, h / 2.0),
            color: dim(INK, self.alpha),
            size: theme::CAPTION.into(),
            font: theme::SANS_SEMI,
            align_x: iced::alignment::Horizontal::Left.into(),
            align_y: iced::alignment::Vertical::Center,
            ..canvas::Text::default()
        });
        frame.fill_text(canvas::Text {
            content: self.value.clone(),
            position: Point::new(value_x, h / 2.0),
            color: dim(INK, self.alpha),
            size: 11.0.into(),
            font: theme::MONO,
            align_x: iced::alignment::Horizontal::Left.into(),
            align_y: iced::alignment::Vertical::Center,
            ..canvas::Text::default()
        });
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(&self, state: &StepsState, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if state.grabbed || cursor.is_over(bounds) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

pub fn steps<'a, Message: 'a>(
    label: String,
    value: String,
    at: f32,
    shown: f32,
    snap: f32,
    stops: Vec<f32>,
    on: impl Fn(f32) -> Message + 'a,
) -> Element<'a, Message> {
    Canvas::new(Steps { label, value, at, shown, snap, alpha: fade(), stops, on: Box::new(on), release: None })
        .width(Length::Fill)
        .height(26.0)
        .into()
}

pub fn steps_released<'a, Message: 'a>(
    label: String,
    value: String,
    at: f32,
    shown: f32,
    snap: f32,
    on: impl Fn(f32) -> Message + 'a,
    release: impl Fn() -> Message + 'a,
) -> Element<'a, Message> {
    Canvas::new(Steps { label, value, at, shown, snap, alpha: fade(), stops: Vec::new(), on: Box::new(on), release: Some(Box::new(release)) })
        .width(Length::Fill)
        .height(26.0)
        .into()
}

pub struct Mark {
    pub glyph: String,
    pub on: bool,
    pub k: f32,
    pub alpha: f32,
}

impl<Message> canvas::Program<Message> for Mark {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let side = bounds.width.min(bounds.height);
        let centre = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let k = self.k.clamp(0.0, 1.0);
        let ground = Color::from_rgba(1.0, 1.0, 1.0, 0.1);
        let colour = Color {
            r: ground.r + (ACCENT.r - ground.r) * k,
            g: ground.g + (ACCENT.g - ground.g) * k,
            b: ground.b + (ACCENT.b - ground.b) * k,
            a: ground.a + (ACCENT.a - ground.a) * k,
        };
        frame.fill(&Path::circle(centre, side / 2.0), dim(colour, self.alpha));
        if self.glyph.is_empty() {
            let r = side * 0.3;
            let stroke = |width: f32, colour: Color| {
                Stroke::default()
                    .with_width(width)
                    .with_color(dim(colour, self.alpha))
                    .with_line_cap(canvas::LineCap::Round)
                    .with_line_join(canvas::LineJoin::Round)
            };
            if k > 0.02 {
                let start = Point::new(centre.x - r * 0.92, centre.y + r * 0.06);
                let elbow = Point::new(centre.x - r * 0.24, centre.y + r * 0.72);
                let end = Point::new(centre.x + r * 0.92, centre.y - r * 0.66);
                let first = (k / 0.4).clamp(0.0, 1.0);
                let second = ((k - 0.4) / 0.6).clamp(0.0, 1.0);
                let a = Point::new(start.x + (elbow.x - start.x) * first, start.y + (elbow.y - start.y) * first);
                let tick = Path::new(|b| {
                    b.move_to(start);
                    b.line_to(a);
                    if second > 0.0 {
                        b.line_to(Point::new(elbow.x + (end.x - elbow.x) * second, elbow.y + (end.y - elbow.y) * second));
                    }
                });
                frame.stroke(&tick, stroke(side * 0.1, dim(Color::WHITE, self.alpha)));
            }
            if k < 0.98 {
                let gone = 1.0 - k;
                let arm = r * 0.62 * gone;
                let faintly = Color { a: 0.6 * gone, ..MUTED };
                let cross = Path::new(|b| {
                    b.move_to(Point::new(centre.x - arm, centre.y - arm));
                    b.line_to(Point::new(centre.x + arm, centre.y + arm));
                    b.move_to(Point::new(centre.x + arm, centre.y - arm));
                    b.line_to(Point::new(centre.x - arm, centre.y + arm));
                });
                frame.stroke(&cross, stroke(side * 0.075, dim(faintly, self.alpha)));
            }
        } else {
            frame.fill_text(canvas::Text {
                content: self.glyph.clone(),
                position: centre,
                color: dim(if k > 0.5 { Color::WHITE } else { INK }, self.alpha),
                size: (if self.glyph.chars().count() > 2 { side * 0.34 } else { side * 0.4 }).into(),
                font: theme::MONO_BOLD,
                align_x: iced::alignment::Horizontal::Center.into(),
                align_y: iced::alignment::Vertical::Center,
                ..canvas::Text::default()
            });
        }
        vec![frame.into_geometry()]
    }
}

pub fn mark<'a, Message: 'a>(glyph: &str, on: bool, k: f32, side: f32) -> Element<'a, Message> {
    Canvas::new(Mark { glyph: glyph.to_owned(), on, k, alpha: fade() }).width(side).height(side).into()
}

pub struct Wrap<'a, Message> {
    children: Vec<Element<'a, Message>>,
    spacing: f32,
}

pub fn wrap<'a, Message: 'a>(children: Vec<Element<'a, Message>>, spacing: f32) -> Wrap<'a, Message> {
    Wrap { children, spacing }
}

impl<Message> iced::advanced::Widget<Message, Theme, Renderer> for Wrap<'_, Message> {
    fn children(&self) -> Vec<iced::advanced::widget::Tree> {
        self.children.iter().map(iced::advanced::widget::Tree::new).collect()
    }

    fn diff(&self, tree: &mut iced::advanced::widget::Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size { width: Length::Fill, height: Length::Shrink }
    }

    fn layout(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        let max = limits.max();
        let child_limits = iced::advanced::layout::Limits::new(Size::ZERO, Size::new(max.width, f32::INFINITY));
        let mut nodes = Vec::with_capacity(self.children.len());
        let (mut x, mut y, mut row_h, mut widest) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        for (child, state) in self.children.iter_mut().zip(tree.children.iter_mut()) {
            let node = child.as_widget_mut().layout(state, renderer, &child_limits);
            let size = node.size();
            if x > 0.0 && x + size.width > max.width {
                x = 0.0;
                y += row_h + self.spacing;
                row_h = 0.0;
            }
            nodes.push(node.move_to(Point::new(x, y)));
            x += size.width + self.spacing;
            row_h = row_h.max(size.height);
            widest = widest.max(x - self.spacing);
        }
        let height = if nodes.is_empty() { 0.0 } else { y + row_h };
        iced::advanced::layout::Node::with_children(Size::new(max.width.min(widest.max(max.width)), height), nodes)
    }

    fn operate(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        for ((child, state), place) in self.children.iter_mut().zip(tree.children.iter_mut()).zip(layout.children()) {
            child.as_widget_mut().operate(state, place, renderer, operation);
        }
    }

    fn update(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        event: &iced::Event,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        for ((child, state), place) in self.children.iter_mut().zip(tree.children.iter_mut()).zip(layout.children()) {
            child.as_widget_mut().update(state, event, place, cursor, renderer, clipboard, shell, viewport);
        }
    }

    fn mouse_interaction(
        &self,
        tree: &iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.children
            .iter()
            .zip(tree.children.iter())
            .zip(layout.children())
            .map(|((child, state), place)| child.as_widget().mouse_interaction(state, place, cursor, viewport, renderer))
            .max()
            .unwrap_or_default()
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        for ((child, state), place) in self.children.iter().zip(tree.children.iter()).zip(layout.children()) {
            child.as_widget().draw(state, renderer, theme, style, place, cursor, viewport);
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        iced::advanced::overlay::from_children(&mut self.children, tree, layout, renderer, viewport, translation)
    }
}

impl<'a, Message: 'a> From<Wrap<'a, Message>> for Element<'a, Message> {
    fn from(wrap: Wrap<'a, Message>) -> Element<'a, Message> {
        Element::new(wrap)
    }
}

pub struct Drifting<'a, Message> {
    content: Element<'a, Message>,
    since: std::time::Instant,
}

pub fn drifting<'a, Message: 'a>(content: impl Into<Element<'a, Message>>, since: std::time::Instant) -> Drifting<'a, Message> {
    Drifting { content: content.into(), since }
}

impl<Message> iced::advanced::Widget<Message, Theme, Renderer> for Drifting<'_, Message> {
    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<iced::advanced::widget::Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut iced::advanced::widget::Tree) {
        self.content.as_widget().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        Size { width: Length::Fill, height: self.content.as_widget().size().height }
    }

    fn layout(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        let loose = iced::advanced::layout::Limits::new(Size::ZERO, Size::new(f32::INFINITY, limits.max().height));
        let inner = self.content.as_widget_mut().layout(tree, renderer, &loose);
        let size = inner.size();
        let room = limits.max().width;
        iced::advanced::layout::Node::with_children(Size::new(room.min(size.width), size.height), vec![inner])
    }

    fn operate(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        if let Some(inner) = layout.children().next() {
            self.content.as_widget_mut().operate(tree, inner, renderer, operation);
        }
    }

    fn update(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        event: &iced::Event,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let Some(inner) = layout.children().next() else {
            return;
        };
        if matches!(event, iced::Event::Window(iced::window::Event::RedrawRequested(_))) && inner.bounds().width > layout.bounds().width {
            shell.request_redraw();
        }
        self.content.as_widget_mut().update(tree, event, inner, cursor, renderer, clipboard, shell, viewport);
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let Some(inner) = layout.children().next() else {
            return;
        };
        use iced::advanced::Renderer as _;
        let room = layout.bounds().width;
        let over = (inner.bounds().width - room).max(0.0);
        let at = drift(self.since.elapsed().as_secs_f32());
        let shift = if over > 0.0 { -over * at.clamp(0.0, 1.0) } else { 0.0 };
        renderer.with_layer(layout.bounds(), |renderer| {
            renderer.with_translation(iced::Vector::new(shift, 0.0), |renderer| {
                self.content.as_widget().draw(tree, renderer, theme, style, inner, cursor, viewport);
            });
        });
    }
}

impl<'a, Message: 'a> From<Drifting<'a, Message>> for Element<'a, Message> {
    fn from(drifting: Drifting<'a, Message>) -> Element<'a, Message> {
        Element::new(drifting)
    }
}

fn drift(elapsed: f32) -> f32 {
    let wait = 1.4;
    let travel = 2.6;
    let span = 2.0 * (wait + travel);
    let at = elapsed % span;
    let ease = |k: f32| k * k * (3.0 - 2.0 * k);
    if at < wait {
        0.0
    } else if at < wait + travel {
        ease((at - wait) / travel)
    } else if at < span - travel {
        1.0
    } else {
        1.0 - ease((at - (span - travel)) / travel)
    }
}

impl Machine {
    pub fn here() -> Machine {
        match std::env::consts::OS {
            "macos" => Machine::Mac,
            "windows" => Machine::Windows,
            _ => Machine::Linux,
        }
    }

    fn drawing(self) -> &'static [u8] {
        match self {
            Machine::Mac => include_bytes!("../assets/marks/apple.svg"),
            Machine::Windows => include_bytes!("../assets/marks/windows.svg"),
            Machine::Linux => include_bytes!("../assets/marks/linux.svg"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Machine {
    Mac,
    Windows,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Ru,
    En,
}

impl Lang {
    fn drawing(self) -> &'static [u8] {
        match self {
            Lang::Ru => include_bytes!("../assets/marks/flag-ru.svg"),
            Lang::En => include_bytes!("../assets/marks/flag-gb.svg"),
        }
    }
}

fn drawn(bytes: &'static [u8]) -> iced::widget::svg::Handle {
    iced::widget::svg::Handle::from_memory(bytes)
}

pub fn badge<'a, Message: 'a>(kind: Machine, side: f32) -> Element<'a, Message> {
    let k = fade();
    let mark = iced::widget::svg(drawn(kind.drawing()))
        .width(side * 0.58)
        .height(side * 0.58)
        .opacity(k)
        .style(move |_: &Theme, _| iced::widget::svg::Style { color: Some(Color { a: 0.92, ..INK }) });
    let ground = Canvas::new(Round { alpha: k }).width(side).height(side);
    iced::widget::stack![
        ground,
        container(mark).width(side).height(side).align_x(iced::alignment::Horizontal::Center).align_y(iced::alignment::Vertical::Center)
    ]
        .width(side)
        .height(side)
        .into()
}

pub struct Round {
    pub alpha: f32,
}

impl<Message> canvas::Program<Message> for Round {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let side = bounds.width.min(bounds.height);
        let tile = Path::rounded_rectangle(Point::new(0.5, 0.5), Size::new(side - 1.0, side - 1.0), (side * 0.22).into());
        frame.fill(&tile, dim(Color::from_rgb8(0x0c, 0x0a, 0x0b), self.alpha));
        vec![frame.into_geometry()]
    }
}

pub struct Rim {
    pub on: bool,
    pub k: f32,
    pub alpha: f32,
}

impl<Message> canvas::Program<Message> for Rim {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let side = bounds.width.min(bounds.height);
        let centre = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let edge = if self.on { Color { a: 0.85, ..ACCENT } } else { Color::from_rgba(1.0, 1.0, 1.0, 0.14) };
        frame.stroke(
            &Path::circle(centre, side / 2.0 - 0.75),
            Stroke::default().with_width(1.5 + 1.0 * self.k).with_color(dim(edge, self.alpha)),
        );
        vec![frame.into_geometry()]
    }
}

pub fn flag<'a, Message: 'a>(which: Lang, on: bool, k: f32, side: f32) -> Element<'a, Message> {
    let alpha = fade();
    let picture = iced::widget::svg(drawn(which.drawing())).width(side - 1.5).height(side - 1.5).opacity(alpha);
    iced::widget::stack![
        container(picture)
            .width(side)
            .height(side)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center),
        Canvas::new(Rim { on, k, alpha }).width(side).height(side)
    ]
    .width(side)
    .height(side)
    .into()
}

pub struct FrameMark {
    pub k: f32,
    pub alpha: f32,
}

impl<Message> canvas::Program<Message> for FrameMark {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let k = self.k.clamp(0.0, 1.0);
        let edge = Path::rounded_rectangle(Point::new(0.5, 0.5), Size::new(bounds.width - 1.0, bounds.height - 1.0), 10.0.into());
        let colour = Color {
            a: (0.1 + 0.8 * k) * self.alpha,
            ..if k > 0.02 { ACCENT } else { Color::WHITE }
        };
        frame.stroke(&edge, Stroke::default().with_width(1.0 + 1.5 * k).with_color(colour));
        vec![frame.into_geometry()]
    }
}

pub fn frame_mark<'a, Message: 'a>(k: f32, side: f32) -> Element<'a, Message> {
    Canvas::new(FrameMark { k, alpha: fade() }).width(side).height(side).into()
}

pub struct Scaled<'a, Message> {
    content: Element<'a, Message>,
    factor: f32,
}

pub fn scaled<'a, Message: 'a>(content: impl Into<Element<'a, Message>>, factor: f32) -> Scaled<'a, Message> {
    Scaled { content: content.into(), factor: factor.clamp(0.3, 3.0) }
}

impl<Message> Scaled<'_, Message> {
    fn inward(&self, origin: Point, cursor: mouse::Cursor) -> mouse::Cursor {
        let map = |at: Point| Point::new(origin.x + (at.x - origin.x) / self.factor, origin.y + (at.y - origin.y) / self.factor);
        match cursor {
            mouse::Cursor::Available(at) => mouse::Cursor::Available(map(at)),
            mouse::Cursor::Levitating(at) => mouse::Cursor::Levitating(map(at)),
            mouse::Cursor::Unavailable => mouse::Cursor::Unavailable,
        }
    }

    fn transformation(&self, origin: Point) -> iced::Transformation {
        iced::Transformation::translate(origin.x, origin.y) * iced::Transformation::scale(self.factor) * iced::Transformation::translate(-origin.x, -origin.y)
    }
}

impl<Message> iced::advanced::Widget<Message, Theme, Renderer> for Scaled<'_, Message> {
    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<iced::advanced::widget::Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut iced::advanced::widget::Tree) {
        self.content.as_widget().diff(tree);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(&mut self, tree: &mut iced::advanced::widget::Tree, renderer: &Renderer, limits: &iced::advanced::layout::Limits) -> iced::advanced::layout::Node {
        let inner = iced::advanced::layout::Limits::new(limits.min() * (1.0 / self.factor), limits.max() * (1.0 / self.factor));
        let node = self.content.as_widget_mut().layout(tree, renderer, &inner);
        let size = node.size();
        iced::advanced::layout::Node::with_children(size * self.factor, vec![node])
    }

    fn operate(&mut self, tree: &mut iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, renderer: &Renderer, operation: &mut dyn iced::advanced::widget::Operation) {
        if let Some(inner) = layout.children().next() {
            self.content.as_widget_mut().operate(tree, inner, renderer, operation);
        }
    }

    fn update(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        event: &iced::Event,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let Some(inner) = layout.children().next() else {
            return;
        };
        let origin = layout.bounds().position();
        let cursor = self.inward(origin, cursor);
        let event = match event {
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                iced::Event::Mouse(mouse::Event::CursorMoved { position: Point::new(origin.x + (position.x - origin.x) / self.factor, origin.y + (position.y - origin.y) / self.factor) })
            }
            other => other.clone(),
        };
        let seen = *viewport * self.transformation(origin).inverse();
        self.content.as_widget_mut().update(tree, &event, inner, cursor, renderer, clipboard, shell, &seen);
    }

    fn mouse_interaction(&self, tree: &iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        let Some(inner) = layout.children().next() else {
            return mouse::Interaction::default();
        };
        let origin = layout.bounds().position();
        let seen = *viewport * self.transformation(origin).inverse();
        self.content.as_widget().mouse_interaction(tree, inner, self.inward(origin, cursor), &seen, renderer)
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        let Some(inner) = layout.children().next() else {
            return;
        };
        let origin = layout.bounds().position();
        let transformation = self.transformation(origin);
        let seen = *viewport * transformation.inverse();
        let cursor = self.inward(origin, cursor);
        renderer.with_transformation(transformation, |renderer| {
            self.content.as_widget().draw(tree, renderer, theme, style, inner, cursor, &seen);
        });
    }

    fn overlay<'b>(&'b mut self, tree: &'b mut iced::advanced::widget::Tree, layout: iced::advanced::Layout<'b>, renderer: &Renderer, viewport: &Rectangle, translation: iced::Vector) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        let inner = layout.children().next()?;
        let origin = layout.bounds().position();
        let translation = iced::Vector::new((origin.x + translation.x) / self.factor - origin.x, (origin.y + translation.y) / self.factor - origin.y);
        let seen = *viewport * iced::Transformation::scale(self.factor).inverse();
        let content = self.content.as_widget_mut().overlay(tree, inner, renderer, &seen, translation)?;
        Some(iced::advanced::overlay::Element::new(Box::new(ScaledOverlay { content, factor: self.factor })))
    }
}

struct ScaledOverlay<'a, Message> {
    content: iced::advanced::overlay::Element<'a, Message, Theme, Renderer>,
    factor: f32,
}

impl<Message> iced::advanced::Overlay<Message, Theme, Renderer> for ScaledOverlay<'_, Message> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> iced::advanced::layout::Node {
        let inner = self.content.as_overlay_mut().layout(renderer, bounds * (1.0 / self.factor));
        let position = inner.bounds().position();
        let scaled = Point::new(position.x * self.factor, position.y * self.factor);
        let size = inner.size() * self.factor;
        iced::advanced::layout::Node::with_children(size, vec![inner.move_to(Point::new(position.x - scaled.x, position.y - scaled.y))]).move_to(scaled)
    }

    fn draw(&self, renderer: &mut Renderer, theme: &Theme, style: &iced::advanced::renderer::Style, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor) {
        use iced::advanced::Renderer as _;
        if let Some(inner) = layout.children().next() {
            let cursor = cursor * iced::Transformation::scale(self.factor).inverse();
            renderer.with_transformation(iced::Transformation::scale(self.factor), |renderer| self.content.as_overlay().draw(renderer, theme, style, inner, cursor));
        }
    }

    fn operate(&mut self, layout: iced::advanced::Layout<'_>, renderer: &Renderer, operation: &mut dyn iced::advanced::widget::Operation) {
        if let Some(inner) = layout.children().next() { self.content.as_overlay_mut().operate(inner, renderer, operation); }
    }

    fn update(&mut self, event: &iced::Event, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, renderer: &Renderer, clipboard: &mut dyn iced::advanced::Clipboard, shell: &mut iced::advanced::Shell<'_, Message>) {
        if let Some(inner) = layout.children().next() {
            let event = match event {
                iced::Event::Mouse(mouse::Event::CursorMoved { position }) => iced::Event::Mouse(mouse::Event::CursorMoved { position: Point::new(position.x / self.factor, position.y / self.factor) }),
                other => other.clone(),
            };
            self.content.as_overlay_mut().update(&event, inner, cursor * iced::Transformation::scale(self.factor).inverse(), renderer, clipboard, shell);
        }
    }

    fn mouse_interaction(&self, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, renderer: &Renderer) -> mouse::Interaction {
        layout.children().next().map_or(mouse::Interaction::None, |inner| self.content.as_overlay().mouse_interaction(inner, cursor * iced::Transformation::scale(self.factor).inverse(), renderer))
    }

    fn overlay<'a>(&'a mut self, layout: iced::advanced::Layout<'a>, renderer: &Renderer) -> Option<iced::advanced::overlay::Element<'a, Message, Theme, Renderer>> {
        let inner = layout.children().next()?;
        let content = self.content.as_overlay_mut().overlay(inner, renderer)?;
        Some(iced::advanced::overlay::Element::new(Box::new(ScaledOverlay { content, factor: self.factor })))
    }

    fn index(&self) -> f32 { self.content.as_overlay().index() }
}

impl<'a, Message: 'a> From<Scaled<'a, Message>> for Element<'a, Message> {
    fn from(scaled: Scaled<'a, Message>) -> Element<'a, Message> {
        Element::new(scaled)
    }
}

pub struct Clipped<'a, Message> {
    content: Element<'a, Message>,
}

pub fn clipped<'a, Message: 'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    Element::new(Clipped { content: content.into() })
}

impl<Message> iced::advanced::Widget<Message, Theme, Renderer> for Clipped<'_, Message> {
    fn tag(&self) -> iced::advanced::widget::tree::Tag { self.content.as_widget().tag() }
    fn state(&self) -> iced::advanced::widget::tree::State { self.content.as_widget().state() }
    fn children(&self) -> Vec<iced::advanced::widget::Tree> { self.content.as_widget().children() }
    fn diff(&self, tree: &mut iced::advanced::widget::Tree) { self.content.as_widget().diff(tree); }
    fn size(&self) -> Size<Length> { self.content.as_widget().size() }

    fn layout(&mut self, tree: &mut iced::advanced::widget::Tree, renderer: &Renderer, limits: &iced::advanced::layout::Limits) -> iced::advanced::layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(&mut self, tree: &mut iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, renderer: &Renderer, operation: &mut dyn iced::advanced::widget::Operation) {
        self.content.as_widget_mut().operate(tree, layout, renderer, operation);
    }

    fn update(&mut self, tree: &mut iced::advanced::widget::Tree, event: &iced::Event, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, renderer: &Renderer, clipboard: &mut dyn iced::advanced::Clipboard, shell: &mut iced::advanced::Shell<'_, Message>, viewport: &Rectangle) {
        self.content.as_widget_mut().update(tree, event, layout, cursor, renderer, clipboard, shell, viewport);
    }

    fn mouse_interaction(&self, tree: &iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn draw(&self, tree: &iced::advanced::widget::Tree, renderer: &mut Renderer, theme: &Theme, style: &iced::advanced::renderer::Style, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle) {
        use iced::advanced::Renderer as _;
        if let Some(bounds) = layout.bounds().intersection(viewport) {
            renderer.with_layer(bounds, |renderer| self.content.as_widget().draw(tree, renderer, theme, style, layout, cursor, &bounds));
        }
    }

    fn overlay<'b>(&'b mut self, tree: &'b mut iced::advanced::widget::Tree, layout: iced::advanced::Layout<'b>, renderer: &Renderer, viewport: &Rectangle, translation: iced::Vector) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(tree, layout, renderer, viewport, translation)
    }
}

pub struct Glass<'a, Message> {
    content: Element<'a, Message>,
    picture: Option<image::Handle>,
    window: Size,
    tint: Color,
    edge: Color,
    radius: f32,
    alpha: f32,
    depth: f32,
}

pub fn glass<'a, Message: 'a>(content: impl Into<Element<'a, Message>>, picture: Option<image::Handle>, window: Size) -> Element<'a, Message> {
    glass_of(content, picture, window, 0.74)
}

pub fn glass_deep<'a, Message: 'a>(content: impl Into<Element<'a, Message>>, picture: Option<image::Handle>, window: Size) -> Element<'a, Message> {
    glass_of(content, picture, window, 0.86)
}

pub fn frosted(picture: &image::Handle) -> Option<image::Handle> {
    static KEPT: std::sync::Mutex<Vec<(iced::advanced::image::Id, Option<image::Handle>)>> = std::sync::Mutex::new(Vec::new());
    let mut kept = KEPT.lock().ok()?;
    if let Some((_, made)) = kept.iter().find(|(id, _)| *id == picture.id()) {
        return made.clone();
    }
    let made = match picture {
        image::Handle::Rgba { width, height, pixels, .. } => ::image::RgbaImage::from_raw(*width, *height, pixels.to_vec()).map(|full| {
            let wide = 96u32;
            let high = ((*height as f32 / (*width).max(1) as f32) * wide as f32).round().max(1.0) as u32;
            let small = ::image::imageops::resize(&full, wide, high, ::image::imageops::FilterType::Triangle);
            let soft = ::image::imageops::blur(&small, 5.0);
            image::Handle::from_rgba(soft.width(), soft.height(), soft.into_raw())
        }),
        _ => None,
    };
    if kept.len() > 32 {
        kept.clear();
    }
    kept.push((picture.id(), made.clone()));
    made
}

fn glass_of<'a, Message: 'a>(content: impl Into<Element<'a, Message>>, picture: Option<image::Handle>, window: Size, depth: f32) -> Element<'a, Message> {
    let picture = picture.as_ref().and_then(frosted);
    let depth = if picture.is_some() { depth } else { 0.95 };
    let alpha = fade();
    Element::new(Glass {
        content: content.into(),
        picture,
        window,
        tint: Color::from_rgba(0.086, 0.039, 0.059, depth * alpha),
        edge: Color::from_rgba(1.0, 1.0, 1.0, 0.14 * alpha),
        radius: 18.0,
        alpha,
        depth,
    })
}

pub fn glass_piece(pixels: &[u8], wide: u32, high: u32, bounds: Rectangle, window: Size, radius: f32, tint: [f32; 3], depth: f32) -> Option<(u32, u32, Vec<u8>)> {
    if wide == 0 || high == 0 || pixels.len() < (wide as usize) * (high as usize) * 4 || bounds.width < 1.0 || bounds.height < 1.0 {
        return None;
    }
    let out_wide = (bounds.width.round() as u32).clamp(1, 4096);
    let out_high = (bounds.height.round() as u32).clamp(1, 4096);
    let cover = glass_cover(Size::new(wide as f32, high as f32), window);
    let r = radius.min(bounds.width / 2.0).min(bounds.height / 2.0).max(0.0);
    let along = |count: u32, length: f32, start: f32, origin: f32, span: f32, size: u32| -> Vec<(usize, usize, f32, f32)> {
        (0..count)
            .map(|at| {
                let local = (at as f32 + 0.5) * length / count as f32;
                let source = ((start + local - origin) / span.max(1.0) * size as f32 - 0.5).clamp(0.0, size as f32 - 1.0);
                let low = source.floor() as usize;
                (low, (low + 1).min(size as usize - 1), source - low as f32, local.min(length - local))
            })
            .collect()
    };
    let columns = along(out_wide, bounds.width, bounds.x, cover.x, cover.width, wide);
    let rows = along(out_high, bounds.height, bounds.y, cover.y, cover.height, high);
    let mut out = Vec::with_capacity(out_wide as usize * out_high as usize * 4);
    let line = wide as usize * 4;
    for (top, bottom, down, near_y) in &rows {
        for (left, right, across, near_x) in &columns {
            for channel in 0..3 {
                let a = pixels[top * line + left * 4 + channel] as f32;
                let b = pixels[top * line + right * 4 + channel] as f32;
                let c = pixels[bottom * line + left * 4 + channel] as f32;
                let d = pixels[bottom * line + right * 4 + channel] as f32;
                let upper = a + (b - a) * across;
                let lower = c + (d - c) * across;
                let seen = upper + (lower - upper) * down;
                out.push((seen * (1.0 - depth) + tint[channel] * 255.0 * depth).round().clamp(0.0, 255.0) as u8);
            }
            let inside = if *near_x < r && *near_y < r { (r - ((r - near_x).powi(2) + (r - near_y).powi(2)).sqrt() - 0.5).clamp(0.0, 1.0) } else { (near_x.min(*near_y) - 0.5).clamp(0.0, 1.0) };
            out.push((inside * 255.0).round() as u8);
        }
    }
    Some((out_wide, out_high, out))
}

type GlassCut = (iced::advanced::image::Id, [i32; 11]);

fn glass_cut(picture: &image::Handle, bounds: Rectangle, window: Size, radius: f32, tint: [f32; 3], depth: f32) -> Option<image::Handle> {
    static KEPT: std::sync::Mutex<Vec<(GlassCut, image::Handle)>> = std::sync::Mutex::new(Vec::new());
    let half = |value: f32| (value * 2.0).round() as i32;
    let key: GlassCut = (picture.id(), [half(bounds.x), half(bounds.y), half(bounds.width), half(bounds.height), half(window.width), half(window.height), half(radius), half(tint[0] * 500.0), half(tint[1] * 500.0), half(tint[2] * 500.0), half(depth * 500.0)]);
    let mut kept = KEPT.lock().ok()?;
    if let Some((_, made)) = kept.iter().find(|(other, _)| *other == key) {
        return Some(made.clone());
    }
    let image::Handle::Rgba { width, height, pixels, .. } = picture else {
        return None;
    };
    let (wide, high, cut) = glass_piece(pixels, *width, *height, bounds, window, radius, tint, depth)?;
    let made = image::Handle::from_rgba(wide, high, cut);
    if kept.len() >= 24 {
        kept.remove(0);
    }
    kept.push((key, made.clone()));
    Some(made)
}

pub fn glass_cover(picture: Size, window: Size) -> Rectangle {
    let scale = (window.width / picture.width.max(1.0)).max(window.height / picture.height.max(1.0));
    let size = Size::new(picture.width * scale, picture.height * scale);
    Rectangle::new(Point::new((window.width - size.width) / 2.0, (window.height - size.height) / 2.0), size)
}

impl<Message> iced::advanced::Widget<Message, Theme, Renderer> for Glass<'_, Message> {
    fn tag(&self) -> iced::advanced::widget::tree::Tag { self.content.as_widget().tag() }
    fn state(&self) -> iced::advanced::widget::tree::State { self.content.as_widget().state() }
    fn children(&self) -> Vec<iced::advanced::widget::Tree> { self.content.as_widget().children() }
    fn diff(&self, tree: &mut iced::advanced::widget::Tree) { self.content.as_widget().diff(tree); }
    fn size(&self) -> Size<Length> { self.content.as_widget().size() }

    fn layout(&mut self, tree: &mut iced::advanced::widget::Tree, renderer: &Renderer, limits: &iced::advanced::layout::Limits) -> iced::advanced::layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }

    fn operate(&mut self, tree: &mut iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, renderer: &Renderer, operation: &mut dyn iced::advanced::widget::Operation) {
        self.content.as_widget_mut().operate(tree, layout, renderer, operation);
    }

    fn update(&mut self, tree: &mut iced::advanced::widget::Tree, event: &iced::Event, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, renderer: &Renderer, clipboard: &mut dyn iced::advanced::Clipboard, shell: &mut iced::advanced::Shell<'_, Message>, viewport: &Rectangle) {
        self.content.as_widget_mut().update(tree, event, layout, cursor, renderer, clipboard, shell, viewport);
    }

    fn mouse_interaction(&self, tree: &iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn draw(&self, tree: &iced::advanced::widget::Tree, renderer: &mut Renderer, theme: &Theme, style: &iced::advanced::renderer::Style, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle) {
        use iced::advanced::image::Renderer as _;
        use iced::advanced::Renderer as _;
        let bounds = layout.bounds();
        let solid = Color { a: self.alpha, ..theme::GROUND };
        if self.picture.is_some() {
            renderer.with_layer(bounds, |renderer| {
                renderer.fill_quad(iced::advanced::renderer::Quad { bounds, border: iced::Border { radius: self.radius.into(), ..iced::Border::default() }, ..iced::advanced::renderer::Quad::default() }, solid);
            });
        }
        let baked = self.picture.as_ref().and_then(|picture| glass_cut(picture, bounds, self.window, self.radius, [self.tint.r, self.tint.g, self.tint.b], self.depth.clamp(0.0, 1.0)));
        let fill = if baked.is_some() { Color::TRANSPARENT } else { self.tint };
        if let Some(piece) = baked {
            renderer.with_layer(bounds, |renderer| {
                renderer.draw_image(
                    iced::advanced::image::Image { handle: piece, filter_method: image::FilterMethod::Linear, rotation: iced::Radians(0.0), border_radius: 0.0.into(), opacity: self.alpha, snap: true },
                    bounds,
                    bounds,
                );
            });
        }
        renderer.with_layer(bounds, |renderer| {
            renderer.fill_quad(
                iced::advanced::renderer::Quad { bounds, border: iced::Border { color: self.edge, width: 1.0, radius: self.radius.into() }, ..iced::advanced::renderer::Quad::default() },
                fill,
            );
            self.content.as_widget().draw(tree, renderer, theme, style, layout, cursor, viewport);
        });
    }

    fn overlay<'b>(&'b mut self, tree: &'b mut iced::advanced::widget::Tree, layout: iced::advanced::Layout<'b>, renderer: &Renderer, viewport: &Rectangle, translation: iced::Vector) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(tree, layout, renderer, viewport, translation)
    }
}

pub struct Dashes {
    pub radius: f32,
    pub colour: Color,
}

#[derive(Debug, Default)]
pub struct March {
    since: Option<std::time::Instant>,
    shift: f32,
}

const MARCH_SPEED: f32 = 9.0;
const MARCH_EVERY: f32 = 1.0 / 30.0;
const DASH: f32 = 8.0;

pub(crate) fn dash_period(around: f32) -> f32 {
    if around <= DASH { DASH } else { around / (around / DASH).round().max(1.0) }
}

impl<Message> canvas::Program<Message> for Dashes {
    type State = March;

    fn update(&self, state: &mut March, event: &iced::Event, _: Rectangle, _: mouse::Cursor) -> Option<canvas::Action<Message>> {
        if let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            let since = *state.since.get_or_insert(*now);
            state.shift = now.saturating_duration_since(since).as_secs_f32() * MARCH_SPEED;
            return Some(canvas::Action::request_redraw_at(*now + std::time::Duration::from_secs_f32(MARCH_EVERY)));
        }
        None
    }

    fn draw(&self, state: &March, renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let (left, top, right, bottom) = (0.75, 0.75, bounds.width - 0.75, bounds.height - 0.75);
        if right <= left || bottom <= top {
            return vec![frame.into_geometry()];
        }
        let round = self.radius.min((right - left) / 2.0).min((bottom - top) / 2.0).max(0.0);
        let flat = right - left - 2.0 * round;
        let period = dash_period(2.0 * flat + 2.0 * (bottom - top - 2.0 * round) + std::f32::consts::TAU * round);
        let lead = (state.shift % period).min(flat.max(0.0));
        let path = Path::new(|path| {
            path.move_to(Point::new(left + round + lead, top));
            path.line_to(Point::new(right - round, top));
            path.arc_to(Point::new(right, top), Point::new(right, top + round), round);
            path.line_to(Point::new(right, bottom - round));
            path.arc_to(Point::new(right, bottom), Point::new(right - round, bottom), round);
            path.line_to(Point::new(left + round, bottom));
            path.arc_to(Point::new(left, bottom), Point::new(left, bottom - round), round);
            path.line_to(Point::new(left, top + round));
            path.arc_to(Point::new(left, top), Point::new(left + round, top), round);
            path.line_to(Point::new(left + round + lead, top));
        });
        frame.stroke(&path, canvas::Stroke { style: canvas::Style::Solid(self.colour), width: 1.0, line_dash: canvas::LineDash { segments: &[period / 2.0, period / 2.0], offset: 0 }, ..canvas::Stroke::default() });
        vec![frame.into_geometry()]
    }
}

const RISE_STEPS: usize = 7;

pub fn rise<'a, Message: 'a>(colour: Color, high: f32) -> Element<'a, Message> {
    let k = fade();
    let bands = (1..=RISE_STEPS).map(|step| {
        let wanted = step as f32 / (RISE_STEPS + 1) as f32;
        let cover = (1.0 - (1.0 - wanted).powf(2.2)) * k;
        iced::widget::container(iced::widget::Space::new()).width(Length::Fill).height(high / RISE_STEPS as f32).style(move |_| iced::widget::container::Style { background: Some(iced::Background::Color(Color { a: cover, ..colour })), ..iced::widget::container::Style::default() }).into()
    });
    iced::widget::Column::with_children(bands).width(Length::Fill).into()
}

pub fn dashed<'a, Message: 'a>(content: impl Into<Element<'a, Message>>, radius: f32, colour: Color) -> Element<'a, Message> {
    iced::widget::stack![content.into(), Canvas::new(Dashes { radius, colour: faded(colour) }).width(Length::Fill).height(Length::Fill)].into()
}

pub fn glass_warm<'a, Message: 'a>(content: impl Into<Element<'a, Message>>, picture: Option<image::Handle>, window: Size) -> Element<'a, Message> {
    let picture = picture.as_ref().and_then(frosted);
    let alpha = fade();
    Element::new(Glass { content: content.into(), picture, window, tint: Color::from_rgba(0.16, 0.04, 0.055, 0.86 * alpha), edge: Color::from_rgba(0.886, 0.282, 0.282, 0.45 * alpha), radius: 18.0, alpha, depth: 0.86 })
}

pub const RESIZE: f32 = 0.28;

pub struct Smooth<'a, Message> {
    content: Element<'a, Message>,
}

#[derive(Debug, Default)]
struct Resizing {
    to: Option<f32>,
    from: f32,
    since: Option<std::time::Instant>,
    now: Option<std::time::Instant>,
    due: bool,
}

impl Resizing {
    fn high(&self) -> f32 {
        let to = self.to.unwrap_or(0.0);
        if self.due {
            return self.from;
        }
        match (self.since, self.now) {
            (Some(since), Some(now)) => {
                let x = (now.saturating_duration_since(since).as_secs_f32() / RESIZE).clamp(0.0, 1.0);
                self.from + (to - self.from) * (1.0 - (1.0 - x).powi(3))
            }
            _ => to,
        }
    }

    fn aim(&mut self, high: f32) {
        match self.to {
            None => self.to = Some(high),
            Some(to) if (to - high).abs() > 0.5 => {
                self.from = self.high();
                self.to = Some(high);
                self.since = None;
                self.due = true;
            }
            Some(_) => {}
        }
    }

    fn step(&mut self, now: std::time::Instant) -> bool {
        self.now = Some(now);
        if self.due {
            self.due = false;
            self.since = Some(now);
        }
        match self.since {
            Some(since) if now.saturating_duration_since(since).as_secs_f32() < RESIZE => true,
            Some(_) => {
                self.since = None;
                false
            }
            None => false,
        }
    }
}

pub fn smooth<'a, Message: 'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    Element::new(Smooth { content: content.into() })
}

impl<Message> iced::advanced::Widget<Message, Theme, Renderer> for Smooth<'_, Message> {
    fn tag(&self) -> iced::advanced::widget::tree::Tag { iced::advanced::widget::tree::Tag::of::<Resizing>() }
    fn state(&self) -> iced::advanced::widget::tree::State { iced::advanced::widget::tree::State::new(Resizing::default()) }
    fn children(&self) -> Vec<iced::advanced::widget::Tree> { vec![iced::advanced::widget::Tree::new(&self.content)] }
    fn diff(&self, tree: &mut iced::advanced::widget::Tree) { tree.diff_children(std::slice::from_ref(&self.content)); }
    fn size(&self) -> Size<Length> { Size { width: self.content.as_widget().size().width, height: Length::Shrink } }
    fn layout(&mut self, tree: &mut iced::advanced::widget::Tree, renderer: &Renderer, limits: &iced::advanced::layout::Limits) -> iced::advanced::layout::Node {
        let node = self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits);
        let state = tree.state.downcast_mut::<Resizing>();
        state.aim(node.size().height);
        let size = Size::new(node.size().width, state.high());
        iced::advanced::layout::Node::with_children(size, vec![node])
    }
    fn operate(&mut self, tree: &mut iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, renderer: &Renderer, operation: &mut dyn iced::advanced::widget::Operation) {
        if let Some(inner) = layout.children().next() { self.content.as_widget_mut().operate(&mut tree.children[0], inner, renderer, operation); }
    }
    fn update(&mut self, tree: &mut iced::advanced::widget::Tree, event: &iced::Event, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, renderer: &Renderer, clipboard: &mut dyn iced::advanced::Clipboard, shell: &mut iced::advanced::Shell<'_, Message>, viewport: &Rectangle) {
        if let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            if tree.state.downcast_mut::<Resizing>().step(*now) {
                shell.invalidate_layout();
                shell.request_redraw();
            }
        }
        if let Some(inner) = layout.children().next() { self.content.as_widget_mut().update(&mut tree.children[0], event, inner, cursor, renderer, clipboard, shell, viewport); }
    }
    fn mouse_interaction(&self, tree: &iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        layout.children().next().map_or(mouse::Interaction::None, |inner| self.content.as_widget().mouse_interaction(&tree.children[0], inner, cursor, viewport, renderer))
    }
    fn draw(&self, tree: &iced::advanced::widget::Tree, renderer: &mut Renderer, theme: &Theme, style: &iced::advanced::renderer::Style, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle) {
        use iced::advanced::Renderer as _;
        let Some(inner) = layout.children().next() else {
            return;
        };
        if (inner.bounds().height - layout.bounds().height).abs() <= 0.5 {
            self.content.as_widget().draw(&tree.children[0], renderer, theme, style, inner, cursor, viewport);
        } else if let Some(bounds) = layout.bounds().intersection(viewport) {
            renderer.with_layer(bounds, |renderer| self.content.as_widget().draw(&tree.children[0], renderer, theme, style, inner, cursor, &bounds));
        }
    }
    fn overlay<'b>(&'b mut self, tree: &'b mut iced::advanced::widget::Tree, layout: iced::advanced::Layout<'b>, renderer: &Renderer, viewport: &Rectangle, translation: iced::Vector) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        let inner = layout.children().next()?;
        self.content.as_widget_mut().overlay(&mut tree.children[0], inner, renderer, viewport, translation)
    }
}

pub fn collapsing<'a, Message: 'a>(content: impl Into<Element<'a, Message>>, fraction: f32) -> Element<'a, Message> {
    Element::new(Collapsing { content: content.into(), fraction: fraction.clamp(0.0, 1.0) })
}

struct Collapsing<'a, Message> { content: Element<'a, Message>, fraction: f32 }

impl<Message> iced::advanced::Widget<Message, Theme, Renderer> for Collapsing<'_, Message> {
    fn tag(&self) -> iced::advanced::widget::tree::Tag { self.content.as_widget().tag() }
    fn state(&self) -> iced::advanced::widget::tree::State { self.content.as_widget().state() }
    fn children(&self) -> Vec<iced::advanced::widget::Tree> { self.content.as_widget().children() }
    fn diff(&self, tree: &mut iced::advanced::widget::Tree) { self.content.as_widget().diff(tree); }
    fn size(&self) -> Size<Length> { Size { width: self.content.as_widget().size().width, height: Length::Shrink } }
    fn layout(&mut self, tree: &mut iced::advanced::widget::Tree, renderer: &Renderer, limits: &iced::advanced::layout::Limits) -> iced::advanced::layout::Node {
        let node = self.content.as_widget_mut().layout(tree, renderer, limits);
        let size = Size::new(node.size().width, node.size().height * self.fraction);
        iced::advanced::layout::Node::with_children(size, vec![node])
    }
    fn operate(&mut self, tree: &mut iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, renderer: &Renderer, operation: &mut dyn iced::advanced::widget::Operation) {
        if let Some(inner) = layout.children().next() { self.content.as_widget_mut().operate(tree, inner, renderer, operation); }
    }
    fn update(&mut self, tree: &mut iced::advanced::widget::Tree, event: &iced::Event, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, renderer: &Renderer, clipboard: &mut dyn iced::advanced::Clipboard, shell: &mut iced::advanced::Shell<'_, Message>, viewport: &Rectangle) {
        if self.fraction < 0.999 { return; }
        if let Some(inner) = layout.children().next() { self.content.as_widget_mut().update(tree, event, inner, cursor, renderer, clipboard, shell, viewport); }
    }
    fn mouse_interaction(&self, tree: &iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        if self.fraction < 0.999 { return mouse::Interaction::None; }
        layout.children().next().map_or(mouse::Interaction::None, |inner| self.content.as_widget().mouse_interaction(tree, inner, cursor, viewport, renderer))
    }
    fn draw(&self, tree: &iced::advanced::widget::Tree, renderer: &mut Renderer, theme: &Theme, style: &iced::advanced::renderer::Style, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle) {
        use iced::advanced::Renderer as _;
        if let (Some(bounds), Some(inner)) = (layout.bounds().intersection(viewport), layout.children().next()) {
            renderer.with_layer(bounds, |renderer| self.content.as_widget().draw(tree, renderer, theme, style, inner, cursor, &bounds));
        }
    }
}

pub fn framed(handle: &iced::widget::image::Handle, wide: f32, high: f32, radius: impl Into<iced::border::Radius>) -> iced::widget::Image {
    let picture = iced::widget::image(handle.clone()).width(Length::Fill).height(high).border_radius(radius);
    match handle {
        iced::widget::image::Handle::Rgba { width, height, .. } if *width > 0 && *height > 0 && wide > 0.0 && high > 0.0 => {
            let want = wide / high;
            let (w, h) = (*width as f32, *height as f32);
            let (cut_w, cut_h) = if w / h > want { ((h * want).round().max(1.0), h) } else { (w, (w / want).round().max(1.0)) };
            let region = Rectangle { x: ((w - cut_w) / 2.0) as u32, y: ((h - cut_h) / 2.0) as u32, width: cut_w as u32, height: cut_h as u32 };
            picture.crop(region).content_fit(iced::ContentFit::Fill)
        }
        _ => picture.content_fit(iced::ContentFit::Cover),
    }
}

pub fn bare_input(k: f32) -> impl Fn(&Theme, iced::widget::text_input::Status) -> iced::widget::text_input::Style {
    move |_, _| iced::widget::text_input::Style {
        background: iced::Background::Color(Color::TRANSPARENT),
        border: iced::Border::default(),
        icon: Color { a: k, ..theme::FAINT },
        placeholder: Color { a: k, ..theme::FAINT },
        value: Color { a: k, ..theme::INK },
        selection: Color::from_rgba(0.886, 0.282, 0.282, 0.35 * k),
    }
}

fn pictograph(c: char) -> bool {
    matches!(c as u32, 0x1F000..=0x1FAFF | 0x2300..=0x23FF | 0x2600..=0x2604 | 0x2606..=0x2714 | 0x2716..=0x27BF | 0x2B00..=0x2BFF | 0xFE00..=0xFE0F | 0x200D | 0x20E3 | 0xE0020..=0xE007F | 0x3030 | 0x303D | 0x3297 | 0x3299)
}

pub fn settled(words: &str) -> String {
    if fade() >= 0.999 {
        words.to_owned()
    } else {
        words.chars().filter(|c| !pictograph(*c)).collect()
    }
}

pub fn hidden_bar() -> iced::widget::scrollable::Direction {
    iced::widget::scrollable::Direction::Vertical(iced::widget::scrollable::Scrollbar::hidden())
}

pub struct Piece {
    words: String,
    font: iced::Font,
    size: f32,
    colour: Color,
    gap: f32,
}

pub fn piece(words: impl Into<String>, font: iced::Font, size: f32, colour: Color) -> Piece {
    Piece { words: settled(&words.into()), font, size, colour: faded(colour), gap: 0.0 }
}

pub fn pieces(parts: Vec<String>, font: iced::Font, size: f32, colour: Color) -> Vec<Piece> {
    parts.into_iter().enumerate().map(|(at, words)| piece(words, font, size, colour).after(if at == 0 { 0.0 } else { 14.0 })).collect()
}

impl Piece {
    pub fn after(mut self, gap: f32) -> Piece {
        self.gap = gap;
        self
    }
}

pub fn text_width(words: &str, font: iced::Font, size: f32) -> f32 {
    use iced::advanced::text::Paragraph as _;
    Paragraph::with_text(iced::advanced::text::Text {
        content: words,
        bounds: Size::INFINITE,
        size: iced::Pixels(size),
        line_height: iced::widget::text::LineHeight::default(),
        font,
        align_x: iced::widget::text::Alignment::Left,
        align_y: iced::alignment::Vertical::Top,
        shaping: iced::widget::text::Shaping::default(),
        wrapping: iced::widget::text::Wrapping::None,
    })
    .min_width()
}

pub fn fit_size(words: &str, font: iced::Font, base: f32, least: f32, room: f32) -> f32 {
    let wide = text_width(words, font, base);
    if wide <= room || wide <= 0.0 || room <= 0.0 {
        return base;
    }
    (base * room / wide).max(least).min(base)
}

pub struct Marquee {
    pieces: Vec<Piece>,
    width: Length,
    centred: bool,
}

pub fn marquee(pieces: Vec<Piece>) -> Marquee {
    Marquee { pieces, width: Length::Fill, centred: false }
}

pub const SCROLL_LONG_TEXT: bool = true;

pub fn moving_text<'a, Message: 'a>(words: String, font: iced::Font, size: f32, colour: Color) -> Element<'a, Message> {
    if SCROLL_LONG_TEXT {
        marquee(vec![piece(words, font, size, colour)]).into()
    } else {
        text(words).font(font).size(size).color(faded(colour)).width(Length::Fill).into()
    }
}

impl Marquee {
    pub fn width(mut self, width: impl Into<Length>) -> Marquee {
        self.width = width.into();
        self
    }

    pub fn centred(mut self) -> Marquee {
        self.centred = true;
        self
    }
}

type Paragraph = <Renderer as iced::advanced::text::Renderer>::Paragraph;

#[derive(Default)]
struct MarqueeState {
    laid: Vec<(String, u32, iced::Font)>,
    paragraphs: Vec<Paragraph>,
    ellipses: Vec<Paragraph>,
    born: Option<std::time::Instant>,
    now: Option<std::time::Instant>,
}

impl MarqueeState {
    fn wide(&self, pieces: &[Piece]) -> f32 {
        self.paragraphs.iter().zip(pieces).map(|(paragraph, piece)| piece.gap + iced::advanced::text::Paragraph::min_width(paragraph)).sum()
    }
}

const MARQUEE_REST: f32 = 2.0;
const MARQUEE_SPEED: f32 = 26.0;
const MARQUEE_EDGE: f32 = 22.0;
const MARQUEE_SLICES: usize = 6;

fn glide(over: f32, spent: f32) -> (f32, f32) {
    let travel = (over / MARQUEE_SPEED).max(0.8);
    let cycle = 2.0 * (MARQUEE_REST + travel);
    let t = spent.rem_euclid(cycle);
    let ease = |x: f32| x * x * (3.0 - 2.0 * x);
    if t < MARQUEE_REST {
        (0.0, MARQUEE_REST - t)
    } else if t < MARQUEE_REST + travel {
        (over * ease((t - MARQUEE_REST) / travel), 0.0)
    } else if t < 2.0 * MARQUEE_REST + travel {
        (over, 2.0 * MARQUEE_REST + travel - t)
    } else {
        (over * (1.0 - ease((t - 2.0 * MARQUEE_REST - travel) / travel)), 0.0)
    }
}

impl<M> iced::advanced::Widget<M, Theme, Renderer> for Marquee {
    fn operate(&mut self, _: &mut iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, _: &Renderer, operation: &mut dyn iced::advanced::widget::Operation) {
        for piece in &self.pieces { operation.text(None, layout.bounds(), &piece.words); }
    }

    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        iced::advanced::widget::tree::Tag::of::<MarqueeState>()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        iced::advanced::widget::tree::State::new(MarqueeState::default())
    }

    fn size(&self) -> Size<Length> {
        Size { width: self.width, height: Length::Shrink }
    }

    fn layout(&mut self, tree: &mut iced::advanced::widget::Tree, _: &Renderer, limits: &iced::advanced::layout::Limits) -> iced::advanced::layout::Node {
        use iced::advanced::text::Paragraph as _;
        let state = tree.state.downcast_mut::<MarqueeState>();
        let key: Vec<(String, u32, iced::Font)> = self.pieces.iter().map(|piece| (piece.words.clone(), piece.size.to_bits(), piece.font)).collect();
        if state.laid != key {
            state.paragraphs = self
                .pieces
                .iter()
                .map(|piece| {
                    Paragraph::with_text(iced::advanced::text::Text {
                        content: piece.words.as_str(),
                        bounds: Size::INFINITE,
                        size: iced::Pixels(piece.size),
                        line_height: iced::widget::text::LineHeight::default(),
                        font: piece.font,
                        align_x: iced::widget::text::Alignment::Left,
                        align_y: iced::alignment::Vertical::Top,
                        shaping: iced::widget::text::Shaping::default(),
                        wrapping: iced::widget::text::Wrapping::None,
                    })
                })
                .collect();
            state.ellipses = self
                .pieces
                .iter()
                .map(|piece| {
                    Paragraph::with_text(iced::advanced::text::Text {
                        content: "\u{2026}",
                        bounds: Size::INFINITE,
                        size: iced::Pixels(piece.size),
                        line_height: iced::widget::text::LineHeight::default(),
                        font: piece.font,
                        align_x: iced::widget::text::Alignment::Left,
                        align_y: iced::alignment::Vertical::Top,
                        shaping: iced::widget::text::Shaping::default(),
                        wrapping: iced::widget::text::Wrapping::None,
                    })
                })
                .collect();
            state.laid = key;
            state.born = None;
        }
        let high = state.paragraphs.iter().map(|paragraph| paragraph.min_height()).fold(0.0, f32::max);
        iced::advanced::layout::Node::new(limits.resolve(self.width, Length::Shrink, Size::new(state.wide(&self.pieces), high)))
    }

    fn update(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        event: &iced::Event,
        layout: iced::advanced::Layout<'_>,
        _: mouse::Cursor,
        _: &Renderer,
        _: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, M>,
        _: &Rectangle,
    ) {
        if let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            let state = tree.state.downcast_mut::<MarqueeState>();
            let born = *state.born.get_or_insert(*now);
            state.now = Some(*now);
            let over = state.wide(&self.pieces) - layout.bounds().width;
            if over > 0.5 {
                let (_, rest) = glide(over, now.saturating_duration_since(born).as_secs_f32());
                shell.request_redraw_at(iced::window::RedrawRequest::At(*now + std::time::Duration::from_secs_f32(rest.max(0.016))));
            }
        }
    }

    fn draw(&self, tree: &iced::advanced::widget::Tree, renderer: &mut Renderer, _: &Theme, _: &iced::advanced::renderer::Style, layout: iced::advanced::Layout<'_>, _: mouse::Cursor, viewport: &Rectangle) {
        use iced::advanced::text::{Paragraph as _, Renderer as _};
        let state = tree.state.downcast_ref::<MarqueeState>();
        let bounds = layout.bounds();
        let Some(visible) = bounds.intersection(viewport) else {
            return;
        };
        let wide = state.wide(&self.pieces);
        let over = wide - bounds.width;
        let draw_at = |renderer: &mut Renderer, start: f32, alpha: f32, clip: Rectangle| {
            let mut x = start;
            for (paragraph, piece) in state.paragraphs.iter().zip(&self.pieces) {
                x += piece.gap;
                let y = bounds.y + (bounds.height - paragraph.min_height()) / 2.0;
                renderer.fill_paragraph(paragraph, Point::new(x, y), Color { a: piece.colour.a * alpha, ..piece.colour }, clip);
                x += paragraph.min_width();
            }
        };
        if over <= 0.5 {
            let start = if self.centred { bounds.x + (bounds.width - wide) / 2.0 } else { bounds.x };
            draw_at(renderer, start, 1.0, visible);
            return;
        }
        let spent = state.now.zip(state.born).map_or(0.0, |(now, born)| now.saturating_duration_since(born).as_secs_f32());
        let (offset, _) = glide(over, spent);
        if offset < 0.5 && !self.centred {
            let last = self.pieces.len().saturating_sub(1);
            let dots = state.ellipses.get(last).map_or(0.0, |dots| dots.min_width());
            let limit = (bounds.width - dots).max(0.0);
            let mut at = 0.0;
            let mut cut = last;
            for (index, (paragraph, piece)) in state.paragraphs.iter().zip(&self.pieces).enumerate() {
                at += piece.gap + paragraph.min_width();
                if at > limit {
                    cut = index;
                    break;
                }
            }
            if let Some(kept) = (Rectangle { x: bounds.x, y: bounds.y, width: limit, height: bounds.height }).intersection(&visible) {
                draw_at(renderer, bounds.x, 1.0, kept);
            }
            if let (Some(dots), Some(piece)) = (state.ellipses.get(cut), self.pieces.get(cut)) {
                let y = bounds.y + (bounds.height - dots.min_height()) / 2.0;
                renderer.fill_paragraph(dots, Point::new(bounds.x + limit, y), piece.colour, visible);
            }
            return;
        }
        let start = bounds.x - offset;
        let edge = MARQUEE_EDGE.min(bounds.width / 3.0);
        let fade_left = (offset / edge).clamp(0.0, 1.0);
        let fade_right = ((over - offset) / edge).clamp(0.0, 1.0);
        let cut = |from: f32, to: f32| Rectangle { x: from, y: bounds.y, width: (to - from).max(0.0), height: bounds.height }.intersection(&visible);
        if let Some(middle) = cut(bounds.x + edge, bounds.x + bounds.width - edge) {
            draw_at(renderer, start, 1.0, middle);
        }
        for slice in 0..MARQUEE_SLICES {
            let near = slice as f32 / MARQUEE_SLICES as f32;
            let far = (slice + 1) as f32 / MARQUEE_SLICES as f32;
            let depth = (slice as f32 + 0.5) / MARQUEE_SLICES as f32;
            if let Some(left) = cut(bounds.x + edge * near, bounds.x + edge * far) {
                draw_at(renderer, start, 1.0 - fade_left * (1.0 - depth), left);
            }
            if let Some(right) = cut(bounds.x + bounds.width - edge * far, bounds.x + bounds.width - edge * near) {
                draw_at(renderer, start, 1.0 - fade_right * (1.0 - depth), right);
            }
        }
    }
}

impl<'a, M: 'a> From<Marquee> for Element<'a, M> {
    fn from(marquee: Marquee) -> Element<'a, M> {
        Element::new(marquee)
    }
}

pub const APPEAR: f32 = 0.34;
pub const STAGGER: f32 = 0.04;
pub const APPEAR_ALL: f32 = 1.4;
pub const TALLY: f32 = 0.8;

pub fn tally(t: f32, delay: f32) -> f64 {
    let x = ((t - delay) / TALLY).clamp(0.0, 1.0);
    f64::from(1.0 - (1.0 - x).powi(3))
}

pub fn appear(t: f32, index: usize) -> f32 {
    let x = ((t - index.min(16) as f32 * STAGGER) / APPEAR).clamp(0.0, 1.0);
    1.0 - (1.0 - x).powi(3)
}

pub fn appearing<'a, M: 'a>(k: f32, rise: f32, build: impl FnOnce() -> Element<'a, M>) -> Element<'a, M> {
    let content = if k >= 0.999 { build() } else { fading(fade() * k, build) };
    lifted(content, k, rise)
}

pub fn lifted<'a, M: 'a>(content: Element<'a, M>, k: f32, rise: f32) -> Element<'a, M> {
    Element::new(Lift { content, dx: 0.0, dy: if k >= 0.999 { 0.0 } else { (1.0 - k) * rise } })
}

pub fn slid<'a, M: 'a>(content: Element<'a, M>, k: f32, run: f32) -> Element<'a, M> {
    Element::new(Lift { content, dx: if k >= 0.999 { 0.0 } else { (1.0 - k) * run }, dy: 0.0 })
}

pub fn sliding_in<'a, M: 'a>(k: f32, run: f32, build: impl FnOnce() -> Element<'a, M>) -> Element<'a, M> {
    let content = if k >= 0.999 { build() } else { fading(fade() * k, build) };
    slid(content, k, run)
}

pub struct Lift<'a, Message> {
    content: Element<'a, Message>,
    dx: f32,
    dy: f32,
}

impl<Message> iced::advanced::Widget<Message, Theme, Renderer> for Lift<'_, Message> {
    fn children(&self) -> Vec<iced::advanced::widget::Tree> {
        vec![iced::advanced::widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut iced::advanced::widget::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(&mut self, tree: &mut iced::advanced::widget::Tree, renderer: &Renderer, limits: &iced::advanced::layout::Limits) -> iced::advanced::layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(&mut self, tree: &mut iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, renderer: &Renderer, operation: &mut dyn iced::advanced::widget::Operation) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        event: &iced::Event,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);
    }

    fn mouse_interaction(&self, tree: &iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        if self.dy.abs() < 0.01 && self.dx.abs() < 0.01 {
            self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
            return;
        }
        use iced::advanced::Renderer as _;
        let shift = iced::Vector::new(self.dx, self.dy);
        let seen = *viewport - shift;
        renderer.with_translation(shift, |renderer| {
            self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, &seen);
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

pub struct Inert<'a, Message> {
    content: Element<'a, Message>,
    still: bool,
}

pub fn inert<'a, Message: 'a>(content: impl Into<Element<'a, Message>>, still: bool) -> Element<'a, Message> {
    Element::new(Inert { content: content.into(), still })
}

impl<Message> iced::advanced::Widget<Message, Theme, Renderer> for Inert<'_, Message> {
    fn children(&self) -> Vec<iced::advanced::widget::Tree> {
        vec![iced::advanced::widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut iced::advanced::widget::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(&mut self, tree: &mut iced::advanced::widget::Tree, renderer: &Renderer, limits: &iced::advanced::layout::Limits) -> iced::advanced::layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(&mut self, tree: &mut iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, renderer: &Renderer, operation: &mut dyn iced::advanced::widget::Operation) {
        if !self.still {
            self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
        }
    }

    fn update(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        event: &iced::Event,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if !self.still || matches!(event, iced::Event::Window(_)) {
            let cursor = if self.still { mouse::Cursor::Unavailable } else { cursor };
            self.content.as_widget_mut().update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);
        }
    }

    fn mouse_interaction(&self, tree: &iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        if self.still {
            return mouse::Interaction::None;
        }
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let cursor = if self.still { mouse::Cursor::Unavailable } else { cursor };
        self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        if self.still {
            return None;
        }
        self.content.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

pub struct Under<'a, Message> {
    anchor: Element<'a, Message>,
    sheet: Option<Element<'a, Message>>,
    gap: f32,
    lean: Lean,
    dismiss: Option<Message>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lean {
    Left(f32),
    Right,
}

pub fn under<'a, Message: Clone + 'a>(anchor: impl Into<Element<'a, Message>>, sheet: Option<Element<'a, Message>>, dismiss: Message) -> Element<'a, Message> {
    Element::new(Under { anchor: anchor.into(), sheet, gap: 6.0, lean: Lean::Left(0.0), dismiss: Some(dismiss) })
}

pub fn under_leaning<'a, Message: Clone + 'a>(anchor: impl Into<Element<'a, Message>>, sheet: Option<Element<'a, Message>>, dismiss: Message, lean: Lean, gap: f32) -> Element<'a, Message> {
    Element::new(Under { anchor: anchor.into(), sheet, gap, lean, dismiss: Some(dismiss) })
}

pub fn under_staying<'a, Message: Clone + 'a>(anchor: impl Into<Element<'a, Message>>, sheet: Option<Element<'a, Message>>, lean: Lean, gap: f32) -> Element<'a, Message> {
    Element::new(Under { anchor: anchor.into(), sheet, gap, lean, dismiss: None })
}

const UNDER_EDGE: f32 = 8.0;
const DROP_TIME: f32 = 0.18;
const DROP_RISE: f32 = 10.0;

#[derive(Debug, Default)]
struct Dropping {
    since: Option<std::time::Instant>,
    shown: Option<f32>,
    was_shut: bool,
}

impl Dropping {
    fn eased(&self) -> f32 {
        let x = self.shown.unwrap_or(1.0).clamp(0.0, 1.0);
        1.0 - (1.0 - x).powi(3)
    }
}

pub(crate) fn under_place(anchor: Rectangle, sheet: Size, window: Size, gap: f32, lean: Lean) -> Point {
    let from = match lean {
        Lean::Left(shift) => anchor.x + shift,
        Lean::Right => anchor.x + anchor.width - sheet.width,
    };
    let x = from.min(window.width - sheet.width - UNDER_EDGE).max(UNDER_EDGE);
    let below = anchor.y + anchor.height + gap;
    let y = if below + sheet.height <= window.height - UNDER_EDGE { below } else { (anchor.y - gap - sheet.height).max(UNDER_EDGE) };
    Point::new(x, y)
}

impl<Message: Clone> iced::advanced::Widget<Message, Theme, Renderer> for Under<'_, Message> {
    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        iced::advanced::widget::tree::Tag::of::<Dropping>()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        iced::advanced::widget::tree::State::new(Dropping::default())
    }

    fn children(&self) -> Vec<iced::advanced::widget::Tree> {
        vec![iced::advanced::widget::Tree::new(&self.anchor), self.sheet.as_ref().map_or_else(iced::advanced::widget::Tree::empty, iced::advanced::widget::Tree::new)]
    }

    fn diff(&self, tree: &mut iced::advanced::widget::Tree) {
        if tree.children.len() != 2 {
            tree.children = self.children();
            return;
        }
        tree.children[0].diff(&self.anchor);
        match &self.sheet {
            Some(sheet) => tree.children[1].diff(sheet),
            None => tree.children[1] = iced::advanced::widget::Tree::empty(),
        }
    }

    fn size(&self) -> Size<Length> {
        self.anchor.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.anchor.as_widget().size_hint()
    }

    fn layout(&mut self, tree: &mut iced::advanced::widget::Tree, renderer: &Renderer, limits: &iced::advanced::layout::Limits) -> iced::advanced::layout::Node {
        self.anchor.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(&mut self, tree: &mut iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, renderer: &Renderer, operation: &mut dyn iced::advanced::widget::Operation) {
        self.anchor.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        event: &iced::Event,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.anchor.as_widget_mut().update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);
    }

    fn mouse_interaction(&self, tree: &iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        self.anchor.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.anchor.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        let iced::advanced::widget::Tree { state, children, .. } = tree;
        let dropping = state.downcast_mut::<Dropping>();
        let (first, second) = children.split_at_mut(1);
        match self.sheet.as_mut() {
            Some(sheet) => Some(iced::advanced::overlay::Element::new(Box::new(UnderSheet { sheet, tree: &mut second[0], dropping, anchor: layout.bounds() + translation, gap: self.gap, lean: self.lean, dismiss: self.dismiss.clone() }))),
            None => {
                *dropping = Dropping { was_shut: true, ..Dropping::default() };
                self.anchor.as_widget_mut().overlay(&mut first[0], layout, renderer, viewport, translation)
            }
        }
    }
}

struct UnderSheet<'a, 'b, Message> {
    sheet: &'b mut Element<'a, Message>,
    tree: &'b mut iced::advanced::widget::Tree,
    dropping: &'b mut Dropping,
    anchor: Rectangle,
    gap: f32,
    lean: Lean,
    dismiss: Option<Message>,
}

impl<Message: Clone> iced::advanced::Overlay<Message, Theme, Renderer> for UnderSheet<'_, '_, Message> {
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> iced::advanced::layout::Node {
        let limits = iced::advanced::layout::Limits::new(Size::ZERO, Size::new((bounds.width - 2.0 * UNDER_EDGE).max(0.0), (bounds.height - 2.0 * UNDER_EDGE).max(0.0)));
        let node = self.sheet.as_widget_mut().layout(self.tree, renderer, &limits);
        let at = under_place(self.anchor, node.size(), bounds, self.gap, self.lean);
        node.move_to(at)
    }

    fn draw(&self, renderer: &mut Renderer, theme: &Theme, style: &iced::advanced::renderer::Style, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor) {
        use iced::advanced::Renderer as _;
        let eased = self.dropping.eased();
        if eased >= 0.999 {
            self.sheet.as_widget().draw(self.tree, renderer, theme, style, layout, cursor, &layout.bounds());
            return;
        }
        let bounds = layout.bounds();
        let below = bounds.y >= self.anchor.y;
        let shift = (1.0 - eased) * DROP_RISE * if below { -1.0 } else { 1.0 };
        renderer.with_translation(iced::Vector::new(0.0, shift), |renderer| {
            self.sheet.as_widget().draw(self.tree, renderer, theme, style, layout, cursor, &bounds);
            renderer.with_layer(bounds, |renderer| {
                renderer.fill_quad(iced::advanced::renderer::Quad { bounds, border: iced::Border { radius: 14.0.into(), ..iced::Border::default() }, ..iced::advanced::renderer::Quad::default() }, Color { a: 1.0 - eased, ..theme::GROUND });
            });
        });
    }

    fn operate(&mut self, layout: iced::advanced::Layout<'_>, renderer: &Renderer, operation: &mut dyn iced::advanced::widget::Operation) {
        self.sheet.as_widget_mut().operate(self.tree, layout, renderer, operation);
    }

    fn update(&mut self, event: &iced::Event, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, renderer: &Renderer, clipboard: &mut dyn iced::advanced::Clipboard, shell: &mut iced::advanced::Shell<'_, Message>) {
        if let (true, iced::Event::Window(iced::window::Event::RedrawRequested(now))) = (self.dropping.was_shut, event) {
            let since = *self.dropping.since.get_or_insert(*now);
            let shown = (now.saturating_duration_since(since).as_secs_f32() / DROP_TIME).min(1.0);
            self.dropping.shown = Some(shown);
            if shown < 1.0 {
                shell.request_redraw();
            }
        }
        let viewport = layout.bounds();
        self.sheet.as_widget_mut().update(self.tree, event, layout, cursor, renderer, clipboard, shell, &viewport);
        if let iced::Event::Mouse(mouse::Event::ButtonPressed(_)) = event {
            if cursor.is_over(viewport) {
                shell.capture_event();
            } else if let Some(dismiss) = self.dismiss.clone().filter(|_| !cursor.is_over(self.anchor)) {
                shell.publish(dismiss);
                shell.capture_event();
            }
        }
    }

    fn mouse_interaction(&self, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, renderer: &Renderer) -> mouse::Interaction {
        let inside = self.sheet.as_widget().mouse_interaction(self.tree, layout, cursor, &layout.bounds(), renderer);
        if inside == mouse::Interaction::None && cursor.is_over(layout.bounds()) { mouse::Interaction::Idle } else { inside }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Glow {
    pub radius: f32,
    pub edge: Color,
    pub wash: Color,
    pub shadow: Color,
    pub lift: f32,
    pub scale: f32,
}

impl Glow {
    pub fn card(radius: f32) -> Glow {
        Glow { radius, edge: Color::from_rgba(1.0, 1.0, 1.0, 0.16), wash: Color::TRANSPARENT, shadow: Color::from_rgba(0.0, 0.0, 0.0, 0.45), lift: 3.0, scale: 1.0 }
    }

    pub fn row(radius: f32) -> Glow {
        Glow { radius, edge: Color::TRANSPARENT, wash: Color::from_rgba(1.0, 1.0, 1.0, 0.035), shadow: Color::TRANSPARENT, lift: 0.0, scale: 1.0 }
    }

    pub fn tile(radius: f32) -> Glow {
        Glow { radius, edge: Color::from_rgba(1.0, 1.0, 1.0, 0.1), wash: Color::from_rgba(1.0, 1.0, 1.0, 0.025), shadow: Color::TRANSPARENT, lift: 0.0, scale: 1.0 }
    }

    pub fn edge(mut self, colour: Color) -> Glow {
        self.edge = colour;
        self
    }

    pub fn shadow(mut self, colour: Color) -> Glow {
        self.shadow = colour;
        self
    }

    pub fn lift(mut self, lift: f32) -> Glow {
        self.lift = lift;
        self
    }

    pub fn scale(mut self, scale: f32) -> Glow {
        self.scale = scale.max(1.0);
        self
    }
}

pub fn calm(style: impl Fn(&Theme, button::Status) -> button::Style + 'static) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| match status {
        button::Status::Hovered | button::Status::Pressed => style(theme, button::Status::Active),
        other => style(theme, other),
    }
}

pub struct Hover<'a, Message> {
    content: Element<'a, Message>,
    glow: Glow,
}

pub fn hover<'a, Message: 'a>(content: impl Into<Element<'a, Message>>, glow: Glow) -> Element<'a, Message> {
    let k = fade();
    let dim = |colour: Color| Color { a: colour.a * k, ..colour };
    Element::new(Hover { content: content.into(), glow: Glow { edge: dim(glow.edge), wash: dim(glow.wash), shadow: dim(glow.shadow), ..glow } })
}

#[derive(Debug, Default)]
struct HoverState {
    over: bool,
    lit: f32,
    last: Option<std::time::Instant>,
}

impl<Message> iced::advanced::Widget<Message, Theme, Renderer> for Hover<'_, Message> {
    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        iced::advanced::widget::tree::Tag::of::<HoverState>()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        iced::advanced::widget::tree::State::new(HoverState::default())
    }

    fn children(&self) -> Vec<iced::advanced::widget::Tree> {
        vec![iced::advanced::widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut iced::advanced::widget::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(&mut self, tree: &mut iced::advanced::widget::Tree, renderer: &Renderer, limits: &iced::advanced::layout::Limits) -> iced::advanced::layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(&mut self, tree: &mut iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, renderer: &Renderer, operation: &mut dyn iced::advanced::widget::Operation) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        event: &iced::Event,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);
        if let iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            if let Some(at) = cursor.position_over(layout.bounds()) {
                PRESSED.with(|pressed| pressed.set(Some((layout.bounds(), at))));
            }
        }
        let state = tree.state.downcast_mut::<HoverState>();
        match event {
            iced::Event::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::CursorLeft | mouse::Event::WheelScrolled { .. }) => {
                let over = matches!(event, iced::Event::Mouse(mouse::Event::CursorLeft)).then_some(false).unwrap_or_else(|| cursor.is_over(layout.bounds()));
                if over != state.over {
                    state.over = over;
                    state.last = None;
                    shell.request_redraw();
                }
            }
            iced::Event::Window(iced::window::Event::RedrawRequested(now)) => {
                let target = if state.over { 1.0 } else { 0.0 };
                if (state.lit - target).abs() < 0.003 {
                    state.lit = target;
                    state.last = None;
                } else {
                    let dt = elapsed(&mut state.last, *now);
                    state.lit = toward(state.lit, target, 0.2, dt);
                    shell.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(&self, tree: &iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        let lit = tree.state.downcast_ref::<HoverState>().lit;
        if lit < 0.002 {
            self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
            return;
        }
        let glow = self.glow;
        let rise = iced::Vector::new(0.0, -glow.lift * lit);
        let bounds = layout.bounds() + rise;
        let quad = |border: iced::Border, shadow: iced::Shadow| iced::advanced::renderer::Quad { bounds, border, shadow, snap: true };
        if glow.shadow.a > 0.0 {
            renderer.fill_quad(
                quad(iced::Border { radius: glow.radius.into(), ..iced::Border::default() }, iced::Shadow { color: Color { a: glow.shadow.a * lit, ..glow.shadow }, offset: iced::Vector::new(0.0, 8.0 * lit), blur_radius: 22.0 }),
                iced::Background::Color(Color::TRANSPARENT),
            );
        }
        if glow.wash.a > 0.0 {
            renderer.fill_quad(quad(iced::Border { radius: glow.radius.into(), ..iced::Border::default() }, iced::Shadow::default()), iced::Background::Color(Color { a: glow.wash.a * lit, ..glow.wash }));
        }
        if glow.lift > 0.0 || glow.scale > 1.0 {
            let centre = bounds.center();
            let transformation = iced::Transformation::translate(centre.x, centre.y - glow.lift * lit)
                * iced::Transformation::scale(1.0 + (glow.scale - 1.0) * lit)
                * iced::Transformation::translate(-centre.x, -centre.y);
            let seen = *viewport - rise;
            renderer.with_transformation(transformation, |renderer| {
                self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, &seen);
            });
        } else {
            self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
        }
        if glow.edge.a > 0.0 {
            if let Some(reach) = bounds.expand(2.0).intersection(viewport) {
                renderer.with_layer(reach, |renderer| {
                    renderer.fill_quad(quad(iced::Border { color: Color { a: glow.edge.a * lit, ..glow.edge }, width: 1.0, radius: glow.radius.into() }, iced::Shadow::default()), iced::Background::Color(Color::TRANSPARENT));
                });
            }
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Pill {
    pub fill: Color,
    pub edge: Color,
    pub radius: f32,
    pub underline: Option<f32>,
}

pub struct Slide<'a, Message> {
    content: Element<'a, Message>,
    active: usize,
    pill: Pill,
}

pub fn sliding<'a, Message: 'a>(content: impl Into<Element<'a, Message>>, active: usize, pill: Pill) -> Element<'a, Message> {
    let k = fade();
    let pill = Pill { fill: Color { a: pill.fill.a * k, ..pill.fill }, edge: Color { a: pill.edge.a * k, ..pill.edge }, ..pill };
    Element::new(Slide { content: content.into(), active, pill })
}

const SLIDE: f32 = 0.26;

#[derive(Debug, Default)]
struct SlideState {
    to: Option<usize>,
    from: Option<Rectangle>,
    started: Option<std::time::Instant>,
    now: Option<std::time::Instant>,
}

fn slots(layout: iced::advanced::Layout<'_>) -> Vec<Rectangle> {
    let origin = layout.bounds().position();
    layout.children().map(|child| child.bounds()).map(|bounds| Rectangle { x: bounds.x - origin.x, y: bounds.y - origin.y, ..bounds }).collect()
}

fn blend(from: Rectangle, to: Rectangle, k: f32) -> Rectangle {
    let at = |a: f32, b: f32| a + (b - a) * k;
    Rectangle { x: at(from.x, to.x), y: at(from.y, to.y), width: at(from.width, to.width), height: at(from.height, to.height) }
}

impl SlideState {
    fn shown(&self, slots: &[Rectangle]) -> Option<Rectangle> {
        let target = *slots.get(self.to?)?;
        let from = self.from.unwrap_or(target);
        let spent = self.now.zip(self.started).map_or(SLIDE, |(now, started)| now.saturating_duration_since(started).as_secs_f32());
        let k = (spent / SLIDE).clamp(0.0, 1.0);
        Some(blend(from, target, 1.0 - (1.0 - k).powi(3)))
    }
}

impl<Message> iced::advanced::Widget<Message, Theme, Renderer> for Slide<'_, Message> {
    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        iced::advanced::widget::tree::Tag::of::<SlideState>()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        iced::advanced::widget::tree::State::new(SlideState::default())
    }

    fn children(&self) -> Vec<iced::advanced::widget::Tree> {
        vec![iced::advanced::widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut iced::advanced::widget::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(&mut self, tree: &mut iced::advanced::widget::Tree, renderer: &Renderer, limits: &iced::advanced::layout::Limits) -> iced::advanced::layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(&mut self, tree: &mut iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, renderer: &Renderer, operation: &mut dyn iced::advanced::widget::Operation) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        event: &iced::Event,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);
        if let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            let state = tree.state.downcast_mut::<SlideState>();
            let slots = slots(layout);
            state.now = Some(*now);
            match state.to {
                None => state.to = Some(self.active),
                Some(to) if to != self.active => {
                    state.from = state.shown(&slots);
                    state.to = Some(self.active);
                    state.started = Some(*now);
                }
                _ => {}
            }
            if state.started.is_some_and(|started| now.saturating_duration_since(started).as_secs_f32() < SLIDE) {
                shell.request_redraw();
            }
        }
    }

    fn mouse_interaction(&self, tree: &iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        let state = tree.state.downcast_ref::<SlideState>();
        let slots = slots(layout);
        let shown = state.shown(&slots).or_else(|| slots.get(self.active).copied());
        if let Some(shown) = shown {
            let origin = layout.bounds().position();
            let mut bounds = Rectangle { x: shown.x + origin.x, y: shown.y + origin.y, ..shown };
            if let Some(inset) = self.pill.underline {
                bounds = Rectangle { y: bounds.y + bounds.height - inset - 2.0, height: 2.0, ..bounds };
            }
            renderer.fill_quad(
                iced::advanced::renderer::Quad { bounds, border: iced::Border { color: self.pill.edge, width: if self.pill.edge.a > 0.0 { 1.0 } else { 0.0 }, radius: self.pill.radius.into() }, shadow: iced::Shadow::default(), snap: true },
                iced::Background::Color(self.pill.fill),
            );
        }
        self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

pub struct Tapped<'a, Message> {
    content: Element<'a, Message>,
    on_tap: Box<dyn Fn(Point, Option<Rectangle>) -> Message + 'a>,
}

pub fn tapped<'a, Message: 'a>(content: impl Into<Element<'a, Message>>, on_tap: impl Fn(Point, Option<Rectangle>) -> Message + 'a) -> Element<'a, Message> {
    Element::new(Tapped { content: content.into(), on_tap: Box::new(on_tap) })
}

impl<Message> iced::advanced::Widget<Message, Theme, Renderer> for Tapped<'_, Message> {
    fn children(&self) -> Vec<iced::advanced::widget::Tree> {
        vec![iced::advanced::widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut iced::advanced::widget::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(&mut self, tree: &mut iced::advanced::widget::Tree, renderer: &Renderer, limits: &iced::advanced::layout::Limits) -> iced::advanced::layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(&mut self, tree: &mut iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, renderer: &Renderer, operation: &mut dyn iced::advanced::widget::Operation) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        event: &iced::Event,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let press = matches!(event, iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)));
        if press {
            PRESSED.with(|pressed| pressed.set(None));
        }
        self.content.as_widget_mut().update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);
        if press {
            if let Some(at) = cursor.position_over(layout.bounds()) {
                let card = PRESSED.with(|pressed| pressed.take()).map(|(bounds, seen)| Rectangle { x: bounds.x + at.x - seen.x, y: bounds.y + at.y - seen.y, ..bounds });
                shell.publish((self.on_tap)(at, card));
            }
        }
    }

    fn mouse_interaction(&self, tree: &iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut iced::advanced::widget::Tree,
        layout: iced::advanced::Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: iced::Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

pub struct Reveal<'a, Message> {
    content: Element<'a, Message>,
    k: f32,
}

pub fn reveal<'a, Message: 'a>(k: f32, build: impl FnOnce() -> Element<'a, Message>) -> Element<'a, Message> {
    let k = k.clamp(0.0, 1.0);
    let content = if k >= 0.999 { build() } else { fading(fade() * k, build) };
    Element::new(Reveal { content, k })
}

impl<Message> iced::advanced::Widget<Message, Theme, Renderer> for Reveal<'_, Message> {
    fn children(&self) -> Vec<iced::advanced::widget::Tree> {
        vec![iced::advanced::widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut iced::advanced::widget::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        Size { width: self.content.as_widget().size().width, height: Length::Shrink }
    }

    fn layout(&mut self, tree: &mut iced::advanced::widget::Tree, renderer: &Renderer, limits: &iced::advanced::layout::Limits) -> iced::advanced::layout::Node {
        let inner = self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits);
        let full = inner.size();
        iced::advanced::layout::Node::with_children(Size::new(full.width, full.height * self.k), vec![inner])
    }

    fn operate(&mut self, tree: &mut iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, renderer: &Renderer, operation: &mut dyn iced::advanced::widget::Operation) {
        if let Some(inner) = layout.children().next() {
            self.content.as_widget_mut().operate(&mut tree.children[0], inner, renderer, operation);
        }
    }

    fn update(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        event: &iced::Event,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if self.k < 0.999 {
            return;
        }
        if let Some(inner) = layout.children().next() {
            self.content.as_widget_mut().update(&mut tree.children[0], event, inner, cursor, renderer, clipboard, shell, viewport);
        }
    }

    fn mouse_interaction(&self, tree: &iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        layout.children().next().map_or(mouse::Interaction::None, |inner| self.content.as_widget().mouse_interaction(&tree.children[0], inner, cursor, viewport, renderer))
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        let Some(inner) = layout.children().next() else {
            return;
        };
        if self.k >= 0.999 {
            self.content.as_widget().draw(&tree.children[0], renderer, theme, style, inner, cursor, viewport);
            return;
        }
        if let Some(clip) = layout.bounds().intersection(viewport) {
            renderer.with_layer(clip, |renderer| {
                self.content.as_widget().draw(&tree.children[0], renderer, theme, style, inner, cursor, &clip);
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sliding_highlight_retargets_from_the_visible_position_and_settles_after_resize() {
        use iced::advanced::{Widget, widget::Tree, layout::Limits, Layout, Shell, renderer::Headless};
        let renderer = iced_test::futures::futures::executor::block_on(Renderer::new(theme::SANS, iced::Pixels(14.0), Some("tiny-skia"))).unwrap();
        let make = |active| Slide::<()> {
            content: row![Space::new().width(80.0).height(30.0), Space::new().width(120.0).height(30.0), Space::new().width(60.0).height(30.0)].spacing(6).into(),
            active,
            pill: Pill { fill: ACCENT, edge: Color::TRANSPARENT, radius: 8.0, underline: None },
        };
        let mut slide = make(0);
        let mut tree = Tree::new(&slide as &dyn Widget<(), Theme, Renderer>);
        let start = std::time::Instant::now();
        let viewport = Rectangle::new(Point::ORIGIN, Size::new(400.0, 100.0));
        let frame = |slide: &mut Slide<'_, ()>, tree: &mut Tree, now| {
            let node = slide.layout(tree, &renderer, &Limits::new(Size::ZERO, viewport.size()));
            let layout = Layout::new(&node);
            let mut messages = Vec::new();
            let mut shell = Shell::new(&mut messages);
            slide.update(tree, &iced::Event::Window(iced::window::Event::RedrawRequested(now)), layout, mouse::Cursor::Unavailable, &renderer, &mut iced::advanced::clipboard::Null, &mut shell, &viewport);
            tree.state.downcast_ref::<SlideState>().shown(&slots(layout)).unwrap()
        };
        let first = frame(&mut slide, &mut tree, start);
        slide.active = 1;
        assert_eq!(frame(&mut slide, &mut tree, start), first, "changing selection cannot jump to its destination");
        let half = start + std::time::Duration::from_millis(130);
        let shown = frame(&mut slide, &mut tree, half);
        assert!(shown.x > first.x && shown.x < 86.0);
        slide.active = 2;
        assert_eq!(frame(&mut slide, &mut tree, half), shown, "a rapid second click must continue from the visible highlight");
        slide.content = row![Space::new().width(100.0).height(36.0), Space::new().width(160.0).height(36.0), Space::new().width(90.0).height(36.0)].spacing(6).into();
        slide.diff(&mut tree);
        let settled = frame(&mut slide, &mut tree, half + std::time::Duration::from_secs(1));
        assert_eq!(settled, Rectangle { x: 272.0, y: 0.0, width: 90.0, height: 36.0 });
        assert_eq!(frame(&mut slide, &mut tree, half + std::time::Duration::from_secs(2)), settled);
    }

    #[test]
    fn a_card_whose_content_grows_takes_its_new_height_gradually() {
        use iced::advanced::{Widget, widget::Tree, layout::Limits, Layout, Shell, renderer::Headless};
        let renderer = iced_test::futures::futures::executor::block_on(Renderer::new(theme::SANS, iced::Pixels(14.0), Some("tiny-skia"))).unwrap();
        let of = |high: f32| Smooth::<()> { content: Space::new().width(200.0).height(high).into() };
        let mut card = of(80.0);
        let mut tree = Tree::new(&card as &dyn Widget<(), Theme, Renderer>);
        let start = std::time::Instant::now();
        let viewport = Rectangle::new(Point::ORIGIN, Size::new(400.0, 600.0));
        let frame = |card: &mut Smooth<'_, ()>, tree: &mut Tree, now| {
            let limits = Limits::new(Size::ZERO, viewport.size());
            let node = card.layout(tree, &renderer, &limits);
            let mut messages = Vec::new();
            let mut shell = Shell::new(&mut messages);
            card.update(tree, &iced::Event::Window(iced::window::Event::RedrawRequested(now)), Layout::new(&node), mouse::Cursor::Unavailable, &renderer, &mut iced::advanced::clipboard::Null, &mut shell, &viewport);
            let asks = shell.is_layout_invalid();
            (card.layout(tree, &renderer, &limits).size().height, asks)
        };
        assert_eq!(frame(&mut card, &mut tree, start), (80.0, false), "the first layout is the content's own height and asks for nothing");

        card = of(160.0);
        card.diff(&mut tree);
        let later = start + std::time::Duration::from_secs(3);
        assert_eq!(frame(&mut card, &mut tree, later), (80.0, true), "new content cannot take its room in one frame, however long the card sat still before");
        let (half, moving) = frame(&mut card, &mut tree, later + std::time::Duration::from_secs_f32(RESIZE / 2.0));
        assert!(half > 80.0 && half < 160.0 && moving, "{half}");
        card = of(40.0);
        card.diff(&mut tree);
        let (turned, _) = frame(&mut card, &mut tree, later + std::time::Duration::from_secs_f32(RESIZE / 2.0));
        assert_eq!(turned, half, "content that changes again mid-way is followed from the height on screen");
        assert_eq!(frame(&mut card, &mut tree, later + std::time::Duration::from_secs(1)), (40.0, false), "and once it has arrived the card asks for no more frames");
    }

    #[test]
    fn the_scale_follows_the_monitor_and_the_window_grows_only_when_it_must() {
        assert_eq!(auto_scale_for(1080.0), 100);
        assert_eq!(auto_scale_for(1440.0), 120);
        assert_eq!(auto_scale_for(768.0), 80);
        assert_eq!(auto_scale_for(982.0), 95);
        assert_eq!(auto_scale_for(4320.0), 160);
        let least = Size::new(980.0, 720.0);
        assert_eq!(refit(Size::new(980.0, 720.0), 1.0, 1.2, least), Some(least));
        assert_eq!(refit(Size::new(1600.0, 1000.0), 1.0, 1.2, least), None);
        assert_eq!(refit(Size::new(980.0, 720.0), 1.2, 1.0, least), None);
        assert_eq!(refit(Size::new(980.0, 720.0), 1.0, 1.0, least), None);
    }

    #[test]
    fn repeated_resize_events_do_not_compound_the_automatic_scale() {
        let mut scale = 1.0;
        for (height, window) in [(1080.0, Size::new(1920.0, 1080.0)), (768.0, Size::new(1280.0, 720.0)), (1440.0, Size::new(2560.0, 1440.0)), (982.0, Size::new(980.0, 720.0))] {
            let event = viewport_at(window, 1.0, scale);
            let next = auto_scale_for(height) as f32 / 100.0;
            let mut viewport = viewport_at(event, scale, next);
            scale = next;
            for _ in 0..20 {
                let next = auto_scale_for(height) as f32 / 100.0;
                assert_eq!(next, scale);
                viewport = viewport_at(viewport, scale, next);
            }
            let restored = viewport_at(viewport, scale, 1.0);
            assert!((restored.width - window.width).abs() < 0.001);
            assert!((restored.height - window.height).abs() < 0.001);
        }
    }

    #[test]
    fn scaled_dropdowns_open_at_the_control_and_accept_the_visible_choice() {
        use iced::widget::{container, pick_list};
        for factor in [0.85, 1.17, 1.5, 2.0, 3.0] {
            let menu = pick_list(["NM", "HD", "DT"], Some("NM"), str::to_owned).text_size(16.0).text_line_height(text::LineHeight::Relative(1.25)).padding(8).width(120.0);
            let content = container(menu).padding([44, 63]);
            let mut screen = iced_test::Simulator::with_size(crate::settings(), Size::new(1280.0, 720.0), scaled(content, factor));
            screen.point_at(Point::new(83.0 * factor, 62.0 * factor));
            let _ = screen.simulate(iced_test::simulator::click());
            let position = Point::new(83.0 * factor, 170.0 * factor);
            screen.point_at(position);
            let _ = screen.simulate([iced::Event::Mouse(mouse::Event::CursorMoved { position })]);
            let _ = screen.simulate(iced_test::simulator::click());
            assert_eq!(screen.into_messages().collect::<Vec<_>>(), ["DT"]);
        }
        let menu = pick_list(["NM", "HD", "DT"], Some("NM"), str::to_owned).text_size(16.0).text_line_height(text::LineHeight::Relative(1.25)).padding(8).width(120.0);
        let content = container(scaled(container(menu).padding([44, 63]), 0.95)).padding([25, 31]);
        let mut screen = iced_test::Simulator::with_size(crate::settings(), Size::new(1280.0, 720.0), scaled(content, 1.5));
        screen.point_at(Point::new((31.0 + 83.0 * 0.95) * 1.5, (25.0 + 62.0 * 0.95) * 1.5));
        let _ = screen.simulate(iced_test::simulator::click());
        let position = Point::new((31.0 + 83.0 * 0.95) * 1.5, (25.0 + 170.0 * 0.95) * 1.5);
        screen.point_at(position);
        let _ = screen.simulate([iced::Event::Mouse(mouse::Event::CursorMoved { position })]);
        let _ = screen.simulate(iced_test::simulator::click());
        assert_eq!(screen.into_messages().collect::<Vec<_>>(), ["DT"]);
    }

    #[test]
    fn sensed_hover_does_not_reannounce_when_the_pointer_moves_inside() {
        assert!(!sensed_changed(true, true, true, false));
        assert!(sensed_changed(true, true, true, true));
        assert!(sensed_changed(true, false, false, false));
        assert!(sensed_changed(false, true, false, false));
    }
}

thread_local! {
    static PLACES: std::cell::RefCell<std::collections::HashMap<String, f32>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

pub fn places() -> std::collections::HashMap<String, f32> {
    PLACES.with(|places| places.borrow().clone())
}

pub struct Flip<'a, Message> {
    content: Element<'a, Message>,
    key: String,
    from: Option<f32>,
    k: f32,
}

pub fn flip<'a, Message: 'a>(content: impl Into<Element<'a, Message>>, key: String, from: Option<f32>, k: f32) -> Flip<'a, Message> {
    Flip { content: content.into(), key, from, k: k.clamp(0.0, 1.0) }
}

impl<Message> iced::advanced::Widget<Message, Theme, Renderer> for Flip<'_, Message> {
    fn children(&self) -> Vec<iced::advanced::widget::Tree> {
        vec![iced::advanced::widget::Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut iced::advanced::widget::Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(&mut self, tree: &mut iced::advanced::widget::Tree, renderer: &Renderer, limits: &iced::advanced::layout::Limits) -> iced::advanced::layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(&mut self, tree: &mut iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, renderer: &Renderer, operation: &mut dyn iced::advanced::widget::Operation) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        event: &iced::Event,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);
    }

    fn mouse_interaction(&self, tree: &iced::advanced::widget::Tree, layout: iced::advanced::Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let x = layout.bounds().x;
        PLACES.with(|places| {
            places.borrow_mut().insert(self.key.clone(), x);
        });
        match self.from.filter(|_| self.k < 1.0) {
            Some(from) => {
                let shift = iced::Vector::new((from - x) * (1.0 - self.k), 0.0);
                iced::advanced::Renderer::with_translation(renderer, shift, |renderer| {
                    self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, &Rectangle { x: viewport.x - shift.x, ..*viewport });
                });
            }
            None => self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport),
        }
    }
}

impl<'a, Message: 'a> From<Flip<'a, Message>> for Element<'a, Message> {
    fn from(flip: Flip<'a, Message>) -> Element<'a, Message> {
        Element::new(flip)
    }
}

#[cfg(test)]
mod glass_tests {
    use super::*;

    #[test]
    fn the_picture_in_a_glass_panel_is_cut_to_its_rounded_shape_and_taken_from_where_the_panel_stands() {
        let (wide, high) = (96u32, 60u32);
        let mut pixels = Vec::with_capacity((wide * high * 4) as usize);
        for _ in 0..high {
            for x in 0..wide {
                pixels.extend_from_slice(if x < wide / 2 { &[200, 20, 20, 255] } else { &[20, 20, 200, 255] });
            }
        }
        let window = Size::new(1280.0, 800.0);
        let at = |cut: &(u32, u32, Vec<u8>), x: u32, y: u32| -> [u8; 4] {
            let from = ((y * cut.0 + x) * 4) as usize;
            [cut.2[from], cut.2[from + 1], cut.2[from + 2], cut.2[from + 3]]
        };
        let left = glass_piece(&pixels, wide, high, Rectangle::new(Point::new(40.0, 84.0), Size::new(420.0, 200.0)), window, 18.0, [0.0, 0.0, 0.0], 0.0).unwrap();
        assert_eq!((left.0, left.1, left.2.len()), (420, 200, 420 * 200 * 4), "one picture point for each point of the panel");
        for corner in [(0, 0), (419, 0), (0, 199), (419, 199)] {
            assert_eq!(at(&left, corner.0, corner.1)[3], 0, "the very corner at {corner:?} is outside the rounded shape");
        }
        for inside in [(210, 1), (1, 100), (418, 100), (210, 198), (18, 18), (401, 181), (6, 6)] {
            assert_eq!(at(&left, inside.0, inside.1)[3], 255, "{inside:?} is inside the rounded shape");
        }
        let edge = at(&left, 6, 5);
        assert!(edge[3] > 0 && edge[3] < 255 && edge[0] > 150, "the edge of the corner is soft and keeps its colour: {edge:?}");
        assert_eq!((at(&left, 5, 5)[3], at(&left, 210, 0)[3], at(&left, 419, 100)[3]), (0, 0, 0), "the picture ends a point inside the edge line, so nothing of it shows outside the panel");
        assert!(at(&left, 210, 100)[0] > 150 && at(&left, 210, 100)[2] < 60, "a panel on the left shows the left of the scene");
        let right = glass_piece(&pixels, wide, high, Rectangle::new(Point::new(820.0, 84.0), Size::new(420.0, 200.0)), window, 18.0, [0.0, 0.0, 0.0], 0.0).unwrap();
        assert!(at(&right, 210, 100)[2] > 150 && at(&right, 210, 100)[0] < 60, "a panel on the right shows the right of the scene");
        let across = glass_piece(&pixels, wide, high, Rectangle::new(Point::new(430.0, 84.0), Size::new(420.0, 200.0)), window, 18.0, [0.0, 0.0, 0.0], 0.0).unwrap();
        assert!(at(&across, 40, 100)[0] > 150 && at(&across, 380, 100)[2] > 150, "a panel over the middle shows both");
        let small = glass_piece(&pixels, wide, high, Rectangle::new(Point::ORIGIN, Size::new(20.0, 10.0)), window, 18.0, [0.0, 0.0, 0.0], 0.0).unwrap();
        assert_eq!((small.0, small.1), (20, 10));
        assert_eq!(at(&small, 10, 5)[3], 255, "a panel smaller than its rounding still has a middle");
        assert!(glass_piece(&pixels, wide, high, Rectangle::new(Point::ORIGIN, Size::new(0.0, 10.0)), window, 18.0, [0.0, 0.0, 0.0], 0.0).is_none());
        assert!(glass_piece(&pixels[..16], wide, high, Rectangle::new(Point::ORIGIN, Size::new(20.0, 10.0)), window, 18.0, [0.0, 0.0, 0.0], 0.0).is_none());
        let dark = glass_piece(&pixels, wide, high, Rectangle::new(Point::new(40.0, 84.0), Size::new(420.0, 200.0)), window, 18.0, [0.086, 0.039, 0.059], 0.74).unwrap();
        let seen = at(&dark, 210, 100);
        assert!(seen[0] > 55 && seen[0] < 80 && seen[2] < 30 && seen[3] == 255, "the dark tint is mixed into the picture itself, so no renderer can lighten it: {seen:?}");
        let cover = glass_cover(Size::new(96.0, 54.0), Size::new(1280.0, 800.0));
        assert!(cover.width >= 1280.0 && cover.height >= 800.0 && cover.x <= 0.0 && cover.y <= 0.0, "the picture covers the window like the scene behind it");
    }
}

#[cfg(test)]
mod under_tests {
    use super::*;

    #[test]
    fn a_sheet_hangs_under_its_anchor_stays_in_the_window_and_goes_above_when_there_is_no_room_below() {
        let window = Size::new(1000.0, 600.0);
        let sheet = Size::new(300.0, 200.0);
        let under = under_place(Rectangle::new(Point::new(100.0, 100.0), Size::new(80.0, 30.0)), sheet, window, 6.0, Lean::Left(0.0));
        assert_eq!(under, Point::new(100.0, 136.0));
        let right = under_place(Rectangle::new(Point::new(900.0, 100.0), Size::new(80.0, 30.0)), sheet, window, 6.0, Lean::Left(0.0));
        assert_eq!(right.x, 1000.0 - 300.0 - 8.0);
        let low = under_place(Rectangle::new(Point::new(100.0, 500.0), Size::new(80.0, 30.0)), sheet, window, 6.0, Lean::Left(0.0));
        assert_eq!(low.y, 500.0 - 6.0 - 200.0);
        let cramped = under_place(Rectangle::new(Point::new(-40.0, 120.0), Size::new(80.0, 30.0)), Size::new(300.0, 590.0), window, 6.0, Lean::Left(0.0));
        assert_eq!(cramped, Point::new(8.0, 8.0));
        assert_eq!(keys_for("Ctrl Shift Z", true), "⇧⌘Z");
        assert_eq!(keys_for("Нажмите Ctrl K или Delete", true), "Нажмите ⌘K или ⌫");
        assert_eq!(keys_for("Ctrl D", false), "Ctrl D");
        let whole = dash_period(1003.0);
        assert!((1003.0 / whole - (1003.0 / whole).round()).abs() < 0.001 && (whole - 8.0).abs() < 0.1, "dashes go round a frame a whole number of times, so the march has no seam");
        let anchor = Rectangle::new(Point::new(500.0, 100.0), Size::new(200.0, 30.0));
        assert_eq!(under_place(anchor, sheet, window, 32.0, Lean::Right), Point::new(400.0, 162.0), "its right edge meets the anchor's");
        assert_eq!(under_place(anchor, sheet, window, 4.0, Lean::Left(60.0)), Point::new(560.0, 134.0), "it stands shifted from the anchor's left edge");
    }
}
