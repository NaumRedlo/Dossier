use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use iced::animation::Easing;
use iced::widget::{
    button, column, container, float, image, mouse_area, pin, row, scrollable, stack, text,
    tooltip, Space,
};
use iced::{window, Animation, Color, ContentFit, Element, Length, Padding, Subscription, Task, Vector};

use crate::lang::{typed, Words};
use crate::library::{self, Entry, Grade, Library};
use crate::maps;
use crate::render::{self, Step};
use crate::scan;
use crate::settings::Settings;
use crate::sources::Kind;
use crate::theme::{self, ACCENT, FAINT, INK, MUTED};
use crate::ui::{self, Line, Mood};

pub const ENTER: Duration = Duration::from_millis(1200);
pub const ARRIVE: Duration = Duration::from_millis(450);
pub const SWAP: Duration = Duration::from_millis(450);
pub const LIFT: Duration = Duration::from_millis(200);
pub const BRAND_WIDTH: f32 = 144.0;
const CREST_HOME: (f32, f32) = (40.0, 24.0);
const CREST_RISE: f32 = 8.0;
const JOURNAL_RISE: f32 = 140.0;
const THUMB: (u32, u32) = (176, 100);
const SCENE_WIDTH: u32 = 960;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    None,
    Worker,
    Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Library),
    Thumb(String, image::Handle),
    Scene(String, Option<image::Handle>),
    Length(PathBuf, i64),
    Choose(usize),
    Step(i32),
    Escape,
    Hover(Option<usize>),
    Show(Overlay),
    OpenFolder,
    Render,
    StopRender,
    Rendered(Step),
    OpenOut,
    ShowOut,
    GetMap,
    StopFetch,
    Fetched(maps::Step),
    Look,
    StopLook,
    Looked(scan::Step),
    Dropped(PathBuf),
    Resized(f32),
    Tick(Instant),
}

#[derive(Debug, Clone, Default)]
pub struct Shown {
    pub date: String,
    pub player: String,
    pub map: String,
    pub meta: String,
    pub accuracy: String,
    pub outcome: String,
}

#[derive(Debug, Clone)]
pub struct Rendering {
    pub path: PathBuf,
    pub reached: Vec<Step>,
    pub out: Option<PathBuf>,
}

impl Rendering {
    pub fn last(&self) -> Option<&Step> {
        self.reached.last()
    }

    pub fn is_over(&self) -> bool {
        self.last().is_some_and(Step::is_last)
    }
}

#[derive(Debug, Clone)]
pub struct Fetching {
    pub hash: String,
    pub reached: Vec<maps::Step>,
}

impl Fetching {
    pub fn last(&self) -> Option<&maps::Step> {
        self.reached.last()
    }

    pub fn is_over(&self) -> bool {
        self.last().is_some_and(maps::Step::is_last)
    }
}

#[derive(Clone)]
pub struct Main {
    pub words: Words,
    pub settings: Settings,
    pub library: Option<Library>,
    pub chosen: Option<usize>,
    pub before: Option<Shown>,
    pub search: String,
    pub hover: Option<usize>,
    pub thumbs: HashMap<String, image::Handle>,
    pub scenes: HashMap<String, image::Handle>,
    pub scene_before: Option<Option<image::Handle>>,
    pub lengths: HashMap<PathBuf, i64>,
    pub overlay: Overlay,
    pub rendering: Option<Rendering>,
    pub fetching: Option<Fetching>,
    pub looking: Option<scan::Step>,
    pub ffmpeg: Option<PathBuf>,
    pub now: Instant,
    pub started: Instant,
    pub now_unix: i64,
    pub enter: Animation<bool>,
    pub arrive: Animation<bool>,
    pub swap: Animation<bool>,
    pub lift: Animation<bool>,
    pub lifted: Option<usize>,
    pub width: f32,
    strip_id: iced::widget::Id,
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

impl Main {
    pub fn new(words: Words, settings: Settings) -> (Main, Task<Message>) {
        let sources = settings.sources.clone();
        let made = Main {
            words,
            settings,
            library: None,
            chosen: None,
            before: None,
            search: String::new(),
            hover: None,
            thumbs: HashMap::new(),
            scenes: HashMap::new(),
            scene_before: None,
            lengths: HashMap::new(),
            overlay: Overlay::None,
            rendering: None,
            fetching: None,
            looking: None,
            ffmpeg: crate::checks::ffmpeg_on_path(),
            now: Instant::now(),
            started: Instant::now(),
            now_unix: unix_now(),
            enter: Animation::new(false).duration(ENTER).easing(Easing::EaseOutCubic),
            arrive: Animation::new(false).duration(ARRIVE).easing(Easing::EaseOutCubic).go(true, Instant::now()),
            swap: Animation::new(true).duration(SWAP).easing(Easing::EaseOutCubic),
            lift: Animation::new(false).duration(LIFT).easing(Easing::EaseOutCubic),
            lifted: None,
            width: crate::WINDOW.width,
            strip_id: iced::widget::Id::unique(),
        };
        (made, ui::in_thread(move || library::read(&sources)).map(Message::Loaded))
    }

    pub fn staged(words: Words, settings: Settings, library: Library, chosen: Option<usize>) -> Main {
        let (mut made, _) = Main::new(words, settings);
        made.library = Some(library);
        made.chosen = chosen;
        made.enter = Animation::new(true);
        made.arrive = Animation::new(true);
        made
    }

    pub fn moving(&self) -> bool {
        self.rendering.as_ref().is_some_and(|r| !r.is_over())
            || self.fetching.as_ref().is_some_and(|f| !f.is_over())
            || matches!(self.looking, Some(scan::Step::Looking { .. }))
            || self.enter.is_animating(self.now)
            || self.arrive.is_animating(self.now)
            || self.swap.is_animating(self.now)
            || self.lift.is_animating(self.now)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut parts = vec![iced::event::listen_with(|event, status, _| match (event, status) {
            (iced::Event::Window(window::Event::FileDropped(path)), _) => Some(Message::Dropped(path)),
            (iced::Event::Window(window::Event::Resized(size)), _) => Some(Message::Resized(size.width)),
            (iced::Event::Window(window::Event::Opened { size, .. }), _) => Some(Message::Resized(size.width)),
            (iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, .. }), iced::event::Status::Ignored) => {
                use iced::keyboard::key::{Key, Named};
                match key.as_ref() {
                    Key::Named(Named::ArrowLeft) | Key::Named(Named::ArrowUp) => Some(Message::Step(-1)),
                    Key::Named(Named::ArrowRight) | Key::Named(Named::ArrowDown) => Some(Message::Step(1)),
                    Key::Named(Named::Escape) => Some(Message::Escape),
                    _ => None,
                }
            }
            (iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, .. }), iced::event::Status::Captured) => {
                use iced::keyboard::key::{Key, Named};
                match key.as_ref() {
                    Key::Named(Named::ArrowUp) => Some(Message::Step(-1)),
                    Key::Named(Named::ArrowDown) => Some(Message::Step(1)),
                    Key::Named(Named::Escape) => Some(Message::Escape),
                    _ => None,
                }
            }
            _ => None,
        })];
        if self.moving() {
            parts.push(window::frames().map(Message::Tick));
        }
        Subscription::batch(parts)
    }

    pub fn entries(&self) -> &[Entry] {
        self.library.as_ref().map_or(&[], |l| &l.entries)
    }

    pub fn visible(&self) -> Vec<usize> {
        self.entries()
            .iter()
            .enumerate()
            .filter(|(_, e)| e.matches(&self.search))
            .map(|(i, _)| i)
            .collect()
    }

    pub fn chosen_entry(&self) -> Option<&Entry> {
        self.chosen.and_then(|i| self.entries().get(i))
    }

    fn shown(&self, entry: &Entry) -> Shown {
        let w = &self.words;
        let meta = {
            let mut parts: Vec<String> = Vec::new();
            parts.push(format!("{}x", entry.combo));
            if let Some(ms) = self.lengths.get(&entry.path) {
                parts.push(w.length(*ms));
            }
            parts.join(" · ")
        };
        Shown {
            date: format!("{} · {} · {}", w.day(entry.played_at, self.now_unix), w.clock(entry.played_at), entry.client.tag()),
            player: entry.player.clone(),
            map: entry.map_line().unwrap_or_else(|| w.t("unknown-map")),
            meta,
            accuracy: w.percent(entry.accuracy),
            outcome: entry.outcome.mark(),
        }
    }

    fn choose(&mut self, at: usize) -> Task<Message> {
        if self.chosen == Some(at) {
            return Task::none();
        }
        let before = self.chosen_entry().map(|e| self.shown(e));
        let scene_before = self.chosen_entry().map(|e| self.scenes.get(&e.map_hash).cloned());
        self.before = before;
        self.scene_before = scene_before;
        self.chosen = Some(at);
        if self.rendering.as_ref().is_some_and(Rendering::is_over) {
            self.rendering = None;
        }
        if self.fetching.as_ref().is_some_and(Fetching::is_over) {
            self.fetching = None;
        }
        self.swap = Animation::new(false).duration(SWAP).easing(Easing::EaseOutCubic).go(true, Instant::now());
        self.fetch_for_chosen()
    }

    fn fetch_for_chosen(&self) -> Task<Message> {
        let Some(entry) = self.chosen_entry() else {
            return Task::none();
        };
        let mut tasks = Vec::new();
        if !self.scenes.contains_key(&entry.map_hash) {
            let hash = entry.map_hash.clone();
            let background = entry.map.as_ref().and_then(|m| m.background.clone());
            tasks.push(ui::in_thread(move || Message::Scene(hash, background.as_deref().and_then(|p| decoded(p, SCENE_WIDTH, None)))));
        }
        if !self.lengths.contains_key(&entry.path) {
            let path = entry.path.clone();
            tasks.push(ui::in_thread(move || Message::Length(path.clone(), length_of(&path))));
        }
        Task::batch(tasks)
    }

    fn thumbs_task(&self) -> Task<Message> {
        let wanted: Vec<(String, PathBuf)> = {
            let mut seen = std::collections::HashSet::new();
            self.entries()
                .iter()
                .filter_map(|e| {
                    let bg = e.map.as_ref()?.background.clone()?;
                    seen.insert(e.map_hash.clone()).then(|| (e.map_hash.clone(), bg))
                })
                .collect()
        };
        ui::streamed(move |push| {
            for (hash, path) in wanted {
                if let Some(handle) = decoded(&path, THUMB.0, Some(THUMB)) {
                    if !push(Message::Thumb(hash, handle)) {
                        return;
                    }
                }
            }
        })
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loaded(library) => {
                self.library = Some(library);
                self.now_unix = unix_now();
                self.enter = Animation::new(false).duration(ENTER).easing(Easing::EaseOutCubic).go(true, Instant::now());
                let first = self.visible().first().copied();
                self.chosen = first;
                Task::batch([self.fetch_for_chosen(), self.thumbs_task()])
            }
            Message::Thumb(hash, handle) => {
                self.thumbs.insert(hash, handle);
                Task::none()
            }
            Message::Scene(hash, handle) => {
                if let Some(handle) = handle {
                    if self.scenes.len() > 24 {
                        self.scenes.clear();
                    }
                    self.scenes.insert(hash, handle);
                }
                Task::none()
            }
            Message::Length(path, ms) => {
                self.lengths.insert(path, ms);
                Task::none()
            }
            Message::Choose(at) => self.choose(at),
            Message::Step(by) => {
                let visible = self.visible();
                if visible.is_empty() {
                    return Task::none();
                }
                let at = match self.chosen.and_then(|c| visible.iter().position(|v| *v == c)) {
                    Some(at) => (at as i32 + by).clamp(0, visible.len() as i32 - 1) as usize,
                    None => 0,
                };
                self.choose(visible[at])
            }
            Message::Escape => {
                self.overlay = Overlay::None;
                Task::none()
            }
            Message::Hover(at) => {
                self.hover = at;
                match at {
                    Some(at) => {
                        self.lifted = Some(at);
                        self.lift.go_mut(true, Instant::now());
                    }
                    None => self.lift.go_mut(false, Instant::now()),
                }
                Task::none()
            }
            Message::Show(overlay) => {
                self.overlay = overlay;
                Task::none()
            }
            Message::OpenFolder => {
                if let Some(entry) = self.chosen_entry() {
                    let _ = open::that_detached(entry.path.parent().unwrap_or(Path::new(".")));
                }
                Task::none()
            }
            Message::Render => {
                if self.rendering.as_ref().is_some_and(|r| !r.is_over()) {
                    return Task::none();
                }
                let (Some(entry), Some(ffmpeg)) = (self.chosen_entry(), self.ffmpeg.clone()) else {
                    return Task::none();
                };
                let Some(map) = &entry.map else {
                    return Task::none();
                };
                let out = render::renders_dir().join(render::file_name(&entry.player, &map.line()));
                let ask = render::Ask {
                    replay: entry.path.clone(),
                    map: map.file.clone(),
                    map_hash: entry.map_hash.clone(),
                    ffmpeg,
                    out,
                };
                self.rendering = Some(Rendering { path: entry.path.clone(), reached: Vec::new(), out: None });
                render::run(ask).map(Message::Rendered)
            }
            Message::StopRender => {
                render::stop();
                Task::none()
            }
            Message::Rendered(step) => {
                if let Some(rendering) = &mut self.rendering {
                    if let Step::Saved(path) = &step {
                        rendering.out = Some(path.clone());
                    }
                    rendering.reached.push(step);
                }
                Task::none()
            }
            Message::GetMap => {
                if self.fetching.as_ref().is_some_and(|f| !f.is_over()) {
                    return Task::none();
                }
                let Some(entry) = self.chosen_entry() else {
                    return Task::none();
                };
                let hash = entry.map_hash.clone();
                let live: Vec<&crate::sources::Source> = self.settings.sources.iter().filter(|s| s.on).collect();
                let songs = live
                    .iter()
                    .find(|s| s.kind == Kind::Own)
                    .or_else(|| live.iter().find(|s| s.kind == Kind::Folder))
                    .and_then(|s| s.songs.clone())
                    .unwrap_or_else(|| crate::sources::own_root().join("Songs"));
                self.fetching = Some(Fetching { hash: hash.clone(), reached: Vec::new() });
                maps::fetch(hash, songs).map(Message::Fetched)
            }
            Message::StopFetch => {
                maps::stop();
                Task::none()
            }
            Message::Fetched(step) => {
                let Some(fetching) = &mut self.fetching else {
                    return Task::none();
                };
                let hash = fetching.hash.clone();
                if let maps::Step::Done(map) = &step {
                    let map = map.clone();
                    self.fetching = None;
                    if let Some(library) = &mut self.library {
                        for entry in library.entries.iter_mut().filter(|e| e.map_hash == hash) {
                            entry.map = Some(map.clone());
                        }
                        library.maps += 1;
                    }
                    if !self.settings.sources.iter().any(|s| s.kind == Kind::Own) {
                        if let Ok(own) = crate::sources::own() {
                            self.settings.sources.push(own);
                            let _ = self.settings.save();
                        }
                    }
                    if self.chosen_entry().is_some_and(|e| e.map_hash == hash) {
                        self.scene_before = Some(None);
                        self.swap = Animation::new(false).duration(SWAP).easing(Easing::EaseOutCubic).go(true, Instant::now());
                    }
                    let background = map.background.clone();
                    let for_thumb = background.clone();
                    let hash_for_thumb = hash.clone();
                    return Task::batch([
                        ui::in_thread(move || Message::Scene(hash, background.as_deref().and_then(|p| decoded(p, SCENE_WIDTH, None)))),
                        ui::in_thread(move || match for_thumb.as_deref().and_then(|p| decoded(p, THUMB.0, Some(THUMB))) {
                            Some(handle) => Message::Thumb(hash_for_thumb, handle),
                            None => Message::Hover(None),
                        }),
                    ]);
                }
                fetching.reached.push(step);
                Task::none()
            }
            Message::Look => {
                if matches!(self.looking, Some(scan::Step::Looking { .. })) {
                    return Task::none();
                }
                self.looking = Some(scan::Step::Looking { files: 0, found: 0, seconds: 0 });
                let known: Vec<PathBuf> = self
                    .settings
                    .sources
                    .iter()
                    .filter(|s| s.kind != Kind::Found)
                    .map(|s| s.root.clone())
                    .collect();
                scan::run(known).map(Message::Looked)
            }
            Message::StopLook => {
                scan::stop();
                Task::none()
            }
            Message::Looked(step) => {
                if let scan::Step::Done(paths) = &step {
                    let _ = scan::remember(paths);
                    if !paths.is_empty() && !self.settings.sources.iter().any(|s| s.kind == Kind::Found) {
                        self.settings.sources.push(scan::source(paths));
                    }
                    if let Some(found) = self.settings.sources.iter_mut().find(|s| s.kind == Kind::Found) {
                        found.replay_count = paths.len() as u64;
                    }
                    let _ = self.settings.save();
                    self.looking = Some(step);
                    let sources = self.settings.sources.clone();
                    return ui::in_thread(move || library::read(&sources)).map(Message::Loaded);
                }
                self.looking = Some(step);
                Task::none()
            }
            Message::OpenOut => {
                if let Some(out) = self.rendering.as_ref().and_then(|r| r.out.clone()) {
                    let _ = open::that_detached(out);
                }
                Task::none()
            }
            Message::ShowOut => {
                if let Some(out) = self.rendering.as_ref().and_then(|r| r.out.clone()) {
                    let _ = open::that_detached(out.parent().unwrap_or(Path::new(".")));
                }
                Task::none()
            }
            Message::Dropped(path) => {
                if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("osr")) {
                    return Task::none();
                }
                let sources = self.settings.sources.clone();
                ui::in_thread(move || {
                    let into = crate::sources::own_root().join("Replays");
                    let _ = std::fs::create_dir_all(&into);
                    if let Some(name) = path.file_name() {
                        let _ = std::fs::copy(&path, into.join(name));
                    }
                    let mut sources = sources;
                    if !sources.iter().any(|s| s.kind == Kind::Own) {
                        if let Ok(own) = crate::sources::own() {
                            sources.push(own);
                        }
                    }
                    library::read(&sources)
                })
                .map(Message::Loaded)
            }
            Message::Resized(width) => {
                self.width = width;
                Task::none()
            }
            Message::Tick(now) => {
                self.now = now;
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let k = self.enter.interpolate(0.0, 1.0, self.now);
        let s = self.swap.interpolate(0.0, 1.0, self.now);
        let loaded = self.library.is_some();
        let mut layers = stack![];
        if let Some(entry) = self.chosen_entry() {
            let alpha = k.min(1.0);
            let scene: Element<'_, Message> = match self.scenes.get(&entry.map_hash) {
                Some(handle) => image(handle.clone())
                    .content_fit(ContentFit::Cover)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .opacity(alpha * s)
                    .into(),
                None => container(ui::hatch()).width(Length::Fill).height(Length::Fill).into(),
            };
            if let Some(Some(before)) = &self.scene_before {
                if s < 1.0 {
                    layers = layers.push(
                        image(before.clone())
                            .content_fit(ContentFit::Cover)
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .opacity(alpha * (1.0 - s)),
                    );
                }
            }
            layers = layers.push(scene);
        }
        if loaded {
            layers = layers.push(ui::fading(k, || self.body(k, s)));
        }
        let early = self.arrive.interpolate(0.0, 1.0, self.now);
        let crest = ui::fading(early, ui::brand);
        layers = layers.push(pin(float(crest).translate(move |_, _| Vector::new(0.0, (1.0 - early) * CREST_RISE))).x(CREST_HOME.0).y(CREST_HOME.1));
        if self.overlay != Overlay::None {
            layers = layers.push(self.overlay_view());
        }
        layers.width(Length::Fill).height(Length::Fill).into()
    }

    fn body(&self, k: f32, s: f32) -> Element<'_, Message> {
        let late = ((k - 0.7) / 0.3).clamp(0.0, 1.0);
        column![
            ui::fading(ui::fade() * late, || self.chrome()),
            Space::new().height(Length::Fill),
            self.viewer(s),
            self.journal(),
            Space::new().height((1.0 - k) * JOURNAL_RISE),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn chrome(&self) -> Element<'_, Message> {
        let w = &self.words;
        let word = |key: &str, on: bool, msg: Message| {
            button(text(w.t(key)).font(theme::SANS_SEMI).size(theme::BODY))
                .padding([6, 0])
                .style(theme::word(on))
                .on_press(msg)
        };
        let words = row![
            word("replays", self.overlay == Overlay::None, Message::Show(Overlay::None)),
            word("worker", self.overlay == Overlay::Worker, Message::Show(Overlay::Worker)),
            word("settings", self.overlay == Overlay::Settings, Message::Show(Overlay::Settings)),
        ]
        .spacing(22);
        container(
            row![Space::new().width(BRAND_WIDTH), ui::grow(), words]
                .align_y(iced::Center)
                .height(theme::CONTROL_HEIGHT + 4.0),
        )
        .padding(Padding { top: 22.0, right: 40.0, bottom: 0.0, left: 40.0 })
        .width(Length::Fill)
        .into()
    }

    fn viewer(&self, s: f32) -> Element<'_, Message> {
        let w = &self.words;
        let Some(entry) = self.chosen_entry() else {
            let lower: Element<'_, Message> = match &self.looking {
                Some(step) => self.look_ledger(step),
                None => container(ui::primary(w.t("look-on-device"), Some(Message::Look))).padding(Padding::ZERO.top(14.0)).into(),
            };
            let empty = column![
                text(w.t("no-replays")).font(theme::SANS_SEMI).size(theme::TITLE).color(ui::faded(INK)),
                ui::cap(w.t("drop-here")),
                lower,
            ]
            .spacing(6);
            return container(empty).padding(Padding { top: 0.0, right: 40.0, bottom: 34.0, left: 40.0 }).width(Length::Fill).into();
        };
        let now = self.shown(entry);
        let was = self.before.clone().unwrap_or_default();
        let retype = |from: &str, to: &str| if s < 1.0 { typed(from, to, s) } else { to.to_owned() };
        let date = retype(&was.date, &now.date);
        let player = retype(&was.player, &now.player);
        let map = retype(&was.map, &now.map);
        let accuracy = retype(&was.accuracy, &now.accuracy);
        let mut meta = row![].spacing(8).align_y(iced::Center);
        for acronym in &entry.mods {
            meta = meta.push(mod_badge(acronym));
        }
        if !entry.mods.is_empty() {
            meta = meta.push(ui::mono("·".to_owned(), FAINT));
        }
        meta = meta.push(ui::mono(format!("{}x", entry.combo), MUTED));
        if let Some(ms) = self.lengths.get(&entry.path) {
            meta = meta.push(ui::mono("·".to_owned(), FAINT));
            meta = meta.push(ui::mono(w.length(*ms), MUTED));
        }
        let busy = self.rendering.as_ref().is_some_and(|r| !r.is_over());
        let fetching_now = self.fetching.as_ref().is_some_and(|f| !f.is_over());
        let rendering_this = self.rendering.as_ref().filter(|r| r.path == entry.path);
        let fetching_this = self.fetching.as_ref().filter(|f| f.hash == entry.map_hash);
        let lower: Element<'_, Message> = match (rendering_this, fetching_this) {
            (Some(rendering), _) => self.render_ledger(rendering),
            (None, Some(fetching)) => self.fetch_ledger(fetching),
            (None, None) => {
                let action = if entry.map.is_some() {
                    ui::primary(w.t("render"), (self.ffmpeg.is_some() && !busy).then_some(Message::Render))
                } else {
                    ui::primary(w.t("get-the-map"), (!fetching_now).then_some(Message::GetMap))
                };
                column![container(meta).padding(Padding::ZERO.top(4.0)), container(action).padding(Padding::ZERO.top(14.0))]
                    .spacing(2)
                    .into()
            }
        };
        let left = column![
            text(date).font(theme::MONO).size(theme::CAPTION).color(ui::faded(MUTED)),
            text(player).font(theme::SANS_SEMI).size(30.0).color(ui::faded(INK)),
            text(map).font(theme::SANS).size(theme::BODY).color(ui::faded(MUTED)),
            lower,
        ]
        .spacing(2);
        let outcome_colour = if entry.outcome.is_bad() { ACCENT } else { MUTED };
        let right = column![
            text(accuracy).font(theme::MONO_BOLD).size(48.0).color(ui::faded(INK)),
            container(text(now.outcome).font(theme::MONO_BOLD).size(theme::CAPTION).color(ui::faded(outcome_colour)))
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(2)
        .align_x(iced::alignment::Horizontal::Right);
        container(row![left, ui::grow(), right].align_y(iced::alignment::Vertical::Bottom))
            .padding(Padding { top: 0.0, right: 40.0, bottom: 28.0, left: 40.0 })
            .width(Length::Fill)
            .into()
    }

    fn render_ledger(&self, rendering: &Rendering) -> Element<'_, Message> {
        let w = &self.words;
        let reached = |wanted: fn(&Step) -> bool| rendering.reached.iter().any(wanted);
        let last = rendering.last();
        if let Some(Step::Saved(_)) = last {
            let line = row![
                ui::mono(w.t("rendered"), MUTED),
                ui::mono("·".to_owned(), FAINT),
                ui::link(w.t("open"), Message::OpenOut),
                ui::mono("·".to_owned(), FAINT),
                ui::link(w.t("show-in-folder"), Message::ShowOut),
            ]
            .spacing(8)
            .align_y(iced::Center);
            return container(line).padding(Padding::ZERO.top(8.0)).into();
        }
        let failed = match last {
            Some(Step::Failed(why)) => Some(why.clone()),
            Some(Step::Stopped) => Some(w.t("stopped")),
            _ => None,
        };
        let stages: [(&str, fn(&Step) -> bool); 5] = [
            ("replay-read", |s| matches!(s, Step::ReplayRead)),
            ("map-on-disk", |s| matches!(s, Step::MapOnDisk)),
            ("judged", |s| matches!(s, Step::Judged)),
            ("drawing", |s| matches!(s, Step::Encoded)),
            ("saving", |s| matches!(s, Step::Saved(_))),
        ];
        let mut lines = Vec::new();
        let mut current_seen = false;
        for (key, done) in stages {
            let is_done = reached(done);
            let mood = if is_done {
                Mood::Done
            } else if !current_seen {
                current_seen = true;
                if failed.is_some() { Mood::Failed } else { Mood::Now }
            } else {
                Mood::Todo
            };
            let mut line = Line::new(mood, w.t(key));
            if key == "drawing" && mood == Mood::Now {
                if let Some(Step::Drawing { frames, of, left_seconds }) = rendering.reached.iter().rev().find(|s| matches!(s, Step::Drawing { .. })) {
                    line = line.detail(format!(
                        "{} / {} · {}",
                        w.lang().group(*frames),
                        w.lang().group(*of),
                        w.n("seconds-left", left_seconds.round().max(0.0) as u64)
                    ));
                }
            }
            if mood == Mood::Failed {
                if let Some(why) = &failed {
                    line = line.detail(why.chars().take(72).collect::<String>());
                }
            }
            if mood == Mood::Now {
                line = line.breathing(self.breath());
            }
            lines.push(line);
        }
        let ledger = ui::ledger(&lines, None);
        let foot: Element<'_, Message> = if failed.is_some() {
            ui::quiet(w.t("render"), (self.ffmpeg.is_some()).then_some(Message::Render))
        } else {
            ui::quiet(w.t("stop"), Some(Message::StopRender))
        };
        column![container(ledger).padding(Padding::ZERO.top(8.0)), container(foot).padding(Padding::ZERO.top(6.0))]
            .into()
    }

    fn fetch_ledger(&self, fetching: &Fetching) -> Element<'_, Message> {
        use maps::Step as S;
        let w = &self.words;
        let last = fetching.last();
        let failed = match last {
            Some(S::Failed(why)) => Some(why.clone()),
            Some(S::Nowhere) => Some(w.t("not-on-any-mirror")),
            Some(S::Stopped) => Some(w.t("stopped")),
            _ => None,
        };
        let found = fetching.reached.iter().find_map(|s| match s {
            S::Found(found) => Some(found.clone()),
            _ => None,
        });
        let downloading = fetching.reached.iter().rev().find_map(|s| match s {
            S::Downloading { from, done, total } => Some((*from, *done, *total)),
            _ => None,
        });
        let stage = |reached: fn(&S) -> bool| fetching.reached.iter().any(reached);
        let stages: [(&str, fn(&S) -> bool); 4] = [
            ("looking-up", |s| matches!(s, S::Found(_))),
            ("fetch-downloading", |s| matches!(s, S::Unpacking)),
            ("unpacking-into", |s| matches!(s, S::Checking)),
            ("checking-hash", |s| matches!(s, S::Done(_))),
        ];
        let mut lines = Vec::new();
        let mut current_seen = false;
        for (key, done) in stages {
            let is_done = stage(done);
            let mood = if is_done {
                Mood::Done
            } else if !current_seen {
                current_seen = true;
                if failed.is_some() { Mood::Failed } else { Mood::Now }
            } else {
                Mood::Todo
            };
            let name = match (key, &found) {
                ("looking-up", Some(found)) => w.who("found-on", found.from),
                _ => w.t(key),
            };
            let mut line = Line::new(mood, name);
            match key {
                "looking-up" => {
                    if let Some(found) = &found {
                        line = line.detail(found.line());
                    }
                }
                "fetch-downloading" => {
                    if let Some((from, done, total)) = downloading {
                        let size = match total {
                            Some(total) => w.mb_of(done, total),
                            None => w.mb(done),
                        };
                        line = line.detail(format!("{size} · {from}"));
                    }
                }
                _ => {}
            }
            if mood == Mood::Failed {
                if let Some(why) = &failed {
                    line = line.detail(why.chars().take(72).collect::<String>());
                }
            }
            if mood == Mood::Now {
                line = line.breathing(self.breath());
            }
            lines.push(line);
        }
        let ledger = ui::ledger(&lines, None);
        let foot: Element<'_, Message> = if failed.is_some() {
            ui::quiet(w.t("try-again"), Some(Message::GetMap))
        } else {
            ui::quiet(w.t("stop"), Some(Message::StopFetch))
        };
        column![container(ledger).padding(Padding::ZERO.top(8.0)), container(foot).padding(Padding::ZERO.top(6.0))]
            .into()
    }

    fn look_ledger(&self, step: &scan::Step) -> Element<'_, Message> {
        let w = &self.words;
        let line = match step {
            scan::Step::Looking { files, found, seconds } => Line::new(Mood::Now, w.t("looking-on-device"))
                .detail(format!(
                    "{} · {} · {}",
                    w.count("files-label", *files),
                    w.count("replays-label", *found),
                    w.n("seconds-left", *seconds)
                ))
                .breathing(self.breath()),
            scan::Step::Done(paths) if paths.is_empty() => Line::new(Mood::Failed, w.t("nothing-on-device")),
            scan::Step::Done(paths) => Line::new(Mood::Done, w.t("found-on-device")).detail(w.count("replays-label", paths.len() as u64)),
            scan::Step::Stopped => Line::new(Mood::Failed, w.t("looking-on-device")).detail(w.t("stopped")),
        };
        let foot: Element<'_, Message> = match step {
            scan::Step::Looking { .. } => ui::quiet(w.t("stop"), Some(Message::StopLook)),
            _ => ui::quiet(w.t("try-again"), Some(Message::Look)),
        };
        column![container(ui::ledger(&[line], None)).padding(Padding::ZERO.top(8.0)), container(foot).padding(Padding::ZERO.top(6.0))]
            .into()
    }

    fn breath(&self) -> f32 {
        let period = 1.6;
        let t = self.now.duration_since(self.started).as_secs_f32();
        (0.5 - 0.5 * (t / period * std::f32::consts::TAU).cos()).clamp(0.0, 1.0)
    }

    fn journal(&self) -> Element<'_, Message> {
        let w = &self.words;
        let visible = self.visible();
        let entries = self.entries();
        let mut days: Vec<(String, Vec<usize>)> = Vec::new();
        for at in &visible {
            let label = w.day(entries[*at].played_at, self.now_unix);
            match days.last_mut() {
                Some((day, list)) if *day == label => list.push(*at),
                _ => days.push((label, vec![*at])),
            }
        }
        let mut strip = row![].spacing(22).align_y(iced::alignment::Vertical::Bottom);
        for (label, list) in &days {
            let on_day = list.iter().any(|i| Some(*i) == self.chosen);
            let dot = container(Space::new().width(5.0).height(5.0)).style(move |_| container::Style {
                background: Some(iced::Background::Color(ui::faded(if on_day { ACCENT } else { FAINT }))),
                border: iced::Border { radius: 3.0.into(), ..iced::Border::default() },
                ..container::Style::default()
            });
            let head = row![dot, text(label.clone()).font(theme::MONO).size(11.0).color(ui::faded(if on_day { INK } else { FAINT }))]
                .spacing(6)
                .align_y(iced::Center);
            let mut frames = row![].spacing(6).align_y(iced::alignment::Vertical::Bottom);
            for at in list {
                frames = frames.push(self.frame(*at, &entries[*at]));
            }
            strip = strip.push(column![head, frames].spacing(6));
        }
        let strip = scrollable(strip)
            .id(self.strip_id.clone())
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new().width(0).scroller_width(0).margin(0),
            ))
            .width(Length::Fill);
        let position = self
            .chosen
            .and_then(|c| visible.iter().position(|v| *v == c))
            .map_or(0, |p| p + 1);
        let counter = ui::mono(format!("{} / {}", position, visible.len()), FAINT);
        let rail = container(Space::new().height(1.0)).width(Length::Fill).style(theme::rule_high);
        container(column![rail, container(strip).padding(Padding::ZERO.top(10.0)), container(counter).width(Length::Fill).center_x(Length::Fill)].spacing(4))
            .padding(Padding { top: 0.0, right: 40.0, bottom: 10.0, left: 40.0 })
            .width(Length::Fill)
            .into()
    }

    fn frame(&self, at: usize, entry: &Entry) -> Element<'_, Message> {
        let chosen = self.chosen == Some(at);
        let lit = self.hover == Some(at);
        let rise = if self.lifted == Some(at) { self.lift.interpolate(0.0, 1.0, self.now) } else { 0.0 };
        let (w, h) = if chosen { (theme::FRAME_W + 8.0, theme::FRAME_H + 4.0) } else { (theme::FRAME_W, theme::FRAME_H) };
        let inner = if chosen { 4.0 } else { 2.0 };
        let picture: Element<'_, Message> = match (self.thumbs.get(&entry.map_hash), entry.map.is_some()) {
            (Some(handle), _) => image(handle.clone())
                .content_fit(ContentFit::Cover)
                .width(w - inner)
                .height(h - inner)
                .border_radius(theme::FRAME_RADIUS - 2.0)
                .opacity(ui::fade() * if chosen { 1.0 } else { 0.55 + 0.45 * rise })
                .into(),
            (None, true) => Space::new().width(w - inner).height(h - inner).into(),
            (None, false) => container(ui::fine_hatch()).width(w - inner).height(h - inner).into(),
        };
        let busy_here = self.rendering.as_ref().is_some_and(|r| r.path == entry.path && !r.is_over())
            || self.fetching.as_ref().is_some_and(|f| f.hash == entry.map_hash && !f.is_over());
        let picture: Element<'_, Message> = if busy_here {
            let dot = container(Space::new().width(8.0).height(8.0)).style(|_| container::Style {
                background: Some(iced::Background::Color(ACCENT)),
                border: iced::Border { radius: 4.0.into(), ..iced::Border::default() },
                shadow: iced::Shadow { color: Color::from_rgba(0.027, 0.012, 0.016, 0.6), offset: Vector::ZERO, blur_radius: 10.0 },
                ..container::Style::default()
            });
            stack![picture, container(dot).width(Length::Fill).height(Length::Fill).center(Length::Fill)].into()
        } else {
            picture
        };
        let edge = if chosen { 2.0 } else { 1.0 };
        let pressed = button(container(picture).width(w - 2.0 * edge).height(h - 2.0 * edge))
            .padding(edge)
            .style(theme::frame(chosen, lit))
            .on_press(Message::Choose(at));
        let sensed = mouse_area(pressed).on_enter(Message::Hover(Some(at))).on_exit(Message::Hover(None));
        let caption = self.caption(entry);
        let tipped = tooltip(sensed, caption, tooltip::Position::Top).gap(10).padding(0);
        let scale = if chosen { 1.0 } else { 1.0 + 0.04 * rise };
        float(tipped).scale(scale).translate(move |_, _| Vector::new(0.0, -2.0 * rise)).into()
    }

    fn caption(&self, entry: &Entry) -> Element<'_, Message> {
        let w = &self.words;
        let mut how = row![].spacing(6).align_y(iced::Center);
        for acronym in &entry.mods {
            how = how.push(mod_badge(acronym));
        }
        if !entry.mods.is_empty() {
            how = how.push(ui::mono("·".to_owned(), FAINT));
        }
        how = how.push(ui::mono(format!("{}x", entry.combo), MUTED));
        how = how.push(ui::mono("·".to_owned(), FAINT));
        how = how.push(text(entry.outcome.mark()).font(theme::MONO_BOLD).size(11.0).color(ui::faded(if entry.outcome.is_bad() { ACCENT } else { INK })));
        how = how.push(ui::mono("·".to_owned(), FAINT));
        how = how.push(text(entry.grade.letter()).font(theme::MONO_BOLD).size(11.0).color(grade_colour(entry.grade)));
        let inside = column![
            row![
                text(entry.player.clone()).font(theme::SANS_SEMI).size(theme::BODY).color(ui::faded(INK)),
                ui::grow(),
                text(w.percent(entry.accuracy)).font(theme::MONO_BOLD).size(13.0).color(ui::faded(INK)),
            ]
            .spacing(18)
            .align_y(iced::Center),
            text(entry.map_line().unwrap_or_else(|| w.t("unknown-map"))).font(theme::SANS).size(theme::CAPTION).color(ui::faded(MUTED)),
            container(how).padding(Padding::ZERO.top(4.0)),
        ]
        .spacing(1)
        .width(Length::Shrink);
        container(inside).padding([10, 12]).style(|_| container::Style {
            background: Some(iced::Background::Color(Color::from_rgba(0.047, 0.02, 0.027, 0.96))),
            border: iced::Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.1), width: 1.0, radius: 10.0.into() },
            ..container::Style::default()
        })
        .into()
    }

    fn overlay_view(&self) -> Element<'_, Message> {
        let w = &self.words;
        let name = match self.overlay {
            Overlay::Worker => w.t("worker"),
            _ => w.t("settings"),
        };
        let card = ui::card(
            column![ui::title(name), ui::cap(w.t("coming-later"))].spacing(6).into(),
            Some(row![ui::grow(), ui::quiet(w.t("back-to-replays"), Some(Message::Show(Overlay::None)))].into()),
        );
        stack![
            ui::veil(theme::SCRIM),
            container(container(card).width(theme::COLUMN).padding(Padding::ZERO.top(150.0)))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

pub fn grade_colour(grade: Grade) -> Color {
    match grade {
        Grade::Ss => theme::GRADE_SS,
        Grade::S => theme::GRADE_S,
        Grade::A => theme::GRADE_A,
        Grade::B => theme::GRADE_B,
        Grade::C => theme::GRADE_C,
        Grade::D | Grade::F => theme::GRADE_D,
    }
}

pub fn mod_colour(acronym: &str) -> Color {
    match acronym {
        "HR" | "DT" | "NC" | "HD" | "FL" | "SD" | "PF" | "FI" | "AC" | "DC" => theme::MOD_HARD,
        "EZ" | "NF" | "HT" | "DC_" => theme::MOD_EASY,
        "RX" | "AP" | "AT" | "SO" | "CN" => theme::MOD_AUTO,
        _ => theme::MOD_OTHER,
    }
}

fn mod_badge<'a, Message: 'a>(acronym: &str) -> Element<'a, Message> {
    container(text(acronym.to_owned()).font(theme::MONO_BOLD).size(10.0))
        .padding([1, 6])
        .style(theme::badge(mod_colour(acronym)))
        .into()
}

pub fn decoded(path: &Path, max_width: u32, cover: Option<(u32, u32)>) -> Option<image::Handle> {
    let picture = ::image::open(path).ok()?;
    let picture = match cover {
        Some((w, h)) => {
            let scale = (w as f64 / picture.width() as f64).max(h as f64 / picture.height() as f64);
            let (sw, sh) = ((picture.width() as f64 * scale).ceil() as u32, (picture.height() as f64 * scale).ceil() as u32);
            let resized = picture.resize_exact(sw.max(w), sh.max(h), ::image::imageops::FilterType::Triangle);
            resized.crop_imm((resized.width() - w) / 2, (resized.height() - h) / 2, w, h)
        }
        None => {
            let scaled = if picture.width() > max_width {
                let h = (picture.height() as f64 * max_width as f64 / picture.width() as f64).round() as u32;
                picture.resize_exact(max_width, h.max(1), ::image::imageops::FilterType::Triangle)
            } else {
                picture
            };
            scaled.fast_blur(3.0)
        }
    };
    let mut rgba = picture.to_rgba8();
    if cover.is_none() {
        dim_into_ground(&mut rgba);
    }
    Some(image::Handle::from_rgba(rgba.width(), rgba.height(), rgba.into_raw()))
}

const DIM: [(f32, f32); 5] = [(0.0, 0.98), (0.3, 0.66), (0.5, 0.66), (0.72, 0.96), (1.0, 0.99)];

pub fn dim_at(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    for pair in DIM.windows(2) {
        let ((t0, a0), (t1, a1)) = (pair[0], pair[1]);
        if t <= t1 {
            let k = if t1 > t0 { (t - t0) / (t1 - t0) } else { 1.0 };
            let soft = k * k * (3.0 - 2.0 * k);
            return a0 + (a1 - a0) * soft;
        }
    }
    DIM[DIM.len() - 1].1
}

fn dim_into_ground(rgba: &mut ::image::RgbaImage) {
    let ground = [theme::GROUND.r * 255.0, theme::GROUND.g * 255.0, theme::GROUND.b * 255.0];
    let height = rgba.height().max(1);
    for (y, row) in rgba.rows_mut().enumerate() {
        let a = dim_at(y as f32 / (height - 1).max(1) as f32);
        for pixel in row {
            for c in 0..3 {
                pixel[c] = (pixel[c] as f32 * (1.0 - a) + ground[c] * a).round().clamp(0.0, 255.0) as u8;
            }
            pixel[3] = 255;
        }
    }
}

pub fn length_of(path: &Path) -> i64 {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| dossier_replay::Replay::parse(&bytes).ok())
        .map_or(0, |r| r.duration_ms())
}

impl std::fmt::Debug for Main {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Main").field("chosen", &self.chosen).field("search", &self.search).finish()
    }
}
