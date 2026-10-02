use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::widget::{button, column, container, image, pin, row, scrollable, stack, text, Space};
use iced::{mouse, Background, Border, Color, Element, Length, Padding, Point, Radians, Rectangle, Renderer, Shadow, Theme};

use crate::community::{wire, Person, Title};
use crate::community_screen::{self as screen, Ground, Message, Scored};
use crate::glyphs::{glyph, Icon};
use crate::lang::Words;
use crate::theme::{self, ACCENT, FAINT, INK, MUTED};
use crate::ui;

pub(crate) const WIDE: f32 = 920.0;
const HEAD: f32 = 184.0;
const STRIP: f32 = 44.0;
const RING: f32 = 136.0;
const SIDE: f32 = 26.0;
const BOARD: f32 = 234.0;
const LINE: f32 = 42.0;
const GAP: f32 = 6.0;
const EDGE: f32 = 22.0;
const MARK: f32 = 72.0;
const TITLE_HIGH: f32 = 606.0;
const TITLE_LEFT: f32 = 300.0;
const GOLD: Color = theme::GRADE_S;
const GREEN: Color = theme::HIT_100;
const BLUE: Color = theme::HIT_300;
const YOURS: Color = Color::from_rgb(0.941, 0.439, 0.439);
const COVER_SEEN: f32 = 0.1;
const COVER_FADE: [(f32, f32); 6] = [(0.0, 0.0), (0.4, 0.0), (0.6, 0.3), (0.78, 0.7), (0.9, 0.93), (1.0, 1.0)];

fn tinted(colour: Color, a: f32) -> Color {
    Color { a: a * ui::fade(), ..colour }
}

fn blend(over: Color, a: f32) -> Color {
    let base = theme::SLAB_SOLID;
    Color { r: base.r + (over.r - base.r) * a, g: base.g + (over.g - base.g) * a, b: base.b + (over.b - base.b) * a, a: ui::fade() }
}

const TRACK: Color = Color::from_rgb(0.122, 0.09, 0.098);

fn well(k: f32, radius: f32) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.26 * k))),
        border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.06 * k), width: 1.0, radius: radius.into() },
        ..container::Style::default()
    }
}

fn tile<'a>(inside: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    let k = ui::fade();
    container(inside).padding([14, 16]).width(Length::Fill).height(Length::Fill).style(move |_| well(k, 14.0)).into()
}

fn caption<'a>(words: String) -> Element<'a, Message> {
    text(words).font(theme::SANS).size(12.0).wrapping(text::Wrapping::None).color(ui::faded(FAINT)).into()
}

fn mono<'a>(words: String, size: f32, colour: Color, strong: bool) -> Element<'a, Message> {
    text(words).font(if strong { theme::MONO_BOLD } else { theme::MONO }).size(size).wrapping(text::Wrapping::None).color(ui::faded(colour)).into()
}

fn filled<'a>(colour: Color, high: f32) -> Element<'a, Message> {
    let colour = tinted(colour, colour.a);
    container(Space::new().height(high))
        .width(Length::Fill)
        .style(move |_| container::Style { background: Some(Background::Color(colour)), border: Border { radius: (high / 2.0).into(), ..Border::default() }, ..container::Style::default() })
        .into()
}

fn bar<'a>(parts: &[(f32, Color)], high: f32) -> Element<'a, Message> {
    let k = ui::fade();
    let mut line = row![].spacing(2).height(high);
    let mut used = 0u16;
    for (share, colour) in parts {
        let wide = ((share.clamp(0.0, 1.0) * 1000.0).round() as u16).min(1000 - used.min(1000));
        if wide > 0 {
            line = line.push(container(filled(*colour, high)).width(Length::FillPortion(wide)));
            used += wide;
        }
    }
    if used < 1000 {
        line = line.push(Space::new().width(Length::FillPortion(1000 - used)));
    }
    container(line)
        .width(Length::Fill)
        .height(high)
        .style(move |_| container::Style { background: Some(Background::Color(Color { a: k, ..TRACK })), border: Border { radius: (high / 2.0).into(), ..Border::default() }, ..container::Style::default() })
        .into()
}

fn chip<'a>(words: String, colour: Color) -> Element<'a, Message> {
    let edge = tinted(colour, 0.55);
    container(mono(words, 11.0, colour, false))
        .padding([1, 7])
        .style(move |_| container::Style { border: Border { color: edge, width: 1.0, radius: 6.0.into() }, ..container::Style::default() })
        .into()
}

fn close<'a>() -> Element<'a, Message> {
    button(container(text("✕").font(theme::SANS_SEMI).size(theme::LEAD).color(ui::faded(MUTED))).width(36.0).height(36.0).center(36.0))
        .padding(0)
        .style(ui::button_faded(theme::bare))
        .on_press(Message::Unread)
        .into()
}

pub(crate) fn named(line: &str) -> (String, String, String) {
    let (head, version) = match line.rfind(" [") {
        Some(at) if line.ends_with(']') => (&line[..at], &line[at + 2..line.len() - 1]),
        _ => (line, ""),
    };
    let (artist, title) = head.split_once(" — ").unwrap_or(("", head));
    (artist.to_owned(), title.to_owned(), version.to_owned())
}

pub(crate) fn short(value: f64) -> String {
    let size = value.abs();
    if size >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if size >= 10_000.0 {
        format!("{:.0}k", value / 1_000.0)
    } else {
        format!("{value:.0}")
    }
}

pub(crate) fn signed(value: f64) -> String {
    match value {
        v if v >= 0.5 => format!("+{}", short(v)),
        v if v <= -0.5 => format!("−{}", short(-v)),
        _ => "0".to_owned(),
    }
}

pub(crate) fn board_high(rows: usize) -> f32 {
    (rows as f32 * (LINE + GAP) - GAP).clamp(3.0 * (LINE + GAP) - GAP, BOARD)
}

fn edged<'a>(rolled: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    crate::glide::brim(rolled).on(theme::SLAB_SOLID).both().high(EDGE).into()
}

fn length(seconds: u32) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn worth(play: &wire::Play, by_score: bool) -> f64 {
    match (by_score, play.pp > 0.0) {
        (true, _) => play.score as f64,
        (false, true) => f64::from(play.pp),
        (false, false) => f64::from(play.pp_if.unwrap_or(0.0)),
    }
}

fn worth_said(w: &Words, play: &wire::Play, by_score: bool) -> String {
    match (by_score, play.pp > 0.0, play.pp_if) {
        (true, _, _) => w.lang().group(play.score),
        (false, true, _) => format!("{} pp", screen::decimal(w, play.pp, 0)),
        (false, false, Some(guess)) => format!("≈{} pp", screen::decimal(w, guess, 0)),
        (false, false, None) => "—".to_owned(),
    }
}

fn initial(name: &str) -> String {
    name.chars().next().map(|first| first.to_uppercase().to_string()).unwrap_or_default()
}

fn row_face<'a>(ground: &Ground<'a>, said: &wire::BoardRow, side: f32) -> Element<'a, Message> {
    match ground.catalog.people.iter().find(|person| person.id == said.who) {
        Some(person) => screen::face(ground, person, side),
        None => screen::round(ground.pictures.get(&said.avatar), &initial(&said.name), side, ground.picture_shown(&said.avatar)),
    }
}

fn line_style(edge: Color, fill: Option<Color>, wide: f32, radius: f32) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let wash = fill.or(lit.then_some(Color::from_rgba(1.0, 1.0, 1.0, 0.04)));
        button::Style { background: wash.map(Background::Color), text_color: INK, border: Border { color: edge, width: wide, radius: radius.into() }, shadow: Shadow::default(), snap: true }
    }
}

fn framed(chosen: bool, you: bool) -> (Color, Option<Color>, f32) {
    match (chosen, you) {
        (true, _) => (Color::from_rgb(0.541, 0.455, 0.188), Some(Color::from_rgb(0.09, 0.071, 0.039)), 1.0),
        (false, true) => (Color::from_rgb(0.561, 0.196, 0.204), None, 1.0),
        (false, false) => (Color::from_rgba(1.0, 1.0, 1.0, 0.07), None, 1.0),
    }
}

fn table_line<'a>(ground: &Ground<'a>, said: &'a wire::BoardRow, by_score: bool, chosen: bool) -> Element<'a, Message> {
    let w = ground.words;
    let known = ground.catalog.people.iter().position(|person| person.id == said.who);
    let place = mono(format!("#{}", said.place), 12.0, screen::medal(said.place as usize).unwrap_or(FAINT), true);
    let mut name = row![text(said.name.clone()).font(theme::SANS_SEMI).size(14.0).wrapping(text::Wrapping::None).color(ui::faded(INK))].spacing(8).align_y(iced::Center);
    if said.you {
        name = name.push(chip(w.t("score-you"), YOURS));
    }
    let when = said.play.at.map_or(String::new(), |at| w.day(at, ground.now_unix));
    let combo = said.play.combo.map_or(String::new(), |combo| format!("{}x", w.lang().group(u64::from(combo))));
    let line = row![
        container(place).width(30.0),
        row_face(ground, said, 24.0),
        container(name).width(Length::Fill).clip(true),
        container(text(when).font(theme::SANS).size(12.0).wrapping(text::Wrapping::None).color(ui::faded(if chosen { GOLD } else { FAINT }))).width(110.0),
        container(mono(w.percent(f64::from(said.play.accuracy)), 12.0, MUTED, false)).width(64.0).align_x(iced::alignment::Horizontal::Right),
        container(mono(combo, 12.0, MUTED, false)).width(64.0).align_x(iced::alignment::Horizontal::Right),
        container(mono(worth_said(w, &said.play, by_score), 13.0, INK, true)).width(92.0).align_x(iced::alignment::Horizontal::Right),
    ]
    .spacing(12)
    .align_y(iced::Center);
    let (edge, fill, wide) = framed(chosen, said.you);
    let pressed = button(container(line).height(LINE).center_y(LINE).padding([0, 14])).padding(0).width(Length::Fill).style(ui::button_faded(ui::calm(line_style(edge, fill, wide, 10.0))));
    match known {
        Some(at) => pressed.on_press(Message::Person(Some(at))).into(),
        None => pressed.into(),
    }
}

const COMPANION_ROWS: usize = 3;

fn companion_line<'a>(ground: &Ground<'a>, said: &'a wire::BoardRow, by_score: bool) -> Element<'a, Message> {
    let w = ground.words;
    let mut name = row![text(said.name.clone()).font(theme::SANS_SEMI).size(13.0).wrapping(text::Wrapping::None).color(ui::faded(INK))].spacing(6).align_y(iced::Center);
    if said.you {
        name = name.push(chip(w.t("score-you"), YOURS));
    }
    row![
        container(mono(format!("#{}", said.place), 12.0, screen::medal(said.place as usize).unwrap_or(FAINT), true)).width(30.0),
        row_face(ground, said, 22.0),
        container(name).width(Length::Fill).clip(true),
        mono(worth_said(w, &said.play, by_score), 13.0, INK, true),
    ]
    .spacing(10)
    .align_y(iced::Center)
    .into()
}

pub(crate) fn companion<'a>(ground: &Ground<'a>, beatmap: u64, line: String, width: f32) -> Element<'a, Message> {
    let w = ground.words;
    let note = |words: String, colour: Color| -> Element<'a, Message> { text(words).font(theme::SANS).size(12.0).color(ui::faded(colour)).into() };
    let board = ground.boards.get(&beatmap);
    let mut head = row![mono(w.t("score-board").to_uppercase(), 11.0, FAINT, false), ui::grow()].align_y(iced::Center);
    if let Some(board) = board.filter(|board| board.players > 0) {
        head = head.push(mono(w.count("players", u64::from(board.players)), 11.0, FAINT, false));
    }
    let map = container(ui::marquee(vec![ui::piece(line, theme::SANS_SEMI, 13.0, INK)])).width(Length::Fill).clip(true);
    let body: Element<'a, Message> = match board {
        Some(board) if board.rows.is_empty() => note(w.t("score-board-empty"), FAINT),
        Some(board) => {
            let by_score = board.by_score();
            let mut lines = column![].spacing(6);
            for said in board.rows.iter().take(COMPANION_ROWS) {
                lines = lines.push(companion_line(ground, said, by_score));
            }
            if let Some(own) = board.rows.iter().skip(COMPANION_ROWS).find(|said| said.you) {
                lines = lines.push(companion_line(ground, own, by_score));
            }
            lines.into()
        }
        None if ground.boards_failed.contains(&beatmap) => note(w.t("score-board-failed"), ACCENT),
        None => note(w.t("score-board-loading"), FAINT),
    };
    container(column![head, map, body].spacing(8)).padding([12, 14]).width(width).style(ui::box_faded(theme::notification(false))).into()
}

fn table<'a>(ground: &Ground<'a>, board: &'a wire::MapBoard, scored: &Scored) -> Element<'a, Message> {
    let mut lines = column![].spacing(GAP);
    for said in &board.rows {
        lines = lines.push(table_line(ground, said, board.by_score(), said.who == scored.who));
    }
    edged(scrollable(lines).direction(ui::hidden_bar()).height(board_high(board.rows.len())).width(Length::Fill))
}

fn placed_at<'a>(share: f32, inside: Element<'a, Message>) -> Element<'a, Message> {
    let before = (share.clamp(0.0, 1.0) * 10_000.0).round() as u16;
    let mut line = row![];
    if before > 0 {
        line = line.push(Space::new().width(Length::FillPortion(before)));
    }
    line = line.push(inside);
    if before < 10_000 {
        line = line.push(Space::new().width(Length::FillPortion(10_000 - before)));
    }
    line.width(Length::Fill).into()
}

fn scale<'a>(ground: &Ground<'a>, board: &'a wire::MapBoard, scored: &Scored) -> Element<'a, Message> {
    let w = ground.words;
    let k = ui::fade();
    let by_score = board.by_score();
    let values: Vec<f64> = board.rows.iter().map(|said| worth(&said.play, by_score)).collect();
    let (low, high) = values.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), value| (low.min(*value), high.max(*value)));
    let share = |value: f64| if high > low { ((value - low) / (high - low)) as f32 } else { 0.5 };
    let own = board.rows.iter().position(|said| said.you);
    let hero = board.rows.iter().position(|said| said.who == scored.who);
    let base = own.or(hero).map(|at| values[at]);
    let crowded = board.rows.len() > 8;
    let ends = |at: usize| values[at] == high || values[at] == low;

    let mut order: Vec<usize> = (0..board.rows.len()).collect();
    order.sort_by_key(|at| (Some(*at) == hero, Some(*at) == own, ends(*at)));
    let inset = Padding { top: 44.0, right: MARK / 2.0, bottom: 0.0, left: MARK / 2.0 };
    let line = container(Space::new().height(4.0)).width(Length::Fill).style(move |_| container::Style {
        background: Some(Background::Color(Color { a: k, ..Color::from_rgb(0.165, 0.125, 0.133) })),
        border: Border { radius: 2.0.into(), ..Border::default() },
        ..container::Style::default()
    });
    let mut layers: Vec<Element<'a, Message>> = vec![container(line).padding(inset).width(Length::Fill).height(Length::Fill).into()];
    if let (Some(own), Some(hero)) = (own, hero) {
        let (from, to) = (share(values[own]).min(share(values[hero])), share(values[own]).max(share(values[hero])));
        if to - from > 0.002 {
            let lit = container(Space::new().height(4.0)).width(Length::FillPortion(((to - from) * 10_000.0).round().max(1.0) as u16)).style(move |_| container::Style {
                background: Some(Background::Color(Color { a: k, ..Color::from_rgb(0.541, 0.455, 0.188) })),
                ..container::Style::default()
            });
            let mut span = row![];
            if from > 0.0005 {
                span = span.push(Space::new().width(Length::FillPortion((from * 10_000.0).round().max(1.0) as u16)));
            }
            span = span.push(lit);
            if to < 0.9995 {
                span = span.push(Space::new().width(Length::FillPortion(((1.0 - to) * 10_000.0).round().max(1.0) as u16)));
            }
            layers.push(container(span).padding(inset).into());
        }
    }
    for at in order {
        let said = &board.rows[at];
        let (is_hero, is_own) = (Some(at) == hero, Some(at) == own);
        let told = !crowded || is_hero || is_own || ends(at);
        let side = if is_hero { 44.0 } else if told { 28.0 } else { 20.0 };
        let colour = if is_hero { GOLD } else if is_own { YOURS } else { MUTED };
        let above: Element<'a, Message> = match (told, is_own, base) {
            (false, _, _) => Space::new().into(),
            (true, true, _) => mono(w.t("score-you"), 12.0, colour, true),
            (true, false, Some(base)) if own.is_some() || !is_hero => mono(signed(values[at] - base), if is_hero { 15.0 } else { 12.0 }, colour, is_hero),
            _ => Space::new().into(),
        };
        let ring = if is_hero || is_own { tinted(colour, 1.0) } else { Color::TRANSPARENT };
        let band = if is_hero { 3.0 } else { 2.0 };
        let face = container(row_face(ground, said, side - band * 2.0))
            .width(side)
            .height(side)
            .center(side)
            .style(move |_| container::Style { border: Border { color: ring, width: band, radius: (side / 2.0).into() }, ..container::Style::default() });
        let below: Element<'a, Message> = match told {
            true => mono(if by_score { short(values[at]) } else { format!("{:.0}", values[at]) }, if is_hero { 13.0 } else { 12.0 }, if is_hero || is_own { colour } else { FAINT }, is_hero),
            false => Space::new().into(),
        };
        let name: Element<'a, Message> = match told && (is_hero || ends(at)) && !is_own {
            true => text(said.name.clone()).font(theme::SANS).size(12.0).wrapping(text::Wrapping::None).color(ui::faded(if is_hero { INK } else { FAINT })).into(),
            false => Space::new().into(),
        };
        let mark = column![
            container(above).height(20.0).align_y(iced::Bottom),
            container(face).height(44.0).center_y(44.0),
            container(below).height(16.0),
            container(name).height(16.0),
        ]
        .spacing(4)
        .width(MARK)
        .align_x(iced::Center);
        layers.push(placed_at(share(values[at]), mark.into()));
    }
    container(container(iced::widget::Stack::with_children(layers).width(Length::Fill).height(108.0)).center_y(Length::Fill))
        .height(board_high(board.rows.len()))
        .width(Length::Fill)
        .padding([0, 20])
        .style(move |_| well(k, 14.0))
        .into()
}

fn switch<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let k = ui::fade();
    let side = |label: String, on: bool, to: bool| {
        button(container(text(label).font(theme::SANS_SEMI).size(13.0).wrapping(text::Wrapping::None)).center_y(32.0))
            .padding([0, 16])
            .style(ui::button_faded(move |_: &Theme, status: button::Status| {
                let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
                button::Style {
                    background: match (on, lit) {
                        (true, _) => Some(Background::Color(Color::from_rgb(0.231, 0.09, 0.102))),
                        (false, true) => Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05))),
                        (false, false) => None,
                    },
                    text_color: if on { INK } else { MUTED },
                    border: Border { radius: 8.0.into(), ..Border::default() },
                    shadow: Shadow::default(),
                    snap: true,
                }
            }))
            .on_press(Message::ScoreScale(to))
    };
    container(row![side(w.t("score-table"), !ground.score_scale, false), side(w.t("score-scale"), ground.score_scale, true)].spacing(2))
        .padding(3)
        .style(move |_| well(k, 10.0))
        .into()
}

fn cover_back<'a>(ground: &Ground<'a>, scored: &Scored) -> Element<'a, Message> {
    let k = ui::fade();
    let top = iced::border::Radius { top_left: 15.0, top_right: 15.0, bottom_right: 0.0, bottom_left: 0.0 };
    let under = container(Space::new().width(Length::Fill).height(HEAD)).style(move |_| container::Style {
        background: Some(Background::Color(Color { a: k, ..Color::from_rgb(0.106, 0.067, 0.078) })),
        border: Border { radius: top, ..Border::default() },
        ..container::Style::default()
    });
    let into_body = COVER_FADE.iter().fold(iced::gradient::Linear::new(Radians(std::f32::consts::PI)), |shade, (at, a)| shade.add_stop(*at, Color { a: a * k, ..theme::SLAB_SOLID }));
    let fade = container(Space::new().width(Length::Fill).height(HEAD)).style(move |_| container::Style { background: Some(Background::Gradient(into_body.into())), ..container::Style::default() });
    match scored.map.cover().and_then(|url| ground.pictures.get(&screen::wide(&url))) {
        Some(handle) => {
            let shown = scored.map.cover().map_or(1.0, |url| ground.picture_shown(&screen::wide(&url)));
            stack![under, image(handle.clone()).content_fit(iced::ContentFit::Cover).width(Length::Fill).height(HEAD).border_radius(15.0).opacity(COVER_SEEN * k * shown), fade].into()
        }
        None => under.into(),
    }
}

pub(crate) fn score<'a>(ground: &Ground<'a>, scored: &'a Scored) -> Element<'a, Message> {
    let w = ground.words;
    let k = ui::fade();
    let play = &scored.play;
    let known = ground.catalog.people.iter().position(|person| person.id == scored.who);
    let board = scored.map.beatmap.and_then(|beatmap| ground.boards.get(&beatmap).map(|board| (beatmap, board)));
    let grade = if scored.passed { play.grade.clone() } else { "F".to_owned() };
    let shade = screen::grade_colour(&grade);
    let (artist, title, version) = named(&scored.map.line);

    let mut name = row![text(title).font(theme::SANS_SEMI).size(30.0).wrapping(text::Wrapping::None).color(ui::faded(INK))].spacing(12).align_y(iced::Center);
    if !version.is_empty() {
        name = name.push(chip(version, INK));
    }
    let mut titles = column![container(name).clip(true)].spacing(6).width(Length::Fill);
    if !artist.is_empty() {
        titles = titles.push(text(artist).font(theme::SANS).size(15.0).wrapping(text::Wrapping::None).color(ui::faded(MUTED)));
    }
    let pp = match (play.pp > 0.0, play.pp_if) {
        (true, _) => format!("{} pp", screen::decimal(w, play.pp, 0)),
        (false, Some(guess)) => format!("≈{} pp", screen::decimal(w, guess, 0)),
        (false, None) => "—".to_owned(),
    };
    let mut facts = row![].spacing(14);
    if let Some(stars) = scored.map.stars.or(play.stars) {
        facts = facts.push(mono(format!("{}★", screen::decimal(w, stars, 2)), 13.0, MUTED, false));
    }
    if let Some(about) = board.and_then(|(_, board)| board.map.as_ref()) {
        if let Some(bpm) = about.bpm {
            facts = facts.push(mono(format!("{bpm:.0} BPM"), 13.0, MUTED, false));
        }
        if let Some(seconds) = about.length {
            facts = facts.push(mono(length(seconds), 13.0, MUTED, false));
        }
    }
    let right = column![mono(pp, 36.0, INK, true), facts].spacing(6).align_x(iced::Right);
    let when = if play.at > 0 { format!("{} {}", w.day(play.at, ground.now_unix), w.clock(play.at)) } else { String::new() };
    let head = stack![
        cover_back(ground, scored),
        container(column![mono(when, 12.0, MUTED, false), mono(if play.witnessed { w.t("score-witnessed") } else { String::new() }, 11.0, FAINT, false)].spacing(4).align_x(iced::Center))
            .center_x(Length::Fill)
            .padding(Padding::ZERO.top(20.0)),
        container(row![ui::grow(), close()]).padding(Padding { top: 8.0, right: 10.0, bottom: 0.0, left: 0.0 }),
        container(row![Space::new().width(RING + 4.0), titles, right].spacing(18).align_y(iced::Bottom))
            .height(HEAD)
            .align_y(iced::Bottom)
            .padding(Padding { top: 0.0, right: SIDE, bottom: 16.0, left: SIDE }),
    ]
    .height(HEAD);

    let face: Element<'a, Message> = match known {
        Some(at) => screen::face(ground, &ground.catalog.people[at], 28.0),
        None => screen::round(None, &initial(&scored.name), 28.0, 1.0),
    };
    let who = button(row![face, text(scored.name.clone()).font(theme::SANS_SEMI).size(16.0).wrapping(text::Wrapping::None).color(ui::faded(INK))].spacing(10).align_y(iced::Center))
        .padding([1, 2])
        .style(ui::button_faded(theme::bare));
    let who = match known {
        Some(at) => who.on_press(Message::Person(Some(at))),
        None => who,
    };
    let points: Element<'a, Message> = match play.score > 0 {
        true => mono(w.lang().group(play.score), 20.0, INK, true),
        false => Space::new().into(),
    };
    let strip = container(row![Space::new().width(RING + 12.0), who, screen::mods(&play.mods), container(points).center_x(Length::Fill)].spacing(10).align_y(iced::Center))
        .height(STRIP)
        .align_y(iced::Bottom)
        .padding([0.0, SIDE]);
    let rim = tinted(shade, 1.0);
    let ring = container(text(grade).font(theme::MONO_BOLD).size(68.0).color(ui::faded(shade))).width(RING).height(RING).center(RING).style(move |_| container::Style {
        background: Some(Background::Color(Color { a: k, ..theme::SLAB_SOLID })),
        border: Border { color: rim, width: 5.0, radius: (RING / 2.0).into() },
        ..container::Style::default()
    });
    let top = stack![column![head, strip], pin(ring).x(SIDE).y(HEAD + STRIP - RING)].height(HEAD + STRIP);

    let accuracy = tile(column![caption(w.t("score-accuracy")), mono(w.percent(f64::from(play.accuracy)), 24.0, INK, true), ui::grow_tall(), bar(&[(play.accuracy / 100.0, shade)], 8.0)].spacing(10));
    let mut combo_line = row![].spacing(8).align_y(iced::Bottom);
    let most = play.max_combo.or(board.and_then(|(_, board)| board.map.as_ref()).and_then(|about| about.max_combo)).filter(|most| *most > 0);
    let reached = match (play.combo, most) {
        (Some(combo), Some(most)) => Some(combo as f32 / most as f32),
        (_, None) if play.full_combo => Some(1.0),
        _ => None,
    };
    match (play.combo, most) {
        (Some(combo), Some(most)) => {
            combo_line = combo_line
                .push(mono(format!("{}x", w.lang().group(u64::from(combo))), 24.0, INK, true))
                .push(container(mono(format!("/ {}x", w.lang().group(u64::from(most))), 15.0, FAINT, false)).padding(Padding::ZERO.bottom(3.0)));
        }
        (Some(combo), None) => combo_line = combo_line.push(mono(format!("{}x", w.lang().group(u64::from(combo))), 24.0, INK, true)),
        _ => combo_line = combo_line.push(mono("—".to_owned(), 24.0, FAINT, true)),
    }
    let mut combo = column![caption(w.t("score-combo")), combo_line, ui::grow_tall()].spacing(10);
    if let Some(share) = reached {
        combo = combo.push(bar(&[(share, GREEN)], 8.0));
    }
    let combo = tile(combo);
    let marks = [(play.counts[0], BLUE, "300".to_owned()), (play.counts[1], GREEN, "100".to_owned()), (play.counts[2], GOLD, "50".to_owned()), (play.counts[3], YOURS, w.n("score-miss", u64::from(play.counts[3].unwrap_or(0))))];
    let total: u32 = marks.iter().filter_map(|(count, _, _)| *count).sum();
    let mut counts = row![].align_y(iced::Bottom);
    let mut shares: Vec<(f32, Color)> = Vec::new();
    let mut shown = 0;
    for (count, colour, label) in marks {
        let Some(count) = count else {
            continue;
        };
        if shown > 0 {
            counts = counts.push(ui::grow());
        }
        shown += 1;
        counts = counts.push(
            row![mono(w.lang().group(u64::from(count)), 18.0, INK, true), container(text(label).font(theme::SANS).size(12.0).wrapping(text::Wrapping::None).color(ui::faded(colour))).padding(Padding::ZERO.bottom(2.0))]
                .spacing(6)
                .align_y(iced::Bottom),
        );
        if count > 0 && total > 0 {
            shares.push(((count as f32 / total as f32).max(0.012), colour));
        }
    }
    let hits = tile(column![caption(w.t("score-hits")), counts, ui::grow_tall(), bar(&shares, 8.0)].spacing(10));
    let tiles = row![container(accuracy).width(Length::FillPortion(2)), container(combo).width(Length::FillPortion(2)), container(hits).width(Length::FillPortion(3))].spacing(10).height(118.0);

    let mut body = column![tiles].spacing(16);
    let note = |words: String, colour: Color| -> Element<'a, Message> {
        container(text(words).font(theme::SANS).size(theme::BODY).color(ui::faded(colour))).center(Length::Fill).height(board_high(0)).width(Length::Fill).style(move |_| well(k, 14.0)).into()
    };
    let label = || mono(w.t("score-board").to_uppercase(), 11.0, FAINT, false);
    match (scored.map.beatmap, board) {
        (_, Some((beatmap, board))) => {
            let mut over = row![label()].spacing(12).align_y(iced::Center);
            if ground.boards_waiting.contains(&beatmap) {
                over = over.push(caption(w.t("score-board-refreshing")));
            }
            body = body.push(over.push(ui::grow()).push(switch(ground)));
            body = body.push(ui::smooth(match (board.rows.is_empty(), ground.score_scale) {
                (true, _) => note(w.t("score-board-empty"), FAINT),
                (false, true) => scale(ground, board, scored),
                (false, false) => table(ground, board, scored),
            }));
        }
        (Some(beatmap), None) if ground.boards_failed.contains(&beatmap) => body = body.push(label()).push(ui::smooth(note(w.t("score-board-failed"), ACCENT))),
        (Some(_), None) if !ground.catalog.staged => body = body.push(label()).push(ui::smooth(note(w.t("score-board-loading"), FAINT))),
        _ => {}
    }
    let mut deeds = row![].spacing(8);
    if let Some(beatmap) = scored.map.beatmap {
        deeds = deeds.push(ui::primary(w.t("score-open-map"), Some(Message::Open(format!("https://osu.ppy.sh/b/{beatmap}")))));
    }
    if let Some(at) = known {
        deeds = deeds.push(ui::quiet(w.t("score-open-player"), Some(Message::Person(Some(at)))));
    }
    body = body.push(deeds);
    let rolled = scrollable(container(body).padding(Padding { top: 24.0, right: SIDE, bottom: 22.0, left: SIDE })).direction(ui::hidden_bar()).height(Length::Shrink);
    container(column![top, rolled]).padding(1).width(Length::Fill).into()
}

struct Dial {
    share: f32,
    colour: Color,
    track: Color,
    disc: Color,
    band: f32,
}

impl<M> canvas::Program<M> for Dial {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let centre = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let outer = bounds.width.min(bounds.height) / 2.0;
        let radius = outer - self.band / 2.0;
        frame.fill(&Path::circle(centre, outer), self.disc);
        let stroke = |colour: Color| Stroke::default().with_color(colour).with_width(self.band).with_line_cap(canvas::LineCap::Round);
        frame.stroke(&Path::circle(centre, radius), stroke(self.track));
        let share = self.share.clamp(0.0, 1.0);
        if share > 0.002 {
            let start = -std::f32::consts::FRAC_PI_2;
            let arc = Path::new(|path| path.arc(canvas::path::Arc { center: centre, radius, start_angle: Radians(start), end_angle: Radians(start + std::f32::consts::TAU * share) }));
            frame.stroke(&arc, stroke(self.colour));
        }
        vec![frame.into_geometry()]
    }
}

pub(crate) fn rarity_place(catalog: &crate::community::Catalog, code: &str) -> (usize, usize, usize) {
    let held = |title: &Title| catalog.held.get(&title.code).copied().unwrap_or(0).max(catalog.holders(&title.code).len() as u32);
    let total = catalog.titles.len();
    let Some(mine) = catalog.title_of(code).map(held) else {
        return (0, 0, total);
    };
    let rarer = catalog.titles.iter().filter(|title| held(title) < mine).count();
    let same = catalog.titles.iter().filter(|title| held(title) == mine).count();
    (rarer + 1, rarer + same, total)
}

fn holder_card<'a>(ground: &Ground<'a>, at: usize, person: &'a Person, earned: Option<i64>, first: Option<i64>, chosen: bool) -> Element<'a, Message> {
    let w = ground.words;
    let mut name = row![text(person.name.clone()).font(theme::SANS_SEMI).size(15.0).wrapping(text::Wrapping::None).color(ui::faded(INK))].spacing(6).align_y(iced::Center);
    if person.you {
        name = name.push(chip(w.t("score-you"), YOURS));
    }
    let second: Element<'a, Message> = match (earned, first) {
        (Some(mine), Some(first)) if mine == first => chip(w.t("title-first"), GOLD),
        (Some(mine), Some(first)) => caption(w.with("title-later", &[("days", w.n("days-long", ((mine - first).max(0) as u64).div_ceil(86_400).max(1)))])),
        _ => Space::new().height(15.0).into(),
    };
    let when = earned.map_or(String::new(), |at| w.day(at, ground.now_unix));
    let inside = column![screen::face(ground, person, 52.0), container(name).clip(true), second, mono(when, 12.0, MUTED, false)].spacing(6).align_x(iced::Center).width(Length::Fill);
    let (edge, fill, wide) = match chosen {
        true => (GOLD, Some(Color::from_rgb(0.09, 0.071, 0.039)), 2.0),
        false => framed(false, person.you),
    };
    button(container(inside).padding([14, 10]))
        .padding(0)
        .width(Length::Fill)
        .style(ui::button_faded(ui::calm(line_style(edge, fill, wide, 14.0))))
        .on_press(Message::Person(Some(at)))
        .into()
}

fn waiting_card<'a>(ground: &Ground<'a>, you: &'a Person, left: Option<u32>) -> Element<'a, Message> {
    let w = ground.words;
    let k = ui::fade();
    let name = row![text(you.name.clone()).font(theme::SANS_SEMI).size(15.0).wrapping(text::Wrapping::None).color(ui::faded(INK)), chip(w.t("score-you"), YOURS)].spacing(6).align_y(iced::Center);
    let left = left.map_or(String::new(), |left| w.with("title-left", &[("n", w.lang().group(u64::from(left)))]));
    let inside = column![ui::fading(k * 0.45, || screen::face(ground, you, 52.0)), container(name).clip(true), caption(w.t("title-not-yet")), mono(left, 12.0, MUTED, false)].spacing(6).align_x(iced::Center).width(Length::Fill);
    container(inside)
        .padding([14, 10])
        .width(Length::Fill)
        .style(move |_| container::Style { border: Border { color: Color::from_rgba(0.353, 0.137, 0.149, k), width: 1.0, radius: 14.0.into() }, ..container::Style::default() })
        .into()
}

pub(crate) fn title<'a>(ground: &Ground<'a>, code: &str, who: Option<i64>) -> Element<'a, Message> {
    let w = ground.words;
    let k = ui::fade();
    let catalog = ground.catalog;
    let Some(title) = catalog.title_of(code) else {
        return container(row![caption(code.to_owned()), ui::grow(), close()]).padding(20).into();
    };
    let holders = catalog.holders(code);
    let tint = title.rarity.colour();
    let earned = |person: &Person| -> Option<i64> {
        person.title_dates.get(code).copied().or_else(|| catalog.me.as_ref().filter(|me| me.person.id == person.id).and_then(|me| me.title_dates.get(code).copied()))
    };
    let mut ordered = holders.clone();
    ordered.sort_by_key(|at| (earned(&catalog.people[*at]).unwrap_or(i64::MAX), *at));
    let first = ordered.iter().filter_map(|at| earned(&catalog.people[*at])).min();
    let server = catalog.held.get(code).copied().unwrap_or(0).max(holders.len() as u32);
    let everyone = catalog.players.max(catalog.people.len() as u32).max(1);
    let share = server as f32 / everyone as f32;
    let (first_place, last_place, of) = rarity_place(catalog, code);

    let emblem = stack![
        Canvas::new(Dial { share, colour: tinted(tint, 1.0), track: blend(tint, 0.3), disc: Color { a: k, ..theme::SLAB_SOLID }, band: 6.0 }).width(RING).height(RING),
        container(glyph(Icon::Trophy, 48.0, tint)).width(RING).height(RING).center(RING),
    ];
    let mut ticks = row![].spacing(3);
    for at in 1..=of {
        let shade = blend(tint, if (first_place..=last_place).contains(&at) { 1.0 } else if at < first_place { 0.34 } else { 0.14 });
        ticks = ticks.push(container(Space::new().height(8.0)).width(Length::FillPortion(1)).style(move |_| container::Style { background: Some(Background::Color(shade)), border: Border { radius: 2.0.into(), ..Border::default() }, ..container::Style::default() }));
    }
    let left = column![
        emblem,
        container(mono(w.t(title.rarity.key()).to_uppercase(), 11.0, tint, false)).padding(Padding::ZERO.top(6.0)),
        text(title.name(w.lang()).to_owned()).font(theme::SANS_SEMI).size(26.0).color(ui::faded(INK)).align_x(iced::Center),
        text(title.about(w.lang()).to_owned()).font(theme::SANS).size(14.0).color(ui::faded(MUTED)).align_x(iced::Center),
        ui::grow_tall(),
        column![mono(format!("{:.0}%", share * 100.0), 22.0, tint, true), text(w.t("title-share-said")).font(theme::SANS).size(12.0).color(ui::faded(MUTED))].spacing(4).width(Length::Fill),
        column![
            caption(w.with("title-rarity-among", &[("n", of.to_string())])),
            ticks,
            row![text(w.t("title-rare-end")).font(theme::SANS).size(11.0).color(ui::faded(FAINT)), ui::grow(), text(w.t("title-common-end")).font(theme::SANS).size(11.0).color(ui::faded(FAINT))],
        ]
        .spacing(6)
        .width(Length::Fill),
    ]
    .spacing(10)
    .align_x(iced::Center);
    let (wash, edge) = (blend(tint, 0.09), blend(tint, 0.3));
    let left = container(left).width(TITLE_LEFT).height(TITLE_HIGH).padding(Padding { top: 30.0, right: 24.0, bottom: 24.0, left: 24.0 }).style(move |_| container::Style {
        background: Some(Background::Color(wash)),
        border: Border { color: edge, width: 1.0, radius: iced::border::Radius { top_left: 15.0, top_right: 0.0, bottom_right: 0.0, bottom_left: 15.0 } },
        ..container::Style::default()
    });

    let you = catalog.people.iter().find(|person| person.you);
    let yours = you.is_some_and(|you| you.titles.iter().any(|held| held == code));
    let progress = catalog.me.as_ref().and_then(|me| me.title_progress.get(code).copied());
    let mut cards: Vec<Element<'a, Message>> = ordered.iter().map(|at| holder_card(ground, *at, &catalog.people[*at], earned(&catalog.people[*at]), first, who == Some(catalog.people[*at].id))).collect();
    if let (Some(you), false) = (you, yours) {
        cards.push(waiting_card(ground, you, progress.filter(|_| title.target > 1).map(|value| title.target.saturating_sub(value))));
    }
    let waiting: Vec<&Person> = catalog.people.iter().filter(|person| !person.you && !person.titles.iter().any(|held| held == code)).collect();
    let mut list = column![screen::grid(cards, 3, 10.0)].spacing(14);
    if !waiting.is_empty() {
        let mut faces = row![].spacing(4).align_y(iced::Center);
        for person in waiting.iter().take(5) {
            faces = faces.push(screen::face(ground, person, 26.0));
        }
        list = list.push(
            container(row![faces, text(w.count("players", waiting.len() as u64)).font(theme::SANS_SEMI).size(13.0).color(ui::faded(INK)), caption(w.t("title-others-wait"))].spacing(12).align_y(iced::Center))
                .padding([10, 14])
                .width(Length::Fill)
                .style(move |_| container::Style { border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.07 * k), width: 1.0, radius: 12.0.into() }, ..container::Style::default() }),
        );
    }
    let mut right = column![
        row![mono(w.t("title-holders").to_uppercase(), 11.0, FAINT, false), ui::grow(), close()].align_y(iced::Center),
        edged(scrollable(list).direction(ui::hidden_bar()).height(Length::Fill)),
    ]
    .spacing(10);
    match (yours, you, progress) {
        (true, Some(you), _) => {
            let since = earned(you).map_or(w.t("title-yours"), |at| w.with("title-yours-since", &[("day", w.day(at, ground.now_unix))]));
            let worn = catalog.shown_title(you).map_or(w.t("title-none-worn"), |worn| w.with("title-worn-now", &[("title", worn.name(w.lang()).to_owned())]));
            right = right.push(
                container(row![column![text(since).font(theme::SANS_SEMI).size(14.0).color(ui::faded(INK)), caption(worn)].spacing(3).width(Length::Fill), crate::dossier::wear_button_anywhere(ground, you, title)].spacing(14).align_y(iced::Center))
                    .padding([14, 16])
                    .style(move |_| container::Style { background: Some(Background::Color(wash)), border: Border { color: edge, width: 1.0, radius: 14.0.into() }, ..container::Style::default() }),
            );
        }
        (false, Some(_), Some(value)) if title.target > 1 => {
            let done = (value.min(title.target) as f32 / title.target as f32) * 4.0;
            let mut steps = row![].spacing(4);
            let mut marks = row![].spacing(4);
            for at in 0..4u32 {
                steps = steps.push(container(bar(&[((done - at as f32).clamp(0.0, 1.0), ACCENT)], 10.0)).width(Length::FillPortion(1)));
                marks = marks.push(container(mono(w.lang().group(u64::from(title.target) * u64::from(at + 1) / 4), 11.0, FAINT, false)).width(Length::FillPortion(1)).align_x(iced::alignment::Horizontal::Right));
            }
            let head = row![text(w.t("title-progress")).font(theme::SANS_SEMI).size(14.0).color(ui::faded(INK)), ui::grow(), mono(w.of(u64::from(value.min(title.target)), u64::from(title.target)), 14.0, INK, false)].align_y(iced::Center);
            right = right.push(
                container(column![head, steps, marks].spacing(10))
                    .padding([14, 16])
                    .style(move |_| container::Style { border: Border { color: Color::from_rgba(0.353, 0.137, 0.149, k), width: 1.0, radius: 14.0.into() }, ..container::Style::default() }),
            );
        }
        _ => {}
    }
    let right = container(right).height(TITLE_HIGH).width(Length::Fill).padding(Padding { top: 14.0, right: 18.0, bottom: 22.0, left: 24.0 });
    container(scrollable(row![left, right]).direction(ui::hidden_bar()).height(Length::Shrink)).padding(1).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_map_line_gives_its_song_its_artist_and_its_difficulty() {
        assert_eq!(named("Phoneboy — Nevermind [Insane]"), ("Phoneboy".to_owned(), "Nevermind".to_owned(), "Insane".to_owned()));
        assert_eq!(named("xi — FREEDOM DiVE"), ("xi".to_owned(), "FREEDOM DiVE".to_owned(), String::new()));
        assert_eq!(named("Blue Zenith [FOUR DIMENSIONS]"), (String::new(), "Blue Zenith".to_owned(), "FOUR DIMENSIONS".to_owned()));
    }

    #[test]
    fn a_gap_is_signed_and_a_large_score_is_shortened() {
        assert_eq!(signed(117.4), "+117");
        assert_eq!(signed(-56.0), "−56");
        assert_eq!(signed(0.2), "0");
        assert_eq!(signed(240_000.0), "+240k");
        assert_eq!(short(1_250_000.0), "1.2M");
    }

    #[test]
    fn a_short_board_takes_less_room_and_a_long_one_no_more_than_five_lines() {
        assert_eq!(board_high(0), board_high(3));
        assert_eq!(board_high(3), 3.0 * LINE + 2.0 * GAP);
        assert!(board_high(4) > board_high(3) && board_high(4) < BOARD);
        assert_eq!(board_high(5), BOARD);
        assert_eq!(board_high(40), BOARD);
    }

    #[test]
    fn the_rarest_title_comes_first_and_the_most_common_last() {
        let mut catalog = crate::community::Catalog::staged(Vec::new(), "NaumRedlo", 1_789_560_000);
        catalog.held = [("ss_8star", 1), ("archivist", 1), ("registered", 43)].into_iter().map(|(code, held)| (code.to_owned(), held)).collect();
        let of = catalog.titles.len();
        assert_eq!(rarity_place(&catalog, "registered"), (of, of, of));
        assert!(rarity_place(&catalog, "archivist").0 < rarity_place(&catalog, "registered").0);
    }

    #[test]
    fn titles_held_by_as_many_share_their_place_instead_of_being_told_apart_by_their_names() {
        let mut catalog = crate::community::Catalog::staged(Vec::new(), "NaumRedlo", 1_789_560_000);
        let of = catalog.titles.len();
        for person in &mut catalog.people {
            person.titles.clear();
        }
        catalog.held = catalog.titles.iter().enumerate().map(|(at, title)| (title.code.clone(), if at < 3 { 2 } else if at < 5 { 7 } else { 40 })).collect();
        let first = catalog.titles[0].code.clone();
        assert_eq!(rarity_place(&catalog, &first), (1, 3, of), "three titles are equally rare and all three hold places one to three");
        assert_eq!(rarity_place(&catalog, &catalog.titles[2].code), (1, 3, of));
        assert_eq!(rarity_place(&catalog, &catalog.titles[3].code), (4, 5, of));
        assert_eq!(rarity_place(&catalog, &catalog.titles[of - 1].code), (6, of, of), "and the commonest is shared with every title as common");
        assert_eq!(rarity_place(&catalog, "no-such-title"), (0, 0, of));
    }

    #[test]
    fn a_title_nobody_holds_is_as_rare_as_every_other_that_nobody_holds() {
        let mut catalog = crate::community::Catalog::staged(Vec::new(), "NaumRedlo", 1_789_560_000);
        let of = catalog.titles.len();
        catalog.held = std::collections::HashMap::new();
        for person in &mut catalog.people {
            person.titles.clear();
        }
        let (first, last, total) = rarity_place(&catalog, &catalog.titles[of - 1].code);
        assert_eq!((first, last, total), (1, of, of), "with no one holding anything every title shares the same place");
    }
}
