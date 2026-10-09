use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use iced::widget::{button, column, container, image, pick_list, row, scrollable, stack, text, text_input, Space};
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
#[cfg(test)]
const SLOT_HEAD: f32 = 34.0;
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
    pub gone: Option<Box<Editor>>,
    pub starting: bool,
    pub publish: bool,
    pub asking_withdraw: bool,
    pub lean: Option<Skill>,
    pub wholesale: bool,
    pub spread: bool,
}

impl Editor {
    pub fn at(id: String) -> Editor {
        Editor { id, selected: None, panel: Panel::Closed, query: String::new(), source: SourceTab::Search, collection: None, collection_page: 0, untouched: false, replace: false, share: false, choosing: false, marked: Vec::new(), bulk: None, asking_delete: false, grouped: true, last_mod: Mod::Nm, authors: String::new(), colour_draft: String::new(), last_category: String::new(), last_colour: None, category_creator: false, category_draft: String::new(), editing_authors: true, gone: None, starting: false, publish: false, asking_withdraw: false, lean: None, wholesale: false, spread: false }
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
    Makers(Option<String>),
    Begin(Start),
    PublishSheet(bool),
    Conflict(Keep),
    HideGuide,
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
    AskDelete(bool),
    DeletePool,
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
    ReadBest(bool),
    ReadPaste(String, Input, String),
    Suggest(u64, Arc<HashMap<String, Map>>, Mod, f64, HashSet<String>, Arc<AtomicBool>),
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
    pub screen: f32,
    pub panel: f32,
    pub gone: f32,
    pub today: i64,
    pub high: f32,
    pub remote: HashMap<String, f32>,
}

impl Clocks {
    pub fn settled() -> Clocks {
        Clocks { screen: SETTLED, panel: SETTLED, gone: SETTLED, today: 0, high: 0.0, remote: HashMap::new() }
    }

    fn of(&self, id: &str) -> f32 {
        self.remote.get(id).copied().unwrap_or(SETTLED)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Marks {
    screen: Option<(String, Instant)>,
    panel: Option<(&'static str, Instant)>,
    gone: Option<Instant>,
    remote: HashMap<String, Instant>,
    began: bool,
}

const WINDOW: f32 = 1.8;

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
        for item in &state.publications {
            self.remote.entry(item.id.clone()).or_insert(if self.began { now } else { long_ago });
        }
        self.began = true;
    }

    pub fn clocks(&self, now: Instant) -> Clocks {
        let age = |at: Instant| now.saturating_duration_since(at).as_secs_f32().min(SETTLED);
        match (&self.screen, &self.panel) {
            (Some((_, screen)), Some((_, panel))) => Clocks { screen: age(*screen), panel: age(*panel), gone: self.gone.map_or(SETTLED, age), today: 0, high: 0.0, remote: self.remote.iter().map(|(id, at)| (id.clone(), age(*at))).collect() },
            _ => Clocks::settled(),
        }
    }

    pub fn animating(&self, now: Instant) -> bool {
        let young = |at: &Instant| now.saturating_duration_since(*at).as_secs_f32() < WINDOW;
        self.screen.iter().any(|(_, at)| young(at)) || self.panel.iter().any(|(_, at)| young(at)) || self.gone.iter().any(young) || self.remote.values().any(young)
    }
}

#[derive(Clone)]
pub struct State {
    pub faces: HashMap<String, image::Handle>,
    pub makers: Option<String>,
    pub collection_shelf: bool,
    pub shelf: Shelf,
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
            faces: HashMap::new(),
            makers: None,
            collection_shelf: false,
            shelf: Shelf::Mine,
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

    fn remember(&mut self, at: usize) {
        if let Some(pool) = self.list.get(at) {
            self.undo.push((pool.id.clone(), pool.clone()));
            if self.undo.len() > 40 {
                self.undo.remove(0);
            }
        }
    }

    fn marked_slots(&self) -> Vec<usize> {
        match &self.screen {
            Screen::Editor(editor) => {
                let mut marked = editor.marked.clone();
                marked.sort_unstable();
                marked.dedup();
                marked
            }
            _ => Vec::new(),
        }
    }

    fn reorder_slots(&mut self, at: usize, order: &[usize], now: i64) {
        let fingerprint = self.list[at].fingerprint();
        let slots = self.list[at].slots.clone();
        self.list[at].slots = order.iter().map(|slot| slots[*slot].clone()).collect();
        self.follow_order(order);
        self.cancel_suggestion();
        if let Some(importing) = self.importing.as_mut().filter(|importing| importing.pool == self.list[at].id && importing.fingerprint == fingerprint) {
            importing.fingerprint = self.list[at].fingerprint();
        }
        self.save(at, now);
    }

    fn follow_order(&mut self, order: &[usize]) {
        let position = |old: usize| order.iter().position(|slot| *slot == old);
        let place = |place: &mut Place| {
            match *place {
                Place::Slot(old) => { if let Some(at) = position(old) { *place = Place::Slot(at); } }
                Place::Replace(old) => { if let Some(at) = position(old) { *place = Place::Replace(at); } }
                Place::End => {}
            }
        };
        if let Some(editor) = self.editor_mut() {
            editor.selected = editor.selected.and_then(position);
            editor.marked = editor.marked.iter().filter_map(|at| position(*at)).collect();
            editor.marked.sort_unstable();
        }
        match &mut self.finding {
            Some(Finding::Found(candidate)) => place(&mut candidate.place),
            Some(Finding::Fetching(fetching)) => { if let Some(target) = &mut fetching.place { place(target); } }
            _ => {}
        }
        if let Some(importing) = &mut self.importing {
            place(&mut importing.place);
        }
        if let Some(Notice::Already(old)) = &mut self.notice {
            if let Some(at) = position(*old) { *old = at; }
        }
    }

    fn save(&mut self, at: usize, now: i64) {
        if self.suggestions.as_ref().is_some_and(|suggestion| self.list.get(at).is_some_and(|pool| pool.id == suggestion.pool && pool.fingerprint() != suggestion.fingerprint)) {
            self.cancel_suggestion();
        }
        if let Some(pool) = self.list.get_mut(at) {
            pool.changed_at = now;
        }
        self.save_quiet(at);
        self.list.sort_by(|a, b| b.changed_at.cmp(&a.changed_at).then_with(|| a.id.cmp(&b.id)));
    }

    fn save_quiet(&mut self, at: usize) {
        if let Some(pool) = self.list.get(at) {
            match pools::save(&self.dir, pool) {
                Ok(()) => {
                    self.unsaved.remove(&pool.id);
                    self.save_errors.remove(&pool.id);
                }
                Err(why) => {
                    self.unsaved.insert(pool.id.clone(), Instant::now() + std::time::Duration::from_secs(5));
                    self.save_errors.insert(pool.id.clone(), why);
                }
            }
        }
    }

    fn defer_save(&mut self, at: usize, now: i64) {
        if let Some(pool) = self.list.get_mut(at) {
            pool.changed_at = now;
            self.unsaved.insert(pool.id.clone(), Instant::now() + std::time::Duration::from_millis(400));
        }
    }

    pub fn has_unsaved(&self) -> bool { !self.unsaved.is_empty() }

    pub fn save_failed(&self) -> bool { !self.save_errors.is_empty() }

    pub fn flush_saves(&mut self, force: bool) -> bool {
        let now = Instant::now();
        let due: Vec<_> = self.unsaved.iter().filter(|(_, at)| force || now >= **at).map(|(id, _)| id.clone()).collect();
        for id in due {
            if let Some(at) = self.list.iter().position(|pool| pool.id == id) {
                self.save_quiet(at);
            }
        }
        self.list.sort_by(|a, b| b.changed_at.cmp(&a.changed_at).then_with(|| a.id.cmp(&b.id)));
        !self.has_unsaved()
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

    fn resolve(&mut self, target: Target) -> Effect {
        self.resolve_request = self.resolve_request.wrapping_add(1);
        Effect::Resolve(self.resolve_request, target)
    }

    fn fetch(&mut self, hash: String, stop: Arc<AtomicBool>) -> Effect {
        self.fetch_request = self.fetch_request.wrapping_add(1);
        self.fetching = Some((self.fetch_request, hash.clone()));
        Effect::Fetch(self.fetch_request, hash, stop)
    }

    fn cancel_network(&mut self) {
        if let Some(Finding::Fetching(fetching)) = &self.finding {
            fetching.stop.store(true, Ordering::SeqCst);
        }
        if let Screen::Open(opening) = &self.screen {
            opening.stop.store(true, Ordering::SeqCst);
        }
        self.finding = None;
        self.fetching = None;
    }

    fn cancel_import(&mut self) {
        self.importing = None;
        self.importing_pool = None;
        self.dropped.clear();
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

    fn link_next(&mut self) -> Vec<Effect> {
        if self.finding.is_some() || self.importing.is_some() || self.editing().is_none() {
            return Vec::new();
        }
        match self.links.pop_front() {
            Some(link) => self.update(Message::Pasted(link), self.clock),
            None => {
                self.linking = false;
                Vec::new()
            }
        }
    }

    fn import_next(&mut self) -> Vec<Effect> {
        if self.importing.is_some() || self.editing().is_none() || matches!(self.finding, Some(Finding::Found(_) | Finding::Fetching(_))) {
            return Vec::new();
        }
        let Some(path) = self.dropped.pop_front() else { return Vec::new() };
        self.file_request = self.file_request.wrapping_add(1);
        let pool = self.editing().unwrap();
        self.importing = Some(FileImport { request: self.file_request, pool: pool.id.clone(), fingerprint: pool.fingerprint(), place: self.default_place() });
        if path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("osz")) {
            vec![Effect::ImportArchive(self.file_request, path)]
        } else {
            vec![Effect::ImportMap(self.file_request, path)]
        }
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
        self.suggestions = Some(Suggestions { request, pool: id, fingerprint, slot, mods, target, maps: target.is_none().then(Vec::new), stop: stop.clone() });
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
        match self.songs.clone() {
            Some(songs) => vec![Effect::Suggest(request, songs, mods, target, excluded, stop)],
            None => {
                self.pending_suggestion = Some(slot);
                if !self.reading {
                    self.reading = true;
                    vec![Effect::ReadSongs]
                } else {
                    Vec::new()
                }
            }
        }
    }

    pub fn update(&mut self, message: Message, now: i64) -> Vec<Effect> {
        let shown = match &self.screen {
            Screen::Editor(editor) if matches!(self.panel_key(), "panel" | "publish") => Some(editor.clone()),
            _ => None,
        };
        self.clock = now;
        let before = self.guide;
        if self.guide.on {
            match &message {
                Message::SetMod(..) | Message::BulkMod(..) | Message::Category(..) => self.guide.modded = true,
                Message::Move(..) => self.guide.moved = true,
                Message::Share(true) | Message::Published(_, Ok(_)) => self.guide.shared = true,
                _ => {}
            }
        }
        self.depth += 1;
        let mut effects = std::mem::take(&mut self.pending);
        effects.extend(self.handle(message, now));
        self.depth -= 1;
        if self.guide.on && self.editing().is_some_and(|pool| !pool.collection && self.guide.steps(pool).iter().all(|done| *done)) {
            self.guide.on = false;
        }
        if self.depth == 0 && self.guide != before {
            effects.push(Effect::Guide(self.guide));
        }
        if let (Some(was), "none") = (shown, self.panel_key()) {
            if let Some(editor) = self.editor_mut().filter(|editor| editor.id == was.id) {
                editor.gone = Some(Box::new(Editor { gone: None, ..was }));
            }
        }
        let filled = self.editing().is_some_and(|pool| pool.filled() > 0);
        let open = self.panel_key() != "none";
        if let Some(editor) = self.editor_mut().filter(|editor| editor.starting && (filled || open)) {
            editor.starting = false;
        }
        if self.depth == 0 {
            if !matches!(&self.screen, Screen::Editor(editor) if editor.panel == Panel::Add) {
                self.links.clear();
                self.linking = false;
                if let Some(editor) = self.editor_mut() {
                    editor.lean = None;
                }
            }
            effects.extend(self.link_next());
        }
        effects
    }

    fn handle(&mut self, message: Message, now: i64) -> Vec<Effect> {
        let mut effects = Vec::new();
        match message {
            Message::FlushSaves => { self.flush_saves(false); }
            Message::RetrySaves => { self.flush_saves(true); }
            Message::AuthorsDraft(said) => {
                let apply = said.ends_with(char::is_whitespace) && said.chars().filter(|ch| *ch == '"').count() % 2 == 0;
                if let Some(editor) = self.editor_mut() { editor.authors = said; }
                if apply { effects.extend(self.update(Message::ApplyAuthors, now)); }
            }
            Message::ApplyAuthors => { if let Screen::Editor(editor) = &self.screen { effects.extend(self.update(Message::Authors(editor.authors.clone()), now)); } if let Some(editor) = self.editor_mut() { editor.editing_authors = false; } }
            Message::EditAuthors => { if let Some(editor) = self.editor_mut() { editor.editing_authors = true; } }
            Message::OwnCompiler(id, name) => {
                if let Some(at) = self.list.iter().position(|pool| pool.id == id && !name.is_empty() && pool.compiler != name) {
                    self.list[at].compiler = name.clone(); self.save(at, now);
                    effects.push(Effect::Faces(vec![name]));
                }
            }
            Message::Face(name, face) => { if let Some(face) = face { self.faces.insert(name.to_lowercase(), face); } }
            Message::MoveCategory(name, down) => {
                if let Some(at) = self.editing_at() {
                    let mut order = category_order(&self.list[at]);
                    if let Some(from) = order.iter().position(|key| key == &name) {
                        let to = if down { from.saturating_add(1) } else { from.saturating_sub(1) };
                        if to < order.len() && to != from { self.remember(at); order.swap(from, to); self.list[at].category_order = order; self.save(at, now); }
                    }
                }
            }
            Message::Begin(way) => {
                if let Some(editor) = self.editor_mut() { editor.starting = false; }
                if let Some(at) = self.editing_at() {
                    if self.list[at].frame == Frame::Free && self.list[at].slots.is_empty() {
                        self.list[at].slots = vec![Slot::empty(Mod::Nm); 4];
                        self.save(at, now);
                    }
                }
                match way {
                    Start::Empty => {}
                    Start::Links => {
                        effects.extend(self.update(Message::AddPanel(true), now));
                        if let Some(pool) = self.editing() { effects.push(Effect::ReadPaste(pool.id.clone(), Input::Query, String::new())); }
                    }
                    Start::Collections => {
                        effects.extend(self.update(Message::AddPanel(true), now));
                        effects.extend(self.update(Message::Source(SourceTab::Collections), now));
                        if let Some(editor) = self.editor_mut() { editor.wholesale = true; }
                    }
                    Start::Best => {
                        if let Some(editor) = self.editor_mut() { editor.spread = true; }
                        if self.best.is_none() {
                            effects.push(Effect::ReadBest(false));
                        }
                        effects.extend(self.spread_best(now));
                    }
                }
            }
            Message::Makers(asked) => { self.makers = if asked == self.makers { None } else { asked }; }
            Message::Catalogue(collections) => {
                self.makers = None;
                self.collection_shelf = collections;
                self.screen = Screen::Shelf;
                self.publications.clear();
                self.catalogue_error = None;
                self.catalogue_loading = true;
                effects.push(Effect::Catalogue(collections, 0));
                if collections && self.collections.is_none() && !self.collecting { self.collecting = true; effects.push(Effect::ReadCollections); }
                if collections && self.songs.is_none() && !self.reading { self.reading = true; effects.push(Effect::ReadSongs); }
            }
            Message::Shelf(tab) => {
                self.makers = None;
                self.shelf = tab;
                if self.collection_shelf || self.screen != Screen::Shelf {
                    effects.extend(self.update(Message::Catalogue(false), now));
                }
            }
            Message::SavePublication(id) => {
                let held = self.list.iter().any(|pool| pool.saved.as_ref().is_some_and(|saved| saved.id == id));
                if let Some(publication) = self.publications.iter().find(|row| row.id == id && row.kind == "pool" && !row.mine).filter(|_| !held).cloned() {
                    if let Some(mut pool) = serde_json::to_vec(&publication.content).ok().and_then(|bytes| pool_share::from_file(&bytes, now).ok()) {
                        pool.published_revision = 0;
                        pool.saved = Some(pools::Saved { id: publication.id.clone(), code: publication.code.clone(), revision: publication.revision, publisher: pool.compiler.clone() });
                        self.list.push(pool);
                        self.save(self.list.len() - 1, now);
                        effects.extend(self.measure_effects());
                        if self.songs.is_none() && !self.reading { self.reading = true; effects.push(Effect::ReadSongs); }
                    }
                }
            }
            Message::CatalogueMore => {
                if !self.catalogue_loading { self.catalogue_loading = true; effects.push(Effect::Catalogue(self.collection_shelf, self.publications.len())); }
            }
            Message::CatalogueLoaded(collections, append, result) => {
                if collections == self.collection_shelf {
                    self.catalogue_loading = false;
                    match result {
                        Ok(rows) => {
                            let names: Vec<_> = rows.iter().flat_map(publication_authors).filter(|name| !self.faces.contains_key(&name.to_lowercase())).collect();
                            if !names.is_empty() { effects.push(Effect::Faces(names)); }
                            self.catalogue_more = rows.len() == 100; if !append { self.publications.clear(); } self.publications.extend(rows); self.catalogue_error = None;
                            if let Some((id, keep)) = self.resolving.take() {
                                effects.extend(self.settle_conflict(&id, keep, now));
                            }
                        }
                        Err(why) => {
                            self.catalogue_error = Some(why);
                            self.resolving = None;
                        }
                    }
                }
            }
            Message::HideGuide => self.guide.on = false,
            Message::Lean(skill) => {
                effects.extend(self.update(Message::AddPanel(true), now));
                if let Some(editor) = self.editor_mut() {
                    editor.lean = Some(skill);
                }
                effects.extend(self.update(Message::Source(SourceTab::Suggest), now));
            }
            Message::PublishSheet(open) => {
                if open {
                    effects.extend(self.update(Message::Choosing(false), now));
                    self.catalogue_error = None;
                }
                if let Some(editor) = self.editor_mut().filter(|editor| !editor.starting) {
                    editor.publish = open;
                    editor.asking_withdraw = false;
                    if open {
                        editor.panel = Panel::Closed;
                        editor.share = false;
                    }
                }
            }
            Message::ShowSlot(at) => {
                effects.extend(self.update(Message::PublishSheet(false), now));
                effects.extend(self.update(Message::Select(Some(at)), now));
            }
            Message::AskWithdraw(asking) => {
                if let Some(editor) = self.editor_mut() {
                    editor.asking_withdraw = asking;
                }
            }
            Message::Withdraw => {
                if !self.publishing {
                    if let Some(pool) = self.editing().filter(|pool| pool.published_revision > 0).cloned() {
                        self.publishing = true;
                        self.catalogue_error = None;
                        effects.push(Effect::Withdraw(pool));
                    }
                }
            }
            Message::Withdrawn(id, result) => {
                self.publishing = false;
                match result {
                    Ok(()) => {
                        if let Some(at) = self.list.iter().position(|pool| pool.id == id) { self.list[at].published_revision = 0; self.save(at, now); }
                        self.publications.retain(|row| !(row.mine && row.local_id == id));
                        self.catalogue_error = None;
                        if let Some(editor) = self.editor_mut().filter(|editor| editor.id == id) { editor.publish = false; editor.asking_withdraw = false; }
                        effects.push(Effect::Say("catalogue-withdrawn"));
                    }
                    Err(why) => {
                        self.catalogue_error = Some(why);
                        if let Some(editor) = self.editor_mut() { editor.asking_withdraw = false; }
                    }
                }
            }
            Message::Publish => {
                if !self.publishing {
                    if let Some(pool) = self.editing().filter(|pool| publish_ready(pool)).cloned() {
                        self.publishing = true;
                        self.catalogue_error = None;
                        effects.push(Effect::Publish(pool));
                    }
                }
            }
            Message::Published(id, result) => {
                self.publishing = false;
                match result {
                    Ok(publication) => {
                        if let Some(at) = self.list.iter().position(|pool| pool.id == id) { self.list[at].published_revision = publication.revision; self.save(at, now); }
                        if self.conflict.as_deref() == Some(id.as_str()) { self.conflict = None; }
                        self.publications.retain(|p| p.id != publication.id);
                        self.publications.insert(0, publication);
                        self.catalogue_error = None;
                        if let Some(editor) = self.editor_mut().filter(|editor| editor.id == id && editor.publish) {
                            editor.publish = false;
                            effects.push(Effect::Say("catalogue-published-now"));
                        }
                    }
                    Err(why) if why == "409" => {
                        self.catalogue_error = None;
                        self.conflict = Some(id.clone());
                        let collection = self.list.iter().find(|pool| pool.id == id).is_some_and(|pool| pool.collection);
                        if let Some(editor) = self.editor_mut().filter(|editor| editor.id == id) { editor.publish = false; }
                        effects.extend(self.refresh_for(collection));
                    }
                    Err(why) => self.catalogue_error = Some(why),
                }
            }
            Message::Conflict(keep) => {
                if let Some(pool) = self.editing().filter(|pool| self.conflict.as_deref() == Some(pool.id.as_str()) && self.resolving.is_none()) {
                    let (id, collection) = (pool.id.clone(), pool.collection);
                    self.resolving = Some((id, keep));
                    effects.extend(self.refresh_for(collection));
                }
            }
            Message::OpenPublication(id) => {
                self.makers = None;
                if let Some(publication) = self.publications.iter().find(|row| row.id == id).cloned() {
                    let parsed = if publication.kind == "collection" {
                        let collection = Collection { name: publication.name.clone(), hashes: publication.content["hashes"].as_array().into_iter().flatten().filter_map(|h| h.as_str().map(str::to_owned)).collect() };
                        Some(collection_pool(&collection, self.songs.as_deref(), now))
                    } else { serde_json::to_vec(&publication.content).ok().and_then(|bytes| pool_share::from_file(&bytes, now).ok()) };
                    if let Some(mut pool) = parsed {
                        if publication.mine { pool.id = publication.local_id; pool.published_revision = publication.revision; }
                        if let Some(at) = self.list.iter().position(|item| item.id == pool.id && item.published_revision != pool.published_revision) {
                            let mut draft = self.list[at].clone();
                            draft.id = Pool::new(Frame::Free, "", now).id;
                            draft.published_revision = 0;
                            self.list[at] = pool.clone();
                            self.save(at, now);
                            self.list.push(draft);
                            self.save(self.list.len() - 1, now);
                        }
                        if let Some(existing) = self.list.iter().find(|item| item.id == pool.id) { effects.extend(self.update(Message::Open(existing.id.clone()), now)); }
                        else { let id = pool.id.clone(); self.list.push(pool); self.save(self.list.len() - 1, now); effects.extend(self.update(Message::Open(id), now)); }
                    }
                }
            }
            Message::ImportCollection(at) => {
                if let Some(collection) = self.collections.as_ref().and_then(|collections| collections.get(at)) {
                    let pool = collection_pool(collection, self.songs.as_deref(), now);
                    let id = pool.id.clone(); self.list.push(pool); self.save(self.list.len() - 1, now);
                    effects.extend(self.update(Message::Open(id), now));
                }
            }
            Message::New => {
                self.makers = None;
                self.cancel_network();
                self.cancel_suggestion();
                self.cancel_import();
                let mut pool = Pool::new(Frame::Free, "", now);
                pool.collection = self.collection_shelf;
                pool.slots = vec![Slot::empty(Mod::Nm); 4];
                let id = pool.id.clone();
                self.list.insert(0, pool);
                self.save(0, now);
                effects.push(Effect::OwnCompiler(id.clone()));
                let starting = !self.collection_shelf;
                self.screen = Screen::Editor(Editor { untouched: true, starting, ..Editor::at(id) });
            }
            Message::Open(id) => {
                self.makers = None;
                if self.list.iter().any(|pool| pool.id == id) {
                    self.cancel_network();
                    self.cancel_suggestion();
                    self.cancel_import();
                    self.screen = Screen::Editor(Editor::at(id.clone()));
                    let authors = self.editing().map(|pool| pool.authors.iter().map(|name| if name.contains(' ') { format!("\"{name}\"") } else { name.clone() }).collect::<Vec<_>>().join(" ")).unwrap_or_default();
                    if let Some(editor) = self.editor_mut() { editor.editing_authors = authors.is_empty(); editor.authors = authors; }
                    effects.push(Effect::OwnCompiler(id.clone()));
                    if let Some(pool) = self.editing() { effects.push(Effect::Faces(pool.authors.iter().cloned().chain(std::iter::once(pool.compiler.clone())).filter(|name| !name.is_empty()).collect())); }
                    if self.songs.is_none() && !self.reading {
                        self.reading = true;
                        effects.push(Effect::ReadSongs);
                    }
                }
            }
            Message::Back => {
                if !self.flush_saves(true) { return effects; }
                self.cancel_network();
                self.cancel_import();
                self.cancel_suggestion();
                if let (Some(at), Screen::Editor(editor)) = (self.editing_at(), self.screen.clone()) {
                    if editor.untouched && self.list[at].filled() == 0 && self.list[at].name.is_empty() {
                        let id = self.list.remove(at).id;
                        let _ = pools::remove(&self.dir, &id);
                    }
                }
                if let Screen::Open(opening) = &self.screen {
                    opening.stop.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                self.screen = Screen::Shelf;
                self.notice = None;
                self.refused = None;
            }
            Message::Rename(name) => {
                if let Some(at) = self.editing_at() {
                    self.list[at].name = name;
                    if let Some(editor) = self.editor_mut() {
                        editor.untouched = false;
                    }
                    self.defer_save(at, now);
                }
            }
            Message::Authors(said) => {
                if let Some(at) = self.editing_at() {
                    let mut authors = Vec::new();
                    for author in author_names(&said) {
                        if !authors.iter().any(|known: &String| known.eq_ignore_ascii_case(&author)) { authors.push(author); }
                    }
                    self.list[at].authors = authors;
                    effects.push(Effect::Faces(self.list[at].authors.clone()));
                    if let Some(editor) = self.editor_mut() { editor.authors = said; editor.untouched = false; }
                    self.save(at, now);
                }
            }
            Message::Category(slot, name) => {
                if let Some(at) = self.editing_at() {
                    let colour = self.list[at].categories.get(&name).copied().or_else(|| self.list[at].slots.iter().find(|slot| slot.category == name).and_then(|slot| slot.colour));
                    if let Some(editor) = self.editor_mut() { editor.last_category = name.clone(); editor.last_colour = colour; }
                }
                if let Some(at) = self.editing_at().filter(|at| slot < self.list[*at].slots.len()) {
                    self.remember(at);
                    self.list[at].slots[slot].category = name.clone();
                    if let Some(colour) = self.list[at].categories.get(&name).copied().or_else(|| self.list[at].slots.iter().find(|item| item.category == name && item.colour.is_some()).and_then(|item| item.colour)) { self.list[at].slots[slot].colour = Some(colour); }
                    if let Some(editor) = self.editor_mut() { editor.last_category = name; }
                    self.save(at, now);
                }
            }
            Message::Author(name) => { self.makers = None; effects.push(Effect::Author(name)); }
            Message::CreateCategory(open) => {
                if let Some(editor) = self.editor_mut() { editor.category_creator = open; editor.category_draft.clear(); editor.colour_draft = "#5ec2d0".into(); }
            }
            Message::CategoryDraft(name) => {
                if let Some(editor) = self.editor_mut() { editor.category_draft = name.chars().filter(|c| c.is_alphabetic()).flat_map(char::to_uppercase).take(2).collect(); }
            }
            Message::SaveCategory(slot) => {
                let draft = match &self.screen { Screen::Editor(editor) => Some((editor.category_draft.clone(), editor.colour_draft.clone())), _ => None };
                if let Some((name, colour)) = draft.filter(|(name, colour)| !name.is_empty() && !Mod::ALL.iter().any(|m| m.code() == name) && parse_colour(colour).is_some()) {
                    if let Some(at) = self.editing_at() { self.remember(at); self.list[at].categories.insert(name.clone(), parse_colour(&colour).unwrap()); self.save(at, now); }
                    if let Some(editor) = self.editor_mut() { editor.category_creator = false; }
                    effects.extend(self.update(Message::Category(slot, name), now));
                    effects.extend(self.update(Message::Colour(slot, colour), now));
                    if let Some(editor) = self.editor_mut() { editor.category_creator = false; }
                }
            }
            Message::Colour(slot, value) => {
                if let Some(editor) = self.editor_mut() { editor.colour_draft = value.clone(); }
                if matches!(&self.screen, Screen::Editor(editor) if editor.category_creator) { return effects; }
                if let (Some(at), Some(colour)) = (self.editing_at(), parse_colour(&value)) {
                    if let Some(category) = self.list[at].slots.get(slot).map(|slot| slot.category.clone()).filter(|category| !category.is_empty()) {
                        self.remember(at);
                        self.list[at].categories.insert(category.clone(), colour);
                        for slot in &mut self.list[at].slots { if slot.category == category { slot.colour = Some(colour); } }
                        if let Some(editor) = self.editor_mut() { editor.last_colour = Some(colour); }
                        self.save(at, now);
                    }
                }
            }
            Message::UseFrame(frame) => {
                if let Some(at) = self.editing_at() {
                    if self.list[at].filled() == 0 {
                        self.list[at].frame = frame;
                        self.list[at].slots = frame.mods().into_iter().map(Slot::empty).collect();
                        if let Some(editor) = self.editor_mut() {
                            editor.selected = None;
                        }
                        self.save(at, now);
                    }
                }
            }
            Message::Select(slot) => {
                self.cancel_suggestion();
                self.notice = None;
                let empty = slot.is_some_and(|at| self.editing().and_then(|pool| pool.slots.get(at)).is_some_and(Slot::is_empty));
                let mods = slot.and_then(|at| self.editing()?.slots.get(at).map(|slot| slot.mods));
                let category = slot.and_then(|at| self.editing()?.slots.get(at).map(|slot| (slot.category.clone(), slot.colour, colour_hex(slot_colour(slot)))));
                if let Some(editor) = self.editor_mut() {
                    if let Some(mods) = mods { editor.last_mod = mods; }
                    if let Some((name, colour, hex)) = category { editor.last_category = name; editor.last_colour = colour; editor.colour_draft = hex; }
                    editor.selected = slot;
                    editor.replace = false;
                    editor.panel = if empty { Panel::Add } else if slot.is_some() { Panel::Slot } else { Panel::Closed };
                }
                if empty && self.songs.is_none() && !self.reading {
                    self.reading = true;
                    effects.push(Effect::ReadSongs);
                }
                let place = self.default_place();
                if let Some(Finding::Found(candidate)) = &mut self.finding { candidate.place = place; }
            }
            Message::Replace => {
                self.notice = None;
                if let Some(editor) = self.editor_mut() {
                    editor.replace = editor.selected.is_some();
                    editor.panel = Panel::Add;
                }
                if self.songs.is_none() && !self.reading {
                    self.reading = true;
                    effects.push(Effect::ReadSongs);
                }
                let place = self.default_place();
                if let Some(Finding::Found(candidate)) = &mut self.finding { candidate.place = place; }
            }
            Message::ReplaceAt(slot) => {
                if self.editing().is_some_and(|pool| slot < pool.slots.len()) {
                    effects.extend(self.update(Message::Select(Some(slot)), now));
                    effects.extend(self.update(Message::Replace, now));
                }
            }
            Message::Grouped(on) => {
                if let Some(editor) = self.editor_mut() {
                    editor.grouped = on;
                }
            }
            Message::SetMod(slot, mods) => {
                if self.editing().is_some_and(|pool| slot < pool.slots.len()) {
                    if let Some(editor) = self.editor_mut() { editor.last_mod = mods; editor.last_category.clear(); editor.last_colour = None; }
                }
                if let Some(at) = self.editing_at() {
                    if self.list[at].slots.get(slot).is_some_and(|entry| entry.mods != mods || !entry.category.is_empty()) {
                        self.remember(at);
                        self.list[at].slots[slot].mods = mods;
                        self.list[at].slots[slot].category.clear();
                        self.list[at].slots[slot].colour = None;
                        self.list[at].slots[slot].measure = None;
                        self.save(at, now);
                        effects.extend(self.measure_effects());
                    }
                }
            }
            Message::AddMod(mods) => {
                if let Some(editor) = self.editor_mut() { editor.last_mod = mods; editor.last_category.clear(); editor.last_colour = None; }
                if let Some(slot) = self.editing().and_then(|pool| match &self.screen {
                    Screen::Editor(editor) => target(pool, editor),
                    _ => None,
                }) {
                    effects.extend(self.update(Message::SetMod(slot, mods), now));
                }
            }
            Message::Clear(slot) => {
                if let Some(at) = self.editing_at() {
                    self.remember(at);
                    let pool = &mut self.list[at];
                    if slot < pool.slots.len() {
                        if pool.frame == Frame::Free {
                            pool.slots.remove(slot);
                        } else {
                            pool.slots[slot].clear();
                        }
                        if let Some(editor) = self.editor_mut() {
                            editor.selected = None;
                            editor.panel = Panel::Closed;
                        }
                        self.save(at, now);
                    }
                }
            }
            Message::AddPanel(open) => {
                if open {
                    if let Some(at) = self.editing_at() {
                        let empty = match &self.screen {
                            Screen::Editor(editor) => editor.selected.filter(|slot| self.list[at].slots.get(*slot).is_some_and(Slot::is_empty)),
                            _ => None,
                        }.or_else(|| self.list[at].first_empty());
                        let slot = match empty {
                            Some(slot) => slot,
                            None => {
                                self.remember(at);
                                let slot = self.list[at].slots.len();
                                let mods = match &self.screen { Screen::Editor(editor) => editor.last_mod, _ => Mod::Nm };
                                self.list[at].slots.push(Slot::empty(mods));
                                self.save(at, now);
                                slot
                            }
                        };
                        let automatic = matches!(&self.screen, Screen::Editor(editor) if editor.selected != Some(slot));
                        if automatic && self.list[at].frame == Frame::Free {
                            let mods = match &self.screen { Screen::Editor(editor) => editor.last_mod, _ => Mod::Nm };
                            let (category, colour) = match &self.screen { Screen::Editor(editor) => (editor.last_category.clone(), editor.last_colour), _ => (String::new(), None) };
                            if self.list[at].slots[slot].mods != mods || self.list[at].slots[slot].category != category || self.list[at].slots[slot].colour != colour {
                                self.remember(at);
                                self.list[at].slots[slot].mods = mods;
                                self.list[at].slots[slot].category = category;
                                self.list[at].slots[slot].colour = colour;
                                self.list[at].slots[slot].measure = None;
                                self.save(at, now);
                            }
                        }
                        if let Some(editor) = self.editor_mut() { editor.selected = Some(slot); }
                    }
                }
                if !open {
                    self.cancel_suggestion();
                }
                self.notice = None;
                if let Some(editor) = self.editor_mut() {
                    editor.replace = false;
                    editor.panel = if open { Panel::Add } else if editor.selected.is_some() { Panel::Slot } else { Panel::Closed };
                }
                if open && self.songs.is_none() && !self.reading {
                    self.reading = true;
                    effects.push(Effect::ReadSongs);
                }
                let place = self.default_place();
                if let Some(Finding::Found(candidate)) = &mut self.finding { candidate.place = place; }
            }
            Message::Source(source) => {
                if source == SourceTab::Suggest {
                    if let Some(slot) = self.editing().and_then(|pool| match &self.screen {
                        Screen::Editor(editor) => editor.selected.filter(|slot| pool.slots.get(*slot).is_some_and(Slot::is_empty)).or_else(|| pool.first_empty()),
                        _ => None,
                    }) {
                        effects.extend(self.suggest_slot(slot));
                    }
                } else {
                    self.cancel_suggestion();
                }
                if let Some(editor) = self.editor_mut() {
                    editor.panel = Panel::Add;
                    editor.source = source;
                }
                if let Some(Finding::Fetching(fetching)) = &self.finding {
                    fetching.stop.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                self.finding = None;
                self.notice = None;
                if source == SourceTab::Collections && !self.collecting {
                    self.collecting = true;
                    effects.push(Effect::ReadCollections);
                }
                if source == SourceTab::Best && self.best.is_none() {
                    effects.push(Effect::ReadBest(false));
                }
            }
            Message::RefreshBest => effects.push(Effect::ReadBest(true)),
            Message::Best(at) => {
                if matches!(self.finding, Some(Finding::Fetching(_))) {
                    return effects;
                }
                let Some(score) = self.best.as_ref().and_then(|scores| scores.get(at)).cloned() else { return effects };
                let Some(target) = score_target(&score) else { return effects };
                let hash = score.hash.to_ascii_lowercase();
                if self.songs.as_ref().is_some_and(|songs| songs.contains_key(&hash)) {
                    self.finding = None;
                    effects.extend(self.put(&hash, now));
                } else {
                    let query = match &target {
                        Target::Hash(hash) => hash.clone(),
                        Target::Beatmap { id, .. } => format!("https://osu.ppy.sh/beatmaps/{id}"),
                        Target::Set { .. } => unreachable!(),
                    };
                    if let Some(editor) = self.editor_mut() {
                        editor.query = query;
                    }
                    self.notice = None;
                    self.finding = Some(Finding::Asking);
                    effects.push(self.resolve(target));
                }
            }
            Message::Collection(collection) => {
                if let Some(editor) = self.editor_mut() {
                    editor.collection = collection;
                    editor.collection_page = 0;
                }
                if let Some(Finding::Fetching(fetching)) = &self.finding {
                    fetching.stop.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                self.finding = None;
                let whole = matches!(&self.screen, Screen::Editor(editor) if editor.wholesale);
                if let (true, Some(at)) = (whole, collection) {
                    if let Some(editor) = self.editor_mut() { editor.wholesale = false; editor.last_mod = Mod::Nm; }
                    let held: HashSet<String> = self.editing().map(|pool| pool.slots.iter().filter_map(|slot| slot.hash.clone()).collect()).unwrap_or_default();
                    let mut seen = HashSet::new();
                    let queue: Vec<String> = self.collections.as_ref().and_then(|all| all.get(at)).map(|chosen| chosen.hashes.iter().filter(|hash| !held.contains(*hash) && seen.insert((*hash).clone())).take(WHOLESALE_MOST).cloned().collect()).unwrap_or_default();
                    if !queue.is_empty() {
                        let total = queue.len();
                        self.finding = Some(Finding::Fetching(Fetching { queue, total, step: None, place: None, stop: Arc::new(std::sync::atomic::AtomicBool::new(false)) }));
                        effects.extend(self.advance(now));
                    }
                }
            }
            Message::CollectionPage(page) => {
                if let Some(editor) = self.editor_mut() {
                    editor.collection_page = page;
                }
            }
            Message::CollectionHash(hash) => {
                if self.songs.as_ref().is_some_and(|songs| songs.contains_key(&hash)) {
                    effects.extend(self.put(&hash, now));
                } else if pool_links::parse(&hash).is_ok() {
                    self.finding = Some(Finding::Asking);
                    effects.push(self.resolve(Target::Hash(hash)));
                }
            }
            Message::Collections(collections) => {
                self.collections = Some(collections);
                self.collecting = false;
            }
            Message::Suggest(slot) => effects.extend(self.suggest_slot(slot)),
            Message::Suggested(request, maps) => {
                let current = self.suggestions.as_ref().filter(|suggestion| suggestion.request == request).map(|suggestion| (suggestion.pool.clone(), suggestion.fingerprint.clone(), suggestion.slot, suggestion.mods));
                if let Some((id, fingerprint, slot, mods)) = current {
                    let valid = self.editing().is_some_and(|pool| pool.id == id && pool.fingerprint() == fingerprint && pool.slots.get(slot).is_some_and(|entry| entry.is_empty() && entry.mods == mods));
                    if valid {
                        for (hash, measure) in &maps {
                            self.measures.put(hash, mods, Ok(*measure));
                        }
                        let lean = match &self.screen { Screen::Editor(editor) => editor.lean, _ => None };
                        if let Some(suggestion) = &mut self.suggestions {
                            suggestion.maps = Some(pools::leaning(maps, lean));
                        }
                    }
                }
            }
            Message::Query(query) => {
                self.notice = None;
                let text = query.trim().to_owned();
                if let Some(editor) = self.editor_mut() {
                    editor.query = query;
                }
                effects.extend(self.look_at(&text));
            }
            Message::PasteInto(input, contents) => {
                if let Some(pool) = self.editing() {
                    effects.push(Effect::ReadPaste(pool.id.clone(), input, contents));
                }
            }
            Message::PastedInto(id, input, contents, pasted) => {
                if self.editing().is_some_and(|pool| pool.id == id) {
                    let message = if is_paste(&pasted) || (matches!(input, Input::Query) && !links_in(&pasted).is_empty()) {
                        Message::Pasted(pasted)
                    } else {
                        match input {
                            Input::Name => Message::Rename(contents),
                            Input::Query => Message::Query(contents),
                            Input::Note(slot) => Message::Note(slot, contents),
                        }
                    };
                    effects.extend(self.update(message, now));
                }
            }
            Message::Pasted(text) if pool_share::is_text(&text) => {
                match pool_share::from_text(&text, now) {
                    Ok(pool) => effects.extend(self.begin_open(pool)),
                    Err(refused) => self.refused = Some(refused),
                }
            }
            Message::Pasted(text) => {
                let mut rest = links_in(&text);
                let text = if rest.is_empty() { text.trim().to_owned() } else { rest.remove(0) };
                let linkish = match pool_links::parse(&text) {
                    Ok(_) | Err(pool_links::Refusal::Mode(_)) => true,
                    Err(pool_links::Refusal::NotALink) => false,
                };
                if self.editing_at().is_none() {
                    if !linkish {
                        return effects;
                    }
                    self.update(Message::New, now);
                }
                if text.is_empty() {
                    return effects;
                }
                self.cancel_suggestion();
                self.notice = None;
                if let Some(editor) = self.editor_mut() {
                    editor.panel = Panel::Add;
                    editor.source = SourceTab::Search;
                    editor.replace = false;
                    editor.query = text.clone();
                }
                effects.extend(self.update(Message::AddPanel(true), now));
                if !rest.is_empty() {
                    self.links = rest.into();
                    self.linking = true;
                }
                if self.songs.is_none() && !self.reading {
                    self.reading = true;
                    effects.push(Effect::ReadSongs);
                }
                effects.extend(self.look_at(&text));
            }
            Message::Resolved(request, said) => {
                if request == self.resolve_request && matches!(self.finding, Some(Finding::Asking)) {
                    match said {
                        Ok(found) => {
                            for score in self.best.iter_mut().flatten() {
                                if let Some(difficulty) = found.difficulties.iter().find(|difficulty| score_id(score) == Some(difficulty.id)) {
                                    score.hash = difficulty.hash.clone();
                                }
                            }
                            let choice = found.picked.or_else(|| (found.difficulties.len() == 1).then_some(0));
                            let place = self.default_place();
                            effects.push(Effect::Cover(found.cover(), found.set));
                            self.finding = Some(Finding::Found(Candidate { found, choice, place, cover: None }));
                            if self.linking && choice.is_some() {
                                effects.extend(self.confirm(false, now));
                            }
                        }
                        Err(Why::Nowhere) => self.finding = Some(Finding::Missing),
                        Err(Why::Mode(mode)) => self.finding = Some(Finding::Refused(mode)),
                        Err(Why::Silent(why)) => self.finding = Some(Finding::Silent(why)),
                    }
                }
            }
            Message::Cover(set, handle) => {
                if let Some(Finding::Found(candidate)) = &mut self.finding {
                    if candidate.found.set == set {
                        candidate.cover = handle;
                    }
                }
            }
            Message::Choose(at) => {
                if let Some(Finding::Found(candidate)) = &mut self.finding {
                    if at < candidate.found.difficulties.len() {
                        candidate.choice = Some(at);
                    }
                }
            }
            Message::Aim(place) => {
                if let Some(Finding::Found(candidate)) = &mut self.finding {
                    candidate.place = place;
                }
            }
            Message::Confirm => effects.extend(self.confirm(false, now)),
            Message::ConfirmAll => effects.extend(self.confirm(true, now)),
            Message::Dismiss => {
                self.cancel_network();
                self.notice = None;
                effects.extend(self.import_next());
            }
            Message::FetchSlot(at) => {
                let wanted = self.editing().and_then(|pool| pool.slots.get(at)).and_then(|slot| slot.hash.clone());
                if let (Some(hash), false) = (wanted, matches!(self.finding, Some(Finding::Fetching(_)))) {
                    self.fetched.remove(&hash);
                    self.finding = Some(Finding::Fetching(Fetching { queue: vec![hash], total: 1, step: None, place: Some(Place::Slot(at)), stop: Arc::new(std::sync::atomic::AtomicBool::new(false)) }));
                    effects.extend(self.advance(now));
                }
            }
            Message::Retry => {
                let text = match &self.screen {
                    Screen::Editor(editor) => editor.query.trim().to_owned(),
                    Screen::Shelf | Screen::Open(_) => String::new(),
                };
                self.finding = None;
                effects.extend(self.look_at(&text));
            }
            Message::Step(request, hash, step) => {
                if self.fetching.as_ref() == Some(&(request, hash.clone())) {
                    if matches!(step, crate::maps::Step::Done(_) | crate::maps::Step::Nowhere | crate::maps::Step::Failed(_) | crate::maps::Step::Stopped) {
                        self.fetching = None;
                    }
                    effects.extend(self.stepped(&hash, step, now));
                }
            }
            Message::Share(open) => {
                if let Some(editor) = self.editor_mut() {
                    editor.share = open;
                }
            }
            Message::CopyText => {
                if let Some(pool) = self.editing() {
                    effects.push(Effect::Copy(pool_share::to_text(pool), "pool-copied"));
                }
            }
            Message::CopyHash => {
                if let Some(pool) = self.editing() {
                    effects.push(Effect::Copy(pool.fingerprint(), "pool-copied-hash"));
                }
            }
            Message::SaveImage => {
                if self.exporting.is_none() {
                    if let Some(pool) = self.editing().filter(|pool| pool.filled() > 0).cloned() {
                        self.export_request = self.export_request.wrapping_add(1);
                        self.exporting = Some(self.export_request);
                        effects.push(Effect::SaveImage(self.export_request, pool));
                    }
                }
            }
            Message::ImageSaved(request, said) => {
                if self.exporting == Some(request) {
                    self.exporting = None;
                    match said {
                        Ok(Some(_)) => effects.push(Effect::Say("pool-image-saved")),
                        Err(_) => effects.push(Effect::Say("pool-save-failed")),
                        Ok(None) => {}
                    }
                }
            }
            Message::SaveFile => {
                if let Some(pool) = self.editing() {
                    effects.push(Effect::SaveFile(file_name_of(pool), pool_share::to_file(pool)));
                }
            }
            Message::OpenFile => effects.push(Effect::PickFile),
            Message::Imported(said) => match said {
                Ok(pool) => effects.extend(self.begin_open(pool)),
                Err(refused) => self.refused = Some(refused),
            },
            Message::Dropped(path) => {
                if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("pool")) {
                    if let Some(Finding::Fetching(fetching)) = &self.finding {
                        fetching.stop.store(true, Ordering::SeqCst);
                    }
                    self.finding = None;
                    self.update(Message::Back, now);
                    self.file_request = self.file_request.wrapping_add(1);
                    self.importing_pool = Some(self.file_request);
                    effects.push(Effect::ReadPool(self.file_request, path));
                } else if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("osu") || ext.eq_ignore_ascii_case("osz")) {
                    if matches!(self.screen, Screen::Open(_)) {
                        effects.push(Effect::Say("pool-file-open-first"));
                        return effects;
                    }
                    if self.editing().is_none() {
                        self.update(Message::New, now);
                    }
                    self.dropped.push_back(path);
                    effects.extend(self.import_next());
                }
            }
            Message::MapFile(request, said) => {
                let Some(importing) = self.importing.as_ref().filter(|importing| importing.request == request).cloned() else { return effects };
                self.importing = None;
                if !self.editing().is_some_and(|pool| pool.id == importing.pool && pool.fingerprint() == importing.fingerprint) {
                    self.cancel_import();
                    return effects;
                }
                match said {
                    Ok((hash, map)) => {
                        self.fetched.insert(hash.clone(), map.clone());
                        Arc::make_mut(self.songs.get_or_insert_with(|| Arc::new(HashMap::new()))).insert(hash.clone(), map.clone());
                        if let Some(Finding::Fetching(fetching)) = &self.finding {
                            fetching.stop.store(true, Ordering::SeqCst);
                        }
                        self.finding = None;
                        self.notice = None;
                        effects.extend(self.place_map(&hash, map, Some(importing.place), now));
                    }
                    Err(why) => effects.push(Effect::Say(match why {
                        crate::pool_files::Refused::Mode => "pool-file-mode",
                        crate::pool_files::Refused::Save => "pool-file-save-failed",
                        crate::pool_files::Refused::Read | crate::pool_files::Refused::Map => "pool-file-invalid",
                    })),
                }
                effects.extend(self.import_next());
            }
            Message::ArchiveFile(request, said) => {
                let Some(importing) = self.importing.as_ref().filter(|importing| importing.request == request).cloned() else { return effects };
                self.importing = None;
                if !self.editing().is_some_and(|pool| pool.id == importing.pool && pool.fingerprint() == importing.fingerprint) {
                    self.cancel_import();
                    return effects;
                }
                match said {
                    Ok(maps) if !maps.is_empty() => {
                        self.cancel_network();
                        let first = &maps[0].1;
                        let found = pool_links::Found {
                            set: 0,
                            artist: first.artist.clone(),
                            title: first.title.clone(),
                            picked: (maps.len() == 1).then_some(0),
                            difficulties: maps.iter().map(|(hash, map)| pool_links::Difficulty { id: 0, hash: hash.clone(), version: map.version.clone(), stars: 0.0 }).collect(),
                        };
                        for (hash, map) in maps {
                            self.fetched.insert(hash.clone(), map.clone());
                            Arc::make_mut(self.songs.get_or_insert_with(|| Arc::new(HashMap::new()))).insert(hash, map);
                        }
                        let choice = found.picked;
                        self.finding = Some(Finding::Found(Candidate { found, choice, place: importing.place, cover: None }));
                        if let Some(editor) = self.editor_mut() { editor.panel = Panel::Add; editor.source = SourceTab::Search; }
                        effects.extend(self.measure_effects());
                        if let Some(Finding::Found(candidate)) = &self.finding {
                            for difficulty in &candidate.found.difficulties {
                                if self.asked.insert((difficulty.hash.clone(), Mod::Nm)) {
                                    let map = self.songs.as_ref().unwrap()[&difficulty.hash].clone();
                                    effects.push(Effect::Measure(difficulty.hash.clone(), map, Mod::Nm));
                                }
                            }
                        }
                    }
                    Err(why) => {
                        effects.push(Effect::Say(match why {
                            crate::pool_files::Refused::Mode => "pool-file-mode",
                            crate::pool_files::Refused::Save => "pool-file-save-failed",
                            crate::pool_files::Refused::Read | crate::pool_files::Refused::Map => "pool-file-invalid",
                        }));
                        effects.extend(self.import_next());
                    }
                    Ok(_) => {
                        effects.push(Effect::Say("pool-file-invalid"));
                        effects.extend(self.import_next());
                    }
                }
            }
            Message::PoolFile(request, said) => {
                if self.importing_pool == Some(request) {
                    self.importing_pool = None;
                    effects.extend(self.update(Message::Imported(said), now));
                }
            }
            Message::GetMissing => effects.extend(self.get_missing(now)),
            Message::Keep => effects.extend(self.keep_opened(now)),
            Message::Saved(_) => {}
            Message::Choosing(on) => {
                if let Some(editor) = self.editor_mut() {
                    editor.choosing = on;
                    editor.marked.clear();
                    editor.bulk = None;
                    if on {
                        editor.panel = Panel::Closed;
                        editor.selected = None;
                    }
                }
            }
            Message::Mark(slot) => {
                if let Some(editor) = self.editor_mut() {
                    match editor.marked.iter().position(|marked| *marked == slot) {
                        Some(at) => {
                            editor.marked.remove(at);
                        }
                        None => editor.marked.push(slot),
                    }
                    editor.bulk = None;
                }
            }
            Message::Bulk(kind) => {
                if let Some(editor) = self.editor_mut() {
                    editor.bulk = if editor.bulk == kind { None } else { kind };
                }
            }
            Message::BulkMod(mods) => {
                let marked = self.marked_slots();
                if let Some(at) = self.editing_at() {
                    if !marked.is_empty() {
                        self.remember(at);
                        for slot in marked {
                            if let Some(entry) = self.list[at].slots.get_mut(slot) {
                                if entry.mods != mods {
                                    entry.mods = mods;
                                    entry.measure = None;
                                }
                            }
                        }
                        self.save(at, now);
                        effects.extend(self.measure_effects());
                    }
                }
                if let Some(editor) = self.editor_mut() {
                    editor.bulk = None;
                }
            }
            Message::Move(id, keys, key, before) => {
                if let Some(at) = self.editing_at() {
                    if self.list[at].id == id && slot_keys(&self.list[at]) == keys {
                        if let Some(order) = moved_order(&keys, key, before) {
                            self.remember(at);
                            self.reorder_slots(at, &order, now);
                        }
                    }
                }
            }
            Message::Shift(down) => {
                let marked = self.marked_slots();
                if let Some(at) = self.editing_at() {
                    let length = self.list[at].slots.len();
                    if shifted(&marked, length, down) != marked {
                        self.remember(at);
                        let mut order: Vec<usize> = (0..length).collect();
                        shift_in(&mut order, &marked, down);
                        self.reorder_slots(at, &order, now);
                    }
                }
            }
            Message::RemoveMarked => {
                let marked = self.marked_slots();
                if let Some(at) = self.editing_at() {
                    if !marked.is_empty() {
                        self.remember(at);
                        let free = self.list[at].frame == Frame::Free;
                        for slot in marked.iter().rev() {
                            if *slot < self.list[at].slots.len() {
                                if free {
                                    self.list[at].slots.remove(*slot);
                                } else {
                                    self.list[at].slots[*slot].clear();
                                }
                            }
                        }
                        self.save(at, now);
                    }
                }
                if let Some(editor) = self.editor_mut() {
                    editor.marked.clear();
                    editor.bulk = None;
                    editor.selected = None;
                }
            }
            Message::Undo => {
                let current = match &self.screen {
                    Screen::Editor(editor) => Some(editor.id.clone()),
                    _ => None,
                };
                if let Some(id) = current {
                    if let Some(from) = self.undo.iter().rposition(|(pool, _)| *pool == id) {
                        let (_, before) = self.undo.remove(from);
                        if let Some(at) = self.list.iter().position(|pool| pool.id == id) {
                            let current_keys = slot_keys(&self.list[at]);
                            let restored_keys = slot_keys(&before);
                            if current_keys.len() == restored_keys.len() && restored_keys.iter().all(|key| current_keys.contains(key)) {
                                let order: Vec<usize> = restored_keys.iter().map(|key| current_keys.iter().position(|old| old == key).unwrap()).collect();
                                self.follow_order(&order);
                                if let Some(importing) = self.importing.as_mut().filter(|importing| importing.pool == id && importing.fingerprint == self.list[at].fingerprint()) {
                                    importing.fingerprint = before.fingerprint();
                                }
                            }
                            self.list[at] = before;
                            let length = self.list[at].slots.len();
                            self.save(at, now);
                            if let Some(editor) = self.editor_mut() {
                                editor.marked.retain(|slot| *slot < length);
                                editor.selected = editor.selected.filter(|slot| *slot < length);
                                if editor.selected.is_none() && editor.panel == Panel::Slot {
                                    editor.panel = Panel::Closed;
                                }
                            }
                            effects.extend(self.measure_effects());
                        }
                    }
                }
            }
            Message::AskDelete(ask) => {
                if let Some(editor) = self.editor_mut() {
                    editor.asking_delete = ask;
                }
            }
            Message::DeletePool => {
                if let Some(at) = self.editing_at() {
                    self.cancel_network();
                    self.cancel_import();
                    self.cancel_suggestion();
                    let id = self.list.remove(at).id;
                    let _ = pools::remove(&self.dir, &id);
                    self.unsaved.remove(&id);
                    self.save_errors.remove(&id);
                    self.undo.retain(|(pool, _)| *pool != id);
                    self.screen = Screen::Shelf;
                    self.notice = None;
                }
            }
            Message::Put(hash) => effects.extend(self.put(&hash, now)),
            Message::Note(slot, note) => {
                if let Some(at) = self.editing_at() {
                    if let Some(entry) = self.list[at].slots.get_mut(slot) {
                        entry.note = note;
                        self.defer_save(at, now);
                    }
                }
            }
            Message::Songs(songs) => {
                let mut songs = songs;
                if !self.fetched.is_empty() {
                    let merged = Arc::make_mut(&mut songs);
                    for (hash, map) in &self.fetched {
                        merged.entry(hash.clone()).or_insert_with(|| map.clone());
                    }
                }
                self.songs = Some(songs);
                for pool in &mut self.list {
                    if pool.collection {
                        for slot in &mut pool.slots {
                            if let Some(hash) = slot.hash.clone() {
                                if let Some(map) = self.songs.as_ref().and_then(|songs| songs.get(&hash)) { slot.fill(&hash, map); }
                            }
                        }
                    }
                }
                self.reading = false;
                effects.extend(self.measure_effects());
                if let Some(slot) = self.pending_suggestion.take() {
                    effects.extend(self.suggest_slot(slot));
                }
            }
            Message::Measured(hash, mods, said) => {
                self.measures.put(&hash, mods, said);
                self.apply_measures();
                if let Some(slot) = self.suggestions.as_ref().filter(|suggestion| suggestion.target.is_none()).map(|suggestion| suggestion.slot) {
                    effects.extend(self.suggest_slot(slot));
                }
            }
        }
        effects.extend(self.import_next());
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
        if let Some(editor) = self.editor_mut() {
            editor.untouched = false;
            editor.last_mod = mods;
            editor.last_category = category;
            editor.last_colour = colour;
            editor.replace = false;
            editor.selected = Some(slot);
            if picked_suggestion {
                editor.panel = Panel::Slot;
                editor.source = SourceTab::Search;
            }
        }
        self.save(at, now);
        self.measure_effects()
    }

    fn look_at(&mut self, text: &str) -> Vec<Effect> {
        if !matches!(self.finding, Some(Finding::Fetching(_))) {
            self.finding = None;
        } else {
            return Vec::new();
        }
        match pool_links::parse(text) {
            Ok(target) => {
                self.finding = Some(Finding::Asking);
                vec![self.resolve(target)]
            }
            Err(pool_links::Refusal::Mode(mode)) => {
                self.finding = Some(Finding::Refused(mode));
                Vec::new()
            }
            Err(pool_links::Refusal::NotALink) => {
                if text.starts_with("http") || text.contains("ppy.sh") {
                    self.notice = Some(Notice::NotALink);
                }
                Vec::new()
            }
        }
    }

    fn confirm(&mut self, all: bool, now: i64) -> Vec<Effect> {
        let Some(Finding::Found(candidate)) = self.finding.take() else { return Vec::new() };
        let queue: Vec<String> = if all {
            candidate.found.difficulties.iter().map(|difficulty| difficulty.hash.clone()).collect()
        } else {
            match candidate.choice.and_then(|at| candidate.found.difficulties.get(at)) {
                Some(difficulty) => vec![difficulty.hash.clone()],
                None => {
                    self.finding = Some(Finding::Found(candidate));
                    return Vec::new();
                }
            }
        };
        let total = queue.len();
        self.finding = Some(Finding::Fetching(Fetching { queue, total, step: None, place: Some(candidate.place), stop: Arc::new(std::sync::atomic::AtomicBool::new(false)) }));
        self.advance(now)
    }

    fn advance(&mut self, now: i64) -> Vec<Effect> {
        let mut effects = Vec::new();
        loop {
            let Some(Finding::Fetching(fetching)) = &mut self.finding else { return effects };
            if fetching.queue.is_empty() {
                self.finding = None;
                if let Some(editor) = self.editor_mut() {
                    editor.query.clear();
                }
                effects.extend(self.import_next());
                return effects;
            }
            let hash = fetching.queue.remove(0);
            let place = fetching.place;
            let stop = fetching.stop.clone();
            fetching.step = None;
            let known = self.songs.as_ref().and_then(|songs| songs.get(&hash)).or_else(|| self.fetched.get(&hash)).cloned();
            match known {
                Some(map) => {
                    if let Some(Finding::Fetching(fetching)) = &mut self.finding {
                        fetching.place = None;
                    }
                    effects.extend(self.place_map(&hash, map, place, now));
                }
                None => {
                    effects.push(self.fetch(hash, stop));
                    return effects;
                }
            }
        }
    }

    fn begin_open(&mut self, pool: Pool) -> Vec<Effect> {
        self.cancel_network();
        self.cancel_import();
        self.cancel_suggestion();
        if let Some(Finding::Fetching(fetching)) = &self.finding {
            fetching.stop.store(true, Ordering::SeqCst);
        }
        self.refused = None;
        self.finding = None;
        self.screen = Screen::Open(Opening { pool, queue: Vec::new(), total: 0, step: None, stop: Arc::new(std::sync::atomic::AtomicBool::new(false)), running: false, lost: 0 });
        let mut effects = Vec::new();
        if self.songs.is_none() && !self.reading {
            self.reading = true;
            effects.push(Effect::ReadSongs);
        }
        effects.extend(self.measure_effects());
        effects
    }

    pub fn missing(&self, pool: &Pool) -> Vec<String> {
        let Some(songs) = &self.songs else { return Vec::new() };
        let mut seen = HashSet::new();
        pool.slots
            .iter()
            .filter_map(|slot| slot.hash.clone())
            .filter(|hash| !songs.contains_key(hash) && !self.fetched.contains_key(hash))
            .filter(|hash| seen.insert(hash.clone()))
            .collect()
    }

    fn get_missing(&mut self, now: i64) -> Vec<Effect> {
        let queue = match &self.screen {
            Screen::Open(opening) if !opening.running => self.missing(&opening.pool),
            _ => return Vec::new(),
        };
        if queue.is_empty() {
            return Vec::new();
        }
        if let Screen::Open(opening) = &mut self.screen {
            opening.total = queue.len();
            opening.queue = queue;
            opening.running = true;
            opening.lost = 0;
            opening.stop.store(false, std::sync::atomic::Ordering::SeqCst);
        }
        self.advance_open(now)
    }

    fn advance_open(&mut self, _now: i64) -> Vec<Effect> {
        loop {
            let Screen::Open(opening) = &mut self.screen else { return Vec::new() };
            if opening.queue.is_empty() {
                opening.running = false;
                opening.step = None;
                return self.measure_effects();
            }
            let hash = opening.queue.remove(0);
            opening.step = None;
            let stop = opening.stop.clone();
            let known = self.songs.as_ref().is_some_and(|songs| songs.contains_key(&hash)) || self.fetched.contains_key(&hash);
            if !known {
                return vec![self.fetch(hash, stop)];
            }
        }
    }

    fn keep_opened(&mut self, now: i64) -> Vec<Effect> {
        self.cancel_network();
        let Screen::Open(opening) = std::mem::replace(&mut self.screen, Screen::Shelf) else { return Vec::new() };
        opening.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        let mut pool = opening.pool;
        pool.made_at = now;
        pool.changed_at = now;
        let id = pool.id.clone();
        self.list.insert(0, pool);
        self.save(0, now);
        self.screen = Screen::Editor(Editor::at(id));
        self.measure_effects()
    }

    fn stepped(&mut self, hash: &str, step: crate::maps::Step, now: i64) -> Vec<Effect> {
        use crate::maps::Step;
        if let Screen::Open(opening) = &mut self.screen {
            opening.step = Some(step.clone());
            return match step {
                Step::Done(map) => {
                    self.fetched.insert(hash.to_owned(), map.clone());
                    if let Some(songs) = self.songs.as_mut() {
                        Arc::make_mut(songs).insert(hash.to_owned(), map);
                    }
                    let mut effects = self.measure_effects();
                    effects.extend(self.advance_open(now));
                    effects
                }
                Step::Nowhere | Step::Failed(_) => {
                    opening.lost += 1;
                    self.advance_open(now)
                }
                Step::Stopped => {
                    opening.queue.clear();
                    opening.running = false;
                    Vec::new()
                }
                Step::Looking | Step::Found(_) | Step::Downloading { .. } | Step::Unpacking | Step::Checking => Vec::new(),
            };
        }
        let Some(Finding::Fetching(fetching)) = &mut self.finding else { return Vec::new() };
        fetching.step = Some(step.clone());
        match step {
            Step::Done(map) => {
                let place = fetching.place.take();
                self.fetched.insert(hash.to_owned(), map.clone());
                if let Some(songs) = self.songs.as_mut() {
                    Arc::make_mut(songs).insert(hash.to_owned(), map.clone());
                }
                let mut effects = self.place_map(hash, map, place, now);
                effects.extend(self.advance(now));
                effects
            }
            Step::Nowhere => {
                self.finding = Some(Finding::Missing);
                Vec::new()
            }
            Step::Failed(why) => {
                self.finding = Some(Finding::Silent(why));
                Vec::new()
            }
            Step::Stopped => {
                self.finding = None;
                Vec::new()
            }
            Step::Looking | Step::Found(_) | Step::Downloading { .. } | Step::Unpacking | Step::Checking => Vec::new(),
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
                for pool in self.list.iter().filter(|pool| !pool.collection && pool.saved.is_some() == (self.shelf == Shelf::Saved)) {
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
                let kind = self.collection_shelf;
                let most = if self.roomy_drafts() { STRIP_MOST } else { 3 };
                for pool in self.list.iter().filter(|pool| pool.collection == kind) {
                    for slot in pool.slots.iter().filter_map(|slot| slot.hash.as_deref()).take(most) {
                        add(slot);
                    }
                }
                if self.roomy_published() {
                    for item in self.publications.iter().filter(|item| (item.kind == "collection") == kind) {
                        for hash in published_hashes(item).into_iter().flatten().take(STRIP_MOST) {
                            add(&hash);
                        }
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
        _ => pool.first_empty(),
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
        Mod::Nm => [0x5e, 0xc2, 0xd0], Mod::Dt => [0x61, 0x2d, 0x9b], Mod::Hr => [0xb4, 0x14, 0x18],
        Mod::Hd => [0x9b, 0x58, 0x2d], Mod::Fm => [0x9b, 0x2d, 0x55], Mod::Tb => [0x5c, 0x1b, 0x1d],
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
                border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.04 * k), width: 1.0, radius: round.into() },
                ..container::Style::default()
            })
            .into(),
    }
}

fn hatched_cover<'a>() -> Element<'a, Message> {
    let k = ui::fade();
    let frame = container(container(ui::fine_hatch()).width(Length::Fill).height(Length::Fill).padding(2.0))
        .width(COVER_WIDE)
        .height(COVER_HIGH)
        .style(move |_| container::Style {
            border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.035 * k), width: 1.0, radius: COVER_ROUND.into() },
            ..container::Style::default()
        });
    stack![frame, container(glyph(Icon::Plus, 24.0, FAINT)).width(COVER_WIDE).height(COVER_HIGH).center(Length::Fill)].width(COVER_WIDE).height(COVER_HIGH).into()
}

fn number_badge<'a>(number: usize, lit: bool) -> Element<'a, Message> {
    let k = ui::fade();
    container(text(number.to_string()).font(theme::MONO_BOLD).size(12.0).color(ui::faded(if lit { INK } else { MUTED })))
        .width(28.0)
        .height(28.0)
        .center(28.0)
        .style(move |_| container::Style {
            background: Some(Background::Color(if lit { Color::from_rgba(0.886, 0.282, 0.282, 0.12 * k) } else { Color::from_rgba(1.0, 1.0, 1.0, 0.028 * k) })),
            border: Border { radius: 14.0.into(), ..Border::default() },
            ..container::Style::default()
        })
        .into()
}

fn mod_badge<'a>(mods: Mod, lit: bool) -> Element<'a, Message> {
    let k = ui::fade();
    container(text(mods.code()).font(theme::MONO).size(11.0).color(ui::faded(if lit { INK } else { MUTED })))
        .padding([4, 9])
        .style(move |_| container::Style {
            background: Some(Background::Color(if lit { Color::from_rgba(0.886, 0.282, 0.282, 0.09 * k) } else { Color::from_rgba(1.0, 1.0, 1.0, 0.02 * k) })),
            border: Border { radius: 8.0.into(), ..Border::default() },
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
            border: Border { color: if selected { Color::from_rgba(0.886, 0.282, 0.282, 0.6) } else { Color::TRANSPARENT }, width: 1.0, radius: 12.0.into() },
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
            border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, if hot { 0.07 } else { 0.04 }), width: 1.0, radius: 14.0.into() },
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
            border: Border { color: Color { a: theme::LINE.a * k, ..theme::LINE }, width: 1.0, radius: theme::CARD_RADIUS.into() },
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
            border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.07 * k), width: 1.0, radius: 16.0.into() },
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
        let colour = if filled { crate::dossier::star_colour(stars as f32) } else { Color::from_rgba(1.0, 1.0, 1.0, 0.18) };
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

fn published_hashes(item: &crate::bot::Publication) -> Vec<Option<String>> {
    match item.kind.as_str() {
        "collection" => item.content["hashes"].as_array().into_iter().flatten().map(|hash| hash.as_str().map(str::to_owned)).collect(),
        _ => item.content["slots"].as_array().into_iter().flatten().map(|slot| slot["hash"].as_str().map(str::to_owned)).collect(),
    }
}

fn strip<'a>(thumbs: &HashMap<String, image::Handle>, maps: &[(Option<String>, Color)], room: f32) -> Element<'a, Message> {
    let asked = maps.len().min(STRIP_MOST).max(1) as f32;
    let each = ((room - STRIP_GAP * (asked - 1.0)) / asked).clamp(40.0, 112.0).floor();
    let high = (each * 0.56).clamp(28.0, 60.0).floor();
    let fit = (((room + STRIP_GAP) / (each + STRIP_GAP)).floor() as usize).max(2);
    let (shown, more) = if maps.len() > fit { (fit - 1, maps.len() - (fit - 1)) } else { (maps.len(), 0) };
    let k = ui::fade();
    let mut line = row![].spacing(STRIP_GAP).align_y(iced::alignment::Vertical::Top);
    for (hash, colour) in maps.iter().take(shown) {
        let colour = *colour;
        let tick = container(Space::new()).width(each).height(3.0).style(move |_| container::Style {
            background: Some(Background::Color(Color { a: colour.a * k, ..colour })),
            border: Border { radius: 1.5.into(), ..Border::default() },
            ..container::Style::default()
        });
        line = line.push(column![cover(thumbs, hash.as_deref(), each, high, 6.0), tick].spacing(4));
    }
    if more > 0 {
        line = line.push(container(mono(format!("+{more}"), 12.0, MUTED)).width(each).center_y(high).align_x(iced::alignment::Horizontal::Center));
    }
    line.into()
}

fn cells<'a>(parts: Vec<(Col, Element<'a, Message>)>) -> iced::widget::Row<'a, Message> {
    row(parts.into_iter().map(|(col, part)| -> Element<'a, Message> {
        match col {
            Col::Fill => container(part).width(Length::Fill).clip(true).into(),
            Col::Wide(wide) => container(part).width(wide).clip(true).into(),
            Col::End(wide) => container(part).width(wide).align_x(iced::alignment::Horizontal::Right).into(),
        }
    }))
    .spacing(LEDGER_GAP)
    .align_y(iced::Center)
}

fn ledger_line<'a>() -> Element<'a, Message> {
    let k = ui::fade();
    container(Space::new()).width(Length::Fill).height(1.0).style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.04 * k))), ..container::Style::default() }).into()
}

fn ledger_box<'a>(inside: Element<'a, Message>) -> Element<'a, Message> {
    let k = ui::fade();
    container(inside)
        .width(Length::Fill)
        .padding([6, 0])
        .style(move |theme| {
            let slab = theme::slab(theme);
            container::Style {
                background: slab.background.map(|fill| match fill { Background::Color(colour) => Background::Color(Color { a: colour.a * k, ..colour }), other => other }),
                border: Border { color: Color { a: slab.border.color.a * k, ..slab.border.color }, width: 1.0, radius: 14.0.into() },
                ..container::Style::default()
            }
        })
        .into()
}

fn ledger_row(_: &iced::Theme, status: button::Status) -> button::Style {
    let hot = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: hot.then_some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.016))),
        text_color: INK,
        border: Border::default(),
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

fn ledger_head<'a>(parts: Vec<(Col, String)>) -> Element<'a, Message> {
    container(cells(parts.into_iter().map(|(col, label)| (col, ui::mono_small(label.to_uppercase(), MUTED))).collect())).padding([8.0, LEDGER_PAD]).into()
}

fn ringed<'a>(inside: Element<'a, Message>, round: f32) -> Element<'a, Message> {
    let k = ui::fade();
    container(inside)
        .padding(2)
        .style(move |_| container::Style { background: Some(Background::Color(Color { a: k, ..theme::GROUND })), border: Border { radius: (round + 2.0).into(), ..Border::default() }, ..container::Style::default() })
        .into()
}

fn overlapped<'a>(pieces: Vec<Element<'a, Message>>, step: f32, each: f32) -> Element<'a, Message> {
    let wide = each + step * pieces.len().saturating_sub(1) as f32;
    let mut layers: Vec<Element<'a, Message>> = vec![Space::new().width(wide).height(each).into()];
    layers.extend(pieces.into_iter().enumerate().map(|(at, piece)| -> Element<'a, Message> { container(piece).padding(Padding { left: at as f32 * step, ..Padding::ZERO }).into() }));
    iced::widget::Stack::with_children(layers).width(wide).height(each).into()
}

fn cover_stack<'a>(thumbs: &HashMap<String, image::Handle>, hashes: &[Option<&str>]) -> Element<'a, Message> {
    let shown: Vec<Option<&str>> = if hashes.is_empty() { vec![None] } else { hashes.iter().take(3).copied().collect() };
    let pieces = shown.into_iter().map(|hash| ringed(cover(thumbs, hash, 40.0, 40.0, THUMB_ROUND), THUMB_ROUND)).collect();
    container(overlapped(pieces, 24.0, 44.0)).width(92.0).into()
}

fn face<'a>(state: &'a State, name: &str) -> Element<'a, Message> {
    match state.faces.get(&name.to_lowercase()) {
        Some(face) => image(crate::crops::fitted(face, FACE, FACE, FACE / 2.0)).width(FACE).height(FACE).content_fit(iced::ContentFit::Fill).opacity(ui::fade()).into(),
        None => {
            let k = ui::fade();
            container(glyph(Icon::Person, 16.0, MUTED))
                .center(FACE)
                .style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.03 * k))), border: Border { radius: (FACE / 2.0).into(), ..Border::default() }, ..container::Style::default() })
                .into()
        }
    }
}

fn named_face<'a>(state: &'a State, name: &str, most: f32) -> Element<'a, Message> {
    let wide = (ui::text_width(name, theme::SANS_SEMI, 13.0) + 2.0).min(most);
    row![face(state, name), container(ui::moving_text(name.to_owned(), theme::SANS_SEMI, 13.0, INK)).width(wide)].spacing(7).align_y(iced::Center).into()
}

fn makers_sheet<'a>(state: &'a State, names: &[String], words: &'a Words) -> Element<'a, Message> {
    let k = ui::fade();
    let mut list = column![].spacing(2);
    for pair in names.chunks(2) {
        list = list.push(row(pair.iter().map(|name| container(author_chip(state, name)).width(Length::FillPortion(1)).into()).collect::<Vec<Element<'a, Message>>>()).spacing(4));
    }
    let head = row![ui::mono_small(words.t("pool-mappoolers").to_uppercase(), MUTED), ui::grow(), mono(names.len().to_string(), 13.0, INK)].align_y(iced::Center);
    container(column![container(head).padding([0, 9]), list].spacing(8))
        .width(340.0)
        .padding([14, 7])
        .style(move |_| container::Style {
            background: Some(Background::Color(Color { a: k, ..Color::from_rgb8(0x1d, 0x0b, 0x0f) })),
            border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.16 * k), width: 1.0, radius: 12.0.into() },
            shadow: iced::Shadow { color: Color::from_rgba(0.0, 0.0, 0.0, 0.45 * k), offset: iced::Vector::new(0.0, 16.0), blur_radius: 40.0 },
            ..container::Style::default()
        })
        .into()
}

fn makers_cell<'a>(state: &'a State, key: String, names: &[String], wide: f32, words: &'a Words) -> Element<'a, Message> {
    if names.is_empty() {
        return faded_text(words.t("ledger-no-mappoolers"), 13.0, FAINT);
    }
    let each = (wide - 10.0) / names.len() as f32 - FACE - 7.0;
    if names.len() <= NAMED_MOST && each >= 56.0 {
        return row(names.iter().map(|name| named_face(state, name, each)).collect::<Vec<_>>()).spacing(10).align_y(iced::Center).into();
    }
    let faces = overlapped(names.iter().take(FACES_MOST).map(|name| ringed(face(state, name), FACE / 2.0)).collect(), 18.0, FACE + 4.0);
    let mut line = row![faces].spacing(8).align_y(iced::Center);
    if names.len() > FACES_MOST {
        let k = ui::fade();
        line = line.push(container(mono(format!("+{}", names.len() - FACES_MOST), 11.5, INK)).padding([2, 7]).style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.03 * k))),
            border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.1 * k), width: 1.0, radius: 11.0.into() },
            ..container::Style::default()
        }));
    }
    let open = state.makers.as_deref() == Some(key.as_str());
    let anchor = button(line).padding([3, 5]).style(ui::button_faded(surface_style(open))).on_press(Message::Makers(Some(key)));
    ui::under(anchor, open.then(|| makers_sheet(state, names, words)), Message::Makers(None))
}

fn stars_span<'a>(words: &Words, low: Option<f64>, high: Option<f64>) -> Element<'a, Message> {
    let pill = |stars: f64| crate::dossier::star_pill(stars_of(words, stars), stars as f32, 13.5);
    match (low, high) {
        (Some(low), Some(high)) if (high - low).abs() >= 0.005 => row![pill(low), pill(high)].spacing(6).align_y(iced::Center).into(),
        (Some(low), _) => pill(low),
        _ => Space::new().into(),
    }
}

struct Shape {
    makers: Option<f32>,
    stars: bool,
    length: bool,
    bars: bool,
    changed: bool,
    publisher: bool,
}

fn shape(width: f32, collection: bool) -> Shape {
    Shape {
        makers: (!collection).then_some(if width >= 1000.0 { 270.0 } else if width >= 760.0 { 220.0 } else { 104.0 }),
        stars: width >= 620.0,
        length: width >= 860.0,
        bars: width >= 1060.0,
        changed: width >= 700.0,
        publisher: width >= 640.0,
    }
}

fn draft_row<'a>(state: &'a State, pool: &'a Pool, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, shape: &Shape, today: i64, room: Option<f32>) -> Element<'a, Message> {
    let hashes: Vec<Option<&str>> = pool.slots.iter().filter(|slot| !slot.is_empty()).map(|slot| slot.hash.as_deref()).collect();
    let name = if pool.name.is_empty() { words.t(if pool.collection { "collections-new" } else { "pool-untitled" }) } else { pool.name.clone() };
    let facts = pool.facts();
    let milliseconds: i64 = pool.slots.iter().filter_map(|slot| slot.measure.as_ref()).map(|measure| measure.length_ms).sum();
    let title = text(name).font(theme::SANS_SEMI).size(if room.is_some() { 16.0 } else { 15.0 }).color(ui::faded(INK)).wrapping(text::Wrapping::None);
    let lead: Element<'a, Message> = if room.is_some() { title.into() } else { row![cover_stack(thumbs, &hashes), title].spacing(12).align_y(iced::Center).into() };
    let mut parts: Vec<(Col, Element<'a, Message>)> = vec![(Col::Fill, lead)];
    if let Some(wide) = shape.makers {
        parts.push((Col::Wide(wide), makers_cell(state, format!("draft:{}", pool.id), &pool.authors, wide, words)));
    }
    parts.push((Col::Wide(44.0), mono(facts.cards.to_string(), 13.0, INK)));
    if shape.stars {
        parts.push((Col::Wide(STARS_WIDE), stars_span(words, facts.low, facts.high)));
    }
    if shape.length {
        parts.push((Col::Wide(64.0), mono(if milliseconds > 0 { words.length(milliseconds) } else { String::new() }, 13.0, INK)));
    }
    if shape.bars {
        parts.push((Col::Wide(96.0), bars(pool)));
    }
    if shape.changed {
        parts.push((Col::End(132.0), mono(words.day(pool.changed_at, today), 12.5, MUTED)));
    }
    let Some(room) = room.filter(|_| !pool.slots.is_empty()) else {
        return button(container(cells(parts)).center_y(Length::Fill)).padding([0.0, LEDGER_PAD]).height(64.0).width(Length::Fill).style(ui::button_faded(ledger_row)).on_press(Message::Open(pool.id.clone())).into();
    };
    let grey = Color::from_rgb8(95, 95, 95);
    let maps: Vec<(Option<String>, Color)> = pool.slots.iter().map(|slot| (slot.hash.clone(), if pool.collection { grey } else { slot_colour(slot) })).collect();
    let inside = column![container(cells(parts)).height(44.0).center_y(44.0), strip(thumbs, &maps, room)].spacing(8);
    button(inside).padding([12.0, LEDGER_PAD]).width(Length::Fill).style(ui::button_faded(ledger_row)).on_press(Message::Open(pool.id.clone())).into()
}

fn new_row<'a>(words: &Words, collection: bool) -> Element<'a, Message> {
    let k = ui::fade();
    let plus = container(glyph(Icon::Plus, 18.0, MUTED)).center(40.0).style(move |_| container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.14 * k))),
        border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.08 * k), width: 1.0, radius: THUMB_ROUND.into() },
        ..container::Style::default()
    });
    let label = text(words.t(if collection { "collections-new" } else { "pools-new" })).font(theme::SANS_SEMI).size(15.0).color(ui::faded(MUTED));
    button(container(row![container(plus).padding(2), label].spacing(12).align_y(iced::Center)).center_y(Length::Fill)).padding([0.0, LEDGER_PAD]).height(64.0).width(Length::Fill).style(ui::button_faded(ledger_row)).on_press(Message::New).into()
}

fn published_row<'a>(state: &'a State, item: &'a crate::bot::Publication, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, shape: &Shape, collection: bool, room: Option<f32>) -> Element<'a, Message> {
    let count = if collection { item.content["hashes"].as_array().map_or(0, Vec::len) } else { item.content["slots"].as_array().map_or(0, |slots| slots.iter().filter(|slot| slot["hash"].is_string()).count()) };
    let mut parts: Vec<(Col, Element<'a, Message>)> = vec![(Col::Fill, text(item.name.clone()).font(theme::SANS_SEMI).size(15.0).color(ui::faded(INK)).wrapping(text::Wrapping::None).into())];
    if let Some(wide) = shape.makers {
        let names: Vec<String> = item.content["authors"].as_array().into_iter().flatten().filter_map(|name| name.as_str().map(str::to_owned)).collect();
        parts.push((Col::Wide(wide), makers_cell(state, format!("published:{}", item.id), &names, wide, words)));
    }
    if shape.publisher {
        let publisher: Element<'a, Message> = match item.content["compiler"].as_str().filter(|name| !name.is_empty()) {
            Some(name) => named_face(state, name, 150.0),
            None => Space::new().into(),
        };
        parts.push((Col::Wide(190.0), publisher));
    }
    parts.push((Col::Wide(44.0), mono(count.to_string(), 13.0, INK)));
    parts.push((Col::End(132.0), faded_text(words.t(if item.mine { "ledger-edit" } else { "catalogue-copy" }), 13.0, if item.mine { INK } else { MUTED })));
    let Some(room) = room.filter(|_| count > 0) else {
        return button(container(cells(parts)).center_y(Length::Fill)).padding([0.0, LEDGER_PAD]).height(56.0).width(Length::Fill).style(ui::button_faded(ledger_row)).on_press(Message::OpenPublication(item.id.clone())).into();
    };
    let grey = Color::from_rgb8(95, 95, 95);
    let parsed = (!collection).then(|| serde_json::to_vec(&item.content).ok().and_then(|bytes| pool_share::from_file(&bytes, 0).ok())).flatten();
    let maps: Vec<(Option<String>, Color)> = match &parsed {
        Some(pool) => pool.slots.iter().map(|slot| (slot.hash.clone(), slot_colour(slot))).collect(),
        None => published_hashes(item).into_iter().map(|hash| (hash, grey)).collect(),
    };
    let inside = column![container(cells(parts)).height(44.0).center_y(44.0), strip(thumbs, &maps, room)].spacing(8);
    button(inside).padding([12.0, LEDGER_PAD]).width(Length::Fill).style(ui::button_faded(ledger_row)).on_press(Message::OpenPublication(item.id.clone())).into()
}

fn refusal_line<'a>(refused: &pool_share::Refused, words: &Words) -> Element<'a, Message> {
    let key = match refused {
        pool_share::Refused::NotAPool => "pool-refused-not",
        pool_share::Refused::Newer => "pool-refused-newer",
        pool_share::Refused::Damaged => "pool-refused-damaged",
    };
    faded_text(words.t(key), 13.0, ACCENT)
}

fn shelf<'a>(state: &'a State, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, width: f32, t: f32, clocks: &Clocks) -> Element<'a, Message> {
    let collection = state.collection_shelf;
    let shape = shape(width, collection);
    let room = (width - 2.0 * LEDGER_PAD - 2.0).max(120.0);
    let drafts_room = state.roomy_drafts().then_some(room);
    let published_room = state.roomy_published().then_some(room);
    let head = move || -> Element<'a, Message> {
        let tab = |label: String, lit: bool, press: Message| -> Element<'a, Message> {
            button(text(label).font(theme::SANS_SEMI).size(13.5).color(ui::faded(if lit { INK } else { MUTED }))).padding([7, 14]).style(ui::button_faded(theme::bare)).on_press(press).into()
        };
        let at = if collection { 3 } else { match state.shelf { Shelf::Mine => 0, Shelf::Saved => 1, Shelf::Published => 2 } };
        let tabs = ui::sliding(
            row![
                tab(words.t("shelf-mine"), at == 0, Message::Shelf(Shelf::Mine)),
                tab(words.t("shelf-saved"), at == 1, Message::Shelf(Shelf::Saved)),
                tab(words.t("catalogue-published"), at == 2, Message::Shelf(Shelf::Published)),
                tab(words.t("catalogue-collections"), at == 3, Message::Catalogue(true)),
            ].spacing(4),
            at,
            ui::Pill { fill: Color::from_rgba(0.886, 0.282, 0.282, 0.15), edge: Color::from_rgba(0.886, 0.282, 0.282, 0.4), radius: 8.0, underline: None },
        );
        let mut line = row![semi(words.t("pools-title"), 22.0, INK), tabs, ui::grow()].spacing(16).align_y(iced::Center);
        if !collection && width >= 860.0 {
            line = line.push(quiet_button(words.t("pools-open"), Message::OpenFile));
        }
        line.push(primary_button(words.t(if collection { "catalogue-new" } else { "pools-new" }), Message::New)).into()
    };
    let mut page = column![ui::appearing(ui::appear(t, 0), 10.0, head)].spacing(18);
    if let Some(refused) = &state.refused {
        page = page.push(ui::appearing(ui::appear(t, 1), 8.0, || refusal_line(refused, words)));
    }
    if !collection {
        return showcase(state, page, words, thumbs, width, t, clocks);
    }
    let drafts_head = || -> Element<'a, Message> {
        let mut parts = vec![(Col::Fill, words.t("catalogue-drafts"))];
        if let Some(wide) = shape.makers { parts.push((Col::Wide(wide), words.t("pool-mappoolers"))); }
        parts.push((Col::Wide(44.0), words.t("ledger-cards")));
        if shape.stars { parts.push((Col::Wide(STARS_WIDE), words.t("ledger-stars"))); }
        if shape.length { parts.push((Col::Wide(64.0), words.t("ledger-length"))); }
        if shape.bars { parts.push((Col::Wide(96.0), words.t("ledger-difficulty"))); }
        if shape.changed { parts.push((Col::End(132.0), words.t("ledger-changed"))); }
        ledger_head(parts)
    };
    let mut drafts = column![ui::appearing(ui::appear(t, 1), 8.0, drafts_head)];
    drafts = drafts.push(ledger_line()).push(ui::appearing(ui::appear(t, 2), 8.0, || new_row(words, collection)));
    for (at, pool) in state.list.iter().filter(|pool| pool.collection == collection).enumerate() {
        drafts = drafts.push(ledger_line()).push(ui::appearing(ui::appear(t, 3 + at), 8.0, || draft_row(state, pool, words, thumbs, &shape, clocks.today, drafts_room)));
    }
    page = page.push(ledger_box(drafts.into()));
    if collection {
        if let Some(collections) = &state.collections {
            page = page.push(ui::appearing(ui::appear(t, 3), 8.0, || ui::mono_small(words.t("catalogue-game-collections"), MUTED)));
            for (at, found) in collections.iter().enumerate() {
                page = page.push(ui::appearing(ui::appear(t, 3 + at), 8.0, || quiet_button(found.name.clone(), Message::ImportCollection(at))));
            }
        }
    }
    let published_head = || -> Element<'a, Message> {
        let mut parts: Vec<(Col, Element<'a, Message>)> = vec![(Col::Fill, ui::mono_small(words.t("catalogue-published").to_uppercase(), MUTED))];
        if let Some(wide) = shape.makers { parts.push((Col::Wide(wide), ui::mono_small(words.t("pool-mappoolers").to_uppercase(), MUTED))); }
        if shape.publisher { parts.push((Col::Wide(190.0), ui::mono_small(words.t("pool-publisher").to_uppercase(), MUTED))); }
        parts.push((Col::Wide(44.0), ui::mono_small(words.t("ledger-cards").to_uppercase(), MUTED)));
        parts.push((Col::End(132.0), quiet_button(words.t("catalogue-refresh"), Message::Catalogue(collection))));
        container(cells(parts)).padding([2.0, LEDGER_PAD]).into()
    };
    let mut published = column![ui::appearing(ui::appear(t, 4), 8.0, published_head)];
    if state.catalogue_loading {
        published = published.push(ledger_line()).push(ui::appearing(ui::appear(t, 5), 8.0, || container(faded_text(words.t("catalogue-loading"), 13.0, MUTED)).padding([14.0, LEDGER_PAD]).into()));
    }
    if state.publications.is_empty() && !state.catalogue_loading {
        published = published.push(ledger_line()).push(ui::appearing(ui::appear(t, 5), 8.0, || container(faded_text(words.t("catalogue-empty"), 13.0, MUTED)).padding([14.0, LEDGER_PAD]).into()));
    }
    for (at, item) in state.publications.iter().filter(|item| (item.kind == "collection") == collection).enumerate() {
        let seen = t.min(clocks.of(&item.id));
        published = published.push(ledger_line()).push(ui::appearing(ui::appear(seen, 5 + at), 8.0, || published_row(state, item, words, thumbs, &shape, collection, published_room)));
    }
    page = page.push(ledger_box(published.into()));
    if let Some(error) = &state.catalogue_error {
        page = page.push(ui::appearing(ui::appear(t, 5), 8.0, || catalogue_error(error, words)));
    }
    if state.catalogue_more {
        page = page.push(quiet_button(words.t("catalogue-more"), Message::CatalogueMore));
    }
    page.into()
}

const MOSAIC: usize = 8;
const HERO_COVERS: usize = 15;
const CARD_GAP: f32 = 16.0;
const CARD_TOP: f32 = 128.0;
const CARD_FILL: Color = Color::from_rgb(0.09, 0.039, 0.055);

struct Shown {
    title: String,
    makers: Vec<String>,
    hashes: Vec<Option<String>>,
    cards: usize,
    length: i64,
    low: Option<f64>,
    high: Option<f64>,
    mods: Vec<(Mod, usize)>,
    when: String,
}

fn shown_of(pool: &Pool, words: &Words, when: String) -> Shown {
    let facts = pool.facts();
    let filled: Vec<&Slot> = pool.slots.iter().filter(|slot| !slot.is_empty()).collect();
    Shown {
        title: if pool.name.is_empty() { words.t("pool-untitled") } else { pool.name.clone() },
        makers: pool.authors.clone(),
        hashes: filled.iter().map(|slot| slot.hash.clone()).collect(),
        cards: facts.cards,
        length: pool.slots.iter().filter_map(|slot| slot.measure.as_ref()).map(|measure| measure.length_ms).sum(),
        low: facts.low,
        high: facts.high,
        mods: Mod::ALL.into_iter().map(|mods| (mods, filled.iter().filter(|slot| slot.mods == mods).count())).filter(|(_, count)| *count > 0).collect(),
        when,
    }
}

fn plate<'a>(inside: Element<'a, Message>) -> Element<'a, Message> {
    let k = ui::fade();
    container(inside)
        .padding([3, 10])
        .style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(0.05, 0.02, 0.03, 0.66 * k))), border: Border { radius: 11.0.into(), ..Border::default() }, ..container::Style::default() })
        .into()
}

fn code_plate<'a>(code: &str, revision: u64) -> Element<'a, Message> {
    let number = text(format!("#{revision}")).font(theme::MONO_BOLD).size(12.0).color(ui::faded(Color::from_rgb(0.91, 0.416, 0.416)));
    if code.is_empty() {
        return plate(number.into());
    }
    plate(row![text(code.to_owned()).font(theme::MONO_BOLD).size(12.0).color(ui::faded(INK)), number].into())
}

fn word_plate<'a>(said: String) -> Element<'a, Message> {
    plate(text(said).font(theme::SANS_SEMI).size(12.0).color(ui::faded(INK)).into())
}

fn tile<'a>(thumbs: &HashMap<String, image::Handle>, hash: Option<&str>, wide: f32, high: f32, round: f32) -> Element<'a, Message> {
    let k = ui::fade();
    match hash.and_then(|hash| thumbs.get(hash)) {
        Some(handle) => image(crate::crops::fitted(handle, wide, high, round)).content_fit(iced::ContentFit::Fill).width(Length::Fill).height(high).border_radius(round).opacity(k).into(),
        None => container(Space::new())
            .width(Length::Fill)
            .height(high)
            .style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.03 * k))), border: Border { radius: round.into(), ..Border::default() }, ..container::Style::default() })
            .into(),
    }
}

fn mosaic<'a>(thumbs: &HashMap<String, image::Handle>, hashes: &[Option<String>], across: usize, down: usize, wide: f32, high: f32, gap: f32, round: f32) -> Element<'a, Message> {
    let each_wide = ((wide - gap * (across as f32 - 1.0)) / across as f32).max(1.0);
    let each_high = ((high - gap * (down as f32 - 1.0)) / down as f32).max(1.0);
    let mut lines = column![].spacing(gap);
    for line in 0..down {
        let mut tiles = row![].spacing(gap);
        for at in 0..across {
            let spot = line * across + at;
            let hash = match hashes.len() {
                0 => None,
                held if held >= across => hashes[spot % held].as_deref(),
                _ => hashes.get(spot).and_then(|hash| hash.as_deref()),
            };
            tiles = tiles.push(tile(thumbs, hash, each_wide, each_high, round));
        }
        lines = lines.push(tiles);
    }
    lines.into()
}

fn makers_line<'a>(state: &'a State, names: &[String], words: &Words, size: f32) -> Element<'a, Message> {
    if names.is_empty() {
        return container(faded_text(words.t("shelf-no-makers"), size, FAINT)).height(FACE + 4.0).center_y(FACE + 4.0).into();
    }
    let faces = overlapped(names.iter().take(3).map(|name| ringed(face(state, name), FACE / 2.0)).collect(), 18.0, FACE + 4.0);
    row![faces, container(text(names.join(", ")).font(theme::SANS_SEMI).size(size).color(ui::faded(INK)).wrapping(text::Wrapping::None)).width(Length::Fill).clip(true)].spacing(8).align_y(iced::Center).into()
}

fn fact<'a>(value: String, label: String) -> Element<'a, Message> {
    column![text(value).font(theme::MONO_BOLD).size(15.0).color(ui::faded(INK)), faded_text(label, 12.0, MUTED)].spacing(1).into()
}

fn mod_strip<'a>(mods: &[(Mod, usize)], legend: bool) -> Element<'a, Message> {
    let k = ui::fade();
    let mut bar = row![].spacing(3);
    let mut said = row![].spacing(12);
    for (mods, count) in mods {
        let tint = mod_tint(*mods);
        bar = bar.push(container(Space::new().height(5.0)).width(Length::FillPortion(*count as u16)).style(move |_| container::Style { background: Some(Background::Color(Color { a: k, ..tint })), border: Border { radius: 2.5.into(), ..Border::default() }, ..container::Style::default() }));
        said = said.push(row![text(mods.code()).font(theme::MONO_BOLD).size(11.0).color(ui::faded(tint)), text(count.to_string()).font(theme::MONO).size(11.0).color(ui::faded(MUTED))].spacing(4));
    }
    if mods.is_empty() {
        return Space::new().height(0.0).into();
    }
    if legend { column![bar, said].spacing(6).into() } else { bar.into() }
}

fn card_style(_: &iced::Theme, status: button::Status) -> button::Style {
    let hot = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(Background::Color(CARD_FILL)),
        text_color: INK,
        border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, if hot { 0.3 } else { 0.12 }), width: 1.0, radius: 16.0.into() },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

fn shade<'a>(from: f32) -> Element<'a, Message> {
    let k = ui::fade();
    let fade = iced::gradient::Linear::new(iced::Radians(std::f32::consts::PI))
        .add_stop(0.0, Color { a: from * k, ..CARD_FILL })
        .add_stop(0.5, Color { a: 0.55 * k, ..CARD_FILL })
        .add_stop(1.0, Color { a: k, ..CARD_FILL });
    container(Space::new()).width(Length::Fill).height(Length::Fill).style(move |_| container::Style { background: Some(Background::Gradient(fade.into())), ..container::Style::default() }).into()
}

fn pool_card<'a>(state: &'a State, shown: Shown, mark: Element<'a, Message>, foot: Option<Element<'a, Message>>, press: Option<Message>, wide: f32, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>) -> Element<'a, Message> {
    let k = ui::fade();
    let top = row![mark, ui::grow(), plate(text(shown.when.clone()).font(theme::MONO).size(12.0).color(ui::faded(INK)).into())].spacing(8).align_y(iced::Center);
    let title = text(shown.title.clone()).font(theme::SANS_SEMI).size(22.0).color(ui::faded(INK)).wrapping(text::Wrapping::None);
    let head = iced::widget::stack![
        container(mosaic(thumbs, &shown.hashes, 4, 2, wide - 22.0, CARD_TOP - 10.0, 4.0, 6.0)).padding(Padding { left: 10.0, right: 10.0, top: 10.0, bottom: 0.0 }),
        shade(0.0),
        container(top).padding([18, 18]).width(Length::Fill),
        container(container(title).width(Length::Fill).clip(true)).padding(Padding { left: 16.0, right: 16.0, bottom: 4.0, top: 0.0 }).height(Length::Fill).align_y(iced::alignment::Vertical::Bottom),
    ]
    .width(Length::Fill)
    .height(CARD_TOP);
    let mut facts = row![fact(shown.cards.to_string(), words.t("shelf-cards"))].spacing(18).align_y(iced::alignment::Vertical::Bottom);
    if shown.length > 0 {
        facts = facts.push(fact(words.length(shown.length), words.t("shelf-length")));
    }
    facts = facts.push(ui::grow()).push(stars_span(words, shown.low, shown.high));
    let mut body = column![makers_line(state, &shown.makers, words, 14.0), facts, mod_strip(&shown.mods, true)].spacing(12);
    if let Some(foot) = foot {
        let line = container(Space::new().height(1.0)).width(Length::Fill).style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08 * k))), ..container::Style::default() });
        body = body.push(column![line, foot].spacing(10));
    }
    let inside = column![container(head).clip(true), container(body).padding(Padding { left: 16.0, right: 16.0, top: 8.0, bottom: 14.0 })];
    let made = button(container(inside).clip(true)).padding(1).width(Length::FillPortion(1)).style(ui::button_faded(card_style));
    match press {
        Some(press) => made.on_press(press).into(),
        None => made.into(),
    }
}

fn hero<'a>(state: &'a State, pool: &'a Pool, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, width: f32, today: i64) -> Element<'a, Message> {
    let k = ui::fade();
    let shown = shown_of(pool, words, words.day(pool.changed_at, today));
    let beside = width >= 900.0;
    let side = if beside { (width * 0.34).clamp(280.0, 420.0) } else { 0.0 };
    let mark: Element<'a, Message> = match (&pool.saved, pool.published_revision) {
        (Some(saved), _) => code_plate(&saved.code, saved.revision),
        (None, 0) => word_plate(words.t("shelf-draft")),
        (None, revision) => code_plate(state.publications.iter().find(|item| item.mine && item.local_id == pool.id).map(|item| item.code.as_str()).unwrap_or_default(), revision),
    };
    let mut facts = row![fact(shown.cards.to_string(), words.t("shelf-cards"))].spacing(28).align_y(iced::alignment::Vertical::Bottom);
    if shown.length > 0 {
        facts = facts.push(fact(words.length(shown.length), words.t("shelf-length")));
    }
    if shown.low.is_some() {
        facts = facts.push(column![stars_span(words, shown.low, shown.high), faded_text(words.t("shelf-stars"), 12.0, MUTED)].spacing(3));
    }
    facts = facts.push(fact(shown.when.clone(), words.t("shelf-changed")));
    let left = column![
        row![ui::mono_small(words.t("shelf-latest").to_uppercase(), MUTED), mark].spacing(10).align_y(iced::Center),
        container(text(shown.title.clone()).font(theme::SANS_SEMI).size(30.0).color(ui::faded(INK)).wrapping(text::Wrapping::None)).width(Length::Fill).clip(true),
        makers_line(state, &shown.makers, words, 14.0),
        Space::new().height(Length::Fill),
        facts,
        mod_strip(&shown.mods, true),
    ]
    .spacing(14)
    .width(Length::Fill);
    let mut inside = row![left].spacing(32);
    if beside {
        let open = container(text(words.t("shelf-open")).font(theme::SANS_SEMI).size(15.0).color(ui::faded(INK)))
            .padding([10, 18])
            .style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.14 * k))), border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.2 * k), width: 1.0, radius: 10.0.into() }, ..container::Style::default() });
        inside = inside.push(column![mosaic(thumbs, &shown.hashes, 5, 3, side, side * 0.36, 6.0, 6.0), Space::new().height(Length::Fill), container(open).align_right(Length::Fill)].width(side));
    }
    button(container(inside).padding([26, 30]).height(288.0))
        .padding(1)
        .width(Length::Fill)
        .style(ui::button_faded(|_: &iced::Theme, status: button::Status| {
            let hot = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let fade = iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2))
                .add_stop(0.0, Color::from_rgb(0.105, 0.043, 0.062))
                .add_stop(1.0, Color::from_rgb(0.20, 0.07, 0.10));
            button::Style { background: Some(Background::Gradient(fade.into())), text_color: INK, border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, if hot { 0.3 } else { 0.12 }), width: 1.0, radius: 20.0.into() }, shadow: iced::Shadow::default(), snap: true }
        }))
        .on_press(Message::Open(pool.id.clone()))
        .into()
}

fn tiled<'a>(cards: Vec<Element<'a, Message>>, across: usize) -> Element<'a, Message> {
    let mut lines = column![].spacing(CARD_GAP);
    let mut line = row![].spacing(CARD_GAP);
    let mut held = 0;
    for card in cards {
        line = line.push(card);
        held += 1;
        if held == across {
            lines = lines.push(line);
            line = row![].spacing(CARD_GAP);
            held = 0;
        }
    }
    if held > 0 {
        for _ in held..across {
            line = line.push(Space::new().width(Length::FillPortion(1)));
        }
        lines = lines.push(line);
    }
    lines.into()
}

fn missing_of(state: &State, pool: &Pool) -> Option<usize> {
    let songs = state.songs.as_ref()?;
    Some(pool.slots.iter().filter_map(|slot| slot.hash.as_ref()).filter(|hash| !songs.contains_key(*hash)).count())
}

fn showcase<'a>(state: &'a State, mut page: iced::widget::Column<'a, Message>, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, width: f32, t: f32, clocks: &Clocks) -> Element<'a, Message> {
    let across = if width >= 960.0 { 3 } else if width >= 620.0 { 2 } else { 1 };
    let wide = ((width - CARD_GAP * (across as f32 - 1.0)) / across as f32).floor();
    let note = |key: &str| -> Element<'a, Message> { container(faded_text(words.t(key), 14.0, MUTED)).padding([18, 2]).into() };
    let mark_of = |pool: &Pool| -> Element<'a, Message> {
        match (&pool.saved, pool.published_revision) {
            (Some(saved), _) => code_plate(&saved.code, saved.revision),
            (None, 0) => word_plate(words.t("shelf-draft")),
            (None, revision) => code_plate(state.publications.iter().find(|item| item.mine && item.local_id == pool.id).map(|item| item.code.as_str()).unwrap_or_default(), revision),
        }
    };
    match state.shelf {
        Shelf::Mine => {
            let mine: Vec<&Pool> = state.list.iter().filter(|pool| !pool.collection && pool.saved.is_none()).collect();
            match mine.split_first() {
                None => page = page.push(ui::appearing(ui::appear(t, 1), 8.0, || note("shelf-empty-mine"))),
                Some((first, rest)) => {
                    page = page.push(ui::appearing(ui::appear(t, 1), 10.0, || hero(state, first, words, thumbs, width, clocks.today)));
                    let cards = rest.iter().enumerate().map(|(at, pool)| ui::appearing(ui::appear(t, 2 + at / across), 10.0, || pool_card(state, shown_of(pool, words, words.day(pool.changed_at, clocks.today)), mark_of(pool), None, Some(Message::Open(pool.id.clone())), wide, words, thumbs))).collect();
                    page = page.push(tiled(cards, across));
                }
            }
        }
        Shelf::Saved => {
            let saved: Vec<&Pool> = state.list.iter().filter(|pool| !pool.collection && pool.saved.is_some()).collect();
            if saved.is_empty() {
                page = page.push(ui::appearing(ui::appear(t, 1), 8.0, || note("shelf-empty-saved")));
            }
            let cards = saved.iter().enumerate().map(|(at, pool)| {
                let foot: Element<'a, Message> = match missing_of(state, pool) {
                    Some(0) => row![glyph(Icon::Check, 13.0, Color::from_rgb(0.765, 0.831, 0.651)), faded_text(words.t("shelf-all-here"), 13.0, Color::from_rgb(0.765, 0.831, 0.651))].spacing(8).align_y(iced::Center).into(),
                    Some(missing) => faded_text(words.n("shelf-missing", missing as u64), 13.0, MUTED),
                    None => faded_text(words.t("shelf-checking"), 13.0, FAINT),
                };
                ui::appearing(ui::appear(t, 1 + at / across), 10.0, || pool_card(state, shown_of(pool, words, words.day(pool.changed_at, clocks.today)), mark_of(pool), Some(container(foot).height(30.0).center_y(30.0).into()), Some(Message::Open(pool.id.clone())), wide, words, thumbs))
            }).collect();
            page = page.push(tiled(cards, across));
        }
        Shelf::Published => {
            let items: Vec<&crate::bot::Publication> = state.publications.iter().filter(|item| item.kind == "pool").collect();
            if state.catalogue_loading && items.is_empty() {
                page = page.push(ui::appearing(ui::appear(t, 1), 8.0, || note("catalogue-loading")));
            } else if items.is_empty() {
                page = page.push(ui::appearing(ui::appear(t, 1), 8.0, || note("catalogue-empty")));
            }
            let cards = items.iter().enumerate().filter_map(|(at, item)| {
                let pool = serde_json::to_vec(&item.content).ok().and_then(|bytes| pool_share::from_file(&bytes, 0).ok())?;
                let mut shown = shown_of(&pool, words, String::new());
                shown.title = item.name.clone();
                shown.when = format!("#{}", item.revision);
                let held = state.list.iter().any(|pool| pool.saved.as_ref().is_some_and(|saved| saved.id == item.id));
                let action: Element<'a, Message> = if item.mine {
                    crate::billing::outlined(words.t("ledger-edit"), Some(Message::OpenPublication(item.id.clone())))
                } else if held {
                    plate(text(words.t("shelf-saved-mark")).font(theme::SANS_SEMI).size(12.0).color(ui::faded(Color::from_rgb(0.765, 0.831, 0.651))).into())
                } else {
                    crate::billing::outlined(words.t("shelf-save"), Some(Message::SavePublication(item.id.clone())))
                };
                let publisher: Element<'a, Message> = match pool.compiler.as_str() {
                    "" => Space::new().width(Length::Fill).into(),
                    name => row![ui::mono_small(words.t("shelf-published-by").to_uppercase(), MUTED), container(named_face(state, name, 120.0)).width(Length::Fill).clip(true)].spacing(8).align_y(iced::Center).width(Length::Fill).into(),
                };
                let foot = row![publisher, action].spacing(8).align_y(iced::Center);
                let mark = if item.code.is_empty() { Space::new().into() } else { plate(text(item.code.clone()).font(theme::MONO_BOLD).size(12.0).color(ui::faded(INK)).into()) };
                let seen = t.min(clocks.of(&item.id));
                Some(ui::appearing(ui::appear(seen, 1 + at / across), 10.0, || pool_card(state, shown, mark, Some(foot.into()), item.mine.then(|| Message::OpenPublication(item.id.clone())), wide, words, thumbs)))
            }).collect();
            page = page.push(tiled(cards, across));
            if let Some(error) = &state.catalogue_error {
                page = page.push(ui::appearing(ui::appear(t, 2), 8.0, || catalogue_error(error, words)));
            }
            if state.catalogue_more {
                page = page.push(quiet_button(words.t("catalogue-more"), Message::CatalogueMore));
            }
        }
    }
    page.into()
}

fn catalogue_error<'a>(error: &str, words: &Words) -> Element<'a, Message> {
    let said = words.t(match error { "401" => "catalogue-sign-in", "403" => "catalogue-profile-required", "409" => "catalogue-conflict", "400" => "catalogue-invalid", _ => "catalogue-unavailable" });
    let k = ui::fade();
    container(row![glyph(Icon::Warn, 15.0, ACCENT), faded_text(said, 13.0, ACCENT)].spacing(10).align_y(iced::Center))
        .padding([8, 14])
        .style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.886, 0.282, 0.282, 0.018 * k))),
            border: Border { color: Color::from_rgba(0.886, 0.282, 0.282, 0.16 * k), width: 1.0, radius: 10.0.into() },
            ..container::Style::default()
        })
        .into()
}

fn conflict_banner<'a>(state: &State, pool: &Pool, words: &'a Words, width: f32) -> Element<'a, Message> {
    let k = ui::fade();
    let busy = state.resolving.is_some() || state.catalogue_loading;
    let mut told = column![semi(words.t("catalogue-conflict-title"), 15.0, WARNED)].spacing(4);
    if let Some(theirs) = state.server_revision(pool).filter(|theirs| *theirs != pool.published_revision) {
        told = told.push(mono(words.with("catalogue-conflict-versions", &[("theirs", theirs.to_string()), ("mine", pool.published_revision.to_string())]), 13.0, INK));
    }
    let actions = row![
        crate::billing::outlined(words.t("catalogue-conflict-theirs"), (!busy).then_some(Message::Conflict(Keep::Theirs))),
        crate::billing::outlined(words.t("catalogue-conflict-mine"), (!busy).then_some(Message::Conflict(Keep::Mine))),
    ]
    .spacing(8);
    let said = row![glyph(Icon::Close, 16.0, WARNED), container(told).width(Length::Fill)].spacing(14).align_y(iced::Center);
    let inside: Element<'a, Message> = if width < 900.0 { column![said, actions].spacing(12).into() } else { said.push(actions).into() };
    container(inside)
        .padding([14, 18])
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.886, 0.282, 0.282, 0.05 * k))),
            border: Border { color: Color::from_rgba(0.886, 0.282, 0.282, 0.4 * k), width: 1.0, radius: 12.0.into() },
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

fn check_box<'a>(on: bool) -> Element<'a, Message> {
    let k = ui::fade();
    let inside: Element<'a, Message> = if on { glyph(Icon::Check, 13.0, Color::WHITE) } else { Space::new().into() };
    container(inside)
        .width(22.0)
        .height(22.0)
        .center(22.0)
        .style(move |_| container::Style {
            background: on.then_some(Background::Color(Color { a: k, ..ACCENT })),
            border: Border { color: if on { Color { a: k, ..ACCENT } } else { Color::from_rgba(1.0, 1.0, 1.0, 0.16 * k) }, width: 1.0, radius: 6.0.into() },
            ..container::Style::default()
        })
        .into()
}

fn slot_row<'a>(pool: &'a Pool, at: usize, selected: bool, choosing: bool, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, songs: Option<&Arc<HashMap<String, Map>>>, ail: Ail) -> Element<'a, Message> {
    let slot = &pool.slots[at];
    let mut left = row![].spacing(10).align_y(iced::Center);
    if choosing {
        left = left.push(check_box(selected));
    }
    left = left.push(number_badge(category_number(pool, at), selected));
    if !pool.collection {
        left = left.push(if choosing { mod_badge(slot.mods, selected) } else { category_menu(pool, at) });
    }
    let left: Element<'a, Message> = left.into();
    let inside: Element<'a, Message> = match slot.hash.as_deref() {
        None => row![left, hatched_cover(), container(semi(words.t("pool-slot-empty"), 14.5, MUTED)).width(Length::Fill)].spacing(14).align_y(iced::Center).into(),
        Some(hash) => {
            let _ = songs;
            let trouble = |key: &str| -> Element<'a, Message> { row![glyph(Icon::Close, 11.0, WARNED), faded_text(words.t(key), 12.5, WARNED)].spacing(6).align_y(iced::Center).into() };
            let under: Element<'a, Message> = match (ail, slot.measure.is_some()) {
                (Ail::Missing, _) => trouble("pool-no-songs"),
                (Ail::Unmeasured, _) => trouble("pool-unmeasured"),
                (Ail::Fetching, _) => row![ui::dot(8.0), faded_text(words.t("pool-from-mirror"), 12.5, INK)].spacing(6).align_y(iced::Center).into(),
                (Ail::Well, false) => faded_text(words.t("pool-measuring"), 12.5, FAINT),
                (Ail::Well, true) => ui::moving_text(slot.artist.clone(), theme::SANS, 12.5, MUTED),
            };
            let details = column![ui::moving_text(slot.title.clone(), theme::SANS_SEMI, 14.5, INK), under].spacing(2).width(Length::Fill);
            let stars: Element<'a, Message> = slot.measure.map(|measure| crate::dossier::star_pill(stars_of(words, measure.stars), measure.stars as f32, 15.0)).unwrap_or_else(|| Space::new().into());
            row![left, cover(thumbs, Some(hash), COVER_WIDE, COVER_HIGH, COVER_ROUND), container(details).width(Length::Fill).clip(true), stars].spacing(14).align_y(iced::Center).into()
        }
    };
    let colour = if pool.collection { Color::from_rgb8(95, 95, 95) } else { slot_colour(slot) };
    let row_button = button(container(inside).height(ROW_HIGH).align_y(iced::Center))
        .padding([0, 14])
        .width(Length::Fill)
        .style(ui::button_faded(move |theme, status| {
            let mut style = row_style(selected)(theme, status);
            style.background = None;
            style.border.color = ui::faded(colour);
            style.border.width = if selected || matches!(status, button::Status::Hovered | button::Status::Pressed) { 2.0 } else { 0.7 };
            style
        }))
        .on_press(if choosing { Message::Mark(at) } else { Message::Select(Some(at)) });
    let action: Option<Element<'a, Message>> = match (ail, choosing) {
        (Ail::Missing, false) => Some(crate::billing::outlined(words.t("pool-find-mirror"), Some(Message::FetchSlot(at)))),
        (Ail::Unmeasured, false) => Some(crate::billing::outlined(words.t("pool-fetch-again"), Some(Message::FetchSlot(at)))),
        (Ail::Fetching, false) => Some(quiet_button(words.t("pool-link-cancel"), Message::Dismiss)),
        _ => None,
    };
    match action {
        Some(action) => iced::widget::stack![row_button, container(action).align_right(Length::Fill).center_y(ROW_HIGH).padding(Padding::ZERO.right(14.0))].into(),
        None => row_button.into(),
    }
}

fn category_menu<'a>(pool: &Pool, at: usize) -> Element<'a, Message> {
    let length = pool.slots.len();
    let mut categories: Vec<String> = Mod::ALL.iter().map(|mods| mods.code().to_owned()).chain(pool.categories.keys().cloned()).chain(pool.slots.iter().filter(|slot| !slot.category.is_empty()).map(|slot| slot.category.clone())).collect();
    categories.sort();
    categories.dedup();
    let selected = pool.slots.get(at).map(|slot| if slot.category.is_empty() { slot.mods.code().to_owned() } else { slot.category.clone() }).unwrap_or_else(|| "NM".into());
    let (k, ink, muted, edge) = (ui::fade(), ui::faded(INK), ui::faded(MUTED), ui::faded(Color::from_rgba(1.0, 1.0, 1.0, 0.08)));
    pick_list(categories, Some(selected), move |name| {
        match Mod::ALL.into_iter().find(|mods| mods.code() == name) {
            Some(mods) if at < length => Message::SetMod(at, mods),
            Some(mods) => Message::AddMod(mods),
            None => Message::Category(at, name),
        }
    }).font(theme::MONO).text_size(12.0).padding([5, 8]).width(70.0)
        .menu_style(|_| iced::widget::overlay::menu::Style { background: Background::Color(theme::GROUND), border: Border { color: MUTED, width: 1.0, radius: 7.0.into() }, text_color: INK, selected_text_color: INK, selected_background: Background::Color(Color::from_rgb8(65, 30, 32)), shadow: iced::Shadow::default() })
        .style(move |_, status| pick_list::Style { text_color: ink, placeholder_color: muted, handle_color: muted, background: Background::Color(Color::from_rgba(1.0, 1.0, 1.0, k * if matches!(status, pick_list::Status::Hovered | pick_list::Status::Opened { .. }) { 0.08 } else { 0.03 })), border: Border { color: edge, width: 1.0, radius: 7.0.into() } }).into()
}

fn mod_menu<'a>(mods: Mod, change: impl Fn(Mod) -> Message + 'a) -> Element<'a, Message> {
    pick_list(Mod::ALL.map(Mod::code), Some(mods.code()), move |code| {
        change(Mod::ALL.into_iter().find(|mods| mods.code() == code).unwrap())
    })
        .font(theme::MONO)
        .text_size(12.0)
        .padding([5, 8])
        .width(70.0)
        .menu_style(|_| iced::widget::overlay::menu::Style {
            background: Background::Color(theme::GROUND),
            border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.12), width: 1.0, radius: 7.0.into() },
            text_color: INK, selected_text_color: INK,
            selected_background: Background::Color(Color::from_rgba(0.886, 0.282, 0.282, 0.22)),
            shadow: iced::Shadow::default(),
        })
        .style(|_, status| pick_list::Style {
            text_color: ui::faded(INK), placeholder_color: ui::faded(MUTED), handle_color: ui::faded(MUTED),
            background: Background::Color(Color::from_rgba(1.0, 1.0, 1.0, if matches!(status, pick_list::Status::Hovered | pick_list::Status::Opened { .. }) { 0.08 } else { 0.03 })),
            border: Border { color: ui::faded(Color::from_rgba(1.0, 1.0, 1.0, 0.08)), width: 1.0, radius: 7.0.into() },
        })
        .into()
}

fn figure<'a>(label: String, value: String, size: f32, colour: Color) -> Element<'a, Message> {
    column![ui::mono_small(label, FAINT), text(value).font(theme::MONO_BOLD).size(size).wrapping(text::Wrapping::None).color(ui::faded(colour))].spacing(4).into()
}

fn profile_bars<'a>(measure: &Measure, words: &Words) -> Element<'a, Message> {
    let k = ui::fade();
    let percents = measure.percents();
    let mut list = column![ui::mono_small(words.t("pool-slot-profile"), FAINT)].spacing(10);
    for (at, skill) in Skill::ALL.into_iter().enumerate() {
        let filled = percents[at].clamp(0, 100) as u16;
        let track = container(
            row![
                container(Space::new().height(5.0)).width(Length::FillPortion(filled.max(1))).style(move |_| container::Style { background: Some(Background::Color(Color { a: k, ..ACCENT })), border: Border { radius: 2.5.into(), ..Border::default() }, ..container::Style::default() }),
                Space::new().width(Length::FillPortion((100 - filled).max(1))),
            ],
        )
        .width(Length::Fill)
        .style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.03 * k))), border: Border { radius: 2.5.into(), ..Border::default() }, ..container::Style::default() });
        list = list.push(
            row![
                container(faded_text(words.t(skill.key()), 13.0, MUTED)).width(110.0),
                track,
                container(mono(format!("{}%", percents[at]), 12.0, MUTED)).width(40.0).align_x(iced::alignment::Horizontal::Right),
            ]
            .spacing(12)
            .align_y(iced::Center),
        );
    }
    list.into()
}

fn category_fields<'a>(pool: &'a Pool, editor: &'a Editor, at: usize, words: &'a Words) -> Element<'a, Message> {
    if pool.collection { return Space::new().into(); }
    if !editor.category_creator { return quiet_button(words.t("pool-create-category"), Message::CreateCategory(true)); }
    let name = text_input(&words.t("pool-custom-category"), &editor.category_draft).on_input(Message::CategoryDraft).font(theme::SANS).size(12.0).padding([7, 10]).style(theme::field_faded(ui::fade()));
    let mut fields = column![name].spacing(8);
    fields = fields.push(row![ui::mono_small(words.t("pool-category-colour"), MUTED), text_input("#5ec2d0", &editor.colour_draft).on_input(move |value| Message::Colour(at, value)).font(theme::MONO).size(12.0).padding([7, 10]).style(theme::field_faded(ui::fade())).width(120.0)].spacing(12).align_y(iced::Center));
    let valid = !editor.category_draft.is_empty() && !Mod::ALL.iter().any(|m| m.code() == editor.category_draft) && parse_colour(&editor.colour_draft).is_some();
    fields = fields.push(row![ui::primary(words.t("pool-create-category"), valid.then_some(Message::SaveCategory(at))), quiet_button(words.t("pool-delete-no"), Message::CreateCategory(false))].spacing(8));
    fields.into()
}

fn slot_panel<'a>(pool: &'a Pool, editor: &'a Editor, at: usize, wide: f32, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, songs: Option<&Arc<HashMap<String, Map>>>) -> Element<'a, Message> {
    let slot = &pool.slots[at];
    let close = button(glyph(Icon::Close, 14.0, FAINT)).padding(6).style(ui::button_faded(theme::bare)).on_press(Message::Select(None));
    let category = if pool.collection { Space::new().into() } else { category_menu(pool, at) };
    let mut body = column![row![number_badge(category_number(pool, at), true), category, ui::grow(), close].spacing(10).align_y(iced::Center)].spacing(16);
    body = body.push(category_fields(pool, editor, at, words));
    match slot.hash.as_deref() {
        None => {
            body = body.push(semi(words.t("pool-slot-empty"), 18.0, INK));
            body = body.push(faded_text(words.t("pool-slot-need"), 13.0, MUTED));
            body = body.push(primary_button(words.t("pool-add"), Message::AddPanel(true)));
        }
        Some(hash) => {
            body = body.push(cover(thumbs, Some(hash), wide - 32.0, 150.0, HERO_ROUND));
            body = body.push(column![wrapped_semi(slot.title.clone(), 20.0, INK), ui::moving_text(slot.artist.clone(), theme::SANS, 14.0, MUTED), ui::moving_text(slot.version.clone(), theme::MONO, 12.0, FAINT)].spacing(3).width(Length::Fill));
            match slot.measure {
                Some(measure) => {
                    body = body.push(
                        row![
                            container(figure(words.t("pool-stars"), stars_of(words, measure.stars), 26.0, crate::dossier::star_colour(measure.stars as f32))).width(Length::Fill),
                            container(figure(words.t("pool-bpm"), format!("{:.0}", measure.bpm), 18.0, INK)).width(Length::Fill),
                            container(figure(words.t("pool-length"), pools::clock(measure.length_ms), 18.0, INK)).width(Length::Fill),
                            container(figure(words.t("pool-combo"), measure.max_combo.to_string(), 18.0, INK)).width(Length::Fill),
                        ]
                        .spacing(12)
                        .align_y(iced::alignment::Vertical::Bottom),
                    );
                    let one = |value: f64| screen::decimal(words, value as f32, 1);
                    body = body.push(
                        row![
                            container(figure("AR".to_owned(), one(measure.ar), 16.0, INK)).width(Length::Fill),
                            container(figure("OD".to_owned(), one(measure.od), 16.0, INK)).width(Length::Fill),
                            container(figure("CS".to_owned(), one(measure.cs), 16.0, INK)).width(Length::Fill),
                            container(figure("HP".to_owned(), one(measure.hp), 16.0, INK)).width(Length::Fill),
                        ]
                        .spacing(12),
                    );
                    body = body.push(profile_bars(&measure, words));
                }
                None if songs.is_some_and(|songs| !songs.contains_key(hash)) => body = body.push(mono(words.t("pool-no-disk"), 12.0, ACCENT)),
                None => body = body.push(mono(words.t("pool-measuring"), 12.0, FAINT)),
            }
        }
    }
    if !slot.is_empty() {
        body = body.push(
            column![
                ui::mono_small(words.t("pool-slot-note"), FAINT),
                text_input("", &slot.note).on_input(move |note| Message::Note(at, note)).on_paste(move |contents| Message::PasteInto(Input::Note(at), contents)).font(theme::SANS).size(13.0).padding([8, 12]).style(theme::field_faded(ui::fade())),
            ]
            .spacing(8),
        );
        body = body.push(
            row![
                quiet_button(words.t("pool-replace"), Message::Replace),
                button(text(words.t("pool-remove")).font(theme::SANS_SEMI).size(13.0).color(ui::faded(MUTED))).padding([8, 12]).style(ui::button_faded(theme::bare)).on_press(Message::Clear(at)),
            ]
            .spacing(8)
            .align_y(iced::Center),
        );
    }
    panel_box(body.into())
}

fn finding_card<'a>(finding: &'a Finding, editor: &'a Editor, pool: &'a Pool, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>) -> Element<'a, Message> {
    match finding {
        Finding::Asking => mono(words.t("pool-link-asking"), 12.0, FAINT),
        Finding::Refused(mode) => {
            let inside = column![
                row![glyph(Icon::Warn, 16.0, ACCENT), semi(words.with("pool-mode-title", &[("mode", pool_links::mode_name(mode).to_owned())]), 14.5, INK)].spacing(10).align_y(iced::Center),
                para(words.t("pool-mode-text"), 13.0, MUTED),
                quiet_button(words.t("pool-got-it"), Message::Dismiss),
            ]
            .spacing(10);
            panel_box(inside.into())
        }
        Finding::Missing => {
            let inside = column![row![glyph(Icon::Warn, 16.0, ACCENT), semi(words.t("pool-missing"), 14.0, INK)].spacing(10).align_y(iced::Center), quiet_button(words.t("pool-got-it"), Message::Dismiss)].spacing(10);
            panel_box(inside.into())
        }
        Finding::Silent(_) => {
            let inside = column![row![glyph(Icon::Warn, 16.0, ACCENT), semi(words.t("pool-silent"), 14.0, INK)].spacing(10).align_y(iced::Center), quiet_button(words.t("pool-retry"), Message::Retry)].spacing(10);
            panel_box(inside.into())
        }
        Finding::Fetching(fetching) => fetching_card(fetching, words),
        Finding::Found(candidate) => candidate_card(candidate, editor, pool, words, thumbs),
    }
}

fn stage_of(step: Option<&crate::maps::Step>) -> usize {
    use crate::maps::Step;
    match step {
        None | Some(Step::Looking) => 0,
        Some(Step::Found(_) | Step::Downloading { .. }) => 1,
        Some(Step::Unpacking) => 2,
        Some(_) => 3,
    }
}

fn fetching_card<'a>(fetching: &'a Fetching, words: &'a Words) -> Element<'a, Message> {
    use crate::maps::Step;
    let stage = stage_of(fetching.step.as_ref());
    let done_n = fetching.total - fetching.queue.len();
    let megabytes = |bytes: u64| screen::decimal(words, bytes as f32 / 1_048_576.0, 1);
    let download = match &fetching.step {
        Some(Step::Downloading { done, total: Some(total), .. }) => words.with("pool-step-download-of", &[("done", megabytes(*done)), ("total", megabytes(*total))]),
        _ => words.t("pool-step-download"),
    };
    let lines = [
        words.t(if stage == 0 { "pool-step-looking" } else { "pool-step-found" }),
        download,
        words.t("pool-step-unpack"),
        words.t("pool-step-check"),
    ];
    let mut list = column![].spacing(8);
    for (at, line) in lines.into_iter().enumerate() {
        let mark: Element<'a, Message> = if at < stage {
            glyph(Icon::Check, 14.0, ACCENT)
        } else if at == stage {
            container(container(Space::new()).width(7.0).height(7.0).style({ let dot = ui::faded(ACCENT); move |_| container::Style { background: Some(Background::Color(dot)), border: Border { radius: 3.5.into(), ..Border::default() }, ..container::Style::default() } })).center(14.0).into()
        } else {
            Space::new().width(14.0).into()
        };
        let colour = if at == stage { INK } else if at < stage { MUTED } else { FAINT };
        list = list.push(row![mark, faded_text(line, 13.5, colour)].spacing(10).align_y(iced::Center));
    }
    let inside = column![
        row![semi(words.with("pool-fetching", &[("n", (done_n + 1).min(fetching.total).to_string()), ("total", fetching.total.to_string())]), 14.5, INK), ui::grow(), button(glyph(Icon::Close, 14.0, FAINT)).padding([4, 6]).style(ui::button_faded(theme::bare)).on_press(Message::Dismiss)].align_y(iced::Center),
        list,
    ]
    .spacing(12);
    panel_box(inside.into())
}

fn candidate_card<'a>(candidate: &'a Candidate, _editor: &'a Editor, _pool: &'a Pool, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>) -> Element<'a, Message> {
    let found = &candidate.found;
    let many = found.difficulties.len() > 1;
    let title = if many { words.n("pool-link-set", found.difficulties.len() as u64) } else { words.t("pool-link-found") };
    let heading = row![glyph(Icon::Chain, 15.0, ACCENT), semi(title, 14.5, INK)].spacing(10).align_y(iced::Center);
    let local_cover = candidate.found.difficulties.get(candidate.choice.unwrap_or(0)).and_then(|difficulty| thumbs.get(&difficulty.hash));
    let thumb: Element<'a, Message> = match local_cover.or(candidate.cover.as_ref()) {
        Some(handle) => image(crate::crops::fitted(handle, PANEL_WIDE - 64.0, 110.0, 12.0)).content_fit(iced::ContentFit::Fill).width(Length::Fill).height(110.0).border_radius(12.0).opacity(ui::fade()).into(),
        None => container(Space::new())
            .width(Length::Fill)
            .height(110.0)
            .style(|_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.012))), border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.04), width: 1.0, radius: 10.0.into() }, ..container::Style::default() })
            .into(),
    };
    let names = column![wrapped_semi(found.title.clone(), 17.0, INK), ui::moving_text(found.artist.clone(), theme::SANS, 13.5, MUTED)].spacing(2).width(Length::Fill);
    let mut list = column![].spacing(6);
    for (at, difficulty) in found.difficulties.iter().enumerate() {
        let lit = candidate.choice == Some(at);
        let colour = crate::dossier::star_colour(difficulty.stars as f32);
        let dot = container(Space::new()).width(9.0).height(9.0).style(move |_| container::Style { background: Some(Background::Color(colour)), border: Border { radius: 4.5.into(), ..Border::default() }, ..container::Style::default() });
        let line = row![dot, wrapped_semi(difficulty.version.clone(), 13.5, INK), container(mono(stars_of(words, difficulty.stars), 13.0, colour)).width(48.0).align_x(iced::Right)].spacing(10).align_y(iced::Center);
        let made: Element<'a, Message> = if many {
            button(line).padding([8, 12]).width(Length::Fill).style(ui::button_faded(row_style(lit))).on_press(Message::Choose(at)).into()
        } else {
            container(line).padding([4, 4]).into()
        };
        list = list.push(made);
    }
    let mut actions = row![].spacing(8).align_y(iced::Center);
    let put_label = words.t("pool-link-put");
    actions = actions.push(if candidate.choice.is_some() { primary_button(put_label, Message::Confirm) } else { ui::primary(put_label, None) });
    actions = actions.push(button(text(words.t("pool-link-cancel")).font(theme::SANS_SEMI).size(13.0).color(ui::faded(MUTED))).padding([8, 12]).style(ui::button_faded(theme::bare)).on_press(Message::Dismiss));
    let inside = column![heading, thumb, names, list, actions].spacing(12);
    panel_box(inside.into())
}

fn add_panel<'a>(state: &'a State, editor: &'a Editor, pool: &'a Pool, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>) -> Element<'a, Message> {
    let mut body = column![].spacing(14);
    let sources = [(SourceTab::Search, "pool-tab-search"), (SourceTab::Collections, "pool-tab-collections"), (SourceTab::Best, "pool-tab-best"), (SourceTab::Suggest, "pool-tab-suggest")];
    let buttons = row(sources.iter().map(|(source, key)| {
        let lit = editor.source == *source;
        button(text(words.t(key)).font(theme::SANS_SEMI).size(13.5).color(ui::faded(if lit { INK } else { MUTED })).width(Length::Fill).center())
            .padding([8, 12]).width(Length::Fill).style(ui::button_faded(theme::bare)).on_press(Message::Source(*source)).into()
    })).spacing(6);
    let active = sources.iter().position(|(source, _)| *source == editor.source).unwrap_or(0);
    let tabs = ui::sliding(buttons, active, ui::Pill { fill: Color::from_rgba(0.886, 0.282, 0.282, 0.15), edge: Color::from_rgba(0.886, 0.282, 0.282, 0.4), radius: 8.0, underline: None });
    let mut heading = row![].spacing(10).align_y(iced::Center);
    if let Some(at) = target(pool, editor) {
        heading = heading.push(number_badge(category_number(pool, at), true));
        if !pool.collection { heading = heading.push(category_menu(pool, at)); }
    } else {
        if !pool.collection { heading = heading.push(mod_menu(editor.last_mod, Message::AddMod)); }
    }
    heading = heading.push(ui::grow()).push(button(glyph(Icon::Close, 14.0, FAINT)).padding([8, 6]).style(ui::button_faded(theme::bare)).on_press(Message::Select(None)));
    body = body.push(heading).push(tabs);
    body = body.push(category_fields(pool, editor, target(pool, editor).unwrap_or(pool.slots.len()), words));
    if matches!(editor.source, SourceTab::Collections | SourceTab::Best) {
        if let Some(Notice::Already(at)) = &state.notice {
            body = body.push(faded_text(words.with("pool-already", &[("n", (at + 1).to_string())]), 13.0, ACCENT));
        }
        if let Some(finding) = &state.finding {
            body = body.push(finding_card(finding, editor, pool, words, thumbs));
        }
        body = body.push(if editor.source == SourceTab::Best { best_panel(state, words, thumbs) } else { collection_panel(state, editor, words, thumbs) });
        return ui::smooth(panel_box(body.into()));
    }
    if editor.source == SourceTab::Suggest {
        body = body.push(suggestion_panel(state, words, thumbs));
        return ui::smooth(panel_box(body.into()));
    }
    body = body.push(
        text_input(&words.t("pool-search-hint"), &editor.query)
            .id(iced::widget::Id::new("pool-search"))
            .on_input(Message::Query)
            .on_paste(|contents| Message::PasteInto(Input::Query, contents))
            .font(theme::SANS)
            .size(13.5)
            .padding([9, 12])
            .style(theme::field_faded(ui::fade())),
    );
    if let Some(Notice::Already(at)) = &state.notice {
        body = body.push(faded_text(words.with("pool-already", &[("n", (at + 1).to_string())]), 13.0, ACCENT));
    }
    if let Some(Notice::NotALink) = &state.notice {
        body = body.push(para(words.t("pool-not-link"), 13.0, ACCENT));
    }
    if let Some(finding) = &state.finding {
        body = body.push(finding_card(finding, editor, pool, words, thumbs));
    } else if state.songs.is_none() {
        body = body.push(mono(words.t("pool-reading"), 12.0, FAINT));
    } else if editor.query.trim().is_empty() {
        body = body.push(para(words.t("pool-search-start"), 13.0, MUTED));
    } else {
        let found = state.search(&editor.query);
        if found.is_empty() {
            body = body.push(faded_text(words.t("pool-nothing"), 13.0, MUTED));
        }
        let mut list = column![].spacing(2);
        for (hash, map) in found {
            let inside = row![
                cover(thumbs, Some(&hash), 56.0, 32.0, THUMB_ROUND),
                column![semi(map.title.clone(), 13.5, INK), faded_text(map.artist.clone(), 12.0, MUTED), mono(map.version.clone(), 11.0, FAINT)].spacing(1).width(Length::Fill),
                glyph(Icon::Plus, 16.0, MUTED),
            ]
            .spacing(12)
            .align_y(iced::Center);
            list = list.push(button(inside).padding([6, 8]).width(Length::Fill).style(ui::button_faded(ui::calm(theme::row(false)))).on_press(Message::Put(hash)));
        }
        body = body.push(results_list(list));
    }
    ui::smooth(panel_box(body.into()))
}

fn best_panel<'a>(state: &'a State, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>) -> Element<'a, Message> {
    let refresh = quiet_button(words.t("pool-best-refresh"), Message::RefreshBest);
    let Some(scores) = &state.best else {
        return column![para(words.t("pool-best-no-profile"), 13.0, MUTED), refresh].spacing(12).into();
    };
    if scores.is_empty() {
        return column![para(words.t("pool-best-empty"), 13.0, MUTED), refresh].spacing(12).into();
    }
    if state.songs.is_none() {
        return para(words.t("pool-reading"), 13.0, MUTED);
    }
    let mut list = column![].spacing(2);
    for (at, score) in scores.iter().enumerate() {
        let hash = score.hash.to_ascii_lowercase();
        let local = state.songs.as_ref().is_some_and(|songs| songs.contains_key(&hash));
        let mut names = column![semi(score.title.clone(), 13.5, INK), faded_text(score.artist.clone(), 12.0, MUTED), mono(score.version.clone(), 11.0, FAINT)].spacing(1).width(Length::Fill);
        if !local {
            names = names.push(faded_text(words.t("pool-best-missing"), 11.5, FAINT));
        }
        let inside = row![
            cover(thumbs, Some(&hash), 56.0, 32.0, THUMB_ROUND),
            names,
            mono(format!("{} pp", words.lang().group(score.pp.max(0.0).round() as u64)), 12.0, MUTED),
        ]
        .spacing(12)
        .align_y(iced::Center);
        list = list.push(button(inside).padding([6, 8]).width(Length::Fill).style(ui::button_faded(ui::calm(theme::row(false)))).on_press(Message::Best(at)));
    }
    column![refresh, results_list(list)].spacing(10).into()
}

fn suggestion_panel<'a>(state: &'a State, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>) -> Element<'a, Message> {
    let lean = match &state.screen { Screen::Editor(editor) => editor.lean, _ => None };
    let Some(suggestion) = &state.suggestions else {
        return para(words.t("pool-suggest-no-slot"), 13.0, MUTED);
    };
    let Some(target) = suggestion.target else {
        return para(words.t("pool-suggest-needs-map"), 13.0, MUTED);
    };
    let Some(maps) = &suggestion.maps else {
        return para(words.t("pool-suggest-running"), 13.0, MUTED);
    };
    if maps.is_empty() {
        return para(words.t("pool-suggest-empty"), 13.0, MUTED);
    }
    let mut list = column![].spacing(2);
    for (hash, measure) in maps {
        let Some(map) = state.songs.as_ref().and_then(|songs| songs.get(hash)) else { continue };
        let mut inside = row![
            cover(thumbs, Some(hash), 56.0, 32.0, THUMB_ROUND),
            column![semi(map.title.clone(), 13.5, INK), faded_text(map.artist.clone(), 12.0, MUTED), mono(map.version.clone(), 11.0, FAINT)].spacing(1).width(Length::Fill),
        ]
        .spacing(12)
        .align_y(iced::Center);
        if let Some(at) = lean.and_then(|skill| Skill::ALL.iter().position(|other| *other == skill)) {
            inside = inside.push(mono(format!("{}%", measure.percents()[at]), 12.5, INK));
        }
        inside = inside.push(mono(stars_of(words, measure.stars), 12.5, crate::dossier::star_colour(measure.stars as f32)));
        list = list.push(button(inside).padding([6, 8]).width(Length::Fill).style(ui::button_faded(ui::calm(theme::row(false)))).on_press(Message::Put(hash.clone())));
    }
    let mut told = column![faded_text(words.with("pool-suggest-target", &[("stars", stars_of(words, target))]), 12.5, MUTED)].spacing(4);
    if let Some(skill) = lean {
        told = told.push(faded_text(words.t(skill.lean_key()), 13.5, INK));
    }
    column![told, results_list(list)].spacing(12).into()
}

fn collection_panel<'a>(state: &'a State, editor: &'a Editor, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>) -> Element<'a, Message> {
    let Some(collections) = &state.collections else {
        return para(words.t("pool-collections-reading"), 13.0, MUTED);
    };
    if collections.is_empty() {
        return para(words.t("pool-collections-empty"), 13.0, MUTED);
    }
    if state.songs.is_none() {
        return para(words.t("pool-reading"), 13.0, MUTED);
    }
    let songs = state.songs.as_ref();
    let Some(collection) = editor.collection.and_then(|at| collections.get(at)) else {
        let mut list = column![].spacing(4);
        for (at, collection) in collections.iter().enumerate() {
            list = list.push(button(semi(collection.name.clone(), 13.5, INK)).padding([9, 10]).width(Length::Fill).style(ui::button_faded(ui::calm(theme::row(false)))).on_press(Message::Collection(Some(at))));
        }
        return results_list(list);
    };
    let heading = quiet_button(words.t("pool-collection-back"), Message::Collection(None));
    let pages = collection.hashes.len().div_ceil(COLLECTION_PAGE).max(1);
    let page = editor.collection_page.min(pages - 1);
    let mut list = column![].spacing(2);
    for hash in collection.hashes.iter().skip(page * COLLECTION_PAGE).take(COLLECTION_PAGE) {
        let row: Element<'a, Message> = match songs.and_then(|songs| songs.get(hash)) {
            Some(map) => row![
                cover(thumbs, Some(hash), 56.0, 32.0, THUMB_ROUND),
                column![semi(map.title.clone(), 13.5, INK), faded_text(map.artist.clone(), 12.0, MUTED), mono(map.version.clone(), 11.0, FAINT)].spacing(1).width(Length::Fill),
                glyph(Icon::Plus, 16.0, MUTED),
            ]
            .spacing(12)
            .align_y(iced::Center)
            .into(),
            None => row![
                cover(thumbs, None, 56.0, 32.0, THUMB_ROUND),
                column![mono(hash.clone(), 11.0, MUTED), faded_text(words.t("pool-collection-missing"), 12.0, FAINT)].spacing(2).width(Length::Fill),
                glyph(Icon::Plus, 16.0, MUTED),
            ]
            .spacing(12)
            .align_y(iced::Center)
            .into(),
        };
        list = list.push(button(row).padding([6, 8]).width(Length::Fill).style(ui::button_faded(ui::calm(theme::row(false)))).on_press(Message::CollectionHash(hash.clone())));
    }
    let mut body = column![heading, results_list(list)].spacing(10);
    if pages > 1 {
        let mut controls = row![].spacing(10).align_y(iced::Center);
        if page > 0 {
            controls = controls.push(quiet_button(words.t("pool-collection-prev"), Message::CollectionPage(page - 1)));
        }
        controls = controls.push(ui::grow()).push(mono(format!("{} / {}", page + 1, pages), 11.5, MUTED));
        if page + 1 < pages {
            controls = controls.push(quiet_button(words.t("pool-collection-next"), Message::CollectionPage(page + 1)));
        }
        body = body.push(controls);
    }
    body.into()
}

fn share_row<'a>(icon: Icon, title: String, hint: String, action: String, press: Option<Message>) -> Element<'a, Message> {
    let k = ui::fade();
    let tile = container(glyph(icon, 16.0, MUTED))
        .width(38.0)
        .height(38.0)
        .center(38.0)
        .style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.012 * k))), border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.04 * k), width: 1.0, radius: 10.0.into() }, ..container::Style::default() });
    let words = column![semi(title, 14.0, INK), container(mono(hint, 11.5, FAINT)).clip(true)].spacing(3).width(Length::Fill);
    row![tile, words, match press {
        Some(press) => quiet_button(action, press),
        None => ui::quiet(action, None),
    }]
    .spacing(14)
    .align_y(iced::Center)
    .into()
}

fn share_modal<'a>(pool: &'a Pool, words: &'a Words, exporting: bool) -> Element<'a, Message> {
    let line = pool_share::to_text(pool);
    let shown = if line.chars().count() > 46 { format!("{}…", line.chars().take(46).collect::<String>()) } else { line };
    let hash = row![
        ui::mono_small(words.t("pool-hash"), FAINT),
        text(pool.fingerprint()).font(theme::MONO_BOLD).size(14.0).color(ui::faded(INK)),
        button(glyph(Icon::Copy, 14.0, MUTED)).padding([4, 6]).style(ui::button_faded(theme::bare)).on_press(Message::CopyHash),
    ]
    .spacing(10)
    .align_y(iced::Center);
    let inside = column![
        row![semi(words.t("pool-share-title"), 18.0, INK), ui::grow(), hash].align_y(iced::Center),
        share_row(Icon::File, words.t("pool-share-file"), words.t("pool-share-file-hint"), words.t("pool-save"), Some(Message::SaveFile)),
        share_row(Icon::Copy, words.t("pool-share-string"), shown, words.t("pool-copy"), Some(Message::CopyText)),
        share_row(Icon::Pool, words.t("pool-share-image"), words.t("pool-share-image-hint"), words.t(if exporting { "pool-image-saving" } else { "pool-save" }), (!exporting).then_some(Message::SaveImage)),
        para(words.t("pool-share-note"), 12.5, MUTED),
        row![ui::grow(), quiet_button(words.t("pool-close"), Message::Share(false))],
    ]
    .spacing(16);
    let card = sheet_box(inside.into());
    let veil = iced::widget::mouse_area(ui::veil(theme::SCRIM)).on_press(Message::Share(false));
    iced::widget::stack![veil, container(container(card).max_width(620.0).padding(24)).center(Length::Fill)].into()
}

const BALANCE_WIDE: f32 = 380.0;
const BALANCE_BESIDE: f32 = 1100.0;
const BALANCE_GAP: f32 = 22.0;
const BALANCE_FROM: usize = 2;
const CALM_BAR: Color = Color::from_rgb(0.635, 0.227, 0.227);

fn balance_bar<'a>(share: f64, lead: bool) -> Element<'a, Message> {
    let k = ui::fade();
    let filled = ((share * 200.0).round() as u16).clamp(1, 100);
    let ink = if lead { ACCENT } else { CALM_BAR };
    container(row![
        container(Space::new().height(6.0)).width(Length::FillPortion(filled)).style(move |_| container::Style { background: Some(Background::Color(Color { a: k, ..ink })), border: Border { radius: 3.0.into(), ..Border::default() }, ..container::Style::default() }),
        Space::new().width(Length::FillPortion((100 - filled).max(1))),
    ])
    .width(Length::Fill)
    .style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05 * k))), border: Border { radius: 3.0.into(), ..Border::default() }, ..container::Style::default() })
    .into()
}

fn balance_block<'a>(pool: &'a Pool, words: &'a Words, beside: bool) -> Option<Element<'a, Message>> {
    if pool.collection || pool.measured() < BALANCE_FROM {
        return None;
    }
    let profile = pool.profile()?;
    let percents = pools::whole_percents(profile);
    let balance = pool.balance()?;
    let lead = match balance {
        pools::Balance::Heavy(skill, _) => Skill::ALL.iter().position(|other| *other == skill),
        pools::Balance::Even => None,
    };
    let caption = ui::mono_small(words.t("pool-balance").to_uppercase(), MUTED);
    let verdict = semi(match balance { pools::Balance::Heavy(skill, _) => words.t(skill.outweighs_key()), pools::Balance::Even => words.t("pool-even") }, 16.0, INK);
    let share: Option<Element<'a, Message>> = match balance {
        pools::Balance::Heavy(_, percent) => Some(text(format!("{percent}%")).font(theme::MONO_BOLD).size(22.0).color(ui::faded(INK)).into()),
        pools::Balance::Even => None,
    };
    let pick = match balance {
        pools::Balance::Heavy(..) => pool.weakest().map(|skill| crate::billing::outlined(words.t(skill.pick_key()), Some(Message::Lean(skill)))),
        pools::Balance::Even => None,
    };
    let k = ui::fade();
    let plate = move |theme: &iced::Theme| {
        let slab = theme::slab(theme);
        container::Style {
            background: slab.background.map(|fill| match fill { Background::Color(colour) => Background::Color(Color { a: colour.a * k, ..colour }), other => other }),
            border: Border { color: Color { a: slab.border.color.a * k, ..slab.border.color }, width: 1.0, radius: 14.0.into() },
            ..container::Style::default()
        }
    };
    if beside {
        let mut bars = column![].spacing(12);
        for (at, skill) in Skill::ALL.into_iter().enumerate() {
            let ink = if lead == Some(at) { INK } else { MUTED };
            bars = bars.push(row![container(faded_text(words.t(skill.key()), 14.0, ink)).width(116.0), balance_bar(profile[at], lead == Some(at)), container(mono(percents[at].to_string(), 14.0, ink)).width(34.0).align_x(iced::alignment::Horizontal::Right)].spacing(12).align_y(iced::Center));
        }
        let mut head = row![container(verdict).width(Length::Fill)].spacing(12).align_y(iced::Center);
        if let Some(share) = share {
            head = head.push(share);
        }
        let mut inside = column![caption, head, bars].spacing(16);
        if let Some(pick) = pick {
            inside = inside.push(pick);
        }
        return Some(container(inside).padding(22).width(BALANCE_WIDE).style(plate).into());
    }
    let mut told = row![verdict].spacing(14).align_y(iced::Center);
    if let Some(share) = share {
        told = told.push(share);
    }
    let mut head = row![column![caption, told].spacing(6), ui::grow()].spacing(16).align_y(iced::Center);
    if let Some(pick) = pick {
        head = head.push(pick);
    }
    let mut bars = row![].spacing(22);
    for (at, skill) in Skill::ALL.into_iter().enumerate() {
        let ink = if lead == Some(at) { INK } else { MUTED };
        bars = bars.push(column![row![faded_text(words.t(skill.key()), 14.0, ink), ui::grow(), mono(percents[at].to_string(), 14.0, ink)].align_y(iced::Center), balance_bar(profile[at], lead == Some(at))].spacing(8).width(Length::FillPortion(1)));
    }
    Some(container(column![head, bars].spacing(16)).padding([18, 22]).width(Length::Fill).style(plate).into())
}

fn guide_block<'a>(state: &State, editor: &Editor, pool: &'a Pool, words: &'a Words, beside: bool) -> Option<Element<'a, Message>> {
    if !state.guide.on || pool.collection || editor.starting {
        return None;
    }
    let steps = state.guide.steps(pool);
    let done = steps.iter().filter(|step| **step).count();
    let current = steps.iter().position(|step| !*step)?;
    let k = ui::fade();
    let step = |at: usize| -> Element<'a, Message> {
        let mark: Element<'a, Message> = if steps[at] {
            glyph(Icon::Check, 16.0, ACCENT)
        } else {
            let (side, fill) = if at == current { (8.0, Color { a: k, ..ACCENT }) } else { (6.0, Color::from_rgba(1.0, 1.0, 1.0, 0.28 * k)) };
            container(container(Space::new().width(side).height(side)).style(move |_| container::Style { background: Some(Background::Color(fill)), border: Border { radius: (side / 2.0).into(), ..Border::default() }, ..container::Style::default() })).center(16.0).into()
        };
        let said: Element<'a, Message> = if at == current { semi(words.t(pools::Guide::STEPS[at]), 14.5, INK) } else { faded_text(words.t(pools::Guide::STEPS[at]), 14.5, MUTED) };
        row![mark, said].spacing(10).align_y(iced::Center).into()
    };
    let caption = ui::mono_small(words.t("guide-title").to_uppercase(), MUTED);
    let count = text(words.of(done as u64, steps.len() as u64)).font(theme::MONO_BOLD).size(14.0).color(ui::faded(INK));
    let about = para(words.t(&format!("{}-about", pools::Guide::STEPS[current])), 14.0, INK);
    let hide = quiet_button(words.t("guide-hide"), Message::HideGuide);
    let plate = move |theme: &iced::Theme| {
        let slab = theme::slab(theme);
        container::Style {
            background: slab.background.map(|fill| match fill { Background::Color(colour) => Background::Color(Color { a: colour.a * k, ..colour }), other => other }),
            border: Border { color: Color { a: slab.border.color.a * k, ..slab.border.color }, width: 1.0, radius: 14.0.into() },
            ..container::Style::default()
        }
    };
    if beside {
        let list = column((0..steps.len()).map(step).collect::<Vec<_>>()).spacing(12);
        let inside = column![row![caption, ui::grow(), count].align_y(iced::Center), list, about, hide, para(words.t("guide-back"), 13.0, MUTED)].spacing(16);
        return Some(container(inside).padding(22).width(BALANCE_WIDE).style(plate).into());
    }
    let list = row((0..steps.len()).map(step).collect::<Vec<_>>()).spacing(22).wrap();
    let inside = column![row![caption, count, ui::grow(), hide].spacing(14).align_y(iced::Center), list, about].spacing(14);
    Some(container(inside).padding([18, 22]).width(Length::Fill).style(plate).into())
}

const WARNED: Color = Color::from_rgb(1.0, 0.61, 0.61);

fn check_line<'a>(check: &Check, words: &Words) -> Element<'a, Message> {
    let k = ui::fade();
    let mark: Element<'a, Message> = match check.verdict {
        Verdict::Fine => glyph(Icon::Check, 16.0, ACCENT),
        Verdict::Stop => glyph(Icon::Close, 16.0, WARNED),
        Verdict::Note => container(container(Space::new().width(6).height(6)).style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.28 * k))), border: Border { radius: 3.0.into(), ..Border::default() }, ..container::Style::default() })).center(16.0).into(),
    };
    let said: Element<'a, Message> = match check.verdict {
        Verdict::Stop => text(check.words(words)).font(theme::MONO_BOLD).size(14.0).color(ui::faded(WARNED)).into(),
        Verdict::Fine | Verdict::Note => text(check.words(words)).font(theme::MONO).size(14.0).color(ui::faded(if check.verdict == Verdict::Fine { MUTED } else { FAINT })).into(),
    };
    let mut line = row![mark, container(said).width(Length::Fill)].spacing(12).align_y(iced::Center);
    if let Some(slot) = check.slot {
        line = line.push(crate::billing::outlined(words.t("publish-show-slot"), Some(Message::ShowSlot(slot))));
    }
    container(line).center_y(34.0).into()
}

fn publish_panel<'a>(state: &'a State, editor: &'a Editor, pool: &'a Pool, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>) -> Element<'a, Message> {
    let k = ui::fade();
    let caption = |key: &str| ui::mono_small(words.t(key).to_uppercase(), MUTED);
    let named = !pool.name.trim().is_empty();
    let name = if named { pool.name.clone() } else { words.t(if pool.collection { "collection-name-hint" } else { "pool-name-hint" }) };
    let head = row![
        column![caption("publish-title"), semi(name.clone(), 21.0, if named { INK } else { MUTED })].spacing(5).width(Length::Fill),
        button(glyph(Icon::Close, 16.0, MUTED)).padding(12).style(ui::button_faded(theme::bare)).on_press(Message::PublishSheet(false)),
    ]
    .spacing(12)
    .align_y(iced::Center);
    let checks = publish_checks(pool);
    let ready = checks.iter().all(|check| check.verdict != Verdict::Stop);
    let mut list = column![caption("publish-before")].spacing(4);
    for check in &checks {
        list = list.push(check_line(check, words));
    }
    let hashes: Vec<Option<&str>> = pool.slots.iter().filter(|slot| !slot.is_empty()).take(3).map(|slot| slot.hash.as_deref()).collect();
    let mut shown = row![cover_stack(thumbs, &hashes), container(ui::moving_text(name, theme::SANS_SEMI, 16.0, if named { INK } else { MUTED })).width(Length::Fill)].spacing(14).align_y(iced::Center);
    if !pool.compiler.is_empty() {
        shown = shown.push(named_face(state, &pool.compiler, 150.0));
    }
    shown = shown.push(text(pool.filled().to_string()).font(theme::MONO_BOLD).size(15.0).color(ui::faded(INK)));
    let preview = container(shown).padding([12, 16]).width(Length::Fill).style(move |theme| {
        let slab = theme::slab(theme);
        container::Style {
            background: slab.background.map(|fill| match fill { Background::Color(colour) => Background::Color(Color { a: colour.a * k, ..colour }), other => other }),
            border: Border { color: Color { a: slab.border.color.a * k, ..slab.border.color }, width: 1.0, radius: 12.0.into() },
            ..container::Style::default()
        }
    });
    let mut facts = row![].spacing(40);
    if !pool.compiler.is_empty() {
        facts = facts.push(column![caption("pool-publisher"), named_face(state, &pool.compiler, 220.0), faded_text(words.t("publish-who"), 13.0, FAINT)].spacing(6));
    }
    let mut version = column![caption("publish-version"), container(text((pool.published_revision + 1).to_string()).font(theme::MONO_BOLD).size(16.0).color(ui::faded(INK))).center_y(FACE)].spacing(6);
    if pool.published_revision > 0 {
        version = version.push(faded_text(words.n("publish-replaces", pool.published_revision), 13.0, FAINT));
    }
    facts = facts.push(version);
    let go = ui::primary(
        words.t(if state.publishing { "catalogue-saving" } else if pool.published_revision > 0 { "catalogue-update" } else { "catalogue-publish" }),
        (ready && !state.publishing).then_some(Message::Publish),
    );
    let mut foot = row![go, crate::billing::outlined(words.t("publish-cancel"), Some(Message::PublishSheet(false))), ui::grow()].spacing(8).align_y(iced::Center);
    if pool.published_revision > 0 {
        foot = if editor.asking_withdraw {
            foot.push(primary_button(words.t("catalogue-withdraw-yes"), Message::Withdraw)).push(quiet_button(words.t("pool-delete-no"), Message::AskWithdraw(false)))
        } else {
            foot.push(button(text(words.t("catalogue-withdraw")).font(theme::SANS_SEMI).size(14.0)).padding([10, 14]).style(ui::button_faded(theme::danger_words)).on_press_maybe((!state.publishing).then_some(Message::AskWithdraw(true))))
        };
    }
    let mut inside = column![head, list, column![caption("publish-seen"), preview].spacing(10), facts].spacing(22);
    if let Some(error) = &state.catalogue_error {
        inside = inside.push(catalogue_error(error, words));
    }
    container(inside.push(foot))
        .padding(24)
        .width(Length::Fill)
        .style(move |theme| container::Style {
            background: Some(Background::Color(Color { a: theme::RAISED.a * k, ..theme::RAISED })),
            border: Border { color: Color { a: theme::LINE.a * k, ..theme::LINE }, width: 1.0, radius: theme::CARD_RADIUS.into() },
            ..theme::card(theme)
        })
        .into()
}

fn bulk_bar<'a>(editor: &'a Editor, words: &'a Words) -> Element<'a, Message> {
    let count = editor.marked.len();
    let mut buttons = row![
        semi(words.n("pool-marked", count as u64), 14.0, INK),
        quiet_button(words.t("pool-bulk-mod"), Message::Bulk(Some(Bulk::Mod))),
        quiet_button(words.t("pool-bulk-shift"), Message::Bulk(Some(Bulk::Shift))),
        quiet_button(words.t("pool-bulk-remove"), Message::RemoveMarked),
        button(text(words.t("pool-bulk-clear")).font(theme::SANS_SEMI).size(13.0).color(ui::faded(MUTED))).padding([8, 12]).style(ui::button_faded(theme::bare)).on_press(Message::Choosing(false)),
    ]
    .spacing(12)
    .align_y(iced::Center);
    let mut inside = column![].spacing(12).align_x(iced::Center);
    match editor.bulk {
        Some(Bulk::Mod) => {
            let mut pills = row![].spacing(8);
            for mods in Mod::ALL {
                pills = pills.push(button(text(mods.code()).font(theme::SANS_SEMI).size(13.5)).padding([7, 14]).style(ui::button_faded(theme::filter_chip(false))).on_press(Message::BulkMod(mods)));
            }
            inside = inside.push(pills);
        }
        Some(Bulk::Shift) => {
            let pills = row![
                button(text(words.t("pool-shift-up")).font(theme::SANS_SEMI).size(13.5)).padding([7, 14]).style(ui::button_faded(theme::filter_chip(false))).on_press(Message::Shift(false)),
                button(text(words.t("pool-shift-down")).font(theme::SANS_SEMI).size(13.5)).padding([7, 14]).style(ui::button_faded(theme::filter_chip(false))).on_press(Message::Shift(true)),
            ]
            .spacing(8);
            inside = inside.push(pills);
        }
        None => {}
    }
    buttons = buttons.align_y(iced::Center);
    inside = inside.push(buttons);
    let bar = sheet_box(inside.into());
    container(container(bar).max_width(640.0)).width(Length::Fill).height(Length::Fill).align_x(iced::alignment::Horizontal::Center).align_y(iced::alignment::Vertical::Bottom).padding(24).into()
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
        ledger = ledger.push(mono(words.n("pool-open-lost", opening.lost as u64), 12.0, ACCENT));
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
            (None, true, _) => mono(words.t("pool-gone"), 12.0, ACCENT),
            _ => Space::new().width(0.0).into(),
        };
        let inner: Element<'a, Message> = if slot.hash.is_none() {
            row![badges, cover(thumbs, None, COVER_WIDE, COVER_HIGH, COVER_ROUND), column![semi(words.t("pool-slot-empty"), 14.0, MUTED)].width(Length::Fill)].spacing(14).align_y(iced::Center).into()
        } else {
            row![
                badges,
                cover(thumbs, slot.hash.as_deref(), COVER_WIDE, COVER_HIGH, COVER_ROUND),
                column![semi(slot.title.clone(), 14.0, INK), faded_text(slot.artist.clone(), 12.5, MUTED), mono(slot.version.clone(), 11.0, FAINT)].spacing(1).width(Length::Fill),
                figure,
            ]
            .spacing(14)
            .align_y(iced::Center)
            .into()
        };
        list = list.push(container(inner).height(ROW_HIGH).align_y(iced::Center).padding([4, 10]));
    }
    let right = column![ui::mono_small(words.t("pool-open-body"), FAINT), list].spacing(10);
    let body: Element<'a, Message> = row![container(left).width(Length::Fixed(OPEN_WIDE)), container(right).width(Length::Fill)].spacing(32).align_y(iced::alignment::Vertical::Top).into();
    column![ui::appearing(ui::appear(t, 0), 10.0, || head.into()), ui::appearing(ui::appear(t, 1), 10.0, || body)].spacing(24).into()
}

fn back_button<'a>(words: &Words, collection: bool) -> Element<'a, Message> {
    button(text(words.t(if collection { "collection-back" } else { "pool-back" })).font(theme::SANS_SEMI).size(14.0))
        .padding([10, 16])
        .style(ui::button_faded(|_, status| button::Style {
            background: Some(Background::Color(Color::from_rgb8(32, 25, 26))),
            text_color: INK,
            border: Border { color: if matches!(status, button::Status::Hovered | button::Status::Pressed) { ACCENT } else { Color::from_rgb8(80, 62, 64) }, width: 1.0, radius: 12.0.into() },
            ..button::Style::default()
        }))
        .on_press(Message::Back)
        .into()
}

fn editor_toolbar<'a>(state: &'a State, editor: &'a Editor, pool: &'a Pool, words: &'a Words) -> Element<'a, Message> {
    let mut left = row![back_button(words, pool.collection)].spacing(8).align_y(iced::Center);
    if state.undo.iter().any(|(id, _)| *id == pool.id) {
        left = left.push(quiet_button(words.t("pool-undo"), Message::Undo));
    }
    if state.unsaved.contains_key(&pool.id) && !state.save_errors.contains_key(&pool.id) {
        left = left.push(text(words.t("pool-saving")).size(12).color(MUTED));
    }
    let mut buttons = row![].spacing(8).align_y(iced::Center);
    buttons = buttons.push(ui::quiet(
        words.t(if state.publishing { "catalogue-saving" } else if pool.published_revision > 0 { "catalogue-update" } else { "catalogue-publish" }),
        (!state.publishing && !editor.starting).then_some(Message::PublishSheet(true)),
    ));
    if pool.filled() > 0 {
        buttons = buttons.push(quiet_button(words.t("pool-share"), Message::Share(true)));
    }
    let danger: Element<'a, Message> = if editor.asking_delete {
        row![primary_button(words.t("pool-delete-yes"), Message::DeletePool), quiet_button(words.t("pool-delete-no"), Message::AskDelete(false))].spacing(8).into()
    } else {
        button(text(words.t(if pool.collection { "collection-delete" } else { "pool-delete" })).font(theme::SANS_SEMI).size(14.0)).padding([10, 14]).style(ui::button_faded(theme::danger_words)).on_press(Message::AskDelete(true)).into()
    };
    buttons = buttons.push(danger);
    if !editor.starting {
        buttons = buttons.push(primary_button(words.t("pool-add"), Message::AddPanel(editor.panel != Panel::Add)));
    }
    row![left, ui::grow(), buttons].spacing(12).align_y(iced::Center).into()
}

fn editor_title<'a>(pool: &'a Pool, words: &'a Words) -> Element<'a, Message> {
    text_input(&words.t(if pool.collection { "collection-name-hint" } else { "pool-name-hint" }), &pool.name)
        .on_input(Message::Rename)
        .on_paste(|contents| Message::PasteInto(Input::Name, contents))
        .font(theme::SANS_SEMI)
        .size(30.0)
        .padding([2, 0])
        .style(ui::bare_input(ui::fade()))
        .width(Length::Fill)
        .into()
}

fn credit_group<'a>(label: String, body: Element<'a, Message>) -> Element<'a, Message> {
    row![ui::mono_small(label.to_uppercase(), MUTED), body].spacing(10).align_y(iced::Center).into()
}

fn editor_credits<'a>(state: &'a State, editor: &'a Editor, pool: &'a Pool, words: &'a Words) -> Option<Element<'a, Message>> {
    let publisher = (!pool.compiler.is_empty()).then(|| credit_group(words.t("pool-publisher"), author_chip(state, &pool.compiler)));
    if pool.collection {
        return publisher;
    }
    let mut makers = row(pool.authors.iter().map(|author| author_chip(state, author)).collect::<Vec<_>>()).spacing(2).align_y(iced::Center);
    let makers: Element<'a, Message> = if editor.editing_authors {
        text_input(&words.t("pool-authors"), &editor.authors).on_input(Message::AuthorsDraft).on_submit(Message::ApplyAuthors).font(theme::SANS).size(13.0).padding([5, 0]).style(ui::bare_input(ui::fade())).width(Length::Fill).into()
    } else {
        makers = makers.push(button(glyph(Icon::Gear, 14.0, MUTED)).padding(6).style(ui::button_faded(theme::bare)).on_press(Message::EditAuthors));
        makers.wrap().into()
    };
    let mut line = row![container(credit_group(words.t("pool-mappoolers"), makers)).width(Length::Fill)].spacing(24).align_y(iced::Center);
    if let Some(publisher) = publisher {
        line = line.push(publisher);
    }
    Some(line.into())
}

fn mod_tint(mods: Mod) -> Color {
    let base = slot_colour(&Slot::empty(mods));
    Color { r: base.r + (1.0 - base.r) * 0.45, g: base.g + (1.0 - base.g) * 0.45, b: base.b + (1.0 - base.b) * 0.45, a: 1.0 }
}

fn frame_tags<'a>(frame: Frame, words: &Words) -> Element<'a, Message> {
    let k = ui::fade();
    let tag = move |label: String, ink: Color, fill: Color| -> Element<'a, Message> {
        container(text(label).font(theme::MONO_BOLD).size(11.5).color(Color { a: ink.a * k, ..ink }))
            .padding([4, 8])
            .style(move |_| container::Style { background: Some(Background::Color(Color { a: fill.a * k, ..fill })), border: Border { radius: 6.0.into(), ..Border::default() }, ..container::Style::default() })
            .into()
    };
    let mut counted: Vec<(Mod, usize)> = Vec::new();
    for mods in frame.mods() {
        match counted.last_mut() {
            Some((last, count)) if *last == mods => *count += 1,
            _ => counted.push((mods, 1)),
        }
    }
    if counted.is_empty() {
        return ui::dashed(container(text(words.t("pool-frame-own")).font(theme::MONO_BOLD).size(11.5).color(ui::faded(MUTED))).padding([4, 8]), 6.0, Color::from_rgba(1.0, 1.0, 1.0, 0.2));
    }
    row(counted.into_iter().map(|(mods, count)| {
        let base = slot_colour(&Slot::empty(mods));
        let label = if count > 1 { format!("{} {count}", mods.code()) } else { mods.code().to_owned() };
        tag(label, mod_tint(mods), Color { a: 0.22, ..base })
    }).collect::<Vec<_>>())
    .spacing(6)
    .wrap()
    .into()
}

fn start_tile(lit: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| {
        let hot = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let (fill, edge) = match (lit, hot) {
            (true, _) => (Color::from_rgba(0.886, 0.282, 0.282, 0.05), Color::from_rgba(0.886, 0.282, 0.282, 0.4)),
            (false, true) => (Color::from_rgba(1.0, 1.0, 1.0, 0.016), Color::from_rgba(1.0, 1.0, 1.0, 0.1)),
            (false, false) => (Color::from_rgba(1.0, 1.0, 1.0, 0.008), Color::from_rgba(1.0, 1.0, 1.0, 0.04)),
        };
        button::Style { background: Some(Background::Color(fill)), text_color: INK, border: Border { color: edge, width: 1.0, radius: 12.0.into() }, shadow: iced::Shadow::default(), snap: true }
    }
}

const START_WIDE: f32 = 1290.0;
const START_TALL: f32 = 640.0;
const START_GROWTH: f32 = 1.4;

fn start_scale(width: f32) -> f32 {
    (width / START_WIDE).clamp(1.0, START_GROWTH)
}

fn start_view<'a>(state: &'a State, editor: &'a Editor, pool: &'a Pool, words: &'a Words, width: f32, high: f32, t: f32) -> Element<'a, Message> {
    let grown = start_scale(width);
    let width = width / grown;
    let caption = |key: &str| ui::mono_small(words.t(key).to_uppercase(), MUTED);
    let frames = [(Frame::Duel, Icon::Swords, "pool-start-duel", false), (Frame::Stage, Icon::Trophy, "pool-start-stage", true), (Frame::Free, Icon::Shapes, "pool-frame-free", true)];
    let narrow = width < 900.0;
    let frame_high = if narrow || width >= 1110.0 { 104.0 } else { 126.0 };
    let frame_tile = |frame: Frame, icon: Icon, key: &str, offered: bool| -> Element<'a, Message> {
        let build = || -> Element<'a, Message> {
            let mut name = row![semi(words.t(key), 16.0, if offered { INK } else { MUTED })].spacing(8).align_y(iced::Center);
            if !offered {
                let k = ui::fade();
                name = name.push(container(mono(words.t("pool-frame-soon"), 11.0, MUTED)).padding([1, 7]).style(move |_| container::Style { border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.16 * k), width: 1.0, radius: 9.0.into() }, ..container::Style::default() }));
            }
            let inside = row![glyph(icon, 28.0, if offered { INK } else { FAINT }), column![name, frame_tags(frame, words)].spacing(10).width(Length::Fill)].spacing(16);
            let tile = button(container(inside).padding(16).width(Length::Fill).height(Length::Fill)).padding(0).width(Length::Fill).height(frame_high).style(ui::button_faded(start_tile(offered && pool.frame == frame)));
            if offered { tile.on_press(Message::UseFrame(frame)).into() } else { tile.into() }
        };
        if offered { build() } else { ui::fading(ui::fade() * 0.6, build) }
    };
    let ways = [
        (Start::Empty, Icon::Plus, "pool-start-empty"),
        (Start::Links, Icon::Chain, "pool-start-links"),
        (Start::Collections, Icon::Pool, "pool-start-collections"),
        (Start::Best, Icon::Medal, "pool-start-best"),
    ];
    let way_tile = |way: Start, icon: Icon, key: &str| -> Element<'a, Message> {
        let inside = column![glyph(icon, 26.0, INK), semi(words.t(key), 16.0, INK), text(words.t(&format!("{key}-about"))).font(theme::SANS).size(13.5).color(ui::faded(MUTED))].spacing(12);
        button(container(inside).padding(20).width(Length::Fill).height(Length::Fill)).padding(0).width(Length::Fill).height(156.0).style(ui::button_faded(start_tile(false))).on_press(Message::Begin(way)).into()
    };
    let lined = |tiles: Vec<Element<'a, Message>>, across: usize| -> Element<'a, Message> {
        let mut rows = column![].spacing(16);
        let mut tiles = tiles.into_iter().peekable();
        while tiles.peek().is_some() {
            let mut line = row![].spacing(16);
            for at in 0..across {
                line = match tiles.next() {
                    Some(tile) => line.push(container(tile).width(Length::FillPortion(1))),
                    None if at > 0 => line.push(Space::new().width(Length::FillPortion(1))),
                    None => line,
                };
            }
            rows = rows.push(line);
        }
        rows.into()
    };
    let frame_tiles: Vec<Element<'a, Message>> = frames.iter().map(|(frame, icon, key, offered)| frame_tile(*frame, *icon, key, *offered)).collect();
    let way_tiles: Vec<Element<'a, Message>> = ways.iter().map(|(way, icon, key)| way_tile(*way, *icon, key)).collect();
    let foot = row![faded_text(words.t("pool-start-note"), 13.0, MUTED), ui::grow(), primary_button(words.t("pool-start-go"), Message::Begin(Start::Empty))].spacing(12).align_y(iced::Center);
    let page = column![
        ui::appearing(ui::appear(t, 0), 8.0, || editor_toolbar(state, editor, pool, words)),
        ui::appearing(ui::appear(t, 1), 8.0, || {
            let k = ui::fade();
            let line = container(Space::new().height(1.0)).width(Length::Fill).style(move |_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.16 * k))), ..container::Style::default() });
            column![caption("pool-start-name"), editor_title(pool, words), line].spacing(8).into()
        }),
        ui::appearing(ui::appear(t, 2), 8.0, || column![caption("pool-start-frame"), lined(frame_tiles, if narrow { 1 } else { 3 })].spacing(12).into()),
        ui::appearing(ui::appear(t, 3), 8.0, || column![caption("pool-start-how"), lined(way_tiles, if narrow { 2 } else { 4 })].spacing(12).into()),
    ]
    .spacing(24);
    let tall = (high - 40.0) / grown;
    let foot = ui::appearing(ui::appear(t, 4), 8.0, || foot.into());
    let page: Element<'a, Message> = if !narrow && tall >= START_TALL {
        container(column![page, Space::new().height(Length::Fill), foot]).height(tall).into()
    } else {
        column![page, foot].spacing(24).into()
    };
    ui::scaled(page, grown).into()
}

fn editor_view<'a>(state: &'a State, editor: &'a Editor, pool: &'a Pool, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, width: f32, high: f32, t: f32) -> Element<'a, Message> {
    if editor.starting {
        return start_view(state, editor, pool, words, width, high, t);
    }
    let mut head = column![
        ui::appearing(ui::appear(t, 0), 8.0, || editor_toolbar(state, editor, pool, words)),
        ui::appearing(ui::appear(t, 1), 8.0, || editor_title(pool, words)),
    ]
    .spacing(14);
    if let Some(credits) = ui::fading(ui::fade() * ui::appear(t, 2), || editor_credits(state, editor, pool, words)) {
        head = head.push(ui::lifted(credits, ui::appear(t, 2), 8.0));
    }
    if state.conflict.as_deref() == Some(pool.id.as_str()) {
        head = head.push(ui::appearing(ui::appear(t, 3), 8.0, || conflict_banner(state, pool, words, width)));
    } else if let Some(error) = &state.catalogue_error {
        head = head.push(ui::appearing(ui::appear(t, 3), 8.0, || catalogue_error(error, words)));
    }

    let songs = state.songs.as_ref();
    let beside = width >= BALANCE_BESIDE;
    let guide = ui::fading(ui::fade() * ui::appear(t, 3), || guide_block(state, editor, pool, words, beside)).map(|block| ui::lifted(block, ui::appear(t, 3), 8.0));
    let balance = ui::fading(ui::fade() * ui::appear(t, 3), || balance_block(pool, words, beside)).map(|block| ui::lifted(block, ui::appear(t, 3), 8.0));
    let extras: Vec<Element<'a, Message>> = guide.into_iter().chain(balance).collect();
    let board_wide = if beside && !extras.is_empty() { width - BALANCE_WIDE - BALANCE_GAP } else { width };
    let columns = ((board_wide + 10.0) / 510.0).floor().clamp(1.0, 3.0) as usize;
    let keys = slot_keys(pool);
    let board = |slots: Vec<usize>, group: Option<Mod>| -> Element<'a, Message> {
        let pieces = slots.iter().map(|at| {
            let lit = if editor.choosing { editor.marked.contains(at) } else { editor.selected == Some(*at) };
            (keys[*at], ui::appearing(ui::appear(t, *at + 4), 6.0, || slot_row(pool, *at, lit, editor.choosing, words, thumbs, songs, state.ail(&pool.slots[*at]))))
        }).collect();
        let next = group.and_then(|_| slots.last()).and_then(|last| keys.get(last + 1)).copied();
        let snapshot = keys.clone();
        crate::board::board(pieces, 10.0, move |key, before| Message::Move(pool.id.clone(), snapshot.clone(), key, before.or(next)))
            .identity(format!("{}:{}:{:?}", pool.id, group.map_or("order", Mod::code), slots.first()))
            .across(columns)
            .anywhere()
            .grab_from(0.0)
            .grab_high(ROW_HIGH + 16.0)
            .radius(TILE_ROUND)
            .solid(theme::GROUND)
            .into()
    };
    let list: Element<'a, Message> = if pool.slots.is_empty() {
        ui::appearing(ui::appear(t, 4), 8.0, || container(faded_text(words.t("pool-free-empty"), 14.0, MUTED)).padding([24, 10]).into())
    } else if editor.grouped && !pool.collection {
        let mut sections = column![].spacing(24);
        let mut custom = std::collections::BTreeMap::<&str, Vec<usize>>::new();
        for (at, slot) in pool.slots.iter().enumerate() { custom.entry(if slot.category.is_empty() { slot.mods.code() } else { &slot.category }).or_default().push(at); }
        let order = category_order(pool);
        for (at, name) in order.iter().enumerate() {
            let Some(slots) = custom.remove(name.as_str()) else { continue };
            let mods = pool.slots[slots[0]].mods;
            let first = slots[0];
            let heading = ui::appearing(ui::appear(t, first + 4), 6.0, || {
                let up = button(glyph(Icon::Up, 13.0, MUTED)).padding(5).style(ui::button_faded(theme::bare)).on_press_maybe((at > 0).then_some(Message::MoveCategory(name.clone(), false)));
                let down = button(glyph(Icon::Down, 13.0, MUTED)).padding(5).style(ui::button_faded(theme::bare)).on_press_maybe((at + 1 < order.len()).then_some(Message::MoveCategory(name.clone(), true)));
                container(row![semi(name.clone(), 15.0, MUTED), up, down].spacing(8).align_y(iced::Center)).padding([0, 12]).into()
            });
            sections = sections.push(column![heading, board(slots, Some(mods))].spacing(10));
        }
        sections.into()
    } else {
        board((0..pool.slots.len()).collect(), None)
    };

    if extras.is_empty() {
        return column![head, list].spacing(22).into();
    }
    if beside {
        return column![head, row![container(list).width(Length::Fill), column(extras).spacing(BALANCE_GAP).width(BALANCE_WIDE)].spacing(BALANCE_GAP)].spacing(22).into();
    }
    let mut page = column![head].spacing(22);
    for block in extras {
        page = page.push(block);
    }
    page.push(list).into()
}

pub fn view<'a>(state: &'a State, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, width: f32, section_t: f32, clocks: &Clocks) -> Element<'a, Message> {
    let t = section_t.min(clocks.screen);
    let panel_t = clocks.panel.min((t - PANEL_LATE).max(0.0));
    let max_width = if matches!(state.screen, Screen::Editor(_)) { 1800.0 } else { 1180.0 };
    let content_width = (width - 48.0).min(max_width);
    let page = match &state.screen {
        Screen::Open(opening) => open_view(state, opening, words, thumbs, t),
        Screen::Shelf => shelf(state, words, thumbs, content_width, t, clocks),
        Screen::Editor(editor) => match state.list.iter().find(|pool| pool.id == editor.id) {
            Some(pool) => editor_view(state, editor, pool, words, thumbs, content_width, clocks.high, t),
            None => shelf(state, words, thumbs, content_width, t, clocks),
        },
    };
    let page: Element<'a, Message> = if !state.save_errors.is_empty() {
        column![row![text(words.t("pool-unsaved")).color(ACCENT).width(Length::Fill), ui::quiet(words.t("pool-save-retry"), Some(Message::RetrySaves))].spacing(12).align_y(iced::Center), page].spacing(12).into()
    } else { page };
    let rolled = scrollable(container(container(page).max_width(max_width)).center_x(Length::Fill).padding(Padding { top: 12.0, right: 24.0, bottom: 28.0, left: 24.0 }))
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
        if editor.choosing && !editor.marked.is_empty() {
            layers.push(ui::appearing(k, 18.0, || bulk_bar(editor, words)));
        } else if let (true, Some(pool)) = (editor.share, pool) {
            layers.push(ui::fading(ui::fade() * k, || share_modal(pool, words, state.exporting.is_some())));
        } else if let (false, Some(pool)) = (editor.choosing, pool) {
            let open = sheet_of(editor, pool).map(|sheet| (editor, sheet));
            let leaving = editor.gone.as_deref().filter(|_| open.is_none() && clocks.gone < ui::APPEAR).and_then(|was| sheet_of(was, pool).map(|sheet| (was, sheet)));
            let still = open.is_none();
            let k = if still { 1.0 - ui::appear(clocks.gone, 0) } else { k };
            if let Some((shown, sheet)) = open.or(leaving) {
                let close = match sheet {
                    Sheet::Publish => Message::PublishSheet(false),
                    Sheet::Add | Sheet::Slot(_) => Message::Select(None),
                };
                let veil = ui::fading(ui::fade() * k, || iced::widget::opaque(iced::widget::mouse_area(ui::veil(theme::SCRIM)).on_press(close)));
                let wide = PANEL_WIDE.min((width - 32.0).max(280.0));
                let sheet = ui::appearing(k, PANEL_RISE, || {
                    let side = match sheet {
                        Sheet::Slot(at) => slot_panel(pool, shown, at, wide, words, thumbs, state.songs.as_ref()),
                        Sheet::Add => add_panel(state, shown, pool, words, thumbs),
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
    Add,
    Slot(usize),
}

fn sheet_of(editor: &Editor, pool: &Pool) -> Option<Sheet> {
    if editor.publish {
        return Some(Sheet::Publish);
    }
    match (editor.panel, editor.selected.filter(|at| *at < pool.slots.len())) {
        (Panel::Add, _) => Some(Sheet::Add),
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
const PANEL_RISE: f32 = 14.0;

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dossier-pools-screen-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn map(title: &str, artist: &str, version: &str) -> Map {
        Map { file: PathBuf::from(format!("/songs/{title}.osu")), artist: artist.to_owned(), title: title.to_owned(), version: version.to_owned(), background: Some(PathBuf::from(format!("/songs/{title}.jpg"))) }
    }

    fn state_with_songs(name: &str) -> State {
        let mut state = State::new(scratch(name));
        let mut songs = HashMap::new();
        for (hash, title, artist, version) in [("a1", "Glass Orchard", "Nova Tide", "Garden"), ("b2", "Salt and Static", "Marrow", "Another"), ("c3", "Ninth Window", "Kite and Ash", "Hard"), ("d4", "Glass Harbour", "Nova Tide", "Expert")] {
            songs.insert(hash.to_owned(), map(title, artist, version));
        }
        state.songs = Some(Arc::new(songs));
        state.loaded = true;
        state
    }

    fn measure(stars: f64) -> Measure {
        Measure { stars, bpm: 180.0, length_ms: 120_000, ar: 9.0, od: 8.0, cs: 4.0, hp: 5.0, max_combo: 700, aim: 2.0, speed: 2.0, reading: 0.5, stamina: 1.0 }
    }

    fn open_new(state: &mut State) -> String {
        state.update(Message::New, 1_790_000_000);
        state.update(Message::UseFrame(Frame::Duel), 1_790_000_000);
        state.update(Message::Begin(Start::Empty), 1_790_000_000);
        state.editor_mut().unwrap().grouped = false;
        state.editing().expect("an editor").id.clone()
    }

    #[test]
    fn regression_failed_saves_keep_edits_and_retry_the_latest_version() {
        let mut state = state_with_songs("save-retry");
        let id = open_new(&mut state);
        let before = pools::load_all(&state.dir)[0].clone();
        let staging = state.dir.join(format!("{id}.pool.part"));
        std::fs::create_dir(&staging).unwrap();
        state.update(Message::Rename("Changed".into()), 101);
        assert!(!state.flush_saves(true));
        assert!(state.save_errors.contains_key(&id));
        assert_eq!(pools::load_all(&state.dir)[0].name, before.name);
        state.update(Message::Back, 102);
        assert!(matches!(state.screen, Screen::Editor(_)));
        state.update(Message::Rename("Latest".into()), 103);
        std::fs::remove_dir(&staging).unwrap();
        state.update(Message::RetrySaves, 104);
        assert!(!state.has_unsaved() && state.save_errors.is_empty());
        assert_eq!(pools::load_all(&state.dir)[0].name, "Latest");
        let _ = std::fs::remove_dir_all(&state.dir);
    }

    #[test]
    fn regression_typing_is_coalesced_and_leaving_flushes_it() {
        let mut state = state_with_songs("save-debounce");
        open_new(&mut state);
        let before = pools::load_all(&state.dir)[0].clone();
        for name in ["N", "Ne", "New name"] { state.update(Message::Rename(name.into()), 101); }
        state.update(Message::Note(0, "Latest note".into()), 102);
        state.update(Message::FlushSaves, 103);
        assert_eq!(pools::load_all(&state.dir)[0].name, before.name);
        assert!(state.has_unsaved());
        state.update(Message::Back, 104);
        let kept = pools::load_all(&state.dir).remove(0);
        assert_eq!(kept.name, "New name");
        assert_eq!(kept.slots[0].note, "Latest note");
        assert!(!state.has_unsaved());
        let _ = std::fs::remove_dir_all(&state.dir);
    }

    #[test]
    fn a_new_pool_starts_with_four_free_slots_and_grows_without_a_preset() {
        let mut state = state_with_songs("free-default");
        state.update(Message::New, 100);
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.grouped));
        assert_eq!(state.editing().unwrap().frame, Frame::Free);
        assert_eq!(state.editing().unwrap().slots.len(), 4);
        assert!(state.editing().unwrap().slots.iter().all(|slot| slot.is_empty() && slot.mods == Mod::Nm));
        state.update(Message::Put("a1".into()), 101);
        state.update(Message::Put("b2".into()), 102);
        assert_eq!(state.editing().unwrap().slots.len(), 4);
        state.update(Message::Clear(0), 103);
        assert_eq!(state.editing().unwrap().slots.len(), 3);
        assert_eq!(pools::load_all(&state.dir)[0].slots[0].hash.as_deref(), Some("b2"));
    }

    #[test]
    fn adding_a_map_uses_a_free_slot_and_keeps_its_category() {
        let mut state = state_with_songs("add-placeholder");
        state.update(Message::New, 100);
        state.update(Message::AddPanel(true), 101);
        assert_eq!(state.editing().unwrap().slots.len(), 4);
        let Screen::Editor(editor) = &state.screen else { panic!("editor") };
        assert_eq!(editor.selected, Some(0));
        assert_eq!(editor.panel, Panel::Add);
        state.update(Message::SetMod(0, Mod::Hr), 102);
        state.update(Message::AddPanel(false), 103);
        state.update(Message::AddPanel(true), 104);
        assert_eq!(state.editing().unwrap().slots.len(), 4);
        state.update(Message::Put("a1".into()), 105);
        assert_eq!(state.editing().unwrap().slots[0].mods, Mod::Hr);
        assert_eq!(state.editing().unwrap().slots[0].hash.as_deref(), Some("a1"));
        state.update(Message::AddPanel(true), 106);
        assert_eq!(state.editing().unwrap().slots.len(), 4);
        assert!(state.editing().unwrap().slots[1].is_empty());
        assert_eq!(pools::load_all(&state.dir)[0].slots, state.editing().unwrap().slots);
    }

    #[test]
    fn adding_and_pasting_keep_the_last_category_but_explicit_slots_keep_their_own() {
        let mut state = state_with_songs("remember-category");
        state.update(Message::New, 100);
        state.update(Message::AddPanel(true), 101);
        state.update(Message::SetMod(0, Mod::Hd), 102);
        state.update(Message::Put("a1".into()), 103);
        state.update(Message::AddPanel(true), 104);
        assert_eq!(state.editing().unwrap().slots[1].mods, Mod::Hd);
        state.update(Message::Put("b2".into()), 105);
        state.update(Message::Put("c3".into()), 106);
        assert_eq!(state.editing().unwrap().slots[2].mods, Mod::Hd);
        state.update(Message::Select(Some(3)), 107);
        state.update(Message::Put("d4".into()), 108);
        assert_eq!(state.editing().unwrap().slots[3].mods, Mod::Nm);
        state.update(Message::Pasted(LINK.into()), 109);
        state.update(Message::AddMod(Mod::Hr), 110);
        state.update(Message::Resolved(state.resolve_request, Ok(found(&["e5"], Some(0)))), 111);
        state.place_map("e5", map("Fifth", "Artist", "Hard"), Some(state.default_place()), 112);
        assert_eq!(state.editing().unwrap().slots[4].mods, Mod::Hr);
        state.update(Message::AddPanel(true), 113);
        assert_eq!(state.editing().unwrap().slots[5].mods, Mod::Hr);
    }

    #[test]
    fn authors_and_custom_colours_are_saved_and_follow_the_next_map() {
        let mut state = state_with_songs("custom-category-authors");
        state.update(Message::New, 100);
        state.update(Message::Authors("Alice, Bob, alice".into()), 101);
        state.update(Message::AddPanel(true), 102);
        state.update(Message::Category(0, "Aim".into()), 103);
        state.update(Message::Colour(0, "#123456".into()), 104);
        state.update(Message::Put("a1".into()), 105);
        state.update(Message::AddPanel(true), 106);
        state.update(Message::Put("b2".into()), 107);
        let pool = state.editing().unwrap();
        assert_eq!(pool.authors, ["Alice", "Bob"]);
        assert_eq!(pool.slots[1].category, "Aim");
        assert_eq!(pool.slots[1].colour, Some([0x12, 0x34, 0x56]));
        state.update(Message::Colour(1, "#abcdef".into()), 108);
        state.update(Message::Colour(1, "#xx".into()), 109);
        let saved = pools::load_all(&state.dir).remove(0);
        assert_eq!(saved.authors, ["Alice", "Bob"]);
        assert!(saved.slots[..2].iter().all(|slot| slot.colour == Some([0xab, 0xcd, 0xef])));
        let id = saved.id;
        state.update(Message::Back, 110);
        state.update(Message::Open(id), 111);
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.authors == "Alice Bob" && editor.grouped));
    }

    #[test]
    fn author_spaces_and_category_creation_preserve_named_profiles_and_definitions() {
        let mut state = state_with_songs("category-creation");
        state.update(Message::New, 100);
        state.update(Message::Authors("Alice \"Mapper One\" Bob alice".into()), 101);
        assert_eq!(state.editing().unwrap().authors, ["Alice", "Mapper One", "Bob"]);
        state.update(Message::CreateCategory(true), 102);
        state.update(Message::CategoryDraft("aim 123".into()), 103);
        state.update(Message::Colour(0, "#abcdef".into()), 104);
        assert!(state.editing().unwrap().slots[0].category.is_empty());
        state.update(Message::SaveCategory(0), 105);
        assert_eq!(state.editing().unwrap().slots[0].category, "AI");
        assert_eq!(state.editing().unwrap().slots[0].colour, Some([171, 205, 239]));
        state.update(Message::SetMod(0, Mod::Hr), 106);
        assert_eq!(state.editing().unwrap().categories["AI"], [171, 205, 239]);
        state.update(Message::Category(1, "AI".into()), 107);
        assert_eq!(state.editing().unwrap().slots[1].colour, Some([171, 205, 239]));
        let saved = pools::load_all(&state.dir).remove(0);
        assert_eq!(saved.categories["AI"], [171, 205, 239]);
        let shared = pool_share::from_file(&pool_share::to_file(&saved), 108).unwrap();
        assert_eq!(shared.categories, saved.categories);
    }

    #[test]
    fn collection_publication_keeps_its_kind_and_failed_saves_keep_the_draft() {
        let mut state = state_with_songs("collection-publication");
        state.collections = Some(vec![Collection { name: "Favorites".into(), hashes: vec!["a1".into(), "b2".into()] }]);
        state.update(Message::ImportCollection(0), 100);
        let id = state.editing().unwrap().id.clone();
        assert!(state.editing().unwrap().collection);
        assert!(state.update(Message::Publish, 101).is_empty(), "nobody is signed in yet");
        state.update(Message::OwnCompiler(id.clone(), "NaumRedlo".into()), 101);
        let actions = state.update(Message::Publish, 101);
        assert!(matches!(actions.as_slice(), [Effect::Publish(pool)] if pool.collection && pool.filled() == 2));
        assert!(state.update(Message::Publish, 102).is_empty());
        state.update(Message::Published(id.clone(), Err("503".into())), 103);
        assert!(!state.publishing);
        assert_eq!(state.editing().unwrap().filled(), 2);
        assert_eq!(state.catalogue_error.as_deref(), Some("503"));
        let shared = pool_share::from_file(&pool_share::to_file(state.editing().unwrap()), 104).unwrap();
        assert!(shared.collection);
    }

    #[test]
    fn categories_start_in_tournament_order_and_manual_order_survives_sharing() {
        let mut state = state_with_songs("tournament-order");
        state.update(Message::New, 100);
        state.list[0].slots = Mod::ALL.iter().map(|mods| Slot::empty(*mods)).collect();
        assert_eq!(category_order(&state.list[0]), ["NM", "HD", "HR", "DT", "FM", "TB"]);
        state.update(Message::MoveCategory("DT".into(), false), 101);
        assert_eq!(category_order(&state.list[0]), ["NM", "HD", "DT", "HR", "FM", "TB"]);
        let saved = pool_share::from_file(&pool_share::to_file(&state.list[0]), 102).unwrap();
        assert_eq!(category_order(&saved), category_order(&state.list[0]));
    }

    #[test]
    fn author_names_apply_only_after_confirmation() {
        let mut state = state_with_songs("confirmed-credits");
        state.update(Message::New, 100);
        state.update(Message::AuthorsDraft("Alice Bob".into()), 101);
        assert!(state.editing().unwrap().authors.is_empty());
        let effects = state.update(Message::ApplyAuthors, 102);
        assert_eq!(state.editing().unwrap().authors, ["Alice", "Bob"]);
        assert!(effects.iter().any(|effect| matches!(effect, Effect::Faces(names) if names == &["Alice", "Bob"])));
    }

    #[test]
    fn the_publisher_is_whoever_is_signed_in_and_is_asked_for_again_each_time_the_pool_is_opened() {
        let mut state = state_with_songs("pinned-publisher");
        let effects = state.update(Message::New, 100);
        let id = state.editing().unwrap().id.clone();
        assert!(effects.iter().any(|effect| matches!(effect, Effect::OwnCompiler(asked) if asked == &id)));
        let effects = state.update(Message::OwnCompiler(id.clone(), "Mapper One".into()), 101);
        assert_eq!(state.editing().unwrap().compiler, "Mapper One");
        assert!(effects.iter().any(|effect| matches!(effect, Effect::Faces(names) if names == &["Mapper One"])));
        assert!(state.update(Message::OwnCompiler(id.clone(), String::new()), 102).is_empty());
        assert_eq!(state.editing().unwrap().compiler, "Mapper One");
        assert!(state.update(Message::OwnCompiler(id.clone(), "Mapper One".into()), 103).is_empty());
        state.update(Message::Select(None), 104);
        state.update(Message::Put("a1".into()), 104);
        state.update(Message::Back, 104);
        let effects = state.update(Message::Open(id.clone()), 105);
        assert!(effects.iter().any(|effect| matches!(effect, Effect::OwnCompiler(asked) if asked == &id)));
        state.update(Message::OwnCompiler(id.clone(), "Another Account".into()), 106);
        assert_eq!(state.editing().unwrap().compiler, "Another Account");
        let shared = pool_share::from_file(&pool_share::to_file(state.editing().unwrap()), 107).unwrap();
        assert_eq!(shared.compiler, "Another Account");
    }

    #[test]
    fn a_collection_names_only_its_publisher_and_nobody_can_retype_that_name() {
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        let shown = |collection: bool| {
            let mut state = pool_with_maps(if collection { "collection-credits" } else { "pool-credits" }, &["a1", "b2"]);
            state.editor_mut().unwrap().panel = Panel::Closed;
            let at = state.editing_at().unwrap();
            state.list[at].collection = collection;
            state.list[at].authors = vec!["Alice".into()];
            state.list[at].compiler = "Builder".into();
            state.editor_mut().unwrap().editing_authors = false;
            state
        };
        let collection = shown(true);
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1200.0, 900.0), view(&collection, &words, &thumbs, 1200.0, 1.0, &Clocks::settled()));
        assert!(screen.find(words.t("pool-mappoolers").to_uppercase()).is_err());
        assert!(screen.find("Alice").is_err());
        assert!(screen.find(words.t("pool-publisher").to_uppercase()).is_ok());
        screen.click("Builder").unwrap();
        assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::Author(name)] if name == "Builder"));
        let pool = shown(false);
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1200.0, 900.0), view(&pool, &words, &thumbs, 1200.0, 1.0, &Clocks::settled()));
        assert!(screen.find(words.t("pool-mappoolers").to_uppercase()).is_ok());
        assert!(screen.find("Alice").is_ok());
        assert!(screen.find("Builder").is_ok());
        let mut signed_out = shown(true);
        let at = signed_out.editing_at().unwrap();
        signed_out.list[at].compiler.clear();
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1200.0, 900.0), view(&signed_out, &words, &thumbs, 1200.0, 1.0, &Clocks::settled()));
        assert!(screen.find(words.t("pool-publisher").to_uppercase()).is_err());
    }

    #[test]
    fn a_space_confirms_names_and_the_shelf_requests_their_avatars() {
        let mut state = state_with_songs("space-credits");
        state.update(Message::New, 100);
        let effects = state.update(Message::AuthorsDraft("Alice ".into()), 101);
        assert_eq!(state.editing().unwrap().authors, ["Alice"]);
        assert!(effects.iter().any(|effect| matches!(effect, Effect::Faces(names) if names == &["Alice"])));
        state.update(Message::EditAuthors, 102);
        state.update(Message::AuthorsDraft("Alice \"Mapper ".into()), 103);
        assert_eq!(state.editing().unwrap().authors, ["Alice"]);
        state.update(Message::AuthorsDraft("Alice \"Mapper One\" ".into()), 104);
        assert_eq!(state.editing().unwrap().authors, ["Alice", "Mapper One"]);
        let id = state.editing().unwrap().id.clone();
        state.update(Message::OwnCompiler(id, "Builder".into()), 105);
        assert_eq!(state.editing().unwrap().compiler, "Builder");
        let effects = state.open();
        assert!(effects.iter().any(|effect| matches!(effect, Effect::Faces(names) if names.contains(&"Alice".into()) && names.contains(&"Builder".into()))));
    }

    fn shown(state: &State) -> &Editor {
        match &state.screen {
            Screen::Editor(editor) => editor,
            _ => panic!("the editor is not open"),
        }
    }

    fn picture(state: &State, words: &Words, size: iced::Size, pointer: iced::Point, clocks: &Clocks, name: &str) -> ::image::RgbaImage {
        let thumbs = HashMap::new();
        let mut screen = iced_test::Simulator::with_size(crate::settings(), size, view(state, words, &thumbs, size.width, 1.0, clocks));
        screen.point_at(pointer);
        let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved { position: pointer })]);
        let shot = screen.snapshot(&crate::theme::theme()).unwrap();
        let file = crate::gallery::write_snapshot(&shot, &scratch(&format!("picture-{name}")).join(name)).unwrap();
        ::image::open(file).unwrap().to_rgba8()
    }

    #[test]
    fn tiles_behind_an_open_panel_do_not_answer_the_pointer_and_answer_it_again_once_it_is_closed() {
        let words = Words::new(crate::lang::Lang::En);
        let size = iced::Size::new(1400.0, 900.0);
        let mut state = pool_with_maps("veil-hover", &["a1", "b2", "c3"]);
        state.update(Message::Select(Some(0)), 1_790_000_100);
        let thumbs = HashMap::new();
        let mut screen = iced_test::Simulator::with_size(crate::settings(), size, view(&state, &words, &thumbs, size.width, 1.0, &Clocks::settled()));
        let tile = screen.find("Salt and Static").unwrap().visible_bounds().unwrap().center();
        assert!(tile.x < 400.0 || tile.x > 1000.0, "the tile has to lie beside the panel, it is at {tile:?}");
        screen.point_at(tile);
        let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)), iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left))]);
        assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::Select(None)]));
        let away = iced::Point::new(6.0, 6.0);
        let open = Clocks::settled();
        assert!(picture(&state, &words, size, tile, &open, "open-on-tile") == picture(&state, &words, size, away, &open, "open-away"));
        state.update(Message::Select(None), 1_790_000_101);
        assert!(picture(&state, &words, size, tile, &open, "closed-on-tile") != picture(&state, &words, size, away, &open, "closed-away"));
    }

    #[test]
    fn the_panel_stands_in_the_middle_of_the_window_and_its_four_sources_share_one_line() {
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        let mut state = state_with_songs("centred-panel");
        open_new(&mut state);
        state.update(Message::Select(Some(0)), 1_790_000_001);
        assert_eq!(shown(&state).panel, Panel::Add);
        for width in [980.0f32, 1400.0, 1920.0] {
            let size = iced::Size::new(width, 900.0);
            let mut screen = iced_test::Simulator::with_size(crate::settings(), size, view(&state, &words, &thumbs, width, 1.0, &Clocks::settled()));
            let tabs: Vec<iced::Rectangle> = ["pool-tab-search", "pool-tab-collections", "pool-tab-best", "pool-tab-suggest"].iter().map(|key| screen.find(words.t(key)).unwrap().visible_bounds().unwrap()).collect();
            assert!(tabs.iter().all(|tab| (tab.center().y - tabs[0].center().y).abs() < 0.5), "{width}: {tabs:?}");
            assert!(tabs.windows(2).all(|pair| pair[0].x + pair[0].width <= pair[1].x), "{width}: {tabs:?}");
            let grown = sheet_scale(width);
            let left = width / 2.0 - PANEL_WIDE * grown / 2.0;
            let middle = left + ((tabs[0].x + tabs[3].x + tabs[3].width) / 2.0 - left) * grown;
            assert!((middle - width / 2.0).abs() < 3.0, "{width}: the sources are centred on {middle}");
            assert!(tabs[3].x + tabs[3].width - tabs[0].x > 380.0, "{width}: {tabs:?}");
        }
    }

    #[test]
    fn a_closed_panel_stays_while_it_fades_takes_no_clicks_and_is_gone_when_the_fade_is_over() {
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        let size = iced::Size::new(1400.0, 900.0);
        let mut state = pool_with_maps("leaving-panel", &["a1", "b2"]);
        state.update(Message::Select(Some(1)), 1_790_000_100);
        assert!(shown(&state).gone.is_none());
        let buttons: Vec<iced::Point> = {
            let mut live = iced_test::Simulator::with_size(crate::settings(), size, view(&state, &words, &thumbs, size.width, 1.0, &Clocks::settled()));
            ["pool-remove", "pool-replace"].iter().map(|key| live.find(words.t(key)).unwrap().visible_bounds().unwrap().center()).collect()
        };
        state.update(Message::Select(None), 1_790_000_101);
        let gone = shown(&state).gone.as_deref().expect("what was shown is remembered");
        assert_eq!((gone.panel, gone.selected), (Panel::Slot, Some(1)));
        let at = |gone: f32| Clocks { gone, ..Clocks::settled() };
        let mut fading = iced_test::Simulator::with_size(crate::settings(), size, view(&state, &words, &thumbs, size.width, 1.0, &at(0.1)));
        assert!(fading.find(words.t("pool-remove")).is_err());
        for button in buttons {
            fading.point_at(button);
            let _ = fading.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)), iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left))]);
        }
        assert_eq!(fading.into_messages().count(), 0);
        let mut tile = iced_test::Simulator::with_size(crate::settings(), size, view(&state, &words, &thumbs, size.width, 1.0, &at(0.1)));
        tile.click("Glass Orchard").unwrap();
        assert!(matches!(tile.into_messages().collect::<Vec<_>>().as_slice(), [Message::Select(Some(0))]));
        let early = picture(&state, &words, size, iced::Point::new(6.0, 6.0), &at(0.02), "leaving-early");
        let late = picture(&state, &words, size, iced::Point::new(6.0, 6.0), &at(0.25), "leaving-late");
        let done = picture(&state, &words, size, iced::Point::new(6.0, 6.0), &at(ui::APPEAR), "leaving-done");
        let differs = |a: &::image::RgbaImage, b: &::image::RgbaImage| a.pixels().zip(b.pixels()).filter(|(x, y)| x != y).count();
        assert!(differs(&early, &done) > differs(&late, &done) && differs(&late, &done) > 0);
    }

    #[test]
    fn a_closing_panel_leaves_no_dark_slab_behind_while_it_fades_or_after() {
        let words = Words::new(crate::lang::Lang::En);
        let size = iced::Size::new(1400.0, 900.0);
        let away = iced::Point::new(6.0, 6.0);
        let at = |gone: f32| Clocks { gone, ..Clocks::settled() };
        let far = |a: &::image::RgbaImage, b: &::image::RgbaImage, by: i32| a.pixels().zip(b.pixels()).filter(|(x, y)| (0..3).any(|c| (x.0[c] as i32 - y.0[c] as i32).abs() > by)).count();
        for (name, slot) in [("slab-slot", Some(1)), ("slab-add", None)] {
            let mut state = pool_with_maps(name, &["a1", "b2"]);
            match slot {
                Some(slot) => { state.update(Message::Select(Some(slot)), 1_790_000_100); }
                None => { state.update(Message::AddPanel(true), 1_790_000_100); }
            }
            assert_eq!(state.panel_key(), "panel");
            let open = picture(&state, &words, size, away, &Clocks::settled(), &format!("{name}-open"));
            state.update(Message::Select(None), 1_790_000_101);
            let rest = picture(&state, &words, size, away, &Clocks::settled(), &format!("{name}-rest"));
            let start = picture(&state, &words, size, away, &at(0.0), &format!("{name}-start"));
            let nearly = picture(&state, &words, size, away, &at(ui::APPEAR * 0.97), &format!("{name}-nearly"));
            let over = picture(&state, &words, size, away, &at(ui::APPEAR), &format!("{name}-over"));
            assert!(far(&open, &rest, 12) > 20_000, "{name}: an open panel covers a good part of the page");
            let scale = open.width() as f32 / size.width;
            let inside = |image: &::image::RgbaImage| ::image::imageops::crop_imm(image, (420.0 * scale) as u32, (300.0 * scale) as u32, (560.0 * scale) as u32, (300.0 * scale) as u32).to_image();
            assert!(far(&inside(&start), &inside(&open), 12) < 800, "{name}: the fade starts from what was open, {} pixels moved", far(&inside(&start), &inside(&open), 12));
            assert!(far(&inside(&open), &inside(&rest), 12) > 10_000, "{name}: the panel covers that part of the page");
            assert_eq!(far(&nearly, &rest, 12), 0, "{name}: nothing of the panel is left at the end of the fade");
            assert!(over == rest, "{name}: after the fade the page is as if the panel had never been");
        }
    }

    fn shelf_with(name: &str, makers: &[&str]) -> (State, String) {
        let mut state = pool_with_maps(name, &["a1", "b2", "c3"]);
        state.update(Message::Rename("Spring duel".into()), 1_790_000_200);
        let at = state.editing_at().unwrap();
        state.list[at].authors = makers.iter().map(|name| (*name).to_owned()).collect();
        let id = state.list[at].id.clone();
        state.update(Message::Back, 1_790_000_201);
        assert!(matches!(state.screen, Screen::Shelf));
        (state, id)
    }

    #[test]
    fn the_latest_draft_is_a_wide_card_with_its_facts_and_the_others_are_tiles() {
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        let (mut state, id) = shelf_with("showcase-mine", &["Alice", "Bob"]);
        let at = state.list.iter().position(|pool| pool.id == id).unwrap();
        state.list[at].slots[0].measure = Some(measure(4.52));
        state.list[at].slots[1].measure = Some(measure(6.35));
        state.list[at].changed_at = 1_790_000_000;
        let mut other = Pool::new(Frame::Free, "Second pool", 1_780_000_000);
        other.slots = vec![Slot::empty(Mod::Nm)];
        other.slots[0].hash = Some("a".repeat(32));
        let other_id = other.id.clone();
        state.list.push(other);
        let clocks = Clocks { today: 1_790_000_000, ..Clocks::settled() };
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1400.0, 900.0), view(&state, &words, &thumbs, 1400.0, 1.0, &clocks));
        for label in ["shelf-mine", "shelf-saved", "catalogue-published", "catalogue-collections", "shelf-open", "shelf-changed"] {
            assert!(screen.find(words.t(label)).is_ok(), "{label}");
        }
        assert!(screen.find(words.t("shelf-latest").to_uppercase()).is_ok());
        assert!(screen.find("Spring duel").is_ok() && screen.find("Second pool").is_ok());
        assert!(screen.find("Alice, Bob").is_ok());
        assert!(screen.find("4.52").is_ok() && screen.find("6.35").is_ok());
        assert!(screen.find(words.t("shelf-no-makers")).is_ok(), "the tile says that nobody is listed");
        screen.click("Spring duel").unwrap();
        assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::Open(open)] if *open == id));
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1400.0, 900.0), view(&state, &words, &thumbs, 1400.0, 1.0, &clocks));
        screen.click("Second pool").unwrap();
        assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::Open(open)] if *open == other_id));
    }

    #[test]
    fn the_shelf_asks_for_the_covers_of_what_its_tab_shows() {
        let (mut state, _) = shelf_with("showcase-covers", &[]);
        state.songs = Some(Arc::new(HashMap::new()));
        let own: Vec<String> = state.list[0].slots.iter().filter_map(|slot| slot.hash.clone()).collect();
        assert!(!own.is_empty());
        assert_eq!(state.covers().into_iter().map(|(hash, _)| hash).collect::<Vec<_>>(), own);
        state.shelf = Shelf::Saved;
        assert!(state.covers().is_empty(), "a draft is not a saved pool");
        let theirs = pool_share::to_file(&state.list[0]);
        state.publications.push(crate::bot::Publication { code: "XQIS43GB".into(), id: "theirs".into(), kind: "pool".into(), local_id: "x".into(), revision: 3, name: "Theirs".into(), content: serde_json::from_slice(&theirs).unwrap(), mine: false });
        state.shelf = Shelf::Published;
        assert_eq!(state.covers().into_iter().map(|(hash, _)| hash).collect::<Vec<_>>(), own);
    }

    #[test]
    fn a_published_pool_shows_its_code_and_is_saved_once_into_its_own_tab() {
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        let (mut state, id) = shelf_with("showcase-published", &["Alice"]);
        let mut pool = state.list.iter().find(|pool| pool.id == id).unwrap().clone();
        pool.compiler = "Builder".into();
        let content: serde_json::Value = serde_json::from_slice(&pool_share::to_file(&pool)).unwrap();
        state.publications.push(crate::bot::Publication { code: "XQIS43GB".into(), id: "theirs".into(), kind: "pool".into(), local_id: "x".into(), revision: 3, name: "Someone's pool".into(), content: content.clone(), mine: false });
        state.publications.push(crate::bot::Publication { code: "K7M2PD4T".into(), id: "ours".into(), kind: "pool".into(), local_id: id.clone(), revision: 2, name: "My own pool".into(), content, mine: true });
        state.update(Message::Shelf(Shelf::Published), 1_790_000_000);
        state.publications.truncate(2);
        assert_eq!(state.shelf, Shelf::Published);
        {
        let mut screen = look(&state, &words, &thumbs);
        for said in ["Someone's pool", "My own pool", "XQIS43GB", "K7M2PD4T", "#3", "#2", "Builder"] {
            assert!(screen.find(said).is_ok(), "{said}");
        }
        assert!(screen.find(words.t("shelf-published-by").to_uppercase()).is_ok());
        screen.click(words.t("ledger-edit")).unwrap();
        assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::OpenPublication(open)] if open == "ours"));
        let mut screen = look(&state, &words, &thumbs);
        screen.click(words.t("shelf-save")).unwrap();
        assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::SavePublication(save)] if save == "theirs"));
        }
        let before = state.list.len();
        state.update(Message::SavePublication("theirs".into()), 1_790_000_001);
        state.update(Message::SavePublication("theirs".into()), 1_790_000_002);
        state.update(Message::SavePublication("ours".into()), 1_790_000_003);
        assert_eq!(state.list.len(), before + 1, "saved once, and one's own publication is not saved as a copy");
        let saved = state.list.iter().find(|pool| pool.saved.is_some()).unwrap().clone();
        assert_eq!(saved.saved, Some(pools::Saved { id: "theirs".into(), code: "XQIS43GB".into(), revision: 3, publisher: "Builder".into() }));
        assert_eq!(saved.published_revision, 0);
        assert_eq!(pools::load_all(&state.dir).iter().filter(|pool| pool.saved.is_some()).count(), 1, "the mark survives on disk");
        {
            let mut screen = look(&state, &words, &thumbs);
            assert!(screen.find(words.t("shelf-saved-mark")).is_ok() && screen.find(words.t("shelf-save")).is_err());
        }
        state.update(Message::Shelf(Shelf::Saved), 1_790_000_004);
        state.songs = None;
        {
            let mut screen = look(&state, &words, &thumbs);
            assert!(screen.find("XQIS43GB").is_ok() && screen.find("#3").is_ok());
            assert!(screen.find(words.t("shelf-checking")).is_ok(), "before the game's maps are read nothing is claimed about them");
        }
        state.songs = Some(Arc::new(HashMap::new()));
        {
            let mut screen = look(&state, &words, &thumbs);
            let wanted = match saved_count(&state) { 0 => words.t("shelf-all-here"), missing => words.n("shelf-missing", missing) };
            assert!(screen.find(wanted).is_ok());
        }
        state.update(Message::Shelf(Shelf::Mine), 1_790_000_005);
        let mut screen = look(&state, &words, &thumbs);
        assert!(screen.find("XQIS43GB").is_err(), "a saved pool is not among the drafts");
    }

    fn look<'a>(state: &'a State, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>) -> iced_test::Simulator<'a, Message> {
        iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1400.0, 900.0), view(state, words, thumbs, 1400.0, 1.0, &Clocks::settled()))
    }

    fn saved_count(state: &State) -> u64 {
        state.list.iter().find(|pool| pool.saved.is_some()).map_or(0, |pool| pool.slots.iter().filter(|slot| slot.hash.is_some()).count() as u64)
    }

    #[test]
    fn a_narrow_window_keeps_the_tiles_inside_and_an_empty_tab_says_what_to_do() {
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        let (mut state, _) = shelf_with("showcase-narrow", &["Alice"]);
        let mut other = Pool::new(Frame::Free, "Second pool", 1_780_000_000);
        other.slots = vec![Slot::empty(Mod::Nm)];
        state.list.push(other);
        for width in [560.0_f32, 760.0, 1400.0] {
            let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(width, 1400.0), view(&state, &words, &thumbs, width, 1.0, &Clocks::settled()));
            for name in ["Spring duel", "Second pool"] {
                let found = screen.find(name).unwrap().visible_bounds().unwrap();
                assert!(found.x >= 0.0 && found.x + found.width <= width, "{name} at {width}: {found:?}");
            }
        }
        state.shelf = Shelf::Saved;
        {
            let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(980.0, 900.0), view(&state, &words, &thumbs, 980.0, 1.0, &Clocks::settled()));
            assert!(screen.find(words.t("shelf-empty-saved")).is_ok());
        }
        state.list.clear();
        state.shelf = Shelf::Mine;
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(980.0, 900.0), view(&state, &words, &thumbs, 980.0, 1.0, &Clocks::settled()));
        assert!(screen.find(words.t("shelf-empty-mine")).is_ok());
    }

    #[test]
    fn closing_starts_its_own_clock_and_opening_again_forgets_it() {
        let mut state = pool_with_maps("leaving-clock", &["a1", "b2"]);
        let mut marks = Marks::default();
        let start = Instant::now();
        marks.observe(&state, start);
        state.update(Message::Select(Some(0)), 1_790_000_100);
        marks.observe(&state, start + Duration::from_secs(10));
        assert_eq!(marks.clocks(start + Duration::from_secs(11)).gone, SETTLED);
        state.update(Message::Select(None), 1_790_000_101);
        marks.observe(&state, start + Duration::from_secs(20));
        let soon = start + Duration::from_secs(20) + Duration::from_millis(100);
        assert!((marks.clocks(soon).gone - 0.1).abs() < 0.001);
        assert!(marks.animating(soon));
        assert!(!marks.animating(start + Duration::from_secs(30)));
        state.update(Message::Select(Some(1)), 1_790_000_102);
        marks.observe(&state, start + Duration::from_secs(20) + Duration::from_millis(150));
        assert_eq!(marks.clocks(start + Duration::from_secs(20) + Duration::from_millis(200)).gone, SETTLED);
    }

    #[test]
    fn category_numbers_follow_each_category_after_changes_and_reordering() {
        let mut state = state_with_songs("category-numbers");
        state.update(Message::New, 100);
        state.update(Message::SetMod(1, Mod::Hd), 101);
        state.update(Message::SetMod(3, Mod::Hd), 102);
        let numbers = |pool: &Pool| (0..pool.slots.len()).map(|at| category_number(pool, at)).collect::<Vec<_>>();
        assert_eq!(numbers(state.editing().unwrap()), vec![1, 1, 2, 2]);
        state.list[0].slots.swap(1, 2);
        assert_eq!(numbers(state.editing().unwrap()), vec![1, 2, 1, 2]);
        state.update(Message::SetMod(2, Mod::Hr), 103);
        assert_eq!(numbers(state.editing().unwrap()), vec![1, 2, 1, 1]);
        state.update(Message::Category(0, "Tech".into()), 104);
        state.update(Message::Category(2, "Tech".into()), 105);
        assert_eq!(numbers(state.editing().unwrap()), vec![1, 1, 2, 1]);
    }

    #[test]
    fn long_candidate_names_scroll_and_leave_the_star_rating_visible() {
        let mut pool = Pool::new(Frame::Free, "", 100);
        pool.slots.push(Slot::empty(Mod::Hd));
        let editor = Editor::at(pool.id.clone());
        let mut found = found(&["a1", "b2"], Some(0));
        found.title = "THE BADDEST feat. (G)I-DLE, Bea Miller, Wolftyla".into();
        found.difficulties[0].version = "All that you've left behind, has become my everything".into();
        let candidate = Candidate { found, choice: Some(0), place: Place::Slot(0), cover: None };
        let thumbs = HashMap::new();
        let words = Words::new(crate::lang::Lang::Ru);
        for width in [340.0, 400.0] {
            let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(width, 800.0), candidate_card(&candidate, &editor, &pool, &words, &thumbs));
            let title = screen.find(candidate.found.title.clone()).unwrap().visible_bounds().unwrap();
            let difficulty = screen.find(candidate.found.difficulties[0].version.clone()).unwrap().visible_bounds().unwrap();
            let stars = screen.find(stars_of(&words, 3.0)).unwrap().visible_bounds().unwrap();
            assert!(title.height <= 25.0 && title.x + title.width <= width);
            assert!(difficulty.height <= 25.0 && difficulty.x + difficulty.width <= stars.x);
            assert!(stars.x + stars.width <= width);
            if let Some(dir) = std::env::var_os("DOSSIER_POOL_MENU_REVIEW").map(PathBuf::from) {
                crate::gallery::write_snapshot(&screen.snapshot(&theme::theme()).unwrap(), &dir.join(format!("long-map-{width}"))).unwrap();
            }
        }
    }

    #[test]
    fn clicking_a_free_slot_retargets_the_visible_candidate_and_keeps_its_category() {
        let mut state = state_with_songs("free-slot-candidate");
        state.update(Message::New, 100);
        state.update(Message::Begin(Start::Empty), 100);
        state.finding = Some(Finding::Found(Candidate { found: found(&["a1", "b2"], Some(0)), choice: Some(0), place: Place::Slot(0), cover: None }));
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1000.0, 1000.0), view(&state, &words, &thumbs, 1000.0, 1.0, &Clocks::settled()));
        let number = screen.find("4").unwrap().visible_bounds().unwrap();
        screen.point_at(number.center() + iced::Vector::new(320.0, 0.0));
        let _ = screen.simulate(iced_test::simulator::click());
        let messages: Vec<_> = screen.into_messages().collect();
        assert!(matches!(messages.as_slice(), [Message::Select(Some(3))]), "{messages:?}");
        for message in messages { state.update(message, 101); }
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.panel == Panel::Add && editor.selected == Some(3)));
        assert!(matches!(&state.finding, Some(Finding::Found(candidate)) if candidate.place == Place::Slot(3)));
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1500.0, 1000.0), view(&state, &words, &thumbs, 1500.0, 1.0, &Clocks::settled()));
        assert!(screen.find(words.t("pool-link-put-all")).is_err());
        assert!(screen.find(words.t("pool-place-end")).is_err());
        drop(screen);
        state.update(Message::SetMod(3, Mod::Hr), 102);
        state.update(Message::Confirm, 103);
        assert!(state.editing().unwrap().slots[..3].iter().all(Slot::is_empty));
        assert_eq!(state.editing().unwrap().slots[3].hash.as_deref(), Some("a1"));
        assert_eq!(state.editing().unwrap().slots[3].mods, Mod::Hr);
        state.update(Message::Undo, 104);
        assert!(state.editing().unwrap().slots.iter().all(Slot::is_empty));
        assert_eq!(pools::load_all(&state.dir)[0].slots[3].mods, Mod::Hr);
    }

    #[test]
    fn covers_include_imported_candidates_and_maps_without_local_art() {
        let mut state = state_with_songs("candidate-covers");
        state.update(Message::New, 100);
        state.finding = Some(Finding::Found(Candidate { found: found(&["a1", "b2"], None), choice: None, place: Place::End, cover: None }));
        Arc::make_mut(state.songs.as_mut().unwrap()).get_mut("b2").unwrap().background = None;
        let covers = state.covers();
        assert!(covers.contains(&("a1".into(), Some(PathBuf::from("/songs/Glass Orchard.jpg")))));
        assert!(covers.contains(&("b2".into(), None)));
    }

    #[test]
    fn cached_measures_return_after_bulk_category_changes_without_a_new_answer() {
        let mut state = pool_with_maps("bulk-cache", &["a1", "b2"]);
        for hash in ["a1", "b2"] {
            state.update(Message::Measured(hash.into(), Mod::Nm, Ok(measure(4.5))), 101);
            state.update(Message::Measured(hash.into(), Mod::Dt, Ok(measure(5.6))), 102);
        }
        state.update(Message::Choosing(true), 103);
        state.update(Message::Mark(0), 104);
        state.update(Message::Mark(1), 105);
        for (mods, stars) in [(Mod::Dt, 5.6), (Mod::Nm, 4.5), (Mod::Dt, 5.6)] {
            assert!(state.update(Message::BulkMod(mods), 106).is_empty());
            assert!(state.editing().unwrap().slots[..2].iter().all(|slot| slot.measure.unwrap().stars == stars));
        }
        state.update(Message::Undo, 107);
        assert!(state.editing().unwrap().slots[..2].iter().all(|slot| slot.mods == Mod::Nm && slot.measure.unwrap().stars == 4.5));
    }

    #[test]
    fn out_of_order_search_and_cancelled_downloads_cannot_replace_current_work() {
        let mut state = state_with_songs("network-stale");
        state.update(Message::New, 100);
        state.update(Message::Query("https://osu.ppy.sh/beatmaps/1".into()), 101);
        let old = state.resolve_request;
        state.update(Message::Query("https://osu.ppy.sh/beatmaps/2".into()), 102);
        state.update(Message::Resolved(old, Ok(found(&["wrong"], Some(0)))), 103);
        assert!(matches!(state.finding, Some(Finding::Asking)));
        state.update(Message::Resolved(state.resolve_request, Ok(found(&["zz9"], Some(0)))), 104);
        state.update(Message::Confirm, 105);
        let first = state.fetch_request;
        let stop = match &state.finding { Some(Finding::Fetching(fetching)) => fetching.stop.clone(), _ => panic!("download") };
        state.update(Message::Back, 106);
        assert!(stop.load(Ordering::SeqCst));
        state.update(Message::New, 107);
        state.update(Message::Query(LINK.into()), 108);
        state.update(Message::Resolved(state.resolve_request, Ok(found(&["zz9"], Some(0)))), 109);
        state.update(Message::Confirm, 110);
        let current = state.fetch_request;
        assert_ne!(first, current);
        state.update(Message::Step(first, "zz9".into(), crate::maps::Step::Done(map("Old", "A", "B"))), 111);
        assert_eq!(state.editing().unwrap().filled(), 0);
        assert!(matches!(state.finding, Some(Finding::Fetching(_))));
        state.update(Message::Step(current, "wrong".into(), crate::maps::Step::Nowhere), 112);
        assert!(matches!(state.finding, Some(Finding::Fetching(_))));
        state.update(Message::Step(current, "zz9".into(), crate::maps::Step::Done(map("Current", "A", "B"))), 113);
        assert_eq!(state.editing().unwrap().slots[0].title, "Current");
    }

    #[test]
    fn pasting_links_over_focused_fields_keeps_their_text_and_uses_the_raw_link() {
        let mut state = state_with_songs("paste-fields");
        state.update(Message::New, 100);
        state.update(Message::Rename("My pool".into()), 101);
        state.update(Message::Put("a1".into()), 102);
        let id = state.editing().unwrap().id.clone();
        for input in [Input::Name, Input::Query, Input::Note(0)] {
            state.update(Message::PastedInto(id.clone(), input, format!("Existing {LINK}"), LINK.into()), 103);
            assert_eq!(state.editing().unwrap().name, "My pool");
            assert!(state.editing().unwrap().slots[0].note.is_empty());
            assert!(matches!(state.finding, Some(Finding::Asking)));
            assert!(matches!(&state.screen, Screen::Editor(editor) if editor.query == LINK));
        }
        state.update(Message::PastedInto(id.clone(), Input::Name, "My pool updated".into(), " updated".into()), 104);
        assert_eq!(state.editing().unwrap().name, "My pool updated");
        state.update(Message::New, 105);
        state.update(Message::PastedInto(id, Input::Name, "Wrong pool".into(), LINK.into()), 106);
        assert!(state.editing().unwrap().name.is_empty());
        assert!(state.finding.is_none());
    }

    #[test]
    fn a_dropped_archive_offers_a_choice_and_waits_before_importing_the_next_file() {
        let mut state = state_with_songs("archive-choice");
        state.update(Message::New, 100);
        let effects = state.update(Message::Dropped("set.OSZ".into()), 101);
        let request = match effects.as_slice() { [Effect::ImportArchive(request, _)] => *request, _ => panic!("archive import") };
        state.update(Message::Dropped("next.osu".into()), 102);
        state.update(Message::ArchiveFile(request, Ok(vec![("e5".into(), map("Set", "Artist", "Easy")), ("f6".into(), map("Set", "Artist", "Hard"))])), 103);
        assert!(matches!(&state.finding, Some(Finding::Found(candidate)) if candidate.choice.is_none() && candidate.found.difficulties.len() == 2));
        assert!(state.importing.is_none());
        assert_eq!(state.dropped.len(), 1);
        state.update(Message::Measured("f6".into(), Mod::Nm, Ok(measure(5.6))), 104);
        assert!(matches!(&state.finding, Some(Finding::Found(candidate)) if candidate.found.difficulties[1].stars == 5.6));
        state.update(Message::Choose(1), 105);
        let next = state.update(Message::Confirm, 106);
        assert_eq!(state.editing().unwrap().filled(), 1);
        assert_eq!(state.editing().unwrap().slots[0].hash.as_deref(), Some("f6"));
        assert_eq!(state.editing().unwrap().slots[0].measure.unwrap().stars, 5.6);
        assert!(next.iter().any(|effect| matches!(effect, Effect::ImportMap(_, _))));
        state.update(Message::Back, 107);
        state.update(Message::New, 108);
        state.update(Message::ArchiveFile(request, Ok(vec![("old".into(), map("Old", "Artist", "Easy"))])), 109);
        assert!(state.finding.is_none());
        assert_eq!(state.editing().unwrap().filled(), 0);
    }

    #[test]
    fn dropped_maps_are_queued_keep_the_slot_mods_and_survive_a_later_songs_read() {
        let mut state = state_with_songs("dropped");
        let id = open_new(&mut state);
        state.update(Message::Select(Some(3)), 1_790_000_001);
        let first = state.update(Message::Dropped(PathBuf::from("first.osu")), 1_790_000_002);
        let request = match first.as_slice() {
            [Effect::ImportMap(request, path)] if path == &PathBuf::from("first.osu") => *request,
            _ => panic!("a chart is imported in the background"),
        };
        assert!(state.update(Message::Dropped(PathBuf::from("second.OSU")), 1_790_000_003).is_empty());
        let next = state.update(Message::MapFile(request, Ok(("e5".into(), map("First", "A", "Hard")))), 1_790_000_004);
        assert_eq!(state.editing().unwrap().slots[3].hash.as_deref(), Some("e5"));
        assert_eq!(state.editing().unwrap().slots[3].mods, Mod::Hr);
        assert!(next.iter().any(|effect| matches!(effect, Effect::Measure(hash, _, Mod::Hr) if hash == "e5")));
        let request = next.iter().find_map(|effect| match effect { Effect::ImportMap(request, _) => Some(*request), _ => None }).unwrap();
        state.update(Message::MapFile(request, Ok(("f6".into(), map("Second", "A", "Hard")))), 1_790_000_005);
        assert_eq!(state.editing().unwrap().slots[0].hash.as_deref(), Some("f6"));
        state.update(Message::Songs(Arc::new(HashMap::new())), 1_790_000_006);
        assert!(state.songs.as_ref().unwrap().contains_key("e5"));
        assert!(state.songs.as_ref().unwrap().contains_key("f6"));
        let kept = pools::load_all(&state.dir);
        assert_eq!(kept.iter().find(|pool| pool.id == id).unwrap().slots[3].hash.as_deref(), Some("e5"));
        assert!(state.importing.is_none() && state.dropped.is_empty());
    }

    #[test]
    fn an_import_answer_cannot_write_to_a_different_or_changed_pool() {
        let mut state = state_with_songs("dropped-stale");
        open_new(&mut state);
        state.update(Message::Dropped(PathBuf::from("first.osu")), 1_790_000_001);
        let request = state.importing.as_ref().unwrap().request;
        state.update(Message::New, 1_790_000_002);
        state.update(Message::UseFrame(Frame::Duel), 1_790_000_002);
        state.update(Message::MapFile(request, Ok(("e5".into(), map("First", "A", "Hard")))), 1_790_000_003);
        assert_eq!(state.editing().unwrap().filled(), 0);
        state.update(Message::Dropped(PathBuf::from("first.osu")), 1_790_000_004);
        let request = state.importing.as_ref().unwrap().request;
        state.update(Message::SetMod(0, Mod::Dt), 1_790_000_005);
        state.update(Message::MapFile(request, Ok(("e5".into(), map("First", "A", "Hard")))), 1_790_000_006);
        assert_eq!(state.editing().unwrap().filled(), 0);
        assert!(state.importing.is_none());
        assert!(matches!(state.update(Message::Dropped(PathBuf::from("pool.POOL")), 1_790_000_007).as_slice(), [Effect::ReadPool(_, _)]));
        let request = state.importing_pool.unwrap();
        state.update(Message::PoolFile(request, Err(pool_share::Refused::NotAPool)), 1_790_000_008);
        assert!(matches!(state.screen, Screen::Shelf));
        assert_eq!(state.refused, Some(pool_share::Refused::NotAPool));
        state.update(Message::Dropped(PathBuf::from("pool.POOL")), 1_790_000_009);
        let request = state.importing_pool.unwrap();
        state.update(Message::New, 1_790_000_010);
        state.update(Message::PoolFile(request, Ok(Pool::new(Frame::Duel, "Late", 1_790_000_011))), 1_790_000_011);
        assert!(matches!(state.screen, Screen::Editor(_)));
        assert_ne!(state.editing().unwrap().name, "Late");
    }

    #[test]
    fn a_bad_dropped_map_reports_the_reason_and_continues_with_the_next_file() {
        let mut state = state_with_songs("dropped-error");
        state.screen = Screen::Shelf;
        state.update(Message::Dropped(PathBuf::from("bad.osu")), 1_790_000_001);
        let request = state.importing.as_ref().unwrap().request;
        state.update(Message::Dropped(PathBuf::from("good.osu")), 1_790_000_002);
        let next = state.update(Message::MapFile(request, Err(crate::pool_files::Refused::Mode)), 1_790_000_003);
        assert!(matches!(next.as_slice(), [Effect::Say("pool-file-mode"), Effect::ImportMap(_, _)]));
        assert_eq!(state.editing().unwrap().filled(), 0);
    }

    #[test]
    fn best_plays_keep_one_play_per_map_and_choose_local_or_remote_maps() {
        let mut state = state_with_songs("best");
        let hash = "0123456789abcdef0123456789abcdef";
        Arc::make_mut(state.songs.as_mut().unwrap()).insert(hash.into(), map("A", "B", "C"));
        let score = |id, pp, hash: &str| crate::community::wire::Score { beatmap_id: id, pp, hash: hash.into(), ..Default::default() };
        state.set_best(Some(&[
            score(1.0, 200.0, &hash.to_ascii_uppercase()), score(1.0, 100.0, hash), score(0.0, 80.0, hash), score(2.0, 150.0, ""), score(0.0, 500.0, ""), score(3.5, 500.0, ""), score(4.0, f64::NAN, ""),
        ]));
        assert_eq!(state.best.as_ref().unwrap().len(), 2);
        open_new(&mut state);
        assert!(state.update(Message::Source(SourceTab::Best), 1_790_000_001).is_empty());
        state.update(Message::Best(0), 1_790_000_002);
        assert_eq!(state.editing().unwrap().slots[0].hash.as_deref(), Some(hash));
        state.update(Message::Best(0), 1_790_000_003);
        assert_eq!(state.notice, Some(Notice::Already(0)));
        let effects = state.update(Message::Best(1), 1_790_000_004);
        assert!(matches!(effects.as_slice(), [Effect::Resolve(_, Target::Beatmap { id: 2, .. })]));
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.query == "https://osu.ppy.sh/beatmaps/2"));
        state.update(Message::Resolved(state.resolve_request, Err(Why::Silent("offline".into()))), 1_790_000_005);
        assert!(matches!(state.update(Message::Retry, 1_790_000_006).as_slice(), [Effect::Resolve(_, Target::Beatmap { id: 2, .. })]));
        let downloaded = "fedcba9876543210fedcba9876543210";
        state.update(Message::Resolved(state.resolve_request, Ok(pool_links::Found {
            set: 10, artist: "B".into(), title: "A".into(), picked: Some(0),
            difficulties: vec![pool_links::Difficulty { id: 2, hash: downloaded.into(), version: "C".into(), stars: 5.0 }],
        })), 1_790_000_007);
        assert_eq!(state.best.as_ref().unwrap()[1].hash, downloaded);
        assert!(matches!(state.update(Message::Confirm, 1_790_000_008).as_slice(), [Effect::Fetch(_, hash, _)] if hash == downloaded));
        state.update(Message::Step(state.fetch_request, downloaded.into(), crate::maps::Step::Done(map("A", "B", "C"))), 1_790_000_009);
        assert_eq!(state.editing().unwrap().slots[1].hash.as_deref(), Some(downloaded));
        assert!(state.update(Message::Best(1), 1_790_000_010).is_empty());
        assert_eq!(state.notice, Some(Notice::Already(1)));
        state.set_best(None);
        assert!(matches!(state.update(Message::Source(SourceTab::Best), 1_790_000_011).as_slice(), [Effect::ReadBest(false)]));
        assert!(matches!(state.update(Message::RefreshBest, 1_790_000_012).as_slice(), [Effect::ReadBest(true)]));
    }

    #[test]
    fn collections_add_local_maps_and_resolve_missing_hashes() {
        let mut state = state_with_songs("collections");
        open_new(&mut state);
        assert!(matches!(state.update(Message::AddPanel(true), 1_790_000_001).as_slice(), []));
        assert!(matches!(state.update(Message::Source(SourceTab::Collections), 1_790_000_002).as_slice(), [Effect::ReadCollections]));
        let missing = "0123456789abcdef0123456789abcdef".to_owned();
        state.update(Message::Collections(vec![Collection { name: "Favorites".into(), hashes: vec!["a1".into(), missing.clone()] }]), 1_790_000_003);
        state.update(Message::Collection(Some(0)), 1_790_000_004);
        state.update(Message::CollectionHash("a1".into()), 1_790_000_005);
        assert_eq!(state.editing().unwrap().slots[0].hash.as_deref(), Some("a1"));
        let effects = state.update(Message::CollectionHash(missing.clone()), 1_790_000_006);
        assert!(matches!(effects.as_slice(), [Effect::Resolve(_, Target::Hash(hash))] if hash == &missing));
        assert!(matches!(state.finding, Some(Finding::Asking)));
    }

    #[test]
    fn suggestions_match_neighbours_ignore_stale_answers_and_fill_the_empty_slot() {
        let mut state = state_with_songs("suggestions");
        open_new(&mut state);
        state.update(Message::Put("a1".into()), 1_790_000_001);
        state.update(Message::Measured("a1".into(), Mod::Nm, Ok(measure(5.0))), 1_790_000_002);
        let effects = state.update(Message::Suggest(1), 1_790_000_003);
        let request = match effects.as_slice() {
            [Effect::Suggest(request, _, Mod::Nm, target, excluded, _)] if *target == 5.0 && excluded.contains("a1") => *request,
            _ => panic!("the empty slot needs a background suggestion scan"),
        };
        state.update(Message::Suggested(request + 1, vec![("b2".into(), measure(5.1))]), 1_790_000_004);
        assert!(state.suggestions.as_ref().unwrap().maps.is_none());
        state.update(Message::Suggested(request, vec![("b2".into(), measure(5.1))]), 1_790_000_005);
        assert_eq!(state.suggestions.as_ref().unwrap().maps.as_ref().unwrap().len(), 1);
        state.update(Message::Put("b2".into()), 1_790_000_006);
        assert_eq!(state.editing().unwrap().slots[1].hash.as_deref(), Some("b2"));
        assert_eq!(state.editing().unwrap().slots[1].measure.unwrap().stars, 5.1);
        assert!(state.suggestions.is_none());
        assert!(matches!(state.screen, Screen::Editor(Editor { panel: Panel::Slot, .. })));
    }

    #[test]
    fn suggestion_target_uses_the_nearest_maps_on_both_sides() {
        let mut pool = Pool::new(Frame::Duel, "Test", 1);
        pool.slots[0].measure = Some(measure(4.0));
        pool.slots[2].measure = Some(measure(6.0));
        pool.slots[5].measure = Some(measure(8.0));
        assert_eq!(suggestion_target(&pool, 1), Some(5.0));
        assert_eq!(suggestion_target(&pool, 3), Some(7.0));
        assert_eq!(suggestion_target(&pool, 6), Some(8.0));
        assert_eq!(suggestion_target(&Pool::new(Frame::Duel, "Empty", 1), 1), None);
    }

    #[test]
    fn a_new_pool_opens_in_the_editor_and_an_untouched_one_is_removed_on_leaving() {
        let mut state = state_with_songs("untouched");
        let id = open_new(&mut state);
        assert_eq!(state.list.len(), 1);
        assert_eq!(state.editing().unwrap().slots.len(), 7);
        assert_eq!(pools::load_all(&state.dir).len(), 1, "it is on the disk at once");
        state.update(Message::Back, 1_790_000_001);
        assert!(state.list.is_empty());
        assert!(pools::load_all(&state.dir).is_empty());
        assert_eq!(state.screen, Screen::Shelf);
        assert!(!state.dir.join(format!("{id}.pool")).exists());
    }

    #[test]
    fn a_named_pool_stays_when_it_has_no_maps_yet() {
        let mut state = state_with_songs("named");
        open_new(&mut state);
        state.update(Message::Rename("Spring".into()), 1_790_000_002);
        state.update(Message::Back, 1_790_000_003);
        assert_eq!(state.list.len(), 1);
        assert_eq!(pools::load_all(&state.dir)[0].name, "Spring");
    }

    #[test]
    fn the_frame_can_be_changed_only_while_the_pool_is_empty() {
        let mut state = state_with_songs("frames");
        open_new(&mut state);
        state.update(Message::UseFrame(Frame::Stage), 1_790_000_002);
        assert_eq!(state.editing().unwrap().slots.len(), 12);
        state.update(Message::UseFrame(Frame::Free), 1_790_000_003);
        assert!(state.editing().unwrap().slots.is_empty());
        state.update(Message::Put("a1".into()), 1_790_000_004);
        state.update(Message::UseFrame(Frame::Duel), 1_790_000_005);
        assert_eq!(state.editing().unwrap().frame, Frame::Free, "a pool with a map keeps its frame");
    }

    #[test]
    fn a_map_goes_to_the_first_empty_slot_then_to_the_selected_one() {
        let mut state = state_with_songs("put");
        open_new(&mut state);
        state.update(Message::Put("a1".into()), 1_790_000_002);
        state.update(Message::Put("b2".into()), 1_790_000_003);
        let pool = state.editing().unwrap();
        assert_eq!((pool.slots[0].hash.as_deref(), pool.slots[1].hash.as_deref()), (Some("a1"), Some("b2")));
        assert_eq!(pool.slots[0].title, "Glass Orchard");
        assert_eq!(state.editing().unwrap().filled(), 2, "the second map does not replace the first");
        state.update(Message::Select(Some(0)), 1_790_000_004);
        state.update(Message::Put("c3".into()), 1_790_000_004);
        assert_eq!(state.editing().unwrap().slots[2].hash.as_deref(), Some("c3"), "a filled slot that is only selected is not replaced");
        state.update(Message::Select(Some(0)), 1_790_000_005);
        state.update(Message::Replace, 1_790_000_005);
        state.update(Message::Put("d4".into()), 1_790_000_005);
        let pool = state.editing().unwrap();
        assert_eq!(pool.slots[0].hash.as_deref(), Some("d4"), "replacing is done on request");
        assert_eq!(pool.filled(), 3);
        assert_eq!(pools::load_all(&state.dir)[0].slots[0].hash.as_deref(), Some("d4"));
    }

    #[test]
    fn a_map_already_in_the_pool_is_refused_with_its_slot() {
        let mut state = state_with_songs("twice");
        open_new(&mut state);
        state.update(Message::Put("a1".into()), 1_790_000_002);
        state.update(Message::Select(None), 1_790_000_003);
        state.update(Message::Put("a1".into()), 1_790_000_004);
        assert_eq!(state.notice, Some(Notice::Already(0)));
        assert_eq!(state.editing().unwrap().filled(), 1);
        state.update(Message::Query("g".into()), 1_790_000_005);
        assert_eq!(state.notice, None, "typing clears the notice");
    }

    #[test]
    fn a_full_frame_grows_by_a_slot_and_a_free_pool_grows_from_nothing() {
        let mut state = state_with_songs("grow");
        open_new(&mut state);
        for (at, hash) in ["a1", "b2", "c3", "d4"].into_iter().enumerate() {
            state.update(Message::Select(None), 1_790_000_002 + at as i64);
            state.update(Message::Put(hash.into()), 1_790_000_010 + at as i64);
        }
        assert_eq!(state.editing().unwrap().filled(), 4);
        let mut free = state_with_songs("free");
        open_new(&mut free);
        free.update(Message::UseFrame(Frame::Free), 1_790_000_002);
        free.update(Message::Put("a1".into()), 1_790_000_003);
        assert_eq!(free.editing().unwrap().slots.len(), 1);
        assert_eq!(free.editing().unwrap().slots[0].mods, Mod::Nm);
        free.update(Message::Clear(0), 1_790_000_004);
        assert!(free.editing().unwrap().slots.is_empty(), "removing a map from a free pool removes its slot");
    }

    #[test]
    fn clearing_a_slot_in_a_frame_keeps_the_slot_and_its_mod() {
        let mut state = state_with_songs("clear");
        open_new(&mut state);
        state.update(Message::Put("a1".into()), 1_790_000_002);
        state.update(Message::SetMod(0, Mod::Hd), 1_790_000_003);
        state.update(Message::Clear(0), 1_790_000_004);
        let pool = state.editing().unwrap();
        assert_eq!(pool.slots.len(), 7);
        assert!(pool.slots[0].is_empty());
        assert_eq!(pool.slots[0].mods, Mod::Hd);
    }

    #[test]
    fn a_measure_arriving_fills_the_slots_and_reaches_the_disk() {
        let mut state = state_with_songs("measure");
        open_new(&mut state);
        let effects = state.update(Message::Put("a1".into()), 1_790_000_002);
        assert!(matches!(effects.as_slice(), [Effect::Measure(hash, _, Mod::Nm)] if hash == "a1"));
        assert!(state.editing().unwrap().slots[0].measure.is_none());
        state.update(Message::Measured("a1".into(), Mod::Nm, Ok(measure(4.5))), 1_790_000_003);
        assert_eq!(state.editing().unwrap().slots[0].measure, Some(measure(4.5)));
        assert_eq!(pools::load_all(&state.dir)[0].slots[0].measure, Some(measure(4.5)));
        let again = state.update(Message::Put("a1".into()), 1_790_000_004);
        assert!(again.is_empty(), "a map measured once is not measured again");
    }

    #[test]
    fn changing_the_mod_asks_for_a_new_measure_and_keeps_the_old_ones_cached() {
        let mut state = state_with_songs("mods");
        open_new(&mut state);
        state.update(Message::Put("a1".into()), 1_790_000_002);
        state.update(Message::Measured("a1".into(), Mod::Nm, Ok(measure(4.5))), 1_790_000_003);
        let effects = state.update(Message::SetMod(0, Mod::Dt), 1_790_000_004);
        assert!(matches!(effects.as_slice(), [Effect::Measure(_, _, Mod::Dt)]));
        assert!(state.editing().unwrap().slots[0].measure.is_none());
        state.update(Message::Measured("a1".into(), Mod::Dt, Ok(measure(5.6))), 1_790_000_005);
        assert_eq!(state.editing().unwrap().slots[0].measure.unwrap().stars, 5.6);
        let back = state.update(Message::SetMod(0, Mod::Nm), 1_790_000_006);
        assert!(back.is_empty(), "the old measure comes from the cache");
        assert_eq!(state.editing().unwrap().slots[0].measure.unwrap().stars, 4.5);
        assert_eq!(pools::load_all(&state.dir)[0].slots[0].measure.unwrap().stars, 4.5);
        state.update(Message::Undo, 1_790_000_007);
        assert_eq!(state.editing().unwrap().slots[0].mods, Mod::Dt);
        assert_eq!(state.editing().unwrap().slots[0].measure.unwrap().stars, 5.6);
    }

    #[test]
    fn opening_the_shelf_reads_songs_once_and_a_pool_from_an_older_calculation_is_measured_again() {
        let dir = scratch("old");
        let mut pool = Pool::new(Frame::Duel, "Old", 1_790_000_000);
        pool.slots[0].hash = Some("a1".into());
        pool.slots[0].measure = Some(measure(3.0));
        pool.calc = 0;
        pools::save(&dir, &pool).unwrap();
        let mut state = State::new(dir);
        let first = state.open();
        assert!(matches!(first.as_slice(), [Effect::ReadSongs, Effect::Catalogue(false, 0)]));
        assert!(state.list[0].slots[0].measure.is_none());
        assert_eq!(state.list[0].calc, pools::CALC_VERSION);
        assert!(state.open().is_empty(), "reading is already under way");
        let mut songs = HashMap::new();
        songs.insert("a1".to_owned(), map("Glass Orchard", "Nova Tide", "Garden"));
        let after = state.update(Message::Songs(Arc::new(songs)), 1_790_000_100);
        assert!(matches!(after.as_slice(), [Effect::Measure(hash, _, Mod::Nm)] if hash == "a1"));
    }

    #[test]
    fn search_needs_every_word_and_lists_at_most_forty_by_title() {
        let state = state_with_songs("search");
        let found: Vec<String> = state.search("nova  glass").into_iter().map(|(_, map)| map.title).collect();
        assert_eq!(found, ["Glass Harbour", "Glass Orchard"]);
        assert!(state.search("   ").is_empty());
        assert!(state.search("zzz").is_empty());
        let mut many = HashMap::new();
        for at in 0..100 {
            many.insert(format!("h{at:03}"), map(&format!("Song {at:03}"), "Artist", "Normal"));
        }
        let mut crowded = State::new(scratch("crowd"));
        crowded.songs = Some(Arc::new(many));
        let listed = crowded.search("song");
        assert_eq!(listed.len(), RESULTS_MOST);
        assert!(listed.windows(2).all(|pair| pair[0].1.title <= pair[1].1.title));
    }

    #[test]
    fn covers_ask_for_the_first_four_maps_of_each_pool_on_the_shelf() {
        let mut state = state_with_songs("covers");
        open_new(&mut state);
        for hash in ["a1", "b2", "c3", "d4"] {
            state.update(Message::Select(None), 1_790_000_002);
            state.update(Message::Put(hash.into()), 1_790_000_003);
        }
        state.update(Message::Rename("Four".into()), 1_790_000_004);
        state.update(Message::Back, 1_790_000_005);
        let wanted: Vec<String> = state.covers().into_iter().map(|(hash, _)| hash).collect();
        assert_eq!(wanted.len(), 4);
        assert!(wanted.contains(&"a1".to_owned()) && wanted.contains(&"d4".to_owned()));
    }

    #[test]
    fn a_map_that_is_not_on_the_disk_keeps_its_label_and_has_no_measure() {
        let mut state = state_with_songs("away");
        open_new(&mut state);
        state.update(Message::Put("a1".into()), 1_790_000_002);
        state.update(Message::Measured("a1".into(), Mod::Nm, Err("gone".into())), 1_790_000_003);
        let slot = &state.editing().unwrap().slots[0];
        assert_eq!(slot.title, "Glass Orchard");
        assert!(slot.measure.is_none());
        assert!(matches!(state.measures.get("a1", Mod::Nm), Some(Err(_))));
    }

    fn found(hashes: &[&str], picked: Option<usize>) -> pool_links::Found {
        pool_links::Found {
            set: 77,
            artist: "Nova Tide".into(),
            title: "Glass Orchard".into(),
            difficulties: hashes.iter().enumerate().map(|(at, hash)| pool_links::Difficulty { id: 100 + at as u64, hash: (*hash).into(), version: format!("Diff {at}"), stars: 3.0 + at as f64 }).collect(),
            picked,
        }
    }

    const LINK: &str = "https://osu.ppy.sh/beatmapsets/77#osu/100";

    fn in_conflict(name: &str) -> (State, String, crate::bot::Publication) {
        let (mut state, id) = ready_to_publish(name);
        state.update(Message::Published(id.clone(), Ok(publication(&id, 3))), 1_790_000_110);
        state.update(Message::PublishSheet(true), 1_790_000_111);
        state.update(Message::Publish, 1_790_000_112);
        let effects = state.update(Message::Published(id.clone(), Err("409".into())), 1_790_000_113);
        assert!(matches!(effects.as_slice(), [Effect::Catalogue(false, 0)]), "the catalogue is asked what the server holds: {effects:?}");
        assert_eq!(state.conflict.as_deref(), Some(id.as_str()));
        assert!(state.catalogue_error.is_none());
        assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.publish), "the sheet steps aside for the conflict");
        let mut theirs = state.editing().unwrap().clone();
        theirs.name = "Spring duel remote".into();
        let row = crate::bot::Publication { content: serde_json::from_slice(&pool_share::to_file(&theirs)).unwrap(), name: theirs.name.clone(), ..publication(&id, 4) };
        state.update(Message::CatalogueLoaded(false, false, Ok(vec![row.clone()])), 1_790_000_114);
        assert_eq!(state.server_revision(state.editing().unwrap()), Some(4));
        assert_eq!(state.conflict.as_deref(), Some(id.as_str()), "knowing the server version does not settle anything");
        (state, id, row)
    }

    #[test]
    fn a_conflict_is_told_with_both_versions_and_two_ways_out() {
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        let (state, _, _) = in_conflict("conflict-told");
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1300.0, 900.0), view(&state, &words, &thumbs, 1300.0, 1.0, &Clocks::settled()));
        assert!(screen.find(words.t("catalogue-conflict-title")).is_ok());
        assert!(screen.find("the server has version 4, you have version 3").is_ok());
        assert!(screen.find(words.t("catalogue-conflict")).is_err(), "the bare sentence gives way to the banner");
        screen.click(words.t("catalogue-conflict-mine").as_str()).unwrap();
        assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::Conflict(Keep::Mine)]));
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1300.0, 900.0), view(&state, &words, &thumbs, 1300.0, 1.0, &Clocks::settled()));
        screen.click(words.t("catalogue-conflict-theirs").as_str()).unwrap();
        assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::Conflict(Keep::Theirs)]));
    }

    #[test]
    fn either_way_out_of_a_conflict_keeps_both_versions_and_opens_the_chosen_one() {
        for keep in [Keep::Theirs, Keep::Mine] {
            let (mut state, id, row) = in_conflict(&format!("conflict-{keep:?}"));
            let effects = state.update(Message::Conflict(keep), 1_790_000_120);
            assert!(matches!(effects.as_slice(), [Effect::Catalogue(false, 0)]), "{keep:?}: {effects:?}");
            assert!(state.update(Message::Conflict(keep), 1_790_000_121).is_empty(), "{keep:?}: one answer at a time");
            let effects = state.update(Message::CatalogueLoaded(false, false, Ok(vec![row])), 1_790_000_122);
            assert!(effects.iter().any(|effect| matches!(effect, Effect::Say("catalogue-conflict-kept"))), "{keep:?}: {effects:?}");
            assert!(state.conflict.is_none() && state.resolving.is_none());
            let theirs = state.list.iter().find(|pool| pool.id == id).unwrap();
            assert_eq!((theirs.name.as_str(), theirs.published_revision), ("Spring duel remote", 4), "{keep:?}");
            let mine = state.list.iter().find(|pool| pool.id != id && pool.name == "Spring duel v3").expect("my variant is kept under its own name");
            assert_eq!(mine.published_revision, 0, "{keep:?}: the copy is a fresh draft");
            assert_eq!(mine.filled(), 2);
            let opened = state.editing().unwrap().id.clone();
            assert_eq!(opened == id, keep == Keep::Theirs, "{keep:?}");
            let kept = pools::load_all(&state.dir);
            assert!(kept.iter().any(|pool| pool.name == "Spring duel v3") && kept.iter().any(|pool| pool.name == "Spring duel remote"), "{keep:?}: both are on disk");
        }
    }

    #[test]
    fn a_conflict_over_a_removed_publication_lets_the_pool_be_published_anew() {
        let (mut state, id, _) = in_conflict("conflict-gone");
        state.update(Message::Conflict(Keep::Theirs), 1_790_000_120);
        let effects = state.update(Message::CatalogueLoaded(false, false, Ok(Vec::new())), 1_790_000_121);
        assert!(matches!(effects.as_slice(), [Effect::Say("catalogue-conflict-gone")]), "{effects:?}");
        assert_eq!(state.editing().unwrap().published_revision, 0);
        assert_eq!(state.list.iter().filter(|pool| pool.id == id).count(), 1);
        assert!(state.conflict.is_none());
        let (mut failing, _, _) = in_conflict("conflict-offline");
        failing.update(Message::Conflict(Keep::Mine), 1_790_000_120);
        failing.update(Message::CatalogueLoaded(false, false, Err("503".into())), 1_790_000_121);
        assert!(failing.resolving.is_none() && failing.conflict.is_some(), "a failed refresh leaves the question open");
    }

    #[test]
    fn the_first_steps_follow_what_was_done_and_end_when_all_five_are() {
        let mut state = state_with_songs("guide-steps");
        state.guide = pools::Guide::default();
        state.update(Message::New, 100);
        state.update(Message::Begin(Start::Empty), 100);
        let steps = |state: &State| state.guide.steps(state.editing().unwrap());
        assert_eq!(steps(&state), [false; 5]);
        let effects = state.update(Message::Rename("Spring duel".into()), 101);
        assert!(effects.iter().all(|effect| !matches!(effect, Effect::Guide(_))), "a name is read from the pool, nothing is kept for it");
        state.update(Message::Put("a1".into()), 102);
        state.update(Message::Put("b2".into()), 103);
        assert_eq!(steps(&state), [true, true, false, false, false]);
        let effects = state.update(Message::SetMod(0, Mod::Hd), 104);
        assert!(matches!(effects.iter().filter(|effect| matches!(effect, Effect::Guide(_))).collect::<Vec<_>>().as_slice(), [Effect::Guide(guide)] if guide.modded && guide.on), "{effects:?}");
        let keys = slot_keys(state.editing().unwrap());
        let id = state.editing().unwrap().id.clone();
        let effects = state.update(Message::Move(id, keys.clone(), keys[1], Some(keys[0])), 105);
        assert!(effects.iter().any(|effect| matches!(effect, Effect::Guide(guide) if guide.moved)));
        assert_eq!(steps(&state), [true, true, true, true, false]);
        let effects = state.update(Message::Share(true), 106);
        assert!(effects.iter().any(|effect| matches!(effect, Effect::Guide(guide) if guide.shared && !guide.on)), "the fifth step ends the hints: {effects:?}");
        assert!(!state.guide.on);
        assert!(state.update(Message::Share(false), 107).iter().all(|effect| !matches!(effect, Effect::Guide(_))));
    }

    #[test]
    fn the_first_steps_show_in_a_pool_only_while_they_are_wanted_and_can_be_hidden() {
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        let mut state = pool_with_maps("guide-card", &["a1", "b2"]);
        state.update(Message::Select(None), 1_790_000_100);
        let seen = |state: &State, width: f32, label: &str| iced_test::Simulator::with_size(crate::settings(), iced::Size::new(width, 900.0), view(state, &words, &thumbs, width, 1.0, &Clocks::settled())).find(label).is_ok();
        let caption = words.t("guide-title").to_uppercase();
        assert!(!seen(&state, 1300.0, &caption), "hints are off until the settings say otherwise");
        state.guide = pools::Guide::default();
        for width in [980.0, 1300.0, 1500.0, 2096.0] {
            assert!(seen(&state, width, &caption), "{width}");
            for key in pools::Guide::STEPS {
                assert!(seen(&state, width, &words.t(key)), "{width}: {key}");
            }
            assert!(seen(&state, width, &words.t("guide-name-about")), "{width}: the step in hand is explained");
            assert!(seen(&state, width, &words.of(1, 5)), "{width}");
        }
        assert!(seen(&state, 1500.0, &words.t("guide-back")));
        state.update(Message::Rename("Spring duel".into()), 1_790_000_101);
        assert!(seen(&state, 1300.0, &words.t("guide-mod-about")) && !seen(&state, 1300.0, &words.t("guide-name-about")));
        let at = state.editing_at().unwrap();
        state.list[at].collection = true;
        assert!(!seen(&state, 1300.0, &caption), "a collection has no first steps");
        state.list[at].collection = false;
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1300.0, 900.0), view(&state, &words, &thumbs, 1300.0, 1.0, &Clocks::settled()));
        screen.click(words.t("guide-hide").as_str()).unwrap();
        assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::HideGuide]));
        let effects = state.update(Message::HideGuide, 1_790_000_102);
        assert!(matches!(effects.as_slice(), [Effect::Guide(guide)] if !guide.on));
        assert!(!seen(&state, 1300.0, &caption));
    }

    fn skilled(aim: f64, speed: f64, reading: f64, stamina: f64) -> Measure {
        Measure { aim, speed, reading, stamina, ..measure(5.0) }
    }

    #[test]
    fn the_pool_balance_shows_from_two_measured_maps_and_offers_a_map_for_the_weakest_skill() {
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        let mut state = pool_with_maps("balance", &["a1", "b2", "c3"]);
        state.update(Message::Select(None), 1_790_000_100);
        let at = state.editing_at().unwrap();
        let seen = |state: &State, width: f32, label: &str| iced_test::Simulator::with_size(crate::settings(), iced::Size::new(width, 900.0), view(state, &words, &thumbs, width, 1.0, &Clocks::settled())).find(label).is_ok();
        let caption = words.t("pool-balance").to_uppercase();
        state.list[at].slots[0].measure = Some(skilled(4.0, 1.0, 0.3, 0.7));
        assert!(!seen(&state, 1300.0, &caption), "one map is not a balance yet");
        state.list[at].slots[1].measure = Some(skilled(3.6, 1.2, 0.4, 0.8));
        for width in [980.0, 1300.0, 1500.0, 2096.0] {
            assert!(seen(&state, width, &caption), "{width}");
            assert!(seen(&state, width, &words.t("pool-heavy-aim")), "{width}");
            assert!(seen(&state, width, &words.t("pool-balance-pick-reading")), "{width}");
            for skill in Skill::ALL {
                assert!(seen(&state, width, &words.t(skill.key())), "{width}: {skill:?}");
            }
        }
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1300.0, 900.0), view(&state, &words, &thumbs, 1300.0, 1.0, &Clocks::settled()));
        screen.click(words.t("pool-balance-pick-reading").as_str()).unwrap();
        assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::Lean(Skill::Reading)]));
        state.list[at].slots[0].measure = Some(skilled(1.0, 1.0, 1.0, 1.0));
        state.list[at].slots[1].measure = Some(skilled(1.0, 1.0, 1.0, 1.0));
        assert!(seen(&state, 1300.0, &words.t("pool-even")));
        assert!(!seen(&state, 1300.0, &words.t("pool-balance-pick-aim")) && !seen(&state, 1300.0, &words.t("pool-balance-pick-reading")), "an even pool asks for nothing");
        state.list[at].collection = true;
        assert!(!seen(&state, 1300.0, &caption), "a collection has no balance");
    }

    #[test]
    fn asking_for_a_map_of_a_skill_opens_the_suggestions_ordered_by_that_skill() {
        let mut state = pool_with_maps("balance-lean", &["a1", "b2"]);
        state.update(Message::Select(None), 1_790_000_100);
        let at = state.editing_at().unwrap();
        state.list[at].slots[0].measure = Some(skilled(4.0, 1.0, 0.3, 0.7));
        state.list[at].slots[1].measure = Some(skilled(3.6, 1.2, 0.4, 0.8));
        let effects = state.update(Message::Lean(Skill::Reading), 1_790_000_101);
        assert!(effects.iter().any(|effect| matches!(effect, Effect::Suggest(..))), "{effects:?}");
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.panel == Panel::Add && editor.source == SourceTab::Suggest && editor.lean == Some(Skill::Reading)));
        let request = state.suggestions.as_ref().unwrap().request;
        let found: Vec<(String, Measure)> = (0..12).map(|at| (format!("m{at}"), skilled(3.0, 1.0, at as f64 * 0.2, 1.0))).collect();
        state.update(Message::Suggested(request, found), 1_790_000_102);
        let maps = state.suggestions.as_ref().unwrap().maps.clone().unwrap();
        assert_eq!(maps.len(), pools::SUGGESTED);
        assert_eq!(maps[0].0, "m11", "the map with the most reading comes first");
        assert!(maps.windows(2).all(|pair| pair[0].1.shares()[2] >= pair[1].1.shares()[2]));
        state.update(Message::Select(None), 1_790_000_103);
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.lean.is_none()), "the leaning ends with the panel");
    }

    #[test]
    fn the_checks_before_publishing_tell_what_stops_it_and_what_is_only_worth_knowing() {
        let mut state = state_with_songs("publish-checks");
        state.update(Message::New, 100);
        state.update(Message::Begin(Start::Empty), 100);
        let id = state.editing().unwrap().id.clone();
        let told = |state: &State| publish_checks(state.editing().unwrap()).into_iter().map(|check| (check.verdict, check.key)).collect::<Vec<_>>();
        assert_eq!(told(&state), vec![(Verdict::Stop, "catalogue-sign-in"), (Verdict::Stop, "publish-name-missing"), (Verdict::Stop, "publish-no-maps"), (Verdict::Note, "publish-no-mappoolers")]);
        assert!(!publish_ready(state.editing().unwrap()));
        state.update(Message::OwnCompiler(id, "NaumRedlo".into()), 101);
        state.update(Message::Rename("Spring duel".into()), 102);
        state.update(Message::Put("a1".into()), 103);
        assert_eq!(told(&state), vec![(Verdict::Fine, "publish-name-set"), (Verdict::Note, "publish-filled"), (Verdict::Note, "publish-unmeasured"), (Verdict::Note, "publish-no-mappoolers")]);
        let checks = publish_checks(state.editing().unwrap());
        assert_eq!((checks[1].count, checks[1].slot), (Some((1, 4)), Some(1)), "the first empty slot can be shown");
        assert_eq!((checks[2].count, checks[2].slot), (Some((1, 1)), Some(0)), "so can the first map still being measured");
        assert!(publish_ready(state.editing().unwrap()), "notes do not stop a publication");
        let words = Words::new(crate::lang::Lang::En);
        assert_eq!(checks[1].words(&words), "1 slot of 4 is filled");
        let at = state.editing_at().unwrap();
        state.list[at].slots.truncate(1);
        state.list[at].slots[0].measure = Some(measure(4.5));
        state.list[at].authors = vec!["kotofey".into()];
        assert_eq!(told(&state), vec![(Verdict::Fine, "publish-name-set"), (Verdict::Fine, "publish-filled"), (Verdict::Fine, "publish-measured")]);
        state.list[at].collection = true;
        assert_eq!(told(&state), vec![(Verdict::Fine, "publish-name-set"), (Verdict::Fine, "publish-maps")], "a collection has no slots to fill, no difficulty and no mappoolers");
    }

    fn ready_to_publish(name: &str) -> (State, String) {
        let mut state = pool_with_maps(name, &["a1", "b2"]);
        let id = state.editing().unwrap().id.clone();
        state.update(Message::Rename("Spring duel".into()), 1_790_000_100);
        state.update(Message::OwnCompiler(id.clone(), "NaumRedlo".into()), 1_790_000_101);
        state.update(Message::Select(None), 1_790_000_102);
        (state, id)
    }

    fn publication(id: &str, revision: u64) -> crate::bot::Publication {
        crate::bot::Publication { code: String::new(), id: format!("pub-{id}"), kind: "pool".into(), local_id: id.to_owned(), revision, name: "Spring duel".into(), content: serde_json::json!({}), mine: true }
    }

    #[test]
    fn the_publication_sheet_opens_from_the_toolbar_sends_only_a_ready_pool_and_closes_once_published() {
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        let (mut state, id) = ready_to_publish("publish-sheet");
        let pressed = |state: &State, label: &str| -> Vec<Message> {
            let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1300.0, 900.0), view(state, &words, &thumbs, 1300.0, 1.0, &Clocks::settled()));
            screen.click(label).unwrap_or_else(|_| panic!("{label} is on screen"));
            screen.into_messages().collect()
        };
        assert!(matches!(pressed(&state, &words.t("catalogue-publish")).as_slice(), [Message::PublishSheet(true)]), "the toolbar asks first");
        state.update(Message::Select(Some(0)), 1_790_000_110);
        state.update(Message::PublishSheet(true), 1_790_000_111);
        assert_eq!(state.panel_key(), "publish");
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.publish && editor.panel == Panel::Closed), "the sheet replaces an open panel");
        {
            let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1300.0, 900.0), view(&state, &words, &thumbs, 1300.0, 1.0, &Clocks::settled()));
            for key in ["publish-before", "publish-seen", "pool-publisher", "publish-version"] {
                assert!(screen.find(words.t(key).to_uppercase()).is_ok(), "{key}");
            }
            for key in ["publish-name-set", "publish-no-mappoolers", "publish-show-slot", "publish-cancel"] {
                assert!(screen.find(words.t(key)).is_ok(), "{key}");
            }
            assert!(screen.find(words.t("catalogue-withdraw")).is_err(), "nothing is published yet");
        }
        let at = state.editing_at().unwrap();
        let name = std::mem::take(&mut state.list[at].name);
        assert!(state.update(Message::Publish, 1_790_000_112).is_empty(), "a pool without a name stays home");
        state.list[at].name = name;
        let effects = state.update(Message::Publish, 1_790_000_113);
        assert!(matches!(effects.as_slice(), [Effect::Publish(pool)] if pool.id == id));
        state.update(Message::Published(id.clone(), Err("503".into())), 1_790_000_114);
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.publish), "a refusal is read on the sheet");
        assert_eq!(state.catalogue_error.as_deref(), Some("503"));
        state.update(Message::Publish, 1_790_000_115);
        let effects = state.update(Message::Published(id.clone(), Ok(publication(&id, 1))), 1_790_000_116);
        assert!(matches!(effects.as_slice(), [Effect::Say("catalogue-published-now")]), "{effects:?}");
        assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.publish));
        assert_eq!(state.panel_key(), "none");
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.gone.as_ref().is_some_and(|was| was.publish)), "the sheet fades out like the panels do");
        assert_eq!(state.editing().unwrap().published_revision, 1);
    }

    #[test]
    fn the_sheets_grow_only_in_a_large_window() {
        assert_eq!(sheet_scale(929.0), 1.0);
        assert_eq!(sheet_scale(1334.0), 1.0);
        assert_eq!(sheet_scale(1600.0), 1.0);
        assert!((sheet_scale(1867.0) - 1.167).abs() < 0.01);
        assert_eq!(sheet_scale(2096.0), SHEET_GROWTH);
        let words = Words::new(crate::lang::Lang::Ru);
        let (mut state, _) = ready_to_publish("sheet-grown");
        state.update(Message::PublishSheet(true), 1_790_000_110);
        let button = |width: f32| {
            let image = picture(&state, &words, iced::Size::new(width, 1100.0), iced::Point::new(4.0, 4.0), &Clocks::settled(), &format!("sheet-grown-{width}"));
            let scale = image.width() as f32 / width;
            let red = |pixel: &::image::Rgba<u8>| pixel.0[0] > 190 && pixel.0[1] < 110 && pixel.0[2] < 110;
            let rows: Vec<u32> = (image.height() / 3..image.height()).filter(|y| (0..image.width()).filter(|x| red(image.get_pixel(*x, *y))).count() as f32 > 60.0 * scale).collect();
            assert!(!rows.is_empty(), "{width}: the main button of the sheet is drawn");
            (rows[rows.len() - 1] - rows[0] + 1) as f32 / scale
        };
        let (small, large) = (button(1400.0), button(2096.0));
        assert!(large > small * 1.2, "the sheet is larger in a large window: {small} and {large}");
    }

    #[test]
    fn a_check_leads_to_its_slot_and_a_publication_is_withdrawn_only_after_a_second_yes() {
        let (mut state, id) = ready_to_publish("publish-withdraw");
        state.update(Message::PublishSheet(true), 1_790_000_110);
        state.update(Message::ShowSlot(1), 1_790_000_111);
        assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.publish && editor.selected == Some(1)));
        state.update(Message::Select(None), 1_790_000_112);
        assert!(state.update(Message::Withdraw, 1_790_000_113).is_empty(), "there is nothing to withdraw before a publication");
        state.update(Message::Published(id.clone(), Ok(publication(&id, 2))), 1_790_000_114);
        assert_eq!(state.publications.len(), 1);
        state.update(Message::PublishSheet(true), 1_790_000_115);
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        let labels = |state: &State| {
            let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1300.0, 900.0), view(state, &words, &thumbs, 1300.0, 1.0, &Clocks::settled()));
            (screen.find(words.t("catalogue-withdraw")).is_ok(), screen.find(words.t("catalogue-withdraw-yes")).is_ok(), screen.find(words.t("catalogue-update")).is_ok(), screen.find("3").is_ok())
        };
        assert_eq!(labels(&state), (true, false, true, true), "a published pool offers an update, its next version and the way out");
        state.update(Message::AskWithdraw(true), 1_790_000_116);
        assert_eq!(labels(&state), (false, true, true, true));
        let effects = state.update(Message::Withdraw, 1_790_000_117);
        assert!(matches!(effects.as_slice(), [Effect::Withdraw(pool)] if pool.id == id));
        assert!(state.publishing);
        state.update(Message::Withdrawn(id.clone(), Err("503".into())), 1_790_000_118);
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.publish && !editor.asking_withdraw));
        assert_eq!(state.editing().unwrap().published_revision, 2, "a failed withdrawal changes nothing");
        state.update(Message::AskWithdraw(true), 1_790_000_119);
        state.update(Message::Withdraw, 1_790_000_120);
        let effects = state.update(Message::Withdrawn(id.clone(), Ok(())), 1_790_000_121);
        assert!(matches!(effects.as_slice(), [Effect::Say("catalogue-withdrawn")]), "{effects:?}");
        assert_eq!(state.editing().unwrap().published_revision, 0);
        assert!(state.publications.is_empty());
        assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.publish));
        assert_eq!(pools::load_all(&state.dir).iter().find(|pool| pool.id == id).unwrap().published_revision, 0);
    }

    #[test]
    fn a_new_pool_begins_on_the_start_screen_and_a_new_collection_goes_straight_to_the_editor() {
        let mut state = state_with_songs("start-new");
        state.update(Message::New, 100);
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.starting && editor.untouched));
        state.update(Message::Rename("Spring duel".into()), 101);
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.starting), "a name does not leave the start");
        state.update(Message::Begin(Start::Empty), 102);
        assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.starting && editor.panel == Panel::Closed));
        assert_eq!(state.editing().unwrap().slots.len(), 4);
        let id = state.editing().unwrap().id.clone();
        state.update(Message::Back, 103);
        state.collection_shelf = true;
        state.update(Message::New, 104);
        assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.starting));
        state.update(Message::Back, 105);
        state.collection_shelf = false;
        state.update(Message::Open(id), 106);
        assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.starting), "an opened pool never starts over");
    }

    #[test]
    fn the_start_screen_offers_two_frames_and_keeps_the_duel_for_later() {
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        let mut state = state_with_songs("start-frames");
        state.update(Message::New, 100);
        let pressed = |state: &State, label: &str| -> Vec<Message> {
            let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1300.0, 900.0), view(state, &words, &thumbs, 1300.0, 1.0, &Clocks::settled()));
            screen.click(label).unwrap_or_else(|_| panic!("{label} is on the start screen"));
            screen.into_messages().collect()
        };
        assert!(pressed(&state, &words.t("pool-start-duel")).is_empty(), "the duel is shown and cannot be chosen yet");
        assert!(matches!(pressed(&state, &words.t("pool-start-stage")).as_slice(), [Message::UseFrame(Frame::Stage)]));
        assert!(matches!(pressed(&state, &words.t("pool-frame-free")).as_slice(), [Message::UseFrame(Frame::Free)]));
        {
            let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1300.0, 900.0), view(&state, &words, &thumbs, 1300.0, 1.0, &Clocks::settled()));
            for label in ["NM 4", "HD 2", "HR 2", "DT 2", "FM", "TB", "NM 2"] {
                assert!(screen.find(label).is_ok(), "{label}");
            }
            for key in ["pool-frame-soon", "pool-frame-own", "pool-start-empty", "pool-start-links", "pool-start-collections", "pool-start-best", "pool-start-go"] {
                assert!(screen.find(words.t(key)).is_ok(), "{key}");
            }
            assert!(screen.find(words.t("pool-add")).is_err(), "the start has one main button");
        }
        state.update(Message::UseFrame(Frame::Stage), 101);
        assert_eq!(state.editing().unwrap().slots.len(), 12);
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.starting));
        state.update(Message::UseFrame(Frame::Free), 102);
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.starting));
        assert!(matches!(pressed(&state, &words.t("pool-start-go")).as_slice(), [Message::Begin(Start::Empty)]));
        assert!(matches!(pressed(&state, &words.t("pool-start-best")).as_slice(), [Message::Begin(Start::Best)]));
        state.update(Message::Begin(Start::Empty), 103);
        assert_eq!(state.editing().unwrap().slots.len(), 4, "a free pool begins with the same four slots whichever frame was tried first");
        assert!(state.editing().unwrap().slots.iter().all(|slot| slot.is_empty() && slot.mods == Mod::Nm));
    }

    #[test]
    fn the_start_screen_grows_with_a_wide_window_and_stays_as_drawn_on_a_small_one() {
        assert_eq!(start_scale(929.0), 1.0);
        assert_eq!(start_scale(1286.0), 1.0);
        assert!((start_scale(1562.0) - 1.21).abs() < 0.01);
        assert!((start_scale(1800.0) - 1.395).abs() < 0.01);
        assert_eq!(start_scale(4000.0), START_GROWTH);
        let words = Words::new(crate::lang::Lang::Ru);
        let mut state = state_with_songs("start-grown");
        state.update(Message::New, 100);
        let button = |width: f32| {
            let image = picture(&state, &words, iced::Size::new(width, 1100.0), iced::Point::new(4.0, 4.0), &Clocks::settled(), &format!("start-grown-{width}"));
            let scale = image.width() as f32 / width;
            let red = |pixel: &::image::Rgba<u8>| pixel.0[0] > 190 && pixel.0[1] < 110 && pixel.0[2] < 110;
            let rows: Vec<u32> = (image.height() / 5..image.height()).filter(|y| (0..image.width()).filter(|x| red(image.get_pixel(*x, *y))).count() as f32 > 40.0 * scale).collect();
            assert!(!rows.is_empty(), "{width}: the main button is drawn");
            ((rows[rows.len() - 1] - rows[0] + 1) as f32 / scale, (rows[rows.len() - 1] + 1) as f32 / scale)
        };
        let (small_high, small_end) = button(1334.0);
        let (large_high, large_end) = button(2096.0);
        assert!(large_high > small_high * 1.3, "the controls are larger in a wide window: {small_high} and {large_high}");
        assert!(large_end > small_end * 1.25 && large_end < 1000.0, "the page fills more of a large window and still fits it: {small_end} and {large_end}");
    }

    #[test]
    fn each_way_to_begin_leaves_the_start_and_opens_its_source() {
        let mut state = state_with_songs("start-ways");
        state.update(Message::New, 100);
        let id = state.editing().unwrap().id.clone();
        let effects = state.update(Message::Begin(Start::Links), 101);
        assert!(effects.iter().any(|effect| matches!(effect, Effect::ReadPaste(pool, Input::Query, contents) if pool == &id && contents.is_empty())), "{effects:?}");
        assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.starting && editor.panel == Panel::Add && editor.source == SourceTab::Search));
        state.update(Message::Back, 102);
        state.update(Message::New, 103);
        let effects = state.update(Message::Begin(Start::Collections), 104);
        assert!(effects.iter().any(|effect| matches!(effect, Effect::ReadCollections)), "{effects:?}");
        assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.starting && editor.wholesale && editor.panel == Panel::Add && editor.source == SourceTab::Collections));
        state.update(Message::Back, 105);
        state.update(Message::New, 106);
        let effects = state.update(Message::Begin(Start::Best), 107);
        assert!(effects.iter().any(|effect| matches!(effect, Effect::ReadBest(false))), "{effects:?}");
        assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.starting && editor.spread && editor.panel == Panel::Closed), "the best plays are waited for, no panel opens");
        state.update(Message::Back, 108);
        state.update(Message::New, 109);
        state.update(Message::Put("a1".into()), 110);
        assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.starting), "a map dropped on the start goes straight to the board");
    }

    #[test]
    fn a_slot_in_trouble_says_what_is_wrong_and_offers_the_way_out() {
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        let mut state = pool_with_maps("slot-ail", &["a1", "b2"]);
        state.update(Message::Select(None), 1_790_000_100);
        let at = state.editing_at().unwrap();
        assert_eq!(state.ail(&state.list[at].slots[0]), Ail::Well);
        state.list[at].slots[1].hash = Some("f".repeat(32));
        state.list[at].slots[1].measure = None;
        assert_eq!(state.ail(&state.list[at].slots[1]), Ail::Missing);
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1300.0, 900.0), view(&state, &words, &thumbs, 1300.0, 1.0, &Clocks::settled()));
        assert!(screen.find(words.t("pool-no-songs")).is_ok());
        screen.click(words.t("pool-find-mirror").as_str()).unwrap();
        assert!(matches!(screen.into_messages().collect::<Vec<_>>().as_slice(), [Message::FetchSlot(1)]), "the button acts on its own, the row behind it stays shut");
        let effects = state.update(Message::FetchSlot(1), 1_790_000_101);
        assert!(effects.iter().any(|effect| matches!(effect, Effect::Fetch(..))), "{effects:?}");
        assert_eq!(state.ail(&state.list[at].slots[1]), Ail::Fetching);
        assert!(state.update(Message::FetchSlot(0), 1_790_000_102).is_empty(), "one download at a time");
        state.update(Message::Dismiss, 1_790_000_103);
        assert_eq!(state.ail(&state.list[at].slots[1]), Ail::Missing);
        state.list[at].slots[0].measure = None;
        state.measures.put("a1", state.list[at].slots[0].mods, Err("broken".into()));
        assert_eq!(state.ail(&state.list[at].slots[0]), Ail::Unmeasured);
    }

    #[test]
    fn a_whole_collection_becomes_the_draft_when_the_start_asked_for_it() {
        let mut state = state_with_songs("start-collection");
        state.update(Message::New, 100);
        state.update(Message::Begin(Start::Collections), 101);
        state.update(Message::Collections(vec![Collection { name: "Favorites".into(), hashes: vec!["a1".into(), "b2".into(), "a1".into(), "c3".into()] }]), 102);
        state.update(Message::Collection(Some(0)), 103);
        let hashes: Vec<Option<String>> = state.editing().unwrap().slots.iter().map(|slot| slot.hash.clone()).collect();
        assert_eq!(&hashes[..3], &[Some("a1".to_owned()), Some("b2".to_owned()), Some("c3".to_owned())], "every map of the collection, each once");
        assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.wholesale));
        state.update(Message::Collection(None), 104);
        state.update(Message::Collection(Some(0)), 105);
        assert_eq!(state.editing().unwrap().filled(), 3, "opening a collection again takes nothing twice");
    }

    #[test]
    fn best_plays_are_laid_out_by_the_mods_of_the_frame() {
        assert!(played_with(Mod::Nm, "") && played_with(Mod::Nm, "NF,CL") && !played_with(Mod::Nm, "HD"));
        assert!(played_with(Mod::Hd, "HD") && !played_with(Mod::Hd, "HD,DT") && !played_with(Mod::Hd, "HD,HR"));
        assert!(played_with(Mod::Hr, "HD,HR") && !played_with(Mod::Hr, "HR,DT"));
        assert!(played_with(Mod::Dt, "HD,DT") && played_with(Mod::Dt, "NC") && !played_with(Mod::Dt, "HD"));
        assert!(played_with(Mod::Fm, "EZ") && played_with(Mod::Tb, ""));
        let mut state = state_with_songs("start-best");
        state.update(Message::New, 100);
        state.update(Message::UseFrame(Frame::Duel), 100);
        let effects = state.update(Message::Begin(Start::Best), 101);
        assert!(effects.iter().any(|effect| matches!(effect, Effect::ReadBest(false))));
        assert_eq!(state.editing().unwrap().filled(), 0);
        let played = |hash: &str, mods: &str, pp: f64| crate::community::wire::Score { hash: hash.into(), mods: mods.into(), pp, beatmap_id: pp, ..Default::default() };
        state.set_best(Some(&[played("a1", "HD", 400.0), played("b2", "", 300.0), played("c3", "DT", 250.0), played("d4", "", 200.0), played("zz", "", 900.0)]));
        let pool = state.editing().unwrap();
        let placed: Vec<(Mod, Option<&str>)> = pool.slots.iter().map(|slot| (slot.mods, slot.hash.as_deref())).collect();
        assert_eq!(placed[0], (Mod::Nm, Some("b2")), "{placed:?}");
        assert_eq!(placed[1], (Mod::Nm, Some("d4")));
        assert_eq!(placed[2], (Mod::Hd, Some("a1")));
        assert_eq!(placed[4], (Mod::Dt, Some("c3")));
        assert!(pool.slots.iter().all(|slot| slot.hash.as_deref() != Some("zz")), "a play whose map is not on this computer is left out");
        assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.spread && editor.panel == Panel::Closed));
    }

    #[test]
    fn several_links_pasted_at_once_are_taken_one_after_another() {
        let mut state = state_with_songs("links-queue");
        state.update(Message::New, 100);
        let pasted = format!("NM1 <{LINK}>\nNM2: https://osu.ppy.sh/beatmapsets/78#osu/200,\nhttps://osu.ppy.sh/users/5\nhttps://osu.ppy.sh/beatmapsets/79#osu/300");
        assert_eq!(links_in(&pasted).len(), 3);
        assert!(!is_paste(&pasted), "words around the links do not pull a paste out of a text field");
        assert!(is_paste(&format!("{LINK}\nhttps://osu.ppy.sh/beatmapsets/78#osu/200")));
        let id = state.editing().unwrap().id.clone();
        let effects = state.update(Message::PastedInto(id.clone(), Input::Query, String::new(), pasted.clone()), 101);
        assert!(matches!(effects.iter().filter(|effect| matches!(effect, Effect::Resolve(..))).collect::<Vec<_>>().as_slice(), [Effect::Resolve(_, Target::Set { id: 77, beatmap: Some(100), .. })]), "{effects:?}");
        assert_eq!(state.links.len(), 2);
        assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.starting && editor.panel == Panel::Add));
        let effects = state.update(Message::Resolved(state.resolve_request, Ok(found(&["a1"], Some(0)))), 102);
        assert_eq!(state.editing().unwrap().slots[0].hash.as_deref(), Some("a1"), "a link that names one difficulty is taken without a question");
        assert!(effects.iter().any(|effect| matches!(effect, Effect::Resolve(_, Target::Set { id: 78, beatmap: Some(200), .. }))), "{effects:?}");
        assert_eq!(state.links.len(), 1);
        let effects = state.update(Message::Resolved(state.resolve_request, Ok(found(&["b2", "c3"], None))), 103);
        assert!(effects.iter().all(|effect| !matches!(effect, Effect::Resolve(..))));
        assert!(matches!(&state.finding, Some(Finding::Found(candidate)) if candidate.choice.is_none()), "a set waits for the difficulty to be picked");
        state.update(Message::Choose(1), 104);
        let effects = state.update(Message::Confirm, 105);
        assert_eq!(state.editing().unwrap().slots[1].hash.as_deref(), Some("c3"));
        assert!(effects.iter().any(|effect| matches!(effect, Effect::Resolve(_, Target::Set { id: 79, beatmap: Some(300), .. }))), "{effects:?}");
        state.update(Message::Resolved(state.resolve_request, Err(Why::Nowhere)), 106);
        assert!(matches!(state.finding, Some(Finding::Missing)));
        state.update(Message::Dismiss, 107);
        assert!(state.links.is_empty() && !state.linking);
        assert_eq!(state.editing().unwrap().filled(), 2);
        let effects = state.update(Message::PastedInto(id, Input::Name, "Name".into(), pasted), 108);
        assert!(effects.iter().all(|effect| !matches!(effect, Effect::Resolve(..))), "a pool name takes its text as text");
    }

    #[test]
    fn closing_the_add_panel_drops_the_links_still_waiting() {
        let mut state = state_with_songs("links-closed");
        state.update(Message::New, 100);
        state.update(Message::Pasted(format!("{LINK} https://osu.ppy.sh/beatmapsets/78#osu/200 https://osu.ppy.sh/beatmapsets/79#osu/300")), 101);
        assert_eq!(state.links.len(), 2);
        state.update(Message::AddPanel(false), 102);
        assert!(state.links.is_empty() && !state.linking);
        let effects = state.update(Message::Resolved(state.resolve_request, Ok(found(&["a1"], Some(0)))), 103);
        assert!(effects.iter().all(|effect| !matches!(effect, Effect::Resolve(..))), "nothing reopens the panel");
        assert!(matches!(state.finding, Some(Finding::Found(_))));
    }

    #[test]
    fn links_pasted_on_the_shelf_keep_their_queue_through_the_new_pool() {
        let mut state = state_with_songs("links-shelf");
        let effects = state.update(Message::Pasted(format!("{LINK}\nhttps://osu.ppy.sh/beatmapsets/78#osu/200")), 100);
        assert!(effects.iter().any(|effect| matches!(effect, Effect::Resolve(_, Target::Set { id: 77, .. }))));
        assert_eq!(state.links.len(), 1);
        assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.starting && editor.panel == Panel::Add));
    }

    #[test]
    fn a_link_pasted_on_the_shelf_makes_a_pool_opens_the_add_panel_and_asks_the_mirror() {
        let mut state = state_with_songs("paste-shelf");
        let effects = state.update(Message::Pasted(format!("  {LINK}\n")), 1_790_000_001);
        assert!(matches!(effects.as_slice(), [Effect::Resolve(_, Target::Set { id: 77, beatmap: Some(100), .. })]), "{effects:?}");
        let editor = match &state.screen {
            Screen::Editor(editor) => editor.clone(),
            Screen::Shelf | Screen::Open(_) => panic!("a pool was opened"),
        };
        assert_eq!(editor.panel, Panel::Add);
        assert_eq!(editor.query, LINK);
        assert!(matches!(state.finding, Some(Finding::Asking)));
    }

    #[test]
    fn pasting_a_set_in_an_existing_pool_reveals_the_difficulty_picker_from_every_source() {
        for source in [SourceTab::Search, SourceTab::Collections, SourceTab::Best, SourceTab::Suggest] {
            let mut state = state_with_songs(&format!("paste-editor-{source:?}"));
            state.update(Message::New, 100);
            state.update(Message::Rename("My pool".into()), 101);
            state.update(Message::Put("a1".into()), 102);
            let before = state.editing().unwrap().clone();
            state.update(Message::Source(source), 103);
            state.update(Message::AddPanel(false), 104);
            let effects = state.update(Message::Pasted("https://osu.ppy.sh/beatmapsets/77".into()), 105);
            assert!(effects.iter().any(|effect| matches!(effect, Effect::Resolve(_, Target::Set { id: 77, beatmap: None, .. }))));
            let Screen::Editor(editor) = &state.screen else { panic!("the same editor stays open") };
            assert_eq!(editor.id, before.id);
            assert_eq!(editor.panel, Panel::Add);
            assert_eq!(editor.source, SourceTab::Search);
            assert_eq!(state.editing().unwrap(), &before);
            state.update(Message::Resolved(state.resolve_request, Ok(found(&["b2", "c3"], None))), 106);
            let Some(Finding::Found(candidate)) = &state.finding else { panic!("difficulty selection is ready") };
            assert_eq!(candidate.choice, None);
            state.update(Message::Choose(1), 107);
            state.update(Message::Confirm, 108);
            let pool = state.editing().unwrap();
            assert_eq!(pool.id, before.id);
            assert_eq!(pool.name, before.name);
            assert_eq!(pool.slots.len(), before.slots.len());
            assert_eq!(pool.slots[0], before.slots[0]);
            assert_eq!(pool.slots[1].hash.as_deref(), Some("c3"));
        }
    }

    #[test]
    fn words_pasted_on_the_shelf_do_nothing_but_inside_a_pool_they_become_the_search() {
        let mut state = state_with_songs("paste-words");
        assert!(state.update(Message::Pasted("hello there".into()), 1_790_000_001).is_empty());
        assert_eq!(state.screen, Screen::Shelf);
        assert!(state.list.is_empty());
        open_new(&mut state);
        state.update(Message::Pasted("glass".into()), 1_790_000_002);
        let Screen::Editor(editor) = &state.screen else { panic!("still in the editor") };
        assert_eq!((editor.panel, editor.query.as_str()), (Panel::Add, "glass"));
        assert!(state.finding.is_none());
    }

    #[test]
    fn a_resolved_map_offers_the_automatic_slot_and_asks_for_its_cover() {
        let mut state = state_with_songs("resolved");
        open_new(&mut state);
        state.update(Message::Pasted(LINK.into()), 1_790_000_002);
        let effects = state.update(Message::Resolved(state.resolve_request, Ok(found(&["a1"], Some(0)))), 1_790_000_003);
        assert!(matches!(effects.as_slice(), [Effect::Cover(url, 77)] if url.ends_with("/77/covers/cover.jpg")));
        let Some(Finding::Found(candidate)) = &state.finding else { panic!("found") };
        assert_eq!((candidate.choice, candidate.place), (Some(0), Place::Slot(0)));
        state.update(Message::Cover(77, None), 1_790_000_004);
        assert!(matches!(state.finding, Some(Finding::Found(_))));
    }

    #[test]
    fn a_set_waits_for_a_choice_and_a_map_on_the_disk_is_placed_without_a_download() {
        let mut state = state_with_songs("choice");
        open_new(&mut state);
        state.update(Message::Pasted("https://osu.ppy.sh/beatmapsets/77".into()), 1_790_000_002);
        state.update(Message::Resolved(state.resolve_request, Ok(found(&["a1", "b2", "c3"], None))), 1_790_000_003);
        let Some(Finding::Found(candidate)) = &state.finding else { panic!("found") };
        assert_eq!(candidate.choice, None);
        assert!(state.update(Message::Confirm, 1_790_000_004).is_empty(), "nothing is placed before a choice");
        assert!(matches!(state.finding, Some(Finding::Found(_))), "the card stays");
        state.update(Message::Choose(1), 1_790_000_005);
        state.update(Message::Aim(Place::Slot(3)), 1_790_000_005);
        let effects = state.update(Message::Confirm, 1_790_000_006);
        assert!(effects.iter().all(|effect| !matches!(effect, Effect::Fetch(..))), "the map is already on the disk");
        assert!(state.finding.is_none());
        assert_eq!(state.editing().unwrap().slots[3].hash.as_deref(), Some("b2"));
    }

    #[test]
    fn a_map_that_is_not_on_the_disk_is_fetched_step_by_step_and_then_placed() {
        use crate::maps::Step;
        let mut state = state_with_songs("fetch");
        open_new(&mut state);
        state.update(Message::Pasted(LINK.into()), 1_790_000_002);
        state.update(Message::Resolved(state.resolve_request, Ok(found(&["zz9"], Some(0)))), 1_790_000_003);
        state.update(Message::Aim(Place::Slot(2)), 1_790_000_004);
        let effects = state.update(Message::Confirm, 1_790_000_005);
        assert!(matches!(effects.as_slice(), [Effect::Fetch(_, hash, _)] if hash == "zz9"));
        let Some(Finding::Fetching(fetching)) = &state.finding else { panic!("fetching") };
        assert_eq!((fetching.total, fetching.queue.len()), (1, 0));
        state.update(Message::Step(state.fetch_request, "zz9".into(), Step::Downloading { from: "osu.direct", done: 3_000_000, total: Some(5_000_000) }), 1_790_000_006);
        assert_eq!(stage_of(match &state.finding { Some(Finding::Fetching(f)) => f.step.as_ref(), _ => None }), 1);
        let map = map("Fetched Song", "Artist", "Normal");
        state.update(Message::Step(state.fetch_request, "zz9".into(), Step::Done(map)), 1_790_000_007);
        assert!(state.finding.is_none());
        assert_eq!(state.editing().unwrap().slots[2].hash.as_deref(), Some("zz9"));
        assert!(state.fetched.contains_key("zz9"));
        assert!(state.songs.as_ref().unwrap().contains_key("zz9"));
    }

    #[test]
    fn putting_all_places_what_is_on_the_disk_at_once_and_fetches_the_rest_into_the_next_slots() {
        use crate::maps::Step;
        let mut state = state_with_songs("all");
        open_new(&mut state);
        state.update(Message::Pasted("https://osu.ppy.sh/beatmapsets/77".into()), 1_790_000_002);
        state.update(Message::Resolved(state.resolve_request, Ok(found(&["a1", "zz9", "c3"], None))), 1_790_000_003);
        let effects = state.update(Message::ConfirmAll, 1_790_000_004);
        let fetches: Vec<&Effect> = effects.iter().filter(|effect| matches!(effect, Effect::Fetch(..))).collect();
        assert!(matches!(fetches.as_slice(), [Effect::Fetch(_, hash, _)] if hash == "zz9"), "the first is placed, the second is fetched: {effects:?}");
        assert!(effects.iter().any(|effect| matches!(effect, Effect::Measure(hash, _, _) if hash == "a1")), "the map on the disk is measured");
        assert_eq!(state.editing().unwrap().slots[0].hash.as_deref(), Some("a1"));
        let Some(Finding::Fetching(fetching)) = &state.finding else { panic!("fetching") };
        assert_eq!((fetching.total, fetching.queue.clone()), (3, vec!["c3".to_owned()]));
        state.update(Message::Step(state.fetch_request, "zz9".into(), Step::Done(map("Second", "Artist", "Hard"))), 1_790_000_005);
        let pool = state.editing().unwrap();
        assert_eq!((pool.slots[0].hash.as_deref(), pool.slots[1].hash.as_deref(), pool.slots[2].hash.as_deref()), (Some("a1"), Some("zz9"), Some("c3")));
        assert!(state.finding.is_none());
    }

    #[test]
    fn a_taiko_link_is_refused_and_a_not_a_map_link_is_told_under_the_field() {
        let mut state = state_with_songs("refuse");
        open_new(&mut state);
        let effects = state.update(Message::Query("https://osu.ppy.sh/beatmapsets/5#taiko/6".into()), 1_790_000_002);
        assert!(effects.is_empty());
        assert!(matches!(&state.finding, Some(Finding::Refused(mode)) if mode == "taiko"));
        state.update(Message::Dismiss, 1_790_000_003);
        assert!(state.finding.is_none());
        state.update(Message::Query("https://example.com/not-a-map".into()), 1_790_000_004);
        assert_eq!(state.notice, Some(Notice::NotALink), "a foreign address is told to be no map link");
        assert!(state.finding.is_none());
        state.update(Message::Query("https://osu.ppy.sh/users/2".into()), 1_790_000_005);
        assert_eq!(state.notice, Some(Notice::NotALink));
        state.update(Message::Query("glass".into()), 1_790_000_006);
        assert_eq!(state.notice, None);
    }

    #[test]
    fn typing_ordinary_words_clears_an_old_finding_and_a_silent_mirror_can_be_retried() {
        let mut state = state_with_songs("silent");
        open_new(&mut state);
        state.update(Message::Query(LINK.into()), 1_790_000_002);
        assert!(matches!(state.finding, Some(Finding::Asking)));
        state.update(Message::Resolved(state.resolve_request, Err(Why::Silent("down".into()))), 1_790_000_003);
        assert!(matches!(state.finding, Some(Finding::Silent(_))));
        let again = state.update(Message::Retry, 1_790_000_004);
        assert!(matches!(again.as_slice(), [Effect::Resolve(_, _)]));
        assert!(matches!(state.finding, Some(Finding::Asking)));
        state.update(Message::Resolved(state.resolve_request, Err(Why::Nowhere)), 1_790_000_005);
        assert!(matches!(state.finding, Some(Finding::Missing)));
        state.update(Message::Query("glass".into()), 1_790_000_006);
        assert!(state.finding.is_none());
    }

    #[test]
    fn an_answer_that_arrives_after_the_card_was_closed_is_ignored() {
        let mut state = state_with_songs("stale");
        open_new(&mut state);
        state.update(Message::Query(LINK.into()), 1_790_000_002);
        state.update(Message::Dismiss, 1_790_000_003);
        state.update(Message::Resolved(state.resolve_request, Ok(found(&["a1"], Some(0)))), 1_790_000_004);
        assert!(state.finding.is_none());
    }

    #[test]
    fn closing_a_download_stops_it_and_a_failed_or_empty_download_is_told() {
        use crate::maps::Step;
        let mut state = state_with_songs("stop");
        open_new(&mut state);
        state.update(Message::Query(LINK.into()), 1_790_000_002);
        state.update(Message::Resolved(state.resolve_request, Ok(found(&["zz9"], Some(0)))), 1_790_000_003);
        state.update(Message::Confirm, 1_790_000_004);
        let stop = match &state.finding {
            Some(Finding::Fetching(fetching)) => fetching.stop.clone(),
            _ => panic!("fetching"),
        };
        state.update(Message::Dismiss, 1_790_000_005);
        assert!(stop.load(std::sync::atomic::Ordering::SeqCst));
        assert!(state.finding.is_none());
        state.update(Message::Step(state.fetch_request, "zz9".into(), Step::Stopped), 1_790_000_006);
        assert!(state.finding.is_none(), "a late step changes nothing");

        let mut failing = state_with_songs("fail");
        open_new(&mut failing);
        failing.update(Message::Query(LINK.into()), 1_790_000_002);
        failing.update(Message::Resolved(failing.resolve_request, Ok(found(&["zz9"], Some(0)))), 1_790_000_003);
        failing.update(Message::Confirm, 1_790_000_004);
        failing.update(Message::Step(failing.fetch_request, "zz9".into(), Step::Nowhere), 1_790_000_005);
        assert!(matches!(failing.finding, Some(Finding::Missing)));
        failing.update(Message::Dismiss, 1_790_000_006);
        failing.update(Message::Query(LINK.into()), 1_790_000_007);
        failing.update(Message::Resolved(failing.resolve_request, Ok(found(&["zz9"], Some(0)))), 1_790_000_008);
        failing.update(Message::Confirm, 1_790_000_009);
        failing.update(Message::Step(failing.fetch_request, "zz9".into(), Step::Failed("no space".into())), 1_790_000_010);
        assert!(matches!(&failing.finding, Some(Finding::Silent(why)) if why == "no space"));
    }

    #[test]
    fn a_map_fetched_while_the_songs_are_still_being_read_survives_their_arrival() {
        use crate::maps::Step;
        let mut state = State::new(scratch("late"));
        state.loaded = true;
        state.update(Message::New, 1_790_000_001);
        state.update(Message::Query(LINK.into()), 1_790_000_002);
        state.update(Message::Resolved(state.resolve_request, Ok(found(&["zz9"], Some(0)))), 1_790_000_003);
        state.update(Message::Confirm, 1_790_000_004);
        let effects = state.update(Message::Step(state.fetch_request, "zz9".into(), Step::Done(map("Fetched", "A", "N"))), 1_790_000_005);
        assert!(effects.is_empty(), "no index yet, so nothing is measured yet");
        assert_eq!(state.editing().unwrap().slots[0].hash.as_deref(), Some("zz9"));
        let mut songs = HashMap::new();
        songs.insert("a1".to_owned(), map("Other", "A", "N"));
        let after = state.update(Message::Songs(Arc::new(songs)), 1_790_000_006);
        assert!(state.songs.as_ref().unwrap().contains_key("zz9"), "the fetched map is merged into the index");
        assert!(matches!(after.as_slice(), [Effect::Measure(hash, _, _)] if hash == "zz9"));
    }

    fn shared_pool(hashes: &[&str]) -> Pool {
        let mut pool = Pool::new(Frame::Duel, "Cup: round/1", 1_790_000_000);
        for (at, hash) in hashes.iter().enumerate() {
            pool.slots[at].hash = Some((*hash).to_owned());
            pool.slots[at].artist = "Artist".into();
            pool.slots[at].title = format!("Song {at}");
            pool.slots[at].version = "Normal".into();
        }
        pool
    }

    #[test]
    fn image_export_uses_a_snapshot_blocks_duplicates_and_unlocks_after_success_cancel_or_failure() {
        let mut state = pool_with_maps("image-export", &["a1", "b2"]);
        state.update(Message::Rename("First name".into()), 1_790_000_100);
        let effects = state.update(Message::SaveImage, 1_790_000_101);
        let (request, exported) = match effects.as_slice() {
            [Effect::SaveImage(request, pool)] => (*request, pool.clone()),
            _ => panic!("export the current pool"),
        };
        assert_eq!(state.exporting, Some(request));
        assert!(state.update(Message::SaveImage, 1_790_000_102).is_empty());
        state.update(Message::Rename("Changed later".into()), 1_790_000_103);
        assert_eq!(exported.name, "First name");
        assert_eq!(exported.filled(), 2);
        assert!(state.update(Message::ImageSaved(request + 1, Ok(None)), 1_790_000_104).is_empty());
        assert_eq!(state.exporting, Some(request));
        let effects = state.update(Message::ImageSaved(request, Ok(Some(PathBuf::from("pool.png")))), 1_790_000_105);
        assert!(matches!(effects.as_slice(), [Effect::Say("pool-image-saved")]));
        assert!(state.exporting.is_none());
        state.update(Message::SaveImage, 1_790_000_106);
        let cancelled = state.exporting.unwrap();
        assert_ne!(cancelled, request);
        assert!(state.update(Message::ImageSaved(request, Ok(None)), 1_790_000_107).is_empty());
        assert_eq!(state.exporting, Some(cancelled));
        assert!(state.update(Message::ImageSaved(cancelled, Ok(None)), 1_790_000_108).is_empty());
        assert!(state.exporting.is_none());
        state.update(Message::SaveImage, 1_790_000_109);
        let failed = state.exporting.unwrap();
        let effects = state.update(Message::ImageSaved(failed, Err("disk full".into())), 1_790_000_110);
        assert!(matches!(effects.as_slice(), [Effect::Say("pool-save-failed")]));
        assert!(state.exporting.is_none());
        state.update(Message::Back, 1_790_000_111);
        assert!(state.update(Message::SaveImage, 1_790_000_112).is_empty());
    }

    #[test]
    fn sharing_offers_the_file_the_line_and_the_hash_and_closes() {
        let mut state = state_with_songs("share");
        open_new(&mut state);
        state.update(Message::Put("a1".into()), 1_790_000_002);
        state.update(Message::Rename("Cup: round/1".into()), 1_790_000_003);
        state.update(Message::Share(true), 1_790_000_004);
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.share));
        let pool = state.editing().unwrap().clone();
        let copied = state.update(Message::CopyText, 1_790_000_005);
        assert!(matches!(copied.as_slice(), [Effect::Copy(text, "pool-copied")] if text == &pool_share::to_text(&pool)));
        let hashed = state.update(Message::CopyHash, 1_790_000_006);
        assert!(matches!(hashed.as_slice(), [Effect::Copy(text, "pool-copied-hash")] if text == &pool.fingerprint() && text.len() == 7));
        let saved = state.update(Message::SaveFile, 1_790_000_007);
        assert!(matches!(saved.as_slice(), [Effect::SaveFile(name, bytes)] if name == "Cup round 1.pool" && bytes == &pool_share::to_file(&pool)), "{saved:?}");
        state.update(Message::Share(false), 1_790_000_008);
        assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.share));
        assert_eq!(file_name_of(&Pool::new(Frame::Free, "", 1)), "pool.pool");
    }

    #[test]
    fn a_pool_string_pasted_on_the_shelf_opens_the_pool_and_reads_the_songs() {
        let mut state = State::new(scratch("open-string"));
        state.loaded = true;
        let text = pool_share::to_text(&shared_pool(&["a1", "zz9"]));
        let effects = state.update(Message::Pasted(text), 1_790_000_001);
        assert!(matches!(effects.as_slice(), [Effect::ReadSongs]), "{effects:?}");
        let Screen::Open(opening) = &state.screen else { panic!("the open screen") };
        assert_eq!(opening.pool.name, "Cup: round/1");
        assert!(state.list.is_empty(), "nothing is saved before the person says so");
    }

    #[test]
    fn a_cut_string_or_a_foreign_file_is_told_on_the_shelf() {
        let mut state = state_with_songs("refused");
        let text = pool_share::to_text(&shared_pool(&["a1"]));
        state.update(Message::Pasted(text[..text.len() / 2].to_owned()), 1_790_000_001);
        assert_eq!(state.refused, Some(pool_share::Refused::Damaged));
        assert_eq!(state.screen, Screen::Shelf);
        state.update(Message::Imported(Err(pool_share::Refused::NotAPool)), 1_790_000_002);
        assert_eq!(state.refused, Some(pool_share::Refused::NotAPool));
        state.update(Message::Imported(Ok(shared_pool(&["a1"]))), 1_790_000_003);
        assert!(state.refused.is_none());
        assert!(matches!(state.screen, Screen::Open(_)));
    }

    #[test]
    fn the_missing_maps_are_downloaded_in_turn_and_measured_and_the_pool_is_kept() {
        use crate::maps::Step;
        let mut state = state_with_songs("open-flow");
        state.update(Message::Imported(Ok(shared_pool(&["a1", "zz9", "yy8", "a1"]))), 1_790_000_001);
        let Screen::Open(opening) = state.screen.clone() else { panic!("open") };
        assert_eq!(state.missing(&opening.pool), vec!["zz9".to_owned(), "yy8".to_owned()], "each missing map is listed once");
        let effects = state.update(Message::GetMissing, 1_790_000_002);
        assert!(matches!(effects.as_slice(), [Effect::Fetch(_, hash, _)] if hash == "zz9"));
        assert!(state.update(Message::GetMissing, 1_790_000_003).is_empty(), "a second press does not start a second run");
        let next = state.update(Message::Step(state.fetch_request, "zz9".into(), Step::Done(map("Fetched One", "A", "N"))), 1_790_000_004);
        assert!(next.iter().any(|effect| matches!(effect, Effect::Measure(hash, _, _) if hash == "zz9")), "the arrived map is measured");
        assert!(next.iter().any(|effect| matches!(effect, Effect::Fetch(_, hash, _) if hash == "yy8")), "the next one is asked for");
        let after = state.update(Message::Step(state.fetch_request, "yy8".into(), Step::Nowhere), 1_790_000_005);
        assert!(after.iter().all(|effect| !matches!(effect, Effect::Fetch(..))));
        let Screen::Open(opening) = state.screen.clone() else { panic!("open") };
        assert_eq!((opening.running, opening.lost), (false, 1));
        state.update(Message::Measured("zz9".into(), Mod::Nm, Ok(measure(5.5))), 1_790_000_006);
        let Screen::Open(opening) = state.screen.clone() else { panic!("open") };
        assert_eq!(opening.pool.slots[1].measure, Some(measure(5.5)));

        let kept = state.update(Message::Keep, 1_790_000_007);
        assert!(matches!(&state.screen, Screen::Editor(_)));
        assert_eq!(state.list.len(), 1);
        assert_eq!(state.list[0].made_at, 1_790_000_007);
        assert_eq!(pools::load_all(&state.dir).len(), 1);
        assert!(kept.iter().all(|effect| !matches!(effect, Effect::Fetch(..))));
    }

    #[test]
    fn leaving_the_open_screen_stops_the_downloads_and_forgets_the_draft() {
        let mut state = state_with_songs("open-leave");
        state.update(Message::Imported(Ok(shared_pool(&["zz9"]))), 1_790_000_001);
        state.update(Message::GetMissing, 1_790_000_002);
        let stop = match &state.screen {
            Screen::Open(opening) => opening.stop.clone(),
            _ => panic!("open"),
        };
        state.update(Message::Back, 1_790_000_003);
        assert!(stop.load(std::sync::atomic::Ordering::SeqCst));
        assert_eq!(state.screen, Screen::Shelf);
        assert!(state.list.is_empty());
        assert!(pools::load_all(&state.dir).is_empty());
    }

    #[test]
    fn nothing_is_asked_for_while_the_songs_are_still_being_read() {
        let mut state = State::new(scratch("open-wait"));
        state.loaded = true;
        state.update(Message::Imported(Ok(shared_pool(&["zz9"]))), 1_790_000_001);
        assert!(state.update(Message::GetMissing, 1_790_000_002).is_empty());
        let Screen::Open(opening) = &state.screen else { panic!("open") };
        assert!(state.missing(&opening.pool).is_empty(), "what is missing is not known yet");
    }

    fn pool_with_maps(name: &str, hashes: &[&str]) -> State {
        let mut state = state_with_songs(name);
        open_new(&mut state);
        for (at, hash) in hashes.iter().enumerate() {
            state.update(Message::Select(None), 1_790_000_002 + at as i64);
            state.update(Message::Put((*hash).into()), 1_790_000_020 + at as i64);
        }
        state
    }

    fn hashes_of(state: &State) -> Vec<Option<String>> {
        state.editing().unwrap().slots.iter().map(|slot| slot.hash.clone()).collect()
    }

    #[test]
    fn grouping_only_changes_the_view_and_keeps_selection_and_saved_order() {
        let mut state = pool_with_maps("group-view", &["a1", "b2", "c3"]);
        state.update(Message::SetMod(1, Mod::Hr), 1_790_000_100);
        state.update(Message::Select(Some(1)), 1_790_000_101);
        let pool = state.editing().unwrap().clone();
        let undo = state.undo.len();
        state.update(Message::Grouped(true), 1_790_000_102);
        let groups = slot_groups(state.editing().unwrap());
        assert_eq!(groups[0], (Mod::Nm, vec![0]));
        assert!(groups.iter().find(|(mods, _)| *mods == Mod::Hr).unwrap().1.contains(&1));
        assert_eq!(state.editing().unwrap().slots, pool.slots);
        assert_eq!(pools::load_all(&state.dir)[0].slots, pool.slots);
        assert_eq!(state.undo.len(), undo);
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.grouped && editor.selected == Some(1)));
        state.update(Message::Grouped(false), 1_790_000_103);
        assert_eq!(state.editing().unwrap().slots, pool.slots);
    }

    #[test]
    fn replacing_from_a_tile_targets_that_tile_and_undo_restores_its_map() {
        let mut state = pool_with_maps("inline-replace", &["a1", "b2"]);
        state.update(Message::Note(1, "Keep this role".into()), 1_790_000_100);
        let before = state.editing().unwrap().slots.clone();
        state.update(Message::Select(Some(0)), 1_790_000_101);
        state.update(Message::ReplaceAt(1), 1_790_000_102);
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.selected == Some(1) && editor.replace && editor.panel == Panel::Add));
        state.update(Message::Put("c3".into()), 1_790_000_103);
        assert_eq!(state.editing().unwrap().slots[0], before[0]);
        assert_eq!(state.editing().unwrap().slots[1].hash.as_deref(), Some("c3"));
        assert_eq!(state.editing().unwrap().slots[1].mods, before[1].mods);
        state.update(Message::Undo, 1_790_000_104);
        assert_eq!(state.editing().unwrap().slots, before);
        state.update(Message::ReplaceAt(usize::MAX), 1_790_000_105);
        assert_eq!(state.editing().unwrap().slots, before);
    }

    #[test]
    fn holding_the_category_header_does_not_open_details_or_start_a_drag() {
        let mut state = pool_with_maps("inline-controls", &["a1", "b2", "c3"]);
        state.editor_mut().unwrap().panel = Panel::Closed;
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1000.0, 900.0), view(&state, &words, &thumbs, 1000.0, 1.0, &Clocks::settled()));
        assert!(screen.find(words.t("pool-replace-short")).is_err());
        assert!(screen.find(words.t("pool-remove-short")).is_err());
        let number = screen.find("1").unwrap().visible_bounds().unwrap();
        let category = number.center() + iced::Vector::new(56.0, 0.0);
        screen.point_at(category);
        let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left))]);
        let position = category + iced::Vector::new(0.0, 100.0);
        screen.point_at(position);
        let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved { position })]);
        let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left))]);
        assert!(screen.into_messages().all(|message| !matches!(message, Message::Move(..) | Message::Select(..))));
    }

    #[test]
    fn the_category_menu_changes_only_its_tile_and_replaces_the_old_measure() {
        let mut state = pool_with_maps("inline-category", &["a1", "b2"]);
        state.editor_mut().unwrap().panel = Panel::Closed;
        state.update(Message::Measured("a1".into(), Mod::Nm, Ok(measure(4.0))), 1_790_000_100);
        let before = state.editing().unwrap().slots.clone();
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1000.0, 900.0), view(&state, &words, &thumbs, 1000.0, 1.0, &Clocks::settled()));
        let number = screen.find("1").unwrap().visible_bounds().unwrap();
        let category = number.center() + iced::Vector::new(56.0, 0.0);
        screen.point_at(category);
        let _ = screen.simulate(iced_test::simulator::click());
        if let Some(dir) = std::env::var_os("DOSSIER_POOL_MENU_REVIEW").map(PathBuf::from) {
            let shot = screen.snapshot(&theme::theme()).unwrap();
            crate::gallery::write_snapshot(&shot, &dir.join("pool-category-menu")).unwrap();
        }
        let position = category + iced::Vector::new(0.0, 100.0);
        screen.point_at(position);
        let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved { position })]);
        let _ = screen.simulate(iced_test::simulator::click());
        let messages: Vec<_> = screen.into_messages().collect();
        assert!(matches!(messages.as_slice(), [Message::SetMod(0, Mod::Hr)]), "{messages:?}");
        for message in messages { state.update(message, 1_790_000_101); }
        assert_eq!(state.editing().unwrap().slots[0].mods, Mod::Hr);
        assert!(state.editing().unwrap().slots[0].measure.is_none());
        assert_eq!(&state.editing().unwrap().slots[1..], &before[1..]);
        state.update(Message::Measured("a1".into(), Mod::Hr, Ok(measure(6.0))), 1_790_000_102);
        assert_eq!(state.editing().unwrap().slots[0].measure.unwrap().stars, 6.0);
        state.update(Message::Undo, 1_790_000_103);
        assert_eq!(state.editing().unwrap().slots, before);
    }

    #[test]
    fn dragging_to_the_end_of_a_category_keeps_the_next_category_in_place() {
        let mut state = pool_with_maps("group-drag", &["a1", "b2", "c3"]);
        state.editor_mut().unwrap().panel = Panel::Closed;
        state.update(Message::Grouped(true), 1_790_000_100);
        let before = state.editing().unwrap().slots.clone();
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1000.0, 900.0), view(&state, &words, &thumbs, 1000.0, 1.0, &Clocks::settled()));
        let first = screen.find("Glass Orchard").unwrap().visible_bounds().unwrap();
        let second = screen.find("Salt and Static").unwrap().visible_bounds().unwrap();
        screen.point_at(first.center());
        let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left))]);
        let position = iced::Point::new(first.center_x(), second.y + ROW_HIGH + 50.0);
        screen.point_at(position);
        let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved { position })]);
        let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left))]);
        let messages: Vec<_> = screen.into_messages().collect();
        let keys = slot_keys(state.editing().unwrap());
        assert!(matches!(messages.as_slice(), [Message::Move(_, _, key, Some(next))] if *key == keys[0] && *next == keys[2]), "{messages:?}");
        for message in messages { state.update(message, 1_790_000_101); }
        assert_eq!(&state.editing().unwrap().slots[..2], &[before[1].clone(), before[0].clone()]);
        assert_eq!(&state.editing().unwrap().slots[2..], &before[2..]);
        state.update(Message::Undo, 1_790_000_102);
        assert_eq!(state.editing().unwrap().slots, before);
    }

    fn move_slot(state: &mut State, from: usize, before: Option<usize>, now: i64) {
        let pool = state.editing().unwrap();
        let keys = slot_keys(pool);
        let message = Message::Move(pool.id.clone(), keys.clone(), keys[from], before.map(|at| keys[at]));
        state.update(message, now);
    }

    #[test]
    fn dragging_keeps_the_entire_slot_selection_and_marks_and_undo_restores_the_order() {
        let mut state = pool_with_maps("drag", &["a1", "b2", "c3"]);
        state.update(Message::Note(2, "Keep this chart".into()), 1_790_000_090);
        state.update(Message::Measured("c3".into(), Mod::Hd, Ok(measure(5.5))), 1_790_000_091);
        state.update(Message::Select(Some(1)), 1_790_000_092);
        let before = state.editing().unwrap().slots.clone();
        let count = state.undo.len();
        move_slot(&mut state, 2, Some(0), 1_790_000_100);
        assert_eq!(&hashes_of(&state)[..3], [Some("c3".into()), Some("a1".into()), Some("b2".into())]);
        assert_eq!(state.editing().unwrap().slots[0], before[2], "mods, note and measures travel together");
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.selected == Some(2)));
        assert_eq!(state.undo.len(), count + 1);
        assert_eq!(pools::load_all(&state.dir)[0].slots, state.editing().unwrap().slots);
        state.update(Message::Undo, 1_790_000_101);
        assert_eq!(state.editing().unwrap().slots, before);
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.selected == Some(1)));
        state.update(Message::Choosing(true), 1_790_000_102);
        state.update(Message::Mark(0), 1_790_000_103);
        state.update(Message::Mark(2), 1_790_000_104);
        move_slot(&mut state, 0, None, 1_790_000_105);
        assert_eq!(state.marked_slots(), vec![1, 6]);
        state.update(Message::Undo, 1_790_000_106);
        assert_eq!(state.marked_slots(), vec![0, 2]);
    }

    #[test]
    fn a_drag_with_an_old_order_or_a_different_pool_is_ignored_and_noop_adds_no_undo() {
        let mut state = pool_with_maps("drag-stale", &["a1", "b2", "c3"]);
        let pool = state.editing().unwrap();
        let keys = slot_keys(pool);
        let message = Message::Move(pool.id.clone(), keys.clone(), keys[2], Some(keys[0]));
        let count = state.undo.len();
        move_slot(&mut state, 0, Some(1), 1_790_000_100);
        move_slot(&mut state, 0, Some(0), 1_790_000_101);
        assert_eq!(state.undo.len(), count);
        state.update(Message::SetMod(0, Mod::Dt), 1_790_000_102);
        let before = state.editing().unwrap().clone();
        state.update(message.clone(), 1_790_000_103);
        assert_eq!(state.editing().unwrap(), &before);
        state.update(Message::New, 1_790_000_104);
        let next = state.editing().unwrap().clone();
        state.update(message, 1_790_000_105);
        assert_eq!(state.editing().unwrap(), &next);
    }

    #[test]
    fn an_empty_slot_moves_with_its_mod_and_an_inflight_file_import_follows_it() {
        let mut state = pool_with_maps("drag-import", &["a1"]);
        state.update(Message::Select(Some(3)), 1_790_000_100);
        state.update(Message::Dropped(PathBuf::from("first.osu")), 1_790_000_101);
        let request = state.importing.as_ref().unwrap().request;
        move_slot(&mut state, 3, Some(0), 1_790_000_102);
        assert_eq!(state.editing().unwrap().slots[0].mods, Mod::Hr);
        assert_eq!(state.importing.as_ref().unwrap().place, Place::Slot(0));
        state.update(Message::MapFile(request, Ok(("e5".into(), map("First", "A", "Hard")))), 1_790_000_103);
        assert_eq!(state.editing().unwrap().slots[0].hash.as_deref(), Some("e5"));
        assert_eq!(state.editing().unwrap().slots[0].mods, Mod::Hr);
    }

    #[test]
    fn moving_slots_does_not_revive_an_import_invalidated_by_an_earlier_edit() {
        let mut state = pool_with_maps("drag-stale-import", &["a1"]);
        state.update(Message::Select(Some(3)), 1_790_000_100);
        state.update(Message::Dropped(PathBuf::from("first.osu")), 1_790_000_101);
        let request = state.importing.as_ref().unwrap().request;
        state.update(Message::SetMod(3, Mod::Dt), 1_790_000_102);
        move_slot(&mut state, 3, Some(0), 1_790_000_103);
        state.update(Message::MapFile(request, Ok(("e5".into(), map("First", "A", "Hard")))), 1_790_000_104);
        assert_eq!(state.editing().unwrap().filled(), 1);
    }

    #[test]
    fn downloaded_maps_and_replacements_follow_their_target_when_it_moves() {
        let mut state = pool_with_maps("drag-fetch", &["a1", "b2"]);
        state.update(Message::Select(Some(3)), 1_790_000_100);
        state.update(Message::Pasted(LINK.into()), 1_790_000_101);
        state.update(Message::Resolved(state.resolve_request, Ok(found(&["zz9"], Some(0)))), 1_790_000_102);
        state.update(Message::Aim(Place::Slot(3)), 1_790_000_103);
        state.update(Message::Confirm, 1_790_000_104);
        move_slot(&mut state, 3, Some(0), 1_790_000_105);
        assert!(matches!(&state.finding, Some(Finding::Fetching(fetching)) if fetching.place == Some(Place::Slot(0))));
        state.update(Message::Step(state.fetch_request, "zz9".into(), crate::maps::Step::Done(map("Fetched", "A", "Hard"))), 1_790_000_106);
        assert_eq!(state.editing().unwrap().slots[0].hash.as_deref(), Some("zz9"));
        state.update(Message::Pasted(LINK.into()), 1_790_000_107);
        state.update(Message::Resolved(state.resolve_request, Ok(found(&["c3"], Some(0)))), 1_790_000_108);
        state.update(Message::Aim(Place::Replace(2)), 1_790_000_109);
        move_slot(&mut state, 2, Some(0), 1_790_000_110);
        assert!(matches!(&state.finding, Some(Finding::Found(candidate)) if candidate.place == Place::Replace(0)));
        state.update(Message::Confirm, 1_790_000_111);
        assert_eq!(state.editing().unwrap().slots[0].hash.as_deref(), Some("c3"));
    }

    #[test]
    fn slot_keys_stay_unique_for_repeated_empty_slots_and_ignore_measurement_updates() {
        let mut pool = Pool::new(Frame::Duel, "", 1_790_000_000);
        let keys = slot_keys(&pool);
        assert_eq!(keys.iter().collect::<HashSet<_>>().len(), pool.slots.len());
        pool.slots[0].measure = Some(measure(5.0));
        assert_eq!(slot_keys(&pool), keys);
        pool.slots.swap(3, 4);
        let after = slot_keys(&pool);
        assert_eq!(after[4], keys[3]);
        assert_eq!(after[3], keys[4]);
    }

    #[test]
    fn editor_tiles_use_two_columns_only_when_the_window_has_room() {
        let mut state = pool_with_maps("responsive-view", &["a1", "b2", "c3"]);
        state.editor_mut().unwrap().panel = Panel::Closed;
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        for (width, columns) in [(980.0, 1), (1280.0, 2), (1920.0, 3)] {
            let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(width, 900.0), view(&state, &words, &thumbs, width, 1.0, &Clocks::settled()));
            let first = screen.find("Glass Orchard").unwrap().visible_bounds().unwrap();
            let second = screen.find("Salt and Static").unwrap().visible_bounds().unwrap();
            if columns == 1 { assert!(second.y > first.y); }
            else { assert!((first.y - second.y).abs() < 1.0 && second.x > first.x); }
            assert!(first.width > 100.0 && second.width > 100.0);
        }
    }

    #[test]
    fn adding_a_map_in_a_small_window_keeps_the_source_buttons_visible() {
        let mut state = pool_with_maps("add-overlay-view", &["a1", "b2", "c3"]);
        state.update(Message::AddPanel(true), 1_790_000_100);
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(980.0, 600.0), view(&state, &words, &thumbs, 980.0, 1.0, &Clocks::settled()));
        for key in ["pool-tab-search", "pool-tab-collections", "pool-tab-best", "pool-tab-suggest"] {
            let bounds = screen.find(words.t(key)).unwrap().visible_bounds().unwrap();
            assert!(bounds.x > 190.0 && bounds.x + bounds.width < 790.0 && bounds.y > 16.0 && bounds.y + bounds.height < 600.0, "{key}: {bounds:?}");
        }
    }

    #[test]
    fn empty_search_stays_compact_and_the_panel_blocks_background_clicks_at_every_width() {
        let mut state = pool_with_maps("compact-add-panel", &["a1", "b2"]);
        state.update(Message::AddPanel(true), 101);
        state.update(Message::Query("nothing matches this".into()), 102);
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        let Screen::Editor(editor) = &state.screen else { panic!("editor") };
        let mut panel = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(PANEL_WIDE, 800.0), iced::widget::column![add_panel(&state, editor, state.editing().unwrap(), &words, &thumbs), text("end-of-panel")]);
        let bottom = panel.find("end-of-panel").unwrap().visible_bounds().unwrap();
        assert!(bottom.y < 300.0, "{bottom:?}");
        for width in [980.0, 1280.0, 1920.0] {
            let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(width, 800.0), view(&state, &words, &thumbs, width, 1.0, &Clocks::settled()));
            assert!(screen.find(words.t("pool-grouped")).is_err());
            screen.point_at(iced::Point::new(180.0, 300.0));
            let _ = screen.simulate(iced_test::simulator::click());
            let messages: Vec<_> = screen.into_messages().collect();
            assert!(matches!(messages.as_slice(), [Message::Select(None)]), "{width}: {messages:?}");
        }
    }

    #[test]
    fn dragging_in_the_editor_publishes_a_move_without_opening_the_slot() {
        let mut state = pool_with_maps("drag-view", &["a1", "b2", "c3"]);
        state.editor_mut().unwrap().panel = Panel::Closed;
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1000.0, 900.0), view(&state, &words, &thumbs, 1000.0, 1.0, &Clocks::settled()));
        let first = screen.find("Glass Orchard").unwrap().visible_bounds().unwrap();
        let third = screen.find("Ninth Window").unwrap().visible_bounds().unwrap();
        screen.point_at(first.center());
        let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left))]);
        let position = iced::Point::new(first.center_x(), third.y + ROW_HIGH + 20.0);
        screen.point_at(position);
        let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved { position })]);
        if let Some(dir) = std::env::var_os("DOSSIER_POOL_DRAG_REVIEW").map(PathBuf::from) {
            let mut now = std::time::Instant::now();
            for _ in 0..12 {
                now += std::time::Duration::from_millis(16);
                let _ = screen.simulate([iced::Event::Window(iced::window::Event::RedrawRequested(now))]);
            }
            let shot = screen.snapshot(&theme::theme()).unwrap();
            crate::gallery::write_snapshot(&shot, &dir.join("pool-dragging")).unwrap();
        }
        let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left))]);
        let keys = slot_keys(state.editing().unwrap());
        let messages = screen.into_messages().collect::<Vec<_>>();
        assert!(matches!(messages.as_slice(), [Message::Move(_, _, key, Some(before))] if *key == keys[0] && *before == keys[3]), "{messages:?}");
    }

    #[test]
    fn carrying_a_slot_to_the_page_edge_scrolls_and_escape_stops_it() {
        let mut state = pool_with_maps("drag-edge", &["a1", "b2", "c3", "d4"]);
        state.editor_mut().unwrap().panel = Panel::Closed;
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1000.0, 480.0), view(&state, &words, &thumbs, 1000.0, 1.0, &Clocks::settled()));
        let first = screen.find("Glass Orchard").unwrap().visible_bounds().unwrap();
        screen.point_at(first.center());
        let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left))]);
        let position = iced::Point::new(first.center_x(), 470.0);
        screen.point_at(position);
        let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::CursorMoved { position })]);
        let mut now = std::time::Instant::now();
        for _ in 0..40 {
            now += std::time::Duration::from_millis(16);
            let _ = screen.simulate([iced::Event::Window(iced::window::Event::RedrawRequested(now))]);
        }
        assert!(screen.find("Add map").unwrap().visible_bounds().is_none(), "a carried slot must scroll the header off screen");
        screen.tap_key(iced::keyboard::key::Named::Escape);
        for _ in 0..40 {
            now += std::time::Duration::from_millis(16);
            let _ = screen.simulate([iced::Event::Window(iced::window::Event::RedrawRequested(now))]);
        }
        assert!(screen.into_messages().all(|message| !matches!(message, Message::Move(..) | Message::Select(..))));
    }

    #[test]
    fn marks_toggle_and_leaving_the_choosing_mode_clears_them() {
        let mut state = pool_with_maps("marks", &["a1", "b2", "c3"]);
        state.update(Message::Choosing(true), 1_790_000_100);
        state.update(Message::Mark(0), 1_790_000_101);
        state.update(Message::Mark(2), 1_790_000_102);
        state.update(Message::Mark(0), 1_790_000_103);
        assert_eq!(state.marked_slots(), vec![2]);
        state.update(Message::Choosing(false), 1_790_000_104);
        assert!(state.marked_slots().is_empty());
        assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.choosing));
    }

    #[test]
    fn a_bulk_mod_change_reaches_every_marked_slot_and_can_be_undone() {
        let mut state = pool_with_maps("bulk-mod", &["a1", "b2", "c3"]);
        for (at, hash) in ["a1", "b2", "c3"].into_iter().enumerate() {
            state.update(Message::Measured(hash.into(), Mod::Nm, Ok(measure(4.0 + at as f64))), 1_790_000_050);
        }
        state.update(Message::Choosing(true), 1_790_000_100);
        state.update(Message::Mark(0), 1_790_000_101);
        state.update(Message::Mark(1), 1_790_000_102);
        state.update(Message::Bulk(Some(Bulk::Mod)), 1_790_000_103);
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.bulk == Some(Bulk::Mod)));
        let effects = state.update(Message::BulkMod(Mod::Dt), 1_790_000_104);
        let pool = state.editing().unwrap();
        assert_eq!((pool.slots[0].mods, pool.slots[1].mods), (Mod::Dt, Mod::Dt));
        assert_ne!(pool.slots[2].mods, Mod::Dt, "an unmarked slot keeps its mod");
        assert!(pool.slots[0].measure.is_none() && pool.slots[1].measure.is_none());
        assert_eq!(effects.iter().filter(|effect| matches!(effect, Effect::Measure(_, _, Mod::Dt))).count(), 2);
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.bulk.is_none()));
        state.update(Message::Undo, 1_790_000_105);
        let pool = state.editing().unwrap();
        assert_eq!((pool.slots[0].mods, pool.slots[1].mods), (Mod::Nm, Mod::Nm));
        assert!(pool.slots[0].measure.is_some(), "the old numbers come back with the slot");
    }

    #[test]
    fn marked_slots_move_as_a_block_and_stop_at_the_ends() {
        assert_eq!(shifted(&[1, 2], 5, false), vec![0, 1]);
        assert_eq!(shifted(&[0, 1], 5, false), vec![0, 1], "the top is a wall");
        assert_eq!(shifted(&[2, 4], 5, true), vec![3, 4], "a block at the bottom stays and the other moves up to it");
        assert_eq!(shifted(&[3, 4], 5, true), vec![3, 4]);
        let mut state = pool_with_maps("shift", &["a1", "b2", "c3", "d4"]);
        state.update(Message::Choosing(true), 1_790_000_100);
        state.update(Message::Mark(1), 1_790_000_101);
        state.update(Message::Mark(2), 1_790_000_102);
        state.update(Message::Shift(true), 1_790_000_103);
        assert_eq!(&hashes_of(&state)[..4], [Some("a1".to_owned()), Some("d4".to_owned()), Some("b2".to_owned()), Some("c3".to_owned())]);
        assert_eq!(state.marked_slots(), vec![2, 3], "the marks follow the maps");
        state.update(Message::Shift(false), 1_790_000_104);
        assert_eq!(&hashes_of(&state)[..4], [Some("a1".to_owned()), Some("b2".to_owned()), Some("c3".to_owned()), Some("d4".to_owned())]);
        state.update(Message::Undo, 1_790_000_105);
        assert_eq!(&hashes_of(&state)[..4], [Some("a1".to_owned()), Some("d4".to_owned()), Some("b2".to_owned()), Some("c3".to_owned())], "undo steps back one move at a time");
        assert_eq!(pools::load_all(&state.dir)[0].slots[1].hash.as_deref(), Some("d4"));
    }

    #[test]
    fn removing_the_marked_keeps_frame_slots_and_drops_free_ones_and_undo_brings_them_back() {
        let mut state = pool_with_maps("remove", &["a1", "b2", "c3"]);
        state.update(Message::Choosing(true), 1_790_000_100);
        state.update(Message::Mark(0), 1_790_000_101);
        state.update(Message::Mark(2), 1_790_000_102);
        state.update(Message::RemoveMarked, 1_790_000_103);
        let pool = state.editing().unwrap();
        assert_eq!(pool.slots.len(), 7);
        assert_eq!((pool.slots[0].hash.clone(), pool.slots[1].hash.clone(), pool.slots[2].hash.clone()), (None, Some("b2".to_owned()), None));
        assert!(state.marked_slots().is_empty());
        state.update(Message::Undo, 1_790_000_104);
        assert_eq!(state.editing().unwrap().filled(), 3);

        let mut free = state_with_songs("remove-free");
        open_new(&mut free);
        free.update(Message::UseFrame(Frame::Free), 1_790_000_002);
        for hash in ["a1", "b2", "c3"] {
            free.update(Message::Put(hash.into()), 1_790_000_003);
            free.update(Message::Select(None), 1_790_000_004);
        }
        free.update(Message::Choosing(true), 1_790_000_100);
        free.update(Message::Mark(1), 1_790_000_101);
        free.update(Message::RemoveMarked, 1_790_000_102);
        assert_eq!(hashes_of(&free), vec![Some("a1".to_owned()), Some("c3".to_owned())]);
    }

    #[test]
    fn undo_steps_back_through_a_put_and_does_nothing_when_there_is_nothing_to_undo() {
        let mut state = pool_with_maps("undo", &["a1", "b2"]);
        assert_eq!(state.editing().unwrap().filled(), 2);
        state.update(Message::Undo, 1_790_000_100);
        assert_eq!(state.editing().unwrap().filled(), 1);
        state.update(Message::Undo, 1_790_000_101);
        assert_eq!(state.editing().unwrap().filled(), 0);
        let before = state.editing().unwrap().clone();
        state.update(Message::Undo, 1_790_000_102);
        assert_eq!(state.editing().unwrap(), &before);
    }

    #[test]
    fn deleting_a_pool_asks_first_removes_the_file_and_returns_to_the_shelf() {
        let mut state = pool_with_maps("delete", &["a1"]);
        let id = state.editing().unwrap().id.clone();
        state.update(Message::AskDelete(true), 1_790_000_100);
        assert!(matches!(&state.screen, Screen::Editor(editor) if editor.asking_delete));
        state.update(Message::AskDelete(false), 1_790_000_101);
        assert!(matches!(&state.screen, Screen::Editor(editor) if !editor.asking_delete));
        assert!(state.dir.join(format!("{id}.pool")).exists());
        state.update(Message::AskDelete(true), 1_790_000_102);
        state.update(Message::DeletePool, 1_790_000_103);
        assert_eq!(state.screen, Screen::Shelf);
        assert!(state.list.is_empty());
        assert!(!state.dir.join(format!("{id}.pool")).exists());
        assert!(state.undo.iter().all(|(pool, _)| *pool != id), "nothing of a deleted pool stays in the undo steps");
    }

    fn published(id: &str) -> crate::bot::Publication {
        crate::bot::Publication { code: String::new(), id: id.to_owned(), kind: "pool".to_owned(), local_id: String::new(), revision: 1, name: id.to_owned(), content: serde_json::json!({}), mine: false }
    }

    #[test]
    fn the_first_look_is_settled_and_a_new_screen_a_panel_and_a_new_publication_each_start_their_own_clock() {
        let mut state = State::new(scratch("clocks"));
        state.publications.push(published("old"));
        let mut marks = Marks::default();
        let start = Instant::now();
        assert!(marks.clocks(start).screen >= SETTLED - 0.01, "before anything is seen nothing is young");
        marks.observe(&state, start);
        let first = marks.clocks(start);
        assert!(first.screen >= SETTLED - 0.01 && first.panel >= SETTLED - 0.01, "what is already there does not animate");
        assert!(first.of("old") >= SETTLED - 0.01);
        assert!(!marks.animating(start), "and does not ask for frames");
        marks.observe(&state, start + Duration::from_secs(5));
        assert!(marks.clocks(start + Duration::from_secs(5)).screen >= SETTLED - 0.01, "nothing restarts while nothing changes");

        let _ = state.update(Message::New, 1);
        assert!(matches!(state.screen, Screen::Editor(_)));
        let at = start + Duration::from_secs(10);
        marks.observe(&state, at);
        assert!(marks.clocks(at).screen < 0.01, "a new screen starts its clock");
        assert!(marks.animating(at), "and asks for frames");
        let later = marks.clocks(at + Duration::from_millis(500));
        assert!((later.screen - 0.5).abs() < 0.02, "it keeps counting: {}", later.screen);
        assert!(later.panel >= SETTLED - 0.01, "a panel that was never opened has no clock");
        assert!(!marks.animating(at + Duration::from_secs(4)), "and stops asking once it is over");

        let _ = state.update(Message::AddPanel(true), 1);
        let opened = start + Duration::from_secs(12);
        marks.observe(&state, opened);
        assert!(marks.clocks(opened).panel < 0.01, "opening a panel starts its clock");
        let again = marks.clocks(opened + Duration::from_millis(300));
        assert!((again.panel - 0.3).abs() < 0.02);
        assert!(again.screen > 1.0, "and leaves the screen's clock alone");

        let _ = state.update(Message::Back, 1);
        marks.observe(&state, start + Duration::from_secs(14));
        assert!(marks.clocks(start + Duration::from_secs(14)).screen < 0.01, "going back is a new screen too");

        state.publications.push(published("new"));
        let seen = start + Duration::from_secs(20);
        marks.observe(&state, seen);
        let clocks = marks.clocks(seen);
        assert!(clocks.of("new") < 0.01, "an arrival is stamped when it is first seen");
        assert!(clocks.of("old") >= SETTLED - 0.01, "and the ones that were there are not");
        assert!(marks.animating(seen));
        let next = marks.clocks(seen + Duration::from_millis(200));
        assert!((next.of("new") - 0.2).abs() < 0.02);
        assert_eq!(clocks.of("nobody"), SETTLED);
    }

    #[test]
    fn a_stamp_taken_after_the_last_tick_counts_from_nothing_instead_of_going_backwards() {
        let mut state = State::new(scratch("late-stamp"));
        let mut marks = Marks::default();
        let tick = Instant::now();
        marks.observe(&state, tick);
        let _ = state.update(Message::New, 1);
        marks.observe(&state, tick + Duration::from_secs(1));
        let stale = marks.clocks(tick);
        assert_eq!(stale.screen, 0.0, "a clock that is behind the stamp reads zero");
        assert!(marks.animating(tick), "and still asks for the frames that will move it");
    }

    #[test]
    fn a_screen_is_told_from_another_by_what_it_shows_and_a_panel_by_whether_one_is_open() {
        let mut state = State::new(scratch("keys"));
        assert_eq!(state.screen_key(), "shelf:false:Mine");
        state.shelf = Shelf::Saved;
        assert_eq!(state.screen_key(), "shelf:false:Saved");
        state.shelf = Shelf::Mine;
        state.collection_shelf = true;
        assert_eq!(state.screen_key(), "shelf:true:Mine");
        assert_eq!(state.panel_key(), "none");
        state.collection_shelf = false;
        let _ = state.update(Message::New, 1);
        let id = match &state.screen {
            Screen::Editor(editor) => editor.id.clone(),
            _ => panic!("an editor"),
        };
        assert_eq!(state.screen_key(), format!("editor:{id}"));
        let _ = state.update(Message::AddPanel(true), 1);
        assert_eq!(state.panel_key(), "panel");
        let _ = state.update(Message::Select(None), 1);
        assert_eq!(state.panel_key(), "none");
        if let Screen::Editor(editor) = &mut state.screen {
            editor.share = true;
        }
        assert_eq!(state.panel_key(), "share");
    }
}
