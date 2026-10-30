mod persistence;
mod history;
mod transfers;
mod update;

mod catalogue;
use catalogue::*;
pub use catalogue::{backing_key};
mod slots;
use slots::*;
mod publication;
use publication::*;
mod editor_view;
use editor_view::*;
pub use editor_view::{commands, slot_found, saved_words, pool_foot, profile_link, map_link, map_id};

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use iced::widget::{button, column, container, image, pick_list, row, scrollable, text, text_input, Space};
use iced::{Background, Border, Color, Element, Length, Padding};
use md5::{Digest, Md5};

use crate::community_screen as screen;
use crate::glyphs::{glyph, Icon};
use crate::lang::Words;
use crate::library::Map;
use crate::pool_collections::Collection;
use crate::pool_links::{self, Target, Why};
use crate::pool_share;
use crate::pools::{self, Frame, Measure, Measures, Mod, Pool, Skill, Slot};
use crate::theme::{self, ACCENT, FAINT, INK, MUTED};
use crate::ui;

const ROW_HIGH: f32 = 56.0;
const COVER_WIDE: f32 = 72.0;
const COVER_HIGH: f32 = 40.0;
const COVER_ROUND: f32 = 8.0;
const TILE_ROUND: f32 = 12.0;
const HERO_ROUND: f32 = 14.0;
const THUMB_ROUND: f32 = 8.0;
const PANEL_WIDE: f32 = 600.0;
const OPEN_WIDE: f32 = 400.0;
const RESULTS_MOST: usize = 40;
const COLLECTION_PAGE: usize = 80;
const SCROLL_ID: &str = "pools-scroll";
const BACKDROP_HIGH: f32 = 420.0;
const CONTROL: f32 = theme::CONTROL_HEIGHT - 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Closed,
    Slot,
    Add,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceTab {
    Search,
    Collections,
    Best,
    Suggest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ail {
    Well,
    Missing,
    Fetching,
    Unmeasured,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keep {
    Theirs,
    Mine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    Empty,
    Links,
    Collections,
    Best,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bulk {
    Mod,
    Shift,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Editor {
    pub id: String,
    pub selected: Option<usize>,
    pub panel: Panel,
    pub query: String,
    pub source: SourceTab,
    pub collection: Option<usize>,
    pub collection_page: usize,
    pub game_collection: Option<String>,
    pub untouched: bool,
    pub replace: bool,
    pub share: bool,
    pub choosing: bool,
    pub marked: Vec<usize>,
    pub bulk: Option<Bulk>,
    pub asking_delete: bool,
    pub grouped: bool,
    pub last_mod: Mod,
    pub authors: String,
    pub colour_draft: String,
    pub last_category: String,
    pub last_colour: Option<[u8; 3]>,
    pub category_creator: bool,
    pub category_draft: String,
    pub editing_authors: bool,
    pub author_draft: String,
    pub categories: bool,
    pub tint: [u8; 3],
    pub gone: Option<Box<Editor>>,
    pub starting: bool,
    pub publish: bool,
    pub asking_withdraw: bool,
    pub lean: Option<Skill>,
    pub wholesale: bool,
    pub spread: bool,
    pub find: String,
    pub menu: Option<usize>,
    pub palette: bool,
    pub command: String,
    pub only_empty: bool,
    pub history: bool,
    pub fetch_all: bool,
    pub fetch_tried: Vec<String>,
    pub backdrop: bool,
}

impl Editor {
    pub fn at(id: String) -> Editor {
        Editor { id, selected: None, panel: Panel::Closed, query: String::new(), source: SourceTab::Search, collection: None, collection_page: 0, game_collection: None, untouched: false, replace: false, share: false, choosing: false, marked: Vec::new(), bulk: None, asking_delete: false, grouped: true, last_mod: Mod::Nm, authors: String::new(), colour_draft: String::new(), last_category: String::new(), last_colour: None, category_creator: false, category_draft: String::new(), editing_authors: false, author_draft: String::new(), categories: false, tint: SWATCHES[0], gone: None, starting: false, publish: false, asking_withdraw: false, lean: None, wholesale: false, spread: false, find: String::new(), menu: None, palette: false, command: String::new(), only_empty: false, history: false, fetch_all: false, fetch_tried: Vec::new(), backdrop: false }
    }
}

#[derive(Debug, Clone)]
pub struct Opening {
    pub pool: Pool,
    pub queue: Vec<String>,
    pub total: usize,
    pub step: Option<crate::maps::Step>,
    pub stop: Arc<std::sync::atomic::AtomicBool>,
    pub running: bool,
    pub lost: usize,
}

#[derive(Debug, Clone)]
pub struct Pull {
    pub pool: String,
    pub queue: Vec<String>,
    pub total: usize,
    pub lost: usize,
    pub current: Option<String>,
    pub step: Option<crate::maps::Step>,
    pub stop: Arc<std::sync::atomic::AtomicBool>,
    pub over: Option<Result<usize, String>>,
}

impl Pull {
    pub fn running(&self) -> bool {
        self.current.is_some() || !self.queue.is_empty()
    }

    pub fn done(&self) -> usize {
        self.total.saturating_sub(self.queue.len() + usize::from(self.current.is_some()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hot {
    Add,
    Note,
    Replace,
    Double,
    Copy,
    Remove,
    Text,
    Cancel,
    Confirm,
}

pub const SWATCHES: [[u8; 3]; 10] = [[0x5e, 0xc2, 0xd0], [0xc9, 0x87, 0x3f], [0xe0, 0x48, 0x4c], [0x8c, 0x5a, 0xd0], [0xc0, 0x48, 0x7a], [0xa0, 0x5a, 0x5c], [0x7f, 0xb0, 0x69], [0xd9, 0xc0, 0x4a], [0x4a, 0x8f, 0xd9], [0x9a, 0xa0, 0xa6]];

pub const OFFER_SPAN: f64 = 0.2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shelf {
    #[default]
    Mine,
    Saved,
    Published,
}

#[derive(Debug, Clone)]
pub enum Screen {
    Shelf,
    Editor(Editor),
    Open(Opening),
}

impl PartialEq for Screen {
    fn eq(&self, other: &Screen) -> bool {
        match (self, other) {
            (Screen::Shelf, Screen::Shelf) => true,
            (Screen::Editor(a), Screen::Editor(b)) => a == b,
            (Screen::Open(a), Screen::Open(b)) => a.pool.id == b.pool.id,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    Already(usize),
    NotALink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Place {
    Slot(usize),
    End,
    Replace(usize),
}

#[derive(Debug, Clone)]
pub struct Candidate {
    pub found: pool_links::Found,
    pub choice: Option<usize>,
    pub place: Place,
    pub cover: Option<image::Handle>,
}

#[derive(Debug, Clone)]
pub struct Fetching {
    pub queue: Vec<String>,
    pub total: usize,
    pub step: Option<crate::maps::Step>,
    pub place: Option<Place>,
    pub stop: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Debug, Clone)]
pub struct Suggestions {
    pub request: u64,
    pub pool: String,
    pub fingerprint: String,
    pub slot: usize,
    pub mods: Mod,
    pub target: Option<f64>,
    pub maps: Option<Vec<(String, Measure)>>,
    pub offered: Option<Result<Vec<crate::bot::Offered>, String>>,
    pub stop: Arc<AtomicBool>,
}

#[derive(Debug, Clone)]
pub struct FileImport {
    pub request: u64,
    pub pool: String,
    pub fingerprint: String,
    pub place: Place,
}

#[derive(Debug, Clone)]
pub enum Finding {
    Asking,
    Found(Candidate),
    Fetching(Fetching),
    Refused(String),
    Missing,
    Silent(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotKey([u8; 16], usize);

fn slot_keys(pool: &Pool) -> Vec<SlotKey> {
    let mut seen = HashMap::new();
    pool.slots.iter().map(|slot| {
        let content = serde_json::to_vec(&(&slot.hash, slot.mods, &slot.category, slot.colour, &slot.note, &slot.artist, &slot.title, &slot.version, slot.set)).expect("slot identity");
        let digest: [u8; 16] = Md5::digest(content).into();
        let occurrence = seen.entry(digest).or_insert(0);
        let key = SlotKey(digest, *occurrence);
        *occurrence += 1;
        key
    }).collect()
}

#[cfg(test)]
fn slot_groups(pool: &Pool) -> Vec<(Mod, Vec<usize>)> {
    Mod::ALL.into_iter().filter_map(|mods| {
        let slots: Vec<usize> = pool.slots.iter().enumerate().filter_map(|(at, slot)| (slot.mods == mods).then_some(at)).collect();
        (!slots.is_empty()).then_some((mods, slots))
    }).collect()
}

#[derive(Debug, Clone, Copy)]
pub enum Input {
    Name,
    Query,
    Note(usize),
}

#[derive(Debug, Clone)]
pub enum Message {
    FlushSaves,
    RetrySaves,
    Catalogue(bool),
    Shelf(Shelf),
    SavePublication(String),
    Pull(String),
    Renew(String),
    StopPull,
    Collected(String, Result<usize, String>),
    Uncollected(String, Result<bool, String>),
    Makers(Option<String>),
    Begin(Start),
    PublishSheet(bool),
    Conflict(Keep),
    HideGuide,
    GuideNext,
    GuideGot,
    RetryCatalogue,
    Lean(Skill),
    ShowSlot(usize),
    AskWithdraw(bool),
    Withdraw,
    Withdrawn(String, Result<(), String>),
    CatalogueMore,
    CatalogueLoaded(bool, bool, Result<Vec<crate::bot::Publication>, String>),
    OpenPublication(String),
    ImportCollection(usize),
    Publish,
    Published(String, Result<crate::bot::Publication, String>),
    New,
    Open(String),
    Back,
    Rename(String),
    Authors(String),
    AuthorsDraft(String),
    ApplyAuthors,
    EditAuthors,
    AuthorDraft(String),
    AddAuthor,
    DropAuthor(String),
    Categories(bool),
    Tint(Option<String>, [u8; 3]),
    DropCategory(String),
    NewCategory,
    OwnCompiler(String, String),
    Face(String, Option<image::Handle>),
    MoveCategory(String, bool),
    Author(String),
    CreateCategory(bool),
    CategoryDraft(String),
    SaveCategory(usize),
    Category(usize, String),
    Colour(usize, String),
    UseFrame(Frame),
    Select(Option<usize>),
    Replace,
    ReplaceAt(usize),
    Grouped(bool),
    SetMod(usize, Mod),
    AddMod(Mod),
    Clear(usize),
    AddPanel(bool),
    Source(SourceTab),
    Collection(Option<usize>),
    CollectionPage(usize),
    CollectionHash(String),
    Collections(Vec<Collection>),
    Best(usize),
    RefreshBest,
    Suggest(usize),
    Suggested(u64, Vec<(String, Measure)>),
    Offered(u64, Result<Vec<crate::bot::Offered>, String>),
    TakeOffered(u64),
    Query(String),
    Put(String),
    Note(usize, String),
    Songs(Arc<HashMap<String, Map>>),
    Measured(String, Mod, Result<Measure, String>),
    Pasted(String),
    PasteInto(Input, String),
    PastedInto(String, Input, String, String),
    Resolved(u64, Result<pool_links::Found, Why>),
    Cover(u64, Option<image::Handle>),
    Choose(usize),
    Aim(Place),
    Confirm,
    ConfirmAll,
    Dismiss,
    FetchSlot(usize),
    Retry,
    Step(u64, String, crate::maps::Step),
    Share(bool),
    CopyText,
    CopyHash,
    SaveFile,
    SaveImage,
    ImageSaved(u64, Result<Option<PathBuf>, String>),
    OpenFile,
    Imported(Result<Pool, pool_share::Refused>),
    Dropped(PathBuf),
    MapFile(u64, Result<(String, Map), crate::pool_files::Refused>),
    ArchiveFile(u64, Result<Vec<(String, Map)>, crate::pool_files::Refused>),
    PoolFile(u64, Result<Pool, pool_share::Refused>),
    GetMissing,
    Keep,
    Saved(Result<Option<PathBuf>, String>),
    Choosing(bool),
    Mark(usize),
    Bulk(Option<Bulk>),
    BulkMod(Mod),
    Shift(bool),
    Move(String, Vec<SlotKey>, SlotKey, Option<SlotKey>),
    RemoveMarked,
    Undo,
    Redo,
    Find(String),
    Duplicate(usize),
    SlotMenu(Option<usize>),
    Palette(bool),
    Command(String),
    Chosen(Box<Message>),
    CopySlot(usize),
    CopySlotId(usize),
    SortStars,
    OnlyEmpty,
    FetchMissing,
    DropStray,
    BackdropPanel(bool),
    BackdropFrom(Option<String>),
    BackdropPick,
    BackdropPicked(Option<PathBuf>),
    BackdropDim(u8),
    BackdropBlur(bool),
    BackdropCard(bool),
    BackdropLoaded(String, Option<image::Handle>),
    History(bool),
    OpenVersion(u64),
    Hot(Hot),
    AskDelete(bool),
    DeletePool,
}

impl Message {
    pub fn edits(&self) -> bool {
        matches!(
            self,
            Message::Rename(_) | Message::Authors(_) | Message::AuthorsDraft(_) | Message::ApplyAuthors | Message::AuthorDraft(_) | Message::AddAuthor | Message::DropAuthor(_)
                | Message::MoveCategory(..) | Message::Category(..) | Message::Colour(..) | Message::SaveCategory(_) | Message::NewCategory | Message::DropCategory(_) | Message::Tint(Some(_), _)
                | Message::UseFrame(_) | Message::SetMod(..) | Message::AddMod(_) | Message::Clear(_) | Message::AddPanel(true) | Message::Put(_) | Message::Note(..) | Message::Pasted(_)
                | Message::Replace | Message::ReplaceAt(_) | Message::Confirm | Message::ConfirmAll | Message::Move(..) | Message::RemoveMarked | Message::BulkMod(_) | Message::Shift(_)
                | Message::Duplicate(_) | Message::SortStars | Message::Suggest(_) | Message::Publish | Message::PublishSheet(true) | Message::Undo | Message::Redo
                | Message::BackdropFrom(_) | Message::BackdropPick | Message::BackdropDim(_) | Message::BackdropBlur(_) | Message::BackdropCard(_)
                | Message::Hot(Hot::Add | Hot::Replace | Hot::Double | Hot::Remove)
        )
    }
}

#[derive(Debug, Clone)]
pub enum Effect {
    Faces(Vec<String>),
    OwnCompiler(String),
    Catalogue(bool, usize),
    Publish(Pool),
    Withdraw(Pool),
    Author(String),
    ReadSongs,
    ReadCollections,
    Collect(String, String, Vec<String>),
    Uncollect(String, String),
    PickBackdrop,
    LoadBackdrop(String, PathBuf, bool),
    ReadBest(bool),
    ReadPaste(String, Input, String),
    Suggest(u64, Arc<HashMap<String, Map>>, Mod, f64, HashSet<String>, Arc<AtomicBool>),
    SuggestRemote(u64, Mod, f64, f64),
    Measure(String, Map, Mod),
    Resolve(u64, Target),
    Cover(String, u64),
    Fetch(u64, String, Arc<std::sync::atomic::AtomicBool>),
    Copy(String, &'static str),
    SaveFile(String, Vec<u8>),
    SaveImage(u64, Pool),
    PickFile,
    ReadPool(u64, PathBuf),
    ImportMap(u64, PathBuf),
    ImportArchive(u64, PathBuf),
    Say(&'static str),
    Guide(pools::Guide),
}

pub const SETTLED: f32 = 60.0;

#[derive(Debug, Clone)]
pub struct Clocks {
    pub page: f32,
    pub screen: f32,
    pub panel: f32,
    pub gone: f32,
    pub today: i64,
    pub high: f32,
    pub remote: HashMap<String, f32>,
    pub shut: Vec<(&'static str, f32)>,
    pub opened: Vec<(&'static str, f32)>,
}

fn leaving(shut: &[(&'static str, f32)], name: &str) -> Option<f32> {
    shut.iter().find(|(held, age)| *held == name && *age < SHEET_OUT).map(|(_, age)| 1.0 - age / SHEET_OUT)
}

impl Clocks {
    pub fn settled() -> Clocks {
        Clocks { page: SETTLED, screen: SETTLED, panel: SETTLED, gone: SETTLED, today: 0, high: 0.0, remote: HashMap::new(), shut: Vec::new(), opened: Vec::new() }
    }

    fn of(&self, id: &str) -> f32 {
        self.remote.get(id).copied().unwrap_or(SETTLED)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Marks {
    page: Option<(String, Instant)>,
    screen: Option<(String, Instant)>,
    panel: Option<(&'static str, Instant)>,
    gone: Option<Instant>,
    remote: HashMap<String, Instant>,
    began: bool,
    sheets: Option<[bool; 3]>,
    shut: Vec<(&'static str, Instant)>,
    opened: Vec<(&'static str, Instant)>,
}

const WINDOW: f32 = 1.8;
const SHEET_OUT: f32 = 0.16;
const SHEETS: [&str; 3] = ["categories", "backdrop", "share"];

impl Marks {
    pub fn observe(&mut self, state: &State, now: Instant) {
        let long_ago = now.checked_sub(Duration::from_secs(3600)).unwrap_or(now);
        let screen = state.screen_key();
        let screen_at = match &self.screen {
            Some((was, at)) if *was == screen => *at,
            Some(_) => now,
            None => long_ago,
        };
        self.screen = Some((screen, screen_at));
        let page = state.page_key();
        let page_at = match &self.page {
            Some((was, at)) if *was == page => *at,
            Some(_) => now,
            None => long_ago,
        };
        self.page = Some((page, page_at));
        let panel = state.panel_key();
        let panel_at = match self.panel {
            Some((was, at)) if was == panel => at,
            Some(_) => now,
            None => long_ago,
        };
        match self.panel {
            Some(("panel" | "publish", _)) if panel == "none" => self.gone = Some(now),
            Some((was, _)) if was != panel => self.gone = None,
            _ => {}
        }
        self.panel = Some((panel, panel_at));
        let sheets = state.sheets();
        if let Some(were) = self.sheets {
            for (at, name) in SHEETS.iter().enumerate() {
                if were[at] != sheets[at] {
                    self.shut.retain(|(held, _)| held != name);
                    self.opened.retain(|(held, _)| held != name);
                    if were[at] {
                        self.shut.push((name, now));
                    } else {
                        self.opened.push((name, now));
                    }
                }
            }
        }
        self.sheets = Some(sheets);
        for item in &state.publications {
            self.remote.entry(item.id.clone()).or_insert(if self.began { now } else { long_ago });
        }
        self.began = true;
    }

    pub fn clocks(&self, now: Instant) -> Clocks {
        let age = |at: Instant| now.saturating_duration_since(at).as_secs_f32().min(SETTLED);
        match (&self.screen, &self.panel) {
            (Some((_, screen)), Some((_, panel))) => Clocks { page: self.page.as_ref().map_or(SETTLED, |(_, at)| age(*at)), screen: age(*screen), panel: age(*panel), gone: self.gone.map_or(SETTLED, age), today: 0, high: 0.0, remote: self.remote.iter().map(|(id, at)| (id.clone(), age(*at))).collect(), shut: self.shut.iter().map(|(name, at)| (*name, age(*at))).collect(), opened: self.opened.iter().map(|(name, at)| (*name, age(*at))).collect() },
            _ => Clocks::settled(),
        }
    }

    pub fn animating(&self, now: Instant) -> bool {
        let young = |at: &Instant| now.saturating_duration_since(*at).as_secs_f32() < WINDOW;
        self.screen.iter().any(|(_, at)| young(at)) || self.panel.iter().any(|(_, at)| young(at)) || self.gone.iter().any(young) || self.remote.values().any(young) || self.shut.iter().any(|(_, at)| young(at)) || self.opened.iter().any(|(_, at)| young(at))
    }
}

#[derive(Clone)]
pub struct State {
    pub shift: bool,
    pub faces: HashMap<String, image::Handle>,
    pub makers: Option<String>,
    pub collection_shelf: bool,
    pub shelf: Shelf,
    pub pull: Option<Pull>,
    pub uncollect_refused: bool,
    pub delete_error: Option<String>,
    pub publications: Vec<crate::bot::Publication>,
    pub catalogue_loading: bool,
    pub catalogue_more: bool,
    pub publishing: bool,
    pub catalogue_error: Option<String>,
    pub list: Vec<Pool>,
    pub loaded: bool,
    pub screen: Screen,
    pub songs: Option<Arc<HashMap<String, Map>>>,
    pub reading: bool,
    pub collections: Option<Vec<Collection>>,
    pub collecting: bool,
    pub best: Option<Vec<crate::community::wire::Score>>,
    pub importing: Option<FileImport>,
    pub importing_pool: Option<u64>,
    pub exporting: Option<u64>,
    pub export_request: u64,
    pub dropped: VecDeque<PathBuf>,
    pub links: VecDeque<String>,
    pub guide: pools::Guide,
    pub conflict: Option<String>,
    pending: Vec<Effect>,
    pub resolving: Option<(String, Keep)>,
    pub linking: bool,
    clock: i64,
    depth: u8,
    pub file_request: u64,
    pub suggestions: Option<Suggestions>,
    pub pending_suggestion: Option<usize>,
    pub suggestion_request: u64,
    pub measures: Measures,
    pub asked: HashSet<(String, Mod)>,
    pub notice: Option<Notice>,
    pub finding: Option<Finding>,
    pub resolve_request: u64,
    pub fetch_request: u64,
    pub fetching: Option<(u64, String)>,
    pub fetched: HashMap<String, Map>,
    pub refused: Option<pool_share::Refused>,
    pub undo: Vec<(String, Pool)>,
    pub redo: Vec<(String, Pool)>,
    pub versions: Vec<Pool>,
    pub stray: Option<String>,
    pub backdrop: Option<(String, image::Handle)>,
    pub backdrop_refused: bool,
    pub dir: PathBuf,
    unsaved: HashMap<String, Instant>,
    save_errors: HashMap<String, String>,
}

impl State {
    pub fn screen_key(&self) -> String {
        match &self.screen {
            Screen::Shelf => format!("shelf:{}:{:?}", self.collection_shelf, self.shelf),
            Screen::Editor(editor) => format!("editor:{}", editor.id),
            Screen::Open(_) => "open".to_owned(),
        }
    }

    pub fn page_key(&self) -> String {
        match &self.screen {
            Screen::Shelf => "shelf".to_owned(),
            Screen::Editor(editor) => format!("editor:{}", editor.id),
            Screen::Open(_) => "open".to_owned(),
        }
    }

    pub fn sheets(&self) -> [bool; 3] {
        match &self.screen {
            Screen::Editor(editor) => [editor.categories, editor.backdrop, editor.share],
            _ => [false; 3],
        }
    }

    pub fn panel_key(&self) -> &'static str {
        match &self.screen {
            Screen::Editor(editor) if editor.share => "share",
            Screen::Editor(editor) if editor.publish => "publish",
            Screen::Editor(editor) if editor.choosing && !editor.marked.is_empty() => "bulk",
            Screen::Editor(editor) if !editor.choosing && (editor.panel == Panel::Add || (editor.panel == Panel::Slot && editor.selected.is_some())) => "panel",
            _ => "none",
        }
    }

    pub fn guided(mut self, guide: pools::Guide) -> State {
        self.guide = guide;
        self
    }

    pub fn new(dir: PathBuf) -> State {
        State {
            shift: false,
            faces: HashMap::new(),
            makers: None,
            collection_shelf: false,
            shelf: Shelf::Mine,
            pull: None,
            uncollect_refused: false,
            delete_error: None,
            publications: Vec::new(),
            catalogue_loading: false,
            catalogue_more: false,
            publishing: false,
            catalogue_error: None,
            list: Vec::new(),
            loaded: false,
            screen: Screen::Shelf,
            songs: None,
            reading: false,
            collections: None,
            collecting: false,
            best: None,
            importing: None,
            importing_pool: None,
            exporting: None,
            export_request: 0,
            dropped: VecDeque::new(),
            links: VecDeque::new(),
            guide: pools::Guide { on: false, ..pools::Guide::default() },
            conflict: None,
            pending: Vec::new(),
            resolving: None,
            linking: false,
            clock: 0,
            depth: 0,
            file_request: 0,
            suggestions: None,
            pending_suggestion: None,
            suggestion_request: 0,
            measures: Measures::default(),
            asked: HashSet::new(),
            notice: None,
            finding: None,
            resolve_request: 0,
            fetch_request: 0,
            fetching: None,
            fetched: HashMap::new(),
            refused: None,
            undo: Vec::new(),
            redo: Vec::new(),
            versions: Vec::new(),
            stray: None,
            backdrop: None,
            backdrop_refused: false,
            dir,
            unsaved: HashMap::new(),
            save_errors: HashMap::new(),
        }
    }

    pub fn open(&mut self) -> Vec<Effect> {
        self.cancel_network();
        self.cancel_import();
        if !self.loaded {
            self.list = pools::load_all(&self.dir);
            for pool in &mut self.list {
                if pool.calc != pools::CALC_VERSION {
                    pool.calc = pools::CALC_VERSION;
                    for slot in &mut pool.slots {
                        slot.measure = None;
                    }
                }
            }
            self.loaded = true;
        }
        self.screen = Screen::Shelf;
        let mut effects = Vec::new();
        if self.songs.is_none() && !self.reading && self.list.iter().any(|pool| pool.filled() > 0) {
            self.reading = true;
            effects.push(Effect::ReadSongs);
        }
        effects.extend(self.measure_effects());
        let names: Vec<_> = self.list.iter().flat_map(|pool| pool.authors.iter().chain(std::iter::once(&pool.compiler))).filter(|name| !name.is_empty() && !self.faces.contains_key(&name.to_lowercase())).cloned().collect();
        if !names.is_empty() { effects.push(Effect::Faces(names)); }
        if !self.catalogue_loading { self.catalogue_loading = true; effects.push(Effect::Catalogue(self.collection_shelf, 0)); }
        effects
    }

    pub fn cancel_next(&self) -> Option<Message> {
        let Screen::Editor(editor) = &self.screen else {
            return None;
        };
        if editor.menu.is_some() {
            Some(Message::SlotMenu(None))
        } else if editor.history {
            Some(Message::History(false))
        } else if editor.palette {
            Some(Message::Palette(false))
        } else if editor.share {
            Some(Message::Share(false))
        } else if editor.asking_withdraw {
            Some(Message::AskWithdraw(false))
        } else if editor.publish {
            Some(Message::PublishSheet(false))
        } else if editor.asking_delete {
            Some(Message::AskDelete(false))
        } else if self.finding.is_some() {
            Some(Message::Dismiss)
        } else if editor.categories {
            Some(Message::Categories(false))
        } else if editor.backdrop {
            Some(Message::BackdropPanel(false))
        } else if editor.panel != Panel::Closed || editor.selected.is_some() {
            Some(Message::Select(None))
        } else if editor.choosing {
            Some(Message::Choosing(false))
        } else if !editor.find.is_empty() {
            Some(Message::Find(String::new()))
        } else {
            None
        }
    }

    pub fn confirm_next(&self) -> Option<Message> {
        let Screen::Editor(editor) = &self.screen else {
            return None;
        };
        if editor.asking_delete {
            Some(Message::DeletePool)
        } else if editor.asking_withdraw {
            (!self.publishing).then_some(Message::Withdraw)
        } else if editor.publish {
            self.editing().filter(|pool| !self.publishing && publish_ready(pool)).map(|_| Message::Publish)
        } else if matches!(self.finding, Some(Finding::Found(_))) {
            Some(Message::Confirm)
        } else {
            None
        }
    }

    pub fn editing(&self) -> Option<&Pool> {
        match &self.screen {
            Screen::Editor(editor) => self.list.iter().find(|pool| pool.id == editor.id),
            Screen::Shelf | Screen::Open(_) => None,
        }
    }

    fn spread_best(&mut self, now: i64) -> Vec<Effect> {
        let wanted = matches!(&self.screen, Screen::Editor(editor) if editor.spread);
        let (true, Some(at), Some(best), Some(songs)) = (wanted, self.editing_at(), self.best.clone(), self.songs.clone()) else { return Vec::new() };
        if let Some(editor) = self.editor_mut() { editor.spread = false; }
        let mut effects = Vec::new();
        let mut used: HashSet<String> = self.list[at].slots.iter().filter_map(|slot| slot.hash.clone()).collect();
        for slot in 0..self.list[at].slots.len() {
            if !self.list[at].slots[slot].is_empty() {
                continue;
            }
            let mods = self.list[at].slots[slot].mods;
            let found = best.iter().map(|score| (score.hash.to_ascii_lowercase(), score)).find(|(hash, score)| !used.contains(hash) && songs.contains_key(hash) && played_with(mods, &score.mods));
            if let Some((hash, _)) = found {
                let map = songs[&hash].clone();
                used.insert(hash.clone());
                effects.extend(self.place_map(&hash, map, Some(Place::Slot(slot)), now));
            }
        }
        if let Some(editor) = self.editor_mut() { editor.selected = None; editor.panel = Panel::Closed; }
        effects
    }

    pub fn set_best(&mut self, scores: Option<&[crate::community::wire::Score]>) {
        self.best = scores.map(|scores| {
            let mut scores: Vec<_> = scores.iter().filter(|score| score.pp.is_finite() && score_target(score).is_some()).cloned().collect();
            scores.sort_by(|a, b| b.pp.total_cmp(&a.pp));
            let mut ids = HashSet::new();
            let mut hashes = HashSet::new();
            scores.retain(|score| {
                let repeated_id = score_id(score).is_some_and(|id| !ids.insert(id));
                let repeated_hash = match score_target(score) {
                    Some(Target::Hash(hash)) => !hashes.insert(hash),
                    _ => false,
                };
                !repeated_id && !repeated_hash
            });
            scores.truncate(100);
            scores
        });
        let placed = self.spread_best(self.clock);
        self.pending.extend(placed);
    }

    fn editing_at(&self) -> Option<usize> {
        match &self.screen {
            Screen::Editor(editor) => self.list.iter().position(|pool| pool.id == editor.id),
            Screen::Shelf | Screen::Open(_) => None,
        }
    }

    fn editor_mut(&mut self) -> Option<&mut Editor> {
        match &mut self.screen {
            Screen::Editor(editor) => Some(editor),
            Screen::Shelf | Screen::Open(_) => None,
        }
    }

    fn measure_effects(&mut self) -> Vec<Effect> {
        self.apply_measures();
        let Some(songs) = self.songs.clone() else {
            return Vec::new();
        };
        let mut effects = Vec::new();
        let draft = match &self.screen {
            Screen::Open(opening) => Some(&opening.pool),
            _ => None,
        };
        for pool in self.list.iter().chain(draft) {
            for slot in &pool.slots {
                let Some(hash) = slot.hash.as_deref() else { continue };
                if slot.measure.is_some() || self.measures.has(hash, slot.mods) {
                    continue;
                }
                let Some(map) = songs.get(hash) else { continue };
                if self.asked.insert((hash.to_owned(), slot.mods)) {
                    effects.push(Effect::Measure(hash.to_owned(), map.clone(), slot.mods));
                }
            }
        }
        effects
    }

    fn apply_measures(&mut self) {
        let mut changed = Vec::new();
        for (at, pool) in self.list.iter_mut().enumerate() {
            let mut touched = false;
            for slot in &mut pool.slots {
                if slot.measure.is_some() {
                    continue;
                }
                if let Some(hash) = slot.hash.as_deref() {
                    if let Some(Ok(measure)) = self.measures.get(hash, slot.mods) {
                        slot.measure = Some(*measure);
                        touched = true;
                    }
                }
            }
            if touched {
                changed.push(at);
            }
        }
        for at in changed {
            self.save_quiet(at);
        }
        if let Some(Finding::Found(candidate)) = &mut self.finding {
            for difficulty in &mut candidate.found.difficulties {
                if let Some(Ok(measure)) = self.measures.get(&difficulty.hash, Mod::Nm) { difficulty.stars = measure.stars; }
            }
        }
        if let Screen::Open(opening) = &mut self.screen {
            for slot in &mut opening.pool.slots {
                if slot.measure.is_some() {
                    continue;
                }
                if let Some(hash) = slot.hash.as_deref() {
                    if let Some(Ok(measure)) = self.measures.get(hash, slot.mods) {
                        slot.measure = Some(*measure);
                    }
                }
            }
        }
    }

    fn cancel_suggestion(&mut self) {
        if let Some(suggestion) = self.suggestions.take() {
            suggestion.stop.store(true, Ordering::SeqCst);
        }
        self.pending_suggestion = None;
    }

    fn refresh_for(&mut self, collection: bool) -> Vec<Effect> {
        if self.catalogue_loading {
            return Vec::new();
        }
        self.collection_shelf = collection;
        self.catalogue_loading = true;
        vec![Effect::Catalogue(collection, 0)]
    }

    pub fn ail(&self, slot: &Slot) -> Ail {
        let Some(hash) = slot.hash.as_deref() else { return Ail::Well };
        if self.fetching.as_ref().is_some_and(|(_, busy)| busy == hash) {
            return Ail::Fetching;
        }
        if self.songs.as_ref().is_some_and(|songs| !songs.contains_key(hash)) && !self.fetched.contains_key(hash) {
            return Ail::Missing;
        }
        if slot.measure.is_none() && matches!(self.measures.get(hash, slot.mods), Some(Err(_))) {
            return Ail::Unmeasured;
        }
        Ail::Well
    }

    pub fn server_revision(&self, pool: &Pool) -> Option<u64> {
        self.publications.iter().find(|row| row.mine && row.local_id == pool.id).map(|row| row.revision)
    }

    fn settle_conflict(&mut self, id: &str, keep: Keep, now: i64) -> Vec<Effect> {
        let mut effects = Vec::new();
        self.conflict = None;
        let Some(at) = self.list.iter().position(|pool| pool.id == id) else { return effects };
        let (name, mine) = (self.list[at].name.clone(), self.list[at].published_revision);
        match self.publications.iter().find(|row| row.mine && row.local_id == id).map(|row| (row.id.clone(), row.revision)) {
            Some((_, theirs)) if theirs == mine => effects.push(Effect::Say("catalogue-conflict-none")),
            Some((publication, _)) => {
                let before: HashSet<String> = self.list.iter().map(|pool| pool.id.clone()).collect();
                effects.extend(self.update(Message::OpenPublication(publication), now));
                if let Some(copy) = self.list.iter().position(|pool| !before.contains(&pool.id)) {
                    let copied = self.list[copy].id.clone();
                    self.list[copy].name = format!("{name} v{mine}");
                    self.save(copy, now);
                    if keep == Keep::Mine {
                        effects.extend(self.update(Message::Open(copied), now));
                    }
                    effects.push(Effect::Say("catalogue-conflict-kept"));
                }
            }
            None => {
                self.list[at].published_revision = 0;
                self.save(at, now);
                effects.push(Effect::Say("catalogue-conflict-gone"));
            }
        }
        effects
    }

    fn suggest_slot(&mut self, slot: usize) -> Vec<Effect> {
        let Some((id, fingerprint, mods, target, excluded, has_maps)) = self.editing().and_then(|pool| {
            let entry = pool.slots.get(slot)?;
            entry.is_empty().then(|| (
                pool.id.clone(),
                pool.fingerprint(),
                entry.mods,
                suggestion_target(pool, slot),
                pool.slots.iter().filter_map(|slot| slot.hash.clone()).collect::<HashSet<_>>(),
                pool.filled() > 0,
            ))
        }) else { return Vec::new() };
        self.cancel_suggestion();
        if let Some(Finding::Fetching(fetching)) = &self.finding {
            fetching.stop.store(true, Ordering::SeqCst);
        }
        self.finding = None;
        self.notice = None;
        if let Some(editor) = self.editor_mut() {
            editor.selected = Some(slot);
            editor.panel = Panel::Add;
            editor.source = SourceTab::Suggest;
        }
        self.suggestion_request = self.suggestion_request.wrapping_add(1);
        let request = self.suggestion_request;
        let stop = Arc::new(AtomicBool::new(false));
        self.suggestions = Some(Suggestions { request, pool: id, fingerprint, slot, mods, target, maps: target.is_none().then(Vec::new), offered: None, stop: stop.clone() });
        let Some(target) = target else {
            if has_maps && self.songs.is_none() {
                self.pending_suggestion = Some(slot);
                if !self.reading {
                    self.reading = true;
                    return vec![Effect::ReadSongs];
                }
            }
            return Vec::new();
        };
        let custom = self.editing().and_then(|pool| pool.slots.get(slot)).is_some_and(|entry| !entry.category.is_empty());
        let mut effects = if custom { Vec::new() } else { vec![Effect::SuggestRemote(request, mods, (target - OFFER_SPAN).max(0.0), target + OFFER_SPAN)] };
        match self.songs.clone() {
            Some(songs) => effects.push(Effect::Suggest(request, songs, mods, target, excluded, stop)),
            None => {
                self.pending_suggestion = Some(slot);
                if !self.reading {
                    self.reading = true;
                    effects.push(Effect::ReadSongs);
                }
            }
        }
        effects
    }

    fn put(&mut self, hash: &str, now: i64) -> Vec<Effect> {
        let Some(map) = self.songs.as_ref().and_then(|songs| songs.get(hash)).cloned() else { return Vec::new() };
        self.place_map(hash, map, None, now)
    }

    fn default_place(&self) -> Place {
        match &self.screen {
            Screen::Editor(editor) => match self.list.iter().find(|pool| pool.id == editor.id).and_then(|pool| target(pool, editor)) {
                Some(slot) => Place::Slot(slot),
                None => Place::End,
            },
            Screen::Shelf | Screen::Open(_) => Place::End,
        }
    }

    fn place_map(&mut self, hash: &str, map: Map, place: Option<Place>, now: i64) -> Vec<Effect> {
        let Some(at) = self.editing_at() else { return Vec::new() };
        let aimed = match (&self.screen, place) {
            (_, Some(Place::End)) => None,
            (_, Some(Place::Slot(slot) | Place::Replace(slot))) if slot < self.list[at].slots.len() => Some(slot),
            (Screen::Editor(editor), _) => target(&self.list[at], editor),
            (Screen::Shelf | Screen::Open(_), _) => None,
        };
        if let Some(already) = self.list[at].slots.iter().position(|slot| slot.hash.as_deref() == Some(hash)) {
            if aimed != Some(already) {
                self.notice = Some(Notice::Already(already));
                return Vec::new();
            }
        }
        let picked_suggestion = matches!(&self.screen, Screen::Editor(editor) if editor.source == SourceTab::Suggest);
        self.cancel_suggestion();
        self.remember(at);
        let mods = match &self.screen { Screen::Editor(editor) => editor.last_mod, _ => Mod::Nm };
        let inherit = matches!(&self.screen, Screen::Editor(editor) if editor.selected != aimed && self.list[at].frame == Frame::Free);
        let (category, colour) = match &self.screen { Screen::Editor(editor) => (editor.last_category.clone(), editor.last_colour), _ => (String::new(), None) };
        let pool = &mut self.list[at];
        let slot = match aimed {
            Some(slot) => slot,
            None => {
                pool.slots.push(Slot::empty(mods));
                pool.slots.len() - 1
            }
        };
        if (inherit || aimed.is_none()) && pool.slots[slot].is_empty() { pool.slots[slot].mods = mods; pool.slots[slot].category = category; pool.slots[slot].colour = colour; }
        pool.slots[slot].fill(hash, &map);
        let cached = match self.measures.get(hash, pool.slots[slot].mods) {
            Some(Ok(measure)) => Some(*measure),
            _ => None,
        };
        pool.slots[slot].measure = cached;
        let mods = pool.slots[slot].mods;
        let category = pool.slots[slot].category.clone();
        let colour = pool.slots[slot].colour;
        self.notice = None;
        let onward = if self.links.is_empty() || picked_suggestion {
            None
        } else {
            Some(match self.list[at].first_empty() {
                Some(free) => free,
                None => {
                    self.list[at].slots.push(Slot::empty(mods));
                    self.list[at].slots.len() - 1
                }
            })
        };
        if let Some(editor) = self.editor_mut() {
            editor.untouched = false;
            editor.last_mod = mods;
            editor.last_category = category;
            editor.last_colour = colour;
            editor.replace = false;
            editor.selected = Some(slot);
            editor.query.clear();
            if picked_suggestion {
                editor.panel = Panel::Slot;
                editor.source = SourceTab::Search;
            } else if editor.panel == Panel::Add {
                match onward {
                    Some(free) => editor.selected = Some(free),
                    None => editor.panel = Panel::Closed,
                }
            }
        }
        self.save(at, now);
        self.measure_effects()
    }

    pub fn newer_of(&self, pool: &Pool) -> Option<&crate::bot::Publication> {
        let saved = pool.saved.as_ref()?;
        self.publications.iter().find(|item| item.id == saved.id && item.revision > saved.revision)
    }

    pub fn backdrop_key(&self) -> Option<(String, PathBuf, bool)> {
        let pool = self.editing()?;
        let backdrop = pool.backdrop.as_ref()?;
        let path = match &backdrop.from {
            pools::BackdropFrom::Map(hash) => self.songs.as_ref()?.get(hash)?.background.clone()?,
            pools::BackdropFrom::File(path) => path.clone(),
        };
        Some((format!("{}:{}:{}", pool.id, path.display(), backdrop.blur), path, backdrop.blur))
    }

    pub fn backdrop_shown(&self) -> Option<(&image::Handle, u8)> {
        let (key, _, _) = self.backdrop_key()?;
        let dim = self.editing()?.backdrop.as_ref()?.dim;
        self.backdrop.as_ref().filter(|(held, _)| *held == key).map(|(_, picture)| (picture, dim))
    }

    fn backdrop_effect(&mut self) -> Vec<Effect> {
        match self.backdrop_key() {
            Some((key, _, _)) if self.backdrop.as_ref().is_some_and(|(held, _)| *held == key) => Vec::new(),
            Some((key, path, blur)) => vec![Effect::LoadBackdrop(key, path, blur)],
            None => {
                self.backdrop = None;
                Vec::new()
            }
        }
    }

    pub fn search(&self, query: &str) -> Vec<(String, Map)> {
        let Some(songs) = &self.songs else { return Vec::new() };
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        if words.is_empty() {
            return Vec::new();
        }
        let mut found: Vec<(String, Map)> = songs
            .iter()
            .filter(|(_, map)| {
                let line = map.line().to_lowercase();
                words.iter().all(|word| line.contains(word))
            })
            .map(|(hash, map)| (hash.clone(), map.clone()))
            .collect();
        found.sort_by(|a, b| a.1.title.to_lowercase().cmp(&b.1.title.to_lowercase()).then_with(|| a.1.version.cmp(&b.1.version)).then_with(|| a.0.cmp(&b.0)));
        found.truncate(RESULTS_MOST);
        found
    }

    fn roomy_drafts(&self) -> bool {
        self.list.iter().filter(|pool| pool.collection == self.collection_shelf).count() <= ROOMY_MOST
    }

    fn roomy_published(&self) -> bool {
        self.publications.iter().filter(|item| (item.kind == "collection") == self.collection_shelf).count() <= ROOMY_MOST
    }

    pub fn covers(&self) -> Vec<(String, Option<PathBuf>)> {
        let Some(songs) = &self.songs else { return Vec::new() };
        let mut seen = HashSet::new();
        let mut wanted = Vec::new();
        let mut add = |hash: &str| {
            if seen.insert(hash.to_owned()) {
                wanted.push((hash.to_owned(), songs.get(hash).and_then(|map| map.background.clone())));
            }
        };
        if let Some(Finding::Found(candidate)) = &self.finding {
            for difficulty in &candidate.found.difficulties { add(&difficulty.hash); }
        }
        match &self.screen {
            Screen::Shelf if !self.collection_shelf => {
                for pool in self.list.iter().filter(|pool| if self.shelf == Shelf::Saved { pool.saved.is_some() } else { !pool.collection && pool.saved.is_none() }) {
                    for slot in pool.slots.iter().filter_map(|slot| slot.hash.as_deref()).take(HERO_COVERS) {
                        add(slot);
                    }
                }
                if self.shelf == Shelf::Published {
                    for item in self.publications.iter().filter(|item| item.kind == "pool") {
                        for hash in published_hashes(item).into_iter().flatten().take(MOSAIC) {
                            add(&hash);
                        }
                    }
                }
            }
            Screen::Shelf => {
                for pool in self.list.iter().filter(|pool| pool.collection && pool.saved.is_none()) {
                    for slot in pool.slots.iter().filter_map(|slot| slot.hash.as_deref()).take(MOSAIC) {
                        add(slot);
                    }
                }
                for found in self.collections.iter().flatten().take(GAME_MOST) {
                    for hash in found.hashes.iter().take(4) {
                        add(hash);
                    }
                }
                for item in self.publications.iter().filter(|item| item.kind == "collection") {
                    for hash in published_hashes(item).into_iter().flatten().take(MOSAIC) {
                        add(&hash);
                    }
                }
            }
            Screen::Open(opening) => {
                for hash in opening.pool.slots.iter().filter_map(|slot| slot.hash.as_deref()) {
                    add(hash);
                }
            }
            Screen::Editor(editor) => {
                if let Some(pool) = self.list.iter().find(|pool| pool.id == editor.id) {
                    for hash in pool.slots.iter().filter_map(|slot| slot.hash.as_deref()) {
                        add(hash);
                    }
                }
                if editor.panel == Panel::Add {
                    match editor.source {
                        SourceTab::Search => {
                            for (hash, _) in self.search(&editor.query).iter().take(12) {
                                add(hash);
                            }
                        }
                        SourceTab::Collections => {
                            if let Some(collection) = editor.collection.and_then(|at| self.collections.as_ref()?.get(at)) {
                                for hash in collection.hashes.iter().skip(editor.collection_page * COLLECTION_PAGE).take(12) {
                                    add(hash);
                                }
                            }
                        }
                        SourceTab::Suggest => {
                            if let Some(maps) = self.suggestions.as_ref().and_then(|suggestion| suggestion.maps.as_ref()) {
                                for (hash, _) in maps {
                                    add(hash);
                                }
                            }
                        }
                        SourceTab::Best => {
                            for score in self.best.iter().flatten().take(12) {
                                add(&score.hash.to_ascii_lowercase());
                            }
                        }
                    }
                }
            }
        }
        if matches!(self.screen, Screen::Shelf) {
            for pool in &self.list {
                let path = pool.backdrop.as_ref().and_then(|backdrop| match &backdrop.from {
                    pools::BackdropFrom::Map(hash) => songs.get(hash).and_then(|map| map.background.clone()),
                    pools::BackdropFrom::File(path) => Some(path.clone()),
                });
                if let (Some(key), Some(path)) = (backing_key(pool), path) {
                    wanted.push((key, Some(path)));
                }
            }
        }
        wanted
    }
}

fn moved_order(keys: &[SlotKey], key: SlotKey, before: Option<SlotKey>) -> Option<Vec<usize>> {
    let from = keys.iter().position(|slot| *slot == key)?;
    if before == Some(key) { return None; }
    let mut order: Vec<usize> = (0..keys.len()).filter(|at| *at != from).collect();
    let to = match before {
        Some(before) => order.iter().position(|at| keys[*at] == before)?,
        None => order.len(),
    };
    order.insert(to, from);
    order.iter().enumerate().any(|(at, slot)| at != *slot).then_some(order)
}

fn shift_in<T>(items: &mut [T], marked: &[usize], down: bool) -> Vec<usize> {
    let length = items.len();
    let mut at: Vec<usize> = marked.to_vec();
    at.sort_unstable();
    if down {
        for index in (0..at.len()).rev() {
            let slot = at[index];
            if slot + 1 < length && !at.contains(&(slot + 1)) {
                items.swap(slot, slot + 1);
                at[index] = slot + 1;
            }
        }
    } else {
        for index in 0..at.len() {
            let slot = at[index];
            if slot > 0 && !at.contains(&(slot - 1)) {
                items.swap(slot, slot - 1);
                at[index] = slot - 1;
            }
        }
    }
    at.sort_unstable();
    at
}

fn shifted(marked: &[usize], length: usize, down: bool) -> Vec<usize> {
    let mut placeholder = vec![(); length];
    shift_in(&mut placeholder, marked, down)
}

fn suggestion_target(pool: &Pool, slot: usize) -> Option<f64> {
    let valid = |entry: &Slot| entry.measure.map(|measure| measure.stars).filter(|stars| stars.is_finite() && *stars > 0.0);
    let left = pool.slots.get(..slot)?.iter().rev().find_map(valid);
    let right = pool.slots.get(slot + 1..)?.iter().find_map(valid);
    match (left, right) {
        (Some(left), Some(right)) => Some((left + right) / 2.0),
        (Some(stars), None) | (None, Some(stars)) => Some(stars),
        (None, None) => {
            let measured: Vec<f64> = pool.slots.iter().filter_map(valid).collect();
            (!measured.is_empty()).then(|| measured.iter().sum::<f64>() / measured.len() as f64)
        }
    }
}

fn score_id(score: &crate::community::wire::Score) -> Option<u64> {
    let id = score.beatmap_id;
    (id.is_finite() && id > 0.0 && id < u64::MAX as f64 && id.fract() == 0.0).then_some(id as u64)
}

fn score_target(score: &crate::community::wire::Score) -> Option<Target> {
    match pool_links::parse(&score.hash) {
        Ok(Target::Hash(hash)) => Some(Target::Hash(hash)),
        _ => score_id(score).map(|id| Target::Beatmap { id, mode: Some("osu".into()) }),
    }
}

fn file_name_of(pool: &Pool) -> String {
    let name = crate::maps::tidy(&pool.name);
    format!("{}.pool", if name.is_empty() { "pool".to_owned() } else { name })
}

pub fn target(pool: &Pool, editor: &Editor) -> Option<usize> {
    match editor.selected {
        Some(at) if at < pool.slots.len() && (pool.slots[at].is_empty() || editor.replace) => Some(at),
        _ => {
            let akin = |slot: &Slot| if editor.last_category.is_empty() { slot.category.is_empty() && slot.mods == editor.last_mod } else { slot.category == editor.last_category };
            pool.slots.iter().position(|slot| slot.is_empty() && akin(slot)).or_else(|| pool.first_empty())
        }
    }
}

fn faded_text<'a>(words: String, size: f32, colour: Color) -> Element<'a, Message> {
    text(words).font(theme::SANS).size(size).wrapping(text::Wrapping::None).color(ui::faded(colour)).into()
}

fn para<'a>(words: String, size: f32, colour: Color) -> Element<'a, Message> {
    text(words).font(theme::SANS).size(size).color(ui::faded(colour)).width(Length::Fill).into()
}

fn semi<'a>(words: String, size: f32, colour: Color) -> Element<'a, Message> {
    text(words).font(theme::SANS_SEMI).size(size).wrapping(text::Wrapping::None).color(ui::faded(colour)).into()
}

fn wrapped_semi<'a>(words: String, size: f32, colour: Color) -> Element<'a, Message> {
    ui::moving_text(words, theme::SANS_SEMI, size, colour)
}

fn category_number(pool: &Pool, at: usize) -> usize {
    let target = &pool.slots[at];
    pool.slots[..=at].iter().filter(|slot| slot.category == target.category && (!target.category.is_empty() || slot.mods == target.mods)).count()
}

fn category_order(pool: &Pool) -> Vec<String> {
    let present: HashSet<String> = pool.slots.iter().map(|slot| if slot.category.is_empty() { slot.mods.code().to_owned() } else { slot.category.clone() }).collect();
    let mut order = Vec::new();
    let mut custom: Vec<_> = present.iter().filter(|name| !Mod::ALL.iter().any(|mods| mods.code() == name.as_str())).cloned().collect();
    custom.sort();
    for name in pool.category_order.iter().cloned().chain(Mod::ALL.iter().map(|mods| mods.code().to_owned())).chain(custom) {
        if present.contains(&name) && !order.contains(&name) { order.push(name); }
    }
    order
}

fn author_names(said: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut name = String::new();
    let mut quoted = false;
    for c in said.chars() {
        if c == '"' { quoted = !quoted; }
        else if !quoted && (c.is_whitespace() || c == ',' || c == ';') {
            if !name.is_empty() { names.push(std::mem::take(&mut name)); }
        } else { name.push(c); }
    }
    if !name.is_empty() { names.push(name); }
    names
}

fn published_pool(publication: &crate::bot::Publication, songs: Option<&HashMap<String, Map>>, now: i64) -> Option<Pool> {
    if publication.kind == "collection" {
        let hashes: Vec<String> = publication.content["hashes"].as_array().into_iter().flatten().filter_map(|hash| hash.as_str().map(str::to_owned)).collect();
        return (!hashes.is_empty()).then(|| collection_pool(&Collection { name: publication.name.clone(), hashes }, songs, now));
    }
    serde_json::to_vec(&publication.content).ok().and_then(|bytes| pool_share::from_file(&bytes, now).ok())
}

fn collection_pool(collection: &Collection, songs: Option<&HashMap<String, Map>>, now: i64) -> Pool {
    let mut pool = Pool::new(Frame::Free, &collection.name, now);
    pool.collection = true;
    pool.slots = collection.hashes.iter().map(|hash| {
        let mut slot = Slot::empty(Mod::Nm);
        if let Some(map) = songs.and_then(|songs| songs.get(hash)) { slot.fill(hash, map); }
        else { slot.hash = Some(hash.clone()); slot.title = hash.clone(); }
        slot
    }).collect();
    pool
}

fn slot_colour(slot: &Slot) -> Color {
    let rgb = if !slot.category.is_empty() { slot.colour } else { None }.unwrap_or(match slot.mods {
        Mod::Nm => [0x4f, 0xd0, 0xe2], Mod::Dt => [0x86, 0x4a, 0xe6], Mod::Hr => [0xe6, 0x2c, 0x30],
        Mod::Hd => [0xe0, 0x86, 0x32], Mod::Fm => [0xe0, 0x3c, 0x80], Mod::Tb => [0xb0, 0x3c, 0x42],
    });
    Color::from_rgb8(rgb[0], rgb[1], rgb[2])
}

fn colour_hex(colour: Color) -> String {
    format!("#{:02x}{:02x}{:02x}", (colour.r * 255.0).round() as u8, (colour.g * 255.0).round() as u8, (colour.b * 255.0).round() as u8)
}

fn parse_colour(value: &str) -> Option<[u8; 3]> {
    let value = value.trim().trim_start_matches('#');
    if value.len() != 6 || !value.is_ascii() { return None; }
    Some([u8::from_str_radix(&value[..2], 16).ok()?, u8::from_str_radix(&value[2..4], 16).ok()?, u8::from_str_radix(&value[4..], 16).ok()?])
}

fn is_link(text: &str) -> bool {
    matches!(pool_links::parse(text), Ok(_) | Err(pool_links::Refusal::Mode(_)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Fine,
    Stop,
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub verdict: Verdict,
    pub key: &'static str,
    pub count: Option<(u64, u64)>,
    pub slot: Option<usize>,
}

impl Check {
    fn plain(verdict: Verdict, key: &'static str) -> Check {
        Check { verdict, key, count: None, slot: None }
    }

    pub fn words(&self, words: &Words) -> String {
        match self.count {
            Some((n, total)) => words.n_of(self.key, n, total),
            None => words.t(self.key),
        }
    }
}

pub fn publish_checks(pool: &Pool) -> Vec<Check> {
    let filled = pool.filled();
    let total = pool.slots.len();
    let waiting: Vec<usize> = pool.slots.iter().enumerate().filter(|(_, slot)| !slot.is_empty() && slot.measure.is_none()).map(|(at, _)| at).collect();
    let mut checks = Vec::new();
    if pool.compiler.trim().is_empty() {
        checks.push(Check::plain(Verdict::Stop, "catalogue-sign-in"));
    }
    checks.push(if pool.name.trim().is_empty() { Check::plain(Verdict::Stop, "publish-name-missing") } else { Check::plain(Verdict::Fine, "publish-name-set") });
    checks.push(match filled {
        0 => Check::plain(Verdict::Stop, "publish-no-maps"),
        _ if pool.collection => Check { verdict: Verdict::Fine, key: "publish-maps", count: Some((filled as u64, filled as u64)), slot: None },
        _ if filled == total => Check { verdict: Verdict::Fine, key: "publish-filled", count: Some((filled as u64, total as u64)), slot: None },
        _ => Check { verdict: Verdict::Note, key: "publish-filled", count: Some((filled as u64, total as u64)), slot: pool.first_empty() },
    });
    if filled > 0 && !pool.collection {
        checks.push(match waiting.first() {
            None => Check::plain(Verdict::Fine, "publish-measured"),
            Some(first) => Check { verdict: Verdict::Note, key: "publish-unmeasured", count: Some((waiting.len() as u64, filled as u64)), slot: Some(*first) },
        });
    }
    if !pool.collection && pool.authors.is_empty() {
        checks.push(Check::plain(Verdict::Note, "publish-no-mappoolers"));
    }
    checks
}

pub fn publish_ready(pool: &Pool) -> bool {
    publish_checks(pool).iter().all(|check| check.verdict != Verdict::Stop)
}

const WHOLESALE_MOST: usize = 100;

pub fn played_with(slot: Mod, mods: &str) -> bool {
    let mods = mods.to_ascii_uppercase();
    let has = |code: &str| mods.contains(code);
    let fast = has("DT") || has("NC");
    match slot {
        Mod::Nm => !["HD", "HR", "DT", "NC", "EZ", "HT", "FL"].iter().any(|code| has(code)),
        Mod::Hd => has("HD") && !has("HR") && !fast,
        Mod::Hr => has("HR") && !fast,
        Mod::Dt => fast,
        Mod::Fm | Mod::Tb => true,
    }
}

pub fn links_in(text: &str) -> Vec<String> {
    text.split_whitespace().map(|word| word.trim_matches(|sign: char| "<>()[]{},;\"'".contains(sign))).filter(|word| is_link(word)).map(str::to_owned).collect()
}

pub fn is_paste(text: &str) -> bool {
    pool_share::is_text(text) || is_link(text) || (text.split_whitespace().count() > 1 && links_in(text).len() == text.split_whitespace().count())
}

fn mono<'a>(words: String, size: f32, colour: Color) -> Element<'a, Message> {
    text(words).font(theme::MONO).size(size).wrapping(text::Wrapping::None).color(ui::faded(colour)).into()
}

fn stars_of(words: &Words, value: f64) -> String {
    screen::decimal(words, value as f32, 2)
}

fn cover<'a>(thumbs: &HashMap<String, image::Handle>, hash: Option<&str>, wide: f32, high: f32, round: f32) -> Element<'a, Message> {
    let k = ui::fade();
    match hash.and_then(|hash| thumbs.get(hash)) {
        Some(handle) => image(crate::crops::fitted(handle, wide, high, round)).content_fit(iced::ContentFit::Fill).width(wide).height(high).border_radius(round).opacity(k).into(),
        None => container(Space::new())
            .width(wide)
            .height(high)
            .style(move |_| container::Style {
                background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.012 * k))),
                border: Border { color: crate::theme::outline(Color::from_rgba(1.0, 1.0, 1.0, 0.04 * k)), width: 1.0, radius: round.into() },
                ..container::Style::default()
            })
            .into(),
    }
}

fn hatched_cover<'a>() -> Element<'a, Message> {
    let k = ui::fade();
    let base = ui::mix(theme::GROUND, Color::BLACK, 0.26);
    let frame = container(container(ui::band_hatch(ui::mix(base, Color::WHITE, 0.05))).width(Length::Fill).height(Length::Fill).padding(2.0))
        .width(COVER_WIDE)
        .height(COVER_HIGH)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color { a: k, ..base })),
            border: Border { color: crate::theme::outline(soft(0.035, k)), width: 1.0, radius: COVER_ROUND.into() },
            ..container::Style::default()
        });
    frame.into()
}

fn tinted(colour: Color, share: f32, k: f32) -> Color {
    Color { a: k, ..ui::mix(theme::GROUND, colour, share) }
}

const WARM: Color = Color::from_rgb(0.98, 0.80, 0.80);

fn soft(share: f32, k: f32) -> Color {
    tinted(WARM, share * 0.8, k)
}

fn number_badge<'a>(number: usize, lit: bool) -> Element<'a, Message> {
    let k = ui::fade();
    container(text(number.to_string()).font(theme::MONO_BOLD).size(12.0).color(ui::faded(INK)))
        .width(28.0)
        .height(28.0)
        .center(28.0)
        .style(move |_| container::Style {
            background: Some(Background::Color(if lit { tinted(ACCENT, 0.2, k) } else { soft(0.08, k) })),
            border: Border { radius: 14.0.into(), ..Border::default() },
            ..container::Style::default()
        })
        .into()
}

fn mod_badge<'a>(mods: Mod, lit: bool) -> Element<'a, Message> {
    let k = ui::fade();
    let base = slot_colour(&Slot::empty(mods));
    container(text(mods.code()).font(theme::MONO_BOLD).size(12.0).color(ui::faded(ui::mix(base, Color::WHITE, 0.45))))
        .padding([4, 8])
        .style(move |_| container::Style {
            background: Some(Background::Color(tinted(base, if lit { 0.28 } else { 0.16 }, k))),
            border: Border { radius: 6.0.into(), ..Border::default() },
            ..container::Style::default()
        })
        .into()
}

fn row_style(selected: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| {
        let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: Some(Background::Color(if selected {
                Color::from_rgba(0.886, 0.282, 0.282, 0.035)
            } else if lit {
                Color::from_rgba(1.0, 1.0, 1.0, 0.012)
            } else {
                Color::TRANSPARENT
            })),
            text_color: INK,
            border: Border { color: crate::theme::outline(if selected { Color::from_rgba(0.886, 0.282, 0.282, 0.6) } else { Color::TRANSPARENT }), width: 1.0, radius: 12.0.into() },
            shadow: iced::Shadow::default(),
            snap: true,
        }
    }
}

fn surface_style(lit: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| {
        let hot = lit || matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, if hot { 0.016 } else { 0.008 }))),
            text_color: INK,
            border: Border { color: crate::theme::outline(Color::from_rgba(1.0, 1.0, 1.0, if hot { 0.07 } else { 0.04 })), width: 1.0, radius: 14.0.into() },
            shadow: iced::Shadow::default(),
            snap: true,
        }
    }
}

fn panel_box<'a>(inside: Element<'a, Message>) -> Element<'a, Message> {
    let k = ui::fade();
    container(inside)
        .padding(16)
        .width(Length::Fill)
        .style(move |theme| container::Style {
            background: Some(Background::Color(Color { a: theme::RAISED.a * k, ..theme::RAISED })),
            border: Border { color: crate::theme::outline(Color { a: theme::LINE.a * k, ..theme::LINE }), width: 1.0, radius: theme::CARD_RADIUS.into() },
            ..theme::card(theme)
        })
        .into()
}

fn results_list<'a>(list: iced::widget::Column<'a, Message>) -> Element<'a, Message> {
    container(scrollable(list).height(Length::Shrink).direction(ui::hidden_bar()).style(ui::thin_scroll)).max_height(420.0).into()
}

fn sheet_box<'a>(inside: Element<'a, Message>) -> Element<'a, Message> {
    let k = ui::fade();
    container(inside)
        .padding(22)
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.075, 0.035, 0.045, k))),
            border: Border { color: crate::theme::outline(soft(0.07, k)), width: 1.0, radius: 16.0.into() },
            shadow: iced::Shadow { color: Color::from_rgba(0.0, 0.0, 0.0, 0.5 * k), offset: iced::Vector::new(0.0, 12.0), blur_radius: 32.0 },
            ..container::Style::default()
        })
        .into()
}

fn primary_button<'a>(label: String, press: Message) -> Element<'a, Message> {
    ui::primary(label, Some(press))
}

fn quiet_button<'a>(label: String, press: Message) -> Element<'a, Message> {
    ui::quiet(label, Some(press))
}

fn outlined<'a>(label: String, press: Option<Message>) -> Element<'a, Message> {
    button(container(text(label).font(theme::SANS_SEMI).size(theme::BODY).wrapping(text::Wrapping::None)).center_y(CONTROL))
        .padding([0, 12])
        .style(ui::button_faded(|_: &iced::Theme, status: button::Status| {
            let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: lit.then_some(Background::Color(soft(0.08, 1.0))),
                text_color: if matches!(status, button::Status::Disabled) { FAINT } else { INK },
                border: Border { color: crate::theme::outline(soft(0.16, 1.0)), width: 1.0, radius: 8.0.into() },
                shadow: iced::Shadow::default(),
                snap: true,
            }
        }))
        .on_press_maybe(press)
        .into()
}

fn plain_button<'a>(label: String, press: Message) -> Element<'a, Message> {
    button(container(text(label).font(theme::SANS_SEMI).size(theme::BODY).color(ui::faded(MUTED))).center_y(CONTROL)).padding([0, 12]).style(ui::button_faded(theme::bare)).on_press(press).into()
}

fn icon_button<'a>(icon: Icon, size: f32, ink: Color, press: Option<Message>) -> Element<'a, Message> {
    button(container(glyph(icon, size, ink)).center(CONTROL)).padding(0).style(ui::button_faded(theme::bare)).on_press_maybe(press).into()
}

fn crumbs<'a>(words: &Words, back: Option<Message>, title: Element<'a, Message>) -> Element<'a, Message> {
    let first: Element<'a, Message> = match back {
        Some(back) => button(text(words.t("pools-title")).font(theme::SANS).size(13.0).color(ui::faded(FAINT))).padding(0).style(ui::button_faded(theme::bare)).on_press(back).into(),
        None => faded_text(words.t("pools-crumb"), 13.0, FAINT),
    };
    column![first, title].spacing(6).into()
}

fn bars<'a>(pool: &Pool) -> Element<'a, Message> {
    let k = ui::fade();
    let mut line = row![].spacing(2).align_y(iced::alignment::Vertical::Bottom);
    let top = pool.slots.iter().filter_map(|slot| slot.measure.map(|m| m.stars)).fold(1.0_f64, f64::max);
    for slot in &pool.slots {
        let (stars, filled) = match slot.measure {
            Some(measure) => (measure.stars, true),
            None => (0.0, false),
        };
        let colour = if filled { crate::dossier::star_colour(stars as f32) } else { soft(0.18, 1.0) };
        let high = if filled { 8.0 + (stars / top).clamp(0.0, 1.0) as f32 * 16.0 } else { 3.0 };
        line = line.push(container(Space::new()).width(5.0).height(high).style(move |_| container::Style {
            background: Some(Background::Color(Color { a: colour.a * k, ..colour })),
            border: Border { radius: 1.5.into(), ..Border::default() },
            ..container::Style::default()
        }));
    }
    line.into()
}

#[derive(Clone, Copy)]
enum Col {
    Fill,
    Wide(f32),
    End(f32),
}

const LEDGER_PAD: f32 = 18.0;
const LEDGER_GAP: f32 = 16.0;
const FACE: f32 = 26.0;
const FACES_MOST: usize = 4;
const NAMED_MOST: usize = 2;
const STARS_WIDE: f32 = 142.0;
const ROOMY_MOST: usize = 4;
const STRIP_MOST: usize = 24;
const STRIP_GAP: f32 = 6.0;

fn conflict_banner<'a>(state: &State, pool: &Pool, words: &'a Words, width: f32) -> Element<'a, Message> {
    let k = ui::fade();
    let busy = state.resolving.is_some() || state.catalogue_loading;
    let mut told = column![semi(words.t("catalogue-conflict-title"), 15.0, WARNED)].spacing(4);
    if let Some(theirs) = state.server_revision(pool).filter(|theirs| *theirs != pool.published_revision) {
        told = told.push(faded_text(words.with("catalogue-conflict-versions", &[("theirs", theirs.to_string()), ("mine", pool.published_revision.to_string())]), 13.0, INK));
    }
    let actions = row![
        outlined(words.t("catalogue-conflict-theirs"), (!busy).then_some(Message::Conflict(Keep::Theirs))),
        outlined(words.t("catalogue-conflict-mine"), (!busy).then_some(Message::Conflict(Keep::Mine))),
    ]
    .spacing(8);
    let said = row![glyph(Icon::Close, 16.0, WARNED), container(told).width(Length::Fill)].spacing(14).align_y(iced::Center);
    let inside: Element<'a, Message> = if width < 900.0 { column![said, actions].spacing(12).into() } else { said.push(actions).into() };
    container(inside)
        .padding([14, 18])
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.886, 0.282, 0.282, 0.05 * k))),
            border: Border { color: crate::theme::outline(Color::from_rgba(0.886, 0.282, 0.282, 0.4 * k)), width: 1.0, radius: 12.0.into() },
            ..container::Style::default()
        })
        .into()
}

fn publication_authors(item: &crate::bot::Publication) -> Vec<String> {
    let mut names: Vec<String> = item.content["authors"].as_array().into_iter().flatten().filter_map(|name| name.as_str().map(str::to_owned)).collect();
    if let Some(compiler) = item.content["compiler"].as_str().filter(|name| !name.is_empty()) {
        if !names.iter().any(|name| name.eq_ignore_ascii_case(compiler)) { names.push(compiler.to_owned()); }
    }
    names
}

fn author_identity<'a>(state: &'a State, name: &str) -> Element<'a, Message> {
    let face: Element<'a, Message> = match state.faces.get(&name.to_lowercase()) {
        Some(face) => image(crate::crops::fitted(face, 26.0, 26.0, 13.0)).width(26.0).height(26.0).content_fit(iced::ContentFit::Fill).opacity(ui::fade()).into(),
        None => container(glyph(Icon::Person, 18.0, MUTED)).center(26.0).into(),
    };
    let wide = (ui::text_width(name, theme::SANS_SEMI, 13.0) + 2.0).min(129.0);
    row![face, container(ui::moving_text(name.to_owned(), theme::SANS_SEMI, 13.0, INK)).width(wide)].spacing(7).align_y(iced::Center).into()
}

fn author_chip<'a>(state: &'a State, name: &str) -> Element<'a, Message> {
    button(author_identity(state, name)).padding([5, 9]).style(ui::button_faded(theme::bare)).on_press(Message::Author(name.to_owned())).into()
}

fn open_view<'a>(state: &'a State, opening: &'a Opening, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, t: f32) -> Element<'a, Message> {
    let pool = &opening.pool;
    let have: Vec<&Slot> = pool.slots.iter().filter(|slot| slot.hash.is_some()).collect();
    let total = have.len();
    let known = state.songs.is_some();
    let missing = state.missing(pool);
    let present = total.saturating_sub(missing.len());
    let head = crumbs(words, Some(Message::Back), semi(words.t("pool-open-title"), 28.0, INK));
    let name = if pool.name.is_empty() { words.t("pool-untitled") } else { pool.name.clone() };
    let mark = |done: bool, active: bool| -> Element<'a, Message> {
        if done {
            glyph(Icon::Check, 14.0, ACCENT)
        } else if active {
            container(container(Space::new()).width(7.0).height(7.0).style({ let dot = ui::faded(ACCENT); move |_| container::Style { background: Some(Background::Color(dot)), border: Border { radius: 3.5.into(), ..Border::default() }, ..container::Style::default() } })).center(14.0).into()
        } else {
            Space::new().width(14.0).into()
        }
    };
    let line = |done: bool, active: bool, said: String| -> Element<'a, Message> { row![mark(done, active), faded_text(said, 13.5, if active { INK } else if done { MUTED } else { FAINT })].spacing(10).align_y(iced::Center).into() };
    let fetching_line = match &opening.step {
        Some(crate::maps::Step::Downloading { done, total: Some(size), .. }) => {
            let mb = |bytes: u64| screen::decimal(words, bytes as f32 / 1_048_576.0, 1);
            words.with("pool-step-download-of", &[("done", mb(*done)), ("total", mb(*size))])
        }
        _ if opening.running => words.n("pool-open-fetching", (opening.total).saturating_sub(opening.queue.len()) as u64),
        _ => words.t("pool-open-fetch-idle"),
    };
    let measured = pool.slots.iter().filter(|slot| slot.hash.is_some()).all(|slot| slot.measure.is_some()) && total > 0;
    let mut ledger = column![
        semi(words.with("pool-opening", &[("name", name)]), 15.0, INK),
        line(true, false, words.t("pool-open-read")),
        line(known, !known, if known { words.with("pool-open-found", &[("n", present.to_string()), ("total", total.to_string())]) } else { words.t("pool-reading") }),
    ]
    .spacing(10);
    if !missing.is_empty() || opening.running || opening.lost > 0 {
        ledger = ledger.push(line(!opening.running && missing.is_empty(), opening.running, fetching_line));
    }
    ledger = ledger.push(line(measured, known && !measured && missing.is_empty(), words.t("pool-open-measure")));
    if opening.lost > 0 && !opening.running {
        ledger = ledger.push(faded_text(words.n("pool-open-lost", opening.lost as u64), 12.0, ACCENT));
    }
    let mut actions = column![].spacing(10);
    if known && !missing.is_empty() && !opening.running {
        actions = actions.push(primary_button(words.t("pool-open-get"), Message::GetMissing));
        actions = actions.push(quiet_button(words.t("pool-open-keep-bare"), Message::Keep));
    } else if opening.running {
        actions = actions.push(ui::primary(words.t("pool-open-keep"), None));
    } else {
        actions = actions.push(primary_button(words.t("pool-open-keep"), Message::Keep));
    }
    let left = panel_box(column![ledger, actions, para(words.t("pool-open-note"), 12.0, MUTED)].spacing(18).into());

    let mut list = column![].spacing(4);
    for (at, slot) in pool.slots.iter().enumerate() {
        let gone = slot.hash.as_deref().is_some_and(|hash| known && !state.songs.as_ref().is_some_and(|songs| songs.contains_key(hash)));
        let badges = row![number_badge(category_number(pool, at), false), mod_badge(slot.mods, false)].spacing(8).align_y(iced::Center).width(88.0);
        let figure: Element<'a, Message> = match (&slot.measure, gone, slot.hash.is_some()) {
            (Some(measure), _, _) => text(stars_of(words, measure.stars)).font(theme::MONO_BOLD).size(19.0).wrapping(text::Wrapping::None).color(ui::faded(crate::dossier::star_colour(measure.stars as f32))).into(),
            (None, true, _) => faded_text(words.t("pool-gone"), 12.0, ACCENT),
            _ => Space::new().width(0.0).into(),
        };
        let inner: Element<'a, Message> = if slot.hash.is_none() {
            row![badges, cover(thumbs, None, COVER_WIDE, COVER_HIGH, COVER_ROUND), column![semi(words.t("pool-slot-empty"), 14.0, MUTED)].width(Length::Fill)].spacing(14).align_y(iced::Center).into()
        } else {
            row![
                badges,
                cover(thumbs, slot.hash.as_deref(), COVER_WIDE, COVER_HIGH, COVER_ROUND),
                column![semi(slot.title.clone(), 14.0, INK), faded_text(slot.artist.clone(), 12.5, MUTED), faded_text(slot.version.clone(), 11.0, FAINT)].spacing(1).width(Length::Fill),
                figure,
            ]
            .spacing(14)
            .align_y(iced::Center)
            .into()
        };
        list = list.push(container(inner).height(ROW_HIGH).align_y(iced::Center).padding([4, 10]));
    }
    let right = column![ui::label(words.t("pool-open-body"), FAINT), list].spacing(10);
    let body: Element<'a, Message> = row![container(left).width(Length::Fixed(OPEN_WIDE)), container(right).width(Length::Fill)].spacing(32).align_y(iced::alignment::Vertical::Top).into();
    column![ui::appearing(ui::appear(t, 0), || head.into()), ui::appearing(ui::appear(t, 1), || body)].spacing(24).into()
}

const PANEL_BESIDE: f32 = 1100.0;
const PANEL_GAP: f32 = 22.0;
const BACKDROP_WIDE: f32 = 400.0;
const TABLE_GAP: f32 = 16.0;
const TABLE_LEAST: f32 = if cfg!(test) { 430.0 } else { 290.0 };

fn coach<'a>(step: usize, words: &'a Words) -> Element<'a, Message> {
    let k = ui::fade();
    let key = pools::Guide::TOUR[step.min(pools::Guide::TOUR.len() - 1)];
    let last = step + 1 >= pools::Guide::TOUR.len();
    let inside = column![
        row![
            text(words.with("guide-step-of", &[("n", (step + 1).to_string()), ("total", pools::Guide::TOUR.len().to_string())])).font(theme::MONO).size(12.0).color(ui::faded(ACCENT)),
            text(words.t(key)).font(theme::SANS_SEMI).size(14.0).color(ui::faded(INK)),
        ]
        .spacing(10)
        .align_y(iced::Center),
        para(ui::keys(&words.t(&format!("{key}-how"))), 13.5, MUTED),
        row![primary_button(words.t(if last { "tour-done" } else { "tour-next" }), Message::GuideNext), plain_button(words.t("guide-hide"), Message::HideGuide)].spacing(8).align_y(iced::Center),
    ]
    .spacing(10);
    container(inside)
        .padding([14, 16])
        .max_width(COACH_WIDE)
        .style(move |_| container::Style { background: Some(Background::Color(tinted(ACCENT, 0.10, k))), border: Border { color: crate::theme::outline(tinted(ACCENT, 0.45, k)), width: 1.0, radius: 12.0.into() }, ..container::Style::default() })
        .into()
}

const COACH_WIDE: f32 = 560.0;

const EDITOR_GROWN: f32 = if cfg!(test) { 1.0 } else { 1.14 };

pub fn view<'a>(state: &'a State, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, width: f32, section_t: f32, clocks: &Clocks) -> Element<'a, Message> {
    if matches!(&state.screen, Screen::Editor(editor) if !editor.starting) {
        return ui::scaled(view_at(state, words, thumbs, width / EDITOR_GROWN, section_t, clocks), EDITOR_GROWN).into();
    }
    view_at(state, words, thumbs, width, section_t, clocks)
}

fn view_at<'a>(state: &'a State, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, width: f32, section_t: f32, clocks: &Clocks) -> Element<'a, Message> {
    let t = section_t.min(clocks.screen);
    let panel_t = clocks.panel.min((t - PANEL_LATE).max(0.0));
    let max_width = if matches!(state.screen, Screen::Editor(_)) { 1800.0 } else { 1180.0 };
    let content_width = (width - 48.0).min(max_width);
    let page = match &state.screen {
        Screen::Open(opening) => open_view(state, opening, words, thumbs, t),
        Screen::Shelf => shelf(state, words, thumbs, content_width, t, section_t.min(clocks.page), clocks),
        Screen::Editor(editor) => match state.list.iter().find(|pool| pool.id == editor.id) {
            Some(pool) => editor_view(state, editor, pool, words, thumbs, content_width, clocks.high, t, &clocks.shut, clocks.opened.iter().find(|(name, _)| *name == "backdrop").map_or(SETTLED, |(_, age)| *age), clocks.gone),
            None => shelf(state, words, thumbs, content_width, t, section_t.min(clocks.page), clocks),
        },
    };
    let page: Element<'a, Message> = if !state.save_errors.is_empty() {
        column![row![text(words.t("pool-unsaved")).color(ACCENT).width(Length::Fill), ui::quiet(words.t("pool-save-retry"), Some(Message::RetrySaves))].spacing(12).align_y(iced::Center), page].spacing(12).into()
    } else { page };
    let footed = match &state.screen {
        Screen::Editor(editor) if !editor.starting => state.list.iter().find(|pool| pool.id == editor.id).filter(|pool| pool.filled() > 0),
        _ => None,
    };
    let rolled = scrollable(container(container(page).max_width(max_width)).center_x(Length::Fill).padding(Padding { top: 12.0, right: 24.0, bottom: if footed.is_some() { 108.0 } else { 28.0 }, left: 24.0 }))
        .id(iced::widget::Id::new(SCROLL_ID))
        .style(ui::thin_scroll)
        .direction(ui::hidden_bar())
        .width(Length::Fill)
        .height(Length::Fill);
    let page: Element<'a, Message> = crate::glide::brim(crate::glide::edged(rolled, iced::widget::Id::new(SCROLL_ID))).into();
    if let Screen::Editor(editor) = &state.screen {
        let k = ui::appear(panel_t, 0);
        let pool = state.list.iter().find(|pool| pool.id == editor.id);
        let mut layers = vec![page];
        if let Some((picture, dim)) = state.backdrop_shown().filter(|_| !editor.starting) {
            let k = ui::fade() * ui::appear(t, 0);
            let picture = picture.clone();
            let ground = theme::GROUND.into_rgba8();
            let shown = iced::widget::responsive(move |size| {
                image(crate::crops::sunk(&picture, size.width.max(1.0), BACKDROP_HIGH, dim, [ground[0], ground[1], ground[2]])).content_fit(iced::ContentFit::Fill).width(Length::Fill).height(BACKDROP_HIGH).opacity(k).into()
            });
            layers.insert(0, column![container(shown).width(Length::Fill).height(BACKDROP_HIGH), Space::new().height(Length::Fill)].into());
        }
        if let Some(pool) = footed.filter(|_| !(editor.choosing && !editor.marked.is_empty())) {
            let k = ui::fade() * ui::appear(t, 5);
            layers.push(ui::fading(k, || {
                let strip = container(container(pool_foot(state, pool, words)).max_width(max_width)).center_x(Length::Fill).padding(Padding { top: 8.0, right: 24.0, bottom: 18.0, left: 24.0 }).style(move |_| container::Style { background: Some(Background::Color(Color { a: k, ..theme::GROUND })), ..container::Style::default() });
                container(column![ui::rise(theme::GROUND, 28.0), strip]).height(Length::Fill).align_y(iced::alignment::Vertical::Bottom).into()
            }));
        }
        if let Some(error) = state.catalogue_error.as_ref().filter(|_| state.conflict.is_none() && !editor.publish) {
            let lift = if footed.is_some() { 84.0 } else { 20.0 };
            layers.push(ui::appearing(ui::appear(t, 5), || container(catalogue_toast(error, words)).width(Length::Fill).height(Length::Fill).align_x(iced::alignment::Horizontal::Right).align_y(iced::alignment::Vertical::Bottom).padding(Padding { right: 24.0, bottom: lift, ..Padding::ZERO }).into()));
        }
        if editor.choosing && !editor.marked.is_empty() {
            layers.push(ui::appearing(k, || bulk_bar(editor, words)));
        } else if let (true, Some(pool)) = (editor.share, pool) {
            layers.push(ui::fading(ui::fade() * k, || share_modal(pool, words, state.exporting.is_some())));
        } else if let (Some(out), Some(pool)) = (leaving(&clocks.shut, "share"), pool) {
            layers.push(ui::fading(ui::fade() * out * out, || share_modal(pool, words, false)));
        } else if let (false, Some(pool)) = (editor.choosing, pool) {
            let open = sheet_of(editor, pool).map(|sheet| (editor, sheet));
            let leaving = editor.gone.as_deref().filter(|_| open.is_none() && clocks.gone < ui::APPEAR).and_then(|was| sheet_of(was, pool).map(|sheet| (was, sheet)));
            let still = open.is_none();
            let k = if still { 1.0 - ui::appear(clocks.gone, 0) } else { k };
            if let Some((shown, sheet)) = open.or(leaving) {
                let close = match sheet {
                    Sheet::Publish => Message::PublishSheet(false),
                    Sheet::Slot(_) => Message::Select(None),
                };
                let veil = ui::fading(ui::fade() * k, || iced::widget::opaque(iced::widget::mouse_area(ui::veil(theme::SCRIM)).on_press(close)));
                let wide = PANEL_WIDE.min((width - 32.0).max(280.0));
                let sheet = ui::appearing(k, || {
                    let side = match sheet {
                        Sheet::Slot(at) => slot_panel(pool, shown, at, wide, words, thumbs, state.songs.as_ref()),
                        Sheet::Publish => publish_panel(state, shown, pool, words, thumbs),
                    };
                    let seen = ui::fade();
                    container(scrollable(side).height(Length::Shrink).style(ui::thin_scroll).direction(ui::hidden_bar()))
                        .width(wide)
                        .height(Length::Shrink)
                        .style(move |_| container::Style { background: Some(Background::Color(Color { a: seen, ..theme::GROUND })), border: Border { radius: 14.0.into(), ..Border::default() }, ..container::Style::default() })
                        .into()
                });
                layers.push(ui::inert(veil, still));
                layers.push(ui::inert(container(ui::scaled(sheet, sheet_scale(width))).center(Length::Fill).padding(16), still));
            }
        }
        return iced::widget::Stack::with_children(layers).into();
    }
    page
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sheet {
    Publish,
    Slot(usize),
}

fn sheet_of(editor: &Editor, pool: &Pool) -> Option<Sheet> {
    if editor.publish {
        return Some(Sheet::Publish);
    }
    match (editor.panel, editor.selected.filter(|at| *at < pool.slots.len())) {
        (Panel::Add, _) => None,
        (Panel::Slot, Some(at)) => Some(Sheet::Slot(at)),
        _ => None,
    }
}

const SHEET_WIDE: f32 = 1600.0;
const SHEET_GROWTH: f32 = 1.3;

fn sheet_scale(width: f32) -> f32 {
    (width / SHEET_WIDE).clamp(1.0, SHEET_GROWTH)
}

const PANEL_LATE: f32 = 0.25;

#[cfg(test)]
mod tests;
