use std::time::{Duration, Instant};

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::widget::{tree, Tree, Widget};
use iced::advanced::{renderer, Clipboard, Shell};
use iced::widget::{button, column, container, mouse_area, row, scrollable, stack, text, Space};
use iced::{mouse, Background, Border, Color, Element, Length, Padding, Rectangle, Shadow, Size, Theme, Vector};

use crate::community::{Board, Catalog, Happening, Kind, LivePlay, Person};
use crate::community_screen::{self as screen, Ground, Message, Reading, Section};
use crate::glyphs::{glyph, Icon};
use crate::news::{self, News};
use crate::theme::{self, ACCENT, FAINT, INK, MUTED};
use crate::ui;

pub const SPOT_EVERY: Duration = Duration::from_secs(5);
pub const RANK_EVERY: Duration = Duration::from_secs(6);
pub const SPOT_SWAP: Duration = Duration::from_millis(450);
pub const RANK_GROW: Duration = Duration::from_millis(900);

const LEFT_WIDE: f32 = 268.0;
const RIGHT_WIDE: f32 = 304.0;
const CENTRE_MOST: f32 = 860.0;
const GAP: f32 = 18.0;
const GREEN: Color = theme::HIT_100;
const LINK_BLUE: Color = Color::from_rgb(0.345, 0.682, 0.988);
const PINK: Color = Color::from_rgb(1.0, 0.4, 0.671);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stream {
    #[default]
    All,
    Group,
    News,
}

impl Stream {
    pub const ALL: [Stream; 3] = [Stream::All, Stream::Group, Stream::News];

    pub fn key(self) -> &'static str {
        match self {
            Stream::All => "filter-all",
            Stream::Group => "stream-group",
            Stream::News => "filter-news",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Filter {
    #[default]
    All,
    Plays,
    Top,
    Titles,
    Ranks,
}

impl Filter {
    pub const ALL: [Filter; 5] = [Filter::All, Filter::Plays, Filter::Top, Filter::Titles, Filter::Ranks];

    pub fn key(self) -> &'static str {
        match self {
            Filter::All => "filter-all",
            Filter::Plays => "filter-plays",
            Filter::Top => "filter-top",
            Filter::Titles => "filter-titles",
            Filter::Ranks => "filter-ranks",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Source {
    #[default]
    All,
    Osu,
    Channels,
    Builds,
}

impl Source {
    pub const ALL: [Source; 4] = [Source::All, Source::Osu, Source::Channels, Source::Builds];

    pub fn key(self) -> &'static str {
        match self {
            Source::All => "filter-all",
            Source::Osu => "source-osu",
            Source::Channels => "source-channels",
            Source::Builds => "filter-builds",
        }
    }
}

#[derive(Clone, Copy)]
enum Item<'a> {
    Play(&'a LivePlay),
    Happened(&'a Happening),
    Story(&'a news::Story),
    Post(&'a news::Post),
    Build(&'a news::Build),
}

#[derive(Clone, Copy)]
struct Event<'a> {
    at: i64,
    item: Item<'a>,
}

impl Event<'_> {
    fn key(&self) -> String {
        match self.item {
            Item::Play(play) => format!("play:{}:{}:{}:{}", play.who, play.map, play.at, (play.pp * 100.0).round()),
            Item::Happened(happened) => format!("happened:{}:{}", happened.who, happened.at),
            Item::Story(story) => format!("story:{}", story.url),
            Item::Post(post) => format!("post:{}", post.url),
            Item::Build(build) => format!("build:{}", build.url),
        }
    }

    fn in_group(&self) -> bool {
        matches!(self.item, Item::Play(_) | Item::Happened(_))
    }

    fn admitted(&self, filter: Filter) -> bool {
        match (filter, self.item) {
            (_, Item::Story(_) | Item::Post(_) | Item::Build(_)) => false,
            (Filter::All, _) => true,
            (Filter::Plays, Item::Play(_)) => true,
            (Filter::Plays | Filter::Top, Item::Happened(happened)) => matches!(happened.kind, Kind::TopPlay { .. }) || (filter == Filter::Plays && matches!(happened.kind, Kind::Render { .. })),
            (Filter::Titles, Item::Happened(happened)) => matches!(happened.kind, Kind::Title(_)),
            (Filter::Ranks, Item::Happened(happened)) => matches!(happened.kind, Kind::Climb { .. }),
            _ => false,
        }
    }

    fn sourced(&self, source: Source) -> bool {
        matches!(
            (source, self.item),
            (Source::All, Item::Story(_) | Item::Post(_) | Item::Build(_)) | (Source::Osu, Item::Story(_)) | (Source::Channels, Item::Post(_)) | (Source::Builds, Item::Build(_))
        )
    }

    fn found(&self, catalog: &Catalog, query: &str) -> bool {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return true;
        }
        let has = |words: &str| words.to_lowercase().contains(&query);
        let map_has = |map: Option<usize>| catalog.map(map).is_some_and(|map| has(&map.line));
        match self.item {
            Item::Play(play) => catalog.people.get(play.who).is_some_and(|person| has(&person.name)) || map_has(Some(play.map)),
            Item::Happened(happened) => {
                let title = match &happened.kind {
                    Kind::Title(code) => catalog.title_of(code).is_some_and(|title| has(&title.name[0]) || has(&title.name[1])),
                    _ => false,
                };
                catalog.people.get(happened.who).is_some_and(|person| has(&person.name)) || map_has(happened.map) || title
            }
            Item::Story(story) => has(&story.title) || has(&story.lead),
            Item::Post(post) => has(&post.text) || has(&post.channel) || has(&post.name),
            Item::Build(build) => has(&build.version) || build.changes.iter().any(|change| has(&change.title)),
        }
    }
}

fn chosen_posts<'a>(news: &'a News, channels: &[String]) -> impl Iterator<Item = &'a news::Post> + 'a {
    let channels: Vec<String> = channels.iter().map(|c| c.to_ascii_lowercase()).collect();
    news.posts.iter().filter(move |post| channels.contains(&post.channel.to_ascii_lowercase()))
}

fn events<'a>(catalog: &'a Catalog, news: &'a News, channels: &[String], live_shown: usize) -> Vec<Event<'a>> {
    let shown = live_shown.min(catalog.live.len());
    let mut out: Vec<Event<'a>> = Vec::new();
    out.extend(catalog.live[..shown].iter().map(|play| Event { at: play.at, item: Item::Play(play) }));
    out.extend(catalog.feed.iter().map(|happened| Event { at: happened.at, item: Item::Happened(happened) }));
    out.extend(news.stories.iter().take(12).map(|story| Event { at: story.at, item: Item::Story(story) }));
    out.extend(chosen_posts(news, channels).take(20).map(|post| Event { at: post.at, item: Item::Post(post) }));
    out.extend(news.builds.iter().take(8).map(|build| Event { at: build.at, item: Item::Build(build) }));
    out.sort_by(|a, b| b.at.cmp(&a.at));
    out.truncate(90);
    out
}

pub fn newest(catalog: &Catalog, news: &News, channels: &[String], live_shown: usize) -> i64 {
    events(catalog, news, channels, live_shown).first().map_or(0, |event| event.at)
}

fn surface(open: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let lit = open || matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: Some(Background::Color(if lit { Color::from_rgba(1.0, 1.0, 1.0, 0.016) } else { theme::RAISED })),
            text_color: INK,
            border: Border { color: if lit { Color::from_rgba(1.0, 1.0, 1.0, 0.06) } else { theme::LINE }, width: 1.0, radius: 14.0.into() },
            shadow: if lit { Shadow { color: Color::from_rgba(0.0, 0.0, 0.0, 0.35), offset: Vector::new(0.0, 8.0), blur_radius: 24.0 } } else { Shadow::default() },
            snap: true,
        }
    }
}

fn quiet(_: &Theme, status: button::Status) -> button::Style {
    let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(Background::Color(if lit { Color::from_rgba(1.0, 1.0, 1.0, 0.03) } else { Color::TRANSPARENT })),
        text_color: if lit { INK } else { MUTED },
        border: Border { color: if lit { Color::from_rgba(1.0, 1.0, 1.0, 0.07) } else { theme::LINE }, width: 1.0, radius: 8.0.into() },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn card<'a>(inside: impl Into<Element<'a, Message>>, padding: impl Into<Padding>) -> container::Container<'a, Message> {
    container(inside).padding(padding).width(Length::Fill).style(ui::box_faded(|theme: &Theme| container::Style { border: Border { radius: 14.0.into(), ..theme::slab(theme).border }, ..theme::slab(theme) }))
}

fn caption<'a>(words: String) -> Element<'a, Message> {
    ui::mono_small(words.to_uppercase(), FAINT)
}

fn day_divider<'a>(ground: &Ground<'a>, at: i64) -> Element<'a, Message> {
    let w = ground.words;
    let today = ground.now_unix - ground.now_unix.rem_euclid(86_400);
    let said = if at >= today {
        w.t("day-today")
    } else if at >= today - 86_400 {
        w.t("day-yesterday")
    } else {
        w.day(at, ground.now_unix)
    };
    let k = ui::fade();
    row![
        caption(said),
        container(Space::new().height(1.0)).width(Length::Fill).style(move |_| container::Style { background: Some(Background::Color(Color { a: theme::LINE.a * k, ..theme::LINE })), ..container::Style::default() }),
    ]
    .spacing(10)
    .align_y(iced::Center)
    .padding([4, 2])
    .into()
}

pub(crate) fn counts_row<'a>(ground: &Ground<'a>, counts: [Option<u32>; 4], combo: Option<(u32, u32)>, stars: Option<f32>) -> Element<'a, Message> {
    let w = ground.words;
    let k = ui::fade();
    let cells = [("300", counts[0], LINK_BLUE), ("100", counts[1], GREEN), ("50", counts[2], theme::GRADE_S), ("✕", counts[3], ACCENT)];
    let mut shown = row![].spacing(8);
    for (label, value, colour) in cells {
        let Some(value) = value else {
            continue;
        };
        shown = shown.push(
            container(column![ui::mono_small(label.to_owned(), colour), text(w.lang().group(u64::from(value))).font(theme::SANS_SEMI).size(16.0).color(ui::faded(INK))].spacing(2))
                .padding([8, 10])
                .width(Length::FillPortion(1))
                .style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.22 * k))), border: Border { radius: 10.0.into(), ..Border::default() }, ..container::Style::default() }),
        );
    }
    let mut facts: Vec<String> = Vec::new();
    if let Some((combo, max)) = combo {
        facts.push(format!("{} / {}x", w.lang().group(u64::from(combo)), w.lang().group(u64::from(max))));
    }
    if let Some(stars) = stars {
        facts.push(format!("{}★", screen::decimal(w, stars, 2)));
    }
    column![shown, ui::mono_small(facts.join(" · "), MUTED)].spacing(8).into()
}

fn map_cover<'a>(ground: &Ground<'a>, map: usize) -> Option<&'a iced::widget::image::Handle> {
    ground.catalog.map(Some(map)).and_then(|map| ground.thumbs.get(&map.hash).or_else(|| map.card().and_then(|card| ground.pictures.get(&card))))
}

fn map_title(ground: &Ground<'_>, map: usize) -> String {
    let line = screen::line_of(ground, map);
    match line.split_once(" — ") {
        Some((_, rest)) => rest.to_owned(),
        None => line,
    }
}

fn post_cover<'a>(ground: &Ground<'a>, post: &news::Post) -> Option<&'a iced::widget::image::Handle> {
    post.cover().and_then(|url| ground.pictures.get(url))
}

fn first_line(words: &str) -> String {
    words.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or_default().to_owned()
}

fn clock<'a>(ground: &Ground<'a>, at: i64, wide: f32) -> Element<'a, Message> {
    container(ui::mono_small(ground.words.clock(at), MUTED)).width(wide).into()
}

fn tinted_square<'a>(icon: Icon, colour: Color, side: f32) -> Element<'a, Message> {
    let k = ui::fade();
    container(glyph(icon, side * 0.54, colour))
        .width(side)
        .height(side)
        .center(side)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color { a: 0.14 * k, ..colour })),
            border: Border { color: Color { a: 0.42 * k, ..colour }, width: 1.0, radius: (side * 0.3).into() },
            ..container::Style::default()
        })
        .into()
}

fn row_press(lit_colour: Option<Color>) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let k = ui::fade();
        let wash = lit_colour.map(|colour| Background::Gradient(iced::Gradient::Linear(
            iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2)).add_stop(0.0, Color { a: 0.022 * k, ..colour }).add_stop(0.6, Color { a: 0.0, ..colour }),
        )));
        button::Style {
            background: if lit { Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.035))) } else { wash },
            text_color: INK,
            border: Border { radius: 9.0.into(), ..Border::default() },
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

struct Line<'a> {
    icon: Option<(Icon, Color)>,
    who: Option<usize>,
    name: String,
    under: String,
    tail: Element<'a, Message>,
    press: Message,
    details: Option<Element<'a, Message>>,
    tint: Option<Color>,
    cover: Option<usize>,
    accuracy: Option<f32>,
    mods: Vec<String>,
}

fn line_of<'a>(ground: &Ground<'a>, event: &Event<'a>) -> Option<Line<'a>> {
    let w = ground.words;
    let catalog = ground.catalog;
    let key = event.key();
    Some(match event.item {
        Item::Play(play) => {
            let person = catalog.people.get(play.who)?;
            let tail: Element<'a, Message> = match (play.passed, play.pp >= 0.5) {
                (false, _) => row![Space::new().width(50.0), crate::dossier::grade_badge("F", 24.0)].spacing(10).align_y(iced::Center).into(),
                (true, true) => row![container(text(format!("{} pp", screen::decimal(w, play.pp, 0))).font(theme::SANS_SEMI).size(13.0).wrapping(text::Wrapping::None).color(ui::faded(INK))).width(64.0).align_x(iced::alignment::Horizontal::Right), crate::dossier::grade_badge(&play.grade, 24.0)].spacing(10).align_y(iced::Center).into(),
                (true, false) => row![Space::new().width(64.0), crate::dossier::grade_badge(&play.grade, 24.0)].spacing(10).align_y(iced::Center).into(),
            };
            Line { icon: None, who: Some(play.who), name: person.name.clone(), under: map_title(ground, play.map), tail, press: Message::Person(Some(play.who)), details: None, tint: None, cover: Some(play.map), accuracy: Some(play.accuracy), mods: play.mods.clone() }
        }
        Item::Happened(happened) => {
            let person = catalog.people.get(happened.who)?;
            match &happened.kind {
                Kind::TopPlay { pp, place, mods } => {
                    let map = happened.map?;
                    let play = person.top.iter().find(|play| play.map == map && (play.pp - pp).abs() < 0.5);
                    let details = ground.open_events.contains(&key).then(|| play.map(|play| counts_row(ground, play.counts, play.combo.zip(play.max_combo), play.stars))).flatten();
                    let colour = Color::from_rgb(0.941, 0.408, 0.408);
                    Line {
                        icon: Some((Icon::Star, colour)),
                        who: None,
                        name: person.name.clone(),
                        under: format!("{} · {} · {}", w.t("kind-top"), w.n("top-place", u64::from(*place)), map_title(ground, map)),
                        tail: row![container(text(format!("{} pp", screen::decimal(w, *pp, 0))).font(theme::SANS_SEMI).size(13.0).wrapping(text::Wrapping::None).color(ui::faded(colour))).width(64.0).align_x(iced::alignment::Horizontal::Right), container(glyph(Icon::Star, 14.0, colour)).width(24.0).center_x(24.0)].spacing(10).align_y(iced::Center).into(),
                        press: Message::Toggle(key),
                        details,
                        tint: Some(colour),
                        cover: None,
                        accuracy: play.map(|play| play.accuracy),
                        mods: mods.clone(),
                    }
                }
                Kind::Render { accuracy, mods } => {
                    let map = happened.map?;
                    Line { icon: None, who: Some(happened.who), name: person.name.clone(), under: map_title(ground, map), tail: row![Space::new().width(64.0), container(glyph(Icon::Film, 16.0, MUTED)).width(24.0).center_x(24.0)].spacing(10).align_y(iced::Center).into(), press: Message::Person(Some(happened.who)), details: None, tint: None, cover: Some(map), accuracy: Some(*accuracy), mods: mods.clone() }
                }
                Kind::Title(code) => {
                    let title = catalog.title_of(code)?;
                    let colour = title.rarity.colour();
                    Line {
                        icon: Some((Icon::Trophy, colour)),
                        who: None,
                        name: person.name.clone(),
                        under: format!("{} · {}", w.t("kind-title"), title.name(w.lang())),
                        tail: ui::mono_small(w.t(title.rarity.key()), colour),
                        press: Message::Person(Some(happened.who)),
                        details: None,
                        tint: Some(colour),
                        cover: None,
                        accuracy: None,
                        mods: Vec::new(),
                    }
                }
                Kind::Climb { board, from, to } => Line {
                    icon: Some((Icon::Up, GREEN)),
                    who: None,
                    name: person.name.clone(),
                    under: format!("{} · {} #{from}", w.t(board.key()), w.t("climb-was")),
                    tail: text(format!("#{to}")).font(theme::SANS_SEMI).size(14.0).color(ui::faded(GREEN)).into(),
                    press: Message::Board(*board),
                    details: None,
                    tint: Some(GREEN),
                    cover: None,
                    accuracy: None,
                    mods: Vec::new(),
                },
            }
        }
        _ => return None,
    })
}

fn journal_row<'a>(ground: &Ground<'a>, event: &Event<'a>, wide: bool) -> Option<Element<'a, Message>> {
    let Line { icon, who, name, under, tail, press, details, tint, cover, accuracy, mods } = line_of(ground, event)?;
    let lead: Element<'a, Message> = match (icon, who.and_then(|at| ground.catalog.people.get(at))) {
        (Some((icon, colour)), _) => tinted_square(icon, colour, 26.0),
        (None, Some(person)) => screen::face(ground, person, 26.0),
        (None, None) => Space::new().width(26.0).into(),
    };
    let high = if wide { 42.0 } else { 46.0 };
    let line: Element<'a, Message> = if wide {
        let map_cell: Element<'a, Message> = match cover {
            Some(map) => {
                let thumb: Element<'a, Message> = match map_cover(ground, map) {
                    Some(handle) => iced::widget::image(handle.clone()).content_fit(iced::ContentFit::Cover).width(46.0).height(26.0).border_radius(5.0).opacity(ui::fade()).into(),
                    None => container(ui::fine_hatch()).width(46.0).height(26.0).into(),
                };
                row![thumb, container(text(under.clone()).font(theme::SANS).size(13.0).wrapping(text::Wrapping::None).color(ui::faded(INK))).width(Length::Fill).clip(true)].spacing(10).align_y(iced::Center).into()
            }
            None => container(text(under.clone()).font(theme::SANS).size(13.0).wrapping(text::Wrapping::None).color(ui::faded(tint.unwrap_or(MUTED)))).width(Length::Fill).clip(true).into(),
        };
        row![
            clock(ground, event.at, 46.0),
            container(row![lead, text(name).font(theme::SANS_SEMI).size(13.0).wrapping(text::Wrapping::None).color(ui::faded(INK))].spacing(9).align_y(iced::Center)).width(170.0).clip(true),
            container(map_cell).width(Length::Fill).clip(true),
            container(accuracy.map_or_else(|| Space::new().width(0.0).into(), |accuracy| ui::mono_small(ground.words.percent(f64::from(accuracy)), MUTED))).width(64.0).align_x(iced::alignment::Horizontal::Right),
            container(screen::mods(&mods)).width(78.0).align_x(iced::alignment::Horizontal::Right),
            tail,
        ]
        .spacing(12)
        .align_y(iced::Center)
        .into()
    } else {
        row![
            clock(ground, event.at, 40.0),
            lead,
            container(
                column![
                    text(name).font(theme::SANS_SEMI).size(12.5).wrapping(text::Wrapping::None).color(ui::faded(INK)),
                    text(under).font(theme::SANS).size(12.0).wrapping(text::Wrapping::None).color(ui::faded(if cover.is_some() { MUTED } else { tint.unwrap_or(MUTED) })),
                ]
                .spacing(1),
            )
            .width(Length::Fill)
            .clip(true),
            tail,
        ]
        .spacing(10)
        .align_y(iced::Center)
        .into()
    };
    let mut inside = column![container(line).height(high).center_y(high).padding([0, 10])];
    if let Some(details) = details {
        inside = inside.push(container(details).padding(Padding { top: 2.0, right: 10.0, bottom: 10.0, left: 50.0 }));
    }
    Some(button(inside).padding(0).width(Length::Fill).style(ui::button_faded(row_press(tint))).on_press(press).into())
}

fn news_card<'a>(ground: &Ground<'a>, event: &Event<'a>) -> Option<Element<'a, Message>> {
    let w = ground.words;
    let when = screen::since(ground, event.at);
    let head = |icon: Icon, colour: Color, source: String| -> Element<'a, Message> {
        row![glyph(icon, 13.0, colour), text(source).font(theme::SANS_SEMI).size(12.5).wrapping(text::Wrapping::None).color(ui::faded(INK)), ui::mono_small(when.clone(), MUTED)].spacing(8).align_y(iced::Center).into()
    };
    let (media, header, body, press): (Option<Element<'a, Message>>, Element<'a, Message>, Element<'a, Message>, Message) = match event.item {
        Item::Post(post) => {
            let media: Option<Element<'a, Message>> = match (post.image.as_deref(), post.videos.first()) {
                (Some(url), _) => ground.pictures.get(url).map(|handle| iced::widget::image(handle.clone()).content_fit(iced::ContentFit::Cover).width(Length::Fill).height(124.0).border_radius(10.0).opacity(ui::fade()).into()),
                (None, Some(video)) => Some(screen::video_tile(ground, video, true, 124.0)),
                (None, None) => None,
            };
            let spans: Vec<news::Span> = post.body.iter().take(2).filter_map(|block| match block {
                news::Block::Text(spans) | news::Block::Quote(spans) | news::Block::Item(spans) | news::Block::Heading(spans) => Some(spans.clone()),
                news::Block::Image(_) => None,
            }).flatten().collect();
            let words: Element<'a, Message> = if spans.is_empty() { text(ui::shortened(post.text.clone(), 200)).font(theme::SANS).size(12.5).color(ui::faded(INK)).into() } else { screen::rich(&spans, INK, 12.5) };
            (media, head(Icon::Send, LINK_BLUE, format!("@{}", post.channel)), container(words).max_height(88.0).clip(true).into(), Message::Read(Reading::Post(post.clone())))
        }
        Item::Story(story) => {
            let media = story.image.as_deref().and_then(|url| ground.pictures.get(url)).map(|handle| iced::widget::image(handle.clone()).content_fit(iced::ContentFit::Cover).width(Length::Fill).height(110.0).border_radius(10.0).opacity(ui::fade()).into());
            let body = column![
                text(story.title.clone()).font(theme::SANS_SEMI).size(13.5).color(ui::faded(INK)),
                text(ui::shortened(story.lead.clone(), 140)).font(theme::SANS).size(12.0).color(ui::faded(MUTED)),
            ]
            .spacing(4);
            (media, head(Icon::News, LINK_BLUE, "osu!".to_owned()), body.into(), Message::Read(Reading::Story(story.clone())))
        }
        Item::Build(build) => {
            let mut lines = column![].spacing(3);
            for (at, change) in build.changes.iter().take(3).enumerate() {
                lines = lines.push(text(format!("· {}", change.title)).font(theme::SANS).size(12.0).wrapping(text::Wrapping::None).color(ui::faded(if at == 0 || change.major { INK } else { MUTED })));
            }
            if build.changes.len() > 3 {
                lines = lines.push(ui::mono_small(w.n("more-changes-n", (build.changes.len() - 3) as u64), FAINT));
            }
            (None, head(Icon::Gear, PINK, format!("{} {}", build.stream, build.version)), container(lines).clip(true).into(), Message::Read(Reading::Build(build.clone())))
        }
        _ => return None,
    };
    let mut inside = column![].spacing(8);
    if let Some(media) = media {
        inside = inside.push(media);
    }
    inside = inside.push(header).push(body);
    Some(button(container(inside).padding(10)).padding(0).width(Length::Fill).style(ui::button_faded(surface(false))).on_press(press).into())
}

fn shelf_card<'a>(ground: &Ground<'a>, icon: Icon, colour: Color, label: String, who: Option<usize>, big: String, small: String, cover: Option<&'a iced::widget::image::Handle>, press: Message) -> Element<'a, Message> {
    let high = 146.0;
    let k = ui::fade();
    let mut lower = column![].spacing(5);
    if let Some(person) = who.and_then(|at| ground.catalog.people.get(at)) {
        lower = lower.push(row![screen::face(ground, person, 20.0), text(person.name.clone()).font(theme::SANS_SEMI).size(12.5).wrapping(text::Wrapping::None).color(ui::faded(INK))].spacing(7).align_y(iced::Center));
    }
    lower = lower
        .push(text(big).font(theme::SANS_SEMI).size(20.0).wrapping(text::Wrapping::None).color(ui::faded(INK)))
        .push(text(small).font(theme::SANS).size(11.5).wrapping(text::Wrapping::None).color(ui::faded(MUTED)));
    let inside = column![row![glyph(icon, 13.0, colour), ui::mono_small(label.to_uppercase(), colour)].spacing(7).align_y(iced::Center), ui::grow_tall(), container(lower).clip(true)]
        .height(high)
        .padding([12, 12]);
    button(stack![screen::backdrop(cover, high, 14.0, colour, false), inside].height(high))
        .padding(0)
        .width(Length::FillPortion(1))
        .style(ui::button_faded(move |_, status: button::Status| {
            let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: Some(Background::Color(Color::from_rgba(0.055, 0.025, 0.033, 0.96))),
                text_color: INK,
                border: Border { color: Color { a: if lit { 0.7 * k } else { 0.32 * k }, ..colour }, width: 1.0, radius: 14.0.into() },
                shadow: if lit { Shadow { color: Color { a: 0.25 * k, ..colour }, offset: Vector::ZERO, blur_radius: 18.0 } } else { Shadow::default() },
                snap: true,
            }
        }))
        .on_press(press)
        .into()
}

fn shelf<'a>(ground: &Ground<'a>, seen: &[Event<'a>]) -> Option<Element<'a, Message>> {
    let w = ground.words;
    let catalog = ground.catalog;
    let mut cards: Vec<Element<'a, Message>> = Vec::new();
    let happened = |want: fn(&Kind) -> bool| seen.iter().find_map(|event| match event.item {
        Item::Happened(happened) if want(&happened.kind) => Some(happened),
        _ => None,
    });
    if let Some(happened) = happened(|kind| matches!(kind, Kind::TopPlay { .. })) {
        if let (Kind::TopPlay { pp, place, .. }, Some(map)) = (&happened.kind, happened.map) {
            cards.push(shelf_card(ground, Icon::Star, Color::from_rgb(0.941, 0.408, 0.408), w.t("kind-top"), Some(happened.who), format!("{} pp", screen::decimal(w, *pp, 0)), format!("{} · {}", w.n("top-place", u64::from(*place)), map_title(ground, map)), map_cover(ground, map), Message::Person(Some(happened.who))));
        }
    }
    if let Some(happened) = happened(|kind| matches!(kind, Kind::Title(_))) {
        if let Some(title) = match &happened.kind {
            Kind::Title(code) => catalog.title_of(code),
            _ => None,
        } {
            let holders = catalog.holders(&title.code).len();
            cards.push(shelf_card(ground, Icon::Trophy, title.rarity.colour(), w.t("kind-title"), Some(happened.who), title.name(w.lang()).to_owned(), format!("{} · {} {}", w.t(title.rarity.key()), w.t("held-by"), w.of(holders as u64, catalog.people.len() as u64)), None, Message::Person(Some(happened.who))));
        }
    }
    if let Some(happened) = happened(|kind| matches!(kind, Kind::Climb { .. })) {
        if let Kind::Climb { board, from, to } = &happened.kind {
            cards.push(shelf_card(ground, Icon::Up, GREEN, w.t("kind-climb"), Some(happened.who), format!("#{to} · {}", w.t(board.short_key())), format!("{} #{from}", w.t("climb-was")), None, Message::Board(*board)));
        }
    }
    let story = seen.iter().find_map(|event| match event.item {
        Item::Post(post) => Some((first_line(&post.text), format!("@{} · {}", post.channel, screen::since(ground, post.at)), post_cover(ground, post), Message::Read(Reading::Post(post.clone())))),
        Item::Story(story) => Some((story.title.clone(), format!("osu! · {}", screen::since(ground, story.at)), story.image.as_deref().and_then(|url| ground.pictures.get(url)), Message::Read(Reading::Story(story.clone())))),
        _ => None,
    });
    if let Some((big, small, cover, press)) = story {
        cards.push(shelf_card(ground, Icon::Send, LINK_BLUE, w.t("kind-news"), None, big, small, cover, press));
    }
    if cards.is_empty() {
        return None;
    }
    Some(column![caption(w.t("highlights")), iced::widget::Row::with_children(cards).spacing(10)].spacing(8).into())
}

fn counted<'a, T: Copy + PartialEq + 'static>(ground: &Ground<'a>, options: &[T], chosen: T, key: fn(T) -> &'static str, count: impl Fn(T) -> usize, message: fn(T) -> Message) -> Element<'a, Message> {
    let w = ground.words;
    let mut pills: Vec<Element<'a, Message>> = Vec::new();
    for option in options.iter().copied() {
        let n = count(option);
        if n == 0 && option != chosen {
            continue;
        }
        pills.push(
            button(row![text(w.t(key(option))).font(theme::SANS_SEMI).size(12.0), ui::mono_small(n.to_string(), FAINT)].spacing(6).align_y(iced::Center))
                .padding([5, 11])
                .style(ui::button_faded(theme::pill(option == chosen)))
                .on_press(message(option))
                .into(),
        );
    }
    ui::wrap(pills, 6.0).into()
}

fn by_day<'a>(ground: &Ground<'a>, list: &[Event<'a>], draw: impl Fn(&Event<'a>) -> Option<Element<'a, Message>>) -> Vec<Element<'a, Message>> {
    let today = ground.now_unix - ground.now_unix.rem_euclid(86_400);
    let day_of = |at: i64| if at >= today { 0 } else { (today - at) / 86_400 + 1 };
    let mut last: Option<i64> = None;
    let mut out = Vec::new();
    for event in list {
        let day = day_of(event.at);
        if last != Some(day) {
            out.push(day_divider(ground, event.at));
            last = Some(day);
        }
        if let Some(made) = draw(event) {
            out.push(made);
        }
    }
    out
}

fn timeline<'a>(ground: &Ground<'a>, wide: f32) -> Element<'a, Message> {
    let w = ground.words;
    let all = events(ground.catalog, ground.news, ground.channels, ground.live_shown);
    let fresh = all.iter().filter(|event| event.at > ground.seen && ground.seen > 0).count();
    let seen: Vec<Event<'a>> = all.iter().copied().filter(|event| !(event.at > ground.seen && ground.seen > 0)).filter(|event| event.found(ground.catalog, ground.query)).collect();
    let group_all: Vec<Event<'a>> = seen.iter().copied().filter(Event::in_group).collect();
    let news_all: Vec<Event<'a>> = seen.iter().copied().filter(|event| !event.in_group()).collect();
    let group: Vec<Event<'a>> = group_all.iter().copied().filter(|event| event.admitted(ground.filter)).collect();
    let news: Vec<Event<'a>> = news_all.iter().copied().filter(|event| event.sourced(ground.source)).collect();

    let streams = screen::segmented(Stream::ALL.iter().map(|stream| (w.t(stream.key()), ground.stream == *stream, Message::Stream(*stream))).collect());
    let mut top = row![streams, ui::grow()].spacing(10).align_y(iced::Center);
    if fresh > 0 {
        top = top.push(
            button(row![glyph(Icon::Up, 12.0, Color::WHITE), text(w.n("fresh-events", fresh as u64)).font(theme::SANS_SEMI).size(12.0).color(Color::WHITE)].spacing(6).align_y(iced::Center))
                .padding([6, 12])
                .style(ui::button_faded(|_, status| button::Style {
                    background: Some(Background::Color(if matches!(status, button::Status::Hovered) { Color::from_rgb(0.925, 0.337, 0.337) } else { ACCENT })),
                    text_color: Color::WHITE,
                    border: Border { radius: 16.0.into(), ..Border::default() },
                    shadow: Shadow { color: Color::from_rgba(0.886, 0.282, 0.282, 0.35), offset: Vector::new(0.0, 4.0), blur_radius: 14.0 },
                    snap: true,
                }))
                .on_press(Message::Reveal),
        );
    }
    top = top.push(
        container(
            row![
                glyph(Icon::Search, 13.0, FAINT),
                iced::widget::text_input(&w.t("feed-search"), ground.query).on_input(Message::Search).size(12.0).padding(0).style(ui::bare_input).width(Length::Fill),
            ]
            .spacing(8)
            .align_y(iced::Center),
        )
        .padding([6, 10])
        .width(if wide < 560.0 { 150.0 } else { 200.0 })
        .style(|_| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.25 * ui::fade()))),
            border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.07 * ui::fade()), width: 1.0, radius: 8.0.into() },
            ..container::Style::default()
        }),
    );
    let mut page = column![top].spacing(12);
    if let Some(shelf) = shelf(ground, &seen) {
        page = page.push(shelf);
    }

    let group_counts = |filter: Filter| group_all.iter().filter(|event| event.admitted(filter)).count();
    let news_counts = |source: Source| news_all.iter().filter(|event| event.sourced(source)).count();
    let group_pills = counted(ground, &Filter::ALL, ground.filter, Filter::key, group_counts, Message::Filter);
    let news_pills = counted(ground, &Source::ALL, ground.source, Source::key, news_counts, Message::Source);
    let nothing = || -> Element<'a, Message> { container(ui::mono_small(w.t("nothing-yet"), FAINT)).center_x(Length::Fill).padding(28).into() };
    let heading = |icon: Icon, key: &str, n: usize| -> Element<'a, Message> {
        row![glyph(icon, 14.0, INK), text(w.t(key)).font(theme::SANS_SEMI).size(14.0).color(ui::faded(INK)), ui::mono_small(w.count("events", n as u64), MUTED)].spacing(8).align_y(iced::Center).into()
    };
    let group_panel = |wide_rows: bool| -> Element<'a, Message> {
        let mut rows = column![].spacing(2);
        for made in by_day(ground, &group, |event| journal_row(ground, event, wide_rows)) {
            rows = rows.push(made);
        }
        if group.is_empty() {
            rows = rows.push(nothing());
        }
        card(column![container(heading(Icon::Play, "stream-group", group_all.len())).padding([0, 6]), container(group_pills).padding([0, 6]), rows].spacing(10), [12, 8]).into()
    };
    let news_list = |columns: usize| -> Element<'a, Message> {
        let mut cards: Vec<Element<'a, Message>> = Vec::new();
        for event in &news {
            if let Some(made) = news_card(ground, event) {
                cards.push(made);
            }
        }
        if cards.is_empty() {
            return nothing();
        }
        if columns > 1 {
            screen::grid(cards, columns, 12.0)
        } else {
            iced::widget::Column::with_children(cards).spacing(10).into()
        }
    };
    let news_panel = |columns: usize| -> Element<'a, Message> { column![heading(Icon::News, "filter-news", news_all.len()), news_pills, news_list(columns)].spacing(10).into() };

    let body: Element<'a, Message> = match ground.stream {
        Stream::Group => group_panel(wide >= 640.0),
        Stream::News => news_panel(if wide >= 560.0 { 2 } else { 1 }),
        Stream::All if wide >= 620.0 => row![container(group_panel(false)).width(Length::FillPortion(3)), container(news_panel(1)).width(Length::FillPortion(2))].spacing(14).into(),
        Stream::All => column![group_panel(false), news_panel(1)].spacing(14).into(),
    };
    page = page.push(body);
    scrollable(container(page).padding(Padding { top: 2.0, right: 10.0, bottom: 28.0, left: 2.0 }))
        .id(iced::widget::Id::new("community-feed"))
        .style(ui::thin_scroll)
        .direction(ui::hidden_bar())
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

pub fn ring<'a>(ground: &Ground<'a>, you: &Person, side: f32, share: f32, band: f32) -> Element<'a, Message> {
    let inner = side - band * 2.0 - 6.0;
    let face = screen::face(ground, you, inner);
    stack![
        iced::widget::Canvas::new(Arc { share, band, alpha: ui::fade() }).width(side).height(side),
        container(face).width(side).height(side).center(side),
    ]
    .width(side)
    .height(side)
    .into()
}

pub struct Arc {
    pub share: f32,
    pub band: f32,
    pub alpha: f32,
}

impl<M> iced::widget::canvas::Program<M> for Arc {
    type State = ();

    fn draw(&self, _: &(), renderer: &iced::Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<iced::widget::canvas::Geometry> {
        use iced::widget::canvas::{Frame, Path, Stroke};
        let mut frame = Frame::new(renderer, bounds.size());
        let centre = iced::Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius = bounds.width.min(bounds.height) / 2.0 - self.band / 2.0 - 1.0;
        let a = self.alpha;
        frame.stroke(&Path::circle(centre, radius), Stroke::default().with_color(Color::from_rgba(0.243, 0.188, 0.204, a)).with_width(self.band));
        let arc = Path::new(|b| {
            b.arc(iced::widget::canvas::path::Arc {
                center: centre,
                radius,
                start_angle: iced::Radians(-std::f32::consts::FRAC_PI_2),
                end_angle: iced::Radians(-std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * self.share.clamp(0.0, 1.0)),
            });
        });
        frame.stroke(&arc, Stroke::default().with_color(Color::from_rgba(0.894, 0.298, 0.298, a)).with_width(self.band).with_line_cap(iced::widget::canvas::LineCap::Round));
        vec![frame.into_geometry()]
    }
}

fn figure<'a>(value: String, label: String, delta: Option<(String, bool)>, colour: Color) -> Element<'a, Message> {
    let mut under = row![ui::mono_small(label, FAINT)].spacing(4);
    if let Some((delta, up)) = delta {
        under = under.push(ui::mono_small(delta, if up { GREEN } else { ACCENT }));
    }
    column![text(value).font(theme::SANS_SEMI).size(15.0).wrapping(text::Wrapping::None).color(ui::faded(colour)), under].spacing(1).width(Length::FillPortion(1)).into()
}

fn me_card<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let Some(you) = ground.catalog.you() else {
        return card(ui::mono_small(w.t("nothing-yet"), FAINT), 16).into();
    };
    let card_data = ground.card.cloned().or_else(|| ground.catalog.card_of());
    let (level, share) = card_data.as_ref().map_or((you.level, 0.0), |c| (c.level as u32, (c.level_progress / 100.0) as f32));
    let title: Element<'a, Message> = screen::title_line(ground, you, 12.0);
    let country_rank = card_data.as_ref().map_or(0.0, |c| c.country_rank);
    let mut place: Vec<String> = Vec::new();
    if country_rank > 0.0 {
        place.push(format!("#{}", w.lang().group(country_rank as u64)));
    }
    if level > 0 {
        place.push(format!("{} {level}", w.t("level-short")));
    }
    let gained = |at: usize| you.gained[at];
    let signed = |value: f64, places: usize| -> Option<(String, bool)> {
        (value.abs() > 0.004).then(|| (format!("{}{}", if value > 0.0 { "+" } else { "−" }, screen::decimal(w, value.abs() as f32, places)), value > 0.0))
    };
    let figures = row![
        figure(w.lang().group(u64::from(you.pp)), w.t("board-pp"), signed(gained(0), 0), Color::from_rgb(0.941, 0.408, 0.408)),
        figure(screen::rank_of(w, you.rank), w.t("metric-world"), None, INK),
        figure(w.percent(f64::from(you.accuracy)), w.t("metric-accuracy-short"), signed(gained(1), 2), INK),
    ]
    .spacing(8);
    let k = ui::fade();
    let streak = container(
        row![
            glyph(Icon::Flame, 15.0, Color::from_rgb(0.941, 0.408, 0.408)),
            text(w.n("streak-card", u64::from(you.streak))).font(theme::SANS).size(12.0).color(ui::faded(INK)).width(Length::Fill),
            ui::mono_small(w.n("streak-best-n", u64::from(you.streak_best.max(you.streak))), MUTED),
        ]
        .spacing(8)
        .align_y(iced::Center),
    )
    .padding([8, 10])
    .style(move |_| container::Style {
        background: Some(Background::Color(Color::from_rgba(0.886, 0.282, 0.282, 0.08 * k))),
        border: Border { color: Color::from_rgba(0.886, 0.282, 0.282, 0.22 * k), width: 1.0, radius: 10.0.into() },
        ..container::Style::default()
    });
    let open = button(container(text(w.t("my-profile-open")).font(theme::SANS_SEMI).size(12.0)).center_x(Length::Fill))
        .padding([8, 0])
        .width(Length::Fill)
        .style(ui::button_faded(quiet))
        .on_press(Message::Section(Section::Profile));
    let who = row![
        ring(ground, you, 64.0, share, 3.5),
        column![
            text(you.name.clone()).font(theme::SANS_SEMI).size(17.0).wrapping(text::Wrapping::None).color(ui::faded(INK)),
            title,
            row![screen::flag(ground, &you.country, 11.0), ui::mono_small(place.join(" · "), MUTED)].spacing(6).align_y(iced::Center),
        ]
        .spacing(3),
    ]
    .spacing(12)
    .align_y(iced::Center);
    let mut inside = column![who, figures].spacing(12);
    if you.streak > 0 {
        inside = inside.push(streak);
    }
    card(inside.push(open), 16).into()
}

fn channels_card<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let status = match ground.channels.first() {
        Some(channel) => screen::source_state(ground, &news::channel_source(channel)),
        None => Space::new().width(0.0).into(),
    };
    card(column![row![caption(w.t("channels-head")), ui::grow(), status].align_y(iced::Center), screen::channel_tools(ground)].spacing(10), [14, 16]).into()
}

struct Spot {
    who: usize,
    why: &'static str,
    big: String,
    small: String,
    tint: Color,
}

fn spots(ground: &Ground<'_>) -> Vec<Spot> {
    let w = ground.words;
    let people = &ground.catalog.people;
    let mut out: Vec<Spot> = Vec::new();
    let mut taken: Vec<usize> = Vec::new();
    let mut push = |who: Option<usize>, why: &'static str, big: String, small: String, tint: Color, out: &mut Vec<Spot>| {
        if let Some(who) = who.filter(|who| !taken.contains(who)) {
            taken.push(who);
            out.push(Spot { who, why, big, small, tint });
        }
    };
    let coral = Color::from_rgb(0.941, 0.408, 0.408);
    let best = |key: fn(&Person) -> f64| (0..people.len()).max_by(|a, b| key(&people[*a]).total_cmp(&key(&people[*b])));
    if let Some(at) = best(|p| p.gained[0]).filter(|at| people[*at].gained[0] > 0.0) {
        let p = &people[at];
        push(Some(at), "spot-gain", format!("+{} pp", screen::decimal(w, p.gained[0] as f32, 0)), screen::pp_of(w, p.pp), GREEN, &mut out);
    }
    if let Some(at) = best(|p| f64::from(p.accuracy)) {
        let p = &people[at];
        push(Some(at), "spot-accuracy", w.percent(f64::from(p.accuracy)), screen::rank_of(w, p.rank), INK, &mut out);
    }
    if let Some(happened) = ground.catalog.feed.iter().find(|h| matches!(h.kind, Kind::Title(_))) {
        if let Kind::Title(code) = &happened.kind {
            if let Some(title) = ground.catalog.title_of(code) {
                push(Some(happened.who), "spot-title", title.name(w.lang()).to_owned(), screen::since(ground, happened.at), title.rarity.colour(), &mut out);
            }
        }
    }
    if let Some(at) = best(|p| f64::from(p.streak)).filter(|at| people[*at].streak > 0) {
        let p = &people[at];
        push(Some(at), "spot-streak", w.n("streak-card", u64::from(p.streak)), w.n("streak-best-n", u64::from(p.streak_best.max(p.streak))), coral, &mut out);
    }
    if let Some(at) = people.iter().position(|p| p.you) {
        let p = &people[at];
        let place = ground.catalog.ranked(Board::Pp).iter().position(|x| *x == at).map_or(0, |x| x + 1);
        push(Some(at), "spot-you", format!("#{place}"), screen::pp_of(w, p.pp), coral, &mut out);
    }
    if let Some(at) = best(|p| f64::from(p.pp)) {
        let p = &people[at];
        push(Some(at), "spot-top", screen::pp_of(w, p.pp), screen::rank_of(w, p.rank), theme::GRADE_S, &mut out);
    }
    out
}

fn dot_style(on: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: Some(Background::Color(if on {
            ACCENT
        } else if matches!(status, button::Status::Hovered) {
            Color::from_rgba(1.0, 1.0, 1.0, 0.12)
        } else {
            Color::from_rgba(1.0, 1.0, 1.0, 0.05)
        })),
        text_color: INK,
        border: Border { radius: 3.0.into(), ..Border::default() },
        shadow: Shadow::default(),
        snap: true,
    }
}

fn spotlight<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let list = spots(ground);
    if list.is_empty() {
        return Space::new().height(0.0).into();
    }
    let at = ground.spot % list.len();
    let spot = &list[at];
    let person = &ground.catalog.people[spot.who];
    let mut dots = row![].spacing(5).align_y(iced::Center);
    for index in 0..list.len() {
        dots = dots.push(button(Space::new().width(if index == at { 16.0 } else { 6.0 }).height(6.0)).padding(0).style(ui::button_faded(dot_style(index == at))).on_press(Message::Spot(index)));
    }
    let shown = ui::fading(ui::fade() * ground.spot_k, || -> Element<'a, Message> {
        let k = ui::fade();
        let colour = screen::avatar_colour(&person.name);
        let face = container(screen::face(ground, person, 46.0)).padding(3).style(move |_| container::Style {
            background: Some(Background::Color(Color { a: k, ..colour })),
            border: Border { radius: 26.0.into(), ..Border::default() },
            shadow: Shadow { color: Color { a: 0.5 * k, ..colour }, offset: Vector::ZERO, blur_radius: 16.0 },
            ..container::Style::default()
        });
        let named = column![
            row![
                container(text(person.name.clone()).font(theme::SANS_SEMI).size(17.0).wrapping(text::Wrapping::None).color(ui::faded(INK))).clip(true),
                screen::flag(ground, &person.country, 11.0),
            ]
            .spacing(7)
            .align_y(iced::Center),
            text(w.t(spot.why)).font(theme::SANS).size(12.0).color(ui::faded(MUTED)),
        ]
        .spacing(3)
        .width(Length::Fill);
        let figure = row![
            container(text(spot.big.clone()).font(theme::SANS_SEMI).size(24.0).wrapping(text::Wrapping::None).color(ui::faded(spot.tint))).clip(true),
            ui::grow(),
            ui::mono_small(spot.small.clone(), MUTED),
        ]
        .spacing(10)
        .align_y(iced::alignment::Vertical::Bottom);
        let lift = (1.0 - ground.spot_k) * 8.0;
        container(column![row![face, named].spacing(12).align_y(iced::Center), figure].spacing(10)).padding(Padding::ZERO.top(lift)).into()
    });
    let body = column![row![caption(w.t("spot-head")), ui::grow(), dots].align_y(iced::Center), container(shown).height(98.0).clip(true)].spacing(12);
    mouse_area(card(body, [14, 16])).on_enter(Message::SpotHold(true)).on_exit(Message::SpotHold(false)).on_press(Message::Person(Some(spot.who))).into()
}

fn friends_card<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let catalog = ground.catalog;
    let online = catalog.friends.iter().filter(|f| f.online).count();
    let mut list = column![].spacing(2);
    let mut order: Vec<&crate::community::Friend> = catalog.friends.iter().collect();
    order.sort_by_key(|f| (!f.online, f.minutes_away(ground.now_unix)));
    for friend in order.iter().take(4) {
        let k = ui::fade();
        let colour = if friend.online { GREEN } else { Color::from_rgb(0.29, 0.25, 0.25) };
        let dot = container(Space::new().width(9.0).height(9.0)).style(move |_| container::Style {
            background: Some(Background::Color(Color { a: k, ..colour })),
            border: Border { color: Color::from_rgba(0.08, 0.03, 0.035, k), width: 2.0, radius: 5.0.into() },
            ..container::Style::default()
        });
        let face = stack![screen::friend_face(ground, friend, 28.0), container(dot).width(28.0).height(28.0).align_x(iced::alignment::Horizontal::Right).align_y(iced::alignment::Vertical::Bottom)];
        let status = if friend.online { w.t("online") } else { screen::ago(w, friend.minutes_away(ground.now_unix)) };
        list = list.push(
            button(
                row![
                    face,
                    column![
                        row![text(friend.name.clone()).font(theme::SANS_SEMI).size(13.0).wrapping(text::Wrapping::None).color(ui::faded(INK)), screen::flag(ground, &friend.country, 10.0)].spacing(6).align_y(iced::Center),
                        text(status).font(theme::SANS).size(11.0).color(ui::faded(if friend.online { GREEN } else { FAINT })),
                    ]
                    .spacing(1)
                    .width(Length::Fill),
                    ui::mono_small(format!("{} pp", w.lang().group(u64::from(friend.pp))), MUTED),
                ]
                .spacing(10)
                .align_y(iced::Center),
            )
            .padding([5, 6])
            .width(Length::Fill)
            .style(ui::button_faded(theme::row(false)))
            .on_press(Message::Open(format!("https://osu.ppy.sh/users/{}", friend.name))),
        );
    }
    let said = screen::friends_said(ground);
    let body: Element<'a, Message> = match said {
        Some(said) => container(said).height(80.0).into(),
        None => list.into(),
    };
    card(
        column![row![caption(w.t("friends-head")), ui::grow(), ui::mono_small(w.n("friends-online", online as u64), GREEN)].align_y(iced::Center), body].spacing(8),
        [14, 16],
    )
    .into()
}

fn tab_style(on: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let lit = matches!(status, button::Status::Hovered);
        button::Style {
            background: Some(Background::Color(if on { Color::from_rgba(0.886, 0.282, 0.282, 0.14) } else if lit { Color::from_rgba(1.0, 1.0, 1.0, 0.02) } else { Color::TRANSPARENT })),
            text_color: if on || lit { INK } else { FAINT },
            border: Border { color: if on { Color::from_rgba(0.886, 0.282, 0.282, 0.45) } else { Color::TRANSPARENT }, width: 1.0, radius: 6.0.into() },
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

fn leaderboard<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let catalog = ground.catalog;
    let chosen = ground.rank % Board::ALL.len();
    let board = Board::ALL[chosen];
    let mut tabs = row![].spacing(2);
    for (index, each) in Board::ALL.iter().enumerate() {
        tabs = tabs.push(
            button(container(text(w.t(each.short_key())).font(theme::SANS_SEMI).size(11.0).wrapping(text::Wrapping::None)).center_x(Length::Fill))
                .padding([4, 0])
                .width(Length::FillPortion(1))
                .style(ui::button_faded(tab_style(index == chosen)))
                .on_press(Message::Rank(index)),
        );
    }
    let k = ui::fade();
    let tabs = container(tabs).padding(2).style(move |_| container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.22 * k))),
        border: Border { radius: 8.0.into(), ..Border::default() },
        ..container::Style::default()
    });
    let order = catalog.ranked(board);
    let top = order.first().map_or(1.0, |at| board.value(&catalog.people[*at])).max(f64::EPSILON);
    let floor = match board {
        Board::Accuracy => order.last().map_or(0.0, |at| board.value(&catalog.people[*at])) - 1.0,
        _ => 0.0,
    };
    let mut rows = column![].spacing(2);
    let mut shown: Vec<usize> = order.iter().copied().take(5).collect();
    if let Some(you) = order.iter().position(|at| catalog.people[*at].you).filter(|place| *place >= 5) {
        shown.push(order[you]);
    }
    for at in shown {
        let person = &catalog.people[at];
        let place = order.iter().position(|x| *x == at).map_or(0, |x| x + 1);
        let share = (((board.value(person) - floor) / (top - floor)).clamp(0.04, 1.0) as f32) * ground.rank_k;
        let you = person.you;
        let bar_colour = if you { ACCENT } else { Color::from_rgba(0.925, 0.906, 0.886, 0.55) };
        let filled = (share * 1000.0).round().max(1.0) as u16;
        let bar = row![
            container(Space::new().height(3.0)).width(Length::FillPortion(filled)).style(move |_| container::Style { background: Some(Background::Color(Color { a: bar_colour.a * k, ..bar_colour })), border: Border { radius: 2.0.into(), ..Border::default() }, ..container::Style::default() }),
            Space::new().width(Length::FillPortion(1000u16.saturating_sub(filled).max(1))).height(3.0),
        ];
        let place_colour = match place {
            1 => theme::GRADE_S,
            2 => Color::from_rgb8(200, 204, 220),
            3 => Color::from_rgb8(205, 127, 50),
            _ => MUTED,
        };
        rows = rows.push(
            button(
                column![
                    row![
                        container(text(format!("{place}")).font(theme::MONO_BOLD).size(11.0).color(ui::faded(place_colour))).width(16.0).align_x(iced::alignment::Horizontal::Right),
                        screen::face(ground, person, 22.0),
                        container(text(person.name.clone()).font(theme::SANS_SEMI).size(13.0).wrapping(text::Wrapping::None).color(ui::faded(if you { Color::from_rgb(0.941, 0.408, 0.408) } else { INK }))).width(Length::Fill).clip(true),
                        screen::moved(person.moved[board.index()]),
                        ui::mono_small(screen::shown_value(w, board, person), INK),
                    ]
                    .spacing(8)
                    .align_y(iced::Center),
                    container(bar).padding(Padding::ZERO.left(54.0)),
                ]
                .spacing(5),
            )
            .padding([5, 6])
            .width(Length::Fill)
            .style(ui::button_faded(theme::row(you)))
            .on_press(Message::Person(Some(at))),
        );
    }
    let drain = Drain { started: ground.rank_started, length: RANK_EVERY, held: ground.rank_held, alpha: ui::fade() };
    let body = column![
        row![caption(w.t(board.key())), ui::grow(), ui::mono_small(w.n("week-short", u64::from(catalog.week)), FAINT)].align_y(iced::Center),
        tabs,
        Element::from(drain),
        rows,
        button(text(w.t("all-boards")).font(theme::SANS_SEMI).size(12.0).color(ui::faded(ACCENT))).padding([4, 6]).style(ui::button_faded(theme::bare)).on_press(Message::Section(Section::Boards)),
    ]
    .spacing(8);
    mouse_area(card(body, [14, 16])).on_enter(Message::RankHold(true)).on_exit(Message::RankHold(false)).into()
}

pub fn view<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let room = ground.width - 80.0;
    let wide_enough = room >= LEFT_WIDE + RIGHT_WIDE + 520.0 + GAP * 2.0;
    let medium = room >= RIGHT_WIDE + 460.0 + GAP;
    let right_parts = |me_first: bool| -> Element<'a, Message> {
        let mut side = column![].spacing(12);
        if me_first {
            side = side.push(me_card(ground));
        }
        side = side.push(spotlight(ground)).push(friends_card(ground)).push(leaderboard(ground));
        if me_first {
            side = side.push(channels_card(ground));
        }
        scrollable(container(side).padding(Padding { top: 2.0, right: 8.0, bottom: 28.0, left: 0.0 })).style(ui::thin_scroll).direction(ui::hidden_bar()).height(Length::Fill).into()
    };
    let content: Element<'a, Message> = if wide_enough {
        let left_wide = (room * 0.2).clamp(LEFT_WIDE, 360.0);
        let right_wide = (room * 0.23).clamp(RIGHT_WIDE, 400.0);
        let middle = (room - left_wide - right_wide - GAP * 2.0).min(CENTRE_MOST);
        let left = scrollable(container(column![me_card(ground), channels_card(ground)].spacing(12)).padding(Padding { top: 2.0, right: 6.0, bottom: 28.0, left: 0.0 })).style(ui::thin_scroll).direction(ui::hidden_bar()).height(Length::Fill);
        container(
            row![
                container(left).width(left_wide).height(Length::Fill),
                container(timeline(ground, middle)).width(middle).height(Length::Fill),
                container(right_parts(false)).width(right_wide).height(Length::Fill),
            ]
            .spacing(GAP),
        )
        .center_x(Length::Fill)
        .into()
    } else if medium {
        let right_wide = (room * 0.3).clamp(RIGHT_WIDE, 380.0);
        row![container(timeline(ground, room - right_wide - GAP)).width(Length::Fill).height(Length::Fill), container(right_parts(true)).width(right_wide).height(Length::Fill)].spacing(GAP).into()
    } else {
        timeline(ground, room)
    };
    container(content).padding(Padding { top: 12.0, right: 40.0, bottom: 0.0, left: 40.0 }).width(Length::Fill).height(Length::Fill).into()
}

pub struct Drain {
    pub started: Instant,
    pub length: Duration,
    pub held: Option<f32>,
    pub alpha: f32,
}

#[derive(Default)]
struct DrainState {
    now: Option<Instant>,
}

impl<M> Widget<M, Theme, iced::Renderer> for Drain {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<DrainState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(DrainState::default())
    }

    fn size(&self) -> Size<Length> {
        Size { width: Length::Fill, height: Length::Fixed(2.0) }
    }

    fn layout(&mut self, _: &mut Tree, _: &iced::Renderer, limits: &layout::Limits) -> Node {
        Node::new(Size::new(limits.max().width, 2.0))
    }

    fn update(&mut self, tree: &mut Tree, event: &iced::Event, _: Layout<'_>, _: mouse::Cursor, _: &iced::Renderer, _: &mut dyn Clipboard, shell: &mut Shell<'_, M>, _: &Rectangle) {
        if let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            tree.state.downcast_mut::<DrainState>().now = Some(*now);
            if self.held.is_none() {
                shell.request_redraw_at(iced::window::RedrawRequest::At(*now + Duration::from_millis(33)));
            }
        }
    }

    fn draw(&self, tree: &Tree, renderer: &mut iced::Renderer, _: &Theme, _: &renderer::Style, layout: Layout<'_>, _: mouse::Cursor, _: &Rectangle) {
        use iced::advanced::Renderer as _;
        let bounds = layout.bounds();
        let now = tree.state.downcast_ref::<DrainState>().now.unwrap_or(self.started);
        let spent = now.saturating_duration_since(self.started).as_secs_f32() / self.length.as_secs_f32();
        let left = self.held.unwrap_or((1.0 - spent).clamp(0.0, 1.0));
        renderer.fill_quad(renderer::Quad { bounds, border: Border { radius: 1.0.into(), ..Border::default() }, ..renderer::Quad::default() }, Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.012 * self.alpha)));
        renderer.fill_quad(
            renderer::Quad { bounds: Rectangle { width: bounds.width * left, ..bounds }, border: Border { radius: 1.0.into(), ..Border::default() }, ..renderer::Quad::default() },
            Background::Color(Color { a: self.alpha, ..ACCENT }),
        );
    }
}

impl<'a, M: 'a> From<Drain> for Element<'a, M> {
    fn from(drain: Drain) -> Element<'a, M> {
        Element::new(drain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_filter_lets_through_only_its_own_events() {
        let catalog = Catalog::staged(Vec::new(), "", 1_790_000_000);
        let news = News::default();
        let all = events(&catalog, &news, &[], catalog.live.len());
        assert!(all.windows(2).all(|pair| pair[0].at >= pair[1].at), "newest first");
        let titles = all.iter().filter(|event| event.admitted(Filter::Titles)).count();
        let tops = all.iter().filter(|event| event.admitted(Filter::Top)).count();
        let plays = all.iter().filter(|event| event.admitted(Filter::Plays)).count();
        assert_eq!(titles, 3);
        assert_eq!(tops, 2);
        assert!(plays > tops, "plays take in the top plays too");
        assert_eq!(all.iter().filter(|event| event.admitted(Filter::All)).count(), all.len());
    }

    #[test]
    fn the_search_keeps_only_what_names_it() {
        let catalog = Catalog::staged(Vec::new(), "", 1_790_000_000);
        let news = News::default();
        let all = events(&catalog, &news, &[], catalog.live.len());
        let found: Vec<&Event> = all.iter().filter(|event| event.found(&catalog, "KOTO")).collect();
        assert!(!found.is_empty());
        assert!(found.iter().all(|event| match event.item {
            Item::Play(play) => catalog.people[play.who].name == "kotofey",
            Item::Happened(happened) => catalog.people[happened.who].name == "kotofey",
            _ => false,
        }));
        assert_eq!(all.iter().filter(|event| event.found(&catalog, "  ")).count(), all.len());
    }

    #[test]
    fn news_splits_by_where_it_came_from() {
        let catalog = Catalog::staged(Vec::new(), "", 1_790_000_000);
        let news = News {
            stories: vec![news::Story { title: "Recap".into(), at: 1_789_990_000, ..news::Story::default() }],
            posts: vec![news::Post { channel: "osunewsru".into(), text: "FC".into(), url: "https://t.me/osunewsru/1".into(), at: 1_789_991_000, ..news::Post::default() }],
            builds: vec![news::Build { stream: "Lazer".into(), version: "2026.921.0".into(), at: 1_789_992_000, ..news::Build::default() }],
            ..News::default()
        };
        let all = events(&catalog, &news, &["osunewsru".to_owned()], catalog.live.len());
        let count = |source: Source| all.iter().filter(|event| event.sourced(source)).count();
        assert_eq!((count(Source::All), count(Source::Osu), count(Source::Channels), count(Source::Builds)), (3, 1, 1, 1));
        assert!(all.iter().filter(|event| !event.in_group()).all(|event| !event.admitted(Filter::All)), "news never enters the group's journal");
    }

    #[test]
    fn keys_tell_events_apart() {
        let catalog = Catalog::staged(Vec::new(), "", 1_790_000_000);
        let news = News::default();
        let all = events(&catalog, &news, &[], catalog.live.len());
        let mut keys: Vec<String> = all.iter().map(Event::key).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), all.len());
    }
}
