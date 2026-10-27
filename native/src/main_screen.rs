mod notifications;
mod account;
mod billing;
mod video_control;

mod video_view;
const VIDEO_THUMB: (u32, u32) = (96, 54);

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use iced::animation::Easing;
use iced::widget::{
    button, column, container, image, mouse_area, pin, row, scrollable, stack, text, Space,
};
use iced::{window, Animation, Color, ContentFit, Element, Length, Padding, Point, Subscription, Task};

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
#[path = "select.rs"]
pub(crate) mod select;
#[path = "reel.rs"]
pub mod reel;

pub const ENTER: Duration = Duration::from_millis(1200);
pub const ARRIVE: Duration = Duration::from_millis(450);
pub const SWAP: Duration = Duration::from_millis(450);
pub const LIFT: Duration = Duration::from_millis(200);
const LIVE_FIRST: usize = 6;
const FETCHES_AT_ONCE: usize = 3;
const LIVE_EVERY: Duration = Duration::from_secs(6);
const PRESENCE_EVERY: Duration = Duration::from_secs(5);
const PANEL_SHOW: Duration = Duration::from_millis(420);
const FOLD: Duration = Duration::from_millis(280);
const NOTICE_FADE: Duration = Duration::from_millis(200);
const COMMUNITY_EVERY: Duration = Duration::from_secs(30);
const BOARD_FRESH: Duration = Duration::from_secs(60);
const HISTORY_EVERY: Duration = Duration::from_secs(600);
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
const THUMB: (u32, u32) = (176, 100);
const POOL_COVER: (u32, u32) = (768, 432);
const SCENE_WIDTH: u32 = 960;
const REST_AFTER: Duration = Duration::from_secs(120);
const REST_FADE: Duration = Duration::from_millis(800);
const REST_BRAND_SCALE: f32 = reel::LOGO;

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
    Reel(u64, live::Frame),
    Community(crate::community_screen::Message),
    NewsBuilds(Result<Vec<crate::news::Build>, String>),
    NewsStories(Result<Vec<crate::news::Story>, String>),
    NewsThreads(Result<Vec<crate::news::Thread>, String>),
    NewsPosts(String, Result<Vec<crate::news::Post>, String>),
    NewsPicture(String, Option<image::Handle>),
    NewsFrost(String, crate::community_screen::Frost),
    LiveArrive,
    CommunityArrived(Result<crate::community::wire::Community, crate::bot::Refused>),
    Paste,
    PoolClipboard(String),
    Undo,
    Redo,
    PoolCommands,
    PoolHot(crate::pools_screen::Hot),
    PoolHeld(bool),
    MapBoard(u64, bool, Result<crate::community::wire::MapBoard, String>),
    EveryoneArrived(Result<crate::community::wire::Everyone, String>),
    WornArrived(std::collections::HashMap<i64, crate::bot::Worn>),
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
    Billing(crate::billing::Message),
    BillingReply(u64, crate::billing::Message),
    RetryPictures,
    FacesArrived,
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
    PresenceTick,
    PresenceAnswered(Result<Option<u32>, Refused>),
    EveryoneAgain,
    OpenTelegram,
    CopyLink,
    Rescaled(f32),
    LaterSignIn,
    SignOut,
    Known(Result<bot::Me, Refused>),
    Avatar(Option<image::Handle>),
    SendVideo,
    Sending(u64),
    Sent(Result<bot::Sent, String>),
    Sharing(sharing::Message),
    Witness(crate::witness::Event),
    WitnessDelivered(crate::witness_delivery::Delivery, Result<(), Refused>),
    WitnessDeliveryTick,
    HistoryTold(crate::history::Sent),
    ToastHover(u64, bool),
    ToastClose(u64),
    OpenVideo(usize),
    VideoReady(u64, PathBuf, Option<videos::Probe>, player::Prepared),
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
    SortBy(crate::carousel::Sort),
    GroupBy(crate::carousel::Group),
    CarouselDrag(f32),
    CarouselDrop(f32),
    StarsAt(f32, bool),
    StarsDone,
    FilterToggle(select::Toggle),
    FiltersReset,
    SearchLine(bool),
    FiltersPanel(bool),
    Jump(i32),
    Wheel(f32),
    Banner(String, Option<image::Handle>),
    Rated(String, Option<crate::rating::Rating>),
    RatingsDone,
    ShareMenu(bool),
    ShareVideo(PathBuf),
    SendToPlayer(PathBuf),
    Render,
    StopRender,
    Unqueue(PathBuf),
    Rendered(Step),
    OpenOut,
    ShowOut,
    GetMap,
    AutoFetch,
    FetchMap(String),
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
    Head,
    Bell,
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
    pub auto_fetched: std::collections::HashSet<String>,
    pub missed: maps::Missed,
    pub online: Option<u32>,
    everyone_tries: u8,
    missed_read: bool,
    pub scene_before: Option<Option<image::Handle>>,
    pub lengths: HashMap<PathBuf, i64>,
    pub combos: HashMap<PathBuf, Option<u32>>,
    pub store: videos::Store,
    pub player: Option<std::rc::Rc<std::cell::RefCell<player::Player>>>,
    minimized: bool,
    presence_pending: bool,
    hidden: bool,
    window_id: Option<window::Id>,
    resume_player: Option<std::rc::Weak<std::cell::RefCell<player::Player>>>,
    pub open_video: Option<usize>,
    video_request: Option<PathBuf>,
    video_serial: u64,
    pub asking_delete: bool,
    pub skin_room: bool,
    pub skin_delete: Option<PathBuf>,
    pub skin_deleting: bool,
    pub witness_asking: bool,
    pub ratings: crate::rating::Ratings,
    ratings_loaded: bool,
    ratings_running: bool,
    ratings_fresh: usize,
    pub sort: crate::carousel::Sort,
    pub group: crate::carousel::Group,
    pub filter: crate::carousel::Filter,
    hold: Option<select::Hold>,
    scroll: Option<f32>,
    scroll_speed: f32,
    unfold_at: Option<Instant>,
    unfold_from: f32,
    star_drag: bool,
    pub(crate) filters_open: bool,
    pub(crate) search_open: bool,
    pub(crate) search_fade: Animation<bool>,
    pub(crate) filters_fade: Animation<bool>,
    folding: Vec<select::Fold>,
    spring_at: Option<Instant>,
    pub(crate) focus: Option<select::Focus>,
    pub(crate) reel: reel::Reel,
    live_rested: bool,
    laid_cache: std::cell::RefCell<Option<(select::LaidKey, std::rc::Rc<crate::carousel::Laid>)>>,
    glow_epoch: Instant,
    share_open: bool,
    low_held: bool,
    free: Option<f32>,
    banners: HashMap<String, image::Handle>,
    banners_asked: std::collections::HashSet<String>,
    banners_at: Instant,
    banners_waiting: bool,
    cover_frames: std::cell::RefCell<HashMap<String, crate::film::Frame>>,
    pub room_fade: Animation<bool>,
    pub skin_scenes: HashMap<PathBuf, image::Handle>,
    pub cinema: Animation<bool>,
    pub resting: Animation<bool>,
    rest_waiting: bool,
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
    pub error_fade: Animation<bool>,
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
    pub worn: std::collections::HashMap<i64, crate::bot::Worn>,
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
    pub billing: crate::billing::State,
    billing_epoch: u64,
    billing_status_pending: bool,
    billing_plans_pending: bool,
    pub bell_flash: Animation<bool>,
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
    pub live_before: Option<crate::film::Frame>,
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
    pub told: Animation<bool>,
    pub lifts: HashMap<usize, Animation<bool>>,
    pub width: f32,
    pub height: f32,
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
    news_picture_failures: HashMap<String, Instant>,
    pub news_loading: std::collections::HashSet<String>,
    pub news_failed: std::collections::HashSet<String>,
    pub channel_draft: String,
    pub live_shown: usize,
    feed_arrivals: crate::chronicle::Arrivals,
    feed_offset: f32,
    pools_marks: crate::pools_screen::Marks,
    feed_leaving: crate::chronicle::Leaving,
    pub link_shown: Option<crate::bot::Link>,
    pub feed_held: std::collections::HashSet<String>,
    feed_pictures: crate::chronicle::Pictures,
    thumb_pictures: crate::chronicle::Pictures,
    pub community_reading: Option<crate::community_screen::Reading>,
    pub map_boards: HashMap<u64, crate::community::wire::MapBoard>,
    pub map_boards_waiting: std::collections::HashSet<u64>,
    pub map_boards_failed: std::collections::HashSet<u64>,
    map_boards_fresh: HashMap<u64, Instant>,
    pub score_pick: Option<i64>,
    pub read_fade: Animation<bool>,
    pub read_over_person: bool,
    pub witness: crate::witness::Seen,
    pub witness_control: Option<std::sync::Arc<crate::witness::Control>>,
    pub sittings: crate::witness::Sittings,
    pub witness_delivery: crate::witness_delivery::Queue,
    pub history_running: bool,
    pub history_at: Option<Instant>,
    pub people_from: crate::community_screen::PeopleFrom,
    pub people_query: String,
    pub people_filter: crate::community_screen::PeopleFilter,
    pub title_scope: crate::community_screen::TitleScope,
    pub people_order: crate::community_screen::PeopleOrder,
    pub people_orders_open: bool,
    pub community_standing: crate::community_screen::Standing,
    pub community_card: Option<crate::community::wire::Card>,
    pub shown_card: Option<crate::community::wire::Card>,
    pub flags: HashMap<String, iced::widget::svg::Handle>,
    flags_asked: std::collections::HashSet<String>,
    card_asked: Option<Instant>,
    pub feed_filter: crate::chronicle::Filter,
    pub feed_query: String,
    pub feed_open: std::collections::HashSet<String>,
    feed_fold_at: HashMap<String, Instant>,
    community_tap: Option<iced::Rectangle>,
    panel_from: Option<iced::Rectangle>,
    person_from: Option<iced::Rectangle>,
    pub(crate) section_at: Instant,
    pub(crate) shift_at: Instant,
    pub(crate) stream_at: Instant,
    pub(crate) person_at: Instant,
    pub dossier_metric: crate::dossier::Metric,
    pub dossier_span: u32,
    pub grade_hover: Option<usize>,
    clips_loading: std::collections::HashSet<String>,
    pub clip: Option<Clip>,
    pub osu_card: Option<crate::community::wire::Card>,
    osu_asked: Option<Instant>,
    pub community_fetch: crate::community_screen::Fetch,
    community_asked: Option<Instant>,
    pub pools: crate::pools_screen::State,
    pub duels: Box<crate::duels_screen::State>,
    pub desk: Box<crate::desk_screen::State>,
    pub duel_link: Option<std::sync::Arc<crate::duels::Link>>,
    pub duel_preview: Option<std::sync::Arc<crate::preview::Preview>>,
    pub duel_leaving: bool,
    pub duel_left: bool,
    pool_covers_asked: std::collections::HashSet<String>,
    refresh_asked: bool,
    refresh_answered: Option<Instant>,
    notice_fade: Animation<bool>,
    notice_kept: Option<crate::community_screen::Notice>,
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
        let pool_guide = settings.pool_guide;
        let billing_email = settings.billing_email.clone();
        let billing_currency = if words.lang() == crate::lang::Lang::Ru { "RUB" } else { "USD" };
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
            auto_fetched: std::collections::HashSet::new(),
            missed: maps::Missed::default(),
            online: None,
            everyone_tries: 0,
            missed_read: false,
            scene_before: None,
            lengths: HashMap::new(),
            combos: HashMap::new(),
            store: videos::Store::load(),
            player: None,
            minimized: false,
            presence_pending: false,
            hidden: false,
            window_id: None,
            resume_player: None,
            open_video: None,
            video_request: None,
            video_serial: 0,
            asking_delete: false,
            skin_room: false,
            skin_delete: None,
            skin_deleting: false,
            witness_asking: false,
            ratings: crate::rating::Ratings::default(),
            ratings_loaded: false,
            ratings_running: false,
            ratings_fresh: 0,
            sort: crate::carousel::Sort::default(),
            group: crate::carousel::Group::default(),
            filter: crate::carousel::Filter::default(),
            hold: None,
            scroll: None,
            scroll_speed: 0.0,
            unfold_at: None,
            unfold_from: 0.0,
            star_drag: false,
            filters_open: false,
            search_open: false,
            search_fade: Animation::new(false).duration(Duration::from_millis(150)).easing(Easing::EaseOutCubic),
            filters_fade: Animation::new(false).duration(Duration::from_millis(150)).easing(Easing::EaseOutCubic),
            folding: Vec::new(),
            spring_at: None,
            focus: None,
            reel: reel::Reel::default(),
            live_rested: false,
            laid_cache: std::cell::RefCell::new(None),
            glow_epoch: Instant::now(),
            share_open: false,
            low_held: true,
            free: None,
            banners: HashMap::new(),
            banners_asked: std::collections::HashSet::new(),
            banners_at: Instant::now(),
            banners_waiting: true,
            cover_frames: std::cell::RefCell::new(HashMap::new()),
            room_fade: Animation::new(false).duration(CINEMA).easing(Easing::EaseOutCubic),
            skin_scenes: HashMap::new(),
            cinema: Animation::new(false).duration(CINEMA).easing(Easing::EaseOutCubic),
            resting: Animation::new(false).duration(REST_FADE).easing(Easing::EaseInOutCubic),
            rest_waiting: false,
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
            error_fade: Animation::new(false).duration(crate::billing::FADE).easing(Easing::EaseOutCubic),
            arrivals: HashMap::new(),
            scenes_due: false,
            leaving: HashMap::new(),
            overlay_fade: Animation::new(false).duration(OVERLAY_FADE).easing(Easing::EaseOutCubic),
            ground_fade: Animation::new(false).duration(GROUND_UP).easing(Easing::EaseOutCubic),
            overlay_drawn: Overlay::None,
            side: Side::App,
            renaming: None,
            everyone: Vec::new(),
            worn: std::collections::HashMap::new(),
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
            billing: crate::billing::State::new(billing_email, billing_currency),
            billing_epoch: 0,
            billing_status_pending: false,
            billing_plans_pending: false,
            bell_flash: Animation::new(false).duration(BELL_FLASH).easing(Easing::EaseOutCubic),
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
            told: Animation::new(true).duration(SWAP).easing(Easing::EaseOutCubic),
            lifts: HashMap::new(),
            width: crate::WINDOW.width,
            height: crate::WINDOW.height,
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
            news_picture_failures: HashMap::new(),
            news_loading: std::collections::HashSet::new(),
            news_failed: std::collections::HashSet::new(),
            channel_draft: String::new(),
            live_shown: LIVE_FIRST,
            feed_arrivals: crate::chronicle::Arrivals::default(),
            feed_offset: 0.0,
            pools_marks: crate::pools_screen::Marks::default(),
            feed_leaving: crate::chronicle::Leaving::default(),
            link_shown: None,
            feed_held: std::collections::HashSet::new(),
            feed_pictures: crate::chronicle::Pictures::default(),
            thumb_pictures: crate::chronicle::Pictures::default(),
            community_reading: None,
            map_boards: HashMap::new(),
            map_boards_waiting: std::collections::HashSet::new(),
            map_boards_failed: std::collections::HashSet::new(),
            map_boards_fresh: HashMap::new(),
            score_pick: None,
            read_fade: Animation::new(false),
            read_over_person: false,
            witness: crate::witness::Seen::default(),
            witness_control: None,
            sittings: crate::witness::Sittings::default(),
            witness_delivery: crate::witness_delivery::Queue::own(),
            history_running: false,
            history_at: None,
            people_from: crate::community_screen::PeopleFrom::Chat,
            people_query: String::new(),
            people_filter: crate::community_screen::PeopleFilter::All,
            title_scope: crate::community_screen::TitleScope::Chat,
            people_order: crate::community_screen::PeopleOrder::Pp,
            people_orders_open: false,
            community_standing: crate::community_screen::Standing::General,
            community_card: None,
            shown_card: None,
            flags: HashMap::new(),
            flags_asked: std::collections::HashSet::new(),
            card_asked: None,
            feed_filter: crate::chronicle::Filter::All,
            feed_query: String::new(),
            feed_open: std::collections::HashSet::new(),
            feed_fold_at: HashMap::new(),
            community_tap: None,
            panel_from: None,
            person_from: None,
            section_at: Instant::now() - Duration::from_secs(3600),
            shift_at: Instant::now() - Duration::from_secs(3600),
            stream_at: Instant::now() - Duration::from_secs(3600),
            person_at: Instant::now() - Duration::from_secs(3600),
            dossier_metric: crate::dossier::Metric::Rank,
            dossier_span: 90,
            grade_hover: None,
            clips_loading: std::collections::HashSet::new(),
            clip: None,
            osu_card: None,
            osu_asked: None,
            community_fetch: crate::community_screen::Fetch::Staged,
            community_asked: None,
            pools: crate::pools_screen::State::new(crate::pools::pools_dir()).guided(pool_guide),
            duels: Box::default(),
            desk: Box::default(),
            duel_link: None,
            duel_preview: None,
            duel_leaving: false,
            duel_left: false,
            pool_covers_asked: std::collections::HashSet::new(),
            refresh_asked: false,
            refresh_answered: None,
            notice_fade: Animation::new(false).duration(NOTICE_FADE).easing(Easing::EaseOutCubic),
            notice_kept: None,
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
        let duels = if made.settings.token.is_empty() { Task::none() } else { Task::done(Message::Community(crate::community_screen::Message::Duels(crate::duels_screen::Message::Again))) };
        (made, Task::batch([read_library(sources, Message::Loaded), adopt, who, warm, duels]))
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
        let witness = Task::batch([self.witness_task(), self.delivery_task()]);
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

    fn witness_overlay(&self) -> Option<crate::witness::Overlay> {
        self.settings.witness_overlay.then(|| crate::witness::Overlay { english: self.settings.lang == crate::lang::Lang::En, keeps: self.settings.witness_keep })
    }

    fn witness_shown(&self) {
        if let Some(control) = &self.witness_control {
            control.show(self.witness_overlay());
        }
    }

    pub fn witness_task(&mut self) -> Task<Message> {
        if self.gallery || self.witness_control.is_some() || !self.settings.witness_on {
            return Task::none();
        }
        if self.settings.witness_keep && !self.settings.sources.iter().any(|source| source.is_witnessed()) {
            self.witness_source();
            let _ = self.settings.save();
        }
        let control = std::sync::Arc::new(crate::witness::Control::default());
        control.show(self.witness_overlay());
        self.witness_control = Some(control.clone());
        let untold = self.witness.untold;
        self.witness = crate::witness::Seen { status: crate::witness::Status::Absent, ..crate::witness::Seen::default() };
        self.witness.pending = self.witness_delivery.pending() as u32;
        self.witness.untold = untold || self.witness_delivery.broken();
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
        let Some(player) = self.community.as_ref().and_then(|catalog| catalog.people.iter().find(|person| person.you)).map(|you| you.name.clone()).filter(|name| !name.trim().is_empty()) else {
            return Task::none();
        };
        self.history_running = true;
        self.history_at = Some(now);
        let (server, token, name, after) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone(), self.settings.history_sent);
        ui::in_thread(move || {
            let digest = crate::history::gather(&root, after, &player);
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
        self.queue_delivery(crate::witness_delivery::Request::Session(sitting))
    }

    fn queue_delivery(&mut self, request: crate::witness_delivery::Request) -> Task<Message> {
        let owner = crate::witness_delivery::owner(&self.settings.server, &self.settings.token);
        if self.witness_delivery.enqueue(crate::witness_delivery::Delivery { owner, request }).is_err() {
            self.witness.untold = true;
            return Task::none();
        }
        self.witness.pending = self.witness_delivery.pending() as u32;
        self.delivery_task()
    }

    fn delivery_task(&mut self) -> Task<Message> {
        if self.gallery || self.settings.token.is_empty() {
            return Task::none();
        }
        let owner = crate::witness_delivery::owner(&self.settings.server, &self.settings.token);
        let Some(delivery) = self.witness_delivery.ready(&owner, Instant::now()) else {
            return Task::none();
        };
        let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        ui::in_thread(move || {
            let result = match &delivery.request {
                crate::witness_delivery::Request::Play(play) => crate::bot::witnessed(&server, &token, &name, play),
                crate::witness_delivery::Request::Session(sitting) => crate::bot::witness_session(&server, &token, &name, sitting),
                crate::witness_delivery::Request::Deferred { .. } => Err(Refused::Said("400".into())),
            };
            Message::WitnessDelivered(delivery, result)
        })
    }

    fn resolve_witness(&mut self) -> Task<Message> {
        if self.settings.token.is_empty() {
            return Task::none();
        }
        let owner = crate::witness_delivery::owner(&self.settings.server, &self.settings.token);
        if !self.witness_delivery.has_deferred(&owner) {
            return self.delivery_task();
        }
        let Some(player) = self.community.as_ref().and_then(|catalog| catalog.people.iter().find(|person| person.you)).map(|you| you.name.clone()).filter(|name| !name.is_empty()) else {
            return self.community_task(false);
        };
        if self.witness_delivery.resolve(&owner, &player).is_err() {
            self.witness.untold = true;
            return Task::none();
        }
        self.witness.pending = self.witness_delivery.pending() as u32;
        self.delivery_task()
    }

    fn tell_task(&mut self, kept: &crate::witness::Kept) -> Option<Task<Message>> {
        if self.settings.token.is_empty() {
            return None;
        }
        let request = match self.community.as_ref().and_then(|catalog| catalog.people.iter().find(|person| person.you)) {
            Some(you) => crate::witness_delivery::Request::Play(crate::witness::told(kept, &you.name, unix_now())?),
            None if kept.watched == Some(false) => crate::witness_delivery::Request::Deferred { kept: kept.clone(), ended: unix_now() },
            None => return None,
        };
        Some(self.queue_delivery(request))
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
        crate::refit_window(ui::refit(iced::Size::new(self.width, self.height), before, after, crate::WINDOW).map(|size| ui::viewport_at(size, after, 1.0)), crate::minimum_window(self.settings.ui_scale))
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
                if let Some(control) = &self.witness_control {
                    control.stop();
                }
                iced::exit()
            }
            Err(why) => {
                self.update = UpdateState::Failed { release: Some(release), why };
                self.update_wanted = false;
                Task::none()
            }
        }
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
        if self.resting.value() {
            return self.resting.is_animating(self.now) || self.reel.fading(self.now)
                || self.live.as_ref().is_some_and(|live| !live.control.settled());
        }
        self.rendering.as_ref().is_some_and(|r| !r.is_over())
            || self.fetch_running()
            || matches!(self.looking, Some(scan::Step::Looking { .. }))
            || self.live.as_ref().is_some_and(|l| l.fade.is_animating(self.now) || !(l.control.paused() && l.control.settled()))
            || self.trail.as_ref().is_some_and(|t| t.paused_at.is_none())
            || self.progress_target().is_some_and(|t| (t - self.progress_shown).abs() > 0.001)
            || self.enter.is_animating(self.now)
            || self.arrive.is_animating(self.now)
            || self.swap.is_animating(self.now)
            || self.told.is_animating(self.now)
            || self.lifts.values().any(|l| l.is_animating(self.now))
            || self.player.as_ref().is_some_and(|p| !p.borrow().paused)
            || self.cinema.is_animating(self.now)
            || self.resting.is_animating(self.now)
            || self.reel.fading(self.now)
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
            || self.search_fade.value() != self.search_shown()
            || self.filters_fade.value() != self.filters_open
            || self.search_fade.is_animating(self.now)
            || self.filters_fade.is_animating(self.now)
            || self.marks.values().any(|m| m.is_animating(self.now))
            || self.slides_settling()
            || (self.overlay == Overlay::Community && self.feed_arrivals.animating(self.now))
            || (self.overlay == Overlay::Community && self.feed_pictures.animating(self.now))
            || (self.overlay == Overlay::Community && self.feed_leaving.animating(self.now))
            || (self.overlay == Overlay::Community && self.community_section == crate::community_screen::Section::Pools && self.pools_marks.animating(self.now))
            || (self.overlay == Overlay::Community && (self.notice_fade.is_animating(self.now) || self.refresh_answered.is_some_and(|at| self.now.saturating_duration_since(at).as_secs_f32() < crate::community_screen::DONE_HOLD + 0.1)))
            || self.thumb_pictures.animating(self.now)
            || self.feed_fold_at.values().any(|at| self.now.saturating_duration_since(*at) < FOLD)
            || self.read_fade.is_animating(self.now)
            || self.person_fade.is_animating(self.now)
            || (self.overlay == Overlay::Community && [self.section_at, self.shift_at, self.stream_at, self.person_at].iter().any(|at| self.now.saturating_duration_since(*at).as_secs_f32() < ui::APPEAR_ALL))
            || (self.community_reading.is_some() && !self.read_fade.value())
            || self.arrivals.values().any(|a| a.is_animating(self.now))
            || self.leaving.values().any(|a| a.is_animating(self.now))
            || self.billing.animating(self.now)
            || self.bell_flash.is_animating(self.now)
            || self.error_fade.is_animating(self.now)
            || self.sending.as_ref().is_some_and(|s| s.over.is_none())
            || self.gliding()
            || self.glowing()
            || (self.banners_waiting && self.overlay == Overlay::None && !self.gallery)
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
            (iced::Event::Window(window::Event::Opened { size, .. }), _) => Some(Message::WindowOpened(id, size.width, size.height)),
            (iced::Event::Window(window::Event::Rescaled(scale)), _) => Some(Message::Rescaled(scale)),
            (iced::Event::Window(window::Event::Focused | window::Event::Unfocused), _) => Some(Message::CheckMinimized(id)),
            (iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(modifiers)), _) => Some(Message::PoolHeld(modifiers.shift())),
            (iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, physical_key, modifiers, .. }), iced::event::Status::Ignored) => {
                use iced::keyboard::key::{Key, Named};
                if paste_shortcut(&key, physical_key, modifiers) {
                    return Some(Message::UserInput(Some(Box::new(Message::Paste))));
                }
                match key.as_ref() {
                    Key::Named(Named::ArrowLeft) => Some(Message::Key(Named::ArrowLeft)),
                    Key::Named(Named::ArrowUp) => Some(Message::Key(Named::ArrowUp)),
                    Key::Named(Named::ArrowRight) => Some(Message::Key(Named::ArrowRight)),
                    Key::Named(Named::ArrowDown) => Some(Message::Key(Named::ArrowDown)),
                    Key::Named(Named::Space) => Some(Message::Key(Named::Space)),
                    Key::Named(Named::Escape) => Some(Message::Escape),
                    Key::Named(Named::Enter) => Some(Message::PoolHot(crate::pools_screen::Hot::Confirm)),
                    Key::Character(letter) if modifiers.command() && !modifiers.shift() && latin(letter) == Some('z') => Some(Message::Undo),
                    Key::Character(letter) if modifiers.command() && modifiers.shift() && latin(letter) == Some('z') => Some(Message::Redo),
                    Key::Character(letter) if modifiers.command() && latin(letter) == Some('k') => Some(Message::PoolCommands),
                    Key::Character(letter) if modifiers.command() && latin(letter) == Some('f') => Some(Message::SearchLine(true)),
                    Key::Character(letter) if modifiers.command() && latin(letter) == Some('n') => Some(Message::PoolHot(crate::pools_screen::Hot::Add)),
                    Key::Character(letter) if modifiers.command() && modifiers.shift() && latin(letter) == Some('c') => Some(Message::PoolHot(crate::pools_screen::Hot::Text)),
                    Key::Character(letter) if modifiers.command() && latin(letter) == Some('c') => Some(Message::PoolHot(crate::pools_screen::Hot::Copy)),
                    Key::Character(letter) if modifiers.command() && latin(letter) == Some('d') => Some(Message::PoolHot(crate::pools_screen::Hot::Double)),
                    Key::Named(Named::Backspace) if modifiers.command() => Some(Message::PoolHot(crate::pools_screen::Hot::Remove)),
                    Key::Named(Named::Delete) => Some(Message::PoolHot(crate::pools_screen::Hot::Remove)),
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
            if self.billing.waiting() {
                parts.push(iced::time::every(crate::billing::EVERY).map(|_| Message::Billing(crate::billing::Message::Poll)));
            }
            parts.push(iced::time::every(PRESENCE_EVERY).map(|_| Message::PresenceTick));
            parts.push(iced::time::every(sharing::EVERY).map(|_| Message::Sharing(sharing::Message::Tick)));
            if self.witness_delivery.pending() > 0 {
                parts.push(iced::time::every(Duration::from_secs(2)).map(|_| Message::WitnessDeliveryTick));
            }
        }
        if crate::mapper_faces::waiting() && self.overlay == Overlay::Community {
            parts.push(iced::time::every(Duration::from_millis(300)).map(|_| Message::FacesArrived));
        }
        if !self.gallery && !self.hidden && !self.minimized && self.overlay == Overlay::Community && self.community_section == crate::community_screen::Section::Balance && !self.duels.roles.is_empty() {
            parts.push(iced::time::every(Duration::from_secs(2)).map(|_| Message::Community(crate::community_screen::Message::Desk(crate::desk_screen::Message::PresenceTick))));
            if self.desk.posting || self.desk.loading || self.desk.inspecting.is_some() {
                parts.push(iced::time::every(Duration::from_millis(60)).map(|_| Message::Community(crate::community_screen::Message::Desk(crate::desk_screen::Message::BusyFrame))));
            }
        }
        if self.desk.hearing.is_some() && self.overlay == Overlay::Community && self.community_section == crate::community_screen::Section::Balance {
            parts.push(iced::time::every(Duration::from_millis(250)).map(|_| Message::Community(crate::community_screen::Message::Desk(crate::desk_screen::Message::Heard))));
        }
        if !self.gallery && !self.hidden && !self.minimized && self.duels.clock_running() {
            parts.push(iced::time::every(Duration::from_millis(250)).map(|_| Message::Community(crate::community_screen::Message::Duels(crate::duels_screen::Message::Clock))));
        }
        let moving = self.duels.lively() || self.duels.animating();
        if (self.duels.watching.is_some() || moving) && self.overlay == Overlay::Community && self.community_section == crate::community_screen::Section::Duels {
            let every = if moving { Duration::from_millis(16) } else { Duration::from_millis(33) };
            parts.push(iced::time::every(every).map(|_| Message::Community(crate::community_screen::Message::Duels(crate::duels_screen::Message::Frame))));
        }
        if !self.settings.token.is_empty() && !self.gallery {
            let here = self.overlay == Overlay::Community && self.community_section == crate::community_screen::Section::Duels;
            let every = if self.duels.polling() { Duration::from_secs(2) } else if here { Duration::from_secs(10) } else { Duration::from_secs(15) };
            parts.push(iced::time::every(every).map(|_| Message::Community(crate::community_screen::Message::Duels(crate::duels_screen::Message::Again))));
        }
        if self.pools.has_unsaved() {
            parts.push(iced::time::every(Duration::from_millis(250)).map(|_| Message::Community(crate::community_screen::Message::Pools(crate::pools_screen::Message::FlushSaves))));
        }
        if !self.gallery && !self.news_picture_failures.is_empty() {
            parts.push(iced::time::every(Duration::from_secs(30)).map(|_| Message::RetryPictures));
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
            if self.settings.auto_fetch_maps && !self.gallery {
                parts.push(iced::time::every(Duration::from_secs(3)).map(|_| Message::AutoFetch));
            }
        }
        if self.settings.auto_flip && self.library.is_some() && self.overlay == Overlay::None && self.player.is_none() {
            parts.push(iced::time::every(AUTO_EVERY).map(|_| Message::AutoNext));
        }
        if matches!(self.update, UpdateState::Ready { .. }) && (self.update_wanted || self.settings.quiet_updates) {
            parts.push(iced::time::every(Duration::from_secs(5)).map(|_| Message::UpdateIdle));
        }
        let moving = self.moving();
        if moving || (self.resting.value() && self.reel.running()) {
            parts.push(window::frames().map(Message::Tick));
        }
        if !moving && self.toasts.iter().any(|toast| toast.shown.value() && !toast.hovered) {
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
        if self.overlay == Overlay::Settings && self.side == Side::Bot && !self.settings.token.is_empty() {
            parts.push(iced::time::every(Duration::from_secs(10)).map(|_| Message::FarmTick));
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
        self.laid().order.clone()
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
        self.focus = None;
        if self.chosen == Some(at) {
            return Task::none();
        }
        self.glide_from_here();
        self.share_open = false;
        let before = self.chosen_entry().map(|e| self.shown(e));
        let map_before = self.chosen_entry().map(|e| e.map_hash.clone());
        let scene_before = self.chosen_entry().map(|e| self.scenes.get(&e.map_hash).cloned());
        self.before = before;
        self.chosen = Some(at);
        self.settle();
        let same_map = map_before.is_some() && self.chosen_entry().map(|e| &e.map_hash) == map_before.as_ref();
        if self.rendering.as_ref().is_some_and(Rendering::is_over) {
            self.rendering = None;
        }
        self.fetching.retain(|fetching| !fetching.is_over());
        if !same_map {
            let midway = self.swap_waits || self.swap.interpolate(0.0, 1.0, Instant::now()) < 0.5;
            if !midway {
                self.scene_before = scene_before;
            }
            self.swap_waits = self.chosen_entry().is_some_and(|e| e.map.is_some() && !self.scenes.contains_key(&e.map_hash));
            if !self.swap_waits {
                self.swap = Animation::new(false).duration(SWAP).easing(Easing::EaseOutCubic).go(true, Instant::now());
            }
        }
        self.told = Animation::new(false).duration(SWAP).easing(Easing::EaseOutCubic).go(true, Instant::now());
        Task::batch([self.fetch_for_chosen(), self.start_live()])
    }

    fn start_live(&mut self) -> Task<Message> {
        if let Some(live) = self.live.take() {
            live.control.stop();
            self.live_before = live.frame.or(self.live_before.take());
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
                start: 0.0,
                once: false,
            })
        }) else {
            if let Some(entry) = self.chosen_entry() {
                let path = entry.path.clone();
                return ui::in_thread(move || Message::Traced(path.clone(), std::sync::Arc::new(live::trace(&path))));
            }
            return Task::none();
        };
        let control = std::sync::Arc::new(live::Control::default());
        control.suspend(self.minimized);
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

    fn glide_from_here(&mut self) {
        if self.gallery {
            return;
        }
        let laid = self.laid();
        let target = self.carousel_target();
        let shown = self.display(&laid);
        let open = laid.rows.iter().position(|row| matches!(row, crate::carousel::Row::Head { open: true, .. }));
        let nested: Vec<crate::carousel::Row> = match open {
            Some(at) => laid.rows[at + 1..].iter().take_while(|row| row.depth() > 0.0).cloned().collect(),
            None => Vec::new(),
        };
        let head = open.and_then(|at| match &laid.rows[at] { crate::carousel::Row::Head { title, first, .. } => Some((title.clone(), *first)), _ => None });
        self.hold = Some(select::Hold { rows: shown.rows, factors: shown.factors, scroll: self.scroll.unwrap_or(target), head, nested });
    }

    fn settle(&mut self) {
        self.settle_with(true);
    }

    fn regroup(&mut self) {
        self.settle_with(false);
    }

    fn settle_with(&mut self, unfold: bool) {
        self.banners_waiting = true;
        let Some(hold) = self.hold.take() else {
            return;
        };
        self.free = None;
        let laid = self.laid();
        let open = laid.rows.iter().position(|row| matches!(row, crate::carousel::Row::Head { open: true, .. }));
        let titled = |rows: &[crate::carousel::Row], at: Option<usize>| at.and_then(|at| match &rows[at] { crate::carousel::Row::Head { title, first, .. } => Some((title.clone(), *first)), _ => None });
        let now_at = Instant::now().max(self.now);
        let head_now = titled(&laid.rows, open);
        if unfold {
            if hold.head != head_now {
                let was_open = self.unfolded_at(now_at);
                self.folding.retain(|fold| fold.left(now_at) > 0.001);
                let back = head_now.as_ref().and_then(|head| self.folding.iter().position(|fold| &fold.head == head));
                let start = back.map_or(0.0, |at| self.folding.remove(at).left(now_at));
                if let Some(head) = hold.head.clone().filter(|_| !hold.nested.is_empty()) {
                    self.folding.push(select::Fold { head, rows: hold.nested.clone(), at: now_at, from: was_open });
                }
                self.unfold_at = Some(now_at);
                self.unfold_from = start;
            }
        } else {
            self.unfold_at = None;
            self.unfold_from = 0.0;
            self.folding.clear();
        }
        let shown = self.display(&laid);
        let by_title = |rows: &[crate::carousel::Row], factors: &[f32], title: &str| rows.iter().position(|row| matches!(row, crate::carousel::Row::Head { title: known, .. } if known == title)).map(|at| select::shown_top(rows, factors, at));
        let replay_top = |rows: &[crate::carousel::Row], factors: &[f32]| self.chosen.and_then(|chosen| rows.iter().position(|row| matches!(row, crate::carousel::Row::Replay { at, .. } if *at == chosen))).map(|at| select::shown_top(rows, factors, at));
        let (was, now) = match (replay_top(&hold.rows, &hold.factors), replay_top(&shown.rows, &shown.factors)) {
            (Some(was), Some(now)) => (Some(was), Some(now)),
            _ => match head_now.as_ref() {
                Some((title, _)) => (by_title(&hold.rows, &hold.factors, title), by_title(&shown.rows, &shown.factors, title)),
                None => (select::row_top(&hold.rows, &hold.factors, self.chosen), select::row_top(&shown.rows, &shown.factors, self.chosen)),
            },
        };
        self.scroll = Some(match (was, now) {
            (Some(was), Some(now)) => hold.scroll + was - now,
            _ => hold.scroll,
        });
    }

    fn keep_choice(&mut self) -> Task<Message> {
        let laid = self.laid();
        match laid.chosen {
            Some(at) if Some(at) != self.chosen => self.choose(at),
            _ => Task::none(),
        }
    }

    fn ratings_task(&mut self) -> Task<Message> {
        if self.ratings_running || self.gallery {
            return Task::none();
        }
        if !self.ratings_loaded {
            self.ratings = crate::rating::Ratings::load(&crate::rating::Ratings::file());
            self.ratings_loaded = true;
        }
        let wanted: Vec<(PathBuf, library::Map, String, String)> = self
            .entries()
            .iter()
            .filter(|entry| !self.ratings.known.get(&entry.replay_hash).is_some_and(crate::rating::Rating::complete))
            .filter_map(|entry| entry.map.clone().map(|map| (entry.path.clone(), map, entry.map_hash.clone(), entry.replay_hash.clone())))
            .collect();
        if wanted.is_empty() {
            return Task::none();
        }
        self.ratings_running = true;
        ui::streamed(move |push| {
            for (path, map, map_hash, replay_hash) in wanted {
                if !push(Message::Rated(replay_hash, crate::rating::rate(&path, &map, &map_hash))) {
                    return;
                }
            }
            let _ = push(Message::RatingsDone);
        })
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
                .filter(|e| e.map.is_none())
                .map(|e| &e.map_hash)
                .chain(self.sharing.videos.iter().map(|video| &video.map_hash))
                .filter(|hash| !hash.is_empty() && !self.thumbs.contains_key(*hash) && !self.covers_asked.contains(*hash))
                .filter(|hash| seen.insert((*hash).clone()))
                .take(COVERS_AT_ONCE)
                .cloned()
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

    fn duel_window(&mut self, message: crate::duels_screen::Message) -> Task<Message> {
        match message {
            crate::duels_screen::Message::Open => {
                let shown = if self.overlay == Overlay::Community { Task::none() } else { self.update(Message::Show(Overlay::Community)) };
                let gone = self.update(Message::Community(crate::community_screen::Message::Go(crate::community_screen::Section::Duels, None)));
                Task::batch([shown, gone])
            }
            crate::duels_screen::Message::Leave => {
                self.duel_leaving = false;
                self.duel_left = true;
                let _ = self.pools.flush_saves(true);
                iced::exit()
            }
            _ => {
                self.duel_leaving = false;
                Task::none()
            }
        }
    }

    fn tell_duel(&mut self, event: &crate::witness::Event) {
        if let crate::witness::Event::Playing(progress) = event {
            self.duels.played_ms = Some(progress.time);
        }
        if let (crate::witness::Event::Playing(progress), Some(link), Some(duel)) = (event, &self.duel_link, self.duels.shown().filter(|duel| duel.state == "PLAYING")) {
            let hits = progress.n300 + progress.n100 + progress.n50 + progress.miss;
            let accuracy = if hits == 0 { 1.0 } else { f64::from(300 * progress.n300 + 100 * progress.n100 + 50 * progress.n50) / f64::from(300 * hits) };
            link.send(crate::duels::Order::Witness(duel.id.clone(), serde_json::json!({ "score": progress.score, "combo": progress.combo, "accuracy": accuracy, "time": progress.time })));
        }
    }

    fn desk_work(&mut self, effects: Vec<crate::desk_screen::Effect>) -> Task<Message> {
        use crate::desk_screen::{Effect, Message as K, Post};
        let account = crate::duels::Account { server: self.settings.server.clone(), token: self.settings.token.clone(), device: self.settings.device.clone() };
        let offline = account.token.is_empty() || self.gallery;
        let roles = self.duels.roles.clone();
        let wrap = |message: K| Message::Community(crate::community_screen::Message::Desk(message));
        let mut tasks: Vec<Task<Message>> = Vec::new();
        for effect in effects {
            let account = account.clone();
            tasks.push(match effect {
                Effect::Load if offline => Task::none(),
                Effect::Load => {
                    let roles = roles.clone();
                    ui::in_thread(move || K::Loaded(crate::desk::load(&account, &roles).map_err(|why| why.to_string()))).map(wrap)
                }
                Effect::Refresh(_) if offline => Task::none(),
                Effect::Refresh(serial) => {
                    let roles = roles.clone();
                    ui::in_thread(move || K::Refreshed(serial, crate::desk::load(&account, &roles).map_err(|why| why.to_string()))).map(wrap)
                }
                Effect::RefreshMap(..) if offline => Task::none(),
                Effect::RefreshMap(serial, beatmap) => ui::in_thread(move || K::RefreshedMap(serial, beatmap, crate::desk::inspect(&account, beatmap).map_err(|why| why.to_string()))).map(wrap),
                Effect::Cover(_) if self.gallery => Task::none(),
                Effect::Cover(set) => ui::in_thread(move || {
                    let url = format!("https://assets.ppy.sh/beatmaps/{set}/covers/cover@2x.jpg");
                    let handle = crate::news::picture(&url).and_then(|bytes| decoded_from(&bytes, POOL_COVER.0, Some(POOL_COVER)));
                    K::Cover(set, handle)
                }).map(wrap),
                Effect::Hear(beatmap) => {
                    self.duel_preview = beatmap.filter(|_| !self.gallery).map(|beatmap| std::sync::Arc::new(crate::preview::Preview::start(beatmap)));
                    Task::none()
                }
                Effect::Cell(..) if offline => Task::none(),
                Effect::Cell(slot, band) => ui::in_thread(move || { let result = crate::desk::cell(&account, &slot, band).map_err(|why| why.to_string()); K::Held(slot, band, result) }).map(wrap),
                Effect::Inspect(_) if offline => Task::none(),
                Effect::Inspect(beatmap) => ui::in_thread(move || K::Inspected(beatmap, crate::desk::inspect(&account, beatmap).map_err(|why| why.to_string()))).map(wrap),
                Effect::Open(url) => {
                    let _ = open::that(url);
                    Task::none()
                }
                Effect::About(_) if self.gallery => Task::none(),
                Effect::About(beatmap) => ui::in_thread(move || K::About(beatmap, crate::desk::about(beatmap))).map(wrap),
                Effect::Archive => Task::perform(
                    async {
                        let picked = rfd::AsyncFileDialog::new().add_filter("osu!", &["osz"]).pick_file().await?;
                        Some(picked.path().to_path_buf())
                    },
                    |picked| picked.map_or_else(Vec::new, |path| crate::desk::archive_maps(&path)),
                ).map(move |found| wrap(K::Archived(found))),
                Effect::Older(_) if offline => Task::done(wrap(K::JournalOlder(Err("offline".to_owned())))),
                Effect::Older(before) => ui::in_thread(move || K::JournalOlder(crate::desk::journal_before(&account, before).map_err(|why| why.to_string()))).map(wrap),
                Effect::AddMap { beatmap, .. } if offline => Task::done(wrap(K::MapAdded(beatmap, Err("offline".to_owned())))),
                Effect::AddMap { beatmap, slot, reason } => ui::in_thread(move || K::MapAdded(beatmap, crate::desk::add_map(&account, beatmap, slot.as_deref(), &reason).map_err(|why| why.to_string()))).map(wrap),
                Effect::Train(_) if offline => Task::done(wrap(K::TrainStarted(Err("offline".to_owned())))),
                Effect::Train(reason) => ui::in_thread(move || K::TrainStarted(crate::desk::train(&account, &reason).map_err(|why| why.to_string()))).map(wrap),
                Effect::TrainingJob(job) if offline => Task::done(wrap(K::TrainingState(job, Err("offline".to_owned())))),
                Effect::TrainingJob(job) => ui::in_thread(move || { let said = crate::desk::training_job(&account, &job).map_err(|why| why.to_string()); K::TrainingState(job, said) }).map(wrap),
                Effect::Post(_) if offline => Task::done(wrap(K::Posted(Err("offline".to_owned())))),
                Effect::Post(post) => ui::in_thread(move || {
                    let said = match post {
                        Post::Review { beatmap, decision, slot, secondary, reason } => crate::desk::review(&account, beatmap, decision, slot.as_deref(), secondary.as_ref().map(Option::as_deref), &reason),
                        Post::Label { cell, beatmap, label, reason } => crate::desk::label(&account, &cell, beatmap, &label, &reason),
                        Post::Tournament { name, year, pool, reason } => crate::desk::tournament(&account, &name, year, &pool, &reason),
                        Post::Publish { version, reason } => crate::desk::publish(&account, &version, &reason),
                        Post::Rollback { reason } => crate::desk::rollback(&account, &reason),
                        Post::Finalize { duel, match_id, game_id, reason } => crate::desk::finalize(&account, &duel, match_id, game_id, &reason),
                        Post::Cancel { duel, reason } => crate::desk::cancel(&account, &duel, &reason),
                        Post::Grant { who, role, reason } => crate::desk::grant(&account, &who, &role, &reason),
                        Post::Revoke { player, role, reason } => crate::desk::revoke(&account, player, &role, &reason),
                        Post::Discover { band, reason } => crate::desk::discover(&account, band, &reason),
                        Post::Delete { version, reason } => crate::desk::delete_model(&account, &version, &reason),
                        Post::RenameBatch { name, year, new_name, new_year, reason } => crate::desk::rename_batch(&account, &name, year, &new_name, new_year, &reason),
                        Post::DeleteBatch { name, year, beatmap, reason } => crate::desk::delete_batch(&account, &name, year, beatmap, &reason),
                        Post::ExcludeBatch { name, year, beatmap, excluded, reason } => crate::desk::exclude_batch(&account, &name, year, beatmap, excluded, &reason),
                    };
                    K::Posted(said.map(|_| ()).map_err(|why| why.to_string()))
                }).map(wrap),
            });
        }
        Task::batch(tasks)
    }

    fn duels_work(&mut self, effects: Vec<crate::duels_screen::Effect>) -> Task<Message> {
        use crate::duels_screen::{Effect, Message as D, Snapshot};
        let account = crate::duels::Account { server: self.settings.server.clone(), token: self.settings.token.clone(), device: self.settings.device.clone() };
        let offline = account.token.is_empty() || self.gallery;
        let wrap = |message: D| Message::Community(crate::community_screen::Message::Duels(message));
        let mut tasks: Vec<Task<Message>> = Vec::new();
        for effect in effects {
            let account = account.clone();
            tasks.push(match effect {
                Effect::Load if offline => Task::none(),
                Effect::Load => {
                    let permissions = account.clone();
                    Task::batch([
                        ui::in_thread(move || D::Roles(crate::duels::roles(&permissions).map_err(|why| why.to_string()))).map(wrap),
                        ui::in_thread(move || {
                            let found = (|| { let mine = crate::duels::mine(&account)?; Ok::<Snapshot, crate::duels::Failure>(Snapshot { queue: mine.queue, me: crate::duels::me(&account)?, live: crate::duels::live(&account)?, mine: mine.duels, notices: crate::duels::notices(&account).unwrap_or_default() }) })();
                            D::Loaded(found.map_err(|why| why.to_string()))
                        }).map(wrap),
                    ])
                },
                Effect::Queue(enter) => ui::in_thread(move || {
                    let said = if offline { Err("offline".to_owned()) } else { crate::duels::queue(&account, enter, &crate::duels::key()).map(|state| state.queue).map_err(|why| why.to_string()) };
                    D::Queued(said)
                }).map(wrap),
                Effect::Answer(duel, version, accept) => ui::in_thread(move || {
                    let said = if offline { Err("offline".to_owned()) } else {
                        crate::duels::act(&account, &duel, if accept { "accept" } else { "decline" }, serde_json::json!({}), Some(version), &crate::duels::key()).map_err(|why| why.to_string())
                    };
                    D::Answered(said)
                }).map(wrap),
                Effect::Read(_) if offline => Task::none(),
                Effect::Read(duel) => ui::in_thread(move || D::Read(crate::duels::read(&account, &duel).map_err(|why| why.to_string()))).map(wrap),
                Effect::Act(duel, action, body, version) => ui::in_thread(move || {
                    let said = if offline { Err("offline".to_owned()) } else { crate::duels::act(&account, &duel, action, body, version, &crate::duels::key()).map_err(|why| why.to_string()) };
                    D::Acted(said)
                }).map(wrap),
                Effect::Cover(set) => ui::in_thread(move || {
                    let url = format!("https://assets.ppy.sh/beatmaps/{set}/covers/cover@2x.jpg");
                    let handle = crate::news::picture(&url).and_then(|bytes| decoded_from(&bytes, POOL_COVER.0, Some(POOL_COVER)));
                    D::Cover(set, handle)
                }).map(wrap),
                Effect::Hear(beatmap) => {
                    self.duel_preview = beatmap.filter(|_| !self.gallery).map(|beatmap| std::sync::Arc::new(crate::preview::Preview::start(beatmap)));
                    Task::none()
                }
                Effect::Copy(text) => {
                    self.say(self.words.t("duels-copied"));
                    iced::clipboard::write(text)
                }
                Effect::Link(false) => {
                    self.duel_link = None;
                    if let Some(control) = &self.witness_control {
                        control.stream_cursor(None);
                    }
                    Task::none()
                }
                Effect::Link(true) if offline || self.duel_link.is_some() => Task::none(),
                Effect::Link(true) => {
                    let (cursor_in, cursor_out) = std::sync::mpsc::channel();
                    if let Some(control) = &self.witness_control {
                        control.stream_cursor(Some(cursor_in));
                    }
                    let (heard_in, heard_out) = std::sync::mpsc::channel::<crate::duels::Heard>();
                    self.duel_link = Some(std::sync::Arc::new(crate::duels::connect(account, Some(cursor_out), move |heard| heard_in.send(heard).is_ok())));
                    ui::streamed(move |push: &mut dyn FnMut(Message) -> bool| {
                        while let Ok(heard) = heard_out.recv() {
                            if !push(Message::Community(crate::community_screen::Message::Duels(D::Heard(heard)))) {
                                return;
                            }
                        }
                    })
                }
                Effect::Tempo(_) if offline => Task::none(),
                Effect::Tempo(beatmap) => ui::in_thread(move || D::Tempo(beatmap, crate::desk::about(beatmap).and_then(|found| found.bpm))).map(wrap),
                Effect::Order(order) => {
                    if let Some(link) = &self.duel_link {
                        link.send(order);
                    }
                    Task::none()
                }
                Effect::Download(sets) => ui::in_thread(move || D::Downloaded(crate::preview::fetch_sets(&sets))).map(wrap),
                Effect::Tell(told, ids) => {
                    for (key, name) in told {
                        let said = self.words.with(key, &[("name", name)]);
                        self.announce(crate::notices::Mark::Plain, said, String::new(), String::new(), String::new(), crate::notices::Link::Duels);
                    }
                    if offline {
                        Task::none()
                    } else {
                        ui::in_thread(move || { let _ = crate::duels::seen(&account, &ids); }).discard()
                    }
                }
                Effect::Invite(player) => ui::in_thread(move || {
                    let said = if offline { Err("offline".to_owned()) } else { crate::duels::invite(&account, player, &crate::duels::key()).map_err(|why| why.to_string()) };
                    D::Answered(said)
                }).map(wrap),
                Effect::Playing(now) => {
                    if let Some(link) = &self.duel_link {
                        link.send(crate::duels::Order::Playing(now));
                    }
                    Task::none()
                }
            });
        }
        Task::batch(tasks)
    }

    fn pools_work(&mut self, effects: Vec<crate::pools_screen::Effect>) -> Task<Message> {
        use crate::pools_screen::{Effect, Message as P};
        let mut tasks: Vec<Task<Message>> = Vec::new();
        for effect in effects {
            match effect {
                Effect::OwnCompiler(id) => {
                    let name = self.osu_card.as_ref().map(|card| card.username.clone()).or_else(|| self.community_card.as_ref().map(|card| card.username.clone())).unwrap_or_default();
                    tasks.push(Task::done(Message::Community(crate::community_screen::Message::Pools(P::OwnCompiler(id, name)))));
                }
                Effect::Faces(names) => {
                    let names: Vec<_> = names.into_iter().filter(|name| !self.pools.faces.contains_key(&name.to_lowercase())).collect();
                    tasks.push(ui::streamed(move |push| {
                        for name in names {
                            let card = crate::osu_profile::load_player(&name).or_else(|| crate::osu_profile::fetch(&name).ok());
                            let face = card.and_then(|card| crate::news::picture(&card.avatar_url)).and_then(|bytes| decoded_from(&bytes, 96, Some((96, 96))));
                            if !push(Message::Community(crate::community_screen::Message::Pools(P::Face(name, face)))) { return; }
                        }
                    }));
                }
                Effect::SuggestRemote(request, mods, low, high) => {
                    let (server, token, device) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
                    let offline = token.is_empty() || self.gallery;
                    tasks.push(ui::in_thread(move || {
                        let found = if offline { Err("offline".to_owned()) } else { crate::bot::suggested(&server, &token, &device, low, high, mods.code()).map_err(|e| e.to_string()) };
                        Message::Community(crate::community_screen::Message::Pools(P::Offered(request, found)))
                    }));
                }
                Effect::Catalogue(collections, offset) => {
                    let (server, token, device) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
                    tasks.push(ui::in_thread(move || Message::Community(crate::community_screen::Message::Pools(P::CatalogueLoaded(collections, offset > 0, crate::bot::publications(&server, &token, &device, if collections { "collection" } else { "pool" }, offset).map_err(|e| e.to_string()))))));
                }
                Effect::Publish(pool) => {
                    let (server, token, device) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
                    let source = pool.backdrop.as_ref().filter(|_| !pool.collection).and_then(|backdrop| match &backdrop.from {
                        crate::pools::BackdropFrom::Map(hash) => self.pools.songs.as_ref()?.get(hash)?.background.clone(),
                        crate::pools::BackdropFrom::File(path) => Some(path.clone()),
                    });
                    tasks.push(ui::in_thread(move || {
                        let picture = source.and_then(|path| crate::pools::backdrop_copy(&path));
                        let result = crate::bot::publish(&server, &token, &device, &pool, picture.as_deref()).map_err(|e| e.to_string());
                        Message::Community(crate::community_screen::Message::Pools(P::Published(pool.id, result)))
                    }));
                }
                Effect::Withdraw(pool) => {
                    let (server, token, device) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
                    tasks.push(ui::in_thread(move || { let result = crate::bot::withdraw(&server, &token, &device, &pool).map_err(|e| e.to_string()); Message::Community(crate::community_screen::Message::Pools(P::Withdrawn(pool.id, result))) }));
                }
                Effect::Author(name) => {
                    if let Some(catalog) = self.community.as_mut() {
                        let at = catalog.people.iter().position(|person| person.name.eq_ignore_ascii_case(&name)).unwrap_or_else(|| {
                            let person = self.everyone.iter().find(|person| person.name.eq_ignore_ascii_case(&name)).map(|person| crate::community::Person { id: person.id, player: person.player, name: person.name.clone(), avatar: person.avatar.clone(), outside: true, ..Default::default() }).unwrap_or_else(|| crate::community::Person { id: -9_000_000 - catalog.people.len() as i64, name: name.clone(), outside: true, ..Default::default() });
                            catalog.people.push(person);
                            catalog.people.len() - 1
                        });
                        tasks.push(self.update(Message::Community(crate::community_screen::Message::Person(Some(at)))));
                    }
                }
                Effect::ReadSongs => {
                    let mut songs = library::songs_of(&self.settings.sources);
                    let own = crate::sources::own_root().join("Songs");
                    if !songs.contains(&own) {
                        songs.push(own);
                    }
                    let sources = self.settings.sources.clone();
                    tasks.push(ui::in_thread(move || {
                        for root in crate::pool_collections::roots(&sources) {
                            let path = crate::sources::stable_at(&root).and_then(|source| source.songs).unwrap_or_else(|| root.join("Songs"));
                            if path.is_dir() && !songs.contains(&path) { songs.push(path); }
                        }
                        let found = library::Index::load(&songs).by_hash;
                        Message::Community(crate::community_screen::Message::Pools(P::Songs(std::sync::Arc::new(found))))
                    }));
                }
                Effect::Collect(id, name, hashes) => {
                    let sources = self.settings.sources.clone();
                    tasks.push(ui::in_thread(move || {
                        let result = crate::pool_collections::add_to_game(&sources, &name, &hashes).map_err(|refused| refused.code());
                        Message::Community(crate::community_screen::Message::Pools(P::Collected(id, result)))
                    }));
                }
                Effect::Uncollect(id, name) => {
                    let sources = self.settings.sources.clone();
                    tasks.push(ui::in_thread(move || {
                        let result = crate::pool_collections::remove_from_game(&sources, &name).map_err(|refused| refused.code());
                        Message::Community(crate::community_screen::Message::Pools(P::Uncollected(id, result)))
                    }));
                }
                Effect::ReadCollections => {
                    let sources = self.settings.sources.clone();
                    tasks.push(ui::in_thread(move || {
                        let found = crate::pool_collections::read(&sources);
                        Message::Community(crate::community_screen::Message::Pools(P::Collections(found)))
                    }));
                }
                Effect::ReadPaste(id, input, contents) => {
                    tasks.push(iced::clipboard::read().map(move |text| Message::Community(crate::community_screen::Message::Pools(P::PastedInto(id.clone(), input, contents.clone(), text.unwrap_or_default())))));
                }
                Effect::ReadBest(force) => {
                    tasks.push(self.card_task(force));
                    let connected = self.community_card.as_ref().is_some_and(|card| !card.username.trim().is_empty())
                        || self.osu_card.as_ref().is_some_and(|card| !card.username.trim().is_empty())
                        || self.community.as_ref().is_some_and(|catalog| !catalog.staged && catalog.you().is_some());
                    if connected {
                        tasks.push(self.osu_task(force));
                    }
                }
                Effect::Suggest(request, songs, mods, target, excluded, stop) => {
                    tasks.push(ui::in_thread(move || {
                        let found = crate::pools::suggest(&songs, mods, target, &excluded, &stop);
                        Message::Community(crate::community_screen::Message::Pools(P::Suggested(request, found)))
                    }));
                }
                Effect::Measure(hash, map, mods) => {
                    tasks.push(ui::in_thread(move || {
                        let said = crate::pools::measure(&map, &hash, mods);
                        Message::Community(crate::community_screen::Message::Pools(P::Measured(hash, mods, said)))
                    }));
                }
                Effect::Resolve(request, target) => {
                    tasks.push(ui::in_thread(move || Message::Community(crate::community_screen::Message::Pools(P::Resolved(request, crate::pool_links::resolve(&target))))));
                }
                Effect::Cover(url, set) => {
                    tasks.push(ui::in_thread(move || {
                        let handle = crate::news::picture(&url.replace("/covers/cover.jpg", "/covers/cover@2x.jpg"))
                            .or_else(|| crate::news::picture(&url))
                            .and_then(|bytes| decoded_from(&bytes, POOL_COVER.0, Some(POOL_COVER)));
                        Message::Community(crate::community_screen::Message::Pools(P::Cover(set, handle)))
                    }));
                }
                Effect::Copy(text, key) => {
                    let said = self.words.t(key);
                    self.say(said);
                    tasks.push(iced::clipboard::write(text));
                }
                Effect::SaveImage(request, pool) => {
                    let name = crate::pool_card::file_name(&pool);
                    let lang = self.words.lang();
                    let backing = self.pools.backdrop.as_ref().filter(|(key, _)| key.starts_with(&format!("{}:", pool.id))).map(|(_, picture)| picture.clone());
                    let paths: HashMap<String, PathBuf> = pool.slots.iter().filter_map(|slot| {
                        let hash = slot.hash.as_ref()?;
                        let path = self.pools.songs.as_ref()?.get(hash)?.background.as_ref()?;
                        Some((hash.clone(), path.clone()))
                    }).collect();
                    let cached: HashMap<String, image::Handle> = pool.slots.iter().filter_map(|slot| {
                        let hash = slot.hash.as_ref()?;
                        self.thumbs.get(hash).map(|image| (hash.clone(), image.clone()))
                    }).collect();
                    tasks.push(Task::perform(
                        async move { rfd::AsyncFileDialog::new().add_filter("PNG", &["png"]).set_file_name(&name).save_file().await.map(|file| file.path().to_path_buf()) },
                        |path| path,
                    ).then(move |path| match path {
                        Some(path) => {
                            let pool = pool.clone();
                            let paths = paths.clone();
                            let cached = cached.clone();
                            let backing = backing.clone();
                            ui::in_thread(move || {
                                let covers = crate::pool_card::covers(&pool, &paths, &cached);
                                let said = crate::pool_card::render(&pool, &crate::lang::Words::new(lang), &covers, backing.as_ref())
                                    .and_then(|bytes| crate::pool_card::save(&path, &bytes)).map(|()| Some(path));
                                Message::Community(crate::community_screen::Message::Pools(P::ImageSaved(request, said)))
                            })
                        }
                        None => Task::done(Message::Community(crate::community_screen::Message::Pools(P::ImageSaved(request, Ok(None))))),
                    }));
                }
                Effect::SaveFile(name, bytes) => {
                    tasks.push(Task::perform(
                        async move {
                            let picked = rfd::AsyncFileDialog::new().add_filter("Dossier pool", &["pool"]).set_file_name(name.as_str()).save_file().await;
                            match picked {
                                Some(file) => std::fs::write(file.path(), bytes).map(|()| Some(file.path().to_path_buf())).map_err(|why| why.to_string()),
                                None => Ok(None),
                            }
                        },
                        |said| Message::Community(crate::community_screen::Message::Pools(P::Saved(said))),
                    ));
                }
                Effect::PickBackdrop => {
                    tasks.push(Task::perform(
                        async { rfd::AsyncFileDialog::new().add_filter("JPG, PNG", &["jpg", "jpeg", "png"]).pick_file().await.map(|file| file.path().to_path_buf()) },
                        |picked| Message::Community(crate::community_screen::Message::Pools(P::BackdropPicked(picked))),
                    ));
                }
                Effect::LoadBackdrop(key, path, blur) => {
                    tasks.push(ui::in_thread(move || {
                        let picture = decoded(&path, 1600, None).and_then(|picture| if blur { ui::frosted(&picture) } else { Some(picture) });
                        Message::Community(crate::community_screen::Message::Pools(P::BackdropLoaded(key, picture)))
                    }));
                }
                Effect::PickFile => {
                    tasks.push(Task::perform(
                        async {
                            let picked = rfd::AsyncFileDialog::new().add_filter("Dossier pool", &["pool"]).pick_file().await?;
                            Some(crate::pool_files::read_pool(picked.path(), unix_now()))
                        },
                        |said| match said {
                            Some(said) => Message::Community(crate::community_screen::Message::Pools(P::Imported(said))),
                            None => Message::Community(crate::community_screen::Message::Pools(P::Saved(Ok(None)))),
                        },
                    ));
                }
                Effect::ReadPool(request, path) => {
                    tasks.push(ui::in_thread(move || Message::Community(crate::community_screen::Message::Pools(P::PoolFile(request, crate::pool_files::read_pool(&path, unix_now()))))));
                }
                Effect::ImportArchive(request, path) => {
                    let songs = crate::sources::own_root().join("Songs");
                    tasks.push(ui::in_thread(move || Message::Community(crate::community_screen::Message::Pools(P::ArchiveFile(request, crate::pool_files::import_archive(&path, &songs))))));
                }
                Effect::ImportMap(request, path) => {
                    let songs = crate::sources::own_root().join("Songs");
                    tasks.push(ui::in_thread(move || Message::Community(crate::community_screen::Message::Pools(P::MapFile(request, crate::pool_files::import_map(&path, &songs))))));
                }
                Effect::Say(key) => {
                    let said = self.words.t(key);
                    self.say(said);
                }
                Effect::Guide(guide) => {
                    self.settings.pool_guide = guide;
                    let _ = self.settings.save();
                }
                Effect::Fetch(request, hash, stop) => {
                    let songs = crate::sources::own_root().join("Songs");
                    let named = hash.clone();
                    tasks.push(crate::maps::fetch(hash, songs, stop).map(move |step| Message::Community(crate::community_screen::Message::Pools(P::Step(request, named.clone(), step)))));
                }
            }
        }
        let wanted: Vec<(String, Option<PathBuf>)> = self
            .pools
            .covers()
            .into_iter()
            .filter(|(hash, _)| self.pool_covers_asked.insert(hash.clone()))
            .collect();
        if !wanted.is_empty() {
            tasks.push(ui::streamed(move |push| {
                for (hash, path) in wanted {
                    let handle = path.as_deref().and_then(|path| decoded(path, POOL_COVER.0, Some(POOL_COVER))).or_else(|| {
                        let known = crate::maps::known(&hash)?;
                        let bytes = crate::news::picture(&known.cover().replace("/covers/cover.jpg", "/covers/cover@2x.jpg"))
                            .or_else(|| crate::news::picture(&known.cover()))?;
                        decoded_from(&bytes, POOL_COVER.0, Some(POOL_COVER))
                    });
                    if let Some(handle) = handle {
                        if !push(Message::Thumb(hash, handle)) {
                            return;
                        }
                    }
                }
            }));
        }
        Task::batch(tasks)
    }

    fn banners_task(&mut self, now: Instant) -> Task<Message> {
        if self.gallery || self.overlay != Overlay::None || now.saturating_duration_since(self.banners_at) < select::BANNERS_EVERY {
            return Task::none();
        }
        self.banners_at = now;
        let missing: Vec<(String, PathBuf)> = self.near_banners().into_iter().filter(|(hash, _)| !self.banners_asked.contains(hash)).collect();
        self.banners_waiting = missing.len() > select::BANNERS_AT_ONCE;
        let wanted: Vec<(String, PathBuf)> = missing.into_iter().take(select::BANNERS_AT_ONCE).collect();
        if wanted.is_empty() {
            return Task::none();
        }
        for (hash, _) in &wanted {
            self.banners_asked.insert(hash.clone());
        }
        ui::streamed(move |push| {
            for (hash, path) in wanted {
                if !push(Message::Banner(hash, select::banner_of(&path))) {
                    return;
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

    pub fn unveil(&mut self, now: Instant) {
        self.arrive = Animation::new(false).duration(ARRIVE).easing(Easing::EaseOutCubic).go(true, now);
        if self.library.is_some() {
            self.enter = Animation::new(false).duration(ENTER).easing(Easing::EaseOutCubic).go(true, now);
        }
    }

    fn wake(&mut self, now: Instant) {
        self.last_input = now;
        self.rest_waiting = false;
        if self.resting.value() {
            self.resting.go_mut(false, now);
        }
        if !self.resting.is_animating(now) {
            self.reel_stop();
            self.reel.before = None;
        }
        if std::mem::take(&mut self.live_rested) {
            if let Some(live) = &self.live {
                live.control.pause(false);
            }
        }
    }

    fn begin_rest(&mut self, now: Instant) {
        self.rest_waiting = false;
        self.resting.go_mut(true, now);
        if let Some(live) = self.live.as_ref().filter(|live| !live.control.paused()) {
            live.control.pause(true);
            self.live_rested = true;
        }
        self.hover = None;
        self.hover_bounds = None;
        self.hover_since = None;
        for lift in self.lifts.values_mut() { lift.go_mut(false, now); }
    }

    fn check_rest(&mut self, now: Instant) -> Task<Message> {
        if !self.resting.value() && !self.rest_waiting && !self.resting.is_animating(now) {
            self.reel_stop();
            self.reel.before = None;
        }
        if !self.can_rest() {
            self.wake(now);
        } else if !self.resting.value() && !self.rest_waiting && now.saturating_duration_since(self.last_input) >= REST_AFTER {
            self.rest_waiting = true;
            let task = self.reel_next();
            if !self.reel.running() { self.begin_rest(now); }
            return task;
        }
        if self.resting.value() && !self.reel.running() { self.reel_next() } else { Task::none() }
    }

    pub fn set_hidden(&mut self, hidden: bool) {
        self.hidden = hidden;
        self.set_minimized(hidden);
    }

    pub fn prepare_exit(&mut self) -> bool {
        if !self.duel_left && self.duels.shown().is_some_and(|duel| crate::duels_screen::phase_of(&duel.state) != crate::duels_screen::Phase::Result) {
            self.duel_leaving = true;
            return false;
        }
        if self.pools.flush_saves(true) { return true; }
        self.announce(notices::Mark::Bad, self.words.t("pool-unsaved"), String::new(), String::new(), String::new(), notices::Link::None);
        false
    }

    fn set_minimized(&mut self, minimized: bool) {
        if self.minimized == minimized { return; }
        crate::frames::mark("window", Instant::now(), if minimized { "minimized" } else { "restored" });
        self.minimized = minimized;
        if let Some(live) = &self.live { live.control.suspend(minimized); }
        if let Some((_, live)) = &self.reel.now { live.control.suspend(minimized); }
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
        let task = self.handle(message);
        self.watch_pools();
        task
    }

    fn watch_pools(&mut self) {
        if self.overlay == Overlay::Community && self.community_section == crate::community_screen::Section::Pools {
            self.pools_marks.observe(&self.pools, Instant::now());
        }
    }

    fn handle(&mut self, message: Message) -> Task<Message> {
        if self.minimized && matches!(&message,
            Message::Tick(_) | Message::RestCheck(_) | Message::WatchTick | Message::AutoNext
            | Message::UpdateTick | Message::UpdateIdle | Message::Poll | Message::CommunityTick
            | Message::NewsTick | Message::FarmTick | Message::LiveArrive)
        {
            return Task::none();
        }
        match message {
            Message::WindowOpened(id, width, height) => {
                self.window_id = Some(id);
                self.width = width;
                self.height = height;
                window::scale_factor(id).map(Message::Rescaled)
            }
            Message::Rescaled(scale) => {
                ui::set_device_scale(scale);
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
                self.now = now;
                self.check_rest(now)
            }
            Message::Reel(serial, frame) => {
                if !self.resting.value() && !self.rest_waiting && !self.resting.is_animating(Instant::now()) {
                    self.reel_stop();
                    self.reel.before = None;
                    return Task::none();
                }
                let task = self.reel_frame(serial, frame);
                if self.rest_waiting && (self.reel.now.as_ref().is_some_and(|(_, live)| live.frame.is_some()) || !self.reel.running()) {
                    self.begin_rest(Instant::now());
                }
                task
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
                Task::batch([self.watch_task(), self.history_task(false)])
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
                Task::batch([self.thumbs_task(), self.covers_task(), nudge, self.donate_task(), self.replays_give_task(), self.shared_faces_task(), self.ratings_task()])
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
                Task::batch([self.fetch_for_chosen(), self.thumbs_task(), self.covers_task(), self.start_live(), self.watch_task(), self.donate_task(), self.replays_give_task(), self.shared_faces_task(), self.ratings_task()])
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
                let pixels = |handle: &image::Handle| match handle {
                    image::Handle::Rgba { width, height, .. } => u64::from(*width) * u64::from(*height),
                    _ => 0,
                };
                if self.thumbs.get(&hash).is_none_or(|held| pixels(&handle) >= pixels(held)) {
                    self.cover_frames.borrow_mut().remove(&hash);
                    self.thumbs.insert(hash, handle);
                }
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
                self.step_focus(by);
                Task::none()
            }
            Message::CarouselDrag(by) => {
                self.drag_carousel(by);
                Task::none()
            }
            Message::CarouselDrop(speed) => {
                self.drop_carousel(speed);
                Task::none()
            }
            Message::Wheel(lines) => {
                self.drag_carousel(lines * select::WHEEL_STEP);
                Task::none()
            }
            Message::Jump(by) => match self.jump_group(by) {
                Some(at) => self.choose(at),
                None => Task::none(),
            },
            Message::SortBy(sort) => {
                self.glide_from_here();
                self.sort = sort;
                self.regroup();
                self.keep_choice()
            }
            Message::GroupBy(group) => {
                self.glide_from_here();
                self.group = group;
                self.regroup();
                Task::batch([self.keep_choice(), self.shared_faces_task()])
            }
            Message::StarsAt(fraction, pressed) => {
                let value = crate::carousel::star_at(fraction);
                if pressed {
                    self.low_held = crate::carousel::nearer_low(value, self.filter.low, self.filter.high);
                    self.glide_from_here();
                    self.star_drag = true;
                }
                if self.low_held {
                    self.filter.low = value.min(self.filter.high);
                } else {
                    self.filter.high = value.max(self.filter.low);
                }
                Task::none()
            }
            Message::StarsDone => {
                self.star_drag = false;
                self.regroup();
                self.keep_choice()
            }
            Message::FilterToggle(which) => {
                self.glide_from_here();
                match which {
                    select::Toggle::Players => {
                        self.group = if self.group == crate::carousel::Group::Players { crate::carousel::Group::Maps } else { crate::carousel::Group::Players };
                    }
                    select::Toggle::Mine => self.filter.mine = !self.filter.mine,
                    select::Toggle::Witness => self.filter.witness = !self.filter.witness,
                    select::Toggle::Clean => self.filter.clean = !self.filter.clean,
                }
                self.regroup();
                Task::batch([self.keep_choice(), self.shared_faces_task()])
            }
            Message::SearchLine(open) => {
                self.search_open = open;
                if !open {
                    self.filters_open = false;
                    if !self.search.is_empty() {
                        return self.update(Message::Search(String::new()));
                    }
                    return iced::advanced::widget::operate(iced::advanced::widget::operation::focusable::unfocus::<Message>());
                }
                let id = self.search_id.clone();
                iced::advanced::widget::operate(iced::advanced::widget::operation::focusable::focus::<Message>(id))
            }
            Message::FiltersPanel(open) => {
                self.filters_open = open;
                if open {
                    self.search_open = true;
                }
                Task::none()
            }
            Message::FiltersReset => {
                self.glide_from_here();
                self.filter = crate::carousel::Filter::default();
                self.search.clear();
                self.regroup();
                self.keep_choice()
            }
            Message::Rated(hash, rating) => {
                if let Some(rating) = rating {
                    self.ratings.known.insert(hash, rating);
                    self.ratings_fresh += 1;
                    if self.ratings_fresh % 25 == 0 {
                        self.ratings.save(&crate::rating::Ratings::file());
                    }
                }
                Task::none()
            }
            Message::RatingsDone => {
                self.ratings_running = false;
                if self.ratings_fresh > 0 {
                    self.ratings.save(&crate::rating::Ratings::file());
                }
                Task::none()
            }
            Message::ShareMenu(open) => {
                self.share_open = open;
                Task::none()
            }
            Message::ShareVideo(path) => {
                self.share_open = false;
                let Some(at) = self.store.videos.iter().position(|video| video.path == path) else {
                    return Task::none();
                };
                self.open_video = Some(at);
                self.update(Message::SendVideo)
            }
            Message::SendToPlayer(path) => {
                self.share_open = false;
                let Some(at) = self.store.videos.iter().position(|video| video.path == path) else {
                    return Task::none();
                };
                let opened = self.update(Message::OpenVideo(at));
                opened.chain(Task::done(Message::Sharing(sharing::Message::Pick)))
            }
            Message::Key(key) => {
                use iced::keyboard::key::Named;
                let watching = self.player.is_some();
                match (key, watching) {
                    (Named::ArrowLeft, true) => self.update(Message::SeekBy(-5000)),
                    (Named::ArrowRight, true) => self.update(Message::SeekBy(5000)),
                    (Named::Space, true) => self.update(Message::PlayerToggle),
                    (Named::ArrowUp, false) => self.update(Message::Step(-1)),
                    (Named::ArrowDown, false) => self.update(Message::Step(1)),
                    (Named::ArrowLeft | Named::ArrowRight, false) if self.overlay == Overlay::None => {
                        match self.focused() {
                            Some(select::Focus::Head(at)) if self.laid().rows.iter().any(|row| matches!(row, crate::carousel::Row::Head { first, open: false, .. } if *first == at)) => self.choose(at),
                            _ => self.update(Message::Jump(if key == Named::ArrowLeft { -1 } else { 1 })),
                        }
                    }
                    _ => Task::none(),
                }
            }
            Message::Escape => {
                if self.share_open {
                    self.share_open = false;
                } else if self.error_shown.is_some() && self.error_fade.value() {
                    self.error_fade.go_mut(false, Instant::now());
                } else if matches!(self.pairing, Pairing::Asking | Pairing::Waiting { .. } | Pairing::Linking { .. } | Pairing::Unavailable) {
                    self.pairing = Pairing::Idle;
                } else if self.skin_delete.is_some() {
                    return self.prefs(prefs::Message::KeepSkin);
                } else if self.witness_asking {
                    return self.prefs(prefs::Message::WitnessDecline);
                } else if self.billing.shown && self.billing.fade.value() {
                    return self.billing_step(crate::billing::Message::Close);
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
                } else if self.overlay == Overlay::None && self.filters_open {
                    self.filters_open = false;
                } else if self.overlay == Overlay::None && (!self.search.is_empty() || self.search_open) {
                    self.sorting();
                    self.search.clear();
                    self.search_open = false;
                    return iced::advanced::widget::operate(iced::advanced::widget::operation::focusable::unfocus::<Message>());
                } else if self.overlay == Overlay::Community && self.community_section == crate::community_screen::Section::Pools && self.pools.cancel_next().is_some() {
                    return self.update(Message::PoolHot(crate::pools_screen::Hot::Cancel));
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
                self.update(Message::MenuTab(Tab::Head))
            }
            Message::MenuTab(tab) => {
                let now = Instant::now();
                if self.menu == Some(tab) && self.menu_open.value() {
                    return self.update(Message::MenuClose);
                }
                self.menu_open = Animation::new(false).duration(MENU_OPEN).easing(Easing::EaseOutCubic).go(true, now);
                self.menu = Some(tab);
                for toast in self.toasts.iter_mut().filter(|toast| toast.shown.value()) {
                    toast.shown.go_mut(false, now);
                }
                if tab == Tab::Bell {
                    self.notices.see_all();
                    return self.scenes_for_notices();
                }
                return self.billing_asks(false);
            }
            Message::Billing(..)
            | Message::BillingReply(..) => self.handle_billing(message),
            Message::MenuClose => {
                if self.menu_open.value() {
                    self.menu_open = Animation::new(true).duration(MENU_CLOSE).easing(Easing::EaseInCubic).go(false, Instant::now());
                }
                Task::none()
            }
            Message::SeenAll
            | Message::ClearNotices
            | Message::ToastHover(..)
            | Message::ToastClose(..)
            | Message::DismissNotice(..)
            | Message::ShowError(..)
            | Message::HideError
            | Message::ToastLink(..) => self.handle_notifications(message),
            Message::SignIn
            | Message::PairAsked(..)
            | Message::OpenOsu
            | Message::LinkTelegram
            | Message::TelegramAsked(..)
            | Message::PresenceTick
            | Message::PresenceAnswered(..)
            | Message::Poll
            | Message::Polled(..)
            | Message::OpenTelegram
            | Message::CopyLink
            | Message::LaterSignIn
            | Message::SignOut
            | Message::Known(..)
            | Message::Avatar(..)
            | Message::Chats(..)
            | Message::ChatFace(..) => self.handle_account(message),
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
            Message::OpenVideo(..)
            | Message::VideoReady(..)
            | Message::ClosePlayer
            | Message::PlayerMinimize
            | Message::PlayerRestore
            | Message::PlayerToggle
            | Message::SeekBy(..)
            | Message::SeekTo(..)
            | Message::Scrubbing(..)
            | Message::PlayerSpeed(..)
            | Message::StepFrames(..)
            | Message::PlayerLevel(..)
            | Message::PlayerLouder(..)
            | Message::PlayerMute
            | Message::PlayerRate(..)
            | Message::PlayerStir(..)
            | Message::ControlsHover(..)
            | Message::PlayerLoop
            | Message::PlayerWiden
            | Message::PlayerNeighbour(..)
            | Message::RevealVideo
            | Message::AskDeleteOf(..)
            | Message::AskDelete
            | Message::KeepVideo
            | Message::DeleteVideo
            | Message::ClipFetched(..) => self.handle_video_control(message),
            Message::Search(query) => {
                self.glide_from_here();
                self.search = query;
                self.regroup();
                let task = self.keep_choice();
                self.folding.clear();
                self.unfold_at = None;
                self.free = None;
                self.scroll = Some(self.carousel_target());
                self.scroll_speed = 0.0;
                task
            }
            Message::Paste => iced::clipboard::read().map(|text| Message::PoolClipboard(text.unwrap_or_default())),
            Message::PoolClipboard(text) => {
                let in_pools = self.overlay == Overlay::Community && self.community_section == crate::community_screen::Section::Pools;
                if !in_pools && !crate::pools_screen::is_paste(&text) {
                    return Task::none();
                }
                let mut tasks = Vec::new();
                if !in_pools {
                    tasks.push(self.update(Message::Show(Overlay::Community)));
                    tasks.push(self.update(Message::Community(crate::community_screen::Message::Go(crate::community_screen::Section::Pools, None))));
                }
                tasks.push(self.update(Message::Community(crate::community_screen::Message::Pools(crate::pools_screen::Message::Pasted(text)))));
                Task::batch(tasks)
            }
            Message::Undo => {
                if self.overlay == Overlay::Community && self.community_section == crate::community_screen::Section::Pools {
                    return self.update(Message::Community(crate::community_screen::Message::Pools(crate::pools_screen::Message::Undo)));
                }
                Task::none()
            }
            Message::PoolHeld(held) => {
                self.pools.shift = held;
                Task::none()
            }
            Message::PoolHot(crate::pools_screen::Hot::Confirm) if self.overlay == Overlay::None && self.focus.is_some() => match self.focused() {
                Some(select::Focus::Head(first)) | Some(select::Focus::Replay(first)) => {
                    self.focus = None;
                    self.choose(first)
                }
                None => Task::none(),
            },
            Message::PoolHot(hot) => {
                if self.overlay == Overlay::Community && self.community_section == crate::community_screen::Section::Pools {
                    return self.update(Message::Community(crate::community_screen::Message::Pools(crate::pools_screen::Message::Hot(hot))));
                }
                Task::none()
            }
            Message::PoolCommands => {
                if self.overlay == Overlay::Community && self.community_section == crate::community_screen::Section::Pools {
                    return self.update(Message::Community(crate::community_screen::Message::Pools(crate::pools_screen::Message::Palette(true))));
                }
                Task::none()
            }
            Message::Redo => {
                if self.overlay == Overlay::Community && self.community_section == crate::community_screen::Section::Pools {
                    return self.update(Message::Community(crate::community_screen::Message::Pools(crate::pools_screen::Message::Redo)));
                }
                Task::none()
            }
            Message::Typed(letter) => {
                if self.player.is_none() && self.overlay == Overlay::Community && self.community_section == crate::community_screen::Section::Pools {
                    return match letter {
                        'n' | 'т' => self.update(Message::PoolHot(crate::pools_screen::Hot::Note)),
                        'r' | 'к' => self.update(Message::PoolHot(crate::pools_screen::Hot::Replace)),
                        _ => Task::none(),
                    };
                }
                if self.player.is_none() {
                    if self.overlay == Overlay::None && self.menu.is_none() && self.library.is_some() && (letter.is_alphanumeric() || letter == '+') {
                        self.search_open = true;
                        let task = self.update(Message::Search(format!("{}{letter}", self.search)));
                        let id = self.search_id.clone();
                        return Task::batch([task, iced::advanced::widget::operate(iced::advanced::widget::operation::focusable::focus::<Message>(id.clone()))
                            .chain(iced::advanced::widget::operate(iced::advanced::widget::operation::text_input::move_cursor_to_end::<Message>(id)))]);
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
            Message::FolderOpened(result) => {
                if let Err(why) = result {
                    self.say_trouble(why);
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
                if overlay != self.overlay {
                    self.feed_offset = 0.0;
                    self.release_held();
                }
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
                        self.score_pick = None;
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
                    C::ScorePick(who) => self.score_pick = Some(who),
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
                    C::PeopleFilter(filter) => self.people_filter = filter,
                    C::TitleScope(scope) => self.title_scope = scope,
                    C::PeopleOrder(order) => {
                        self.people_order = order;
                        self.people_orders_open = false;
                    }
                    C::PeopleOrders(open) => self.people_orders_open = open,
                    C::Duels(window @ (crate::duels_screen::Message::Open | crate::duels_screen::Message::Stay | crate::duels_screen::Message::Leave)) => return self.duel_window(window),
                    C::Duels(crate::duels_screen::Message::Profile(osu_id)) => {
                        let at = self.community.as_ref().and_then(|catalog| catalog.people.iter().position(|person| person.osu_id == Some(osu_id)));
                        return at.map_or_else(Task::none, |at| self.update(Message::Community(C::Person(Some(at)))));
                    }
                    C::Duels(inner) => {
                        if let Some(player) = self.account.as_ref().and_then(|me| me.player) {
                            self.duels.player = Some(player);
                        }
                        let mut effects = Vec::new();
                        if matches!(inner, crate::duels_screen::Message::Frame) && self.duel_preview.as_ref().is_some_and(|preview| preview.ended()) {
                            self.duel_preview = None;
                            self.desk.hearing = None;
                            effects.extend(self.duels.update(crate::duels_screen::Message::PreviewEnded, unix_now()));
                        }
                        self.now_unix = unix_now();
                        effects.extend(self.duels.update(inner, self.now_unix));
                        return self.duels_work(effects);
                    }
                    C::Desk(inner) => {
                        let poll = matches!(inner, crate::desk_screen::Message::PresenceTick);
                        if matches!(inner, crate::desk_screen::Message::Heard) {
                            if !self.duel_preview.as_ref().is_some_and(|preview| preview.ended()) {
                                return Task::none();
                            }
                            self.desk.silent = self.duel_preview.take().filter(|preview| preview.silent()).map(|preview| preview.beatmap_id);
                        }
                        let effects = self.desk.update(inner, &self.words);
                        let work = self.desk_work(effects);
                        let context = self.desk.presence_context();
                        let active = !self.gallery && !self.hidden && !self.minimized && self.overlay == Overlay::Community && self.community_section == crate::community_screen::Section::Balance;
                        if active && !self.settings.token.is_empty() && !self.duels.roles.is_empty() && !self.desk.presence_busy && (poll || self.desk.presence_sent.as_ref() != Some(&context)) {
                            self.desk.presence_busy = true;
                            self.desk.presence_sent = Some(context.clone());
                            let account = crate::duels::Account { server: self.settings.server.clone(), token: self.settings.token.clone(), device: self.settings.device.clone() };
                            let presence = ui::in_thread(move || crate::desk_screen::Message::Presence(crate::desk::presence(&account, Some(&context.0), context.1).map_err(|why| why.to_string())))
                                .map(|message| Message::Community(C::Desk(message)));
                            return Task::batch([work, presence]);
                        }
                        return work;
                    }
                    C::Pools(inner) => {
                        if let crate::pools_screen::Message::Saved(said) = &inner {
                            match said {
                                Ok(Some(path)) => {
                                    let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
                                    let said = self.words.with("pool-saved", &[("name", name)]);
                                    self.say(said);
                                }
                                Err(_) => {
                                    let said = self.words.t("pool-save-failed");
                                    self.say(said);
                                }
                                Ok(None) => {}
                            }
                            return Task::none();
                        }
                        let failed = self.pools.save_failed();
                        let effects = self.pools.update(inner, unix_now());
                        if !failed && self.pools.save_failed() {
                            self.say(self.words.t("pool-unsaved"));
                        }
                        return self.pools_work(effects);
                    }
                    C::Reconnect => {
                        let shown = self.update(Message::Show(Overlay::Settings));
                        let side = self.update(Message::Prefs(prefs::Message::Side(Side::Bot)));
                        return Task::batch([shown, side]);
                    }
                    C::Again => {
                        let news = self.refresh_news(true);
                        if self.settings.token.is_empty() {
                            self.refresh_asked = false;
                            self.refresh_answered = Some(Instant::now());
                            return news;
                        }
                        self.refresh_asked = true;
                        self.refresh_answered = None;
                        let friends = if self.community_section == crate::community_screen::Section::People { self.friends_task(true) } else { Task::none() };
                        return Task::batch([self.community_task(true), friends, self.card_task(true), news]);
                    }
                    C::Section(section) => {
                        if section != self.community_section {
                            self.section_at = now;
                            self.feed_offset = 0.0;
                            self.release_held();
                        }
                        if self.community_reading.is_some() && self.read_fade.value() {
                            self.read_fade.go_mut(false, now);
                        }
                        if self.community_person.is_some() && self.person_fade.value() {
                            self.person_fade.go_mut(false, now);
                        }
                        self.community_section = section;
                        if section == crate::community_screen::Section::Pools {
                            let effects = self.pools.open();
                            return self.pools_work(effects);
                        }
                        if section == crate::community_screen::Section::Duels {
                            let player = self.account.as_ref().and_then(|me| me.player);
                            let effects = self.duels.open(player);
                            return self.duels_work(effects);
                        }
                        if section == crate::community_screen::Section::Balance {
                            let effects = self.desk.open(&self.duels.roles);
                            return self.desk_work(effects);
                        }
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
                                self.people_filter = crate::community_screen::PeopleFilter::Friends;
                            }
                        }
                        if section == crate::community_screen::Section::People {
                            tasks.push(self.friends_task(false));
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
                        self.person_from = self.community_tap;
                        self.person_fade = Animation::new(false).duration(PANEL_SHOW).easing(Easing::EaseOutCubic).go(true, now);
                        return self.person_task(at);
                    }
                    C::Person(None) => {
                        self.panel_from = self.person_from;
                        self.person_fade.go_mut(false, now);
                    }
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
                        let before = self.feed_before();
                        self.settings.news_channels.retain(|kept| !kept.eq_ignore_ascii_case(&name));
                        let _ = self.settings.save();
                        self.news.take_posts(&name, Vec::new());
                        self.news.fetched.remove(&crate::news::channel_source(&name));
                        self.news.save();
                        self.fresh_from(before);
                    }
                    C::Filter(filter) => {
                        if filter != self.feed_filter {
                            self.stream_at = now;
                        }
                        self.feed_filter = filter;
                    }
                    C::Search(query) => self.feed_query = query,
                    C::FeedScrolled(offset) => {
                        self.feed_offset = offset;
                        if offset <= crate::chronicle::READING_BELOW {
                            self.release_held();
                        }
                    }
                    C::ShowNew => {
                        self.feed_offset = 0.0;
                        self.release_held();
                        return iced::widget::operation::scroll_to(iced::widget::Id::new("community-feed"), iced::widget::scrollable::AbsoluteOffset { x: None, y: Some(0.0) });
                    }
                    C::Toggle(key) => {
                        self.feed_fold_at.retain(|_, at| now.saturating_duration_since(*at) < FOLD);
                        self.feed_fold_at.insert(key.clone(), now);
                        if !self.feed_open.remove(&key) {
                            self.feed_open.insert(key);
                        }
                    }
                    C::Metric(metric) => self.dossier_metric = metric,
                    C::Span(span) => self.dossier_span = span,
                    C::GradeHover(hover) => self.grade_hover = hover,
                    C::Wear(code) => {
                        let chat = self.community.as_ref().and_then(|catalog| catalog.chat);
                        if let Some(me) = self.community.as_mut().and_then(|catalog| catalog.people.iter_mut().find(|p| p.you)) {
                            me.title = code.clone();
                        }
                        let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
                        return ui::in_thread(move || Message::Worn(crate::bot::wear_title(&server, &token, &name, chat, code.as_deref()).map_err(|e| e.to_string())));
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
                        let before = self.feed_before();
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
                        let before = self.feed_before();
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
                        let before = self.feed_before();
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
                        self.news_picture_failures.remove(&url);
                        self.feed_pictures.came(&url, Instant::now());
                        self.news_pictures.insert(url, handle);
                    }
                    None => {
                        self.news_picture_failures.insert(url.clone(), Instant::now());
                        self.feed_pictures.lost(&url, Instant::now());
                    }
                }
                Task::none()
            }
            Message::FacesArrived => Task::none(),
            Message::RetryPictures => {
                let wide = self.community_reading.as_ref().map(|reading| reading.pictures()).unwrap_or_default();
                let tasks = [self.news_pictures_task(), self.community_pictures_task(), self.wide_pictures_task(wide)];
                let now = Instant::now();
                let dormant: Vec<_> = self.news_picture_failures.iter().filter(|(_, failed)| now.saturating_duration_since(**failed) >= Duration::from_secs(30)).map(|(url, _)| url.clone()).collect();
                for url in dormant {
                    self.news_picture_failures.remove(&url);
                    self.news_asked.remove(&url);
                }
                Task::batch(tasks)
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
                if let Some(control) = &self.witness_control {
                    control.context(&event, self.witness.state.as_ref(), &self.pools.list, self.settings.lang == crate::lang::Lang::En);
                }
                self.tell_duel(&event);
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
            Message::WitnessDeliveryTick => self.resolve_witness(),
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
            Message::WitnessDelivered(delivery, result) => {
                let refused = match &result {
                    Err(Refused::NotThere) => true,
                    Err(Refused::Said(code)) => code.parse::<u16>().is_ok_and(|code| (400..500).contains(&code) && code != 429),
                    _ => false,
                };
                let sent = self.witness_delivery.finish(&delivery, result.is_ok(), Instant::now());
                self.witness.pending = self.witness_delivery.pending() as u32;
                self.witness.untold |= sent.is_err() || refused;
                let fresh = if sent == Ok(true) && matches!(delivery.request, crate::witness_delivery::Request::Play(_)) {
                    self.witness.told += 1;
                    self.map_boards_fresh.clear();
                    self.community_task(true)
                } else {
                    Task::none()
                };
                Task::batch([fresh, self.delivery_task()])
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
                let pools = self.pools_work(Vec::new());
                Task::batch([pictures, pools, self.share_task()])
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
            Message::MapBoard(beatmap, fresh, said) => {
                self.map_boards_waiting.remove(&beatmap);
                match said {
                    Ok(board) => {
                        self.map_boards_failed.remove(&beatmap);
                        let faces = self.pictures_task(board.faces().into_iter().map(|face| (face, 128)).collect());
                        self.map_boards.insert(beatmap, board);
                        if fresh {
                            self.map_boards_fresh.insert(beatmap, Instant::now());
                            faces
                        } else {
                            Task::batch([faces, self.map_board_task(beatmap, true)])
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
                let before = self.feed_before();
                let saved = said.clone();
                let save = ui::in_thread(move || { crate::community::wire::save(&saved); Message::Nudged });
                self.now_unix = unix_now();
                let mut fresh = crate::community::Catalog::from_wire(said);
                if self.settings.people_everyone {
                    fresh.welcome(&self.everyone);
                } else {
                    let users: Vec<crate::community::wire::Person> = self.everyone.iter().filter(|said| said.app).cloned().collect();
                    fresh.welcome(&users);
                    fresh.marked(&self.everyone);
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
                if self.refresh_asked {
                    self.refresh_asked = false;
                    self.refresh_answered = Some(Instant::now());
                }
                self.notice_fade.go_mut(false, Instant::now());
                let card = self.card_task(false);
                let friends = self.friends_task(false);
                Task::batch([self.community_pictures_task(), friends, card, save, self.everyone_task(), self.resolve_witness(), self.worn_task()])
            }
            Message::EveryoneArrived(Ok(said)) => {
                self.everyone_tries = 0;
                self.everyone = said.people;
                self.welcome_everyone();
                self.pool_players();
                Task::batch([self.community_pictures_task(), self.worn_task()])
            }
            Message::WornArrived(worn) => {
                self.worn.extend(worn);
                Task::none()
            }
            Message::EveryoneArrived(Err(_)) => {
                if self.everyone_tries >= EVERYONE_TRIES {
                    return Task::none();
                }
                self.everyone_tries += 1;
                ui::in_thread(|| {
                    std::thread::sleep(EVERYONE_AGAIN);
                    Message::EveryoneAgain
                })
            }
            Message::EveryoneAgain => self.everyone_task(),
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
            Message::CommunityArrived(Err(refused)) => {
                use crate::community_screen::{Fault, Fetch};
                let was = match self.community_fetch {
                    Fetch::Fresh(at) => Some(at),
                    Fetch::Failed(was, _) => was,
                    _ => None,
                };
                let fault = Fault::of(&refused);
                self.community_fetch = Fetch::Failed(was, fault);
                self.refresh_asked = false;
                self.notice_kept = Some((was, fault));
                self.notice_fade.go_mut(true, Instant::now());
                Task::none()
            }
            Message::CardArrived(Ok(mut card)) => {
                card.remember_country_rank(self.community_card.as_ref(), unix_now());
                crate::community::wire::save_card(&card);
                self.community_card = Some(card);
                self.remerge();
                let pictures = self.community_pictures_task();
                Task::batch([pictures, self.pools_work(Vec::new())])
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
                    let before = self.feed_before();
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
                self.fetch_map(hash)
            }
            Message::AutoFetch => {
                if !self.settings.auto_fetch_maps || !self.fetch_room() {
                    return Task::none();
                }
                if !self.missed_read && !self.gallery {
                    self.missed_read = true;
                    self.missed = maps::Missed::load(&maps::Missed::file());
                }
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_secs() as i64);
                let wanted = self.entries().iter().find(|entry| entry.map.is_none() && !entry.map_hash.is_empty() && !self.auto_fetched.contains(&entry.map_hash) && !self.missed.rests(&entry.map_hash, now) && self.fetch_of(&entry.map_hash).is_none()).map(|entry| entry.map_hash.clone());
                match wanted {
                    Some(hash) => {
                        self.auto_fetched.insert(hash.clone());
                        self.fetch_map(hash)
                    }
                    None => Task::none(),
                }
            }
            Message::FetchMap(hash) => {
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
                if !self.gallery {
                    if !self.missed_read {
                        self.missed_read = true;
                        self.missed = maps::Missed::load(&maps::Missed::file());
                    }
                    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_secs() as i64);
                    let changed = match &step {
                        maps::Step::Done(_) => self.missed.forget(&hash),
                        other => self.missed.note(&hash, other, now),
                    };
                    if changed {
                        self.missed.save(&maps::Missed::file());
                    }
                }
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
                    live::Frame::Ended => {}
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
                let pool = path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("pool"));
                let chart = path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("osu") || ext.eq_ignore_ascii_case("osz"));
                let in_pools = self.overlay == Overlay::Community && self.community_section == crate::community_screen::Section::Pools;
                if pool || chart {
                    let mut tasks = Vec::new();
                    if !in_pools {
                        tasks.push(self.update(Message::Show(Overlay::Community)));
                        tasks.push(self.update(Message::Community(crate::community_screen::Message::Go(crate::community_screen::Section::Pools, None))));
                    }
                    tasks.push(self.update(Message::Community(crate::community_screen::Message::Pools(crate::pools_screen::Message::Dropped(path)))));
                    return Task::batch(tasks);
                }
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
                self.search_fade.go_mut(self.search_shown(), now);
                self.filters_fade.go_mut(self.filters_open, now);
                let rest = self.check_rest(now);
                let dt = now.saturating_duration_since(self.now).as_secs_f32().min(1.0 / 30.0);
                if self.scenes_due {
                    self.scenes_due = false;
                    let load = self.scenes_for_notices();
                    self.now = now;
                    return Task::batch([rest, load.chain(self.update(Message::Tick(now)))]);
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
                self.spring_carousel(now);
                if let Some(live) = &self.live {
                    if !(live.control.paused() && live.control.settled()) {
                        live.control.request();
                    }
                }
                if self.resting.value() || self.rest_waiting || self.resting.is_animating(now) {
                    if let Some((_, live)) = &self.reel.now { live.control.request(); }
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
                    if toast.shown.value() && !toast.hovered && toast.age(now) > if toast.stays { TOAST_STAY_BAD } else { TOAST_STAY } {
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
                self.billing.settle(now);
                if self.error_shown.is_some() && !self.error_fade.value() && !self.error_fade.is_animating(now) {
                    self.error_shown = None;
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
                self.feed_leaving.settle(now);
                self.thumb_pictures.settle(now);
                if !self.read_fade.value() && !self.read_fade.is_animating(now) {
                    self.community_reading = None;
                }
                let banners = self.banners_task(now);
                Task::batch([rest, self.combo_for_rested(now), banners])
            }
            Message::Banner(hash, handle) => {
                if let Some(handle) = handle {
                    if self.banners.len() >= select::BANNERS_KEPT {
                        let keep: std::collections::HashSet<String> = self.near_banners().into_iter().map(|(hash, _)| hash).collect();
                        self.banners.retain(|known, _| keep.contains(known));
                        self.banners_asked.retain(|known| keep.contains(known));
                        self.cover_frames.borrow_mut().retain(|known, _| keep.contains(known));
                    }
                    self.cover_frames.borrow_mut().remove(&hash);
                    self.banners.insert(hash, handle);
                }
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
        }
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

    fn witness_ask_layer(&self) -> Element<'_, Message> {
        if !self.witness_asking {
            return Space::new().into();
        }
        let w = &self.words;
        let note = |key: &str| text(w.t(key)).font(theme::SANS).size(13.0).wrapping(text::Wrapping::WordOrGlyph).color(MUTED);
        let body = column![
            text(w.t("witness-ask-title")).font(theme::SANS_SEMI).size(20.0).wrapping(text::Wrapping::WordOrGlyph).color(INK),
            note("witness-ask-what"),
            note("witness-ask-support"),
            note("witness-ask-yours"),
            row![ui::quiet(w.t("witness-ask-no"), Some(Message::Prefs(prefs::Message::WitnessDecline))), container(ui::primary(w.t("witness-ask-yes"), Some(Message::Prefs(prefs::Message::WitnessAgree)))).id(iced::widget::Id::new("witness-ask-confirm"))].spacing(8),
        ].spacing(14);
        let card = mouse_area(container(body).padding(24).width(480).style(theme::stage)).on_press(Message::Prefs(prefs::Message::WitnessAskTap));
        let veil = mouse_area(ui::veil(theme::DEEP_SCRIM)).on_press(Message::Prefs(prefs::Message::WitnessDecline));
        stack![veil, container(card).width(Length::Fill).height(Length::Fill).center(Length::Fill)].into()
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



    pub fn view(&self) -> Element<'_, Message> {
        if self.minimized {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        }
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
        let body: Element<'_, Message> = match (loaded, self.reading_line()) {
            (true, _) if buried => blank(),
            (true, _) if awake > 0.001 => ui::fading(k * awake, || self.body(k, s)),
            (true, _) => blank(),
            (false, Some(line)) => pin(ui::fading(self.arrive.interpolate(0.0, 1.0, self.now), || ui::mono_small(line, FAINT))).x(40.0).y((self.height - 40.0).max(0.0)).into(),
            (false, None) => blank(),
        };
        let early = self.arrive.interpolate(0.0, 1.0, self.now) * (1.0 - self.cinema.interpolate(0.0, 1.0, self.now)) * awake;
        let crest = ui::fading(early, ui::brand);
        let crest: Element<'_, Message> = pin(crest).x(CREST_HOME.0).y(CREST_HOME.1 + (1.0 - early) * CREST_RISE).into();
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
            ui::fading(deep, || ui::veil(theme::GROUND_VEIL))
        } else {
            blank()
        };
        let overlay: Element<'_, Message> = if showing { ui::fading(sheet, || self.overlay_view()) } else { blank() };
        let bubble = blank();
        let person = if self.overlay_drawn == Overlay::Community && showing {
            ui::fading(sheet * awake, || self.community_view(true).unwrap_or_else(blank))
        } else { blank() };
        let toasts = self.toast_layer();
        let in_duels = self.overlay == Overlay::Community && self.community_section == crate::community_screen::Section::Duels;
        let wrap = |message: crate::duels_screen::Message| Message::Community(crate::community_screen::Message::Duels(message));
        let duel_strip: Element<'_, Message> = if in_duels { Space::new().into() } else {
            crate::duel_view::away_strip(&self.duels, &self.words, self.now_unix).map(|strip| strip.map(wrap)).unwrap_or_else(|| Space::new().into())
        };
        let duel_leaving: Element<'_, Message> = if self.duel_leaving { crate::duel_view::leaving(&self.words).map(wrap) } else { Space::new().into() };
        let menu = self.menu_layer();
        let signing = self.sign_in_layer();
        let billing = self.billing_layer();
        let sharing = self.share_layer();
        let failure = self.error_layer();
        let ask: Element<'_, Message> = if self.asking_delete || self.ask_fade.is_animating(self.now) {
            self.delete_card()
        } else {
            blank()
        };
        let room = self.skin_room_layer();
        let skin_ask = self.skin_delete_layer();
        let witness_ask = self.witness_ask_layer();
        let share_menu: Element<'_, Message> = if loaded && self.overlay == Overlay::None { self.share_layer_menu() } else { blank() };
        let resting: Element<'_, Message> = if rest > 0.001 || self.resting.value() {
            let mark = container(ui::fading(rest, || ui::brand_scaled(REST_BRAND_SCALE))).id(iced::widget::Id::new("rest-mark"));
            let mark = container(mark).width(Length::Fill).height(Length::Fill).padding(28).align_y(iced::alignment::Vertical::Bottom);
            let shield = iced::widget::opaque(mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
                .on_press(Message::UserInput(None)).on_right_press(Message::UserInput(None))
                .on_middle_press(Message::UserInput(None)).on_scroll(|_| Message::UserInput(None))
                .interaction(iced::mouse::Interaction::Idle));
            stack![shield, mark].into()
        } else { blank() };
        let mini = self.mini_player_layer();
        let rest_scene = if loaded { self.reel_view(rest) } else { blank() };
        let layers = stack![scene_before, scene, body, share_menu, bubble, ground, overlay, chrome_layer, crest, mini, person, room, ask, sharing, duel_strip, toasts, menu, billing, signing, skin_ask, witness_ask, duel_leaving, failure, rest_scene, resting];
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
            .filter(|(entry, live)| live.for_path == entry.path && (live.frame.is_some() || self.live_before.is_some()) && self.overlay == Overlay::None) {
            let seen = if self.live_before.is_some() { 1.0 } else { live.fade.interpolate(0.0, 1.0, self.now) };
            surface = ui::mix(surface, ui::mix(iced::Color::WHITE, theme::GROUND, 0.66), seen);
        }
        if self.entries().is_empty() {
            return column![
                Space::new().height(theme::CONTROL_HEIGHT + 4.0 + 22.0),
                Space::new().height(Length::Fill),
                ui::on_surface(surface, || self.viewer(s)),
            ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        }
        ui::on_surface(surface, || self.select_body(k, s))
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
        let words = row![nav, self.bell_button(), self.circle(CIRCLE_SIDE, true)].spacing(22).align_y(iced::Center);
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
        let badges: Vec<&String> = badges.iter().filter(|acronym| crate::modicons::shown(acronym)).collect();
        for acronym in &badges {
            meta = meta.push(ui::fading(ui::fade() * badge_alpha, || mod_badge(acronym)));
        }
        if !badges.is_empty() {
            meta = meta.push(Space::new().width(8));
        }
        meta = meta.push(ui::mono(retype(&was.meta, &now.meta), MUTED));
        let action: Element<'_, Message> = if with_actions { self.action(entry) } else { Space::new().height(theme::CONTROL_HEIGHT).into() };
        let left = column![
            row![text(date).font(theme::MONO).size(theme::CAPTION).color(ui::faded(MUTED)), text(from).font(theme::MONO).size(theme::CAPTION).color(ui::faded(theme::SHARED))].spacing(14),
            ui::moving_text(player, theme::SANS_SEMI, 30.0, INK),
            ui::moving_text(map, theme::SANS, theme::BODY, MUTED),
            container(meta).padding(Padding::ZERO.top(4.0)),
            container(action).padding(Padding::ZERO.top(14.0)),
        ]
        .spacing(2).width(Length::Fill);
        let outcome_colour = if entry.outcome.is_bad() { ACCENT } else { MUTED };
        let right = column![
            text(accuracy).font(theme::SANS_SEMI).size(48.0).wrapping(text::Wrapping::None).color(ui::faded(INK)),
            container(text(retype(&was.outcome, &now.outcome)).font(theme::MONO_BOLD).size(theme::CAPTION).color(ui::faded(outcome_colour)))
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(2)
        .align_x(iced::alignment::Horizontal::Right);
        let _ = w;
        row![left, container(right).width(220.0)].spacing(20).align_y(iced::alignment::Vertical::Bottom).into()
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

    fn fetch_map(&mut self, hash: String) -> Task<Message> {
        self.update(Message::FetchMap(hash))
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

    fn sorting(&mut self) {
        self.flip_from = ui::places();
        self.flip_at = Some(Instant::now());
    }

    fn shared_avatar(&self, player: &str) -> Option<&str> {
        let wanted = player.to_lowercase();
        let known = self.community.as_ref().and_then(|catalog| catalog.people.iter().find(|person| person.name.to_lowercase() == wanted).map(|person| person.avatar.as_str()));
        known.filter(|avatar| !avatar.is_empty())
            .or_else(|| self.everyone.iter().find(|person| person.name.to_lowercase() == wanted).map(|person| person.avatar.as_str()).filter(|avatar| !avatar.is_empty()))
            .or_else(|| self.people_cards.get(&wanted).map(|card| card.avatar_url.as_str()).filter(|avatar| !avatar.is_empty()))
    }

    fn shared_faces_task(&mut self) -> Task<Message> {
        let everyone = self.group == crate::carousel::Group::Players;
        let mut players: Vec<String> = self.entries().iter().filter(|entry| everyone || crate::mixed::is_shared(&entry.path)).map(|entry| entry.player.to_lowercase()).collect();
        players.sort_unstable();
        players.dedup();
        let wanted = players.iter().filter_map(|player| self.shared_avatar(player).map(|url| (url.to_owned(), 128))).collect();
        let pictures = self.pictures_task(wanted);
        let missing: Vec<_> = players.into_iter().filter(|player| self.shared_avatar(player).is_none() && self.people_asked.insert(format!("card:{player}"))).collect();
        if missing.is_empty() { return pictures; }
        Task::batch([pictures, ui::streamed(move |push| {
            for player in missing {
                let card = crate::osu_profile::load_player(&player).filter(|card| !card.avatar_url.is_empty()).map(Ok).unwrap_or_else(|| crate::osu_profile::fetch(&player));
                if !push(Message::PersonCard(player, card)) { return; }
            }
        })])
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

    fn feed_before(&self) -> crate::chronicle::Before {
        let frozen = (self.overlay == Overlay::Community && self.community_section == crate::community_screen::Section::Feed)
            .then(|| self.community.as_ref().map(|catalog| crate::chronicle::Frozen::of(catalog, &self.news, &self.settings.news_channels, self.live_shown)))
            .flatten();
        crate::chronicle::Before { keys: self.feed_keys(), frozen }
    }

    fn fresh_from(&mut self, before: crate::chronicle::Before) {
        let now = Instant::now();
        let keys = self.feed_keys();
        let known: std::collections::HashSet<&String> = before.keys.iter().collect();
        if self.feed_offset > crate::chronicle::READING_BELOW && self.community_section == crate::community_screen::Section::Feed {
            let arrived: Vec<String> = keys.iter().filter(|key| !known.contains(key)).cloned().collect();
            self.feed_held.extend(arrived);
            let shown: Vec<String> = keys.iter().filter(|key| !self.feed_held.contains(*key)).cloned().collect();
            self.feed_arrivals.refresh(&before.keys, &shown, now);
        } else {
            self.feed_arrivals.refresh(&before.keys, &keys, now);
        }
        self.feed_leaving.start(before, &keys, &self.feed_held, now);
        let present: std::collections::HashSet<&String> = keys.iter().collect();
        self.feed_held.retain(|key| present.contains(key));
    }

    fn release_held(&mut self) {
        if self.feed_held.is_empty() {
            return;
        }
        let keys = self.feed_keys();
        let shown: Vec<String> = keys.iter().filter(|key| !self.feed_held.contains(*key)).cloned().collect();
        self.feed_held.clear();
        self.feed_arrivals.refresh(&shown, &keys, Instant::now());
    }

    fn news_heard(&mut self, source: &str) {
        self.now_unix = unix_now();
        self.news_failed.remove(source);
        self.news.mark(source, unix_now());
        self.news.save();
    }

    fn fetch_channel(&mut self, channel: String) -> Task<Message> {
        self.news_loading.insert(crate::news::channel_source(&channel));
        let (server, token, device) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        ui::in_thread(move || Message::NewsPosts(channel.clone(), crate::news::fetch_posts_through(&server, &token, &device, &channel)))
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
        let candidates: Vec<_> = stories.chain(posts).collect();
        for (url, size) in candidates {
            if self.ask_picture(&url, Instant::now()) {
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
        if staged || self.settings.token.is_empty() || self.map_boards_waiting.contains(&beatmap) || (fresh && self.map_boards_fresh.get(&beatmap).is_some_and(|at| at.elapsed() < BOARD_FRESH)) {
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
        if !matches!(self.community_fetch, crate::community_screen::Fetch::Fresh(_) | crate::community_screen::Fetch::Failed(..)) {
            self.community_fetch = crate::community_screen::Fetch::Loading;
        }
        let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        let chat = self.community_chat();
        ui::in_thread(move || Message::CommunityArrived(crate::bot::community(&server, &token, &name, chat)))
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
        self.pools.set_best(self.shown_card.as_ref().map(|card| card.top_scores.as_slice()));
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


    fn share_task(&mut self) -> Task<Message> {
        let staged = self.community.as_ref().is_none_or(|catalog| catalog.staged);
        let Some(card) = self.osu_card.clone().filter(|_| !staged && !self.settings.token.is_empty()) else {
            return Task::none();
        };
        let (server, token, device) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        ui::in_thread(move || Message::CardShared(crate::bot::share_card(&server, &token, &device, &card).map_err(|e| e.to_string())))
    }

    fn ask_picture(&mut self, url: &str, now: Instant) -> bool {
        if url.is_empty() || self.news_pictures.contains_key(url) { return false; }
        if let Some(failed) = self.news_picture_failures.get(url) {
            if now.saturating_duration_since(*failed) < Duration::from_secs(30) { return false; }
            self.news_picture_failures.remove(url);
            self.news_asked.remove(url);
        }
        self.news_asked.insert(url.to_owned())
    }

    fn pictures_task(&mut self, wanted: Vec<(String, u32)>) -> Task<Message> {
        let wanted: Vec<(String, u32)> = wanted.into_iter().filter(|(url, _)| self.ask_picture(url, Instant::now())).collect();
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
        let wanted: Vec<(String, u32)> = wanted.into_iter().filter(|(url, _)| self.ask_picture(url, Instant::now())).collect();
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
        let wanted: Vec<String> = urls.into_iter().filter(|url| self.ask_picture(&crate::community_screen::wide(url), Instant::now())).collect();
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
        let mut entries = vec![
            item("profile", Icon::Person, w.t("community-profile"), section == Section::Profile, Section::Profile, None),
            item("feed", Icon::News, w.t("community-feed"), section == Section::Feed, Section::Feed, None),
            item("people", Icon::Chat, w.t("community-people"), section == Section::People, Section::People, Some(PeopleFrom::Chat)),
            item("compare", Icon::Compare, w.t("community-compare"), section == Section::Compare, Section::Compare, None),
            Entry::Caption(w.t("community-play")),
            item("pools", Icon::Pool, w.t("community-pools"), section == Section::Pools, Section::Pools, None),
            item("duels", Icon::Swords, w.t("community-duels"), section == Section::Duels, Section::Duels, None),
        ];
        if !self.duels.roles.is_empty() {
            entries.push(item("balance", Icon::Scale, w.t("community-balance"), section == Section::Balance, Section::Balance, None));
        }
        entries.extend([
            Entry::Caption(w.t("community-standing")),
            item("boards", Icon::Chart, w.t("community-boards"), section == Section::Boards, Section::Boards, None),
            item("titles", Icon::Trophy, w.t("community-titles"), section == Section::Titles, Section::Titles, None),
        ]);
        entries
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
        let edge = (28.0 / (self.width - crate::sidebar::width_at(open)).max(28.0)).min(1.0);
        let dim = container(Space::new().width(Length::Fill).height(Length::Fill)).style(move |_| container::Style {
            background: Some(iced::Background::Gradient(iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2))
                .add_stop(0.0, Color { a: 0.0, ..theme::GROUND })
                .add_stop(edge, Color { a: SIDE_DIM * open, ..theme::GROUND })
                .add_stop(1.0, Color { a: SIDE_DIM * open, ..theme::GROUND }).into())),
            ..container::Style::default()
        });
        let dimmed = stack![body, dim].width(Length::Fill).height(Length::Fill);
        let workspace = column![Space::new().height(theme::CONTROL_HEIGHT + 4.0 + 22.0), row![side, dimmed].height(Length::Fill)].width(Length::Fill).height(Length::Fill);
        crate::glide::brim(workspace).header().into()
    }

    fn community_ground<'a>(&'a self, catalog: &'a crate::community::Catalog, person_only: bool) -> crate::community_screen::Ground<'a> {
        let leaving = self.feed_leaving.shown(self.now);
        let mut ground = self.ground_of(catalog, person_only, leaving);
        if self.community_section == crate::community_screen::Section::Pools {
            ground.pools_clocks = self.pools_marks.clocks(self.now);
            ground.pools_clocks.today = self.now_unix;
            ground.pools_clocks.high = (self.height - 62.0) / COMMUNITY_SCALE;
        }
        if let Some(leave) = leaving {
            ground.ghost = Some(Box::new(self.ground_of(&leave.frozen.catalog, person_only, None)));
        }
        ground
    }

    fn ground_of<'a>(&'a self, catalog: &'a crate::community::Catalog, person_only: bool, leaving: Option<crate::chronicle::Leave<'a>>) -> crate::community_screen::Ground<'a> {
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
            pools: &self.pools,
            duels: &self.duels,
            desk: &self.desk,
            pools_clocks: crate::pools_screen::Clocks::settled(),
            asked: self.refresh_asked,
            answered_t: self.refresh_answered.map_or(f32::MAX, |at| self.now.saturating_duration_since(at).as_secs_f32()),
            notice: self.notice_kept.filter(|_| self.notice_fade.value() || self.notice_fade.is_animating(self.now)),
            notice_k: self.notice_fade.interpolate(0.0, 1.0, self.now),
            width: if person_only { self.width } else { self.width - crate::sidebar::NARROW } / COMMUNITY_SCALE,
            filter: self.feed_filter,
            query: &self.feed_query,
            open_events: &self.feed_open,
            folds,
            panel_from: self.panel_from,
            section_t: self.now.saturating_duration_since(self.section_at).as_secs_f32().min(60.0),
            shift_t: self.now.saturating_duration_since(self.shift_at).as_secs_f32().min(60.0),
            stream_t: self.now.saturating_duration_since(self.stream_at).as_secs_f32().min(60.0),
            person_t: self.now.saturating_duration_since(self.person_at).as_secs_f32().min(60.0),
            metric: self.dossier_metric,
            span: self.dossier_span,
            grade_hover: self.grade_hover,
            boards: &self.map_boards,
            boards_waiting: &self.map_boards_waiting,
            boards_failed: &self.map_boards_failed,
            score_pick: self.score_pick,
            clips_loading: &self.clips_loading,
            reading: self.community_reading.as_ref(),
            read_above: self.read_over_person,
            read_k: self.read_fade.interpolate(0.0, 1.0, self.now),
            people_from: self.people_from,
            everyone: self.settings.people_everyone,
            worn: &self.worn,
            pool: &self.compare_pool,
            compare: &self.compare,
            compare_query: &self.compare_query,
            people_query: &self.people_query,
            people_filter: self.people_filter,
            title_scope: self.title_scope,
            people_order: self.people_order,
            people_orders_open: self.people_orders_open,
            standing: self.community_standing,
            card: self.shown_card.as_ref(),
            flags: &self.flags,
            avatar: self.avatar.as_ref(),
            chat: self.chat_name(),
            live_shown: self.live_shown,
            held: &self.feed_held,
            leaving,
            ghost: None,
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
            badge: self.billing.badge(),
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
            link: self.link_shown.clone().unwrap_or_else(crate::bot::link_state),
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
                self.witness_shown();
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
            P::AutoFetchMaps(on) => {
                self.remember_mark("auto-fetch-maps", on);
                self.settings.auto_fetch_maps = on;
                keep(&self.settings);
                Task::none()
            }
            P::ShowBadge(on) => {
                self.remember_mark("show-badge", on);
                self.billing_step(crate::billing::Message::Wear(Some(on), None, None))
            }
            P::ShowTitle(on) => {
                self.remember_mark("show-title", on);
                self.billing_step(crate::billing::Message::Wear(None, Some(on), None))
            }
            P::WearTitle(offer) => self.billing_step(crate::billing::Message::Wear(None, None, Some(offer))),
            P::PoolHints(on) => {
                self.remember_mark("pool-hints", on);
                let guide = if on { crate::pools::Guide::default() } else { crate::pools::Guide { on: false, ..self.settings.pool_guide } };
                self.settings.pool_guide = guide;
                self.pools.guide = guide;
                keep(&self.settings);
                Task::none()
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
            P::WitnessRun(true) => {
                self.witness_asking = true;
                Task::none()
            }
            P::WitnessRun(false) => {
                self.remember_mark("witness-run", false);
                self.witness_asking = false;
                self.settings.witness_on = false;
                keep(&self.settings);
                if let Some(control) = self.witness_control.take() {
                    control.stop();
                }
                self.witness.status = crate::witness::Status::Off;
                self.witness.state = None;
                self.witness.playing = None;
                Task::none()
            }
            P::WitnessAgree => {
                if !self.witness_asking {
                    return Task::none();
                }
                self.witness_asking = false;
                self.remember_mark("witness-run", true);
                self.settings.witness_on = true;
                keep(&self.settings);
                self.witness_task()
            }
            P::WitnessDecline => {
                self.witness_asking = false;
                Task::none()
            }
            P::WitnessAskTap => Task::none(),
            P::Witness(on) => {
                self.remember_mark("witness", on);
                self.settings.witness_keep = on;
                if on {
                    self.witness_source();
                }
                keep(&self.settings);
                self.witness_shown();
                Task::none()
            }
            P::WitnessOverlay(on) => {
                self.remember_mark("witness-overlay", on);
                self.settings.witness_overlay = on;
                keep(&self.settings);
                self.witness_shown();
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
const BILLING_WIDE: f32 = 1500.0;
const BILLING_SHEET: f32 = 1060.0;
const BILLING_GROWTH: f32 = 1.3;
const MENU_W: f32 = 420.0;
const BELL_W: f32 = 432.0;
const BELL_BADGE: f32 = 18.0;
const CLEAR_LEAVE: Duration = Duration::from_millis(180);
const CLEAR_STEP: Duration = Duration::from_millis(28);
const CLEAR_STEPS: usize = 8;
const BELL_FLASH: Duration = Duration::from_millis(900);
const MENU_TOP: f32 = 80.0;
const BADGE: f32 = 18.0;
const BADGE_OUT: f32 = 4.0;

impl Main {
    fn circle(&self, side: f32, pressable: bool) -> Element<'_, Message> {
        let ring = 0.0;
        let _ = pressable;
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

    pub(crate) fn glass_picture(&self) -> Option<image::Handle> {
        if self.overlay != Overlay::None || self.player.is_some() {
            return None;
        }
        self.chosen_entry().and_then(|entry| self.scenes.get(&entry.map_hash)).cloned()
    }


    fn menu_layer(&self) -> Element<'_, Message> {
        let Some(tab) = self.menu else {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        };
        let open = self.menu_open.interpolate(0.0, 1.0, self.now);
        let window = iced::Size::new(self.width, self.height);
        let (panel, wide, right) = match tab {
            Tab::Head => (self.menu_head(), MENU_W, 40.0),
            Tab::Bell => (self.bell_panel(), BELL_W, 40.0 + CIRCLE_SIDE + 22.0),
        };
        let whole = ui::fading(ui::fade() * (open * 2.5).min(1.0), || {
            let framed = iced::widget::opaque(ui::glass(container(panel).width(wide), self.glass_picture(), window));
            let icon = 22.0 + (theme::CONTROL_HEIGHT + 4.0) / 2.0;
            ui::grown(framed, Point::new(1.0 - (CIRCLE_SIDE / 2.0) / wide, 0.0), (1.0 - open) * (MENU_TOP - icon), 0.02 + 0.98 * open)
        });
        let x = (self.width - right - wide).max(16.0);
        let backdrop: Element<'_, Message> = if self.menu_open.value() {
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill)).on_press(Message::MenuClose).into()
        } else {
            Space::new().width(Length::Fill).height(Length::Fill).into()
        };
        stack![backdrop, pin(whole).x(x).y(MENU_TOP)].width(Length::Fill).height(Length::Fill).into()
    }

    fn tile<'a>(&'a self, inside: Element<'a, Message>) -> Element<'a, Message> {
        container(inside)
            .padding([12, 14])
            .width(Length::Fill)
            .style(ui::box_faded(|_| container::Style {
                background: Some(iced::Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.24))),
                border: iced::Border { color: crate::theme::outline(Color::from_rgba(1.0, 1.0, 1.0, 0.05)), width: 1.0, radius: 12.0.into() },
                ..container::Style::default()
            }))
            .into()
    }







    fn background_for(&self, map_hash: &str) -> Option<PathBuf> {
        self.entries()
            .iter()
            .find(|e| e.map_hash == map_hash)
            .and_then(|e| e.map.as_ref().and_then(|m| m.background.clone()))
            .or_else(|| self.store.videos.iter().find(|v| v.map_hash == map_hash).and_then(|v| v.background.clone()))
    }


    fn card<'a>(&'a self, inside: Element<'a, Message>, bad: bool) -> Element<'a, Message> {
        container(inside)
            .padding([12, 14])
            .width(Length::Fill)
            .style(ui::box_faded(move |theme| if bad { theme::tile_bad(theme) } else { theme::notification(false)(theme) }))
            .into()
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

    fn worn_task(&self) -> Task<Message> {
        if self.settings.token.is_empty() {
            return Task::none();
        }
        let mut players: Vec<i64> = self.community.iter().flat_map(|catalog| catalog.people.iter().filter_map(|person| person.player)).chain(self.everyone.iter().filter_map(|person| person.player)).filter(|player| !self.worn.contains_key(player)).collect();
        players.sort_unstable();
        players.dedup();
        if players.is_empty() {
            return Task::none();
        }
        let (server, token, name) = (self.settings.server.clone(), self.settings.token.clone(), self.settings.device.clone());
        ui::in_thread(move || Message::WornArrived(crate::bot::badges(&server, &token, &name, &players).unwrap_or_default()))
    }

    fn everyone_task(&self) -> Task<Message> {
        let staged = self.community.as_ref().is_none_or(|catalog| catalog.staged);
        if self.settings.token.is_empty() || staged {
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
            let users: Vec<crate::community::wire::Person> = self.everyone.iter().filter(|said| said.app).cloned().collect();
            catalog.farewell();
            catalog.welcome(&users);
            catalog.marked(&self.everyone);
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
pub fn latin(letter: &str) -> Option<char> {
    let sign = letter.chars().next()?.to_lowercase().next()?;
    Some(match sign {
        'я' => 'z',
        'л' => 'k',
        'т' => 'n',
        'с' => 'c',
        'в' => 'd',
        'к' => 'r',
        'м' => 'v',
        other => other,
    })
}

pub const TOAST_STAY: Duration = Duration::from_secs(6);
pub const TOAST_STAY_BAD: Duration = Duration::from_secs(12);
const EVERYONE_TRIES: u8 = 4;
const EVERYONE_AGAIN: Duration = Duration::from_secs(20);
const BELL_PULSE: f32 = 1.6;
const TOASTS_AT_MOST: usize = 3;


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
const FLIP: Duration = Duration::from_millis(320);
const MINIMIZED_POLL: Duration = Duration::from_secs(15);
const AUTO_EVERY: Duration = Duration::from_secs(20);
const AUTO_IDLE: Duration = Duration::from_secs(12);
const COVERS_AT_ONCE: usize = 240;

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
    mod_badge_sized(acronym, 22.0)
}

pub fn mod_badge_sized<'a, Message: 'a>(acronym: &str, size: f32) -> Element<'a, Message> {
    let alpha = ui::fade();
    ui::hover(crate::modicons::badge(acronym, size, 1.0), ui::Glow::tile(size / 2.0).edge(Color { a: 0.35 * alpha, ..theme::ON_ACCENT }).lift(1.0).scale(1.06))
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
    if replay.mode == dossier_replay::GameMode::Mania {
        return dossier_produce::render::mania_max_combo(&beatmap, &replay);
    }
    Some(dossier_assay::max_combo(&beatmap, replay.mods))
}

fn paste_shortcut(key: &iced::keyboard::Key, physical_key: iced::keyboard::key::Physical, modifiers: iced::keyboard::Modifiers) -> bool {
    use iced::keyboard::key::{Code, Key, Physical};
    (modifiers.command() || modifiers.control()) && !modifiers.alt()
        && (physical_key == Physical::Code(Code::KeyV) || matches!(key.as_ref(), Key::Character(letter) if letter.eq_ignore_ascii_case("v")))
}

#[cfg(test)]
mod tests;
