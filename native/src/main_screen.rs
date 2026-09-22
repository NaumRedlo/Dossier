use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use iced::animation::Easing;
use iced::widget::{
    button, column, container, float, image, mouse_area, pin, row, scrollable, stack, text, Space,
};
use iced::{window, Animation, Color, ContentFit, Element, Length, Padding, Point, Subscription, Task, Vector};

use crate::lang::{typed, Words};
use crate::library::{self, Entry, Grade, Library};
use crate::bot::{self, Paired, Refused};
use crate::{notices, player, videos};
use crate::live;
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
pub const LIVE_FADE: Duration = Duration::from_millis(640);
pub const BRAND_WIDTH: f32 = 144.0;
const CREST_HOME: (f32, f32) = (40.0, 24.0);
const CREST_RISE: f32 = 8.0;
const JOURNAL_RISE: f32 = 140.0;
const BUBBLE_W: f32 = 300.0;
const BUBBLE_H: f32 = 88.0;
const CARET: f32 = 8.0;
const THUMB: (u32, u32) = (176, 100);
const SCENE_WIDTH: u32 = 960;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    None,
    Videos,
    Community,
    Worker,
    Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Library),
    Thumb(String, image::Handle),
    Scene(String, Option<image::Handle>),
    Length(PathBuf, i64),
    MaxCombo(PathBuf, Option<u32>),
    Key(iced::keyboard::key::Named),
    Adopted(Vec<videos::Video>),
    ToastLink(u64),
    ShowError(u64),
    DismissNotice(u64),
    HideError,
    Circle,
    MenuTab(Tab),
    MenuClose,
    SeenAll,
    SignIn,
    PairAsked(Result<(String, String), Refused>),
    Poll,
    Polled(Result<Paired, Refused>),
    OpenTelegram,
    CopyLink,
    LaterSignIn,
    SignOut,
    Known(Result<bot::Me, Refused>),
    Avatar(Option<image::Handle>),
    SendVideo,
    Sending(u64),
    Sent(Result<i64, String>),
    ToastHover(u64, bool),
    ToastClose(u64),
    OpenVideo(usize),
    ClosePlayer,
    PlayerToggle,
    SeekTo(f32),
    SeekBy(i64),
    RevealVideo,
    AskDelete,
    KeepVideo,
    DeleteVideo,
    Choose(usize),
    Step(i32),
    Escape,
    Hover(Option<usize>),
    Over(usize, iced::Rectangle),
    HoverStaged(usize),
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
    Live(live::Frame),
    TogglePlay,
    Strip(scrollable::Viewport),
    ScrubTo(f32),
    Traced(PathBuf, std::sync::Arc<Vec<(f64, f32, f32)>>),
    Dropped(PathBuf),
    Resized(f32, f32),
    Tick(Instant),
}

#[derive(Debug, Clone, Default)]
pub struct Shown {
    pub date: String,
    pub player: String,
    pub map: String,
    pub mods: Vec<String>,
    pub meta: String,
    pub accuracy: String,
    pub outcome: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Account,
    Feed,
    Stats,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pairing {
    Idle,
    Asking,
    Waiting { code: String, link: String },
    Unavailable,
}

#[derive(Debug, Clone)]
pub struct Sending {
    pub path: PathBuf,
    pub done: u64,
    pub total: u64,
    pub over: Option<Result<i64, String>>,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub id: u64,
    pub shown: Animation<bool>,
    pub born: Instant,
    pub hovered: bool,
    pub stays: bool,
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

#[derive(Debug, Clone)]
pub struct Trail {
    pub for_path: PathBuf,
    pub points: std::sync::Arc<Vec<(f64, f32, f32)>>,
    pub from_ms: f64,
    pub to_ms: f64,
    pub started: Instant,
    pub paused_at: Option<f64>,
}

impl Trail {
    pub fn at_ms(&self, now: Instant) -> f64 {
        if let Some(at) = self.paused_at {
            return at;
        }
        let span = (self.to_ms - self.from_ms).max(1.0);
        let elapsed = now.saturating_duration_since(self.started).as_secs_f64() * 1000.0;
        self.from_ms + elapsed % span
    }
}

#[derive(Debug, Clone)]
pub struct Live {
    pub control: std::sync::Arc<live::Control>,
    pub for_path: PathBuf,
    pub frame: Option<image::Handle>,
    pub still: Option<image::Handle>,
    pub at_ms: f64,
    pub fade: Animation<bool>,
    pub rest: Animation<bool>,
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
    pub hover_bounds: Option<iced::Rectangle>,
    pub thumbs: HashMap<String, image::Handle>,
    pub scenes: HashMap<String, image::Handle>,
    pub scene_before: Option<Option<image::Handle>>,
    pub lengths: HashMap<PathBuf, i64>,
    pub combos: HashMap<PathBuf, Option<u32>>,
    pub store: videos::Store,
    pub player: Option<std::rc::Rc<std::cell::RefCell<player::Player>>>,
    pub open_video: Option<usize>,
    pub asking_delete: bool,
    pub notices: notices::Queue,
    pub toasts: Vec<Toast>,
    pub account: Option<bot::Me>,
    pub avatar: Option<image::Handle>,
    pub menu: Option<Tab>,
    pub error_shown: Option<u64>,
    pub arrivals: HashMap<u64, Animation<bool>>,
    pub scenes_due: bool,
    pub leaving: HashMap<u64, Animation<bool>>,
    pub overlay_fade: Animation<bool>,
    pub overlay_drawn: Overlay,
    pub menu_open: Animation<bool>,
    pub tab_fade: Animation<bool>,
    pub seg_from: Tab,
    pub seg_slide: Animation<bool>,
    pub pairing: Pairing,
    pub qr: Option<ui::Qr>,
    pub sending: Option<Sending>,
    pub overlay: Overlay,
    pub rendering: Option<Rendering>,
    pub fetching: Option<Fetching>,
    pub looking: Option<scan::Step>,
    pub live: Option<Live>,
    pub live_before: Option<image::Handle>,
    pub hatch: image::Handle,
    pub trail: Option<Trail>,
    pub strip_view: Option<(f32, f32, f32)>,
    pub progress_shown: f32,
    pub ffmpeg: Option<PathBuf>,
    pub now: Instant,
    pub started: Instant,
    pub now_unix: i64,
    pub enter: Animation<bool>,
    pub arrive: Animation<bool>,
    pub swap: Animation<bool>,
    pub swap_waits: bool,
    pub lifts: HashMap<usize, Animation<bool>>,
    pub width: f32,
    pub height: f32,
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
            hover_bounds: None,
            thumbs: HashMap::new(),
            scenes: HashMap::new(),
            scene_before: None,
            lengths: HashMap::new(),
            combos: HashMap::new(),
            store: videos::Store::load(),
            player: None,
            open_video: None,
            asking_delete: false,
            notices: notices::Queue::load(),
            toasts: Vec::new(),
            account: None,
            avatar: None,
            menu: None,
            error_shown: None,
            arrivals: HashMap::new(),
            scenes_due: false,
            leaving: HashMap::new(),
            overlay_fade: Animation::new(false).duration(OVERLAY_FADE).easing(Easing::EaseOutCubic),
            overlay_drawn: Overlay::None,
            menu_open: Animation::new(false).duration(MENU_OPEN).easing(Easing::EaseOutCubic),
            tab_fade: Animation::new(true).duration(TAB_FADE).easing(Easing::EaseOutCubic),
            seg_from: Tab::Account,
            seg_slide: Animation::new(true).duration(TAB_FADE).easing(Easing::EaseOutCubic),
            pairing: Pairing::Idle,
            qr: None,
            sending: None,
            overlay: Overlay::None,
            rendering: None,
            fetching: None,
            looking: None,
            live: None,
            live_before: None,
            hatch: ui::hatched_picture(live::SIZE.0, live::SIZE.1, live::dim_at),
            trail: None,
            strip_view: None,
            progress_shown: 0.0,
            ffmpeg: crate::checks::ffmpeg_on_path(),
            now: Instant::now(),
            started: Instant::now(),
            now_unix: unix_now(),
            enter: Animation::new(false).duration(ENTER).easing(Easing::EaseOutCubic),
            arrive: Animation::new(false).duration(ARRIVE).easing(Easing::EaseOutCubic).go(true, Instant::now()),
            swap: Animation::new(true).duration(SWAP).easing(Easing::EaseOutCubic),
            swap_waits: false,
            lifts: HashMap::new(),
            width: crate::WINDOW.width,
            height: crate::WINDOW.height,
            strip_id: iced::widget::Id::unique(),
        };
        let strays = videos::strays(&made.store.videos);
        let adopt = match (made.ffmpeg.clone(), strays.is_empty()) {
            (Some(ffmpeg), false) => ui::in_thread(move || Message::Adopted(strays.iter().filter_map(|p| videos::adopt(&ffmpeg, p)).collect())),
            _ => Task::none(),
        };
        let who = made.ask_who();
        (made, Task::batch([ui::in_thread(move || library::read(&sources)).map(Message::Loaded), adopt, who]))
    }

    fn ask_who(&self) -> Task<Message> {
        if self.settings.token.is_empty() {
            return Task::none();
        }
        let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        ui::in_thread(move || Message::Known(bot::me(&server, &token, &name)))
    }

    pub fn signed_in(&self) -> bool {
        !self.settings.token.is_empty()
    }

    pub fn busy(&self) -> bool {
        self.rendering.as_ref().is_some_and(|r| !r.is_over())
            || self.fetching.as_ref().is_some_and(|f| !f.is_over())
            || matches!(self.looking, Some(scan::Step::Looking { .. }))
            || self.sending.as_ref().is_some_and(|s| s.over.is_none())
    }

    pub fn staged(words: Words, settings: Settings, library: Library, chosen: Option<usize>) -> Main {
        let (mut made, _) = Main::new(words, settings);
        made.library = Some(library);
        made.chosen = chosen;
        made.store = videos::Store::default();
        made.notices = notices::Queue::default();
        made.enter = Animation::new(true);
        made.arrive = Animation::new(true);
        made
    }

    pub fn moving(&self) -> bool {
        self.rendering.as_ref().is_some_and(|r| !r.is_over())
            || self.fetching.as_ref().is_some_and(|f| !f.is_over())
            || matches!(self.looking, Some(scan::Step::Looking { .. }))
            || self.live.as_ref().is_some_and(|l| l.fade.is_animating(self.now) || !(l.control.paused() && l.control.settled()))
            || self.trail.as_ref().is_some_and(|t| t.paused_at.is_none())
            || self.progress_target().is_some_and(|t| (t - self.progress_shown).abs() > 0.001)
            || self.enter.is_animating(self.now)
            || self.arrive.is_animating(self.now)
            || self.swap.is_animating(self.now)
            || self.lifts.values().any(|l| l.is_animating(self.now))
            || self.player.as_ref().is_some_and(|p| !p.borrow().paused)
            || !self.toasts.is_empty()
            || self.menu_open.is_animating(self.now)
            || self.overlay_fade.is_animating(self.now)
            || self.arrivals.values().any(|a| a.is_animating(self.now))
            || self.leaving.values().any(|a| a.is_animating(self.now))
            || self.tab_fade.is_animating(self.now)
            || self.seg_slide.is_animating(self.now)
            || self.sending.as_ref().is_some_and(|s| s.over.is_none())
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut parts = vec![iced::event::listen_with(|event, status, _| match (event, status) {
            (iced::Event::Window(window::Event::FileDropped(path)), _) => Some(Message::Dropped(path)),
            (iced::Event::Window(window::Event::Resized(size)), _) => Some(Message::Resized(size.width, size.height)),
            (iced::Event::Window(window::Event::Opened { size, .. }), _) => Some(Message::Resized(size.width, size.height)),
            (iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, .. }), iced::event::Status::Ignored) => {
                use iced::keyboard::key::{Key, Named};
                match key.as_ref() {
                    Key::Named(Named::ArrowLeft) => Some(Message::Key(Named::ArrowLeft)),
                    Key::Named(Named::ArrowUp) => Some(Message::Key(Named::ArrowUp)),
                    Key::Named(Named::ArrowRight) => Some(Message::Key(Named::ArrowRight)),
                    Key::Named(Named::ArrowDown) => Some(Message::Key(Named::ArrowDown)),
                    Key::Named(Named::Space) => Some(Message::Key(Named::Space)),
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
        if matches!(self.pairing, Pairing::Waiting { .. }) {
            parts.push(iced::time::every(Duration::from_secs(3)).map(|_| Message::Poll));
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
            mods: entry.mods.clone(),
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
        self.swap_waits = self.chosen_entry().is_some_and(|e| e.map.is_some() && !self.scenes.contains_key(&e.map_hash));
        if self.rendering.as_ref().is_some_and(Rendering::is_over) {
            self.rendering = None;
        }
        if self.fetching.as_ref().is_some_and(Fetching::is_over) {
            self.fetching = None;
        }
        if !self.swap_waits {
            self.swap = Animation::new(false).duration(SWAP).easing(Easing::EaseOutCubic).go(true, Instant::now());
        }
        Task::batch([self.fetch_for_chosen(), self.start_live()])
    }

    fn start_live(&mut self) -> Task<Message> {
        if let Some(live) = self.live.take() {
            live.control.stop();
            self.live_before = live.frame;
        }
        self.trail = None;
        let Some(ask) = self.chosen_entry().and_then(|entry| {
            let map = entry.map.as_ref()?;
            Some(live::Ask { replay: entry.path.clone(), map: map.file.clone(), map_hash: entry.map_hash.clone() })
        }) else {
            if let Some(entry) = self.chosen_entry() {
                let path = entry.path.clone();
                return ui::in_thread(move || Message::Traced(path.clone(), std::sync::Arc::new(live::trace(&path))));
            }
            return Task::none();
        };
        let control = std::sync::Arc::new(live::Control::default());
        self.live = Some(Live {
            control: control.clone(),
            for_path: ask.replay.clone(),
            frame: None,
            still: None,
            at_ms: 0.0,
            fade: Animation::new(false).duration(LIVE_FADE).easing(Easing::EaseOutCubic),
            rest: Animation::new(false).duration(LIVE_FADE).easing(Easing::EaseOutCubic),
        });
        live::play(ask, control).map(Message::Live)
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
                let entries = self.entries().to_vec();
                self.store.marry(&entries);
                self.enter = Animation::new(false).duration(ENTER).easing(Easing::EaseOutCubic).go(true, Instant::now());
                let first = self.visible().first().copied();
                self.chosen = first;
                Task::batch([self.fetch_for_chosen(), self.thumbs_task(), self.start_live()])
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
                    self.scenes.insert(hash.clone(), handle);
                }
                if self.swap_waits && self.chosen_entry().is_some_and(|e| e.map_hash == hash) {
                    self.swap_waits = false;
                    self.swap = Animation::new(false).duration(SWAP).easing(Easing::EaseOutCubic).go(true, Instant::now());
                }
                Task::none()
            }
            Message::Length(path, ms) => {
                self.lengths.insert(path, ms);
                Task::none()
            }
            Message::MaxCombo(path, combo) => {
                self.combos.insert(path, combo);
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
            Message::Key(key) => {
                use iced::keyboard::key::Named;
                let watching = self.player.is_some();
                match (key, watching) {
                    (Named::ArrowLeft, true) => self.update(Message::SeekBy(-5000)),
                    (Named::ArrowRight, true) => self.update(Message::SeekBy(5000)),
                    (Named::Space, true) => self.update(Message::PlayerToggle),
                    (Named::ArrowLeft | Named::ArrowUp, false) => self.update(Message::Step(-1)),
                    (Named::ArrowRight | Named::ArrowDown, false) => self.update(Message::Step(1)),
                    _ => Task::none(),
                }
            }
            Message::Escape => {
                if self.error_shown.is_some() {
                    self.error_shown = None;
                } else if matches!(self.pairing, Pairing::Asking | Pairing::Waiting { .. } | Pairing::Unavailable) {
                    self.pairing = Pairing::Idle;
                } else if self.menu.is_some() {
                    return self.update(Message::MenuClose);
                } else if self.asking_delete {
                    self.asking_delete = false;
                } else if self.player.is_some() {
                    return self.update(Message::ClosePlayer);
                } else {
                    self.overlay = Overlay::None;
                }
                Task::none()
            }
            Message::Circle => {
                match self.menu {
                    Some(_) => self.update(Message::MenuClose),
                    None => {
                        let last = match self.settings.menu_tab.as_str() {
                            "feed" => Tab::Feed,
                            "stats" => Tab::Stats,
                            _ => Tab::Account,
                        };
                        self.update(Message::MenuTab(last))
                    }
                }
            }
            Message::MenuTab(tab) => {
                let now = Instant::now();
                match self.menu {
                    Some(was) if was != tab => {
                        self.seg_from = was;
                        self.seg_slide = Animation::new(false).duration(TAB_FADE).easing(Easing::EaseOutCubic).go(true, now);
                        self.tab_fade = Animation::new(false).duration(TAB_FADE).easing(Easing::EaseOutCubic).go(true, now);
                    }
                    Some(_) => {}
                    None => {
                        self.seg_from = tab;
                        self.seg_slide = Animation::new(true);
                        self.tab_fade = Animation::new(true);
                        self.menu_open = Animation::new(false).duration(MENU_OPEN).easing(Easing::EaseOutCubic).go(true, now);
                    }
                }
                self.menu = Some(tab);
                let name = match tab {
                    Tab::Account => "account",
                    Tab::Feed => "feed",
                    Tab::Stats => "stats",
                };
                if self.settings.menu_tab != name {
                    self.settings.menu_tab = name.to_owned();
                    let _ = self.settings.save();
                }
                if tab == Tab::Feed {
                    self.notices.see_all();
                    return self.scenes_for_notices();
                }
                Task::none()
            }
            Message::MenuClose => {
                if self.menu_open.value() {
                    self.menu_open = Animation::new(true).duration(MENU_CLOSE).easing(Easing::EaseInCubic).go(false, Instant::now());
                }
                Task::none()
            }
            Message::SeenAll => {
                self.notices.see_all();
                Task::none()
            }
            Message::SignIn => {
                self.menu = None;
                self.menu_open = Animation::new(false).duration(MENU_OPEN).easing(Easing::EaseOutCubic);
                if self.signed_in() || matches!(self.pairing, Pairing::Waiting { .. }) {
                    return Task::none();
                }
                self.pairing = Pairing::Asking;
                let server = self.settings.server.clone();
                let name = self.settings.device.clone();
                ui::in_thread(move || Message::PairAsked(bot::pair(&server, &name).map(|p| (p.code, p.link))))
            }
            Message::PairAsked(Ok((code, link))) => {
                let link = if link.is_empty() {
                    format!("https://t.me/OneNineEightFourGlobalBot?start=pair-{}", bot::tidy(&code))
                } else {
                    link
                };
                self.qr = crate::first_run::qr_for(&link);
                self.pairing = Pairing::Waiting { code: bot::pretty(&code), link };
                Task::none()
            }
            Message::PairAsked(Err(_)) => {
                self.pairing = Pairing::Unavailable;
                Task::none()
            }
            Message::Poll => match &self.pairing {
                Pairing::Waiting { code, .. } => {
                    let server = self.settings.server.clone();
                    let code = bot::tidy(code);
                    ui::in_thread(move || Message::Polled(bot::paired(&server, &code)))
                }
                _ => Task::none(),
            },
            Message::Polled(Ok(Paired::Linked { token, who })) => {
                self.settings.token = token;
                self.settings.linked_as = who;
                let _ = self.settings.save();
                self.pairing = Pairing::Idle;
                self.ask_who()
            }
            Message::Polled(Ok(Paired::Gone)) | Message::Polled(Err(Refused::NotThere)) => {
                self.pairing = Pairing::Idle;
                self.update(Message::SignIn)
            }
            Message::Polled(_) => Task::none(),
            Message::OpenTelegram => {
                if let Pairing::Waiting { link, .. } = &self.pairing {
                    let _ = open::that_detached(link);
                }
                Task::none()
            }
            Message::CopyLink => match &self.pairing {
                Pairing::Waiting { link, .. } => iced::clipboard::write(link.clone()),
                _ => Task::none(),
            },
            Message::LaterSignIn => {
                self.pairing = Pairing::Idle;
                Task::none()
            }
            Message::SignOut => {
                self.settings.token.clear();
                self.settings.linked_as.clear();
                let _ = self.settings.save();
                self.account = None;
                self.avatar = None;
                self.menu = None;
                self.menu_open = Animation::new(false).duration(MENU_OPEN).easing(Easing::EaseOutCubic);
                Task::none()
            }
            Message::Known(Ok(me)) => {
                let wants_avatar = me.avatar;
                let said = if me.username.is_empty() { me.name.clone() } else { format!("@{}", me.username) };
                if !said.is_empty() && self.settings.linked_as != said {
                    self.settings.linked_as = said;
                    let _ = self.settings.save();
                }
                self.account = Some(me);
                if !wants_avatar {
                    return Task::none();
                }
                let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
                ui::in_thread(move || {
                    let bytes = bot::avatar(&server, &token, &name).ok();
                    Message::Avatar(bytes.and_then(|b| decoded_bytes(&b, AVATAR_SIDE)))
                })
            }
            Message::Known(Err(_)) => Task::none(),
            Message::Avatar(handle) => {
                self.avatar = handle;
                Task::none()
            }
            Message::SendVideo => {
                if !self.signed_in() {
                    return self.update(Message::SignIn);
                }
                if self.sending.as_ref().is_some_and(|s| s.over.is_none()) {
                    return Task::none();
                }
                let Some(video) = self.open_video.and_then(|at| self.store.videos.get(at).cloned()) else {
                    return Task::none();
                };
                self.sending = Some(Sending { path: video.path.clone(), done: 0, total: video.size.max(1), over: None });
                let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
                let meta = serde_json::json!({
                    "caption": format!("{} — {}", video.player, video.map_line()),
                    "name": video.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                    "width": video.width,
                    "height": video.height,
                    "duration": (video.length_ms / 1000).max(0),
                });
                let path = video.path.clone();
                ui::streamed(move |push| {
                    let (tx, rx) = std::sync::mpsc::channel::<u64>();
                    let worker = std::thread::spawn(move || bot::send(&server, &token, &name, &path, &meta, move |done| {
                        let _ = tx.send(done);
                    }));
                    let mut last = Instant::now();
                    for done in rx {
                        if last.elapsed() > Duration::from_millis(80) {
                            last = Instant::now();
                            if !push(Message::Sending(done)) {
                                return;
                            }
                        }
                    }
                    let outcome = match worker.join() {
                        Ok(Ok(id)) => Ok(id),
                        Ok(Err(e)) => Err(e.to_string()),
                        Err(_) => Err("sending stopped".to_owned()),
                    };
                    push(Message::Sent(outcome));
                })
            }
            Message::Sending(done) => {
                if let Some(sending) = &mut self.sending {
                    sending.done = done;
                }
                Task::none()
            }
            Message::Sent(outcome) => {
                let Some(sending) = &mut self.sending else {
                    return Task::none();
                };
                let path = sending.path.clone();
                sending.over = Some(outcome.clone());
                let video = self.store.videos.iter().find(|v| v.path == path).cloned();
                let who = self.account.as_ref().map(|a| format!("@{}", a.username)).filter(|u| u.len() > 1).unwrap_or_else(|| self.settings.linked_as.clone());
                match (outcome, video) {
                    (Ok(_), Some(video)) => {
                        self.store.mark_sent(&path, unix_now());
                        let detail = format!("{} — {}", video.player, video.map_line());
                        let note = format!("{} · {}", who, self.words.mb(video.size));
                        self.announce(notices::Mark::Done, self.words.t("sent-notice"), detail, note, video.map_hash.clone(), notices::Link::None);
                    }
                    (Err(why), video) => {
                        let (detail, hash) = video.map(|v| (format!("{} — {}", v.player, v.map_line()), v.map_hash)).unwrap_or_default();
                        self.announce(notices::Mark::Bad, self.words.t("send-failed"), detail, why, hash, notices::Link::None);
                    }
                    _ => {}
                }
                Task::none()
            }
            Message::Adopted(found) => {
                for video in found {
                    self.store.add(video);
                }
                let entries = self.entries().to_vec();
                self.store.marry(&entries);
                Task::none()
            }
            Message::OpenVideo(at) => {
                let Some(video) = self.store.videos.get(at) else {
                    return Task::none();
                };
                let Some(ffmpeg) = &self.ffmpeg else {
                    return Task::none();
                };
                if let Some(old) = self.player.take() {
                    old.borrow_mut().close();
                }
                self.player = Some(std::rc::Rc::new(std::cell::RefCell::new(player::Player::open(ffmpeg, &video.path, video.length_ms, video.fps))));
                self.open_video = Some(at);
                self.asking_delete = false;
                Task::none()
            }
            Message::ClosePlayer => {
                if let Some(player) = self.player.take() {
                    player.borrow_mut().close();
                }
                self.open_video = None;
                self.asking_delete = false;
                Task::none()
            }
            Message::PlayerToggle => {
                if let Some(player) = &self.player {
                    player.borrow_mut().toggle();
                }
                Task::none()
            }
            Message::SeekTo(fraction) => {
                if let Some(player) = &self.player {
                    let mut player = player.borrow_mut();
                    let to = (fraction as f64 * player.length_ms as f64) as i64;
                    player.seek(to);
                }
                Task::none()
            }
            Message::SeekBy(delta) => {
                if let Some(player) = &self.player {
                    player.borrow_mut().seek_by(delta);
                }
                Task::none()
            }
            Message::RevealVideo => {
                if let Some(video) = self.open_video.and_then(|at| self.store.videos.get(at)) {
                    let _ = open::that_detached(video.path.parent().unwrap_or(Path::new(".")));
                }
                Task::none()
            }
            Message::AskDelete => {
                self.asking_delete = true;
                if let Some(player) = &self.player {
                    let mut player = player.borrow_mut();
                    if !player.paused {
                        player.toggle();
                    }
                }
                Task::none()
            }
            Message::KeepVideo => {
                self.asking_delete = false;
                Task::none()
            }
            Message::DeleteVideo => {
                self.asking_delete = false;
                let Some(video) = self.open_video.and_then(|at| self.store.videos.get(at).cloned()) else {
                    return Task::none();
                };
                if let Some(player) = self.player.take() {
                    player.borrow_mut().close();
                }
                self.open_video = None;
                if videos::to_bin(&video.path).is_ok() || !video.path.exists() {
                    self.store.forget(&video.path);
                }
                Task::none()
            }
            Message::Over(at, bounds) => {
                self.hover_bounds = Some(bounds);
                self.update(Message::Hover(Some(at)))
            }
            Message::HoverStaged(at) => {
                let x = 40.0 + theme::FRAME_W + 8.0 + 22.0 + at.saturating_sub(1) as f32 * (theme::FRAME_W + 6.0);
                let bounds = iced::Rectangle::new(Point::new(x, self.height - 10.0 - theme::FRAME_H), iced::Size::new(theme::FRAME_W, theme::FRAME_H));
                self.update(Message::Over(at, bounds))
            }
            Message::Hover(at) => {
                self.hover = at;
                if at.is_none() {
                    self.hover_bounds = None;
                }
                let combo = match at.and_then(|at| self.entries().get(at)) {
                    Some(entry) if !self.combos.contains_key(&entry.path) => {
                        let (path, map, hash) = (entry.path.clone(), entry.map.clone(), entry.map_hash.clone());
                        self.combos.insert(path.clone(), None);
                        ui::in_thread(move || Message::MaxCombo(path.clone(), max_combo_of(&path, map.as_ref(), &hash)))
                    }
                    _ => Task::none(),
                };
                let now = Instant::now();
                for (index, lift) in self.lifts.iter_mut() {
                    if Some(*index) != at {
                        lift.go_mut(false, now);
                    }
                }
                if let Some(at) = at {
                    self.lifts
                        .entry(at)
                        .or_insert_with(|| Animation::new(false).duration(LIFT).easing(Easing::EaseOutCubic))
                        .go_mut(true, now);
                }
                self.lifts.retain(|_, lift| lift.value() || lift.is_animating(now));
                combo
            }
            Message::Show(overlay) => {
                let now = Instant::now();
                if overlay != self.overlay {
                    if overlay == Overlay::None {
                        self.overlay_fade.go_mut(false, now);
                    } else {
                        self.overlay_drawn = overlay;
                        self.overlay_fade = Animation::new(false).duration(OVERLAY_FADE).easing(Easing::EaseOutCubic).go(true, now);
                    }
                }
                self.overlay = overlay;
                if overlay != Overlay::Videos {
                    if let Some(player) = self.player.take() {
                        player.borrow_mut().close();
                    }
                    self.open_video = None;
                    self.asking_delete = false;
                    return Task::none();
                }
                let wanted: Vec<(String, PathBuf)> = self
                    .store
                    .videos
                    .iter()
                    .filter(|v| !self.thumbs.contains_key(&v.map_hash))
                    .filter_map(|v| v.background.clone().map(|bg| (v.map_hash.clone(), bg)))
                    .collect();
                if wanted.is_empty() {
                    return Task::none();
                }
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
                let mut saved = None;
                if let Some(rendering) = &mut self.rendering {
                    if let Step::Saved(path) = &step {
                        rendering.out = Some(path.clone());
                        saved = Some((rendering.path.clone(), path.clone()));
                    }
                    rendering.reached.push(step);
                }
                if let Some((replay, out)) = saved {
                    if let Some(entry) = self.entries().iter().find(|e| e.path == replay).cloned() {
                        let length = self.lengths.get(&replay).copied().unwrap_or_else(|| length_of(&replay));
                        let video = videos::Video::from_render(&entry, out.clone(), length, render::SIZE.0, render::SIZE.1, render::FPS as u32);
                        let detail = format!("{} — {}", video.player, video.map_line());
                        let note = format!("{} · {}", self.words.length(video.length_ms), self.words.mb(video.size));
                        let hash = video.map_hash.clone();
                        self.store.add(video);
                        self.announce(notices::Mark::Done, self.words.t("rendered-notice"), detail, note, hash, notices::Link::OpenVideo(out));
                    }
                }
                if let Some(Step::Failed(why)) = self.rendering.as_ref().and_then(|r| r.last().cloned()) {
                    if let Some(replay) = self.rendering.as_ref().map(|r| r.path.clone()) {
                        let entry = self.entries().iter().find(|e| e.path == replay);
                        let who = entry.map(|e| format!("{} — {}", e.player, e.song().unwrap_or_default())).unwrap_or_default();
                        let hash = entry.map(|e| e.map_hash.clone()).unwrap_or_default();
                        self.announce(notices::Mark::Bad, self.words.t("render-failed"), who, why, hash, notices::Link::RenderAgain(replay));
                    }
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
                    self.announce(notices::Mark::Done, self.words.t("map-fetched"), format!("{} — {}", map.artist, map.title), format!("[{}]", map.version), hash.clone(), notices::Link::None);
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
                    let mut restart = Task::none();
                    if self.chosen_entry().is_some_and(|e| e.map_hash == hash) {
                        self.scene_before = Some(None);
                        self.swap = Animation::new(false).duration(SWAP).easing(Easing::EaseOutCubic).go(true, Instant::now());
                        restart = self.start_live();
                    }
                    let background = map.background.clone();
                    let for_thumb = background.clone();
                    let hash_for_thumb = hash.clone();
                    return Task::batch([
                        restart,
                        ui::in_thread(move || Message::Scene(hash, background.as_deref().and_then(|p| decoded(p, SCENE_WIDTH, None)))),
                        ui::in_thread(move || match for_thumb.as_deref().and_then(|p| decoded(p, THUMB.0, Some(THUMB))) {
                            Some(handle) => Message::Thumb(hash_for_thumb, handle),
                            None => Message::Hover(None),
                        }),
                    ]);
                }
                let failed = match &step {
                    maps::Step::Nowhere => Some(self.words.t("not-on-any-mirror")),
                    maps::Step::Failed(why) => Some(why.clone()),
                    _ => None,
                };
                fetching.reached.push(step);
                if let Some(why) = failed {
                    let who = self.entries().iter().find(|e| e.map_hash == hash).map(|e| e.song().unwrap_or_default()).unwrap_or_default();
                    self.announce(notices::Mark::Bad, self.words.t("map-not-fetched"), who, why, hash, notices::Link::None);
                }
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
            Message::Live(frame) => {
                let Some(live) = &mut self.live else {
                    return Task::none();
                };
                match frame {
                    live::Frame::Picture { handle, at_ms, .. } => {
                        if live.frame.is_none() {
                            live.fade.go_mut(true, Instant::now());
                        }
                        live.frame = Some(handle);
                        live.at_ms = at_ms;
                        if live.rest.value() || live.rest.is_animating(Instant::now()) {
                            live.rest.go_mut(false, Instant::now());
                        }
                    }
                    live::Frame::Still(_) => {}
                    live::Frame::Failed(_) => {
                        live.control.stop();
                        self.live = None;
                    }
                }
                Task::none()
            }
            Message::Strip(viewport) => {
                let offset = viewport.absolute_offset().x;
                if let (Some((before, _, _)), Some(bounds)) = (self.strip_view, self.hover_bounds.as_mut()) {
                    bounds.x -= offset - before;
                }
                self.strip_view = Some((offset, viewport.content_bounds().width, viewport.bounds().width));
                Task::none()
            }
            Message::ScrubTo(fraction) => {
                let (content, shown) = match self.strip_view {
                    Some((_, content, shown)) => (content, shown),
                    None => self.guessed_strip(),
                };
                let x = (fraction * content).clamp(0.0, (content - shown).max(0.0));
                iced::advanced::widget::operate(iced::advanced::widget::operation::scrollable::scroll_to(
                    self.strip_id.clone(),
                    iced::widget::scrollable::AbsoluteOffset { x: Some(x), y: None },
                ))
            }
            Message::TogglePlay => {
                if let Some(live) = &self.live {
                    live.control.pause(!live.control.paused());
                }
                if let Some(trail) = &mut self.trail {
                    match trail.paused_at {
                        Some(at) => {
                            let span = (trail.to_ms - trail.from_ms).max(1.0);
                            trail.started = Instant::now() - Duration::from_secs_f64(((at - trail.from_ms) % span) / 1000.0);
                            trail.paused_at = None;
                        }
                        None => trail.paused_at = Some(trail.at_ms(Instant::now())),
                    }
                }
                Task::none()
            }
            Message::Traced(path, points) => {
                if self.chosen_entry().is_some_and(|e| e.path == path && e.map.is_none()) && points.len() > 1 {
                    let from_ms = points.first().map_or(0.0, |p| p.0);
                    let to_ms = points.last().map_or(0.0, |p| p.0);
                    self.trail = Some(Trail { for_path: path, points, from_ms, to_ms, started: Instant::now(), paused_at: None });
                }
                Task::none()
            }
            Message::OpenOut => {
                let Some(out) = self.rendering.as_ref().and_then(|r| r.out.clone()) else {
                    return Task::none();
                };
                match self.store.videos.iter().position(|v| v.path == out) {
                    Some(at) => {
                        let shown = self.update(Message::Show(Overlay::Videos));
                        shown.chain(self.update(Message::OpenVideo(at)))
                    }
                    None => {
                        let _ = open::that_detached(out);
                        Task::none()
                    }
                }
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
            Message::Resized(width, height) => {
                self.width = width;
                self.height = height;
                Task::none()
            }
            Message::Tick(now) => {
                if self.scenes_due {
                    self.scenes_due = false;
                    let load = self.scenes_for_notices();
                    self.now = now;
                    return load.chain(self.update(Message::Tick(now)));
                }
                if let Some(player) = &self.player {
                    player.borrow_mut().pull();
                }
                self.now = now;
                if let Some(live) = &self.live {
                    if !(live.control.paused() && live.control.settled()) {
                        live.control.request();
                    }
                }
                match self.progress_target() {
                    Some(target) => self.progress_shown += (target - self.progress_shown) * 0.12,
                    None => self.progress_shown = 0.0,
                }
                for toast in &mut self.toasts {
                    if toast.shown.value() && !toast.stays && !toast.hovered && now.duration_since(toast.born) > TOAST_STAY {
                        toast.shown.go_mut(false, now);
                    }
                }
                self.toasts.retain(|t| t.shown.value() || t.shown.is_animating(now));
                if self.menu.is_some() && !self.menu_open.value() && !self.menu_open.is_animating(now) {
                    self.menu = None;
                }
                let gone: Vec<u64> = self.leaving.iter().filter(|(_, a)| !a.value() && !a.is_animating(now)).map(|(id, _)| *id).collect();
                for id in gone {
                    self.leaving.remove(&id);
                    self.arrivals.remove(&id);
                    self.toasts.retain(|t| t.id != id);
                    self.notices.remove(id);
                }
                self.arrivals.retain(|_, a| a.is_animating(now));
                Task::none()
            }
            Message::ToastHover(id, over) => {
                if let Some(toast) = self.toasts.iter_mut().find(|t| t.id == id) {
                    toast.hovered = over;
                    if !over {
                        toast.born = Instant::now();
                    }
                }
                Task::none()
            }
            Message::ToastClose(id) => {
                let now = Instant::now();
                if let Some(toast) = self.toasts.iter_mut().find(|t| t.id == id) {
                    toast.shown.go_mut(false, now);
                }
                Task::none()
            }
            Message::DismissNotice(id) => {
                let now = Instant::now();
                if self.error_shown == Some(id) {
                    self.error_shown = None;
                }
                for toast in self.toasts.iter_mut().filter(|t| t.id == id) {
                    toast.shown.go_mut(false, now);
                }
                self.leaving.entry(id).or_insert_with(|| Animation::new(true).duration(NOTICE_LEAVE).easing(Easing::EaseOutCubic)).go_mut(false, now);
                Task::none()
            }
            Message::ShowError(id) => {
                self.error_shown = Some(id);
                Task::none()
            }
            Message::HideError => {
                self.error_shown = None;
                Task::none()
            }
            Message::ToastLink(id) => {
                self.error_shown = None;
                let link = self.notices.get(id).map(|n| n.link.clone());
                let _ = self.update(Message::ToastClose(id));
                match link {
                    Some(notices::Link::OpenVideo(path)) => {
                        let at = self.store.videos.iter().position(|v| v.path == path);
                        let shown = self.update(Message::Show(Overlay::Videos));
                        match at {
                            Some(at) => shown.chain(self.update(Message::OpenVideo(at))),
                            None => shown,
                        }
                    }
                    Some(notices::Link::RenderAgain(replay)) => {
                        let at = self.entries().iter().position(|e| e.path == replay);
                        match at {
                            Some(at) => {
                                let chosen = self.choose(at);
                                chosen.chain(self.update(Message::Render))
                            }
                            None => Task::none(),
                        }
                    }
                    _ => Task::none(),
                }
            }
        }
    }

    pub fn announce(&mut self, mark: notices::Mark, words: String, detail: String, note: String, map_hash: String, link: notices::Link) {
        let id = self.notices.push(mark, words, detail, note, map_hash, link);
        self.scenes_due = true;
        let now = Instant::now();
        if self.menu == Some(Tab::Feed) {
            self.arrivals.insert(id, Animation::new(false).duration(NOTICE_ARRIVE).easing(Easing::EaseOutCubic).go(true, now));
        }
        while self.toasts.iter().filter(|t| t.shown.value()).count() >= TOASTS_AT_MOST {
            if let Some(oldest) = self.toasts.iter_mut().find(|t| t.shown.value()) {
                oldest.shown.go_mut(false, now);
            } else {
                break;
            }
        }
        self.toasts.push(Toast {
            id,
            shown: Animation::new(false).duration(TOAST_IN).easing(Easing::EaseOutCubic).go(true, now),
            born: now,
            hovered: false,
            stays: mark == notices::Mark::Bad,
        });
    }

    pub fn view(&self) -> Element<'_, Message> {
        let k = self.enter.interpolate(0.0, 1.0, self.now);
        let s = if self.swap_waits { 0.0 } else { self.swap.interpolate(0.0, 1.0, self.now) };
        let loaded = self.library.is_some();
        let blank = || -> Element<'_, Message> { Space::new().width(Length::Fill).height(Length::Fill).into() };
        let alpha = k.min(1.0);
        let full = |handle: &image::Handle, opacity: f32| -> Element<'_, Message> {
            image(handle.clone())
                .content_fit(ContentFit::Cover)
                .width(Length::Fill)
                .height(Length::Fill)
                .opacity(opacity)
                .into()
        };
        let entry = self.chosen_entry();
        let scene_before: Element<'_, Message> = match (&self.scene_before, entry) {
            (Some(Some(before)), Some(_)) if s < 1.0 => full(before, alpha * (1.0 - s)),
            _ => blank(),
        };
        let scene: Element<'_, Message> = match entry {
            Some(entry) => match (self.scenes.get(&entry.map_hash), entry.map.is_some()) {
                (Some(handle), _) => full(handle, alpha * s),
                (None, true) => blank(),
                (None, false) => full(&self.hatch, alpha),
            },
            None => blank(),
        };
        let live_before: Element<'_, Message> = match (&self.live_before, entry) {
            (Some(before), Some(_)) if s < 1.0 => full(before, alpha * (1.0 - s)),
            _ => blank(),
        };
        let live: Element<'_, Message> = match (entry, &self.live, &self.trail) {
            (Some(entry), Some(live), _) if live.for_path == entry.path => match &live.frame {
                Some(handle) => {
                    let seen = live.fade.interpolate(0.0, 1.0, self.now);
                    mouse_area(full(handle, alpha * seen)).on_press(Message::TogglePlay).into()
                }
                None => blank(),
            },
            (Some(entry), _, Some(trail)) if trail.for_path == entry.path => mouse_area(
                iced::widget::canvas(ui::Trail { points: trail.points.clone(), at_ms: trail.at_ms(self.now), window_ms: 3000.0 })
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .on_press(Message::TogglePlay)
            .into(),
            _ => blank(),
        };
        let body: Element<'_, Message> = if loaded { ui::fading(k, || self.body(k, s)) } else { blank() };
        let early = self.arrive.interpolate(0.0, 1.0, self.now);
        let crest = ui::fading(early, ui::brand);
        let crest: Element<'_, Message> =
            pin(float(crest).translate(move |_, _| Vector::new(0.0, (1.0 - early) * CREST_RISE))).x(CREST_HOME.0).y(CREST_HOME.1).into();
        let sheet = self.overlay_fade.interpolate(0.0, 1.0, self.now);
        let overlay: Element<'_, Message> = if self.overlay != Overlay::None || self.overlay_fade.is_animating(self.now) {
            ui::fading(sheet, || self.overlay_view())
        } else {
            blank()
        };
        let bubble = self.bubble_layer();
        let toasts = self.toast_layer();
        let menu = self.menu_layer();
        let signing = self.sign_in_layer();
        let failure = self.error_layer();
        let layers = stack![scene_before, scene, live_before, live, body, bubble, crest, overlay, menu, signing, failure, toasts];
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
            word("videos", self.overlay == Overlay::Videos, Message::Show(Overlay::Videos)),
            word("community", self.overlay == Overlay::Community, Message::Show(Overlay::Community)),
            word("worker", self.overlay == Overlay::Worker, Message::Show(Overlay::Worker)),
            word("settings", self.overlay == Overlay::Settings, Message::Show(Overlay::Settings)),
            self.circle(CIRCLE_SIDE, true),
        ]
        .spacing(22)
        .align_y(iced::Center);
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
        let body = self.block(entry, &was, &now, s, true);
        container(body)
            .padding(Padding { top: 0.0, right: 40.0, bottom: 28.0, left: 40.0 })
            .width(Length::Fill)
            .into()
    }

    fn block(&self, entry: &Entry, was: &Shown, now: &Shown, s: f32, with_actions: bool) -> Element<'_, Message> {
        let w = &self.words;
        let retype = |from: &str, to: &str| if s < 1.0 { typed(from, to, s) } else { to.to_owned() };
        let date = retype(&was.date, &now.date);
        let player = retype(&was.player, &now.player);
        let map = retype(&was.map, &now.map);
        let accuracy = retype(&was.accuracy, &now.accuracy);
        let erasing = crate::lang::ERASING;
        let (badges, badge_alpha) = if s < erasing {
            (&was.mods, 1.0 - s / erasing)
        } else {
            (&now.mods, ((s - erasing) / (1.0 - erasing)).clamp(0.0, 1.0))
        };
        let mut meta = row![].spacing(8).align_y(iced::Center);
        for acronym in badges {
            meta = meta.push(ui::fading(ui::fade() * badge_alpha, || mod_badge(acronym)));
        }
        if !badges.is_empty() {
            meta = meta.push(ui::fading(ui::fade() * badge_alpha, || ui::mono("·".to_owned(), FAINT)));
        }
        meta = meta.push(ui::mono(retype(&was.meta, &now.meta), MUTED));
        let action: Element<'_, Message> = if with_actions { self.action(entry) } else { Space::new().height(theme::CONTROL_HEIGHT).into() };
        let left = column![
            text(date).font(theme::MONO).size(theme::CAPTION).color(ui::faded(MUTED)),
            text(player).font(theme::SANS_SEMI).size(30.0).color(ui::faded(INK)),
            text(map).font(theme::SANS).size(theme::BODY).color(ui::faded(MUTED)),
            container(meta).padding(Padding::ZERO.top(4.0)),
            container(action).padding(Padding::ZERO.top(14.0)),
        ]
        .spacing(2);
        let outcome_colour = if entry.outcome.is_bad() { ACCENT } else { MUTED };
        let right = column![
            text(accuracy).font(theme::MONO_BOLD).size(48.0).color(ui::faded(INK)),
            container(text(retype(&was.outcome, &now.outcome)).font(theme::MONO_BOLD).size(theme::CAPTION).color(ui::faded(outcome_colour)))
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(2)
        .align_x(iced::alignment::Horizontal::Right);
        let _ = w;
        row![left, ui::grow(), right].align_y(iced::alignment::Vertical::Bottom).into()
    }

    fn action(&self, entry: &Entry) -> Element<'_, Message> {
        let w = &self.words;
        let busy = self.rendering.as_ref().is_some_and(|r| !r.is_over());
        let fetching_now = self.fetching.as_ref().is_some_and(|f| !f.is_over());
        let rendering_this = self.rendering.as_ref().filter(|r| r.path == entry.path);
        let fetching_this = self.fetching.as_ref().filter(|f| f.hash == entry.map_hash);
        match (rendering_this, fetching_this) {
            (Some(rendering), _) => self.render_button(rendering),
            (None, Some(fetching)) => self.fetch_button(fetching),
            (None, None) if entry.map.is_some() => ui::primary(w.t("render"), (self.ffmpeg.is_some() && !busy).then_some(Message::Render)),
            (None, None) => ui::primary(w.t("get-the-map"), (!fetching_now).then_some(Message::GetMap)),
        }
    }

    fn render_button(&self, rendering: &Rendering) -> Element<'_, Message> {
        let w = &self.words;
        match rendering.last() {
            Some(Step::Saved(_)) => ui::primary(w.t("open"), Some(Message::OpenOut)),
            Some(Step::Failed(_)) | Some(Step::Stopped) => ui::quiet(w.t("once-more"), (self.ffmpeg.is_some()).then_some(Message::Render)),
            step => {
                let label = match step {
                    None | Some(Step::ReplayRead) => w.t("reading"),
                    Some(Step::MapOnDisk) => w.t("judging"),
                    Some(Step::Judged) | Some(Step::Drawing { .. }) => w.t("drawing"),
                    Some(Step::Encoded) => w.t("saving"),
                    _ => w.t("drawing"),
                };
                ui::progress(label, self.progress_shown, Some(Message::StopRender))
            }
        }
    }

    fn fetch_button(&self, fetching: &Fetching) -> Element<'_, Message> {
        use maps::Step as S;
        let w = &self.words;
        match fetching.last() {
            Some(S::Nowhere) => ui::quiet(w.t("not-found"), Some(Message::GetMap)),
            Some(S::Failed(_)) | Some(S::Stopped) => ui::quiet(w.t("once-more"), Some(Message::GetMap)),
            step => {
                let label = match step {
                    None | Some(S::Looking) => w.t("looking"),
                    Some(S::Found(_)) => w.t("found"),
                    Some(S::Downloading { .. }) => w.t("fetch-downloading"),
                    Some(S::Unpacking) => w.t("unpacking-map"),
                    Some(S::Checking) => w.t("checking"),
                    _ => w.t("looking"),
                };
                ui::progress(label, self.progress_shown, Some(Message::StopFetch))
            }
        }
    }

    pub fn progress_target(&self) -> Option<f32> {
        if let Some(sending) = self.sending.as_ref().filter(|s| s.over.is_none()) {
            return Some(0.02 + 0.96 * (sending.done as f32 / sending.total.max(1) as f32).clamp(0.0, 1.0));
        }
        if let Some(rendering) = self.rendering.as_ref().filter(|r| !r.is_over()) {
            return Some(match rendering.last() {
                None | Some(Step::ReplayRead) => 0.03,
                Some(Step::MapOnDisk) => 0.06,
                Some(Step::Judged) => 0.08,
                Some(Step::Drawing { frames, of, .. }) => 0.08 + 0.88 * if *of > 0 { *frames as f32 / *of as f32 } else { 0.0 },
                Some(Step::Encoded) => 0.98,
                _ => 1.0,
            });
        }
        if let Some(fetching) = self.fetching.as_ref().filter(|f| !f.is_over()) {
            use maps::Step as S;
            return Some(match fetching.last() {
                None | Some(S::Looking) => 0.04,
                Some(S::Found(_)) => 0.1,
                Some(S::Downloading { done, total, .. }) => match total {
                    Some(total) if *total > 0 => 0.1 + 0.8 * (*done as f32 / *total as f32),
                    _ => 0.3,
                },
                Some(S::Unpacking) => 0.93,
                Some(S::Checking) => 0.97,
                _ => 1.0,
            });
        }
        None
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

    fn guessed_strip(&self) -> (f32, f32) {
        let visible = self.visible();
        let entries = self.entries();
        let mut days = 0usize;
        let mut last_day = String::new();
        for at in &visible {
            let label = self.words.day(entries[*at].played_at, self.now_unix);
            if label != last_day {
                days += 1;
                last_day = label;
            }
        }
        let content = visible.len() as f32 * (theme::FRAME_W + 6.0) - 6.0 * days as f32 + 8.0 + days.saturating_sub(1) as f32 * 22.0;
        let shown = (self.width - 80.0 - 80.0).max(1.0);
        (content.max(1.0), shown)
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
            .on_scroll(Message::Strip)
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new().width(0).scroller_width(0).margin(0),
            ))
            .width(Length::Fill);
        let position = self
            .chosen
            .and_then(|c| visible.iter().position(|v| *v == c))
            .map_or(0, |p| p + 1);
        let counter = text(format!("{} / {}", position, visible.len()))
            .font(theme::MONO)
            .size(11.0)
            .color(ui::faded(Color { a: 0.7, ..FAINT }));
        let (guessed_content, guessed_shown) = self.guessed_strip();
        let (start, len) = match self.strip_view {
            Some((offset, content, shown)) if content > 0.0 => (offset / content, (shown / content).min(1.0)),
            _ => (0.0, (guessed_shown / guessed_content.max(1.0)).min(1.0)),
        };
        let scrub = iced::widget::canvas(ui::Scrub { start, len, on: Box::new(Message::ScrubTo) })
            .width(Length::Fill)
            .height(14.0);
        let rail = row![scrub, container(counter).padding(Padding::ZERO.left(14.0))].align_y(iced::Center);
        container(column![rail, container(strip).padding(Padding::ZERO.top(8.0))].spacing(2))
            .padding(Padding { top: 0.0, right: 40.0, bottom: 10.0, left: 40.0 })
            .width(Length::Fill)
            .into()
    }

    fn frame(&self, at: usize, entry: &Entry) -> Element<'_, Message> {
        let chosen = self.chosen == Some(at);
        let lit = self.hover == Some(at);
        let rise = self.lifts.get(&at).map_or(0.0, |lift| lift.interpolate(0.0, 1.0, self.now));
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
        let edge = if chosen { 2.0 } else { 1.0 };
        let pressed = button(container(picture).width(w - 2.0 * edge).height(h - 2.0 * edge))
            .padding(edge)
            .style(theme::frame(chosen, lit))
            .on_press(Message::Choose(at));
        let scale = if chosen { 1.0 } else { 1.0 + 0.03 * rise };
        ui::sensed(pressed, move |bounds| Message::Over(at, bounds), Message::Hover(None))
            .risen(2.0 * rise, scale)
            .into()
    }

    fn bubble_layer(&self) -> Element<'_, Message> {
        let Some(at) = self.hover else {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        };
        let (Some(entry), Some(bounds)) = (self.entries().get(at), self.hover_bounds) else {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        };
        let rise = self.lifts.get(&at).map_or(0.0, |lift| lift.interpolate(0.0, 1.0, self.now));
        let frame_top = bounds.y - 2.0 * rise;
        let x = (bounds.center_x() - BUBBLE_W / 2.0).clamp(16.0, (self.width - BUBBLE_W - 16.0).max(16.0));
        let y = frame_top - 14.0 - BUBBLE_H - CARET;
        let tip = (bounds.center_x() - x).clamp(16.0, BUBBLE_W - 16.0);
        let anchor = Point::new(tip / BUBBLE_W, 1.0);
        let bubble = ui::fading(ui::fade() * rise, || self.bubble(entry, tip));
        pin(ui::grown(bubble, anchor, 0.0, 0.84 + 0.16 * rise)).x(x).y(y).into()
    }

    fn bubble(&self, entry: &Entry, tip: f32) -> Element<'_, Message> {
        let alpha = ui::fade();
        let w = &self.words;
        let small = |words: String, colour: Color| text(words).font(theme::MONO).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(colour));
        let bold = |words: String, colour: Color| text(words).font(theme::MONO_BOLD).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(colour));
        let dot = || small("·".to_owned(), FAINT);
        let [c300, c100, c50, miss] = entry.counts;
        let counts = row![
            small("300".to_owned(), theme::HIT_300),
            bold(c300.to_string(), INK),
            dot(),
            small("100".to_owned(), theme::HIT_100),
            bold(c100.to_string(), INK),
            dot(),
            small("50".to_owned(), theme::HIT_50),
            bold(c50.to_string(), INK),
            dot(),
            small("✕".to_owned(), ACCENT),
            bold(miss.to_string(), INK),
        ]
        .spacing(5)
        .align_y(iced::Center);
        let mut how = row![small(format!("{}x", entry.combo), MUTED)].spacing(5).align_y(iced::Center);
        if let Some(Some(max)) = self.combos.get(&entry.path) {
            how = how.push(small(w.of_max(*max), FAINT));
        }
        if entry.outcome != library::Outcome::Fail {
            how = how.push(dot());
            how = how.push(bold(entry.outcome.mark(), if entry.outcome.is_bad() { ACCENT } else { MUTED }));
        }
        how = how.push(dot());
        how = how.push(small(
            format!("{} · {} {}", entry.client.tag(), w.day(entry.played_at, self.now_unix), w.clock(entry.played_at)),
            FAINT,
        ));
        let head = row![
            text(ui::shortened(entry.player.clone(), 22)).font(theme::SANS_SEMI).size(theme::BODY).wrapping(text::Wrapping::None).color(ui::faded(INK)),
            ui::grow(),
            row![
                text(entry.grade.letter()).font(theme::MONO_BOLD).size(theme::CAPTION).color(ui::faded(grade_colour(entry.grade))),
                text("·").font(theme::MONO).size(theme::CAPTION).color(ui::faded(FAINT)),
                text(w.percent(entry.accuracy)).font(theme::MONO_BOLD).size(theme::CAPTION).color(ui::faded(INK)),
            ]
            .spacing(5)
            .align_y(iced::Center),
        ]
        .spacing(12)
        .align_y(iced::Center);
        let how = container(how).width(Length::Fill).clip(true);
        let inside = column![
            head,
            text(ui::shortened(entry.song().unwrap_or_else(|| w.t("unknown-map")), 44))
                .font(theme::SANS)
                .size(theme::CAPTION)
                .wrapping(text::Wrapping::None)
                .color(ui::faded(MUTED)),
            container(counts).padding(Padding::ZERO.top(3.0)),
            how,
        ]
        .spacing(2)
        .width(Length::Fill);
        let card = container(inside).padding([8, 10]).width(BUBBLE_W).height(BUBBLE_H).clip(true);
        let skin = iced::widget::canvas(ui::Skin { at: tip, alpha }).width(BUBBLE_W).height(BUBBLE_H + CARET);
        stack![skin, column![card, Space::new().height(CARET)]].width(BUBBLE_W).height(BUBBLE_H + CARET).into()
    }

    fn overlay_view(&self) -> Element<'_, Message> {
        let which = if self.overlay == Overlay::None { self.overlay_drawn } else { self.overlay };
        if which == Overlay::Videos {
            return self.videos_view();
        }
        let w = &self.words;
        let (name, why) = match which {
            Overlay::Worker => (w.t("worker"), w.t("coming-later")),
            Overlay::Community => (w.t("community"), w.t("community-why")),
            _ => (w.t("settings"), w.t("coming-later")),
        };
        let card = ui::card(
            column![ui::title(name), ui::cap(why)].spacing(6).into(),
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

impl Main {
    fn videos_view(&self) -> Element<'_, Message> {
        let w = &self.words;
        let page: Element<'_, Message> = if self.store.videos.is_empty() {
            let empty = column![
                text(w.t("no-videos")).font(theme::SANS_SEMI).size(theme::TITLE).color(ui::faded(INK)),
                ui::cap(w.t("render-one")),
                container(ui::primary(w.t("back-to-replays"), Some(Message::Show(Overlay::None)))).padding(Padding::ZERO.top(12.0)),
            ]
            .spacing(6)
            .align_x(iced::Center);
            container(empty).width(Length::Fill).height(Length::Fill).center(Length::Fill).into()
        } else {
            let mut rows = column![self.video_head()].spacing(0).width(Length::Fill);
            for (at, video) in self.store.videos.iter().enumerate() {
                rows = rows.push(self.video_row(at, video));
            }
            scrollable(container(rows).padding(Padding { top: 34.0, right: 28.0, bottom: 24.0, left: 28.0 }))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        };
        let sheet = column![self.chrome(), page].width(Length::Fill).height(Length::Fill);
        let stage: Element<'_, Message> = match (&self.player, self.open_video.and_then(|at| self.store.videos.get(at))) {
            (Some(player), Some(video)) => self.stage(&player.borrow(), video),
            _ => Space::new().width(Length::Fill).height(Length::Fill).into(),
        };
        let ask: Element<'_, Message> = if self.asking_delete {
            self.delete_card()
        } else {
            Space::new().width(Length::Fill).height(Length::Fill).into()
        };
        let crest: Element<'_, Message> = pin(ui::brand()).x(CREST_HOME.0).y(CREST_HOME.1).into();
        stack![ui::veil(theme::GROUND), sheet, crest, stage, ask].width(Length::Fill).height(Length::Fill).into()
    }

    fn video_head(&self) -> Element<'_, Message> {
        let w = &self.words;
        let cell = |key: &str, width: f32| container(ui::mono_small(w.t(key).to_uppercase(), FAINT)).width(width).align_x(iced::alignment::Horizontal::Center);
        let grow = |key: &str| container(ui::mono_small(w.t(key).to_uppercase(), FAINT)).width(Length::Fill);
        let right = |key: &str, width: f32| container(ui::mono_small(w.t(key).to_uppercase(), FAINT)).width(width).align_x(iced::alignment::Horizontal::Right);
        container(
            row![
                Space::new().width(VIDEO_THUMB.0 as f32 + 14.0 + 12.0),
                cell("when", 84.0),
                grow("who-and-map"),
                container(ui::mono_small(w.t("mods").to_uppercase(), FAINT)).width(110.0),
                right("length", 56.0),
                right("size", 84.0),
            ]
            .spacing(14)
            .align_y(iced::Center),
        )
        .padding(Padding::ZERO.right(12.0))
        .height(24.0)
        .into()
    }

    fn video_row(&self, at: usize, video: &videos::Video) -> Element<'_, Message> {
        let w = &self.words;
        let chosen = self.open_video == Some(at);
        let picture: Element<'_, Message> = match self.thumbs.get(&video.map_hash) {
            Some(handle) => image(handle.clone())
                .content_fit(ContentFit::Cover)
                .width(VIDEO_THUMB.0 as f32)
                .height(VIDEO_THUMB.1 as f32)
                .border_radius(6.0)
                .opacity(ui::fade() * if chosen { 1.0 } else { 0.85 })
                .into(),
            None => container(ui::fine_hatch()).width(VIDEO_THUMB.0 as f32).height(VIDEO_THUMB.1 as f32).into(),
        };
        let when = column![
            ui::mono(w.day(video.made_at, self.now_unix), FAINT),
            ui::mono(w.clock(video.made_at), FAINT),
        ]
        .spacing(2)
        .align_x(iced::Center);
        let who = column![
            text(video.player.clone()).font(theme::SANS_SEMI).size(theme::LEAD).wrapping(text::Wrapping::None).color(ui::faded(INK)),
            text(ui::shortened(video.map_line(), 70)).font(theme::SANS).size(theme::BODY).wrapping(text::Wrapping::None).color(ui::faded(MUTED)),
        ]
        .spacing(2);
        let mut mods = row![].spacing(4).align_y(iced::Center);
        for acronym in &video.mods {
            mods = mods.push(mod_badge(acronym));
        }
        let line = row![
            container(picture).width(VIDEO_THUMB.0 as f32).height(VIDEO_THUMB.1 as f32),
            container(when).width(84.0).align_x(iced::alignment::Horizontal::Center),
            container(who).width(Length::Fill).clip(true),
            container(mods).width(110.0),
            container(ui::mono(w.length(video.length_ms), MUTED)).width(56.0).align_x(iced::alignment::Horizontal::Right),
            container(ui::mono(w.mb(video.size), MUTED)).width(84.0).align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(14)
        .align_y(iced::Center);
        button(container(line).height(VIDEO_ROW).width(Length::Fill).center_y(VIDEO_ROW))
            .padding([0, 12])
            .style(ui::button_faded(theme::row(chosen)))
            .on_press(Message::OpenVideo(at))
            .into()
    }

    fn stage(&self, player: &player::Player, video: &videos::Video) -> Element<'_, Message> {
        let w = &self.words;
        let playing = !player.paused;
        let picture: Element<'_, Message> = match &player.frame {
            Some(handle) => image(handle.clone())
                .content_fit(ContentFit::Contain)
                .width(Length::Fill)
                .height(Length::Fill)
                .opacity(ui::fade())
                .border_radius(iced::border::Radius { top_left: theme::CARD_RADIUS - 1.0, top_right: theme::CARD_RADIUS - 1.0, bottom_right: 0.0, bottom_left: 0.0 })
                .into(),
            None => match self.thumbs.get(&video.map_hash) {
                Some(handle) => image(handle.clone())
                    .content_fit(ContentFit::Cover)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .opacity(0.35 * ui::fade())
                    .border_radius(iced::border::Radius { top_left: theme::CARD_RADIUS - 1.0, top_right: theme::CARD_RADIUS - 1.0, bottom_right: 0.0, bottom_left: 0.0 })
                    .into(),
                None => Space::new().width(Length::Fill).height(Length::Fill).into(),
            },
        };
        let mark: Element<'_, Message> = if player.paused {
            container(iced::widget::canvas(ui::PlayMark).width(44.0).height(44.0)).width(Length::Fill).height(Length::Fill).center(Length::Fill).into()
        } else {
            Space::new().width(Length::Fill).height(Length::Fill).into()
        };
        let screen = mouse_area(stack![picture, mark].width(Length::Fill).height(Length::Fill)).on_press(Message::PlayerToggle);
        let seek = iced::widget::canvas(ui::Seek { played: player.fraction(), on: Box::new(Message::SeekTo) }).width(Length::Fill).height(16.0);
        let timeline = row![
            ui::mono_small(w.length(player.at_ms()), INK),
            seek,
            ui::mono_small(w.length(player.length_ms), INK),
        ]
        .spacing(12)
        .align_y(iced::Center);
        let dim = if playing { 0.6 } else { 1.0 };
        let mut facts = row![].spacing(6).align_y(iced::Center);
        for acronym in &video.mods {
            facts = facts.push(mod_badge(acronym));
        }
        if !video.mods.is_empty() {
            facts = facts.push(ui::mono_small("·".to_owned(), FAINT));
        }
        for (i, part) in [
            w.length(video.length_ms),
            format!("{}×{}", video.width, video.height),
            format!("{} fps", video.fps),
            w.mb(video.size),
            format!("{} {}", w.day(video.made_at, self.now_unix), w.clock(video.made_at)),
        ]
        .into_iter()
        .enumerate()
        {
            if i > 0 {
                facts = facts.push(ui::mono_small("·".to_owned(), FAINT));
            }
            facts = facts.push(ui::mono_small(part, FAINT));
        }
        let caption = ui::fading(ui::fade() * dim, || {
            column![
                text(video.player.clone()).font(theme::SANS_SEMI).size(20.0).wrapping(text::Wrapping::None).color(ui::faded(INK)),
                text(video.map_line()).font(theme::SANS).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(MUTED)),
                container(facts).padding(Padding::ZERO.top(4.0)),
            ]
            .spacing(1)
        });
        let sending_this = self.sending.as_ref().filter(|s| s.path == video.path);
        let telegram: Element<'_, Message> = match sending_this {
            Some(sending) if sending.over.is_none() => ui::progress(w.t("sending"), self.progress_shown, None),
            _ => ui::primary(w.t("to-telegram"), Some(Message::SendVideo)),
        };
        let buttons = ui::fading(ui::fade() * dim, || {
            row![telegram, ui::quiet(w.t("in-folder"), Some(Message::RevealVideo)), ui::quiet(w.t("delete"), Some(Message::AskDelete))]
                .spacing(4)
                .align_y(iced::Center)
        });
        let under = row![container(caption).width(Length::Fill).clip(true), buttons].spacing(16).align_y(iced::Center);
        let stage_w = (self.width - 2.0 * STAGE_GAP).max(320.0);
        let room = (self.height - 2.0 * STAGE_GAP - STAGE_UNDER).max(180.0);
        let screen_h = (stage_w * 9.0 / 16.0).min(room);
        let screen_w = (screen_h * 16.0 / 9.0).min(stage_w);
        let inside = column![
            container(screen).width(Length::Fill).height(screen_h),
            container(timeline).padding(Padding { top: 8.0, right: 24.0, bottom: 4.0, left: 24.0 }),
            container(under).padding(Padding { top: 6.0, right: 24.0, bottom: 18.0, left: 24.0 }).height(STAGE_UNDER - 30.0),
        ]
        .width(Length::Fill);
        let card = container(inside).width(screen_w).height(screen_h + STAGE_UNDER).style(ui::box_faded(theme::stage)).clip(true);
        let backdrop = mouse_area(ui::veil(theme::SCRIM)).on_press(Message::ClosePlayer);
        stack![backdrop, container(card).width(Length::Fill).height(Length::Fill).center(Length::Fill)]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn delete_card(&self) -> Element<'_, Message> {
        let w = &self.words;
        let Some(video) = self.open_video.and_then(|at| self.store.videos.get(at)) else {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        };
        let top = column![
            ui::title(w.t("delete-video")),
            ui::why(format!("{} — {} · {}. {}", video.player, video.map_line(), w.mb(video.size), w.t("to-the-bin"))),
        ]
        .spacing(6);
        let bottom = row![ui::grow(), ui::quiet(w.t("keep"), Some(Message::KeepVideo)), ui::primary(w.t("delete"), Some(Message::DeleteVideo))]
            .spacing(4)
            .align_y(iced::Center);
        let card = ui::card(top.into(), Some(bottom.into()));
        stack![
            mouse_area(ui::veil(theme::SCRIM)).on_press(Message::KeepVideo),
            container(container(card).width(theme::COLUMN).padding(Padding::ZERO.top(220.0)))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

const VIDEO_THUMB: (u32, u32) = (96, 54);
const VIDEO_ROW: f32 = 72.0;
const CIRCLE_SIDE: f32 = 28.0;
const AVATAR_SIDE: u32 = 80;
const MENU_W: f32 = 400.0;
const MENU_TOP: f32 = 80.0;
const BADGE: f32 = 16.0;
const BADGE_OUT: f32 = 4.0;
const CORNER_X: f32 = 16.0;
const CORNER_OUT: f32 = 6.0;

impl Main {
    fn circle(&self, side: f32, pressable: bool) -> Element<'_, Message> {
        let ring = if pressable && self.busy() { 1.0 } else { 0.0 };
        let face: Element<'_, Message> = match (&self.avatar, self.signed_in()) {
            (Some(handle), _) => image(handle.clone()).content_fit(ContentFit::Cover).width(side).height(side).border_radius(side / 2.0).opacity(ui::fade()).into(),
            (None, true) => {
                let letter = self
                    .account
                    .as_ref()
                    .map(|a| a.name.clone())
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| self.settings.linked_as.trim_start_matches('@').to_owned())
                    .chars()
                    .next()
                    .map(|c| c.to_uppercase().to_string())
                    .unwrap_or_default();
                iced::widget::canvas(ui::Disc { letter, hatched: false }).width(side).height(side).into()
            }
            (None, false) => iced::widget::canvas(ui::Disc { letter: String::new(), hatched: true }).width(side).height(side).into(),
        };
        let layered = stack![face, iced::widget::canvas(ui::Ring { alpha: ring }).width(side).height(side)].width(side).height(side);
        if pressable {
            button(layered).padding(0).style(theme::bare).on_press(Message::Circle).into()
        } else {
            layered.into()
        }
    }

    fn menu_layer(&self) -> Element<'_, Message> {
        let Some(tab) = self.menu else {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        };
        let open = self.menu_open.interpolate(0.0, 1.0, self.now);
        let opening = self.menu_open.value();
        let swap = self.tab_fade.interpolate(0.0, 1.0, self.now);
        let order = |t: Tab| -> f32 {
            match t {
                Tab::Account => 0.0,
                Tab::Feed => 1.0,
                Tab::Stats => 2.0,
            }
        };
        let gap = order(tab) - order(self.seg_from);
        let direction = if gap == 0.0 { 0.0 } else { gap.signum() };
        let slide = (1.0 - swap) * 28.0 * direction;
        let late = |i: f32| if opening { (open * 1.4 - 0.2 * i).clamp(0.0, 1.0) } else { open };
        let mut stackup = column![].spacing(8).width(MENU_W + CORNER_OUT);
        let k0 = late(0.0);
        stackup = stackup.push(ui::fading(ui::fade() * k0, || ui::grown(self.menu_head(), Point::new(1.0, 0.0), 0.0, 0.9 + 0.1 * k0)));
        let k1 = late(1.0);
        stackup = stackup.push(ui::fading(ui::fade() * k1, || ui::grown(self.menu_segments(tab), Point::new(1.0, 0.0), 0.0, 0.9 + 0.1 * k1)));
        let k2 = late(2.0) * swap;
        let tiles = ui::fading(ui::fade() * k2, || {
            let content = match tab {
                Tab::Account => self.account_tiles(),
                Tab::Feed => self.feed_tiles(),
                Tab::Stats => self.stats_tiles(),
            };
            let mut list = column![].spacing(8).width(MENU_W);
            for tile in content {
                list = list.push(tile);
            }
            ui::grown(list, Point::new(1.0, 0.0), 0.0, 0.9 + 0.1 * late(2.0)).shifted(slide)
        });
        stackup = stackup.push(tiles);
        let x = (self.width - 40.0 - MENU_W).max(16.0);
        let whole = ui::grown(stackup, Point::new(1.0, 0.0), -(1.0 - open) * 14.0, 1.0);
        let backdrop: Element<'_, Message> = if self.menu_open.value() {
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::MenuClose).into()
        } else {
            Space::new().width(Length::Fill).height(Length::Fill).into()
        };
        stack![backdrop, pin(whole).x(x).y(MENU_TOP)].width(Length::Fill).height(Length::Fill).into()
    }

    fn scenery(&self, map_hash: &str, width: f32, height: f32) -> Element<'_, Message> {
        match self.scenes.get(map_hash) {
            Some(handle) if !map_hash.is_empty() => image(handle.clone())
                .content_fit(ContentFit::Cover)
                .width(width)
                .height(height)
                .opacity(0.4 * ui::fade())
                .border_radius(theme::CARD_RADIUS - 2.0)
                .into(),
            _ => Space::new().width(width).height(height).into(),
        }
    }

    fn pictured<'a>(&'a self, inside: Element<'a, Message>, bad: bool, map_hash: &str) -> Element<'a, Message> {
        if !self.scenes.contains_key(map_hash) || map_hash.is_empty() {
            return self.card(inside, bad);
        }
        let face = container(inside).padding([12, 14]).width(Length::Fill);
        let backdrop = self.scenery(map_hash, MENU_W, 64.0);
        container(stack![backdrop, face].width(Length::Fill))
            .width(Length::Fill)
            .style(ui::box_faded(if bad { theme::tile_bad } else { theme::bubble }))
            .clip(true)
            .into()
    }

    fn background_for(&self, map_hash: &str) -> Option<PathBuf> {
        self.entries()
            .iter()
            .find(|e| e.map_hash == map_hash)
            .and_then(|e| e.map.as_ref().and_then(|m| m.background.clone()))
            .or_else(|| self.store.videos.iter().find(|v| v.map_hash == map_hash).and_then(|v| v.background.clone()))
    }

    fn scenes_for_notices(&self) -> Task<Message> {
        let wanted: Vec<(String, PathBuf)> = self
            .notices
            .notices
            .iter()
            .take(6)
            .filter(|n| !n.map_hash.is_empty() && !self.scenes.contains_key(&n.map_hash))
            .filter_map(|n| self.background_for(&n.map_hash).map(|bg| (n.map_hash.clone(), bg)))
            .collect();
        if wanted.is_empty() {
            return Task::none();
        }
        ui::streamed(move |push| {
            for (hash, path) in wanted {
                let handle = decoded(&path, SCENE_WIDTH, None);
                if !push(Message::Scene(hash, handle)) {
                    return;
                }
            }
        })
    }

    fn card<'a>(&'a self, inside: Element<'a, Message>, bad: bool) -> Element<'a, Message> {
        container(inside)
            .padding([12, 14])
            .width(Length::Fill)
            .style(ui::box_faded(if bad { theme::tile_bad } else { theme::bubble }))
            .into()
    }

    fn menu_head(&self) -> Element<'_, Message> {
        let w = &self.words;
        let name = self.account.as_ref().map(|a| a.name.clone()).filter(|n| !n.is_empty()).unwrap_or_else(|| self.settings.linked_as.trim_start_matches('@').to_owned());
        let handle = self.account.as_ref().map(|a| a.username.clone()).filter(|u| !u.is_empty()).map(|u| format!("@{u}"));
        let (title, under) = if self.signed_in() {
            let chat = self.chat_name();
            let mut under = handle.unwrap_or(if chat == "—" { w.t("linked") } else { chat });
            if let Some(me) = &self.account {
                under = format!("{under} · ID {}", me.telegram_id);
            }
            (if name.is_empty() { w.t("signed-in") } else { name }, under)
        } else {
            (w.t("not-signed-in"), w.t("stays-here"))
        };
        let mut head = row![
            self.circle(40.0, false),
            column![
                text(title).font(theme::SANS_SEMI).size(15.0).wrapping(text::Wrapping::None).color(ui::faded(INK)),
                ui::mono_small(under, FAINT),
            ]
            .spacing(2),
            ui::grow(),
        ]
        .spacing(12)
        .align_y(iced::Center);
        if self.signed_in() {
            head = head.push(ui::link(w.t("sign-out"), Message::SignOut));
        }
        self.card(head.into(), false)
    }

    fn menu_segments(&self, tab: Tab) -> Element<'_, Message> {
        let w = &self.words;
        let inner = MENU_W - 2.0 * 4.0;
        let seg_w = (inner - 2.0 * 4.0) / 3.0;
        let at = |t: Tab| match t {
            Tab::Account => 0.0,
            Tab::Feed => 1.0,
            Tab::Stats => 2.0,
        };
        let k = self.seg_slide.interpolate(0.0, 1.0, self.now);
        let x = 4.0 + (at(self.seg_from) + (at(tab) - at(self.seg_from)) * k) * (seg_w + 4.0);
        let pill = container(Space::new().width(seg_w).height(28.0)).style(ui::box_faded(theme::segment_pill));
        let mut words = row![].spacing(4);
        for (key, this) in [("account", Tab::Account), ("feed", Tab::Feed), ("stats", Tab::Stats)] {
            words = words.push(
                button(container(text(w.t(key)).font(theme::SANS_SEMI).size(theme::CAPTION)).width(seg_w).height(28.0).center(Length::Fill))
                    .padding(0)
                    .style(ui::button_faded(theme::segment(tab == this)))
                    .on_press(Message::MenuTab(this)),
            );
        }
        let face = stack![pin(pill).x(x).y(4.0), container(words).padding(4)].width(MENU_W).height(36.0);
        container(face).width(Length::Fill).style(ui::box_faded(theme::bubble)).into()
    }

    fn kv(&self, key: String, value: String) -> Element<'_, Message> {
        row![
            text(key).font(theme::SANS).size(theme::CAPTION).color(ui::faded(MUTED)),
            ui::grow(),
            text(value).font(theme::MONO).size(theme::CAPTION).color(ui::faded(INK)),
        ]
        .spacing(12)
        .align_y(iced::Center)
        .height(26.0)
        .into()
    }

    fn big(&self, value: String, key: String) -> Element<'_, Message> {
        self.card(
            column![
                text(value).font(theme::MONO_BOLD).size(22.0).wrapping(text::Wrapping::None).color(ui::faded(INK)),
                text(key).font(theme::SANS).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(FAINT)),
            ]
            .spacing(2)
            .into(),
            false,
        )
    }

    fn pair<'a>(&'a self, left: Element<'a, Message>, right: Element<'a, Message>) -> Element<'a, Message> {
        row![container(left).width(Length::Fill), container(right).width(Length::Fill)].spacing(8).into()
    }

    fn account_tiles(&self) -> Vec<Element<'_, Message>> {
        let w = &self.words;
        if !self.signed_in() {
            let ask = column![
                text(w.t("why-sign-in")).font(theme::SANS).size(theme::CAPTION).color(ui::faded(MUTED)),
                container(ui::primary(w.t("sign-in"), Some(Message::SignIn))).padding(Padding::ZERO.top(6.0)),
            ]
            .spacing(6);
            return vec![
                self.card(ask.into(), false),
                self.pair(self.big(self.entries().len().to_string(), w.t("replays-in-journal")), self.big(bot::BUILD.to_owned(), w.t("build"))),
            ];
        }
        let rows = column![self.kv(w.t("videos-go-to"), self.chat_name()), self.kv(w.t("worker"), w.t("coming-later"))].spacing(2);
        let videos = format!("{} · {}", self.store.videos.len(), w.mb(self.store.total_size()));
        vec![
            self.card(rows.into(), false),
            self.pair(self.big(bot::BUILD.to_owned(), w.t("build")), self.big(self.store.videos.len().to_string(), format!("{} · {}", w.t("videos").to_lowercase(), videos.split(" · ").nth(1).unwrap_or("")))),
        ]
    }

    fn chat_name(&self) -> String {
        match &self.account {
            Some(me) if !me.username.is_empty() => format!("@{}", me.username),
            Some(me) if !me.name.is_empty() => me.name.clone(),
            _ if !self.settings.linked_as.is_empty() => self.settings.linked_as.clone(),
            _ => "—".to_owned(),
        }
    }

    fn feed_tiles(&self) -> Vec<Element<'_, Message>> {
        let w = &self.words;
        let mut tiles = Vec::new();
        let job = |words: String, detail: String, fraction: f32| -> Element<'_, Message> {
            column![
                row![
                    iced::widget::canvas(ui::Dot).width(8.0).height(8.0),
                    text(words).font(theme::SANS_SEMI).size(theme::CAPTION).color(ui::faded(INK)),
                    container(text(format!("· {detail}")).font(theme::SANS).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(MUTED))).width(Length::Fill).clip(true),
                ]
                .spacing(8)
                .align_y(iced::Center)
                .height(20.0),
                iced::widget::canvas(ui::Thread { fraction }).width(Length::Fill).height(3.0),
            ]
            .spacing(6)
            .into()
        };
        if let Some(rendering) = self.rendering.as_ref().filter(|r| !r.is_over()) {
            let who = self.entries().iter().find(|e| e.path == rendering.path).map(|e| e.title().unwrap_or_default()).unwrap_or_default();
            tiles.push(self.card(job(w.t("drawing"), who, self.progress_shown), false));
        }
        if let Some(fetching) = self.fetching.as_ref().filter(|f| !f.is_over()) {
            let title = self.entries().iter().find(|e| e.map_hash == fetching.hash).map(|e| e.title().unwrap_or_default()).unwrap_or_default();
            tiles.push(self.card(job(w.t("fetch-downloading"), title, self.progress_shown), false));
        }
        if let Some(sending) = self.sending.as_ref().filter(|s| s.over.is_none()) {
            let title = self.store.videos.iter().find(|v| v.path == sending.path).map(|v| v.map_line()).unwrap_or_default();
            tiles.push(self.card(job(w.t("sending"), title, self.progress_shown), false));
        }
        for notice in self.notices.notices.iter().take(5) {
            let bad = notice.mark == notices::Mark::Bad;
            let alive = match (self.leaving.get(&notice.id), self.arrivals.get(&notice.id)) {
                (Some(going), _) => going.interpolate(0.0, 1.0, self.now),
                (None, Some(coming)) => coming.interpolate(0.0, 1.0, self.now),
                _ => 1.0,
            };
            let tile = ui::fading(ui::fade() * alive, || {
                let card = self.pictured(self.notice_row(notice), bad, &notice.map_hash);
                let framed = stack![
                    container(card).padding(Padding { top: CORNER_OUT, right: CORNER_OUT, bottom: 0.0, left: 0.0 }).width(Length::Fill),
                    pin(self.dismiss(notice.id, bad)).x(MENU_W - CORNER_X - 1.0).y(0.0),
                ]
                .width(Length::Fill);
                ui::grown(framed, Point::new(1.0, 0.5), 0.0, 1.0).shifted((1.0 - alive) * 24.0)
            });
            tiles.push(tile.into());
        }
        tiles
    }

    fn notice_row(&self, notice: &notices::Notice) -> Element<'_, Message> {
        let w = &self.words;
        let bad = notice.mark == notices::Mark::Bad;
        let title: Element<'_, Message> = if bad {
            button(text(notice.words.clone()).font(theme::SANS_SEMI).size(theme::CAPTION).wrapping(text::Wrapping::None))
                .padding(0)
                .style(ui::button_faded(theme::danger_words))
                .on_press(Message::ShowError(notice.id))
                .into()
        } else {
            text(notice.words.clone()).font(theme::SANS_SEMI).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(INK)).into()
        };
        let mut second = notice.detail.clone();
        if !notice.note.is_empty() && !bad {
            second = if second.is_empty() { notice.note.clone() } else { format!("{second} · {}", notice.note) };
        }
        let below: Element<'_, Message> = if second.is_empty() {
            Space::new().height(0.0).into()
        } else {
            container(text(second).font(theme::SANS).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(MUTED)))
                .width(Length::Fill)
                .clip(true)
                .into()
        };
        let mut side = column![ui::mono_small(w.clock(notice.at), FAINT)].spacing(2).align_x(iced::alignment::Horizontal::Right);
        if matches!(notice.link, notices::Link::RenderAgain(_) | notices::Link::OpenVideo(_)) {
            let words = if matches!(notice.link, notices::Link::OpenVideo(_)) { w.t("open") } else { w.t("once-more") };
            side = side.push(ui::small_button(words, Message::ToastLink(notice.id)));
        }
        row![
            self.notice_mark(notice, 40.0),
            column![title, below].spacing(1).width(Length::Fill),
            container(side).align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(12)
        .align_y(iced::Center)
        .into()
    }

    fn error_layer(&self) -> Element<'_, Message> {
        let Some(notice) = self.error_shown.and_then(|id| self.notices.get(id)) else {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        };
        let w = &self.words;
        let mut lines = column![ui::title(notice.words.clone()), ui::why(notice.detail.clone())].spacing(6);
        if !notice.note.is_empty() {
            lines = lines.push(container(ui::mono(notice.note.clone(), MUTED)).padding(Padding::ZERO.top(8.0)));
        }
        lines = lines.push(container(ui::cap(format!("{} · {}", w.day(notice.at, self.now_unix), w.clock(notice.at)))).padding(Padding::ZERO.top(4.0)));
        let mut bottom = row![ui::grow(), ui::quiet(w.t("close"), Some(Message::HideError))].spacing(4).align_y(iced::Center);
        if matches!(notice.link, notices::Link::RenderAgain(_)) {
            bottom = bottom.push(ui::primary(w.t("once-more"), Some(Message::ToastLink(notice.id))));
        }
        let card = ui::card(lines.into(), Some(bottom.into()));
        stack![
            mouse_area(ui::veil(theme::SCRIM)).on_press(Message::HideError),
            container(container(card).width(theme::COLUMN).padding(Padding::ZERO.top(180.0)))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn notice_mark(&self, notice: &notices::Notice, side: f32) -> Element<'_, Message> {
        let glyph = match notice.mark {
            notices::Mark::Done => "✓",
            notices::Mark::Bad => "✕",
            notices::Mark::Plain => "·",
        };
        let picture: Element<'_, Message> = match self.thumbs.get(&notice.map_hash) {
            Some(handle) if !notice.map_hash.is_empty() => image(handle.clone())
                .content_fit(ContentFit::Cover)
                .width(side)
                .height(side)
                .border_radius(8.0)
                .opacity(ui::fade() * 0.9)
                .into(),
            _ => container(Space::new().width(side).height(side)).style(ui::box_faded(theme::chip)).into(),
        };
        let badge = container(text(glyph).font(theme::MONO_BOLD).size(10.0).color(ui::faded(INK)))
            .width(BADGE)
            .height(BADGE)
            .center(BADGE)
            .style(ui::box_faded(theme::badge_of(if notice.mark == notices::Mark::Bad { ACCENT } else { theme::GRADE_A })));
        let reach = side + BADGE_OUT;
        stack![
            container(picture).width(reach).height(reach),
            pin(badge).x(side + BADGE_OUT - BADGE).y(side + BADGE_OUT - BADGE),
        ]
        .width(reach)
        .height(reach)
        .into()
    }

    fn dismiss(&self, id: u64, bad: bool) -> Element<'_, Message> {
        let glyph = iced::widget::canvas(ui::Cross { colour: if bad { ACCENT } else { MUTED } }).width(CORNER_X).height(CORNER_X);
        button(glyph)
            .padding(0)
            .style(ui::button_faded(theme::corner(bad)))
            .on_press(Message::DismissNotice(id))
            .into()
    }

    fn stats_tiles(&self) -> Vec<Element<'_, Message>> {
        let w = &self.words;
        let sent = self.store.videos.iter().filter(|v| v.sent_at.is_some()).count();
        let sent_size: u64 = self.store.videos.iter().filter(|v| v.sent_at.is_some()).map(|v| v.size).sum();
        let heading = |key: &str| container(ui::mono_small(w.t(key).to_uppercase(), FAINT)).padding(Padding::ZERO.bottom(8.0));
        let worker = column![heading("as-worker"), self.kv(w.t("jobs-done"), w.t("coming-later"))].spacing(0);
        let device = column![
            heading("on-this-device"),
            self.pair(
                self.big(self.entries().len().to_string(), w.t("replays-in-journal")),
                self.big(self.store.videos.len().to_string(), format!("{} · {}", w.t("videos").to_lowercase(), w.mb(self.store.total_size()))),
            ),
            self.pair(self.big(sent.to_string(), format!("{} · {}", w.t("sent-count").to_lowercase(), w.mb(sent_size))), self.big(bot::BUILD.to_owned(), w.t("build"))),
        ]
        .spacing(8);
        vec![self.card(worker.into(), false), self.card(device.into(), false)]
    }

    fn sign_in_layer(&self) -> Element<'_, Message> {
        if matches!(self.pairing, Pairing::Idle) {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        }
        let w = &self.words;
        let mut left = column![ui::title(w.t("sign-in")), ui::why(w.t("sign-in-how"))].spacing(6).width(Length::Fill);
        match &self.pairing {
            Pairing::Waiting { code, .. } => {
                left = left.push(container(text(code.clone()).font(theme::MONO_BOLD).size(26.0).color(ui::faded(INK))).padding(Padding::ZERO.top(12.0)));
                left = left.push(ui::cap(w.t("code-lasts")));
                left = left.push(container(row![iced::widget::canvas(ui::Dot).width(8.0).height(8.0), ui::mono_small(w.t("waiting-confirm"), MUTED)].spacing(8).align_y(iced::Center)).padding(Padding::ZERO.top(10.0)));
            }
            Pairing::Unavailable => {
                left = left.push(container(ui::cap(w.t("no-pairing-yet"))).padding(Padding::ZERO.top(12.0)));
            }
            _ => {
                left = left.push(container(ui::mono_small(w.t("asking-bot"), MUTED)).padding(Padding::ZERO.top(12.0)));
            }
        }
        let mut sides = row![left].spacing(24).align_y(iced::Top);
        if let (Some(qr), Pairing::Waiting { .. }) = (&self.qr, &self.pairing) {
            sides = sides.push(ui::qr(qr));
        }
        let waiting = matches!(self.pairing, Pairing::Waiting { .. });
        let bottom = row![
            ui::primary(w.t("open-telegram"), waiting.then_some(Message::OpenTelegram)),
            ui::quiet(w.t("copy-link"), waiting.then_some(Message::CopyLink)),
            ui::grow(),
            ui::quiet(w.t("later-word"), Some(Message::LaterSignIn)),
        ]
        .spacing(4)
        .align_y(iced::Center);
        let card = ui::card(sides.into(), Some(bottom.into()));
        stack![
            mouse_area(ui::veil(theme::SCRIM)).on_press(Message::LaterSignIn),
            container(container(card).width(theme::COLUMN).padding(Padding::ZERO.top(180.0)))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

pub fn decoded_bytes(bytes: &[u8], side: u32) -> Option<image::Handle> {
    let picture = ::image::load_from_memory(bytes).ok()?;
    let picture = picture.resize_to_fill(side, side, ::image::imageops::FilterType::Lanczos3).to_rgba8();
    Some(image::Handle::from_rgba(side, side, picture.into_raw()))
}
const TOAST_W: f32 = 380.0;
const TOAST_H: f32 = 74.0;
const TOAST_TOP: f32 = 92.0;
pub const TOAST_IN: Duration = Duration::from_millis(180);
pub const MENU_OPEN: Duration = Duration::from_millis(320);
pub const MENU_CLOSE: Duration = Duration::from_millis(170);
pub const TAB_FADE: Duration = Duration::from_millis(180);
pub const NOTICE_LEAVE: Duration = Duration::from_millis(200);
pub const NOTICE_ARRIVE: Duration = Duration::from_millis(260);
pub const OVERLAY_FADE: Duration = Duration::from_millis(220);
pub const TOAST_STAY: Duration = Duration::from_secs(6);
const TOASTS_AT_MOST: usize = 3;

impl Main {
    fn toast_layer(&self) -> Element<'_, Message> {
        if self.toasts.is_empty() {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        }
        let mut layers = stack![].width(Length::Fill).height(Length::Fill);
        let mut slot = 0.0;
        for toast in &self.toasts {
            let Some(notice) = self.notices.get(toast.id) else {
                continue;
            };
            let k = toast.shown.interpolate(0.0, 1.0, self.now);
            let home = (self.width - 40.0 - TOAST_W).max(16.0);
            let x = home + (1.0 - k) * (TOAST_W + 48.0);
            let y = TOAST_TOP + slot - CORNER_OUT;
            let age = self.now.saturating_duration_since(toast.born).as_secs_f32();
            let pulse = if age < 3.0 && toast.shown.value() { (std::f32::consts::PI * age).sin().powi(2) } else { 0.0 };
            let card = self.toast(toast, notice, pulse);
            layers = layers.push(pin(card).x(x).y(y));
            if toast.shown.value() {
                slot += TOAST_H + 8.0;
            }
        }
        layers.into()
    }

    fn toast(&self, toast: &Toast, notice: &notices::Notice, pulse: f32) -> Element<'_, Message> {
        let w = &self.words;
        let bad = notice.mark == notices::Mark::Bad;
        let words = row![
            text(notice.words.clone()).font(theme::SANS_SEMI).size(theme::BODY).wrapping(text::Wrapping::None).color(ui::faded(if bad { ACCENT } else { INK })),
        ];
        let detail = text(notice.detail.clone()).font(theme::SANS).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(MUTED));
        let note = if bad { String::new() } else { notice.note.clone() };
        let note = text(note).font(theme::MONO).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(FAINT));
        let column = column![words, detail, note].spacing(1).width(Length::Fill);
        let mut line = row![self.notice_mark(notice, 44.0), container(column).width(Length::Fill).clip(true)].spacing(12).align_y(iced::Center);
        let link = match notice.link {
            notices::Link::OpenVideo(_) => Some(w.t("open")),
            notices::Link::RenderAgain(_) => Some(w.t("once-more")),
            notices::Link::None => None,
        };
        if let Some(words) = link {
            line = line.push(ui::small_button(words, Message::ToastLink(toast.id)));
        }
        let face = container(line).padding([12, 14]).width(TOAST_W).height(TOAST_H);
        let backdrop = self.scenery(&notice.map_hash, TOAST_W, TOAST_H);
        let card = container(stack![backdrop, face].width(TOAST_W).height(TOAST_H)).width(TOAST_W).height(TOAST_H).style(theme::toast(pulse, bad)).clip(true);
        let sensed = mouse_area(card).on_enter(Message::ToastHover(toast.id, true)).on_exit(Message::ToastHover(toast.id, false));
        stack![
            pin(sensed).x(0.0).y(CORNER_OUT),
            pin(self.dismiss(toast.id, bad)).x(TOAST_W - CORNER_X + CORNER_OUT - 1.0).y(0.0),
        ]
        .width(TOAST_W + CORNER_OUT)
        .height(TOAST_H + CORNER_OUT)
        .into()
    }
}
const STAGE_GAP: f32 = 40.0;
const STAGE_UNDER: f32 = 118.0;

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
    let colour = mod_colour(acronym);
    let alpha = ui::fade();
    container(text(acronym.to_owned()).font(theme::MONO_BOLD).size(10.0).color(ui::faded(theme::ON_ACCENT)))
        .padding([1, 6])
        .style(theme::badge(Color { a: colour.a * alpha, ..colour }))
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

pub fn max_combo_of(path: &Path, map: Option<&library::Map>, hash: &str) -> Option<u32> {
    let map = map?;
    let bytes = std::fs::read(path).ok()?;
    let replay = dossier_replay::Replay::parse(&bytes).ok()?;
    let found = dossier_produce::locate::load_map(&map.file, hash).ok()?;
    let beatmap = dossier_beatmap::Beatmap::parse(&found.text).ok()?;
    Some(dossier_assay::max_combo(&beatmap, replay.mods))
}
