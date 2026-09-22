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
    pub dragging: Option<Tile>,
    pub renaming: Option<&'a String>,
    pub skins: &'a [PathBuf],
    pub skin_face: Option<&'a iced::widget::image::Handle>,
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
    Music(f32),
    Hitsounds(f32),
    PlayerLevel(f32),
    Unlink,
    SignOut,
    Drag(Tile),
    DragAt(iced::Point),
    DropBefore(Option<Tile>),
    Dropped,
}

pub fn view<'a>(ground: &Ground<'a>) -> Element<'a, Message> {
    let w = ground.words;
    let word = |key: &str, side: Side| {
        button(text(w.t(key)).font(theme::SANS_SEMI).size(theme::BODY))
            .padding([4, 0])
            .style(theme::word(ground.side == side))
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
    for (at, tile) in order(kept, all).into_iter().enumerate() {
        let late = (ground.came * 1.7 - 0.09 * at as f32).clamp(0.0, 1.0);
        tiles.push(draggable(ground, tile, late));
    }
    let swap = ground.swap.clamp(0.0, 1.0);
    let slid = ui::grown(ui::wrap(tiles, 10.0), iced::Point::new(0.5, 0.0), 0.0, 1.0).shifted((1.0 - swap) * 26.0 * ground.swap_from);
    let grid = container(ui::fading(ui::fade() * swap, || slid)).padding(Padding { top: 16.0, right: 40.0, bottom: 24.0, left: 40.0 });
    column![container(switch).padding(Padding::ZERO.top(22.0)), grid].width(Length::Fill).into()
}

fn draggable<'a>(ground: &Ground<'a>, tile: Tile, late: f32) -> Element<'a, Message> {
    let held = ground.dragging == Some(tile);
    let face = one(ground, tile);
    let card: Element<'a, Message> = ui::fading(ui::fade() * late, || {
        let made = container(face).padding([12, 14]).style(ui::box_faded(theme::slab));
        if held {
            ui::hollow(made).into()
        } else {
            Element::from(made)
        }
    });
    let risen = ui::grown(card, iced::Point::new(0.5, 0.0), -(1.0 - late) * 10.0, 0.96 + 0.04 * late);
    ui::dragged(risen.into(), tile, Message::Drag(tile), Message::DropBefore(Some(tile)), Message::Dropped, held)
}

pub fn floating<'a>(ground: &Ground<'a>, tile: Tile) -> Element<'a, Message> {
    container(one(ground, tile))
        .padding([12, 14])
        .style(ui::box_faded(theme::slab_held))
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
        Some(message) => button(inside).padding([2, 2]).style(theme::bare).on_press(message).into(),
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
        .style(theme::pill(on))
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
        .style(if hot { theme::small_hot } else { theme::small })
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
            line(ground, "lang-ru", "RU", "Русский".to_owned(), String::new(), s.lang == Lang::Ru, Some(Message::PickLang(Lang::Ru))),
            line(ground, "lang-en", "EN", "English".to_owned(), String::new(), s.lang == Lang::En, Some(Message::PickLang(Lang::En))),
        ]
        .spacing(2)
        .into(),
        Tile::Device => column![
            head(w, "device-tile"),
            container(
                text_input("", ground.renaming.unwrap_or(&s.device))
                    .on_input(Message::Rename)
                    .on_submit(Message::RenameDone)
                    .font(theme::MONO)
                    .size(theme::CAPTION)
                    .padding([6, 10])
                    .width(180.0)
                    .style(theme::field)
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
            let face: Element<'a, Message> = match ground.skin_face {
                Some(handle) => iced::widget::image(handle.clone())
                    .content_fit(iced::ContentFit::Cover)
                    .width(44.0)
                    .height(44.0)
                    .border_radius(8.0)
                    .opacity(ui::fade())
                    .into(),
                None => container(ui::hatch()).width(44.0).height(44.0).into(),
            };
            let top = row![
                face,
                column![
                    text(match &chosen {
                        Some(folder) => ui::shortened(folder.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), 22),
                        None => w.t("own-skin"),
                    })
                    .font(theme::SANS_SEMI)
                    .size(theme::CAPTION)
                    .wrapping(text::Wrapping::None)
                    .color(ui::faded(INK)),
                    text(w.n("skins-found", ground.skins.len() as u64)).font(theme::SANS).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(FAINT)),
                ]
                .spacing(2),
            ]
            .spacing(10)
            .align_y(iced::Center);
            let mut rows = column![head(w, "skins"), top].spacing(2);
            rows = rows.push(container(Space::new().height(6.0)));
            if s.skin.is_some() {
                rows = rows.push(line(ground, "skin-own", "·", w.t("own-skin"), String::new(), false, Some(Message::Skin(None))));
            }
            for folder in ground.skins.iter().take(5) {
                let name = folder.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                let picked = chosen.as_deref() == Some(folder.as_path());
                rows = rows.push(line(
                    ground,
                    &format!("skin-{name}"),
                    "skn",
                    ui::shortened(name, 22),
                    String::new(),
                    picked,
                    Some(Message::Skin(Some(folder.clone()))),
                ));
            }
            let mut deeds = row![deed(w.t("rescan"), Message::RescanSkins, false)].spacing(6);
            if chosen.is_some() {
                deeds = deeds.push(deed(w.t("in-folder"), Message::OpenSkin, false));
            }
            rows.push(container(deeds).padding(Padding::ZERO.top(6.0))).into()
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
                None => ui::mark("", true, 1.0, 40.0),
            };
            let who = row![
                face,
                column![
                    text(if name.is_empty() { w.t("signed-in") } else { name }).font(theme::SANS_SEMI).size(theme::BODY).wrapping(text::Wrapping::None).color(ui::faded(INK)),
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
                rows = rows.push(line(
                    ground,
                    &format!("chat-{}", chat.id),
                    if chat.private { "@" } else { "#" },
                    chat.title.clone(),
                    under,
                    Some(chat.id) == here,
                    Some(Message::Chat(chat.id)),
                ));
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
            line(ground, "this-device", "·", s.device.clone(), w.t("linked"), false, None),
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
        assert_eq!(nearest(0.0, &HEIGHTS), 720);
        assert_eq!(nearest(0.5, &HEIGHTS), 1080);
        assert_eq!(nearest(1.0, &HEIGHTS), 1440);
        assert_eq!(nearest(0.4, &CRFS), 20);
        assert_eq!(at(60, &RATES), 1.0);
    }
}
