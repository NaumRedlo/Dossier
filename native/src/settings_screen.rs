use std::path::PathBuf;

use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Element, Length, Padding};

use crate::lang::{Lang, Words};
use crate::settings::{Settings, CRFS, HEIGHTS, RATES};
use crate::sources::{self, Kind, Source};
use crate::theme::{self, ACCENT, FAINT, INK, MUTED};
use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    App,
    Bot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tile {
    Render,
    Language,
    Device,
    Scene,
    Sources,
    Skins,
    Sound,
    Videos,
    Maps,
    Builds,
    Account,
    Chats,
    Worker,
    ThisDevice,
}

impl Tile {
    pub fn tag(self) -> &'static str {
        match self {
            Tile::Render => "render",
            Tile::Language => "language",
            Tile::Device => "device",
            Tile::Scene => "scene",
            Tile::Sources => "sources",
            Tile::Skins => "skins",
            Tile::Sound => "sound",
            Tile::Videos => "videos",
            Tile::Maps => "maps",
            Tile::Builds => "builds",
            Tile::Account => "account",
            Tile::Chats => "chats",
            Tile::Worker => "worker",
            Tile::ThisDevice => "this-device",
        }
    }

    pub fn of(tag: &str) -> Option<Tile> {
        APP.iter().chain(BOT.iter()).copied().find(|tile| tile.tag() == tag)
    }
}

pub const APP: [Tile; 10] = [
    Tile::Render,
    Tile::Language,
    Tile::Device,
    Tile::Scene,
    Tile::Sound,
    Tile::Sources,
    Tile::Skins,
    Tile::Videos,
    Tile::Maps,
    Tile::Builds,
];

pub const BOT: [Tile; 4] = [Tile::Account, Tile::Chats, Tile::Worker, Tile::ThisDevice];

pub fn order(kept: &[String], all: &[Tile]) -> Vec<Tile> {
    let mut out: Vec<Tile> = kept.iter().filter_map(|tag| Tile::of(tag)).filter(|tile| all.contains(tile)).collect();
    for tile in all {
        if !out.contains(tile) {
            out.push(*tile);
        }
    }
    out
}

pub fn moved(kept: &[String], all: &[Tile], what: Tile, before: Option<Tile>) -> Vec<String> {
    let mut list = order(kept, all);
    list.retain(|tile| *tile != what);
    let at = before.and_then(|edge| list.iter().position(|tile| *tile == edge)).unwrap_or(list.len());
    list.insert(at.min(list.len()), what);
    list.iter().map(|tile| tile.tag().to_owned()).collect()
}

pub struct Ground<'a> {
    pub words: &'a Words,
    pub settings: &'a Settings,
    pub side: Side,
    pub replays: usize,
    pub videos: usize,
    pub videos_size: u64,
    pub maps: usize,
    pub maps_size: u64,
    pub cache_size: u64,
    pub ffmpeg: Option<String>,
    pub account: Option<&'a crate::bot::Me>,
    pub avatar: Option<&'a iced::widget::image::Handle>,
    pub chats: &'a [crate::bot::Chat],
    pub chat_faces: &'a std::collections::HashMap<i64, iced::widget::image::Handle>,
    pub dragging: Option<Tile>,
    pub landing: Option<Tile>,
    pub leaving: Option<Tile>,
    pub leaving_open: f32,
    pub landed: Option<(Tile, f32)>,
    pub opening: f32,
    pub closing: f32,
    pub renaming: Option<&'a String>,
    pub skins: &'a [PathBuf],
    pub skin_faces: &'a std::collections::HashMap<PathBuf, iced::widget::image::Handle>,
    pub marks: &'a std::collections::HashMap<String, f32>,
    pub slides: &'a std::collections::HashMap<String, (f32, f32)>,
    pub came: f32,
    pub swap: f32,
    pub swap_from: f32,
}

#[derive(Debug, Clone)]
pub enum Message {
    Side(Side),
    PickLang(Lang),
    Rename(String),
    RenameDone,
    Scene(bool),
    PauseUnfocused(bool),
    Height(f32),
    Rate(f32),
    Crf(f32),
    Source(usize, bool),
    AddFolder,
    Added(Option<Source>),
    OpenRenders,
    PickRenders,
    PickedRenders(Option<PathBuf>),
    OpenMaps,
    ClearCache,
    CheckBuild,
    GetFfmpeg,
    Chat(i64),
    Worker(bool),
    Skin(Option<PathBuf>),
    RescanSkins,
    OpenSkin,
    AddSkin,
    MoreSkins,
    AddedSkin(Option<PathBuf>),
    OpenSkinsFolder,
    Music(f32),
    Hitsounds(f32),
    PlayerLevel(f32),
    Unlink,
    SignOut,
    Drag(Tile),
    DragAt(iced::Point),
    DropBefore(Option<Tile>, bool),
    Dropped,
}

pub fn view<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let word = |key: &str, side: Side| {
        let on = ground.side == side;
        let k = ui::fade();
        let bar = container(Space::new().height(2.0)).width(Length::Fill).style(move |_| iced::widget::container::Style {
            background: Some(iced::Background::Color(ui::dim(if on { ACCENT } else { iced::Color::TRANSPARENT }, k))),
            border: iced::Border { radius: 1.0.into(), ..iced::Border::default() },
            ..iced::widget::container::Style::default()
        });
        button(column![text(w.t(key)).font(theme::SANS_SEMI).size(theme::BODY), bar].spacing(5).width(Length::Shrink))
            .padding([4, 0])
            .style(ui::button_faded(theme::word(on)))
            .on_press(Message::Side(side))
    };
    let switch = container(row![word("app-side", Side::App), word("bot-side", Side::Bot)].spacing(28))
        .width(Length::Fill)
        .center_x(Length::Fill);
    let (kept, all) = match ground.side {
        Side::App => (&ground.settings.tiles_app, &APP[..]),
        Side::Bot => (&ground.settings.tiles_bot, &BOT[..]),
    };
    let mut tiles: Vec<Element<'a, Message>> = Vec::new();
    let held = ground.dragging;
    for (at, tile) in order(kept, all).into_iter().enumerate() {
        let late = (ground.came * 1.7 - 0.09 * at as f32).clamp(0.0, 1.0);
        if held.is_some() && ground.leaving == Some(tile) && ground.leaving_open > 0.01 {
            tiles.push(room(ground, held.unwrap_or(tile), ground.leaving_open));
        }
        if held.is_some() && ground.landing == Some(tile) && ground.opening > 0.01 {
            tiles.push(room(ground, held.unwrap_or(tile), ground.opening));
        }
        if held == Some(tile) {
            if ground.closing > 0.01 {
                tiles.push(room(ground, tile, ground.closing));
            }
            continue;
        }
        tiles.push(draggable(ground, tile, late));
    }
    let swap = ground.swap.clamp(0.0, 1.0);
    let slid = ui::grown(ui::wrap(tiles, 10.0), iced::Point::new(0.5, 0.0), 0.0, 1.0).shifted((1.0 - swap) * 26.0 * ground.swap_from);
    let grid = container(ui::fading(ui::fade() * swap, || slid)).padding(Padding { top: 16.0, right: 40.0, bottom: 24.0, left: 40.0 });
    column![container(switch).padding(Padding::ZERO.top(22.0)), grid].width(Length::Fill).into()
}

fn draggable<'a>(ground: &Ground<'a>, tile: Tile, late: f32) -> Element<'a, Message> {
    let settling = ground.landed.filter(|(what, _)| *what == tile).map(|(_, k)| k);
    let k = (ui::fade() * late).clamp(0.0, 1.0);
    let inside: Element<'a, Message> = ui::fading(k.powf(2.2), || one(ground, tile));
    let card: Element<'a, Message> = container(inside).padding([12, 14]).style(ui::box_at(theme::slab, k.powf(0.6))).into();
    let grown = match settling {
        Some(k) => 1.06 - 0.06 * k,
        None => 0.96 + 0.04 * late,
    };
    let risen = ui::grown(card, iced::Point::new(0.5, 0.5), 0.0, grown);
    ui::dragged(
        risen.into(),
        tile,
        Message::Drag(tile),
        move |before| Message::DropBefore(Some(tile), before),
        Message::Dropped,
        ground.dragging.is_some(),
    )
}

fn room<'a>(ground: &Ground<'a>, tile: Tile, open: f32) -> Element<'a, Message> {
    let inside = container(one(ground, tile)).padding([12, 14]);
    ui::hollow(inside).opened(open.clamp(0.0, 1.0)).into()
}

pub fn floating<'a>(ground: &Ground<'a>, tile: Tile) -> Element<'a, Message> {
    container(one(ground, tile))
        .padding([12, 14])
        .style(ui::box_faded(theme::slab_held))
        .into()
}

fn machine_name() -> String {
    match ui::Machine::here() {
        ui::Machine::Mac => "macOS".to_owned(),
        ui::Machine::Windows => "Windows".to_owned(),
        ui::Machine::Linux => "Linux".to_owned(),
    }
}

fn tongue<'a>(ground: &Ground<'a>, lang: Lang, name: &str, on: bool) -> Element<'a, Message> {
    let id = if lang == Lang::Ru { "lang-ru" } else { "lang-en" };
    let k = mark_at(ground, id, on);
    let face = ui::flag(if lang == Lang::Ru { ui::Lang::Ru } else { ui::Lang::En }, on, k, 28.0);
    let words = text(name.to_owned())
        .font(theme::SANS_SEMI)
        .size(theme::CAPTION)
        .wrapping(text::Wrapping::None)
        .color(ui::faded(if on { INK } else { MUTED }));
    button(row![face, words].spacing(10).align_y(iced::Center))
        .padding([2, 2])
        .style(ui::button_faded(theme::bare))
        .on_press(Message::PickLang(lang))
        .into()
}

fn first_letter(name: &str) -> String {
    name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()
}

fn chat_line<'a>(
    ground: &Ground<'a>,
    chat: &crate::bot::Chat,
    face: Option<&iced::widget::image::Handle>,
    under: String,
    on: bool,
) -> Element<'a, Message> {
    let k = mark_at(ground, &format!("chat-{}", chat.id), on);
    let mark: Element<'a, Message> = match face {
        Some(handle) => {
            let picture = iced::widget::image(handle.clone())
                .content_fit(iced::ContentFit::Cover)
                .width(28.0)
                .height(28.0)
                .border_radius(14.0)
                .opacity(ui::fade());
            iced::widget::stack![picture, ui::ring(k, 28.0)].width(28.0).height(28.0).into()
        }
        None => ui::mark(if chat.private { "@" } else { "#" }, on, k, 28.0),
    };
    let words = column![
        text(chat.title.clone()).font(theme::SANS_SEMI).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(INK)),
        text(under).font(theme::SANS).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(MUTED)),
    ]
    .spacing(1);
    button(row![mark, words].spacing(10).align_y(iced::Center))
        .padding([2, 2])
        .style(ui::button_faded(theme::bare))
        .on_press(Message::Chat(chat.id))
        .into()
}

fn mark_at(ground: &Ground<'_>, id: &str, on: bool) -> f32 {
    ground.marks.get(id).copied().unwrap_or(if on { 1.0 } else { 0.0 })
}

fn slide<'a>(ground: &Ground<'a>, id: &'static str, label: String, value: String, at: f32, stops: Vec<f32>, on: impl Fn(f32) -> Message + 'a) -> Element<'a, Message> {
    let (shown, snap) = ground.slides.get(id).copied().unwrap_or((at, 1.0));
    ui::steps(label, value, at, shown, snap, stops, on)
}

fn head<'a>(w: &Words, key: &str) -> Element<'a, Message> {
    container(text(w.t(key)).font(theme::MONO).size(12.0).color(ui::faded(INK)))
        .padding(Padding::ZERO.bottom(8.0))
        .into()
}

fn line<'a>(ground: &Ground<'a>, id: &str, glyph: &str, name: String, under: String, on: bool, press: Option<Message>) -> Element<'a, Message> {
    let k = mark_at(ground, id, on);
    let mut words = column![text(name).font(theme::SANS_SEMI).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(INK))].spacing(1);
    if !under.is_empty() {
        words = words.push(text(under).font(theme::SANS).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(MUTED)));
    }
    let inside = row![ui::mark(glyph, on, k, 28.0), words].spacing(10).align_y(iced::Center);
    match press {
        Some(message) => button(inside).padding([2, 2]).style(ui::button_faded(theme::bare)).on_press(message).into(),
        None => container(inside).padding([2, 2]).into(),
    }
}

fn pill<'a>(ground: &Ground<'a>, id: &str, name: String, on: bool, press: Message) -> Element<'a, Message> {
    let k = mark_at(ground, id, on);
    let inside = row![ui::mark("", on, k, 22.0), text(name).font(theme::SANS_SEMI).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(INK))]
        .spacing(8)
        .align_y(iced::Center);
    button(container(inside).padding([0, 10]).center_y(38.0))
        .padding(0)
        .style(ui::button_faded(theme::pill(on)))
        .on_press(press)
        .into()
}

fn figure<'a>(value: String, under: String) -> Element<'a, Message> {
    column![
        text(value).font(theme::MONO_BOLD).size(22.0).wrapping(text::Wrapping::None).color(ui::faded(INK)),
        text(under).font(theme::SANS).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(FAINT)),
    ]
    .spacing(2)
    .into()
}

fn deed<'a>(words: String, press: Message, hot: bool) -> Element<'a, Message> {
    button(container(text(words).font(theme::SANS_SEMI).size(theme::CAPTION)).center_y(26.0).padding([0, 10]))
        .padding(0)
        .style(ui::button_faded(if hot { theme::small_hot } else { theme::small }))
        .on_press(press)
        .into()
}

fn percent(level: f32) -> String {
    format!("{} %", (level.clamp(0.0, 1.0) * 100.0).round() as u32)
}

fn at(value: u32, of: &[u32]) -> f32 {
    let last = (of.len().max(2) - 1) as f32;
    of.iter().position(|v| *v == value).map_or(0.5, |i| i as f32 / last)
}

fn stops(of: &[u32]) -> Vec<f32> {
    let last = (of.len().max(2) - 1) as f32;
    (1..of.len().saturating_sub(1)).map(|i| i as f32 / last).collect()
}

pub fn nearest(fraction: f32, of: &[u32]) -> u32 {
    let last = (of.len().max(2) - 1) as f32;
    let at = (fraction.clamp(0.0, 1.0) * last).round() as usize;
    of[at.min(of.len() - 1)]
}

fn one<'a>(ground: &Ground<'a>, tile: Tile) -> Element<'a, Message> {
    let w = ground.words;
    let s = ground.settings;
    match tile {
        Tile::Render => column![
            head(w, "render-tile"),
            container(
                column![
                    slide(ground, "height", w.t("render-size"), format!("{}p", s.render_height), at(s.render_height, &HEIGHTS), stops(&HEIGHTS), Message::Height),
                    slide(ground, "rate", w.t("render-frames"), s.render_fps.to_string(), at(s.render_fps, &RATES), stops(&RATES), Message::Rate),
                    slide(ground, "crf", w.t("render-quality"), format!("CRF {}", s.render_crf), at(s.render_crf, &CRFS), stops(&CRFS), Message::Crf),
                ]
                .spacing(6)
                .width(360.0)
            )
            .padding(Padding::ZERO.top(6.0)),
        ]
        .spacing(2)
        .into(),
        Tile::Language => column![
            head(w, "language"),
            tongue(ground, Lang::Ru, "Русский", s.lang == Lang::Ru),
            tongue(ground, Lang::En, "English", s.lang == Lang::En),
        ]
        .spacing(2)
        .into(),
        Tile::Device => column![
            head(w, "device-tile"),
            container(row![ui::badge(ui::Machine::here(), 24.0), text(machine_name()).font(theme::SANS).size(11.0).color(ui::faded(FAINT))].spacing(8).align_y(iced::Center))
                .padding(Padding::ZERO.bottom(4.0)),
            container(
                text_input("", ground.renaming.unwrap_or(&s.device))
                    .on_input(Message::Rename)
                    .on_submit(Message::RenameDone)
                    .font(theme::MONO)
                    .size(theme::CAPTION)
                    .padding([6, 10])
                    .width(180.0)
                    .style(theme::field_faded(ui::fade()))
            )
            .padding(Padding::ZERO.top(6.0)),
        ]
        .spacing(2)
        .into(),
        Tile::Scene => column![
            head(w, "scene-tile"),
            pill(ground, "live", w.t("live-replay"), s.live_scene, Message::Scene(!s.live_scene)),
            pill(ground, "pause", w.t("pause-unfocused"), s.pause_unfocused, Message::PauseUnfocused(!s.pause_unfocused)),
        ]
        .spacing(6)
        .into(),
        Tile::Sources => {
            let mut rows = column![head(w, "sources")].spacing(2);
            for (at, source) in s.sources.iter().enumerate() {
                let under = match (source.replay_count, source.maps) {
                    (replays, Some(maps)) if maps > 0 => format!("{} · {}", w.count("replays-count", replays), w.count("maps-count", maps)),
                    (replays, _) => w.count("replays-count", replays),
                };
                rows = rows.push(line(ground, &format!("source-{at}"), short(source.kind), source.shown(), under, source.on, Some(Message::Source(at, !source.on))));
            }
            rows.push(line(ground, "add-folder", "+", w.t("add-folder"), String::new(), false, Some(Message::AddFolder))).into()
        }
        Tile::Skins => {
            let chosen = s.skin.clone();
            let cell = |name: String, folder: Option<PathBuf>, face: Option<&iced::widget::image::Handle>, picked: bool, k: f32| -> Element<'a, Message> {
                let picture: Element<'a, Message> = match face {
                    Some(handle) => iced::widget::image(handle.clone())
                        .content_fit(iced::ContentFit::Cover)
                        .width(56.0)
                        .height(56.0)
                        .border_radius(10.0)
                        .opacity(ui::fade())
                        .into(),
                    None => container(ui::fine_hatch()).width(56.0).height(56.0).into(),
                };
                let framed = iced::widget::stack![picture, ui::frame_mark(k, 56.0)].width(56.0).height(56.0);
                let inside = column![
                    framed,
                    text(ui::shortened(name, 26))
                        .font(theme::SANS)
                        .size(11.0)
                        .align_x(iced::Center)
                        .width(64.0)
                        .height(28.0)
                        .color(ui::faded(if picked { INK } else { MUTED })),
                ]
                .spacing(4)
                .align_x(iced::Center)
                .width(64.0);
                button(inside)
                    .padding([4, 2])
                    .style(ui::button_faded(theme::bare))
                    .on_press(Message::Skin(folder))
                    .into()
            };
            let mut cells: Vec<Element<'a, Message>> = vec![cell(
                w.t("own-skin-short"),
                None,
                ground.skin_faces.get(std::path::Path::new("")),
                chosen.is_none(),
                mark_at(ground, "skin-own", chosen.is_none()),
            )];
            for folder in ground.skins.iter().take(40) {
                let name = crate::settings::skin_name(folder);
                let picked = chosen.as_deref() == Some(folder.as_path());
                let k = mark_at(ground, &format!("skin-{name}"), picked);
                cells.push(cell(name.clone(), Some(folder.clone()), ground.skin_faces.get(folder), picked, k));
            }
            let mut shelf = row![].spacing(6).align_y(iced::alignment::Vertical::Top);
            for cell in cells {
                shelf = shelf.push(cell);
            }
            let strip = container(
                iced::widget::scrollable(shelf)
                    .anchor_x(iced::widget::scrollable::Anchor::Start)
                    .direction(iced::widget::scrollable::Direction::Horizontal(
                        iced::widget::scrollable::Scrollbar::new().width(0).scroller_width(0).margin(0),
                    ))
                    .width(if ground.skins.is_empty() { 200.0 } else { 366.0 }),
            );
            let packed = ground.skins.iter().filter(|path| crate::settings::is_skin_file(path)).count();
            let said = match (ground.skins.is_empty(), packed) {
                (true, _) => w.t("no-skins"),
                (false, 0) => w.n("skins-found", ground.skins.len() as u64),
                (false, packed) => format!("{} · {}", w.n("skins-found", ground.skins.len() as u64), w.n("skins-packed", packed as u64)),
            };
            let under: Element<'a, Message> = text(said).font(theme::SANS).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(FAINT)).into();
            let deeds = row![
                deed(w.t("more-skins"), Message::MoreSkins, false),
                deed(w.t("add-skin"), Message::AddSkin, false),
                deed(w.t("skins-folder"), Message::OpenSkinsFolder, false)
            ]
            .spacing(6);
            column![head(w, "skins"), strip, container(under).padding(Padding::ZERO.top(2.0)), container(deeds).padding(Padding::ZERO.top(6.0))]
                .spacing(2)
                .into()
        }
        Tile::Sound => column![
            head(w, "sound"),
            container(
                column![
                    slide(ground, "music", w.t("music"), percent(s.music_level), s.music_level, Vec::new(), Message::Music),
                    slide(ground, "hits", w.t("hitsounds"), percent(s.hitsound_level), s.hitsound_level, Vec::new(), Message::Hitsounds),
                    slide(ground, "player", w.t("player-sound"), percent(s.player_level), s.player_level, Vec::new(), Message::PlayerLevel),
                ]
                .spacing(6)
                .width(260.0)
            )
            .padding(Padding::ZERO.top(6.0)),
        ]
        .spacing(2)
        .into(),
        Tile::Videos => column![
            head(w, "videos"),
            figure(ground.videos.to_string(), w.mb(ground.videos_size)),
            container(row![deed(w.t("in-folder"), Message::OpenRenders, false), deed(w.t("change"), Message::PickRenders, false)].spacing(6))
                .padding(Padding::ZERO.top(6.0)),
        ]
        .spacing(2)
        .into(),
        Tile::Maps => column![
            head(w, "maps-and-cache"),
            figure(ground.maps.to_string(), format!("{} · {} {}", w.mb(ground.maps_size), w.t("cache"), w.mb(ground.cache_size))),
            container(row![deed(w.t("in-folder"), Message::OpenMaps, false), deed(w.t("clear"), Message::ClearCache, true)].spacing(6))
                .padding(Padding::ZERO.top(6.0)),
        ]
        .spacing(2)
        .into(),
        Tile::Builds => {
            let engine = crate::bot::BUILD.to_owned();
            let under = match &ground.ffmpeg {
                Some(version) => format!("{} · ffmpeg {version}", w.who("engine-is", crate::bot::ENGINE)),
                None => w.t("no-ffmpeg"),
            };
            column![
                head(w, "builds"),
                figure(engine, under),
                container(row![deed(w.t("check"), Message::CheckBuild, false), deed("ffmpeg".to_owned(), Message::GetFfmpeg, false)].spacing(6))
                    .padding(Padding::ZERO.top(6.0)),
            ]
            .spacing(2)
            .into()
        }
        Tile::Account => {
            let name = ground
                .account
                .map(|me| me.name.clone())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| s.linked_as.trim_start_matches('@').to_owned());
            let name = if name.is_empty() { w.t("signed-in") } else { name };
            let under = match ground.account {
                Some(me) if !me.username.is_empty() => format!("@{} · ID {}", me.username, me.telegram_id),
                Some(me) => format!("ID {}", me.telegram_id),
                None => s.linked_as.clone(),
            };
            let face: Element<'a, Message> = match ground.avatar {
                Some(handle) => iced::widget::image(handle.clone())
                    .content_fit(iced::ContentFit::Cover)
                    .width(40.0)
                    .height(40.0)
                    .border_radius(20.0)
                    .opacity(ui::fade())
                    .into(),
                None => ui::disc(&first_letter(&name), false, 40.0),
            };
            let who = row![
                face,
                column![
                    text(name).font(theme::SANS_SEMI).size(theme::BODY).wrapping(text::Wrapping::None).color(ui::faded(INK)),
                    text(under).font(theme::MONO).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(FAINT)),
                ]
                .spacing(2),
            ]
            .spacing(10)
            .align_y(iced::Center);
            column![head(w, "account"), who, container(deed(w.t("sign-out"), Message::SignOut, false)).padding(Padding::ZERO.top(6.0))]
                .spacing(2)
                .into()
        }
        Tile::Chats => {
            let mut rows = column![head(w, "videos-go-to")].spacing(2);
            let here = s.chat_id.or(ground.account.map(|me| me.telegram_id));
            for chat in ground.chats {
                let under = if chat.private { w.t("private-chat") } else { w.t("group-chat") };
                let face: Option<&iced::widget::image::Handle> = ground.chat_faces.get(&chat.id);
                rows = rows.push(chat_line(ground, chat, face, under, Some(chat.id) == here));
            }
            if ground.chats.is_empty() {
                rows = rows.push(line(ground, "chat-own", "@", s.linked_as.clone(), w.t("private-chat"), true, None));
            }
            rows.into()
        }
        Tile::Worker => column![
            head(w, "worker"),
            pill(ground, "worker", w.t("take-jobs"), false, Message::Worker(false)),
            container(text(w.t("coming-later")).font(theme::SANS).size(11.0).color(ui::faded(FAINT))).padding(Padding::ZERO.top(4.0)),
        ]
        .spacing(6)
        .into(),
        Tile::ThisDevice => column![
            head(w, "this-device"),
            row![
                ui::badge(ui::Machine::here(), 28.0),
                column![
                    text(s.device.clone()).font(theme::SANS_SEMI).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(INK)),
                    text(w.t("linked")).font(theme::SANS).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(MUTED)),
                ]
                .spacing(1),
            ]
            .spacing(10)
            .align_y(iced::Center),
            container(deed(w.t("unlink"), Message::Unlink, true)).padding(Padding::ZERO.top(6.0)),
        ]
        .spacing(2)
        .into(),
    }
}

fn short(kind: Kind) -> &'static str {
    match kind {
        Kind::Stable => "stb",
        Kind::Lazer => "lz",
        Kind::Own => "dsr",
        Kind::Folder | Kind::Found => "dir",
    }
}

pub async fn pick_folder() -> Option<Source> {
    let picked = rfd::AsyncFileDialog::new().pick_folder().await?;
    sources::read(picked.path())
}

pub async fn pick_renders() -> Option<PathBuf> {
    let picked = rfd::AsyncFileDialog::new().pick_folder().await?;
    Some(picked.path().to_path_buf())
}

pub fn folder_size(root: &std::path::Path) -> u64 {
    let Ok(read) = std::fs::read_dir(root) else {
        return 0;
    };
    read.flatten()
        .map(|entry| match entry.file_type() {
            Ok(kind) if kind.is_dir() => folder_size(&entry.path()),
            Ok(_) => entry.metadata().map(|meta| meta.len()).unwrap_or(0),
            Err(_) => 0,
        })
        .sum()
}

pub fn clear_cache() {
    let root = sources::own_root();
    for name in ["maps.json", "found.json"] {
        let _ = std::fs::remove_file(root.join(name));
    }
}

pub fn blank<'a>() -> Element<'a, Message> {
    Space::new().width(Length::Fill).height(Length::Fill).into()
}

pub const ACCENT_HINT: iced::Color = ACCENT;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_moved_tile_keeps_the_others_in_order() {
        let kept: Vec<String> = Vec::new();
        let after = moved(&kept, &APP, Tile::Builds, Some(Tile::Language));
        assert_eq!(after[0], Tile::Render.tag());
        assert_eq!(after[1], Tile::Builds.tag());
        assert_eq!(after[2], Tile::Language.tag());
        assert_eq!(after.len(), APP.len());
        let last = moved(&after, &APP, Tile::Render, None);
        assert_eq!(last.last().map(String::as_str), Some(Tile::Render.tag()));
    }

    #[test]
    fn a_slider_lands_on_its_own_steps() {
        assert_eq!(nearest(0.0, &HEIGHTS), HEIGHTS[0]);
        assert_eq!(nearest(1.0, &HEIGHTS), HEIGHTS[HEIGHTS.len() - 1]);
        assert_eq!(nearest(0.5, &HEIGHTS), 1080);
        assert_eq!(nearest(0.51, &CRFS), 20);
        assert_eq!(at(RATES[RATES.len() - 1], &RATES), 1.0);
        assert_eq!(at(HEIGHTS[0], &HEIGHTS), 0.0);
    }
}
