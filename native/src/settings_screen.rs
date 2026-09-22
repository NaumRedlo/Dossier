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
    Videos,
    Maps,
    Builds,
    Account,
    Chats,
    Tells,
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
            Tile::Videos => "videos",
            Tile::Maps => "maps",
            Tile::Builds => "builds",
            Tile::Account => "account",
            Tile::Chats => "chats",
            Tile::Tells => "tells",
            Tile::Worker => "worker",
            Tile::ThisDevice => "this-device",
        }
    }

    pub fn of(tag: &str) -> Option<Tile> {
        APP.iter().chain(BOT.iter()).copied().find(|tile| tile.tag() == tag)
    }
}

pub const APP: [Tile; 8] = [
    Tile::Render,
    Tile::Language,
    Tile::Device,
    Tile::Scene,
    Tile::Sources,
    Tile::Videos,
    Tile::Maps,
    Tile::Builds,
];

pub const BOT: [Tile; 5] = [Tile::Account, Tile::Chats, Tile::Tells, Tile::Worker, Tile::ThisDevice];

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
    Tell(&'static str, bool),
    Worker(bool),
    Unlink,
    SignOut,
    Drag(Tile),
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
    for tile in order(kept, all) {
        tiles.push(draggable(ground, tile));
    }
    let grid = container(ui::wrap(tiles, 10.0)).padding(Padding { top: 16.0, right: 40.0, bottom: 24.0, left: 40.0 });
    column![container(switch).padding(Padding::ZERO.top(22.0)), grid].width(Length::Fill).into()
}

fn draggable<'a>(ground: &Ground<'a>, tile: Tile) -> Element<'a, Message> {
    let held = ground.dragging == Some(tile);
    let face = one(ground, tile);
    let card = container(face)
        .padding([12, 14])
        .style(ui::box_faded(if held { theme::slab_held } else { theme::slab }))
        .into();
    ui::dragged(card, tile, Message::Drag(tile), Message::DropBefore(Some(tile)), Message::Dropped, held)
}

fn head<'a>(w: &Words, key: &str) -> Element<'a, Message> {
    text(w.t(key)).font(theme::SANS_SEMI).size(13.0).color(ui::faded(INK)).into()
}

fn line<'a>(glyph: &str, name: String, under: String, on: bool, press: Option<Message>) -> Element<'a, Message> {
    let mut words = column![text(name).font(theme::SANS_SEMI).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(INK))].spacing(1);
    if !under.is_empty() {
        words = words.push(text(under).font(theme::SANS).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(MUTED)));
    }
    let inside = row![ui::mark(glyph, on, 28.0), words].spacing(10).align_y(iced::Center);
    match press {
        Some(message) => button(inside).padding([2, 2]).style(theme::bare).on_press(message).into(),
        None => container(inside).padding([2, 2]).into(),
    }
}

fn pill<'a>(name: String, on: bool, press: Message) -> Element<'a, Message> {
    let inside = row![ui::mark("", on, 22.0), text(name).font(theme::SANS_SEMI).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(INK))]
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
                    ui::steps(w.t("render-size"), format!("{}p", s.render_height), at(s.render_height, &HEIGHTS), stops(&HEIGHTS), Message::Height),
                    ui::steps(w.t("render-frames"), s.render_fps.to_string(), at(s.render_fps, &RATES), stops(&RATES), Message::Rate),
                    ui::steps(w.t("render-quality"), format!("CRF {}", s.render_crf), at(s.render_crf, &CRFS), stops(&CRFS), Message::Crf),
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
            line("RU", "Русский".to_owned(), String::new(), s.lang == Lang::Ru, Some(Message::PickLang(Lang::Ru))),
            line("EN", "English".to_owned(), String::new(), s.lang == Lang::En, Some(Message::PickLang(Lang::En))),
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
            pill(w.t("live-replay"), s.live_scene, Message::Scene(!s.live_scene)),
            pill(w.t("pause-unfocused"), s.pause_unfocused, Message::PauseUnfocused(!s.pause_unfocused)),
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
                rows = rows.push(line(short(source.kind), source.shown(), under, source.on, Some(Message::Source(at, !source.on))));
            }
            rows.push(line("+", w.t("add-folder"), String::new(), false, Some(Message::AddFolder))).into()
        }
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
                None => ui::mark("", true, 40.0),
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
                    if chat.private { "@" } else { "#" },
                    chat.title.clone(),
                    under,
                    Some(chat.id) == here,
                    Some(Message::Chat(chat.id)),
                ));
            }
            if ground.chats.is_empty() {
                rows = rows.push(line("@", s.linked_as.clone(), w.t("private-chat"), true, None));
            }
            rows.into()
        }
        Tile::Tells => column![
            head(w, "bot-tells"),
            pill(w.t("rendered-notice"), s.tell.rendered, Message::Tell("rendered", !s.tell.rendered)),
            pill(w.t("errors"), s.tell.errors, Message::Tell("errors", !s.tell.errors)),
            pill(w.t("fetched-maps"), s.tell.maps, Message::Tell("maps", !s.tell.maps)),
            pill(w.t("worker-jobs"), s.tell.worker, Message::Tell("worker", !s.tell.worker)),
        ]
        .spacing(6)
        .into(),
        Tile::Worker => column![
            head(w, "worker"),
            pill(w.t("take-jobs"), false, Message::Worker(false)),
            container(text(w.t("coming-later")).font(theme::SANS).size(11.0).color(ui::faded(FAINT))).padding(Padding::ZERO.top(4.0)),
        ]
        .spacing(6)
        .into(),
        Tile::ThisDevice => column![
            head(w, "this-device"),
            line("·", s.device.clone(), w.t("linked"), false, None),
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
