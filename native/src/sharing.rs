use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::animation::Easing;
use iced::widget::{button, column, container, image, mouse_area, row, scrollable, stack, text, Space};
use iced::{Animation, ContentFit, Element, Length, Padding, Task};

use super::{
    covered_bytes, mod_badge, read_library, unix_now, Bill, Main, Message as Outer, Overlay, Sending, CONTROLS_IN, PICTURE_INSET, PICTURE_RADIUS, STAGE_GAP, STAGE_OPEN,
    STAGE_TOP, STAGE_UNDER, VIDEO_DATE_W, VIDEO_ROW, VIDEO_THUMB,
};
use crate::inbox::{self, Accept, Face, Look, Received};
use crate::sources::Kind;
use crate::theme::{self, FAINT, INK, MUTED};
use crate::{bot, library, mixed, notices, player, ui, videos};

pub const EVERY: Duration = Duration::from_secs(60);
const SYNC_EVERY: Duration = Duration::from_secs(600);
const TOLD_AT_ONCE: usize = 3;
const LIST_HIGH: f32 = 296.0;
const FACE: f32 = 32.0;
const FROM_W: f32 = 170.0;
const STOPPED: &str = "stopped";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Mine,
    Received,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tab(Tab),
    Tick,
    Arrived(Result<inbox::Inbox, String>),
    Thumb(u64, Option<image::Handle>),
    Open(u64),
    Watch,
    Getting(u64, u64, u64),
    Got(u64, Result<(PathBuf, videos::Probe), String>),
    StopGetting,
    ToTelegram,
    Passed(u64, Result<(), String>),
    Draw,
    Drawn(u64, Result<(PathBuf, Box<library::Library>), String>),
    Remove,
    Pick,
    Receivers(Result<Vec<Face>, String>),
    Toggle(i64),
    Send,
    Shared(PathBuf, Option<u64>, bool, Result<inbox::Shared, String>),
    Close,
    Accept(Accept),
    Accepted(Accept, Result<String, String>),
    Replays(Result<mixed::State, String>),
    ShareReplays(bool),
    ReplaysSwitched(bool, Result<mixed::State, String>),
    ReplaysGiven(usize),
    ReplaysSynced(Result<mixed::Change, String>),
    Quiet,
}

#[derive(Debug, Clone)]
pub struct Getting {
    pub id: u64,
    pub done: u64,
    pub total: u64,
    pub stop: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
pub struct Picker {
    pub path: PathBuf,
    pub picked: Vec<i64>,
    pub busy: bool,
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pub tab: Tab,
    pub videos: Vec<Received>,
    pub accept: Accept,
    pub registered: bool,
    pub loaded: bool,
    pub failed: bool,
    pub asked: Option<Instant>,
    pub thumbs: HashMap<u64, image::Handle>,
    pub thumbs_asked: HashSet<u64>,
    pub open: Option<u64>,
    pub getting: Option<Getting>,
    pub passing: HashSet<u64>,
    pub passed: HashSet<u64>,
    pub drawing: Option<u64>,
    pub picker: Option<Picker>,
    pub receivers: Option<Vec<Face>>,
    pub receivers_failed: Option<String>,
    pub looks: HashMap<PathBuf, Look>,
    pub draw_wanted: Option<String>,
    pub render_look: Option<Look>,
    pub replays: Option<mixed::State>,
    pub replays_synced: Option<Instant>,
}

impl State {
    pub fn of(&self, id: u64) -> Option<&Received> {
        self.videos.iter().find(|video| video.id == id)
    }

    pub fn opened(&self) -> Option<&Received> {
        self.open.and_then(|id| self.of(id))
    }
}

fn first_letter(name: &str) -> String {
    name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default()
}

fn told(push: &mut dyn FnMut(Outer) -> bool, message: Message) -> bool {
    push(Outer::Sharing(message))
}

impl Main {
    fn bot_keys(&self) -> (String, String, String) {
        (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone())
    }

    pub(super) fn inbox_task(&mut self, force: bool) -> Task<Outer> {
        if self.settings.token.is_empty() || self.gallery {
            return Task::none();
        }
        let now = Instant::now();
        if !force && self.sharing.asked.is_some_and(|at| now.saturating_duration_since(at) < EVERY - Duration::from_secs(2)) {
            return Task::none();
        }
        self.sharing.asked = Some(now);
        let (server, token, name) = self.bot_keys();
        ui::in_thread(move || Outer::Sharing(Message::Arrived(bot::inbox(&server, &token, &name).map_err(|e| e.to_string()))))
    }

    pub(super) fn replays_state_task(&self) -> Task<Outer> {
        if self.settings.token.is_empty() || self.gallery {
            return Task::none();
        }
        let (server, token, name) = self.bot_keys();
        ui::in_thread(move || Outer::Sharing(Message::Replays(bot::replays_state(&server, &token, &name).map_err(|e| e.to_string()))))
    }

    pub(super) fn replays_give_task(&self) -> Task<Outer> {
        let Some(state) = self.sharing.replays.as_ref().filter(|state| state.on && !state.name.is_empty()) else {
            return Task::none();
        };
        if self.settings.token.is_empty() || self.gallery {
            return Task::none();
        }
        let own = state.name.to_lowercase();
        let plays: Vec<mixed::Play> = self
            .entries()
            .iter()
            .filter(|entry| !mixed::is_shared(&entry.path) && entry.player.to_lowercase() == own)
            .take(state.most.max(1) as usize)
            .map(|entry| {
                let (artist, title, version) = match (&entry.map, &entry.named) {
                    (Some(map), _) => (map.artist.clone(), map.title.clone(), map.version.clone()),
                    (None, Some(named)) => (named.artist.clone(), named.title.clone(), named.version.clone()),
                    (None, None) => Default::default(),
                };
                mixed::Play { path: entry.path.clone(), hash: entry.replay_hash.clone(), artist, title, version }
            })
            .collect();
        if plays.is_empty() {
            return Task::none();
        }
        let (server, token, name) = self.bot_keys();
        ui::in_thread(move || Outer::Sharing(Message::ReplaysGiven(mixed::give(&server, &token, &name, &plays, &mixed::ledger(), |_| {}).unwrap_or(0))))
    }

    pub(super) fn replays_sync_task(&mut self, force: bool) -> Task<Outer> {
        if self.settings.token.is_empty() || self.gallery || !self.settings.sources.iter().any(|source| source.is_shared() && source.on) {
            return Task::none();
        }
        let now = Instant::now();
        if !force && self.sharing.replays_synced.is_some_and(|at| now.saturating_duration_since(at) < SYNC_EVERY) {
            return Task::none();
        }
        self.sharing.replays_synced = Some(now);
        let (server, token, name) = self.bot_keys();
        let everyone = self.settings.people_everyone;
        ui::in_thread(move || Outer::Sharing(Message::ReplaysSynced(mixed::sync(&server, &token, &name, everyone).map_err(|e| e.to_string()))))
    }

    pub(super) fn offer_shared_source(&mut self) {
        if self.settings.token.is_empty() || self.gallery || self.settings.sources.iter().any(|source| source.is_shared()) {
            return;
        }
        self.settings.sources.push(crate::sources::shared(false));
        let _ = self.settings.save();
    }

    pub(super) fn send_meta(&self, video: &videos::Video, chat: Option<i64>) -> serde_json::Value {
        serde_json::json!({
            "caption": format!("{} — {}", video.player, video.map_line()),
            "name": video.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
            "width": video.width,
            "height": video.height,
            "duration": (video.length_ms / 1000).max(0),
            "chat": chat,
            "player": video.player,
            "song": video.song,
            "version": video.version,
            "mods": video.mods,
            "map_hash": video.map_hash,
            "settings": video.look,
        })
    }

    fn faces_task(&mut self, urls: Vec<String>) -> Task<Outer> {
        let wanted: Vec<String> = urls
            .into_iter()
            .filter(|url| !url.is_empty() && !self.news_pictures.contains_key(url) && self.news_asked.insert(url.clone()))
            .collect();
        if wanted.is_empty() {
            return Task::none();
        }
        ui::streamed(move |push| {
            for url in wanted {
                let handle = crate::news::picture(&url).and_then(|bytes| covered_bytes(&bytes, 128, 128));
                if !push(Outer::NewsPicture(url, handle)) {
                    return;
                }
            }
        })
    }

    fn inbox_thumbs_task(&mut self) -> Task<Outer> {
        let wanted: Vec<u64> = self
            .sharing
            .videos
            .iter()
            .filter(|video| video.thumb && !self.sharing.thumbs.contains_key(&video.id))
            .map(|video| video.id)
            .collect();
        let wanted: Vec<u64> = wanted.into_iter().filter(|id| self.sharing.thumbs_asked.insert(*id)).collect();
        if wanted.is_empty() {
            return Task::none();
        }
        let (server, token, name) = self.bot_keys();
        ui::streamed(move |push| {
            for id in wanted {
                let handle = bot::inbox_thumb(&server, &token, &name, id).ok().and_then(|bytes| covered_bytes(&bytes, VIDEO_THUMB.0 * 2, VIDEO_THUMB.1 * 2));
                if !told(push, Message::Thumb(id, handle)) {
                    return;
                }
            }
        })
    }

    fn probe_received(&self, id: u64, path: PathBuf) -> Task<Outer> {
        let ffmpeg = self.ffmpeg.clone();
        ui::in_thread(move || {
            let media = ffmpeg.ok_or_else(|| "no ffmpeg".to_owned()).and_then(|ffmpeg| videos::probe(&ffmpeg, &path).ok_or_else(|| "the video does not read".to_owned()));
            Outer::Sharing(Message::Got(id, media.map(|media| (path, media))))
        })
    }

    fn play_received(&mut self, path: PathBuf, media: videos::Probe) {
        let Some(ffmpeg) = self.ffmpeg.clone() else {
            return;
        };
        if let Some(old) = self.player.take() {
            old.borrow_mut().close();
        }
        self.leaving_player = false;
        self.mini_player = false;
        let now = Instant::now();
        self.stirred = now;
        self.over_controls = false;
        self.controls = Animation::new(true).duration(CONTROLS_IN).easing(Easing::EaseOutCubic);
        let manner = player::Manner { level: self.settings.player_level, muted: self.settings.player_muted, rate: self.settings.player_rate };
        self.player = Some(std::rc::Rc::new(std::cell::RefCell::new(player::Player::open(&ffmpeg, &path, media, manner))));
        self.open_video = None;
        self.clip = None;
        self.asking_delete = false;
        self.scrubbing = None;
        self.hint = None;
        self.cinema.go_mut(true, now);
    }

    fn shut_received(&mut self) {
        if let Some(player) = self.player.take() {
            player.borrow_mut().close();
        }
        if let Some(getting) = &self.sharing.getting {
            getting.stop.store(true, Ordering::SeqCst);
        }
        self.sharing.open = None;
        self.leaving_player = false;
        self.mini_player = false;
        self.shut_cinema();
    }

    fn accept_now(&mut self, accept: Accept) {
        if self.sharing.accept != accept {
            for one in Accept::ALL {
                let key = format!("accept-{}", one.tag());
                self.marks.remove(&key);
                self.marks_now.remove(&key);
            }
        }
        self.sharing.accept = accept;
    }

    fn told_of(&self, video: &videos::Video) -> (String, String) {
        (format!("{} — {}", video.player, video.map_line()), video.map_hash.clone())
    }

    pub(super) fn sharing_update(&mut self, message: Message) -> Task<Outer> {
        match message {
            Message::Quiet => Task::none(),
            Message::Tab(tab) => {
                self.sharing.tab = tab;
                match tab {
                    Tab::Received => self.inbox_task(true),
                    Tab::Mine => Task::none(),
                }
            }
            Message::Tick => Task::batch([self.inbox_task(false), self.replays_sync_task(false)]),
            Message::Arrived(Err(_)) => {
                self.sharing.failed = !self.sharing.loaded;
                Task::none()
            }
            Message::Arrived(Ok(inbox)) => {
                self.sharing.loaded = true;
                self.sharing.failed = false;
                self.sharing.registered = inbox.registered;
                self.accept_now(Accept::of(&inbox.accept));
                let news: Vec<Received> = inbox::fresh(self.settings.inbox_heard, &inbox.videos).into_iter().cloned().collect();
                self.sharing.videos = inbox.videos;
                if self.sharing.open.is_some() && self.sharing.opened().is_none() {
                    self.shut_received();
                }
                for video in news.iter().take(TOLD_AT_ONCE).rev() {
                    let words = self.words.with("received-notice", &[("name", video.sender().to_owned())]);
                    let detail = format!("{} — {}", video.player, video.map_line());
                    let note = format!("{}  {}", self.words.length(video.duration as i64 * 1000), self.words.mb(video.size));
                    self.announce(notices::Mark::Plain, words, detail, note, video.map_hash.clone(), notices::Link::Received(video.id));
                }
                if let Some(latest) = news.iter().map(|video| video.sent_at).max() {
                    self.settings.inbox_heard = latest;
                    let _ = self.settings.save();
                }
                let faces: Vec<String> = self.sharing.videos.iter().filter_map(|video| video.from.as_ref()).map(|face| face.avatar.clone()).collect();
                Task::batch([self.inbox_thumbs_task(), self.faces_task(faces)])
            }
            Message::Thumb(id, handle) => {
                if let Some(handle) = handle {
                    self.sharing.thumbs.insert(id, handle);
                }
                Task::none()
            }
            Message::Open(id) => {
                let Some(got) = self.sharing.of(id).cloned() else {
                    return Task::none();
                };
                let mut tasks = Vec::new();
                if self.overlay != Overlay::Videos {
                    tasks.push(self.update(Outer::Show(Overlay::Videos)));
                }
                self.sharing.tab = Tab::Received;
                if self.sharing.open == Some(id) && !self.leaving_player && !self.mini_player {
                    return Task::batch(tasks);
                }
                if let Some(old) = self.player.take() {
                    old.borrow_mut().close();
                }
                if let Some(getting) = self.sharing.getting.as_ref().filter(|getting| getting.id != id) {
                    getting.stop.store(true, Ordering::SeqCst);
                }
                let now = Instant::now();
                self.open_video = None;
                self.clip = None;
                self.asking_delete = false;
                self.leaving_player = false;
                self.mini_player = false;
                self.sharing.open = Some(id);
                self.stage_open = Animation::new(false).duration(STAGE_OPEN).easing(Easing::EaseOutCubic).go(true, now);
                self.cinema.go_mut(true, now);
                if !got.seen {
                    if let Some(video) = self.sharing.videos.iter_mut().find(|video| video.id == id) {
                        video.seen = true;
                    }
                    let (server, token, name) = self.bot_keys();
                    tasks.push(ui::in_thread(move || {
                        let _ = bot::inbox_seen(&server, &token, &name, id);
                        Outer::Sharing(Message::Quiet)
                    }));
                }
                if let Some(path) = got.cached() {
                    tasks.push(self.probe_received(id, path));
                }
                Task::batch(tasks)
            }
            Message::Watch => {
                let Some(got) = self.sharing.opened().cloned() else {
                    return Task::none();
                };
                if self.sharing.getting.is_some() || self.player.is_some() {
                    return Task::none();
                }
                if let Some(path) = got.cached() {
                    return self.probe_received(got.id, path);
                }
                let stop = Arc::new(AtomicBool::new(false));
                self.sharing.getting = Some(Getting { id: got.id, done: 0, total: got.size, stop: stop.clone() });
                let (server, token, name) = self.bot_keys();
                let ffmpeg = self.ffmpeg.clone();
                let id = got.id;
                ui::streamed(move |push| {
                    let into = inbox::video_path(id);
                    let mut last = Instant::now();
                    let fetched = bot::inbox_video(&server, &token, &name, id, &into, |done, total| {
                        if last.elapsed() > Duration::from_millis(80) {
                            last = Instant::now();
                            if !told(push, Message::Getting(id, done, total)) {
                                return false;
                            }
                        }
                        !stop.load(Ordering::SeqCst)
                    });
                    let outcome = match fetched {
                        Err(_) if stop.load(Ordering::SeqCst) => Err(STOPPED.to_owned()),
                        Err(why) => Err(why.to_string()),
                        Ok(_) => ffmpeg
                            .ok_or_else(|| "no ffmpeg".to_owned())
                            .and_then(|ffmpeg| videos::probe(&ffmpeg, &into).ok_or_else(|| "the video does not read".to_owned()))
                            .map(|media| (into, media)),
                    };
                    told(push, Message::Got(id, outcome));
                })
            }
            Message::Getting(id, done, total) => {
                if let Some(getting) = self.sharing.getting.as_mut().filter(|getting| getting.id == id) {
                    getting.done = done;
                    if total > 0 {
                        getting.total = total;
                    }
                }
                Task::none()
            }
            Message::Got(id, outcome) => {
                if self.sharing.getting.as_ref().is_some_and(|getting| getting.id == id) {
                    self.sharing.getting = None;
                }
                match outcome {
                    Ok((path, media)) => {
                        if self.sharing.open == Some(id) && self.overlay == Overlay::Videos && !self.leaving_player {
                            self.play_received(path, media);
                        }
                    }
                    Err(why) if why == STOPPED => {}
                    Err(why) => {
                        let (detail, hash) = self.sharing.of(id).map(|got| (format!("{} — {}", got.player, got.map_line()), got.map_hash.clone())).unwrap_or_default();
                        self.announce(notices::Mark::Bad, self.words.t("received-failed"), detail, why, hash, notices::Link::Received(id));
                    }
                }
                Task::none()
            }
            Message::StopGetting => {
                if let Some(getting) = &self.sharing.getting {
                    getting.stop.store(true, Ordering::SeqCst);
                }
                Task::none()
            }
            Message::ToTelegram => {
                let Some(id) = self.sharing.open else {
                    return Task::none();
                };
                if !self.sharing.passing.insert(id) {
                    return Task::none();
                }
                let (server, token, name) = self.bot_keys();
                ui::in_thread(move || Outer::Sharing(Message::Passed(id, bot::inbox_telegram(&server, &token, &name, id).map_err(|e| e.to_string()))))
            }
            Message::Passed(id, outcome) => {
                self.sharing.passing.remove(&id);
                match outcome {
                    Ok(()) => {
                        self.sharing.passed.insert(id);
                        let words = self.words.t("received-passed");
                        self.say(words);
                    }
                    Err(why) => {
                        let (detail, hash) = self.sharing.of(id).map(|got| (format!("{} — {}", got.player, got.map_line()), got.map_hash.clone())).unwrap_or_default();
                        self.announce(notices::Mark::Bad, self.words.t("send-failed"), detail, why, hash, notices::Link::Received(id));
                    }
                }
                Task::none()
            }
            Message::Draw => {
                let Some(got) = self.sharing.opened().cloned() else {
                    return Task::none();
                };
                if !got.replay || self.sharing.drawing.is_some() {
                    return Task::none();
                }
                self.sharing.drawing = Some(got.id);
                let (server, token, name) = self.bot_keys();
                let sources = self.settings.sources.clone();
                let file = got.replay_name();
                let id = got.id;
                ui::in_thread(move || {
                    let drawn = bot::inbox_replay(&server, &token, &name, id).map_err(|e| e.to_string()).and_then(|bytes| {
                        let into = crate::sources::own_root().join("Replays");
                        std::fs::create_dir_all(&into).map_err(|e| e.to_string())?;
                        let path = into.join(file);
                        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
                        let mut sources = sources;
                        if !sources.iter().any(|s| s.kind == Kind::Own) {
                            if let Ok(own) = crate::sources::own() {
                                sources.push(own);
                            }
                        }
                        Ok((path, Box::new(library::read(&sources))))
                    });
                    Outer::Sharing(Message::Drawn(id, drawn))
                })
            }
            Message::Drawn(id, outcome) => {
                self.sharing.drawing = None;
                let (path, library) = match outcome {
                    Ok(found) => found,
                    Err(why) => {
                        self.announce(notices::Mark::Bad, self.words.t("received-no-replay"), String::new(), why, String::new(), notices::Link::Received(id));
                        return Task::none();
                    }
                };
                if let Some(look) = self.sharing.of(id).and_then(|got| got.settings.clone()) {
                    self.sharing.looks.insert(path.clone(), look);
                }
                if !self.settings.sources.iter().any(|s| s.kind == Kind::Own) {
                    if let Ok(own) = crate::sources::own() {
                        self.settings.sources.push(own);
                        let _ = self.settings.save();
                    }
                }
                self.shut_received();
                let loaded = self.update(Outer::Loaded(*library));
                let shown = self.update(Outer::Show(Overlay::None));
                let Some(at) = self.entries().iter().position(|entry| entry.path == path) else {
                    self.announce(notices::Mark::Bad, self.words.t("received-no-replay"), String::new(), String::new(), String::new(), notices::Link::Received(id));
                    return Task::batch([loaded, shown]);
                };
                self.search.clear();
                let chosen = self.choose(at);
                let entry = &self.entries()[at];
                let next = match entry.map.is_some() {
                    true => self.update(Outer::Render),
                    false => {
                        self.sharing.draw_wanted = Some(entry.map_hash.clone());
                        self.update(Outer::GetMap)
                    }
                };
                Task::batch([loaded, shown, chosen, next])
            }
            Message::Remove => {
                let Some(id) = self.sharing.open else {
                    return Task::none();
                };
                self.shut_received();
                self.sharing.videos.retain(|video| video.id != id);
                self.sharing.thumbs.remove(&id);
                let (server, token, name) = self.bot_keys();
                ui::in_thread(move || {
                    let _ = std::fs::remove_file(inbox::video_path(id));
                    let _ = bot::inbox_drop(&server, &token, &name, id);
                    Outer::Sharing(Message::Quiet)
                })
            }
            Message::Pick => {
                if !self.signed_in() {
                    return self.update(Outer::SignIn);
                }
                if !self.has_telegram() {
                    return self.update(Outer::LinkTelegram);
                }
                let Some(video) = self.open_video.and_then(|at| self.store.videos.get(at)) else {
                    return Task::none();
                };
                let path = video.path.clone();
                if let Some(player) = &self.player {
                    let mut player = player.borrow_mut();
                    if !player.paused {
                        player.toggle();
                    }
                }
                for face in self.sharing.receivers.iter().flatten() {
                    let key = format!("to-{}", face.player);
                    self.marks.remove(&key);
                    self.marks_now.remove(&key);
                }
                self.sharing.picker = Some(Picker { path, picked: Vec::new(), busy: false });
                self.sharing.receivers_failed = None;
                let (server, token, name) = self.bot_keys();
                ui::in_thread(move || Outer::Sharing(Message::Receivers(bot::receivers(&server, &token, &name).map_err(|e| e.to_string()))))
            }
            Message::Receivers(Ok(people)) => {
                let faces: Vec<String> = people.iter().map(|face| face.avatar.clone()).collect();
                if let Some(picker) = &mut self.sharing.picker {
                    picker.picked.retain(|player| people.iter().any(|face| face.player == *player));
                }
                self.sharing.receivers = Some(people);
                self.sharing.receivers_failed = None;
                self.faces_task(faces)
            }
            Message::Receivers(Err(why)) => {
                self.sharing.receivers = None;
                self.sharing.receivers_failed = Some(why);
                Task::none()
            }
            Message::Toggle(player) => {
                let Some(picker) = self.sharing.picker.as_mut().filter(|picker| !picker.busy) else {
                    return Task::none();
                };
                let on = !picker.picked.contains(&player);
                match on {
                    true => picker.picked.push(player),
                    false => picker.picked.retain(|one| *one != player),
                }
                self.remember_mark(&format!("to-{player}"), on);
                Task::none()
            }
            Message::Send => {
                let Some(picker) = self.sharing.picker.as_mut().filter(|picker| !picker.busy && !picker.picked.is_empty()) else {
                    return Task::none();
                };
                let Some(video) = self.store.videos.iter().find(|video| video.path == picker.path).cloned() else {
                    return Task::none();
                };
                picker.busy = true;
                let to = picker.picked.clone();
                if video.remote.is_none() {
                    self.sending = Some(Sending { path: video.path.clone(), done: 0, total: video.size.max(1), over: None });
                }
                let meta = self.send_meta(&video, None);
                let (server, token, name) = self.bot_keys();
                ui::streamed(move |push| {
                    let mut remote = video.remote;
                    let mut uploaded = false;
                    let outcome = loop {
                        let id = match remote {
                            Some(id) => id,
                            None => {
                                uploaded = true;
                                let (tx, rx) = std::sync::mpsc::channel::<u64>();
                                let (server, token, name, path, meta) = (server.clone(), token.clone(), name.clone(), video.path.clone(), meta.clone());
                                let worker = std::thread::spawn(move || {
                                    bot::send(&server, &token, &name, &path, &meta, move |done| {
                                        let _ = tx.send(done);
                                    })
                                });
                                let mut last = Instant::now();
                                for done in rx {
                                    if last.elapsed() > Duration::from_millis(80) {
                                        last = Instant::now();
                                        if !push(Outer::Sending(done)) {
                                            return;
                                        }
                                    }
                                }
                                match worker.join() {
                                    Ok(Ok(sent)) => match sent.video {
                                        Some(id) => {
                                            remote = Some(id);
                                            id
                                        }
                                        None => break Err("the bot did not keep the video".to_owned()),
                                    },
                                    Ok(Err(why)) => break Err(why.to_string()),
                                    Err(_) => break Err("sending stopped".to_owned()),
                                }
                            }
                        };
                        if let Ok(bytes) = std::fs::read(&video.replay) {
                            let _ = bot::video_replay(&server, &token, &name, id, bytes);
                        }
                        match bot::share(&server, &token, &name, id, &to) {
                            Ok(shared) => break Ok(shared),
                            Err(bot::Refused::NotThere) if !uploaded => remote = None,
                            Err(why) => break Err(why.to_string()),
                        }
                    };
                    told(push, Message::Shared(video.path.clone(), remote, uploaded, outcome));
                })
            }
            Message::Shared(path, remote, uploaded, outcome) => {
                if let Some(id) = remote {
                    self.store.remember_remote(&path, id);
                }
                if let Some(sending) = self.sending.as_mut().filter(|sending| sending.path == path && sending.over.is_none()) {
                    let bytes = sending.total;
                    sending.over = Some(outcome.as_ref().map(|_| 0).map_err(Clone::clone));
                    if uploaded && remote.is_some() {
                        self.store.mark_sent(&path, unix_now(), bytes);
                    }
                }
                if let Some(picker) = &mut self.sharing.picker {
                    picker.busy = false;
                }
                let (detail, hash) = self.store.videos.iter().find(|video| video.path == path).map(|video| self.told_of(video)).unwrap_or_default();
                let named = |players: &[i64], known: &Option<Vec<Face>>| -> String {
                    let names: Vec<String> = players.iter().filter_map(|player| known.iter().flatten().find(|face| face.player == *player)).map(|face| face.name.clone()).collect();
                    names.join(", ")
                };
                match outcome {
                    Ok(shared) => {
                        if !shared.sent.is_empty() {
                            let note = named(&shared.sent, &self.sharing.receivers);
                            self.announce(notices::Mark::Done, self.words.t("shared-notice"), detail.clone(), note, hash.clone(), notices::Link::None);
                        }
                        if !shared.refused.is_empty() {
                            let players: Vec<i64> = shared.refused.iter().map(|refusal| refusal.player).collect();
                            let why = match shared.refused[0].why.as_str() {
                                "full" => self.words.t("share-why-full"),
                                "limit" => self.words.t("share-why-limit"),
                                _ => self.words.t("share-why-closed"),
                            };
                            let note = format!("{}  {why}", named(&players, &self.sharing.receivers));
                            self.announce(notices::Mark::Bad, self.words.t("share-refused"), detail, note, hash, notices::Link::None);
                        }
                        self.sharing.picker = None;
                    }
                    Err(why) => self.announce(notices::Mark::Bad, self.words.t("send-failed"), detail, why, hash, notices::Link::None),
                }
                Task::none()
            }
            Message::Close => {
                if self.sharing.picker.as_ref().is_some_and(|picker| !picker.busy) {
                    self.sharing.picker = None;
                }
                Task::none()
            }
            Message::Accept(accept) => {
                let was = self.sharing.accept;
                if was == accept || self.settings.token.is_empty() {
                    return Task::none();
                }
                self.remember_mark(&format!("accept-{}", was.tag()), false);
                self.remember_mark(&format!("accept-{}", accept.tag()), true);
                self.sharing.accept = accept;
                let (server, token, name) = self.bot_keys();
                ui::in_thread(move || Outer::Sharing(Message::Accepted(was, bot::accept(&server, &token, &name, accept.tag()).map_err(|e| e.to_string()))))
            }
            Message::Accepted(_, Ok(said)) => {
                self.accept_now(Accept::of(&said));
                Task::none()
            }
            Message::Replays(Ok(state)) => {
                mixed::allow(state.on);
                self.marks.remove("share-replays");
                self.marks_now.remove("share-replays");
                self.sharing.replays = Some(state);
                self.replays_give_task()
            }
            Message::Replays(Err(_)) => Task::none(),
            Message::ShareReplays(on) => {
                let Some(state) = self.sharing.replays.as_mut().filter(|state| state.on != on) else {
                    return Task::none();
                };
                state.on = on;
                mixed::allow(on);
                if !on {
                    state.count = 0;
                    mixed::forget_given();
                }
                self.remember_mark("share-replays", on);
                let (server, token, name) = self.bot_keys();
                ui::in_thread(move || Outer::Sharing(Message::ReplaysSwitched(!on, bot::replays_switch(&server, &token, &name, on).map_err(|e| e.to_string()))))
            }
            Message::ReplaysSwitched(_, Ok(state)) => {
                mixed::allow(state.on);
                self.sharing.replays = Some(state);
                self.replays_give_task()
            }
            Message::ReplaysSwitched(was, Err(_)) => {
                if let Some(state) = &mut self.sharing.replays {
                    state.on = was;
                }
                mixed::allow(was);
                self.remember_mark("share-replays", was);
                let words = self.words.t("accept-failed");
                self.say(words);
                Task::none()
            }
            Message::ReplaysGiven(sent) => match sent {
                0 => Task::none(),
                _ => self.replays_state_task(),
            },
            Message::ReplaysSynced(Err(_)) => Task::none(),
            Message::ReplaysSynced(Ok(change)) => {
                if let Some(source) = self.settings.sources.iter_mut().find(|source| source.is_shared()) {
                    let counted = crate::sources::shared(source.on).replay_count;
                    if source.replay_count != counted {
                        source.replay_count = counted;
                        let _ = self.settings.save();
                    }
                }
                if change == mixed::Change::default() || self.refreshing {
                    return Task::none();
                }
                self.refreshing = true;
                self.refreshed_at = Instant::now();
                read_library(self.settings.sources.clone(), Outer::Refreshed)
            }
            Message::Accepted(was, Err(_)) => {
                let now = self.sharing.accept;
                self.remember_mark(&format!("accept-{}", now.tag()), false);
                self.remember_mark(&format!("accept-{}", was.tag()), true);
                self.sharing.accept = was;
                let words = self.words.t("accept-failed");
                self.say(words);
                Task::none()
            }
        }
    }

    pub(super) fn videos_switch(&self) -> Element<'_, Outer> {
        let w = &self.words;
        let unseen = inbox::unseen(&self.sharing.videos);
        ui::switch(vec![
            (w.t("videos-mine"), None, self.sharing.tab == Tab::Mine, Outer::Sharing(Message::Tab(Tab::Mine))),
            (w.t("videos-received"), (unseen > 0).then(|| unseen.to_string()), self.sharing.tab == Tab::Received, Outer::Sharing(Message::Tab(Tab::Received))),
        ])
    }

    fn received_still(&self, got: &Received) -> Option<&image::Handle> {
        self.thumbs.get(&got.map_hash).or_else(|| self.sharing.thumbs.get(&got.id))
    }

    fn face<'a>(&'a self, face: Option<&Face>, side: f32) -> Element<'a, Outer> {
        let name = face.map_or("", |face| face.name.as_str());
        match face.and_then(|face| self.news_pictures.get(&face.avatar)) {
            Some(handle) => image(handle.clone()).content_fit(ContentFit::Cover).width(side).height(side).border_radius(side / 2.0).opacity(ui::fade()).into(),
            None => ui::disc(&first_letter(name), false, side),
        }
    }

    pub(super) fn received_empty(&self) -> Element<'_, Outer> {
        let w = &self.words;
        let (title, under) = match (self.sharing.loaded, self.sharing.registered) {
            (false, _) if self.sharing.failed => (w.t("received-unavailable"), String::new()),
            (false, _) => (w.t("asking-bot"), String::new()),
            (true, false) => (w.t("no-received"), w.t("share-not-registered")),
            (true, true) => (w.t("no-received"), w.t("no-received-how")),
        };
        let empty = column![text(title).font(theme::SANS_SEMI).size(theme::TITLE).color(ui::faded(INK)), ui::cap(under)].spacing(6).align_x(iced::Center);
        container(empty).width(Length::Fill).height(Length::Fill).center(Length::Fill).into()
    }

    pub(super) fn received_head(&self) -> Element<'_, Outer> {
        let w = &self.words;
        let cell = |key: &str, width: f32| container(ui::mono_small(w.t(key).to_uppercase(), FAINT)).width(width).align_x(iced::alignment::Horizontal::Center);
        let right = |key: &str, width: f32| container(ui::mono_small(w.t(key).to_uppercase(), FAINT)).width(width).align_x(iced::alignment::Horizontal::Right);
        container(
            row![
                Space::new().width(VIDEO_THUMB.0 as f32),
                cell("when", VIDEO_DATE_W),
                container(ui::mono_small(w.t("who-and-map").to_uppercase(), FAINT)).width(Length::Fill),
                container(ui::mono_small(w.t("from-whom").to_uppercase(), FAINT)).width(FROM_W),
                right("length", 56.0),
                right("size", 84.0),
            ]
            .spacing(14)
            .align_y(iced::Center),
        )
        .padding([0, 12])
        .height(24.0)
        .into()
    }

    pub(super) fn received_row<'a>(&'a self, got: &'a Received) -> Element<'a, Outer> {
        let w = &self.words;
        let chosen = self.sharing.open == Some(got.id);
        let picture: Element<'_, Outer> = match self.received_still(got) {
            Some(handle) => image(handle.clone())
                .content_fit(ContentFit::Cover)
                .width(VIDEO_THUMB.0 as f32)
                .height(VIDEO_THUMB.1 as f32)
                .border_radius(6.0)
                .opacity(ui::fade() * if chosen { 1.0 } else { 0.85 })
                .into(),
            None => container(ui::fine_hatch()).width(VIDEO_THUMB.0 as f32).height(VIDEO_THUMB.1 as f32).into(),
        };
        let stamp = w.compact_date(got.sent_at, self.now_unix);
        let (day, time) = stamp.rsplit_once(' ').unwrap_or((&stamp, ""));
        let when = column![ui::mono_small(day.to_owned(), if got.seen { MUTED } else { INK }), ui::mono_small(time.to_owned(), FAINT)]
            .spacing(2)
            .align_x(iced::alignment::Horizontal::Center);
        let mut named = row![].spacing(6).align_y(iced::Center);
        if !got.seen {
            named = named.push(ui::dot(7.0));
        }
        named = named.push(text(got.player.clone()).font(theme::SANS_SEMI).size(theme::LEAD).wrapping(text::Wrapping::None).color(ui::faded(INK)));
        for acronym in &got.mods {
            named = named.push(mod_badge(acronym));
        }
        let who = column![named, text(ui::shortened(got.map_line(), 60)).font(theme::SANS).size(theme::BODY).wrapping(text::Wrapping::None).color(ui::faded(MUTED))].spacing(2);
        let from = row![
            self.face(got.from.as_ref(), 24.0),
            text(ui::shortened(got.sender().to_owned(), 16)).font(theme::SANS_SEMI).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(INK)),
        ]
        .spacing(8)
        .align_y(iced::Center);
        let size = match self.sharing.getting.as_ref().filter(|getting| getting.id == got.id) {
            Some(getting) => ui::mono(format!("{} %", (getting.done * 100 / getting.total.max(1)).min(100)), INK),
            None => ui::mono(w.mb(got.size), MUTED),
        };
        let line = row![
            container(picture).width(VIDEO_THUMB.0 as f32).height(VIDEO_THUMB.1 as f32),
            container(when).width(VIDEO_DATE_W).align_x(iced::alignment::Horizontal::Center),
            container(who).width(Length::Fill).clip(true),
            container(from).width(FROM_W).clip(true),
            container(ui::mono(w.length(got.duration as i64 * 1000), MUTED)).width(56.0).align_x(iced::alignment::Horizontal::Right),
            container(size).width(84.0).align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(14)
        .align_y(iced::Center);
        button(container(line).height(VIDEO_ROW).width(Length::Fill).center_y(VIDEO_ROW))
            .padding([0, 12])
            .style(ui::button_faded(theme::row(chosen)))
            .on_press(Outer::Sharing(Message::Open(got.id)))
            .into()
    }

    fn received_buttons<'a>(&'a self, got: &'a Received) -> Element<'a, Outer> {
        let w = &self.words;
        let mut buttons = row![ui::grow()].spacing(6).align_y(iced::Center);
        if self.player.is_some() {
            buttons = buttons.push(ui::springy(ui::quiet(w.t("in-folder"), Some(Outer::RevealVideo)), 0.04));
        }
        buttons = buttons.push(ui::springy(ui::quiet(w.t("received-remove"), Some(Outer::Sharing(Message::Remove))), 0.04));
        if got.replay {
            let free = self.sharing.drawing.is_none();
            buttons = buttons.push(ui::springy(ui::quiet(w.t("received-draw"), free.then_some(Outer::Sharing(Message::Draw))), 0.04));
        }
        if !self.has_telegram() {
            return buttons.into();
        }
        let telegram = match (self.sharing.passed.contains(&got.id), self.sharing.passing.contains(&got.id)) {
            (true, _) => ui::primary(w.t("received-passed"), None),
            (false, true) => ui::primary(w.t("received-telegram"), None),
            (false, false) => ui::primary(w.t("received-telegram"), Some(Outer::Sharing(Message::ToTelegram))),
        };
        buttons.push(ui::springy(telegram, 0.03)).into()
    }

    fn sender_aside<'a>(&'a self, got: &'a Received) -> Element<'a, Outer> {
        row![
            self.face(got.from.as_ref(), 26.0),
            text(got.sender().to_owned()).font(theme::SANS_SEMI).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(MUTED)),
        ]
        .spacing(8)
        .align_y(iced::Center)
        .into()
    }

    pub(super) fn received_bill<'a>(&'a self, got: &'a Received) -> Bill<'a> {
        Bill {
            still: self.received_still(got),
            name: got.player.clone(),
            mods: &got.mods,
            line: got.map_line(),
            buttons: self.received_buttons(got),
            earlier: None,
            later: None,
            aside: Some(self.sender_aside(got)),
        }
    }

    pub(super) fn received_stage<'a>(&'a self, got: &'a Received) -> Element<'a, Outer> {
        let w = &self.words;
        let round: iced::border::Radius = PICTURE_RADIUS.into();
        let room_w = (self.width - 2.0 * STAGE_GAP - 2.0 * PICTURE_INSET).max(320.0);
        let room_h = (self.height - 2.0 * STAGE_GAP - STAGE_TOP - STAGE_UNDER - 2.0 * PICTURE_INSET).max(180.0);
        let screen_h = (room_w * 9.0 / 16.0).min(room_h).floor();
        let screen_w = (screen_h * 16.0 / 9.0).min(room_w).floor();
        let picture: Element<'_, Outer> = match self.received_still(got) {
            Some(handle) => image(handle.clone()).content_fit(ContentFit::Cover).width(Length::Fill).height(Length::Fill).opacity(0.35 * ui::fade()).border_radius(round).into(),
            None => Space::new().width(Length::Fill).height(Length::Fill).into(),
        };
        let getting = self.sharing.getting.as_ref().filter(|getting| getting.id == got.id);
        let middle: Element<'_, Outer> = match getting {
            Some(getting) => {
                let part = (getting.done as f32 / getting.total.max(1) as f32).clamp(0.0, 1.0);
                column![
                    ui::progress(w.t("received-getting"), part, None),
                    ui::mono_small(w.mb_of(getting.done, getting.total), MUTED),
                    container(ui::quiet(w.t("received-stop"), Some(Outer::Sharing(Message::StopGetting)))).padding(Padding::ZERO.top(6.0)),
                ]
                .spacing(8)
                .align_x(iced::Center)
                .into()
            }
            None => ui::halo(ui::Control::Play, 72.0, 1.0),
        };
        let inside = stack![picture, container(middle).width(Length::Fill).height(Length::Fill).center(Length::Fill)].width(screen_w).height(screen_h);
        let screen = container(inside).width(screen_w).height(screen_h).style(ui::box_faded(theme::screen)).clip(true);
        let screen: Element<'_, Outer> = match getting {
            Some(_) => screen.into(),
            None => mouse_area(screen).interaction(iced::mouse::Interaction::Pointer).on_press(Outer::Sharing(Message::Watch)).into(),
        };
        let mut named = row![text(got.player.clone()).font(theme::SANS_SEMI).size(theme::LEAD).wrapping(text::Wrapping::None).color(ui::faded(INK))].spacing(6).align_y(iced::Center);
        for acronym in &got.mods {
            named = named.push(mod_badge(acronym));
        }
        let title = row![
            column![named, text(ui::shortened(got.map_line(), 62)).font(theme::SANS).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(MUTED))].spacing(2),
            ui::grow(),
            self.sender_aside(got),
            ui::control_button(ui::Control::Close, 18.0, Some(Outer::ClosePlayer), false),
        ]
        .spacing(12)
        .align_y(iced::Center);
        let inside = column![
            container(title)
                .height(STAGE_TOP)
                .align_y(iced::alignment::Vertical::Center)
                .padding(Padding { top: 2.0, right: PICTURE_INSET - 7.0, bottom: 0.0, left: PICTURE_INSET }),
            container(screen).width(Length::Fill).height(screen_h).center_x(Length::Fill),
            container(self.received_buttons(got))
                .padding(Padding { top: 0.0, right: PICTURE_INSET, bottom: 0.0, left: PICTURE_INSET })
                .height(STAGE_UNDER)
                .align_y(iced::alignment::Vertical::Center)
                .clip(true),
        ]
        .width(Length::Fill);
        let card = container(inside).width(screen_w + 2.0 * PICTURE_INSET).height(screen_h + STAGE_TOP + STAGE_UNDER);
        let backdrop = mouse_area(ui::veil(theme::CINEMA_SCRIM)).on_press(Outer::ClosePlayer);
        let opened = self.stage_open.interpolate(0.0, 1.0, self.now);
        let card = ui::grown(card, iced::Point::new(0.5, 0.5), -(1.0 - opened) * 14.0, 0.965 + 0.035 * opened);
        stack![backdrop, container(card).width(Length::Fill).height(Length::Fill).center(Length::Fill)].width(Length::Fill).height(Length::Fill).into()
    }

    pub(super) fn share_layer(&self) -> Element<'_, Outer> {
        let Some(picker) = &self.sharing.picker else {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        };
        let w = &self.words;
        let about = self.store.videos.iter().find(|video| video.path == picker.path).map(|video| format!("{} — {}", video.player, video.map_line())).unwrap_or_default();
        let mut top = column![ui::title(w.t("share-title")), ui::why(ui::shortened(about, 64))].spacing(6).width(Length::Fill);
        match (&self.sharing.receivers, &self.sharing.receivers_failed) {
            (_, Some(why)) => {
                let said = if why == "not there" { w.t("share-not-registered") } else { w.t("share-failed-list") };
                top = top.push(container(ui::cap(said)).padding(Padding::ZERO.top(12.0)));
            }
            (None, None) => {
                top = top.push(container(ui::mono_small(w.t("asking-bot"), MUTED)).padding(Padding::ZERO.top(12.0)));
            }
            (Some(people), None) if people.is_empty() => {
                top = top.push(container(column![ui::body(w.t("share-nobody"), INK), ui::cap(w.t("share-nobody-how"))].spacing(4)).padding(Padding::ZERO.top(12.0)));
            }
            (Some(people), None) => {
                let mut rows = column![].spacing(2).width(Length::Fill);
                for face in people {
                    let on = picker.picked.contains(&face.player);
                    let k = self.marks_now.get(&format!("to-{}", face.player)).copied().unwrap_or(if on { 1.0 } else { 0.0 });
                    let under = if face.shared { w.t("share-shared-chat") } else { w.t("share-on-server") };
                    let words = column![
                        text(face.name.clone()).font(theme::SANS_SEMI).size(theme::BODY).wrapping(text::Wrapping::None).color(ui::faded(INK)),
                        text(under).font(theme::SANS).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(MUTED)),
                    ]
                    .spacing(1);
                    let line = row![self.face(Some(face), FACE), words, ui::grow(), ui::mark("", on, k, 22.0)].spacing(10).align_y(iced::Center);
                    rows = rows.push(
                        button(container(line).padding([0, 8]).center_y(46.0))
                            .width(Length::Fill)
                            .padding(0)
                            .style(ui::button_faded(theme::row(false)))
                            .on_press(Outer::Sharing(Message::Toggle(face.player))),
                    );
                }
                let list = scrollable(rows).direction(ui::hidden_bar()).width(Length::Fill);
                top = top.push(container(list).max_height(LIST_HIGH).padding(Padding::ZERO.top(12.0)));
            }
        }
        let ready = !picker.busy && !picker.picked.is_empty() && self.sharing.receivers.is_some();
        let send: Element<'_, Outer> = match self.sending.as_ref().filter(|sending| picker.busy && sending.path == picker.path && sending.over.is_none()) {
            Some(_) => ui::progress(w.t("sending"), self.progress_shown, None),
            None => ui::primary(w.t("share-send"), ready.then_some(Outer::Sharing(Message::Send))),
        };
        let bottom = row![send, ui::grow(), ui::quiet(w.t("share-cancel"), (!picker.busy).then_some(Outer::Sharing(Message::Close)))].spacing(4).align_y(iced::Center);
        let card = ui::sheet(top.into(), Some(bottom.into()));
        stack![
            mouse_area(ui::veil(theme::SCRIM)).on_press(Outer::Sharing(Message::Close)),
            container(container(card).width(theme::COLUMN).padding(Padding::ZERO.top(150.0))).width(Length::Fill).height(Length::Fill).center_x(Length::Fill),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
