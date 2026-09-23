use std::collections::HashMap;

use iced::widget::{button, column, container, image, mouse_area, row, scrollable, stack, text, text_input, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Shadow, Theme};

use crate::community::{Board, Catalog, Happening, Kind, Person, Rarity, Title, TITLES};
use crate::lang::Words;
use crate::news::{self, News};
use crate::theme::{self, ACCENT, FAINT, INK, MUTED};
use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Feed,
    People,
    Boards,
    Titles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Panel {
    Live,
    Updates,
    News,
    Reddit,
    Channels,
    Group,
}

impl Panel {
    pub const ALL: [Panel; 6] = [Panel::Live, Panel::Updates, Panel::News, Panel::Reddit, Panel::Channels, Panel::Group];

    pub fn tag(self) -> &'static str {
        match self {
            Panel::Live => "live",
            Panel::Updates => "updates",
            Panel::News => "news",
            Panel::Reddit => "reddit",
            Panel::Channels => "channels",
            Panel::Group => "group",
        }
    }

    pub fn of(tag: &str) -> Option<Panel> {
        Panel::ALL.into_iter().find(|panel| panel.tag() == tag)
    }
}

pub fn panels(kept: &[String]) -> Vec<Panel> {
    let mut out: Vec<Panel> = kept.iter().filter_map(|tag| Panel::of(tag)).collect();
    out.dedup();
    for panel in Panel::ALL {
        if !out.contains(&panel) {
            out.push(panel);
        }
    }
    out
}

pub fn panel_moved(kept: &[String], what: Panel, before: Option<Panel>) -> Vec<String> {
    let mut list = panels(kept);
    list.retain(|panel| *panel != what);
    let at = before.and_then(|edge| list.iter().position(|panel| *panel == edge)).unwrap_or(list.len());
    list.insert(at.min(list.len()), what);
    list.iter().map(|panel| panel.tag().to_owned()).collect()
}

#[derive(Debug, Clone)]
pub enum Message {
    Section(Section),
    Board(Board),
    Person(Option<usize>),
    Open(String),
    Refresh,
    ChannelDraft(String),
    ChannelAdd,
    ChannelRemove(String),
    Moved(Panel, Option<Panel>),
}

pub struct Ground<'a> {
    pub words: &'a Words,
    pub catalog: &'a Catalog,
    pub thumbs: &'a HashMap<String, image::Handle>,
    pub section: Section,
    pub board: Board,
    pub person: Option<usize>,
    pub now_unix: i64,
    pub news: &'a News,
    pub pictures: &'a HashMap<String, image::Handle>,
    pub loading: &'a std::collections::HashSet<String>,
    pub failed: &'a std::collections::HashSet<String>,
    pub channels: &'a [String],
    pub channel_draft: &'a str,
    pub panels: Vec<Panel>,
    pub live_shown: usize,
    pub live_k: f32,
}

const FEED_WIDE: f32 = 760.0;
const PANEL_WIDE: f32 = 445.0;
const PANEL_HIGH: f32 = 336.0;
const LIVE_ROW: f32 = 40.0;
const LIVE_SHOWN: usize = 6;
const CARD_WIDE: f32 = 290.0;
const DRAWER_WIDE: f32 = 400.0;
const GRID_GAP: f32 = 10.0;

pub fn view<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let word = |key: &str, section: Section| {
        let on = ground.section == section;
        let k = ui::fade();
        let bar = container(Space::new().height(2.0)).width(Length::Fill).style(move |_| container::Style {
            background: Some(Background::Color(ui::dim(if on { ACCENT } else { Color::TRANSPARENT }, k))),
            border: Border { radius: 1.0.into(), ..Border::default() },
            ..container::Style::default()
        });
        button(column![text(w.t(key)).font(theme::SANS_SEMI).size(theme::BODY), bar].spacing(5).width(Length::Shrink))
            .padding([4, 0])
            .style(ui::button_faded(theme::word(on)))
            .on_press(Message::Section(section))
    };
    let switch = row![
        word("community-feed", Section::Feed),
        word("community-people", Section::People),
        word("community-boards", Section::Boards),
        word("community-titles", Section::Titles),
    ]
    .spacing(28);
    let note = ui::mono_small(
        format!("{} · {} · {}", ground.catalog.group, w.count("players", ground.catalog.people.len() as u64), w.t("community-staged")),
        FAINT,
    );
    let head = column![container(switch).center_x(Length::Fill), container(note).center_x(Length::Fill)].spacing(10);
    let body = match ground.section {
        Section::Feed => feed(ground),
        Section::People => people(ground),
        Section::Boards => boards(ground),
        Section::Titles => titles(ground),
    };
    let page = column![container(head).padding(Padding::ZERO.top(22.0)), body].width(Length::Fill).height(Length::Fill);
    match ground.person.and_then(|at| ground.catalog.people.get(at).map(|person| (at, person))) {
        Some((_, person)) => stack![page, drawer(ground, person)].width(Length::Fill).height(Length::Fill).into(),
        None => page.into(),
    }
}

fn rolled<'a>(inside: Element<'a, Message>) -> Element<'a, Message> {
    scrollable(container(inside).center_x(Length::Fill).padding(Padding { top: 12.0, right: 40.0, bottom: 28.0, left: 40.0 }))
        .style(ui::thin_scroll)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn picture<'a>(ground: &Ground<'a>, map: Option<usize>, wide: f32, high: f32) -> Element<'a, Message> {
    match ground.catalog.map(map).and_then(|map| ground.thumbs.get(&map.hash)) {
        Some(handle) => image(handle.clone())
            .content_fit(iced::ContentFit::Cover)
            .width(wide)
            .height(high)
            .border_radius(6.0)
            .opacity(ui::fade())
            .into(),
        None => container(ui::fine_hatch()).width(wide).height(high).into(),
    }
}

fn seal<'a>(words: String, colour: Color, wide: f32, high: f32, size: f32) -> Element<'a, Message> {
    let k = ui::fade();
    container(text(words).font(theme::MONO_BOLD).size(size).color(ui::faded(colour)))
        .width(wide)
        .height(high)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color { a: 0.12 * k, ..colour })),
            border: Border { color: Color { a: 0.35 * k, ..colour }, width: 1.0, radius: 6.0.into() },
            ..container::Style::default()
        })
        .into()
}

fn mods<'a>(list: &[&'static str]) -> Element<'a, Message> {
    let mut shown = row![].spacing(4).align_y(iced::Center);
    for acronym in list {
        shown = shown.push(crate::main_screen::mod_badge(acronym));
    }
    shown.into()
}

fn title_line<'a>(ground: &Ground<'a>, person: &Person, size: f32) -> Element<'a, Message> {
    match person.shown_title() {
        Some(title) => text(title.name(ground.words.lang())).font(theme::SANS).size(size).wrapping(text::Wrapping::None).color(ui::faded(title.rarity.colour())).into(),
        None => text(ground.words.t("no-title")).font(theme::SANS).size(size).wrapping(text::Wrapping::None).color(ui::faded(FAINT)).into(),
    }
}

fn pp_of(words: &Words, pp: u32) -> String {
    format!("{} pp", words.lang().group(u64::from(pp)))
}

fn decimal(words: &Words, value: f32, places: usize) -> String {
    let made = format!("{value:.places$}");
    match words.lang() {
        crate::lang::Lang::En => made,
        crate::lang::Lang::Ru => made.replace('.', ","),
    }
}

fn shown_value(words: &Words, board: Board, person: &Person) -> String {
    match board {
        Board::Pp => pp_of(words, person.pp),
        Board::Accuracy => words.percent(f64::from(person.accuracy)),
        Board::Plays => words.lang().group(u64::from(person.plays)),
        Board::Hours => format!("{} {}", words.lang().group(u64::from(person.hours)), words.t("hours-short")),
        Board::Score => words.lang().group(person.score),
        Board::HitsPerPlay => decimal(words, person.hits_per_play, 1),
    }
}

fn moved<'a>(by: i32) -> Element<'a, Message> {
    match by {
        0 => ui::mono_small("—".to_owned(), FAINT),
        up if up > 0 => ui::mono_small(format!("▲{up}"), theme::HIT_100),
        down => ui::mono_small(format!("▼{}", -down), ACCENT),
    }
}

fn feed<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let pieces: Vec<(Panel, Element<'a, Message>)> = ground
        .panels
        .iter()
        .map(|panel| {
            let made = match panel {
                Panel::Live => live_panel(ground),
                Panel::Updates => updates_panel(ground),
                Panel::News => news_panel(ground),
                Panel::Reddit => reddit_panel(ground),
                Panel::Channels => channels_panel(ground),
                Panel::Group => group_panel(ground),
            };
            (*panel, made)
        })
        .collect();
    let board = crate::board::board(pieces, GRID_GAP, Message::Moved).solid(theme::SLAB_SOLID);
    let rolled = scrollable(container(container(board).max_width(PANEL_WIDE * 2.0 + GRID_GAP)).center_x(Length::Fill).padding(Padding { top: 12.0, right: 40.0, bottom: 28.0, left: 40.0 }))
        .id(iced::widget::Id::new("community-feed"))
        .style(ui::thin_scroll)
        .width(Length::Fill)
        .height(Length::Fill);
    crate::glide::edged(rolled, iced::widget::Id::new("community-feed")).into()
}

fn panel<'a>(title: String, status: Element<'a, Message>, body: Element<'a, Message>) -> Element<'a, Message> {
    let head = row![ui::mono_small(title.to_uppercase(), FAINT), ui::grow(), status].align_y(iced::Center).height(20.0);
    container(column![head, container(body).width(Length::Fill).height(Length::Fill).clip(true)].spacing(10))
        .padding([14, 16])
        .width(PANEL_WIDE)
        .height(PANEL_HIGH)
        .style(ui::box_faded(theme::slab))
        .into()
}

fn source_state<'a>(ground: &Ground<'a>, source: &str) -> Element<'a, Message> {
    let w = ground.words;
    let quiet = |words: String, colour: Color| -> Element<'a, Message> {
        button(ui::mono_small(words, colour)).padding(0).style(ui::button_faded(theme::bare)).on_press(Message::Refresh).into()
    };
    if ground.loading.contains(source) {
        return ui::mono_small(w.t("news-loading"), FAINT);
    }
    if ground.failed.contains(source) {
        return quiet(w.t("news-failed"), ACCENT);
    }
    match ground.news.fetched_at(source) {
        Some(at) => quiet(format!("{} {}", w.t("news-updated"), w.clock(at)), FAINT),
        None => Space::new().width(0.0).into(),
    }
}

fn staged_mark<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let k = ui::fade();
    let dot = container(Space::new().width(6.0).height(6.0)).style(move |_| container::Style {
        background: Some(Background::Color(Color { a: k, ..ACCENT })),
        border: Border { radius: 3.0.into(), ..Border::default() },
        ..container::Style::default()
    });
    row![dot, ui::mono_small(ground.words.t("community-staged"), FAINT)].spacing(6).align_y(iced::Center).into()
}

fn linked<'a>(inside: Element<'a, Message>, url: &str) -> Element<'a, Message> {
    button(inside).padding([4, 6]).width(Length::Fill).style(ui::button_faded(theme::row(false))).on_press(Message::Open(url.to_owned())).into()
}

fn empty<'a>(words: String) -> Element<'a, Message> {
    container(ui::mono_small(words, FAINT)).width(Length::Fill).height(Length::Fill).center(Length::Fill).into()
}

fn thumb<'a>(ground: &Ground<'a>, url: Option<&str>, wide: f32, high: f32) -> Element<'a, Message> {
    match url.and_then(|url| ground.pictures.get(url)) {
        Some(handle) => image(handle.clone()).content_fit(iced::ContentFit::Cover).width(wide).height(high).border_radius(6.0).opacity(ui::fade()).into(),
        None => container(ui::fine_hatch()).width(wide).height(high).into(),
    }
}

fn when<'a>(ground: &Ground<'a>, at: i64) -> String {
    let w = ground.words;
    let day = w.day(at, ground.now_unix);
    if ground.now_unix - at < 86_400 && ground.now_unix >= at {
        w.clock(at)
    } else {
        day
    }
}

fn live_panel<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let pool = &ground.catalog.live;
    let shown = ground.live_shown.min(pool.len());
    let mut rows = column![].spacing(0);
    for (place, play) in pool[..shown].iter().rev().take(LIVE_SHOWN).enumerate() {
        let person = &ground.catalog.people[play.who];
        let line = ground.catalog.maps.get(play.map).map(|map| map.line.clone()).unwrap_or_default();
        let grade = match play.grade {
            "SS" => theme::GRADE_SS,
            "S" => theme::GRADE_S,
            "A" => theme::GRADE_A,
            "B" => theme::GRADE_B,
            _ => theme::GRADE_C,
        };
        let ago = if place == 0 { w.t("live-now") } else { w.n("minutes-ago", place as u64 * 3) };
        let k = if place == 0 { ground.live_k.clamp(0.0, 1.0) } else { 1.0 };
        let made = ui::fading(ui::fade() * k, || -> Element<'a, Message> {
            row![
                ui::disc(&person.initial(), false, 26.0),
                column![
                    row![
                        text(person.name.clone()).font(theme::SANS_SEMI).size(theme::CAPTION + 1.0).wrapping(text::Wrapping::None).color(ui::faded(INK)),
                        ui::mono_small(ago, FAINT),
                    ]
                    .spacing(8)
                    .align_y(iced::Center),
                    text(ui::shortened(line, 38)).font(theme::SANS).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(MUTED)),
                ]
                .spacing(1)
                .width(Length::Fill),
                column![
                    row![mods(&play.mods), ui::mono(format!("{} pp", decimal(w, play.pp, 0)), INK)].spacing(6).align_y(iced::Center),
                    row![ui::mono_small(w.percent(f64::from(play.accuracy)), MUTED), text(play.grade).font(theme::MONO_BOLD).size(11.0).color(ui::faded(grade))].spacing(6),
                ]
                .spacing(1)
                .align_x(iced::alignment::Horizontal::Right),
            ]
            .spacing(10)
            .align_y(iced::Center)
            .into()
        });
        rows = rows.push(container(made).height(LIVE_ROW * k).clip(true).center_y(LIVE_ROW * k));
    }
    panel(w.t("panel-live"), staged_mark(ground), rows.into())
}

fn stream_colour(stream: &str) -> Color {
    match stream.to_ascii_lowercase().as_str() {
        s if s.contains("lazer") => Color::from_rgb8(0xff, 0x66, 0xab),
        s if s.contains("tachyon") => Color::from_rgb8(0xb0, 0x6c, 0xe8),
        s if s.contains("stable") || s.contains("cutting") || s.contains("beta") => Color::from_rgb8(0x58, 0xae, 0xfc),
        _ => MUTED,
    }
}

fn updates_panel<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let builds = &ground.news.builds;
    let body: Element<'a, Message> = if builds.is_empty() {
        empty(w.t("nothing-yet"))
    } else {
        let mut list = column![].spacing(2);
        for build in builds.iter().take(3) {
            let colour = stream_colour(&build.stream);
            let k = ui::fade();
            let pill = container(text(build.stream.clone()).font(theme::MONO_BOLD).size(10.0).color(ui::faded(colour))).padding([1, 6]).style(move |_| container::Style {
                background: Some(Background::Color(Color { a: 0.14 * k, ..colour })),
                border: Border { radius: 4.0.into(), ..Border::default() },
                ..container::Style::default()
            });
            let mut lines = column![row![
                pill,
                ui::mono(build.version.clone(), INK),
                ui::grow(),
                ui::mono_small(when(ground, build.at), FAINT),
            ]
            .spacing(8)
            .align_y(iced::Center)]
            .spacing(3);
            for change in build.changes.iter().take(2) {
                lines = lines.push(
                    text(ui::shortened(format!("· {}", change.title), 56))
                        .font(theme::SANS)
                        .size(11.5)
                        .wrapping(text::Wrapping::None)
                        .color(ui::faded(if change.major { INK } else { MUTED })),
                );
            }
            if build.changes.len() > 2 {
                lines = lines.push(ui::mono_small(w.n("more-changes", (build.changes.len() - 2) as u64), FAINT));
            }
            list = list.push(linked(lines.into(), &build.url));
        }
        list.into()
    };
    panel(w.t("panel-updates"), source_state(ground, news::UPDATES), body)
}

fn news_panel<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let stories = &ground.news.stories;
    let body: Element<'a, Message> = if stories.is_empty() {
        empty(w.t("nothing-yet"))
    } else {
        let mut list = column![].spacing(2);
        for story in stories.iter().take(3) {
            let inside = row![
                thumb(ground, story.image.as_deref(), 96.0, 54.0),
                column![
                    text(ui::shortened(story.title.clone(), 46)).font(theme::SANS_SEMI).size(theme::CAPTION + 1.0).wrapping(text::Wrapping::None).color(ui::faded(INK)),
                    text(ui::shortened(story.lead.clone(), 52)).font(theme::SANS).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(MUTED)),
                    ui::mono_small(when(ground, story.at), FAINT),
                ]
                .spacing(2)
                .width(Length::Fill),
            ]
            .spacing(12)
            .align_y(iced::Center);
            list = list.push(linked(inside.into(), &story.url));
        }
        list.into()
    };
    panel(w.t("panel-news"), source_state(ground, news::STORIES), body)
}

fn reddit_panel<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let threads = &ground.news.threads;
    let body: Element<'a, Message> = if threads.is_empty() {
        empty(w.t(if ground.failed.contains(news::THREADS) { "news-unreachable" } else { "nothing-yet" }))
    } else {
        let mut list = column![].spacing(0);
        for thread in threads.iter().take(6) {
            let inside = column![
                text(ui::shortened(thread.title.clone(), 58)).font(theme::SANS).size(theme::CAPTION + 1.0).wrapping(text::Wrapping::None).color(ui::faded(INK)),
                ui::mono_small(format!("u/{} · {}", thread.author, when(ground, thread.at)), FAINT),
            ]
            .spacing(1);
            list = list.push(linked(inside.into(), &thread.url));
        }
        list.into()
    };
    panel(w.t("panel-reddit"), source_state(ground, news::THREADS), body)
}

fn channels_panel<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let k = ui::fade();
    let mut chips = row![].spacing(6).align_y(iced::Center);
    for channel in ground.channels {
        let chip = row![
            ui::mono_small(format!("@{channel}"), MUTED),
            button(text("✕").size(10.0).color(ui::faded(FAINT))).padding([0, 2]).style(ui::button_faded(theme::bare)).on_press(Message::ChannelRemove(channel.clone())),
        ]
        .spacing(4)
        .align_y(iced::Center);
        chips = chips.push(container(chip).padding([2, 8]).style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05 * k))),
            border: Border { radius: 10.0.into(), ..Border::default() },
            ..container::Style::default()
        }));
    }
    let adding = row![
        text_input(&w.t("channel-hint"), ground.channel_draft)
            .on_input(Message::ChannelDraft)
            .on_submit(Message::ChannelAdd)
            .size(12.0)
            .padding([4, 8])
            .width(Length::Fill),
        button(text(w.t("channel-add")).font(theme::SANS_SEMI).size(theme::CAPTION)).padding([5, 10]).style(ui::button_faded(theme::pill(false))).on_press(Message::ChannelAdd),
    ]
    .spacing(6)
    .align_y(iced::Center);
    let posts: Vec<&news::Post> = ground.news.posts.iter().filter(|post| ground.channels.iter().any(|c| c.eq_ignore_ascii_case(&post.channel))).take(2).collect();
    let mut list = column![chips, adding].spacing(8);
    if posts.is_empty() {
        list = list.push(empty(w.t(if ground.channels.is_empty() { "channels-empty" } else { "nothing-yet" })));
    }
    for post in posts {
        let words = column![
            row![
                text(post.name.clone()).font(theme::SANS_SEMI).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(INK)),
                ui::mono_small(when(ground, post.at), FAINT),
            ]
            .spacing(8)
            .align_y(iced::Center),
            container(text(ui::shortened(post.text.clone(), 150)).font(theme::SANS).size(11.5).color(ui::faded(MUTED))).height(46.0).clip(true),
        ]
        .spacing(2)
        .width(Length::Fill);
        let inside: Element<'a, Message> = match post.image.as_deref() {
            Some(url) => row![thumb(ground, Some(url), 64.0, 64.0), words].spacing(10).align_y(iced::Center).into(),
            None => words.into(),
        };
        list = list.push(linked(inside, &post.url));
    }
    let channel_sources: Vec<String> = ground.channels.iter().map(|c| news::channel_source(c)).collect();
    let status = match channel_sources.first() {
        Some(source) => source_state(ground, source),
        None => Space::new().width(0.0).into(),
    };
    panel(w.t("panel-channels"), status, list.into())
}

fn group_panel<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let mut list = column![].spacing(0);
    for happening in ground.catalog.feed.iter().take(5) {
        list = list.push(happening_line(ground, happening));
    }
    panel(w.t("panel-group"), staged_mark(ground), list.into())
}

fn happening_line<'a>(ground: &Ground<'a>, happening: &Happening) -> Element<'a, Message> {
    let w = ground.words;
    let lang = w.lang();
    let person = &ground.catalog.people[happening.who];
    let (left, kind, detail): (Element<'a, Message>, String, String) = match &happening.kind {
        Kind::Render { accuracy, .. } => (
            picture(ground, happening.map, 48.0, 27.0),
            w.t("happened-render"),
            format!("{} · {}", ground.catalog.map(happening.map).map(|map| map.line.clone()).unwrap_or_default(), w.percent(f64::from(*accuracy))),
        ),
        Kind::TopPlay { pp, place, .. } => (
            picture(ground, happening.map, 48.0, 27.0),
            format!("{} #{place}", w.t("happened-top-play")),
            format!("{} · {} pp", ground.catalog.map(happening.map).map(|map| map.line.clone()).unwrap_or_default(), decimal(w, *pp, 0)),
        ),
        Kind::Title(code) => {
            let title = crate::community::title_of(code);
            (
                seal("★".to_owned(), title.map_or(MUTED, |title| title.rarity.colour()), 48.0, 27.0, 12.0),
                w.t("happened-title"),
                title.map_or(String::new(), |title| title.name(lang).to_owned()),
            )
        }
        Kind::Climb { board, from, to } => (seal(format!("#{to}"), INK, 48.0, 27.0, 11.0), w.t("happened-climb"), format!("{} · {from} → {to}", w.t(board.key()))),
    };
    let line = row![
        left,
        column![
            row![
                text(person.name.clone()).font(theme::SANS_SEMI).size(theme::CAPTION + 1.0).wrapping(text::Wrapping::None).color(ui::faded(INK)),
                ui::mono_small(kind, MUTED),
                ui::grow(),
                ui::mono_small(when(ground, happening.at), FAINT),
            ]
            .spacing(8)
            .align_y(iced::Center),
            text(ui::shortened(detail, 52)).font(theme::SANS).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(MUTED)),
        ]
        .spacing(1)
        .width(Length::Fill),
    ]
    .spacing(10)
    .align_y(iced::Center);
    button(line).padding([5, 6]).width(Length::Fill).style(ui::button_faded(theme::row(false))).on_press(Message::Person(Some(happening.who))).into()
}

fn stat<'a>(value: String, label: String) -> Element<'a, Message> {
    column![
        text(value).font(theme::MONO_BOLD).size(13.0).wrapping(text::Wrapping::None).color(ui::faded(INK)),
        text(label).font(theme::SANS).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(FAINT)),
    ]
    .spacing(1)
    .into()
}

fn slab<'a>(inside: Element<'a, Message>, wide: f32) -> container::Container<'a, Message> {
    container(inside).padding([14, 16]).width(wide).style(ui::box_faded(theme::slab))
}

fn people<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let cards: Vec<Element<'a, Message>> = ground
        .catalog
        .people
        .iter()
        .enumerate()
        .map(|(at, person)| {
            let head = row![
                ui::disc(&person.initial(), false, 40.0),
                column![
                    row![
                        text(person.name.clone()).font(theme::SANS_SEMI).size(theme::LEAD).wrapping(text::Wrapping::None).color(ui::faded(INK)),
                        ui::mono_small(person.country.to_owned(), FAINT),
                    ]
                    .spacing(8)
                    .align_y(iced::Center),
                    title_line(ground, person, theme::CAPTION),
                ]
                .spacing(2),
            ]
            .spacing(12)
            .align_y(iced::Center);
            let numbers = row![
                stat(w.lang().group(u64::from(person.pp)), w.t("board-pp")),
                stat(format!("#{}", w.lang().group(u64::from(person.rank))), w.t("global-rank")),
                stat(w.percent(f64::from(person.accuracy)), w.t("board-accuracy")),
            ]
            .spacing(18);
            let mut facts = vec![w.count("games", u64::from(person.plays)), format!("{} {}", w.lang().group(u64::from(person.hours)), w.t("hours-short"))];
            if person.streak > 0 {
                facts.push(w.n("streak-card", u64::from(person.streak)));
            }
            let under = text(facts.join(" · ")).font(theme::MONO).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(FAINT));
            let card = slab(column![head, numbers, under].spacing(12).into(), CARD_WIDE);
            button(card).padding(0).style(ui::button_faded(theme::bare)).on_press(Message::Person(Some(at))).into()
        })
        .collect();
    rolled(container(ui::wrap(cards, GRID_GAP)).max_width(CARD_WIDE * 3.0 + GRID_GAP * 2.0).into())
}

fn bar<'a>(share: f32) -> Element<'a, Message> {
    let k = ui::fade();
    let filled = (share.clamp(0.0, 1.0) * 1000.0).round() as u16;
    let part = |colour: Color, portion: u16| {
        container(Space::new().height(4.0)).width(Length::FillPortion(portion.max(1))).style(move |_| container::Style {
            background: Some(Background::Color(Color { a: colour.a * k, ..colour })),
            border: Border { radius: 2.0.into(), ..Border::default() },
            ..container::Style::default()
        })
    };
    let mut made = row![].spacing(0).width(140.0);
    if filled > 0 {
        made = made.push(part(ACCENT, filled));
    }
    if filled < 1000 {
        made = made.push(part(Color::from_rgba(1.0, 1.0, 1.0, 0.08), 1000 - filled));
    }
    made.into()
}

fn boards<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let mut chips = row![].spacing(6);
    for board in Board::ALL {
        chips = chips.push(
            button(text(w.t(board.key())).font(theme::SANS_SEMI).size(theme::CAPTION))
                .padding([5, 12])
                .style(ui::button_faded(theme::pill(ground.board == board)))
                .on_press(Message::Board(board)),
        );
    }
    let order = ground.catalog.ranked(ground.board);
    let top = order.first().map_or(1.0, |at| ground.board.value(&ground.catalog.people[*at])).max(f64::EPSILON);
    let mut list = column![
        container(chips).center_x(Length::Fill),
        container(ui::mono_small(w.n("boards-week", u64::from(ground.catalog.week)), FAINT)).center_x(Length::Fill).padding(Padding::ZERO.bottom(8.0)),
    ]
    .spacing(10)
    .width(FEED_WIDE);
    for (place, at) in order.iter().enumerate() {
        let person = &ground.catalog.people[*at];
        let colour = if place < 3 { INK } else { MUTED };
        let line = row![
            container(ui::mono(format!("{}", place + 1), colour)).width(26.0).align_x(iced::alignment::Horizontal::Right),
            container(moved(person.moved[ground.board.index()])).width(40.0),
            ui::disc(&person.initial(), false, 30.0),
            container(
                column![
                    text(person.name.clone()).font(theme::SANS_SEMI).size(theme::BODY).wrapping(text::Wrapping::None).color(ui::faded(INK)),
                    title_line(ground, person, 11.0),
                ]
                .spacing(1),
            )
            .width(Length::Fill)
            .clip(true),
            container(ui::mono(shown_value(w, ground.board, person), INK)).width(130.0).align_x(iced::alignment::Horizontal::Right),
            bar((ground.board.value(person) / top) as f32),
        ]
        .spacing(14)
        .align_y(iced::Center);
        list = list.push(
            button(container(line).height(52.0).center_y(52.0).width(Length::Fill))
                .padding([0, 12])
                .style(ui::button_faded(theme::row(ground.person == Some(*at))))
                .on_press(Message::Person(Some(*at))),
        );
    }
    rolled(list.into())
}

fn faces<'a>(ground: &Ground<'a>, holders: &[usize]) -> Element<'a, Message> {
    if holders.is_empty() {
        return ui::mono_small(ground.words.t("nobody-yet"), FAINT);
    }
    let mut shown = row![].spacing(4).align_y(iced::Center);
    for at in holders.iter().take(5) {
        shown = shown.push(ui::disc(&ground.catalog.people[*at].initial(), false, 20.0));
    }
    if holders.len() > 5 {
        shown = shown.push(ui::mono_small(format!("+{}", holders.len() - 5), FAINT));
    }
    shown.into()
}

fn title_card<'a>(ground: &Ground<'a>, title: &Title) -> Element<'a, Message> {
    let w = ground.words;
    let holders = ground.catalog.holders(title.code);
    let known = !holders.is_empty() || title.rarity != Rarity::Secret;
    let colour = if holders.is_empty() { FAINT } else { title.rarity.colour() };
    let name = if known { title.name(w.lang()).to_owned() } else { "???".to_owned() };
    let about = if known { title.about(w.lang()).to_owned() } else { w.t("secret-title") };
    slab(
        column![
            text(name).font(theme::SANS_SEMI).size(theme::BODY).wrapping(text::Wrapping::None).color(ui::faded(colour)),
            text(about).font(theme::SANS).size(11.0).color(ui::faded(MUTED)),
            faces(ground, &holders),
        ]
        .spacing(6)
        .into(),
        CARD_WIDE,
    )
    .into()
}

fn titles<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let mut list = column![].spacing(22).max_width(CARD_WIDE * 3.0 + GRID_GAP * 2.0);
    for rarity in Rarity::ALL {
        let defs: Vec<&Title> = TITLES.iter().filter(|title| title.rarity == rarity).collect();
        if defs.is_empty() {
            continue;
        }
        let held = defs.iter().filter(|title| !ground.catalog.holders(title.code).is_empty()).count();
        let head = row![
            text(w.t(rarity.key())).font(theme::SANS_SEMI).size(theme::BODY).color(ui::faded(rarity.colour())),
            ui::mono_small(format!("{} {}", w.t("unlocked"), w.of(held as u64, defs.len() as u64)), FAINT),
        ]
        .spacing(10)
        .align_y(iced::Center);
        let cards: Vec<Element<'a, Message>> = defs.iter().map(|title| title_card(ground, title)).collect();
        list = list.push(column![head, ui::wrap(cards, GRID_GAP)].spacing(10));
    }
    rolled(list.into())
}

fn drawer_style(_: &Theme) -> container::Style {
    let k = ui::fade();
    container::Style {
        text_color: None,
        background: Some(Background::Color(Color { a: k, ..theme::SLAB_SOLID })),
        border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.08 * k), width: 1.0, radius: 0.0.into() },
        shadow: Shadow { color: Color::from_rgba(0.0, 0.0, 0.0, 0.5 * k), offset: iced::Vector::new(-24.0, 0.0), blur_radius: 60.0 },
        snap: true,
    }
}

fn drawer<'a>(ground: &Ground<'a>, person: &Person) -> Element<'a, Message> {
    let w = ground.words;
    let lang = w.lang();
    let close = button(text("✕").font(theme::SANS_SEMI).size(theme::LEAD).color(ui::faded(MUTED)))
        .padding([2, 8])
        .style(ui::button_faded(theme::bare))
        .on_press(Message::Person(None));
    let head = row![
        ui::disc(&person.initial(), false, 60.0),
        column![
            text(person.name.clone()).font(theme::SANS_SEMI).size(theme::TITLE).wrapping(text::Wrapping::None).color(ui::faded(INK)),
            row![ui::mono_small(person.country.to_owned(), FAINT), title_line(ground, person, theme::CAPTION)].spacing(8).align_y(iced::Center),
        ]
        .spacing(4)
        .width(Length::Fill),
        close,
    ]
    .spacing(14)
    .align_y(iced::Center);
    let kv = |key: String, value: String| {
        row![
            text(key).font(theme::SANS).size(theme::CAPTION).color(ui::faded(MUTED)),
            ui::grow(),
            ui::mono(value, INK),
        ]
        .height(24.0)
        .align_y(iced::Center)
    };
    let figures = column![
        kv(w.t("board-pp"), pp_of(w, person.pp)),
        kv(w.t("global-rank"), format!("#{}", w.lang().group(u64::from(person.rank)))),
        kv(w.t("board-accuracy"), w.percent(f64::from(person.accuracy))),
        kv(w.t("board-plays"), w.lang().group(u64::from(person.plays))),
        kv(w.t("board-hours"), format!("{} {}", w.lang().group(u64::from(person.hours)), w.t("hours-short"))),
        kv(w.t("board-score"), w.lang().group(person.score)),
        kv(w.t("board-hits"), decimal(w, person.hits_per_play, 1)),
    ]
    .spacing(0);
    let mut plays = column![ui::mono_small(w.t("top-plays").to_uppercase(), FAINT)].spacing(8);
    for play in &person.top {
        let grade = match play.grade {
            "SS" => theme::GRADE_SS,
            "S" => theme::GRADE_S,
            "A" => theme::GRADE_A,
            "B" => theme::GRADE_B,
            _ => theme::GRADE_C,
        };
        let line = ground.catalog.maps.get(play.map).map(|map| map.line.clone()).unwrap_or_default();
        plays = plays.push(
            row![
                picture(ground, Some(play.map), 64.0, 36.0),
                column![
                    text(ui::shortened(line, 34)).font(theme::SANS).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(INK)),
                    row![ui::mono_small(w.percent(f64::from(play.accuracy)), MUTED), mods(&play.mods)].spacing(8).align_y(iced::Center),
                ]
                .spacing(3)
                .width(Length::Fill),
                column![
                    ui::mono(format!("{} pp", decimal(w, play.pp, 0)), INK),
                    text(play.grade).font(theme::MONO_BOLD).size(12.0).color(ui::faded(grade)),
                ]
                .spacing(2)
                .align_x(iced::alignment::Horizontal::Right),
            ]
            .spacing(12)
            .align_y(iced::Center),
        );
    }
    let mut held: Vec<&Title> = person.titles.iter().filter_map(|code| crate::community::title_of(code)).collect();
    held.sort_by_key(|title| std::cmp::Reverse(Rarity::ALL.iter().position(|rarity| *rarity == title.rarity)));
    let chips: Vec<Element<'a, Message>> = held
        .iter()
        .map(|title| {
            let colour = title.rarity.colour();
            let k = ui::fade();
            container(text(title.name(lang)).font(theme::SANS_SEMI).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(colour)))
                .padding([3, 8])
                .style(move |_| container::Style {
                    background: Some(Background::Color(Color { a: 0.1 * k, ..colour })),
                    border: Border { color: Color { a: 0.3 * k, ..colour }, width: 1.0, radius: 10.0.into() },
                    ..container::Style::default()
                })
                .into()
        })
        .collect();
    let titles = column![ui::mono_small(format!("{} · {}", w.t("community-titles").to_uppercase(), held.len()), FAINT), ui::wrap(chips, 6.0)].spacing(8);
    let figures = match person.streak {
        0 => figures,
        days => figures.push(kv(w.t("streak"), w.n("streak-days", u64::from(days)))),
    };
    let inside = column![head, figures, plays, titles].spacing(22).padding(Padding { top: 24.0, right: 24.0, bottom: 28.0, left: 24.0 });
    let panel = container(scrollable(inside).style(ui::thin_scroll).height(Length::Fill)).width(DRAWER_WIDE).height(Length::Fill).style(drawer_style);
    stack![
        mouse_area(ui::veil(Color::from_rgba(0.027, 0.012, 0.016, 0.45))).on_press(Message::Person(None)),
        container(panel).width(Length::Fill).height(Length::Fill).align_x(iced::alignment::Horizontal::Right),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
