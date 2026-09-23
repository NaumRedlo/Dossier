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
            Reading::Post(post) => post.image.iter().cloned().collect(),
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
    pub card: Option<&'a crate::community::wire::Card>,
    pub flags: &'a HashMap<String, iced::widget::svg::Handle>,
    pub avatar: Option<&'a image::Handle>,
    pub chat: String,
    pub fetch: Fetch,
    pub width: f32,
    pub filter: crate::chronicle::Filter,
    pub open_events: &'a std::collections::HashSet<String>,
    pub seen: i64,
    pub spot: usize,
    pub spot_k: f32,
    pub rank: usize,
    pub rank_k: f32,
    pub rank_started: std::time::Instant,
    pub rank_held: bool,
    pub metric: crate::dossier::Metric,
    pub span: u32,
    pub grade_hover: Option<usize>,
    pub title_pick: Option<&'a str>,
    pub play_open: Option<usize>,
}

const FEED_WIDE: f32 = 760.0;
const READ_WIDE: f32 = 760.0;
const CARD_WIDE: f32 = 290.0;
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
        .style(ui::thin_scroll)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn hatch<'a>(wide: f32, high: f32) -> Element<'a, Message> {
    container(ui::fine_hatch()).width(wide).height(high).into()
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
    match by {
        0 => ui::mono_small("—".to_owned(), FAINT),
        up if up > 0 => ui::mono_small(format!("▲{up}"), theme::HIT_100),
        down => ui::mono_small(format!("▼{}", -down), ACCENT),
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

pub(crate) fn thumb<'a>(ground: &Ground<'a>, url: Option<&str>, wide: f32, high: f32) -> Element<'a, Message> {
    match url.and_then(|url| ground.pictures.get(url)) {
        Some(handle) => image(handle.clone()).content_fit(iced::ContentFit::Cover).width(wide).height(high).border_radius(6.0).opacity(ui::fade()).into(),
        None => hatch(wide, high),
    }
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

fn stream_colour(stream: &str) -> Color {
    match stream.to_ascii_lowercase().as_str() {
        s if s.contains("lazer") => Color::from_rgb8(0xff, 0x66, 0xab),
        s if s.contains("tachyon") => Color::from_rgb8(0xb0, 0x6c, 0xe8),
        s if s.contains("stable") || s.contains("cutting") || s.contains("beta") => Color::from_rgb8(0x58, 0xae, 0xfc),
        _ => MUTED,
    }
}

pub(crate) fn stream_pill<'a>(stream: &str) -> Element<'a, Message> {
    let colour = stream_colour(stream);
    let k = ui::fade();
    container(text(stream.to_owned()).font(theme::MONO_BOLD).size(10.0).color(ui::faded(colour)))
        .padding([1, 6])
        .style(move |_| container::Style {
            background: Some(Background::Color(Color { a: 0.14 * k, ..colour })),
            border: Border { radius: 4.0.into(), ..Border::default() },
            ..container::Style::default()
        })
        .into()
}

pub(crate) fn people_switch<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let pill = |key: &str, from: PeopleFrom| {
        button(text(w.t(key)).font(theme::SANS_SEMI).size(theme::CAPTION))
            .padding([3, 10])
            .style(ui::button_faded(theme::pill(ground.people_from == from)))
            .on_press(Message::PeopleFrom(from))
    };
    let whose = match ground.people_from {
        PeopleFrom::Chat if !ground.catalog.group.is_empty() => ground.catalog.group.clone(),
        PeopleFrom::Chat => ground.chat.clone(),
        PeopleFrom::Game => w.t("friends-in-osu"),
    };
    row![pill("people-chat", PeopleFrom::Chat), pill("people-game", PeopleFrom::Game), ui::grow(), ui::mono_small(ui::shortened(whose, 22), FAINT)]
        .spacing(6)
        .align_y(iced::Center)
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
                ui::mono_small(friend.country.clone(), FAINT),
            ]
            .spacing(6)
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
    scrollable(container(inside).padding(Padding::ZERO.right(10.0))).style(ui::thin_scroll).width(Length::Fill).height(Length::Fill).into()
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
                if let Some(url) = &post.image {
                    body.push(picture(url));
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
    let wide = CARD_WIDE * 3.0 + GRID_GAP * 2.0;
    let switch = container(people_switch(ground)).max_width(wide);
    if ground.people_from == PeopleFrom::Game {
        let body: Element<'a, Message> = match friends_said(ground) {
            Some(said) => container(said).height(160.0).max_width(wide).into(),
            None => ui::wrap(ground.catalog.friends.iter().map(|friend| container(friend_line(ground, friend)).width(CARD_WIDE).into()).collect(), GRID_GAP).into(),
        };
        return rolled(column![switch, container(body).max_width(wide)].spacing(14).into());
    }
    if ground.catalog.people.is_empty() {
        return rolled(column![switch, container(empty(w.t("community-no-group"))).height(160.0)].spacing(14).into());
    }
    let cards: Vec<Element<'a, Message>> = ground
        .catalog
        .people
        .iter()
        .enumerate()
        .map(|(at, person)| {
            let head = row![
                face(ground, person, 40.0),
                column![
                    row![
                        text(person.name.clone()).font(theme::SANS_SEMI).size(theme::LEAD).wrapping(text::Wrapping::None).color(ui::faded(INK)),
                        ui::mono_small(person.country.clone(), FAINT),
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
                stat(rank_of(w, person.rank), w.t("global-rank")),
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
    rolled(column![switch, container(ui::wrap(cards, GRID_GAP)).max_width(wide)].spacing(14).into())
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
            face(ground, person, 30.0),
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
        face(ground, person, 60.0),
        column![
            text(person.name.clone()).font(theme::SANS_SEMI).size(theme::TITLE).wrapping(text::Wrapping::None).color(ui::faded(INK)),
            row![ui::mono_small(person.country.clone(), FAINT), title_line(ground, person, theme::CAPTION)].spacing(8).align_y(iced::Center),
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
    let panel = container(scrollable(inside).style(ui::thin_scroll).height(Length::Fill)).width(DRAWER_WIDE).height(Length::Fill).style(drawer_style);
    stack![
        mouse_area(ui::veil(Color::from_rgba(0.027, 0.012, 0.016, 0.45))).on_press(Message::Person(None)),
        container(panel).width(Length::Fill).height(Length::Fill).align_x(iced::alignment::Horizontal::Right),
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
