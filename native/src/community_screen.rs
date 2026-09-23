use std::collections::HashMap;

use iced::widget::{button, column, container, image, mouse_area, row, scrollable, stack, text, Space};
use iced::{Background, Border, Color, Element, Length, Padding, Shadow, Theme};

use crate::community::{Board, Catalog, Happening, Kind, Person, Rarity, Title, TITLES};
use crate::lang::Words;
use crate::theme::{self, ACCENT, FAINT, INK, MUTED};
use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Feed,
    People,
    Boards,
    Titles,
}

#[derive(Debug, Clone)]
pub enum Message {
    Section(Section),
    Board(Board),
    Person(Option<usize>),
}

pub struct Ground<'a> {
    pub words: &'a Words,
    pub catalog: &'a Catalog,
    pub thumbs: &'a HashMap<String, image::Handle>,
    pub section: Section,
    pub board: Board,
    pub person: Option<usize>,
    pub now_unix: i64,
}

const FEED_WIDE: f32 = 760.0;
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
    let w = ground.words;
    let mut list = column![].spacing(2).width(FEED_WIDE);
    let mut last_day: Option<String> = None;
    for happening in &ground.catalog.feed {
        let day = w.day(happening.at, ground.now_unix);
        if last_day.as_ref() != Some(&day) {
            list = list.push(container(ui::mono_small(day.to_uppercase(), FAINT)).padding(Padding { top: 14.0, right: 12.0, bottom: 6.0, left: 12.0 }));
            last_day = Some(day);
        }
        list = list.push(happening_row(ground, happening));
    }
    rolled(list.into())
}

fn happening_row<'a>(ground: &Ground<'a>, happening: &Happening) -> Element<'a, Message> {
    let w = ground.words;
    let lang = w.lang();
    let person = &ground.catalog.people[happening.who];
    let map_line = |ground: &Ground<'a>| {
        let line = ground.catalog.map(happening.map).map(|map| map.line.clone()).unwrap_or_default();
        text(ui::shortened(line, 64)).font(theme::SANS).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(MUTED))
    };
    let (left, kind, detail, right): (Element<'a, Message>, String, Element<'a, Message>, Element<'a, Message>) = match &happening.kind {
        Kind::Render { accuracy, mods: list } => (
            picture(ground, happening.map, 96.0, 54.0),
            w.t("happened-render"),
            map_line(ground).into(),
            row![mods(list), ui::mono(w.percent(f64::from(*accuracy)), INK)].spacing(8).align_y(iced::Center).into(),
        ),
        Kind::TopPlay { pp, place, mods: list } => (
            picture(ground, happening.map, 96.0, 54.0),
            format!("{} #{place}", w.t("happened-top-play")),
            map_line(ground).into(),
            row![mods(list), ui::mono(format!("{} pp", decimal(w, *pp, 0)), INK)].spacing(8).align_y(iced::Center).into(),
        ),
        Kind::Title(code) => {
            let title = crate::community::title_of(code);
            let colour = title.map_or(MUTED, |title| title.rarity.colour());
            let rarity = title.map_or_else(String::new, |title| w.t(title.rarity.key()));
            (
                seal("★".to_owned(), colour, 96.0, 54.0, 20.0),
                w.t("happened-title"),
                row![
                    text(title.map_or("", |title| title.name(lang))).font(theme::SANS_SEMI).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(colour)),
                    text(title.map_or("", |title| title.about(lang)).to_owned()).font(theme::SANS).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(FAINT)),
                ]
                .spacing(8)
                .into(),
                ui::mono_small(rarity.to_lowercase(), colour),
            )
        }
        Kind::Climb { board, from, to } => (
            seal(format!("#{to}"), INK, 96.0, 54.0, 18.0),
            w.t("happened-climb"),
            text(format!("{} · {from} → {to}", w.t(board.key()))).font(theme::SANS).size(theme::CAPTION).color(ui::faded(MUTED)).into(),
            moved(*from as i32 - *to as i32),
        ),
    };
    let middle = column![
        row![
            text(person.name.clone()).font(theme::SANS_SEMI).size(theme::BODY).wrapping(text::Wrapping::None).color(ui::faded(INK)),
            ui::mono_small(kind, MUTED),
        ]
        .spacing(8)
        .align_y(iced::Center),
        detail,
    ]
    .spacing(3);
    let when = ui::mono_small(w.clock(happening.at), FAINT);
    let line = row![
        container(left).width(96.0),
        container(middle).width(Length::Fill).clip(true),
        column![right, when].spacing(4).align_x(iced::alignment::Horizontal::Right),
    ]
    .spacing(14)
    .align_y(iced::Center);
    button(container(line).height(70.0).center_y(70.0).width(Length::Fill))
        .padding([0, 12])
        .style(ui::button_faded(theme::row(false)))
        .on_press(Message::Person(Some(happening.who)))
        .into()
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
