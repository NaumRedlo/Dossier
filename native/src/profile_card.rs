use std::collections::HashMap;

use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::widget::{button, column, container, image, row, stack, svg, text, Space};
use iced::{color, mouse, Background, Border, Color, Element, Font, Length, Padding, Point, Rectangle, Renderer, Shadow, Size, Theme, Vector};

use crate::community::wire::{Card, Score};
use crate::lang::{Lang, Words};
use crate::theme;
use crate::ui;

const CARD: Color = color!(0x171318);
const CARD_LINE: Color = color!(0x4a3438);
const PANEL: Color = color!(0x1e181e);
const PANEL_LINE: Color = color!(0x402e32);
const TEXT: Color = color!(0xeceaee);
const MUTED: Color = color!(0x9c9096);
const RED: Color = color!(0xe24848);
const CORAL: Color = color!(0xf06868);
const POSITIVE: Color = color!(0x7ade8e);
const TRACK: Color = color!(0x3e3034);
const DIVIDER: Color = color!(0x443236);
const HEART: Color = color!(0xff6eb2);
const HANDLE: Color = color!(0xbc9698);
const LABEL: Color = color!(0xd0cede);
const RING: Color = color!(0xe44c4c);
const BLANK: Color = color!(0x34282a);
const POSTER: Color = color!(0x2c2224);
const POSTER_ACC: Color = color!(0xcdcbd6);
const LINE: Color = color!(0xec5c5c);
const DOT: Color = color!(0xf57878);
const GRID: Color = color!(0x2e2628);
const GRADE_A: Color = color!(0x50c850);
const GOLD: Color = color!(0xffd700);
const SILVER: Color = color!(0xdcdcf0);
const GRADE_B: Color = color!(0x508cdc);
const GRADE_C: Color = color!(0xc89632);
const GRADE_D: Color = color!(0xc83232);
const GRADE_F: Color = color!(0x646464);

pub struct Seen<'a> {
    pub words: &'a Words,
    pub pictures: &'a HashMap<String, image::Handle>,
    pub thumbs: &'a HashMap<String, image::Handle>,
    pub flags: &'a HashMap<String, svg::Handle>,
    pub avatar: Option<&'a image::Handle>,
    pub title: Option<(String, Color)>,
    pub now: i64,
}

pub fn flag_url(country: &str) -> Option<String> {
    let code = country.trim();
    if code.len() != 2 || !code.chars().all(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    let points: Vec<String> = code.to_ascii_uppercase().chars().map(|c| format!("{:x}", 0x1f1e6 + (c as u32 - 'A' as u32))).collect();
    Some(format!("https://osu.ppy.sh/assets/images/flags/{}.svg", points.join("-")))
}

pub fn pictures(card: &Card) -> Vec<(String, u32)> {
    let mut wanted = Vec::new();
    if !card.avatar_url.is_empty() {
        wanted.push((card.avatar_url.clone(), 256));
    }
    if !card.cover_url.is_empty() {
        wanted.push((card.cover_url.clone(), 1400));
    }
    wanted.extend(card.top_scores.iter().filter_map(Score::cover).map(|url| (url, 400)));
    wanted
}

fn spaced(n: f64) -> String {
    let whole = n.max(0.0).round() as u64;
    let digits = whole.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (at, ch) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at) % 3 == 0 {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}

fn date(iso: &str) -> String {
    let day = iso.split('T').next().unwrap_or_default();
    let parts: Vec<&str> = day.split('-').collect();
    match parts.as_slice() {
        [y, m, d, ..] if !y.is_empty() => format!("{d}.{m}.{y}"),
        _ => "—".to_owned(),
    }
}

fn grade_letter(rank: &str) -> String {
    match rank.to_ascii_uppercase().as_str() {
        "X" | "XH" | "SS" | "SSH" => "X".to_owned(),
        "SH" => "S".to_owned(),
        other if other.is_empty() => "F".to_owned(),
        other => other.to_owned(),
    }
}

fn grade_colour(rank: &str) -> Color {
    match rank.to_ascii_uppercase().as_str() {
        "X" | "SS" => GOLD,
        "XH" | "SSH" => SILVER,
        "S" => GOLD,
        "SH" => SILVER,
        "A" => GRADE_A,
        "B" => GRADE_B,
        "C" => GRADE_C,
        "D" => GRADE_D,
        _ => GRADE_F,
    }
}

fn say<'a, M: 'a>(words: impl Into<String>, font: Font, size: f32, colour: Color) -> Element<'a, M> {
    text(words.into()).font(font).size(size).wrapping(text::Wrapping::None).color(ui::faded(colour)).into()
}

fn fill_style(fill: Color, line: Color, radius: f32) -> impl Fn(&Theme) -> container::Style {
    let k = ui::fade();
    move |_| container::Style {
        background: Some(Background::Color(Color { a: fill.a * k, ..fill })),
        border: Border { color: Color { a: line.a * k, ..line }, width: if line.a > 0.0 { 1.0 } else { 0.0 }, radius: radius.into() },
        ..container::Style::default()
    }
}

fn panel<'a, M: 'a>(inside: impl Into<Element<'a, M>>, radius: f32, padding: Padding) -> container::Container<'a, M> {
    container(inside).padding(padding).width(Length::Fill).style(fill_style(PANEL, PANEL_LINE, radius))
}

fn section<'a, M: 'a>(words: String, size: f32) -> Element<'a, M> {
    say(words, theme::SANS_SEMI, size, RED)
}

fn divider<'a, M: 'a>() -> Element<'a, M> {
    container(Space::new().height(1.0)).width(Length::Fill).style(fill_style(DIVIDER, Color::TRANSPARENT, 0.0)).into()
}

fn seen_said(words: &Words, card: &Card, now: i64) -> (String, Color) {
    if card.is_online {
        return (words.t("card-online"), POSITIVE);
    }
    let Some(at) = crate::news::unix_of(&card.last_visit) else {
        return (words.t("card-hidden"), MUTED);
    };
    let seconds = (now - at).max(0);
    let said = match seconds {
        s if s < 90 => words.t("live-now"),
        s if s < 3600 => words.n("minutes-ago", (s / 60) as u64),
        s if s < 86_400 => words.n("hours-ago", (s / 3600) as u64),
        s if s < 35 * 86_400 => words.n("days-ago", (s / 86_400) as u64),
        _ => date(&card.last_visit),
    };
    (said, TEXT)
}

fn ring<'a, M: 'a>(seen: &Seen<'a>, card: &Card, side: f32, band: f32) -> Element<'a, M> {
    let k = ui::fade();
    let inner: Element<'a, M> = match seen.pictures.get(&card.avatar_url).or(seen.avatar) {
        Some(handle) => image(handle.clone()).content_fit(iced::ContentFit::Cover).width(side).height(side).border_radius(side / 2.0).opacity(k).into(),
        None => container(Space::new().width(side).height(side)).style(fill_style(BLANK, Color::TRANSPARENT, side / 2.0)).into(),
    };
    let outer = side + band * 2.0;
    container(inner)
        .padding(band)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color { a: k, ..RING })),
            border: Border { radius: (outer / 2.0).into(), ..Border::default() },
            shadow: Shadow { color: Color::from_rgba8(228, 72, 72, 0.6 * k), offset: Vector::ZERO, blur_radius: outer * 0.15 },
            ..container::Style::default()
        })
        .into()
}

fn heart<'a, M: 'a>(high: f32) -> Element<'a, M> {
    let k = ui::fade();
    let glyph = Canvas::new(Glyph { icon: Icon::Heart, alpha: k, colour: Color::WHITE }).width(high * 0.62).height(high * 0.62);
    container(glyph)
        .width(high * 1.6)
        .height(high)
        .center(Length::Shrink)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color { a: k, ..HEART })),
            border: Border { radius: (high / 2.0).into(), ..Border::default() },
            shadow: Shadow { color: Color::from_rgba8(255, 110, 178, 0.5 * k), offset: Vector::ZERO, blur_radius: high * 0.5 },
            ..container::Style::default()
        })
        .into()
}

fn flag<'a, M: 'a>(seen: &Seen<'a>, card: &Card, high: f32) -> Option<Element<'a, M>> {
    let handle = seen.flags.get(&card.country.to_ascii_lowercase())?;
    Some(svg(handle.clone()).width(high * 1.5).height(high).opacity(ui::fade()).into())
}

struct Sizes {
    side: f32,
    band: f32,
    name: f32,
    handle: f32,
    title: f32,
    country: f32,
    label: f32,
    rank: f32,
    country_rank: f32,
    high: f32,
    flag: f32,
}

fn hero<'a, M: 'a>(seen: &Seen<'a>, card: &Card, compact: bool) -> Element<'a, M> {
    let w = seen.words;
    let z = if compact {
        Sizes { side: 58.0, band: 3.5, name: 19.0, handle: 11.5, title: 13.5, country: 11.5, label: 10.5, rank: 19.0, country_rank: 16.0, high: 92.0, flag: 12.0 }
    } else {
        Sizes { side: 138.0, band: 5.5, name: 36.0, handle: 18.0, title: 22.0, country: 15.0, label: 13.0, rank: 32.0, country_rank: 28.0, high: 200.0, flag: 20.0 }
    };
    let k = ui::fade();
    let radius = if compact { 10.0 } else { 16.0 };
    let back: Element<'a, M> = match seen.pictures.get(&card.cover_url) {
        Some(handle) => image(handle.clone()).content_fit(iced::ContentFit::Cover).width(Length::Fill).height(z.high).border_radius(radius).opacity(k).into(),
        None => container(Space::new().width(Length::Fill).height(z.high)).style(fill_style(CARD, Color::TRANSPARENT, radius)).into(),
    };
    let scrim = container(Space::new().width(Length::Fill).height(z.high)).style(fill_style(Color::from_rgba8(12, 7, 9, 0.62), Color::TRANSPARENT, radius));
    let mut name = row![say(card.username.clone(), theme::SANS_SEMI, z.name, TEXT)].spacing(10).align_y(iced::Center);
    if card.is_supporter {
        name = name.push(heart(z.name * 0.62));
    }
    let mut lines = column![name].spacing(if compact { 1.0 } else { 3.0 });
    if !card.handle.is_empty() {
        lines = lines.push(say(card.handle.clone(), theme::SANS, z.handle, HANDLE));
    }
    if let Some((title, colour)) = &seen.title {
        lines = lines.push(say(title.clone(), theme::SANS_SEMI, z.title, *colour));
    }
    let place = if compact || card.country_name.is_empty() { card.country.to_ascii_uppercase() } else { card.country_name.clone() };
    let place = if place.is_empty() { w.t("card-unknown-country") } else { place };
    let mut country = row![].spacing(if compact { 6.0 } else { 10.0 }).align_y(iced::Center);
    if let Some(flag) = flag(seen, card, z.flag) {
        country = country.push(flag);
    }
    lines = lines.push(country.push(say(place, theme::SANS_SEMI, z.country, TEXT)));
    let rank = |n: f64| if n > 0.0 { format!("#{}", spaced(n)) } else { "—".to_owned() };
    let ranks = column![
        say(w.t("card-global-rank"), theme::SANS_SEMI, z.label, MUTED),
        say(rank(card.global_rank), theme::SANS_SEMI, z.rank, TEXT),
        Space::new().height(if compact { 3.0 } else { 18.0 }),
        say(w.t("card-country-rank"), theme::SANS_SEMI, z.label, MUTED),
        say(rank(card.country_rank), theme::SANS_SEMI, z.country_rank, CORAL),
    ]
    .spacing(if compact { 0.0 } else { 2.0 });
    let front = row![ring(seen, card, z.side, z.band), lines, ui::grow(), ranks]
        .spacing(if compact { 12.0 } else { 26.0 })
        .align_y(iced::Center)
        .padding(if compact { Padding::from([0, 12]) } else { Padding::from([0, 30]) })
        .height(z.high);
    stack![back, scrim, front].width(Length::Fill).height(z.high).into()
}

fn level_bar<'a, M: 'a>(progress: f64, high: f32) -> Element<'a, M> {
    let k = ui::fade();
    let filled = progress.clamp(0.0, 100.0).round() as u16;
    let mut inside = row![].height(high);
    if filled > 0 {
        let gradient = iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2))
            .add_stop(0.0, Color::from_rgba8(200, 52, 52, k))
            .add_stop(1.0, Color::from_rgba8(240, 124, 96, k));
        inside = inside.push(container(Space::new().height(high)).width(Length::FillPortion(filled)).style(move |_| container::Style {
            background: Some(Background::Gradient(iced::Gradient::Linear(gradient))),
            border: Border { radius: (high / 2.0).into(), ..Border::default() },
            ..container::Style::default()
        }));
    }
    if filled < 100 {
        inside = inside.push(Space::new().width(Length::FillPortion(100 - filled)).height(high));
    }
    container(inside).width(Length::Fill).height(high).style(fill_style(TRACK, Color::TRANSPARENT, high / 2.0)).into()
}

fn strip<'a, M: 'a>(seen: &Seen<'a>, card: &Card, compact: bool) -> Element<'a, M> {
    let w = seen.words;
    let (label, value) = if compact { (10.5, 17.0) } else { (14.0, 30.0) };
    let cell = |key: &str, said: String, colour: Color| -> Element<'a, M> {
        column![say(w.t(key), theme::SANS_SEMI, label, MUTED), say(said, theme::SANS_SEMI, value, colour)].spacing(if compact { 1.0 } else { 4.0 }).into()
    };
    let pp = if card.pp > 0.0 { format!("{}pp", spaced(card.pp)) } else { "—".to_owned() };
    let level = column![
        say(w.t("card-level"), theme::SANS_SEMI, label, MUTED),
        row![
            say(format!("{}", card.level.floor() as u64), theme::SANS_SEMI, value, CORAL),
            column![
                container(say(format!("{}%", card.level_progress.round() as u64), theme::SANS_SEMI, if compact { 10.0 } else { 15.0 }, CORAL)).width(Length::Fill).align_x(iced::alignment::Horizontal::Right),
                level_bar(card.level_progress, if compact { 6.0 } else { 9.0 }),
            ]
            .spacing(2)
            .width(Length::Fill),
        ]
        .spacing(if compact { 8.0 } else { 14.0 })
        .align_y(iced::Center),
    ]
    .spacing(if compact { 1.0 } else { 4.0 })
    .width(Length::FillPortion(if compact { 5 } else { 6 }));
    let mut cells = row![
        container(cell("card-pp", pp, CORAL)).width(Length::FillPortion(4)),
        container(cell("card-accuracy", format!("{:.2}%", card.accuracy), TEXT)).width(Length::FillPortion(4)),
        container(cell("card-plays", spaced(card.play_count), TEXT)).width(Length::FillPortion(4)),
        level,
    ]
    .spacing(if compact { 10.0 } else { 22.0 })
    .align_y(iced::Center);
    if !compact {
        let (seen_text, seen_colour) = seen_said(w, card, seen.now);
        let joined = if card.join_date.is_empty() { "—".to_owned() } else { date(&card.join_date) };
        cells = cells.push(
            container(
                column![
                    say(w.t("card-joined"), theme::SANS_SEMI, 13.0, MUTED),
                    say(joined, theme::SANS_SEMI, 16.0, TEXT),
                    Space::new().height(4.0),
                    say(w.t("card-seen"), theme::SANS_SEMI, 13.0, MUTED),
                    say(seen_text, theme::SANS_SEMI, 16.0, seen_colour),
                ]
                .spacing(1),
            )
            .width(Length::FillPortion(3)),
        );
    }
    panel(cells, if compact { 10.0 } else { 14.0 }, if compact { Padding::from([8, 12]) } else { Padding::from([14, 24]) }).into()
}

fn glowing<'a, M: 'a>(letter: &str, size: f32, colour: Color) -> Element<'a, M> {
    let k = ui::fade();
    let side = size * 1.7;
    let glow = container(Space::new().width(size * 0.7).height(size * 0.5)).style(move |_| container::Style {
        background: Some(Background::Color(Color { a: 0.2 * k, ..colour })),
        border: Border { radius: (size * 0.3).into(), ..Border::default() },
        shadow: Shadow { color: Color { a: 0.55 * k, ..colour }, offset: Vector::ZERO, blur_radius: size * 0.75 },
        ..container::Style::default()
    });
    stack![
        container(glow).width(side).height(side).center(Length::Fixed(side)),
        container(say(letter.to_owned(), theme::SANS_SEMI, size, colour)).width(side).height(side).center(Length::Fixed(side)),
    ]
    .width(side)
    .height(side)
    .into()
}

fn grades<'a, M: 'a>(card: &Card, compact: bool) -> Element<'a, M> {
    let g = &card.grade_counts;
    let entries = [("A", g.a, GRADE_A), ("S", g.s, GOLD), ("S", g.sh, SILVER), ("SS", g.ss, GOLD), ("SS", g.ssh, SILVER)];
    let (letter, count, bar) = if compact { (16.0, 11.0, 7.0) } else { (28.0, 17.0, 13.0) };
    let mut letters = row![].width(Length::Fill);
    for (said, n, colour) in entries {
        letters = letters.push(
            column![glowing(said, letter, colour), say(spaced(n), theme::SANS_SEMI, count, TEXT)]
                .spacing(if compact { 0.0 } else { 4.0 })
                .align_x(iced::alignment::Horizontal::Center)
                .width(Length::Fill),
        );
    }
    let total: f64 = entries.iter().map(|(_, n, _)| n.max(0.0)).sum();
    let k = ui::fade();
    let mut segments = row![].height(bar);
    let shown: Vec<(u16, Color)> = entries.iter().filter(|(_, n, _)| *n > 0.0).map(|(_, n, colour)| (((n / total.max(1.0)) * 1000.0).round().max(1.0) as u16, *colour)).collect();
    for (at, (share, colour)) in shown.iter().enumerate() {
        let first = at == 0;
        let last = at + 1 == shown.len();
        let radius = iced::border::Radius {
            top_left: if first { bar / 2.0 } else { 0.0 },
            bottom_left: if first { bar / 2.0 } else { 0.0 },
            top_right: if last { bar / 2.0 } else { 0.0 },
            bottom_right: if last { bar / 2.0 } else { 0.0 },
        };
        let colour = *colour;
        segments = segments.push(container(Space::new().height(bar)).width(Length::FillPortion(*share)).style(move |_| container::Style {
            background: Some(Background::Color(Color { a: k, ..colour })),
            border: Border { radius, ..Border::default() },
            ..container::Style::default()
        }));
    }
    let bar_line: Element<'a, M> = if shown.is_empty() {
        container(Space::new().height(bar)).width(Length::Fill).style(fill_style(TRACK, Color::TRANSPARENT, bar / 2.0)).into()
    } else {
        segments.width(Length::Fill).into()
    };
    column![letters, bar_line].spacing(if compact { 5.0 } else { 12.0 }).into()
}

fn poster<'a, M: Clone + 'a>(seen: &Seen<'a>, score: Option<&Score>, high: f32, compact: bool, open: fn(String) -> M) -> Element<'a, M> {
    let k = ui::fade();
    let radius = if compact { 7.0 } else { 12.0 };
    let handle = score.and_then(|score| score.cover().and_then(|url| seen.pictures.get(&url)).or_else(|| seen.thumbs.get(&score.hash)));
    let back: Element<'a, M> = match handle {
        Some(handle) => image(handle.clone()).content_fit(iced::ContentFit::Cover).width(Length::Fill).height(high).border_radius(radius).opacity(k).into(),
        None => container(Space::new().width(Length::Fill).height(high)).style(fill_style(POSTER, Color::TRANSPARENT, radius)).into(),
    };
    let shade_gradient = iced::gradient::Linear::new(iced::Radians(std::f32::consts::PI))
        .add_stop(0.0, Color::from_rgba8(12, 7, 9, 0.0))
        .add_stop(0.38, Color::from_rgba8(12, 7, 9, 0.0))
        .add_stop(1.0, Color::from_rgba8(12, 7, 9, 0.92 * k));
    let shade = container(Space::new().width(Length::Fill).height(high)).style(move |_| container::Style {
        background: Some(Background::Gradient(iced::Gradient::Linear(shade_gradient))),
        border: Border { color: Color { a: k, ..PANEL_LINE }, width: 1.0, radius: radius.into() },
        ..container::Style::default()
    });
    let Some(score) = score else {
        return stack![back, shade].width(Length::Fill).height(high).into();
    };
    let letter = grade_letter(&score.rank);
    let (grade_size, pp_size, acc_size) = if compact { (17.0, 10.5, 8.5) } else { (34.0, 14.0, 11.0) };
    let pp = say(format!("{}pp", score.pp.floor() as u64), theme::SANS_SEMI, pp_size, TEXT);
    let figures: Element<'a, M> = if letter == "X" {
        container(pp).width(Length::Fill).align_x(iced::alignment::Horizontal::Center).into()
    } else {
        column![pp, say(format!("{:.2}%", score.accuracy), theme::SANS, acc_size, POSTER_ACC)].align_x(iced::alignment::Horizontal::Center).width(Length::Fill).into()
    };
    let info = container(row![say(letter, theme::SANS_SEMI, grade_size, grade_colour(&score.rank)), figures].spacing(if compact { 2.0 } else { 6.0 }).align_y(iced::alignment::Vertical::Bottom))
        .width(Length::Fill)
        .height(high)
        .padding(if compact { Padding::from([4, 5]) } else { Padding::from([10, 9]) })
        .align_y(iced::alignment::Vertical::Bottom);
    let made: Element<'a, M> = stack![back, shade, info].width(Length::Fill).height(high).into();
    match score.page() {
        Some(page) => button(made).padding(0).width(Length::Fill).style(ui::button_faded(theme::bare)).on_press(open(page)).into(),
        None => made,
    }
}

fn posters<'a, M: Clone + 'a>(seen: &Seen<'a>, card: &Card, high: f32, compact: bool, open: fn(String) -> M) -> Element<'a, M> {
    let mut shown = row![].spacing(if compact { 6.0 } else { 12.0 }).width(Length::Fill);
    for at in 0..5 {
        shown = shown.push(container(poster(seen, card.top_scores.get(at), high, compact, open)).width(Length::Fill));
    }
    shown.into()
}

fn stats<'a, M: 'a>(seen: &Seen<'a>, card: &Card) -> Element<'a, M> {
    let w = seen.words;
    let average = if card.play_count > 0.0 { spaced((card.total_hits / card.play_count).round()) } else { "—".to_owned() };
    let hours = format!("{}{}", spaced((card.play_seconds / 3600.0).floor()), w.t("card-hours-suffix"));
    let rows = [
        (Icon::Target, "card-hits", spaced(card.total_hits)),
        (Icon::Keys, "card-average-hits", average),
        (Icon::Chain, "card-combo", format!("{}x", spaced(card.maximum_combo))),
        (Icon::Play, "card-replays", spaced(card.replays_watched)),
        (Icon::Star, "card-score", spaced(card.total_score)),
        (Icon::Clock, "card-hours", hours),
    ];
    let k = ui::fade();
    let mut listed = column![].spacing(9);
    for (icon, key, value) in rows {
        listed = listed.push(
            row![
                Canvas::new(Glyph { icon, alpha: k, colour: TEXT }).width(19.0).height(19.0),
                say(w.t(key), theme::SANS, 15.0, LABEL),
                ui::grow(),
                say(value, theme::SANS_SEMI, 15.0, TEXT),
            ]
            .spacing(11)
            .align_y(iced::Center),
        );
    }
    listed.into()
}

pub fn wide<'a, M: Clone + 'a>(seen: &Seen<'a>, card: &Card, open: fn(String) -> M) -> Element<'a, M> {
    let w = seen.words;
    let total = row![
        say(w.t("card-total-maps"), theme::SANS_SEMI, 15.0, RED),
        say(spaced(card.total_maps), theme::SANS_SEMI, 15.0, TEXT),
    ]
    .spacing(10)
    .align_y(iced::Center);
    let left = panel(
        column![
            section(w.t("card-grades"), 15.0),
            grades(card, false),
            total,
            divider(),
            section(w.t("card-top"), 15.0),
            posters(seen, card, 116.0, false, open),
        ]
        .spacing(14),
        16.0,
        Padding::from([18, 24]),
    )
    .width(Length::FillPortion(628))
    .height(Length::Fill);
    let history = Canvas::new(History {
        values: card.rank_history.clone(),
        marks: [w.t("card-90d"), w.t("card-60d"), w.t("card-30d"), w.t("card-now")],
        empty: w.t("card-no-data"),
        alpha: ui::fade(),
    })
    .width(Length::Fill)
    .height(150.0);
    let right = panel(
        column![
            section(w.t("card-stats"), 15.0),
            stats(seen, card),
            divider(),
            container(section(w.t("card-history"), 15.0)).width(Length::Fill).align_x(iced::alignment::Horizontal::Center),
            history,
        ]
        .spacing(13),
        16.0,
        Padding::from([18, 24]),
    )
    .width(Length::FillPortion(548))
    .height(Length::Fill);
    let inside = column![hero(seen, card, false), strip(seen, card, false), row![left, right].spacing(14)].spacing(14);
    container(inside).padding(14).width(Length::Fill).style(fill_style(CARD, CARD_LINE, 20.0)).into()
}

pub fn compact<'a, M: Clone + 'a>(seen: &Seen<'a>, card: &Card, open: fn(String) -> M) -> Element<'a, M> {
    column![
        hero(seen, card, true),
        strip(seen, card, true),
        panel(grades(card, true), 10.0, Padding::from([6, 10])),
        posters(seen, card, 46.0, true, open),
    ]
    .spacing(8)
    .into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Icon {
    Target,
    Keys,
    Chain,
    Play,
    Star,
    Clock,
    Heart,
}

struct Glyph {
    icon: Icon,
    alpha: f32,
    colour: Color,
}

fn line(colour: Color, width: f32) -> Stroke<'static> {
    Stroke::default().with_color(colour).with_width(width).with_line_cap(canvas::LineCap::Round).with_line_join(canvas::LineJoin::Round)
}

impl<Message> canvas::Program<Message> for Glyph {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let s = bounds.width.min(bounds.height) / 20.0;
        let at = |x: f32, y: f32| Point::new(x * s, y * s);
        let ink = Color { a: self.colour.a * self.alpha, ..self.colour };
        let pen = line(ink, 1.7 * s);
        match self.icon {
            Icon::Target => {
                frame.stroke(&Path::circle(at(10.0, 10.0), 8.0 * s), pen);
                frame.stroke(&Path::circle(at(10.0, 10.0), 4.5 * s), pen);
                frame.fill(&Path::circle(at(10.0, 10.0), 1.6 * s), ink);
                frame.stroke(&Path::line(at(10.0, 10.0), at(17.5, 2.5)), pen);
            }
            Icon::Keys => {
                frame.stroke(&Path::rounded_rectangle(at(1.5, 4.5), Size::new(17.0 * s, 11.0 * s), (2.5 * s).into()), pen);
                for (x, y) in [(5.0, 8.0), (8.3, 8.0), (11.7, 8.0), (15.0, 8.0)] {
                    frame.fill(&Path::circle(at(x, y), 1.0 * s), ink);
                }
                frame.stroke(&Path::line(at(6.0, 12.0), at(14.0, 12.0)), pen);
            }
            Icon::Chain => {
                frame.stroke(&Path::rounded_rectangle(at(2.0, 8.5), Size::new(9.0 * s, 5.5 * s), (2.75 * s).into()), pen);
                frame.stroke(&Path::rounded_rectangle(at(9.0, 6.0), Size::new(9.0 * s, 5.5 * s), (2.75 * s).into()), pen);
            }
            Icon::Play => {
                frame.stroke(&Path::circle(at(10.0, 10.0), 8.0 * s), pen);
                let triangle = Path::new(|b| {
                    b.move_to(at(8.0, 6.3));
                    b.line_to(at(14.0, 10.0));
                    b.line_to(at(8.0, 13.7));
                    b.close();
                });
                frame.fill(&triangle, ink);
            }
            Icon::Star => {
                let star = Path::new(|b| {
                    for point in 0..10 {
                        let reach = if point % 2 == 0 { 8.8 } else { 3.9 };
                        let angle = -std::f32::consts::FRAC_PI_2 + point as f32 * std::f32::consts::PI / 5.0;
                        let p = at(10.0 + reach * angle.cos(), 10.6 + reach * angle.sin());
                        if point == 0 {
                            b.move_to(p);
                        } else {
                            b.line_to(p);
                        }
                    }
                    b.close();
                });
                frame.fill(&star, ink);
            }
            Icon::Clock => {
                frame.stroke(&Path::circle(at(10.0, 10.0), 8.0 * s), pen);
                let hands = Path::new(|b| {
                    b.move_to(at(10.0, 5.2));
                    b.line_to(at(10.0, 10.0));
                    b.line_to(at(13.5, 12.2));
                });
                frame.stroke(&hands, pen);
            }
            Icon::Heart => {
                let heart = Path::new(|b| {
                    b.move_to(at(10.0, 17.5));
                    b.bezier_curve_to(at(3.0, 12.5), at(0.5, 8.5), at(3.0, 4.6));
                    b.bezier_curve_to(at(5.2, 1.6), at(9.0, 2.2), at(10.0, 5.6));
                    b.bezier_curve_to(at(11.0, 2.2), at(14.8, 1.6), at(17.0, 4.6));
                    b.bezier_curve_to(at(19.5, 8.5), at(17.0, 12.5), at(10.0, 17.5));
                    b.close();
                });
                frame.fill(&heart, ink);
            }
        }
        vec![frame.into_geometry()]
    }
}

struct History {
    values: Vec<f64>,
    marks: [String; 4],
    empty: String,
    alpha: f32,
}

impl<Message> canvas::Program<Message> for History {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let k = self.alpha;
        let fade = |colour: Color| Color { a: colour.a * k, ..colour };
        let label = |content: String, position: Point, align: iced::widget::text::Alignment| canvas::Text {
            content,
            position,
            color: fade(MUTED),
            size: 11.5.into(),
            font: theme::SANS,
            align_x: align,
            align_y: iced::alignment::Vertical::Center,
            ..canvas::Text::default()
        };
        let x0 = 62.0;
        let x1 = bounds.width - 2.0;
        let y0 = 8.0;
        let y1 = bounds.height - 24.0;
        if self.values.len() < 2 {
            frame.fill_text(label(self.empty.clone(), Point::new(bounds.width / 2.0, bounds.height / 2.0), iced::widget::text::Alignment::Center));
            return vec![frame.into_geometry()];
        }
        let lo = self.values.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = self.values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let spread = (hi - lo).max(1.0);
        let lo = lo - spread * 0.12;
        let spread = spread * 1.24;
        for step in 0..4 {
            let y = y0 + (y1 - y0) * step as f32 / 3.0;
            frame.stroke(&Path::line(Point::new(x0, y), Point::new(x1, y)), Stroke::default().with_color(fade(GRID)).with_width(1.0));
            let value = lo + spread * f64::from(step) / 3.0;
            frame.fill_text(label(format!("#{}", spaced(value)), Point::new(x0 - 8.0, y), iced::widget::text::Alignment::Right));
        }
        let step = (x1 - x0) / (self.values.len() - 1) as f32;
        let points: Vec<Point> = self
            .values
            .iter()
            .enumerate()
            .map(|(at, value)| Point::new(x0 + step * at as f32, y0 + ((value - lo) / spread) as f32 * (y1 - y0)))
            .collect();
        let area = Path::new(|b| {
            b.move_to(Point::new(points[0].x, y1));
            for point in &points {
                b.line_to(*point);
            }
            b.line_to(Point::new(points[points.len() - 1].x, y1));
            b.close();
        });
        frame.fill(&area, Color::from_rgba8(228, 72, 72, 0.22 * k));
        let curve = Path::new(|b| {
            b.move_to(points[0]);
            for pair in points.windows(2) {
                let middle = Point::new((pair[0].x + pair[1].x) / 2.0, (pair[0].y + pair[1].y) / 2.0);
                b.quadratic_curve_to(pair[0], middle);
            }
            b.line_to(points[points.len() - 1]);
        });
        frame.stroke(&curve, line(fade(LINE), 2.6));
        frame.fill(&Path::circle(points[points.len() - 1], 4.5), fade(DOT));
        let ground = y1 + 14.0;
        for (at, mark) in self.marks.iter().enumerate() {
            let x = x0 + (x1 - x0) * at as f32 / 3.0;
            let align = match at {
                0 => iced::widget::text::Alignment::Left,
                3 => iced::widget::text::Alignment::Right,
                _ => iced::widget::text::Alignment::Center,
            };
            frame.fill_text(label(mark.clone(), Point::new(x, ground), align));
        }
        vec![frame.into_geometry()]
    }
}

pub fn title_of(lang: Lang, catalog: &crate::community::Catalog, code: &str) -> Option<(String, Color)> {
    catalog.title_of(code).map(|title| (title.name(lang).to_owned(), title.rarity.colour()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_country_names_its_flag_the_way_osu_files_it() {
        assert_eq!(flag_url("ru").as_deref(), Some("https://osu.ppy.sh/assets/images/flags/1f1f7-1f1fa.svg"));
        assert_eq!(flag_url("XX").as_deref(), Some("https://osu.ppy.sh/assets/images/flags/1f1fd-1f1fd.svg"));
        assert_eq!(flag_url("—"), None);
    }

    #[test]
    fn numbers_and_dates_read_as_on_the_bots_card() {
        assert_eq!(spaced(15234.0), "15 234");
        assert_eq!(spaced(1_234_567_890.0), "1 234 567 890");
        assert_eq!(spaced(412.0), "412");
        assert_eq!(date("2018-05-12T00:00:00+00:00"), "12.05.2018");
        assert_eq!(date(""), "—");
        assert_eq!(grade_letter("XH"), "X");
        assert_eq!(grade_letter("SH"), "S");
    }
}
