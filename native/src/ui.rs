use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::widget::{button, column, container, image, row, text, Space};
use iced::{mouse, Color, ContentFit, Element, Length, Point, Rectangle, Renderer, Size, Theme};

use crate::theme::{self, ACCENT, FAINT, INK, MUTED};

thread_local! {
    static FADE: std::cell::Cell<f32> = const { std::cell::Cell::new(1.0) };
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
    Color {
        a: colour.a * fade(),
        ..colour
    }
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
const ACCENT_DEEP: Color = Color::from_rgb(0.788, 0.204, 0.184);

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

pub fn brand<'a, Message: 'a>() -> Element<'a, Message> {
    let alpha = fade();
    row![
        Canvas::new(Emblem { alpha }).width(EMBLEM).height(EMBLEM),
        container(Space::new().width(1.0).height(22.0)).style(move |theme| {
            let mut style = theme::rule_high(theme);
            if let Some(iced::Background::Color(c)) = style.background {
                style.background = Some(iced::Background::Color(Color { a: c.a * alpha, ..c }));
            }
            style
        }),
        text("Dossier").font(theme::SANS_SEMI).size(20.0).color(faded(INK)),
    ]
    .spacing(14)
    .align_y(iced::Center)
    .into()
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
        text("·").font(theme::SANS).size(theme::LEAD).color(FAINT),
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
        let mut words = row![text(line.name.clone()).font(face).size(theme::BODY).color(colour)].spacing(7);
        if !line.detail.is_empty() {
            words = words.push(text("·").font(theme::MONO).size(theme::BODY).color(faded(FAINT)));
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

pub fn card<'a, Message: 'a>(top: Element<'a, Message>, bottom: Option<Element<'a, Message>>) -> Element<'a, Message> {
    let mut inside = column![container(top).padding([20, 24])];
    if let Some(bottom) = bottom {
        inside = inside.push(container(Space::new().height(1.0)).width(Length::Fill).style(theme::rule));
        inside = inside.push(container(bottom).padding(24));
    }
    container(inside)
        .width(Length::Fill)
        .style(theme::card)
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

fn dimmed(style: impl Fn(&Theme, button::Status) -> button::Style + 'static, k: f32) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let mut made = style(theme, status);
        made.text_color = Color { a: made.text_color.a * k, ..made.text_color };
        if let Some(iced::Background::Color(colour)) = made.background {
            made.background = Some(iced::Background::Color(Color { a: colour.a * k, ..colour }));
        }
        made.border.color = Color { a: made.border.color.a * k, ..made.border.color };
        made
    }
}

fn dimmed_box(style: impl Fn(&Theme) -> container::Style + 'static, k: f32) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let mut made = style(theme);
        if let Some(iced::Background::Color(colour)) = made.background {
            made.background = Some(iced::Background::Color(Color { a: colour.a * k, ..colour }));
        }
        made.border.color = Color { a: made.border.color.a * k, ..made.border.color };
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
            text(value).font(theme::MONO_BOLD).size(theme::TITLE).color(faded(INK)),
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

pub fn sized(w: f32, h: f32) -> Size {
    Size::new(w, h)
}

pub struct Hatch {
    pub stroke: f32,
    pub step: f32,
    pub alpha: f32,
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
                Stroke::default().with_width(self.stroke).with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.03 * self.alpha.clamp(0.0, 1.0))),
            );
            x += self.step;
        }
        vec![frame.into_geometry()]
    }
}

pub fn hatch<'a, Message: 'a>() -> Element<'a, Message> {
    Canvas::new(Hatch { stroke: 8.0, step: 18.0, alpha: fade() }).width(Length::Fill).height(Length::Fill).into()
}

pub fn fine_hatch<'a, Message: 'a>() -> Element<'a, Message> {
    Canvas::new(Hatch { stroke: 1.5, step: 5.0, alpha: fade() }).width(Length::Fill).height(Length::Fill).into()
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
        frame.stroke(&track, Stroke::default().with_width(2.0).with_color(faded(Color::from_rgba(1.0, 1.0, 1.0, 0.06))));
        if self.len < 1.0 {
            let x0 = bounds.width * self.start;
            let x1 = bounds.width * (self.start + self.len);
            let thumb = Path::line(Point::new(x0, y), Point::new(x1, y));
            let colour = if lit { MUTED } else { Color { a: 0.55, ..MUTED } };
            frame.stroke(&thumb, Stroke::default().with_width(2.0).with_color(faded(colour)).with_line_cap(canvas::LineCap::Round));
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
}

#[derive(Debug, Default)]
struct SensedState {
    inside: bool,
}

pub fn sensed<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    on_enter: impl Fn(Rectangle) -> Message + 'a,
    on_exit: Message,
) -> Sensed<'a, Message> {
    Sensed { content: content.into(), on_enter: Box::new(on_enter), on_exit, lift: 0.0, scale: 1.0 }
}

impl<Message> Sensed<'_, Message> {
    pub fn risen(mut self, lift: f32, scale: f32) -> Self {
        self.lift = lift;
        self.scale = scale;
        self
    }
}

fn about(bounds: Rectangle, anchor: Point, lift: f32, scale: f32) -> iced::Transformation {
    let (cx, cy) = (bounds.x + bounds.width * anchor.x, bounds.y + bounds.height * anchor.y);
    iced::Transformation::translate(cx, cy - lift) * iced::Transformation::scale(scale) * iced::Transformation::translate(-cx, -cy)
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
        self.content
            .as_widget_mut()
            .update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);
        let window = match event {
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) => Some(*position),
            iced::Event::Mouse(mouse::Event::CursorLeft) => None,
            _ => return,
        };
        let bounds = layout.bounds();
        let seen = bounds.intersection(viewport).unwrap_or(Rectangle::new(bounds.position(), Size::ZERO));
        let inside = cursor.is_over(seen);
        let state = tree.state.downcast_mut::<SensedState>();
        if inside != state.inside {
            state.inside = inside;
            if inside {
                let shift = match (cursor.position(), window) {
                    (Some(local), Some(window)) => local - window,
                    _ => iced::Vector::ZERO,
                };
                shell.publish((self.on_enter)(bounds - shift));
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
        let fraction_at = |x: f32| ((x - bounds.x) / bounds.width.max(1.0)).clamp(0.0, 1.0);
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
        let track = Path::line(Point::new(0.0, y), Point::new(bounds.width, y));
        frame.stroke(
            &track,
            Stroke::default().with_width(thick).with_color(dim(Color::from_rgba(1.0, 1.0, 1.0, 0.16), self.alpha)).with_line_cap(canvas::LineCap::Round),
        );
        let x = bounds.width * self.played.clamp(0.0, 1.0);
        if let Some(over) = state.over.filter(|_| lit) {
            let ahead = bounds.width * over;
            if ahead > x {
                frame.stroke(
                    &Path::line(Point::new(x, y), Point::new(ahead, y)),
                    Stroke::default().with_width(thick).with_color(dim(Color { a: 0.3, ..INK }, self.alpha)).with_line_cap(canvas::LineCap::Round),
                );
            }
        }
        if x > 0.5 {
            let done = Path::line(Point::new(0.0, y), Point::new(x, y));
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
        frame.fill(
            &Path::rounded_rectangle(Point::ORIGIN, bounds.size(), (h / 2.0).into()),
            dim(Color::from_rgba(1.0, 1.0, 1.0, 0.06 + 0.05 * glow), self.alpha),
        );
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
        let fraction_at = |x: f32| ((x - bounds.x - 3.0) / (bounds.width - 6.0).max(1.0)).clamp(0.0, 1.0);
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
        let x0 = 3.0;
        let x1 = bounds.width - 3.0;
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
}

#[derive(Debug, Default)]
pub struct StepsState {
    grabbed: bool,
    label_side: f32,
    value_side: f32,
    last: Option<std::time::Instant>,
}

const STEP_INSET: f32 = 5.0;
const HANDLE_ROOM: f32 = 6.0;
const STEP_SNAP: f32 = 0.035;

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
                Some(canvas::Action::capture())
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
        for stop in &self.stops {
            let sx = STEP_INSET + stop * (w - 2.0 * STEP_INSET);
            let near = 1.0 - ((sx - x).abs() / 26.0).clamp(0.0, 1.0);
            let dot = 5.0;
            let width = dot + (wide - dot) * near;
            let height = dot + (h - 2.0 * inset - dot) * near;
            let alpha = (0.2 - 0.14 * near) * (1.0 - near * 0.5);
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
        let label_wide = self.label.chars().count() as f32 * 6.8 + 4.0;
        let label_x = 10.0 + (w - 20.0 - label_wide) * state.label_side;
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
        let value_wide = self.value.chars().count() as f32 * 6.7 + 2.0;
        let right_side = x + wide / 2.0 + 10.0;
        let left_side = x - wide / 2.0 - 10.0 - value_wide;
        let value_x = right_side + (left_side - right_side) * state.value_side;
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
    Canvas::new(Steps { label, value, at, shown, snap, alpha: fade(), stops, on: Box::new(on) })
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
        let centre = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        frame.fill(&Path::circle(centre, side / 2.0), dim(Color::from_rgba(1.0, 1.0, 1.0, 0.1), self.alpha));
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
}

impl<'a, Message: 'a> From<Scaled<'a, Message>> for Element<'a, Message> {
    fn from(scaled: Scaled<'a, Message>) -> Element<'a, Message> {
        Element::new(scaled)
    }
}

pub fn hidden_bar() -> iced::widget::scrollable::Direction {
    iced::widget::scrollable::Direction::Vertical(iced::widget::scrollable::Scrollbar::hidden())
}
