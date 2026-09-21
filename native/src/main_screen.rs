use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use iced::animation::Easing;
use iced::widget::{
    button, column, container, float, image, mouse_area, pin, row, scrollable, stack, text, Space,
};
use iced::{window, Animation, Color, ContentFit, Element, Length, Padding, Subscription, Task, Vector};

use crate::lang::{typed, Words};
use crate::library::{self, Entry, Grade, Library};
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
pub const PEEK: Duration = Duration::from_millis(180);
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
    Live(live::Frame),
    TogglePlay,
    Strip(scrollable::Viewport),
    ScrubTo(f32),
    Traced(PathBuf, std::sync::Arc<Vec<(f64, f32, f32)>>),
    Dropped(PathBuf),
    Resized(f32),
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
    pub thumbs: HashMap<String, image::Handle>,
    pub scenes: HashMap<String, image::Handle>,
    pub scene_before: Option<Option<image::Handle>>,
    pub lengths: HashMap<PathBuf, i64>,
    pub overlay: Overlay,
    pub rendering: Option<Rendering>,
    pub fetching: Option<Fetching>,
    pub looking: Option<scan::Step>,
    pub live: Option<Live>,
    pub live_before: Option<image::Handle>,
    pub hatch: image::Handle,
    pub trail: Option<Trail>,
    pub strip_view: Option<(f32, f32, f32)>,
    pub peek: Animation<bool>,
    pub peeked: Option<usize>,
    pub progress_shown: f32,
    pub ffmpeg: Option<PathBuf>,
    pub now: Instant,
    pub started: Instant,
    pub now_unix: i64,
    pub enter: Animation<bool>,
    pub arrive: Animation<bool>,
    pub swap: Animation<bool>,
    pub swap_waits: bool,
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
            live: None,
            live_before: None,
            hatch: ui::hatched_picture(live::SIZE.0, live::SIZE.1, live::dim_at),
            trail: None,
            strip_view: None,
            peek: Animation::new(false).duration(PEEK).easing(Easing::EaseOutCubic),
            peeked: None,
            progress_shown: 0.0,
            ffmpeg: crate::checks::ffmpeg_on_path(),
            now: Instant::now(),
            started: Instant::now(),
            now_unix: unix_now(),
            enter: Animation::new(false).duration(ENTER).easing(Easing::EaseOutCubic),
            arrive: Animation::new(false).duration(ARRIVE).easing(Easing::EaseOutCubic).go(true, Instant::now()),
            swap: Animation::new(true).duration(SWAP).easing(Easing::EaseOutCubic),
            swap_waits: false,
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
            || self.live.as_ref().is_some_and(|l| l.fade.is_animating(self.now) || l.rest.is_animating(self.now))
            || self.trail.as_ref().is_some_and(|t| t.paused_at.is_none())
            || self.peek.is_animating(self.now)
            || self.progress_target().is_some_and(|t| (t - self.progress_shown).abs() > 0.001)
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
        self.peek = Animation::new(false).duration(PEEK).easing(Easing::EaseOutCubic);
        self.peeked = None;
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
                        if Some(at) != self.chosen {
                            self.peeked = Some(at);
                            self.peek.go_mut(true, Instant::now());
                        } else {
                            self.peek.go_mut(false, Instant::now());
                        }
                    }
                    None => {
                        self.lift.go_mut(false, Instant::now());
                        self.peek.go_mut(false, Instant::now());
                    }
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
                    live::Frame::Still(handle) => {
                        live.still = Some(handle);
                        live.rest.go_mut(true, Instant::now());
                    }
                    live::Frame::Failed(_) => {
                        live.control.stop();
                        self.live = None;
                    }
                }
                Task::none()
            }
            Message::Strip(viewport) => {
                let offset = viewport.absolute_offset().x;
                self.strip_view = Some((offset, viewport.content_bounds().width, viewport.bounds().width));
                Task::none()
            }
            Message::ScrubTo(fraction) => {
                let Some((_, content, shown)) = self.strip_view else {
                    return Task::none();
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
                match self.progress_target() {
                    Some(target) => self.progress_shown += (target - self.progress_shown) * 0.12,
                    None => self.progress_shown = 0.0,
                }
                Task::none()
            }
        }
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
                    let resting = live.rest.interpolate(0.0, 1.0, self.now);
                    let moving: Element<'_, Message> = full(handle, alpha * seen);
                    let layered: Element<'_, Message> = match &live.still {
                        Some(still) if resting > 0.0 => stack![moving, full(still, alpha * seen * resting)].into(),
                        _ => moving,
                    };
                    mouse_area(layered).on_press(Message::TogglePlay).into()
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
        let overlay: Element<'_, Message> = if self.overlay != Overlay::None { self.overlay_view() } else { blank() };
        let layers = stack![scene_before, scene, live_before, live, body, crest, overlay];
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
        let chosen_block = self.block(entry, &was, &now, s, true);
        let h = self.peek.interpolate(0.0, 1.0, self.now);
        let peeked = self.peeked.and_then(|at| self.entries().get(at)).filter(|e| e.path != entry.path);
        let body: Element<'_, Message> = match peeked {
            Some(other) if h > 0.0 => {
                let shown = self.shown(other);
                stack![
                    ui::fading(ui::fade() * (1.0 - h), || self.block(entry, &was, &now, s, true)),
                    ui::fading(ui::fade() * h, || self.block(other, &shown, &shown, 1.0, false)),
                ]
                .into()
            }
            _ => chosen_block,
        };
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
            Some(Step::Saved(_)) => row![
                ui::primary(w.t("open"), Some(Message::OpenOut)),
                container(ui::link(w.t("in-folder"), Message::ShowOut)).padding(Padding::ZERO.left(6.0)),
            ]
            .align_y(iced::Center)
            .into(),
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
        let guessed_content: f32 = days
            .iter()
            .map(|(_, list)| list.len() as f32 * (theme::FRAME_W + 6.0) - 6.0 + 8.0)
            .sum::<f32>()
            + (days.len().saturating_sub(1)) as f32 * 22.0;
        let guessed_shown = (self.width - 80.0 - 80.0).max(1.0);
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
        let scale = if chosen { 1.0 } else { 1.0 + 0.04 * rise };
        float(sensed).scale(scale).translate(move |_, _| Vector::new(0.0, -2.0 * rise)).into()
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
