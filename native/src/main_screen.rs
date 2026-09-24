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
use crate::settings_screen::{self as prefs, Side};
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
const LIVE_FIRST: usize = 6;
const LIVE_EVERY: Duration = Duration::from_secs(6);
const LIVE_ARRIVE: Duration = Duration::from_millis(420);
const STAGE_SHOW: Duration = Duration::from_millis(340);
const COMMUNITY_EVERY: Duration = Duration::from_secs(60);
const FRIENDS_EVERY: Duration = Duration::from_secs(120);
const CARD_EVERY: Duration = Duration::from_secs(300);
const COMMUNITY_SCALE: f32 = 0.84;
const SHARED_FRESH: i64 = 3 * 3600;
const HOVER_REST: Duration = Duration::from_millis(160);
pub const LIVE_FADE: Duration = Duration::from_millis(640);
pub const BRAND_WIDTH: f32 = 144.0;
const CREST_HOME: (f32, f32) = (40.0, 24.0);
const CREST_RISE: f32 = 8.0;
const JOURNAL_RISE: f32 = 140.0;
const BUBBLE_W: f32 = 340.0;
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
    Community(crate::community_screen::Message),
    NewsBuilds(Result<Vec<crate::news::Build>, String>),
    NewsStories(Result<Vec<crate::news::Story>, String>),
    NewsThreads(Result<Vec<crate::news::Thread>, String>),
    NewsPosts(String, Result<Vec<crate::news::Post>, String>),
    NewsPicture(String, Option<image::Handle>),
    LiveArrive,
    CommunityArrived(Result<crate::community::wire::Community, String>),
    FriendsArrived(Result<crate::bot::Friends, String>),
    CardArrived(Result<crate::community::wire::Card, String>),
    Flag(String, Option<Vec<u8>>),
    CommunityTick,
    FeedClock,
    ClipFetched(String, Result<(PathBuf, i64, u32), String>),
    OsuProfile(Result<crate::community::wire::Card, String>),
    PersonCard(String, Result<crate::community::wire::Card, String>),
    PersonDossier(i64, Result<crate::community::wire::Me, String>),
    CardShared(Result<(), String>),
    ReadFirst,
    Loaded(Library),
    Thumb(String, image::Handle),
    Scene(String, Option<image::Handle>),
    Length(PathBuf, i64),
    MaxCombo(PathBuf, Option<u32>),
    Key(iced::keyboard::key::Named),
    Prefs(prefs::Message),
    Retype(Instant),
    Chats(Vec<crate::bot::Chat>),
    Skins(Vec<PathBuf>),
    SkinFace(PathBuf, Option<image::Handle>),
    SkinScene(PathBuf, Option<image::Handle>),
    ShowSkins(bool),
    ChatFace(i64, Option<image::Handle>),
    Sized(u64, u64, u64),
    Stored(prefs::Storage),
    Ffmpeg(Option<String>),
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
    Scrubbing(f32),
    StepFrames(i64),
    PlayerLevel(f32),
    PlayerLouder(f32),
    PlayerMute,
    PlayerRate(i32),
    PlayerSpeed(f32),
    PlayerLoop,
    PlayerWiden,
    PlayerStir(iced::Point),
    ControlsHover(bool),
    PlayerNeighbour(i32),
    Typed(char),
    RevealVideo,
    AskDelete,
    AskDeleteOf(usize),
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

struct Bill<'a> {
    still: Option<&'a image::Handle>,
    name: String,
    mods: &'a [String],
    line: String,
    buttons: Element<'a, Message>,
    earlier: Option<Message>,
    later: Option<Message>,
}

#[derive(Debug, Clone)]
pub struct Clip {
    pub path: PathBuf,
    pub link: String,
    pub from: String,
    pub said: String,
    pub thumb: Option<String>,
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
    pub frame: Option<crate::film::Frame>,
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
    pub hover_since: Option<(usize, Instant)>,
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
    pub skin_room: bool,
    pub room_fade: Animation<bool>,
    pub skin_scenes: HashMap<PathBuf, image::Handle>,
    pub cinema: Animation<bool>,
    pub widened: Animation<bool>,
    pub scrubbing: Option<f32>,
    pub controls: Animation<bool>,
    pub ask_fade: Animation<bool>,
    pub stage_open: Animation<bool>,
    pub leaving_player: bool,
    pub pointer: Option<iced::Point>,
    pub over_controls: bool,
    pub stirred: Instant,
    pub hint: Option<(String, Instant)>,
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
    pub ground_fade: Animation<bool>,
    pub overlay_drawn: Overlay,
    pub side: Side,
    pub renaming: Option<String>,
    pub chats: Vec<crate::bot::Chat>,
    pub skins: Vec<PathBuf>,
    pub skin_faces: HashMap<PathBuf, image::Handle>,
    pub chat_faces: HashMap<i64, image::Handle>,
    pub marks: HashMap<String, Animation<bool>>,
    pub marks_now: HashMap<String, f32>,
    pub slides: HashMap<String, (f32, f32)>,
    pub slid_at: HashMap<String, Instant>,
    pub lang_swap: bool,
    pub paused_by_hand: bool,
    pub turning: Option<Overlay>,
    pub side_fade: Animation<bool>,
    pub side_swap: f32,
    pub sizes: (u64, u64, u64),
    pub storage: prefs::Storage,
    pub ffmpeg_version: Option<String>,
    pub retype: Animation<bool>,
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
    strip_aim: Option<(u64, f32)>,
    pub community: Option<crate::community::Catalog>,
    pub community_section: crate::community_screen::Section,
    pub community_board: crate::community::Board,
    pub community_person: Option<usize>,
    pub person_fade: Animation<bool>,
    pub people_cards: HashMap<String, crate::community::wire::Card>,
    pub people_dossiers: HashMap<i64, crate::community::Me>,
    people_asked: std::collections::HashSet<String>,
    pub news: crate::news::News,
    news_loaded: bool,
    pub news_pictures: HashMap<String, image::Handle>,
    news_asked: std::collections::HashSet<String>,
    pub news_loading: std::collections::HashSet<String>,
    pub news_failed: std::collections::HashSet<String>,
    pub channel_draft: String,
    pub live_shown: usize,
    live_arrived: Option<Instant>,
    pub community_reading: Option<crate::community_screen::Reading>,
    pub read_fade: Animation<bool>,
    pub people_from: crate::community_screen::PeopleFrom,
    pub community_standing: crate::community_screen::Standing,
    pub community_card: Option<crate::community::wire::Card>,
    pub shown_card: Option<crate::community::wire::Card>,
    pub flags: HashMap<String, iced::widget::svg::Handle>,
    flags_asked: std::collections::HashSet<String>,
    card_asked: Option<Instant>,
    pub feed_filter: crate::chronicle::Filter,
    pub feed_source: crate::chronicle::Source,
    pub feed_stream: crate::chronicle::Stream,
    pub feed_query: String,
    pub feed_open: std::collections::HashSet<String>,
    pub feed_seen: i64,
    pub spot: usize,
    spot_at: Instant,
    pub(crate) section_at: Instant,
    pub(crate) shift_at: Instant,
    pub(crate) person_at: Instant,
    pub(crate) play_at: Instant,
    pub(crate) spot_due: Instant,
    pub(crate) spot_held: Option<Instant>,
    pub rank: usize,
    rank_at: Instant,
    pub(crate) rank_due: Instant,
    pub(crate) rank_held: Option<Instant>,
    pub dossier_metric: crate::dossier::Metric,
    pub dossier_span: u32,
    pub grade_hover: Option<usize>,
    pub title_pick: Option<String>,
    pub play_open: Option<usize>,
    clips_loading: std::collections::HashSet<String>,
    pub clip: Option<Clip>,
    pub osu_card: Option<crate::community::wire::Card>,
    osu_asked: Option<Instant>,
    pub community_fetch: crate::community_screen::Fetch,
    community_asked: Option<Instant>,
    friends_asked: Option<Instant>,
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
            hover_since: None,
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
            skin_room: false,
            room_fade: Animation::new(false).duration(CINEMA).easing(Easing::EaseOutCubic),
            skin_scenes: HashMap::new(),
            cinema: Animation::new(false).duration(CINEMA).easing(Easing::EaseOutCubic),
            widened: Animation::new(false).duration(WIDEN).easing(Easing::EaseOutCubic),
            scrubbing: None,
            controls: Animation::new(true).duration(CONTROLS_FADE).easing(Easing::EaseOutCubic),
            ask_fade: Animation::new(false).duration(CINEMA).easing(Easing::EaseOutCubic),
            stage_open: Animation::new(false).duration(STAGE_OPEN).easing(Easing::EaseOutCubic),
            leaving_player: false,
            pointer: None,
            over_controls: false,
            stirred: Instant::now(),
            hint: None,
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
            ground_fade: Animation::new(false).duration(GROUND_UP).easing(Easing::EaseOutCubic),
            overlay_drawn: Overlay::None,
            side: Side::App,
            renaming: None,
            chats: Vec::new(),
            skins: Vec::new(),
            skin_faces: HashMap::new(),
            chat_faces: HashMap::new(),
            marks: HashMap::new(),
            marks_now: HashMap::new(),
            slides: HashMap::new(),
            slid_at: HashMap::new(),
            lang_swap: false,
            paused_by_hand: false,
            turning: None,
            side_fade: Animation::new(true),
            side_swap: 0.0,
            sizes: (0, 0, 0),
            storage: prefs::Storage::default(),
            ffmpeg_version: None,
            retype: Animation::new(true),
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
            strip_aim: None,
            community: None,
            community_section: crate::community_screen::Section::Feed,
            community_board: crate::community::Board::Pp,
            community_person: None,
            person_fade: Animation::new(false),
            people_cards: HashMap::new(),
            people_dossiers: HashMap::new(),
            people_asked: std::collections::HashSet::new(),
            news: crate::news::News::default(),
            news_loaded: false,
            news_pictures: HashMap::new(),
            news_asked: std::collections::HashSet::new(),
            news_loading: std::collections::HashSet::new(),
            news_failed: std::collections::HashSet::new(),
            channel_draft: String::new(),
            live_shown: LIVE_FIRST,
            live_arrived: None,
            community_reading: None,
            read_fade: Animation::new(false),
            people_from: crate::community_screen::PeopleFrom::Chat,
            community_standing: crate::community_screen::Standing::General,
            community_card: None,
            shown_card: None,
            flags: HashMap::new(),
            flags_asked: std::collections::HashSet::new(),
            card_asked: None,
            feed_filter: crate::chronicle::Filter::All,
            feed_source: crate::chronicle::Source::All,
            feed_stream: crate::chronicle::Stream::All,
            feed_query: String::new(),
            feed_open: std::collections::HashSet::new(),
            feed_seen: 0,
            spot: 0,
            spot_at: Instant::now() - Duration::from_secs(3600),
            section_at: Instant::now() - Duration::from_secs(3600),
            shift_at: Instant::now() - Duration::from_secs(3600),
            person_at: Instant::now() - Duration::from_secs(3600),
            play_at: Instant::now() - Duration::from_secs(3600),
            spot_due: Instant::now(),
            spot_held: None,
            rank: 0,
            rank_at: Instant::now() - Duration::from_secs(3600),
            rank_due: Instant::now(),
            rank_held: None,
            dossier_metric: crate::dossier::Metric::Rank,
            dossier_span: 90,
            grade_hover: None,
            title_pick: None,
            play_open: None,
            clips_loading: std::collections::HashSet::new(),
            clip: None,
            osu_card: None,
            osu_asked: None,
            community_fetch: crate::community_screen::Fetch::Staged,
            community_asked: None,
            friends_asked: None,
        };
        let strays = videos::strays(&made.store.videos, &made.settings.renders_dir());
        let adopt = match (made.ffmpeg.clone(), strays.is_empty()) {
            (Some(ffmpeg), false) => ui::in_thread(move || Message::Adopted(strays.iter().filter_map(|p| videos::adopt(&ffmpeg, p)).collect())),
            _ => Task::none(),
        };
        let who = made.ask_who();
        let warm = if made.settings.token.is_empty() { Task::none() } else { warm_pictures() };
        (made, Task::batch([ui::in_thread(move || library::read(&sources)).map(Message::Loaded), adopt, who, warm]))
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
            || self.cinema.is_animating(self.now)
            || self.cinema.value() != (self.player.is_some() && !self.leaving_player)
            || self.stage_open.is_animating(self.now)
            || self.leaving_player
            || self.controls.is_animating(self.now)
            || self.ask_fade.is_animating(self.now)
            || self.ask_fade.value() != self.asking_delete
            || self.player.as_ref().is_some_and(|p| p.borrow().paused && !self.controls.value())
            || self.room_fade.is_animating(self.now)
            || self.widened.is_animating(self.now)
            || self.hint.is_some()
            || !self.toasts.is_empty()
            || self.menu_open.is_animating(self.now)
            || self.overlay_fade.is_animating(self.now)
            || self.ground_fade.is_animating(self.now)
            || self.turning.is_some()
            || self.retype.is_animating(self.now)
            || self.side_fade.is_animating(self.now)
            || self.marks.values().any(|m| m.is_animating(self.now))
            || self.slides_settling()
            || self.live_arrived.is_some_and(|at| self.now.saturating_duration_since(at) < LIVE_ARRIVE)
            || self.read_fade.is_animating(self.now)
            || self.person_fade.is_animating(self.now)
            || (self.overlay == Overlay::Community && [self.section_at, self.shift_at, self.person_at, self.play_at].iter().any(|at| self.now.saturating_duration_since(*at).as_secs_f32() < ui::APPEAR_ALL))
            || (self.overlay == Overlay::Community && self.now.saturating_duration_since(self.spot_at) < crate::chronicle::SPOT_SWAP)
            || (self.overlay == Overlay::Community && self.now.saturating_duration_since(self.rank_at) < crate::chronicle::RANK_GROW)
            || (self.community_reading.is_some() && !self.read_fade.value())
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
                    Key::Character(letter) => letter.chars().next().map(|c| Message::Typed(c.to_lowercase().next().unwrap_or(c))),
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
        if self.overlay == Overlay::Community && !self.settings.token.is_empty() {
            parts.push(iced::time::every(COMMUNITY_EVERY).map(|_| Message::CommunityTick));
        }
        if self.overlay == Overlay::Community && self.community_section == crate::community_screen::Section::Feed {
            parts.push(iced::time::every(Duration::from_millis(500)).map(|_| Message::FeedClock));
        }
        let live_waiting = self.community.as_ref().is_some_and(|catalog| self.live_shown < catalog.live.len());
        if self.overlay == Overlay::Community && self.community_section == crate::community_screen::Section::Feed && live_waiting {
            parts.push(iced::time::every(LIVE_EVERY).map(|_| Message::LiveArrive));
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
            self.live_before = live.frame.as_ref().map(crate::film::Frame::still);
        }
        self.trail = None;
        if !self.settings.live_scene {
            return Task::none();
        }
        let Some(ask) = self.chosen_entry().and_then(|entry| {
            let map = entry.map.as_ref()?;
            Some(live::Ask {
                replay: entry.path.clone(),
                map: map.file.clone(),
                map_hash: entry.map_hash.clone(),
                skin: self.settings.skin.clone(),
            })
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
                } else if self.skin_room {
                    return self.update(Message::ShowSkins(false));
                } else if self.asking_delete {
                    self.asking_delete = false;
                } else if self.player.is_some() {
                    if self.widened.value() {
                        return self.update(Message::PlayerWiden);
                    }
                    return self.update(Message::ClosePlayer);
                } else if self.overlay == Overlay::Community && self.community_reading.is_some() && self.read_fade.value() {
                    self.read_fade.go_mut(false, Instant::now());
                } else if self.overlay == Overlay::Community && self.community_person.is_some() && self.person_fade.value() {
                    self.person_fade.go_mut(false, Instant::now());
                } else if self.overlay == Overlay::Community {
                    self.feed_query.clear();
                    self.channel_draft.clear();
                    return iced::advanced::widget::operate(iced::advanced::widget::operation::focusable::unfocus::<Message>());
                } else if self.overlay != Overlay::None {
                    return self.update(Message::Show(Overlay::None));
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
                let opening = self.menu.is_none();
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
                let chats = if opening { self.chats_task() } else { Task::none() };
                if tab == Tab::Feed {
                    self.notices.see_all();
                    return Task::batch([self.scenes_for_notices(), chats]);
                }
                chats
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
                    "chat": self.settings.chat_id,
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
            Message::Sized(videos, maps, cache) => {
                self.sizes = (videos, maps, cache);
                Task::none()
            }
            Message::Stored(storage) => {
                self.storage = storage;
                Task::none()
            }
            Message::Ffmpeg(version) => {
                self.ffmpeg_version = version;
                Task::none()
            }
            Message::Skins(found) => {
                self.skins = found;
                self.skin_look()
            }
            Message::SkinFace(folder, handle) => {
                if let Some(handle) = handle {
                    self.skin_faces.insert(folder, handle);
                }
                Task::none()
            }
            Message::SkinScene(folder, handle) => {
                if let Some(handle) = handle {
                    self.skin_scenes.insert(folder, handle);
                }
                Task::none()
            }
            Message::ShowSkins(open) => {
                self.skin_room = open;
                self.room_fade.go_mut(open, Instant::now());
                match open {
                    true => self.skin_scenery(),
                    false => Task::none(),
                }
            }
            Message::Chats(chats) => {
                let wanted: Vec<i64> = chats.iter().filter(|chat| chat.photo && !self.chat_faces.contains_key(&chat.id)).map(|chat| chat.id).collect();
                self.chats = chats;
                let named = self.settings.chat_id.and_then(|id| self.chats.iter().find(|chat| chat.id == id)).map(|chat| chat.title.clone());
                if let Some(title) = named.filter(|title| *title != self.settings.chat_title) {
                    self.settings.chat_title = title;
                    let _ = self.settings.save();
                }
                if wanted.is_empty() {
                    return Task::none();
                }
                let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
                ui::streamed(move |push| {
                    for id in wanted {
                        let handle = crate::bot::chat_avatar(&server, &token, &name, id)
                            .ok()
                            .and_then(|bytes| decoded_bytes(&bytes, AVATAR_SIDE));
                        if !push(Message::ChatFace(id, handle)) {
                            return;
                        }
                    }
                })
            }
            Message::ChatFace(id, handle) => {
                if let Some(handle) = handle {
                    self.chat_faces.insert(id, handle);
                }
                Task::none()
            }
            Message::Retype(_) => Task::none(),
            Message::Prefs(inner) => self.prefs(inner),
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
                if self.overlay != Overlay::Videos {
                    let shown = self.update(Message::Show(Overlay::Videos));
                    return shown.chain(self.update(Message::OpenVideo(at)));
                }
                let Some(ffmpeg) = &self.ffmpeg else {
                    return Task::none();
                };
                let fresh = self.player.is_none() || self.leaving_player;
                if let Some(old) = self.player.take() {
                    old.borrow_mut().close();
                }
                self.leaving_player = false;
                if fresh {
                    self.stage_open = Animation::new(false).duration(STAGE_OPEN).easing(Easing::EaseOutCubic).go(true, Instant::now());
                }
                self.stirred = Instant::now();
                self.over_controls = false;
                self.controls = Animation::new(true).duration(CONTROLS_IN).easing(Easing::EaseOutCubic);
                let manner = player::Manner {
                    level: self.settings.player_level,
                    muted: self.settings.player_muted,
                    rate: self.settings.player_rate,
                };
                self.player = Some(std::rc::Rc::new(std::cell::RefCell::new(player::Player::open(ffmpeg, &video.path, video.length_ms, video.fps, manner))));
                self.open_video = Some(at);
                self.asking_delete = false;
                self.scrubbing = None;
                self.hint = None;
                self.cinema.go_mut(true, Instant::now());
                Task::none()
            }
            Message::ClosePlayer => {
                if self.player.is_none() || self.leaving_player {
                    return Task::none();
                }
                if let Some(player) = &self.player {
                    let mut player = player.borrow_mut();
                    if !player.paused {
                        player.toggle();
                    }
                }
                let now = Instant::now();
                self.leaving_player = true;
                self.stage_open.go_mut(false, now);
                self.cinema.go_mut(false, now);
                Task::none()
            }
            Message::PlayerToggle => {
                if let Some(player) = &self.player {
                    player.borrow_mut().toggle();
                }
                Task::none()
            }
            Message::SeekTo(fraction) => {
                self.scrubbing = None;
                if let Some(player) = &self.player {
                    let mut player = player.borrow_mut();
                    let to = (fraction as f64 * player.length_ms as f64) as i64;
                    player.seek(to);
                }
                Task::none()
            }
            Message::Scrubbing(fraction) => {
                self.scrubbing = Some(fraction.clamp(0.0, 1.0));
                Task::none()
            }
            Message::SeekBy(delta) => {
                if let Some(player) = &self.player {
                    player.borrow_mut().seek_by(delta);
                }
                let sign = if delta > 0 { "+" } else { "−" };
                self.say(format!("{sign}{} {}", delta.abs() / 1000, self.words.t("seconds-short")));
                Task::none()
            }
            Message::StepFrames(by) => {
                if let Some(player) = &self.player {
                    player.borrow_mut().step(by);
                }
                self.say(format!("{}{by}", if by > 0 { "+" } else { "" }));
                Task::none()
            }
            Message::PlayerLevel(level) => {
                let level = level.clamp(0.0, 1.0);
                self.settings.player_level = level;
                self.settings.player_muted = false;
                let _ = self.settings.save();
                if let Some(player) = &self.player {
                    let mut player = player.borrow_mut();
                    player.set_level(level);
                    player.set_muted(false);
                }
                self.say(format!("{} %", (level * 100.0).round() as i32));
                Task::none()
            }
            Message::PlayerLouder(by) => {
                let level = (self.settings.player_level + by.clamp(-1.0, 1.0) * 0.05).clamp(0.0, 1.0);
                self.update(Message::PlayerLevel(level))
            }
            Message::PlayerMute => {
                let muted = !self.settings.player_muted;
                self.settings.player_muted = muted;
                let _ = self.settings.save();
                if let Some(player) = &self.player {
                    player.borrow_mut().set_muted(muted);
                }
                self.say(self.words.t(if muted { "sound-off" } else { "sound-on" }));
                Task::none()
            }
            Message::PlayerRate(by) => {
                let rate = player::next_rate(self.settings.player_rate, by);
                self.settings.player_rate = rate;
                let _ = self.settings.save();
                if let Some(player) = &self.player {
                    player.borrow_mut().set_rate(rate);
                }
                self.say(format!("×{}", self.words.rate(rate)));
                Task::none()
            }
            Message::PlayerSpeed(rate) => {
                if (rate - self.settings.player_rate).abs() < 0.001 {
                    return Task::none();
                }
                self.settings.player_rate = rate;
                let _ = self.settings.save();
                if let Some(player) = &self.player {
                    player.borrow_mut().set_rate(rate);
                }
                self.say(format!("×{}", self.words.rate(rate)));
                Task::none()
            }
            Message::PlayerStir(at) => {
                let moved = self.pointer.is_none_or(|was| (was.x - at.x).abs() + (was.y - at.y).abs() > 4.0);
                self.pointer = Some(at);
                if !moved {
                    return Task::none();
                }
                let now = Instant::now();
                self.stirred = now;
                if !self.controls.value() {
                    self.show_controls(true, now);
                }
                Task::none()
            }
            Message::ControlsHover(over) => {
                self.over_controls = over;
                if over && !self.controls.value() {
                    self.show_controls(true, Instant::now());
                }
                Task::none()
            }
            Message::PlayerLoop => {
                let over = !self.settings.player_loop;
                self.settings.player_loop = over;
                let _ = self.settings.save();
                self.say(self.words.t(if over { "loop-on" } else { "loop-off" }));
                Task::none()
            }
            Message::PlayerWiden => {
                let now = Instant::now();
                let wide = !self.widened.value();
                self.widened.go_mut(wide, now);
                Task::none()
            }
            Message::PlayerNeighbour(by) => {
                let Some(at) = self.open_video else {
                    return Task::none();
                };
                let next = at as i32 + by;
                if next < 0 || next as usize >= self.store.videos.len() {
                    return Task::none();
                }
                self.update(Message::OpenVideo(next as usize))
            }
            Message::Typed(letter) => {
                if self.player.is_none() {
                    return Task::none();
                }
                match letter {
                    'm' | 'ь' => self.update(Message::PlayerMute),
                    'l' | 'д' => self.update(Message::PlayerLoop),
                    'f' | 'а' => self.update(Message::PlayerWiden),
                    'k' | 'л' => self.update(Message::PlayerToggle),
                    'j' | 'о' => self.update(Message::SeekBy(-10000)),
                    ',' | 'б' => self.update(Message::StepFrames(-1)),
                    '.' | 'ю' => self.update(Message::StepFrames(1)),
                    '[' | 'х' => self.update(Message::PlayerRate(-1)),
                    ']' | 'ъ' => self.update(Message::PlayerRate(1)),
                    digit if digit.is_ascii_digit() => {
                        let part = digit.to_digit(10).unwrap_or(0) as f32 / 10.0;
                        self.update(Message::SeekTo(part))
                    }
                    _ => Task::none(),
                }
            }
            Message::RevealVideo => {
                if let Some(clip) = &self.clip {
                    let _ = open::that_detached(clip.path.parent().unwrap_or(Path::new(".")));
                } else if let Some(video) = self.open_video.and_then(|at| self.store.videos.get(at)) {
                    let _ = open::that_detached(video.path.parent().unwrap_or(Path::new(".")));
                }
                Task::none()
            }
            Message::AskDeleteOf(at) => {
                if self.store.videos.get(at).is_some() {
                    self.open_video = Some(at);
                    self.asking_delete = true;
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
                self.shut_cinema();
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
                let now = Instant::now();
                self.hover_since = at.map(|at| (at, now));
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
                Task::none()
            }
            Message::Show(overlay) => {
                let now = Instant::now();
                if overlay == Overlay::Settings {
                    self.side = if self.settings.settings_tab == "bot" { Side::Bot } else { Side::App };
                    let renders = self.settings.renders_dir();
                    let songs = crate::sources::own_root().join("Songs");
                    let root = crate::sources::own_root();
                    let measured = renders.clone();
                    let sizes = ui::in_thread(move || {
                        Message::Sized(
                            prefs::folder_size(&renders),
                            prefs::folder_size(&songs),
                            prefs::folder_size(&root.join("maps.json")) + prefs::folder_size(&root.join("found.json")),
                        )
                    });
                    let stored = ui::in_thread(move || Message::Stored(prefs::measure(&measured)));
                    let skins = self.look_for_skins();
                    let ffmpeg = self.ffmpeg.clone();
                    let version = ui::in_thread(move || Message::Ffmpeg(ffmpeg.as_deref().and_then(crate::checks::ffmpeg_version)));
                    let chats = self.chats_task();
                    self.turn_to(overlay, now);
                    self.overlay = overlay;
                    self.rest_live(true);
                    return Task::batch([sizes, stored, version, chats, skins]);
                }
                let news = match overlay == Overlay::Community {
                    true => {
                        self.now_unix = unix_now();
                        if self.community.is_none() {
                            self.community = Some(self.first_community());
                        }
                        self.feed_seen = self.newest_event();
                        if !self.news_loaded {
                            self.news = crate::news::News::load();
                            self.news_loaded = true;
                        }
                        if self.osu_card.is_none() {
                            self.osu_card = crate::osu_profile::load();
                            self.remerge();
                            self.dress_staged_you();
                        }
                        Task::batch([self.refresh_news(false), self.news_pictures_task(), self.community_task(false), self.community_pictures_task(), self.osu_task(false)])
                    }
                    false => Task::none(),
                };
                if overlay != self.overlay {
                    self.turn_to(overlay, now);
                    if overlay == Overlay::Community {
                        self.section_at = now;
                    }
                }
                self.overlay = overlay;
                self.rest_live(overlay != Overlay::None);
                if overlay != Overlay::Videos {
                    if let Some(player) = self.player.take() {
                        player.borrow_mut().close();
                    }
                    self.open_video = None;
                    self.clip = None;
                    self.asking_delete = false;
                    self.shut_cinema();
                    return news;
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
            Message::Community(inner) => {
                use crate::community_screen::Message as C;
                let now = Instant::now();
                match inner {
                    C::Read(reading) => {
                        let wanted = reading.pictures();
                        self.community_reading = Some(reading);
                        self.read_fade = Animation::new(false).duration(STAGE_SHOW).easing(Easing::EaseOutCubic).go(true, now);
                        return self.wide_pictures_task(wanted);
                    }
                    C::Unread => self.read_fade.go_mut(false, now),
                    C::Standing(standing) => {
                        if standing != self.community_standing {
                            self.shift_at = now;
                        }
                        self.community_standing = standing;
                    }
                    C::PeopleFrom(from) => {
                        if from != self.people_from {
                            self.shift_at = now;
                        }
                        self.people_from = from;
                        if from == crate::community_screen::PeopleFrom::Game {
                            return self.friends_task(false);
                        }
                    }
                    C::Again => {
                        let friends = match self.people_from {
                            crate::community_screen::PeopleFrom::Game => self.friends_task(true),
                            crate::community_screen::PeopleFrom::Chat => Task::none(),
                        };
                        return Task::batch([self.community_task(true), friends, self.card_task(true)]);
                    }
                    C::Section(section) => {
                        if section != self.community_section {
                            self.section_at = now;
                        }
                        if self.community_reading.is_some() && self.read_fade.value() {
                            self.read_fade.go_mut(false, now);
                        }
                        self.community_section = section;
                        self.community_person = None;
                    }
                    C::Board(board) => {
                        if self.community_section != crate::community_screen::Section::Boards {
                            self.section_at = now;
                        } else if board != self.community_board {
                            self.shift_at = now;
                        }
                        self.community_board = board;
                        self.community_section = crate::community_screen::Section::Boards;
                    }
                    C::Person(Some(at)) => {
                        self.person_at = now;
                        self.community_person = Some(at);
                        self.person_fade = Animation::new(false).duration(STAGE_SHOW).easing(Easing::EaseOutCubic).go(true, now);
                        return self.person_task(at);
                    }
                    C::Person(None) => self.person_fade.go_mut(false, now),
                    C::Open(url) => {
                        let _ = open::that_detached(url);
                    }
                    C::Refresh => return self.refresh_news(true),
                    C::ChannelDraft(said) => self.channel_draft = said,
                    C::ChannelAdd => {
                        let Some(name) = crate::news::channel_name(&self.channel_draft) else {
                            return Task::none();
                        };
                        self.channel_draft.clear();
                        if self.settings.news_channels.iter().any(|kept| kept.eq_ignore_ascii_case(&name)) {
                            return Task::none();
                        }
                        self.settings.news_channels.push(name.clone());
                        let _ = self.settings.save();
                        return self.fetch_channel(name);
                    }
                    C::ChannelRemove(name) => {
                        self.settings.news_channels.retain(|kept| !kept.eq_ignore_ascii_case(&name));
                        let _ = self.settings.save();
                        self.news.take_posts(&name, Vec::new());
                        self.news.fetched.remove(&crate::news::channel_source(&name));
                        self.news.save();
                    }
                    C::Filter(filter) => {
                        if filter != self.feed_filter {
                            self.shift_at = now;
                        }
                        self.feed_filter = filter;
                    }
                    C::Source(source) => {
                        if source != self.feed_source {
                            self.shift_at = now;
                        }
                        self.feed_source = source;
                    }
                    C::Stream(stream) => {
                        if stream != self.feed_stream {
                            self.shift_at = now;
                        }
                        self.feed_stream = stream;
                    }
                    C::Search(query) => self.feed_query = query,
                    C::Toggle(key) => {
                        if !self.feed_open.remove(&key) {
                            self.feed_open.insert(key);
                        }
                    }
                    C::Reveal => self.feed_seen = self.newest_event(),
                    C::Spot(index) => {
                        if index != self.spot {
                            self.spot = index;
                            self.spot_at = now;
                        }
                        self.spot_due = now;
                        self.spot_held = self.spot_held.map(|_| now);
                    }
                    C::SpotHold(held) => match (held, self.spot_held) {
                        (true, None) => self.spot_held = Some(now),
                        (false, Some(since)) => {
                            self.spot_due += now.saturating_duration_since(since);
                            self.spot_held = None;
                        }
                        _ => {}
                    },
                    C::Rank(index) => {
                        if index != self.rank {
                            self.rank = index;
                            self.rank_at = now;
                        }
                        self.rank_due = now;
                        self.rank_held = self.rank_held.map(|_| now);
                    }
                    C::RankHold(held) => match (held, self.rank_held) {
                        (true, None) => self.rank_held = Some(now),
                        (false, Some(since)) => {
                            self.rank_due += now.saturating_duration_since(since);
                            self.rank_held = None;
                        }
                        _ => {}
                    },
                    C::Metric(metric) => self.dossier_metric = metric,
                    C::Span(span) => self.dossier_span = span,
                    C::GradeHover(hover) => self.grade_hover = hover,
                    C::TitlePick(code) => self.title_pick = Some(code),
                    C::PlayOpen(index) => {
                        self.play_at = now;
                        self.play_open = if self.play_open == Some(index) { None } else { Some(index) };
                    }
                    C::PlayClip(src, link) => {
                        let Some(src) = src else {
                            let _ = open::that_detached(link);
                            return Task::none();
                        };
                        if !self.clips_loading.insert(link.clone()) {
                            return Task::none();
                        }
                        let path = crate::news::clip_path(&link);
                        let ffmpeg = self.ffmpeg.clone();
                        return ui::in_thread(move || {
                            let saved = match path.exists() {
                                true => Ok(()),
                                false => crate::news::save_to(&src, &path),
                            };
                            let probed = saved.and_then(|_| {
                                let ffmpeg = ffmpeg.ok_or("no ffmpeg")?;
                                let probe = videos::probe(&ffmpeg, &path).ok_or("the clip does not read")?;
                                Ok((path, probe.length_ms, probe.fps))
                            });
                            Message::ClipFetched(link, probed)
                        });
                    }
                }
                Task::none()
            }
            Message::NewsBuilds(result) => {
                let source = crate::news::UPDATES;
                self.news_loading.remove(source);
                match result {
                    Ok(builds) => {
                        self.news.builds = builds;
                        self.news_heard(source);
                    }
                    Err(_) => {
                        self.news_failed.insert(source.to_owned());
                    }
                }
                Task::none()
            }
            Message::NewsStories(result) => {
                let source = crate::news::STORIES;
                self.news_loading.remove(source);
                match result {
                    Ok(stories) => {
                        self.news.stories = stories;
                        self.news_heard(source);
                        self.news_pictures_task()
                    }
                    Err(_) => {
                        self.news_failed.insert(source.to_owned());
                        Task::none()
                    }
                }
            }
            Message::NewsThreads(result) => {
                let source = crate::news::THREADS;
                self.news_loading.remove(source);
                match result {
                    Ok(threads) => {
                        self.news.threads = threads;
                        self.news_heard(source);
                    }
                    Err(_) => {
                        self.news_failed.insert(source.to_owned());
                    }
                }
                Task::none()
            }
            Message::NewsPosts(channel, result) => {
                let source = crate::news::channel_source(&channel);
                self.news_loading.remove(&source);
                match result {
                    Ok(posts) => {
                        self.news.take_posts(&channel, posts);
                        self.news_heard(&source);
                        self.news_pictures_task()
                    }
                    Err(_) => {
                        self.news_failed.insert(source);
                        Task::none()
                    }
                }
            }
            Message::NewsPicture(url, handle) => {
                if let Some(handle) = handle {
                    self.news_pictures.insert(url, handle);
                }
                Task::none()
            }
            Message::ReadFirst => match self.news.stories.first().cloned() {
                Some(story) => self.update(Message::Community(crate::community_screen::Message::Read(crate::community_screen::Reading::Story(story)))),
                None => Task::none(),
            },
            Message::CommunityTick => self.community_task(false),
            Message::OsuProfile(Ok(card)) => {
                crate::osu_profile::save(&card);
                self.osu_card = Some(card);
                self.remerge();
                self.dress_staged_you();
                let pictures = self.community_pictures_task();
                Task::batch([pictures, self.share_task()])
            }
            Message::OsuProfile(Err(_)) => Task::none(),
            Message::PersonCard(name, said) => {
                self.people_asked.remove(&format!("card:{name}"));
                match said {
                    Ok(card) => {
                        let wanted: Vec<(String, u32)> = card.pictures();
                        self.people_cards.insert(name, card);
                        self.pictures_task(wanted)
                    }
                    Err(_) => Task::none(),
                }
            }
            Message::PersonDossier(id, said) => {
                self.people_asked.remove(&format!("me:{id}"));
                let Some(catalog) = self.community.as_mut() else {
                    return Task::none();
                };
                let Some(at) = catalog.people.iter().position(|person| person.id == id) else {
                    return Task::none();
                };
                let name = catalog.people[at].name.to_lowercase();
                let mut shared = None;
                if let Ok(said) = said {
                    let dossier = catalog.take_someone(&said);
                    self.people_dossiers.insert(id, dossier);
                    let fresh = said.card_at.is_some_and(|at| unix_now() - at < SHARED_FRESH);
                    shared = said.card.filter(|card| card.pp > 0.0 || !card.username.is_empty()).map(|card| (card, fresh));
                }
                let mut tasks = vec![self.community_pictures_task()];
                match shared {
                    Some((card, fresh)) => {
                        let wanted: Vec<(String, u32)> = card.pictures();
                        self.people_cards.entry(name.clone()).or_insert(card);
                        tasks.push(self.pictures_task(wanted));
                        if !fresh {
                            tasks.push(self.scrape_task(at));
                        }
                    }
                    None => tasks.push(self.scrape_task(at)),
                }
                Task::batch(tasks)
            }
            Message::CardShared(_) => Task::none(),
            Message::ClipFetched(link, probed) => {
                self.clips_loading.remove(&link);
                match probed {
                    Ok((path, length_ms, fps)) if self.overlay == Overlay::Community => self.play_clip(link, path, length_ms, fps),
                    Ok((path, _, _)) if self.ffmpeg.is_none() => {
                        let _ = open::that_detached(&path);
                    }
                    Ok(_) => {}
                    Err(_) => {
                        let _ = open::that_detached(&link);
                    }
                }
                Task::none()
            }
            Message::FeedClock => {
                let now = Instant::now();
                if self.spot_held.is_none() && now.saturating_duration_since(self.spot_due) >= crate::chronicle::SPOT_EVERY {
                    self.spot = self.spot.wrapping_add(1);
                    self.spot_at = now;
                    self.spot_due = now;
                }
                if self.rank_held.is_none() && now.saturating_duration_since(self.rank_due) >= crate::chronicle::RANK_EVERY {
                    self.rank = (self.rank + 1) % crate::community::Board::ALL.len();
                    self.rank_at = now;
                    self.rank_due = now;
                }
                Task::none()
            }
            Message::CommunityArrived(Ok(said)) => {
                crate::community::wire::save(&said);
                self.now_unix = unix_now();
                let mut fresh = crate::community::Catalog::from_wire(said);
                let previous = self.community.take();
                if let Some(previous) = previous.as_ref().filter(|previous| !previous.staged) {
                    if matches!(previous.friends_state, crate::community::Friends::Ready | crate::community::Friends::Need(_) | crate::community::Friends::Failed) {
                        fresh.friends = previous.friends.clone();
                        fresh.friends_state = previous.friends_state.clone();
                    }
                }
                let newest = previous.as_ref().filter(|previous| !previous.staged).and_then(|previous| previous.live.last().map(|play| play.at));
                let newer = newest.map_or(0, |newest| fresh.live.iter().filter(|play| play.at > newest).count());
                self.live_shown = fresh.live.len() - newer.min(fresh.live.len());
                if self.community_person.is_some_and(|at| at >= fresh.people.len()) {
                    self.community_person = None;
                }
                self.community = Some(fresh);
                self.community_fetch = crate::community_screen::Fetch::Fresh(self.now_unix);
                let card = self.card_task(false);
                let friends = self.friends_task(false);
                Task::batch([self.community_pictures_task(), friends, card])
            }
            Message::CommunityArrived(Err(_)) => {
                self.community_fetch = crate::community_screen::Fetch::Failed;
                if self.community.as_ref().is_some_and(|catalog| !catalog.staged && catalog.people.is_empty() && catalog.me.is_none()) {
                    self.community = Some(self.staged_community());
                }
                Task::none()
            }
            Message::CardArrived(Ok(card)) => {
                crate::community::wire::save_card(&card);
                self.community_card = Some(card);
                self.remerge();
                self.community_pictures_task()
            }
            Message::CardArrived(Err(_)) => Task::none(),
            Message::Flag(code, bytes) => {
                if let Some(bytes) = bytes {
                    self.flags.insert(code, iced::widget::svg::Handle::from_memory(flag_shape(bytes)));
                }
                Task::none()
            }
            Message::FriendsArrived(said) => {
                if let Some(catalog) = self.community.as_mut().filter(|catalog| !catalog.staged) {
                    match said {
                        Ok(crate::bot::Friends::Listed(listed)) => catalog.take_friends(listed),
                        Ok(crate::bot::Friends::Need(need)) => catalog.friends_state = crate::community::Friends::Need(need),
                        Err(_) => catalog.friends_state = crate::community::Friends::Failed,
                    }
                }
                self.community_pictures_task()
            }
            Message::LiveArrive => {
                let pool = self.community.as_ref().map_or(0, |catalog| catalog.live.len());
                if self.live_shown < pool {
                    self.live_shown += 1;
                    self.live_arrived = Some(Instant::now());
                }
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
                let out = self.settings.renders_dir().join(render::file_name(&entry.player, &map.line()));
                let ask = render::Ask {
                    replay: entry.path.clone(),
                    map: map.file.clone(),
                    map_hash: entry.map_hash.clone(),
                    ffmpeg,
                    out,
                    size: self.settings.render_size(),
                    fps: self.settings.render_fps,
                    crf: self.settings.render_crf,
                    skin: self.settings.skin.clone(),
                    music_level: self.settings.music_level,
                    hitsound_level: self.settings.hitsound_level,
                    play: render::Play {
                        hud: self.settings.hud,
                        cursor_grows: self.settings.cursor_grows,
                        dim: (self.settings.background_dim * 100.0).round() as u32,
                        blur: (self.settings.background_blur * 100.0).round() as u32,
                        map_sounds: self.settings.map_sounds,
                        skin_sounds: self.settings.skin_sounds,
                    },
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
                        let video = videos::Video::from_render(&entry, out.clone(), length, self.settings.render_size().0, self.settings.render_size().1, self.settings.render_fps);
                        let detail = format!("{} — {}", video.player, video.map_line());
                        let note = format!("{} · {}", self.words.length(video.length_ms), self.words.mb(video.size));
                        let hash = video.map_hash.clone();
                        self.store.add(video);
                        let watching_it = self.overlay == Overlay::None && self.chosen_entry().is_some_and(|chosen| chosen.path == replay);
                        let words = self.words.t("rendered-notice");
                        match watching_it {
                            true => {
                                let _ = self.write_notice(notices::Mark::Done, words, detail, note, hash, notices::Link::OpenVideo(out));
                            }
                            false => self.announce(notices::Mark::Done, words, detail, note, hash, notices::Link::OpenVideo(out)),
                        }
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
                    live::Frame::Picture { frame, at_ms, .. } => {
                        if live.frame.is_none() {
                            live.fade.go_mut(true, Instant::now());
                        }
                        live.frame = Some(frame);
                        live.at_ms = at_ms;
                        if live.rest.value() || live.rest.is_animating(Instant::now()) {
                            live.rest.go_mut(false, Instant::now());
                        }
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
                let serial = self.strip_aim.map_or(1, |(serial, _)| serial + 1);
                self.strip_aim = Some((serial, x));
                Task::none()
            }
            Message::TogglePlay => {
                if self.overlay != Overlay::None {
                    return Task::none();
                }
                self.paused_by_hand = !self.paused_by_hand;
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
                let skinnish = crate::settings::is_skin_file(&path) || (path.is_dir() && !crate::settings::skins_under(&path).is_empty());
                if skinnish || (path.is_dir() && crate::settings::looks_like_skin(&path)) {
                    let named = crate::settings::skin_name(&path);
                    let taken = self.prefs(prefs::Message::AddedSkin(Some(path)));
                    self.announce(notices::Mark::Done, self.words.t("skin-added"), named, String::new(), String::new(), notices::Link::None);
                    return taken;
                }
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
                let dt = now.saturating_duration_since(self.now).as_secs_f32().min(1.0 / 30.0);
                if self.scenes_due {
                    self.scenes_due = false;
                    let load = self.scenes_for_notices();
                    self.now = now;
                    return load.chain(self.update(Message::Tick(now)));
                }
                if let Some(player) = &self.player {
                    let mut player = player.borrow_mut();
                    player.pull();
                    if player.ended() && self.settings.player_loop {
                        player.seek(0);
                    }
                }
                if self.hint.as_ref().is_some_and(|(_, since)| now.saturating_duration_since(*since) > HINT_SHOWN) {
                    self.hint = None;
                }
                if self.leaving_player && !self.stage_open.is_animating(now) {
                    self.finish_closing();
                }
                let watching = self.player.is_some() && !self.leaving_player;
                if self.cinema.value() != watching {
                    self.cinema.go_mut(watching, now);
                }
                if self.ask_fade.value() != self.asking_delete {
                    self.ask_fade.go_mut(self.asking_delete, now);
                }
                let resting = self.player.as_ref().is_some_and(|p| p.borrow().paused);
                let shown = resting || self.scrubbing.is_some() || self.over_controls || now.saturating_duration_since(self.stirred) < CONTROLS_STAY;
                if watching && self.controls.value() != shown {
                    self.show_controls(shown, now);
                }
                if !watching && self.widened.value() {
                    self.widened.go_mut(false, now);
                }
                self.now = now;
                if let Some(live) = &self.live {
                    if !(live.control.paused() && live.control.settled()) {
                        live.control.request();
                    }
                }
                match self.progress_target() {
                    Some(target) => self.progress_shown = ui::toward(self.progress_shown, target, 0.12, dt),
                    None => self.progress_shown = 0.0,
                }
                self.marks_now = self.marks.iter().map(|(id, mark)| (id.clone(), mark.interpolate(0.0, 1.0, now))).collect();
                self.ease_slides(dt);
                self.words.typed_up_to(self.retype.interpolate(0.0, 1.0, now));
                if !self.retype.is_animating(now) {
                    self.words.settle();
                }
                for toast in &mut self.toasts {
                    if toast.shown.value() && !toast.stays && !toast.hovered && now.duration_since(toast.born) > TOAST_STAY {
                        toast.shown.go_mut(false, now);
                    }
                }
                self.toasts.retain(|t| t.shown.value() || t.shown.is_animating(now));
                if let Some(next) = self.turning {
                    if !self.overlay_fade.value() && !self.overlay_fade.is_animating(now) {
                        self.turning = None;
                        self.overlay_drawn = next;
                        self.overlay_fade = Animation::new(false).duration(OVERLAY_FADE).easing(Easing::EaseOutCubic).go(true, now);
                    }
                }
                if self.lang_swap && !self.overlay_fade.value() && !self.overlay_fade.is_animating(now) {
                    self.lang_swap = false;
                    self.overlay_fade = Animation::new(false).duration(LANG_FADE).easing(Easing::EaseOutCubic).go(true, now);
                }
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
                if !self.read_fade.value() && !self.read_fade.is_animating(now) {
                    self.community_reading = None;
                }
                self.combo_for_rested(now)
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

    fn show_controls(&mut self, shown: bool, now: Instant) {
        let at = self.controls.interpolate(0.0, 1.0, now);
        let span = if shown { CONTROLS_IN } else { CONTROLS_OUT };
        let left = if shown { 1.0 - at } else { at };
        self.controls = Animation::new(!shown)
            .duration(span.mul_f32(left.clamp(0.2, 1.0)))
            .easing(if shown { Easing::EaseOutCubic } else { Easing::EaseInOutCubic })
            .go(shown, now);
    }

    fn play_clip(&mut self, link: String, path: PathBuf, length_ms: i64, fps: u32) {
        let Some(ffmpeg) = self.ffmpeg.clone() else {
            return;
        };
        let post = self.news.posts.iter().find(|post| post.videos.iter().any(|video| video.link == link));
        let thumb = post.and_then(|post| post.videos.iter().find(|video| video.link == link)).and_then(|video| video.thumb.clone());
        let clip = Clip {
            path: path.clone(),
            from: post.map_or_else(String::new, |post| if post.name.is_empty() { format!("@{}", post.channel) } else { post.name.clone() }),
            said: post.map_or_else(String::new, |post| post.text.lines().map(str::trim).find(|line| !line.is_empty()).unwrap_or_default().to_owned()),
            link,
            thumb,
        };
        let fresh = self.player.is_none() || self.leaving_player;
        if let Some(old) = self.player.take() {
            old.borrow_mut().close();
        }
        self.leaving_player = false;
        let now = Instant::now();
        if fresh {
            self.stage_open = Animation::new(false).duration(STAGE_OPEN).easing(Easing::EaseOutCubic).go(true, now);
        }
        self.stirred = now;
        self.over_controls = false;
        self.controls = Animation::new(true).duration(CONTROLS_IN).easing(Easing::EaseOutCubic);
        let manner = player::Manner { level: self.settings.player_level, muted: self.settings.player_muted, rate: self.settings.player_rate };
        self.player = Some(std::rc::Rc::new(std::cell::RefCell::new(player::Player::open(&ffmpeg, &path, length_ms, fps, manner))));
        self.open_video = None;
        self.clip = Some(clip);
        self.asking_delete = false;
        self.scrubbing = None;
        self.hint = None;
        self.cinema.go_mut(true, now);
    }

    fn finish_closing(&mut self) {
        if let Some(player) = self.player.take() {
            player.borrow_mut().close();
        }
        self.leaving_player = false;
        self.open_video = None;
        self.clip = None;
        self.asking_delete = false;
        self.over_controls = false;
        self.pointer = None;
        self.shut_cinema();
    }

    fn shut_cinema(&mut self) {
        let now = Instant::now();
        self.cinema.go_mut(false, now);
        self.widened.go_mut(false, now);
        self.scrubbing = None;
        self.hint = None;
    }

    fn skin_scenery(&self) -> Task<Message> {
        let wanted: Vec<PathBuf> = std::iter::once(PathBuf::new())
            .chain(self.skins.iter().cloned())
            .filter(|folder| !self.skin_scenes.contains_key(folder))
            .collect();
        if wanted.is_empty() {
            return Task::none();
        }
        ui::streamed(move |push| {
            for folder in wanted {
                let at = (!folder.as_os_str().is_empty()).then_some(folder.as_path());
                let rgba = crate::settings::skin_pattern(at, PATTERN.0, PATTERN.1);
                let handle = image::Handle::from_rgba(PATTERN.0, PATTERN.1, rgba);
                if !push(Message::SkinScene(folder, Some(handle))) {
                    return;
                }
            }
        })
    }

    fn skin_room_layer(&self) -> Element<'_, Message> {
        let k = self.room_fade.interpolate(0.0, 1.0, self.now);
        if !self.skin_room && k < 0.001 {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        }
        ui::fading(k, || {
            let w = &self.words;
            let chosen = self.settings.skin.clone();
            let panel = |name: String, folder: Option<PathBuf>, picture: Option<&image::Handle>, picked: bool| -> Element<'_, Message> {
                let face: Element<'_, Message> = match picture {
                    Some(handle) => image(handle.clone()).width(PANEL.0).height(PANEL.1).opacity(ui::fade()).into(),
                    None => container(ui::fine_hatch()).width(PANEL.0).height(PANEL.1).into(),
                };
                let fits = name.chars().count() as f32 * NAME_WIDTH <= PANEL.0 - 8.0;
                let words = text(name).font(theme::SANS_SEMI).size(theme::BODY).wrapping(text::Wrapping::None).color(ui::faded(if picked { INK } else { MUTED }));
                let label: Element<'_, Message> = match fits {
                    true => container(words).width(PANEL.0).height(22.0).center_x(PANEL.0).align_y(iced::alignment::Vertical::Center).into(),
                    false => ui::trailing(words.into(), PANEL.0, 22.0, if picked { theme::ROOM_PICKED } else { theme::ROOM_GROUND }),
                };
                let inside = column![container(face).width(PANEL.0).height(PANEL.1).style(ui::box_faded(theme::screen)).clip(true), label].spacing(6);
                button(inside)
                    .padding(6)
                    .style(ui::button_faded(theme::slot_choice(picked)))
                    .on_press(Message::Prefs(prefs::Message::Skin(folder)))
                    .into()
            };
            let mut cells: Vec<Element<'_, Message>> = vec![panel(
                w.t("own-skin-short"),
                None,
                self.skin_scenes.get(Path::new("")),
                chosen.is_none(),
            )];
            for folder in &self.skins {
                let picked = chosen.as_deref() == Some(folder.as_path());
                cells.push(panel(crate::settings::skin_name(folder), Some(folder.clone()), self.skin_scenes.get(folder), picked));
            }
            let title = row![
                text(w.t("skins")).font(theme::SANS_SEMI).size(20.0).color(ui::faded(INK)),
                ui::grow(),
                ui::control_button(ui::Control::Close, 18.0, Some(Message::ShowSkins(false)), false),
            ]
            .align_y(iced::Center);
            let each = PANEL.0 + 12.0 + ROOM_GAP;
            let per = ((((self.width - 120.0).clamp(420.0, 1180.0) - 2.0 * ROOM_SIDE + ROOM_GAP) / each).floor() as usize).clamp(1, cells.len().max(1));
            let wide = per as f32 * each - ROOM_GAP + 2.0 * ROOM_SIDE;
            let lines = cells.len().div_ceil(per);
            let line_high = PANEL.1 + 12.0 + 26.0;
            let room_high = lines as f32 * line_high + (lines.saturating_sub(1)) as f32 * ROOM_GAP + ROOM_SIDE;
            let tall = (ROOM_TOP + room_high).min(self.height - 150.0).max(ROOM_TOP + line_high);
            let mut grid = column![].spacing(ROOM_GAP);
            let mut line = row![].spacing(ROOM_GAP);
            let mut at = 0;
            for cell in cells {
                line = line.push(cell);
                at += 1;
                if at % per == 0 {
                    grid = grid.push(line);
                    line = row![].spacing(ROOM_GAP);
                }
            }
            if at % per != 0 {
                grid = grid.push(line);
            }
            let inside = column![
                container(title)
                    .height(ROOM_TOP)
                    .align_y(iced::alignment::Vertical::Center)
                    .padding(Padding { top: 2.0, right: ROOM_SIDE - 7.0, bottom: 0.0, left: ROOM_SIDE + 6.0 }),
                scrollable(container(grid).width(Length::Fill).padding(Padding { top: 0.0, right: ROOM_SIDE, bottom: ROOM_SIDE, left: ROOM_SIDE }))
                    .anchor_y(scrollable::Anchor::Start)
                    .style(ui::thin_scroll)
                    .height(tall - ROOM_TOP),
            ]
            .width(Length::Fill);
            let card = container(inside).width(wide).height(tall).style(ui::box_faded(theme::stage)).clip(true);
            stack![
                mouse_area(ui::veil(theme::SCRIM)).on_press(Message::ShowSkins(false)),
                container(card).width(Length::Fill).height(Length::Fill).center(Length::Fill)
            ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        })
    }

    fn skin_again(&mut self) -> Task<Message> {
        if self.live.is_none() && self.trail.is_none() {
            return Task::none();
        }
        let again = self.start_live();
        self.rest_live(self.overlay != Overlay::None);
        again
    }

    fn look_for_skins(&self) -> Task<Message> {
        let sources = self.settings.sources.clone();
        let own = self.settings.own_skins.clone();
        let near = {
            let (sources, own) = (sources.clone(), own.clone());
            ui::in_thread(move || {
                let _ = crate::settings::adopt_skin_files(&crate::settings::skins_root());
                Message::Skins(crate::settings::skins_in(&sources, &own))
            })
        };
        let far = ui::in_thread(move || Message::Skins(crate::settings::hunt_skins(&sources, &own)));
        Task::batch([near, far])
    }

    fn say_trouble(&mut self, why: String) {
        self.announce(notices::Mark::Bad, self.words.t("skin-failed"), String::new(), why, String::new(), notices::Link::None);
    }

    pub fn write_notice(&mut self, mark: notices::Mark, words: String, detail: String, note: String, map_hash: String, link: notices::Link) -> u64 {
        let id = self.notices.push(mark, words, detail, note, map_hash, link);
        self.scenes_due = true;
        if self.menu == Some(Tab::Feed) {
            self.arrivals.insert(id, Animation::new(false).duration(NOTICE_ARRIVE).easing(Easing::EaseOutCubic).go(true, Instant::now()));
        }
        id
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
            (Some(entry), Some(live), _) if live.for_path == entry.path && self.overlay == Overlay::None => match &live.frame {
                Some(frame) => {
                    let seen = live.fade.interpolate(0.0, 1.0, self.now);
                    mouse_area(crate::film::show(frame, crate::film::Fit::Cover, alpha * seen)).on_press(Message::TogglePlay).into()
                }
                None => blank(),
            },
            (Some(entry), _, Some(trail)) if trail.for_path == entry.path && self.overlay == Overlay::None => mouse_area(
                iced::widget::canvas(ui::Trail { points: trail.points.clone(), at_ms: trail.at_ms(self.now), window_ms: 3000.0 })
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .on_press(Message::TogglePlay)
            .into(),
            _ => blank(),
        };
        let body: Element<'_, Message> = if loaded { ui::fading(k, || self.body(k, s)) } else { blank() };
        let early = self.arrive.interpolate(0.0, 1.0, self.now) * (1.0 - self.cinema.interpolate(0.0, 1.0, self.now));
        let crest = ui::fading(early, ui::brand);
        let crest: Element<'_, Message> =
            pin(float(crest).translate(move |_, _| Vector::new(0.0, (1.0 - early) * CREST_RISE))).x(CREST_HOME.0).y(CREST_HOME.1).into();
        let sheet = self.overlay_fade.interpolate(0.0, 1.0, self.now);
        let showing = self.overlay != Overlay::None || self.overlay_fade.is_animating(self.now);
        let late = ((k - 0.7) / 0.3).clamp(0.0, 1.0);
        let watching = 1.0 - self.cinema.interpolate(0.0, 1.0, self.now);
        let chrome_layer: Element<'_, Message> = if loaded && watching > 0.001 {
            ui::fading(alpha * late * watching, || self.chrome())
        } else {
            blank()
        };
        let deep = self.ground_fade.interpolate(0.0, 1.0, self.now);
        let ground: Element<'_, Message> = if deep > 0.001 {
            ui::fading(deep, || ui::veil(theme::GROUND))
        } else {
            blank()
        };
        let overlay: Element<'_, Message> = if showing { ui::fading(sheet, || self.overlay_view()) } else { blank() };
        let bubble = self.bubble_layer();
        let toasts = self.toast_layer();
        let menu = self.menu_layer();
        let signing = self.sign_in_layer();
        let failure = self.error_layer();
        let ask: Element<'_, Message> = if self.asking_delete || self.ask_fade.is_animating(self.now) {
            self.delete_card()
        } else {
            blank()
        };
        let room = self.skin_room_layer();
        let layers = stack![scene_before, scene, live_before, live, body, bubble, ground, overlay, chrome_layer, crest, room, ask, menu, signing, failure, toasts];
        layers.width(Length::Fill).height(Length::Fill).into()
    }

    fn body(&self, k: f32, s: f32) -> Element<'_, Message> {
        column![
            Space::new().height(theme::CONTROL_HEIGHT + 4.0 + 22.0),
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
            word("settings", self.overlay == Overlay::Settings, Message::Show(Overlay::Settings)),
            self.circle(CIRCLE_SIDE, true),
        ]
        .spacing(22)
        .align_y(iced::Center);
        let mut top = row![Space::new().width(BRAND_WIDTH)].align_y(iced::Center).height(theme::CONTROL_HEIGHT + 4.0);
        if let Some(tabs) = self.part_tabs() {
            top = top.push(Space::new().width(34.0)).push(tabs);
        }
        container(top.push(ui::grow()).push(words))
        .padding(Padding { top: 22.0, right: 40.0, bottom: 0.0, left: 40.0 })
        .width(Length::Fill)
        .into()
    }

    fn part_tabs(&self) -> Option<Element<'_, Message>> {
        use crate::community_screen::{Message as C, Section};
        let w = &self.words;
        let (size, gap) = match self.width {
            wide if wide >= 1200.0 => (17.0, 26.0),
            wide if wide >= 1060.0 => (15.5, 22.0),
            _ => (14.0, 20.0),
        };
        let tab = |key: &str, on: bool, msg: Message| -> Element<'_, Message> {
            button(column![Space::new().height(2.0), text(w.t(key)).font(theme::SANS_SEMI).size(size), Space::new().height(2.0)].spacing(5))
                .padding(0)
                .style(ui::button_faded(theme::word(on)))
                .on_press(msg)
                .into()
        };
        let pill = ui::Pill { fill: ACCENT, edge: Color::TRANSPARENT, radius: 1.0, underline: Some(0.0) };
        let sheet = self.overlay_fade.interpolate(0.0, 1.0, self.now);
        match self.overlay {
            Overlay::Community if self.community.is_some() => Some(ui::fading(ui::fade() * sheet, || {
                let parts = [
                    ("community-profile", Section::Profile),
                    ("community-feed", Section::Feed),
                    ("community-people", Section::People),
                    ("community-boards", Section::Boards),
                    ("community-titles", Section::Titles),
                ];
                let active = parts.iter().position(|(_, section)| *section == self.community_section).unwrap_or(0);
                let line = iced::widget::Row::with_children(parts.iter().map(|(key, section)| tab(key, *section == self.community_section, Message::Community(C::Section(*section))))).spacing(gap).align_y(iced::Center);
                ui::sliding(line, active, pill)
            })),
            Overlay::Settings => Some(ui::fading(ui::fade() * sheet, || {
                let parts = [("app-side", Side::App), ("bot-side", Side::Bot)];
                let active = parts.iter().position(|(_, side)| *side == self.side).unwrap_or(0);
                let line = iced::widget::Row::with_children(parts.iter().map(|(key, side)| tab(key, *side == self.side, Message::Prefs(prefs::Message::Side(*side))))).spacing(gap).align_y(iced::Center);
                ui::sliding(line, active, pill)
            })),
            _ => None,
        }
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
        let rendered = self
            .store
            .videos
            .iter()
            .position(|v| v.replay == entry.path || (!v.replay_hash.is_empty() && v.replay_hash == entry.replay_hash));
        match (rendering_this, fetching_this, rendered) {
            (Some(rendering), _, _) if !rendering.is_over() => self.render_button(rendering),
            (Some(rendering), _, None) => self.render_button(rendering),
            (_, _, Some(at)) => row![ui::primary(w.t("open"), Some(Message::OpenVideo(at))), ui::quiet(w.t("delete"), Some(Message::AskDeleteOf(at)))]
                .spacing(4)
                .align_y(iced::Center)
                .into(),
            (None, Some(fetching), _) => self.fetch_button(fetching),
            (None, None, None) if entry.map.is_some() => ui::primary(w.t("render"), (self.ffmpeg.is_some() && !busy).then_some(Message::Render)),
            (None, None, None) => ui::primary(w.t("get-the-map"), (!fetching_now).then_some(Message::GetMap)),
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
                    Some(S::Checking) => w.t("checking-map"),
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
        let strip = crate::glide::glide(strip, self.strip_id.clone()).aimed(self.strip_aim, self.strip_view.map_or(0.0, |(offset, _, _)| offset));
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
        let doubled = matches!(entry.outcome, library::Outcome::Misses(n) if u32::from(n) == u32::from(miss));
        if entry.outcome != library::Outcome::Fail && !doubled {
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
        let how = ui::drifting(how, self.started);
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
        let which = self.overlay_drawn;
        if which == Overlay::Videos {
            return self.videos_view();
        }
        let w = &self.words;
        if which == Overlay::Settings {
            return self.settings_view();
        }
        if which == Overlay::Community {
            if let Some(view) = self.community_view() {
                return view;
            }
        }
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
        let sheet = column![Space::new().height(theme::CONTROL_HEIGHT + 4.0 + 22.0), page].width(Length::Fill).height(Length::Fill);
        let opened = self.stage_open.interpolate(0.0, 1.0, self.now);
        let stage: Element<'_, Message> = match (&self.player, self.open_video.and_then(|at| self.store.videos.get(at))) {
            (Some(player), Some(video)) if opened > 0.001 => ui::fading(ui::fade() * opened, || self.stage(&player.borrow(), self.video_bill(video))),
            _ => Space::new().width(Length::Fill).height(Length::Fill).into(),
        };
        let sheet: Element<'_, Message> = match self.player.is_some() && opened >= 0.999 {
            true => Space::new().width(Length::Fill).height(Length::Fill).into(),
            false => sheet.into(),
        };
        let watching = 1.0 - self.cinema.interpolate(0.0, 1.0, self.now);
        let crest: Element<'_, Message> = match watching > 0.001 {
            true => pin(ui::fading(ui::fade() * watching, ui::brand)).x(CREST_HOME.0).y(CREST_HOME.1).into(),
            false => Space::new().width(Length::Fill).height(Length::Fill).into(),
        };
        stack![sheet, crest, stage].width(Length::Fill).height(Length::Fill).into()
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

    fn say(&mut self, words: String) {
        self.hint = Some((words, Instant::now()));
    }

    fn video_bill<'a>(&'a self, video: &'a videos::Video) -> Bill<'a> {
        let w = &self.words;
        let sending_this = self.sending.as_ref().filter(|s| s.path == video.path);
        let telegram: Element<'_, Message> = match sending_this {
            Some(sending) if sending.over.is_none() => ui::progress(w.t("sending"), self.progress_shown, None),
            _ => ui::primary(w.t("to-telegram"), Some(Message::SendVideo)),
        };
        let buttons = row![
            ui::grow(),
            ui::springy(ui::quiet(w.t("in-folder"), Some(Message::RevealVideo)), 0.04),
            ui::springy(ui::quiet(w.t("delete"), Some(Message::AskDelete)), 0.04),
            ui::springy(telegram, 0.03),
        ]
        .spacing(6)
        .align_y(iced::Center);
        let at = self.open_video.unwrap_or(0);
        Bill {
            still: self.thumbs.get(&video.map_hash),
            name: video.player.clone(),
            mods: &video.mods,
            line: video.map_line(),
            buttons: buttons.into(),
            earlier: (at > 0).then_some(Message::PlayerNeighbour(-1)),
            later: (at + 1 < self.store.videos.len()).then_some(Message::PlayerNeighbour(1)),
        }
    }

    fn clip_bill<'a>(&'a self, clip: &'a Clip) -> Bill<'a> {
        let w = &self.words;
        let buttons = row![
            ui::grow(),
            ui::springy(ui::quiet(w.t("in-folder"), Some(Message::RevealVideo)), 0.04),
            ui::springy(ui::primary(w.t("act-telegram"), Some(Message::Community(crate::community_screen::Message::Open(clip.link.clone())))), 0.03),
        ]
        .spacing(6)
        .align_y(iced::Center);
        Bill {
            still: clip.thumb.as_deref().and_then(|url| self.news_pictures.get(&crate::community_screen::wide(url)).or_else(|| self.news_pictures.get(url))),
            name: clip.from.clone(),
            mods: &[],
            line: clip.said.clone(),
            buttons: buttons.into(),
            earlier: None,
            later: None,
        }
    }

    fn stage<'a>(&'a self, player: &player::Player, bill: Bill<'a>) -> Element<'a, Message> {
        let w = &self.words;
        let playing = !player.paused;
        let wide = self.widened.interpolate(0.0, 1.0, self.now);
        let round: iced::border::Radius = (PICTURE_RADIUS).into();
        let Bill { still, name, mods, line, buttons, earlier, later } = bill;
        let picture: Element<'_, Message> = match &player.frame {
            Some(frame) => crate::film::show(frame, crate::film::Fit::Contain, ui::fade()),
            None => match still {
                Some(handle) => image(handle.clone())
                    .content_fit(ContentFit::Cover)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .opacity(0.35 * ui::fade())
                    .border_radius(round)
                    .into(),
                None => Space::new().width(Length::Fill).height(Length::Fill).into(),
            },
        };
        let mark: Element<'_, Message> = if player.paused && !self.asking_delete {
            let kind = if player.ended() { ui::Control::Again } else { ui::Control::Play };
            container(ui::halo(kind, 72.0, 1.0))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center)
                .into()
        } else {
            Space::new().width(Length::Fill).height(Length::Fill).into()
        };
        let hint: Element<'_, Message> = match &self.hint {
            Some((words, since)) => {
                let gone = self.now.saturating_duration_since(*since).as_secs_f32() / HINT_SHOWN.as_secs_f32();
                let k = (1.0 - gone).clamp(0.0, 1.0);
                container(ui::fading(ui::fade() * k, || {
                    container(text(words.clone()).font(theme::SANS_SEMI).size(theme::BODY).color(ui::faded(INK)))
                        .padding(Padding { top: 6.0, right: 12.0, bottom: 6.0, left: 12.0 })
                        .style(ui::box_at(theme::bubble, ui::fade() * k))
                }))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center)
                .padding(Padding::ZERO.top(18.0))
                .into()
            }
            None => Space::new().width(Length::Fill).height(Length::Fill).into(),
        };
        let gap = STAGE_GAP - (STAGE_GAP - 16.0) * wide;
        let under_h = STAGE_UNDER * (1.0 - wide);
        let room_w = (self.width - 2.0 * gap - 2.0 * PICTURE_INSET).max(320.0);
        let room_h = (self.height - 2.0 * gap - STAGE_TOP - under_h - 2.0 * PICTURE_INSET).max(180.0);
        let screen_h = (room_w * 9.0 / 16.0).min(room_h).floor();
        let screen_w = (screen_h * 16.0 / 9.0).min(room_w).floor();
        let out = self.controls.interpolate(0.0, 1.0, self.now);
        let controls: Element<'_, Message> = match out > 0.004 {
            true => ui::fading(ui::fade() * out, || {
                let shown = self.scrubbing.unwrap_or_else(|| player.fraction());
                let length_ms = player.length_ms;
                let seek = iced::widget::canvas(ui::Seek {
                    played: shown,
                    alpha: ui::fade(),
                    at: Box::new(move |part| {
                        let ms = (part as f64 * length_ms as f64) as i64;
                        format!("{}:{:02}", ms / 60_000, (ms / 1000) % 60)
                    }),
                    on_move: Box::new(Message::Scrubbing),
                    on_drop: Box::new(Message::SeekTo),
                })
                .width(Length::Fill)
                .height(30.0);
                let at_ms = self.scrubbing.map_or_else(|| player.at_ms(), |part| (part as f64 * player.length_ms as f64) as i64);
                let clock = row![
                    ui::mono_small(w.length(at_ms), INK),
                    ui::mono_small("/".to_owned(), FAINT),
                    ui::mono_small(w.length(player.length_ms), MUTED),
                ]
                .spacing(6)
                .align_y(iced::Center);
                let sound = if self.settings.player_muted || self.settings.player_level <= 0.001 {
                    ui::Control::Hushed
                } else if self.settings.player_level < 0.5 {
                    ui::Control::Soft
                } else {
                    ui::Control::Loud
                };
                let level = iced::widget::canvas(ui::Level {
                    at: if self.settings.player_muted { 0.0 } else { self.settings.player_level },
                    alpha: ui::fade(),
                    on: Box::new(Message::PlayerLevel),
                })
                .width(64.0)
                .height(22.0);
                let keys = row![
                    ui::control_button(ui::Control::Earlier, 16.0, earlier, false),
                    ui::control_button(ui::Control::Back, 16.0, Some(Message::SeekBy(-5000)), false),
                    ui::control_button(if playing { ui::Control::Pause } else { ui::Control::Play }, 20.0, Some(Message::PlayerToggle), false),
                    ui::control_button(ui::Control::Ahead, 16.0, Some(Message::SeekBy(5000)), false),
                    ui::control_button(ui::Control::Later, 16.0, later, false),
                    container(clock).padding(Padding::ZERO.left(8.0)),
                    ui::grow(),
                    container(
                        iced::widget::canvas(ui::Speed {
                            at: self.settings.player_rate,
                            stops: player::RATES.to_vec(),
                            words: format!("{}×", w.rate(self.settings.player_rate)),
                            alpha: ui::fade(),
                            on: Box::new(Message::PlayerSpeed),
                        })
                        .width(118.0)
                        .height(26.0)
                    )
                    .padding(Padding::ZERO.right(6.0)),
                    ui::control_button(sound, 16.0, Some(Message::PlayerMute), self.settings.player_muted),
                    container(level).padding(Padding::ZERO.right(4.0)),
                    ui::control_button(ui::Control::Over, 16.0, Some(Message::PlayerLoop), self.settings.player_loop),
                    ui::control_button(
                        if self.widened.value() { ui::Control::Shrink } else { ui::Control::Grow },
                        16.0,
                        Some(Message::PlayerWiden),
                        false,
                    ),
                ]
                .spacing(2)
                .align_y(iced::Center);
                let under_picture = container(column![container(seek).padding(Padding::ZERO.right(4.0).left(4.0)), container(keys).height(40.0)].width(Length::Fill))
                    .width(Length::Fill)
                    .padding(Padding { top: 8.0, right: 10.0, bottom: 2.0, left: 10.0 })
                    .style(theme::under_picture(ui::fade()));
                let held = mouse_area(under_picture)
                    .on_enter(Message::ControlsHover(true))
                    .on_exit(Message::ControlsHover(false))
                    .on_press(Message::ControlsHover(true));
                container(ui::grown(held, iced::Point::new(0.5, 1.0), -(1.0 - out) * 12.0, 1.0))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_y(iced::alignment::Vertical::Bottom)
                    .into()
            }),
            false => Space::new().width(Length::Fill).height(Length::Fill).into(),
        };
        let hidden_pointer = playing && out < 0.01 && !self.asking_delete;
        let screen = mouse_area(
            container(stack![picture, mark, hint, controls].width(screen_w).height(screen_h))
                .width(screen_w)
                .height(screen_h)
                .style(ui::box_faded(theme::screen))
                .clip(true),
        )
        .interaction(if hidden_pointer { iced::mouse::Interaction::Hidden } else { iced::mouse::Interaction::Idle })
        .on_press(Message::PlayerToggle)
        .on_double_click(Message::PlayerWiden)
        .on_move(Message::PlayerStir)
        .on_scroll(|delta| {
            let up = match delta {
                iced::mouse::ScrollDelta::Lines { y, .. } => y,
                iced::mouse::ScrollDelta::Pixels { y, .. } => y / 40.0,
            };
            Message::PlayerLouder(up)
        });
        let mut named = row![text(name).font(theme::SANS_SEMI).size(theme::LEAD).wrapping(text::Wrapping::None).color(ui::faded(INK))]
            .spacing(6)
            .align_y(iced::Center);
        for acronym in mods {
            named = named.push(mod_badge(acronym));
        }
        let title = row![
            column![
                named,
                text(ui::shortened(line, 62)).font(theme::SANS).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(MUTED)),
            ]
            .spacing(2),
            ui::grow(),
            ui::control_button(ui::Control::Close, 18.0, Some(Message::ClosePlayer), false),
        ]
        .spacing(12)
        .align_y(iced::Center);
        let mut inside = column![
            container(title)
                .height(STAGE_TOP)
                .align_y(iced::alignment::Vertical::Center)
                .padding(Padding { top: 2.0, right: PICTURE_INSET - 7.0, bottom: 0.0, left: PICTURE_INSET }),
            container(screen).width(Length::Fill).height(screen_h).center_x(Length::Fill),
        ]
        .width(Length::Fill);
        if under_h > 1.0 {
            inside = inside.push(
                container(buttons)
                    .padding(Padding { top: 0.0, right: PICTURE_INSET, bottom: 0.0, left: PICTURE_INSET })
                    .height(under_h)
                    .align_y(iced::alignment::Vertical::Center)
                    .clip(true),
            );
        }
        let card = container(inside).width(screen_w + 2.0 * PICTURE_INSET).height(screen_h + STAGE_TOP + under_h);
        let backdrop = mouse_area(ui::veil(theme::CINEMA_SCRIM)).on_press(Message::ClosePlayer);
        let opened = self.stage_open.interpolate(0.0, 1.0, self.now);
        let card = ui::grown(card, iced::Point::new(0.5, 0.5), -(1.0 - opened) * 14.0, 0.965 + 0.035 * opened);
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
        let k = self.ask_fade.interpolate(0.0, 1.0, self.now);
        ui::fading(k, || {
            let picture: Element<'_, Message> = match self.thumbs.get(&video.map_hash) {
                Some(handle) => image(handle.clone())
                    .content_fit(ContentFit::Cover)
                    .width(ASK_THUMB.0)
                    .height(ASK_THUMB.1)
                    .border_radius(10.0)
                    .opacity(ui::fade())
                    .into(),
                None => container(ui::fine_hatch()).width(ASK_THUMB.0).height(ASK_THUMB.1).into(),
            };
            let who = match video.map_line().is_empty() {
                true => format!("{} · {}", video.player, w.mb(video.size)),
                false => format!("{} — {} · {}", video.player, ui::shortened(video.map_line(), 44), w.mb(video.size)),
            };
            let middle = column![
                picture,
                container(text(w.t("delete-video")).font(theme::SANS_SEMI).size(20.0).color(ui::faded(INK))).padding(Padding::ZERO.top(16.0)),
                text(who).font(theme::SANS).size(theme::CAPTION).align_x(iced::Center).color(ui::faded(MUTED)),
                text(w.t("to-the-bin")).font(theme::SANS).size(11.0).align_x(iced::Center).color(ui::faded(FAINT)),
                container(
                    row![ui::quiet(w.t("keep"), Some(Message::KeepVideo)), ui::primary(w.t("delete"), Some(Message::DeleteVideo))]
                        .spacing(8)
                        .align_y(iced::Center)
                )
                .padding(Padding::ZERO.top(18.0)),
            ]
            .spacing(6)
            .align_x(iced::Center)
            .width(ASK_WIDE);
            let risen = ui::grown(middle, iced::Point::new(0.5, 0.5), (1.0 - k) * -10.0, 0.97 + 0.03 * k);
            stack![
                mouse_area(ui::veil(theme::DEEP_SCRIM)).on_press(Message::KeepVideo),
                container(risen).width(Length::Fill).height(Length::Fill).center(Length::Fill),
            ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        })
    }
}

const ASK_THUMB: (f32, f32) = (176.0, 99.0);
const ASK_WIDE: f32 = 400.0;
const VIDEO_THUMB: (u32, u32) = (96, 54);
pub const RETYPE: Duration = Duration::from_millis(900);
pub const MARK: Duration = Duration::from_millis(200);
pub const LANG_FADE: Duration = Duration::from_millis(150);

impl Main {
    fn rest_live(&mut self, away: bool) {
        if let Some(live) = &self.live {
            if away {
                live.control.pause(true);
            } else if !self.paused_by_hand {
                live.control.pause(false);
            }
        }
        if let Some(trail) = &mut self.trail {
            match (away, trail.paused_at) {
                (true, None) => trail.paused_at = Some(trail.at_ms(self.now)),
                (false, Some(at)) if !self.paused_by_hand => {
                    let span = (trail.to_ms - trail.from_ms).max(1.0);
                    trail.started = Instant::now() - Duration::from_secs_f64(((at - trail.from_ms) % span) / 1000.0);
                    trail.paused_at = None;
                }
                _ => {}
            }
        }
    }

    fn turn_to(&mut self, overlay: Overlay, now: Instant) {
        let up = overlay != Overlay::None;
        let was = self.ground_fade.interpolate(0.0, 1.0, now);
        if self.ground_fade.value() != up {
            let span = if up { GROUND_UP } else { OVERLAY_FADE };
            self.ground_fade = Animation::new(!up).duration(span).easing(Easing::EaseOutCubic).go(up, now);
            let _ = was;
        }
        let shown = self.overlay != Overlay::None;
        match (shown, overlay) {
            (_, Overlay::None) => {
                self.turning = None;
                self.overlay_fade.go_mut(false, now);
            }
            (false, _) => {
                self.turning = None;
                self.overlay_drawn = overlay;
                self.overlay_fade = Animation::new(false).duration(OVERLAY_FADE).easing(Easing::EaseOutCubic).go(true, now);
            }
            (true, _) => {
                self.turning = Some(overlay);
                self.overlay_fade = Animation::new(true).duration(OVERLAY_FADE / 2).easing(Easing::EaseOutCubic).go(false, now);
            }
        }
    }

    fn skin_look(&self) -> Task<Message> {
        let mut wanted: Vec<PathBuf> = self
            .skins
            .iter()
            .take(40)
            .filter(|folder| !self.skin_faces.contains_key(*folder))
            .cloned()
            .collect();
        if !self.skin_faces.contains_key(Path::new("")) {
            wanted.insert(0, PathBuf::new());
        }
        if wanted.is_empty() {
            return Task::none();
        }
        ui::streamed(move |push| {
            for folder in wanted {
                let handle = match folder.as_os_str().is_empty() {
                    true => Some(image::Handle::from_rgba(160, 160, crate::settings::own_skin_picture(160))),
                    false => crate::settings::skin_picture(&folder, 160).map(|rgba| image::Handle::from_rgba(160, 160, rgba)),
                };
                if !push(Message::SkinFace(folder, handle)) {
                    return;
                }
            }
        })
    }

    fn ease_slides(&mut self, dt: f32) {
        let wanted = self.slider_targets();
        for (id, target) in wanted {
            let settled = self
                .slid_at
                .get(&id)
                .map(|at| self.now.saturating_duration_since(*at).as_millis() > 160)
                .unwrap_or(true);
            let (shown, snap) = self.slides.entry(id).or_insert((target, 1.0));
            *shown = ui::toward(*shown, target, 0.28, dt);
            let want = if settled && (target - *shown).abs() < 0.01 { 1.0 } else { 0.0 };
            *snap = ui::toward(*snap, want, 0.25, dt);
        }
    }

    fn combo_for_rested(&mut self, now: Instant) -> Task<Message> {
        let Some((at, since)) = self.hover_since else {
            return Task::none();
        };
        if now.saturating_duration_since(since) < HOVER_REST || self.hover != Some(at) {
            return Task::none();
        }
        self.hover_since = None;
        match self.entries().get(at) {
            Some(entry) if !self.combos.contains_key(&entry.path) => {
                let (path, map, hash) = (entry.path.clone(), entry.map.clone(), entry.map_hash.clone());
                self.combos.insert(path.clone(), None);
                ui::in_thread(move || Message::MaxCombo(path.clone(), max_combo_of(&path, map.as_ref(), &hash)))
            }
            _ => Task::none(),
        }
    }

    fn slides_settling(&self) -> bool {
        self.slider_targets().into_iter().any(|(id, target)| {
            let Some(&(shown, snap)) = self.slides.get(&id) else {
                return true;
            };
            let recent = self.slid_at.get(&id).is_some_and(|at| self.now.saturating_duration_since(*at).as_millis() <= 160);
            let want = if !recent && (target - shown).abs() < 0.01 { 1.0 } else { 0.0 };
            recent || (target - shown).abs() > 0.0005 || (want - snap).abs() > 0.0005
        })
    }

    fn slider_targets(&self) -> Vec<(String, f32)> {
        use crate::settings::{CRFS, HEIGHTS, RATES};
        let at = |value: u32, of: &[u32]| {
            let last = (of.len().max(2) - 1) as f32;
            of.iter().position(|v| *v == value).map_or(0.5, |i| i as f32 / last)
        };
        vec![
            ("height".to_owned(), at(self.settings.render_height, &HEIGHTS)),
            ("rate".to_owned(), at(self.settings.render_fps, &RATES)),
            ("crf".to_owned(), at(self.settings.render_crf, &CRFS)),
            ("music".to_owned(), self.settings.music_level),
            ("hits".to_owned(), self.settings.hitsound_level),
            ("player".to_owned(), self.settings.player_level),
            ("dim".to_owned(), self.settings.background_dim),
            ("blur".to_owned(), self.settings.background_blur),
        ]
    }

    fn remember_mark(&mut self, id: &str, on: bool) {
        let now = Instant::now();
        self.marks
            .entry(id.to_owned())
            .or_insert_with(|| Animation::new(!on).duration(MARK).easing(Easing::EaseOutCubic))
            .go_mut(on, now);
    }

    fn news_heard(&mut self, source: &str) {
        self.now_unix = unix_now();
        self.news_failed.remove(source);
        self.news.mark(source, unix_now());
        self.news.save();
    }

    fn fetch_channel(&mut self, channel: String) -> Task<Message> {
        self.news_loading.insert(crate::news::channel_source(&channel));
        ui::in_thread(move || Message::NewsPosts(channel.clone(), crate::news::fetch_posts(&channel)))
    }

    fn refresh_news(&mut self, every: bool) -> Task<Message> {
        use crate::news;
        let now = unix_now();
        let due = |main: &Main, source: &str| (every || main.news.stale(source, now)) && !main.news_loading.contains(source);
        let mut tasks = Vec::new();
        if due(self, news::UPDATES) {
            self.news_loading.insert(news::UPDATES.to_owned());
            tasks.push(ui::in_thread(|| Message::NewsBuilds(news::fetch_builds())));
        }
        if due(self, news::STORIES) {
            self.news_loading.insert(news::STORIES.to_owned());
            tasks.push(ui::in_thread(|| Message::NewsStories(news::fetch_stories())));
        }
        for channel in self.settings.news_channels.clone() {
            if due(self, &news::channel_source(&channel)) {
                tasks.push(self.fetch_channel(channel));
            }
        }
        Task::batch(tasks)
    }

    fn news_pictures_task(&mut self) -> Task<Message> {
        let mut wanted: Vec<(String, (u32, u32))> = Vec::new();
        let stories = self.news.stories.iter().take(4).filter_map(|story| story.image.clone()).map(|url| (url, (192, 108)));
        let posts = self.news.posts.iter().take(12).filter_map(|post| post.cover().map(str::to_owned)).map(|url| (url, (128, 128)));
        for (url, size) in stories.chain(posts) {
            if !self.news_pictures.contains_key(&url) && self.news_asked.insert(url.clone()) {
                wanted.push((url, size));
            }
        }
        if wanted.is_empty() {
            return Task::none();
        }
        let mut lanes: Vec<Vec<(String, (u32, u32))>> = vec![Vec::new(); PICTURE_LANES];
        for (at, one) in wanted.into_iter().enumerate() {
            lanes[at % PICTURE_LANES].push(one);
        }
        Task::batch(lanes.into_iter().filter(|lane| !lane.is_empty()).map(|lane| {
            ui::streamed(move |push| {
                for (url, (w, h)) in lane {
                    let handle = crate::news::picture(&url).and_then(|bytes| covered_bytes(&bytes, w, h));
                    if !push(Message::NewsPicture(url, handle)) {
                        return;
                    }
                }
            })
        }))
    }

    fn osu_task(&mut self, force: bool) -> Task<Message> {
        let now = Instant::now();
        if !force && self.osu_asked.is_some_and(|at| now.saturating_duration_since(at) < Duration::from_secs(600)) {
            return Task::none();
        }
        let name = self
            .community_card
            .as_ref()
            .map(|card| card.username.clone())
            .filter(|name| !name.is_empty())
            .or_else(|| self.community.as_ref().and_then(|catalog| catalog.you()).map(|you| you.name.clone()))
            .or_else(|| self.osu_card.as_ref().map(|card| card.username.clone()))
            .unwrap_or_default();
        if name.trim().is_empty() {
            return Task::none();
        }
        self.osu_asked = Some(now);
        ui::in_thread(move || Message::OsuProfile(crate::osu_profile::fetch(&name)))
    }

    fn dress_staged_you(&mut self) {
        let Some(card) = self.osu_card.clone() else {
            return;
        };
        let Some(catalog) = self.community.as_mut().filter(|catalog| catalog.staged) else {
            return;
        };
        let dress = |person: &mut crate::community::Person| {
            person.name = card.username.clone();
            person.pp = card.pp.round() as u32;
            person.rank = card.global_rank as u32;
            person.accuracy = card.accuracy as f32;
            person.plays = card.play_count as u32;
            person.hours = (card.play_seconds / 3600.0) as u32;
            person.score = card.ranked_score as u64;
            person.country = card.country.clone();
            person.level = card.level as u32;
            person.ss = (card.grade_counts.ss + card.grade_counts.ssh) as u32;
            person.s = (card.grade_counts.s + card.grade_counts.sh) as u32;
            person.avatar = card.avatar_url.clone();
            person.cover = card.cover_url.clone();
            person.supporter = card.is_supporter;
            person.joined = crate::news::unix_of(&card.join_date).unwrap_or(0);
            person.gained = [0.0; 6];
        };
        if let Some(you) = catalog.people.iter_mut().find(|person| person.you) {
            dress(you);
        }
        if let Some(me) = catalog.me.as_mut() {
            dress(&mut me.person);
            me.history.clear();
            me.activity.clear();
        }
    }

    fn newest_event(&self) -> i64 {
        self.community.as_ref().map_or(0, |catalog| crate::chronicle::newest(catalog, &self.news, &self.settings.news_channels, self.live_shown))
    }

    fn first_community(&mut self) -> crate::community::Catalog {
        if self.settings.token.is_empty() {
            self.community_fetch = crate::community_screen::Fetch::Staged;
            return self.staged_community();
        }
        match crate::community::wire::load() {
            Some(kept) => {
                self.community_card = crate::community::wire::load_card();
                self.remerge();
                self.community_fetch = crate::community_screen::Fetch::Fresh(kept.at.unwrap_or(self.now_unix));
                let catalog = crate::community::Catalog::from_wire(kept);
                self.live_shown = catalog.live.len();
                catalog
            }
            None => {
                self.community_fetch = crate::community_screen::Fetch::Loading;
                let mut catalog = self.staged_community();
                catalog.people.clear();
                catalog.live.clear();
                catalog.feed.clear();
                catalog.friends.clear();
                catalog.group.clear();
                catalog.staged = false;
                catalog.me = None;
                catalog.friends_state = crate::community::Friends::Waiting;
                catalog
            }
        }
    }

    fn community_task(&mut self, force: bool) -> Task<Message> {
        if self.settings.token.is_empty() {
            self.community_fetch = crate::community_screen::Fetch::Staged;
            return Task::none();
        }
        let now = Instant::now();
        if !force && self.community_asked.is_some_and(|at| now.saturating_duration_since(at) < COMMUNITY_EVERY - Duration::from_secs(2)) {
            return Task::none();
        }
        self.community_asked = Some(now);
        if !matches!(self.community_fetch, crate::community_screen::Fetch::Fresh(_)) {
            self.community_fetch = crate::community_screen::Fetch::Loading;
        }
        let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        let chat = self.settings.chat_id.filter(|id| *id < 0);
        ui::in_thread(move || Message::CommunityArrived(crate::bot::community(&server, &token, &name, chat).map_err(|e| e.to_string())))
    }

    fn friends_task(&mut self, force: bool) -> Task<Message> {
        if self.settings.token.is_empty() || self.community.as_ref().is_none_or(|catalog| catalog.staged) {
            return Task::none();
        }
        let now = Instant::now();
        if !force && self.friends_asked.is_some_and(|at| now.saturating_duration_since(at) < FRIENDS_EVERY) {
            return Task::none();
        }
        self.friends_asked = Some(now);
        if let Some(catalog) = self.community.as_mut() {
            if !matches!(catalog.friends_state, crate::community::Friends::Ready) {
                catalog.friends_state = crate::community::Friends::Waiting;
            }
        }
        let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        ui::in_thread(move || Message::FriendsArrived(crate::bot::friends(&server, &token, &name).map_err(|e| e.to_string())))
    }

    fn card_task(&mut self, force: bool) -> Task<Message> {
        if self.settings.token.is_empty() {
            return Task::none();
        }
        let now = Instant::now();
        if !force && self.card_asked.is_some_and(|at| now.saturating_duration_since(at) < CARD_EVERY) {
            return Task::none();
        }
        self.card_asked = Some(now);
        let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        let chat = self.settings.chat_id.filter(|id| *id < 0);
        ui::in_thread(move || Message::CardArrived(crate::bot::card(&server, &token, &name, chat).map_err(|e| e.to_string())))
    }

    fn flag_task(&mut self) -> Task<Message> {
        let mut codes: Vec<String> = Vec::new();
        if let Some(card) = self.shown_card.as_ref() {
            codes.push(card.country.clone());
        }
        if let Some(catalog) = self.community.as_ref() {
            codes.extend(catalog.people.iter().map(|person| person.country.clone()));
            codes.extend(catalog.friends.iter().map(|friend| friend.country.clone()));
        }
        let wanted: Vec<(String, String)> = codes
            .into_iter()
            .map(|code| code.trim().to_ascii_lowercase())
            .filter_map(|code| crate::community::wire::flag_url(&code).map(|url| (code, url)))
            .filter(|(code, _)| !self.flags.contains_key(code) && self.flags_asked.insert(code.clone()))
            .collect();
        if wanted.is_empty() {
            return Task::none();
        }
        ui::streamed(move |push| {
            for (code, url) in wanted {
                let bytes = flag_bytes(&code, &url);
                if !push(Message::Flag(code, bytes)) {
                    return;
                }
            }
        })
    }

    pub fn remerge(&mut self) {
        self.shown_card = crate::community::wire::enriched(self.community_card.as_ref(), self.osu_card.as_ref());
    }

    fn person_task(&mut self, at: usize) -> Task<Message> {
        let Some(person) = self.community.as_ref().and_then(|catalog| catalog.people.get(at)).cloned() else {
            return Task::none();
        };
        let staged = self.community.as_ref().is_none_or(|catalog| catalog.staged);
        let chat = self.settings.chat_id.filter(|id| *id < 0).or_else(|| self.community.as_ref().and_then(|catalog| catalog.chat));
        match (staged || self.settings.token.is_empty(), chat) {
            (false, Some(chat)) => {
                if self.people_dossiers.contains_key(&person.id) || !self.people_asked.insert(format!("me:{}", person.id)) {
                    return Task::none();
                }
                let (server, token, device, id) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone(), person.id);
                let asked = ui::in_thread(move || Message::PersonDossier(id, crate::bot::person(&server, &token, &device, chat, id).map_err(|e| e.to_string())));
                match person.app {
                    true => asked,
                    false => Task::batch([asked, self.scrape_task(at)]),
                }
            }
            _ => self.scrape_task(at),
        }
    }

    fn scrape_task(&mut self, at: usize) -> Task<Message> {
        let Some(person) = self.community.as_ref().and_then(|catalog| catalog.people.get(at)).cloned() else {
            return Task::none();
        };
        let name = person.name.to_lowercase();
        if !self.people_asked.insert(format!("card:{name}")) {
            return Task::none();
        }
        let asked = person.name.clone();
        ui::in_thread(move || Message::PersonCard(asked.to_lowercase(), crate::osu_profile::fetch(&asked)))
    }

    fn share_task(&mut self) -> Task<Message> {
        let staged = self.community.as_ref().is_none_or(|catalog| catalog.staged);
        let Some(card) = self.osu_card.clone().filter(|_| !staged && !self.settings.token.is_empty()) else {
            return Task::none();
        };
        let (server, token, device) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        ui::in_thread(move || Message::CardShared(crate::bot::share_card(&server, &token, &device, &card).map_err(|e| e.to_string())))
    }

    fn pictures_task(&mut self, wanted: Vec<(String, u32)>) -> Task<Message> {
        let wanted: Vec<(String, u32)> = wanted.into_iter().filter(|(url, _)| !url.is_empty() && !self.news_pictures.contains_key(url) && self.news_asked.insert(url.clone())).collect();
        if wanted.is_empty() {
            return Task::none();
        }
        ui::streamed(move |push| {
            for (url, side) in wanted {
                let handle = crate::news::picture(crate::community::fetched(&url)).and_then(|bytes| match side {
                    crate::community::BACKDROP => backdrop_bytes(&bytes, side),
                    crate::community::POSTER => poster_bytes(&bytes),
                    side if side <= 256 => covered_bytes(&bytes, side, side),
                    side => fitted_bytes(&bytes, side),
                });
                if !push(Message::NewsPicture(url, handle)) {
                    return;
                }
            }
        })
    }

    fn community_pictures_task(&mut self) -> Task<Message> {
        let Some(catalog) = self.community.as_ref() else {
            return Task::none();
        };
        let mut wanted: Vec<(String, u32)> = catalog.pictures();
        if let Some(card) = self.shown_card.clone().or_else(|| catalog.card_of()) {
            wanted.extend(card.pictures());
        }
        let wanted: Vec<(String, u32)> = wanted.into_iter().filter(|(url, _)| !self.news_pictures.contains_key(url) && self.news_asked.insert(url.clone())).collect();
        let flag = self.flag_task();
        if wanted.is_empty() {
            return flag;
        }
        let mut wanted = wanted;
        wanted.sort_by_key(|(_, side)| match *side {
            side if side <= 256 => 0,
            crate::community::POSTER => 1,
            crate::community::BACKDROP => 3,
            _ => 2,
        });
        let mut lanes: Vec<Vec<(String, u32)>> = vec![Vec::new(); PICTURE_LANES];
        for (at, one) in wanted.into_iter().enumerate() {
            lanes[at % PICTURE_LANES].push(one);
        }
        let pictures = lanes.into_iter().filter(|lane| !lane.is_empty()).map(|lane| {
            ui::streamed(move |push| {
                for (url, side) in lane {
                    let handle = crate::news::picture(crate::community::fetched(&url)).and_then(|bytes| match side {
                        crate::community::BACKDROP => backdrop_bytes(&bytes, side),
                        crate::community::POSTER => poster_bytes(&bytes),
                        side if side <= 256 => covered_bytes(&bytes, side, side),
                        side => fitted_bytes(&bytes, side),
                    });
                    if !push(Message::NewsPicture(url, handle)) {
                        return;
                    }
                }
            })
        });
        Task::batch(pictures.chain(std::iter::once(flag)))
    }

    fn wide_pictures_task(&mut self, urls: Vec<String>) -> Task<Message> {
        let wanted: Vec<String> = urls.into_iter().filter(|url| self.news_asked.insert(crate::community_screen::wide(url))).collect();
        if wanted.is_empty() {
            return Task::none();
        }
        ui::streamed(move |push| {
            for url in wanted {
                let handle = crate::news::picture(&url).and_then(|bytes| fitted_bytes(&bytes, 1400));
                if !push(Message::NewsPicture(crate::community_screen::wide(&url), handle)) {
                    return;
                }
            }
        })
    }

    pub fn staged_community(&self) -> crate::community::Catalog {
        let mut seen = std::collections::HashSet::new();
        let maps: Vec<crate::community::MapRef> = self
            .entries()
            .iter()
            .filter(|entry| entry.map.as_ref().is_some_and(|map| map.background.is_some()))
            .filter(|entry| seen.insert(entry.map_hash.clone()))
            .take(8)
            .filter_map(|entry| entry.map.as_ref().map(|map| crate::community::MapRef::local(entry.map_hash.clone(), map.line())))
            .collect();
        let you = self
            .account
            .as_ref()
            .map(|me| me.username.clone())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| self.settings.linked_as.clone());
        crate::community::Catalog::staged(maps, &you, self.now_unix)
    }

    fn community_view(&self) -> Option<Element<'_, Message>> {
        let catalog = self.community.as_ref()?;
        let ground = crate::community_screen::Ground {
            words: &self.words,
            catalog,
            thumbs: &self.thumbs,
            section: self.community_section,
            board: self.community_board,
            person: self.community_person,
            person_k: self.person_fade.interpolate(0.0, 1.0, self.now),
            person_card: self.community_person.and_then(|at| self.community.as_ref()?.people.get(at)).and_then(|person| self.people_cards.get(&person.name.to_lowercase())),
            person_dossier: self.community_person.and_then(|at| self.community.as_ref()?.people.get(at)).and_then(|person| self.people_dossiers.get(&person.id)),
            person_loading: self.community_person.and_then(|at| self.community.as_ref()?.people.get(at)).is_some_and(|person| self.people_asked.contains(&format!("card:{}", person.name.to_lowercase())) || self.people_asked.contains(&format!("me:{}", person.id))),
            now_unix: self.now_unix,
            news: &self.news,
            pictures: &self.news_pictures,
            loading: &self.news_loading,
            failed: &self.news_failed,
            channels: &self.settings.news_channels,
            channel_draft: &self.channel_draft,
            fetch: self.community_fetch,
            width: self.width / COMMUNITY_SCALE,
            filter: self.feed_filter,
            source: self.feed_source,
            stream: self.feed_stream,
            query: &self.feed_query,
            open_events: &self.feed_open,
            seen: self.feed_seen,
            spot: self.spot,
            spot_k: {
                let k = (self.now.saturating_duration_since(self.spot_at).as_secs_f32() / crate::chronicle::SPOT_SWAP.as_secs_f32()).clamp(0.0, 1.0);
                1.0 - (1.0 - k).powi(3)
            },
            section_t: self.now.saturating_duration_since(self.section_at).as_secs_f32().min(60.0),
            shift_t: self.now.saturating_duration_since(self.shift_at).as_secs_f32().min(60.0),
            person_t: self.now.saturating_duration_since(self.person_at).as_secs_f32().min(60.0),
            play_t: self.now.saturating_duration_since(self.play_at).as_secs_f32().min(60.0),
            rank: self.rank,
            rank_k: {
                let k = (self.now.saturating_duration_since(self.rank_at).as_secs_f32() / crate::chronicle::RANK_GROW.as_secs_f32()).clamp(0.0, 1.0);
                1.0 - (1.0 - k).powi(3)
            },
            rank_started: self.rank_due,
            rank_held: self.rank_held.map(|since| 1.0 - (since.saturating_duration_since(self.rank_due).as_secs_f32() / crate::chronicle::RANK_EVERY.as_secs_f32()).clamp(0.0, 1.0)),
            metric: self.dossier_metric,
            span: self.dossier_span,
            grade_hover: self.grade_hover,
            title_pick: self.title_pick.as_deref(),
            play_open: self.play_open,
            clips_loading: &self.clips_loading,
            reading: self.community_reading.as_ref(),
            read_k: self.read_fade.interpolate(0.0, 1.0, self.now),
            people_from: self.people_from,
            standing: self.community_standing,
            card: self.shown_card.as_ref(),
            flags: &self.flags,
            avatar: self.avatar.as_ref(),
            chat: self.chat_name(),
            live_shown: self.live_shown,
            live_k: self.live_arrived.map_or(1.0, |at| {
                let k = (self.now.saturating_duration_since(at).as_secs_f32() / LIVE_ARRIVE.as_secs_f32()).clamp(0.0, 1.0);
                1.0 - (1.0 - k) * (1.0 - k)
            }),
        };
        let opened = self.stage_open.interpolate(0.0, 1.0, self.now);
        let stage = match (&self.player, &self.clip) {
            (Some(player), Some(clip)) if opened > 0.001 => Some(ui::fading(ui::fade() * opened, || self.stage(&player.borrow(), self.clip_bill(clip)))),
            _ => None,
        };
        let body: Element<'_, Message> = crate::community_screen::view(&ground).map(Message::Community);
        let body: Element<'_, Message> = ui::scaled(body, COMMUNITY_SCALE).into();
        let page: Element<'_, Message> = column![Space::new().height(theme::CONTROL_HEIGHT + 4.0 + 22.0), body].width(Length::Fill).height(Length::Fill).into();
        let stage = stage.unwrap_or_else(|| Space::new().width(Length::Fill).height(Length::Fill).into());
        Some(stack![page, stage].width(Length::Fill).height(Length::Fill).into())
    }

    fn settings_view(&self) -> Element<'_, Message> {
        let ground = prefs::Ground {
            words: &self.words,
            settings: &self.settings,
            side: self.side,
            replays: self.entries().len(),
            videos: self.store.videos.len(),
            videos_size: self.sizes.0.max(self.store.total_size()),
            maps: self.library.as_ref().map_or(0, |l| l.maps),
            maps_size: self.sizes.1,
            cache_size: self.sizes.2,
            storage: &self.storage,
            ffmpeg: self.ffmpeg_version.clone(),
            account: self.account.as_ref(),
            avatar: self.avatar.as_ref(),
            chats: &self.chats,
            chat_faces: &self.chat_faces,
            renaming: self.renaming.as_ref(),
            skins: &self.skins,
            skin_faces: &self.skin_faces,
            marks: &self.marks_now,
            slides: &self.slides,
            came: self.overlay_fade.interpolate(0.0, 1.0, self.now),
            swap: self.side_fade.interpolate(0.0, 1.0, self.now),
            swap_from: self.side_swap,
        };
        let body: Element<'_, Message> = Element::from(prefs::view(&ground)).map(Message::Prefs);
        let sheet = column![Space::new().height(theme::CONTROL_HEIGHT + 4.0 + 22.0), body].width(Length::Fill).height(Length::Fill);
        sheet.into()
    }

    fn prefs(&mut self, message: prefs::Message) -> Task<Message> {
        use prefs::Message as P;
        let keep = |settings: &Settings| {
            let _ = settings.save();
        };
        match message {
            P::Side(side) => {
                if side != self.side {
                    self.side_swap = if side == Side::Bot { 1.0 } else { -1.0 };
                    self.side_fade = Animation::new(false).duration(TAB_FADE).easing(Easing::EaseOutCubic).go(true, Instant::now());
                }
                self.side = side;
                let name = if side == Side::Bot { "bot" } else { "app" };
                if self.settings.settings_tab != name {
                    self.settings.settings_tab = name.to_owned();
                    keep(&self.settings);
                }
                Task::none()
            }
            P::PickLang(lang) => {
                self.remember_mark("lang-ru", lang == crate::lang::Lang::Ru);
                self.remember_mark("lang-en", lang == crate::lang::Lang::En);
                if lang == self.settings.lang {
                    return Task::none();
                }
                self.settings.lang = lang;
                keep(&self.settings);
                self.words = Words::new(lang);
                self.retype = Animation::new(true);
                let now = Instant::now();
                self.overlay_fade = Animation::new(true).duration(LANG_FADE).easing(Easing::EaseOutCubic).go(false, now);
                self.lang_swap = true;
                Task::none()
            }
            P::Rename(said) => {
                self.renaming = Some(said);
                Task::none()
            }
            P::RenameDone => {
                if let Some(said) = self.renaming.take() {
                    let tidy = said.trim().to_owned();
                    if !tidy.is_empty() {
                        self.settings.device = tidy;
                        keep(&self.settings);
                    }
                }
                Task::none()
            }
            P::Scene(on) => {
                self.remember_mark("live", on);
                self.settings.live_scene = on;
                keep(&self.settings);
                if on {
                    self.start_live()
                } else {
                    if let Some(live) = self.live.take() {
                        live.control.stop();
                    }
                    self.trail = None;
                    Task::none()
                }
            }
            P::PauseUnfocused(on) => {
                self.remember_mark("pause", on);
                self.settings.pause_unfocused = on;
                keep(&self.settings);
                Task::none()
            }
            P::Height(at) => {
                self.slid_at.insert("height".to_owned(), Instant::now());
                self.settings.render_height = prefs::nearest(at, &crate::settings::HEIGHTS);
                keep(&self.settings);
                Task::none()
            }
            P::Rate(at) => {
                self.slid_at.insert("rate".to_owned(), Instant::now());
                self.settings.render_fps = prefs::nearest(at, &crate::settings::RATES);
                keep(&self.settings);
                Task::none()
            }
            P::Crf(at) => {
                self.slid_at.insert("crf".to_owned(), Instant::now());
                self.settings.render_crf = prefs::nearest(at, &crate::settings::CRFS);
                keep(&self.settings);
                Task::none()
            }
            P::Source(at, on) => {
                self.remember_mark(&format!("source-{at}"), on);
                if let Some(source) = self.settings.sources.get_mut(at) {
                    source.on = on;
                    keep(&self.settings);
                }
                let sources = self.settings.sources.clone();
                ui::in_thread(move || library::read(&sources)).map(Message::Loaded)
            }
            P::AddFolder => Task::perform(prefs::pick_folder(), |found| Message::Prefs(P::Added(found))),
            P::Added(found) => {
                let Some(source) = found else {
                    return Task::none();
                };
                if self.settings.sources.iter().any(|s| s.root == source.root) {
                    return Task::none();
                }
                self.settings.sources.push(source);
                keep(&self.settings);
                let sources = self.settings.sources.clone();
                ui::in_thread(move || library::read(&sources)).map(Message::Loaded)
            }
            P::OpenRenders => {
                let _ = open::that_detached(self.settings.renders_dir());
                Task::none()
            }
            P::PickRenders => Task::perform(prefs::pick_renders(), |picked| Message::Prefs(P::PickedRenders(picked))),
            P::PickedRenders(picked) => {
                let Some(root) = picked else {
                    return Task::none();
                };
                self.settings.renders_dir = Some(root.clone());
                keep(&self.settings);
                self.store = videos::Store::at(root.join("videos.json"));
                let strays = videos::strays(&self.store.videos, &root);
                match (self.ffmpeg.clone(), strays.is_empty()) {
                    (Some(ffmpeg), false) => ui::in_thread(move || Message::Adopted(strays.iter().filter_map(|p| videos::adopt(&ffmpeg, p)).collect())),
                    _ => Task::none(),
                }
            }
            P::OpenMaps => {
                let _ = open::that_detached(crate::sources::own_root().join("Songs"));
                Task::none()
            }
            P::ClearCache => {
                prefs::clear_cache();
                self.sizes.2 = 0;
                Task::none()
            }
            P::OpenData => {
                let _ = open::that_detached(crate::sources::own_root());
                Task::none()
            }
            P::ClearAppCache => {
                prefs::clear_app_cache();
                self.sizes.2 = 0;
                self.flags.clear();
                self.flags_asked.clear();
                let renders = self.settings.renders_dir();
                ui::in_thread(move || Message::Stored(prefs::measure(&renders)))
            }
            P::CheckBuild => {
                let _ = open::that_detached("https://github.com/NaumRedlo/Dossier/releases");
                Task::none()
            }
            P::GetFfmpeg => {
                if self.ffmpeg.is_some() {
                    let _ = open::that_detached(crate::ffmpeg::own_dir());
                    return Task::none();
                }
                Task::run(crate::first_run::fetching_ffmpeg(), |step| match step {
                    crate::ffmpeg::Step::Done(path) => Message::Ffmpeg(crate::checks::ffmpeg_version(&path)),
                    _ => Message::Retype(Instant::now()),
                })
            }
            P::Chat(id) => {
                let was = self.settings.chat_id.or(self.account.as_ref().map(|me| me.telegram_id));
                if let Some(was) = was {
                    self.remember_mark(&format!("chat-{was}"), false);
                }
                self.remember_mark(&format!("chat-{id}"), true);
                self.settings.chat_id = Some(id);
                self.settings.chat_title = self.chats.iter().find(|chat| chat.id == id).map(|chat| chat.title.clone()).unwrap_or_default();
                keep(&self.settings);
                Task::none()
            }
            P::Worker(_) => Task::none(),
            P::Skin(folder) => {
                let folder = match folder.as_deref().filter(|path| crate::settings::is_skin_file(path)) {
                    Some(file) => match crate::settings::unpack_skin(file) {
                        Ok(made) => {
                            let sources = self.settings.sources.clone();
                            let own = self.settings.own_skins.clone();
                            let again = ui::in_thread(move || Message::Skins(crate::settings::hunt_skins(&sources, &own)));
                            return Task::batch([self.prefs(P::Skin(Some(made))), again]);
                        }
                        Err(why) => {
                            self.say_trouble(why);
                            return Task::none();
                        }
                    },
                    None => folder,
                };
                let name = |path: &Option<PathBuf>| match path {
                    Some(path) => format!("skin-{}", crate::settings::skin_name(path)),
                    None => "skin-own".to_owned(),
                };
                self.remember_mark(&name(&self.settings.skin), false);
                self.remember_mark(&name(&folder), true);
                self.settings.skin = folder;
                keep(&self.settings);
                self.skin_again()
            }
            P::RescanSkins => self.look_for_skins(),
            P::AddSkin => Task::perform(prefs::pick_skin(), |picked| Message::Prefs(P::AddedSkin(picked))),
            P::MoreSkins => self.update(Message::ShowSkins(true)),
            P::AddedSkin(picked) => {
                let Some(picked) = picked else {
                    return Task::none();
                };
                let picked = match picked.is_file() && !crate::settings::is_skin_file(&picked) {
                    true => picked.parent().map(Path::to_path_buf).unwrap_or(picked),
                    false => picked,
                };
                let mut taken: Vec<PathBuf> = Vec::new();
                if crate::settings::is_skin_file(&picked) || crate::settings::looks_like_skin(&picked) {
                    match crate::settings::is_skin_file(&picked) {
                        true => match crate::settings::unpack_skin(&picked) {
                            Ok(made) => taken.push(made),
                            Err(why) => {
                                self.say_trouble(why);
                                return Task::none();
                            }
                        },
                        false => taken.push(picked.clone()),
                    }
                } else {
                    taken.extend(crate::settings::skins_under(&picked));
                }
                let Some(first) = taken.first().cloned() else {
                    self.announce(notices::Mark::Bad, self.words.t("no-skin-there"), crate::settings::skin_name(&picked), String::new(), String::new(), notices::Link::None);
                    return Task::none();
                };
                for folder in taken {
                    if !self.settings.own_skins.contains(&folder) {
                        self.settings.own_skins.push(folder);
                    }
                }
                self.settings.skin = Some(first);
                keep(&self.settings);
                Task::batch([self.look_for_skins(), self.skin_again()])
            }
            P::OpenSkinsFolder => {
                let root = crate::settings::skins_root();
                let _ = std::fs::create_dir_all(&root);
                let _ = open::that_detached(root);
                Task::none()
            }
            P::OpenSkin => {
                if let Some(folder) = &self.settings.skin {
                    let _ = open::that_detached(folder);
                }
                Task::none()
            }
            P::Dim(level) => {
                self.slid_at.insert("dim".to_owned(), Instant::now());
                self.settings.background_dim = level.clamp(0.0, 1.0);
                keep(&self.settings);
                Task::none()
            }
            P::Blur(level) => {
                self.slid_at.insert("blur".to_owned(), Instant::now());
                self.settings.background_blur = level.clamp(0.0, 1.0);
                keep(&self.settings);
                Task::none()
            }
            P::Hud(on) => {
                self.remember_mark("hud", on);
                self.settings.hud = on;
                keep(&self.settings);
                Task::none()
            }
            P::CursorGrows(on) => {
                self.remember_mark("cursor-grows", on);
                self.settings.cursor_grows = on;
                keep(&self.settings);
                Task::none()
            }
            P::MapSounds(on) => {
                self.remember_mark("map-sounds", on);
                self.settings.map_sounds = on;
                keep(&self.settings);
                Task::none()
            }
            P::SkinSounds(on) => {
                self.remember_mark("skin-sounds", on);
                self.settings.skin_sounds = on;
                keep(&self.settings);
                Task::none()
            }
            P::Music(level) => {
                self.slid_at.insert("music".to_owned(), Instant::now());
                self.settings.music_level = level.clamp(0.0, 1.0);
                keep(&self.settings);
                Task::none()
            }
            P::Hitsounds(level) => {
                self.slid_at.insert("hits".to_owned(), Instant::now());
                self.settings.hitsound_level = level.clamp(0.0, 1.0);
                keep(&self.settings);
                Task::none()
            }
            P::PlayerLevel(level) => {
                self.slid_at.insert("player".to_owned(), Instant::now());
                self.settings.player_level = level.clamp(0.0, 1.0);
                keep(&self.settings);
                if let Some(player) = &self.player {
                    player.borrow_mut().set_level(self.settings.player_level);
                }
                Task::none()
            }
            P::Unlink | P::SignOut => self.update(Message::SignOut),
            P::Moved(what, before) => {
                match self.side {
                    Side::App => self.settings.tiles_app = prefs::moved(&self.settings.tiles_app, &prefs::APP, what, before),
                    Side::Bot => self.settings.tiles_bot = prefs::moved(&self.settings.tiles_bot, &prefs::BOT, what, before),
                }
                keep(&self.settings);
                Task::none()
            }
        }
    }
}

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
                ui::disc(&letter, false, side)
            }
            (None, false) => ui::disc("", true, side),
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
            let mut under = handle.unwrap_or(if chat == "—" { w.t("linked-status") } else { chat });
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

    fn chats_task(&self) -> Task<Message> {
        if self.settings.token.is_empty() || !self.chats.is_empty() {
            return Task::none();
        }
        let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        ui::in_thread(move || Message::Chats(crate::bot::chats(&server, &token, &name).unwrap_or_default()))
    }

    fn chat_name(&self) -> String {
        let own = self.account.as_ref().map(|me| me.telegram_id);
        if let Some(id) = self.settings.chat_id.filter(|id| Some(*id) != own) {
            let known = self.chats.iter().find(|chat| chat.id == id).map(|chat| chat.title.clone());
            let remembered = Some(self.settings.chat_title.clone());
            return known.into_iter().chain(remembered).find(|title| !title.is_empty()).unwrap_or_else(|| "…".to_owned());
        }
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
                    ui::dot(8.0),
                    text(words).font(theme::SANS_SEMI).size(theme::CAPTION).color(ui::faded(INK)),
                    container(text(format!("· {detail}")).font(theme::SANS).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(MUTED))).width(Length::Fill).clip(true),
                ]
                .spacing(8)
                .align_y(iced::Center)
                .height(20.0),
                ui::thread(fraction),
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
                left = left.push(container(row![ui::dot(8.0), ui::mono_small(w.t("waiting-confirm"), MUTED)].spacing(8).align_y(iced::Center)).padding(Padding::ZERO.top(10.0)));
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

pub fn fitted_bytes(bytes: &[u8], widest: u32) -> Option<image::Handle> {
    let picture = ::image::load_from_memory(bytes).ok()?;
    let picture = if picture.width() > widest { picture.resize(widest, u32::MAX, ::image::imageops::FilterType::Lanczos3) } else { picture };
    let (width, height) = (picture.width(), picture.height());
    Some(image::Handle::from_rgba(width, height, picture.to_rgba8().into_raw()))
}

pub fn poster_bytes(bytes: &[u8]) -> Option<image::Handle> {
    let picture = ::image::load_from_memory(bytes).ok()?;
    let high = picture.height().min(300).max(1);
    let picture = picture.resize(u32::MAX, high, ::image::imageops::FilterType::Lanczos3);
    let (width, height) = (picture.width(), picture.height());
    let want = ((height as f32) * crate::community::POSTER_SHAPE).round() as u32;
    let picture = if width > want { picture.crop_imm((width - want) / 2, 0, want, height) } else {
        let tall = ((width as f32) / crate::community::POSTER_SHAPE).round().max(1.0) as u32;
        picture.crop_imm(0, height.saturating_sub(tall) / 2, width, tall.min(height))
    };
    let (width, height) = (picture.width(), picture.height());
    Some(image::Handle::from_rgba(width, height, picture.to_rgba8().into_raw()))
}

pub fn backdrop_bytes(bytes: &[u8], widest: u32) -> Option<image::Handle> {
    let picture = ::image::load_from_memory(bytes).ok()?;
    let picture = if picture.width() > widest { picture.resize(widest, u32::MAX, ::image::imageops::FilterType::Triangle) } else { picture };
    let mut soft = ::image::imageops::blur(&picture.to_rgba8(), 2.5);
    for pixel in soft.pixels_mut() {
        for channel in 0..3 {
            pixel.0[channel] = (f32::from(pixel.0[channel]) * 0.62) as u8;
        }
    }
    let (width, height) = soft.dimensions();
    Some(image::Handle::from_rgba(width, height, soft.into_raw()))
}

const PICTURE_LANES: usize = 6;

fn warm_pictures() -> Task<Message> {
    ui::in_thread(|| {
        std::thread::sleep(Duration::from_secs(4));
        let mut wanted: Vec<(String, u32)> = Vec::new();
        if let Some(kept) = crate::community::wire::load() {
            wanted.extend(crate::community::Catalog::from_wire(kept).pictures());
        }
        for card in [crate::community::wire::load_card(), crate::osu_profile::load()].into_iter().flatten() {
            wanted.extend(card.pictures());
        }
        let mut urls: Vec<String> = Vec::new();
        for (key, _) in wanted {
            let url = crate::community::fetched(&key).to_owned();
            if !url.is_empty() && !urls.contains(&url) {
                urls.push(url);
            }
        }
        let mut lanes: Vec<Vec<String>> = vec![Vec::new(); 4];
        for (at, url) in urls.into_iter().enumerate() {
            lanes[at % 4].push(url);
        }
        std::thread::scope(|scope| {
            for lane in &lanes {
                scope.spawn(move || {
                    for url in lane {
                        let _ = crate::news::picture(url);
                    }
                });
            }
        });
    })
    .discard()
}

fn flag_bytes(code: &str, url: &str) -> Option<Vec<u8>> {
    let path = crate::sources::own_root().join("cache").join("flags").join(format!("{code}.svg"));
    if let Ok(bytes) = std::fs::read(&path) {
        return Some(bytes);
    }
    let bytes = crate::news::picture(url)?;
    if let Some(folder) = path.parent() {
        let _ = std::fs::create_dir_all(folder);
    }
    let _ = std::fs::write(&path, &bytes);
    Some(bytes)
}

pub fn flag_shape(bytes: Vec<u8>) -> Vec<u8> {
    match String::from_utf8(bytes) {
        Ok(svg) => svg.replacen("viewBox=\"0 0 36 36\"", "viewBox=\"0 5 36 26\"", 1).into_bytes(),
        Err(error) => error.into_bytes(),
    }
}

pub fn covered_bytes(bytes: &[u8], width: u32, height: u32) -> Option<image::Handle> {
    let picture = ::image::load_from_memory(bytes).ok()?;
    let picture = picture.resize_to_fill(width, height, ::image::imageops::FilterType::Lanczos3).to_rgba8();
    Some(image::Handle::from_rgba(width, height, picture.into_raw()))
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
pub const CINEMA: Duration = Duration::from_millis(220);
pub const WIDEN: Duration = Duration::from_millis(260);
pub const HINT_SHOWN: Duration = Duration::from_millis(900);
pub const CONTROLS_FADE: Duration = Duration::from_millis(260);
pub const CONTROLS_IN: Duration = Duration::from_millis(180);
pub const CONTROLS_OUT: Duration = Duration::from_millis(460);
pub const STAGE_OPEN: Duration = Duration::from_millis(280);
pub const CONTROLS_STAY: Duration = Duration::from_secs(3);
pub const OVERLAY_FADE: Duration = Duration::from_millis(220);
pub const GROUND_UP: Duration = Duration::from_millis(110);
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
const STAGE_UNDER: f32 = 62.0;
const STAGE_TOP: f32 = 66.0;
const ROOM_TOP: f32 = 62.0;
const NAME_WIDTH: f32 = 7.9;
const ROOM_SIDE: f32 = 16.0;
const ROOM_GAP: f32 = 10.0;
const PATTERN: (u32, u32) = (520, 292);
const PANEL: (f32, f32) = (246.0, 138.0);
const PICTURE_INSET: f32 = 14.0;
const PICTURE_RADIUS: f32 = 10.0;

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

pub fn mod_badge<'a, Message: 'a>(acronym: &str) -> Element<'a, Message> {
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

#[cfg(test)]
mod tests {
    #[test]
    fn the_menu_names_the_chat_the_videos_go_to() {
        let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().next().expect("a staged screen");
        main.account = Some(crate::bot::Me { telegram_id: 7, name: "Naum Redlo".into(), username: "naumredlo".into(), avatar: false });
        main.chats = vec![crate::bot::Chat { id: -100, title: "Osu Squad".into(), private: false, photo: false }];
        assert_eq!(main.chat_name(), "@naumredlo");
        main.settings.chat_id = Some(-100);
        assert_eq!(main.chat_name(), "Osu Squad");
        main.chats.clear();
        main.settings.chat_title = "Osu Squad".into();
        assert_eq!(main.chat_name(), "Osu Squad", "the title is remembered before the chats arrive");
        main.settings.chat_id = Some(7);
        assert_eq!(main.chat_name(), "@naumredlo");
    }
}
