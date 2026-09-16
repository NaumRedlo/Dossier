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
                Stroke::default().with_width(self.stroke).with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.03)),
            );
            x += self.step;
        }
        vec![frame.into_geometry()]
    }
}

pub fn hatch<'a, Message: 'a>() -> Element<'a, Message> {
    Canvas::new(Hatch { stroke: 8.0, step: 18.0 }).width(Length::Fill).height(Length::Fill).into()
}

pub fn fine_hatch<'a, Message: 'a>() -> Element<'a, Message> {
    Canvas::new(Hatch { stroke: 1.5, step: 5.0 }).width(Length::Fill).height(Length::Fill).into()
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

pub fn veil<'a, Message: 'a>(colour: Color) -> Element<'a, Message> {
    Veil::new(colour).into()
}
