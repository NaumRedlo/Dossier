use std::time::{Duration, Instant};

use iced::widget::{button, column, container, row, scrollable, stack, text, Space};
use iced::{mouse, Background, Border, Color, Element, Length, Padding, Rectangle, Shadow, Theme, Vector};

use crate::community::{Board, Catalog, Happening, Kind, LivePlay, Person};
use crate::community_screen::{self as screen, Ground, Message, Reading, Scored, Section};
use crate::glyphs::{glyph, Icon};
use crate::lanes::{self, Lane};
use crate::news::{self, News};
use crate::theme::{self, ACCENT, FAINT, INK, MUTED};
use crate::ui;

pub const FRESH_GLOW: f32 = 4.0;
pub const READING_BELOW: f32 = 90.0;

#[derive(Debug, Clone, Default)]
pub(crate) struct Arrivals {
    started: std::collections::HashMap<String, Instant>,
}

pub const PICTURE_FADE: f32 = 0.34;

#[derive(Debug, Clone, Default)]
pub(crate) struct Pictures {
    came: std::collections::HashMap<String, Instant>,
    lost: std::collections::HashMap<String, Instant>,
}

impl Pictures {
    pub(crate) fn came(&mut self, url: &str, now: Instant) {
        self.lost.remove(url);
        self.came.insert(url.to_owned(), now);
    }

    pub(crate) fn lost(&mut self, url: &str, now: Instant) {
        self.lost.entry(url.to_owned()).or_insert(now);
    }

    pub(crate) fn settle(&mut self, now: Instant) {
        self.came.retain(|_, at| now.saturating_duration_since(*at).as_secs_f32() < PICTURE_FADE);
    }

    pub(crate) fn animating(&self, now: Instant) -> bool {
        self.came.values().chain(self.lost.values()).any(|at| now.saturating_duration_since(*at).as_secs_f32() < PICTURE_FADE)
    }

    pub(crate) fn shown(&self, now: Instant) -> std::collections::HashMap<String, f32> {
        self.came.iter().filter_map(|(url, at)| {
            let age = now.saturating_duration_since(*at).as_secs_f32();
            (age < PICTURE_FADE).then(|| (url.clone(), ui::appear(age, 0)))
        }).collect()
    }

    pub(crate) fn shown_of(&self, url: &str, now: Instant) -> f32 {
        self.came.get(url).map_or(1.0, |at| ui::appear(now.saturating_duration_since(*at).as_secs_f32(), 0))
    }

    pub(crate) fn room(&self, now: Instant) -> std::collections::HashMap<String, f32> {
        self.lost.iter().map(|(url, at)| (url.clone(), 1.0 - ui::appear(now.saturating_duration_since(*at).as_secs_f32(), 0))).collect()
    }
}

impl Arrivals {
    pub(crate) fn refresh(&mut self, before: &[String], current: &[String], now: Instant) {
        let before: std::collections::HashSet<&String> = before.iter().collect();
        let current_set: std::collections::HashSet<&String> = current.iter().collect();
        self.started.retain(|key, at| current_set.contains(key) && now.saturating_duration_since(*at).as_secs_f32() < FRESH_GLOW);
        for (index, key) in current.iter().filter(|key| !before.contains(key)).enumerate() {
            let delay = Duration::from_secs_f32(ui::STAGGER * index.min(16) as f32);
            self.started.entry(key.clone()).or_insert(now + delay);
        }
    }

    pub(crate) fn ages(&self, now: Instant) -> std::collections::HashMap<String, f32> {
        self.started.iter().filter_map(|(key, at)| {
            let age = now.saturating_duration_since(*at).as_secs_f32();
            (age < FRESH_GLOW).then(|| (key.clone(), age))
        }).collect()
    }

    pub(crate) fn animating(&self, now: Instant) -> bool {
        self.started.values().any(|at| now.saturating_duration_since(*at).as_secs_f32() < FRESH_GLOW)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Frozen {
    pub catalog: Catalog,
    pub news: News,
    pub channels: Vec<String>,
    pub live_shown: usize,
}

impl Frozen {
    pub(crate) fn of(catalog: &Catalog, news: &News, channels: &[String], live_shown: usize) -> Frozen {
        Frozen { catalog: catalog.clone(), news: news.clone(), channels: channels.to_vec(), live_shown }
    }
}

pub(crate) struct Before {
    pub keys: Vec<String>,
    pub frozen: Option<Frozen>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Leaving {
    frozen: Option<Frozen>,
    keys: Vec<String>,
    since: Option<Instant>,
}

#[derive(Clone, Copy)]
pub struct Leave<'a> {
    pub(crate) frozen: &'a Frozen,
    pub(crate) keys: &'a [String],
    pub(crate) open: f32,
}

impl Leaving {
    pub(crate) fn start(&mut self, before: Before, current: &[String], held: &std::collections::HashSet<String>, now: Instant) {
        let Some(frozen) = before.frozen else {
            return;
        };
        let present: std::collections::HashSet<&String> = current.iter().collect();
        let gone: Vec<String> = before.keys.into_iter().filter(|key| !present.contains(key) && !held.contains(key)).collect();
        if gone.is_empty() {
            return;
        }
        self.frozen = Some(frozen);
        self.keys = gone;
        self.since = Some(now);
    }

    fn age(&self, now: Instant) -> Option<f32> {
        self.since.map(|since| now.saturating_duration_since(since).as_secs_f32())
    }

    pub(crate) fn settle(&mut self, now: Instant) {
        if self.age(now).is_some_and(|age| age >= ui::APPEAR) {
            *self = Leaving::default();
        }
    }

    pub(crate) fn animating(&self, now: Instant) -> bool {
        self.age(now).is_some_and(|age| age < ui::APPEAR)
    }

    pub(crate) fn shown(&self, now: Instant) -> Option<Leave<'_>> {
        let age = self.age(now).filter(|age| *age < ui::APPEAR)?;
        Some(Leave { frozen: self.frozen.as_ref()?, keys: &self.keys, open: 1.0 - ui::appear(age, 0) })
    }

    #[cfg(test)]
    pub(crate) fn busy(&self) -> bool {
        self.since.is_some()
    }
}

const PAGE_GAP: f32 = 12.0;
const ROW_GAP: f32 = 1.0;
const PAGE_BELOW: f32 = 28.0;
const RIGHT_LEAST: f32 = 256.0;
const RIGHT_MOST: f32 = 320.0;
const TIMELINE_LEAST: f32 = 540.0;
const CENTRE_MOST: f32 = 820.0;
const GAP: f32 = 18.0;
const GREEN: Color = theme::HIT_100;
const LINK_BLUE: Color = Color::from_rgb(0.345, 0.682, 0.988);
const PINK: Color = Color::from_rgb(1.0, 0.4, 0.671);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Filter {
    #[default]
    All,
    Plays,
    Titles,
    Ranks,
    News,
}

impl Filter {
    pub const ALL: [Filter; 5] = [Filter::All, Filter::Plays, Filter::Titles, Filter::Ranks, Filter::News];

    pub fn key(self) -> &'static str {
        match self {
            Filter::All => "filter-all",
            Filter::Plays => "filter-plays",
            Filter::Titles => "filter-titles",
            Filter::Ranks => "filter-ranks",
            Filter::News => "filter-news",
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
    ghost: bool,
}

fn person_mark(catalog: &Catalog, at: usize) -> String {
    catalog.people.get(at).map_or_else(|| format!("#{at}"), |person| person.id.to_string())
}

fn map_mark(catalog: &Catalog, at: Option<usize>) -> String {
    catalog.map(at).map_or_else(|| at.map_or_else(String::new, |at| format!("#{at}")), |map| map.hash.clone())
}

fn kind_mark(kind: &Kind) -> String {
    match kind {
        Kind::Render { .. } => "render".to_owned(),
        Kind::TopPlay { place, .. } => format!("top{place}"),
        Kind::Title(code) => format!("title-{code}"),
        Kind::Climb { board, .. } => format!("climb-{}", board.key()),
    }
}

impl Event<'_> {
    fn key(&self, catalog: &Catalog) -> String {
        match self.item {
            Item::Play(play) => format!("play:{}:{}:{}:{}", person_mark(catalog, play.who), map_mark(catalog, Some(play.map)), play.at, (play.pp * 100.0).round()),
            Item::Happened(happened) => format!("happened:{}:{}:{}:{}", person_mark(catalog, happened.who), happened.at, kind_mark(&happened.kind), map_mark(catalog, happened.map)),
            Item::Story(story) => format!("story:{}", story.url),
            Item::Post(post) => format!("post:{}", post.url),
            Item::Build(build) => format!("build:{}", build.url),
        }
    }

    fn admitted(&self, filter: Filter) -> bool {
        match (filter, self.item) {
            (Filter::All, _) => true,
            (Filter::News, Item::Story(_) | Item::Post(_) | Item::Build(_)) => true,
            (Filter::Plays, Item::Play(_)) => true,
            (Filter::Plays, Item::Happened(happened)) => matches!(happened.kind, Kind::TopPlay { .. } | Kind::Render { .. }),
            (Filter::Titles, Item::Happened(happened)) => matches!(happened.kind, Kind::Title(_)),
            (Filter::Ranks, Item::Happened(happened)) => matches!(happened.kind, Kind::Climb { .. }),
            _ => false,
        }
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

const POSTS_EACH: usize = 20;

fn chosen_posts<'a>(news: &'a News, channels: &[String]) -> impl Iterator<Item = &'a news::Post> + 'a {
    let channels: Vec<String> = channels.iter().map(|c| c.to_ascii_lowercase()).collect();
    let mut taken: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    news.posts.iter().filter(move |post| {
        let channel = post.channel.to_ascii_lowercase();
        if !channels.contains(&channel) {
            return false;
        }
        let count = taken.entry(channel).or_insert(0);
        *count += 1;
        *count <= POSTS_EACH
    })
}

fn events<'a>(catalog: &'a Catalog, news: &'a News, channels: &[String], live_shown: usize) -> Vec<Event<'a>> {
    let shown = live_shown.min(catalog.live.len());
    let mut out: Vec<Event<'a>> = Vec::new();
    out.extend(catalog.live[..shown].iter().map(|play| Event { at: play.at, item: Item::Play(play), ghost: false }));
    out.extend(catalog.feed.iter().map(|happened| Event { at: happened.at, item: Item::Happened(happened), ghost: false }));
    out.extend(news.stories.iter().take(12).map(|story| Event { at: story.at, item: Item::Story(story), ghost: false }));
    out.extend(chosen_posts(news, channels).map(|post| Event { at: post.at, item: Item::Post(post), ghost: false }));
    out.extend(news.builds.iter().take(8).map(|build| Event { at: build.at, item: Item::Build(build), ghost: false }));
    out.sort_by(|a, b| b.at.cmp(&a.at));
    out.truncate(90);
    out
}

pub fn newest(catalog: &Catalog, news: &News, channels: &[String], live_shown: usize) -> i64 {
    events(catalog, news, channels, live_shown).first().map_or(0, |event| event.at)
}

pub(crate) fn event_keys(catalog: &Catalog, news: &News, channels: &[String], live_shown: usize) -> Vec<String> {
    events(catalog, news, channels, live_shown).iter().map(|event| event.key(catalog)).collect()
}

pub fn card<'a>(inside: impl Into<Element<'a, Message>>, padding: impl Into<Padding>) -> container::Container<'a, Message> {
    container(inside).padding(padding).width(Length::Fill).style(ui::box_faded(|theme: &Theme| container::Style { border: Border { radius: 14.0.into(), ..theme::slab(theme).border }, ..theme::slab(theme) }))
}

fn caption<'a>(words: String) -> Element<'a, Message> {
    ui::mono_small(words.to_uppercase(), FAINT)
}

fn day_divider<'a>(ground: &Ground<'a>, at: i64, counts: [usize; 3]) -> Element<'a, Message> {
    let w = ground.words;
    let said = match w.days_back(at, ground.now_unix) {
        0 => w.t("day-today"),
        1 => w.t("day-yesterday"),
        _ => w.day(at, ground.now_unix),
    };
    let k = ui::fade();
    let mut summary = row![].spacing(12);
    for (count, key) in counts.into_iter().zip(["day-plays-n", "day-records-n", "day-titles-n"]) {
        if count > 0 {
            summary = summary.push(ui::mono_small(w.n(key, count as u64), FAINT));
        }
    }
    row![
        caption(said),
        container(Space::new().height(1.0)).width(Length::Fill).style(move |_| container::Style { background: Some(Background::Color(Color { a: theme::LINE.a * k, ..theme::LINE })), ..container::Style::default() }),
        summary,
    ]
    .spacing(12)
    .align_y(iced::Center)
    .padding([4, 4])
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
    column![shown, iced::widget::Row::with_children(facts.into_iter().map(|fact| ui::mono_small(fact, MUTED))).spacing(14)].spacing(8).into()
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

#[derive(Clone, Copy)]
struct Table {
    cover: (f32, f32),
    right: f32,
}

impl Table {
    fn for_width(wide: f32) -> Table {
        if wide >= 600.0 {
            Table { cover: (104.0, 56.0), right: 150.0 }
        } else {
            Table { cover: (88.0, 48.0), right: 120.0 }
        }
    }

    fn high(self) -> f32 {
        self.cover.1 + 14.0
    }

    fn news_high(self) -> f32 {
        self.cover.1 + 16.0
    }
}

const CORAL: Color = Color::from_rgb(0.941, 0.408, 0.408);
const ROW_RADIUS: f32 = 12.0;
const COVER_RADIUS: f32 = 8.0;

fn blend(tint: Color, share: f32, alpha: f32) -> Color {
    let base = theme::GROUND;
    Color { r: base.r + (tint.r - base.r) * share, g: base.g + (tint.g - base.g) * share, b: base.b + (tint.b - base.b) * share, a: alpha }
}

fn title_and_version(ground: &Ground<'_>, map: usize) -> (String, String) {
    let whole = map_title(ground, map);
    match whole.rfind(" [") {
        Some(at) if whole.ends_with(']') => (whole[..at].to_owned(), whole[at..].to_owned()),
        _ => (whole, String::new()),
    }
}

fn chip<'a>(inside: Element<'a, Message>) -> Element<'a, Message> {
    let k = ui::fade();
    container(inside)
        .padding([2, 6])
        .style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba8(0x14, 0x08, 0x0b, 0.9 * k))), border: Border { radius: 6.0.into(), ..Border::default() }, ..container::Style::default() })
        .into()
}

fn grade_chip<'a>(grade: &str) -> Element<'a, Message> {
    chip(text(grade.to_owned()).font(theme::SANS_SEMI).size(12.0).wrapping(text::Wrapping::None).color(ui::faded(screen::grade_colour(grade))).into())
}

fn corner<'a>(base: Element<'a, Message>, badge: Option<Element<'a, Message>>, table: Table, high: f32) -> Element<'a, Message> {
    match badge {
        Some(badge) => stack![base, container(badge).width(table.cover.0).height(high).align_x(iced::alignment::Horizontal::Right).align_y(iced::alignment::Vertical::Bottom).padding(5)]
            .width(table.cover.0)
            .height(high)
            .into(),
        None => base,
    }
}

fn cover_box<'a>(ground: &Ground<'a>, map: usize, table: Table, badge: Option<Element<'a, Message>>) -> Element<'a, Message> {
    let (wide, high) = table.cover;
    let shown = ground.catalog.map(Some(map)).filter(|map| !ground.thumbs.contains_key(&map.hash)).and_then(|map| map.card()).map_or(1.0, |card| ground.picture_shown(&card));
    let waiting = || -> Element<'a, Message> { container(ui::fine_hatch()).width(wide).height(high).into() };
    let base: Element<'a, Message> = match map_cover(ground, map) {
        Some(handle) => {
            let cover: Element<'a, Message> = iced::widget::image(crate::crops::shaded(handle, wide, high, COVER_RADIUS)).content_fit(iced::ContentFit::Fill).width(wide).height(high).opacity(ui::fade() * shown).into();
            if shown >= 0.999 { cover } else { stack![waiting(), cover].width(wide).height(high).into() }
        }
        None => waiting(),
    };
    corner(base, badge, table, high)
}

fn tile<'a>(wide: f32, high: f32, colour: Color, icon: Icon) -> Element<'a, Message> {
    let k = ui::fade();
    container(glyph(icon, 22.0, colour))
        .center_x(wide)
        .center_y(high)
        .style(move |_| container::Style {
            background: Some(Background::Color(blend(colour, 0.13, k))),
            border: Border { color: blend(colour, 0.3, k), width: 1.0, radius: COVER_RADIUS.into() },
            ..container::Style::default()
        })
        .into()
}

fn when_text(ground: &Ground<'_>, at: i64) -> String {
    if (0..3600).contains(&(ground.now_unix - at)) { screen::since(ground, at) } else { ground.words.clock(at) }
}

fn name_line<'a>(ground: &Ground<'a>, person: &Person, who: usize, at: i64) -> Element<'a, Message> {
    let name = button(text(person.name.clone()).font(theme::SANS_SEMI).size(14.5).wrapping(text::Wrapping::None))
        .padding(0)
        .style(ui::button_faded(|_, status| button::Style {
            background: None,
            text_color: if matches!(status, button::Status::Hovered | button::Status::Pressed) { ACCENT } else { INK },
            border: Border::default(),
            shadow: Shadow::default(),
            snap: true,
        }))
        .on_press(Message::Person(Some(who)));
    row![screen::with_mark(ground, person, name, 14.0), screen::flag(ground, &person.country, 10.0), ui::mono_small(when_text(ground, at), FAINT)].spacing(8).align_y(iced::Center).into()
}

const SPACE: f32 = 4.0;

fn sentence<'a>(verb: String, objects: Vec<ui::Piece>) -> Element<'a, Message> {
    let mut pieces = vec![ui::piece(verb, theme::SANS, 13.5, MUTED)];
    pieces.extend(objects.into_iter().enumerate().map(|(at, object)| if at == 0 { object.after(SPACE) } else { object }));
    ui::marquee(pieces).into()
}

fn big_value<'a>(said: String) -> Element<'a, Message> {
    text(said).font(theme::SANS_SEMI).size(18.0).wrapping(text::Wrapping::None).color(ui::faded(INK)).into()
}

fn play_right<'a>(ground: &Ground<'a>, pp: Option<String>, accuracy: f32, mods: &[String]) -> Element<'a, Message> {
    let said = ground.words.percent(f64::from(accuracy));
    let top: Element<'a, Message> = match pp {
        Some(pp) => row![ui::mono_small(said, MUTED), big_value(pp)].spacing(10).align_y(iced::alignment::Vertical::Bottom).into(),
        None => big_value(said),
    };
    let mut right = column![top].spacing(3).align_x(iced::alignment::Horizontal::Right);
    if mods.iter().any(|acronym| crate::modicons::shown(acronym)) {
        right = right.push(screen::quiet_mods(mods));
    }
    right.into()
}

fn shell<'a>(lead: Element<'a, Message>, name: Element<'a, Message>, under: Element<'a, Message>, right: Element<'a, Message>, table: Table) -> Element<'a, Message> {
    row![lead, column![name, under].spacing(5).width(Length::Fill), container(right).width(table.right).align_x(iced::alignment::Horizontal::Right)].spacing(16).align_y(iced::Center).into()
}

fn row_hover(_: &Theme, _: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: INK,
        border: Border { radius: ROW_RADIUS.into(), ..Border::default() },
        shadow: Shadow::default(),
        snap: true,
    }
}

fn journal_row<'a>(ground: &Ground<'a>, event: &Event<'a>, table: Table) -> Option<Element<'a, Message>> {
    let w = ground.words;
    let catalog = ground.catalog;
    let key = event.key(ground.catalog);
    let (line, press, details): (Element<'a, Message>, Message, Option<(f32, Element<'a, Message>)>) = match event.item {
        Item::Play(play) => {
            let person = catalog.people.get(play.who)?;
            let grade = if play.passed { play.grade.clone() } else { "F".to_owned() };
            let pp = (play.passed && play.pp >= 0.5).then(|| format!("{} PP", screen::decimal(w, play.pp, 0)));
            let press = Scored::of(catalog, play.who, &play.more, play.passed).map_or(Message::Person(Some(play.who)), |scored| Message::Read(Reading::Score(scored)));
            let (title, version) = title_and_version(ground, play.map);
            let under = ui::marquee(vec![ui::piece(title, theme::SANS, 13.5, INK), ui::piece(version, theme::SANS, 13.5, MUTED)]);
            (shell(cover_box(ground, play.map, table, Some(grade_chip(&grade))), name_line(ground, person, play.who, event.at), under.into(), play_right(ground, pp, play.accuracy, &play.mods), table), press, None)
        }
        Item::Happened(happened) => {
            let person = catalog.people.get(happened.who)?;
            match &happened.kind {
                Kind::Render { accuracy, mods } => {
                    let map = happened.map?;
                    let (title, version) = title_and_version(ground, map);
                    let under = ui::marquee(vec![ui::piece(title, theme::SANS, 13.5, INK), ui::piece(version, theme::SANS, 13.5, MUTED)]);
                    let badge = chip(glyph(Icon::Film, 12.0, MUTED));
                    (shell(cover_box(ground, map, table, Some(badge)), name_line(ground, person, happened.who, event.at), under.into(), play_right(ground, None, *accuracy, mods), table), Message::Person(Some(happened.who)), None)
                }
                Kind::TopPlay { pp, place, more, .. } => {
                    let map = happened.map?;
                    let fold = ground.folds.get(&key).copied().unwrap_or(if ground.open_events.contains(&key) { 1.0 } else { 0.0 });
                    let details = (fold > 0.001).then(|| (fold, counts_row(ground, more.counts, more.combo.zip(more.max_combo), more.stars)));
                    let right = column![big_value(format!("{} PP", screen::decimal(w, *pp, 0))), ui::mono_small(w.n("top-place", u64::from(*place)), CORAL)].spacing(3).align_x(iced::alignment::Horizontal::Right);
                    let press = Scored::of(catalog, happened.who, more, true).map_or(Message::Toggle(key), |scored| Message::Read(Reading::Score(scored)));
                    let lead = tile(table.cover.0, table.cover.1, CORAL, Icon::Star);
                    let (title, version) = title_and_version(ground, map);
                    let under = sentence(w.t("event-top"), vec![ui::piece(title, theme::SANS, 13.5, INK), ui::piece(version, theme::SANS, 13.5, MUTED)]);
                    (shell(lead, name_line(ground, person, happened.who, event.at), under, right.into(), table), press, details)
                }
                Kind::Title(code) => {
                    let title = catalog.title_of(code)?;
                    let colour = title.rarity.colour();
                    let lead = tile(table.cover.0, table.cover.1, colour, Icon::Trophy);
                    let under = sentence(w.t("event-title"), vec![ui::piece(title.name(w.lang()).to_owned(), theme::SANS_SEMI, 13.5, colour)]);
                    let holders = catalog.holders(&title.code).len();
                    let held = ui::mono_small(format!("{} {}", w.t("held-by"), w.of(holders as u64, catalog.people.len() as u64)), colour);
                    (shell(lead, name_line(ground, person, happened.who, event.at), under, held, table), Message::TitleOf(code.clone(), Some(person.id)), None)
                }
                Kind::Climb { board, from, to } => {
                    let lead = tile(table.cover.0, table.cover.1, GREEN, Icon::Up);
                    let under = sentence(w.n("event-climb-n", u64::from(from.abs_diff(*to))), vec![ui::piece(if *board == Board::Pp { w.t(board.key()) } else { w.t(board.key()).to_lowercase() }, theme::SANS, 13.5, INK)]);
                    (shell(lead, name_line(ground, person, happened.who, event.at), under, ui::mono_small(format!("#{from} → #{to}"), GREEN), table), Message::Board(*board), None)
                }
            }
        }
        _ => return None,
    };
    let mut inside = column![container(line).height(table.high()).center_y(table.high()).padding([0, 12])];
    if let Some((fold, details)) = details {
        inside = inside.push(ui::reveal(fold, || container(details).padding(Padding { top: 2.0, right: 12.0, bottom: 10.0, left: 12.0 + table.cover.0 + 16.0 }).into()));
    }
    Some(row_card(ground, event, inside.into(), press))
}

fn row_card<'a>(ground: &Ground<'a>, event: &Event<'a>, inside: Element<'a, Message>, press: Message) -> Element<'a, Message> {
    let glow = glow_of(ground, event) * ui::fade();
    let row = ui::hover(button(inside).padding(0).width(Length::Fill).style(ui::button_faded(row_hover)).on_press(press), ui::Glow::row(ROW_RADIUS));
    container(row)
        .style(move |_| container::Style {
            background: (glow > 0.001).then_some(Background::Color(Color { a: 0.09 * glow, ..ACCENT })),
            border: Border { color: Color { a: 0.35 * glow, ..ACCENT }, width: if glow > 0.001 { 1.0 } else { 0.0 }, radius: ROW_RADIUS.into() },
            ..container::Style::default()
        })
        .into()
}

fn pictured<'a>(ground: &Ground<'a>, url: &str, across: f32, high: f32) -> Option<Element<'a, Message>> {
    let k = ui::fade();
    let waiting = || -> Element<'a, Message> {
        container(Space::new())
            .width(across)
            .height(high)
            .style(move |_| container::Style {
                background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.22 * k))),
                border: Border { radius: COVER_RADIUS.into(), ..Border::default() },
                ..container::Style::default()
            })
            .into()
    };
    match ground.pictures.get(url) {
        Some(handle) => {
            let shown = ground.picture_shown(url);
            let picture = iced::widget::image(crate::crops::shaded(handle, across, high, COVER_RADIUS)).content_fit(iced::ContentFit::Fill).width(Length::Fixed(across)).height(high).border_radius(COVER_RADIUS).opacity(k * shown);
            Some(if shown >= 0.999 { picture.into() } else { stack![waiting(), picture].into() })
        }
        None => {
            let room = ground.picture_room(url);
            (room > 0.001).then(|| opening(waiting(), room))
        }
    }
}

fn preview_spans(body: &[news::Block]) -> Vec<news::Span> {
    let mut spans: Vec<news::Span> = Vec::new();
    for block in body.iter().take(2) {
        if let news::Block::Text(piece) | news::Block::Quote(piece) | news::Block::Item(piece) | news::Block::Heading(piece) = block {
            if !spans.is_empty() {
                spans.push(news::Span::plain("\n"));
            }
            spans.extend(piece.iter().cloned());
        }
    }
    spans
}

fn news_row<'a>(ground: &Ground<'a>, event: &Event<'a>, table: Table) -> Option<Element<'a, Message>> {
    let w = ground.words;
    let when = screen::since(ground, event.at);
    let (wide, high) = (table.cover.0, table.news_high());
    let head = |source: String| -> Element<'a, Message> {
        row![text(source).font(theme::SANS_SEMI).size(13.5).wrapping(text::Wrapping::None).color(ui::faded(INK)), ui::mono_small(when.clone(), FAINT)].spacing(8).align_y(iced::Center).into()
    };
    let (lead, header, body, press): (Element<'a, Message>, Element<'a, Message>, Element<'a, Message>, Message) = match event.item {
        Item::Post(post) => {
            let lead: Element<'a, Message> = match (post.image.as_deref(), post.videos.first()) {
                (Some(url), _) => pictured(ground, url, wide, high).unwrap_or_else(|| tile(wide, high, LINK_BLUE, Icon::Send)),
                (None, Some(video)) => screen::video_tile(ground, video, true, high, wide),
                (None, None) => tile(wide, high, LINK_BLUE, Icon::Send),
            };
            let spans = preview_spans(&post.body);
            let words: Element<'a, Message> = if spans.is_empty() { text(ui::shortened(ui::settled(&post.text), 200)).font(theme::SANS).size(13.0).color(ui::faded(MUTED)).into() } else { screen::rich(&spans, MUTED, 13.0) };
            (lead, head(format!("@{}", post.channel)), container(words).max_height(38.0).clip(true).into(), Message::Read(Reading::Post(post.clone())))
        }
        Item::Story(story) => {
            let lead = story.image.as_deref().and_then(|url| pictured(ground, url, wide, high)).unwrap_or_else(|| tile(wide, high, LINK_BLUE, Icon::News));
            let body = column![
                text(ui::settled(&story.title)).font(theme::SANS_SEMI).size(14.0).color(ui::faded(INK)),
                text(ui::shortened(ui::settled(&story.lead), 120)).font(theme::SANS).size(12.5).color(ui::faded(MUTED)),
            ]
            .spacing(3);
            (lead, head("osu!".to_owned()), container(body).max_height(46.0).clip(true).into(), Message::Read(Reading::Story(story.clone())))
        }
        Item::Build(build) => {
            let mut lines = column![].spacing(2);
            for (at, change) in build.changes.iter().take(2).enumerate() {
                lines = lines.push(text(change.title.clone()).font(theme::SANS).size(13.0).wrapping(text::Wrapping::None).color(ui::faded(if at == 0 || change.major { INK } else { MUTED })));
            }
            if build.changes.len() > 2 {
                lines = lines.push(ui::mono_small(w.n("more-changes-n", (build.changes.len() - 2) as u64), FAINT));
            }
            (tile(wide, high, PINK, Icon::Gear), head(format!("{} {}", build.stream, build.version)), container(lines).clip(true).into(), Message::Read(Reading::Build(build.clone())))
        }
        _ => return None,
    };
    let line = row![lead, column![header, body].spacing(5).width(Length::Fill)].spacing(16).align_y(iced::Alignment::Start);
    let inside = container(line).padding([7, 12]);
    Some(row_card(ground, event, inside.into(), press))
}

fn fresh_age(ground: &Ground<'_>, event: &Event<'_>) -> Option<f32> {
    if event.ghost {
        return None;
    }
    ground.arrivals.get(&event.key(ground.catalog)).copied()
}

fn glow_of(ground: &Ground<'_>, event: &Event<'_>) -> f32 {
    if let Some(age) = fresh_age(ground, event) {
        let x = (1.0 - age / FRESH_GLOW).clamp(0.0, 1.0);
        x * x
    } else {
        0.0
    }
}

fn opened(ground: &Ground<'_>, event: &Event<'_>) -> f32 {
    if event.ghost {
        return ground.leaving.map_or(0.0, |leave| leave.open);
    }
    fresh_age(ground, event).map_or(1.0, |age| ui::appear(age, 0))
}

fn opening<'a>(made: Element<'a, Message>, open: f32) -> Element<'a, Message> {
    if open >= 0.999 { made } else { ui::collapsing(made, open) }
}

fn day_counts(list: &[Event<'_>], day: i64, day_of: &dyn Fn(i64) -> i64) -> [usize; 3] {
    let mut counts = [0usize; 3];
    for event in list.iter().filter(|event| !event.ghost && day_of(event.at) == day) {
        match event.item {
            Item::Play(_) => counts[0] += 1,
            Item::Happened(happened) => match happened.kind {
                Kind::Render { .. } => counts[0] += 1,
                Kind::TopPlay { .. } => counts[1] += 1,
                Kind::Title(_) => counts[2] += 1,
                Kind::Climb { .. } => {}
            },
            _ => {}
        }
    }
    counts
}

fn by_day<'a>(ground: &Ground<'a>, list: &[Event<'a>], t: f32, from: usize, draw: impl Fn(&Event<'a>) -> Option<Element<'a, Message>>) -> Vec<Element<'a, Message>> {
    let day_of = |at: i64| ground.words.days_back(at, ground.now_unix);
    let settled_days: std::collections::HashSet<i64> = list.iter().filter(|event| !event.ghost && fresh_age(ground, event).is_none()).map(|event| day_of(event.at)).collect();
    let mut last: Option<i64> = None;
    let mut out = Vec::new();
    for event in list {
        let day = day_of(event.at);
        let open = opened(ground, event);
        let spaced = |made: Element<'a, Message>| -> Element<'a, Message> { container(made).padding(Padding::ZERO.bottom(ROW_GAP)).into() };
        if last != Some(day) {
            let index = from + out.len();
            let counts = day_counts(list, day, &day_of);
            let divider = spaced(ui::appearing(ui::appear(t, index), 10.0, || day_divider(ground, event.at, counts)));
            out.push(if settled_days.contains(&day) { divider } else { opening(divider, open) });
            last = Some(day);
        }
        let k = open.min(ui::appear(t, from + out.len()));
        let made = if k >= 0.999 { draw(event) } else { ui::fading(ui::fade() * k, || draw(event)) };
        if let Some(made) = made {
            out.push(opening(spaced(ui::lifted(made, k, if open >= 0.999 { 10.0 } else { 0.0 })), open));
        }
    }
    out
}

fn refresh_look(pulse: screen::Pulse) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let lit = matches!(status, button::Status::Hovered | button::Status::Pressed) && !matches!(pulse, screen::Pulse::Running | screen::Pulse::Done);
        let (fill, line) = match pulse {
            screen::Pulse::Failed(screen::Fault::Denied) => (if lit { Color::from_rgb8(0xf0, 0x5e, 0x5e) } else { ACCENT }, ACCENT),
            screen::Pulse::Failed(_) => (Color::from_rgba(0.886, 0.282, 0.282, if lit { 0.16 } else { 0.1 }), Color::from_rgba(0.886, 0.282, 0.282, 0.4)),
            _ if lit => (Color::from_rgb8(0x2c, 0x1d, 0x21), Color::from_rgb8(0x55, 0x40, 0x45)),
            _ => (Color::from_rgb8(0x1d, 0x12, 0x16), Color::from_rgb8(0x36, 0x27, 0x2b)),
        };
        button::Style {
            background: Some(Background::Color(fill)),
            text_color: INK,
            border: Border { color: line, width: 1.0, radius: 9.0.into() },
            shadow: Shadow::default(),
            snap: true,
        }
    }
}

const REFRESH_SIDE: f32 = 32.0;

fn refresh_button<'a>(ground: &Ground<'a>) -> Option<Element<'a, Message>> {
    let pulse = screen::pulse(ground)?;
    let w = ground.words;
    let (key, press) = match pulse {
        screen::Pulse::Idle => ("refresh-idle", Some(Message::Again)),
        screen::Pulse::Running => ("refresh-running", None),
        screen::Pulse::Done => ("refresh-done", None),
        screen::Pulse::Failed(screen::Fault::Server) => ("refresh-failed", Some(Message::Again)),
        screen::Pulse::Failed(screen::Fault::Offline) => ("refresh-offline", Some(Message::Again)),
        screen::Pulse::Failed(screen::Fault::Denied) => ("refresh-denied", Some(Message::Reconnect)),
    };
    let mark: Element<'a, Message> = match pulse {
        screen::Pulse::Idle => glyph(Icon::Refresh, 16.0, MUTED),
        screen::Pulse::Running => container(
            container(Space::new()).width(7.0).height(7.0).style(|_| container::Style {
                background: Some(Background::Color(ui::faded(ACCENT))),
                border: Border { radius: 3.5.into(), ..Border::default() },
                ..container::Style::default()
            }),
        )
        .center(16.0)
        .into(),
        screen::Pulse::Done => glyph(Icon::Check, 16.0, ACCENT),
        screen::Pulse::Failed(screen::Fault::Denied) => glyph(Icon::Send, 16.0, Color::WHITE),
        screen::Pulse::Failed(_) => glyph(Icon::Warn, 16.0, ACCENT),
    };
    let made = button(container(mark).center(REFRESH_SIDE - 2.0))
        .padding(0)
        .width(REFRESH_SIDE)
        .height(REFRESH_SIDE)
        .style(ui::button_faded(refresh_look(pulse)));
    let made: Element<'a, Message> = match press {
        Some(message) => made.on_press(message).into(),
        None => made.into(),
    };
    let stamp = match ground.fetch {
        screen::Fetch::Fresh(at) | screen::Fetch::Failed(Some(at), _) => Some(at),
        _ => None,
    };
    let mut told = column![text(w.t(key)).font(theme::SANS_SEMI).size(12.0).wrapping(text::Wrapping::None).color(INK)].spacing(2);
    if let Some(at) = stamp {
        told = told.push(text(w.with("refresh-when", &[("when", format!("{} {}", w.day(at, ground.now_unix), w.clock(at)))])).font(theme::SANS).size(11.5).wrapping(text::Wrapping::None).color(MUTED));
    }
    let tip = container(told).padding([6, 10]).style(|_| container::Style {
        background: Some(Background::Color(Color::from_rgb8(0x1d, 0x12, 0x16))),
        border: Border { color: Color::from_rgb8(0x36, 0x27, 0x2b), width: 1.0, radius: 8.0.into() },
        ..container::Style::default()
    });
    Some(iced::widget::tooltip(made, tip, iced::widget::tooltip::Position::Bottom).gap(6).into())
}

fn empty_card<'a>(ground: &Ground<'a>, icon: Icon, title: String, said: String, action: Option<(String, Message, bool)>) -> Element<'a, Message> {
    let k = ui::fade();
    let mut inside = column![
        glyph(icon, 28.0, FAINT),
        text(title).font(theme::SANS_SEMI).size(15.0).color(ui::faded(INK)),
        container(text(said).font(theme::SANS).size(12.0).color(ui::faded(MUTED)).align_x(iced::alignment::Horizontal::Center)).max_width(260.0),
    ]
    .spacing(10)
    .align_x(iced::Center);
    if let Some((label, press, strong)) = action {
        inside = inside.push(if strong { ui::primary(label, Some(press)) } else { ui::small_button(label, press) });
    }
    let _ = ground;
    container(
        container(inside)
            .padding([32, 20])
            .style(move |_| container::Style { border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.04 * k), width: 1.0, radius: 12.0.into() }, ..container::Style::default() }),
    )
    .center_x(Length::Fill)
    .padding(Padding { top: 18.0, right: 0.0, bottom: 18.0, left: 0.0 })
    .into()
}

fn empty_rows<'a>(ground: &Ground<'a>, everything: bool, table: Table) -> Element<'a, Message> {
    let w = ground.words;
    let query = ground.query.trim();
    if everything {
        if let screen::Fetch::Failed(None, fault) = ground.fetch {
            let (said, action) = match fault {
                screen::Fault::Offline => (w.t("feed-failed-offline"), (w.t("community-retry"), Message::Again, false)),
                screen::Fault::Server => (w.t("feed-failed-server"), (w.t("community-retry"), Message::Again, false)),
                screen::Fault::Denied => (w.t("community-denied"), (w.t("community-connect"), Message::Reconnect, true)),
            };
            return empty_card(ground, Icon::Warn, w.t("feed-failed-title"), said, Some(action));
        }
        if matches!(ground.fetch, screen::Fetch::Loading) {
            return skeleton_rows(ground, table);
        }
    }
    if !query.is_empty() {
        return empty_card(ground, Icon::Search, w.t("feed-search-title"), w.with("feed-search-text", &[("query", query.to_owned())]), Some((w.t("feed-search-action"), Message::Search(String::new()), false)));
    }
    if ground.filter != Filter::All && !everything {
        return empty_card(ground, Icon::Chart, w.t("feed-filter-title"), w.t("feed-filter-text"), Some((w.t("feed-filter-action"), Message::Filter(Filter::All), false)));
    }
    empty_card(ground, Icon::News, w.t("feed-quiet-title"), w.t("feed-quiet-text"), Some((w.t("refresh-idle"), Message::Again, false)))
}

fn skeleton_rows<'a>(ground: &Ground<'a>, table: Table) -> Element<'a, Message> {
    let k = ui::fade();
    let strong = Color::from_rgba8(0x24, 0x18, 0x1c, k);
    let weak = Color::from_rgba8(0x1a, 0x11, 0x14, k);
    let bone = move |wide: f32, high: f32, colour: Color, radius: f32| -> Element<'a, Message> {
        container(Space::new())
            .width(wide)
            .height(high)
            .style(move |_| container::Style { background: Some(Background::Color(colour)), border: Border { radius: radius.into(), ..Border::default() }, ..container::Style::default() })
            .into()
    };
    let _ = ground;
    let mut list = column![].spacing(ROW_GAP);
    for at in 0..7usize {
        let line = row![
            container(ui::fine_hatch()).width(table.cover.0).height(table.cover.1),
            column![bone(110.0 + ((at * 37) % 70) as f32, 13.0, strong, 6.0), bone(170.0 + ((at * 53) % 90) as f32, 11.0, weak, 5.0)].spacing(8).width(Length::Fill),
            column![bone(64.0, 16.0, strong, 6.0), bone(48.0, 10.0, weak, 5.0)].spacing(6).align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(16)
        .align_y(iced::Center);
        list = list.push(container(line).height(table.high()).center_y(table.high()).padding([0, 12]));
    }
    list.into()
}

fn timeline<'a>(ground: &Ground<'a>, wide: f32) -> Element<'a, Message> {
    let w = ground.words;
    let everything = events(ground.catalog, ground.news, ground.channels, ground.live_shown);
    let waiting = everything.iter().filter(|event| ground.held.contains(&event.key(ground.catalog))).count();
    let all: Vec<Event<'a>> = everything.into_iter().filter(|event| !ground.held.contains(&event.key(ground.catalog))).collect();
    let mut list: Vec<Event<'a>> = all.iter().copied().filter(|event| event.found(ground.catalog, ground.query) && event.admitted(ground.filter)).collect();
    if let Some(leave) = ground.leaving {
        let frozen = leave.frozen;
        let leaving = events(&frozen.catalog, &frozen.news, &frozen.channels, frozen.live_shown)
            .into_iter()
            .filter(|event| leave.keys.contains(&event.key(&frozen.catalog)) && event.admitted(ground.filter) && event.found(&frozen.catalog, ground.query))
            .map(|event| Event { ghost: true, ..event });
        list.extend(leaving);
        list.sort_by(|a, b| b.at.cmp(&a.at));
    }

    let filters = screen::segmented_compact(Filter::ALL.iter().map(|filter| (w.t(filter.key()), ground.filter == *filter, Message::Filter(*filter))).collect());
    let mut top = row![filters, ui::grow()].spacing(10).align_y(iced::Center);
    if let Some(refresh) = refresh_button(ground) {
        top = top.push(refresh);
    }
    top = top.push(
        container(
            row![
                glyph(Icon::Search, 13.0, FAINT),
                iced::widget::text_input(&w.t("feed-search"), ground.query).on_input(Message::Search).size(12.0).padding(0).style(ui::bare_input(ui::fade())).width(Length::Fill),
            ]
            .spacing(8)
            .align_y(iced::Center),
        )
        .padding([6, 10])
        .width(if wide < 640.0 { 130.0 } else { 200.0 })
        .style(ui::box_faded(|_| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.25))),
            border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.07), width: 1.0, radius: 8.0.into() },
            ..container::Style::default()
        })),
    );
    let lately = ground.section_t.min(ground.stream_t);
    let below = |made: Element<'a, Message>| -> Element<'a, Message> { container(made).padding(Padding::ZERO.bottom(PAGE_GAP)).into() };
    let mut page = column![below(ui::appearing(ui::appear(ground.section_t, 0), 10.0, || top.into()))];

    let table = Table::for_width(wide - 12.0);
    let mut rows = column![];
    if list.is_empty() {
        rows = rows.push(empty_rows(ground, all.is_empty(), table));
    } else {
        for made in by_day(ground, &list, lately, 2, |event| {
            let on = if event.ghost { ground.ghost.as_deref().unwrap_or(ground) } else { ground };
            match event.item {
                Item::Play(_) | Item::Happened(_) => journal_row(on, event, table),
                _ => news_row(on, event, table),
            }
        }) {
            rows = rows.push(made);
        }
    }
    page = page.push(rows);
    let rolled = scrollable(container(page).padding(Padding { top: 2.0, right: 10.0, bottom: PAGE_BELOW - ROW_GAP, left: 2.0 }))
        .id(iced::widget::Id::new("community-feed"))
        .on_scroll(|viewport| Message::FeedScrolled(viewport.absolute_offset().y))
        .style(ui::thin_scroll)
        .direction(ui::hidden_bar())
        .width(Length::Fill)
        .height(Length::Fill);
    let rolled: Element<'a, Message> = crate::glide::brim(rolled).into();
    if waiting == 0 {
        return rolled;
    }
    let pill = button(row![glyph(Icon::Up, 14.0, Color::WHITE), text(w.n("feed-new-n", waiting as u64)).font(theme::SANS_SEMI).size(13.0).wrapping(text::Wrapping::None).color(Color::WHITE)].spacing(8).align_y(iced::Center))
        .padding([6, 14])
        .style(ui::button_faded(|_, status| {
            let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: Some(Background::Color(if lit { Color::from_rgb8(0xf0, 0x5e, 0x5e) } else { ACCENT })),
                text_color: Color::WHITE,
                border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 15.0.into() },
                shadow: Shadow { color: Color::from_rgba(0.0, 0.0, 0.0, 0.4), offset: Vector::new(0.0, 4.0), blur_radius: 14.0 },
                snap: true,
            }
        }))
        .on_press(Message::ShowNew);
    stack![rolled, container(pill).center_x(Length::Fill).padding(Padding { top: 8.0, ..Padding::ZERO })].width(Length::Fill).height(Length::Fill).into()
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

const ME_FACE: f32 = 52.0;
const ME_LABELLED: f32 = 300.0;
const STRIP_ROW: f32 = 52.0;

fn me_card<'a>(ground: &Ground<'a>, wide: f32) -> Element<'a, Message> {
    let w = ground.words;
    let Some(you) = ground.catalog.you() else {
        return card(ui::mono_small(w.t("nothing-yet"), FAINT), 16).into();
    };
    let card_data = ground.card.cloned().or_else(|| ground.catalog.card_of());
    let share = card_data.as_ref().map_or(0.0, |c| (c.level_progress / 100.0) as f32);
    let signed = |value: f64, places: usize| -> Option<(String, bool)> {
        (value.abs() > 0.004).then(|| (format!("{}{}", if value > 0.0 { "+" } else { "−" }, screen::decimal(w, value.abs() as f32, places)), value > 0.0))
    };
    let at = ground.catalog.people.iter().position(|person| person.you);
    let in_group = at.and_then(|at| ground.catalog.ranked(Board::Pp).iter().position(|x| *x == at)).map(|x| x + 1);
    let f = ui::tally(ground.section_t, 0.1);
    let coral = Color::from_rgb(0.941, 0.408, 0.408);
    let mut head = row![
        ring(ground, you, ME_FACE, share, 3.0),
        column![
            row![ui::marquee(vec![ui::piece(you.name.clone(), theme::SANS_SEMI, 16.0, INK)]).width(Length::Shrink), screen::flag(ground, &you.country, 11.0)]
            .spacing(7)
            .align_y(iced::Center),
            screen::title_line(ground, you, 12.0),
        ]
        .spacing(3)
        .width(Length::Fill),
    ]
    .spacing(12)
    .align_y(iced::Center);
    if let Some(place) = in_group {
        let mut said = column![text(format!("#{}", ((place as f64 * f).round() as usize).max(1))).font(theme::SANS_SEMI).size(24.0).wrapping(text::Wrapping::None).color(ui::faded(screen::medal(place).unwrap_or(INK)))]
            .align_x(iced::alignment::Horizontal::Right);
        if wide >= ME_LABELLED {
            said = said.push(ui::mono_small(w.t("in-group"), FAINT));
        }
        head = head.push(said);
    }
    let mut weekly = row![
        text(w.lang().group((f64::from(you.pp) * f).round() as u64)).font(theme::SANS_SEMI).size(30.0).wrapping(text::Wrapping::None).color(ui::faded(INK)),
        ui::mono_small(w.t("board-pp"), FAINT),
        ui::grow(),
    ]
    .spacing(6)
    .align_y(iced::alignment::Vertical::Bottom);
    if let Some((delta, _)) = signed(you.gained[0], 0) {
        weekly = weekly.push(ui::mono_small(w.with("week-delta", &[("value", delta)]), coral));
    }
    let fact = |label: String, value: String, delta: Option<(String, bool)>| -> Element<'a, Message> {
        let mut line = row![text(label).font(theme::SANS).size(12.0).color(ui::faded(MUTED)).width(Length::Fill), ui::mono_small(value, INK)].spacing(8).align_y(iced::Center);
        if let Some((delta, _)) = delta {
            line = line.push(ui::mono_small(delta, coral));
        }
        line.into()
    };
    let mut facts = column![
        fact(w.t("global-rank"), screen::rank_of(w, if you.rank > 0 { ((f64::from(you.rank) * f).round() as u32).max(1) } else { 0 }), None),
        fact(w.t("board-accuracy"), w.percent(f64::from(you.accuracy) * f), signed(you.gained[1], 2)),
    ]
    .spacing(8);
    if you.streak > 0 {
        facts = facts.push(
            row![glyph(Icon::Flame, 13.0, coral), text(w.n("streak-card", u64::from(you.streak))).font(theme::SANS).size(12.0).wrapping(text::Wrapping::None).color(ui::faded(MUTED))]
                .spacing(7)
                .align_y(iced::Center),
        );
    }
    let open = ui::hover(
        button(container(text(w.t("my-profile-open")).font(theme::SANS_SEMI).size(12.0).wrapping(text::Wrapping::None)).center_x(Length::Fill))
            .padding([8, 14])
            .width(Length::Fill)
            .style(ui::button_faded(ui::calm(framed)))
            .on_press(Message::Section(Section::Profile)),
        ui::Glow::tile(8.0).edge(Color::from_rgba(1.0, 1.0, 1.0, 0.16)),
    );
    card(column![head, weekly, facts, open].spacing(16), 16).into()
}

fn framed(_: &Theme, status: button::Status) -> button::Style {
    let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(Background::Color(if lit { Color::from_rgb8(0x2c, 0x1d, 0x21) } else { Color::from_rgb8(0x1d, 0x12, 0x16) })),
        text_color: INK,
        border: Border { color: if lit { Color::from_rgb8(0x55, 0x40, 0x45) } else { Color::from_rgb8(0x36, 0x27, 0x2b) }, width: 1.0, radius: 8.0.into() },
        shadow: Shadow::default(),
        snap: true,
    }
}

fn channels_card<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let mut body = column![caption(w.t("channels-head")), screen::channel_tools(ground)].spacing(10);
    if let Some(channel) = ground.channels.first() {
        let source = news::channel_source(channel);
        if ground.loading.contains(&source) || ground.failed.contains(&source) {
            body = body.push(screen::source_state(ground, &source));
        }
    }
    card(body, [14, 16])
        .id(iced::widget::Id::new("community-channels-card")).into()
}

fn strip_row<'a>(left: Element<'a, Message>, label: String, dim: bool, press: Message) -> Element<'a, Message> {
    let colour = if dim { MUTED } else { INK };
    let inside = row![
        left,
        text(label).font(theme::SANS_SEMI).size(13.0).color(ui::faded(colour)).width(Length::Fill),
        glyph(Icon::Right, 14.0, FAINT),
    ]
    .spacing(12)
    .align_y(iced::Center);
    ui::hover(
        button(container(inside).height(STRIP_ROW).align_y(iced::Center))
            .padding([0, 14])
            .width(Length::Fill)
            .style(ui::button_faded(ui::calm(theme::row(false))))
            .on_press(press),
        ui::Glow::row(theme::CONTROL_RADIUS),
    )
}

fn empty_ring<'a>(side: f32) -> Element<'a, Message> {
    let k = ui::fade();
    container(Space::new())
        .width(side)
        .height(side)
        .style(move |_| container::Style { border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.18 * k), width: 1.0, radius: (side / 2.0).into() }, ..container::Style::default() })
        .into()
}

fn strip_card<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let catalog = ground.catalog;
    let mut online: Vec<&crate::community::Friend> = catalog.friends.iter().filter(|friend| friend.online).collect();
    online.sort_by_key(|friend| friend.minutes_away(ground.now_unix));
    let friends: Element<'a, Message> = if online.is_empty() {
        empty_ring(24.0)
    } else {
        let mut faces = row![].spacing(-8.0);
        for friend in online.iter().take(3) {
            faces = faces.push(screen::friend_face(ground, friend, 24.0));
        }
        faces.into()
    };
    let list = screen::standings(catalog, Board::Pp, screen::Standing::Adaptive);
    let leaders: Element<'a, Message> = if catalog.collecting || list.order.is_empty() {
        empty_ring(24.0)
    } else {
        let mut faces = row![].spacing(4.0);
        for (at, (who, _)) in list.order.iter().take(3).enumerate() {
            let place = at + 1;
            let k = ui::fade();
            let colour = screen::medal(place).unwrap_or(INK);
            let ring = Color::from_rgb(0.071, 0.039, 0.047);
            let nudge = Padding { top: 1.0, right: 0.0, bottom: 0.0, left: if place == 1 { 2.0 } else { 0.0 } };
            let badge = container(text(place.to_string()).font(theme::MONO_BOLD).size(9.0).wrapping(text::Wrapping::None).color(Color { a: k, ..Color::from_rgb(0.08, 0.04, 0.05) }))
                .width(15.0)
                .height(15.0)
                .padding(nudge)
                .center(15.0)
                .style(move |_| container::Style { background: Some(Background::Color(Color { a: k, ..colour })), border: Border { color: Color { a: k, ..ring }, width: 1.5, radius: 7.5.into() }, ..container::Style::default() });
            faces = faces.push(
                stack![container(screen::face(ground, &catalog.people[*who], 26.0)).padding(Padding { left: 0.0, top: 0.0, right: 0.0, bottom: 0.0 }), container(badge).width(34.0).height(32.0).align_x(iced::alignment::Horizontal::Right).align_y(iced::alignment::Vertical::Bottom)]
                    .width(34.0)
                    .height(32.0),
            );
        }
        faces.into()
    };
    let divider = container(Space::new()).width(Length::Fill).height(1.0).style(|_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.07))), ..container::Style::default() });
    card(
        column![
            strip_row(friends, w.t("strip-friends"), online.is_empty(), Message::Go(Section::People, Some(screen::PeopleFrom::Game))),
            divider,
            strip_row(leaders, w.t("week-leaders"), catalog.collecting || list.order.is_empty(), Message::Section(Section::Boards)),
        ],
        0,
    )
    .clip(true)
    .id(iced::widget::Id::new("community-strip-card"))
    .into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Part {
    Timeline,
    Me,
    Strip,
    Channels,
}

fn plan(room: f32) -> Vec<Lane<Part>> {
    use Part::*;
    if room >= RIGHT_LEAST + TIMELINE_LEAST + GAP {
        let right = (room * 0.22).clamp(RIGHT_LEAST, RIGHT_MOST);
        let middle = (room - right - GAP).min(CENTRE_MOST);
        let x = ((room - right - GAP - middle) / 2.0).max(0.0);
        vec![Lane::fill(x, middle, Timeline), Lane::stack(x + middle + GAP, right, [Me, Strip, Channels])]
    } else {
        vec![Lane::fill(0.0, room, Timeline)]
    }
}

pub fn view<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let room = ground.width - 80.0;
    let planned = plan(room);
    let t = ground.section_t;
    let mut pieces = vec![(Part::Timeline, timeline(ground, lanes::width_of(&planned, Part::Timeline).unwrap_or(room)))];
    for (stack, lane) in planned.iter().filter(|lane| !lane.fill).enumerate() {
        let across = lanes::width_of(&planned, lane.keys[0]).unwrap_or(RIGHT_LEAST);
        for (at, part) in lane.keys.iter().enumerate() {
            let k = ui::appear(t, stack + at);
            let card = match part {
                Part::Me => ui::appearing(k, 12.0, || ui::smooth(me_card(ground, across))),
                Part::Strip => ui::appearing(k, 12.0, || ui::smooth(strip_card(ground))),
                Part::Channels => ui::appearing(k, 12.0, || ui::smooth(channels_card(ground))),
                Part::Timeline => continue,
            };
            pieces.push((*part, card));
        }
    }
    container(lanes::lanes(pieces, plan).spacing(12.0).room(2.0, 28.0)).padding(Padding { top: 12.0, right: 40.0, bottom: 0.0, left: 40.0 }).width(Length::Fill).height(Length::Fill).into()
}

#[cfg(test)]
mod tests {

    use super::preview_spans;

    #[test]
    fn a_post_card_keeps_the_break_between_its_paragraphs() {
        let markup = "<b>kotofey</b> поставил первое<br/>сыграв в 99.21% аккураси<br/><br/><a href=\"?q=%23скор\"><b>#скор</b></a>";
        let body = crate::news::blocks_of(markup, "https://t.me/s/osunewsru", crate::news::Flow::Post);
        let said: String = preview_spans(&body).iter().map(|span| span.text.as_str()).collect();
        assert!(said.contains("аккураси\n#скор"), "{said:?}");
        assert!(said.contains("первое\nсыграв"), "{said:?}");
    }
    use super::*;

    #[test]
    fn asynchronous_sources_do_not_restart_previous_arrivals() {
        let now = Instant::now();
        let mut arrivals = Arrivals::default();
        let old = vec!["old".to_owned()];
        let first = vec!["first".to_owned(), "old".to_owned()];
        arrivals.refresh(&old, &first, now);
        let later = now + Duration::from_millis(200);
        let second = vec!["second".to_owned(), "first".to_owned(), "old".to_owned()];
        arrivals.refresh(&first, &second, later);
        let ages = arrivals.ages(later);
        assert!((ages["first"] - 0.2).abs() < 0.0001);
        assert_eq!(ages["second"], 0.0);
        assert!(!ages.contains_key("old"));
        arrivals.refresh(&second, &second, later + Duration::from_millis(100));
        assert!((arrivals.ages(later + Duration::from_millis(100))["first"] - 0.3).abs() < 0.0001);
        assert!(!arrivals.animating(now + Duration::from_secs(5)));
    }

    #[test]
    fn a_picture_fades_in_once_and_a_lost_one_gives_its_room_back() {
        let now = Instant::now();
        let mut pictures = Pictures::default();
        pictures.came("cover", now);
        assert_eq!(pictures.shown(now)["cover"], 0.0);
        assert!(pictures.animating(now));
        let half = pictures.shown(now + Duration::from_secs_f32(PICTURE_FADE / 2.0))["cover"];
        assert!(half > 0.5 && half < 1.0, "{half}");
        let after = now + Duration::from_secs_f32(PICTURE_FADE + 0.05);
        pictures.settle(after);
        assert!(pictures.shown(after).is_empty(), "a picture that has arrived is simply there");
        assert!(!pictures.animating(after));

        pictures.lost("gone", now);
        assert_eq!(pictures.room(now)["gone"], 1.0);
        pictures.lost("gone", after);
        assert_eq!(pictures.room(after)["gone"], 0.0, "hearing of the loss again does not reopen the room");
        pictures.came("gone", after);
        assert!(!pictures.room(after).contains_key("gone"), "a picture that came after all is no longer lost");
    }

    #[test]
    fn older_new_items_are_animated_and_removed_items_are_forgotten() {
        let now = Instant::now();
        let catalog = Catalog::staged(Vec::new(), "", 1_790_000_000);
        let mut news = News { stories: vec![news::Story { url: "old".into(), at: 1_790_000_001, ..news::Story::default() }], ..News::default() };
        let before = event_keys(&catalog, &news, &[], 0);
        news.stories.push(news::Story { url: "older-but-new-to-the-feed".into(), at: 1_789_000_000, ..news::Story::default() });
        let current = event_keys(&catalog, &news, &[], 0);
        let mut arrivals = Arrivals::default();
        arrivals.refresh(&before, &current, now);
        assert_eq!(arrivals.ages(now)["story:older-but-new-to-the-feed"], 0.0);
        arrivals.refresh(&current, &before, now + Duration::from_millis(100));
        assert!(arrivals.ages(now + Duration::from_millis(100)).is_empty());
    }

    #[test]
    fn a_busy_channel_does_not_crowd_out_a_quiet_one() {
        let mut news = News::default();
        for n in 0..60 {
            news.posts.push(news::Post { channel: "osunow".into(), text: format!("busy {n}"), at: 2_000_000 + n, ..news::Post::default() });
        }
        news.posts.push(news::Post { channel: "osunewsru".into(), text: "quiet".into(), at: 1_000_000, ..news::Post::default() });
        news.posts.sort_by(|a, b| b.at.cmp(&a.at));
        let channels = vec!["osunewsru".to_owned(), "osunow".to_owned()];
        let chosen: Vec<&news::Post> = chosen_posts(&news, &channels).collect();
        assert!(chosen.iter().any(|post| post.channel == "osunewsru"));
        assert_eq!(chosen.iter().filter(|post| post.channel == "osunow").count(), POSTS_EACH);
    }

    #[test]
    fn a_filter_lets_through_only_its_own_events() {
        let catalog = Catalog::staged(Vec::new(), "", 1_790_000_000);
        let news = News::default();
        let all = events(&catalog, &news, &[], catalog.live.len());
        assert!(all.windows(2).all(|pair| pair[0].at >= pair[1].at), "newest first");
        let titles = all.iter().filter(|event| event.admitted(Filter::Titles)).count();
        let tops = all.iter().filter(|event| matches!(event.item, Item::Happened(happened) if matches!(happened.kind, Kind::TopPlay { .. }))).count();
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
    fn news_and_the_group_share_one_stream_and_the_news_filter_keeps_only_news() {
        let catalog = Catalog::staged(Vec::new(), "", 1_790_000_000);
        let news = News {
            stories: vec![news::Story { title: "Recap".into(), at: 1_789_990_000, ..news::Story::default() }],
            posts: vec![news::Post { channel: "osunewsru".into(), text: "FC".into(), url: "https://t.me/osunewsru/1".into(), at: 1_789_991_000, ..news::Post::default() }],
            builds: vec![news::Build { stream: "Lazer".into(), version: "2026.921.0".into(), at: 1_789_992_000, ..news::Build::default() }],
            ..News::default()
        };
        let all = events(&catalog, &news, &["osunewsru".to_owned()], catalog.live.len());
        let count = |filter: Filter| all.iter().filter(|event| event.admitted(filter)).count();
        assert_eq!(count(Filter::News), 3);
        assert_eq!(count(Filter::All), all.len());
        let told_apart = all.iter().filter(|event| matches!(event.item, Item::Story(_) | Item::Post(_) | Item::Build(_)));
        assert!(told_apart.clone().all(|event| event.admitted(Filter::All) && event.admitted(Filter::News) && !event.admitted(Filter::Plays) && !event.admitted(Filter::Titles) && !event.admitted(Filter::Ranks)));
        assert!(all.iter().filter(|event| matches!(event.item, Item::Play(_) | Item::Happened(_))).all(|event| !event.admitted(Filter::News)));
    }

    fn frozen() -> Frozen {
        Frozen::of(&Catalog::staged(Vec::new(), "", 1_790_000_000), &News::default(), &[], 40)
    }

    #[test]
    fn a_row_missing_from_the_new_answer_leaves_slowly_and_one_that_is_still_there_or_was_never_shown_does_not() {
        let now = Instant::now();
        let mut leaving = Leaving::default();
        let held: std::collections::HashSet<String> = ["held".to_owned()].into_iter().collect();
        let before = Before { keys: vec!["a".into(), "b".into(), "held".into()], frozen: Some(frozen()) };
        leaving.start(before, &["a".to_owned()], &held, now);
        assert!(leaving.busy());
        let early = leaving.shown(now).expect("leaving");
        assert_eq!(early.keys, ["b".to_owned()]);
        assert!((early.open - 1.0).abs() < 0.001, "it starts fully open: {}", early.open);
        let middle = leaving.shown(now + Duration::from_secs_f32(ui::APPEAR / 2.0)).expect("still leaving").open;
        assert!(middle > 0.0 && middle < 1.0, "{middle}");
        assert!(leaving.animating(now + Duration::from_millis(10)));
        assert!(leaving.shown(now + Duration::from_secs(1)).is_none());
        leaving.settle(now + Duration::from_secs(1));
        assert!(!leaving.busy());
    }

    #[test]
    fn nothing_leaves_when_nothing_is_missing_or_nothing_was_kept() {
        let now = Instant::now();
        let mut leaving = Leaving::default();
        leaving.start(Before { keys: vec!["a".into()], frozen: Some(frozen()) }, &["a".to_owned(), "b".to_owned()], &Default::default(), now);
        assert!(!leaving.busy());
        leaving.start(Before { keys: vec!["a".into()], frozen: None }, &[], &Default::default(), now);
        assert!(!leaving.busy(), "without the kept rows there is nothing to draw");
    }

    #[test]
    fn a_play_keeps_its_key_when_the_people_are_reordered() {
        let mut catalog = Catalog::staged(Vec::new(), "", 1_790_000_000);
        let news = News::default();
        let before: Vec<String> = events(&catalog, &news, &[], catalog.live.len()).iter().map(|event| event.key(&catalog)).collect();
        let count = catalog.people.len();
        for play in &mut catalog.live {
            play.who = count - 1 - play.who;
        }
        for happened in &mut catalog.feed {
            happened.who = count - 1 - happened.who;
        }
        catalog.people.reverse();
        let after: Vec<String> = events(&catalog, &news, &[], catalog.live.len()).iter().map(|event| event.key(&catalog)).collect();
        let (mut one, mut two) = (before, after);
        one.sort();
        two.sort();
        assert_eq!(one, two);
    }

    #[test]
    fn keys_tell_events_apart() {
        let catalog = Catalog::staged(Vec::new(), "", 1_790_000_000);
        let news = News::default();
        let all = events(&catalog, &news, &[], catalog.live.len());
        let mut keys: Vec<String> = all.iter().map(|event| event.key(&catalog)).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), all.len());
    }
}
