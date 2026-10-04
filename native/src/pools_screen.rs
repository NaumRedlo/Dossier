use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use iced::widget::{button, column, container, image, row, scrollable, text, text_input, Space};
use iced::{Background, Border, Color, Element, Length, Padding};
use md5::{Digest, Md5};

use crate::community_screen as screen;
use crate::glyphs::{glyph, Icon};
use crate::lang::Words;
use crate::library::Map;
use crate::pool_collections::Collection;
use crate::pool_links::{self, Target, Why};
use crate::pool_share;
use crate::pools::{self, Balance, Frame, Measure, Measures, Mod, Pool, Skill, Slot};
use crate::theme::{self, ACCENT, FAINT, INK, MUTED};
use crate::ui;

const ROW_HIGH: f32 = 56.0;
const COVER_WIDE: f32 = 86.0;
const COVER_HIGH: f32 = 48.0;
const COVER_ROUND: f32 = 10.0;
const TILE_ROUND: f32 = 12.0;
const HERO_ROUND: f32 = 14.0;
const THUMB_ROUND: f32 = 8.0;
const PANEL_WIDE: f32 = 400.0;
const PANEL_FROM: f32 = 900.0;
const RESULTS_MOST: usize = 40;
const COLLECTION_PAGE: usize = 80;
const STRIP_BAR: f32 = 64.0;
const STRIP_HIGH: f32 = 72.0;
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
}

impl Editor {
    pub fn at(id: String) -> Editor {
        Editor { id, selected: None, panel: Panel::Closed, query: String::new(), source: SourceTab::Search, collection: None, collection_page: 0, untouched: false, replace: false, share: false, choosing: false, marked: Vec::new(), bulk: None, asking_delete: false }
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
        let content = serde_json::to_vec(&(&slot.hash, slot.mods, &slot.note, &slot.artist, &slot.title, &slot.version, slot.set)).expect("slot identity");
        let digest: [u8; 16] = Md5::digest(content).into();
        let occurrence = seen.entry(digest).or_insert(0);
        let key = SlotKey(digest, *occurrence);
        *occurrence += 1;
        key
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
    Filter(Option<Frame>),
    New,
    Open(String),
    Back,
    Rename(String),
    UseFrame(Frame),
    Select(Option<usize>),
    Replace,
    SetMod(usize, Mod),
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
}

#[derive(Clone)]
pub struct State {
    pub list: Vec<Pool>,
    pub loaded: bool,
    pub screen: Screen,
    pub filter: Option<Frame>,
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
}

impl State {
    pub fn new(dir: PathBuf) -> State {
        State {
            list: Vec::new(),
            loaded: false,
            screen: Screen::Shelf,
            filter: None,
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
        effects
    }

    pub fn editing(&self) -> Option<&Pool> {
        match &self.screen {
            Screen::Editor(editor) => self.list.iter().find(|pool| pool.id == editor.id),
            Screen::Shelf | Screen::Open(_) => None,
        }
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
            let _ = pools::save(&self.dir, pool);
        }
        self.list.sort_by(|a, b| b.changed_at.cmp(&a.changed_at).then_with(|| a.id.cmp(&b.id)));
    }

    fn save_quiet(&mut self, at: usize) {
        if let Some(pool) = self.list.get(at) {
            let _ = pools::save(&self.dir, pool);
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
        let mut effects = Vec::new();
        match message {
            Message::Filter(frame) => self.filter = frame,
            Message::New => {
                self.cancel_network();
                self.cancel_suggestion();
                self.cancel_import();
                let pool = Pool::new(Frame::Free, "", now);
                let id = pool.id.clone();
                self.list.insert(0, pool);
                self.save(0, now);
                self.screen = Screen::Editor(Editor { untouched: true, ..Editor::at(id) });
            }
            Message::Open(id) => {
                if self.list.iter().any(|pool| pool.id == id) {
                    self.cancel_network();
                    self.cancel_suggestion();
                    self.cancel_import();
                    self.screen = Screen::Editor(Editor::at(id));
                    if self.songs.is_none() && !self.reading {
                        self.reading = true;
                        effects.push(Effect::ReadSongs);
                    }
                }
            }
            Message::Back => {
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
                    self.save(at, now);
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
                if let Some(editor) = self.editor_mut() {
                    editor.selected = slot;
                    editor.replace = false;
                    editor.panel = if slot.is_some() { Panel::Slot } else { Panel::Closed };
                }
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
            }
            Message::SetMod(slot, mods) => {
                if let Some(at) = self.editing_at() {
                    if self.list[at].slots.get(slot).is_some_and(|entry| entry.mods != mods) {
                        self.remember(at);
                        self.list[at].slots[slot].mods = mods;
                        self.list[at].slots[slot].measure = None;
                        self.save(at, now);
                        effects.extend(self.measure_effects());
                    }
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
                if source == SourceTab::Collections && self.collections.is_none() && !self.collecting {
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
                        if let Some(suggestion) = &mut self.suggestions {
                            suggestion.maps = Some(maps);
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
                    let message = if is_paste(&pasted) {
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
                let text = text.trim().to_owned();
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
                self.notice = None;
                if let Some(editor) = self.editor_mut() {
                    editor.panel = Panel::Add;
                    editor.replace = false;
                    editor.query = text.clone();
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
                        self.save(at, now);
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
        let pool = &mut self.list[at];
        let slot = match aimed {
            Some(slot) => slot,
            None => {
                pool.slots.push(Slot::empty(Mod::Nm));
                pool.slots.len() - 1
            }
        };
        pool.slots[slot].fill(hash, &map);
        let cached = match self.measures.get(hash, pool.slots[slot].mods) {
            Some(Ok(measure)) => Some(*measure),
            _ => None,
        };
        pool.slots[slot].measure = cached;
        self.notice = None;
        if let Some(editor) = self.editor_mut() {
            editor.untouched = false;
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

    pub fn covers(&self) -> Vec<(String, PathBuf)> {
        let Some(songs) = &self.songs else { return Vec::new() };
        let mut seen = HashSet::new();
        let mut wanted = Vec::new();
        let mut add = |hash: &str| {
            if let Some(path) = songs.get(hash).and_then(|map| map.background.clone()) {
                if seen.insert(hash.to_owned()) {
                    wanted.push((hash.to_owned(), path));
                }
            }
        };
        match &self.screen {
            Screen::Shelf => {
                for pool in &self.list {
                    for slot in pool.slots.iter().filter_map(|slot| slot.hash.as_deref()).take(4) {
                        add(slot);
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

pub fn is_paste(text: &str) -> bool {
    pool_share::is_text(text) || matches!(pool_links::parse(text), Ok(_) | Err(pool_links::Refusal::Mode(_)))
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
        .style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.0077 * k))),
            border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.0392 * k), width: 1.0, radius: 14.0.into() },
            ..container::Style::default()
        })
        .into()
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

fn kind_label(words: &Words, frame: Option<Frame>, all: usize) -> String {
    match frame {
        None => words.with("pools-filter-all", &[("n", all.to_string())]),
        Some(Frame::Duel) => words.t("pools-kind-duel"),
        Some(Frame::Stage) => words.t("pools-kind-stage"),
        Some(Frame::Free) => words.t("pools-kind-free"),
    }
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

fn pool_card<'a>(pool: &'a Pool, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, tile: f32) -> Element<'a, Message> {
    let facts = pool.facts();
    let mut collage = row![].spacing(6);
    let mut shown = 0;
    for slot in pool.slots.iter().filter(|slot| !slot.is_empty()).take(4) {
        collage = collage.push(cover(thumbs, slot.hash.as_deref(), tile, 92.0, TILE_ROUND));
        shown += 1;
    }
    while shown < 4 {
        collage = collage.push(cover(thumbs, None, tile, 92.0, TILE_ROUND));
        shown += 1;
    }
    let name = if pool.name.is_empty() { words.t("pool-untitled") } else { pool.name.clone() };
    let line = row![
        mono(words.n("pool-cards", facts.cards as u64), 12.0, MUTED),
        mono(words.n("pool-minutes", facts.minutes.max(0) as u64), 12.0, MUTED),
        ui::grow(),
        bars(pool),
    ]
    .spacing(14)
    .align_y(iced::alignment::Vertical::Bottom);
    let inside = column![collage, semi(name, 17.0, INK), line].spacing(12);
    button(container(inside).padding(14).width(Length::Fill))
        .padding(0)
        .width(Length::Fill)
        .style(ui::button_faded(surface_style(false)))
        .on_press(Message::Open(pool.id.clone()))
        .into()
}

fn new_card<'a>(words: &Words) -> Element<'a, Message> {
    let inside = column![glyph(Icon::Plus, 22.0, FAINT), faded_text(words.t("pools-new"), 14.0, MUTED)].spacing(8).align_x(iced::Center);
    button(container(inside).width(Length::Fill).height(Length::Fill).center(Length::Fill))
        .padding(0)
        .width(Length::Fill)
        .height(176.0)
        .style(ui::button_faded(|_, status| {
            let hot = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, if hot { 0.2 } else { 0.12 }))),
                text_color: INK,
                border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, if hot { 0.2 } else { 0.1 }), width: 1.0, radius: 14.0.into() },
                shadow: iced::Shadow::default(),
                snap: true,
            }
        }))
        .on_press(Message::New)
        .into()
}

fn grid_rows<'a>(cells: Vec<Element<'a, Message>>, columns: usize) -> Element<'a, Message> {
    let columns = columns.max(1);
    let mut rows = column![].spacing(14);
    let mut cells = cells.into_iter();
    loop {
        let mut line = row![].spacing(14);
        let mut taken = 0;
        for cell in cells.by_ref().take(columns) {
            line = line.push(container(cell).width(Length::FillPortion(1)));
            taken += 1;
        }
        if taken == 0 {
            break;
        }
        for _ in taken..columns {
            line = line.push(Space::new().width(Length::FillPortion(1)));
        }
        rows = rows.push(line);
        if taken < columns {
            break;
        }
    }
    rows.into()
}

fn refusal_line<'a>(refused: &pool_share::Refused, words: &Words) -> Element<'a, Message> {
    let key = match refused {
        pool_share::Refused::NotAPool => "pool-refused-not",
        pool_share::Refused::Newer => "pool-refused-newer",
        pool_share::Refused::Damaged => "pool-refused-damaged",
    };
    faded_text(words.t(key), 13.0, ACCENT)
}

fn empty_shelf<'a>(words: &Words, refused: Option<&pool_share::Refused>, t: f32) -> Element<'a, Message> {
    let k = ui::fade();
    let tile = container(glyph(Icon::Pool, 28.0, FAINT))
        .width(72.0)
        .height(72.0)
        .center(72.0)
        .style(move |_| container::Style {
            border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.06 * k), width: 1.0, radius: 18.0.into() },
            ..container::Style::default()
        });
    let inside = column![
        tile,
        semi(words.t("pools-empty-title"), 22.0, INK),
        container(text(words.t("pools-empty-text")).font(theme::SANS).size(14.0).color(ui::faded(MUTED)).align_x(iced::alignment::Horizontal::Center)).max_width(440.0),
        row![primary_button(words.t("pools-empty-make"), Message::New), quiet_button(words.t("pools-open"), Message::OpenFile)].spacing(10),
        match refused {
            Some(refused) => refusal_line(refused, words),
            None => faded_text(words.t("pools-local"), 12.0, FAINT),
        },
    ]
    .spacing(14)
    .align_x(iced::Center);
    ui::appearing(ui::appear(t, 0), 10.0, || container(inside).center_x(Length::Fill).padding(Padding::ZERO.top(90.0)).into())
}

fn shelf<'a>(state: &'a State, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, width: f32, t: f32) -> Element<'a, Message> {
    if state.list.is_empty() {
        return empty_shelf(words, state.refused.as_ref(), t);
    }
    let head = row![
        crumbs(words, None, semi(words.t("pools-title"), 28.0, INK)),
        ui::grow(),
        quiet_button(words.t("pools-open"), Message::OpenFile),
        primary_button(words.t("pools-new"), Message::New),
    ]
    .spacing(10)
    .align_y(iced::alignment::Vertical::Bottom);
    let mut chips = row![].spacing(8);
    for frame in [None, Some(Frame::Duel), Some(Frame::Stage), Some(Frame::Free)] {
        let present = frame.is_none() || state.list.iter().any(|pool| Some(pool.frame) == frame);
        if !present {
            continue;
        }
        chips = chips.push(
            button(text(kind_label(words, frame, state.list.len())).font(theme::SANS_SEMI).size(13.5).wrapping(text::Wrapping::None))
                .padding([8, 16])
                .style(ui::button_faded(theme::filter_chip(state.filter == frame)))
                .on_press(Message::Filter(frame)),
        );
    }
    let shown: Vec<&Pool> = state.list.iter().filter(|pool| state.filter.is_none_or(|frame| pool.frame == frame)).collect();
    let columns = (((width + 14.0) / (300.0 + 14.0)).floor() as usize).clamp(1, 4);
    let card_wide = (width - 14.0 * (columns as f32 - 1.0)) / columns as f32;
    let tile = ((card_wide - 28.0 - 18.0) / 4.0).floor().max(24.0);
    let mut cells: Vec<Element<'a, Message>> = vec![new_card(words)];
    cells.extend(shown.into_iter().map(|pool| pool_card(pool, words, thumbs, tile)));
    let mut page = column![ui::appearing(ui::appear(t, 0), 10.0, || head.into())].spacing(18);
    if let Some(refused) = &state.refused {
        page = page.push(refusal_line(refused, words));
    }
    page.push(ui::appearing(ui::appear(t, 1), 10.0, || chips.into())).push(ui::appearing(ui::appear(t, 2), 10.0, || grid_rows(cells, columns))).into()
}

fn strip<'a>(pool: &Pool, selected: Option<usize>) -> Element<'a, Message> {
    let k = ui::fade();
    let top = pool.slots.iter().filter_map(|slot| slot.measure.map(|m| m.stars)).fold(1.0_f64, f64::max).max(6.0);
    let mut line = row![].spacing(8).align_y(iced::alignment::Vertical::Bottom);
    for (at, slot) in pool.slots.iter().enumerate() {
        let lit = selected == Some(at);
        let (high, colour) = match slot.measure {
            Some(measure) => (10.0 + (measure.stars / top).clamp(0.0, 1.0) as f32 * (STRIP_HIGH - 10.0), crate::dossier::star_colour(measure.stars as f32)),
            None => (3.0, Color::from_rgba(1.0, 1.0, 1.0, 0.16)),
        };
        let alpha = if slot.measure.is_none() { 1.0 } else if lit { 1.0 } else { 0.55 };
        let bar = container(Space::new()).width(STRIP_BAR).height(high).style(move |_| container::Style {
            background: Some(Background::Color(Color { a: colour.a * alpha * k, ..colour })),
            border: Border { radius: 6.0.into(), ..Border::default() },
            ..container::Style::default()
        });
        let label = container(mono(slot.mods.code().to_owned(), 11.0, if lit { INK } else { FAINT })).width(STRIP_BAR).center_x(STRIP_BAR);
        let column = column![container(bar).height(STRIP_HIGH).align_y(iced::alignment::Vertical::Bottom), label].spacing(6);
        line = line.push(button(column).padding(0).style(ui::button_faded(theme::bare)).on_press(Message::Select(Some(at))));
    }
    scrollable(line).direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::hidden())).style(ui::thin_scroll).into()
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

fn slot_row<'a>(pool: &'a Pool, at: usize, selected: bool, choosing: bool, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, songs: Option<&Arc<HashMap<String, Map>>>) -> Element<'a, Message> {
    let slot = &pool.slots[at];
    let mut left = row![].spacing(8).align_y(iced::Center);
    if choosing {
        left = left.push(check_box(selected));
    }
    let left = left.push(number_badge(at + 1, selected)).push(mod_badge(slot.mods, selected)).width(if choosing { 118.0 } else { 88.0 });
    let inside: Element<'a, Message> = match slot.hash.as_deref() {
        None => row![
            left,
            cover(thumbs, None, COVER_WIDE, COVER_HIGH, COVER_ROUND),
            column![semi(words.t("pool-slot-empty"), 14.0, MUTED), mono(words.t("pool-slot-need"), 11.0, FAINT)].spacing(3).width(Length::Fill),
        ]
        .spacing(14)
        .align_y(iced::Center)
        .into(),
        Some(hash) => {
            let missing = songs.is_some_and(|songs| !songs.contains_key(hash));
            let figures: Element<'a, Message> = match slot.measure {
                Some(measure) => row![
                    mono(format!("{:.0}", measure.bpm), 13.0, MUTED),
                    mono(pools::clock(measure.length_ms), 13.0, MUTED),
                    container(text(stars_of(words, measure.stars)).font(theme::MONO_BOLD).size(19.0).wrapping(text::Wrapping::None).color(ui::faded(crate::dossier::star_colour(measure.stars as f32)))).width(64.0).align_x(iced::alignment::Horizontal::Right),
                ]
                .spacing(24)
                .align_y(iced::Center)
                .into(),
                None if missing => mono(words.t("pool-no-disk"), 12.0, ACCENT),
                None => mono(words.t("pool-measuring"), 12.0, FAINT),
            };
            row![
                left,
                cover(thumbs, Some(hash), COVER_WIDE, COVER_HIGH, COVER_ROUND),
                column![semi(slot.title.clone(), 14.0, INK), faded_text(slot.artist.clone(), 12.5, MUTED), mono(slot.version.clone(), 11.0, FAINT)].spacing(1).width(Length::Fill),
                figures,
            ]
            .spacing(14)
            .align_y(iced::Center)
            .into()
        }
    };
    let row_button = button(container(inside).height(ROW_HIGH).align_y(iced::Center))
        .padding([4, 10])
        .width(Length::Fill)
        .style(ui::button_faded(row_style(selected)))
        .on_press(if choosing { Message::Mark(at) } else { Message::Select(Some(at)) });
    if slot.is_empty() && !choosing {
        row![row_button, quiet_button(words.t("pool-suggest"), Message::Suggest(at))].spacing(8).align_y(iced::Center).into()
    } else {
        row_button.into()
    }
}

fn mod_ribbon<'a>(words: &Words, slot: usize, current: Mod) -> Element<'a, Message> {
    let mut ribbon = row![].spacing(8);
    for mods in Mod::ALL {
        ribbon = ribbon.push(
            button(text(mods.code()).font(theme::SANS_SEMI).size(13.5))
                .padding([8, 16])
                .style(ui::button_faded(theme::filter_chip(mods == current)))
                .on_press(Message::SetMod(slot, mods)),
        );
    }
    column![ui::mono_small(words.t("pool-slot-mod"), FAINT), ribbon].spacing(10).into()
}

fn figure<'a>(label: String, value: String, size: f32, colour: Color) -> Element<'a, Message> {
    column![ui::mono_small(label, FAINT), text(value).font(theme::MONO_BOLD).size(size).wrapping(text::Wrapping::None).color(ui::faded(colour))].spacing(4).into()
}

fn balance_line<'a>(pool: &Pool, words: &Words) -> Option<Element<'a, Message>> {
    let balance = pool.balance()?;
    Some(match balance {
        Balance::Even => faded_text(words.t("pool-even"), 15.0, INK),
        Balance::Heavy(skill, percent) => row![faded_text(words.t(skill.outweighs_key()), 15.0, INK), mono(format!("{percent}%"), 12.0, MUTED)].spacing(12).align_y(iced::Center).into(),
    })
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

fn slot_panel<'a>(pool: &'a Pool, at: usize, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, songs: Option<&Arc<HashMap<String, Map>>>) -> Element<'a, Message> {
    let slot = &pool.slots[at];
    let mut body = column![].spacing(16);
    match slot.hash.as_deref() {
        None => {
            body = body.push(semi(words.t("pool-slot-empty"), 18.0, INK));
            body = body.push(faded_text(words.t("pool-slot-need"), 13.0, MUTED));
            body = body.push(primary_button(words.t("pool-add"), Message::AddPanel(true)));
        }
        Some(hash) => {
            body = body.push(cover(thumbs, Some(hash), PANEL_WIDE - 32.0, 150.0, HERO_ROUND));
            body = body.push(column![semi(slot.title.clone(), 20.0, INK), faded_text(slot.artist.clone(), 14.0, MUTED), mono(slot.version.clone(), 12.0, FAINT)].spacing(3));
            match slot.measure {
                Some(measure) => {
                    body = body.push(
                        row![
                            figure(words.t("pool-stars"), stars_of(words, measure.stars), 30.0, crate::dossier::star_colour(measure.stars as f32)),
                            figure(words.t("pool-bpm"), format!("{:.0}", measure.bpm), 20.0, INK),
                            figure(words.t("pool-length"), pools::clock(measure.length_ms), 20.0, INK),
                            figure(words.t("pool-combo"), measure.max_combo.to_string(), 20.0, INK),
                        ]
                        .spacing(24)
                        .align_y(iced::alignment::Vertical::Bottom),
                    );
                    let one = |value: f64| screen::decimal(words, value as f32, 1);
                    body = body.push(
                        row![
                            figure("AR".to_owned(), one(measure.ar), 16.0, INK),
                            figure("OD".to_owned(), one(measure.od), 16.0, INK),
                            figure("CS".to_owned(), one(measure.cs), 16.0, INK),
                            figure("HP".to_owned(), one(measure.hp), 16.0, INK),
                        ]
                        .spacing(24),
                    );
                    body = body.push(profile_bars(&measure, words));
                }
                None if songs.is_some_and(|songs| !songs.contains_key(hash)) => body = body.push(mono(words.t("pool-no-disk"), 12.0, ACCENT)),
                None => body = body.push(mono(words.t("pool-measuring"), 12.0, FAINT)),
            }
        }
    }
    body = body.push(mod_ribbon(words, at, slot.mods));
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

fn finding_card<'a>(finding: &'a Finding, editor: &'a Editor, pool: &'a Pool, words: &'a Words) -> Element<'a, Message> {
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
        Finding::Found(candidate) => candidate_card(candidate, editor, pool, words),
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
            container(container(Space::new()).width(7.0).height(7.0).style(|_| container::Style { background: Some(Background::Color(ui::faded(ACCENT))), border: Border { radius: 3.5.into(), ..Border::default() }, ..container::Style::default() })).center(14.0).into()
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

fn candidate_card<'a>(candidate: &'a Candidate, editor: &'a Editor, pool: &'a Pool, words: &'a Words) -> Element<'a, Message> {
    let found = &candidate.found;
    let many = found.difficulties.len() > 1;
    let title = if many { words.n("pool-link-set", found.difficulties.len() as u64) } else { words.t("pool-link-found") };
    let heading = row![glyph(Icon::Chain, 15.0, ACCENT), semi(title, 14.5, INK)].spacing(10).align_y(iced::Center);
    let thumb: Element<'a, Message> = match &candidate.cover {
        Some(handle) => image(crate::crops::fitted(handle, PANEL_WIDE - 32.0, 110.0, 12.0)).content_fit(iced::ContentFit::Fill).width(PANEL_WIDE - 32.0).height(110.0).border_radius(12.0).opacity(ui::fade()).into(),
        None => container(Space::new())
            .width(PANEL_WIDE - 32.0)
            .height(110.0)
            .style(|_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.012))), border: Border { color: Color::from_rgba(1.0, 1.0, 1.0, 0.04), width: 1.0, radius: 10.0.into() }, ..container::Style::default() })
            .into(),
    };
    let names = column![semi(found.title.clone(), 17.0, INK), faded_text(found.artist.clone(), 13.5, MUTED)].spacing(2);
    let mut list = column![].spacing(6);
    for (at, difficulty) in found.difficulties.iter().enumerate() {
        let lit = candidate.choice == Some(at);
        let colour = crate::dossier::star_colour(difficulty.stars as f32);
        let dot = container(Space::new()).width(9.0).height(9.0).style(move |_| container::Style { background: Some(Background::Color(colour)), border: Border { radius: 4.5.into(), ..Border::default() }, ..container::Style::default() });
        let line = row![dot, semi(difficulty.version.clone(), 13.5, INK), ui::grow(), mono(stars_of(words, difficulty.stars), 13.0, colour)].spacing(10).align_y(iced::Center);
        let made: Element<'a, Message> = if many {
            button(line).padding([8, 12]).width(Length::Fill).style(ui::button_faded(row_style(lit))).on_press(Message::Choose(at)).into()
        } else {
            container(line).padding([4, 4]).into()
        };
        list = list.push(made);
    }
    let mut chips = row![].spacing(8);
    let mut options: Vec<Place> = Vec::new();
    if let Some(slot) = target(pool, editor) {
        options.push(Place::Slot(slot));
    }
    options.push(Place::End);
    if let Some(selected) = editor.selected.filter(|at| pool.slots.get(*at).is_some_and(|slot| !slot.is_empty())) {
        options.push(Place::Replace(selected));
    }
    for place in options {
        let label = match place {
            Place::Slot(at) => words.with("pool-place-slot", &[("n", (at + 1).to_string()), ("mod", pool.slots.get(at).map_or("NM", |slot| slot.mods.code()).to_owned())]),
            Place::End => words.t("pool-place-end"),
            Place::Replace(at) => words.with("pool-place-replace", &[("n", (at + 1).to_string())]),
        };
        chips = chips.push(button(text(label).font(theme::SANS_SEMI).size(13.0).wrapping(text::Wrapping::None)).padding([7, 14]).style(ui::button_faded(theme::filter_chip(candidate.place == place))).on_press(Message::Aim(place)));
    }
    let mut actions = row![].spacing(8).align_y(iced::Center);
    let put_label = match candidate.choice.and_then(|at| found.difficulties.get(at)) {
        Some(difficulty) => match candidate.place {
            Place::End => words.with("pool-link-put-end", &[("version", difficulty.version.clone())]),
            Place::Slot(at) | Place::Replace(at) => words.with("pool-link-put-to", &[("version", difficulty.version.clone()), ("n", (at + 1).to_string())]),
        },
        None => words.t("pool-link-put"),
    };
    actions = actions.push(if candidate.choice.is_some() { primary_button(put_label, Message::Confirm) } else { ui::primary(put_label, None) });
    if many {
        actions = actions.push(quiet_button(words.t("pool-link-put-all"), Message::ConfirmAll));
    }
    actions = actions.push(button(text(words.t("pool-link-cancel")).font(theme::SANS_SEMI).size(13.0).color(ui::faded(MUTED))).padding([8, 12]).style(ui::button_faded(theme::bare)).on_press(Message::Dismiss));
    let inside = column![
        heading,
        thumb,
        names,
        list,
        ui::mono_small(words.t("pool-link-where"), FAINT),
        ui::wrap(vec![chips.into()], 8.0),
        actions,
    ]
    .spacing(12);
    panel_box(inside.into())
}

fn add_panel<'a>(state: &'a State, editor: &'a Editor, pool: &'a Pool, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>) -> Element<'a, Message> {
    let mut body = column![].spacing(14);
    let sources = [(SourceTab::Search, "pool-tab-search"), (SourceTab::Collections, "pool-tab-collections"), (SourceTab::Best, "pool-tab-best"), (SourceTab::Suggest, "pool-tab-suggest")];
    let tabs = row![
        ui::wrap(sources.into_iter().map(|(source, key)| button(text(words.t(key)).font(theme::SANS_SEMI).size(13.5).wrapping(text::Wrapping::None)).padding([8, 12]).style(ui::button_faded(theme::filter_chip(editor.source == source))).on_press(Message::Source(source)).into()).collect(), 6.0),
        button(glyph(Icon::Close, 14.0, FAINT)).padding([6, 8]).style(ui::button_faded(theme::bare)).on_press(Message::AddPanel(false)),
    ]
    .spacing(8)
    .align_y(iced::Center);
    body = body.push(tabs);
    if matches!(editor.source, SourceTab::Collections | SourceTab::Best) {
        if let Some(Notice::Already(at)) = &state.notice {
            body = body.push(faded_text(words.with("pool-already", &[("n", (at + 1).to_string())]), 13.0, ACCENT));
        }
        if let Some(finding) = &state.finding {
            body = body.push(finding_card(finding, editor, pool, words));
        }
        body = body.push(if editor.source == SourceTab::Best { best_panel(state, words, thumbs) } else { collection_panel(state, editor, words, thumbs) });
        return panel_box(body.into());
    }
    if editor.source == SourceTab::Suggest {
        body = body.push(suggestion_panel(state, words, thumbs));
        return panel_box(body.into());
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
        body = body.push(finding_card(finding, editor, pool, words));
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
        body = body.push(scrollable(list).height(Length::Fixed(420.0)).direction(ui::hidden_bar()).style(ui::thin_scroll));
    }
    panel_box(body.into())
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
    column![refresh, scrollable(list).height(Length::Fixed(420.0)).direction(ui::hidden_bar()).style(ui::thin_scroll)].spacing(10).into()
}

fn suggestion_panel<'a>(state: &'a State, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>) -> Element<'a, Message> {
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
        let inside = row![
            cover(thumbs, Some(hash), 56.0, 32.0, THUMB_ROUND),
            column![semi(map.title.clone(), 13.5, INK), faded_text(map.artist.clone(), 12.0, MUTED), mono(map.version.clone(), 11.0, FAINT)].spacing(1).width(Length::Fill),
            mono(stars_of(words, measure.stars), 12.5, crate::dossier::star_colour(measure.stars as f32)),
        ]
        .spacing(12)
        .align_y(iced::Center);
        list = list.push(button(inside).padding([6, 8]).width(Length::Fill).style(ui::button_faded(ui::calm(theme::row(false)))).on_press(Message::Put(hash.clone())));
    }
    column![
        faded_text(words.with("pool-suggest-target", &[("stars", stars_of(words, target))]), 12.5, MUTED),
        scrollable(list).height(Length::Fixed(420.0)).direction(ui::hidden_bar()).style(ui::thin_scroll),
    ]
    .spacing(12)
    .into()
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
        return scrollable(list).height(Length::Fixed(420.0)).direction(ui::hidden_bar()).style(ui::thin_scroll).into();
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
    let mut body = column![heading, scrollable(list).height(Length::Fixed(420.0)).direction(ui::hidden_bar()).style(ui::thin_scroll)].spacing(10);
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
            container(container(Space::new()).width(7.0).height(7.0).style(|_| container::Style { background: Some(Background::Color(ui::faded(ACCENT))), border: Border { radius: 3.5.into(), ..Border::default() }, ..container::Style::default() })).center(14.0).into()
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
        let badges = row![number_badge(at + 1, false), mod_badge(slot.mods, false)].spacing(8).align_y(iced::Center).width(88.0);
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
    let body: Element<'a, Message> = row![container(left).width(Length::Fixed(PANEL_WIDE)), container(right).width(Length::Fill)].spacing(32).align_y(iced::alignment::Vertical::Top).into();
    column![ui::appearing(ui::appear(t, 0), 10.0, || head.into()), ui::appearing(ui::appear(t, 1), 10.0, || body)].spacing(24).into()
}

fn editor_view<'a>(state: &'a State, editor: &'a Editor, pool: &'a Pool, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, width: f32, t: f32) -> Element<'a, Message> {
    let name_wide = (pool.name.chars().count().max(10) as f32 * 15.5 + 24.0).clamp(240.0, 560.0);
    let field = text_input(&words.t("pool-name-hint"), &pool.name)
        .on_input(Message::Rename)
        .on_paste(|contents| Message::PasteInto(Input::Name, contents))
        .font(theme::SANS_SEMI)
        .size(28.0)
        .padding(0)
        .style(ui::bare_input(ui::fade()))
        .width(name_wide);
    let name: Element<'a, Message> = field.into();
    let mut buttons = row![].spacing(10).align_y(iced::Center);
    if pool.filled() > 0 {
        buttons = buttons.push(quiet_button(words.t("pool-share"), Message::Share(true)));
    }
    buttons = buttons.push(primary_button(words.t("pool-add"), Message::AddPanel(editor.panel != Panel::Add)));
    let head = row![
        container(crumbs(words, Some(Message::Back), name)).width(Length::Fill),
        buttons,
    ]
    .align_y(iced::alignment::Vertical::Bottom)
    .spacing(16);

    let mut under = column![].spacing(14);
    if pool.filled() > 0 {
        let undoable = state.undo.iter().any(|(id, _)| *id == pool.id);
        let mut tools = row![ui::grow()].spacing(12).align_y(iced::Center);
        if undoable {
            tools = tools.push(button(text(words.t("pool-undo")).font(theme::SANS_SEMI).size(13.0).color(ui::faded(MUTED))).padding([6, 10]).style(ui::button_faded(theme::bare)).on_press(Message::Undo));
        }
        tools = tools.push(button(text(words.t(if editor.choosing { "pool-choose-done" } else { "pool-choose" })).font(theme::SANS_SEMI).size(13.0).color(ui::faded(if editor.choosing { ACCENT } else { MUTED }))).padding([6, 10]).style(ui::button_faded(theme::bare)).on_press(Message::Choosing(!editor.choosing)));
        under = under.push(tools);
    }
    if !pool.slots.is_empty() {
        under = under.push(strip(pool, editor.selected));
    }
    if let Some(line) = balance_line(pool, words) {
        under = under.push(line);
    }

    let songs = state.songs.as_ref();
    let keys = slot_keys(pool);
    let pieces = keys.iter().enumerate().map(|(at, key)| {
        let lit = if editor.choosing { editor.marked.contains(&at) } else { editor.selected == Some(at) };
        (*key, slot_row(pool, at, lit, editor.choosing, words, thumbs, songs))
    }).collect();
    let list: Element<'a, Message> = if pool.slots.is_empty() {
        container(faded_text(words.t("pool-free-empty"), 14.0, MUTED)).padding([24, 10]).into()
    } else {
        crate::board::board(pieces, 4.0, move |key, before| Message::Move(pool.id.clone(), keys.clone(), key, before))
            .identity(pool.id.clone())
            .across(1)
            .anywhere()
            .radius(TILE_ROUND)
            .solid(theme::GROUND)
            .into()
    };

    let side: Option<Element<'a, Message>> = match editor.panel {
        Panel::Add => Some(add_panel(state, editor, pool, words, thumbs)),
        Panel::Slot => editor.selected.filter(|at| *at < pool.slots.len()).map(|at| slot_panel(pool, at, words, thumbs, songs)),
        Panel::Closed => None,
    };
    let body: Element<'a, Message> = match side {
        Some(side) if width >= PANEL_FROM => row![container(list).width(Length::FillPortion(3)), container(side).width(Length::Fixed(PANEL_WIDE))].spacing(24).align_y(iced::alignment::Vertical::Top).into(),
        Some(side) => column![side, list].spacing(18).into(),
        None => list.into(),
    };
    let danger: Element<'a, Message> = if editor.asking_delete {
        let name = if pool.name.is_empty() { words.t("pool-untitled") } else { pool.name.clone() };
        row![
            faded_text(words.with("pool-delete-ask", &[("name", name)]), 13.0, MUTED),
            button(text(words.t("pool-delete-yes")).font(theme::SANS_SEMI).size(13.0)).padding([6, 12]).style(ui::button_faded(theme::danger_words)).on_press(Message::DeletePool),
            quiet_button(words.t("pool-delete-no"), Message::AskDelete(false)),
        ]
        .spacing(14)
        .align_y(iced::Center)
        .into()
    } else {
        button(text(words.t("pool-delete")).font(theme::SANS).size(12.5).color(ui::faded(FAINT))).padding([6, 0]).style(ui::button_faded(theme::bare)).on_press(Message::AskDelete(true)).into()
    };
    column![
        ui::appearing(ui::appear(t, 0), 10.0, || head.into()),
        ui::appearing(ui::appear(t, 1), 10.0, || under.into()),
        ui::appearing(ui::appear(t, 2), 10.0, || body),
        container(danger).padding(Padding::ZERO.top(12.0)),
    ]
    .spacing(22)
    .into()
}

pub fn view<'a>(state: &'a State, words: &'a Words, thumbs: &'a HashMap<String, image::Handle>, width: f32, t: f32) -> Element<'a, Message> {
    let page = match &state.screen {
        Screen::Open(opening) => open_view(state, opening, words, thumbs, t),
        Screen::Shelf => shelf(state, words, thumbs, width.min(1180.0), t),
        Screen::Editor(editor) => match state.list.iter().find(|pool| pool.id == editor.id) {
            Some(pool) => editor_view(state, editor, pool, words, thumbs, width, t),
            None => shelf(state, words, thumbs, width.min(1180.0), t),
        },
    };
    let rolled = scrollable(container(container(page).max_width(1180.0)).center_x(Length::Fill).padding(Padding { top: 12.0, right: 40.0, bottom: 28.0, left: 40.0 }))
        .id(iced::widget::Id::new(SCROLL_ID))
        .style(ui::thin_scroll)
        .direction(ui::hidden_bar())
        .width(Length::Fill)
        .height(Length::Fill);
    let page: Element<'a, Message> = crate::glide::brim(crate::glide::edged(rolled, iced::widget::Id::new(SCROLL_ID))).into();
    if let Screen::Editor(editor) = &state.screen {
        if editor.choosing && !editor.marked.is_empty() {
            return iced::widget::stack![page, bulk_bar(editor, words)].into();
        }
        if editor.share {
            if let Some(pool) = state.list.iter().find(|pool| pool.id == editor.id) {
                return iced::widget::stack![page, share_modal(pool, words, state.exporting.is_some())].into();
            }
        }
    }
    page
}

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
        state.editing().expect("an editor").id.clone()
    }

    #[test]
    fn a_new_pool_is_free_and_grows_without_a_preset() {
        let mut state = state_with_songs("free-default");
        state.update(Message::New, 100);
        assert_eq!(state.editing().unwrap().frame, Frame::Free);
        assert!(state.editing().unwrap().slots.is_empty());
        state.update(Message::Put("a1".into()), 101);
        state.update(Message::Put("b2".into()), 102);
        assert_eq!(state.editing().unwrap().slots.len(), 2);
        state.update(Message::Clear(0), 103);
        assert_eq!(state.editing().unwrap().slots.len(), 1);
        assert_eq!(pools::load_all(&state.dir)[0].slots[0].hash.as_deref(), Some("b2"));
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
        assert!(matches!(first.as_slice(), [Effect::ReadSongs]));
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
    fn dragging_in_the_editor_publishes_a_move_without_opening_the_slot() {
        let state = pool_with_maps("drag-view", &["a1", "b2", "c3"]);
        let words = Words::new(crate::lang::Lang::En);
        let thumbs = HashMap::new();
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1000.0, 900.0), view(&state, &words, &thumbs, 1000.0, 1.0));
        let first = screen.find("Glass Orchard").unwrap().visible_bounds().unwrap();
        let third = screen.find("Ninth Window").unwrap().visible_bounds().unwrap();
        screen.point_at(first.center());
        let _ = screen.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left))]);
        let position = iced::Point::new(first.center_x(), third.y + third.height + 28.0);
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
        let mut screen = iced_test::Simulator::with_size(crate::settings(), iced::Size::new(1000.0, 480.0), view(&state, &words, &thumbs, 1000.0, 1.0));
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
        assert!(screen.find("Add maps").unwrap().visible_bounds().is_none(), "a carried slot must scroll the header off screen");
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
}
