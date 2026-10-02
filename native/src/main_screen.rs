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
use crate::updates::State as UpdateState;

#[path = "sharing.rs"]
pub mod sharing;

pub const ENTER: Duration = Duration::from_millis(1200);
pub const ARRIVE: Duration = Duration::from_millis(450);
pub const SWAP: Duration = Duration::from_millis(450);
pub const LIFT: Duration = Duration::from_millis(200);
const LIVE_FIRST: usize = 6;
const FETCHES_AT_ONCE: usize = 3;
const LIVE_EVERY: Duration = Duration::from_secs(6);
const PANEL_SHOW: Duration = Duration::from_millis(420);
const FOLD: Duration = Duration::from_millis(280);
const COMMUNITY_EVERY: Duration = Duration::from_secs(30);
const HISTORY_EVERY: Duration = Duration::from_secs(600);
const COMPANION_DWELL: Duration = Duration::from_millis(900);
const COMPANION_TICK: Duration = Duration::from_millis(250);
const COMPANION_EDGE: f32 = 16.0;
const COMPANION_BELOW: f32 = 24.0;
const COMPANION_WIDE: f32 = 320.0;
const NEWS_EVERY: Duration = Duration::from_secs(60);
const FRIENDS_EVERY: Duration = Duration::from_secs(120);
const CARD_EVERY: Duration = Duration::from_secs(300);
const COMMUNITY_SCALE: f32 = 0.84;
const SIDE_OPEN: Duration = Duration::from_millis(220);
const SIDE_WAIT: Duration = Duration::from_millis(70);
const SIDE_DIM: f32 = 0.34;
const HOVER_REST: Duration = Duration::from_millis(160);
pub const LIVE_FADE: Duration = Duration::from_millis(640);
pub const BRAND_WIDTH: f32 = 144.0;
const CREST_HOME: (f32, f32) = (40.0, 24.0);
const CREST_RISE: f32 = 8.0;
const JOURNAL_RISE: f32 = 140.0;
const BUBBLE_W: f32 = 340.0;
const BUBBLE_H: f32 = 88.0;
const CARET: f32 = 8.0;
const SHARED_MARK: f32 = 20.0;
const SHARED_RING: f32 = 1.5;
const THUMB: (u32, u32) = (176, 100);
const SCENE_WIDTH: u32 = 960;
const REST_AFTER: Duration = Duration::from_secs(120);
const REST_FADE: Duration = Duration::from_millis(600);
const REST_BRAND_SCALE: f32 = 1.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    None,
    Videos,
    Community,
    Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    UserInput(Option<Box<Message>>),
    PointerActivity(Point),
    RestCheck(Instant),
    Community(crate::community_screen::Message),
    NewsBuilds(Result<Vec<crate::news::Build>, String>),
    NewsStories(Result<Vec<crate::news::Story>, String>),
    NewsThreads(Result<Vec<crate::news::Thread>, String>),
    NewsPosts(String, Result<Vec<crate::news::Post>, String>),
    NewsPicture(String, Option<image::Handle>),
    NewsFrost(String, crate::community_screen::Frost),
    LiveArrive,
    CommunityArrived(Result<crate::community::wire::Community, String>),
    MapBoard(u64, bool, Result<crate::community::wire::MapBoard, String>),
    EveryoneArrived(Result<crate::community::wire::Everyone, String>),
    PinRead(Result<crate::community::wire::Pin, String>),
    Pinned(Result<crate::community::wire::Pin, String>),
    FriendsArrived(Result<crate::bot::Friends, String>),
    CardArrived(Result<crate::community::wire::Card, String>),
    Flag(String, Option<Vec<u8>>),
    CommunityTick,
    FarmTick,
    WorkerSwitch(bool),
    Worked(crate::worker::Step),
    FarmHeard(Option<crate::bot::Farm>),
    UpdateTick,
    UpdateChecked(Result<Option<crate::updates::Release>, String>),
    UpdateGot(crate::updates::Step),
    UpdateIdle,
    NewsTick,
    FeedClock,
    ClipFetched(String, Result<(PathBuf, videos::Probe), String>),
    OsuProfile(Result<crate::community::wire::Card, String>),
    PersonCard(String, Result<crate::community::wire::Card, String>),
    PersonDossier(crate::dossier_cache::Request, Result<crate::community::wire::Me, String>),
    PersonDossierCached(crate::dossier_cache::Request, Option<crate::dossier_cache::Entry>),
    CardShared(Result<(), String>),
    Worn(Result<(), String>),
    Nudged,
    Donated(usize),
    ReadFirst,
    Loaded(Library),
    Reading(library::Reading),
    Refreshed(Library),
    WatchTick,
    AutoNext,
    Watched(u64),
    Thumb(String, image::Handle),
    MapKnown(String, crate::maps::Known),
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
    SkinDeleted(PathBuf, Result<(), String>),
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
    ClearNotices,
    SignIn,
    PairAsked(Result<(String, String, bool), Refused>),
    OpenOsu,
    LinkTelegram,
    TelegramAsked(Result<(String, String), String>),
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
    Sent(Result<bot::Sent, String>),
    Sharing(sharing::Message),
    Witness(crate::witness::Event),
    WitnessTold(bool),
    WitnessSitting(bool),
    HistoryTold(crate::history::Sent),
    CompanionTick,
    ToastHover(u64, bool),
    ToastClose(u64),
    OpenVideo(usize),
    VideoReady(PathBuf, Option<videos::Probe>),
    ClosePlayer,
    PlayerMinimize,
    PlayerRestore,
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
    Search(String),
    RevealVideo,
    FolderOpened(Result<(), String>),
    AskDelete,
    AskDeleteOf(usize),
    KeepVideo,
    DeleteVideo,
    Choose(usize),
    Step(i32),
    Escape,
    Hover(Option<usize>),
    SideOpen(bool),
    SideMoved(&'static str, Option<&'static str>),
    HoverLeft(usize),
    Over(usize, iced::Rectangle),
    HoverStaged(usize),
    Show(Overlay),
    OpenFolder,
    Render,
    StopRender,
    Unqueue(PathBuf),
    Rendered(Step),
    OpenOut,
    ShowOut,
    GetMap,
    PickMap,
    MapPicked(Option<PathBuf>),
    StopFetch(u64),
    Fetched(u64, maps::Step),
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
    WindowOpened(window::Id, f32, f32),
    CheckMinimized(window::Id),
    PollMinimized,
    Minimized(Option<bool>),
    Tick(Instant),
}

#[derive(Debug, Clone, Default)]
pub struct Shown {
    pub date: String,
    pub from: String,
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
    Waiting { code: String, link: String, osu: bool },
    Linking { code: String, link: String },
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
    aside: Option<Element<'a, Message>>,
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
    pub paused_at: Option<Instant>,
    pub stays: bool,
}

impl Toast {
    fn age(&self, now: Instant) -> Duration {
        self.paused_at.unwrap_or(now).saturating_duration_since(self.born)
    }

    fn hover(&mut self, over: bool, now: Instant) {
        if over == self.hovered { return; }
        self.hovered = over;
        if over {
            self.paused_at = Some(now);
        } else if let Some(paused) = self.paused_at.take() {
            self.born += now.saturating_duration_since(paused);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Queued {
    pub path: PathBuf,
    pub(crate) ask: render::Ask,
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
    pub id: u64,
    pub hash: String,
    pub reached: Vec<maps::Step>,
    pub shown: f32,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Fetching {
    pub fn new(id: u64, hash: String, reached: Vec<maps::Step>) -> Fetching {
        Fetching { id, hash, reached, shown: 0.0, stop: Default::default() }
    }

    pub fn last(&self) -> Option<&maps::Step> {
        self.reached.last()
    }

    pub fn is_over(&self) -> bool {
        self.last().is_some_and(maps::Step::is_last)
    }

    pub fn target(&self) -> Option<f32> {
        use maps::Step as S;
        if self.is_over() {
            return None;
        }
        Some(match self.last() {
            None | Some(S::Looking) => 0.04,
            Some(S::Found(_)) => 0.1,
            Some(S::Downloading { done, total, .. }) => match total {
                Some(total) if *total > 0 => 0.1 + 0.8 * (*done as f32 / *total as f32),
                _ => 0.3,
            },
            Some(S::Unpacking) => 0.93,
            Some(S::Checking) => 0.97,
            _ => 1.0,
        })
    }

    fn stop(&self) {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
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
    gallery: bool,
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
    minimized: bool,
    hidden: bool,
    window_id: Option<window::Id>,
    resume_player: Option<std::rc::Weak<std::cell::RefCell<player::Player>>>,
    pub open_video: Option<usize>,
    video_request: Option<PathBuf>,
    pub asking_delete: bool,
    pub skin_room: bool,
    pub skin_delete: Option<PathBuf>,
    pub skin_deleting: bool,
    pub room_fade: Animation<bool>,
    pub skin_scenes: HashMap<PathBuf, image::Handle>,
    pub cinema: Animation<bool>,
    pub resting: Animation<bool>,
    last_input: Instant,
    input_pointer: Option<Point>,
    pub widened: Animation<bool>,
    pub scrubbing: Option<f32>,
    pub controls: Animation<bool>,
    pub ask_fade: Animation<bool>,
    pub stage_open: Animation<bool>,
    pub mini_player: bool,
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
    everyone: Vec<crate::community::wire::Person>,
    pub pin: Option<crate::community::wire::Pin>,
    pub compare: Vec<i64>,
    pub compare_query: String,
    pub(crate) compare_pool: Vec<crate::community::Person>,
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
    pub side_open: Animation<bool>,
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
    pub sharing: sharing::State,
    pub overlay: Overlay,
    pub rendering: Option<Rendering>,
    pub queued: Vec<Queued>,
    pub fetching: Vec<Fetching>,
    fetch_serial: u64,
    pub looking: Option<scan::Step>,
    pub live: Option<Live>,
    pub live_before: Option<image::Handle>,
    pub hatch: image::Handle,
    pub trail: Option<Trail>,
    pub strip_view: Option<(f32, f32, f32)>,
    pub progress_shown: f32,
    pub ffmpeg: Option<PathBuf>,
    pub(crate) worker_step: Option<crate::worker::Step>,
    pub(crate) worker_last: Option<crate::worker::Step>,
    pub(crate) worker_running: bool,
    pub(crate) worker_done: u32,
    donated: usize,
    pub(crate) worker_back: u32,
    pub(crate) farm: Option<crate::bot::Farm>,
    pub update: UpdateState,
    pub reading: Option<library::Reading>,
    refreshing: bool,
    refreshed_at: Instant,
    covers_asked: std::collections::HashSet<String>,
    pub(crate) scale_draft: Option<u32>,
    hush_next: bool,
    flip_from: HashMap<String, f32>,
    flip_at: Option<Instant>,
    watch_sig: u64,
    pub(crate) update_wanted: bool,
    update_told: Option<String>,
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
    search_id: iced::widget::Id,
    strip_aim: Option<(u64, f32)>,
    pub community: Option<crate::community::Catalog>,
    pub community_section: crate::community_screen::Section,
    pub community_board: crate::community::Board,
    pub community_person: Option<usize>,
    pub person_fade: Animation<bool>,
    pub people_cards: HashMap<String, crate::community::wire::Card>,
    pub people_dossiers: HashMap<i64, crate::community::Me>,
    dossier_cache: crate::dossier_cache::Cache,
    people_asked: std::collections::HashSet<String>,
    pub news: crate::news::News,
    news_loaded: bool,
    pub news_pictures: HashMap<String, image::Handle>,
    pub news_frosts: HashMap<String, crate::community_screen::Frost>,
    news_asked: std::collections::HashSet<String>,
    pub news_loading: std::collections::HashSet<String>,
    pub news_failed: std::collections::HashSet<String>,
    pub channel_draft: String,
    pub live_shown: usize,
    feed_arrivals: crate::chronicle::Arrivals,
    feed_pictures: crate::chronicle::Pictures,
    thumb_pictures: crate::chronicle::Pictures,
    pub community_reading: Option<crate::community_screen::Reading>,
    pub map_boards: HashMap<u64, crate::community::wire::MapBoard>,
    pub map_boards_waiting: std::collections::HashSet<u64>,
    pub map_boards_failed: std::collections::HashSet<u64>,
    map_boards_fresh: std::collections::HashSet<u64>,
    pub score_scale: bool,
    pub read_fade: Animation<bool>,
    pub read_over_person: bool,
    pub witness: crate::witness::Seen,
    pub witness_control: Option<std::sync::Arc<crate::witness::Control>>,
    pub sittings: crate::witness::Sittings,
    pub history_running: bool,
    pub history_at: Option<Instant>,
    pub companion_fade: Animation<bool>,
    pub companion_at: Option<(u64, Instant)>,
    pub companion_asked: Option<u64>,
    pub companion_shown: Option<(u64, String)>,
    pub people_from: crate::community_screen::PeopleFrom,
    pub people_query: String,
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
    feed_fold_at: HashMap<String, Instant>,
    community_tap: Option<iced::Rectangle>,
    panel_from: Option<iced::Rectangle>,
    pub spot: usize,
    spot_at: Instant,
    pub(crate) section_at: Instant,
    pub(crate) shift_at: Instant,
    pub(crate) stream_at: Instant,
    pub(crate) person_at: Instant,
    pub(crate) play_at: Instant,
    pub(crate) group_at: Instant,
    pub(crate) news_at: Instant,
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

fn reveal_path(path: PathBuf) -> Task<Message> {
    ui::in_thread(move || Message::FolderOpened(crate::desktop::reveal(&path)))
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

impl Main {
    pub fn new(words: Words, settings: Settings) -> (Main, Task<Message>) {
        let worker_done = settings.worker_done;
        crate::donate::allow(settings.donate_replays);
        let donated = crate::donate::given(&crate::donate::ledger()).len();
        let worker_back = settings.worker_back;
        let sources = settings.sources.clone();
        library::only_exported(settings.exported_only);
        let mut made = Main {
            words,
            settings,
            gallery: false,
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
            minimized: false,
            hidden: false,
            window_id: None,
            resume_player: None,
            open_video: None,
            video_request: None,
            asking_delete: false,
            skin_room: false,
            skin_delete: None,
            skin_deleting: false,
            room_fade: Animation::new(false).duration(CINEMA).easing(Easing::EaseOutCubic),
            skin_scenes: HashMap::new(),
            cinema: Animation::new(false).duration(CINEMA).easing(Easing::EaseOutCubic),
            resting: Animation::new(false).duration(REST_FADE).easing(Easing::EaseOutCubic),
            last_input: Instant::now(),
            input_pointer: None,
            widened: Animation::new(false).duration(WIDEN).easing(Easing::EaseOutCubic),
            scrubbing: None,
            controls: Animation::new(true).duration(CONTROLS_FADE).easing(Easing::EaseOutCubic),
            ask_fade: Animation::new(false).duration(CINEMA).easing(Easing::EaseOutCubic),
            stage_open: Animation::new(false).duration(STAGE_OPEN).easing(Easing::EaseOutCubic),
            mini_player: false,
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
            everyone: Vec::new(),
            pin: None,
            compare: Vec::new(),
            compare_query: String::new(),
            compare_pool: Vec::new(),
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
            side_open: Animation::new(false).duration(SIDE_OPEN).easing(Easing::EaseOutCubic).delay(SIDE_WAIT),
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
            sharing: sharing::State::default(),
            overlay: Overlay::None,
            rendering: None,
            queued: Vec::new(),
            fetching: Vec::new(),
            fetch_serial: 0,
            looking: None,
            live: None,
            live_before: None,
            hatch: ui::hatched_picture(live::SIZE.0, live::SIZE.1, |_| 0.0),
            trail: None,
            strip_view: None,
            progress_shown: 0.0,
            ffmpeg: crate::checks::ffmpeg_on_path(),
            worker_step: None,
            worker_last: None,
            worker_running: false,
            worker_done,
            donated,
            worker_back,
            farm: None,
            update: UpdateState::Unknown,
            reading: None,
            refreshing: false,
            refreshed_at: Instant::now(),
            covers_asked: std::collections::HashSet::new(),
            scale_draft: None,
            hush_next: false,
            flip_from: HashMap::new(),
            flip_at: None,
            watch_sig: 0,
            update_wanted: false,
            update_told: None,
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
            search_id: iced::widget::Id::unique(),
            strip_aim: None,
            community: None,
            community_section: crate::community_screen::Section::Feed,
            community_board: crate::community::Board::Pp,
            community_person: None,
            person_fade: Animation::new(false),
            people_cards: HashMap::new(),
            people_dossiers: HashMap::new(),
            dossier_cache: crate::dossier_cache::Cache::default(),
            people_asked: std::collections::HashSet::new(),
            news: crate::news::News::default(),
            news_loaded: false,
            news_pictures: HashMap::new(),
            news_frosts: HashMap::new(),
            news_asked: std::collections::HashSet::new(),
            news_loading: std::collections::HashSet::new(),
            news_failed: std::collections::HashSet::new(),
            channel_draft: String::new(),
            live_shown: LIVE_FIRST,
            feed_arrivals: crate::chronicle::Arrivals::default(),
            feed_pictures: crate::chronicle::Pictures::default(),
            thumb_pictures: crate::chronicle::Pictures::default(),
            community_reading: None,
            map_boards: HashMap::new(),
            map_boards_waiting: std::collections::HashSet::new(),
            map_boards_failed: std::collections::HashSet::new(),
            map_boards_fresh: std::collections::HashSet::new(),
            score_scale: false,
            read_fade: Animation::new(false),
            read_over_person: false,
            witness: crate::witness::Seen::default(),
            witness_control: None,
            sittings: crate::witness::Sittings::default(),
            history_running: false,
            history_at: None,
            companion_fade: Animation::new(false).duration(PANEL_SHOW).easing(Easing::EaseOutCubic),
            companion_at: None,
            companion_asked: None,
            companion_shown: None,
            people_from: crate::community_screen::PeopleFrom::Chat,
            people_query: String::new(),
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
            feed_fold_at: HashMap::new(),
            community_tap: None,
            panel_from: None,
            spot: 0,
            spot_at: Instant::now() - Duration::from_secs(3600),
            section_at: Instant::now() - Duration::from_secs(3600),
            shift_at: Instant::now() - Duration::from_secs(3600),
            stream_at: Instant::now() - Duration::from_secs(3600),
            person_at: Instant::now() - Duration::from_secs(3600),
            play_at: Instant::now() - Duration::from_secs(3600),
            group_at: Instant::now() - Duration::from_secs(3600),
            news_at: Instant::now() - Duration::from_secs(3600),
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
        let warm = if made.settings.worker_on { Task::batch([warm, made.start_worker()]) } else { warm };
        (made, Task::batch([read_library(sources, Message::Loaded), adopt, who, warm]))
    }

    pub fn launched(&mut self) -> Task<Message> {
        crate::updates::touched();
        crate::render::share_cpu(self.settings.cpu_share);
        let ffmpeg = self.ffmpeg.clone();
        let version = ui::in_thread(move || Message::Ffmpeg(ffmpeg.as_deref().and_then(crate::checks::ffmpeg_version)));
        let build = crate::bot::BUILD;
        if self.settings.last_build != build {
            let before = std::mem::replace(&mut self.settings.last_build, build.to_owned());
            let _ = self.settings.save();
            if !before.is_empty() {
                let page = format!("{}/tag/v{build}", crate::updates::PAGE);
                self.announce(notices::Mark::Done, self.words.t("updated"), format!("{before} → {build}"), String::new(), String::new(), notices::Link::Page(page));
            }
        }
        let pin = self.pin_task();
        let witness = self.witness_task();
        if matches!(crate::updates::place(), crate::updates::Place::Source) {
            self.update = UpdateState::Source;
            return Task::batch([version, pin, witness]);
        }
        Task::batch([version, self.check_update(), pin, witness])
    }

    pub fn witness_source(&mut self) {
        match self.settings.sources.iter_mut().find(|source| source.is_witnessed()) {
            Some(source) => {
                source.on = true;
                source.replay_count = crate::sources::witnessed(true).replay_count;
            }
            None => self.settings.sources.push(crate::sources::witnessed(true)),
        }
    }

    pub fn witness_task(&mut self) -> Task<Message> {
        if self.gallery || self.witness_control.is_some() {
            return Task::none();
        }
        if self.settings.witness_keep && !self.settings.sources.iter().any(|source| source.is_witnessed()) {
            self.witness_source();
            let _ = self.settings.save();
        }
        let control = std::sync::Arc::new(crate::witness::Control::default());
        self.witness_control = Some(control.clone());
        self.witness = crate::witness::Seen { status: crate::witness::Status::Absent, ..crate::witness::Seen::default() };
        let player = self.community.as_ref().and_then(|catalog| catalog.people.iter().find(|person| person.you)).map(|you| you.name.clone()).unwrap_or_default();
        let watching = ui::streamed(move |push| crate::witness::run(control, player, &mut |event| push(Message::Witness(event))));
        Task::batch([watching, self.history_task(false)])
    }

    fn stable_root(&self) -> Option<std::path::PathBuf> {
        self.settings.sources.iter().find(|source| source.kind == crate::sources::Kind::Stable && source.on).map(|source| source.root.clone())
    }

    fn history_task(&mut self, force: bool) -> Task<Message> {
        if !self.settings.history_share || self.settings.token.is_empty() || self.gallery || self.history_running {
            return Task::none();
        }
        let now = Instant::now();
        if !force && self.history_at.is_some_and(|at| now.saturating_duration_since(at) < HISTORY_EVERY) {
            return Task::none();
        }
        let Some(root) = self.stable_root() else {
            return Task::none();
        };
        self.history_running = true;
        self.history_at = Some(now);
        let (server, token, name, after) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone(), self.settings.history_sent);
        ui::in_thread(move || {
            let digest = crate::history::gather(&root, after);
            Message::HistoryTold(crate::history::tell(&digest, |chunk| crate::bot::local_scores(&server, &token, &name, chunk).ok()))
        })
    }

    fn sitting_task(&mut self, event: &crate::witness::Event) -> Task<Message> {
        let told = self.sittings.take(event, unix_now(), &crate::witness::time_zone());
        let Some(sitting) = told else {
            return Task::none();
        };
        if self.gallery || self.settings.token.is_empty() {
            return Task::none();
        }
        let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        ui::in_thread(move || Message::WitnessSitting(crate::bot::witness_session(&server, &token, &name, &sitting).is_ok()))
    }

    fn companion_map(&self) -> Option<u64> {
        if !self.settings.witness_companion || self.settings.token.is_empty() || !matches!(self.witness.status, crate::witness::Status::Watching) {
            return None;
        }
        let state = self.witness.state.as_ref()?;
        (state.mode == "SelectPlay" && state.id > 0).then_some(state.id as u64)
    }

    pub fn companion_follow(&mut self) {
        let now = Instant::now();
        match self.companion_map() {
            Some(beatmap) => {
                if self.companion_at.map(|(held, _)| held) != Some(beatmap) {
                    self.companion_at = Some((beatmap, now));
                }
                if let Some(state) = self.witness.state.as_ref() {
                    self.companion_shown = Some((beatmap, state.map_line()));
                }
                if !self.companion_fade.value() {
                    self.companion_fade.go_mut(true, now);
                }
            }
            None => {
                self.companion_at = None;
                if self.companion_fade.value() {
                    self.companion_fade.go_mut(false, now);
                }
            }
        }
    }

    fn tell_task(&self, kept: &crate::witness::Kept) -> Option<Task<Message>> {
        if self.settings.token.is_empty() {
            return None;
        }
        let own = self.community.as_ref().and_then(|catalog| catalog.people.iter().find(|person| person.you)).map(|you| you.name.clone())?;
        let play = crate::witness::told(kept, &own, unix_now())?;
        let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        Some(ui::in_thread(move || Message::WitnessTold(crate::bot::witnessed(&server, &token, &name, &play).is_ok())))
    }

    fn pin_task(&self) -> Task<Message> {
        if self.settings.token.is_empty() {
            return Task::none();
        }
        let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        ui::in_thread(move || Message::PinRead(crate::bot::pinned(&server, &token, &name).map_err(|e| e.to_string())))
    }

    fn rescaled(&self, before: f32) -> Task<Message> {
        let after = ui::scale_of(self.settings.ui_scale);
        crate::refit_window(ui::refit(iced::Size::new(self.width, self.height), before, after, crate::WINDOW))
    }

    fn check_update(&mut self) -> Task<Message> {
        if matches!(self.update, UpdateState::Source | UpdateState::Checking | UpdateState::Getting { .. } | UpdateState::Ready { .. }) {
            return Task::none();
        }
        self.update = UpdateState::Checking;
        ui::in_thread(crate::updates::check).map(Message::UpdateChecked)
    }

    fn get_update(&mut self, wanted: bool) -> Task<Message> {
        self.update_wanted |= wanted;
        match self.update.clone() {
            UpdateState::Ready { .. } if self.update_wanted => self.put_in_when_calm(),
            UpdateState::Found(release) | UpdateState::Failed { release: Some(release), .. } => {
                self.update = UpdateState::Getting { total: Some(release.size).filter(|size| *size > 0), release: release.clone(), done: 0 };
                let place = crate::updates::place();
                ui::streamed(move |push| crate::updates::fetch(&release, &place, push)).map(Message::UpdateGot)
            }
            _ => Task::none(),
        }
    }

    pub(crate) fn calm_for_update(&self) -> bool {
        !crate::render::busy()
            && !crate::worker::holding()
            && self.sending.as_ref().is_none_or(|sending| sending.over.is_some())
            && !self.fetch_running()
            && self.queued.is_empty()
            && !matches!(self.looking, Some(scan::Step::Looking { .. }))
    }

    fn put_in_when_calm(&mut self) -> Task<Message> {
        if self.calm_for_update() {
            self.put_in(false)
        } else {
            Task::none()
        }
    }

    fn put_in(&mut self, quiet: bool) -> Task<Message> {
        let UpdateState::Ready { release, staged } = self.update.clone() else {
            return Task::none();
        };
        let place = crate::updates::place();
        match crate::updates::apply(&staged, &place).and_then(|what| crate::updates::launch(&what, quiet)) {
            Ok(()) => {
                crate::worker::stop();
                iced::exit()
            }
            Err(why) => {
                self.update = UpdateState::Failed { release: Some(release), why };
                self.update_wanted = false;
                Task::none()
            }
        }
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

    pub fn has_telegram(&self) -> bool {
        self.account.as_ref().is_none_or(|me| me.telegram)
    }

    fn watching(&self) -> bool {
        (self.player.is_some() || self.sharing.open.is_some()) && !self.leaving_player && !self.mini_player
    }

    pub fn busy(&self) -> bool {
        self.rendering.as_ref().is_some_and(|r| !r.is_over())
            || !self.queued.is_empty()
            || self.fetch_running()
            || matches!(self.looking, Some(scan::Step::Looking { .. }))
            || self.sending.as_ref().is_some_and(|s| s.over.is_none())
    }

    pub fn staged(words: Words, settings: Settings, library: Library, chosen: Option<usize>) -> Main {
        crate::sources::stage();
        let (mut made, _) = Main::new(words, settings);
        made.gallery = true;
        made.ffmpeg = Some(PathBuf::from("ffmpeg"));
        made.library = Some(library);
        made.chosen = chosen;
        made.store = videos::Store::default();
        made.notices = notices::Queue::default();
        made.enter = Animation::new(true);
        made.arrive = Animation::new(true);
        made
    }

    pub fn moving(&self) -> bool {
        if self.minimized { return false; }
        self.rendering.as_ref().is_some_and(|r| !r.is_over())
            || self.fetch_running()
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
            || self.resting.is_animating(self.now)
            || self.cinema.value() != self.watching()
            || self.stage_open.is_animating(self.now)
            || self.leaving_player
            || self.controls.is_animating(self.now)
            || self.ask_fade.is_animating(self.now)
            || self.ask_fade.value() != self.asking_delete
            || self.player.as_ref().is_some_and(|p| p.borrow().paused && !self.controls.value())
            || self.room_fade.is_animating(self.now)
            || self.flip_at.is_some_and(|at| self.now.saturating_duration_since(at) < FLIP)
            || self.widened.is_animating(self.now)
            || self.hint.is_some()
            || self.toasts.iter().any(|toast| toast.shown.is_animating(self.now))
            || self.menu_open.is_animating(self.now)
            || self.overlay_fade.is_animating(self.now)
            || self.ground_fade.is_animating(self.now)
            || self.turning.is_some()
            || self.retype.is_animating(self.now)
            || self.side_fade.is_animating(self.now)
            || self.side_open.is_animating(self.now)
            || self.marks.values().any(|m| m.is_animating(self.now))
            || self.slides_settling()
            || (self.overlay == Overlay::Community && self.feed_arrivals.animating(self.now))
            || (self.overlay == Overlay::Community && self.feed_pictures.animating(self.now))
            || self.thumb_pictures.animating(self.now)
            || self.feed_fold_at.values().any(|at| self.now.saturating_duration_since(*at) < FOLD)
            || self.read_fade.is_animating(self.now)
            || self.companion_fade.is_animating(self.now)
            || self.person_fade.is_animating(self.now)
            || (self.overlay == Overlay::Community && [self.section_at, self.shift_at, self.stream_at, self.person_at, self.play_at, self.group_at, self.news_at].iter().any(|at| self.now.saturating_duration_since(*at).as_secs_f32() < ui::APPEAR_ALL))
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
        let mut parts = vec![iced::event::listen_with(|event, status, id| {
            if let iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) = event {
                return Some(Message::PointerActivity(position));
            }
            let active = matches!(event,
                iced::Event::Mouse(iced::mouse::Event::ButtonPressed(_) | iced::mouse::Event::ButtonReleased(_) | iced::mouse::Event::WheelScrolled { .. })
                | iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { .. } | iced::keyboard::Event::KeyReleased { .. })
                | iced::Event::Touch(_));
            if active {
                crate::updates::touched();
            }
            let action = match (event, status) {
            (iced::Event::Window(window::Event::FileDropped(path)), _) => Some(Message::Dropped(path)),
            (iced::Event::Window(window::Event::Resized(size)), _) => Some(Message::Resized(size.width, size.height)),
            (iced::Event::Window(window::Event::Opened { size, .. }), _) => Some(Message::WindowOpened(id, size.width, size.height)),
            (iced::Event::Window(window::Event::Focused | window::Event::Unfocused), _) => Some(Message::CheckMinimized(id)),
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
        };
            if active { Some(Message::UserInput(action.map(Box::new))) } else { action }
        })];
        if !self.settings.token.is_empty() && !self.gallery {
            parts.push(iced::time::every(sharing::EVERY).map(|_| Message::Sharing(sharing::Message::Tick)));
        }
        if self.minimized {
            parts.push(iced::time::every(MINIMIZED_POLL).map(|_| Message::PollMinimized));
            return Subscription::batch(parts);
        }
        if self.can_rest() != self.resting.value() {
            parts.push(iced::time::every(Duration::from_secs(1)).map(Message::RestCheck));
        }
        if !matches!(self.update, UpdateState::Source | UpdateState::Unknown) {
            parts.push(iced::time::every(crate::updates::EVERY).map(|_| Message::UpdateTick));
        }
        if self.library.is_some() && !self.refreshing {
            parts.push(iced::time::every(Duration::from_secs(4)).map(|_| Message::WatchTick));
        }
        if self.settings.auto_flip && self.library.is_some() && self.overlay == Overlay::None && self.player.is_none() {
            parts.push(iced::time::every(AUTO_EVERY).map(|_| Message::AutoNext));
        }
        if matches!(self.update, UpdateState::Ready { .. }) && (self.update_wanted || self.settings.quiet_updates) {
            parts.push(iced::time::every(Duration::from_secs(5)).map(|_| Message::UpdateIdle));
        }
        let moving = self.moving();
        if moving {
            parts.push(window::frames().map(Message::Tick));
        }
        if !moving && self.toasts.iter().any(|toast| toast.shown.value() && !toast.stays && !toast.hovered) {
            parts.push(iced::time::every(Duration::from_millis(250)).map(Message::Tick));
        }
        if matches!(self.pairing, Pairing::Waiting { .. } | Pairing::Linking { .. }) {
            parts.push(iced::time::every(Duration::from_secs(3)).map(|_| Message::Poll));
        }
        if self.overlay == Overlay::Community && !self.settings.token.is_empty() {
            parts.push(iced::time::every(COMMUNITY_EVERY).map(|_| Message::CommunityTick));
        }
        if self.overlay == Overlay::Community {
            parts.push(iced::time::every(NEWS_EVERY).map(|_| Message::NewsTick));
        }
        if matches!(self.companion_at, Some((beatmap, _)) if self.companion_asked != Some(beatmap)) {
            parts.push(iced::time::every(COMPANION_TICK).map(|_| Message::CompanionTick));
        }
        if self.overlay == Overlay::Settings && self.side == Side::Bot && !self.settings.token.is_empty() {
            parts.push(iced::time::every(Duration::from_secs(10)).map(|_| Message::FarmTick));
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
            parts.join("  ")
        };
        Shown {
            date: format!("{}  {}  {}", w.day(entry.played_at, self.now_unix), w.clock(entry.played_at), entry.client.tag()),
            from: if crate::mixed::is_shared(&entry.path) { w.t("journal-shared") } else { String::new() },
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
        self.fetching.retain(|fetching| !fetching.is_over());
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
            let elsewhere = entry.map.is_none();
            tasks.push(ui::in_thread(move || {
                let scene = background.as_deref().and_then(|p| decoded(p, SCENE_WIDTH, None)).or_else(|| {
                    elsewhere
                        .then(|| crate::maps::known(&hash))
                        .flatten()
                        .and_then(|known| crate::news::picture(&known.cover()))
                        .and_then(|bytes| decoded_from(&bytes, SCENE_WIDTH, None))
                });
                Message::Scene(hash, scene)
            }));
        }
        if !self.lengths.contains_key(&entry.path) {
            let path = entry.path.clone();
            tasks.push(ui::in_thread(move || Message::Length(path.clone(), length_of(&path))));
        }
        Task::batch(tasks)
    }

    fn watch_task(&self) -> Task<Message> {
        let sources = self.settings.sources.clone();
        ui::in_thread(move || Message::Watched(crate::sources::signature(&sources)))
    }

    pub fn reading_line(&self) -> Option<String> {
        let w = &self.words;
        let reading = self.reading?;
        let group = |n: usize| w.lang().group(n as u64);
        Some(if reading.replays.1 == 0 {
            format!("{} {} / {}", w.t("reading-maps"), group(reading.maps.0), group(reading.maps.1))
        } else {
            format!("{} {} / {}", w.t("reading-replays"), group(reading.replays.0), group(reading.replays.1))
        })
    }

    fn covers_task(&mut self) -> Task<Message> {
        let wanted: Vec<String> = {
            let mut seen = std::collections::HashSet::new();
            self.entries()
                .iter()
                .filter(|e| e.map.is_none() && !self.thumbs.contains_key(&e.map_hash) && !self.covers_asked.contains(&e.map_hash))
                .filter(|e| seen.insert(e.map_hash.clone()))
                .take(COVERS_AT_ONCE)
                .map(|e| e.map_hash.clone())
                .collect()
        };
        if wanted.is_empty() {
            return Task::none();
        }
        self.covers_asked.extend(wanted.iter().cloned());
        ui::streamed(move |push| {
            for hash in wanted {
                let Some(known) = crate::maps::known(&hash) else {
                    continue;
                };
                if !push(Message::MapKnown(hash.clone(), known.clone())) {
                    return;
                }
                if let Some(handle) = crate::news::picture(&known.cover()).and_then(|bytes| decoded_from(&bytes, THUMB.0, Some(THUMB))) {
                    if !push(Message::Thumb(hash, handle)) {
                        return;
                    }
                }
            }
        })
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

    fn can_rest(&self) -> bool {
        self.library.is_some() && self.overlay == Overlay::None && self.player.is_none()
            && !self.cinema.value() && !self.cinema.is_animating(self.now)
            && !self.overlay_fade.is_animating(self.now) && self.turning.is_none()
            && self.menu.is_none() && !self.menu_open.is_animating(self.now)
            && !self.skin_room && !self.room_fade.is_animating(self.now) && self.skin_delete.is_none()
            && !self.asking_delete && !self.ask_fade.is_animating(self.now) && self.error_shown.is_none()
            && matches!(self.pairing, Pairing::Idle) && self.toasts.is_empty()
            && !self.rendering.as_ref().is_some_and(|job| !job.is_over())
            && !self.fetch_running()
            && self.queued.is_empty()
            && !matches!(self.looking, Some(scan::Step::Looking { .. }))
    }

    fn wake(&mut self, now: Instant) {
        self.last_input = now;
        if self.resting.value() {
            self.resting.go_mut(false, now);
        }
    }

    fn check_rest(&mut self, now: Instant) {
        if !self.can_rest() {
            self.wake(now);
        } else if !self.resting.value() && now.saturating_duration_since(self.last_input) >= REST_AFTER {
            self.resting.go_mut(true, now);
            self.hover = None;
            self.hover_bounds = None;
            self.hover_since = None;
            for lift in self.lifts.values_mut() {
                lift.go_mut(false, now);
            }
        }
    }

    pub fn set_hidden(&mut self, hidden: bool) {
        self.hidden = hidden;
        self.set_minimized(hidden);
    }

    fn set_minimized(&mut self, minimized: bool) {
        if self.minimized == minimized { return; }
        crate::frames::mark("window", Instant::now(), if minimized { "minimized" } else { "restored" });
        self.minimized = minimized;
        if minimized {
            self.resume_player = self.player.as_ref().and_then(|player| {
                (!player.borrow().paused).then(|| std::rc::Rc::downgrade(player))
            });
            if self.resume_player.is_some() {
                if let Some(player) = &self.player { player.borrow_mut().toggle(); }
            }
        } else {
            if let (Some(previous), Some(player)) = (self.resume_player.take(), &self.player) {
                if previous.upgrade().is_some_and(|previous| std::rc::Rc::ptr_eq(&previous, player)) {
                    let mut player = player.borrow_mut();
                    if player.paused && !player.ended() { player.toggle(); }
                }
            }
            self.now = Instant::now();
            self.last_input = self.now;
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        if self.minimized && matches!(&message,
            Message::Tick(_) | Message::RestCheck(_) | Message::WatchTick | Message::AutoNext
            | Message::UpdateTick | Message::UpdateIdle | Message::Poll | Message::CommunityTick
            | Message::NewsTick | Message::FarmTick | Message::FeedClock | Message::LiveArrive)
        {
            return Task::none();
        }
        match message {
            Message::WindowOpened(id, width, height) => {
                self.window_id = Some(id);
                self.width = width;
                self.height = height;
                Task::none()
            }
            Message::CheckMinimized(id) => {
                crate::frames::mark("window", Instant::now(), "focus changed");
                self.window_id = Some(id);
                let later = |after: Duration| ui::in_thread(move || {
                    std::thread::sleep(after);
                    Message::PollMinimized
                });
                Task::batch([window::is_minimized(id).map(Message::Minimized), later(Duration::from_millis(600)), later(Duration::from_millis(1800))])
            }
            Message::PollMinimized => self.window_id.map_or_else(Task::none, |id| window::is_minimized(id).map(Message::Minimized)),
            Message::Minimized(Some(minimized)) => {
                self.set_minimized(minimized || self.hidden);
                Task::none()
            }
            Message::Minimized(None) => Task::none(),
            Message::PointerActivity(position) => {
                if self.input_pointer != Some(position) {
                    self.input_pointer = Some(position);
                    crate::updates::touched();
                    self.wake(Instant::now());
                }
                Task::none()
            }
            Message::UserInput(action) => {
                let hidden = self.resting.value() || self.resting.interpolate(0.0, 1.0, self.now) > 0.001;
                let now = Instant::now();
                self.wake(now);
                if hidden { Task::none() } else { action.map_or_else(Task::none, |message| self.update(*message)) }
            }
            Message::RestCheck(now) => {
                self.check_rest(now);
                self.now = now;
                Task::none()
            }
            Message::Reading(reading) => {
                self.reading = Some(reading);
                Task::none()
            }
            Message::AutoNext => {
                let busy = self.rendering.as_ref().is_some_and(|r| !r.is_over())
                    || !self.queued.is_empty()
                    || self.fetch_running()
                    || matches!(self.looking, Some(scan::Step::Looking { .. }));
                if !self.settings.auto_flip || busy || self.overlay != Overlay::None || self.player.is_some() || self.menu.is_some() || crate::updates::idle() < AUTO_IDLE {
                    return Task::none();
                }
                let visible = self.visible();
                if visible.len() < 2 {
                    return Task::none();
                }
                let next = match self.chosen.and_then(|c| visible.iter().position(|v| *v == c)) {
                    Some(at) if at + 1 < visible.len() => visible[at + 1],
                    _ => visible[0],
                };
                self.choose(next)
            }
            Message::WatchTick => {
                if self.refreshing || self.library.is_none() {
                    return Task::none();
                }
                self.watch_task()
            }
            Message::Watched(sig) => {
                if self.refreshing || self.watch_sig == 0 || sig == self.watch_sig {
                    self.watch_sig = if self.refreshing { self.watch_sig } else { sig };
                    return Task::none();
                }
                if self.refreshed_at.elapsed() < REFRESH_AT_MOST {
                    return Task::none();
                }
                self.watch_sig = sig;
                self.refreshing = true;
                self.refreshed_at = Instant::now();
                read_library(self.settings.sources.clone(), Message::Refreshed)
            }
            Message::Refreshed(library) => {
                self.refreshing = false;
                self.reading = None;
                let before: std::collections::HashSet<String> = self.entries().iter().map(|e| e.replay_hash.clone()).collect();
                let was = self.chosen_entry().map(|e| e.path.clone());
                let fresh: Vec<Entry> = library.entries.iter().filter(|e| !before.contains(&e.replay_hash) && !crate::mixed::is_shared(&e.path)).cloned().collect();
                self.library = Some(library);
                self.chosen = was.and_then(|path| self.entries().iter().position(|e| e.path == path)).or_else(|| self.visible().first().copied());
                self.hover = None;
                self.hover_bounds = None;
                self.hover_since = None;
                self.lifts.clear();
                let entries = self.entries().to_vec();
                self.store.marry(&entries);
                let hushed = std::mem::take(&mut self.hush_next);
                let own = self.community.as_ref().and_then(|catalog| catalog.people.iter().find(|p| p.you)).map(|you| you.name.to_lowercase());
                let played = !hushed && !self.settings.token.is_empty() && fresh.iter().any(|e| own.as_ref().is_none_or(|own| e.player.to_lowercase() == *own));
                let nudge = if played {
                    let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
                    ui::in_thread(move || {
                        let _ = crate::bot::played(&server, &token, &name);
                        Message::Nudged
                    })
                } else {
                    Task::none()
                };
                if !before.is_empty() && !hushed {
                    if let Some(newest) = fresh.iter().max_by_key(|e| e.played_at) {
                        let words = if fresh.len() == 1 { self.words.t("new-replay") } else { self.words.count("new-replays", fresh.len() as u64) };
                        let detail = format!("{} — {}", newest.player, newest.song().unwrap_or_else(|| self.words.t("unknown-map")));
                        self.announce(notices::Mark::Done, words, detail, String::new(), newest.map_hash.clone(), notices::Link::Replay(newest.path.clone()));
                    }
                }
                Task::batch([self.thumbs_task(), self.covers_task(), nudge, self.donate_task(), self.replays_give_task()])
            }
            Message::Loaded(library) => {
                self.wake(Instant::now());
                self.reading = None;
                self.refreshing = false;
                self.watch_sig = 0;
                self.library = Some(library);
                self.now_unix = unix_now();
                let entries = self.entries().to_vec();
                self.store.marry(&entries);
                self.enter = Animation::new(false).duration(ENTER).easing(Easing::EaseOutCubic).go(true, Instant::now());
                let first = self.visible().first().copied();
                self.chosen = first;
                Task::batch([self.fetch_for_chosen(), self.thumbs_task(), self.covers_task(), self.start_live(), self.watch_task(), self.donate_task(), self.replays_give_task()])
            }
            Message::MapKnown(hash, known) => {
                if let Some(library) = self.library.as_mut() {
                    for entry in library.entries.iter_mut().filter(|e| e.map_hash == hash && e.map.is_none() && e.named.is_none()) {
                        entry.named = Some(library::Named { artist: known.artist.clone(), title: known.title.clone(), version: known.version.clone() });
                    }
                }
                Task::none()
            }
            Message::Thumb(hash, handle) => {
                self.thumb_pictures.came(&hash, Instant::now());
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
                } else if matches!(self.pairing, Pairing::Asking | Pairing::Waiting { .. } | Pairing::Linking { .. } | Pairing::Unavailable) {
                    self.pairing = Pairing::Idle;
                } else if self.skin_delete.is_some() {
                    return self.prefs(prefs::Message::KeepSkin);
                } else if self.menu.is_some() {
                    return self.update(Message::MenuClose);
                } else if self.skin_room {
                    return self.update(Message::ShowSkins(false));
                } else if self.asking_delete {
                    self.asking_delete = false;
                } else if self.sharing.picker.is_some() {
                    return self.update(Message::Sharing(sharing::Message::Close));
                } else if self.player.is_some() || self.sharing.open.is_some() {
                    if self.widened.value() {
                        return self.update(Message::PlayerWiden);
                    }
                    return self.update(Message::ClosePlayer);
                } else if self.overlay == Overlay::Community && self.community_reading.is_some() && self.read_fade.value() {
                    self.read_fade.go_mut(false, Instant::now());
                } else if self.overlay == Overlay::Community && self.community_person.is_some() && self.person_fade.value() {
                    self.person_fade.go_mut(false, Instant::now());
                } else if self.overlay == Overlay::None && !self.search.is_empty() {
                    self.sorting();
                    self.search.clear();
                    return iced::advanced::widget::operate(iced::advanced::widget::operation::focusable::unfocus::<Message>());
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
            Message::ClearNotices => {
                self.notices.clear();
                self.leaving.clear();
                self.arrivals.clear();
                self.toasts.clear();
                self.error_shown = None;
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
                ui::in_thread(move || Message::PairAsked(bot::pair(&server, &name).map(|p| (p.code, p.link, p.osu))))
            }
            Message::PairAsked(Ok((code, link, osu))) => {
                let link = if link.is_empty() {
                    format!("https://t.me/OneNineEightFourGlobalBot?start=pair-{}", bot::tidy(&code))
                } else {
                    link
                };
                self.qr = crate::first_run::qr_for(&link);
                self.pairing = Pairing::Waiting { code: bot::pretty(&code), link, osu };
                Task::none()
            }
            Message::OpenOsu => {
                if let Pairing::Waiting { code, osu: true, .. } = &self.pairing {
                    let _ = open::that_detached(bot::osu_link(&self.settings.server, code));
                }
                Task::none()
            }
            Message::LinkTelegram => {
                self.menu = None;
                if !self.signed_in() {
                    return self.update(Message::SignIn);
                }
                if self.has_telegram() || !matches!(self.pairing, Pairing::Idle) {
                    return Task::none();
                }
                self.pairing = Pairing::Asking;
                let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
                ui::in_thread(move || Message::TelegramAsked(bot::link_telegram(&server, &token, &name).map(|p| (p.code, p.link)).map_err(|e| e.to_string())))
            }
            Message::TelegramAsked(Ok((code, link))) => {
                self.qr = crate::first_run::qr_for(&link);
                self.pairing = Pairing::Linking { code: bot::pretty(&code), link };
                Task::none()
            }
            Message::TelegramAsked(Err(_)) => {
                self.pairing = Pairing::Unavailable;
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
                Pairing::Linking { .. } => self.ask_who(),
                _ => Task::none(),
            },
            Message::Polled(Ok(Paired::Linked { token, who })) => {
                self.settings.token = token;
                self.sync_dossiers();
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
                if let Pairing::Waiting { link, .. } | Pairing::Linking { link, .. } = &self.pairing {
                    let _ = open::that_detached(link);
                }
                Task::none()
            }
            Message::CopyLink => match &self.pairing {
                Pairing::Waiting { link, .. } | Pairing::Linking { link, .. } => iced::clipboard::write(link.clone()),
                _ => Task::none(),
            },
            Message::LaterSignIn => {
                self.pairing = Pairing::Idle;
                Task::none()
            }
            Message::SignOut => {
                self.settings.token.clear();
                self.dossier_cache.clear();
                self.people_dossiers.clear();
                self.settings.linked_as.clear();
                let _ = self.settings.save();
                self.account = None;
                self.avatar = None;
                self.sharing = sharing::State::default();
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
                if matches!(self.pairing, Pairing::Linking { .. }) && me.telegram {
                    self.pairing = Pairing::Idle;
                    self.chats.clear();
                    let words = self.words.t("telegram-linked");
                    self.say(words);
                }
                let chats = if me.telegram { self.chats_task() } else { Task::none() };
                self.account = Some(me);
                self.offer_shared_source();
                let inbox = Task::batch([self.inbox_task(true), self.replays_state_task(), self.replays_sync_task(true), chats]);
                if !wants_avatar {
                    return inbox;
                }
                let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
                Task::batch([
                    inbox,
                    ui::in_thread(move || {
                        let bytes = bot::avatar(&server, &token, &name).ok();
                        Message::Avatar(bytes.and_then(|b| decoded_bytes(&b, AVATAR_SIDE)))
                    }),
                ])
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
                if !self.has_telegram() {
                    return self.update(Message::LinkTelegram);
                }
                if self.sending.as_ref().is_some_and(|s| s.over.is_none()) {
                    return Task::none();
                }
                let Some(video) = self.open_video.and_then(|at| self.store.videos.get(at).cloned()) else {
                    return Task::none();
                };
                self.sending = Some(Sending { path: video.path.clone(), done: 0, total: video.size.max(1), over: None });
                let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
                let meta = self.send_meta(&video, self.settings.chat_id);
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
                        Ok(Ok(sent)) => Ok(sent),
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
                if sending.over.is_some() {
                    return Task::none();
                }
                let path = sending.path.clone();
                let bytes = sending.total;
                sending.over = Some(outcome.clone().map(|sent| sent.message));
                if let Ok(sent) = &outcome {
                    self.store.mark_sent(&path, unix_now(), bytes);
                    if let Some(remote) = sent.video {
                        self.store.remember_remote(&path, remote);
                    }
                }
                let video = self.store.videos.iter().find(|v| v.path == path).cloned();
                let who = self.account.as_ref().map(|a| format!("@{}", a.username)).filter(|u| u.len() > 1).unwrap_or_else(|| self.settings.linked_as.clone());
                match (outcome, video) {
                    (Ok(_), Some(video)) => {
                        let detail = format!("{} — {}", video.player, video.map_line());
                        let note = format!("{}  {}", who, self.words.mb(video.size));
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
            Message::Skins(mut found) => {
                found.retain(|path| crate::settings::skin_allowed(path, &self.settings.removed_skins, &crate::settings::skins_root()));
                crate::settings::order_skins(&mut found, &self.settings.skin_order);
                self.skins = found;
                self.skin_look()
            }
            Message::SkinDeleted(folder, result) => {
                if self.skin_delete.as_ref() != Some(&folder) || !self.skin_deleting { return Task::none(); }
                self.skin_deleting = false;
                match result {
                    Err(why) => {
                        self.announce(notices::Mark::Bad, self.words.t("skin-delete-failed"), crate::settings::skin_name(&folder), why, String::new(), notices::Link::None);
                        Task::none()
                    }
                    Ok(()) => {
                        self.skin_delete = None;
                        self.skins.retain(|path| *path != folder);
                        self.skin_faces.remove(&folder);
                        self.skin_scenes.remove(&folder);
                        let changed = self.settings.forget_skin(&folder);
                        let _ = self.settings.save();
                        if changed { self.skin_again() } else { Task::none() }
                    }
                }
            }
            Message::SkinFace(folder, handle) => {
                if self.settings.removed_skins.contains(&folder) { return Task::none(); }
                if let Some(handle) = handle {
                    self.skin_faces.insert(folder, handle);
                }
                Task::none()
            }
            Message::SkinScene(folder, handle) => {
                if self.settings.removed_skins.contains(&folder) { return Task::none(); }
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
                let Some(ffmpeg) = self.ffmpeg.clone() else {
                    return Task::none();
                };
                let path = video.path.clone();
                self.video_request = Some(path.clone());
                ui::in_thread(move || {
                    let media = videos::probe(&ffmpeg, &path);
                    Message::VideoReady(path, media)
                })
            }
            Message::VideoReady(path, media) => {
                if self.video_request.as_ref() != Some(&path) || self.overlay != Overlay::Videos {
                    return Task::none();
                }
                self.video_request = None;
                if let Some(media) = media {
                    self.store.refresh_media(&path, media);
                }
                let Some(at) = self.store.videos.iter().position(|v| v.path == path) else {
                    return Task::none();
                };
                let video = &self.store.videos[at];
                let Some(ffmpeg) = &self.ffmpeg else {
                    return Task::none();
                };
                let fresh = self.player.is_none() || self.leaving_player || self.mini_player;
                if let Some(old) = self.player.take() {
                    old.borrow_mut().close();
                }
                self.leaving_player = false;
                self.mini_player = false;
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
                self.player = Some(std::rc::Rc::new(std::cell::RefCell::new(player::Player::open(ffmpeg, &video.path, videos::Probe { length_ms: video.length_ms, width: video.width, height: video.height, fps: video.fps }, manner))));
                self.open_video = Some(at);
                self.clip = None;
                self.sharing.open = None;
                self.asking_delete = false;
                self.scrubbing = None;
                self.hint = None;
                self.cinema.go_mut(true, Instant::now());
                Task::none()
            }
            Message::ClosePlayer => {
                self.video_request = None;
                if (self.player.is_none() && self.sharing.open.is_none()) || self.leaving_player {
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
            Message::PlayerMinimize => {
                if self.player.is_none() || self.leaving_player || self.mini_player {
                    return Task::none();
                }
                self.mini_player = true;
                self.asking_delete = false;
                self.over_controls = false;
                self.scrubbing = None;
                self.pointer = None;
                let now = Instant::now();
                self.stage_open.go_mut(false, now);
                self.cinema.go_mut(false, now);
                self.widened.go_mut(false, now);
                Task::none()
            }
            Message::PlayerRestore => {
                if self.player.is_none() || !self.mini_player || self.leaving_player {
                    return Task::none();
                }
                let home = if self.clip.is_some() { Overlay::Community } else { Overlay::Videos };
                let shown = if self.overlay != home { self.update(Message::Show(home)) } else { Task::none() };
                self.mini_player = false;
                let now = Instant::now();
                self.stage_open = Animation::new(false).duration(STAGE_OPEN).easing(Easing::EaseOutCubic).go(true, now);
                self.cinema.go_mut(true, now);
                self.stirred = now;
                shown
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
            Message::Search(query) => {
                self.sorting();
                self.search = query;
                Task::none()
            }
            Message::Typed(letter) => {
                if self.player.is_none() {
                    if self.overlay == Overlay::None && self.menu.is_none() && self.library.is_some() && (letter.is_alphanumeric() || letter == '+') {
                        self.sorting();
                        self.search.push(letter);
                        let id = self.search_id.clone();
                        return iced::advanced::widget::operate(iced::advanced::widget::operation::focusable::focus::<Message>(id.clone()))
                            .chain(iced::advanced::widget::operate(iced::advanced::widget::operation::text_input::move_cursor_to_end::<Message>(id)));
                    }
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
                    return reveal_path(clip.path.clone());
                } else if let Some(path) = self.sharing.opened().and_then(|got| got.cached()) {
                    return reveal_path(path);
                } else if let Some(video) = self.open_video.and_then(|at| self.store.videos.get(at)) {
                    return reveal_path(video.path.clone());
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
            Message::FolderOpened(result) => {
                if let Err(why) = result {
                    self.say_trouble(why);
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
                self.mini_player = false;
                self.shut_cinema();
                if videos::to_bin(&video.path).is_ok() || !video.path.exists() {
                    self.store.forget(&video.path);
                }
                Task::none()
            }
            Message::Over(at, bounds) => {
                let moved = self.hover_bounds.is_none_or(|before| {
                    (bounds.x - before.x).abs() >= 0.5 || (bounds.y - before.y).abs() >= 0.5
                        || (bounds.width - before.width).abs() >= 0.5 || (bounds.height - before.height).abs() >= 0.5
                });
                if self.hover != Some(at) || moved {
                    self.hover_bounds = Some(bounds);
                }
                self.update(Message::Hover(Some(at)))
            }
            Message::HoverStaged(at) => {
                let x = 40.0 + theme::FRAME_W + 8.0 + 22.0 + at.saturating_sub(1) as f32 * (theme::FRAME_W + 6.0);
                let bounds = iced::Rectangle::new(Point::new(x, self.height - 10.0 - theme::FRAME_H), iced::Size::new(theme::FRAME_W, theme::FRAME_H));
                self.update(Message::Over(at, bounds))
            }
            Message::SideOpen(open) => {
                if self.side_open.value() != open {
                    self.side_open.go_mut(open, Instant::now());
                }
                Task::none()
            }
            Message::SideMoved(what, before) => {
                let mut entries = self.community_entries();
                entries.push(crate::sidebar::Entry::Caption(String::new()));
                entries.extend(self.settings_entries());
                let shown = crate::sidebar::groups(&crate::sidebar::arranged(entries, &self.settings.side_order));
                self.settings.side_order = crate::sidebar::moved(&shown, &self.settings.side_order, what, before);
                let _ = self.settings.save();
                Task::none()
            }
            Message::Hover(at) => {
                if self.hover == at {
                    return Task::none();
                }
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
            Message::HoverLeft(at) => {
                if self.hover == Some(at) {
                    return self.update(Message::Hover(None));
                }
                Task::none()
            }
            Message::Show(overlay) => {
                let now = Instant::now();
                if overlay == Overlay::Settings {
                    self.side = match self.settings.settings_tab.as_str() {
                        "bot" => Side::Bot,
                        _ => Side::App,
                    };
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
                    return Task::batch([sizes, stored, version, chats, skins, self.farm_task()]);
                }
                let news = match overlay == Overlay::Community {
                    true => {
                        self.now_unix = unix_now();
                        if self.community.is_none() {
                            self.community = Some(self.first_community());
                        }
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
                if overlay != Overlay::Videos && !self.mini_player {
                    if let Some(player) = self.player.take() {
                        player.borrow_mut().close();
                    }
                    self.open_video = None;
                    self.clip = None;
                    self.sharing.open = None;
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
                let inbox = self.inbox_task(false);
                if wanted.is_empty() {
                    return inbox;
                }
                Task::batch([
                    inbox,
                    ui::streamed(move |push| {
                        for (hash, path) in wanted {
                            if let Some(handle) = decoded(&path, THUMB.0, Some(THUMB)) {
                                if !push(Message::Thumb(hash, handle)) {
                                    return;
                                }
                            }
                        }
                    }),
                ])
            }
            Message::Community(inner) => {
                use crate::community_screen::Message as C;
                let now = Instant::now();
                match inner {
                    C::Tap(at, card) => self.community_tap = Some(card.unwrap_or(iced::Rectangle { x: at.x - 160.0, y: at.y - 100.0, width: 320.0, height: 200.0 })),
                    C::Read(reading) => {
                        let wanted = reading.pictures();
                        let board = match &reading {
                            crate::community_screen::Reading::Score(scored) => scored.map.beatmap,
                            _ => None,
                        };
                        self.community_reading = Some(reading);
                        self.read_over_person = self.community_person.is_some() && self.person_fade.value();
                        self.panel_from = if self.read_over_person { None } else { self.community_tap };
                        self.read_fade = Animation::new(false).duration(PANEL_SHOW).easing(Easing::EaseOutCubic).go(true, now);
                        let pictures = self.wide_pictures_task(wanted);
                        return match board {
                            Some(beatmap) => Task::batch([pictures, self.map_board_task(beatmap, false)]),
                            None => pictures,
                        };
                    }
                    C::ScoreScale(on) => self.score_scale = on,
                    C::TitleOf(code, who) => return self.update(Message::Community(C::Read(crate::community_screen::Reading::Title { code, who }))),
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
                    C::PeopleSearch(query) => self.people_query = query,
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
                        if self.community_person.is_some() && self.person_fade.value() {
                            self.person_fade.go_mut(false, now);
                        }
                        self.community_section = section;
                        if section == crate::community_screen::Section::Compare {
                            self.pool_players();
                            if self.compare.is_empty() {
                                if let Some(you) = self.compare_pool.iter().find(|person| person.you) {
                                    self.compare.push(you.id);
                                }
                            }
                            return self.everyone_task();
                        }
                    }
                    C::Go(section, from) => {
                        let mut tasks = Vec::new();
                        if let Some(from) = from {
                            if section == self.community_section && from != self.people_from {
                                self.shift_at = now;
                            }
                            self.people_from = from;
                            if from == crate::community_screen::PeopleFrom::Game {
                                tasks.push(self.friends_task(false));
                            }
                        }
                        tasks.push(self.update(Message::Community(C::Section(section))));
                        return Task::batch(tasks);
                    }
                    C::Board(board) => {
                        if self.community_person.is_some() && self.person_fade.value() {
                            self.person_fade.go_mut(false, now);
                        }
                        if self.community_section != crate::community_screen::Section::Boards {
                            self.section_at = now;
                        } else if board != self.community_board {
                            self.shift_at = now;
                        }
                        self.community_board = board;
                        self.community_section = crate::community_screen::Section::Boards;
                    }
                    C::Person(Some(at)) => {
                        if self.read_over_person && self.community_reading.is_some() && self.read_fade.value() {
                            self.read_fade.go_mut(false, now);
                        }
                        self.person_at = now;
                        self.community_person = Some(at);
                        self.panel_from = self.community_tap;
                        self.person_fade = Animation::new(false).duration(PANEL_SHOW).easing(Easing::EaseOutCubic).go(true, now);
                        return self.person_task(at);
                    }
                    C::Person(None) => self.person_fade.go_mut(false, now),
                    C::CompareWith(at) => {
                        let Some(catalog) = self.community.as_ref() else {
                            return Task::none();
                        };
                        let you = catalog.people.iter().find(|person| person.you).map(|person| person.id);
                        let them = catalog.people.get(at).map(|person| person.id);
                        self.compare = you.into_iter().chain(them.filter(|them| Some(*them) != you)).collect();
                        self.compare_query.clear();
                        if self.community_person.is_some() && self.person_fade.value() {
                            self.person_fade.go_mut(false, now);
                        }
                        if self.community_section != crate::community_screen::Section::Compare {
                            self.section_at = now;
                        }
                        self.community_section = crate::community_screen::Section::Compare;
                        self.pool_players();
                        return self.everyone_task();
                    }
                    C::CompareAdd(id) => {
                        if self.compare.len() < crate::compare::MOST && !self.compare.contains(&id) {
                            self.compare.push(id);
                        }
                        self.compare_query.clear();
                    }
                    C::CompareRemove(id) => self.compare.retain(|chosen| *chosen != id),
                    C::CompareSearch(said) => self.compare_query = said,
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
                            self.group_at = now;
                        }
                        self.feed_filter = filter;
                    }
                    C::Source(source) => {
                        if source != self.feed_source {
                            self.news_at = now;
                        }
                        self.feed_source = source;
                    }
                    C::Stream(stream) => {
                        if stream != self.feed_stream {
                            self.stream_at = now;
                        }
                        self.feed_stream = stream;
                    }
                    C::Search(query) => self.feed_query = query,
                    C::Toggle(key) => {
                        self.feed_fold_at.retain(|_, at| now.saturating_duration_since(*at) < FOLD);
                        self.feed_fold_at.insert(key.clone(), now);
                        if !self.feed_open.remove(&key) {
                            self.feed_open.insert(key);
                        }
                    }
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
                    C::Wear(code) => {
                        let chat = self.community.as_ref().and_then(|catalog| catalog.chat);
                        if let Some(me) = self.community.as_mut().and_then(|catalog| catalog.people.iter_mut().find(|p| p.you)) {
                            me.title = code.clone();
                        }
                        let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
                        return ui::in_thread(move || Message::Worn(crate::bot::wear_title(&server, &token, &name, chat, code.as_deref()).map_err(|e| e.to_string())));
                    }
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
                                Ok((path, probe))
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
                        let before = self.feed_keys();
                        self.news.builds = builds;
                        self.news_heard(source);
                        self.fresh_from(before);
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
                        let before = self.feed_keys();
                        self.news.stories = stories;
                        self.news_heard(source);
                        self.fresh_from(before);
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
                        let before = self.feed_keys();
                        self.news.take_posts(&channel, posts);
                        self.news_heard(&source);
                        self.fresh_from(before);
                        self.news_pictures_task()
                    }
                    Err(_) => {
                        self.news_failed.insert(source);
                        Task::none()
                    }
                }
            }
            Message::NewsPicture(url, handle) => {
                match handle {
                    Some(handle) => {
                        self.feed_pictures.came(&url, Instant::now());
                        self.news_pictures.insert(url, handle);
                    }
                    None => self.feed_pictures.lost(&url, Instant::now()),
                }
                Task::none()
            }
            Message::NewsFrost(url, frost) => {
                self.news_frosts.insert(url, frost);
                Task::none()
            }
            Message::ReadFirst => match self.news.stories.first().cloned() {
                Some(story) => self.update(Message::Community(crate::community_screen::Message::Read(crate::community_screen::Reading::Story(story)))),
                None => Task::none(),
            },
            Message::CommunityTick => self.community_task(false),
            Message::Witness(event) => {
                if self.witness_control.is_none() {
                    return Task::none();
                }
                let was_there = matches!(self.witness.status, crate::witness::Status::Loading | crate::witness::Status::Watching | crate::witness::Status::Playing);
                self.witness.take(&event);
                self.companion_follow();
                let leaving = was_there && matches!(event, crate::witness::Event::Gone | crate::witness::Event::Waiting | crate::witness::Event::Absent);
                let sitting = Task::batch([self.sitting_task(&event), if leaving { self.history_task(false) } else { Task::none() }]);
                if let crate::witness::Event::Kept(kept) = &event {
                    let tell = Task::batch([self.tell_task(kept).unwrap_or_else(Task::none), sitting]);
                    if !self.settings.witness_keep {
                        return tell;
                    }
                    match crate::witness::keep(kept, &crate::witness::folder()) {
                        Ok(_) => self.witness.written += 1,
                        Err(why) => self.announce(notices::Mark::Bad, self.words.t("witness-not-kept"), why, String::new(), String::new(), notices::Link::None),
                    }
                    if let Some(source) = self.settings.sources.iter_mut().find(|source| source.is_witnessed()) {
                        source.replay_count = crate::sources::witnessed(source.on).replay_count;
                    }
                    return Task::batch([self.watch_task(), tell]);
                }
                sitting
            }
            Message::WitnessSitting(_) => Task::none(),
            Message::HistoryTold(sent) => {
                self.history_running = false;
                self.witness.history_told += sent.kept;
                self.witness.history_failed = sent.failed;
                if sent.through > self.settings.history_sent {
                    self.settings.history_sent = sent.through;
                    let _ = self.settings.save();
                }
                Task::none()
            }
            Message::WitnessTold(reached) => {
                self.witness.untold = !reached;
                if !reached {
                    return Task::none();
                }
                self.witness.told += 1;
                self.companion_asked = None;
                self.map_boards_fresh.clear();
                self.community_task(true)
            }
            Message::CompanionTick => {
                let Some((beatmap, since)) = self.companion_at else {
                    return Task::none();
                };
                if Instant::now().saturating_duration_since(since) < COMPANION_DWELL {
                    return Task::none();
                }
                self.companion_asked = Some(beatmap);
                let known = self.map_boards.contains_key(&beatmap);
                self.map_board_task(beatmap, known)
            }
            Message::Sharing(message) => self.sharing_update(message),
            Message::FarmTick => self.farm_task(),
            Message::FarmHeard(farm) => {
                if farm.is_some() {
                    self.farm = farm;
                }
                Task::none()
            }
            Message::WorkerSwitch(on) => {
                self.settings.worker_on = on;
                let _ = self.settings.save();
                if on {
                    Task::batch([self.start_worker(), self.farm_task()])
                } else {
                    crate::worker::stop();
                    Task::none()
                }
            }
            Message::Worked(step) => {
                let worked = self.worked(step);
                Task::batch([worked, self.next_render()])
            }
            Message::NewsTick => self.refresh_news(false),
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
            Message::PersonDossierCached(request, entry) => {
                self.sync_dossiers();
                if entry.is_some_and(|entry| self.dossier_cache.cached(&request, entry, unix_now())) {
                    self.show_dossier(request.id)
                } else { Task::none() }
            }
            Message::PersonDossier(request, said) => {
                self.sync_dossiers();
                let Some(entry) = self.dossier_cache.finish(&request, said, unix_now()) else { return Task::none(); };
                let shown = self.show_dossier(request.id);
                let saved = ui::in_thread(move || {
                    crate::dossier_cache::save(&crate::sources::own_root(), &request, &entry);
                    Message::Nudged
                });
                Task::batch([shown, saved])
            }
            Message::Nudged => Task::none(),
            Message::Donated(count) => {
                self.donated = count;
                Task::none()
            }
            Message::Worn(result) => {
                if let Err(why) = result {
                    self.announce(notices::Mark::Bad, self.words.t("title-not-worn"), String::new(), why, String::new(), notices::Link::None);
                }
                self.community_task(true)
            }
            Message::CardShared(_) => Task::none(),
            Message::ClipFetched(link, probed) => {
                self.clips_loading.remove(&link);
                match probed {
                    Ok((path, media)) if self.overlay == Overlay::Community => self.play_clip(link, path, media),
                    Ok((path, _)) if self.ffmpeg.is_none() => {
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
            Message::MapBoard(beatmap, fresh, said) => {
                self.map_boards_waiting.remove(&beatmap);
                match said {
                    Ok(board) => {
                        self.map_boards_failed.remove(&beatmap);
                        self.map_boards.insert(beatmap, board);
                        if fresh {
                            self.map_boards_fresh.insert(beatmap);
                            Task::none()
                        } else {
                            self.map_board_task(beatmap, true)
                        }
                    }
                    Err(_) => {
                        if !self.map_boards.contains_key(&beatmap) {
                            self.map_boards_failed.insert(beatmap);
                        }
                        Task::none()
                    }
                }
            }
            Message::CommunityArrived(Ok(said)) => {
                let before = self.feed_keys();
                let saved = said.clone();
                let save = ui::in_thread(move || { crate::community::wire::save(&saved); Message::Nudged });
                self.now_unix = unix_now();
                let mut fresh = crate::community::Catalog::from_wire(said);
                if self.settings.people_everyone {
                    fresh.welcome(&self.everyone);
                }
                let previous = self.community.take();
                let selected = self.community_person.and_then(|at| previous.as_ref()?.people.get(at)).map(|person| person.id);
                if let Some(previous) = previous.as_ref().filter(|previous| !previous.staged) {
                    if matches!(previous.friends_state, crate::community::Friends::Ready | crate::community::Friends::Need(_) | crate::community::Friends::Failed) {
                        fresh.friends = previous.friends.clone();
                        fresh.friends_state = previous.friends_state.clone();
                    }
                }
                let newest = previous.as_ref().filter(|previous| !previous.staged).and_then(|previous| previous.live.last().map(|play| play.at));
                let newer = newest.map_or(0, |newest| fresh.live.iter().filter(|play| play.at > newest).count());
                self.live_shown = fresh.live.len() - newer.min(fresh.live.len());
                self.community_person = selected.and_then(|id| fresh.people.iter().position(|person| person.id == id));
                self.community = Some(fresh);
                self.sync_dossiers();
                self.people_dossiers.clear();
                if let Some(catalog) = self.community.as_mut() {
                    let people: Vec<_> = catalog.people.iter().map(|person| (person.id, person.you)).collect();
                    for (id, you) in people {
                        if let Some(entry) = self.dossier_cache.get(id) {
                            let mut dossier = catalog.take_someone(&entry.dossier);
                            dossier.person.you = you;
                            self.people_dossiers.insert(id, dossier);
                        }
                    }
                }
                self.fresh_from(before);
                self.pool_players();
                self.community_fetch = crate::community_screen::Fetch::Fresh(self.now_unix);
                let card = self.card_task(false);
                let friends = self.friends_task(false);
                Task::batch([self.community_pictures_task(), friends, card, save, self.everyone_task()])
            }
            Message::EveryoneArrived(Ok(said)) => {
                self.everyone = said.people;
                self.welcome_everyone();
                self.pool_players();
                self.community_pictures_task()
            }
            Message::EveryoneArrived(Err(_)) => Task::none(),
            Message::PinRead(Ok(pin)) => {
                let before = self.community_chat();
                self.pin = Some(pin);
                if self.community.is_some() && self.community_chat() != before {
                    return self.community_task(true);
                }
                Task::none()
            }
            Message::PinRead(Err(_)) => Task::none(),
            Message::Pinned(Ok(pin)) => {
                if pin.error.is_empty() {
                    let words = self.words.t("pin-done");
                    self.say(words);
                    self.pin = Some(pin);
                    return self.community_task(true);
                }
                let day = pin.free_at.map(|at| self.words.day(at, unix_now())).unwrap_or_default();
                let words = if pin.error == "too soon" { self.words.with("pin-too-soon", &[("day", day)]) } else { self.words.t("pin-failed") };
                self.say(words);
                self.pin = Some(crate::community::wire::Pin { error: String::new(), ..pin });
                Task::none()
            }
            Message::Pinned(Err(_)) => {
                let words = self.words.t("pin-failed");
                self.say(words);
                Task::none()
            }
            Message::CommunityArrived(Err(_)) => {
                self.community_fetch = crate::community_screen::Fetch::Failed;
                if self.community.as_ref().is_some_and(|catalog| !catalog.staged && catalog.people.is_empty() && catalog.me.is_none()) {
                    self.community = Some(self.staged_community());
                }
                Task::none()
            }
            Message::CardArrived(Ok(mut card)) => {
                card.remember_country_rank(self.community_card.as_ref(), unix_now());
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
                    let before = self.feed_keys();
                    self.live_shown += 1;
                    self.fresh_from(before);
                }
                Task::none()
            }
            Message::OpenFolder => {
                if let Some(entry) = self.chosen_entry() {
                    return reveal_path(entry.path.clone());
                }
                Task::none()
            }
            Message::Render => {
                let Some(entry) = self.chosen_entry() else {
                    return Task::none();
                };
                let path = entry.path.clone();
                if self.render_running() {
                    let waiting = self.rendering.as_ref().is_some_and(|r| r.path == path) || self.queued.iter().any(|q| q.path == path);
                    if let Some(ask) = self.render_ask(entry).filter(|_| !waiting) {
                        self.queued.push(Queued { path, ask });
                    }
                    return Task::none();
                }
                if crate::worker::drawing() {
                    let words = self.words.t("worker-busy");
                    self.say(words);
                    return Task::none();
                }
                match self.render_ask(entry) {
                    Some(ask) => self.start_render(path, ask),
                    None => Task::none(),
                }
            }
            Message::Unqueue(path) => {
                self.queued.retain(|queued| queued.path != path);
                Task::none()
            }
            Message::StopRender => {
                render::stop();
                Task::none()
            }
            Message::Rendered(step) => {
                let shown = self.rendered(step);
                Task::batch([shown, self.next_render()])
            }
            Message::GetMap => {
                let Some(entry) = self.chosen_entry() else {
                    return Task::none();
                };
                let hash = entry.map_hash.clone();
                if !self.fetch_room() || self.fetch_of(&hash).is_some_and(|f| !f.is_over()) {
                    return Task::none();
                }
                let live: Vec<&crate::sources::Source> = self.settings.sources.iter().filter(|s| s.on).collect();
                let songs = live
                    .iter()
                    .find(|s| s.kind == Kind::Own)
                    .or_else(|| live.iter().find(|s| s.kind == Kind::Folder))
                    .and_then(|s| s.songs.clone())
                    .unwrap_or_else(|| crate::sources::own_root().join("Songs"));
                let fetching = self.start_fetch(hash.clone(), Vec::new());
                maps::fetch(hash, songs, fetching.stop.clone()).map(move |step| Message::Fetched(fetching.id, step))
            }
            Message::PickMap => Task::perform(
                async {
                    let picked = rfd::AsyncFileDialog::new().add_filter("osu!", &["osz", "osu", "zip"]).pick_file().await?;
                    Some(picked.path().to_path_buf())
                },
                Message::MapPicked,
            ),
            Message::MapPicked(None) => Task::none(),
            Message::MapPicked(Some(picked)) => {
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
                let id = self.start_fetch(hash.clone(), vec![maps::Step::Checking]).id;
                ui::in_thread(move || Message::Fetched(id, maps::import(&picked, &songs, &hash)))
            }
            Message::StopFetch(id) => {
                if let Some(fetching) = self.fetching.iter().find(|f| f.id == id) {
                    fetching.stop();
                }
                Task::none()
            }
            Message::Fetched(id, step) => {
                let Some(at) = self.fetching.iter().position(|f| f.id == id) else {
                    return Task::none();
                };
                let hash = self.fetching[at].hash.clone();
                if let maps::Step::Done(map) = &step {
                    let map = map.clone();
                    self.fetching.remove(at);
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
                    let wanted = self.sharing.draw_wanted.as_deref() == Some(hash.as_str());
                    let draw = match wanted && self.chosen_entry().is_some_and(|e| e.map_hash == hash) {
                        true => self.update(Message::Render),
                        false => Task::none(),
                    };
                    if wanted {
                        self.sharing.draw_wanted = None;
                    }
                    return Task::batch([
                        draw,
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
                    maps::Step::Failed(why) if why == maps::NOT_THIS_MAP => Some(self.words.t("not-this-map")),
                    maps::Step::Failed(why) => Some(why.clone()),
                    _ => None,
                };
                self.fetching[at].reached.push(step);
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
                    return reveal_path(out.clone());
                }
                Task::none()
            }
            Message::Dropped(path) => {
                self.wake(Instant::now());
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
                self.check_rest(now);
                let dt = now.saturating_duration_since(self.now).as_secs_f32().min(1.0 / 30.0);
                if self.scenes_due {
                    self.scenes_due = false;
                    let load = self.scenes_for_notices();
                    self.now = now;
                    return load.chain(self.update(Message::Tick(now)));
                }
                if let Some(player) = &self.player {
                    let mut player = player.borrow_mut();
                    if player.pull(now) && crate::frames::on() {
                        crate::frames::mark("film", now, &player.at_ms().to_string());
                    }
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
                let watching = self.watching();
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
                for fetching in &mut self.fetching {
                    if let Some(target) = fetching.target() {
                        fetching.shown = ui::toward(fetching.shown, target, 0.12, dt);
                    }
                }
                self.marks_now = self.marks.iter().map(|(id, mark)| (id.clone(), mark.interpolate(0.0, 1.0, now))).collect();
                self.ease_slides(dt);
                self.words.typed_up_to(self.retype.interpolate(0.0, 1.0, now));
                if !self.retype.is_animating(now) {
                    self.words.settle();
                }
                for toast in &mut self.toasts {
                    if toast.shown.value() && !toast.stays && !toast.hovered && toast.age(now) > TOAST_STAY {
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
                self.feed_pictures.settle(now);
                self.thumb_pictures.settle(now);
                if !self.read_fade.value() && !self.read_fade.is_animating(now) {
                    self.community_reading = None;
                }
                self.combo_for_rested(now)
            }
            Message::ToastHover(id, over) => {
                if let Some(toast) = self.toasts.iter_mut().find(|t| t.id == id) {
                    toast.hover(over, Instant::now());
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
                let arrival = self.arrivals.get(&id).cloned();
                self.leaving.entry(id).or_insert_with(|| arrival.unwrap_or_else(|| Animation::new(true).duration(NOTICE_LEAVE).easing(Easing::EaseOutCubic))).go_mut(false, now);
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
            Message::UpdateTick => self.check_update(),
            Message::UpdateChecked(Ok(Some(release))) => {
                if matches!(&self.update, UpdateState::Getting { release: known, .. } | UpdateState::Ready { release: known, .. } if known.version == release.version) {
                    return Task::none();
                }
                self.update = UpdateState::Found(release.clone());
                if self.settings.quiet_updates || self.update_wanted {
                    return self.get_update(false);
                }
                if self.update_told.as_deref() != Some(release.version.as_str()) {
                    self.update_told = Some(release.version.clone());
                    let detail = if release.pre { format!("Dossier {}  {}", release.version, self.words.t("prerelease")) } else { format!("Dossier {}", release.version) };
                    self.announce(notices::Mark::Plain, self.words.t("update-out"), detail, String::new(), String::new(), notices::Link::Update);
                }
                Task::none()
            }
            Message::UpdateChecked(Ok(None)) => {
                self.update = UpdateState::Latest { at: chrono::Utc::now().timestamp() };
                Task::none()
            }
            Message::UpdateChecked(Err(why)) => {
                self.update = UpdateState::Failed { release: None, why };
                Task::none()
            }
            Message::UpdateGot(step) => match step {
                crate::updates::Step::Downloading { done, total } => {
                    if let UpdateState::Getting { done: was, total: all, .. } = &mut self.update {
                        *was = done;
                        if total.is_some() {
                            *all = total;
                        }
                    }
                    Task::none()
                }
                crate::updates::Step::Unpacking => Task::none(),
                crate::updates::Step::Ready(staged) => {
                    if let Some(release) = self.update.release().cloned() {
                        self.update = UpdateState::Ready { release, staged };
                    }
                    if self.update_wanted {
                        self.put_in_when_calm()
                    } else {
                        Task::none()
                    }
                }
                crate::updates::Step::Failed(why) => {
                    let release = self.update.release().cloned();
                    self.update = UpdateState::Failed { release, why };
                    self.update_wanted = false;
                    Task::none()
                }
            },
            Message::UpdateIdle => {
                if !matches!(self.update, UpdateState::Ready { .. }) || !self.calm_for_update() {
                    return Task::none();
                }
                if self.update_wanted {
                    return self.put_in(false);
                }
                let playing = self.player.as_ref().is_some_and(|player| !player.borrow().paused);
                if self.settings.quiet_updates && !playing && crate::updates::idle() >= crate::updates::IDLE {
                    return self.put_in(true);
                }
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
                    Some(notices::Link::Update) => self.get_update(true),
                    Some(notices::Link::Received(id)) => self.update(Message::Sharing(sharing::Message::Open(id))),
                    Some(notices::Link::Replay(path)) => {
                        let shown = self.update(Message::Show(Overlay::None));
                        match self.entries().iter().position(|e| e.path == path) {
                            Some(at) => {
                                self.search.clear();
                                shown.chain(self.choose(at))
                            }
                            None => shown,
                        }
                    }
                    Some(notices::Link::Page(page)) => {
                        let _ = open::that_detached(page);
                        Task::none()
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

    fn play_clip(&mut self, link: String, path: PathBuf, media: videos::Probe) {
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
        let fresh = self.player.is_none() || self.leaving_player || self.mini_player;
        if let Some(old) = self.player.take() {
            old.borrow_mut().close();
        }
        self.leaving_player = false;
        self.mini_player = false;
        let now = Instant::now();
        if fresh {
            self.stage_open = Animation::new(false).duration(STAGE_OPEN).easing(Easing::EaseOutCubic).go(true, now);
        }
        self.stirred = now;
        self.over_controls = false;
        self.controls = Animation::new(true).duration(CONTROLS_IN).easing(Easing::EaseOutCubic);
        let manner = player::Manner { level: self.settings.player_level, muted: self.settings.player_muted, rate: self.settings.player_rate };
        self.player = Some(std::rc::Rc::new(std::cell::RefCell::new(player::Player::open(&ffmpeg, &path, media, manner))));
        self.open_video = None;
        self.sharing.open = None;
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
        self.mini_player = false;
        self.open_video = None;
        self.clip = None;
        self.sharing.open = None;
        self.asking_delete = false;
        self.over_controls = false;
        self.pointer = None;
        self.shut_cinema();
    }

    fn shut_cinema(&mut self) {
        self.video_request = None;
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

    fn skin_delete_layer(&self) -> Element<'_, Message> {
        let Some(folder) = &self.skin_delete else { return Space::new().into(); };
        let w = &self.words;
        let keep = (!self.skin_deleting).then_some(Message::Prefs(prefs::Message::KeepSkin));
        let delete = (!self.skin_deleting).then_some(Message::Prefs(prefs::Message::DeleteSkin));
        let body = column![
            text(w.with("delete-skin", &[("name", crate::settings::skin_name(folder))])).font(theme::SANS_SEMI).size(20.0).wrapping(text::Wrapping::WordOrGlyph).color(INK),
            text(folder.to_string_lossy().into_owned()).font(theme::MONO).size(11.0).wrapping(text::Wrapping::WordOrGlyph).color(MUTED),
            text(w.t("skin-delete-note")).font(theme::SANS).size(13.0).color(MUTED),
            row![ui::quiet(w.t("keep"), keep.clone()), container(ui::primary(w.t(if self.skin_deleting { "skin-deleting" } else { "delete" }), delete)).id(iced::widget::Id::new("skin-delete-confirm"))].spacing(8),
        ].spacing(16);
        let card = mouse_area(container(body).padding(24).width(440).style(theme::stage)).on_press(Message::Prefs(prefs::Message::SkinDeleteTap));
        let veil = mouse_area(ui::veil(theme::DEEP_SCRIM)).on_press(Message::Prefs(prefs::Message::KeepSkin));
        stack![veil, container(card).width(Length::Fill).height(Length::Fill).center(Length::Fill)].into()
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
                let mut inside = column![container(face).width(PANEL.0).height(PANEL.1).style(ui::box_faded(theme::screen)).clip(true), label].spacing(6);
                let delete: Element<'_, Message> = match folder {
                    Some(path) => container(ui::small_button(w.t("delete"), Message::Prefs(prefs::Message::AskDeleteSkin(path.clone()))))
                        .id(iced::widget::Id::from(format!("skin-remove-{}", path.display())))
                        .width(PANEL.0).height(24).align_x(iced::alignment::Horizontal::Right).into(),
                    None => Space::new().height(24).into(),
                };
                inside = inside.push(delete);
                container(inside)
                    .padding(6)
                    .style(ui::box_faded(move |t| {
                        let look = theme::slot_choice(picked)(t, button::Status::Active);
                        container::Style { background: look.background, border: look.border, shadow: look.shadow, ..container::Style::default() }
                    }))
                    .into()
            };
            let keys = crate::settings::skin_keys(&self.skins);
            let identity = format!("{keys:?}");
            let tapped = keys.clone();
            let mut cells = vec![(usize::MAX, panel(
                w.t("own-skin-short"),
                None,
                self.skin_scenes.get(Path::new("")),
                chosen.is_none(),
            ))];
            for folder in &self.skins {
                let picked = chosen.as_deref() == Some(folder.as_path());
                let key = keys.iter().position(|path| path == folder).unwrap();
                cells.push((key, panel(crate::settings::skin_name(folder), Some(folder.clone()), self.skin_scenes.get(folder), picked)));
            }
            let title = row![
                text(w.t("skins")).font(theme::SANS_SEMI).size(20.0).color(ui::faded(INK)),
                ui::grow(),
                ui::control_button(ui::Control::Close, 18.0, Some(Message::ShowSkins(false)), false),
            ]
            .align_y(iced::Center);
            let title = column![title, text(w.t("skin-reorder-hint")).font(theme::SANS).size(11.0).color(ui::faded(FAINT))].spacing(4);
            let each = PANEL.0 + 12.0 + ROOM_GAP;
            let per = ((((self.width - 120.0).clamp(420.0, 1180.0) - 2.0 * ROOM_SIDE + ROOM_GAP) / each).floor() as usize).clamp(1, cells.len().max(1));
            let wide = per as f32 * each - ROOM_GAP + 2.0 * ROOM_SIDE;
            let lines = cells.len().div_ceil(per);
            let line_high = PANEL.1 + 12.0 + 22.0 + 12.0 + 24.0;
            let room_high = lines as f32 * line_high + (lines.saturating_sub(1)) as f32 * ROOM_GAP + ROOM_SIDE;
            let tall = (ROOM_TOP + room_high).min(self.height - 150.0).max(ROOM_TOP + line_high);
            let grid = crate::board::board(cells, ROOM_GAP, move |what, before| Message::Prefs(prefs::Message::MoveSkin(keys[what].clone(), before.and_then(|key| keys.get(key).cloned()))))
                .fixed_first(usize::MAX).identity(identity).solid(theme::ROOM_GROUND)
                .on_tap(move |key, _| Message::Prefs(prefs::Message::Skin(tapped.get(key).cloned())));
            let inside = column![
                container(title)
                    .height(ROOM_TOP)
                    .align_y(iced::alignment::Vertical::Center)
                    .padding(Padding { top: 2.0, right: ROOM_SIDE - 7.0, bottom: 0.0, left: ROOM_SIDE + 6.0 }),
                crate::glide::brim(
                    scrollable(container(grid).width(Length::Fill).padding(Padding { top: 0.0, right: ROOM_SIDE, bottom: ROOM_SIDE, left: ROOM_SIDE }))
                        .anchor_y(scrollable::Anchor::Start)
                        .style(ui::thin_scroll)
                        .direction(ui::hidden_bar())
                        .height(tall - ROOM_TOP),
                )
                .on(theme::ROOM_GROUND),
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
        let removed = self.settings.removed_skins.clone();
        let near = {
            let (sources, own) = (sources.clone(), own.clone());
            let removed = removed.clone();
            ui::in_thread(move || {
                let _ = crate::settings::adopt_skin_files_except(&crate::settings::skins_root(), &removed);
                Message::Skins(crate::settings::skins_in_except(&sources, &own, &removed))
            })
        };
        let far = ui::in_thread(move || Message::Skins(crate::settings::hunt_skins_except(&sources, &own, &removed)));
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
        while self.toasts.iter().filter(|t| t.shown.value()).count() >= self.toast_capacity() {
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
            paused_at: None,
            stays: mark == notices::Mark::Bad,
        });
    }

    pub fn view(&self) -> Element<'_, Message> {
        let k = self.enter.interpolate(0.0, 1.0, self.now);
        let rest = self.resting.interpolate(0.0, 1.0, self.now);
        let awake = 1.0 - rest;
        let s = if self.swap_waits { 0.0 } else { self.swap.interpolate(0.0, 1.0, self.now) };
        let loaded = self.library.is_some();
        let blank = || -> Element<'_, Message> { Space::new().width(Length::Fill).height(Length::Fill).into() };
        let alpha = k.min(1.0);
        let full = |handle: &image::Handle, opacity: f32| -> Element<'_, Message> {
            let picture = image(handle.clone())
                .content_fit(ContentFit::Cover)
                .width(Length::Fill)
                .height(Length::Fill)
                .opacity(opacity);
            let shade = iced::widget::canvas(ui::SceneShade { alpha: opacity, rest, curve: dim_at })
                .width(Length::Fill).height(Length::Fill);
            stack![picture, shade].into()
        };
        let buried = self.ground_fade.value() && !self.ground_fade.is_animating(self.now);
        let entry = self.chosen_entry().filter(|_| !buried);
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
                    let opacity = alpha * seen;
                    let shade = iced::widget::canvas(ui::SceneShade { alpha: opacity, rest, curve: live::dim_at })
                        .width(Length::Fill).height(Length::Fill);
                    mouse_area(stack![crate::film::show(frame, crate::film::Fit::Cover, opacity), shade]).on_press(Message::TogglePlay).into()
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
        let body: Element<'_, Message> = match (loaded, self.reading_line()) {
            (true, _) if awake > 0.001 => ui::fading(k * awake, || self.body(k, s)),
            (true, _) => blank(),
            (false, Some(line)) => pin(ui::fading(self.arrive.interpolate(0.0, 1.0, self.now), || ui::mono_small(line, FAINT))).x(40.0).y((self.height - 40.0).max(0.0)).into(),
            (false, None) => blank(),
        };
        let early = self.arrive.interpolate(0.0, 1.0, self.now) * (1.0 - self.cinema.interpolate(0.0, 1.0, self.now)) * awake;
        let crest = ui::fading(early, ui::brand);
        let crest: Element<'_, Message> =
            pin(float(crest).translate(move |_, _| Vector::new(0.0, (1.0 - early) * CREST_RISE))).x(CREST_HOME.0).y(CREST_HOME.1).into();
        let sheet = self.overlay_fade.interpolate(0.0, 1.0, self.now);
        let showing = self.overlay != Overlay::None || self.overlay_fade.is_animating(self.now);
        let late = ((k - 0.7) / 0.3).clamp(0.0, 1.0);
        let watching = 1.0 - self.cinema.interpolate(0.0, 1.0, self.now);
        let chrome_layer: Element<'_, Message> = if loaded && watching * awake > 0.001 {
            ui::fading(alpha * late * watching * awake, || self.chrome())
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
        let bubble = if self.resting.value() { blank() } else { self.bubble_layer() };
        let person = if self.overlay_drawn == Overlay::Community && showing {
            ui::fading(sheet * awake, || self.community_view(true).unwrap_or_else(blank))
        } else { blank() };
        let toasts = self.toast_layer();
        let companion = self.companion_layer();
        let menu = self.menu_layer();
        let signing = self.sign_in_layer();
        let sharing = self.share_layer();
        let failure = self.error_layer();
        let ask: Element<'_, Message> = if self.asking_delete || self.ask_fade.is_animating(self.now) {
            self.delete_card()
        } else {
            blank()
        };
        let room = self.skin_room_layer();
        let skin_ask = self.skin_delete_layer();
        let resting: Element<'_, Message> = if rest > 0.001 || self.resting.value() {
            let mark = container(ui::fading(rest, || ui::brand_scaled(REST_BRAND_SCALE))).id(iced::widget::Id::new("rest-mark"));
            let mark = pin(mark).x(CREST_HOME.0).y((self.height - 40.0 - ui::EMBLEM * REST_BRAND_SCALE).max(24.0));
            let shield = iced::widget::opaque(mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
                .on_press(Message::UserInput(None)).on_right_press(Message::UserInput(None))
                .on_middle_press(Message::UserInput(None)).on_scroll(|_| Message::UserInput(None))
                .interaction(iced::mouse::Interaction::Idle));
            stack![shield, mark].into()
        } else { blank() };
        let mini = self.mini_player_layer();
        let layers = stack![scene_before, scene, live_before, live, body, bubble, ground, overlay, chrome_layer, crest, mini, person, room, ask, sharing, menu, signing, skin_ask, failure, companion, toasts, resting];
        layers.width(Length::Fill).height(Length::Fill).into()
    }

    fn body(&self, k: f32, s: f32) -> Element<'_, Message> {
        let size = iced::Size::new(self.width, self.height);
        let surface = self.chosen_entry().and_then(|entry| self.scenes.get(&entry.map_hash))
            .map_or(theme::GROUND, |handle| ui::scene_surface(handle, size));
        let before = self.scene_before.as_ref().and_then(|handle| handle.as_ref())
            .map_or(theme::GROUND, |handle| ui::scene_surface(handle, size));
        let mut surface = ui::mix(before, surface, s);
        if let Some((_, live)) = self.chosen_entry().zip(self.live.as_ref())
            .filter(|(entry, live)| live.for_path == entry.path && live.frame.is_some() && self.overlay == Overlay::None) {
            surface = ui::mix(surface, ui::mix(iced::Color::WHITE, theme::GROUND, 0.66), live.fade.interpolate(0.0, 1.0, self.now));
        }
        column![
            Space::new().height(theme::CONTROL_HEIGHT + 4.0 + 22.0),
            Space::new().height(Length::Fill),
            ui::on_surface(surface, || self.viewer(s)),
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
            button(column![Space::new().height(2.0), text(w.t(key)).font(theme::SANS_SEMI).size(theme::BODY), Space::new().height(2.0)].spacing(4))
                .padding(0)
                .style(ui::button_faded(theme::word(on)))
                .on_press(msg)
        };
        let beckons = self.update.waiting() && self.overlay != Overlay::Settings;
        let settings_word = {
            let mut label = row![text(w.t("settings")).font(theme::SANS_SEMI).size(theme::BODY)].spacing(6).align_y(iced::Center);
            if beckons {
                let k = ui::fade();
                label = label.push(container(Space::new().width(6.0).height(6.0)).style(move |_| container::Style {
                    background: Some(iced::Background::Color(Color { a: k, ..ACCENT })),
                    border: iced::Border { radius: 3.0.into(), ..iced::Border::default() },
                    ..container::Style::default()
                }));
            }
            button(column![Space::new().height(2.0), label, Space::new().height(2.0)].spacing(4))
                .padding(0)
                .style(ui::button_faded(theme::word(self.overlay == Overlay::Settings)))
                .on_press(Message::Show(Overlay::Settings))
        };
        let places = [Overlay::None, Overlay::Videos, Overlay::Community, Overlay::Settings];
        let chosen = places.iter().position(|place| *place == self.overlay).unwrap_or(usize::MAX);
        let nav = row![
            word("replays", self.overlay == Overlay::None, Message::Show(Overlay::None)),
            word("videos", self.overlay == Overlay::Videos, Message::Show(Overlay::Videos)),
            word("community", self.overlay == Overlay::Community, Message::Show(Overlay::Community)),
            settings_word,
        ]
        .spacing(22)
        .align_y(iced::Center);
        let nav = ui::sliding(nav, chosen, ui::Pill { fill: Color { a: 0.85, ..ACCENT }, edge: Color::TRANSPARENT, radius: 1.0, underline: Some(0.0) });
        let words = row![nav, self.circle(CIRCLE_SIDE, true)].spacing(22).align_y(iced::Center);
        let top = row![Space::new().width(BRAND_WIDTH)].align_y(iced::Center).height(theme::CONTROL_HEIGHT + 4.0);
        container(top.push(ui::grow()).push(words))
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
        let from = retype(&was.from, &now.from);
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
            meta = meta.push(Space::new().width(8));
        }
        meta = meta.push(ui::mono(retype(&was.meta, &now.meta), MUTED));
        let action: Element<'_, Message> = if with_actions { self.action(entry) } else { Space::new().height(theme::CONTROL_HEIGHT).into() };
        let left = column![
            row![text(date).font(theme::MONO).size(theme::CAPTION).color(ui::faded(MUTED)), text(from).font(theme::MONO).size(theme::CAPTION).color(ui::faded(theme::SHARED))].spacing(14),
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
        let rendering_this = self.rendering.as_ref().filter(|r| r.path == entry.path);
        let fetching_this = self.fetch_of(&entry.map_hash);
        if self.queued.iter().any(|queued| queued.path == entry.path) {
            return row![ui::quiet(w.t("render-queued"), None), ui::quiet(w.t("unqueue"), Some(Message::Unqueue(entry.path.clone())))]
                .spacing(4)
                .align_y(iced::Center)
                .into();
        }
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
            (None, None, None) if entry.map.is_some() => ui::primary(w.t(if busy { "render-later" } else { "render" }), self.ffmpeg.is_some().then_some(Message::Render)),
            (None, None, None) => ui::primary(w.t("get-the-map"), self.fetch_room().then_some(Message::GetMap)),
        }
    }

    fn render_button(&self, rendering: &Rendering) -> Element<'_, Message> {
        let w = &self.words;
        match rendering.last() {
            Some(Step::Saved(..)) => ui::primary(w.t("open"), Some(Message::OpenOut)),
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
            Some(S::Nowhere) => row![ui::quiet(w.t("not-found"), Some(Message::GetMap)), ui::quiet(w.t("pick-map"), Some(Message::PickMap))].spacing(4).align_y(iced::Center).into(),
            Some(S::Failed(_)) | Some(S::Stopped) => row![ui::quiet(w.t("once-more"), Some(Message::GetMap)), ui::quiet(w.t("pick-map"), Some(Message::PickMap))].spacing(4).align_y(iced::Center).into(),
            step => {
                let label = match step {
                    None | Some(S::Looking) => w.t("looking"),
                    Some(S::Found(_)) => w.t("found"),
                    Some(S::Downloading { .. }) => w.t("fetch-downloading"),
                    Some(S::Unpacking) => w.t("unpacking-map"),
                    Some(S::Checking) => w.t("checking-map"),
                    _ => w.t("looking"),
                };
                ui::progress(label, fetching.shown, Some(Message::StopFetch(fetching.id)))
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
        None
    }

    fn worked(&mut self, step: crate::worker::Step) -> Task<Message> {
        use crate::worker::Step as W;
        match &step {
            W::Stopped => {
                self.worker_running = false;
                self.worker_step = None;
                return if self.settings.worker_on { self.start_worker() } else { Task::none() };
            }
            W::Delivered { .. } => {
                self.worker_done = self.worker_done.saturating_add(1);
                self.settings.worker_done = self.worker_done;
                let _ = self.settings.save();
                self.worker_last = Some(step.clone());
                self.worker_step = None;
                return self.farm_task();
            }
            W::HandedBack { .. } => {
                self.worker_back = self.worker_back.saturating_add(1);
                self.settings.worker_back = self.worker_back;
                let _ = self.settings.save();
                self.worker_last = Some(step.clone());
                self.worker_step = None;
                return self.farm_task();
            }
            _ => self.worker_step = Some(step),
        }
        Task::none()
    }

    fn next_render(&mut self) -> Task<Message> {
        if self.queued.is_empty() || self.render_running() || crate::worker::drawing() {
            return Task::none();
        }
        let next = self.queued.remove(0);
        self.start_render(next.path, next.ask)
    }

    pub(crate) fn shown_build(&self) -> &'static str {
        if self.gallery {
            crate::gallery::SHOWN_BUILD
        } else {
            bot::BUILD
        }
    }

    fn render_running(&self) -> bool {
        self.rendering.as_ref().is_some_and(|r| !r.is_over())
    }

    fn render_ask(&self, entry: &Entry) -> Option<render::Ask> {
        let ffmpeg = self.ffmpeg.clone()?;
        let map = entry.map.as_ref()?;
        let out = self.settings.renders_dir().join(render::file_name(&entry.player, &map.line()));
        let mut ask = render::Ask {
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
            play: render::Play::of(&self.settings),
        };
        if let Some(look) = self.sharing.looks.get(&entry.path) {
            look.dress(&mut ask);
        }
        Some(ask)
    }

    fn start_render(&mut self, path: PathBuf, ask: render::Ask) -> Task<Message> {
        self.sharing.render_look = Some(crate::inbox::Look::of(&ask));
        self.rendering = Some(Rendering { path, reached: Vec::new(), out: None });
        self.progress_shown = 0.0;
        render::run(ask).map(Message::Rendered)
    }

    fn rendered(&mut self, step: Step) -> Task<Message> {
        let mut saved = None;
        if let Some(rendering) = &mut self.rendering {
            if let Step::Saved(path, media) = &step {
                rendering.out = Some(path.clone());
                saved = Some((rendering.path.clone(), path.clone(), *media));
            }
            rendering.reached.push(step);
        }
        if let Some((replay, out, media)) = saved {
            if let Some(entry) = self.entries().iter().find(|e| e.path == replay).cloned() {
                let mut video = videos::Video::from_render(&entry, out.clone(), media.length_ms, media.width, media.height, media.fps);
                video.look = self.sharing.render_look.take();
                let detail = format!("{} — {}", video.player, video.map_line());
                let note = format!("{}  {}", self.words.length(video.length_ms), self.words.mb(video.size));
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

    fn fetch_running(&self) -> bool {
        self.fetching.iter().any(|fetching| !fetching.is_over())
    }

    fn fetch_room(&self) -> bool {
        self.fetching.iter().filter(|fetching| !fetching.is_over()).count() < FETCHES_AT_ONCE
    }

    fn fetch_of(&self, hash: &str) -> Option<&Fetching> {
        self.fetching.iter().find(|fetching| fetching.hash == hash)
    }

    fn start_fetch(&mut self, hash: String, reached: Vec<maps::Step>) -> Fetching {
        if let Some(old) = self.fetch_of(&hash) {
            old.stop();
        }
        self.fetching.retain(|fetching| fetching.hash != hash);
        self.fetch_serial += 1;
        let fetching = Fetching::new(self.fetch_serial, hash, reached);
        self.fetching.push(fetching.clone());
        fetching
    }

    fn look_ledger(&self, step: &scan::Step) -> Element<'_, Message> {
        let w = &self.words;
        let line = match step {
            scan::Step::Looking { files, found, seconds } => Line::new(Mood::Now, w.t("looking-on-device"))
                .detail(format!(
                    "{}  {}  {}",
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
        let (seen_from, seen_to) = match self.strip_view {
            Some((offset, _, shown)) => (offset - shown * STRIP_MARGIN, offset + shown * (1.0 + STRIP_MARGIN)),
            None => (0.0, (self.width - 80.0).max(0.0) * (1.0 + STRIP_MARGIN)),
        };
        let mut x = 0.0f32;
        let mut strip = row![].spacing(22).align_y(iced::alignment::Vertical::Bottom);
        for (label, list) in &days {
            let on_day = list.iter().any(|i| Some(*i) == self.chosen);
            let shade = ui::faded(if on_day { ACCENT } else { FAINT });
            let dot = container(Space::new().width(5.0).height(5.0)).style(move |_| container::Style {
                background: Some(iced::Background::Color(shade)),
                border: iced::Border { radius: 3.0.into(), ..iced::Border::default() },
                ..container::Style::default()
            });
            let head = row![dot, text(label.clone()).font(theme::MONO).size(11.0).color(ui::faded(if on_day { INK } else { FAINT }))]
                .spacing(6)
                .align_y(iced::Center);
            let mut frames = row![].spacing(FRAME_GAP).align_y(iced::alignment::Vertical::Bottom);
            let widths: Vec<f32> = list.iter().map(|at| if self.chosen == Some(*at) { theme::FRAME_W + 8.0 } else { theme::FRAME_W }).collect();
            for slot in slots(&widths, x, (seen_from, seen_to)) {
                frames = match slot {
                    Slot::Frame(index) => frames.push(self.frame(list[index], &entries[list[index]])),
                    Slot::Gap(wide) => frames.push(Space::new().width(wide).height(theme::FRAME_H)),
                };
            }
            x += widths.iter().sum::<f32>() + FRAME_GAP * widths.len().saturating_sub(1) as f32 + 22.0;
            strip = strip.push(column![head, frames].spacing(6));
        }
        let strip = scrollable(strip)
            .id(self.strip_id.clone())
            .on_scroll(Message::Strip)
            .direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new().width(0).scroller_width(0).margin(0),
            ))
            .width(Length::Fill);
        let strip = crate::glide::glide(strip, self.strip_id.clone()).aimed(self.strip_aim, self.strip_view.map_or(0.0, |(offset, _, _)| offset)).grabbed();
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
        let scrub = iced::widget::canvas(ui::Scrub { start, len, alpha: ui::fade(), on: Box::new(Message::ScrubTo) })
            .width(Length::Fill)
            .height(14.0);
        let search = iced::widget::text_input(&w.t("search-journal"), &self.search)
            .id(self.search_id.clone())
            .on_input(Message::Search)
            .font(theme::MONO)
            .size(11.0)
            .padding([3, 10])
            .width(190.0)
            .style(theme::field_faded(ui::fade()));
        let mut rail = row![scrub].align_y(iced::Center);
        if let Some(line) = self.reading_line() {
            rail = rail.push(container(ui::mono_small(line, FAINT)).padding(Padding::ZERO.left(14.0)));
        }
        let rail = rail.push(container(search).padding(Padding::ZERO.left(14.0))).push(container(counter).padding(Padding::ZERO.left(10.0)));
        let strip: Element<'_, Message> = if visible.is_empty() && !self.search.trim().is_empty() {
            container(text(w.t("search-nothing")).font(theme::SANS).size(theme::CAPTION).color(ui::faded(FAINT)))
                .height(theme::FRAME_H + 4.0 + 17.0)
                .center_y(theme::FRAME_H + 4.0 + 17.0)
                .into()
        } else {
            strip.into()
        };
        container(column![rail, container(strip).padding(Padding::ZERO.top(8.0))].spacing(2))
            .padding(Padding { top: 0.0, right: 40.0, bottom: 10.0, left: 40.0 })
            .width(Length::Fill)
            .into()
    }

    fn frame(&self, at: usize, entry: &Entry) -> Element<'_, Message> {
        let k = self.flip_k();
        let key = entry.path.display().to_string();
        let from = self.flip_at.and(self.flip_from.get(&key).copied());
        let built = if from.is_none() && k < 1.0 { ui::fading(ui::fade() * k, || self.frame_drawn(at, entry)) } else { self.frame_drawn(at, entry) };
        ui::flip(built, key, from, k).into()
    }

    fn flip_k(&self) -> f32 {
        match self.flip_at {
            Some(at) => {
                let t = (self.now.saturating_duration_since(at).as_secs_f32() / FLIP.as_secs_f32()).clamp(0.0, 1.0);
                1.0 - (1.0 - t).powi(3)
            }
            None => 1.0,
        }
    }

    fn sorting(&mut self) {
        self.flip_from = ui::places();
        self.flip_at = Some(Instant::now());
    }

    fn frame_drawn(&self, at: usize, entry: &Entry) -> Element<'_, Message> {
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
                .opacity(ui::fade() * self.thumb_pictures.shown_of(&entry.map_hash, self.now) * if chosen { 1.0 } else { 0.55 + 0.45 * rise })
                .into(),
            (None, true) => Space::new().width(w - inner).height(h - inner).into(),
            (None, false) => container(ui::fine_hatch()).width(w - inner).height(h - inner).into(),
        };
        let picture: Element<'_, Message> = match crate::mixed::is_shared(&entry.path) {
            true => {
                stack![picture, container(self.shared_mark(&entry.player)).padding(4)].into()
            }
            false => picture,
        };
        let edge = if chosen { 2.0 } else { 1.0 };
        let pressed = button(ui::clipped(container(picture).width(w - 2.0 * edge).height(h - 2.0 * edge)))
            .padding(edge)
            .style(ui::button_faded(theme::frame(chosen, lit)))
            .on_press(Message::Choose(at));
        let scale = if chosen { 1.0 } else { 1.0 + 0.03 * rise };
        ui::sensed(pressed, move |bounds| Message::Over(at, bounds), Message::HoverLeft(at))
            .keyed(at)
            .horizontal_viewport(40.0)
            .hit_padding(Padding { top: 4.0, right: 2.0, bottom: 0.0, left: 2.0 })
            .risen(2.0 * rise, scale)
            .into()
    }

    fn shared_avatar(&self, player: &str) -> Option<&str> {
        let wanted = player.to_lowercase();
        let known = self.community.as_ref().and_then(|catalog| catalog.people.iter().find(|person| person.name.to_lowercase() == wanted).map(|person| person.avatar.as_str()));
        known.or_else(|| self.everyone.iter().find(|person| person.name.to_lowercase() == wanted).map(|person| person.avatar.as_str())).filter(|avatar| !avatar.is_empty())
    }

    fn shared_mark(&self, player: &str) -> Element<'_, Message> {
        let (fill, ink) = (ui::faded(theme::SHARED), ui::faded(theme::GROUND));
        let round = move |_: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(fill)),
            border: iced::Border { radius: (SHARED_MARK / 2.0).into(), ..iced::Border::default() },
            ..container::Style::default()
        };
        match self.shared_avatar(player).and_then(|avatar| self.news_pictures.get(avatar)) {
            Some(face) => {
                let inside = SHARED_MARK - 2.0 * SHARED_RING;
                container(image(face.clone()).content_fit(ContentFit::Cover).width(inside).height(inside).border_radius(inside / 2.0).opacity(ui::fade())).center(SHARED_MARK).style(round).into()
            }
            None => {
                let initial = player.chars().next().map(|first| first.to_uppercase().to_string()).unwrap_or_default();
                container(text(initial).font(theme::MONO_BOLD).size(10.0).color(ink)).center(SHARED_MARK).style(round).into()
            }
        }
    }

    fn bubble_layer(&self) -> Element<'_, Message> {
        let Some(at) = self.hover else {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        };
        let (Some(entry), Some(bounds)) = (self.entries().get(at), self.hover_bounds) else {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        };
        let rise = self.lifts.get(&at).map_or(0.0, |lift| lift.interpolate(0.0, 1.0, self.now));
        let frame_top = bounds.y - 2.0;
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
        let gap = || Space::new().width(7);
        let [c300, c100, c50, miss] = entry.counts;
        let counts = row![
            small("300".to_owned(), theme::HIT_300),
            bold(c300.to_string(), INK),
            gap(),
            small("100".to_owned(), theme::HIT_100),
            bold(c100.to_string(), INK),
            gap(),
            small("50".to_owned(), theme::HIT_50),
            bold(c50.to_string(), INK),
            gap(),
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
            how = how.push(gap());
            how = how.push(bold(entry.outcome.mark(), if entry.outcome.is_bad() { ACCENT } else { MUTED }));
        }
        how = how.push(gap());
        how = how.push(small(
            format!("{}  {} {}", entry.client.tag(), w.day(entry.played_at, self.now_unix), w.clock(entry.played_at)),
            FAINT,
        ));
        let whose: Element<'_, Message> = if crate::mixed::is_shared(&entry.path) { self.shared_mark(&entry.player) } else { Space::new().into() };
        let head = row![
            row![whose, text(ui::shortened(entry.player.clone(), 22)).font(theme::SANS_SEMI).size(theme::BODY).wrapping(text::Wrapping::None).color(ui::faded(INK))].spacing(7).align_y(iced::Center),
            ui::grow(),
            row![
                text(entry.grade.letter()).font(theme::MONO_BOLD).size(theme::CAPTION).color(ui::faded(grade_colour(entry.grade))),
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
        let card = container(inside).padding([8, 10]).width(BUBBLE_W).height(BUBBLE_H).clip(true).style(theme::bubble_faded(alpha));
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
            if let Some(view) = self.community_view(false) {
                return view;
            }
        }
        let (name, why) = match which {
            Overlay::Community => (w.t("community"), w.t("community-why")),
            _ => (w.t("settings"), w.t("coming-later")),
        };
        let card = ui::sheet(
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
    fn mini_player_layer(&self) -> Element<'_, Message> {
        if !self.mini_player || self.leaving_player {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        }
        let Some(player) = &self.player else {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        };
        let (name, line, still) = if let Some(clip) = &self.clip {
            let still = clip.thumb.as_deref().and_then(|url| self.news_pictures.get(&crate::community_screen::wide(url)).or_else(|| self.news_pictures.get(url)));
            (clip.from.clone(), clip.said.clone(), still)
        } else if let Some(video) = self.open_video.and_then(|at| self.store.videos.get(at)) {
            (video.player.clone(), video.map_line(), self.thumbs.get(&video.map_hash))
        } else if let Some(got) = self.sharing.opened() {
            (got.player.clone(), got.map_line(), self.thumbs.get(&got.map_hash).or_else(|| self.sharing.thumbs.get(&got.id)))
        } else {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        };
        let player = player.borrow();
        let wide = (self.width - 48.0).min(356.0).max(160.0);
        let picture_h = (wide * 9.0 / 16.0).floor();
        let picture: Element<'_, Message> = match &player.frame {
            Some(frame) => crate::film::show(frame, crate::film::Fit::Contain, ui::fade()),
            None => match still {
                Some(handle) => image(handle.clone()).content_fit(ContentFit::Cover).width(Length::Fill).height(Length::Fill).opacity(0.55 * ui::fade()).into(),
                None => container(ui::fine_hatch()).width(Length::Fill).height(Length::Fill).into(),
            },
        };
        let picture = mouse_area(container(picture).width(wide).height(picture_h).style(theme::screen).clip(true))
            .on_press(Message::PlayerRestore);
        let title = column![
            text(ui::shortened(name, 28)).font(theme::SANS_SEMI).size(theme::BODY).wrapping(text::Wrapping::None).color(ui::faded(INK)),
            text(ui::shortened(line, 32)).font(theme::SANS).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(MUTED)),
        ]
        .spacing(2)
        .width(Length::Fill);
        let actions = row![
            ui::control_button(if player.paused { ui::Control::Play } else { ui::Control::Pause }, 16.0, Some(Message::PlayerToggle), false),
            ui::control_button(ui::Control::Grow, 16.0, Some(Message::PlayerRestore), false),
            ui::control_button(ui::Control::Close, 16.0, Some(Message::ClosePlayer), false),
        ]
        .spacing(0)
        .align_y(iced::Center);
        let bar = row![container(title).width(Length::Fill).clip(true), actions]
            .spacing(6)
            .align_y(iced::Center);
        let card = container(column![picture, container(bar).height(58.0).padding([4, 10]).align_y(iced::Center)])
            .width(wide)
            .style(theme::stage)
            .clip(true);
        pin(card)
            .x((self.width - wide - 24.0).max(12.0))
            .y((self.height - picture_h - 58.0 - 24.0).max(12.0))
            .into()
    }

    fn videos_lone<'a>(&'a self, content: Element<'a, Message>) -> Element<'a, Message> {
        match self.signed_in() {
            true => stack![content, container(self.videos_switch()).padding(Padding { top: 34.0, right: 0.0, bottom: 0.0, left: 40.0 })].width(Length::Fill).height(Length::Fill).into(),
            false => content,
        }
    }

    fn videos_listed<'a>(&'a self, head: Element<'a, Message>, lines: Vec<Element<'a, Message>>) -> Element<'a, Message> {
        let mut rows = column![].spacing(0).width(Length::Fill);
        if self.signed_in() {
            rows = rows.push(container(self.videos_switch()).padding(Padding { top: 0.0, right: 0.0, bottom: 20.0, left: 12.0 }));
        }
        rows = rows.push(head);
        for line in lines {
            rows = rows.push(line);
        }
        crate::glide::brim(scrollable(container(rows).padding(Padding { top: 34.0, right: 28.0, bottom: 24.0, left: 28.0 })).direction(ui::hidden_bar()).width(Length::Fill).height(Length::Fill)).into()
    }

    fn videos_view(&self) -> Element<'_, Message> {
        let w = &self.words;
        let received = self.signed_in() && self.sharing.tab == sharing::Tab::Received;
        let page: Element<'_, Message> = match (received, self.sharing.videos.is_empty(), self.store.videos.is_empty()) {
            (true, true, _) => self.videos_lone(self.received_empty()),
            (true, false, _) => self.videos_listed(self.received_head(), self.sharing.videos.iter().map(|got| self.received_row(got)).collect()),
            (false, _, true) => {
                let empty = column![
                    text(w.t("no-videos")).font(theme::SANS_SEMI).size(theme::TITLE).color(ui::faded(INK)),
                    ui::cap(w.t("render-one")),
                    container(ui::primary(w.t("back-to-replays"), Some(Message::Show(Overlay::None)))).padding(Padding::ZERO.top(12.0)),
                ]
                .spacing(6)
                .align_x(iced::Center);
                self.videos_lone(container(empty).width(Length::Fill).height(Length::Fill).center(Length::Fill).into())
            }
            (false, _, false) => self.videos_listed(self.video_head(), self.store.videos.iter().enumerate().map(|(at, video)| self.video_row(at, video)).collect()),
        };
        let sheet = column![Space::new().height(theme::CONTROL_HEIGHT + 4.0 + 22.0), page].width(Length::Fill).height(Length::Fill);
        let opened = self.stage_open.interpolate(0.0, 1.0, self.now);
        let shown = !self.mini_player && opened > 0.001;
        let stage: Element<'_, Message> = match (&self.player, self.open_video.and_then(|at| self.store.videos.get(at)), self.sharing.opened()) {
            (Some(player), Some(video), _) if shown => ui::fading(ui::fade() * opened, || self.stage(&player.borrow(), self.video_bill(video))),
            (Some(player), None, Some(got)) if shown => ui::fading(ui::fade() * opened, || self.stage(&player.borrow(), self.received_bill(got))),
            (None, None, Some(got)) if shown => ui::fading(ui::fade() * opened, || self.received_stage(got)),
            _ => Space::new().width(Length::Fill).height(Length::Fill).into(),
        };
        let sheet: Element<'_, Message> = match (self.player.is_some() || self.sharing.open.is_some()) && !self.mini_player && opened >= 0.999 {
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
                Space::new().width(VIDEO_THUMB.0 as f32),
                cell("when", VIDEO_DATE_W),
                grow("who-and-map"),
                container(ui::mono_small(w.t("mods").to_uppercase(), FAINT)).width(110.0),
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

    fn video_row(&self, at: usize, video: &videos::Video) -> Element<'_, Message> {
        let w = &self.words;
        let chosen = self.open_video == Some(at);
        let waiting = || -> Element<'_, Message> { container(ui::fine_hatch()).width(VIDEO_THUMB.0 as f32).height(VIDEO_THUMB.1 as f32).into() };
        let picture: Element<'_, Message> = match self.thumbs.get(&video.map_hash) {
            Some(handle) => {
                let shown = self.thumb_pictures.shown_of(&video.map_hash, self.now);
                let still: Element<'_, Message> = image(handle.clone())
                    .content_fit(ContentFit::Cover)
                    .width(VIDEO_THUMB.0 as f32)
                    .height(VIDEO_THUMB.1 as f32)
                    .border_radius(6.0)
                    .opacity(ui::fade() * shown * if chosen { 1.0 } else { 0.85 })
                    .into();
                if shown >= 0.999 { still } else { stack![waiting(), still].into() }
            }
            None => waiting(),
        };
        let stamp = w.compact_date(video.made_at, self.now_unix);
        let (day, time) = stamp.rsplit_once(' ').unwrap_or((&stamp, ""));
        let when = column![ui::mono_small(day.to_owned(), MUTED), ui::mono_small(time.to_owned(), FAINT)]
            .spacing(2)
            .align_x(iced::alignment::Horizontal::Center);
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
            container(when).width(VIDEO_DATE_W).align_x(iced::alignment::Horizontal::Center),
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
            ui::springy(ui::quiet(w.t("to-player"), sending_this.is_none_or(|s| s.over.is_some()).then_some(Message::Sharing(sharing::Message::Pick))), 0.04),
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
            aside: None,
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
            aside: None,
        }
    }

    fn stage<'a>(&'a self, player: &player::Player, bill: Bill<'a>) -> Element<'a, Message> {
        let w = &self.words;
        let playing = !player.paused;
        let wide = self.widened.interpolate(0.0, 1.0, self.now);
        let round: iced::border::Radius = (PICTURE_RADIUS).into();
        let Bill { still, name, mods, line, buttons, earlier, later, aside } = bill;
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
                .height(36.0);
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
        let mut title = row![
            column![
                named,
                text(ui::shortened(line, 62)).font(theme::SANS).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(MUTED)),
            ]
            .spacing(2),
            ui::grow(),
        ]
        .spacing(12)
        .align_y(iced::Center);
        if let Some(aside) = aside {
            title = title.push(aside);
        }
        let title = title
            .push(ui::control_button(ui::Control::Mini, 18.0, Some(Message::PlayerMinimize), false))
            .push(ui::control_button(ui::Control::Close, 18.0, Some(Message::ClosePlayer), false));
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
                true => format!("{}  {}", video.player, w.mb(video.size)),
                false => format!("{} — {}  {}", video.player, ui::shortened(video.map_line(), 44), w.mb(video.size)),
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
        self.video_request = None;
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
        use crate::settings::{CPU_SHARES, CRFS, CURSOR_SIZES, HEIGHTS, METER_SIZES, RATES};
        let at = |value: u32, of: &[u32]| {
            let last = (of.len().max(2) - 1) as f32;
            of.iter().position(|v| *v == value).map_or(0.5, |i| i as f32 / last)
        };
        vec![
            ("height".to_owned(), at(self.settings.render_height, &HEIGHTS)),
            ("rate".to_owned(), at(self.settings.render_fps, &RATES)),
            ("crf".to_owned(), at(self.settings.render_crf, &CRFS)),
            ("cpu".to_owned(), at(self.settings.cpu_share, &CPU_SHARES)),
            ("scale".to_owned(), crate::settings::scale_fraction(self.scale_draft.unwrap_or(if self.settings.ui_scale == 0 { ui::auto_scale() } else { self.settings.ui_scale }))),
            ("music".to_owned(), self.settings.music_level),
            ("hits".to_owned(), self.settings.hitsound_level),
            ("player".to_owned(), self.settings.player_level),
            ("dim".to_owned(), self.settings.background_dim),
            ("blur".to_owned(), self.settings.background_blur),
            ("cursor-size".to_owned(), at(self.settings.cursor_size, &CURSOR_SIZES)),
            ("meter-size".to_owned(), at(self.settings.meter_size, &METER_SIZES)),
        ]
    }

    fn remember_mark(&mut self, id: &str, on: bool) {
        let now = Instant::now();
        self.marks
            .entry(id.to_owned())
            .or_insert_with(|| Animation::new(!on).duration(MARK).easing(Easing::EaseOutCubic))
            .go_mut(on, now);
    }

    fn worker_setup(&self) -> Option<crate::worker::Setup> {
        if self.settings.token.is_empty() {
            return None;
        }
        let ffmpeg = self.ffmpeg.clone()?;
        let live: Vec<&crate::sources::Source> = self.settings.sources.iter().filter(|s| s.on).collect();
        let own_songs = live
            .iter()
            .find(|s| s.kind == Kind::Own)
            .or_else(|| live.iter().find(|s| s.kind == Kind::Folder))
            .and_then(|s| s.songs.clone())
            .unwrap_or_else(|| crate::sources::own_root().join("Songs"));
        let mut songs: Vec<PathBuf> = live.iter().filter_map(|s| s.songs.clone()).collect();
        if !songs.contains(&own_songs) {
            songs.push(own_songs.clone());
        }
        Some(crate::worker::Setup { server: self.settings.server.clone(), token: self.settings.token.clone(), name: self.settings.device.clone(), ffmpeg, songs, own_songs })
    }

    fn start_worker(&mut self) -> Task<Message> {
        if self.worker_running {
            return Task::none();
        }
        let Some(setup) = self.worker_setup() else {
            return Task::none();
        };
        self.worker_running = true;
        crate::worker::run(setup).map(Message::Worked)
    }

    fn farm_task(&self) -> Task<Message> {
        if self.settings.token.is_empty() {
            return Task::none();
        }
        let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        ui::in_thread(move || Message::FarmHeard(crate::bot::farm(&server, &token, &name).ok()))
    }

    fn fresh_from(&mut self, before: Vec<String>) {
        self.feed_arrivals.refresh(&before, &self.feed_keys(), Instant::now());
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

    fn feed_keys(&self) -> Vec<String> {
        self.community.as_ref().map_or_else(Vec::new, |catalog| crate::chronicle::event_keys(catalog, &self.news, &self.settings.news_channels, self.live_shown))
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

    fn map_board_task(&mut self, beatmap: u64, fresh: bool) -> Task<Message> {
        let staged = self.community.as_ref().is_none_or(|catalog| catalog.staged);
        if staged || self.settings.token.is_empty() || self.map_boards_waiting.contains(&beatmap) || (fresh && self.map_boards_fresh.contains(&beatmap)) {
            return Task::none();
        }
        self.map_boards_waiting.insert(beatmap);
        self.map_boards_failed.remove(&beatmap);
        let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        let chat = self.community_chat().or_else(|| self.community.as_ref().and_then(|catalog| catalog.chat));
        ui::in_thread(move || Message::MapBoard(beatmap, fresh, crate::bot::map_board(&server, &token, &name, chat, beatmap, fresh).map_err(|e| e.to_string())))
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
        let chat = self.community_chat();
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
        let chat = self.community_chat();
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

    fn sync_dossiers(&mut self) {
        let chat = self.community_chat().or_else(|| self.community.as_ref().and_then(|catalog| catalog.chat)).unwrap_or(0);
        let scope = if self.settings.token.is_empty() { String::new() } else {
            crate::dossier_cache::scope(&self.settings.server, &self.settings.token, &self.settings.device, chat)
        };
        if self.dossier_cache.configure(scope) { self.people_dossiers.clear(); }
    }

    fn show_dossier(&mut self, id: i64) -> Task<Message> {
        let Some(entry) = self.dossier_cache.get(id).cloned() else { return Task::none(); };
        let Some(catalog) = self.community.as_mut() else { return Task::none(); };
        let Some(at) = catalog.people.iter().position(|person| person.id == id) else { return Task::none(); };
        let name = catalog.people[at].name.to_lowercase();
        let mut dossier = catalog.take_someone(&entry.dossier);
        dossier.person.you = catalog.people[at].you;
        self.people_dossiers.insert(id, dossier);
        let mut tasks = vec![self.community_pictures_task()];
        if let Some(mut card) = entry.dossier.card.filter(|card| card.pp > 0.0 || !card.username.is_empty()) {
            card.remember_country_rank(self.people_cards.get(&name), unix_now());
            let wanted = card.pictures();
            self.people_cards.entry(name).or_insert(card);
            tasks.push(self.pictures_task(wanted));
        }
        Task::batch(tasks)
    }

    fn person_task(&mut self, at: usize) -> Task<Message> {
        self.sync_dossiers();
        let Some(person) = self.community.as_ref().and_then(|catalog| catalog.people.get(at)).cloned() else {
            return Task::none();
        };
        let staged = self.community.as_ref().is_none_or(|catalog| catalog.staged);
        let chat = self.community_chat().or_else(|| self.community.as_ref().and_then(|catalog| catalog.chat));
        match (staged || self.settings.token.is_empty(), chat) {
            (false, Some(chat)) => {
                let Some(request) = self.dossier_cache.request(person.id, Instant::now(), unix_now()) else {
                    return self.scrape_task(at);
                };
                let (server, token, device, id) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone(), person.id);
                let outside = person.player.filter(|_| person.outside);
                let asked = ui::streamed(move |push| {
                    let cached = crate::dossier_cache::load(&crate::sources::own_root(), &request);
                    if !push(Message::PersonDossierCached(request.clone(), cached)) { return; }
                    let said = match outside {
                        Some(player) => crate::bot::player(&server, &token, &device, player),
                        None => crate::bot::person(&server, &token, &device, chat, id),
                    }
                    .map_err(|e| e.to_string());
                    let _ = push(Message::PersonDossier(request, said));
                });
                Task::batch([asked, self.scrape_task(at)])
            }
            _ => self.scrape_task(at),
        }
    }

    fn scrape_task(&mut self, at: usize) -> Task<Message> {
        let Some(person) = self.community.as_ref().and_then(|catalog| catalog.people.get(at)).cloned() else {
            return Task::none();
        };
        let name = person.name.to_lowercase();
        let cached = if !self.people_cards.contains_key(&name) { crate::osu_profile::load_player(&name) } else { None };
        let pictures = if let Some(card) = cached {
            let wanted = card.pictures();
            self.people_cards.insert(name.clone(), card);
            self.pictures_task(wanted)
        } else { Task::none() };
        if !self.people_asked.insert(format!("card:{name}")) {
            return pictures;
        }
        let asked = person.name.clone();
        Task::batch([pictures, ui::in_thread(move || Message::PersonCard(asked.to_lowercase(), crate::osu_profile::fetch(&asked)))])
    }

    fn donate_task(&self) -> Task<Message> {
        if !self.settings.donate_replays || self.settings.token.is_empty() {
            return Task::none();
        }
        let plays: Vec<crate::donate::Play> = self
            .entries()
            .iter()
            .filter(|entry| !crate::mixed::is_shared(&entry.path))
            .map(|entry| crate::donate::Play { path: entry.path.clone(), hash: entry.replay_hash.clone() })
            .collect();
        if plays.is_empty() {
            return Task::none();
        }
        let (server, token, device) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        ui::streamed(move |push| {
            let _ = crate::donate::give(&server, &token, &device, &plays, &crate::donate::ledger(), |count| {
                push(count);
            });
        })
        .map(Message::Donated)
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
                let bytes = crate::news::picture(crate::community::fetched(&url));
                if let Some(frost) = bytes.as_deref().filter(|_| side == crate::community::COVER).and_then(crate::community_screen::Frost::of) {
                    if !push(Message::NewsFrost(url.clone(), frost)) {
                        return;
                    }
                }
                let handle = bytes.and_then(|bytes| match side {
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
        for dossier in self.people_dossiers.values() {
            if !dossier.person.avatar.is_empty() { wanted.push((dossier.person.avatar.clone(), 256)); }
            if !dossier.person.cover.is_empty() { wanted.push((dossier.person.cover.clone(), crate::community::COVER)); }
        }
        if let Some(card) = self.shown_card.clone().or_else(|| catalog.card_of()) {
            wanted.extend(card.pictures());
        }
        let mut sharing: Vec<&str> = self.entries().iter().filter(|entry| crate::mixed::is_shared(&entry.path)).map(|entry| entry.player.as_str()).collect();
        sharing.sort_unstable();
        sharing.dedup();
        wanted.extend(sharing.into_iter().filter_map(|player| self.shared_avatar(player)).map(|avatar| (avatar.to_owned(), 128)));
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
                    let bytes = crate::news::picture(crate::community::fetched(&url));
                    if let Some(frost) = bytes.as_deref().filter(|_| side == crate::community::COVER).and_then(crate::community_screen::Frost::of) {
                        if !push(Message::NewsFrost(url.clone(), frost)) {
                            return;
                        }
                    }
                    let handle = bytes.and_then(|bytes| match side {
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

    fn community_entries(&self) -> Vec<crate::sidebar::Entry<Message>> {
        use crate::community_screen::{Message as C, PeopleFrom, Section};
        use crate::glyphs::Icon;
        use crate::sidebar::Entry;
        let w = &self.words;
        let section = self.community_section;
        let item = |key: &'static str, icon: Icon, label: String, on: bool, section: Section, from: Option<PeopleFrom>| Entry::Item { key, icon, label, on, press: Message::Community(C::Go(section, from)) };
        let people = |from: PeopleFrom| section == Section::People && self.people_from == from;
        vec![
            item("profile", Icon::Person, w.t("community-profile"), section == Section::Profile, Section::Profile, None),
            item("feed", Icon::News, w.t("community-feed"), section == Section::Feed, Section::Feed, None),
            Entry::Caption(w.t("community-people")),
            item("chat", Icon::Chat, w.t(if self.settings.people_everyone { "people-everyone-tab" } else { "people-chat" }), people(PeopleFrom::Chat), Section::People, Some(PeopleFrom::Chat)),
            item("game", Icon::Circle, w.t("people-game"), people(PeopleFrom::Game), Section::People, Some(PeopleFrom::Game)),
            item("compare", Icon::Compare, w.t("community-compare"), section == Section::Compare, Section::Compare, None),
            Entry::Caption(w.t("community-standing")),
            item("boards", Icon::Chart, w.t("community-boards"), section == Section::Boards, Section::Boards, None),
            item("titles", Icon::Trophy, w.t("community-titles"), section == Section::Titles, Section::Titles, None),
        ]
    }

    fn settings_entries(&self) -> Vec<crate::sidebar::Entry<Message>> {
        use crate::glyphs::Icon;
        use crate::sidebar::Entry;
        let w = &self.words;
        vec![
            Entry::Item { key: "app-side", icon: Icon::Gear, label: w.t("app-side"), on: self.side == Side::App, press: Message::Prefs(prefs::Message::Side(Side::App)) },
            Entry::Item { key: "bot-side", icon: Icon::Send, label: w.t("bot-side"), on: self.side == Side::Bot, press: Message::Prefs(prefs::Message::Side(Side::Bot)) },
        ]
    }

    fn sided<'a>(&'a self, entries: Vec<crate::sidebar::Entry<Message>>, body: Element<'a, Message>, scale: f32) -> Element<'a, Message> {
        let entries = crate::sidebar::arranged(entries, &self.settings.side_order);
        let open = self.side_open.interpolate(0.0, 1.0, self.now);
        let shrink = ((self.width - crate::sidebar::width_at(open)) / (self.width - crate::sidebar::NARROW)).clamp(0.3, 1.0);
        let side = ui::fading(ui::fade() * self.overlay_fade.interpolate(0.0, 1.0, self.now), || crate::sidebar::view(entries, open, Message::SideOpen, Message::SideMoved));
        let body: Element<'a, Message> = ui::scaled(body, scale * shrink).into();
        let dimmed = stack![body, ui::veil(Color { a: SIDE_DIM * open, ..theme::GROUND })].width(Length::Fill).height(Length::Fill);
        column![Space::new().height(theme::CONTROL_HEIGHT + 4.0 + 22.0), row![side, dimmed].height(Length::Fill)].width(Length::Fill).height(Length::Fill).into()
    }

    fn community_ground<'a>(&'a self, catalog: &'a crate::community::Catalog, person_only: bool) -> crate::community_screen::Ground<'a> {
        let folds: HashMap<String, f32> = self
            .feed_fold_at
            .iter()
            .map(|(key, at)| {
                let x = (self.now.saturating_duration_since(*at).as_secs_f32() / FOLD.as_secs_f32()).clamp(0.0, 1.0);
                let eased = 1.0 - (1.0 - x).powi(3);
                (key.clone(), if self.feed_open.contains(key) { eased } else { 1.0 - eased })
            })
            .collect();
        crate::community_screen::Ground {
            words: &self.words,
            catalog,
            thumbs: &self.thumbs,
            section: self.community_section,
            board: self.community_board,
            person: self.community_person.filter(|_| self.person_fade.value() || self.person_fade.is_animating(self.now)),
            person_k: self.person_fade.interpolate(0.0, 1.0, self.now),
            person_card: self.community_person.and_then(|at| self.community.as_ref()?.people.get(at)).and_then(|person| self.people_cards.get(&person.name.to_lowercase())),
            person_dossier: self.community_person.and_then(|at| self.community.as_ref()?.people.get(at)).and_then(|person| self.people_dossiers.get(&person.id)),
            person_loading: self.community_person.and_then(|at| self.community.as_ref()?.people.get(at)).is_some_and(|person| self.people_asked.contains(&format!("card:{}", person.name.to_lowercase())) || self.dossier_cache.loading(person.id)),
            now_unix: self.now_unix,
            news: &self.news,
            pictures: &self.news_pictures,
            frosts: &self.news_frosts,
            loading: &self.news_loading,
            failed: &self.news_failed,
            channels: &self.settings.news_channels,
            channel_draft: &self.channel_draft,
            fetch: self.community_fetch,
            width: if person_only { self.width } else { self.width - crate::sidebar::NARROW } / COMMUNITY_SCALE,
            filter: self.feed_filter,
            source: self.feed_source,
            stream: self.feed_stream,
            query: &self.feed_query,
            open_events: &self.feed_open,
            folds,
            panel_from: self.panel_from,
            spot: self.spot,
            spot_k: {
                let k = (self.now.saturating_duration_since(self.spot_at).as_secs_f32() / crate::chronicle::SPOT_SWAP.as_secs_f32()).clamp(0.0, 1.0);
                1.0 - (1.0 - k).powi(3)
            },
            section_t: self.now.saturating_duration_since(self.section_at).as_secs_f32().min(60.0),
            shift_t: self.now.saturating_duration_since(self.shift_at).as_secs_f32().min(60.0),
            stream_t: self.now.saturating_duration_since(self.stream_at).as_secs_f32().min(60.0),
            person_t: self.now.saturating_duration_since(self.person_at).as_secs_f32().min(60.0),
            play_t: self.now.saturating_duration_since(self.play_at).as_secs_f32().min(60.0),
            group_t: self.now.saturating_duration_since(self.group_at).as_secs_f32().min(60.0),
            news_t: self.now.saturating_duration_since(self.news_at).as_secs_f32().min(60.0),
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
            boards: &self.map_boards,
            boards_waiting: &self.map_boards_waiting,
            boards_failed: &self.map_boards_failed,
            score_scale: self.score_scale,
            clips_loading: &self.clips_loading,
            reading: self.community_reading.as_ref(),
            read_above: self.read_over_person,
            read_k: self.read_fade.interpolate(0.0, 1.0, self.now),
            people_from: self.people_from,
            everyone: self.settings.people_everyone,
            pool: &self.compare_pool,
            compare: &self.compare,
            compare_query: &self.compare_query,
            people_query: &self.people_query,
            standing: self.community_standing,
            card: self.shown_card.as_ref(),
            flags: &self.flags,
            avatar: self.avatar.as_ref(),
            chat: self.chat_name(),
            live_shown: self.live_shown,
            arrivals: self.feed_arrivals.ages(self.now),
            picture_came: self.feed_pictures.shown(self.now),
            picture_lost: self.feed_pictures.room(self.now),
            pictures_asked: &self.news_asked,
        }
    }

    fn community_view(&self, person_only: bool) -> Option<Element<'_, Message>> {
        let catalog = self.community.as_ref()?;
        let ground = self.community_ground(catalog, person_only);
        if person_only {
            let at = ground.person.filter(|at| *at < catalog.people.len() && ground.person_k > 0.001)?;
            let panel = crate::community_screen::profile_panel(&ground, at).map(Message::Community);
            let panel: Element<'_, Message> = ui::scaled(panel, COMMUNITY_SCALE).into();
            let shield = iced::widget::opaque(mouse_area(ui::veil(Color::from_rgba(0.027, 0.012, 0.016, 0.88 * ground.person_k)))
                .on_press(Message::Community(crate::community_screen::Message::Person(None))).interaction(iced::mouse::Interaction::Idle));
            let page = column![Space::new().height(theme::CONTROL_HEIGHT + 26.0), panel].width(Length::Fill).height(Length::Fill);
            let mut layers: Vec<Element<'_, Message>> = vec![shield.into(), page.into()];
            if let Some(sheet) = crate::community_screen::reading_above(&ground) {
                let sheet: Element<'_, Message> = ui::scaled(sheet.map(Message::Community), COMMUNITY_SCALE).into();
                layers.push(column![Space::new().height(theme::CONTROL_HEIGHT + 26.0), sheet].width(Length::Fill).height(Length::Fill).into());
            }
            return Some(iced::widget::Stack::with_children(layers).width(Length::Fill).height(Length::Fill).into());
        }
        let opened = self.stage_open.interpolate(0.0, 1.0, self.now);
        let stage = match (&self.player, &self.clip) {
            (Some(player), Some(clip)) if !self.mini_player && opened > 0.001 => Some(ui::fading(ui::fade() * opened, || self.stage(&player.borrow(), self.clip_bill(clip)))),
            _ => None,
        };
        let body: Element<'_, Message> = crate::community_screen::view(&ground).map(Message::Community);
        let page = self.sided(self.community_entries(), body, COMMUNITY_SCALE);
        let stage = stage.unwrap_or_else(|| Space::new().width(Length::Fill).height(Length::Fill).into());
        Some(stack![page, stage].width(Length::Fill).height(Length::Fill).into())
    }

    fn settings_view(&self) -> Element<'_, Message> {
        let ground = prefs::Ground {
            words: &self.words,
            settings: &self.settings,
            machine: if self.gallery { ui::Machine::Mac } else { ui::Machine::here() },
            machine_name: if self.gallery { "MacBook Pro".to_owned() } else { prefs::machine_name() },
            side: self.side,
            replays: self.entries().len(),
            videos: self.store.videos.len(),
            videos_size: self.sizes.0.max(self.store.total_size()),
            maps: self.library.as_ref().map_or(0, |l| l.maps),
            maps_size: self.sizes.1,
            cache_size: self.sizes.2,
            storage: &self.storage,
            ffmpeg: self.ffmpeg_version.clone(),
            ffmpeg_found: self.ffmpeg.is_some(),
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
            update: &self.update,
            update_waits: self.update_wanted && matches!(self.update, UpdateState::Ready { .. }) && !self.calm_for_update(),
            worker: self.worker_step.as_ref(),
            worker_last: self.worker_last.as_ref(),
            worker_done: self.worker_done,
            donated: self.donated,
            tray: self.gallery || crate::tray::available(),
            build: self.shown_build(),
            pin: self.pin.as_ref(),
            now_unix: self.now_unix,
            worker_back: self.worker_back,
            farm: self.farm.as_ref(),
            scale_draft: self.scale_draft,
            accept: self.sharing.accept,
            witness: &self.witness,
            accept_ready: self.sharing.loaded && self.sharing.registered,
            accept_unregistered: self.sharing.loaded && !self.sharing.registered,
            share_replays: self.sharing.replays.as_ref(),
        };
        let body: Element<'_, Message> = Element::from(prefs::view(&ground)).map(Message::Prefs);
        self.sided(self.settings_entries(), body, 1.0)
    }

    fn prefs(&mut self, message: prefs::Message) -> Task<Message> {
        use prefs::Message as P;
        let keep = |settings: &Settings| {
            let _ = settings.save();
        };
        match message {
            P::Side(side) => {
                let order = |side: Side| match side {
                    Side::App => 0,
                    Side::Bot => 1,
                };
                if side != self.side {
                    self.side_swap = if order(side) > order(self.side) { 1.0 } else { -1.0 };
                    self.side_fade = Animation::new(false).duration(TAB_FADE).easing(Easing::EaseOutCubic).go(true, Instant::now());
                }
                self.side = side;
                let name = match side {
                    Side::App => "app",
                    Side::Bot => "bot",
                };
                if self.settings.settings_tab != name {
                    self.settings.settings_tab = name.to_owned();
                    keep(&self.settings);
                }
                if side == Side::Bot {
                    return self.farm_task();
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
            P::Donate(on) => {
                self.remember_mark("donate", on);
                self.settings.donate_replays = on;
                crate::donate::allow(on);
                keep(&self.settings);
                self.donate_task()
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
            P::Scale(at) => {
                self.slid_at.insert("scale".to_owned(), Instant::now());
                self.scale_draft = Some(crate::settings::scale_at(at));
                Task::none()
            }
            P::ExportedOnly(on) => {
                self.remember_mark("exported-only", on);
                self.settings.exported_only = on;
                library::only_exported(on);
                keep(&self.settings);
                if self.refreshing {
                    return Task::none();
                }
                self.hush_next = true;
                self.refreshing = true;
                self.refreshed_at = Instant::now();
                read_library(self.settings.sources.clone(), Message::Refreshed)
            }
            P::AutoFlip(on) => {
                self.remember_mark("auto-flip", on);
                self.settings.auto_flip = on;
                keep(&self.settings);
                Task::none()
            }
            P::ScaleDone => {
                let Some(chosen) = self.scale_draft.take() else {
                    return Task::none();
                };
                if chosen == self.settings.ui_scale {
                    return Task::none();
                }
                let before = ui::scale_of(self.settings.ui_scale);
                self.remember_mark("auto-scale", false);
                self.settings.ui_scale = chosen;
                keep(&self.settings);
                self.rescaled(before)
            }
            P::PeopleEveryone(on) => {
                self.remember_mark("people-everyone", on);
                self.settings.people_everyone = on;
                keep(&self.settings);
                self.welcome_everyone();
                Task::batch([self.everyone_task(), self.replays_sync_task(true)])
            }
            P::Accept(accept) => self.sharing_update(sharing::Message::Accept(accept)),
            P::Pin(chat) => {
                if self.pin.as_ref().and_then(|pin| pin.chat) == Some(chat) || self.settings.token.is_empty() {
                    return Task::none();
                }
                let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
                ui::in_thread(move || Message::Pinned(crate::bot::pin(&server, &token, &name, chat).map_err(|e| e.to_string())))
            }
            P::CloseToTray(on) => {
                self.remember_mark("close-to-tray", on);
                self.settings.close_to_tray = on;
                keep(&self.settings);
                Task::none()
            }
            P::WitnessHistory(on) => {
                self.remember_mark("witness-history", on);
                self.settings.history_share = on;
                if on {
                    self.settings.history_sent = 0;
                    self.witness.history_failed = false;
                }
                keep(&self.settings);
                if on { self.history_task(true) } else { Task::none() }
            }
            P::WitnessCompanion(on) => {
                self.remember_mark("witness-companion", on);
                self.settings.witness_companion = on;
                self.companion_follow();
                keep(&self.settings);
                Task::none()
            }
            P::Witness(on) => {
                self.remember_mark("witness", on);
                self.settings.witness_keep = on;
                if on {
                    self.witness_source();
                }
                keep(&self.settings);
                Task::none()
            }
            P::AutoScale(on) => {
                self.remember_mark("auto-scale", on);
                let before = ui::scale_of(self.settings.ui_scale);
                self.settings.ui_scale = if on { 0 } else { ui::auto_scale() };
                keep(&self.settings);
                self.rescaled(before)
            }
            P::Cpu(at) => {
                self.slid_at.insert("cpu".to_owned(), Instant::now());
                self.settings.cpu_share = prefs::nearest(at, &crate::settings::CPU_SHARES);
                crate::render::share_cpu(self.settings.cpu_share);
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
                let sync = match on && self.settings.sources.get(at).is_some_and(|source| source.is_shared()) {
                    true => self.replays_sync_task(true),
                    false => Task::none(),
                };
                let sources = self.settings.sources.clone();
                Task::batch([sync, ui::in_thread(move || library::read(&sources)).map(Message::Loaded)])
            }
            P::ShareReplays(on) => self.sharing_update(sharing::Message::ShareReplays(on)),
            P::LinkTelegram => self.update(Message::LinkTelegram),
            P::RemoveSource(at) => {
                if at >= self.settings.sources.len() {
                    return Task::none();
                }
                self.settings.sources.remove(at);
                keep(&self.settings);
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
                self.dossier_cache.clear();
                self.people_dossiers.clear();
                self.sizes.2 = 0;
                self.flags.clear();
                self.flags_asked.clear();
                let renders = self.settings.renders_dir();
                ui::in_thread(move || Message::Stored(prefs::measure(&renders)))
            }
            P::CheckBuild => self.check_update(),
            P::Update => self.get_update(true),
            P::WhatsNew => {
                let page = self.update.release().map(|release| release.page.clone()).filter(|page| !page.is_empty()).unwrap_or_else(|| crate::updates::PAGE.to_owned());
                let _ = open::that_detached(page);
                Task::none()
            }
            P::QuietUpdates(on) => {
                self.remember_mark("quiet-updates", on);
                self.settings.quiet_updates = on;
                keep(&self.settings);
                if on {
                    self.get_update(false)
                } else {
                    Task::none()
                }
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
                self.sync_dossiers();
                self.settings.chat_title = self.chats.iter().find(|chat| chat.id == id).map(|chat| chat.title.clone()).unwrap_or_default();
                keep(&self.settings);
                Task::none()
            }
            P::Worker(on) => {
                self.remember_mark("worker", on);
                self.update(Message::WorkerSwitch(on))
            }
            P::Skin(folder) => {
                if self.skin_deleting { return Task::none(); }
                let folder = match folder.as_deref().filter(|path| crate::settings::is_skin_file(path)) {
                    Some(file) => match crate::settings::unpack_skin(file) {
                        Ok(made) => {
                            let sources = self.settings.sources.clone();
                            let own = self.settings.own_skins.clone();
                            let removed = self.settings.removed_skins.clone();
                            let again = ui::in_thread(move || Message::Skins(crate::settings::hunt_skins_except(&sources, &own, &removed)));
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
            P::MoveSkin(what, before) => {
                if crate::settings::move_skin(&mut self.skins, &what, before.as_deref()) {
                    self.settings.skin_order = self.skins.clone();
                    keep(&self.settings);
                }
                Task::none()
            }
            P::AskDeleteSkin(folder) => {
                if !self.skin_deleting && self.skins.contains(&folder) { self.skin_delete = Some(folder); }
                Task::none()
            }
            P::KeepSkin => {
                if !self.skin_deleting { self.skin_delete = None; }
                Task::none()
            }
            P::SkinDeleteTap => Task::none(),
            P::DeleteSkin => {
                let Some(folder) = self.skin_delete.clone().filter(|path| self.skins.contains(path)) else { return Task::none(); };
                if self.skin_deleting { return Task::none(); }
                self.skin_deleting = true;
                ui::in_thread(move || { let result = videos::to_bin(&folder); Message::SkinDeleted(folder, result) })
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
                    self.settings.restore_skin(&folder);
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
            P::Effect(effect, on) => {
                self.remember_mark(effect.tag(), on);
                self.settings.set_effect(effect, on);
                keep(&self.settings);
                Task::none()
            }
            P::CursorSize(at) => {
                self.slid_at.insert("cursor-size".to_owned(), Instant::now());
                self.settings.cursor_size = prefs::nearest(at, &crate::settings::CURSOR_SIZES);
                keep(&self.settings);
                Task::none()
            }
            P::MeterSize(at) => {
                self.slid_at.insert("meter-size".to_owned(), Instant::now());
                self.settings.meter_size = prefs::nearest(at, &crate::settings::METER_SIZES);
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
const VIDEO_DATE_W: f32 = 80.0;
const CIRCLE_SIDE: f32 = 28.0;
const AVATAR_SIDE: u32 = 80;
const MENU_W: f32 = 400.0;
const MENU_TOP: f32 = 80.0;
const BADGE: f32 = 18.0;
const BADGE_OUT: f32 = 4.0;
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
            let mut list = column![].spacing(if tab == Tab::Feed { 0 } else { 8 }).width(MENU_W);
            for tile in content {
                list = list.push(tile);
            }
            let list: Element<'_, Message> = if tab == Tab::Feed {
                let height = (self.height - MENU_TOP - 120.0 - 24.0).max(140.0);
                let scroll: Element<'_, Message> = scrollable(container(list).padding(Padding::ZERO.bottom(16.0)))
                    .direction(ui::hidden_bar()).height(height).width(MENU_W).into();
                ui::scroll_fades(scroll, MENU_W, height)
            } else { list.into() };
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
            .style(ui::box_faded(move |theme| if bad { theme::tile_bad(theme) } else { theme::notification(false)(theme) }))
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
                under = format!("{under}  ID {}", me.telegram_id);
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
        let active = match tab {
            Tab::Account => 0,
            Tab::Feed => 1,
            Tab::Stats => 2,
        };
        let mut words = row![].spacing(4).width(Length::Fill);
        for (key, this) in [("account", Tab::Account), ("feed", Tab::Feed), ("stats", Tab::Stats)] {
            words = words.push(
                button(container(text(w.t(key)).font(theme::SANS_SEMI).size(theme::CAPTION)).center_x(Length::Fill).center_y(28.0))
                    .width(Length::FillPortion(1))
                    .padding(0)
                    .style(ui::button_faded(theme::segment(tab == this)))
                    .on_press(Message::MenuTab(this)),
            );
        }
        let face = ui::sliding(words, active, ui::Pill {
            fill: Color::from_rgba(1.0, 1.0, 1.0, 0.08), edge: Color::TRANSPARENT, radius: 8.0, underline: None,
        });
        container(face).padding(4).width(MENU_W).height(36.0).style(ui::box_faded(theme::notification(false))).into()
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
                container(text(value).font(theme::MONO_BOLD).size(22.0).wrapping(text::Wrapping::None).color(ui::faded(INK)))
                    .width(Length::Fill)
                    .clip(true),
                container(text(key).font(theme::SANS).size(11.0).wrapping(text::Wrapping::None).color(ui::faded(FAINT)))
                    .width(Length::Fill)
                    .clip(true),
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
                self.big(self.entries().len().to_string(), w.t("replays-in-journal")),
            ];
        }
        let rows = column![self.kv(w.t("videos-go-to"), self.chat_name()), self.kv(w.t("worker"), prefs::worker_said(w, self.worker_step.as_ref(), self.settings.worker_on).0)].spacing(2);
        vec![self.card(rows.into(), false)]
    }

    fn chats_task(&self) -> Task<Message> {
        if self.settings.token.is_empty() || !self.chats.is_empty() {
            return Task::none();
        }
        let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        let chats = ui::in_thread(move || Message::Chats(crate::bot::chats(&server, &token, &name).unwrap_or_default()));
        Task::batch([chats, self.pin_task()])
    }

    fn community_chat(&self) -> Option<i64> {
        self.pin.as_ref().and_then(|pin| pin.chat).or(self.settings.chat_id).filter(|id| *id < 0)
    }

    pub(crate) fn pool_players(&mut self) {
        let Some(catalog) = self.community.as_ref() else {
            self.compare_pool.clear();
            return;
        };
        let mut pool = catalog.people.clone();
        pool.extend(catalog.strangers(&self.everyone));
        self.compare_pool = pool;
        let known: Vec<i64> = self.compare_pool.iter().map(|person| person.id).collect();
        self.compare.retain(|id| known.contains(id));
    }

    fn everyone_task(&self) -> Task<Message> {
        let staged = self.community.as_ref().is_none_or(|catalog| catalog.staged);
        let wanted = self.settings.people_everyone || self.community_section == crate::community_screen::Section::Compare;
        if !wanted || self.settings.token.is_empty() || staged {
            return Task::none();
        }
        let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        ui::in_thread(move || Message::EveryoneArrived(crate::bot::players(&server, &token, &name).map_err(|e| e.to_string())))
    }

    fn welcome_everyone(&mut self) {
        let Some(catalog) = self.community.as_mut().filter(|catalog| !catalog.staged) else {
            return;
        };
        let selected = self.community_person.and_then(|at| catalog.people.get(at)).map(|person| person.id);
        if self.settings.people_everyone {
            catalog.welcome(&self.everyone);
        } else {
            catalog.farewell();
        }
        self.community_person = selected.and_then(|id| catalog.people.iter().position(|person| person.id == id));
        self.pool_players();
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
                    container(text(detail.to_owned()).font(theme::SANS).size(theme::CAPTION).wrapping(text::Wrapping::None).color(ui::faded(MUTED))).width(Length::Fill).clip(true),
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
        for queued in &self.queued {
            let who = self.entries().iter().find(|e| e.path == queued.path).map(|e| e.title().unwrap_or_default()).unwrap_or_default();
            tiles.push(self.card(job(w.t("render-queued"), who, 0.0), false));
        }
        for fetching in self.fetching.iter().filter(|f| !f.is_over()) {
            let title = self.entries().iter().find(|e| e.map_hash == fetching.hash).map(|e| e.title().unwrap_or_default()).unwrap_or_default();
            tiles.push(self.card(job(w.t("fetch-downloading"), title, fetching.shown), false));
        }
        if let Some(sending) = self.sending.as_ref().filter(|s| s.over.is_none()) {
            let title = self.store.videos.iter().find(|v| v.path == sending.path).map(|v| v.map_line()).unwrap_or_default();
            tiles.push(self.card(job(w.t("sending"), title, self.progress_shown), false));
        }
        if self.notices.notices.len() > 5 {
            let clear = row![
                Space::new().width(Length::Fill),
                ui::quiet(w.t("clear-feed"), Some(Message::ClearNotices)),
            ]
            .align_y(iced::Center);
            tiles.push(self.card(container(clear).padding(Padding::ZERO.bottom(2.0)).into(), false));
        }
        let mut tiles: Vec<Element<'_, Message>> = tiles.into_iter().map(|tile| container(tile).padding(Padding::ZERO.bottom(8.0)).into()).collect();
        for notice in &self.notices.notices {
            let alive = match (self.leaving.get(&notice.id), self.arrivals.get(&notice.id)) {
                (Some(going), _) => going.interpolate(0.0, 1.0, self.now),
                (None, Some(coming)) => coming.interpolate(0.0, 1.0, self.now),
                _ => 1.0,
            };
            let tile = ui::fading(ui::fade() * alive, || {
                ui::grown(self.notification_card(notice, None, MENU_W), Point::new(1.0, 0.5), 0.0, 1.0).shifted((1.0 - alive) * 24.0)
            });
            let going = self.leaving.contains_key(&notice.id);
            tiles.push(ui::collapsing(container(tile).padding(Padding::ZERO.bottom(8.0)), if going { alive } else { 1.0 }));
        }
        tiles
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
        lines = lines.push(container(ui::cap(format!("{}  {}", w.day(notice.at, self.now_unix), w.clock(notice.at)))).padding(Padding::ZERO.top(4.0)));
        let mut bottom = row![ui::grow(), ui::quiet(w.t("close"), Some(Message::HideError))].spacing(4).align_y(iced::Center);
        if matches!(notice.link, notices::Link::RenderAgain(_)) {
            bottom = bottom.push(ui::primary(w.t("once-more"), Some(Message::ToastLink(notice.id))));
        }
        let card = ui::sheet(lines.into(), Some(bottom.into()));
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
            notices::Mark::Bad => "!",
            notices::Mark::Plain => "i",
        };
        let tint = match notice.mark { notices::Mark::Bad => ACCENT, notices::Mark::Done => theme::NOTICE_SUCCESS, notices::Mark::Plain => theme::NOTICE_INFO };
        if notice.map_hash.is_empty() || !self.thumbs.contains_key(&notice.map_hash) {
            return container(text(glyph).font(theme::MONO_BOLD).size(20.0).color(ui::faded(tint)))
                .width(side).height(side).center(side)
                .style(ui::box_faded(move |_| container::Style {
                    background: Some(iced::Background::Color(Color { a: 0.08, ..tint })),
                    border: iced::Border { radius: (side / 2.0).into(), ..iced::Border::default() }, ..container::Style::default()
                })).into();
        }
        let picture: Element<'_, Message> = match self.thumbs.get(&notice.map_hash) {
            Some(handle) if !notice.map_hash.is_empty() => ui::clipped(image(handle.clone())
                .content_fit(ContentFit::Cover)
                .width(side)
                .height(side)
                .border_radius(8.0)
                .opacity(ui::fade() * 0.9)),
            _ => container(Space::new().width(side).height(side)).style(ui::box_faded(theme::chip)).into(),
        };
        let badge = container(text(glyph).font(theme::MONO_BOLD).size(11.0).color(ui::faded(INK)))
            .width(BADGE)
            .height(BADGE)
            .center(BADGE)
            .style(ui::box_faded(theme::badge_of(tint)));
        let reach = side + BADGE_OUT;
        stack![
            container(picture).width(reach).height(reach),
            pin(badge).x(side + BADGE_OUT - BADGE).y(side + BADGE_OUT - BADGE),
        ]
        .width(reach)
        .height(reach)
        .into()
    }

    fn stats_tiles(&self) -> Vec<Element<'_, Message>> {
        let w = &self.words;
        let (sent, _) = self.store.delivery_totals();
        let heading = |key: &str| container(ui::mono_small(w.t(key).to_uppercase(), FAINT)).padding(Padding::ZERO.bottom(8.0));
        let delivered = self.farm.as_ref().and_then(|farm| farm.workers.iter().find(|worker| worker.mine)).map_or(self.worker_done as u64, |mine| mine.delivered as u64);
        let worker = column![heading("as-worker"), self.kv(w.t("jobs-done"), delivered.to_string())].spacing(0);
        let device = column![
            heading("on-this-device"),
            self.pair(
                self.big(self.entries().len().to_string(), w.t("replays-in-journal")),
                self.big(self.library.as_ref().map_or(0, |library| library.maps).to_string(), w.t("maps-in-library")),
            ),
            self.pair(self.big(sent.to_string(), w.t("sent-count")), self.big(self.shown_build().to_owned(), w.t("build"))),
        ]
        .spacing(8);
        vec![self.card(worker.into(), false), self.card(device.into(), false)]
    }

    fn sign_in_layer(&self) -> Element<'_, Message> {
        if matches!(self.pairing, Pairing::Idle) {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        }
        let w = &self.words;
        let linking = matches!(self.pairing, Pairing::Linking { .. });
        let (title, how) = if linking { ("link-telegram", "link-telegram-how") } else { ("sign-in", "sign-in-how") };
        let mut left = column![ui::title(w.t(title)), ui::why(w.t(how))].spacing(6).width(Length::Fill);
        match &self.pairing {
            Pairing::Waiting { code, .. } | Pairing::Linking { code, .. } => {
                left = left.push(container(text(code.clone()).font(theme::MONO_BOLD).size(26.0).color(ui::faded(INK))).padding(Padding::ZERO.top(12.0)));
                if !linking {
                    left = left.push(ui::cap(w.t("code-lasts")));
                }
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
        if let (Some(qr), Pairing::Waiting { .. } | Pairing::Linking { .. }) = (&self.qr, &self.pairing) {
            sides = sides.push(ui::qr(qr));
        }
        let waiting = matches!(self.pairing, Pairing::Waiting { .. } | Pairing::Linking { .. });
        let mut bottom = row![ui::primary(w.t("open-telegram"), waiting.then_some(Message::OpenTelegram))].spacing(4).align_y(iced::Center);
        if matches!(self.pairing, Pairing::Waiting { osu: true, .. }) {
            bottom = bottom.push(ui::quiet(w.t("sign-in-osu"), Some(Message::OpenOsu)));
        }
        let bottom = bottom
            .push(ui::quiet(w.t("copy-link"), waiting.then_some(Message::CopyLink)))
            .push(ui::grow())
            .push(ui::quiet(w.t("later-word"), Some(Message::LaterSignIn)));
        let card = ui::sheet(sides.into(), Some(bottom.into()));
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
const TOAST_W: f32 = 340.0;
const TOAST_MAX_H: f32 = 112.0;
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
    fn toast_capacity(&self) -> usize {
        (((self.height - TOAST_TOP - 24.0) / (TOAST_MAX_H + 10.0)).floor() as usize).clamp(1, TOASTS_AT_MOST)
    }

    fn companion_layer(&self) -> Element<'_, Message> {
        let k = self.companion_fade.interpolate(0.0, 1.0, self.now);
        let shown = self.companion_shown.as_ref().filter(|_| k > 0.001);
        let (Some((beatmap, line)), Some(catalog)) = (shown, self.community.as_ref()) else {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        };
        let ground = self.community_ground(catalog, false);
        let card = crate::sheets::companion(&ground, *beatmap, line.clone(), COMPANION_WIDE).map(Message::Community);
        let card = ui::fading(ui::fade() * k, || ui::grown(iced::widget::opaque(card), Point::new(0.0, 1.0), 0.0, 1.0).shifted((k - 1.0) * 24.0));
        container(card)
            .padding(Padding { top: 0.0, right: 0.0, bottom: COMPANION_BELOW, left: crate::sidebar::NARROW + COMPANION_EDGE })
            .align_y(iced::alignment::Vertical::Bottom)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn toast_layer(&self) -> Element<'_, Message> {
        let width = TOAST_W.min((self.width - 32.0).max(240.0));
        let home = (self.width - 40.0 - width).max(16.0);
        let mut cards = column![].width(width);
        for toast in &self.toasts {
            let Some(notice) = self.notices.get(toast.id) else { continue; };
            let k = toast.shown.interpolate(0.0, 1.0, self.now);
            let sensed = ui::fading(ui::fade() * k, || {
                ui::grown(mouse_area(self.notification_card(notice, Some(toast), width))
                    .on_enter(Message::ToastHover(toast.id, true)).on_exit(Message::ToastHover(toast.id, false)),
                    Point::new(1.0, 0.5), 0.0, 1.0).shifted((1.0 - k) * 24.0)
            });
            cards = cards.push(ui::collapsing(container(sensed).padding(Padding::ZERO.bottom(10.0)), k));
        }
        pin(cards).x(home).y(TOAST_TOP).into()
    }

    fn notification_card(&self, notice: &notices::Notice, toast: Option<&Toast>, width: f32) -> Element<'_, Message> {
        let bad = notice.mark == notices::Mark::Bad;
        let close = button(iced::widget::canvas(ui::Cross { colour: MUTED }).width(20.0).height(20.0))
            .padding(0).style(ui::button_faded(theme::notice_action(false)))
            .on_press(if toast.is_some() { Message::ToastClose(notice.id) } else { Message::DismissNotice(notice.id) });
        let title_limit = ((width - 92.0) / 7.0).floor().max(12.0) as usize;
        let title = ui::clipped(container(text(notice_preview(&notice.words, title_limit)).font(theme::SANS_SEMI).size(13.0)
            .wrapping(text::Wrapping::None).color(ui::faded(INK))).width(Length::Fill).height(20.0).center_y(20.0));
        let header = row![title, close].spacing(6).align_y(iced::Center);
        let detail = if notice.detail.is_empty() && bad { notice.note.clone() } else { notice.detail.clone() };
        let mut copy = column![header].spacing(4).width(Length::Fill);
        if !detail.is_empty() {
            let detail_limit = (((width - 70.0) / 6.7) * 3.0).floor().max(36.0) as usize;
            let description = container(text(notice_preview(&detail, detail_limit)).font(theme::SANS).size(12.0)
                .line_height(iced::Pixels(14.0)).wrapping(text::Wrapping::WordOrGlyph)
                .color(ui::faded(MUTED))).width(Length::Fill).max_height(42.0).clip(true);
            copy = copy.push(description);
        }
        let top = row![self.notice_mark(notice, 40.0), copy].spacing(10).align_y(iced::Center);
        let action = |label: String, message, primary| button(text(label).font(theme::SANS_SEMI).size(theme::CAPTION))
            .padding([3, 7]).style(ui::button_faded(theme::notice_action(primary))).on_press(message);
        let mut footer = row![Space::new().width(Length::Fill)].spacing(4).align_y(iced::Center).height(24.0);
        if bad {
            footer = footer.push(action(self.words.t("notice-details"), Message::ShowError(notice.id), false));
        }
        let link = match notice.link {
            notices::Link::OpenVideo(_) | notices::Link::Replay(_) | notices::Link::Received(_) => Some("open"),
            notices::Link::RenderAgain(_) => Some("once-more"),
            notices::Link::Update => Some("update-now"),
            notices::Link::Page(_) => Some("whats-new"),
            notices::Link::None => None,
        };
        if let Some(key) = link { footer = footer.push(action(self.words.t(key), Message::ToastLink(notice.id), true)); }
        let mut content = column![top].spacing(2);
        if bad || link.is_some() { content = content.push(footer); }
        let face = container(content).padding([8, 10]).width(width).max_height(TOAST_MAX_H);
        let card = container(face)
            .width(width).max_height(TOAST_MAX_H).style(ui::box_faded(theme::notification(toast.is_some_and(|t| t.hovered)))).clip(true);
        card.id(iced::widget::Id::new(if toast.is_some() { "toast-card" } else { "notice-card" })).into()
    }
}

fn notice_preview(value: &str, limit: usize) -> String {
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if value.chars().count() <= limit { value } else { format!("{}…", value.chars().take(limit.saturating_sub(1)).collect::<String>()) }
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

const REFRESH_AT_MOST: Duration = Duration::from_secs(20);
const STRIP_MARGIN: f32 = 0.75;
const FLIP: Duration = Duration::from_millis(320);
const MINIMIZED_POLL: Duration = Duration::from_secs(15);
const AUTO_EVERY: Duration = Duration::from_secs(20);
const AUTO_IDLE: Duration = Duration::from_secs(12);
const COVERS_AT_ONCE: usize = 240;

const FRAME_GAP: f32 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Slot {
    Frame(usize),
    Gap(f32),
}

fn slots(widths: &[f32], start: f32, seen: (f32, f32)) -> Vec<Slot> {
    let mut out = Vec::new();
    let mut hidden: Option<(f32, f32)> = None;
    let mut x = start;
    for (index, wide) in widths.iter().copied().enumerate() {
        let gap = if index == 0 { 0.0 } else { FRAME_GAP };
        if x + gap + wide < seen.0 || x > seen.1 {
            let (run, lead) = hidden.unwrap_or((0.0, gap));
            hidden = Some((run + gap + wide, lead));
        } else {
            if let Some((run, lead)) = hidden.take() {
                out.push(Slot::Gap(run - lead));
            }
            out.push(Slot::Frame(index));
        }
        x += gap + wide;
    }
    if let Some((run, lead)) = hidden {
        out.push(Slot::Gap(run - lead));
    }
    out
}

fn read_library(sources: Vec<crate::sources::Source>, done: fn(Library) -> Message) -> Task<Message> {
    ui::streamed(move |push: &mut dyn FnMut(Message) -> bool| {
        let library = library::read_with(&sources, &mut |reading| {
            let _ = push(Message::Reading(reading));
        });
        let _ = push(done(library));
    })
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

pub fn mod_badge<'a, Message: 'a>(acronym: &str) -> Element<'a, Message> {
    let colour = mod_colour(acronym);
    let alpha = ui::fade();
    let badge = container(text(acronym.to_owned()).font(theme::MONO_BOLD).size(10.0).color(ui::faded(theme::ON_ACCENT)))
        .padding([1, 6])
        .style(theme::badge(Color { a: colour.a * alpha, ..colour }));
    ui::hover(badge, ui::Glow::tile(5.0).edge(Color { a: 0.35 * alpha, ..theme::ON_ACCENT }).lift(1.0).scale(1.06))
}

pub fn decoded(path: &Path, max_width: u32, cover: Option<(u32, u32)>) -> Option<image::Handle> {
    let picture = ::image::ImageReader::open(path).ok()?.with_guessed_format().ok()?.decode().ok()?;
    shaped(picture, max_width, cover)
}

pub fn decoded_from(bytes: &[u8], max_width: u32, cover: Option<(u32, u32)>) -> Option<image::Handle> {
    shaped(::image::load_from_memory(bytes).ok()?, max_width, cover)
}

fn shaped(picture: ::image::DynamicImage, max_width: u32, cover: Option<(u32, u32)>) -> Option<image::Handle> {
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
    let rgba = picture.to_rgba8();
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
    use super::{slots, Slot, FRAME_GAP};

    #[test]
    fn minimizing_stops_periodic_work_and_restores_only_the_same_playing_video() {
        let mut main = super::Main::staged(crate::lang::Words::new(crate::lang::Lang::En), crate::settings::Settings::default(), crate::library::Library::default(), None);
        main.window_id = Some(iced::window::Id::unique());
        let player = std::rc::Rc::new(std::cell::RefCell::new(crate::player::Player::still(std::path::Path::new("sample.mp4"), 1_000, 0)));
        player.borrow_mut().toggle();
        main.player = Some(player.clone());
        assert!(!player.borrow().paused);
        main.set_minimized(true);
        assert!(player.borrow().paused);
        assert!(!main.moving());
        assert_eq!(main.subscription().units(), 2);
        let frozen_at = main.now;
        let _ = main.update(super::Message::Tick(frozen_at + std::time::Duration::from_secs(1)));
        assert_eq!(main.now, frozen_at);
        main.set_minimized(false);
        assert!(!player.borrow().paused);
        assert!(main.subscription().units() > 2);
        player.borrow_mut().toggle();
        main.set_minimized(true);
        main.set_minimized(false);
        assert!(player.borrow().paused);
        player.borrow_mut().toggle();
        main.set_minimized(true);
        let replacement = std::rc::Rc::new(std::cell::RefCell::new(crate::player::Player::still(std::path::Path::new("other.mp4"), 1_000, 0)));
        main.player = Some(replacement.clone());
        main.set_minimized(false);
        assert!(replacement.borrow().paused);
    }

    #[test]
    fn compare_opens_with_you_and_them_and_holds_four_at_most() {
        use super::Message as M;
        use crate::community_screen::{Message as C, Section};
        let mut main = super::Main::staged(crate::lang::Words::new(crate::lang::Lang::En), crate::settings::Settings::default(), crate::library::Library::default(), None);
        main.community = Some(chat_catalogue());
        let _ = main.update(M::Community(C::CompareWith(1)));
        assert_eq!(main.community_section, Section::Compare);
        assert_eq!(main.compare, [7, 8], "the dossier's compare did not start with you and them");

        let _ = main.update(M::Community(C::CompareWith(0)));
        assert_eq!(main.compare, [7], "comparing yourself put you in twice");

        main.everyone = serde_json::from_str::<crate::community::wire::Everyone>(r#"{"people": [
            {"id": 901, "player": 91, "name": "a", "pp": 1}, {"id": 902, "player": 92, "name": "b", "pp": 2},
            {"id": 903, "player": 93, "name": "c", "pp": 3}, {"id": 904, "player": 94, "name": "d", "pp": 4}
        ]}"#).unwrap().people;
        main.pool_players();
        for id in [8, 901, 902, 903, 8] {
            let _ = main.update(M::Community(C::CompareAdd(id)));
        }
        assert_eq!(main.compare, [7, 8, 901, 902], "more than four were compared or one came twice");
        let _ = main.update(M::Community(C::CompareSearch("c".into())));
        let _ = main.update(M::Community(C::CompareRemove(901)));
        assert_eq!(main.compare, [7, 8, 902]);
        let _ = main.update(M::Community(C::CompareAdd(903)));
        assert_eq!(main.compare, [7, 8, 902, 903]);
        assert!(main.compare_query.is_empty(), "the search stayed after adding");

        main.compare.clear();
        let _ = main.update(M::Community(C::Section(Section::People)));
        let _ = main.update(M::Community(C::Section(Section::Compare)));
        assert_eq!(main.compare, [7], "an empty comparison did not start with you");
    }

    fn chat_catalogue() -> crate::community::Catalog {
        let said: crate::community::wire::Community = serde_json::from_str(r#"{"chat": -100, "group": "Osu Squad", "people": [
            {"id": 7, "player": 70, "name": "NaumRedlo", "pp": 9870, "you": true},
            {"id": 8, "player": 80, "name": "kotofey", "pp": 12480}
        ]}"#).unwrap();
        crate::community::Catalog::from_wire(said)
    }

    #[test]
    fn every_player_comes_and_goes_with_the_setting_and_the_open_dossier_stays_open() {
        use super::Message as M;
        let mut main = super::Main::staged(crate::lang::Words::new(crate::lang::Lang::En), crate::settings::Settings::default(), crate::library::Library::default(), None);
        main.community = Some(chat_catalogue());
        main.community_person = Some(1);
        let everyone: crate::community::wire::Everyone = serde_json::from_str(r#"{"people": [
            {"id": 7, "player": 70, "name": "NaumRedlo", "pp": 9870},
            {"id": 900, "player": 90, "name": "Mirrorwave", "pp": 11215}
        ]}"#).unwrap();
        let _ = main.update(M::EveryoneArrived(Ok(everyone)));
        assert_eq!(main.community.as_ref().unwrap().people.len(), 2, "every player came in while the setting was off");

        main.settings.people_everyone = true;
        main.welcome_everyone();
        let people = &main.community.as_ref().unwrap().people;
        assert_eq!(people.iter().map(|person| person.name.as_str()).collect::<Vec<_>>(), ["NaumRedlo", "kotofey", "Mirrorwave"]);
        assert!(people[2].outside);
        assert_eq!(main.community_person, Some(1), "the open dossier moved to someone else");

        main.community_person = Some(2);
        main.settings.people_everyone = false;
        main.welcome_everyone();
        assert_eq!(main.community.as_ref().unwrap().people.len(), 2);
        assert_eq!(main.community_person, None, "a dossier of someone no longer listed stayed open");
    }

    #[test]
    fn a_pin_refused_says_when_it_can_move_and_a_pin_moves_the_community() {
        use super::Message as M;
        use crate::community::wire::Pin;
        let mut settings = crate::settings::Settings::default();
        settings.chat_id = Some(-100);
        let mut main = super::Main::staged(crate::lang::Words::new(crate::lang::Lang::En), settings, crate::library::Library::default(), None);
        main.community = Some(chat_catalogue());
        assert_eq!(main.community_chat(), Some(-100));

        let _ = main.update(M::PinRead(Ok(Pin { chat: Some(-200), since: Some(1_790_000_000), free_at: Some(1_792_592_000), error: String::new() })));
        assert_eq!(main.community_chat(), Some(-200), "the community did not follow the pinned chat");

        main.hint = None;
        let _ = main.update(M::Pinned(Ok(Pin { chat: Some(-200), since: Some(1_790_000_000), free_at: Some(1_792_592_000), error: "too soon".into() })));
        let (said, _) = main.hint.clone().expect("nothing was said about the refusal");
        assert!(said.contains(&main.words.day(1_792_592_000, super::unix_now())), "the refusal did not say when: {said}");
        assert_eq!(main.pin.as_ref().and_then(|pin| pin.chat), Some(-200));

        let _ = main.update(M::Pinned(Ok(Pin { chat: Some(-300), since: Some(1_795_000_000), free_at: Some(1_797_592_000), error: String::new() })));
        assert_eq!(main.community_chat(), Some(-300));
        assert_eq!(main.hint.clone().map(|(said, _)| said), Some(main.words.t("pin-done")));
    }

    #[test]
    fn renders_asked_for_while_one_runs_wait_their_turn_and_start_by_themselves() {
        use super::Message as M;
        use crate::render::Step as R;
        let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-rest").unwrap();
        main.ffmpeg = Some(std::path::PathBuf::from("ffmpeg"));
        let drawable: Vec<std::path::PathBuf> = main.entries().iter().filter(|entry| entry.map.is_some()).map(|entry| entry.path.clone()).collect();
        assert!(drawable.len() >= 3, "the staged library needs three replays with maps");
        let press = |main: &mut super::Main, path: &std::path::Path| {
            main.chosen = main.entries().iter().position(|entry| entry.path == path);
            let _ = main.update(M::Render);
        };
        let queue = |main: &super::Main| main.queued.iter().map(|queued| queued.path.clone()).collect::<Vec<_>>();

        press(&mut main, &drawable[0]);
        assert_eq!(main.rendering.as_ref().map(|r| r.path.clone()), Some(drawable[0].clone()));
        assert!(main.queued.is_empty());
        press(&mut main, &drawable[1]);
        press(&mut main, &drawable[1]);
        press(&mut main, &drawable[0]);
        press(&mut main, &drawable[2]);
        assert_eq!(queue(&main), vec![drawable[1].clone(), drawable[2].clone()], "a replay was queued twice or the running one again");
        assert!(main.busy());

        let _ = main.update(M::Unqueue(drawable[2].clone()));
        assert_eq!(queue(&main), vec![drawable[1].clone()]);
        let _ = main.update(M::Rendered(R::Drawing { frames: 10, of: 100, left_seconds: 5.0 }));
        assert_eq!(main.rendering.as_ref().map(|r| r.path.clone()), Some(drawable[0].clone()), "the queue jumped ahead of a running render");

        let _ = main.update(M::Rendered(R::Stopped));
        assert_eq!(main.rendering.as_ref().map(|r| r.path.clone()), Some(drawable[1].clone()), "the next render did not start after a stop");
        assert!(main.queued.is_empty());
        assert!(main.rendering.as_ref().unwrap().reached.is_empty());
        let _ = main.update(M::Rendered(R::Failed("ffmpeg".into())));
        assert!(!main.busy(), "an empty queue kept the app busy");
    }

    #[test]
    fn maps_download_side_by_side_and_each_keeps_its_own_progress_and_stop() {
        use super::Message as M;
        use crate::maps::Step as S;
        use std::sync::atomic::Ordering;
        let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-rest").unwrap();
        let mut hashes: Vec<String> = Vec::new();
        for entry in main.entries() {
            if !hashes.contains(&entry.map_hash) {
                hashes.push(entry.map_hash.clone());
            }
        }
        assert!(hashes.len() >= 4, "the staged library needs four maps");
        let choose = |main: &mut super::Main, hash: &str| {
            main.chosen = main.entries().iter().position(|entry| entry.map_hash == hash);
        };
        for hash in &hashes[..4] {
            choose(&mut main, hash);
            let _ = main.update(M::GetMap);
        }
        assert_eq!(main.fetching.len(), super::FETCHES_AT_ONCE, "a download started past the limit");
        assert!(!main.fetch_room());
        let (first, second) = (main.fetching[0].id, main.fetching[1].id);

        let _ = main.update(M::Fetched(second, S::Downloading { from: "osu.direct", done: 50, total: Some(100) }));
        assert!(main.fetching[0].last().is_none(), "one download's step went to another");
        assert!(matches!(main.fetching[1].last(), Some(S::Downloading { done: 50, .. })));
        let _ = main.update(M::Tick(main.now + std::time::Duration::from_millis(500)));
        assert!(main.fetching[1].shown > main.fetching[0].shown, "the progress is not each download's own");

        let _ = main.update(M::StopFetch(first));
        assert!(main.fetching[0].stop.load(Ordering::SeqCst));
        assert!(!main.fetching[1].stop.load(Ordering::SeqCst), "stopping one download stopped another");
        let _ = main.update(M::Fetched(first, S::Stopped));
        assert!(main.fetch_room(), "a stopped download kept its place");

        let _ = main.update(M::Fetched(first + 100, S::Nowhere));
        assert_eq!(main.fetching.len(), super::FETCHES_AT_ONCE, "a step of no download changed the list");

        choose(&mut main, &hashes[0]);
        let _ = main.update(M::GetMap);
        let again: Vec<_> = main.fetching.iter().filter(|fetching| fetching.hash == hashes[0]).collect();
        assert_eq!(again.len(), 1, "the map is fetched twice");
        assert_ne!(again[0].id, first);
        let _ = main.update(M::Fetched(first, S::Downloading { from: "osu.direct", done: 1, total: None }));
        assert!(main.fetch_of(&hashes[0]).unwrap().last().is_none(), "a late step of the stopped download reached the new one");
    }

    #[test]
    fn notification_hover_preserves_remaining_time_and_ignores_duplicate_enters() {
        use std::time::{Duration, Instant};
        let now = Instant::now();
        let mut toast = super::Toast { id: 1, shown: iced::Animation::new(true), born: now, hovered: false, paused_at: None, stays: false };
        toast.hover(true, now + Duration::from_secs(2));
        toast.hover(true, now + Duration::from_secs(8));
        assert_eq!(toast.age(now + Duration::from_secs(10)), Duration::from_secs(2));
        toast.hover(false, now + Duration::from_secs(12));
        assert_eq!(toast.age(now + Duration::from_secs(15)), Duration::from_secs(5));
        toast.hover(false, now + Duration::from_secs(16));
        assert_eq!(toast.age(now + Duration::from_secs(16)), Duration::from_secs(6));
    }

    #[test]
    fn settled_toasts_do_not_keep_the_window_rendering_at_frame_rate() {
        use std::time::{Duration, Instant};
        let mut main = super::Main::staged(crate::lang::Words::new(crate::lang::Lang::En), crate::settings::Settings::default(), crate::library::Library::default(), None);
        let now = Instant::now() + Duration::from_secs(10);
        let _ = main.update(super::Message::Tick(now));
        assert!(!main.moving(), "the staged screen must be settled before checking toast work");
        main.toasts.push(super::Toast { id: 1, shown: iced::Animation::new(true), born: now, hovered: false, paused_at: None, stays: true });
        assert!(!main.moving(), "a persistent error needs no continuous redraw once it is visible");
        main.toasts[0].stays = false;
        let _ = main.update(super::Message::Tick(now + Duration::from_secs(7)));
        assert!(!main.toasts[0].shown.value());
        assert!(main.moving(), "the dismissal animation still needs frames");
        let _ = main.update(super::Message::Tick(now + Duration::from_secs(8)));
        assert!(main.toasts.is_empty());
        assert!(!main.moving());
    }

    #[test]
    fn dismissing_an_arriving_notification_keeps_the_visible_opacity() {
        let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-menu-feed").unwrap();
        let id = main.notices.notices[0].id;
        let now = std::time::Instant::now();
        main.arrivals.insert(id, iced::Animation::new(false).duration(super::NOTICE_ARRIVE).easing(iced::animation::Easing::EaseOutCubic).go(true, now - std::time::Duration::from_millis(50)));
        let before = main.arrivals[&id].interpolate(0.0_f32, 1.0, now);
        let _ = main.update(super::Message::DismissNotice(id));
        let after = main.leaving[&id].interpolate(0.0_f32, 1.0, std::time::Instant::now());
        assert!((before - after).abs() < 0.02, "dismissal must reverse the current arrival instead of jumping to full opacity: {before} -> {after}");
    }

    #[test]
    fn notification_errors_stay_and_closing_a_popup_keeps_its_history() {
        let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-notifications").unwrap();
        let bad = main.toasts.iter().find(|toast| toast.stays).unwrap().id;
        let _ = main.update(super::Message::Tick(main.now + std::time::Duration::from_secs(20)));
        assert!(main.toasts.iter().find(|toast| toast.id == bad).unwrap().shown.value());
        assert!(main.toasts.iter().filter(|toast| !toast.stays).all(|toast| !toast.shown.value()));
        let _ = main.update(super::Message::ToastClose(bad));
        assert!(!main.toasts.iter().find(|toast| toast.id == bad).unwrap().shown.value());
        assert!(main.notices.get(bad).is_some());
        main.height = 400.0;
        for n in 0..8 {
            main.announce(crate::notices::Mark::Plain, format!("event {n}"), String::new(), String::new(), String::new(), crate::notices::Link::None);
        }
        assert_eq!(main.toasts.iter().filter(|toast| toast.shown.value()).count(), 2);
        assert_eq!(main.notices.notices.len(), 11);
        assert_eq!(super::notice_preview("Карта\nс очень длинным именем", 8), "Карта с…");
    }

    #[test]
    fn repeated_pointer_positions_do_not_wake_the_resting_interface() {
        let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-rest").unwrap();
        main.input_pointer = Some(iced::Point::new(100.0, 100.0));
        main.resting = iced::Animation::new(true);
        let last_input = main.last_input;
        let _ = main.update(super::Message::PointerActivity(iced::Point::new(100.0, 100.0)));
        assert!(main.resting.value());
        assert_eq!(main.last_input, last_input);
        let _ = main.update(super::Message::PointerActivity(iced::Point::new(101.0, 100.0)));
        assert!(!main.resting.value());
    }

    #[test]
    fn the_main_menu_rests_only_after_two_minutes_without_input() {
        use super::{Message, REST_AFTER};
        let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-rest").unwrap();
        let began = std::time::Instant::now();
        main.last_input = began;
        main.now = began;
        main.hover = Some(0);
        let _ = main.update(Message::RestCheck(began + REST_AFTER - std::time::Duration::from_millis(1)));
        assert!(!main.resting.value());
        let _ = main.update(Message::RestCheck(began + REST_AFTER));
        assert!(main.resting.value());
        assert!(main.hover.is_none());
        let _ = main.update(Message::UserInput(Some(Box::new(Message::Typed('x')))));
        assert!(!main.resting.value());
        assert!(main.search.is_empty(), "the first key wakes the interface without triggering a shortcut");
        main.resting = iced::Animation::new(false);
        let _ = main.update(Message::UserInput(Some(Box::new(Message::Typed('x')))));
        assert_eq!(main.search, "x", "keyboard actions work again after waking");
        let touched = main.last_input;
        let _ = main.update(Message::RestCheck(touched + REST_AFTER - std::time::Duration::from_millis(1)));
        assert!(!main.resting.value(), "input resets the deadline");
    }

    #[test]
    fn open_panels_and_active_work_keep_the_interface_visible() {
        use super::{Message, REST_AFTER};
        let states = crate::gallery::main_states(crate::lang::Lang::En);
        for name in ["main-prefs", "main-community-feed", "main-videos", "main-menu-account", "main-player", "main-rendering", "main-fetching"] {
            let (_, base) = states.iter().find(|(state, _)| state == name).unwrap_or_else(|| panic!("missing gallery state {name}"));
            let mut main = base.clone();
            let began = std::time::Instant::now();
            main.last_input = began;
            main.now = began;
            let _ = main.update(Message::RestCheck(began + REST_AFTER * 2));
            assert!(!main.resting.value(), "{name}");
            assert_eq!(main.last_input, began + REST_AFTER * 2, "time in a panel does not consume the next idle interval");
        }
    }

    #[test]
    fn a_cached_profile_keeps_its_content_and_close_control_while_updating() {
        let backdrop = crate::ui::backdrop_handle();
        for lang in crate::lang::Lang::ALL {
            let (_, mut main) = crate::gallery::main_states(lang).into_iter().find(|(name, _)| name == "main-community-person").unwrap();
            main.settings.token = "test-account".into();
            main.settings.chat_id = Some(-42);
            main.community.as_mut().unwrap().staged = false;
            let at = main.community_person.unwrap();
            let person = main.community.as_ref().unwrap().people[at].clone();
            main.sync_dossiers();
            let mut dossier = main.community.as_ref().unwrap().me.clone().unwrap();
            dossier.person = person.clone();
            main.people_dossiers.insert(person.id, dossier);
            main.dossier_cache.request(person.id, std::time::Instant::now(), super::unix_now()).unwrap();
            main.person_fade = iced::Animation::new(true);
            main.person_at -= std::time::Duration::from_secs(2);
            for width in [980.0, 1440.0] {
                main.width = width;
                main.height = 1100.0;
                let mut ui = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(width, main.height), crate::gallery::main_frame(&main, &backdrop));
                assert!(ui.find(main.words.t("dossier-refreshing")).is_ok());
                assert!(ui.find(person.name.as_str()).is_ok());
                assert!(ui.find("✕").is_ok(), "refresh keeps the panel interactive");
                if let Ok(dir) = std::env::var("DOSSIER_DOSSIER_REVIEW") {
                    std::fs::create_dir_all(&dir).unwrap();
                    ui.snapshot(&crate::theme::theme()).unwrap().matches_image(std::path::Path::new(&dir).join(format!("cached-profile-{}-{}", lang.tag(), width as u32))).unwrap();
                }
            }
        }
    }

    #[test]
    fn a_play_witness_saw_is_told_only_by_a_paired_device_and_counted_when_it_arrives() {
        let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-community-person").unwrap();
        let replay = include_bytes!("../tests/fixtures/witness.osr");
        let player = dossier_replay::Replay::heading(replay).unwrap().player;
        let hex: String = replay.iter().map(|byte| format!("{byte:02x}")).collect();
        let kept = crate::witness::Kept { osr: hex, passed: true, watched: Some(false), ..Default::default() };
        main.settings.token.clear();
        assert!(main.tell_task(&kept).is_none(), "an unpaired device told a play");
        main.settings.token = "test-account".into();
        for person in main.community.as_mut().unwrap().people.iter_mut().filter(|person| person.you) {
            person.name = player.clone();
        }
        assert!(main.tell_task(&kept).is_some());
        assert!(main.tell_task(&crate::witness::Kept { watched: Some(true), ..kept.clone() }).is_none());
        let _ = main.update(super::Message::WitnessTold(false));
        assert_eq!((main.witness.told, main.witness.untold), (0, true));
        let _ = main.update(super::Message::WitnessTold(true));
        assert_eq!((main.witness.told, main.witness.untold), (1, false));
        main.witness.take(&crate::witness::Event::Gone);
        assert_eq!((main.witness.told, main.witness.status.clone()), (1, crate::witness::Status::Absent));
    }

    #[test]
    fn the_companion_follows_the_map_under_the_cursor_at_song_select_and_asks_for_it_only_after_a_pause() {
        use crate::witness::{Event, State};
        let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-community-person").unwrap();
        main.settings.token = "test-account".into();
        main.settings.chat_id = Some(-42);
        main.community.as_mut().unwrap().staged = false;
        main.witness_control = Some(std::sync::Arc::new(crate::witness::Control::default()));
        let said = |mode: &str, id: i64| Event::State(State { mode: mode.into(), id, title: "Astral Quantization".into(), ..Default::default() });
        let _ = main.update(super::Message::Witness(Event::Attached { pid: 1, build: String::new(), player: String::new() }));
        assert!(main.companion_at.is_none() && !main.companion_fade.value(), "nothing is shown before song select");
        assert!(main.sittings.current().is_some(), "a game session begins when the client is found");

        let _ = main.update(super::Message::Witness(said("SelectPlay", 7)));
        assert_eq!(main.companion_at.map(|(beatmap, _)| beatmap), Some(7));
        assert!(main.companion_fade.value());
        assert_eq!(main.companion_shown.as_ref().map(|(beatmap, line)| (*beatmap, line.as_str())), Some((7, "Astral Quantization")));

        let _ = main.update(super::Message::CompanionTick);
        assert!(main.companion_asked.is_none() && main.map_boards_waiting.is_empty(), "a map scrolled past is not asked about");

        main.companion_at = Some((7, std::time::Instant::now() - super::COMPANION_DWELL - std::time::Duration::from_millis(1)));
        let _ = main.update(super::Message::CompanionTick);
        assert_eq!(main.companion_asked, Some(7));
        assert!(main.map_boards_waiting.contains(&7), "a map that stayed under the cursor is asked about");

        let _ = main.update(super::Message::Witness(said("SelectPlay", 8)));
        assert_eq!(main.companion_at.map(|(beatmap, _)| beatmap), Some(8), "the next map is the one followed");
        assert!(!main.map_boards_waiting.contains(&8));

        let _ = main.update(super::Message::Witness(said("Play", 8)));
        assert!(main.companion_at.is_none() && !main.companion_fade.value(), "leaving song select puts the card away");

        let _ = main.update(super::Message::Witness(said("SelectPlay", 0)));
        assert!(main.companion_at.is_none(), "a map the server does not know has no board to show");

        let _ = main.update(super::Message::Witness(said("SelectPlay", 9)));
        assert!(main.companion_at.is_some());
        let _ = main.update(super::Message::Prefs(crate::settings_screen::Message::WitnessCompanion(false)));
        assert!(main.companion_at.is_none() && !main.settings.witness_companion, "the switch turns it off");
        let _ = main.update(super::Message::Prefs(crate::settings_screen::Message::WitnessCompanion(true)));
        assert_eq!(main.companion_at.map(|(beatmap, _)| beatmap), Some(9), "and on again, at the same map");

        main.settings.token.clear();
        let _ = main.update(super::Message::Witness(said("SelectPlay", 10)));
        assert!(main.companion_at.is_none(), "an unpaired device has no chat to ask");
    }

    #[test]
    fn local_scores_go_to_the_bot_only_when_asked_for_once_at_a_time_and_from_where_the_last_left_off() {
        let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-community-person").unwrap();
        main.settings.token = "test-account".into();
        main.settings.sources = vec![crate::sources::Source { kind: crate::sources::Kind::Stable, root: std::path::PathBuf::from("/no/such/client"), on: true, ..crate::sources::shared(true) }];
        main.gallery = false;
        let _ = main.history_task(true);
        assert!(!main.history_running, "nothing is told unless the person asked for it");
        let _ = main.update(super::Message::Prefs(crate::settings_screen::Message::WitnessHistory(true)));
        assert!(main.settings.history_share && main.history_running && main.settings.history_sent == 0);
        assert!(main.history_at.is_some());
        let _ = main.history_task(true);
        assert!(main.history_running, "a second sync does not start while one is going");
        let _ = main.update(super::Message::HistoryTold(crate::history::Sent { kept: 40, through: 1_700_000_000, failed: false }));
        assert!(!main.history_running);
        assert_eq!((main.witness.history_told, main.settings.history_sent, main.witness.history_failed), (40, 1_700_000_000, false));
        let _ = main.history_task(false);
        assert!(!main.history_running, "a new sync waits for its turn");
        let _ = main.history_task(true);
        let _ = main.update(super::Message::HistoryTold(crate::history::Sent { kept: 0, through: 0, failed: true }));
        assert_eq!((main.witness.history_failed, main.settings.history_sent), (true, 1_700_000_000), "a failure leaves the mark where it was");
        let _ = main.update(super::Message::Prefs(crate::settings_screen::Message::WitnessHistory(false)));
        main.history_at = None;
        let _ = main.history_task(true);
        assert!(!main.history_running && !main.settings.history_share);
        main.settings.history_share = true;
        main.settings.token.clear();
        let _ = main.history_task(true);
        assert!(!main.history_running, "an unpaired device tells nothing");
    }

    #[test]
    fn a_cached_dossier_stays_visible_during_refresh_and_catalogue_reordering() {
        use crate::community::wire;
        use crate::dossier_cache::Entry;
        let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-community-person").unwrap();
        main.settings.token = "test-account".into();
        main.settings.chat_id = Some(-42);
        main.community.as_mut().unwrap().staged = false;
        let at = main.community.as_ref().unwrap().people.iter().position(|person| !person.you).unwrap();
        let person = main.community.as_ref().unwrap().people[at].clone();
        main.community_person = Some(at);
        main.sync_dossiers();
        let time = super::unix_now();
        let request = main.dossier_cache.request(person.id, std::time::Instant::now(), time).unwrap();
        let dossier = wire::Me {
            person: wire::Person { id: person.id, name: person.name.clone(), ..Default::default() },
            points: 123,
            recent: vec![wire::Recent { play: wire::Play { map: wire::Map { title: "Cached map".into(), ..Default::default() }, ..Default::default() }, ..Default::default() }],
            ..Default::default()
        };
        let entered = main.person_at;
        let fade = main.person_fade.value();
        let _ = main.update(super::Message::PersonDossierCached(request.clone(), Some(Entry { saved_at: time - 120, dossier })));
        assert_eq!(main.people_dossiers[&person.id].points, 123);
        assert!(main.dossier_cache.loading(person.id));
        assert_eq!(main.person_at, entered);
        assert_eq!(main.person_fade.value(), fade);
        let _ = main.update(super::Message::PersonDossier(request.clone(), Err("offline".into())));
        assert_eq!(main.people_dossiers[&person.id].points, 123);
        assert!(!main.dossier_cache.loading(person.id));
        let retry = main.dossier_cache.request(person.id, std::time::Instant::now() + std::time::Duration::from_secs(20), time + 20).unwrap();
        let refreshed = wire::Me {
            points: 124,
            ..main.dossier_cache.get(person.id).unwrap().dossier.clone()
        };
        let _ = main.update(super::Message::PersonDossier(retry, Ok(refreshed)));
        assert_eq!(main.people_dossiers[&person.id].points, 124);
        assert_eq!(main.person_at, entered);
        assert_eq!(main.person_fade.value(), fade);
        let fresh = wire::Community {
            chat: Some(-42),
            people: vec![wire::Person { id: person.id, name: person.name.clone(), ..Default::default() }],
            ..Default::default()
        };
        let _ = main.update(super::Message::CommunityArrived(Ok(fresh)));
        assert_eq!(main.community_person, Some(0), "selection follows identity, not the former row position");
        let shown = &main.people_dossiers[&person.id];
        assert_eq!(shown.points, 124);
        assert!(main.community.as_ref().unwrap().maps[shown.recent[0].map].line.contains("Cached map"));
        assert_eq!(main.person_at, entered, "refresh must not restart the opening animation");
        main.settings.chat_id = Some(-43);
        main.sync_dossiers();
        assert!(main.people_dossiers.is_empty());
        let late = wire::Me { person: wire::Person { id: person.id, ..Default::default() }, points: 999, ..Default::default() };
        let _ = main.update(super::Message::PersonDossier(request, Ok(late)));
        assert!(main.people_dossiers.is_empty(), "an old group's response must not enter the new cache");
    }

    #[test]
    fn selected_community_switches_do_not_restart_panel_animations() {
        use crate::community_screen::{Message as C, PeopleFrom, Section, Standing};
        let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-community-boards").unwrap();
        for standing in [Standing::Adaptive, Standing::General] {
            let _ = main.update(super::Message::Community(C::Standing(standing)));
            let started = main.shift_at;
            let _ = main.update(super::Message::Community(C::Standing(standing)));
            assert_eq!(main.shift_at, started);
        }
        for board in crate::community::Board::ALL {
            let _ = main.update(super::Message::Community(C::Board(board)));
            let started = main.shift_at;
            let _ = main.update(super::Message::Community(C::Board(board)));
            assert_eq!(main.shift_at, started);
        }
        for section in [Section::People, Section::Profile, Section::Feed, Section::Boards, Section::Titles] {
            let _ = main.update(super::Message::Community(C::Section(section)));
            let started = main.section_at;
            let _ = main.update(super::Message::Community(C::Section(section)));
            assert_eq!(main.section_at, started);
        }
        for from in [PeopleFrom::Game, PeopleFrom::Chat] {
            let _ = main.update(super::Message::Community(C::PeopleFrom(from)));
            let started = main.shift_at;
            let _ = main.update(super::Message::Community(C::PeopleFrom(from)));
            assert_eq!(main.shift_at, started);
        }
        for filter in crate::chronicle::Filter::ALL {
            let _ = main.update(super::Message::Community(C::Filter(filter)));
            let started = main.group_at;
            let news = main.news_at;
            let _ = main.update(super::Message::Community(C::Filter(filter)));
            assert_eq!((main.group_at, main.news_at), (started, news));
            assert_eq!(main.feed_filter, filter);
        }
        for source in crate::chronicle::Source::ALL {
            let _ = main.update(super::Message::Community(C::Source(source)));
            let started = main.news_at;
            let group = main.group_at;
            let _ = main.update(super::Message::Community(C::Source(source)));
            assert_eq!((main.news_at, main.group_at), (started, group));
            assert_eq!(main.feed_source, source);
        }
    }

    fn seen(main: &super::Main) -> ::image::RgbaImage {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let shot = crate::gallery::snapshot_main(main, iced::Size::new(1180.0, 760.0)).expect("a frame");
        let stem = std::env::temp_dir().join(format!("dossier-feed-motion-{}-{}", std::process::id(), NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
        let file = crate::gallery::write_snapshot(&shot, &stem).expect("written");
        let pixels = ::image::open(&file).expect("read back").to_rgba8();
        let _ = std::fs::remove_file(file);
        pixels
    }

    fn moved(a: &::image::RgbaImage, b: &::image::RgbaImage) -> (f64, u32) {
        assert_eq!(a.dimensions(), b.dimensions());
        let mut sum = 0u64;
        let (mut top, mut bottom) = (u32::MAX, 0u32);
        for (y, (one, other)) in a.rows().zip(b.rows()).enumerate() {
            let row: u64 = one.zip(other).map(|(p, q)| p.0.iter().zip(q.0.iter()).map(|(x, y)| u64::from(x.abs_diff(*y))).sum::<u64>()).sum();
            if row > 0 {
                top = top.min(y as u32);
                bottom = bottom.max(y as u32);
            }
            sum += row;
        }
        (sum as f64 / (f64::from(a.width()) * f64::from(a.height()) * 4.0), if top == u32::MAX { 0 } else { bottom - top + 1 })
    }

    fn feed_at_rest() -> super::Main {
        let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-community-feed").unwrap();
        main.now_unix = super::unix_now();
        let _ = main.update(super::Message::Tick(std::time::Instant::now()));
        main
    }

    fn after(main: &mut super::Main, wait: std::time::Duration) -> ::image::RgbaImage {
        std::thread::sleep(wait);
        let _ = main.update(super::Message::Tick(std::time::Instant::now()));
        seen(main)
    }

    #[test]
    fn a_play_arriving_in_the_feed_opens_its_row_instead_of_pushing_the_rest_down_at_once() {
        let mut main = feed_at_rest();
        let before = seen(&main);
        let _ = main.update(super::Message::LiveArrive);
        let just = after(&mut main, std::time::Duration::ZERO);
        let settled = after(&mut main, std::time::Duration::from_secs_f32(crate::ui::APPEAR + 0.15));
        let (jump, _) = moved(&before, &just);
        let (whole, _) = moved(&before, &settled);
        assert!(whole > 0.2, "the arrival is on screen: {whole}");
        assert!(jump < whole * 0.25, "the first frame after the arrival moved {jump} of the {whole} it ends up moving");
    }

    #[test]
    fn a_news_picture_has_its_room_before_it_arrives_and_fades_into_it() {
        let mut main = feed_at_rest();
        let url = "test://story-picture".to_owned();
        main.news.stories[0].image = Some(url.clone());
        let bare = seen(&main);
        main.news_asked.insert(url.clone());
        let waiting = seen(&main);
        assert!(moved(&bare, &waiting).0 > 0.2, "a picture that was asked for is given its room at once");

        let picture = iced::widget::image::Handle::from_rgba(192, 108, vec![200u8; 192 * 108 * 4]);
        let _ = main.update(super::Message::NewsPicture(url, Some(picture)));
        let just = after(&mut main, std::time::Duration::ZERO);
        let settled = after(&mut main, std::time::Duration::from_secs_f32(crate::chronicle::PICTURE_FADE + 0.15));
        let (jump, _) = moved(&waiting, &just);
        let (whole, high) = moved(&waiting, &settled);
        assert!(whole > 0.05, "the picture is on screen: {whole}");
        assert!(jump < whole * 0.25, "the first frame after the picture came changed {jump} of {whole}");
        let scale = settled.width() as f32 / 1180.0;
        assert!((high as f32) < 150.0 * scale, "only the picture's own box changed, {high} rows of it; nothing below it moved");
    }

    #[test]
    fn a_news_picture_that_never_comes_gives_its_room_back_gradually() {
        let mut main = feed_at_rest();
        let url = "test://lost-picture".to_owned();
        main.news.stories[0].image = Some(url.clone());
        main.news_asked.insert(url.clone());
        let waiting = seen(&main);
        let _ = main.update(super::Message::NewsPicture(url, None));
        let just = after(&mut main, std::time::Duration::ZERO);
        let settled = after(&mut main, std::time::Duration::from_secs_f32(crate::chronicle::PICTURE_FADE + 0.15));
        let (jump, _) = moved(&waiting, &just);
        let (whole, _) = moved(&waiting, &settled);
        assert!(whole > 0.2, "the room is given back: {whole}");
        assert!(jump < whole * 0.25, "the first frame after the loss moved {jump} of {whole}");
    }

    #[test]
    fn stream_switches_only_restart_the_timeline_transition() {
        use crate::chronicle::Stream;
        use crate::community_screen::Message as CommunityMessage;
        let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-community-feed").unwrap();
        let section = main.section_at;
        let shift = main.shift_at;
        let group = main.group_at;
        let news = main.news_at;
        for stream in [Stream::Group, Stream::News, Stream::All] {
            let previous = main.stream_at;
            let _ = main.update(super::Message::Community(CommunityMessage::Stream(stream)));
            assert!(main.stream_at > previous);
            assert_eq!((main.section_at, main.shift_at, main.group_at, main.news_at), (section, shift, group, news));
            let same = main.stream_at;
            let _ = main.update(super::Message::Community(CommunityMessage::Stream(stream)));
            assert_eq!(main.stream_at, same, "clicking the selected stream must not replay its animation");
        }
    }

    #[test]
    fn a_windowed_strip_is_exactly_as_wide_as_the_whole_one() {
        let widths: Vec<f32> = (0..40).map(|at| if at == 17 { 116.0 } else { 108.0 }).collect();
        let whole = widths.iter().sum::<f32>() + FRAME_GAP * (widths.len() - 1) as f32;
        for (from, to) in [(0.0, 400.0), (900.0, 1600.0), (3000.0, 9000.0), (-500.0, -10.0), (-1.0, 99_999.0), (2000.0, 2001.0)] {
            let laid = slots(&widths, 0.0, (from, to));
            let wide: f32 = laid
                .iter()
                .map(|slot| match slot {
                    Slot::Frame(index) => widths[*index],
                    Slot::Gap(wide) => *wide,
                })
                .sum::<f32>()
                + FRAME_GAP * laid.len().saturating_sub(1) as f32;
            assert!((wide - whole).abs() < 0.01, "window {from}..{to}: {wide} against {whole}");
            for slot in &laid {
                if let Slot::Frame(index) = slot {
                    let x = widths[..*index].iter().sum::<f32>() + FRAME_GAP * *index as f32;
                    assert!(x <= to && x + widths[*index] + FRAME_GAP >= from, "frame {index} is outside {from}..{to}");
                }
            }
        }
    }

    #[test]
    fn a_saved_video_uses_encoded_metadata_instead_of_replay_time_or_current_settings() {
        let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().next().unwrap();
        let replay = main.entries()[0].path.clone();
        main.lengths.insert(replay.clone(), 300_000);
        main.settings.render_fps = 30;
        main.settings.render_height = 720;
        main.rendering = Some(super::Rendering { path: replay, reached: vec![], out: None });
        let media = crate::videos::Probe { length_ms: 202_500, width: 1920, height: 1080, fps: 120.0 };
        let _ = main.update(super::Message::Rendered(crate::render::Step::Saved("finished.mp4".into(), media)));
        let saved = &main.store.videos[0];
        assert_eq!((saved.length_ms, saved.width, saved.height, saved.fps), (202_500, 1920, 1080, 120.0));
    }

    #[test]
    fn a_borrowed_replay_is_drawn_with_the_look_it_came_with_and_the_video_keeps_it() {
        let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().next().unwrap();
        main.ffmpeg = Some("ffmpeg".into());
        main.settings.render_fps = 60;
        let entry = main.entries().iter().find(|entry| entry.map.is_some()).cloned().unwrap();
        let own = main.render_ask(&entry).unwrap();
        let look = crate::inbox::Look { width: 1280, height: 720, fps: 30, music: 0.4, hitsounds: 0.9, play: Some(crate::render::Play { hud: false, ..crate::render::Play::default() }) };
        main.sharing.looks.insert(entry.path.clone(), look.clone());
        let borrowed = main.render_ask(&entry).unwrap();
        assert_eq!((borrowed.size, borrowed.fps, borrowed.play.hud), ((1280, 720), 30, false));
        assert_eq!((borrowed.skin, borrowed.crf, borrowed.out), (own.skin, own.crf, own.out));
        main.sharing.render_look = Some(look.clone());
        main.rendering = Some(super::Rendering { path: entry.path.clone(), reached: vec![], out: None });
        let media = crate::videos::Probe { length_ms: 1000, width: 1280, height: 720, fps: 30.0 };
        let _ = main.update(super::Message::Rendered(crate::render::Step::Saved("borrowed.mp4".into(), media)));
        assert_eq!(main.store.videos[0].look, Some(look.clone()));
        let meta = main.send_meta(&main.store.videos[0], None);
        assert_eq!(meta["settings"]["fps"], 30);
        assert_eq!(meta["settings"]["play"]["hud"], false);
        assert_eq!(meta["player"], entry.player.as_str());
        assert!(meta["chat"].is_null());
    }

    #[test]
    fn drawing_a_received_replay_brings_it_into_the_journal_and_picks_it() {
        let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().find(|(name, _)| name == "main-videos-received-open").unwrap();
        let library = main.library.clone().unwrap();
        let at = library.entries.iter().position(|entry| entry.map.is_some()).unwrap();
        let path = library.entries[at].path.clone();
        main.sharing.videos[0].settings = Some(crate::inbox::Look { width: 1280, height: 720, fps: 30, ..crate::inbox::Look::default() });
        main.sharing.drawing = Some(3);
        let _ = main.update(super::Message::Sharing(super::sharing::Message::Drawn(3, Ok((path.clone(), Box::new(library))))));
        assert_eq!(main.overlay, super::Overlay::None);
        assert_eq!(main.sharing.open, None);
        assert_eq!(main.sharing.drawing, None);
        assert_eq!(main.chosen_entry().map(|entry| entry.path.clone()), Some(path.clone()));
        assert_eq!(main.sharing.looks.get(&path).map(|look| look.fps), Some(30));
    }

    #[test]
    fn replays_brought_from_other_players_do_not_say_a_new_play_happened() {
        let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().next().unwrap();
        let mut library = main.library.clone().unwrap();
        let mut brought = library.entries[0].clone();
        brought.path = crate::sources::shared_root().join(format!("kotofey ({}).osr", "f".repeat(32)));
        brought.replay_hash = "f".repeat(32);
        library.entries.insert(0, brought.clone());
        let before = main.notices.notices.len();
        let _ = main.update(super::Message::Refreshed(library.clone()));
        assert_eq!(main.notices.notices.len(), before);
        assert!(main.entries().iter().any(|entry| entry.replay_hash == brought.replay_hash));
        assert_eq!(main.shown(&brought).from, "another player's replay", "a replay from the server is not told apart in the journal");
        assert_eq!(main.shown(&library.entries[1]).from, "");
        let mut own = brought;
        own.path = "/replays/own.osr".into();
        own.replay_hash = "e".repeat(32);
        library.entries.insert(0, own);
        let _ = main.update(super::Message::Refreshed(library));
        assert_eq!(main.notices.notices.len(), before + 1);
    }

    #[test]
    fn leaving_the_previous_frame_does_not_clear_the_new_hover() {
        let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().next().unwrap();
        let bounds = iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(108.0, 70.0));
        let _ = main.update(super::Message::Over(2, bounds));
        let _ = main.update(super::Message::Over(1, bounds));
        let since = main.hover_since;
        let _ = main.update(super::Message::HoverLeft(2));
        assert_eq!(main.hover, Some(1));
        let _ = main.update(super::Message::Over(1, bounds));
        assert_eq!(main.hover_since, since, "layout updates must not restart the hover animation");
        let _ = main.update(super::Message::HoverLeft(1));
        assert_eq!(main.hover, None);
    }

    #[test]
    fn bubble_anchor_ignores_subpixel_noise_and_follows_real_layout_moves() {
        let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().next().unwrap();
        let bounds = iced::Rectangle::new(iced::Point::new(100.0, 650.0), iced::Size::new(108.0, 70.0));
        let _ = main.update(super::Message::Over(1, bounds));
        let since = main.hover_since;
        let noisy = iced::Rectangle { x: bounds.x + 0.1, y: bounds.y - 0.1, ..bounds };
        let _ = main.update(super::Message::Over(1, noisy));
        assert_eq!(main.hover_bounds, Some(bounds));
        let scrolled = iced::Rectangle { x: bounds.x - 24.0, ..bounds };
        let _ = main.update(super::Message::Over(1, scrolled));
        assert_eq!(main.hover_bounds, Some(scrolled));
        assert_eq!(main.hover_since, since);
    }

    #[test]
    fn operations_menu_can_open_in_every_catalogue() {
        let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().next().unwrap();
        for overlay in [super::Overlay::None, super::Overlay::Videos, super::Overlay::Community, super::Overlay::Settings] {
            main.menu = None;
            let _ = main.update(super::Message::Show(overlay));
            let _ = main.update(super::Message::Circle);
            assert_eq!(main.menu, Some(super::Tab::Account));
        }
    }

    #[test]
    fn the_menu_names_the_chat_the_videos_go_to() {
        let (_, mut main) = crate::gallery::main_states(crate::lang::Lang::En).into_iter().next().expect("a staged screen");
        main.account = Some(crate::bot::Me { telegram_id: 7, name: "Naum Redlo".into(), username: "naumredlo".into(), avatar: false, telegram: true, player: None });
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
