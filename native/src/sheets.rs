use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::widget::{button, column, container, image, row, scrollable, stack, text, Space};
use iced::{mouse, Background, Border, Color, Element, Length, Padding, Point, Radians, Rectangle, Renderer, Shadow, Theme};

use crate::community::{wire, Person, Title};
use crate::community_screen::{self as screen, Ground, Message, Scored};
use crate::glyphs::{glyph, Icon};
use crate::lang::Words;
use crate::theme::{self, ACCENT, FAINT, INK, MUTED};
use crate::ui;

pub(crate) const WIDE: f32 = 920.0;
const HEAD: f32 = 230.0;
const RING: f32 = 136.0;
const SIDE: f32 = 34.0;
const RULE: f32 = 40.0;
const ROW_MOD: f32 = 16.0;
const TURNING: f32 = 16.0;
const MAPPER_FACE: f32 = 20.0;
const SOFT_WIDE: u32 = 480;
const SOFT_KEPT: usize = 8;
const HOVERED: Color = Color::from_rgb(0.102, 0.071, 0.078);
const PICKED: Color = Color::from_rgb(0.137, 0.106, 0.075);
const OWN: Color = Color::from_rgb(0.702, 0.227, 0.227);
const SOFT_INK: Color = Color::from_rgb(0.886, 0.804, 0.792);
const BOARD: f32 = 234.0;
const LINE: f32 = 42.0;
const GAP: f32 = 6.0;
const EDGE: f32 = 22.0;
const TITLE_HIGH: f32 = 606.0;
const TITLE_LEFT: f32 = 300.0;
const GOLD: Color = theme::GRADE_S;
const GREEN: Color = theme::HIT_100;
const BLUE: Color = theme::HIT_300;
const YOURS: Color = Color::from_rgb(0.941, 0.439, 0.439);
const COVER_SEEN: f32 = 0.55;
const COVER_FADE: [(f32, f32); 5] = [(0.0, 0.25), (0.45, 0.45), (0.7, 0.7), (0.9, 0.93), (1.0, 1.0)];

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
        border: Border { color: crate::theme::outline(Color::from_rgba(1.0, 1.0, 1.0, 0.06 * k)), width: 1.0, radius: radius.into() },
        ..container::Style::default()
    }
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
        .style(move |_| container::Style { border: Border { color: crate::theme::outline(edge), width: 1.0, radius: 6.0.into() }, ..container::Style::default() })
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

pub(crate) fn board_high(rows: usize) -> f32 {
    (rows as f32 * (LINE + GAP) - GAP).clamp(3.0 * (LINE + GAP) - GAP, BOARD)
}

fn edged<'a>(rolled: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    crate::glide::brim(rolled).on(theme::SLAB_SOLID).both().high(EDGE).into()
}

fn worth_said(w: &Words, play: &wire::Play, by_score: bool) -> String {
    match (by_score, play.pp > 0.0, play.pp_if) {
        (true, _, _) => w.lang().group(play.score),
        (false, true, _) => format!("{} PP", screen::decimal(w, play.pp, 0)),
        (false, false, Some(guess)) => format!("≈{} PP", screen::decimal(w, guess, 0)),
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
        button::Style { background: wash.map(Background::Color), text_color: INK, border: Border { color: crate::theme::outline(edge), width: wide, radius: radius.into() }, shadow: Shadow::default(), snap: true }
    }
}

fn framed(chosen: bool, you: bool) -> (Color, Option<Color>, f32) {
    match (chosen, you) {
        (true, _) => (Color::from_rgb(0.541, 0.455, 0.188), Some(Color::from_rgb(0.09, 0.071, 0.039)), 1.0),
        (false, true) => (Color::from_rgb(0.561, 0.196, 0.204), None, 1.0),
        (false, false) => (Color::from_rgba(1.0, 1.0, 1.0, 0.07), None, 1.0),
    }
}

thread_local! {
    static SOFT: std::cell::RefCell<std::collections::HashMap<iced::advanced::image::Id, image::Handle>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

fn soft(handle: &image::Handle) -> Option<image::Handle> {
    let image::Handle::Rgba { width, height, pixels, .. } = handle else {
        return None;
    };
    let id = handle.id();
    if let Some(made) = SOFT.with(|kept| kept.borrow().get(&id).cloned()) {
        return Some(made);
    }
    let (wide, high) = (*width as usize, *height as usize);
    if wide == 0 || high == 0 || pixels.len() < wide * high * 4 {
        return None;
    }
    let small_wide = (SOFT_WIDE as usize).min(wide);
    let small_high = ((small_wide as f32 * high as f32 / wide as f32).round() as usize).max(1);
    let mut out = vec![0u8; small_wide * small_high * 4];
    for y in 0..small_high {
        let (top, bottom) = (y * high / small_high, ((y + 1) * high / small_high).max(y * high / small_high + 1));
        for x in 0..small_wide {
            let (left, right) = (x * wide / small_wide, ((x + 1) * wide / small_wide).max(x * wide / small_wide + 1));
            let mut sum = [0u64; 4];
            for row in top..bottom {
                for column in left..right {
                    let at = (row * wide + column) * 4;
                    for channel in 0..4 {
                        sum[channel] += u64::from(pixels[at + channel]);
                    }
                }
            }
            let count = ((bottom - top) * (right - left)) as u64;
            let at = (y * small_wide + x) * 4;
            for channel in 0..4 {
                out[at + channel] = (sum[channel] / count) as u8;
            }
        }
    }
    let made = image::Handle::from_rgba(small_wide as u32, small_high as u32, out);
    SOFT.with(|kept| {
        let mut kept = kept.borrow_mut();
        if kept.len() > SOFT_KEPT {
            kept.clear();
        }
        kept.insert(id, made.clone());
    });
    Some(made)
}

fn board_line<'a>(ground: &Ground<'a>, said: &'a wire::BoardRow, by_score: bool, chosen: bool) -> Element<'a, Message> {
    let w = ground.words;
    let k = ui::fade();
    let place = mono(format!("#{}", said.place), 12.0, screen::medal(said.place as usize).unwrap_or(FAINT), true);
    let mut name = row![text(said.name.clone()).font(theme::SANS_SEMI).size(14.0).wrapping(text::Wrapping::None).color(ui::faded(INK))].spacing(8).align_y(iced::Center);
    let mut mods = row![].spacing(3).align_y(iced::Center);
    for acronym in said.play.mods.iter().filter(|acronym| crate::modicons::shown(acronym)) {
        mods = mods.push(crate::modicons::badge(acronym, ROW_MOD, 1.0));
    }
    name = name.push(mods);
    let when = said.play.at.map_or(String::new(), |at| w.day(at, ground.now_unix));
    let combo = said.play.combo.map_or(String::new(), |combo| format!("{combo}x"));
    let line = row![
        container(place).width(30.0),
        row_face(ground, said, 24.0),
        container(name).width(Length::Fill).clip(true),
        container(text(when).font(theme::SANS).size(12.0).wrapping(text::Wrapping::None).color(ui::faded(FAINT))).width(120.0),
        container(mono(w.percent(f64::from(said.play.accuracy)), 12.0, MUTED, false)).width(64.0).align_x(iced::alignment::Horizontal::Right),
        container(mono(combo, 12.0, MUTED, false)).width(64.0).align_x(iced::alignment::Horizontal::Right),
        container(mono(worth_said(w, &said.play, by_score), 13.0, INK, true)).width(88.0).align_x(iced::alignment::Horizontal::Right),
    ]
    .spacing(12)
    .align_y(iced::Center);
    let you = said.you;
    button(container(line).height(LINE).center_y(LINE).padding([0, 12]))
        .padding(0)
        .width(Length::Fill)
        .style(move |_, status| {
            let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let fill = if chosen { Some(Color { a: k, ..PICKED }) } else { lit.then_some(Color { a: k, ..HOVERED }) };
            button::Style {
                background: fill.map(Background::Color),
                text_color: INK,
                border: Border { color: if you { Color { a: k, ..OWN } } else { Color::TRANSPARENT }, width: if you { 1.5 } else { 0.0 }, radius: 9.0.into() },
                shadow: Shadow::default(),
                snap: true,
            }
        })
        .on_press(Message::ScorePick(said.who))
        .into()
}

fn board_table<'a>(ground: &Ground<'a>, board: &'a wire::MapBoard, chosen: i64) -> Element<'a, Message> {
    let mut lines = column![].spacing(GAP);
    for said in &board.rows {
        lines = lines.push(board_line(ground, said, board.by_score(), said.who == chosen));
    }
    edged(scrollable(lines).direction(ui::hidden_bar()).height(board_high(board.rows.len())).width(Length::Fill))
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
    let fade = container(Space::new().width(Length::Fill).height(HEAD)).style(move |_| container::Style { background: Some(Background::Gradient(into_body.into())), border: Border { radius: top, ..Border::default() }, ..container::Style::default() });
    match scored.map.sharp_cover().and_then(|url| ground.pictures.get(&screen::wide(&url))) {
        Some(handle) => {
            let shown = scored.map.sharp_cover().map_or(1.0, |url| ground.picture_shown(&screen::wide(&url)));
            let picture = soft(handle).unwrap_or_else(|| handle.clone());
            stack![under, image(picture).content_fit(iced::ContentFit::Cover).width(Length::Fill).height(HEAD).border_radius(15.0).opacity(COVER_SEEN * k * shown), fade].into()
        }
        None => under.into(),
    }
}

struct Shown {
    grade: String,
    pp: f32,
    pp_if: Option<f32>,
    score: u64,
    accuracy: f32,
    combo: Option<u32>,
    counts: [Option<u32>; 4],
}

fn same_play(mine: &crate::community::Play, row: &wire::Play) -> bool {
    match (mine.id, row.id) {
        (Some(a), Some(b)) => a == b,
        _ => row.at == Some(mine.at) || ((mine.accuracy - row.accuracy).abs() < 0.01 && mine.combo == row.combo && mine.combo.is_some()),
    }
}

impl Shown {
    fn filled(mut self, other: &Shown) -> Shown {
        if self.pp <= 0.0 {
            self.pp = other.pp;
            self.pp_if = self.pp_if.or(other.pp_if);
        }
        if self.score == 0 {
            self.score = other.score;
        }
        self.combo = self.combo.or(other.combo);
        if self.counts.iter().all(Option::is_none) {
            self.counts = other.counts;
        }
        self
    }

    fn of_row(play: &wire::Play) -> Shown {
        let count = |at: usize| play.counts.get(at).copied().flatten();
        Shown { grade: play.grade.clone(), pp: play.pp, pp_if: play.pp_if, score: play.score, accuracy: play.accuracy, combo: play.combo, counts: [count(0), count(1), count(2), count(3)] }
    }

    fn of_scored(scored: &Scored) -> Shown {
        let play = &scored.play;
        let grade = if scored.passed { play.grade.clone() } else { "F".to_owned() };
        Shown { grade, pp: play.pp, pp_if: play.pp_if, score: play.score, accuracy: play.accuracy, combo: play.combo, counts: play.counts }
    }
}

pub(crate) fn score<'a>(ground: &Ground<'a>, scored: &'a Scored) -> Element<'a, Message> {
    let w = ground.words;
    let k = ui::fade();
    let board = scored.map.beatmap.and_then(|beatmap| ground.boards.get(&beatmap).map(|board| (beatmap, board)));
    let picked = ground.score_pick.and_then(|who| board.and_then(|(_, board)| board.rows.iter().find(|row| row.who == who)));
    let chosen = picked.map_or(scored.who, |row| row.who);
    let own = board.and_then(|(_, board)| board.rows.iter().find(|row| row.who == scored.who && same_play(&scored.play, &row.play)));
    let shown = match (picked, own) {
        (Some(row), Some(mine)) if row.who == mine.who => Shown::of_row(&row.play).filled(&Shown::of_scored(scored)),
        (Some(row), _) => Shown::of_row(&row.play),
        (None, Some(mine)) => Shown::of_scored(scored).filled(&Shown::of_row(&mine.play)),
        (None, None) => Shown::of_scored(scored),
    };
    let shade = screen::grade_colour(&shown.grade);
    let (artist, title, version) = named(&scored.map.line);
    let about = board.and_then(|(_, board)| board.map.as_ref());

    let mut name = row![text(title).font(theme::SANS_SEMI).size(30.0).wrapping(text::Wrapping::None).color(ui::faded(INK))].spacing(10).align_y(iced::Bottom);
    if !version.is_empty() {
        name = name.push(container(text(version).font(theme::SANS_SEMI).size(19.0).wrapping(text::Wrapping::None).color(ui::faded(SOFT_INK))).padding(Padding::ZERO.bottom(3.0)));
    }
    let mut titles = column![container(name).clip(true)].spacing(6).width(Length::Fill);
    if !artist.is_empty() {
        titles = titles.push(text(artist).font(theme::SANS).size(15.0).wrapping(text::Wrapping::None).color(ui::faded(SOFT_INK)));
    }
    let mut facts = row![].spacing(12).align_y(iced::Center);
    if let Some(stars) = scored.map.stars.or(scored.play.stars) {
        facts = facts.push(crate::dossier::star_pill(screen::decimal(w, stars, 2), stars, 12.0));
    }
    if let Some(creator) = about.map(|about| about.map.creator.clone()).filter(|creator| !creator.is_empty()) {
        let face: Element<'a, Message> = match board.and_then(|(_, board)| board.mapper_face()) {
            Some(url) => screen::round(ground.pictures.get(&url), &initial(&creator), MAPPER_FACE, ground.picture_shown(&url)),
            None => glyph(Icon::Mapper, 15.0, SOFT_INK),
        };
        facts = facts.push(row![face, text(creator).font(theme::SANS_SEMI).size(14.0).wrapping(text::Wrapping::None).color(ui::faded(INK))].spacing(7).align_y(iced::Center));
    }
    if scored.play.witnessed && picked.is_none() {
        facts = facts.push(mono(w.t("score-witnessed"), 11.0, FAINT, false));
    }
    let titles = titles.push(container(facts).padding(Padding::ZERO.top(4.0)));
    let letter = container(text(shown.grade.clone()).font(theme::SANS_SEMI).size(104.0).wrapping(text::Wrapping::None).color(ui::faded(shade))).padding(Padding::ZERO.right(4.0));
    let top = stack![
        cover_back(ground, scored),
        container(row![ui::grow(), close()]).padding(Padding { top: 8.0, right: 10.0, bottom: 0.0, left: 0.0 }),
        container(row![letter, titles].spacing(24).align_y(iced::Bottom)).height(HEAD).align_y(iced::Bottom).padding(Padding { top: 0.0, right: SIDE, bottom: 22.0, left: SIDE }),
    ]
    .height(HEAD);

    let stat = |label: String, colour: Color, value: Element<'a, Message>| -> Element<'a, Message> {
        column![text(label).font(theme::SANS).size(12.0).wrapping(text::Wrapping::None).color(ui::faded(colour)), value].spacing(3).into()
    };
    let pp = match (shown.pp > 0.0, shown.pp_if) {
        (true, _) => Some(screen::decimal(w, shown.pp, 0)),
        (false, Some(guess)) => Some(format!("≈{}", screen::decimal(w, guess, 0))),
        (false, None) => (shown.grade == "F").then(|| "0".to_owned()),
    };
    let most = scored.play.max_combo.or(about.and_then(|about| about.max_combo)).filter(|most| *most > 0);
    let combo: Element<'a, Message> = match (shown.combo, most) {
        (Some(combo), Some(most)) => row![mono(format!("{combo}x"), 18.0, INK, true), text(w.t("combo-of")).font(theme::SANS).size(13.0).color(ui::faded(FAINT)), mono(format!("{most}x"), 13.0, FAINT, false)].spacing(5).align_y(iced::Bottom).into(),
        (Some(combo), None) => mono(format!("{combo}x"), 18.0, INK, true),
        _ => mono("—".to_owned(), 18.0, FAINT, true),
    };
    let counted = shown.counts.iter().any(Option::is_some);
    let count = |at: usize| mono(shown.counts[at].or(counted.then_some(0)).map_or("—".to_owned(), |n| n.to_string()), 18.0, INK, true);
    let hits = row![
        stat("300".to_owned(), BLUE, count(0)),
        stat("100".to_owned(), GREEN, count(1)),
        stat("50".to_owned(), GOLD, count(2)),
        stat(w.t("score-misses"), YOURS, count(3)),
    ]
    .spacing(16)
    .align_y(iced::Bottom);
    let rule = container(Space::new().width(1.0).height(RULE)).style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08 * k))), ..container::Style::default() });
    let mut numbers = row![].spacing(30).align_y(iced::Bottom);
    if let Some(pp) = pp {
        numbers = numbers.push(stat("PP".to_owned(), FAINT, mono(pp, 28.0, INK, true)));
    }
    let numbers = numbers.extend([
        stat(w.t("score-points"), FAINT, mono(if shown.score > 0 { w.lang().group(shown.score) } else { "—".to_owned() }, 18.0, INK, true)),
        stat(w.t("score-accuracy"), FAINT, mono(w.percent(f64::from(shown.accuracy)), 18.0, INK, true)),
        stat(w.t("score-combo"), FAINT, combo),
        rule.into(),
        hits.into(),
    ]);
    let panel = container(numbers).padding([16, 24]).style(move |_| container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.3 * k))),
        border: Border { radius: 14.0.into(), ..Border::default() },
        ..container::Style::default()
    });

    let mut body = column![container(panel).center_x(Length::Fill)].spacing(10);
    let note = |words: String, colour: Color| -> Element<'a, Message> {
        container(text(words).font(theme::SANS).size(theme::BODY).color(ui::faded(colour))).center(Length::Fill).height(board_high(0)).width(Length::Fill).style(move |_| well(k, 14.0)).into()
    };
    match (scored.map.beatmap, board) {
        (_, Some((beatmap, board))) => {
            let turning: Element<'a, Message> = if ground.boards_waiting.contains(&beatmap) { ui::turning(TURNING) } else { Space::new().width(TURNING).height(TURNING).into() };
            body = body.push(row![ui::grow(), turning]);
            body = body.push(ui::smooth(if board.rows.is_empty() { note(w.t("score-board-empty"), FAINT) } else { board_table(ground, board, chosen) }));
        }
        (Some(beatmap), None) if ground.boards_failed.contains(&beatmap) => body = body.push(ui::smooth(note(w.t("score-board-failed"), ACCENT))),
        (Some(_), None) if !ground.catalog.staged => body = body.push(ui::smooth(container(ui::turning(26.0)).center(Length::Fill).height(board_high(0)).width(Length::Fill).style(move |_| well(k, 14.0)))),
        _ => {}
    }
    if let Some(beatmap) = scored.map.beatmap {
        body = body.push(container(ui::primary(w.t("score-open-map"), Some(Message::Open(format!("https://osu.ppy.sh/b/{beatmap}"))))).padding(Padding::ZERO.top(6.0)));
    }
    let rolled = scrollable(container(body).padding(Padding { top: 0.0, right: SIDE, bottom: 22.0, left: SIDE })).direction(ui::hidden_bar()).height(Length::Shrink);
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

struct RarityStrip {
    first: usize,
    last: usize,
    of: usize,
    tint: Color,
    alpha: f32,
}

impl<M> canvas::Program<M> for RarityStrip {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let (wide, high) = (bounds.width, bounds.height);
        let lane = 6.0;
        let top = (high - lane) / 2.0;
        let paint = |a: f32| Color { a: a * self.alpha, ..self.tint };
        frame.fill(&Path::rounded_rectangle(Point::new(0.0, top), iced::Size::new(wide, lane), (lane / 2.0).into()), paint(0.22));
        let of = self.of.max(1) as f32;
        if self.first > 0 {
            let from = (self.first - 1) as f32 / of * wide;
            let to = (self.last as f32 / of * wide).max(from + lane);
            frame.fill(&Path::rounded_rectangle(Point::new(0.0, top), iced::Size::new(to, lane), (lane / 2.0).into()), paint(0.45));
            frame.fill(&Path::rounded_rectangle(Point::new(from, 0.0), iced::Size::new(to - from, high), (high / 2.0).min((to - from) / 2.0).into()), paint(1.0));
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
        (Some(mine), Some(first)) => caption(w.with("title-later", &[("days", w.span_days(((mine - first).max(0) as u64).div_ceil(86_400)))])),
        _ => Space::new().height(15.0).into(),
    };
    let when = earned.map_or(String::new(), |at| w.day(at, ground.now_unix));
    let inside = column![screen::face(ground, person, 52.0), container(name).clip(true), second, text(when).font(theme::SANS).size(12.5).color(ui::faded(MUTED))].spacing(6).align_x(iced::Center).width(Length::Fill);
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
        .style(move |_| container::Style { border: Border { color: crate::theme::outline(Color::from_rgba(0.353, 0.137, 0.149, k)), width: 1.0, radius: 14.0.into() }, ..container::Style::default() })
        .into()
}

pub(crate) fn title<'a>(ground: &Ground<'a>, code: &str, who: Option<i64>) -> Element<'a, Message> {
    let w = ground.words;
    let k = ui::fade();
    let catalog = ground.catalog;
    let Some(title) = catalog.title_of(code) else {
        return container(row![caption(code.to_owned()), ui::grow(), close()]).padding(20).into();
    };
    let holders = screen::scoped_holders(ground, code);
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
    let ticks = Canvas::new(RarityStrip { first: first_place, last: last_place, of, tint, alpha: k }).width(Length::Fill).height(12.0);
    let place = if first_place == last_place { format!("#{first_place}") } else { format!("#{first_place}–{last_place}") };
    let left = column![
        emblem,
        container(ui::label(w.t(title.rarity.key()), tint)).padding(Padding::ZERO.top(6.0)),
        text(title.name(w.lang()).to_owned()).font(theme::SANS_SEMI).size(26.0).color(ui::faded(INK)).align_x(iced::Center),
        text(title.about(w.lang()).to_owned()).font(theme::SANS).size(14.0).color(ui::faded(MUTED)).align_x(iced::Center),
        ui::grow_tall(),
        column![mono(format!("{:.0}%", share * 100.0), 22.0, tint, true), text(w.t("title-share-said")).font(theme::SANS).size(12.0).color(ui::faded(MUTED))].spacing(4).width(Length::Fill),
        column![
            row![caption(w.with("title-rarity-among", &[("n", of.to_string())])), ui::grow(), mono(w.with("title-rarity-place", &[("n", place)]), 13.0, INK, false)].align_y(iced::Center),
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
        border: Border { color: crate::theme::outline(edge), width: 1.0, radius: iced::border::Radius { top_left: 15.0, top_right: 0.0, bottom_right: 0.0, bottom_left: 15.0 } },
        ..container::Style::default()
    });

    let you = catalog.people.iter().find(|person| person.you);
    let yours = you.is_some_and(|you| you.titles.iter().any(|held| held == code));
    let progress = catalog.me.as_ref().and_then(|me| me.title_progress.get(code).copied());
    let mut cards: Vec<Element<'a, Message>> = ordered.iter().map(|at| holder_card(ground, *at, &catalog.people[*at], earned(&catalog.people[*at]), first, who == Some(catalog.people[*at].id))).collect();
    if let (Some(you), false) = (you, yours) {
        cards.push(waiting_card(ground, you, progress.filter(|_| title.target > 1).map(|value| title.target.saturating_sub(value))));
    }
    let waiting: Vec<&Person> = catalog.people.iter().filter(|person| ground.title_scope.takes(person) && !person.you && !person.titles.iter().any(|held| held == code)).collect();
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
                .style(move |_| container::Style { border: Border { color: crate::theme::outline(Color::from_rgba(1.0, 1.0, 1.0, 0.07 * k)), width: 1.0, radius: 12.0.into() }, ..container::Style::default() }),
        );
    }
    let mut right = column![
        row![ui::section(w.t(if ground.title_scope == screen::TitleScope::Server { "title-holders-server" } else { "title-holders" })), ui::grow(), close()].align_y(iced::Center),
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
                    .style(move |_| container::Style { background: Some(Background::Color(wash)), border: Border { color: crate::theme::outline(edge), width: 1.0, radius: 14.0.into() }, ..container::Style::default() }),
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
                    .style(move |_| container::Style { border: Border { color: crate::theme::outline(Color::from_rgba(0.353, 0.137, 0.149, k)), width: 1.0, radius: 14.0.into() }, ..container::Style::default() }),
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
