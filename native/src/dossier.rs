use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::widget::{button, column, container, mouse_area, row, scrollable, stack, text, Space};
use iced::{mouse, Background, Border, Color, Element, Length, Padding, Point, Rectangle, Renderer, Shadow, Theme, Vector};

use crate::chronicle::{self, card as slab};
use crate::community::{wire, Board, Person, Rarity, Title};
use crate::community_screen::{self as screen, Ground, Message, Section};
use crate::glyphs::{glyph, Icon};
use crate::theme::{self, ACCENT, FAINT, INK, MUTED};
use crate::ui;

const DAY: i64 = 86_400;
const CORAL: Color = Color::from_rgb(0.941, 0.408, 0.408);
const GREEN: Color = theme::HIT_100;
const LEFT: f32 = 300.0;
const RIGHT: f32 = 296.0;
const SIDE_MOST: f32 = 380.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Metric {
    #[default]
    Pp,
    Rank,
    Accuracy,
    Plays,
    Hours,
}

impl Metric {
    pub const ALL: [Metric; 5] = [Metric::Pp, Metric::Rank, Metric::Accuracy, Metric::Plays, Metric::Hours];

    fn key(self) -> &'static str {
        match self {
            Metric::Pp => "board-pp",
            Metric::Rank => "metric-world",
            Metric::Accuracy => "board-accuracy",
            Metric::Plays => "board-plays",
            Metric::Hours => "board-hours",
        }
    }
}

struct Series {
    points: Vec<(i64, f64)>,
    lower_better: bool,
}

fn series(ground: &Ground<'_>, whose: &Whose<'_>, metric: Metric, span: u32) -> Series {
    let card = &whose.card;
    let now = ground.now_unix;
    let since = now - i64::from(span) * DAY;
    if metric == Metric::Rank {
        let count = card.rank_history.len() as i64;
        let points = card.rank_history.iter().enumerate().map(|(at, value)| (now - (count - 1 - at as i64) * DAY, *value)).filter(|(at, _)| *at >= since).collect();
        return Series { points, lower_better: true };
    }
    let current = match metric {
        Metric::Pp => card.pp,
        Metric::Accuracy => card.accuracy,
        Metric::Plays => card.play_count,
        _ => card.play_seconds / 3600.0,
    };
    let weeks = whose.me.map(|me| me.history.as_slice()).unwrap_or_default();
    let mut points: Vec<(i64, f64)> = weeks
        .iter()
        .filter(|week| week.at >= since && week.at < now - DAY / 2)
        .map(|week| {
            let value = match metric {
                Metric::Pp => week.pp,
                Metric::Accuracy => week.accuracy,
                Metric::Plays => week.plays,
                _ => week.hours,
            };
            (week.at, value)
        })
        .filter(|(_, value)| *value > 0.0)
        .collect();
    if current > 0.0 {
        points.push((now, current));
    }
    Series { points, lower_better: false }
}

fn full(words: &crate::lang::Words, metric: Metric, value: f64) -> String {
    match metric {
        Metric::Pp => format!("{} pp", words.lang().group(value.round() as u64)),
        Metric::Rank => format!("#{}", words.lang().group(value.round() as u64)),
        Metric::Accuracy => words.percent(value),
        Metric::Plays => words.lang().group(value.round() as u64),
        Metric::Hours => format!("{} {}", words.lang().group(value.round() as u64), words.t("hours-short")),
    }
}

fn delta(words: &crate::lang::Words, metric: Metric, series: &Series) -> Option<(String, bool)> {
    let (first, last) = (series.points.first()?.1, series.points.last()?.1);
    let change = last - first;
    if change.abs() < 1e-6 || series.points.len() < 2 {
        return None;
    }
    let better = if series.lower_better { change < 0.0 } else { change > 0.0 };
    let size = change.abs();
    let said = match metric {
        Metric::Pp => format!("{} pp", words.lang().group(size.round() as u64)),
        Metric::Rank => words.lang().group(size.round() as u64),
        Metric::Accuracy => screen::decimal(words, size as f32, 2) + "%",
        Metric::Plays => words.lang().group(size.round() as u64),
        Metric::Hours => format!("{} {}", words.lang().group(size.round() as u64), words.t("hours-short")),
    };
    let sign = if metric == Metric::Rank { "" } else if change > 0.0 { "+" } else { "−" };
    Some((format!("{sign}{said}"), better))
}

fn change<'a>(metric: Metric, said: String, better: bool) -> Element<'a, Message> {
    let colour = if better { GREEN } else { ACCENT };
    if metric != Metric::Rank {
        return ui::mono_small(said, colour);
    }
    row![glyph(if better { Icon::Up } else { Icon::Down }, 10.0, colour), ui::mono_small(said, colour)].spacing(3).align_y(iced::Center).into()
}

fn date(words: &crate::lang::Words, at: i64, now: i64) -> String {
    words.day(at, now)
}

struct Chart {
    points: Vec<(f32, f32)>,
    tips: Vec<(String, String)>,
    marks: [String; 4],
    empty: String,
    alpha: f32,
}

#[derive(Default)]
struct Hover {
    at: Option<usize>,
}

const FOOT: f32 = 22.0;
const EDGE: f32 = 6.0;

fn smooth(points: &[Point], ground: Option<f32>) -> Path {
    let n = points.len();
    let mut slopes = vec![0.0f32; n.saturating_sub(1)];
    for k in 0..n.saturating_sub(1) {
        let dx = points[k + 1].x - points[k].x;
        slopes[k] = if dx.abs() < 0.001 { 0.0 } else { (points[k + 1].y - points[k].y) / dx };
    }
    let mut tangents = vec![0.0f32; n];
    if n >= 2 {
        tangents[0] = slopes[0];
        tangents[n - 1] = slopes[n - 2];
    }
    for k in 1..n.saturating_sub(1) {
        tangents[k] = if slopes[k - 1] * slopes[k] <= 0.0 { 0.0 } else { (slopes[k - 1] + slopes[k]) / 2.0 };
    }
    for k in 0..n.saturating_sub(1) {
        if slopes[k] == 0.0 {
            tangents[k] = 0.0;
            tangents[k + 1] = 0.0;
            continue;
        }
        let a = tangents[k] / slopes[k];
        let b = tangents[k + 1] / slopes[k];
        let h = a * a + b * b;
        if h > 9.0 {
            let t = 3.0 / h.sqrt();
            tangents[k] = t * a * slopes[k];
            tangents[k + 1] = t * b * slopes[k];
        }
    }
    Path::new(|b| {
        match ground {
            Some(ground) => {
                b.move_to(Point::new(points[0].x, ground));
                b.line_to(points[0]);
            }
            None => b.move_to(points[0]),
        }
        for k in 0..n.saturating_sub(1) {
            let dx = (points[k + 1].x - points[k].x) / 3.0;
            b.bezier_curve_to(Point::new(points[k].x + dx, points[k].y + tangents[k] * dx), Point::new(points[k + 1].x - dx, points[k + 1].y - tangents[k + 1] * dx), points[k + 1]);
        }
        if let Some(ground) = ground {
            b.line_to(Point::new(points[n - 1].x, ground));
            b.close();
        }
    })
}

impl Chart {
    fn plot(bounds: Rectangle) -> Rectangle {
        Rectangle { x: EDGE, y: 10.0, width: (bounds.width - EDGE * 2.0).max(1.0), height: (bounds.height - FOOT - 10.0).max(1.0) }
    }

    fn at(&self, plot: Rectangle, index: usize) -> Point {
        let (x, y) = self.points[index];
        Point::new(plot.x + x * plot.width, plot.y + y * plot.height)
    }
}

impl canvas::Program<Message> for Chart {
    type State = Hover;

    fn update(&self, state: &mut Hover, event: &canvas::Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Message>> {
        if !matches!(event, canvas::Event::Mouse(_)) || self.points.len() < 2 {
            return None;
        }
        let plot = Self::plot(bounds);
        let next = cursor.position_in(bounds).filter(|at| at.x >= plot.x - 8.0).map(|at| {
            let share = ((at.x - plot.x) / plot.width).clamp(0.0, 1.0);
            self.points.iter().enumerate().min_by(|a, b| (a.1 .0 - share).abs().total_cmp(&(b.1 .0 - share).abs())).map_or(0, |(index, _)| index)
        });
        if next != state.at {
            state.at = next;
            return Some(canvas::Action::request_redraw());
        }
        None
    }

    fn draw(&self, state: &Hover, renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let k = self.alpha;
        let fade = |colour: Color| Color { a: colour.a * k, ..colour };
        let words = |content: String, position: Point, align: iced::widget::text::Alignment, colour: Color, size: f32| canvas::Text {
            content,
            position,
            color: fade(colour),
            size: size.into(),
            font: theme::MONO,
            align_x: align,
            align_y: iced::alignment::Vertical::Center,
            ..canvas::Text::default()
        };
        let plot = Self::plot(bounds);
        if self.points.len() < 2 {
            frame.fill_text(words(self.empty.clone(), Point::new(bounds.width / 2.0, bounds.height / 2.0), iced::widget::text::Alignment::Center, MUTED, 12.0));
            return vec![frame.into_geometry()];
        }
        for step in 0..4 {
            let y = plot.y + plot.height * step as f32 / 3.0;
            frame.stroke(&Path::line(Point::new(plot.x, y), Point::new(plot.x + plot.width, y)), Stroke::default().with_color(fade(Color::from_rgba(1.0, 1.0, 1.0, 0.012))).with_width(1.0));
        }
        let points: Vec<Point> = (0..self.points.len()).map(|index| self.at(plot, index)).collect();
        let ground = plot.y + plot.height;
        let shade = canvas::gradient::Linear::new(Point::new(0.0, plot.y), Point::new(0.0, ground))
            .add_stop(0.0, fade(Color::from_rgba(0.886, 0.282, 0.282, 0.26)))
            .add_stop(1.0, fade(Color::from_rgba(0.886, 0.282, 0.282, 0.02)));
        frame.fill(&smooth(&points, Some(ground)), canvas::Fill { style: canvas::Style::Gradient(shade.into()), ..canvas::Fill::default() });
        let curve = smooth(&points, None);
        frame.stroke(&curve, Stroke::default().with_color(fade(ACCENT)).with_width(2.4).with_line_join(canvas::LineJoin::Round).with_line_cap(canvas::LineCap::Round));
        for (step, mark) in self.marks.iter().enumerate() {
            let x = plot.x + plot.width * step as f32 / 3.0;
            let align = match step {
                0 => iced::widget::text::Alignment::Left,
                3 => iced::widget::text::Alignment::Right,
                _ => iced::widget::text::Alignment::Center,
            };
            frame.fill_text(words(mark.clone(), Point::new(x, ground + FOOT / 2.0 + 2.0), align, FAINT, 10.5));
        }
        match state.at {
            Some(index) => {
                let point = points[index];
                frame.stroke(&Path::line(Point::new(point.x, plot.y), Point::new(point.x, ground)), Stroke::default().with_color(fade(Color::from_rgba(1.0, 1.0, 1.0, 0.06))).with_width(1.0));
                frame.fill(&Path::circle(point, 6.0), fade(Color::from_rgb(0.08, 0.03, 0.035)));
                frame.fill(&Path::circle(point, 4.0), fade(CORAL));
                let (value, when) = &self.tips[index];
                let wide = 12.0 + (value.chars().count().max(when.chars().count()) as f32) * 7.4;
                let left = (point.x - wide / 2.0).clamp(plot.x, bounds.width - wide - 2.0);
                let top = if point.y - 52.0 > 0.0 { point.y - 52.0 } else { point.y + 12.0 };
                let tip = Rectangle { x: left, y: top, width: wide, height: 40.0 };
                frame.fill(&Path::rounded_rectangle(tip.position(), tip.size(), 8.0.into()), fade(Color::from_rgb(0.118, 0.078, 0.094)));
                frame.stroke(&Path::rounded_rectangle(tip.position(), tip.size(), 8.0.into()), Stroke::default().with_color(fade(Color::from_rgba(1.0, 1.0, 1.0, 0.05))).with_width(1.0));
                frame.fill_text(canvas::Text {
                    content: value.clone(),
                    position: Point::new(left + 8.0, top + 13.0),
                    color: fade(INK),
                    size: 13.0.into(),
                    font: theme::SANS_SEMI,
                    align_y: iced::alignment::Vertical::Center,
                    ..canvas::Text::default()
                });
                frame.fill_text(words(when.clone(), Point::new(left + 8.0, top + 29.0), iced::widget::text::Alignment::Left, MUTED, 10.0));
            }
            None => {
                frame.fill(&Path::circle(points[points.len() - 1], 4.5), fade(CORAL));
            }
        }
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(&self, _: &Hover, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if cursor.is_over(bounds) && self.points.len() > 1 {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

struct Heat {
    days: Vec<(String, u32)>,
    none: String,
    plays: Vec<String>,
    less: String,
    more: String,
    alpha: f32,
}

const CELL: f32 = 14.0;
const CELL_GAP: f32 = 4.0;

fn heat_share(n: u32) -> f32 {
    match n {
        0 => 0.035,
        1..=5 => 0.14,
        6..=10 => 0.3,
        11..=15 => 0.55,
        _ => 0.95,
    }
}

impl canvas::Program<Message> for Heat {
    type State = Hover;

    fn update(&self, state: &mut Hover, event: &canvas::Event, bounds: Rectangle, cursor: mouse::Cursor) -> Option<canvas::Action<Message>> {
        if !matches!(event, canvas::Event::Mouse(_)) {
            return None;
        }
        let next = cursor.position_in(bounds).and_then(|at| {
            let column = ((at.x - HEAT_PAD) / (CELL + CELL_GAP)).floor() as i32;
            let line = ((at.y - HEAT_PAD) / (CELL + CELL_GAP)).floor() as i32;
            let index = column * 7 + line;
            ((0..13).contains(&column) && (0..7).contains(&line) && (index as usize) < self.days.len()).then_some(index as usize)
        });
        if next != state.at {
            state.at = next;
            return Some(canvas::Action::request_redraw());
        }
        None
    }

    fn draw(&self, state: &Hover, renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let k = self.alpha;
        for (index, (_, n)) in self.days.iter().enumerate() {
            let column = (index / 7) as f32;
            let line = (index % 7) as f32;
            let lit = state.at == Some(index);
            let grow = if lit { 2.0 } else { 0.0 };
            let corner = Point::new(HEAT_PAD + column * (CELL + CELL_GAP) - grow / 2.0, HEAT_PAD + line * (CELL + CELL_GAP) - grow / 2.0);
            let side = iced::Size::new(CELL + grow, CELL + grow);
            frame.fill(&Path::rounded_rectangle(corner, side, 3.0.into()), Color::from_rgba(0.886, 0.282, 0.282, heat_share(*n) * k));
            if lit {
                frame.stroke(&Path::rounded_rectangle(corner, side, 3.0.into()), Stroke::default().with_color(Color::from_rgba(1.0, 1.0, 1.0, 0.5 * k)).with_width(1.0));
            }
        }
        let foot = HEAT_PAD + 7.0 * (CELL + CELL_GAP) + 10.0;
        let said = match state.at.and_then(|index| self.days.get(index)) {
            Some((day, 0)) => format!("{day} · {}", self.none),
            Some((day, n)) => format!("{day} · {}", self.plays.get(*n as usize).cloned().unwrap_or_default()),
            None => String::new(),
        };
        frame.fill_text(canvas::Text {
            content: said,
            position: Point::new(HEAT_PAD, foot + 6.0),
            color: Color { a: k, ..MUTED },
            size: 12.0.into(),
            font: theme::SANS,
            align_y: iced::alignment::Vertical::Center,
            ..canvas::Text::default()
        });
        let legend_right = bounds.width;
        let mut x = legend_right - 5.0 * 13.0 - 4.0;
        frame.fill_text(canvas::Text {
            content: self.more.clone(),
            position: Point::new(legend_right, foot + 22.0),
            color: Color { a: k, ..FAINT },
            size: 9.5.into(),
            font: theme::MONO,
            align_x: iced::widget::text::Alignment::Right,
            align_y: iced::alignment::Vertical::Center,
            ..canvas::Text::default()
        });
        x -= (self.more.chars().count() as f32) * 6.0;
        for n in [0, 3, 8, 13, 20] {
            frame.fill(&Path::rounded_rectangle(Point::new(x, foot + 17.0), iced::Size::new(10.0, 10.0), 3.0.into()), Color::from_rgba(0.886, 0.282, 0.282, heat_share(n) * k));
            x += 13.0;
        }
        frame.fill_text(canvas::Text {
            content: self.less.clone(),
            position: Point::new(legend_right - 5.0 * 13.0 - 10.0 - (self.more.chars().count() as f32) * 6.0, foot + 22.0),
            color: Color { a: k, ..FAINT },
            size: 9.5.into(),
            font: theme::MONO,
            align_x: iced::widget::text::Alignment::Right,
            align_y: iced::alignment::Vertical::Center,
            ..canvas::Text::default()
        });
        vec![frame.into_geometry()]
    }
}

fn caption<'a>(words: String) -> Element<'a, Message> {
    ui::mono_small(words.to_uppercase(), FAINT)
}

fn info_line<'a>(icon: Element<'a, Message>, label: String, value: Element<'a, Message>) -> Element<'a, Message> {
    container(row![container(icon).width(18.0).center_x(18.0), text(label).font(theme::SANS).size(12.5).color(ui::faded(MUTED)), container(value).width(Length::Fill).align_x(iced::alignment::Horizontal::Right)].spacing(10).align_y(iced::Center))
        .padding([6, 6])
        .into()
}

fn info_row<'a>(icon: Element<'a, Message>, label: String, value: String, colour: Color) -> Element<'a, Message> {
    container(
        row![
            container(icon).width(18.0).center_x(18.0),
            text(label).font(theme::SANS).size(12.5).color(ui::faded(MUTED)).width(Length::Fill),
            text(value).font(theme::SANS_SEMI).size(12.5).color(ui::faded(colour)),
        ]
        .spacing(10)
        .align_y(iced::Center),
    )
    .padding([6, 6])
    .into()
}

fn primary(_: &Theme, status: button::Status) -> button::Style {
    let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(Background::Color(if lit { Color::from_rgb(0.925, 0.337, 0.337) } else { ACCENT })),
        text_color: Color::WHITE,
        border: Border { radius: 10.0.into(), ..Border::default() },
        shadow: if lit { Shadow { color: Color::from_rgba(0.886, 0.282, 0.282, 0.35), offset: Vector::new(0.0, 6.0), blur_radius: 18.0 } } else { Shadow::default() },
        snap: true,
    }
}

fn outline(_: &Theme, status: button::Status) -> button::Style {
    let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(Background::Color(if lit { Color::from_rgba(1.0, 1.0, 1.0, 0.03) } else { Color::TRANSPARENT })),
        text_color: INK,
        border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, if lit { 0.1 } else { 0.05 }), width: 1.0, radius: 10.0.into() },
        shadow: Shadow::default(),
        snap: true,
    }
}

fn numeric_date(at: i64) -> String {
    chrono::DateTime::from_timestamp(at, 0).map_or_else(String::new, |when| when.format("%d.%m.%Y").to_string())
}

fn years(words: &crate::lang::Words, joined: i64, now: i64) -> Option<String> {
    (joined > 0).then(|| words.n("years", ((now - joined) / (365 * DAY)).max(0) as u64))
}

fn identity<'a>(ground: &Ground<'a>, whose: &Whose<'a>) -> Element<'a, Message> {
    let (you, card) = (whose.person, &whose.card);
    let w = ground.words;
    let share = (card.level_progress / 100.0) as f32;
    let k = ui::fade();
    let side = 156.0;
    let mut face = stack![chronicle::ring(ground, you, side, share, 6.0)].width(side).height(side);
    if card.is_online {
        let dot = container(Space::new().width(26.0).height(26.0)).style(move |_| container::Style {
            background: Some(Background::Color(Color { a: k, ..GREEN })),
            border: Border { color: Color { a: k, ..theme::SLAB_SOLID }, width: 5.0, radius: 13.0.into() },
            shadow: iced::Shadow { color: Color { a: 0.45 * k, ..GREEN }, offset: Vector::ZERO, blur_radius: 10.0 },
            ..container::Style::default()
        });
        face = face.push(container(dot).width(side).height(side).align_x(iced::alignment::Horizontal::Right).align_y(iced::alignment::Vertical::Bottom).padding(Padding { top: 0.0, right: 12.0, bottom: 12.0, left: 0.0 }));
    }
    let mut avatar = stack![container(face).width(Length::Fill).center_x(Length::Fill)].height(side + 14.0);
    if card.level >= 1.0 {
        let medallion = container(
            column![
                text((card.level.floor() as u64).to_string()).font(theme::MONO_BOLD).size(15.0).color(ui::faded(INK)),
                text(w.t("level-short").to_uppercase()).font(theme::MONO).size(8.5).color(ui::faded(MUTED)),
            ]
            .align_x(iced::alignment::Horizontal::Center),
        )
        .width(46.0)
        .height(46.0)
        .center(46.0)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color { a: k, ..theme::SLAB_SOLID })),
            border: Border { color: Color { a: k, ..ACCENT }, width: 1.5, radius: 23.0.into() },
            ..container::Style::default()
        });
        avatar = avatar.push(container(medallion).width(Length::Fill).height(side + 14.0).center_x(Length::Fill).align_y(iced::alignment::Vertical::Bottom));
    }
    let seen = if card.is_online {
        Some((w.t("online"), GREEN))
    } else {
        crate::news::unix_of(&card.last_visit).map(|at| (w.with("last-seen", &[("when", screen::since(ground, at))]), FAINT))
    };
    let mut heading = column![
        avatar,
        text(you.name.clone()).font(theme::SANS_SEMI).size(28.0).wrapping(text::Wrapping::None).color(ui::faded(INK)),
    ]
    .spacing(6)
    .align_x(iced::alignment::Horizontal::Center)
    .width(Length::Fill);
    if !card.handle.is_empty() {
        heading = heading.push(text(card.handle.clone()).font(theme::SANS).size(14.0).color(ui::faded(Color::from_rgb(0.737, 0.588, 0.596))));
    }
    let worn: Element<'a, Message> = match ground.catalog.shown_title(you) {
        Some(title) => screen::title_chip(title, w.lang()),
        None => text(w.t("no-title")).font(theme::SANS).size(12.5).color(ui::faded(FAINT)).into(),
    };
    heading = heading.push(worn);
    if let Some((said, colour)) = seen {
        heading = heading.push(text(said).font(theme::SANS).size(12.0).color(ui::faded(colour)));
    }
    let country = if card.country_name.is_empty() { card.country.to_ascii_uppercase() } else { card.country_name.clone() };
    let country = match card.country_rank {
        rank if rank > 0.0 => format!("{country} · #{}", w.lang().group(rank as u64)),
        _ => country,
    };
    let flag: Element<'a, Message> = match ground.flags.get(&card.country.to_ascii_lowercase()) {
        Some(handle) => iced::widget::svg(handle.clone()).width(16.0).height(11.0).opacity(ui::fade()).into(),
        None => Space::new().width(0.0).into(),
    };
    let place = row![flag, ui::marquee(vec![ui::piece(country, theme::SANS_SEMI, 12.5, INK)]).width(Length::Shrink)].spacing(7).align_y(iced::Center);
    let mut rows = column![info_line(glyph(Icon::Flag, 15.0, MUTED), w.t("info-country"), place.into())].spacing(0);
    if let Some(joined) = crate::news::unix_of(&card.join_date).or((you.joined > 0).then_some(you.joined)) {
        let mut said = format!("{} {}", w.t("info-since-from"), numeric_date(joined));
        if let Some(years) = years(w, joined, ground.now_unix) {
            said = format!("{said} · {years}");
        }
        rows = rows.push(info_row(glyph(Icon::Calendar, 15.0, MUTED), w.t("info-since"), said, INK));
    }
    if you.streak > 0 || you.streak_best > 0 {
        rows = rows.push(info_row(glyph(Icon::Flame, 15.0, MUTED), w.t("streak"), format!("{} · {}", w.n("streak-card", u64::from(you.streak)), w.n("streak-best-n", u64::from(you.streak_best.max(you.streak)))), CORAL));
    }
    if let Some(me) = whose.me {
        if me.duels[0] + me.duels[1] > 0 {
            rows = rows.push(info_row(glyph(Icon::Swords, 15.0, MUTED), w.t("info-duels"), format!("{} : {}", me.duels[0], me.duels[1]), INK));
        }
    }
    let profile = if card.osu_id > 0.0 { format!("https://osu.ppy.sh/users/{}", card.osu_id as u64) } else { format!("https://osu.ppy.sh/users/{}", you.name) };
    let buttons = row![
        ui::hover(
            button(container(row![glyph(Icon::External, 14.0, Color::WHITE), text(w.t("open-osu")).font(theme::SANS_SEMI).size(13.0)].spacing(6).align_y(iced::Center)).center_x(Length::Fill))
                .padding([10, 0])
                .width(Length::Fill)
                .style(ui::button_faded(ui::calm(primary)))
                .on_press(Message::Open(profile)),
            ui::Glow::tile(10.0).edge(Color::from_rgba(1.0, 0.8, 0.8, 0.35)).shadow(Color::from_rgba(0.886, 0.282, 0.282, 0.25)),
        ),
        ui::hover(
            button(container(row![glyph(Icon::Compare, 14.0, INK), text(w.t("compare")).font(theme::SANS_SEMI).size(13.0)].spacing(6).align_y(iced::Center)).center_x(Length::Fill))
                .padding([10, 0])
                .width(Length::Fill)
                .style(ui::button_faded(ui::calm(outline)))
                .on_press(Message::Section(Section::Boards)),
            ui::Glow::tile(10.0),
        ),
    ]
    .spacing(8);
    slab(column![heading, rows, buttons].spacing(12), [18, 18]).into()
}

fn standing<'a>(share: f32, colour: Color) -> Element<'a, Message> {
    let k = ui::fade();
    let filled = (share.clamp(0.0, 1.0) * 1000.0).round().max(1.0) as u16;
    let bar = row![
        container(Space::new().height(3.0)).width(Length::FillPortion(filled)).style(move |_| container::Style {
            background: Some(Background::Color(Color { a: colour.a * k, ..colour })),
            border: Border { radius: 2.0.into(), ..Border::default() },
            ..container::Style::default()
        }),
        Space::new().width(Length::FillPortion(1000u16.saturating_sub(filled).max(1))).height(3.0),
    ];
    container(bar)
        .width(Length::Fill)
        .style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.04 * k))), border: Border { radius: 2.0.into(), ..Border::default() }, ..container::Style::default() })
        .into()
}

fn shift<'a>(by: i32) -> Element<'a, Message> {
    if by == 0 {
        return Space::new().width(0.0).height(0.0).into();
    }
    let (icon, colour) = if by > 0 { (Icon::Up, GREEN) } else { (Icon::Down, ACCENT) };
    row![glyph(icon, 10.0, colour), text(by.unsigned_abs().to_string()).font(theme::MONO_BOLD).size(10.5).wrapping(text::Wrapping::None).color(ui::faded(colour))].spacing(1).align_y(iced::Center).into()
}

fn places<'a>(ground: &Ground<'a>, whose: &Whose<'a>, t: f32) -> Element<'a, Message> {
    let f = ui::tally(t, 0.15);
    let w = ground.words;
    let catalog = ground.catalog;
    let you = whose.at;
    let many = catalog.people.len();
    let mut tiles: Vec<Element<'a, Message>> = Vec::new();
    for board in Board::ALL {
        let order = catalog.ranked(board);
        let place = order.iter().position(|at| *at == you).map_or(0, |x| x + 1);
        let moved = catalog.people[you].moved[board.index()];
        let colour = match place {
            1 => theme::GRADE_S,
            2 => Color::from_rgb8(200, 204, 220),
            3 => Color::from_rgb8(205, 127, 50),
            _ => INK,
        };
        let share = if many > 1 && place > 0 { 1.0 - (place - 1) as f32 / (many - 1) as f32 } else { 1.0 };
        tiles.push(ui::hover(
            button(
                column![
                    row![container(ui::mono_small(w.t(board.short_key()).to_uppercase(), FAINT)).width(Length::Fill).clip(true), shift(moved)].spacing(4).align_y(iced::Center),
                    text(format!("#{}", ((place as f64 * f).round() as usize).max(1))).font(theme::SANS_SEMI).size(22.0).wrapping(text::Wrapping::None).color(ui::faded(colour)),
                    standing(share, if place <= 3 { colour } else { ACCENT }),
                ]
                .spacing(6),
            )
            .padding([10, 12])
            .width(Length::FillPortion(1))
            .style(ui::button_faded(ui::calm(outline)))
            .on_press(Message::Board(board)),
            ui::Glow::tile(10.0).lift(2.0),
        ));
    }
    let mut grid = column![].spacing(6);
    let mut tiles = tiles.into_iter();
    for _ in 0..2 {
        let mut line = row![].spacing(6);
        for _ in 0..3 {
            if let Some(tile) = tiles.next() {
                line = line.push(tile);
            }
        }
        grid = grid.push(line);
    }
    let said = format!("{} · {}", w.count("players", many as u64), w.n("week-short", u64::from(catalog.week)).to_lowercase());
    slab(column![row![caption(w.t("place-head")), ui::grow(), ui::mono_small(said, MUTED)].align_y(iced::Center), grid].spacing(10), [14, 16]).into()
}

fn metric_style(on: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: Some(Background::Color(if on { Color::from_rgba(0.886, 0.282, 0.282, 0.1) } else if lit { Color::from_rgba(1.0, 1.0, 1.0, 0.016) } else { theme::RAISED })),
            text_color: INK,
            border: Border { color: if on { Color::from_rgba(0.886, 0.282, 0.282, 0.55) } else if lit { Color::from_rgba(1.0, 1.0, 1.0, 0.07) } else { theme::LINE }, width: 1.0, radius: 12.0.into() },
            shadow: if on { Shadow { color: Color::from_rgba(0.886, 0.282, 0.282, 0.07), offset: Vector::ZERO, blur_radius: 8.0 } } else { Shadow::default() },
            snap: true,
        }
    }
}

fn metrics<'a>(ground: &Ground<'a>, whose: &Whose<'a>, t: f32) -> Element<'a, Message> {
    let card = &whose.card;
    let w = ground.words;
    let mut tiles = row![].spacing(8);
    for (index, metric) in Metric::ALL.into_iter().enumerate() {
        let f = ui::tally(t, 0.05 * index as f32);
        let current = match metric {
            Metric::Pp => card.pp,
            Metric::Rank => card.global_rank,
            Metric::Accuracy => card.accuracy,
            Metric::Plays => card.play_count,
            Metric::Hours => card.play_seconds / 3600.0,
        };
        let value = if current > 0.0 { full(w, metric, (current * f).max(if metric == Metric::Rank { 1.0 } else { 0.0 })).replace(" pp", "") } else { "—".to_owned() };
        let change = delta(w, metric, &series(ground, whose, metric, 90));
        let mut under = column![ui::mono_small(w.t(metric.key()).to_uppercase(), FAINT)].spacing(1);
        if let Some((said, better)) = change {
            under = under.push(self::change(metric, said, better));
        }
        tiles = tiles.push(ui::hover(
            button(container(column![text(value).font(theme::SANS_SEMI).size(20.0).wrapping(text::Wrapping::None).color(ui::faded(if metric == Metric::Pp { CORAL } else { INK })), under].spacing(3)).width(Length::Fill).clip(true))
                .padding([12, 14])
                .width(Length::FillPortion(1))
                .style(ui::button_faded(ui::calm(metric_style(ground.metric == metric))))
                .on_press(Message::Metric(metric)),
            ui::Glow::tile(12.0).lift(2.0),
        ));
    }
    tiles.into()
}

fn segment(on: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: Some(Background::Color(if !on && matches!(status, button::Status::Hovered) { Color::from_rgba(1.0, 1.0, 1.0, 0.012) } else { Color::TRANSPARENT })),
        text_color: if on { INK } else { FAINT },
        border: Border { radius: 6.0.into(), ..Border::default() },
        shadow: Shadow::default(),
        snap: true,
    }
}

fn chart<'a>(ground: &Ground<'a>, whose: &Whose<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let metric = ground.metric;
    let data = series(ground, whose, metric, ground.span);
    let (lo, hi) = data.points.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), (_, v)| (lo.min(*v), hi.max(*v)));
    let spread = (hi - lo).max(match metric {
        Metric::Accuracy => 0.05,
        _ => 1.0,
    });
    let lo = lo - spread * 0.12;
    let spread = spread * 1.24;
    let (first, last) = (data.points.first().map_or(0, |p| p.0), data.points.last().map_or(1, |p| p.0));
    let length = ((last - first) as f64).max(1.0);
    let points = data
        .points
        .iter()
        .map(|(at, value)| {
            let x = ((at - first) as f64 / length) as f32;
            let ratio = ((value - lo) / spread) as f32;
            (x, if data.lower_better { ratio } else { 1.0 - ratio })
        })
        .collect();
    let tips = data.points.iter().map(|(at, value)| (full(w, metric, *value), date(w, *at, ground.now_unix))).collect();
    let marks: [String; 4] = std::array::from_fn(|step| {
        if step == 3 {
            w.t("day-today").to_lowercase()
        } else {
            let at = first + ((last - first) as f64 * step as f64 / 3.0) as i64;
            date(w, at, ground.now_unix)
        }
    });
    let canvas = Canvas::new(Chart { points, tips, marks, empty: w.t("card-no-data"), alpha: ui::fade() }).width(Length::Fill).height(200.0);
    let mut ranges = row![].spacing(2);
    for span in [30u32, 90] {
        ranges = ranges.push(button(text(w.n("days-short", u64::from(span))).font(theme::MONO_BOLD).size(11.0)).padding([4, 10]).style(ui::button_faded(segment(ground.span == span))).on_press(Message::Span(span)));
    }
    let k = ui::fade();
    let ranges = ui::sliding(ranges, if ground.span == 30 { 0 } else { 1 }, ui::Pill { fill: Color::from_rgba(1.0, 1.0, 1.0, 0.06), edge: Color::TRANSPARENT, radius: 6.0, underline: None });
    let control = container(ranges).padding(2).style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.25 * k))), border: Border { radius: 8.0.into(), ..Border::default() }, ..container::Style::default() });
    let mut head = row![caption(format!("{} · {}", w.t(metric.key()), w.n("days-long", u64::from(ground.span))))].spacing(10).align_y(iced::Center);
    if let Some((said, better)) = delta(w, metric, &data) {
        head = head.push(change(metric, said, better));
    }
    slab(column![head.push(ui::grow()).push(control), canvas].spacing(12), [16, 18]).into()
}

struct Poster<'a> {
    grade: String,
    title: String,
    version: String,
    artist: String,
    mapper: String,
    pp: f64,
    accuracy: f64,
    mods: Vec<String>,
    counts: [Option<u32>; 4],
    combo: Option<u32>,
    most: Option<u32>,
    stars: Option<f32>,
    bpm: Option<f64>,
    length: Option<f64>,
    played: Option<i64>,
    cover: Option<&'a iced::widget::image::Handle>,
    art: Option<&'a iced::widget::image::Handle>,
    set: Option<u64>,
}

impl Poster<'_> {
    fn misses(&self) -> Option<u32> {
        self.counts[3]
    }

    fn full(&self) -> bool {
        self.misses() == Some(0) && self.most.is_none_or(|most| self.combo.is_none_or(|combo| combo + 10 >= most))
    }
}

fn posters_of<'a>(ground: &Ground<'a>, whose: &Whose<'a>) -> Vec<Poster<'a>> {
    let card = &whose.card;
    if card.top_scores.iter().any(|score| score.pp > 0.0 && score.hash.is_empty()) {
        return card
            .top_scores
            .iter()
            .take(5)
            .map(|score| {
                let grade = match score.rank.to_ascii_uppercase().as_str() {
                    "X" | "XH" | "SS" | "SSH" => "SS".to_owned(),
                    "SH" => "S".to_owned(),
                    other => other.to_owned(),
                };
                let counted = |n: f64| (n > 0.0 || score.great > 0.0).then_some(n as u32);
                Poster {
                    grade,
                    title: score.title.clone(),
                    version: score.version.clone(),
                    artist: score.artist.clone(),
                    mapper: score.creator.clone(),
                    pp: score.pp,
                    accuracy: score.accuracy,
                    mods: score.mods.split(',').map(str::trim).filter(|m| !m.is_empty() && *m != "CL" && *m != "NM").map(str::to_owned).collect(),
                    counts: [counted(score.great), counted(score.ok), counted(score.meh), counted(score.miss)],
                    combo: (score.max_combo > 0.0).then_some(score.max_combo as u32),
                    most: (score.map_max_combo > 0.0).then_some(score.map_max_combo as u32),
                    stars: (score.stars > 0.0).then_some(score.stars as f32),
                    bpm: (score.bpm > 0.0).then_some(score.bpm),
                    length: (score.length > 0.0).then_some(score.length),
                    played: crate::news::unix_of(&score.played),
                    cover: score.cover().and_then(|url| ground.pictures.get(&url)).or_else(|| ground.thumbs.get(&score.hash)),
                    art: score.cover().and_then(|url| ground.pictures.get(&crate::community::poster_key(&url))),
                    set: (score.beatmapset_id > 0.0).then_some(score.beatmapset_id as u64),
                }
            })
            .collect();
    }
    whose
        .person
        .top
        .iter()
        .take(5)
        .map(|play| {
            let map = ground.catalog.map(Some(play.map));
            let line = screen::line_of(ground, play.map);
            let (artist, rest) = line.split_once(" — ").map_or((String::new(), line.clone()), |(artist, rest)| (artist.to_owned(), rest.to_owned()));
            let (title, version) = match rest.rsplit_once(" [") {
                Some((title, version)) => (title.to_owned(), version.trim_end_matches(']').to_owned()),
                None => (rest, String::new()),
            };
            Poster {
                grade: play.grade.clone(),
                title,
                version,
                artist,
                mapper: String::new(),
                pp: f64::from(play.pp),
                accuracy: f64::from(play.accuracy),
                mods: play.mods.clone(),
                counts: play.counts,
                combo: play.combo,
                most: play.max_combo,
                stars: play.stars,
                bpm: None,
                length: None,
                played: (play.at > 0).then_some(play.at),
                cover: map.and_then(|map| ground.thumbs.get(&map.hash).or_else(|| map.card().and_then(|card| ground.pictures.get(&card)))),
                art: map.and_then(|map| map.poster()).and_then(|key| ground.pictures.get(&key)),
                set: map.and_then(|map| map.set),
            }
        })
        .collect()
}

pub(crate) fn star_colour(stars: f32) -> Color {
    let stops: [(f32, [u8; 3]); 11] = [
        (0.1, [0x42, 0x90, 0xfb]),
        (1.25, [0x4f, 0xc0, 0xff]),
        (2.0, [0x4f, 0xff, 0xd5]),
        (2.5, [0x7c, 0xff, 0x4f]),
        (3.3, [0xf6, 0xf0, 0x5c]),
        (4.2, [0xff, 0x80, 0x68]),
        (4.9, [0xff, 0x4e, 0x6f]),
        (5.8, [0xc6, 0x45, 0xb8]),
        (6.7, [0x65, 0x63, 0xde]),
        (7.7, [0x18, 0x15, 0x8e]),
        (9.0, [0x00, 0x00, 0x00]),
    ];
    let stars = stars.clamp(0.1, 9.0);
    let at = stops.windows(2).position(|pair| stars <= pair[1].0).unwrap_or(stops.len() - 2);
    let ((from, a), (to, b)) = (stops[at], stops[at + 1]);
    let t = ((stars - from) / (to - from)).clamp(0.0, 1.0);
    let mix = |i: usize| (f32::from(a[i]) + (f32::from(b[i]) - f32::from(a[i])) * t) / 255.0;
    Color::from_rgb(mix(0), mix(1), mix(2))
}

fn badge_pill<'a>(inside: Element<'a, Message>, fill: Color) -> Element<'a, Message> {
    let k = ui::fade();
    container(inside)
        .padding(Padding { top: 2.0, right: 7.0, bottom: 2.0, left: 7.0 })
        .style(move |_| container::Style {
            background: Some(Background::Color(Color { a: fill.a * k, ..fill })),
            border: Border { radius: 8.0.into(), ..Border::default() },
            ..container::Style::default()
        })
        .into()
}

fn poster_card<'a>(ground: &Ground<'a>, index: usize, poster: &Poster<'a>, chosen: bool, wide: f32, f: f64) -> Element<'a, Message> {
    let w = ground.words;
    let colour = screen::grade_colour(&poster.grade);
    let k = ui::fade();
    let ground_colour = Color::from_rgb8(0x17, 0x0d, 0x10);
    let top_round = iced::border::Radius { top_left: 13.0, top_right: 13.0, bottom_right: 0.0, bottom_left: 0.0 };
    let shape = |handle: &iced::widget::image::Handle| match handle {
        iced::widget::image::Handle::Rgba { width, height, .. } if *height > 0 => *width as f32 / *height as f32,
        _ => crate::community::POSTER_SHAPE,
    };
    let fits = |handle: &&iced::widget::image::Handle| shape(handle) < 2.2;
    let (high, picture): (f32, Element<'a, Message>) = match poster.art.or(poster.cover.filter(fits)) {
        Some(handle) => {
            let high = (wide / shape(handle)).clamp(86.0, 150.0);
            let picture = ui::framed(handle, wide, high, 13.0).opacity(k);
            (high, container(picture).width(Length::Fill).height(high).clip(true).into())
        }
        None => {
            let high = (wide / crate::community::POSTER_SHAPE).clamp(86.0, 150.0);
            let picture: Element<'a, Message> = match poster.cover {
                Some(handle) => container(ui::framed(handle, wide, high, 13.0).opacity(k)).width(Length::Fill).height(high).clip(true).into(),
                None => container(ui::fine_hatch()).width(Length::Fill).height(high).into(),
            };
            (high, picture)
        }
    };
    let reach = (high - 4.0) / (high + 20.0);
    let shade = container(Space::new().width(Length::Fill).height(high + 20.0)).style(move |_| container::Style {
        background: Some(Background::Gradient(iced::Gradient::Linear(
            iced::gradient::Linear::new(iced::Radians(std::f32::consts::PI))
                .add_stop(0.0, Color { a: 0.0, ..ground_colour })
                .add_stop(0.45 * reach, Color { a: 0.3 * k, ..ground_colour })
                .add_stop(0.78 * reach, Color { a: 0.86 * k, ..ground_colour })
                .add_stop(reach, Color { a: k, ..ground_colour })
                .add_stop(1.0, Color { a: k, ..ground_colour }),
        ))),
        border: Border { radius: top_round, ..Border::default() },
        ..container::Style::default()
    });
    let numeral = |colour: Color| text((index + 1).to_string()).font(theme::SANS_SEMI).size(34.0).wrapping(text::Wrapping::None).color(colour);
    let place = stack![
        container(numeral(Color { a: 0.6 * k, ..Color::BLACK })).padding(Padding { top: 2.0, right: 0.0, bottom: 0.0, left: 1.5 }),
        numeral(Color { a: 0.97 * k, ..Color::WHITE }),
    ];
    let dusk = container(Space::new().width(Length::Fill).height(high)).style(move |_| container::Style {
        background: Some(Background::Gradient(iced::Gradient::Linear(
            iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_4 * 3.0))
                .add_stop(0.0, Color { a: 0.62 * k, ..ground_colour })
                .add_stop(0.38, Color { a: 0.0, ..ground_colour }),
        ))),
        border: Border { radius: top_round, ..Border::default() },
        ..container::Style::default()
    });
    let mut rating = row![].spacing(5).align_y(iced::Center);
    if let Some(stars) = poster.stars {
        let tint = star_colour(stars);
        let ink = if stars >= 6.5 { Color::from_rgb8(0xff, 0xd9, 0x66) } else { Color::from_rgba(0.0, 0.0, 0.0, 0.8) };
        rating = rating.push(badge_pill(
            row![glyph(Icon::Star, 10.0, ink), text(screen::decimal(w, stars, 2)).font(theme::MONO_BOLD).size(11.0).color(ui::faded(ink))].spacing(3).align_y(iced::Center).into(),
            tint,
        ));
    }
    let head = row![place, ui::grow(), container(rating).padding(Padding::ZERO.top(6.0))].align_y(iced::alignment::Vertical::Top);
    let seal = container(text(poster.grade.clone()).font(theme::SANS_SEMI).size(20.0).color(ui::faded(colour)))
        .width(44.0)
        .height(44.0)
        .center(44.0)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color { a: k, ..ground_colour })),
            border: Border { color: Color { a: k, ..colour }, width: 2.0, radius: 22.0.into() },
            shadow: Shadow { color: Color { a: 0.45 * k, ..colour }, offset: Vector::ZERO, blur_radius: 16.0 },
            ..container::Style::default()
        });
    let mods: Element<'a, Message> = if poster.mods.is_empty() { Space::new().width(0.0).height(0.0).into() } else { screen::mods(&poster.mods) };
    let foot = row![mods, ui::grow(), seal].align_y(iced::alignment::Vertical::Bottom);
    let top = stack![
        picture,
        shade,
        dusk,
        container(head).padding(Padding { top: 2.0, right: 8.0, bottom: 0.0, left: 12.0 }).width(Length::Fill).height(high),
        container(foot).padding(Padding { top: 0.0, right: 10.0, bottom: 0.0, left: 10.0 }).width(Length::Fill).height(high + 20.0).align_y(iced::alignment::Vertical::Bottom),
    ]
    .height(high + 20.0);
    let tail: Element<'a, Message> = match (poster.full(), poster.misses()) {
        (true, _) => badge_pill(text("FC").font(theme::MONO_BOLD).size(10.5).color(ui::faded(GREEN)).into(), Color::from_rgb8(38, 62, 44)),
        (false, Some(misses)) if misses > 0 => badge_pill(text(format!("{misses} ✕")).font(theme::MONO_BOLD).size(10.5).color(ui::faded(CORAL)).into(), Color::from_rgb8(70, 26, 29)),
        _ => Space::new().width(0.0).into(),
    };
    let body = column![
        row![text(screen::decimal(w, (poster.pp * f) as f32, 0)).font(theme::SANS_SEMI).size(27.0).wrapping(text::Wrapping::None).color(ui::faded(INK)), text("pp").font(theme::SANS).size(13.0).color(ui::faded(MUTED))].spacing(4).align_y(iced::alignment::Vertical::Bottom),
        container(
            column![
                container(text(poster.title.clone()).font(theme::SANS_SEMI).size(13.5).color(ui::faded(INK))).max_height(36.0).clip(true),
                ui::marquee(vec![ui::piece(poster.version.clone(), theme::SANS, 11.5, MUTED)]),
            ]
            .spacing(2),
        )
        .height(54.0),
        row![ui::mono_small(w.percent(poster.accuracy * f), INK), ui::grow(), tail].align_y(iced::Center),
    ]
    .spacing(6)
    .padding(Padding { top: 0.0, right: 12.0, bottom: 12.0, left: 12.0 });
    let card = container(column![top, body].spacing(0)).width(Length::Fill).style(move |_| container::Style {
        background: Some(Background::Color(Color { a: k, ..ground_colour })),
        border: Border { radius: 14.0.into(), ..Border::default() },
        shadow: if chosen { Shadow { color: Color { a: 0.28 * k, ..colour }, offset: Vector::ZERO, blur_radius: 24.0 } } else { Shadow::default() },
        ..container::Style::default()
    });
    let frame = button(Space::new().width(Length::Fill).height(Length::Fill))
        .padding(0)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_, _| button::Style {
            background: None,
            text_color: INK,
            border: Border { color: if chosen { Color { a: k, ..colour } } else { Color::from_rgba(1.0, 1.0, 1.0, 0.08 * k) }, width: if chosen { 1.5 } else { 1.0 }, radius: 14.0.into() },
            shadow: Shadow::default(),
            snap: true,
        })
        .on_press(Message::PlayOpen(index));
    let glow = if chosen { ui::Glow::card(14.0).edge(Color::TRANSPARENT).shadow(Color::TRANSPARENT) } else { ui::Glow::card(14.0).edge(Color::from_rgba(1.0, 1.0, 1.0, 0.3)).shadow(Color::from_rgba(0.0, 0.0, 0.0, 0.5)) };
    ui::hover(container(stack![card, frame]).width(Length::FillPortion(1)), glow)
}

fn judgement_bar<'a>(ground: &Ground<'a>, counts: [Option<u32>; 4], f: f64) -> Option<Element<'a, Message>> {
    let w = ground.words;
    let k = ui::fade();
    let shades = [theme::HIT_300, theme::HIT_100, theme::HIT_50, ACCENT];
    let labels = ["300", "100", "50", "✕"];
    let total: u32 = counts.iter().flatten().sum();
    if total == 0 {
        return None;
    }
    let mut bar = row![].spacing(2).height(5.0);
    for (count, colour) in counts.iter().zip(shades) {
        if let Some(n) = count.filter(|n| *n > 0) {
            bar = bar.push(container(Space::new().height(5.0)).width(Length::FillPortion((n.clamp(1, 65_000)) as u16)).style(move |_| container::Style {
                background: Some(Background::Color(Color { a: k, ..colour })),
                border: Border { radius: 3.0.into(), ..Border::default() },
                ..container::Style::default()
            }));
        }
    }
    let mut tiles = row![].spacing(6);
    for ((count, colour), label) in counts.iter().zip(shades).zip(labels) {
        if let Some(n) = count {
            tiles = tiles.push(
                container(column![ui::mono_small(label.to_owned(), colour), text(w.lang().group((f64::from(*n) * f).round() as u64)).font(theme::SANS_SEMI).size(15.0).wrapping(text::Wrapping::None).color(ui::faded(INK))].spacing(1))
                    .padding([6, 10])
                    .width(Length::FillPortion(1))
                    .clip(true)
                    .style(move |_| container::Style {
                        background: Some(Background::Color(Color { a: k, ..Color::from_rgb8(0x12, 0x09, 0x0c) })),
                        border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.07 * k), width: 1.0, radius: 9.0.into() },
                        ..container::Style::default()
                    }),
            );
        }
    }
    Some(column![bar.width(Length::Fill), tiles].spacing(8).into())
}

fn fact_pill<'a>(inside: Element<'a, Message>) -> Element<'a, Message> {
    let k = ui::fade();
    container(inside)
        .padding(Padding { top: 3.0, right: 9.0, bottom: 3.0, left: 9.0 })
        .style(move |_| container::Style {
            background: Some(Background::Color(Color { a: k, ..Color::from_rgb8(0x24, 0x18, 0x1b) })),
            border: Border { radius: 11.0.into(), ..Border::default() },
            ..container::Style::default()
        })
        .into()
}

fn poster_detail<'a>(ground: &Ground<'a>, poster: &Poster<'a>, f: f64) -> Element<'a, Message> {
    let w = ground.words;
    let mut facts: Vec<Element<'a, Message>> = Vec::new();
    let combo_said = match (poster.combo, poster.most) {
        (Some(combo), Some(most)) => Some(format!("{} / {}x", w.lang().group(u64::from(combo)), w.lang().group(u64::from(most)))),
        (Some(combo), None) => Some(format!("{}x", w.lang().group(u64::from(combo)))),
        _ => None,
    };
    if let Some(said) = combo_said {
        let full = poster.full();
        let colour = if full { GREEN } else { MUTED };
        let said = if full { format!("{said} · FC") } else { poster.misses().filter(|n| *n > 0).map_or(said.clone(), |n| format!("{said} · {n} ✕")) };
        facts.push(fact_pill(row![glyph(Icon::Chain, 12.0, colour), ui::mono_small(said, colour)].spacing(5).align_y(iced::Center).into()));
    }
    if let Some(bpm) = poster.bpm {
        facts.push(fact_pill(row![glyph(Icon::Metronome, 12.0, MUTED), ui::mono_small(format!("{} BPM", bpm.round() as u32), MUTED)].spacing(5).align_y(iced::Center).into()));
    }
    if let Some(length) = poster.length {
        facts.push(fact_pill(row![glyph(Icon::Clock, 12.0, MUTED), ui::mono_small(format!("{}:{:02}", length as u32 / 60, length as u32 % 60), MUTED)].spacing(5).align_y(iced::Center).into()));
    }
    if let Some(at) = poster.played {
        facts.push(fact_pill(row![glyph(Icon::Calendar, 12.0, MUTED), ui::mono_small(w.day(at, ground.now_unix), MUTED)].spacing(5).align_y(iced::Center).into()));
    }
    if !poster.mods.is_empty() {
        facts.push(screen::mods(&poster.mods));
    }
    let mut under = vec![format!("[{}]", poster.version)];
    if !poster.mapper.is_empty() {
        under.push(format!("{} {}", w.t("mapped-by"), poster.mapper));
    }
    let mut left = column![].spacing(4).width(Length::FillPortion(13));
    if !poster.artist.is_empty() {
        left = left.push(ui::marquee(vec![ui::piece(poster.artist.clone(), theme::SANS, 12.5, MUTED)]));
    }
    let left = left
        .push(ui::marquee(vec![ui::piece(poster.title.clone(), theme::SANS_SEMI, 19.0, INK)]))
        .push(ui::marquee(vec![ui::piece(under.join(" · "), theme::SANS, 12.5, MUTED)]))
        .push(container(ui::wrap(facts, 6.0)).padding(Padding::ZERO.top(6.0)));
    let mut right = column![].spacing(12).width(Length::FillPortion(10));
    if let Some(bar) = judgement_bar(ground, poster.counts, f) {
        right = right.push(bar);
    }
    if let Some(set) = poster.set {
        right = right.push(row![
            ui::grow(),
            ui::hover(
                button(row![glyph(Icon::External, 13.0, MUTED), text(w.t("open-map")).font(theme::SANS_SEMI).size(12.5).color(ui::faded(MUTED))].spacing(6).align_y(iced::Center))
                    .padding([7, 12])
                    .style(ui::button_faded(ui::calm(outline)))
                    .on_press(Message::Open(format!("https://osu.ppy.sh/beatmapsets/{set}"))),
                ui::Glow::tile(10.0),
            ),
        ]);
    }
    let k = ui::fade();
    let high = 156.0;
    let colour = screen::grade_colour(&poster.grade);
    let content = container(row![left, right].spacing(20).align_y(iced::Center)).padding([14, 18]).width(Length::Fill).height(high).center_y(high);
    container(stack![screen::backdrop(poster.cover, high, 14.0, colour, true), content].height(high))
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color { a: k, ..Color::from_rgb8(0x16, 0x0c, 0x0f) })),
            border: Border { color: Color { a: 0.35 * k, ..colour }, width: 1.0, radius: 14.0.into() },
            ..container::Style::default()
        })
        .into()
}

fn best_plays<'a>(ground: &Ground<'a>, whose: &Whose<'a>, wide: f32, t: f32) -> Element<'a, Message> {
    let w = ground.words;
    let posters = posters_of(ground, whose);
    let head = caption(w.t("best-plays"));
    if posters.is_empty() {
        return slab(column![head, ui::mono_small(w.t("nothing-yet"), FAINT)].spacing(10), [14, 16]).into();
    }
    let chosen = ground.play_open.filter(|at| *at < posters.len()).unwrap_or(0);
    let columns = if wide >= 720.0 { 5 } else { 3 };
    let each = (wide - 32.0 - 10.0 * (columns as f32 - 1.0)) / columns as f32;
    let cards: Vec<Element<'a, Message>> = posters.iter().enumerate().map(|(at, poster)| poster_card(ground, at, poster, at == chosen, each, ui::tally(t, 0.2 + 0.05 * at as f32))).collect();
    let detail = ui::appearing(ui::appear(ground.play_t, 0), 8.0, || poster_detail(ground, &posters[chosen], ui::tally(ground.play_t.min(t), 0.1)));
    slab(column![head, screen::grid(cards, columns, 10.0), detail].spacing(12), [14, 16]).into()
}

fn grades<'a>(ground: &Ground<'a>, card: &wire::Card) -> Element<'a, Message> {
    let w = ground.words;
    let g = &card.grade_counts;
    let entries = [("A", g.a, Color::from_rgb8(80, 200, 80), "grade-a"), ("S", g.s, Color::from_rgb8(255, 215, 0), "grade-s"), ("S", g.sh, Color::from_rgb8(220, 220, 240), "grade-sh"), ("SS", g.ss, Color::from_rgb8(255, 215, 0), "grade-ss"), ("SS", g.ssh, Color::from_rgb8(220, 220, 240), "grade-ssh")];
    let total: f64 = entries.iter().map(|e| e.1.max(0.0)).sum();
    let share = |n: f64| screen::decimal(w, (n / total.max(1.0) * 100.0) as f32, 1) + "%";
    let mut tiles = row![].spacing(5);
    let mut bar = row![].spacing(2).height(8.0);
    for (index, (letter, n, colour, _)) in entries.iter().enumerate() {
        let on = ground.grade_hover == Some(index);
        let dim = ground.grade_hover.is_some() && !on;
        let k = ui::fade() * if dim { 0.5 } else { 1.0 };
        let colour = *colour;
        let tile = ui::fading(k, || -> Element<'a, Message> {
            let k = ui::fade();
            container(
                column![
                    text(letter.to_string()).font(theme::SANS_SEMI).size(20.0).color(ui::faded(colour)),
                    text(w.lang().group(*n as u64)).font(theme::SANS_SEMI).size(13.0).color(ui::faded(INK)),
                    ui::mono_small(share(*n), FAINT),
                ]
                .spacing(1)
                .align_x(iced::alignment::Horizontal::Center),
            )
            .padding([8, 0])
            .center_x(Length::Fill)
            .style(move |_| container::Style {
                background: Some(Background::Color(if on { Color { a: 0.1 * k, ..colour } } else { Color::from_rgba(0.0, 0.0, 0.0, 0.18 * k) })),
                border: Border { color: if on { Color { a: 0.5 * k, ..colour } } else { Color::from_rgba(1.0, 1.0, 1.0, 0.012 * k) }, width: 1.0, radius: 10.0.into() },
                shadow: if on { Shadow { color: Color { a: 0.3 * k, ..colour }, offset: Vector::ZERO, blur_radius: 14.0 } } else { Shadow::default() },
                ..container::Style::default()
            })
            .into()
        });
        tiles = tiles.push(mouse_area(container(tile).width(Length::FillPortion(1))).on_enter(Message::GradeHover(Some(index))));
        if *n > 0.0 {
            let alpha = ui::fade() * if dim { 0.25 } else { 1.0 };
            bar = bar.push(container(Space::new().height(8.0)).width(Length::FillPortion((*n as u16).max(1))).style(move |_| container::Style { background: Some(Background::Color(Color { a: alpha, ..colour })), border: Border { radius: 4.0.into(), ..Border::default() }, ..container::Style::default() }));
        }
    }
    let said = match ground.grade_hover.and_then(|index| entries.get(index)) {
        Some((_, n, colour, key)) => ui::mono_small(format!("{} · {} {}", w.t(key), share(*n), w.t("of-all-grades")), *colour),
        None => ui::mono_small(w.lang().group(total as u64), INK),
    };
    let body = column![row![caption(w.t("card-grades")), ui::grow(), said].align_y(iced::Center), tiles, bar.width(Length::Fill)].spacing(10);
    mouse_area(slab(body, [14, 16])).on_exit(Message::GradeHover(None)).into()
}

fn chip_style(colour: Color, held: bool, on: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let lit = on || matches!(status, button::Status::Hovered);
        button::Style {
            background: Some(Background::Color(if held { Color { a: if lit { 0.2 } else { 0.1 }, ..colour } } else { Color::TRANSPARENT })),
            text_color: if held { colour } else { FAINT },
            border: Border { color: Color { a: if lit { 0.9 } else if held { 0.4 } else { 0.25 }, ..colour }, width: 1.0, radius: 8.0.into() },
            shadow: if on { Shadow { color: Color { a: 0.35, ..colour }, offset: Vector::ZERO, blur_radius: 12.0 } } else { Shadow::default() },
            snap: true,
        }
    }
}

fn titles<'a>(ground: &Ground<'a>, whose: &Whose<'a>) -> Element<'a, Message> {
    let you = whose.person;
    let w = ground.words;
    let catalog = ground.catalog;
    let held = |title: &Title| you.titles.iter().any(|code| *code == title.code);
    let mut bars = row![].spacing(3);
    for rarity in Rarity::ALL {
        let all = catalog.titles.iter().filter(|t| t.rarity == rarity).count();
        if all == 0 {
            continue;
        }
        let got = catalog.titles.iter().filter(|t| t.rarity == rarity && held(t)).count();
        let colour = rarity.colour();
        let k = ui::fade();
        let fill = ((got as f32 / all as f32) * 100.0).round() as u16;
        let mut inner = row![].height(6.0);
        if fill > 0 {
            inner = inner.push(container(Space::new().height(6.0)).width(Length::FillPortion(fill)).style(move |_| container::Style { background: Some(Background::Color(Color { a: k, ..colour })), border: Border { radius: 3.0.into(), ..Border::default() }, ..container::Style::default() }));
        }
        if fill < 100 {
            inner = inner.push(Space::new().width(Length::FillPortion(100 - fill)).height(6.0));
        }
        bars = bars.push(container(inner).width(Length::FillPortion(all as u16)).style(move |_| container::Style { background: Some(Background::Color(Color { a: 0.15 * k, ..colour })), border: Border { radius: 3.0.into(), ..Border::default() }, ..container::Style::default() }));
    }
    let mut owned: Vec<&Title> = catalog.titles.iter().filter(|t| held(t)).collect();
    owned.sort_by_key(|t| std::cmp::Reverse(Rarity::ALL.iter().position(|r| *r == t.rarity)));
    let mut locked: Vec<&Title> = catalog.titles.iter().filter(|t| !held(t)).collect();
    locked.sort_by_key(|t| std::cmp::Reverse(Rarity::ALL.iter().position(|r| *r == t.rarity)));
    let chosen_code = ground.title_pick.map(str::to_owned).or_else(|| you.title.clone()).or_else(|| owned.first().map(|t| t.code.clone()));
    let mut chips: Vec<Element<'a, Message>> = Vec::new();
    for (title, got) in owned.iter().map(|t| (*t, true)).chain(locked.iter().take(5).map(|t| (*t, false))) {
        let secret = !got && title.rarity == Rarity::Secret;
        let name = if secret { "???".to_owned() } else { title.name(w.lang()).to_owned() };
        let on = chosen_code.as_deref() == Some(title.code.as_str());
        chips.push(ui::hover(
            button(text(name).font(theme::SANS_SEMI).size(11.5))
                .padding([4, 9])
                .style(ui::button_faded(ui::calm(chip_style(title.rarity.colour(), got, on))))
                .on_press(Message::TitlePick(title.code.clone())),
            ui::Glow::tile(10.0).edge(Color { a: 0.45, ..title.rarity.colour() }),
        ));
    }
    let detail: Element<'a, Message> = match chosen_code.as_deref().and_then(|code| catalog.title_of(code)) {
        Some(title) => {
            let got = held(title);
            let secret = !got && title.rarity == Rarity::Secret;
            let colour = title.rarity.colour();
            let holders = catalog.holders(&title.code).len();
            let when = match (got, whose.me.and_then(|me| me.title_dates.get(&title.code))) {
                (true, Some(at)) => format!("{} {} · {} {}", w.t("title-got"), w.day(*at, ground.now_unix), w.t("held-by"), w.of(holders as u64, catalog.people.len() as u64)),
                (true, None) => format!("{} {}", w.t("held-by"), w.of(holders as u64, catalog.people.len() as u64)),
                (false, _) => format!("{} · {} {}", w.t("title-not-yet"), w.t("held-by"), w.of(holders as u64, catalog.people.len() as u64)),
            };
            let k = ui::fade();
            container(
                column![
                    row![
                        text(if secret { "???".to_owned() } else { title.name(w.lang()).to_owned() }).font(theme::SANS_SEMI).size(14.0).color(ui::faded(colour)),
                        ui::mono_small(w.t(title.rarity.key()).to_uppercase(), colour),
                    ]
                    .spacing(8)
                    .align_y(iced::Center),
                    text(if secret { w.t("secret-title") } else { title.about(w.lang()).to_owned() }).font(theme::SANS).size(12.0).color(ui::faded(MUTED)),
                    row![ui::mono_small(when, FAINT), ui::grow(), wear_button(ground, you, title)].align_y(iced::Center),
                ]
                .spacing(3),
            )
            .padding([10, 12])
            .width(Length::Fill)
            .style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.24 * k))), border: Border { radius: 10.0.into(), ..Border::default() }, ..container::Style::default() })
            .into()
        }
        None => Space::new().height(0.0).into(),
    };
    let count = w.of(owned.len() as u64, catalog.titles.len() as u64);
    slab(column![row![caption(w.t("community-titles")), ui::grow(), ui::mono_small(count, INK)].align_y(iced::Center), bars.width(Length::Fill), ui::wrap(chips, 6.0), detail].spacing(10), [14, 16]).into()
}

pub fn wear_button<'a>(ground: &Ground<'a>, you: &crate::community::Person, title: &Title) -> Element<'a, Message> {
    let w = ground.words;
    if !you.you || !you.titles.iter().any(|code| *code == title.code) {
        return Space::new().width(0.0).into();
    }
    let worn = you.title.as_deref() == Some(title.code.as_str());
    let (words, press) = if worn { (w.t("take-off-title"), Message::Wear(None)) } else { (w.t("wear-title"), Message::Wear(Some(title.code.clone()))) };
    ui::hover(
        button(text(words).font(theme::SANS_SEMI).size(11.5))
            .padding([4, 11])
            .style(ui::button_faded(ui::calm(chip_style(title.rarity.colour(), true, worn))))
            .on_press(press),
        ui::Glow::tile(10.0).edge(Color { a: 0.45, ..title.rarity.colour() }),
    )
    .into()
}

const HEAT_PAD: f32 = 2.0;

fn activity<'a>(ground: &Ground<'a>, whose: &Whose<'a>) -> Element<'a, Message> {
    let you = whose.person;
    let w = ground.words;
    let today = ground.now_unix - ground.now_unix.rem_euclid(DAY);
    let first = today - 90 * DAY;
    let played = whose.me.map(|me| me.activity.as_slice()).unwrap_or_default();
    let weekday = |at: i64| -> String {
        let index = ((at / DAY) + 3).rem_euclid(7) as usize;
        w.t(["weekday-mon", "weekday-tue", "weekday-wed", "weekday-thu", "weekday-fri", "weekday-sat", "weekday-sun"][index])
    };
    let days: Vec<(String, u32)> = (0..91)
        .map(|index| {
            let at = first + index * DAY;
            let n = played.iter().filter(|(day, _)| (*day - at).abs() < DAY / 2).map(|(_, n)| *n).sum();
            (format!("{}, {}", weekday(at), w.day(at, ground.now_unix)), n)
        })
        .collect();
    let most = days.iter().map(|d| d.1).max().unwrap_or(0) as usize;
    let plays = (0..=most).map(|n| w.n("plays-n", n as u64)).collect();
    let heat = Canvas::new(Heat { days, none: w.t("no-plays"), plays, less: w.t("less"), more: w.t("more"), alpha: ui::fade() }).width(Length::Fill).height(HEAT_PAD + 7.0 * (CELL + CELL_GAP) + 40.0);
    let mut head = row![caption(w.t("activity-head")), ui::grow()].align_y(iced::Center);
    if you.streak > 0 {
        head = head.push(ui::mono_small(w.n("streak-card", u64::from(you.streak)), CORAL));
    }
    slab(column![head, heat].spacing(10), [14, 16]).into()
}

pub struct Whose<'a> {
    pub at: usize,
    pub person: &'a Person,
    pub card: wire::Card,
    pub me: Option<&'a crate::community::Me>,
}

pub fn card_from(person: &Person) -> wire::Card {
    wire::Card {
        username: person.name.clone(),
        pp: f64::from(person.pp),
        global_rank: f64::from(person.rank),
        accuracy: f64::from(person.accuracy),
        play_count: f64::from(person.plays),
        play_seconds: f64::from(person.hours) * 3600.0,
        ranked_score: person.score as f64,
        level: f64::from(person.level),
        country: person.country.clone(),
        avatar_url: person.avatar.clone(),
        cover_url: person.cover.clone(),
        grade_counts: wire::Grades { ss: f64::from(person.ss), s: f64::from(person.s), ..wire::Grades::default() },
        ..wire::Card::default()
    }
}

pub fn columns<'a>(ground: &Ground<'a>, whose: &Whose<'a>, room: f32, t: f32) -> Element<'a, Message> {
    let block = |index: usize, build: &dyn Fn() -> Element<'a, Message>| ui::appearing(ui::appear(t, index), 14.0, build);
    let rolled = |inside: Element<'a, Message>| -> Element<'a, Message> {
        scrollable(container(inside).padding(Padding { top: 2.0, right: 8.0, bottom: 28.0, left: 0.0 })).style(ui::thin_scroll).direction(ui::hidden_bar()).height(Length::Fill).into()
    };
    let wide = room >= LEFT + RIGHT + 540.0 + 32.0;
    let left = (room * 0.21).clamp(LEFT, SIDE_MOST);
    let right = (room * 0.21).clamp(RIGHT, SIDE_MOST);
    let middle_wide = if wide { room - left - right - 32.0 } else { room - left - 16.0 };
    let middle = column![block(0, &|| metrics(ground, whose, t)), block(1, &|| chart(ground, whose)), block(2, &|| best_plays(ground, whose, middle_wide, t))].spacing(14);
    if wide {
        row![
            container(rolled(column![block(0, &|| identity(ground, whose)), block(1, &|| places(ground, whose, t))].spacing(14).into())).width(left).height(Length::Fill),
            container(rolled(middle.into())).width(Length::Fill).height(Length::Fill),
            container(rolled(column![block(1, &|| grades(ground, &whose.card)), block(2, &|| titles(ground, whose)), block(3, &|| activity(ground, whose))].spacing(14).into())).width(right).height(Length::Fill),
        ]
        .spacing(16)
        .into()
    } else {
        row![
            container(rolled(column![block(0, &|| identity(ground, whose)), block(1, &|| places(ground, whose, t)), block(2, &|| grades(ground, &whose.card))].spacing(14).into())).width(left).height(Length::Fill),
            container(rolled(middle.push(block(3, &|| titles(ground, whose))).push(block(4, &|| activity(ground, whose))).into())).width(Length::Fill).height(Length::Fill),
        ]
        .spacing(16)
        .into()
    }
}

pub fn view<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let Some(at) = ground.catalog.people.iter().position(|p| p.you) else {
        return container(ui::mono_small(w.t("nothing-yet"), FAINT)).center(Length::Fill).into();
    };
    let person = &ground.catalog.people[at];
    let Some(card) = ground.card.cloned().or_else(|| ground.catalog.card_of()) else {
        return container(ui::mono_small(w.t("nothing-yet"), FAINT)).center(Length::Fill).into();
    };
    let whose = Whose { at, person, card, me: ground.catalog.me.as_ref() };
    container(columns(ground, &whose, ground.width - 80.0, ground.section_t)).padding(Padding { top: 12.0, right: 40.0, bottom: 0.0, left: 40.0 }).width(Length::Fill).height(Length::Fill).into()
}
