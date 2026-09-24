use std::path::PathBuf;

use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Element, Length, Padding};

use crate::lang::{Lang, Words};
use crate::settings::{Settings, CPU_SHARES, CRFS, HEIGHTS, RATES, SCALES};
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
    Look,
    Device,
    Scene,
    Sources,
    Skins,
    Sound,
    Play,
    Videos,
    Maps,
    Storage,
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
            Tile::Look => "look",
            Tile::Device => "device",
            Tile::Scene => "scene",
            Tile::Sources => "sources",
            Tile::Skins => "skins",
            Tile::Sound => "sound",
            Tile::Play => "play",
            Tile::Videos => "videos",
            Tile::Maps => "maps",
            Tile::Storage => "storage",
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

pub const APP: [Tile; 14] = [
    Tile::Render,
    Tile::Worker,
    Tile::Language,
    Tile::Look,
    Tile::Device,
    Tile::Scene,
    Tile::Sound,
    Tile::Play,
    Tile::Sources,
    Tile::Skins,
    Tile::Videos,
    Tile::Maps,
    Tile::Storage,
    Tile::Builds,
];

pub const BOT: [Tile; 3] = [Tile::Account, Tile::Chats, Tile::ThisDevice];

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
    pub storage: &'a Storage,
    pub ffmpeg: Option<String>,
    pub account: Option<&'a crate::bot::Me>,
    pub avatar: Option<&'a iced::widget::image::Handle>,
    pub chats: &'a [crate::bot::Chat],
    pub chat_faces: &'a std::collections::HashMap<i64, iced::widget::image::Handle>,
    pub renaming: Option<&'a String>,
    pub skins: &'a [PathBuf],
    pub skin_faces: &'a std::collections::HashMap<PathBuf, iced::widget::image::Handle>,
    pub marks: &'a std::collections::HashMap<String, f32>,
    pub slides: &'a std::collections::HashMap<String, (f32, f32)>,
    pub came: f32,
    pub swap: f32,
    pub swap_from: f32,
    pub update: &'a crate::updates::State,
    pub update_waits: bool,
    pub worker: Option<&'a crate::worker::Step>,
    pub worker_last: Option<&'a crate::worker::Step>,
    pub worker_done: u32,
    pub worker_back: u32,
    pub farm: Option<&'a crate::bot::Farm>,
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
    Cpu(f32),
    Scale(f32),
    AutoScale(bool),
    Source(usize, bool),
    AddFolder,
    Added(Option<Source>),
    OpenRenders,
    PickRenders,
    PickedRenders(Option<PathBuf>),
    OpenMaps,
    ClearCache,
    OpenData,
    ClearAppCache,
    CheckBuild,
    Update,
    WhatsNew,
    QuietUpdates(bool),
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
    Dim(f32),
    Blur(f32),
    Hud(bool),
    CursorGrows(bool),
    MapSounds(bool),
    SkinSounds(bool),
    Hitsounds(f32),
    PlayerLevel(f32),
    Unlink,
    SignOut,
    Moved(Tile, Option<Tile>),
}

pub fn view<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let (kept, all) = match ground.side {
        Side::App => (&ground.settings.tiles_app, &APP[..]),
        Side::Bot => (&ground.settings.tiles_bot, &BOT[..]),
    };
    let swap = ground.swap.clamp(0.0, 1.0);
    let tiles = order(kept, all);
    let board = ui::fading(ui::fade() * swap, || {
        let pieces: Vec<(Tile, Element<'a, Message>)> = tiles
            .iter()
            .enumerate()
            .map(|(at, tile)| (*tile, card(ground, *tile, (ground.came * 1.7 - 0.09 * at as f32).clamp(0.0, 1.0))))
            .collect();
        Element::from(ui::grown(crate::board::board(pieces, 10.0, Message::Moved).solid(theme::SLAB_SOLID), iced::Point::new(0.5, 0.0), 0.0, 1.0).shifted((1.0 - swap) * 26.0 * ground.swap_from))
    });
    let grid = container(board).padding(Padding { top: 16.0, right: 40.0, bottom: 28.0, left: 40.0 });
    let rolled = iced::widget::scrollable(grid)
        .id(iced::widget::Id::new("settings-tiles"))
        .anchor_y(iced::widget::scrollable::Anchor::Start)
        .style(ui::thin_scroll)
        .width(Length::Fill)
        .height(Length::Fill);
    let rolled = crate::glide::edged(rolled, iced::widget::Id::new("settings-tiles"));
    column![rolled].width(Length::Fill).height(Length::Fill).into()
}

fn card<'a>(ground: &Ground<'a>, tile: Tile, late: f32) -> Element<'a, Message> {
    let k = (ui::fade() * late).clamp(0.0, 1.0);
    let inside: Element<'a, Message> = ui::fading(k.powf(2.2), || one(ground, tile));
    let card: Element<'a, Message> = container(inside).padding([14, 16]).style(ui::box_at(theme::slab, k.powf(0.6))).into();
    ui::grown(card, iced::Point::new(0.5, 0.5), 0.0, 0.96 + 0.04 * late).into()
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
        .padding(Padding { top: 2.0, right: 0.0, bottom: 12.0, left: 1.0 })
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
                    slide(ground, "cpu", w.t("render-cpu"), format!("{} %", s.cpu_share), at(s.cpu_share, &CPU_SHARES), stops(&CPU_SHARES), Message::Cpu),
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
        Tile::Look => {
            let shown = if s.ui_scale == 0 { ui::auto_scale() } else { s.ui_scale };
            let nearest_stop = SCALES.iter().copied().min_by_key(|stop| stop.abs_diff(shown)).unwrap_or(100);
            let value = if s.ui_scale == 0 { format!("{} · {} %", w.t("scale-auto"), shown) } else { format!("{shown} %") };
            column![
                head(w, "look-tile"),
                pill(ground, "auto-scale", w.t("scale-to-monitor"), s.ui_scale == 0, Message::AutoScale(s.ui_scale != 0)),
                container(slide(ground, "scale", w.t("scale"), value, at(nearest_stop, &SCALES), stops(&SCALES), Message::Scale))
                    .width(280.0)
                    .padding(Padding::ZERO.top(8.0)),
            ]
            .spacing(2)
            .into()
        }
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
                let mut under = match (source.replay_count, source.maps) {
                    (replays, Some(maps)) if maps > 0 => format!("{} · {}", w.count("replays-count", replays), w.count("maps-count", maps)),
                    (replays, _) => w.count("replays-count", replays),
                };
                if source.scores > 0 {
                    under = format!("{under} · {}", w.count("scores-count", source.scores));
                }
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
        Tile::Play => column![
            head(w, "play-tile"),
            container(
                column![
                    slide(ground, "dim", w.t("background-dim"), percent(s.background_dim), s.background_dim, Vec::new(), Message::Dim),
                    slide(ground, "blur", w.t("background-blur"), percent(s.background_blur), s.background_blur, Vec::new(), Message::Blur),
                ]
                .spacing(6)
                .width(280.0)
            )
            .padding(Padding { top: 6.0, right: 0.0, bottom: 4.0, left: 0.0 }),
            ui::wrap(
                vec![
                    pill(ground, "hud", w.t("hud"), s.hud, Message::Hud(!s.hud)),
                    pill(ground, "cursor-grows", w.t("cursor-grows"), s.cursor_grows, Message::CursorGrows(!s.cursor_grows)),
                    pill(ground, "map-sounds", w.t("map-sounds"), s.map_sounds, Message::MapSounds(!s.map_sounds)),
                    pill(ground, "skin-sounds", w.t("skin-sounds"), s.skin_sounds, Message::SkinSounds(!s.skin_sounds)),
                ],
                6.0,
            ),
        ]
        .spacing(4)
        .width(Length::Shrink)
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
        Tile::Storage => {
            let k = ui::fade();
            let store = ground.storage;
            let rest = store.data.saturating_sub(store.videos + store.skins + store.maps + store.cache);
            let parts = [
                ("storage-videos", store.videos, theme::ACCENT),
                ("storage-skins", store.skins, theme::GRADE_C),
                ("storage-maps", store.maps, theme::HIT_300),
                ("storage-cache", store.cache, theme::GRADE_S),
                ("storage-rest", rest, FAINT),
            ];
            let total = parts.iter().map(|part| part.1).sum::<u64>().max(1);
            let mut bar = row![].spacing(2).height(8.0).width(Length::Fill);
            for (_, size, colour) in parts {
                if size > 0 {
                    let share = ((size as f64 / total as f64) * 1000.0).round().max(1.0) as u16;
                    bar = bar.push(container(Space::new().height(8.0)).width(Length::FillPortion(share)).style(move |_| container::Style {
                        background: Some(iced::Background::Color(iced::Color { a: k, ..colour })),
                        border: iced::Border { radius: 3.0.into(), ..iced::Border::default() },
                        ..container::Style::default()
                    }));
                }
            }
            let mut legend = column![].spacing(5);
            for (key, size, colour) in parts {
                let dot = container(Space::new().width(8.0).height(8.0)).style(move |_| container::Style {
                    background: Some(iced::Background::Color(iced::Color { a: k, ..colour })),
                    border: iced::Border { radius: 4.0.into(), ..iced::Border::default() },
                    ..container::Style::default()
                });
                legend = legend.push(
                    row![dot, text(w.t(key)).font(theme::SANS).size(12.0).color(ui::faded(MUTED)), ui::grow(), text(w.mb(size)).font(theme::MONO).size(12.0).color(ui::faded(INK))]
                        .spacing(8)
                        .align_y(iced::Center),
                );
            }
            column![
                head(w, "storage"),
                figure(w.mb(store.data), format!("{} {}", w.t("storage-app"), w.mb(store.app))),
                container(bar).padding(Padding { top: 10.0, right: 0.0, bottom: 8.0, left: 0.0 }),
                legend,
                container(row![deed(w.t("in-folder"), Message::OpenData, false), deed(w.t("clear-cache"), Message::ClearAppCache, true)].spacing(6)).padding(Padding::ZERO.top(8.0)),
            ]
            .spacing(2)
            .width(Length::Fixed(260.0))
            .into()
        }
        Tile::Builds => {
            use crate::updates::State as U;
            let engine = crate::bot::BUILD.to_owned();
            let mut under: Vec<String> = Vec::new();
            if crate::bot::PRERELEASE {
                under.push(w.t("prerelease"));
            }
            if ground.ffmpeg.is_none() {
                under.push(w.t("no-ffmpeg"));
            }
            let under = under.join(" · ");
            let pre = |release: &crate::updates::Release| if release.pre { format!(" · {}", w.t("prerelease")) } else { String::new() };
            let (said, colour) = match ground.update {
                U::Unknown | U::Checking => (w.t("update-checking"), FAINT),
                U::Latest { at } => (format!("{} · {}", w.t("update-latest"), w.clock(*at)), FAINT),
                U::Found(release) => (format!("{} {}{}", w.t("update-found"), release.version, pre(release)), ACCENT),
                U::Getting { release, done, total } => {
                    let how_far = match total {
                        Some(total) if *total > 0 => format!("{} %", (done * 100 / total).min(100)),
                        _ => w.mb(*done),
                    };
                    (format!("{} {} · {how_far}", w.t("update-getting"), release.version), INK)
                }
                U::Ready { release, .. } => {
                    let when = if ground.update_waits {
                        w.t("update-ready-waits")
                    } else if s.quiet_updates {
                        w.t("update-ready-quiet")
                    } else {
                        w.t("update-ready")
                    };
                    (format!("{} · {when}", release.version), INK)
                }
                U::Failed { why, .. } => (format!("{} · {why}", w.t("update-failed")), ACCENT),
                U::Source => (w.t("update-source"), FAINT),
            };
            let said = text(ui::shortened(said, 52)).font(theme::SANS).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(colour));
            let mut deeds = row![].spacing(6);
            match ground.update {
                U::Found(_) | U::Failed { release: Some(_), .. } | U::Ready { .. } => deeds = deeds.push(deed(w.t("update-now"), Message::Update, false)),
                U::Latest { .. } | U::Failed { release: None, .. } => deeds = deeds.push(deed(w.t("check"), Message::CheckBuild, false)),
                _ => {}
            }
            let deeds = deeds.push(deed(w.t("whats-new"), Message::WhatsNew, false)).push(deed("ffmpeg".to_owned(), Message::GetFfmpeg, false));
            column![
                head(w, "builds"),
                figure(engine, under),
                container(said).padding(Padding::ZERO.top(6.0)),
                container(deeds).padding(Padding::ZERO.top(6.0)),
                container(pill(ground, "quiet-updates", w.t("quiet-updates"), s.quiet_updates, Message::QuietUpdates(!s.quiet_updates))).padding(Padding::ZERO.top(8.0)),
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
        Tile::Worker => worker_tile(ground),
        Tile::ThisDevice => column![
            head(w, "this-device"),
            row![
                ui::badge(ui::Machine::here(), 28.0),
                column![
                    text(s.device.clone()).font(theme::SANS_SEMI).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(INK)),
                    text(w.t("linked-status")).font(theme::SANS).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(MUTED)),
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

pub fn worker_said(w: &Words, step: Option<&crate::worker::Step>, on: bool) -> (String, iced::Color) {
    use crate::worker::{Getting, Step as W};
    let (green, gold) = (theme::HIT_100, theme::GRADE_S);
    match (step, on) {
        (_, false) => (w.t("worker-off"), FAINT),
        (None | Some(W::Waiting { .. }) | Some(W::Stopped) | Some(W::Delivered { .. }) | Some(W::HandedBack { .. }), true) => (w.t("worker-waiting"), green),
        (Some(W::Resting), true) => (w.t("worker-resting"), MUTED),
        (Some(W::Offline(why)), true) => (format!("{} · {why}", w.t("worker-offline")), ACCENT),
        (Some(W::Taken { .. }), true) => (w.t("worker-taken"), gold),
        (Some(W::Getting { what, .. }), true) => (
            w.t(match what {
                Getting::Replay => "worker-getting-replay",
                Getting::Map => "worker-getting-map",
                Getting::Skin => "worker-getting-skin",
            }),
            gold,
        ),
        (Some(W::Drawing { .. }), true) => (w.t("worker-drawing"), gold),
        (Some(W::Polishing { .. }), true) => (w.t("worker-polishing"), gold),
        (Some(W::Sending { .. }), true) => (w.t("worker-sending"), gold),
    }
}

fn worker_tile<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    use crate::worker::Step as W;
    let w = ground.words;
    let s = ground.settings;
    let k = ui::fade();
    let on = s.worker_on;
    let (green, gold) = (theme::HIT_100, theme::GRADE_S);
    let dot = move |colour: iced::Color| -> Element<'a, Message> {
        container(Space::new().width(7.0).height(7.0))
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(iced::Color { a: k * colour.a, ..colour })),
                border: iced::Border { radius: 4.0.into(), ..iced::Border::default() },
                ..container::Style::default()
            })
            .into()
    };
    let small = |words: String, colour: iced::Color| -> Element<'a, Message> { text(words).font(theme::SANS).size(11.0).color(ui::faded(colour)).into() };
    let (said, colour) = worker_said(w, ground.worker, on);
    let mut state = row![dot(colour), text(ui::shortened(said, 40)).font(theme::SANS_SEMI).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(colour))]
        .spacing(8)
        .align_y(iced::Center)
        .width(Length::Fill);
    if let Some(W::Drawing { fps, .. }) = ground.worker {
        if *fps > 0.0 {
            state = state.push(ui::grow()).push(ui::mono_small(w.with("worker-fps", &[("n", format!("{}", fps.round() as u64))]), MUTED));
        }
    }
    let mut body = column![head(w, "worker"), pill(ground, "worker", w.t("worker-take"), on, Message::Worker(!on))].spacing(2).width(Length::Fixed(330.0));
    if !on {
        body = body.push(container(small(w.t("worker-about"), FAINT)).padding(Padding::ZERO.top(8.0)));
    }
    body = body.push(container(state).padding(Padding::ZERO.top(10.0)));
    let current = match ground.worker {
        Some(W::Taken { title } | W::Getting { title, .. } | W::Polishing { title }) => Some((title.clone(), None, None)),
        Some(W::Drawing { title, done, of, left_seconds, .. }) => {
            let left = *left_seconds as u64;
            Some((title.clone(), Some(if *of > 0 { *done as f32 / *of as f32 } else { 0.0 }), Some(w.with("worker-left", &[("left", format!("{}:{:02}", left / 60, left % 60))]))))
        }
        Some(W::Sending { title, done, of }) => Some((title.clone(), Some(if *of > 0 { *done as f32 / *of as f32 } else { 0.0 }), None)),
        _ => None,
    };
    if let Some((title, share, left)) = current.filter(|_| on) {
        let mut block = column![ui::marquee(vec![ui::piece(title, theme::SANS, theme::CAPTION, INK)])].spacing(6);
        if let Some(share) = share {
            let filled = (share.clamp(0.0, 1.0) * 1000.0).round().max(1.0) as u16;
            block = block.push(row![
                container(Space::new().height(4.0)).width(Length::FillPortion(filled)).style(move |_| container::Style {
                    background: Some(iced::Background::Color(iced::Color { a: k, ..gold })),
                    border: iced::Border { radius: 2.0.into(), ..iced::Border::default() },
                    ..container::Style::default()
                }),
                Space::new().width(Length::FillPortion(1000u16.saturating_sub(filled).max(1))).height(4.0),
            ]);
        }
        if let Some(left) = left {
            block = block.push(ui::mono_small(left, MUTED));
        }
        body = body.push(container(block).padding(Padding::ZERO.top(8.0)));
    }
    body = body.push(
        container(ui::mono_small(format!("{} {} · {} {}", w.t("worker-delivered"), ground.worker_done, w.t("worker-handed-back"), ground.worker_back), FAINT))
            .padding(Padding::ZERO.top(8.0)),
    );
    let mut problems: Vec<String> = Vec::new();
    if s.token.is_empty() {
        problems.push(w.t("worker-not-paired"));
    }
    if ground.ffmpeg.is_none() {
        problems.push(w.t("worker-no-ffmpeg"));
    }
    for problem in problems {
        body = body.push(container(row![text("✕").font(theme::SANS_SEMI).size(11.0).color(ui::faded(ACCENT)), small(problem, INK)].spacing(6)).padding(Padding::ZERO.top(6.0)));
    }
    match ground.worker_last {
        Some(W::Delivered { title }) => body = body.push(container(small(ui::shortened(w.with("worker-last-delivered", &[("title", title.clone())]), 60), green)).padding(Padding::ZERO.top(6.0))),
        Some(W::HandedBack { title, reason }) => {
            body = body.push(container(small(ui::shortened(w.with("worker-last-back", &[("title", title.clone()), ("reason", reason.clone())]), 80), ACCENT)).padding(Padding::ZERO.top(6.0)))
        }
        _ => {}
    }
    if !s.token.is_empty() {
        let waiting = ground.farm.map_or(0, |farm| farm.waiting);
        let mut farm = column![row![text(w.t("farm-head")).font(theme::SANS_SEMI).size(theme::CAPTION).color(ui::faded(INK)), ui::grow(), ui::mono_small(w.with("farm-waiting", &[("n", waiting.to_string())]), MUTED)].align_y(iced::Center)]
            .spacing(6);
        let workers = ground.farm.map(|farm| farm.workers.clone()).unwrap_or_default();
        if workers.is_empty() {
            farm = farm.push(small(w.t("farm-nobody"), FAINT));
        }
        for worker in workers.into_iter().take(5) {
            let (said, colour) = match worker.state.as_str() {
                "rendering" => (w.t("farm-rendering"), gold),
                "ready" => (w.t("farm-ready"), green),
                _ => (w.t("farm-resting"), FAINT),
            };
            let name = if worker.mine { format!("{} · {}", worker.name, w.t("farm-this")) } else { worker.name.clone() };
            farm = farm.push(
                row![
                    dot(colour),
                    text(ui::shortened(name, 30)).font(theme::SANS).size(12.0).wrapping(text::Wrapping::None).color(ui::faded(if worker.mine { INK } else { MUTED })),
                    ui::grow(),
                    small(said, colour),
                    ui::mono_small(worker.delivered.to_string(), FAINT),
                ]
                .spacing(8)
                .align_y(iced::Center),
            );
        }
        body = body.push(container(farm).padding(Padding::ZERO.top(14.0)));
    }
    body.into()
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

pub async fn pick_skin() -> Option<PathBuf> {
    let picked = rfd::AsyncFileDialog::new().add_filter("osu!", &["osk", "ini"]).pick_file().await?;
    Some(picked.path().to_path_buf())
}

pub async fn pick_renders() -> Option<PathBuf> {
    let picked = rfd::AsyncFileDialog::new().pick_folder().await?;
    Some(picked.path().to_path_buf())
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Storage {
    pub app: u64,
    pub data: u64,
    pub videos: u64,
    pub skins: u64,
    pub maps: u64,
    pub cache: u64,
}

const CACHES: [&str; 8] = ["cache", "maps.json", "found.json", "news.json", "community.json", "card.json", "osu-profile.json", "people.json"];

pub fn app_size() -> u64 {
    let Ok(exe) = std::env::current_exe() else {
        return 0;
    };
    match exe.ancestors().find(|folder| folder.extension().is_some_and(|ext| ext == "app")) {
        Some(bundle) => folder_size(bundle),
        None => folder_size(&exe),
    }
}

pub fn measure(renders: &std::path::Path) -> Storage {
    let root = sources::own_root();
    let videos = folder_size(renders);
    let inside = renders.starts_with(&root);
    Storage {
        app: app_size(),
        data: folder_size(&root) + if inside { 0 } else { videos },
        videos,
        skins: folder_size(&root.join("Skins")) + folder_size(&root.join("worker-skins")),
        maps: folder_size(&root.join("Songs")),
        cache: CACHES.iter().map(|name| folder_size(&root.join(name))).sum(),
    }
}

pub fn clear_app_cache() {
    let root = sources::own_root();
    for name in CACHES {
        let path = root.join(name);
        if path.is_dir() {
            let _ = std::fs::remove_dir_all(&path);
        } else {
            let _ = std::fs::remove_file(&path);
        }
    }
}

pub fn folder_size(root: &std::path::Path) -> u64 {
    if let Ok(meta) = std::fs::symlink_metadata(root) {
        if meta.is_file() {
            return meta.len();
        }
    }
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
        let after = moved(&kept, &APP, Tile::Builds, Some(Tile::Worker));
        assert_eq!(after[0], Tile::Render.tag());
        assert_eq!(after[1], Tile::Builds.tag());
        assert_eq!(after[2], Tile::Worker.tag());
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
