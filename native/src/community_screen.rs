use std::collections::HashMap;

use iced::widget::{button, column, container, image, mouse_area, row, scrollable, stack, text, text_input, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Shadow, Theme};

use crate::community::{Board, Catalog, Friend, Friends, Person, Play, Rarity, Title};
use crate::lang::Words;
use crate::news::{self, News};
use crate::theme::{self, ACCENT, FAINT, INK, MUTED};
use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Profile,
    Feed,
    People,
    Boards,
    Titles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeopleFrom {
    Chat,
    Game,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Standing {
    #[default]
    General,
    Adaptive,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Reading {
    Story(news::Story),
    Post(news::Post),
    Build(news::Build),
}

impl Reading {
    pub fn url(&self) -> &str {
        match self {
            Reading::Story(story) => &story.url,
            Reading::Post(post) => &post.url,
            Reading::Build(build) => &build.url,
        }
    }

    pub fn pictures(&self) -> Vec<String> {
        match self {
            Reading::Story(story) => story
                .image
                .iter()
                .cloned()
                .chain(story.body.iter().filter_map(|block| match block {
                    news::Block::Image(url) => Some(url.clone()),
                    _ => None,
                }))
                .collect(),
            Reading::Post(post) => post.images.iter().cloned().chain(post.image.iter().cloned()).chain(post.videos.iter().filter_map(|video| video.thumb.clone())).collect(),
            Reading::Build(_) => Vec::new(),
        }
    }
}

pub fn wide(url: &str) -> String {
    format!("wide:{url}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fetch {
    Staged,
    Loading,
    Fresh(i64),
    Failed,
}

#[derive(Debug, Clone)]
pub enum Message {
    Read(Reading),
    Unread,
    PeopleFrom(PeopleFrom),
    Standing(Standing),
    Section(Section),
    Board(Board),
    Person(Option<usize>),
    Open(String),
    Refresh,
    Again,
    ChannelDraft(String),
    ChannelAdd,
    ChannelRemove(String),
    Filter(crate::chronicle::Filter),
    Source(crate::chronicle::Source),
    Stream(crate::chronicle::Stream),
    Search(String),
    Toggle(String),
    Reveal,
    Spot(usize),
    SpotHold(bool),
    Rank(usize),
    RankHold(bool),
    Metric(crate::dossier::Metric),
    Span(u32),
    GradeHover(Option<usize>),
    TitlePick(String),
    PlayOpen(usize),
    PlayClip(Option<String>, String),
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
    pub live_shown: usize,
    pub live_k: f32,
    pub reading: Option<&'a Reading>,
    pub read_k: f32,
    pub people_from: PeopleFrom,
    pub standing: Standing,
    pub card: Option<&'a crate::community::wire::Card>,
    pub flags: &'a HashMap<String, iced::widget::svg::Handle>,
    pub avatar: Option<&'a image::Handle>,
    pub chat: String,
    pub fetch: Fetch,
    pub width: f32,
    pub filter: crate::chronicle::Filter,
    pub source: crate::chronicle::Source,
    pub stream: crate::chronicle::Stream,
    pub query: &'a str,
    pub open_events: &'a std::collections::HashSet<String>,
    pub seen: i64,
    pub spot: usize,
    pub spot_k: f32,
    pub rank: usize,
    pub rank_k: f32,
    pub rank_started: std::time::Instant,
    pub rank_held: Option<f32>,
    pub metric: crate::dossier::Metric,
    pub span: u32,
    pub grade_hover: Option<usize>,
    pub title_pick: Option<&'a str>,
    pub play_open: Option<usize>,
    pub clips_loading: &'a std::collections::HashSet<String>,
}

const READ_WIDE: f32 = 760.0;
const DRAWER_WIDE: f32 = 400.0;
const GRID_GAP: f32 = 10.0;
const STAGE_ROOM: Padding = Padding { top: 8.0, right: 40.0, bottom: 28.0, left: 40.0 };

fn smooth(from: f32, to: f32, k: f32) -> f32 {
    let t = ((k - from) / (to - from)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

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
        word("community-profile", Section::Profile),
        word("community-feed", Section::Feed),
        word("community-people", Section::People),
        word("community-boards", Section::Boards),
        word("community-titles", Section::Titles),
    ]
    .spacing(28);
    let head = column![container(switch).center_x(Length::Fill), container(group_note(ground)).center_x(Length::Fill)].spacing(10);
    let body = match ground.section {
        Section::Profile => crate::dossier::view(ground),
        Section::Feed => crate::chronicle::view(ground),
        Section::People => people(ground),
        Section::Boards => boards(ground),
        Section::Titles => titles(ground),
    };
    let page = column![container(head).padding(Padding::ZERO.top(22.0)), body].width(Length::Fill).height(Length::Fill);
    let mut layers: Vec<Element<'a, Message>> = vec![page.into()];
    if let Some(person) = ground.person.and_then(|at| ground.catalog.people.get(at)) {
        layers.push(crate::unfold::shield(drawer(ground, person)).into());
    }
    if let Some(reading) = ground.reading.filter(|_| ground.read_k > 0.001) {
        layers.push(reader(ground, reading));
    }
    iced::widget::Stack::with_children(layers).width(Length::Fill).height(Length::Fill).into()
}

fn group_note<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let catalog = ground.catalog;
    let mut parts: Vec<String> = Vec::new();
    if !catalog.group.is_empty() {
        parts.push(catalog.group.clone());
    }
    parts.push(w.count("players", catalog.people.len() as u64));
    if catalog.staged {
        parts.push(w.t("community-staged"));
    }
    let (tail, colour, again) = match ground.fetch {
        Fetch::Staged => (String::new(), FAINT, false),
        Fetch::Loading => (w.t("news-loading"), FAINT, false),
        Fetch::Fresh(at) => (format!("{} {}", w.t("news-updated"), w.clock(at)), FAINT, true),
        Fetch::Failed => (w.t("news-failed"), ACCENT, true),
    };
    let said = parts.join(" · ");
    let note: Element<'a, Message> = match tail.is_empty() {
        true => ui::mono_small(said, FAINT),
        false => row![ui::mono_small(said + " · ", FAINT), ui::mono_small(tail, colour)].align_y(iced::Center).into(),
    };
    if again {
        button(note).padding(0).style(ui::button_faded(theme::bare)).on_press(Message::Again).into()
    } else {
        note.into()
    }
}

fn rolled<'a>(inside: Element<'a, Message>) -> Element<'a, Message> {
    scrollable(container(inside).center_x(Length::Fill).padding(Padding { top: 12.0, right: 40.0, bottom: 28.0, left: 40.0 }))
        .style(ui::thin_scroll).direction(ui::hidden_bar())
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn hatch<'a>(wide: f32, high: f32) -> Element<'a, Message> {
    container(ui::fine_hatch()).width(wide).height(high).into()
}

pub(crate) fn video_tile<'a>(ground: &Ground<'a>, video: &news::Video, wide: bool, high: f32) -> Element<'a, Message> {
    let w = ground.words;
    let k = ui::fade();
    let handle = video.thumb.as_deref().and_then(|url| if wide { ground.pictures.get(&self::wide(url)) } else { ground.pictures.get(url) });
    let width = if wide { Length::Fill } else { Length::Fixed(high) };
    let back: Element<'a, Message> = match handle {
        Some(handle) => image(handle.clone()).content_fit(iced::ContentFit::Cover).width(width).height(high).border_radius(if wide { 12.0 } else { 6.0 }).opacity(k).into(),
        None => container(ui::fine_hatch()).width(width).height(high).into(),
    };
    let loading = ground.clips_loading.contains(&video.link);
    let circle = if wide { 54.0 } else { 30.0 };
    let (icon, inset) = if video.src.is_some() { (crate::glyphs::Icon::Play, 0.55) } else { (crate::glyphs::Icon::External, 0.42) };
    let play = container(crate::glyphs::glyph(icon, circle * inset, Color::WHITE)).width(circle).height(circle).center(circle).style(move |_| container::Style {
        background: Some(Background::Color(Color::from_rgba(0.047, 0.027, 0.035, 0.6 * k))),
        border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.2 * k), width: 1.0, radius: (circle / 2.0).into() },
        ..container::Style::default()
    });
    let said = if loading {
        w.t("news-loading")
    } else if video.src.is_none() {
        w.t("act-telegram")
    } else {
        video.duration.clone()
    };
    let badge: Element<'a, Message> = if said.is_empty() {
        Space::new().height(0.0).into()
    } else {
        container(ui::mono_small(said, Color::WHITE)).padding([2, 6]).style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.047, 0.027, 0.035, 0.7 * k))),
            border: Border { radius: 5.0.into(), ..Border::default() },
            ..container::Style::default()
        }).into()
    };
    let front = stack![
        container(play).width(width).height(high).center(Length::Fill),
        container(badge).width(width).height(high).padding(if wide { 10 } else { 4 }).align_x(iced::alignment::Horizontal::Right).align_y(iced::alignment::Vertical::Bottom),
    ];
    let tile = stack![back, front].width(width).height(high);
    button(tile).padding(0).style(ui::button_faded(theme::bare)).on_press(Message::PlayClip(video.src.clone(), video.link.clone())).into()
}

pub(crate) fn avatar_colour(name: &str) -> Color {
    let tones = [(0.886, 0.282, 0.282), (0.753, 0.337, 0.478), (0.353, 0.478, 0.784), (0.345, 0.627, 0.690), (0.753, 0.541, 0.227), (0.478, 0.353, 0.784), (0.541, 0.416, 0.290), (0.290, 0.627, 0.478)];
    let sum: usize = name.bytes().map(usize::from).sum();
    let (r, g, b) = tones[sum % tones.len()];
    Color::from_rgb(r, g, b)
}

pub(crate) fn picture<'a>(ground: &Ground<'a>, map: Option<usize>, wide: f32, high: f32) -> Element<'a, Message> {
    if wide <= 0.0 {
        let found = ground.catalog.map(map).and_then(|map| ground.thumbs.get(&map.hash).or_else(|| map.card().and_then(|card| ground.pictures.get(&card))));
        return match found {
            Some(handle) => image(handle.clone()).content_fit(iced::ContentFit::Cover).width(Length::Fill).height(high).border_radius(12.0).opacity(ui::fade()).into(),
            None => container(ui::fine_hatch()).width(Length::Fill).height(high).into(),
        };
    }
    let Some(map) = ground.catalog.map(map) else {
        return hatch(wide, high);
    };
    let handle = ground.thumbs.get(&map.hash).or_else(|| map.card().and_then(|card| ground.pictures.get(&card)));
    match handle {
        Some(handle) => image(handle.clone()).content_fit(iced::ContentFit::Cover).width(wide).height(high).border_radius(6.0).opacity(ui::fade()).into(),
        None => hatch(wide, high),
    }
}

pub(crate) fn round<'a>(handle: Option<&image::Handle>, initial: &str, side: f32) -> Element<'a, Message> {
    match handle {
        Some(handle) => image(handle.clone()).content_fit(iced::ContentFit::Cover).width(side).height(side).border_radius(side / 2.0).opacity(ui::fade()).into(),
        None => ui::disc(initial, false, side),
    }
}

pub(crate) fn face<'a>(ground: &Ground<'a>, person: &Person, side: f32) -> Element<'a, Message> {
    let handle = ground.pictures.get(&person.avatar).or(if person.you { ground.avatar } else { None });
    round(handle, &person.initial(), side)
}

pub(crate) fn friend_face<'a>(ground: &Ground<'a>, friend: &Friend, side: f32) -> Element<'a, Message> {
    round(ground.pictures.get(&friend.avatar), &friend.initial(), side)
}

pub(crate) fn mods<'a>(list: &[String]) -> Element<'a, Message> {
    let mut shown = row![].spacing(4).align_y(iced::Center);
    for acronym in list {
        shown = shown.push(crate::main_screen::mod_badge(acronym));
    }
    shown.into()
}

pub(crate) fn title_line<'a>(ground: &Ground<'a>, person: &Person, size: f32) -> Element<'a, Message> {
    match ground.catalog.shown_title(person) {
        Some(title) => text(title.name(ground.words.lang()).to_owned()).font(theme::SANS).size(size).wrapping(text::Wrapping::None).color(ui::faded(title.rarity.colour())).into(),
        None => text(ground.words.t("no-title")).font(theme::SANS).size(size).wrapping(text::Wrapping::None).color(ui::faded(FAINT)).into(),
    }
}

pub(crate) fn pp_of(words: &Words, pp: u32) -> String {
    format!("{} pp", words.lang().group(u64::from(pp)))
}

pub(crate) fn decimal(words: &Words, value: f32, places: usize) -> String {
    let made = format!("{value:.places$}");
    match words.lang() {
        crate::lang::Lang::En => made,
        crate::lang::Lang::Ru => made.replace('.', ","),
    }
}

pub(crate) fn rank_of(words: &Words, rank: u32) -> String {
    if rank == 0 {
        "—".to_owned()
    } else {
        format!("#{}", words.lang().group(u64::from(rank)))
    }
}

pub(crate) fn shown_value(words: &Words, board: Board, person: &Person) -> String {
    match board {
        Board::Pp => pp_of(words, person.pp),
        Board::Accuracy => words.percent(f64::from(person.accuracy)),
        Board::Plays => words.lang().group(u64::from(person.plays)),
        Board::Hours => format!("{} {}", words.lang().group(u64::from(person.hours)), words.t("hours-short")),
        Board::Score => words.lang().group(person.score),
        Board::HitsPerPlay => decimal(words, person.hits_per_play, 1),
    }
}

pub(crate) fn moved<'a>(by: i32) -> Element<'a, Message> {
    if by == 0 {
        return Space::new().width(0.0).height(0.0).into();
    }
    let (icon, colour) = if by > 0 { (crate::glyphs::Icon::Up, theme::HIT_100) } else { (crate::glyphs::Icon::Down, ACCENT) };
    let k = ui::fade();
    container(row![crate::glyphs::glyph(icon, 10.0, colour), text(by.unsigned_abs().to_string()).font(theme::MONO_BOLD).size(10.5).color(ui::faded(colour))].spacing(2).align_y(iced::Center))
        .padding(Padding { top: 1.0, right: 6.0, bottom: 1.0, left: 4.0 })
        .style(move |_| container::Style {
            background: Some(Background::Color(Color { a: 0.13 * k, ..colour })),
            border: Border { radius: 7.0.into(), ..Border::default() },
            ..container::Style::default()
        })
        .into()
}

pub(crate) fn flag<'a>(ground: &Ground<'a>, code: &str, high: f32) -> Element<'a, Message> {
    let code = code.trim();
    let wide = (high * 36.0 / 26.0).round();
    if let Some(handle) = ground.flags.get(&code.to_ascii_lowercase()) {
        return iced::widget::svg(handle.clone()).width(wide).height(high).opacity(ui::fade()).into();
    }
    if code.chars().count() != 2 {
        return Space::new().width(0.0).height(0.0).into();
    }
    let k = ui::fade();
    container(text(code.to_ascii_uppercase()).font(theme::MONO_BOLD).size((high * 0.62).max(7.5)).color(ui::faded(MUTED)))
        .width(wide)
        .height(high)
        .center_x(wide)
        .center_y(high)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.06 * k))),
            border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.08 * k), width: 1.0, radius: (high * 0.2).into() },
            ..container::Style::default()
        })
        .into()
}

pub(crate) fn columns_for(ground: &Ground<'_>, target: f32) -> usize {
    let room = (ground.width - 80.0).max(target);
    ((room / target).round() as usize).clamp(1, 6)
}

pub(crate) fn grid<'a>(cells: Vec<Element<'a, Message>>, columns: usize, gap: f32) -> Element<'a, Message> {
    let columns = columns.max(1);
    let mut rows = column![].spacing(gap).width(Length::Fill);
    let mut cells = cells.into_iter().peekable();
    while cells.peek().is_some() {
        let mut line = row![].spacing(gap).width(Length::Fill);
        for _ in 0..columns {
            line = line.push(match cells.next() {
                Some(cell) => container(cell).width(Length::FillPortion(1)),
                None => container(Space::new().width(0.0).height(0.0)).width(Length::FillPortion(1)),
            });
        }
        rows = rows.push(line);
    }
    rows.into()
}

fn lifted(_: &Theme, status: button::Status) -> button::Style {
    let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(Background::Color(if lit { Color::from_rgba(0.075, 0.035, 0.045, 0.98) } else { Color::from_rgba(0.055, 0.025, 0.033, 0.96) })),
        text_color: INK,
        border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, if lit { 0.13 } else { 0.07 }), width: 1.0, radius: 16.0.into() },
        shadow: if lit { Shadow { color: Color::from_rgba(0.0, 0.0, 0.0, 0.4), offset: iced::Vector::new(0.0, 10.0), blur_radius: 26.0 } } else { Shadow::default() },
        snap: true,
    }
}

pub(crate) fn source_state<'a>(ground: &Ground<'a>, source: &str) -> Element<'a, Message> {
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

pub(crate) fn empty<'a>(words: String) -> Element<'a, Message> {
    container(ui::mono_small(words, FAINT)).width(Length::Fill).height(Length::Fill).center(Length::Fill).into()
}

pub(crate) fn when<'a>(ground: &Ground<'a>, at: i64) -> String {
    let w = ground.words;
    if ground.now_unix - at < 86_400 && ground.now_unix >= at {
        w.clock(at)
    } else {
        w.day(at, ground.now_unix)
    }
}

pub(crate) fn ago(words: &Words, minutes: u32) -> String {
    match minutes {
        0 => words.t("online"),
        m if m < 60 => words.n("minutes-ago", u64::from(m)),
        m if m < 24 * 60 => words.n("hours-ago", u64::from(m / 60)),
        u32::MAX => "—".to_owned(),
        m => words.n("days-ago", u64::from(m / (24 * 60))),
    }
}

pub(crate) fn since<'a>(ground: &Ground<'a>, at: i64) -> String {
    let w = ground.words;
    let seconds = (ground.now_unix - at).max(0);
    match seconds {
        s if s < 90 => w.t("live-now"),
        s if s < 3600 => w.n("minutes-ago", (s / 60) as u64),
        s if s < 86_400 => w.n("hours-ago", (s / 3600) as u64),
        s => w.n("days-ago", (s / 86_400) as u64),
    }
}

pub(crate) fn grade_colour(grade: &str) -> Color {
    match grade {
        "SS" => theme::GRADE_SS,
        "S" => theme::GRADE_S,
        "A" => theme::GRADE_A,
        "B" => theme::GRADE_B,
        "C" => theme::GRADE_C,
        _ => theme::GRADE_D,
    }
}

pub(crate) fn grade<'a>(letter: &str, size: f32) -> Element<'a, Message> {
    text(letter.to_owned()).font(theme::MONO_BOLD).size(size).color(ui::faded(grade_colour(letter))).into()
}

pub(crate) fn line_of(ground: &Ground<'_>, map: usize) -> String {
    ground.catalog.maps.get(map).map(|map| map.line.clone()).unwrap_or_default()
}

pub(crate) fn play_row<'a>(ground: &Ground<'a>, play: &Play, wide: f32, high: f32, room: usize, with_pp: bool) -> Element<'a, Message> {
    let w = ground.words;
    let right: Element<'a, Message> = if with_pp && play.pp > 0.0 {
        column![ui::mono(format!("{} pp", decimal(w, play.pp, 0)), INK), grade(&play.grade, 12.0)].spacing(2).align_x(iced::alignment::Horizontal::Right).into()
    } else {
        column![grade(&play.grade, 12.0), ui::mono_small(when(ground, play.at), FAINT)].spacing(2).align_x(iced::alignment::Horizontal::Right).into()
    };
    row![
        picture(ground, Some(play.map), wide, high),
        column![
            text(ui::shortened(line_of(ground, play.map), room)).font(theme::SANS).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(INK)),
            row![ui::mono_small(w.percent(f64::from(play.accuracy)), MUTED), mods(&play.mods)].spacing(8).align_y(iced::Center),
        ]
        .spacing(3)
        .width(Length::Fill),
        right,
    ]
    .spacing(12)
    .align_y(iced::Center)
    .into()
}

pub(crate) fn chip<'a>(inside: Element<'a, Message>, colour: Color) -> Element<'a, Message> {
    let k = ui::fade();
    container(inside)
        .padding([3, 8])
        .style(move |_| container::Style {
            background: Some(Background::Color(Color { a: 0.12 * k, ..colour })),
            border: Border { color: Color { a: 0.28 * k, ..colour }, width: 1.0, radius: 7.0.into() },
            ..container::Style::default()
        })
        .into()
}

pub(crate) fn title_chip<'a>(title: &Title, lang: crate::lang::Lang) -> Element<'a, Message> {
    let colour = title.rarity.colour();
    chip(text(title.name(lang).to_owned()).font(theme::SANS_SEMI).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(colour)).into(), colour)
}

pub(crate) fn people_switch<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    container(segmented(vec![
        (w.t("people-chat"), ground.people_from == PeopleFrom::Chat, Message::PeopleFrom(PeopleFrom::Chat)),
        (w.t("people-game"), ground.people_from == PeopleFrom::Game, Message::PeopleFrom(PeopleFrom::Game)),
    ]))
    .center_x(Length::Fill)
    .into()
}

pub(crate) fn friend_line<'a>(ground: &Ground<'a>, friend: &Friend) -> Element<'a, Message> {
    let w = ground.words;
    let k = ui::fade();
    let dot_colour = if friend.online { theme::HIT_100 } else { Color::from_rgba(1.0, 1.0, 1.0, 0.18) };
    let dot = container(Space::new().width(8.0).height(8.0)).style(move |_| container::Style {
        background: Some(Background::Color(Color { a: dot_colour.a * k, ..dot_colour })),
        border: Border { radius: 4.0.into(), ..Border::default() },
        ..container::Style::default()
    });
    let inside = row![
        stack![friend_face(ground, friend, 26.0), container(dot).width(26.0).height(26.0).align_x(iced::alignment::Horizontal::Right).align_y(iced::alignment::Vertical::Bottom)],
        column![
            row![
                text(friend.name.clone()).font(theme::SANS_SEMI).size(theme::CAPTION + 1.0).wrapping(text::Wrapping::None).color(ui::faded(INK)),
                flag(ground, &friend.country, 11.0),
            ]
            .spacing(7)
            .align_y(iced::Center),
            ui::mono_small(ago(w, friend.minutes_away(ground.now_unix)), if friend.online { theme::HIT_100 } else { FAINT }),
        ]
        .spacing(1)
        .width(Length::Fill),
        column![ui::mono(pp_of(w, friend.pp), INK), ui::mono_small(rank_of(w, friend.rank), FAINT)].spacing(1).align_x(iced::alignment::Horizontal::Right),
    ]
    .spacing(10)
    .align_y(iced::Center);
    button(container(inside).padding([4, 6]))
        .padding(0)
        .width(Length::Fill)
        .style(ui::button_faded(theme::row(false)))
        .on_press(Message::Open(format!("https://osu.ppy.sh/users/{}", friend.name)))
        .into()
}

pub(crate) fn friends_said<'a>(ground: &Ground<'a>) -> Option<Element<'a, Message>> {
    let w = ground.words;
    match &ground.catalog.friends_state {
        Friends::Staged | Friends::Ready if ground.catalog.friends.is_empty() => Some(empty(w.t("nothing-yet"))),
        Friends::Staged | Friends::Ready => None,
        Friends::Waiting => Some(empty(w.t("news-loading"))),
        Friends::Need(need) => Some(empty(w.t(if need == "friends" { "friends-relink" } else { "friends-link" }))),
        Friends::Failed => Some(
            container(button(ui::mono_small(w.t("news-failed"), ACCENT)).padding(0).style(ui::button_faded(theme::bare)).on_press(Message::Again))
                .width(Length::Fill)
                .height(Length::Fill)
                .center(Length::Fill)
                .into(),
        ),
    }
}

pub(crate) fn channel_tools<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
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
    column![chips, adding].spacing(8).into()
}

fn stage_look() -> crate::unfold::Look {
    let k = ui::fade();
    crate::unfold::Look {
        fill: Color { a: k, ..theme::SLAB_SOLID },
        line: Color::from_rgba(1.0, 1.0, 1.0, 0.1 * k),
        veil: Color::from_rgba(0.027, 0.012, 0.016, 0.7 * k),
        radius: 16.0,
    }
}

fn stage_rolled<'a>(inside: Element<'a, Message>) -> Element<'a, Message> {
    scrollable(container(inside).padding(Padding::ZERO.right(10.0))).style(ui::thin_scroll).direction(ui::hidden_bar()).width(Length::Fill).height(Length::Fill).into()
}

pub(crate) fn rich<'a>(spans: &[news::Span], colour: Color, size: f32) -> Element<'a, Message> {
    rich_as(spans, colour, size, false)
}

pub(crate) fn rich_as<'a>(spans: &[news::Span], colour: Color, size: f32, heading: bool) -> Element<'a, Message> {
    let made: Vec<iced::widget::text::Span<'a, String>> = spans
        .iter()
        .map(|piece| {
            let font = match (piece.code, piece.bold || heading) {
                (true, _) => theme::MONO,
                (false, true) => theme::SANS_SEMI,
                (false, false) => theme::SANS,
            };
            let span = iced::widget::span(piece.text.clone()).font(font).size(if piece.code { size - 1.0 } else { size }).color(ui::faded(colour));
            match &piece.link {
                Some(link) => span.link(link.clone()).color(ui::faded(ACCENT)).underline(true),
                None => span,
            }
        })
        .collect();
    iced::widget::rich_text(made).on_link_click(Message::Open).line_height(1.45).width(Length::Fill).into()
}

fn blocks<'a>(list: &[news::Block], skip_image: Option<&str>, size: f32, picture: &dyn Fn(&str) -> Element<'a, Message>) -> Vec<Element<'a, Message>> {
    list.iter()
        .filter(|block| !matches!(block, news::Block::Image(url) if Some(url.as_str()) == skip_image))
        .map(|block| match block {
            news::Block::Heading(spans) => rich_as(spans, INK, theme::LEAD + 1.0, true),
            news::Block::Text(spans) => rich(spans, INK, size),
            news::Block::Item(spans) => row![text("•").font(theme::SANS_SEMI).size(size).color(ui::faded(ACCENT)), rich(spans, INK, size)].spacing(10).into(),
            news::Block::Quote(spans) => row![
                container(Space::new()).width(2.0).height(Length::Fill).style(|_| container::Style {
                    background: Some(Background::Color(Color { a: 0.7 * ui::fade(), ..ACCENT })),
                    border: Border { radius: 1.0.into(), ..Border::default() },
                    ..container::Style::default()
                }),
                rich(spans, MUTED, size),
            ]
            .spacing(14)
            .height(Length::Shrink)
            .into(),
            news::Block::Image(url) => picture(url),
        })
        .collect()
}

fn reader<'a>(ground: &Ground<'a>, reading: &'a Reading) -> Element<'a, Message> {
    let w = ground.words;
    let k = ground.read_k;
    let after = ui::fading(ui::fade() * smooth(0.2, 1.0, k), || -> Element<'a, Message> {
        let picture = |url: &str| -> Element<'a, Message> {
            match ground.pictures.get(&wide(url)) {
                Some(handle) => image(handle.clone()).width(Length::Fill).content_fit(iced::ContentFit::Contain).opacity(ui::fade()).into(),
                None => container(ui::fine_hatch()).width(Length::Fill).height(220.0).into(),
            }
        };
        let (source, title, meta, mut body): (String, String, String, Vec<Element<'a, Message>>) = match reading {
            Reading::Story(story) => {
                let story = ground.news.stories.iter().find(|fresh| fresh.url == story.url).unwrap_or(story);
                let mut body: Vec<Element<'a, Message>> = Vec::new();
                if let Some(url) = &story.image {
                    body.push(picture(url));
                }
                if story.body.is_empty() {
                    body.push(rich(&[news::Span::plain(&story.lead)], INK, theme::BODY));
                }
                body.extend(blocks(&story.body, story.image.as_deref(), theme::BODY, &picture));
                (w.t("panel-news"), story.title.clone(), w.day(story.at, ground.now_unix) + " · " + &w.clock(story.at), body)
            }
            Reading::Post(post) => {
                let post = ground.news.posts.iter().find(|fresh| fresh.url == post.url).unwrap_or(post);
                let mut body: Vec<Element<'a, Message>> = Vec::new();
                let mut photos: Vec<&String> = post.images.iter().collect();
                if photos.is_empty() {
                    photos.extend(post.image.iter());
                }
                for url in photos {
                    body.push(picture(url));
                }
                for video in &post.videos {
                    body.push(video_tile(ground, video, true, 300.0));
                }
                if post.body.is_empty() {
                    body.push(rich(&[news::Span::plain(&post.text)], INK, theme::LEAD));
                }
                body.extend(blocks(&post.body, None, theme::LEAD, &picture));
                (format!("@{}", post.channel), post.name.clone(), w.day(post.at, ground.now_unix) + " · " + &w.clock(post.at), body)
            }
            Reading::Build(build) => {
                let build = ground.news.builds.iter().find(|fresh| fresh.url == build.url).unwrap_or(build);
                let mut body: Vec<Element<'a, Message>> = Vec::new();
                let mut categories: Vec<&str> = Vec::new();
                for change in &build.changes {
                    if !categories.contains(&change.category.as_str()) {
                        categories.push(&change.category);
                    }
                }
                for category in categories {
                    body.push(ui::mono_small(category.to_uppercase(), FAINT));
                    for change in build.changes.iter().filter(|change| change.category == category) {
                        body.push(
                            row![
                                text("•").font(theme::SANS_SEMI).size(theme::BODY).color(ui::faded(if change.major { ACCENT } else { FAINT })),
                                text(change.title.clone()).font(if change.major { theme::SANS_SEMI } else { theme::SANS }).size(theme::BODY).color(ui::faded(if change.major { INK } else { MUTED })),
                            ]
                            .spacing(10)
                            .into(),
                        );
                    }
                }
                (build.stream.clone(), format!("{} {}", build.stream, build.version), w.day(build.at, ground.now_unix), body)
            }
        };
        body.push(container(ui::primary(w.t("read-outside"), Some(Message::Open(reading.url().to_owned())))).padding(Padding::ZERO.top(8.0)).into());
        let head = column![
            row![ui::mono_small(source.to_uppercase(), FAINT), ui::grow(), button(text("✕").font(theme::SANS_SEMI).size(theme::LEAD).color(ui::faded(MUTED))).padding([2, 8]).style(ui::button_faded(theme::bare)).on_press(Message::Unread)].align_y(iced::Center),
            text(title).font(theme::SANS_SEMI).size(theme::TITLE).color(ui::faded(INK)),
            ui::mono_small(meta, FAINT),
        ]
        .spacing(6);
        container(column![head, stage_rolled(iced::widget::Column::with_children(body).spacing(14).into())].spacing(16)).padding([22, 26]).width(Length::Fill).height(Length::Fill).into()
    });
    crate::unfold::unfold(after, None, None, k, Message::Unread).wide(READ_WIDE).room(STAGE_ROOM).look(stage_look()).into()
}

fn figure<'a>(value: String, label: String, colour: Color) -> Element<'a, Message> {
    column![
        text(value).font(theme::MONO_BOLD).size(16.0).wrapping(text::Wrapping::None).color(ui::faded(colour)),
        text(label).font(theme::SANS).size(11.5).wrapping(text::Wrapping::None).color(ui::faded(FAINT)),
    ]
    .spacing(2)
    .width(Length::FillPortion(1))
    .into()
}

fn person_card<'a>(ground: &Ground<'a>, at: usize, person: &Person, place: usize) -> Element<'a, Message> {
    let w = ground.words;
    let high = 204.0;
    let place_colour = medal(place).unwrap_or(Color::from_rgba(0.925, 0.906, 0.886, 0.55));
    let head = row![
        ringed(ground, person, 54.0, medal(place).unwrap_or(Color::from_rgba(1.0, 1.0, 1.0, 0.22))),
        column![
            row![
                container(text(person.name.clone()).font(theme::SANS_SEMI).size(18.0).wrapping(text::Wrapping::None).color(ui::faded(INK))).clip(true),
                flag(ground, &person.country, 13.0),
            ]
            .spacing(8)
            .align_y(iced::Center),
            title_line(ground, person, 12.5),
        ]
        .spacing(3)
        .width(Length::Fill),
        column![
            text(format!("#{place}")).font(theme::SANS_SEMI).size(30.0).wrapping(text::Wrapping::None).color(ui::faded(place_colour)),
            ui::mono_small(w.t("in-group"), FAINT),
        ]
        .spacing(0)
        .align_x(iced::alignment::Horizontal::Right),
    ]
    .spacing(14)
    .align_y(iced::Center);
    let numbers = row![
        figure(w.lang().group(u64::from(person.pp)), w.t("board-pp"), Color::from_rgb(0.941, 0.408, 0.408)),
        figure(rank_of(w, person.rank), w.t("global-rank"), INK),
        figure(w.percent(f64::from(person.accuracy)), w.t("board-accuracy"), INK),
    ]
    .spacing(12);
    let mut facts = row![
        crate::glyphs::glyph(crate::glyphs::Icon::Play, 12.0, MUTED),
        ui::mono_small(w.count("games", u64::from(person.plays)), MUTED),
        Space::new().width(6.0),
        crate::glyphs::glyph(crate::glyphs::Icon::Clock, 12.0, MUTED),
        ui::mono_small(format!("{} {}", w.lang().group(u64::from(person.hours)), w.t("hours-short")), MUTED),
    ]
    .spacing(6)
    .align_y(iced::Center);
    if person.streak > 0 {
        facts = facts.push(Space::new().width(6.0)).push(crate::glyphs::glyph(crate::glyphs::Icon::Flame, 12.0, Color::from_rgb(0.941, 0.408, 0.408))).push(ui::mono_small(w.n("streak-card", u64::from(person.streak)), MUTED));
    }
    let k = ui::fade();
    let ground_colour = Color::from_rgb(0.047, 0.027, 0.035);
    let foot = container(container(facts).clip(true))
        .width(Length::Fill)
        .padding(Padding { top: 14.0, right: 20.0, bottom: 14.0, left: 20.0 })
        .style(move |_| container::Style {
            background: Some(Background::Gradient(iced::Gradient::Linear(
                iced::gradient::Linear::new(iced::Radians(std::f32::consts::PI)).add_stop(0.0, Color { a: 0.0, ..ground_colour }).add_stop(1.0, Color { a: 0.55 * k, ..Color::BLACK }),
            ))),
            border: Border { radius: iced::border::Radius { top_left: 0.0, top_right: 0.0, bottom_right: 16.0, bottom_left: 16.0 }, ..Border::default() },
            ..container::Style::default()
        });
    let inside = column![container(column![head, numbers].spacing(18)).padding(Padding { top: 18.0, right: 20.0, bottom: 0.0, left: 20.0 }), ui::grow_tall(), foot].height(high);
    let cover = ground.pictures.get(&person.cover);
    button(stack![backdrop(cover, high, 16.0, avatar_colour(&person.name), false), inside].height(high))
        .padding(0)
        .width(Length::Fill)
        .style(ui::button_faded(lifted))
        .on_press(Message::Person(Some(at)))
        .into()
}

fn people<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let switch = people_switch(ground);
    if ground.people_from == PeopleFrom::Game {
        let body: Element<'a, Message> = match friends_said(ground) {
            Some(said) => container(said).height(160.0).width(Length::Fill).into(),
            None => grid(
                ground.catalog.friends.iter().map(|friend| container(friend_line(ground, friend)).padding([6, 8]).style(ui::box_faded(theme::slab)).into()).collect(),
                columns_for(ground, 360.0),
                GRID_GAP,
            ),
        };
        return spread(column![switch, body].spacing(16).into());
    }
    if ground.catalog.people.is_empty() {
        return spread(column![switch, container(empty(w.t("community-no-group"))).height(160.0)].spacing(16).into());
    }
    let order = ground.catalog.ranked(Board::Pp);
    let cards: Vec<Element<'a, Message>> = order.iter().enumerate().map(|(place, at)| person_card(ground, *at, &ground.catalog.people[*at], place + 1)).collect();
    spread(column![switch, grid(cards, columns_for(ground, 420.0), 14.0)].spacing(16).into())
}

fn spread<'a>(inside: Element<'a, Message>) -> Element<'a, Message> {
    scrollable(container(inside).width(Length::Fill).padding(Padding { top: 12.0, right: 40.0, bottom: 28.0, left: 40.0 }))
        .style(ui::thin_scroll)
        .direction(ui::hidden_bar())
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

pub(crate) fn backdrop<'a>(handle: Option<&image::Handle>, high: f32, radius: f32, tint: Color, across: bool) -> Element<'a, Message> {
    let k = ui::fade();
    let angle = if across { std::f32::consts::FRAC_PI_2 } else { std::f32::consts::PI };
    let Some(handle) = handle else {
        return container(Space::new().width(Length::Fill).height(high))
            .style(move |_| container::Style {
                background: Some(Background::Gradient(iced::Gradient::Linear(
                    iced::gradient::Linear::new(iced::Radians(angle)).add_stop(0.0, Color { a: 0.03 * k, ..tint }).add_stop(1.0, Color { a: 0.0, ..tint }),
                ))),
                border: Border { radius: radius.into(), ..Border::default() },
                ..container::Style::default()
            })
            .into();
    };
    let ground = Color::from_rgb(0.047, 0.027, 0.035);
    let (first, middle, last) = if across { (0.9, 0.94, 0.98) } else { (0.8, 0.9, 0.98) };
    let shade = container(Space::new().width(Length::Fill).height(high)).style(move |_| container::Style {
        background: Some(Background::Gradient(iced::Gradient::Linear(
            iced::gradient::Linear::new(iced::Radians(angle))
                .add_stop(0.0, Color { a: first * k, ..ground })
                .add_stop(0.5, Color { a: middle * k, ..ground })
                .add_stop(1.0, Color { a: last * k, ..ground }),
        ))),
        border: Border { radius: radius.into(), ..Border::default() },
        ..container::Style::default()
    });
    stack![image(handle.clone()).content_fit(iced::ContentFit::Cover).width(Length::Fill).height(high).border_radius(radius).opacity(k), shade].width(Length::Fill).height(high).into()
}

pub(crate) fn medal(place: usize) -> Option<Color> {
    match place {
        1 => Some(theme::GRADE_S),
        2 => Some(Color::from_rgb8(200, 204, 220)),
        3 => Some(Color::from_rgb8(205, 127, 50)),
        _ => None,
    }
}

pub(crate) fn ringed<'a>(ground: &Ground<'a>, person: &Person, side: f32, ring: Color) -> Element<'a, Message> {
    let k = ui::fade();
    container(face(ground, person, side))
        .padding(2)
        .style(move |_| container::Style {
            border: Border { color: Color { a: ring.a * k, ..ring }, width: 2.0, radius: (side / 2.0 + 2.0).into() },
            ..container::Style::default()
        })
        .into()
}

struct Standings {
    order: Vec<(usize, f64)>,
    out: usize,
}

fn standings(catalog: &Catalog, board: Board, standing: Standing) -> Standings {
    let at = board.index();
    match standing {
        Standing::General => {
            let order: Vec<(usize, f64)> = catalog.ranked(board).into_iter().map(|who| (who, board.value(&catalog.people[who]))).filter(|(_, value)| *value > 0.0).collect();
            let out = catalog.people.len() - order.len();
            Standings { order, out }
        }
        Standing::Adaptive => {
            let mut order: Vec<(usize, f64)> = (0..catalog.people.len()).map(|who| (who, catalog.people[who].gained[at])).filter(|(_, gained)| *gained > 0.0).collect();
            order.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| board.value(&catalog.people[b.0]).total_cmp(&board.value(&catalog.people[a.0]))));
            let out = catalog.people.len() - order.len();
            Standings { order, out }
        }
    }
}

fn grown(words: &Words, board: Board, value: f64, plus: bool) -> String {
    let sign = if plus { "+" } else { "" };
    match board {
        Board::Pp => format!("{sign}{} pp", words.lang().group(value.round() as u64)),
        Board::Accuracy => format!("{sign}{} {}", decimal(words, value as f32, 2), words.t("board-acc-unit")),
        Board::Plays | Board::Score => format!("{sign}{}", words.lang().group(value.round() as u64)),
        Board::Hours => {
            let minutes = (value / 60.0).round() as u64;
            match (minutes / 60, minutes % 60) {
                (0, m) => format!("{sign}{m} {}", words.t("minutes-short")),
                (h, 0) => format!("{sign}{h} {}", words.t("hours-short")),
                (h, m) => format!("{sign}{h} {} {m:02} {}", words.t("hours-short"), words.t("minutes-short")),
            }
        }
        Board::HitsPerPlay => decimal(words, value as f32, 1),
    }
}

fn whole(words: &Words, board: Board, person: &Person) -> String {
    match board {
        Board::Pp => words.lang().group(u64::from(person.pp)),
        _ => shown_value(words, board, person),
    }
}

struct Said {
    value: String,
    sub: String,
    note: Option<String>,
    moved: Option<Option<i32>>,
}

fn said_for(ground: &Ground<'_>, list: &Standings, place: usize, who: usize, value: f64) -> Said {
    let w = ground.words;
    let board = ground.board;
    let person = &ground.catalog.people[who];
    match ground.standing {
        Standing::General => Said {
            value: shown_value(w, board, person),
            sub: if board == Board::Pp { rank_of(w, person.rank) } else { String::new() },
            note: None,
            moved: None,
        },
        Standing::Adaptive => {
            let was = person.was[board.index()];
            let note = (person.you && place > 1)
                .then(|| list.order.get(place - 2).map(|above| above.1 - value))
                .flatten()
                .filter(|gap| *gap > 0.0)
                .map(|gap| w.with("board-gap", &[("value", grown(w, board, gap, false)), ("place", (place - 1).to_string())]));
            Said {
                value: grown(w, board, value, board != Board::HitsPerPlay),
                sub: w.with("board-total", &[("value", whole(w, board, person))]),
                note,
                moved: Some((was > 0).then(|| was as i32 - place as i32)),
            }
        }
    }
}

fn movement<'a>(ground: &Ground<'a>, shift: Option<Option<i32>>) -> Element<'a, Message> {
    let k = ui::fade();
    match shift {
        None => Space::new().width(0.0).height(0.0).into(),
        Some(None) => container(text(ground.words.t("board-new")).font(theme::MONO_BOLD).size(10.0).color(ui::faded(theme::HIT_100)))
            .padding([2, 7])
            .style(move |_| container::Style {
                background: Some(Background::Color(Color { a: 0.14 * k, ..theme::HIT_100 })),
                border: Border { radius: 8.0.into(), ..Border::default() },
                ..container::Style::default()
            })
            .into(),
        Some(Some(0)) => ui::mono_small("—".to_owned(), FAINT),
        Some(Some(by)) => moved(by),
    }
}

fn row_style(you: bool, frame: Option<Color>) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let edge = if you { ACCENT } else { frame.unwrap_or(Color::WHITE) };
        let strength = match (you || frame.is_some(), lit) {
            (true, true) => 0.9,
            (true, false) => 0.6,
            (false, true) => 0.14,
            (false, false) => 0.06,
        };
        button::Style {
            background: Some(Background::Color(Color::from_rgba(0.055, 0.025, 0.033, 0.96))),
            text_color: INK,
            border: Border { color: Color { a: strength, ..edge }, width: if you || frame.is_some() { 1.5 } else { 1.0 }, radius: 14.0.into() },
            shadow: match frame.or(you.then_some(ACCENT)) {
                Some(glow) if lit || frame.is_some() => Shadow { color: Color { a: 0.22, ..glow }, offset: iced::Vector::ZERO, blur_radius: 18.0 },
                _ => Shadow::default(),
            },
            snap: true,
        }
    }
}

fn podium_card<'a>(ground: &Ground<'a>, list: &Standings, place: usize, who: usize, value: f64, high: f32) -> Element<'a, Message> {
    let person = &ground.catalog.people[who];
    let colour = medal(place).unwrap_or(INK);
    let said = said_for(ground, list, place, who, value);
    let side = if place == 1 { 68.0 } else { 56.0 };
    let value_colour = if ground.standing == Standing::Adaptive { theme::HIT_100 } else { INK };
    let k = ui::fade();
    let badge = container(text(place.to_string()).font(theme::MONO_BOLD).size(12.0).color(Color { a: k, ..Color::from_rgb(0.08, 0.04, 0.05) }))
        .width(22.0)
        .height(22.0)
        .center(22.0)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color { a: k, ..colour })),
            border: Border { radius: 11.0.into(), ..Border::default() },
            ..container::Style::default()
        });
    let mut inside = column![
        stack![ringed(ground, person, side, colour), container(badge).width(side + 8.0).height(side + 8.0).align_x(iced::alignment::Horizontal::Right).align_y(iced::alignment::Vertical::Bottom)],
        row![container(text(person.name.clone()).font(theme::SANS_SEMI).size(16.0).wrapping(text::Wrapping::None).color(ui::faded(INK))).clip(true), flag(ground, &person.country, 11.0)].spacing(6).align_y(iced::Center),
        title_line(ground, person, 11.5),
        Space::new().height(4.0),
        text(said.value).font(theme::SANS_SEMI).size(if place == 1 { 22.0 } else { 19.0 }).wrapping(text::Wrapping::None).color(ui::faded(value_colour)),
    ]
    .spacing(3)
    .align_x(iced::alignment::Horizontal::Center);
    if !said.sub.is_empty() {
        inside = inside.push(ui::mono_small(said.sub, MUTED));
    }
    if let Some(note) = said.note {
        inside = inside.push(text(note).font(theme::SANS).size(11.5).color(ui::faded(Color::from_rgb(0.941, 0.408, 0.408))));
    }
    if said.moved.is_some() {
        inside = inside.push(container(movement(ground, said.moved)).padding(Padding::ZERO.top(4.0)));
    }
    let content = container(inside).width(Length::Fill).height(high).padding([16, 12]).center_x(Length::Fill).align_y(iced::alignment::Vertical::Bottom);
    let cover = ground.pictures.get(&person.cover);
    button(stack![backdrop(cover, high, 14.0, colour, false), content])
        .padding(0)
        .width(Length::FillPortion(1))
        .style(ui::button_faded(row_style(person.you, Some(colour))))
        .on_press(Message::Person(Some(who)))
        .into()
}

fn board_row<'a>(ground: &Ground<'a>, list: &Standings, place: Option<usize>, who: usize, value: f64) -> Element<'a, Message> {
    let high = 60.0;
    let person = &ground.catalog.people[who];
    let said = match place {
        Some(place) => said_for(ground, list, place, who, value),
        None => Said { value: "—".to_owned(), sub: ground.words.with("board-total", &[("value", whole(ground.words, ground.board, person))]), note: Some(ground.words.t("board-not-played")), moved: None },
    };
    let value_colour = if ground.standing == Standing::Adaptive && place.is_some() { theme::HIT_100 } else { INK };
    let under: Element<'a, Message> = match (&said.note, ground.catalog.shown_title(person)) {
        (Some(note), _) => text(note.clone()).font(theme::SANS).size(11.5).wrapping(text::Wrapping::None).color(ui::faded(Color::from_rgb(0.941, 0.408, 0.408))).into(),
        (None, _) => title_line(ground, person, 11.5),
    };
    let mut line = row![
        container(text(place.map_or_else(|| "—".to_owned(), |p| p.to_string())).font(theme::MONO_BOLD).size(14.0).color(ui::faded(if person.you { ACCENT } else { MUTED }))).width(30.0).align_x(iced::alignment::Horizontal::Center),
        ringed(ground, person, 36.0, if person.you { ACCENT } else { Color::from_rgba(1.0, 1.0, 1.0, 0.16) }),
        container(column![row![text(person.name.clone()).font(theme::SANS_SEMI).size(15.0).wrapping(text::Wrapping::None).color(ui::faded(if person.you { Color::from_rgb(0.941, 0.408, 0.408) } else { INK })), flag(ground, &person.country, 11.0)].spacing(7).align_y(iced::Center), under].spacing(2))
            .width(Length::Fill)
            .clip(true),
    ]
    .spacing(12)
    .align_y(iced::Center);
    if ground.standing == Standing::Adaptive {
        line = line.push(container(movement(ground, said.moved)).width(54.0).center_x(54.0));
    }
    let mut numbers = column![text(said.value).font(theme::SANS_SEMI).size(15.0).wrapping(text::Wrapping::None).color(ui::faded(value_colour))].align_x(iced::alignment::Horizontal::Right).spacing(1);
    if !said.sub.is_empty() {
        numbers = numbers.push(ui::mono_small(said.sub, MUTED));
    }
    line = line.push(container(numbers).width(170.0).align_x(iced::alignment::Horizontal::Right));
    let content = container(line).width(Length::Fill).height(high).padding([0, 16]).center_y(high);
    let cover = ground.pictures.get(&person.cover);
    button(stack![backdrop(cover, high, 14.0, avatar_colour(&person.name), true), content])
        .padding(0)
        .width(Length::Fill)
        .style(ui::button_faded(row_style(person.you, None)))
        .on_press(Message::Person(Some(who)))
        .into()
}

pub(crate) fn segmented<'a>(parts: Vec<(String, bool, Message)>) -> Element<'a, Message> {
    let k = ui::fade();
    let mut line = row![].spacing(2);
    for (label, on, message) in parts {
        line = line.push(
            button(container(text(label).font(theme::SANS_SEMI).size(12.5)).center_x(Length::Fill))
                .padding([6, 0])
                .width(120.0)
                .style(ui::button_faded(move |_, status: button::Status| button::Style {
                    background: Some(Background::Color(if on {
                        Color::from_rgba(0.886, 0.282, 0.282, 0.2)
                    } else if matches!(status, button::Status::Hovered) {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.04)
                    } else {
                        Color::TRANSPARENT
                    })),
                    text_color: if on { INK } else { MUTED },
                    border: Border { color: if on { Color::from_rgba(0.886, 0.282, 0.282, 0.5) } else { Color::TRANSPARENT }, width: 1.0, radius: 8.0.into() },
                    shadow: Shadow::default(),
                    snap: true,
                }))
                .on_press(message),
        );
    }
    container(line)
        .padding(3)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.3 * k))),
            border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.06 * k), width: 1.0, radius: 10.0.into() },
            ..container::Style::default()
        })
        .into()
}

fn boards<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let catalog = ground.catalog;
    let mut chips = row![].spacing(6);
    for board in Board::ALL {
        chips = chips.push(
            button(text(w.t(board.key())).font(theme::SANS_SEMI).size(theme::CAPTION))
                .padding([5, 12])
                .style(ui::button_faded(theme::pill(ground.board == board)))
                .on_press(Message::Board(board)),
        );
    }
    let list = standings(catalog, ground.board, ground.standing);
    let adaptive = ground.standing == Standing::Adaptive;
    let title = w.t(if adaptive { "board-title-adaptive" } else { "board-title-general" });
    let subtitle = if adaptive { w.with("board-week-span", &[("week", catalog.week.to_string()), ("span", w.week_span(catalog.week_began))]) } else { w.t("board-all-time") };
    let many = if adaptive { catalog.people.len() } else { list.order.len() };
    let mut who = w.count("participants", many as u64);
    if adaptive && list.out > 0 {
        who = format!("{who} · {}", w.n("board-sat-out", list.out as u64));
    }
    let head = row![
        column![text(title).font(theme::SANS_SEMI).size(19.0).color(ui::faded(INK)), ui::mono_small(subtitle, MUTED)].spacing(3),
        ui::grow(),
        column![text(w.t(ground.board.key())).font(theme::SANS_SEMI).size(14.0).color(ui::faded(INK)), ui::mono_small(who, FAINT)].spacing(3).align_x(iced::alignment::Horizontal::Right),
    ]
    .align_y(iced::Center);
    let mut page = column![
        container(segmented(vec![
            (w.t("board-general"), ground.standing == Standing::General, Message::Standing(Standing::General)),
            (w.t("board-adaptive"), ground.standing == Standing::Adaptive, Message::Standing(Standing::Adaptive)),
        ]))
        .center_x(Length::Fill),
        container(chips).center_x(Length::Fill),
        container(head).padding(Padding { top: 8.0, right: 4.0, bottom: 2.0, left: 4.0 }),
    ]
    .spacing(12)
    .width(Length::Fill)
    .max_width(920.0);
    if adaptive && catalog.collecting {
        let date = chrono::DateTime::from_timestamp(catalog.week_began + 7 * 86_400 + 3 * 3600, 0).map_or_else(String::new, |at| at.format("%d.%m").to_string());
        page = page.push(container(empty(w.with("board-collecting", &[("date", date)]))).height(200.0));
        return rolled(page.into());
    }
    if list.order.is_empty() {
        page = page.push(container(empty(w.t(if adaptive { "board-no-gain" } else { "nothing-yet" }))).height(200.0));
        return rolled(page.into());
    }
    let podium: Vec<(usize, usize, f64)> = list.order.iter().take(3).enumerate().map(|(at, (who, value))| (at + 1, *who, *value)).collect();
    let mut stand = row![].spacing(12).align_y(iced::alignment::Vertical::Bottom);
    for place in [2usize, 1, 3] {
        match podium.iter().find(|(p, _, _)| *p == place) {
            Some((place, who, value)) => {
                let high = match place {
                    1 => 236.0,
                    2 => 214.0,
                    _ => 202.0,
                };
                stand = stand.push(podium_card(ground, &list, *place, *who, *value, high));
            }
            None => stand = stand.push(Space::new().width(Length::FillPortion(1)).height(0.0)),
        }
    }
    page = page.push(stand);
    let mut rows = column![].spacing(8);
    for (at, (who, value)) in list.order.iter().enumerate().skip(3) {
        rows = rows.push(board_row(ground, &list, Some(at + 1), *who, *value));
    }
    let you_out = catalog.people.iter().position(|p| p.you).filter(|you| !list.order.iter().any(|(who, _)| who == you));
    if let Some(you) = you_out {
        rows = rows.push(container(board_row(ground, &list, None, you, 0.0)).padding(Padding::ZERO.top(8.0)));
    }
    page = page.push(rows);
    rolled(page.into())
}

fn faces<'a>(ground: &Ground<'a>, holders: &[usize]) -> Element<'a, Message> {
    if holders.is_empty() {
        return ui::mono_small(ground.words.t("nobody-yet"), FAINT);
    }
    let mut shown = row![].spacing(4).align_y(iced::Center);
    for at in holders.iter().take(5) {
        shown = shown.push(face(ground, &ground.catalog.people[*at], 20.0));
    }
    if holders.len() > 5 {
        shown = shown.push(ui::mono_small(format!("+{}", holders.len() - 5), FAINT));
    }
    shown.into()
}

fn title_card<'a>(ground: &Ground<'a>, title: &Title) -> Element<'a, Message> {
    let w = ground.words;
    let holders = ground.catalog.holders(&title.code);
    let known = !holders.is_empty() || title.rarity != Rarity::Secret;
    let colour = if holders.is_empty() { FAINT } else { title.rarity.colour() };
    let name = if known { title.name(w.lang()).to_owned() } else { "???".to_owned() };
    let about = if known { title.about(w.lang()).to_owned() } else { w.t("secret-title") };
    container(
        column![
            text(name).font(theme::SANS_SEMI).size(15.0).wrapping(text::Wrapping::None).color(ui::faded(colour)),
            text(about).font(theme::SANS).size(12.0).color(ui::faded(MUTED)),
            faces(ground, &holders),
        ]
        .spacing(7),
    )
    .padding([16, 18])
    .width(Length::Fill)
    .style(ui::box_faded(theme::slab))
    .into()
}

fn titles<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let mut list = column![].spacing(22).width(Length::Fill);
    for rarity in Rarity::ALL {
        let defs: Vec<&Title> = ground.catalog.titles.iter().filter(|title| title.rarity == rarity).collect();
        if defs.is_empty() {
            continue;
        }
        let held = defs.iter().filter(|title| !ground.catalog.holders(&title.code).is_empty()).count();
        let head = row![
            text(w.t(rarity.key())).font(theme::SANS_SEMI).size(theme::BODY).color(ui::faded(rarity.colour())),
            ui::mono_small(format!("{} {}", w.t("unlocked"), w.of(held as u64, defs.len() as u64)), FAINT),
        ]
        .spacing(10)
        .align_y(iced::Center);
        let cards: Vec<Element<'a, Message>> = defs.iter().map(|title| title_card(ground, title)).collect();
        list = list.push(column![head, grid(cards, columns_for(ground, 360.0), GRID_GAP)].spacing(10));
    }
    spread(list.into())
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
        face(ground, person, 60.0),
        column![
            text(person.name.clone()).font(theme::SANS_SEMI).size(theme::TITLE).wrapping(text::Wrapping::None).color(ui::faded(INK)),
            row![flag(ground, &person.country, 13.0), title_line(ground, person, theme::CAPTION)].spacing(8).align_y(iced::Center),
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
        kv(w.t("global-rank"), rank_of(w, person.rank)),
        kv(w.t("board-accuracy"), w.percent(f64::from(person.accuracy))),
        kv(w.t("board-plays"), w.lang().group(u64::from(person.plays))),
        kv(w.t("board-hours"), format!("{} {}", w.lang().group(u64::from(person.hours)), w.t("hours-short"))),
        kv(w.t("board-score"), w.lang().group(person.score)),
        kv(w.t("board-hits"), decimal(w, person.hits_per_play, 1)),
    ]
    .spacing(0);
    let mut plays = column![ui::mono_small(w.t("top-plays").to_uppercase(), FAINT)].spacing(8);
    for play in &person.top {
        plays = plays.push(play_row(ground, play, 64.0, 36.0, 34, true));
    }
    let mut held: Vec<&Title> = person.titles.iter().filter_map(|code| ground.catalog.title_of(code)).collect();
    held.sort_by_key(|title| std::cmp::Reverse(Rarity::ALL.iter().position(|rarity| *rarity == title.rarity)));
    let chips: Vec<Element<'a, Message>> = held.iter().map(|title| title_chip(title, lang)).collect();
    let titles = column![ui::mono_small(format!("{} · {}", w.t("community-titles").to_uppercase(), held.len()), FAINT), ui::wrap(chips, 6.0)].spacing(8);
    let figures = match person.streak {
        0 => figures,
        days => figures.push(kv(w.t("streak"), w.n("streak-days", u64::from(days)))),
    };
    let inside = column![head, figures, plays, titles].spacing(22).padding(Padding { top: 24.0, right: 24.0, bottom: 28.0, left: 24.0 });
    let panel = container(scrollable(inside).style(ui::thin_scroll).direction(ui::hidden_bar()).height(Length::Fill)).width(DRAWER_WIDE).height(Length::Fill).style(drawer_style);
    stack![
        mouse_area(ui::veil(Color::from_rgba(0.027, 0.012, 0.016, 0.45))).on_press(Message::Person(None)),
        container(panel).width(Length::Fill).height(Length::Fill).align_x(iced::alignment::Horizontal::Right),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_adaptive_board_ranks_only_the_week_s_gains() {
        let catalog = Catalog::staged(Vec::new(), "", 1_790_000_000);
        let adaptive = standings(&catalog, Board::Pp, Standing::Adaptive);
        let names: Vec<&str> = adaptive.order.iter().map(|(who, _)| catalog.people[*who].name.as_str()).collect();
        assert_eq!(names, ["kotofey", "ssnowy", "NaumRedlo", "Mirrorwave", "d1ce", "tarakan_3000"]);
        assert_eq!(adaptive.out, 2, "those who gained nothing sit out");
        let general = standings(&catalog, Board::Pp, Standing::General);
        assert_eq!(general.order.len(), catalog.people.len());
        assert_eq!(catalog.people[general.order[0].0].name, "kotofey");
    }

    #[test]
    fn a_gain_reads_the_way_the_bot_writes_it() {
        let words = Words::new(crate::lang::Lang::Ru);
        assert_eq!(grown(&words, Board::Pp, 74.4, true), "+74 pp");
        assert_eq!(grown(&words, Board::Accuracy, 0.12, true), "+0,12 п.п.");
        assert_eq!(grown(&words, Board::Hours, 21.0 * 3600.0 + 5.0 * 60.0, true), "+21 ч 05 м");
        assert_eq!(grown(&words, Board::HitsPerPlay, 612.84, false), "612,8");
    }
}
