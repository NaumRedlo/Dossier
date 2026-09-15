use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::widget::{button, column, container, image, row, text, Space};
use iced::{mouse, Color, ContentFit, Element, Length, Point, Rectangle, Renderer, Size, Theme};

use crate::theme::{self, ACCENT, FAINT, GROUND, INK, MUTED};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    Dot,
    Tick,
    Cross,
    None,
}

const GLYPH: f32 = 14.0;

impl<Message> canvas::Program<Message> for Glyph {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let centre = Point::new(GLYPH / 2.0, GLYPH / 2.0);
        match self {
            Glyph::Dot => {
                frame.fill(&Path::circle(centre, 8.0), theme::ACCENT_SOFT);
                frame.fill(&Path::circle(centre, 4.0), ACCENT);
            }
            Glyph::Tick => {
                let path = Path::new(|b| {
                    b.move_to(Point::new(3.0, 7.5));
                    b.line_to(Point::new(6.0, 10.5));
                    b.line_to(Point::new(12.0, 3.5));
                });
                frame.stroke(&path, stroke(ACCENT, 1.8));
            }
            Glyph::Cross => {
                let path = Path::new(|b| {
                    b.move_to(Point::new(3.5, 3.5));
                    b.line_to(Point::new(10.5, 10.5));
                    b.move_to(Point::new(10.5, 3.5));
                    b.line_to(Point::new(3.5, 10.5));
                });
                frame.stroke(&path, stroke(ACCENT, 1.8));
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
    Canvas::new(which).width(GLYPH).height(GLYPH).into()
}

pub struct Mark;

impl<Message> canvas::Program<Message> for Mark {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let centre = Point::new(14.0, 14.0);
        frame.stroke(
            &Path::circle(centre, 12.5),
            stroke(Color { a: 0.45, ..ACCENT }, 1.2),
        );
        frame.stroke(&Path::circle(centre, 9.4), stroke(ACCENT, 1.8));
        let letter = Path::new(|b| {
            b.move_to(Point::new(11.0, 9.5));
            b.line_to(Point::new(13.4, 9.5));
            b.arc_to(Point::new(19.5, 9.5), Point::new(19.5, 14.0), 4.5);
            b.arc_to(Point::new(19.5, 18.5), Point::new(13.4, 18.5), 4.5);
            b.line_to(Point::new(11.0, 18.5));
            b.close();
        });
        frame.stroke(&letter, stroke(INK, 2.0));
        let slits = Path::new(|b| {
            b.move_to(Point::new(9.6, 12.6));
            b.line_to(Point::new(14.2, 12.6));
            b.move_to(Point::new(9.6, 15.4));
            b.line_to(Point::new(14.2, 15.4));
        });
        frame.stroke(&slits, stroke(GROUND, 1.2));
        vec![frame.into_geometry()]
    }
}

pub fn brand<'a, Message: 'a>() -> Element<'a, Message> {
    row![
        Canvas::new(Mark).width(28.0).height(28.0),
        text("Dossier").font(theme::SANS_SEMI).size(theme::LEAD).color(INK),
    ]
    .spacing(10)
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

pub fn headline<'a, Message: 'a>(which: Glyph, words: String, count: String) -> Element<'a, Message> {
    row![
        glyph(which),
        text(words).font(theme::SANS_SEMI).size(theme::LEAD).color(INK),
        text(format!("· {count}")).font(theme::SANS).size(theme::LEAD).color(MUTED),
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
}

impl Line {
    pub fn new(mood: Mood, name: impl Into<String>) -> Line {
        Line {
            mood,
            name: name.into(),
            detail: String::new(),
            note: None,
        }
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
    let mut rows = column![].spacing(4);
    for line in lines {
        let (which, colour, face) = match line.mood {
            Mood::Done => (Glyph::Tick, MUTED, theme::MONO),
            Mood::Now => (Glyph::Dot, INK, theme::MONO_BOLD),
            Mood::Todo => (Glyph::None, FAINT, theme::MONO),
            Mood::Failed => (Glyph::Cross, ACCENT, theme::MONO_BOLD),
        };
        let detail_colour = match line.mood {
            Mood::Now => MUTED,
            Mood::Failed => ACCENT,
            _ => FAINT,
        };
        let mut words = row![text(line.name.clone()).font(face).size(theme::BODY).color(colour)].spacing(6);
        if !line.detail.is_empty() {
            words = words.push(text(format!("· {}", line.detail)).font(theme::MONO).size(theme::BODY).color(detail_colour));
        }
        rows = rows.push(
            container(row![glyph(which), words].spacing(12).align_y(iced::Center)).height(24.0),
        );
        if let Some((reason, link)) = &line.note {
            let mut note = row![text(reason.clone()).font(theme::SANS).size(theme::CAPTION).color(MUTED)].spacing(6);
            if let Some(link) = link {
                let go = button(text(link.clone()).font(theme::SANS).size(theme::CAPTION).color(ACCENT))
                    .padding(0)
                    .style(theme::link);
                note = note.push(match on_link.clone() {
                    Some(message) => go.on_press(message),
                    None => go,
                });
            }
            rows = rows.push(container(note).padding(iced::Padding::ZERO.left(26.0).bottom(6.0)));
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
    text(words).font(theme::SANS_SEMI).size(theme::LEAD).color(INK).into()
}

pub fn heading<'a, Message: 'a>(name: String, words: String) -> Element<'a, Message> {
    column![title(name), why(words)].spacing(4).into()
}

pub fn gap<'a, Message: 'a>(height: f32) -> Element<'a, Message> {
    Space::new().height(height).into()
}

pub fn why<'a, Message: 'a>(words: String) -> Element<'a, Message> {
    text(words).font(theme::SANS).size(theme::BODY).color(MUTED).into()
}

pub fn cap<'a, Message: 'a>(words: String) -> Element<'a, Message> {
    text(words).font(theme::SANS).size(theme::CAPTION).color(MUTED).into()
}

pub fn faint<'a, Message: 'a>(words: String) -> Element<'a, Message> {
    text(words).font(theme::SANS).size(theme::CAPTION).color(FAINT).into()
}

pub fn primary<'a, Message: Clone + 'a>(words: String, on: Option<Message>) -> Element<'a, Message> {
    let made = button(container(text(words).font(theme::SANS_SEMI).size(theme::BODY)).center_y(theme::CONTROL_HEIGHT - 2.0))
        .padding([0, 12])
        .style(theme::primary);
    match on {
        Some(message) => made.on_press(message).into(),
        None => made.into(),
    }
}

pub fn quiet<'a, Message: Clone + 'a>(words: String, on: Option<Message>) -> Element<'a, Message> {
    let made = button(container(text(words).font(theme::SANS_SEMI).size(theme::BODY)).center_y(theme::CONTROL_HEIGHT - 2.0))
        .padding([0, 12])
        .style(theme::quiet);
    match on {
        Some(message) => made.on_press(message).into(),
        None => made.into(),
    }
}

pub fn link<'a, Message: Clone + 'a>(words: String, on: Message) -> Element<'a, Message> {
    button(text(words).font(theme::SANS).size(theme::CAPTION))
        .padding(0)
        .style(theme::link)
        .on_press(on)
        .into()
}

pub fn tag<'a, Message: 'a>(words: String) -> Element<'a, Message> {
    container(text(words).font(theme::MONO_BOLD).size(theme::CAPTION).color(MUTED))
        .padding([0, 6])
        .style(theme::tag)
        .into()
}

pub fn tile<'a, Message: 'a>(value: String, label: String) -> Element<'a, Message> {
    container(
        column![
            text(value).font(theme::MONO_BOLD).size(theme::TITLE).color(INK),
            text(label).font(theme::SANS).size(theme::CAPTION).color(MUTED),
        ]
        .spacing(2),
    )
    .padding([12, 16])
    .width(Length::Fill)
    .style(theme::tile)
    .into()
}

pub fn well<'a, Message: 'a>(inside: Element<'a, Message>) -> Element<'a, Message> {
    container(inside)
        .padding([0, 12])
        .height(theme::CONTROL_HEIGHT)
        .width(Length::Fill)
        .center_y(theme::CONTROL_HEIGHT)
        .style(theme::well)
        .into()
}

pub fn grow<'a, Message: 'a>() -> Element<'a, Message> {
    Space::new().width(Length::Fill).into()
}

pub fn qr_pixels(matrix: &[Vec<bool>], scale: u32) -> (u32, u32, Vec<u8>) {
    let n = matrix.len() as u32;
    let side = n * scale;
    let mut out = Vec::with_capacity((side * side * 4) as usize);
    for y in 0..side {
        for x in 0..side {
            let dark = matrix[(y / scale) as usize][(x / scale) as usize];
            let v = if dark { [0x14, 0x07, 0x0a, 0xff] } else { [0xff, 0xff, 0xff, 0xff] };
            out.extend_from_slice(&v);
        }
    }
    (side, side, out)
}

pub fn sized(w: f32, h: f32) -> Size {
    Size::new(w, h)
}
