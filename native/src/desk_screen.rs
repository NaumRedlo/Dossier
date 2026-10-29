use std::collections::{HashMap, HashSet};

use iced::widget::{button, column, container, image, mouse_area, row, scrollable, stack, text, text_editor, text_input, Space};
use iced::{mouse, Alignment, Background, Border, Color, Element, Length, Padding};

use crate::desk::{self, Cell, Desk, Held, Task};
use crate::duels_screen::{bold, player_face, strong, words, ROW};
use crate::glyphs::{glyph, Icon};
use crate::lang::Words;
use crate::theme::workspace;
use crate::theme::{self, ACCENT, FAINT, INK, MUTED};
use crate::ui;

mod occupancy;

const TRAINING_WAIT: std::time::Duration = std::time::Duration::from_secs(240);

const GOOD: Color = theme::GRADE_A;
const UNSURE: Color = theme::GRADE_S;
const BAD: Color = iced::color!(0xf38a8a);
const PICKED: Color = Color { a: 0.55, ..ACCENT };
pub(crate) const LIT: Color = Color::from_rgba(0.135, 0.086, 0.098, 0.92);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Queue,
    Label,
    Batches,
    Catalog,
    Model,
    Duels,
    Journal,
    People,
}

impl Tab {
    const ALL: [Tab; 8] = [Tab::Queue, Tab::Label, Tab::Batches, Tab::Catalog, Tab::Model, Tab::Duels, Tab::Journal, Tab::People];

    fn key(self) -> &'static str {
        match self {
            Tab::Queue => "desk-tab-queue",
            Tab::Label => "desk-tab-label",
            Tab::Batches => "desk-tab-batches",
            Tab::Catalog => "desk-tab-catalog",
            Tab::Model => "desk-tab-model",
            Tab::Duels => "desk-tab-duels",
            Tab::Journal => "desk-tab-journal",
            Tab::People => "desk-tab-people",
        }
    }

    fn open(self, roles: &[String]) -> bool {
        match self {
            Tab::Queue | Tab::Label | Tab::Catalog => desk::allows(roles, desk::REVIEWER),
            Tab::Batches => roles.iter().any(|role| role == desk::OWNER),
            Tab::Model => desk::allows(roles, desk::REVIEWER) || desk::allows(roles, desk::PUBLISHER),
            Tab::Duels => desk::allows(roles, desk::OPERATOR),
            Tab::Journal | Tab::People => !roles.is_empty(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Filter {
    #[default]
    All,
    Uncertain,
    Feedback,
    Unassigned,
}

impl Filter {
    const ALL: [Filter; 4] = [Filter::All, Filter::Uncertain, Filter::Feedback, Filter::Unassigned];

    fn holds(self, task: &Task) -> bool {
        match self {
            Filter::All => true,
            Filter::Uncertain => task.reason.starts_with("uncertain"),
            Filter::Feedback => task.reason.starts_with("feedback"),
            Filter::Unassigned => task.kind == "unassigned",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Filter::All => "desk-filter-all",
            Filter::Uncertain => "desk-filter-uncertain",
            Filter::Feedback => "desk-filter-feedback",
            Filter::Unassigned => "desk-filter-unassigned",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Kind {
    #[default]
    All,
    Maps,
    Labels,
    Model,
    Duels,
    People,
}

impl Kind {
    const ALL: [Kind; 6] = [Kind::All, Kind::Maps, Kind::Labels, Kind::Model, Kind::Duels, Kind::People];

    fn of(action: &str) -> Kind {
        match action {
            "discover" | "discover_result" | "auto_discover" => Kind::Maps,
            "corpus_label" | "add_tournament" | "batch_rename" | "batch_delete" | "batch_exclude" | "batch_include" => Kind::Labels,
            "publish_model" | "rollback_model" | "train_model" | "delete_model" => Kind::Model,
            "cancel" | "manual_finalize" => Kind::Duels,
            "grant" | "revoke" => Kind::People,
            other if other.starts_with("map_") => Kind::Maps,
            _ => Kind::All,
        }
    }

    fn holds(self, action: &str) -> bool {
        self == Kind::All || Kind::of(action) == self
    }

    fn key(self) -> &'static str {
        match self {
            Kind::All => "desk-kind-all",
            Kind::Maps => "desk-kind-maps",
            Kind::Labels => "desk-kind-labels",
            Kind::Model => "desk-kind-model",
            Kind::Duels => "desk-kind-duels",
            Kind::People => "desk-kind-people",
        }
    }
}

pub struct Pool(pub text_editor::Content);

impl Default for Pool {
    fn default() -> Self {
        Pool(text_editor::Content::new())
    }
}

impl Clone for Pool {
    fn clone(&self) -> Self {
        Pool(text_editor::Content::with_text(&self.0.text()))
    }
}

impl std::fmt::Debug for Pool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pool({} lines)", self.0.line_count())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Post {
    Review { beatmap: i64, decision: &'static str, slot: Option<String>, secondary: Option<Option<String>>, reason: String },
    Label { cell: Cell, beatmap: Option<i64>, label: String, reason: String },
    Tournament { name: String, year: i32, pool: String, reason: String },
    Publish { version: String, reason: String },
    Rollback { reason: String },
    Finalize { duel: String, match_id: i64, game_id: i64, reason: String },
    Cancel { duel: String, reason: String },
    Grant { who: String, role: String, reason: String },
    Revoke { player: i64, role: String, reason: String },
    Discover { slot: String, band: usize, reason: String },
    Delete { version: String, reason: String },
    RenameBatch { name: String, year: i32, new_name: String, new_year: i32, reason: String },
    DeleteBatch { name: String, year: i32, beatmap: Option<i64>, reason: String },
    ExcludeBatch { name: String, year: i32, beatmap: Option<i64>, excluded: bool, reason: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Load,
    Refresh(u64),
    RefreshMap(u64, i64),
    Cover(i64),
    Hear(Option<i64>),
    Cell(String, usize),
    Post(Post),
    About(i64),
    Details(i64, String),
    Archive,
    Open(String),
    Inspect(i64),
    AddMap { beatmap: i64, slot: Option<String>, reason: String },
    Older(i64),
    Train(String),
    TrainingJob(String),
}

pub const ABOUT_RETRY: std::time::Duration = std::time::Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq)]
pub struct Training {
    pub job: Option<String>,
    pub versions: usize,
    pub since: std::time::Instant,
    pub asking: bool,
}

impl Training {
    fn new(versions: usize) -> Self {
        Self { job: None, versions, since: std::time::Instant::now(), asking: false }
    }

    fn landed(&self, versions: usize) -> bool {
        self.job.is_none() && versions > self.versions
    }

    fn overdue(&self) -> bool {
        self.job.is_none() && self.since.elapsed() > TRAINING_WAIT
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Result<Desk, String>),
    Refreshed(u64, Result<Desk, String>),
    RefreshedMap(u64, i64, Result<Task, String>),
    Inspect(i64),
    Inspected(i64, Result<Task, String>),
    PresenceTick,
    BusyFrame,
    StarsAt(f32, bool),
    StarsDone,
    Presence(Result<desk::PresenceUpdate, String>),
    Tab(Tab),
    Filter(Filter),
    Pick(i64),
    Reason(String),
    Decide(&'static str, Option<String>),
    NotThis(bool),
    Revoke(i64, String),
    Posted(Result<(), String>),
    TrainStarted(Result<Option<String>, String>),
    TrainingState(String, Result<desk::Job, String>),
    Cover(i64, Option<image::Handle>),
    Hear(i64),
    Heard,
    Year(i32),
    Cell(usize),
    Label(String),
    SaveCell,
    Band(String, usize),
    Held(String, usize, Result<Vec<Held>, String>),
    Version(String),
    Publish,
    Rollback,
    Train,
    Delete,
    Again,
    Stuck(String),
    MatchId(String),
    GameId(String),
    Finalize,
    CancelDuel,
    Tournament(usize),
    MapLabel(String),
    Revisit(Cell, i64),
    ResumeLabels,
    Skip,
    Adding(bool),
    Name(String),
    YearText(String),
    Pool(text_editor::Action),
    AddTournament,
    Player(String),
    Role(&'static str),
    Grant,
    About(i64, Option<desk::About>),
    Details(i64, String, Option<desk::details::Details>),
    Picking(bool),
    Archive,
    Archived(Vec<desk::Packed>),
    ArchivePick(i64),
    ArchiveTake,
    ArchiveDrop,
    Batch(String, i32),
    BatchName(String),
    BatchYear(String),
    RenameBatch,
    DeleteBatch(Option<i64>),
    ExcludeBatch(Option<i64>, bool),
    Secondary(Option<String>),
    Discover,
    Open(i64),
    AddingMap(bool),
    MapLink(String),
    LookMap,
    MapSlot(Option<String>),
    AddMap,
    MapAdded(i64, Result<desk::Added, String>),
    JournalKind(Kind),
    JournalPerson(Option<i64>),
    JournalMore,
    JournalOlder(Result<Vec<desk::Entry>, String>),
    OpenMatch(i64),
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pub desk: Desk,
    pub tab: Tab,
    pub loading: bool,
    pub refresh_serial: u64,
    pub refreshing: Option<(u64, String)>,
    pub revision: Option<String>,
    pub refresh_failed: bool,
    pub posting: bool,
    pub failed: Option<String>,
    pub filter: Filter,
    pub stars: Option<(f64, f64)>,
    pub stars_high: bool,
    pub pending_review: Option<i64>,
    pub busy_frame: u32,
    pub picked: Option<i64>,
    pub reason: String,
    pub covers: HashMap<i64, image::Handle>,
    pub asked: HashSet<i64>,
    pub hearing: Option<i64>,
    pub silent: Option<i64>,
    pub year: Option<i32>,
    pub cell: Option<usize>,
    pub label: Option<String>,
    pub band: Option<(String, usize)>,
    pub held: Vec<Held>,
    pub version: Option<String>,
    pub stuck: Option<String>,
    pub match_id: String,
    pub game_id: String,
    pub tournament: Option<usize>,
    pub skipped: HashSet<i64>,
    pub revisiting: Option<(Cell, i64)>,
    pub pending_label: Option<(Cell, i64, String)>,
    pub adding: bool,
    pub pending_batch: Option<(String, i32)>,
    pub notice: Option<String>,
    pub name: String,
    pub year_text: String,
    pub pool: Pool,
    pub player: String,
    pub role: Option<&'static str>,
    pub abouts: HashMap<i64, desk::About>,
    pub details: HashMap<(i64, String), desk::details::Details>,
    pub details_asked: HashMap<(i64, String), std::time::Instant>,
    pub looked: HashSet<i64>,
    pub about_pending: HashSet<i64>,
    pub picking: bool,
    pub training: Option<Training>,
    pub training_lost: bool,
    pub training_failed: Option<String>,
    pub nothing_new: bool,
    pub moving: bool,
    pub confirming: bool,
    pub revoking: Option<(i64, String)>,
    pub inspected: Option<Task>,
    pub inspecting: Option<i64>,
    pub presence: Vec<desk::Presence>,
    pub presence_failed: bool,
    pub presence_busy: bool,
    pub presence_sent: Option<(String, Option<i64>)>,
    pub adding_map: bool,
    pub map_link: String,
    pub map_asked: bool,
    pub map_slot: Option<String>,
    pub map_failed: Option<String>,
    pub missing: HashSet<i64>,
    pub added: Option<i64>,
    pub journal_kind: Kind,
    pub journal_person: Option<i64>,
    pub journal_more: bool,
    pub journal_end: bool,
    pub finalize_sure: bool,
    pub missed_at: HashMap<i64, std::time::Instant>,
    pub archive: Vec<desk::Packed>,
    pub archive_picked: HashSet<i64>,
    pub batch: Option<(String, i32)>,
    pub batch_name: String,
    pub batch_year: String,
    pub batch_sure: Option<Option<i64>>,
    pub secondary: Option<(i64, Option<String>)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tournament {
    pub corpus: String,
    pub year: i32,
    pub stage: Option<String>,
}

use desk::RESERVE;

impl State {
    pub fn reserve(&self) -> Vec<String> {
        let mut found: Vec<String> = self.desk.cells.iter().filter(|cell| cell.slot != "PENDING").map(|cell| cell.slot.clone())
            .chain(RESERVE.iter().map(|slot| (*slot).to_owned()))
            .filter(|slot| slot != "TB" && !desk::SLOTS.contains(&slot.as_str()))
            .collect();
        found.sort_by_key(|slot| (owc_order(slot), slot.clone()));
        found.dedup();
        found
    }

    pub fn open(&mut self, roles: &[String]) -> Vec<Effect> {
        if !self.tab.open(roles) {
            self.tab = Tab::ALL.into_iter().find(|tab| tab.open(roles)).unwrap_or_default();
        }
        if self.loading || roles.is_empty() {
            return Vec::new();
        }
        self.refreshing = None;
        self.loading = true;
        vec![Effect::Load]
    }

    fn matching_tasks(&self, filter: Filter) -> Vec<&Task> {
        self.inspected.iter().chain(self.desk.tasks.iter().filter(|task| self.inspected.as_ref().is_none_or(|held| held.map.beatmap_id != task.map.beatmap_id)))
            .filter(|task| filter.holds(task) && self.stars.is_none_or(|(low, high)| task.map.stars.is_some_and(|stars| f64::from(stars) >= low && (high >= crate::carousel::STARS_TOP || f64::from(stars) <= high)))).collect()
    }

    fn tasks(&self) -> Vec<&Task> {
        let mut sets = HashSet::new();
        self.matching_tasks(self.filter).into_iter().filter(|task| {
            sets.insert(task.map.beatmapset_id.map(|id| (true, id)).unwrap_or((false, task.map.beatmap_id)))
        }).collect()
    }

    pub(crate) fn catalog_slots(&self) -> Vec<String> {
        let mut slots: Vec<_> = desk::SLOTS.iter().chain(RESERVE.iter()).map(|slot| (*slot).to_owned())
            .chain(self.desk.coverage.iter().flat_map(|coverage| coverage.slots.iter().cloned())).collect();
        slots.sort_by_key(|slot| (owc_order(slot), slot.clone()));
        slots.dedup();
        slots
    }

    pub fn presence_animating(&self) -> bool {
        self.presence.iter().any(|person| !person.mine)
    }

    pub fn presence_context(&self) -> (String, Option<i64>) {
        let map = match self.tab {
            Tab::Queue => self.inspecting.or_else(|| self.task().map(|task| task.map.beatmap_id)),
            Tab::Label => self.next_map().map(|(_, map)| map.beatmap_id),
            _ => None,
        };
        let tab = if self.tab == Tab::Batches { Tab::Label } else { self.tab };
        let mut section = tab.key().trim_start_matches("desk-tab-").to_owned();
        if tab == Tab::Catalog {
            if let Some((slot, band)) = &self.band { section = format!("catalog:{slot}:{band}"); }
        }
        (section, map)
    }

    fn task(&self) -> Option<&Task> {
        if self.inspecting.is_some() { return None; }
        let shown = self.matching_tasks(self.filter);
        self.picked.and_then(|id| shown.iter().copied().find(|task| task.map.beatmap_id == id)).or_else(|| shown.first().copied())
    }

    pub fn tournaments(&self) -> Vec<Tournament> {
        let mut found: Vec<Tournament> = Vec::new();
        for cell in self.desk.cells.iter().rev().filter(|cell| cell.corpus == "own-v1") {
            let stage = Some(cell.stage.clone());
            let key = Tournament { corpus: cell.corpus.clone(), year: cell.year, stage };
            if !found.contains(&key) {
                found.push(key);
            }
        }
        found.sort_by_key(|tournament| (tournament.stage.is_none(), -tournament.year));
        found
    }

    fn held_by(&self, tournament: &Tournament) -> Vec<(&Cell, &desk::CorpusMap)> {
        self.desk
            .cells
            .iter()
            .filter(|cell| cell.corpus == tournament.corpus && cell.year == tournament.year && tournament.stage.as_ref().is_none_or(|stage| *stage == cell.stage))
            .flat_map(|cell| cell.maps.iter().map(move |map| (cell, map)))
            .collect()
    }

    fn next_map(&self) -> Option<(&Cell, &desk::CorpusMap)> {
        if let Some((target, beatmap)) = &self.revisiting {
            return self.desk.cells.iter()
                .filter(|cell| cell.corpus == target.corpus && cell.year == target.year && cell.stage == target.stage && cell.slot == target.slot)
                .find_map(|cell| cell.maps.iter().find(|map| map.beatmap_id == *beatmap).map(|map| (cell, map)));
        }
        let tournament = self.tournaments().get(self.tournament.unwrap_or(0))?.clone();
        self.held_by(&tournament).into_iter().find(|(_, map)| !map.verified && !self.skipped.contains(&map.beatmap_id))
    }

    fn covered(&mut self, sets: impl IntoIterator<Item = Option<i64>>) -> Vec<Effect> {
        let mut effects = Vec::new();
        for set in sets.into_iter().flatten() {
            if self.asked.insert(set) {
                effects.push(Effect::Cover(set));
            }
        }
        effects
    }

    fn take_desk(&mut self, mut desk: Desk) {
        if let Some(least) = desk.journal.iter().map(|entry| entry.id).min() {
            desk.journal.extend(self.desk.journal.iter().filter(|entry| entry.id < least).cloned());
        }
        self.desk = desk;
    }

    fn prefill_match(&mut self) {
        if self.match_id.is_empty() {
            if let Some(found) = self.stuck_shown().and_then(|duel| duel.room.as_ref()).and_then(|room| room.match_id) {
                self.match_id = found.to_string();
            }
        }
    }

    fn journal_shown(&self) -> Vec<&desk::Entry> {
        self.desk.journal.iter().filter(|entry| self.journal_kind.holds(&entry.action) && self.journal_person.is_none_or(|player| entry.actor == player)).collect()
    }

    fn post(&mut self, post: Post) -> Vec<Effect> {
        if self.posting {
            return Vec::new();
        }
        self.refreshing = None;
        self.posting = true;
        self.failed = None;
        vec![Effect::Post(post)]
    }

    pub fn update(&mut self, message: Message, w: &Words) -> Vec<Effect> {
        let mut effects = self.step(message, w);
        effects.extend(self.wanted());
        effects
    }

    fn ask(&mut self, beatmap: i64) -> bool {
        if self.missed_at.get(&beatmap).is_some_and(|at| at.elapsed() < ABOUT_RETRY) {
            return false;
        }
        if self.about_pending.len() >= 4 || !self.looked.insert(beatmap) { return false; }
        self.about_pending.insert(beatmap);
        true
    }

    fn wanted(&mut self) -> Vec<Effect> {
        if self.tab == Tab::Queue || (self.tab == Tab::Catalog && !self.adding_map) {
            let mut effects = Vec::new();
            if let Some(map) = self.task().filter(|_| self.tab == Tab::Queue).map(|task| task.map.clone()) {
                let key = (map.beatmap_id, map.revision.clone().unwrap_or_default());
                if map.stars.is_none() && !self.details.contains_key(&key)
                    && self.details_asked.get(&key).is_none_or(|at| at.elapsed() >= ABOUT_RETRY) {
                    self.details_asked.insert(key.clone(), std::time::Instant::now());
                    effects.push(Effect::Details(key.0, key.1));
                }
            }
            let ids: Vec<_> = if self.tab == Tab::Queue {
                self.tasks().into_iter().filter(|task| task.map.title.as_deref().is_none_or(|s| s.trim().is_empty() || s == task.map.beatmap_id.to_string())).map(|task| task.map.beatmap_id).collect()
            } else {
                self.held.iter().filter(|map| map.title.as_deref().is_none_or(|s| s.trim().is_empty() || s == map.beatmap_id.to_string())).map(|map| map.beatmap_id).collect()
            };
            for id in ids {
                if self.ask(id) { effects.push(Effect::About(id)); }
            }
            return effects;
        }
        if self.tab == Tab::Journal {
            let ids: Vec<i64> = self.journal_shown().into_iter().filter_map(journal_map_id).collect();
            return ids.into_iter().filter(|beatmap| self.ask(*beatmap)).map(Effect::About).collect();
        }
        if self.tab == Tab::Catalog && self.adding_map {
            let asked = self.map_asked || self.map_link.contains('/');
            return desk::read_map(&self.map_link).filter(|beatmap| asked && self.ask(*beatmap)).map(Effect::About).into_iter().collect();
        }
        if self.tab == Tab::Batches {
            let ids: Vec<i64> = self.batch_maps().into_iter().filter(|(_, map)| map.title.as_deref().is_none_or(str::is_empty)).map(|(_, map)| map.beatmap_id).take(40).collect();
            return ids.into_iter().filter(|beatmap| self.ask(*beatmap)).map(Effect::About).collect();
        }
        if self.tab != Tab::Label {
            return Vec::new();
        }
        let ids: Vec<i64> = if self.adding {
            read_pool(&self.pool.0.text()).into_iter().map(|(_, beatmap)| beatmap).take(40).collect()
        } else {
            self.next_map().map(|(_, map)| map.beatmap_id).into_iter().collect()
        };
        ids.into_iter().filter(|beatmap| self.ask(*beatmap)).map(Effect::About).collect()
    }

    pub fn batch_shown(&self) -> Option<&desk::Batch> {
        let held = self.batch.as_ref();
        self.desk.batches.iter().find(|batch| held.is_some_and(|(name, year)| batch.name == *name && batch.year == *year)).or_else(|| self.desk.batches.first())
    }

    pub fn secondary_for(&self, task: &Task) -> Option<String> {
        match &self.secondary {
            Some((beatmap, pick)) if *beatmap == task.map.beatmap_id => pick.clone(),
            _ => task.secondary.clone(),
        }
    }

    fn hold_batch(&mut self) {
        if self.batch.is_none() {
            if let Some((name, year)) = self.batch_shown().map(|batch| (batch.name.clone(), batch.year)) {
                self.batch_name = name.clone();
                self.batch_year = year.to_string();
                self.batch = Some((name, year));
            }
        }
    }

    fn batch_maps(&self) -> Vec<(&Cell, &desk::CorpusMap)> {
        let Some(batch) = self.batch_shown() else { return Vec::new() };
        self.desk.cells.iter().filter(|cell| cell.corpus == "own-v1" && cell.year == batch.year && cell.stage == batch.name)
            .flat_map(|cell| cell.maps.iter().map(move |map| (cell, map))).collect()
    }

    pub fn about(&self, beatmap: i64) -> Option<&desk::About> {
        self.abouts.get(&beatmap)
    }

    fn shown_map(&self, source: &desk::Map) -> desk::Map {
        let mut map = source.clone();
        if let Some(about) = self.about(map.beatmap_id) {
            for (to, from) in [(&mut map.title, &about.title), (&mut map.artist, &about.artist),
                (&mut map.version, &about.version), (&mut map.mapper, &about.mapper)] {
                if to.as_deref().is_none_or(|s| s.trim().is_empty() || s == map.beatmap_id.to_string()) && !from.trim().is_empty() {
                    *to = Some(from.clone());
                }
            }
            map.beatmapset_id = map.beatmapset_id.or(about.set);
            map.plain_stars = map.plain_stars.or(about.stars);
            if map.ranked_status.as_deref().is_none_or(str::is_empty) && !about.status.is_empty() { map.ranked_status = Some(about.status.clone()); }
        }
        if let Some(details) = self.details.get(&(map.beatmap_id, map.revision.clone().unwrap_or_default())) {
            let mods = map.mods.iter().filter(|code| code.as_str() != "NF").cloned().collect::<String>();
            map.stars = map.stars.or_else(|| details.stars.get(if mods.is_empty() { "NM" } else { &mods }).copied());
        }
        map
    }

    fn step(&mut self, message: Message, w: &Words) -> Vec<Effect> {
        match message {
            Message::Details(beatmap, revision, found) => {
                if let Some(found) = found {
                    found.fill(self.abouts.entry(beatmap).or_default());
                    let set = found.about.set;
                    self.details.insert((beatmap, revision), found);
                    return self.covered([set]);
                }
                Vec::new()
            }
            Message::Inspect(beatmap) => {
                if self.posting { return Vec::new(); }
                self.inspecting = Some(beatmap);
                self.inspected = None;
                self.picked = Some(beatmap);
                self.tab = Tab::Queue;
                self.filter = Filter::All;
                self.stars = None;
                self.failed = None;
                vec![Effect::Inspect(beatmap)]
            }
            Message::Inspected(beatmap, result) => {
                if self.inspecting != Some(beatmap) { return Vec::new(); }
                self.inspecting = None;
                match result {
                    Ok(task) => {
                        let set = task.map.beatmapset_id;
                        self.inspected = Some(task);
                        self.covered([set])
                    }
                    Err(why) => { self.failed = Some(why); Vec::new() }
                }
            }
            Message::PresenceTick => {
                if self.training.as_ref().is_some_and(Training::overdue) {
                    self.training = None;
                    self.training_lost = true;
                }
                match self.training.as_mut().filter(|training| !training.asking) {
                    Some(Training { job: Some(job), asking, .. }) => {
                        *asking = true;
                        vec![Effect::TrainingJob(job.clone())]
                    }
                    _ => Vec::new(),
                }
            },
            Message::BusyFrame => { self.busy_frame = self.busy_frame.wrapping_add(1); Vec::new() }
            Message::StarsAt(fraction, first) => {
                if !fraction.is_finite() { return Vec::new(); }
                let value = (f64::from(fraction.clamp(0.0, 1.0)) * crate::carousel::STARS_TOP * 10.0).round() / 10.0;
                let (mut low, mut high) = self.stars.unwrap_or((0.0, crate::carousel::STARS_TOP));
                if first { self.stars_high = (value - high).abs() < (value - low).abs(); }
                if self.stars_high { high = value.max(low); } else { low = value.min(high); }
                self.stars = (low > 0.0 || high < crate::carousel::STARS_TOP).then_some((low, high));
                self.picked = None;
                Vec::new()
            }
            Message::StarsDone => Vec::new(),
            Message::Presence(result) => {
                self.presence_busy = false;
                self.presence_failed = result.is_err();
                match result {
                    Ok(update) => {
                        self.presence = update.people;
                        if let Some(revision) = update.revision.filter(|revision| self.revision.as_ref() != Some(revision)) {
                            if !self.loading && !self.posting && self.refreshing.is_none() {
                                self.refresh_serial = self.refresh_serial.wrapping_add(1);
                                self.refreshing = Some((self.refresh_serial, revision));
                                return vec![Effect::Refresh(self.refresh_serial)];
                            }
                        }
                    }
                    Err(_) => self.presence.clear(),
                }
                Vec::new()
            }
            Message::Refreshed(serial, result) => {
                if self.refreshing.as_ref().is_none_or(|(expected, _)| *expected != serial) {
                    return Vec::new();
                }
                let (_, revision) = self.refreshing.take().unwrap();
                self.refresh_failed = result.is_err();
                let Ok(desk) = result else { return Vec::new(); };
                self.revision = Some(revision);
                let label_map = self.next_map().map(|(_, map)| map.beatmap_id);
                let selected = self.task().map(|task| task.map.beatmap_id);
                let batch = self.tournaments().get(self.tournament.unwrap_or(0)).cloned();
                self.take_desk(desk);
                if let Some(batch) = batch {
                    self.tournament = self.tournaments().iter().position(|held| *held == batch);
                }
                if self.tab == Tab::Label && label_map != self.next_map().map(|(_, map)| map.beatmap_id) { self.reason.clear(); }
                if let Some(id) = selected {
                    if self.matching_tasks(self.filter).iter().any(|task| task.map.beatmap_id == id) {
                        self.picked = Some(id);
                    } else {
                        self.picked = None;
                        self.reason.clear();
                        self.moving = false;
                    }
                }
                if self.training.as_ref().is_some_and(|training| training.landed(self.desk.versions.len())) { self.training = None; }
                let sets = self.desk.tasks.iter().map(|task| task.map.beatmapset_id).collect::<Vec<_>>();
                let mut effects = self.covered(sets);
                if let Some((slot, band)) = &self.band { effects.push(Effect::Cell(slot.clone(), *band)); }
                if let Some(task) = &self.inspected { effects.push(Effect::RefreshMap(serial, task.map.beatmap_id)); }
                effects
            }
            Message::RefreshedMap(serial, beatmap, result) => {
                if self.refresh_serial != serial || self.posting || self.loading || self.inspected.as_ref().is_none_or(|task| task.map.beatmap_id != beatmap) { return Vec::new(); }
                match result {
                    Ok(task) => {
                        let set = task.map.beatmapset_id;
                        self.inspected = Some(task);
                        self.covered([set])
                    }
                    Err(_) => { self.refresh_failed = true; self.revision = None; Vec::new() }
                }
            }
            Message::About(beatmap, found) => {
                self.about_pending.remove(&beatmap);
                let Some(found) = found else {
                    self.missing.insert(beatmap);
                    self.looked.remove(&beatmap);
                    self.missed_at.insert(beatmap, std::time::Instant::now());
                    return Vec::new();
                };
                self.missing.remove(&beatmap);
                if found.title.trim().is_empty() || found.title.trim() == beatmap.to_string() {
                    self.looked.remove(&beatmap);
                    self.missed_at.insert(beatmap, std::time::Instant::now());
                } else {
                    self.looked.insert(beatmap);
                    self.missed_at.remove(&beatmap);
                }
                let set = found.set;
                self.abouts.insert(beatmap, found);
                self.covered([set])
            }
            Message::Picking(on) => {
                self.picking = on;
                Vec::new()
            }
            Message::AddingMap(on) => {
                if self.posting { return Vec::new(); }
                self.adding_map = on;
                self.map_failed = None;
                if on {
                    self.map_slot = self.band.as_ref().map(|(slot, _)| slot.clone());
                    self.notice = None;
                }
                Vec::new()
            }
            Message::MapLink(said) => {
                self.map_link = said;
                self.map_asked = false;
                self.map_failed = None;
                Vec::new()
            }
            Message::LookMap => {
                self.map_asked = true;
                if let Some(beatmap) = desk::read_map(&self.map_link) {
                    if self.missing.remove(&beatmap) { self.looked.remove(&beatmap); }
                    self.missed_at.remove(&beatmap);
                }
                Vec::new()
            }
            Message::MapSlot(slot) => {
                self.map_slot = slot;
                Vec::new()
            }
            Message::AddMap => {
                let Some(beatmap) = desk::read_map(&self.map_link) else { return Vec::new() };
                if self.posting || self.about(beatmap).is_some_and(|found| !found.standard) { return Vec::new(); }
                self.refreshing = None;
                self.posting = true;
                self.failed = None;
                self.map_failed = None;
                self.notice = None;
                vec![Effect::AddMap { beatmap, slot: self.map_slot.clone(), reason: self.reason.trim().to_owned() }]
            }
            Message::MapAdded(beatmap, result) => {
                self.posting = false;
                let added = match result {
                    Ok(added) => added,
                    Err(why) => {
                        self.map_failed = Some(add_failure(w, &why));
                        return Vec::new();
                    }
                };
                self.map_link.clear();
                self.map_asked = false;
                self.reason.clear();
                self.adding_map = false;
                self.added = Some(beatmap);
                self.loading = true;
                let mut effects = vec![Effect::Load];
                match (self.map_slot.take(), added.task) {
                    (Some(slot), task) => {
                        self.notice = None;
                        if let Some(band) = task.and_then(|task| task.map.stars).map(desk::band_of) {
                            self.band = Some((slot, band));
                            self.held.clear();
                        }
                        if let Some((slot, band)) = &self.band { effects.push(Effect::Cell(slot.clone(), *band)); }
                    }
                    (None, Some(task)) => {
                        self.notice = Some(w.t("desk-map-added-pick"));
                        self.tab = Tab::Queue;
                        self.filter = Filter::All;
                        self.stars = None;
                        self.picked = Some(beatmap);
                        self.moving = false;
                        let set = task.map.beatmapset_id;
                        self.inspected = Some(task);
                        effects.extend(self.covered([set]));
                    }
                    (None, None) => self.notice = Some(w.t("desk-map-added-pick")),
                }
                effects
            }
            Message::Archive => vec![Effect::Archive],
            Message::Open(beatmap) => vec![Effect::Open(map_page(beatmap))],
            Message::Discover => {
                let Some((slot, band)) = self.band.clone() else { return Vec::new() };
                let reason = w.with("desk-discover-reason", &[("slot", slot.clone()), ("band", band_name(w, band))]);
                self.post(Post::Discover { slot, band, reason })
            }
            Message::Archived(found) => {
                self.archive_picked = found.iter().find(|packed| packed.standard).map(|packed| packed.beatmap).into_iter().collect();
                self.notice = found.is_empty().then(|| w.t("desk-archive-empty"));
                self.archive = found;
                Vec::new()
            }
            Message::ArchivePick(beatmap) => {
                if !self.archive_picked.remove(&beatmap) {
                    self.archive_picked.insert(beatmap);
                }
                Vec::new()
            }
            Message::ArchiveDrop => {
                self.archive.clear();
                self.archive_picked.clear();
                Vec::new()
            }
            Message::ArchiveTake => {
                let lines: Vec<String> = self.archive.iter().filter(|packed| self.archive_picked.contains(&packed.beatmap))
                    .map(|packed| format!("https://osu.ppy.sh/b/{}", packed.beatmap)).collect();
                if !lines.is_empty() {
                    let mut text = self.pool.0.text();
                    if !text.is_empty() && !text.ends_with('\n') {
                        text.push('\n');
                    }
                    text.push_str(&lines.join("\n"));
                    self.pool = Pool(text_editor::Content::with_text(&text));
                }
                self.archive.clear();
                self.archive_picked.clear();
                Vec::new()
            }
            Message::Batch(name, year) => {
                self.batch_name = name.clone();
                self.batch_year = year.to_string();
                self.batch = Some((name, year));
                self.batch_sure = None;
                Vec::new()
            }
            Message::Secondary(pick) => {
                let Some(beatmap) = self.task().map(|task| task.map.beatmap_id) else { return Vec::new() };
                self.secondary = Some((beatmap, pick));
                Vec::new()
            }
            Message::BatchName(said) => {
                self.hold_batch();
                self.batch_name = said;
                Vec::new()
            }
            Message::BatchYear(said) => {
                self.hold_batch();
                self.batch_year = said.chars().filter(char::is_ascii_digit).take(4).collect();
                Vec::new()
            }
            Message::RenameBatch => {
                let Some(batch) = self.batch_shown().cloned() else { return Vec::new() };
                let new_name = self.batch_name.trim().to_owned();
                let Ok(new_year) = self.batch_year.parse::<i32>() else { return Vec::new() };
                if new_name.is_empty() || (new_name == batch.name && new_year == batch.year) { return Vec::new(); }
                self.batch = Some((new_name.chars().take(32).collect(), new_year));
                let reason = self.reason.trim().to_owned();
                self.post(Post::RenameBatch { name: batch.name, year: batch.year, new_name, new_year, reason })
            }
            Message::DeleteBatch(beatmap) => {
                let Some(batch) = self.batch_shown().cloned() else { return Vec::new() };
                if self.batch_sure != Some(beatmap) {
                    self.batch_sure = Some(beatmap);
                    return Vec::new();
                }
                self.batch_sure = None;
                if beatmap.is_none() {
                    self.batch = None;
                }
                let reason = self.reason.trim().to_owned();
                self.post(Post::DeleteBatch { name: batch.name, year: batch.year, beatmap, reason })
            }
            Message::ExcludeBatch(beatmap, excluded) => {
                let Some(batch) = self.batch_shown().cloned() else { return Vec::new() };
                self.batch_sure = None;
                let reason = self.reason.trim().to_owned();
                self.post(Post::ExcludeBatch { name: batch.name, year: batch.year, beatmap, excluded, reason })
            }
            Message::Loaded(found) => {
                if let Some(beatmap) = self.pending_review.take() {
                    self.posting = false;
                    if let Ok(desk) = &found {
                        self.inspected = None;
                        self.picked = desk.tasks.iter().any(|task| task.map.beatmap_id == beatmap).then_some(beatmap);
                    }
                }
                self.refreshing = None;
                self.loading = false;
                match found {
                    Ok(desk) => {
                        if self.training.as_ref().is_some_and(|training| training.landed(desk.versions.len())) {
                            self.training = None;
                        }
                        self.refresh_failed = false;
                        self.take_desk(desk);
                        self.prefill_match();
                        if let Some((name, year)) = self.pending_batch.take() {
                            self.tournament = self.tournaments().iter().position(|batch| batch.corpus == "own-v1" && batch.year == year && batch.stage.as_deref() == Some(&name));
                            self.revisiting = None;
                            self.adding = false;
                            self.picking = false;
                            self.notice = Some(w.t("desk-batch-added"));
                        }
                        let sets: Vec<Option<i64>> = self.desk.tasks.iter().map(|task| task.map.beatmapset_id).collect();
                        self.covered(sets)
                    }
                    Err(why) => {
                        self.failed = Some(why);
                        Vec::new()
                    }
                }
            }
            Message::Tab(tab) => {
                if tab == Tab::Label && self.tab == Tab::Label {
                    self.adding = false;
                }
                if tab == Tab::Batches && self.batch.is_none() {
                    if let Some(first) = self.desk.batches.first() {
                        self.batch_name = first.name.clone();
                        self.batch_year = first.year.to_string();
                    }
                }
                self.tab = tab;
                if tab == Tab::Duels {
                    self.prefill_match();
                }
                self.picking = false;
                self.failed = None;
                let sets: Vec<Option<i64>> = match tab {
                    Tab::Label => self.next_map().map(|(_, map)| map.beatmapset_id).into_iter().collect(),
                    _ => Vec::new(),
                };
                self.covered(sets)
            }
            Message::Filter(filter) => {
                self.filter = filter;
                self.picked = None;
                Vec::new()
            }
            Message::NotThis(moving) => {
                self.moving = moving;
                Vec::new()
            }
            Message::Pick(beatmap) => {
                self.inspecting = None;
                self.picked = Some(beatmap);
                self.moving = false;
                self.reason.clear();
                let set = self.task().and_then(|task| task.map.beatmapset_id);
                self.covered([set])
            }
            Message::Reason(said) => {
                self.reason = said;
                Vec::new()
            }
            Message::Decide(decision, slot) => {
                if self.posting || self.loading { return Vec::new(); }
                let Some(task) = self.task() else { return Vec::new() };
                let secondary = matches!(decision, "APPROVE" | "MOVE").then(|| self.secondary_for(task)).map(|pick| pick.filter(|pick| Some(pick) != slot.as_ref().or(task.slot.as_ref())));
                let post = Post::Review { beatmap: task.map.beatmap_id, decision, slot, secondary, reason: self.reason.trim().to_owned() };
                self.pending_review = Some(task.map.beatmap_id);
                self.post(post)
            }
            Message::Posted(said) => {
                self.posting = false;
                if said.is_err() {
                    self.pending_review = None;
                    self.training = None;
                    self.pending_label = None;
                    self.pending_batch = None;
                }
                match said {
                    Ok(()) => {
                        if self.pending_review.is_none() { self.inspected = None; }
                        if let Some((target, beatmap, label)) = self.pending_label.take() {
                            for cell in &mut self.desk.cells {
                                if cell.corpus == target.corpus && cell.year == target.year && cell.stage == target.stage && cell.slot == target.slot {
                                    if let Some(map) = cell.maps.iter_mut().find(|map| map.beatmap_id == beatmap) {
                                        map.label = label.clone();
                                        map.verified = true;
                                    }
                                }
                            }
                            self.skipped.insert(beatmap);
                            self.revisiting = None;
                        }
                        self.reason.clear();
                        self.moving = false;
                        self.confirming = false;
                        self.finalize_sure = false;
                        if self.pending_review.is_none() { self.picked = None; }
                        self.label = None;
                        self.match_id.clear();
                        self.game_id.clear();
                        self.loading = true;
                        self.posting = self.pending_review.is_some();
                        let mut effects = vec![Effect::Load];
                        if let Some((slot, band)) = &self.band { effects.push(Effect::Cell(slot.clone(), *band)); }
                        effects
                    }
                    Err(why) if why == "NOTHING_NEW" => {
                        self.nothing_new = true;
                        Vec::new()
                    }
                    Err(why) => {
                        self.failed = Some(if why == "MODEL_UNAVAILABLE" { w.t("desk-discovery-no-model") } else if why == "BATCH_UNSUPPORTED" { w.t("desk-batch-unsupported") } else { why });
                        Vec::new()
                    }
                }
            }
            Message::Cover(set, handle) => {
                if let Some(handle) = handle {
                    self.covers.insert(set, handle);
                }
                Vec::new()
            }
            Message::Heard => {
                self.hearing = None;
                Vec::new()
            }
            Message::Hear(beatmap) => {
                self.silent = None;
                self.hearing = if self.hearing == Some(beatmap) { None } else { Some(beatmap) };
                vec![Effect::Hear(self.hearing)]
            }
            Message::Year(year) => {
                self.year = Some(year);
                self.cell = None;
                self.label = None;
                Vec::new()
            }
            Message::Cell(at) => {
                self.cell = Some(at);
                self.label = None;
                let sets: Vec<Option<i64>> = self.desk.cells.get(at).map(|cell| cell.maps.iter().map(|map| map.beatmapset_id).collect()).unwrap_or_default();
                self.covered(sets)
            }
            Message::Label(label) => {
                self.label = Some(label);
                Vec::new()
            }
            Message::SaveCell => {
                let Some(cell) = self.cell.and_then(|at| self.desk.cells.get(at)).cloned() else { return Vec::new() };
                let label = self.label.clone().unwrap_or_else(|| cell.shown());
                let reason = self.reason.trim().to_owned();
                self.post(Post::Label { cell, beatmap: None, label, reason })
            }
            Message::Band(slot, band) => {
                self.band = Some((slot.clone(), band));
                self.held.clear();
                vec![Effect::Cell(slot, band)]
            }
            Message::Held(slot, band, found) => {
                if self.band.as_ref() != Some(&(slot, band)) { return Vec::new(); }
                match found {
                    Ok(held) => {
                        self.held = held;
                        let sets: Vec<Option<i64>> = self.held.iter().map(|map| map.beatmapset_id).collect();
                        return self.covered(sets);
                    }
                    Err(why) => self.failed = Some(why),
                }
                Vec::new()
            }
            Message::Version(version) => {
                self.confirming = false;
                self.version = Some(version);
                Vec::new()
            }
            Message::Publish => {
                let Some(version) = self.version.clone().or_else(|| self.desk.versions.first().map(|found| found.version.clone())) else { return Vec::new() };
                let reason = self.reason.trim().to_owned();
                self.post(Post::Publish { version, reason })
            }
            Message::Train => {
                if self.posting || self.training.is_some() { return Vec::new(); }
                self.training = Some(Training::new(self.desk.versions.len()));
                self.training_lost = false;
                self.training_failed = None;
                self.nothing_new = false;
                self.refreshing = None;
                self.posting = true;
                self.failed = None;
                vec![Effect::Train(self.reason.trim().to_owned())]
            }
            Message::TrainStarted(said) => {
                self.posting = false;
                match said {
                    Ok(job) => {
                        if let Some(training) = self.training.as_mut() { training.job = job; }
                        self.reason.clear();
                        self.loading = true;
                        vec![Effect::Load]
                    }
                    Err(why) => {
                        self.training = None;
                        if why == "NOTHING_NEW" { self.nothing_new = true; } else { self.failed = Some(why); }
                        Vec::new()
                    }
                }
            }
            Message::TrainingState(job, said) => {
                let Some(training) = self.training.as_mut().filter(|training| training.job.as_ref() == Some(&job)) else { return Vec::new() };
                training.asking = false;
                let found = match said {
                    Ok(found) => found,
                    Err(why) if why == "UNKNOWN_JOB" => {
                        self.training = None;
                        self.training_lost = true;
                        return Vec::new();
                    }
                    Err(_) => return Vec::new(),
                };
                match found.state.as_str() {
                    "done" => {
                        self.training = None;
                        self.version = found.version;
                        self.loading = true;
                        vec![Effect::Load]
                    }
                    "nothing_new" => {
                        self.training = None;
                        self.nothing_new = true;
                        Vec::new()
                    }
                    "failed" => {
                        self.training = None;
                        self.training_failed = Some(found.error.unwrap_or_default());
                        Vec::new()
                    }
                    _ => Vec::new(),
                }
            }
            Message::Delete => {
                let Some(version) = self.version.clone().or_else(|| self.desk.versions.first().map(|found| found.version.clone())) else { return Vec::new() };
                if !self.confirming {
                    self.confirming = true;
                    return Vec::new();
                }
                let reason = self.reason.trim().to_owned();
                self.version = None;
                self.post(Post::Delete { version, reason })
            }
            Message::Again => {
                if self.posting { return Vec::new(); }
                self.refreshing = None;
                if self.training.as_ref().is_some_and(Training::overdue) {
                    self.training = None;
                    self.training_lost = true;
                    return Vec::new();
                }
                if self.loading {
                    return Vec::new();
                }
                self.loading = true;
                vec![Effect::Load]
            }
            Message::Rollback => {
                let reason = self.reason.trim().to_owned();
                self.post(Post::Rollback { reason })
            }
            Message::Stuck(duel) => {
                self.confirming = false;
                self.finalize_sure = false;
                self.stuck = Some(duel);
                self.match_id.clear();
                self.game_id.clear();
                self.prefill_match();
                Vec::new()
            }
            Message::MatchId(said) => {
                self.match_id = crate::desk::match_of(&said).map(|found| found.to_string()).unwrap_or_else(|| said.chars().filter(char::is_ascii_digit).collect());
                self.finalize_sure = false;
                Vec::new()
            }
            Message::GameId(said) => {
                self.game_id = said.chars().filter(char::is_ascii_digit).collect();
                self.finalize_sure = false;
                Vec::new()
            }
            Message::Finalize => {
                let Some(duel) = self.stuck_shown().map(|found| found.id.clone()) else { return Vec::new() };
                let (Ok(match_id), Ok(game_id)) = (self.match_id.parse::<i64>(), self.game_id.parse::<i64>()) else { return Vec::new() };
                if !self.finalize_sure {
                    self.finalize_sure = true;
                    return Vec::new();
                }
                self.finalize_sure = false;
                let reason = self.reason.trim().to_owned();
                self.post(Post::Finalize { duel, match_id, game_id, reason })
            }
            Message::OpenMatch(match_id) => vec![Effect::Open(format!("https://osu.ppy.sh/community/matches/{match_id}"))],
            Message::JournalKind(kind) => {
                self.journal_kind = kind;
                Vec::new()
            }
            Message::JournalPerson(player) => {
                self.journal_person = player;
                Vec::new()
            }
            Message::JournalMore => {
                let Some(least) = self.desk.journal.iter().map(|entry| entry.id).min() else { return Vec::new() };
                if self.journal_more || self.journal_end { return Vec::new(); }
                self.journal_more = true;
                vec![Effect::Older(least)]
            }
            Message::JournalOlder(found) => {
                self.journal_more = false;
                match found {
                    Ok(older) => {
                        let least = self.desk.journal.iter().map(|entry| entry.id).min().unwrap_or(i64::MAX);
                        let fresh: Vec<_> = older.into_iter().filter(|entry| entry.id < least).collect();
                        self.journal_end = fresh.is_empty();
                        self.desk.journal.extend(fresh);
                    }
                    Err(why) => self.failed = Some(why),
                }
                Vec::new()
            }
            Message::CancelDuel => {
                let Some(duel) = self.stuck_shown().map(|found| found.id.clone()) else { return Vec::new() };
                if !self.confirming {
                    self.confirming = true;
                    return Vec::new();
                }
                let reason = self.reason.trim().to_owned();
                self.post(Post::Cancel { duel, reason })
            }
            Message::Tournament(at) => {
                if self.posting { return Vec::new(); }
                self.tournament = Some(at);
                self.revisiting = None;
                self.adding = false;
                self.picking = false;
                let sets: Vec<Option<i64>> = self.next_map().map(|(_, map)| map.beatmapset_id).into_iter().collect();
                self.covered(sets)
            }
            Message::MapLabel(label) => {
                if self.posting { return Vec::new(); }
                let Some((cell, map)) = self.next_map().map(|(cell, map)| (cell.clone(), map.clone())) else { return Vec::new() };
                self.pending_label = Some((cell.clone(), map.beatmap_id, label.clone()));
                let mut effects = self.post(Post::Label { cell, beatmap: Some(map.beatmap_id), label, reason: w.t("desk-label-reason") });
                let sets: Vec<Option<i64>> = self.next_map().map(|(_, map)| map.beatmapset_id).into_iter().collect();
                effects.extend(self.covered(sets));
                effects
            }
            Message::Revisit(cell, beatmap) => {
                if self.posting { return Vec::new(); }
                self.revisiting = Some((cell, beatmap));
                self.failed = None;
                let sets: Vec<_> = self.next_map().map(|(_, map)| map.beatmapset_id).into_iter().collect();
                self.covered(sets)
            }
            Message::ResumeLabels => {
                if self.posting { return Vec::new(); }
                self.revisiting = None;
                self.pending_label = None;
                let sets: Vec<_> = self.next_map().map(|(_, map)| map.beatmapset_id).into_iter().collect();
                self.covered(sets)
            }
            Message::Skip => {
                if self.posting { return Vec::new(); }
                self.pending_label = None;
                if let Some(beatmap) = self.next_map().map(|(_, map)| map.beatmap_id) {
                    self.skipped.insert(beatmap);
                }
                let sets: Vec<Option<i64>> = self.next_map().map(|(_, map)| map.beatmapset_id).into_iter().collect();
                self.covered(sets)
            }
            Message::Adding(on) => {
                self.adding = on;
                self.picking = false;
                Vec::new()
            }
            Message::Name(said) => {
                self.name = said;
                Vec::new()
            }
            Message::YearText(said) => {
                self.year_text = said.chars().filter(char::is_ascii_digit).take(4).collect();
                Vec::new()
            }
            Message::Pool(action) => {
                self.pool.0.perform(action);
                Vec::new()
            }
            Message::AddTournament => {
                let year = if self.year_text.is_empty() { 0 } else if let Ok(year) = self.year_text.parse::<i32>() { year } else { return Vec::new() };
                let pool = self.pool.0.text();
                if self.name.trim().is_empty() || pool.trim().is_empty() {
                    return Vec::new();
                }
                let post = Post::Tournament { name: self.name.trim().to_owned(), year, pool, reason: w.with("desk-tournament-reason", &[("name", self.name.trim().to_owned())]) };
                if self.posting { return Vec::new(); }
                self.pending_batch = Some((self.name.trim().chars().take(32).collect(), year));
                self.notice = None;
                self.post(post)
            }
            Message::Player(said) => {
                self.player = said;
                Vec::new()
            }
            Message::Role(role) => {
                self.role = Some(role);
                Vec::new()
            }
            Message::Grant => {
                let (who, Some(role)) = (self.player.trim().to_owned(), self.role) else { return Vec::new() };
                if who.is_empty() {
                    return Vec::new();
                }
                let reason = self.reason.trim().to_owned();
                self.post(Post::Grant { who, role: role.to_owned(), reason })
            }
            Message::Revoke(player, role) => {
                if self.revoking.as_ref() != Some(&(player, role.clone())) {
                    self.revoking = Some((player, role));
                    return Vec::new();
                }
                self.revoking = None;
                let reason = self.reason.trim().to_owned();
                self.post(Post::Revoke { player, role, reason })
            }
        }
    }

    fn stuck_shown(&self) -> Option<&desk::Stuck> {
        self.stuck.as_ref().and_then(|id| self.desk.stuck.iter().find(|found| found.id == *id)).or_else(|| self.desk.stuck.first())
    }
}

fn add_failure(w: &Words, why: &str) -> String {
    match why {
        "MAP_NOT_FOUND" | "NOT_STANDARD" | "MAP_LENGTH" | "NO_FILE" | "PARSE_FAILED" | "REJECTED_STAYS" | "UNKNOWN_SLOT" | "SERVICE_RECOVERING" | "FORBIDDEN" | "STARS_RANGE" => {
            w.t(&format!("desk-add-map-{}", why.to_lowercase().replace('_', "-")))
        }
        "404" | "405" => w.t("desk-add-map-unsupported"),
        _ => w.with("desk-failed", &[("why", why.to_owned())]),
    }
}

fn admitted_line(w: &Words, report: &serde_json::Value) -> Option<String> {
    let admitted = report.pointer("/test/admitted")?;
    let number = |key: &str| admitted.pointer(key).and_then(serde_json::Value::as_u64).unwrap_or_default();
    let part = |key: &str| format!("{:.0}%", admitted.pointer(key).and_then(serde_json::Value::as_f64).unwrap_or_default() * 100.0);
    Some(w.with("desk-admitted", &[("count", w.lang().group(number("/count"))), ("checked", w.lang().group(number("/checked"))),
        ("precision", part("/precision")), ("refused", w.lang().group(number("/refused/count")))]))
}

pub fn version_name(w: &Words, version: &str) -> String {
    match version.strip_prefix("kamerton-").and_then(|number| number.parse::<u32>().ok()) {
        Some(number) => w.with("desk-kamerton", &[("number", number.to_string())]),
        None => short_version(version),
    }
}

pub fn short_version(version: &str) -> String {
    let parts: Vec<&str> = version.split('-').collect();
    match parts.as_slice() {
        [name, stamp, count] if stamp.len() == 12 && stamp.chars().all(|c| c.is_ascii_digit()) => format!("{name}-{}-{count}", &stamp[4..8]),
        _ => version.to_owned(),
    }
}

fn skill(w: &Words, slot: &str) -> String {
    let key = format!("desk-skill-{}", slot.to_lowercase());
    let said = w.t(&key);
    if said == format!("[{key}]") { String::new() } else { said }
}

fn stage_name(w: &Words, stage: &str) -> String {
    let key = format!("desk-stage-{}", stage.replace(' ', "-"));
    let said = w.t(&key);
    if said == key { stage.to_owned() } else { said }
}

fn feature(w: &Words, path: &str) -> String {
    if let Some((mods, inner)) = path.split_once(':') {
        return w.with("desk-feature-with", &[("feature", feature(w, inner)), ("mods", mods.trim_start_matches("NF").to_owned())]);
    }
    let key = format!("desk-feature-{}", path.replace(['.', '_'], "-"));
    let said = w.t(&key);
    if said == format!("[{key}]") { path.to_owned() } else { said }
}

fn ago(w: &Words, stamp: Option<&str>, now: i64) -> String {
    let Some(at) = stamp.and_then(|stamp| chrono::DateTime::parse_from_rfc3339(stamp).ok()) else { return String::new() };
    let seconds = (now - at.timestamp()).max(0);
    match seconds {
        0..120 => w.t("duels-just-now"),
        120..3600 => w.with("desk-minutes", &[("count", (seconds / 60).to_string())]),
        3600..86_400 => w.with("desk-hours", &[("count", (seconds / 3600).to_string())]),
        86_400..172_800 => w.t("desk-yesterday"),
        _ => w.n("desk-days", (seconds / 86_400) as u64),
    }
}

fn panel<'a>(inside: Element<'a, Message>) -> Element<'a, Message> {
    container(inside).padding(workspace::INSET).width(Length::Fill).height(Length::Fill).style(workspace::surface).into()
}

fn tight<'a>(said: String, size: f32, colour: Color) -> Element<'a, Message> {
    text(said).size(size).font(theme::SANS_SEMI).color(colour).wrapping(iced::widget::text::Wrapping::None).into()
}

fn chip_k<'a>(said: String, on: bool, press: Message, k: f32) -> Element<'a, Message> {
    button(container(words(said, 13.0 * k, if on { INK } else { MUTED })).center_y(30.0 * k))
        .padding([0.0, 12.0 * k])
        .style(workspace::choice(on))
        .on_press(press)
        .into()
}

fn main_button_k<'a>(said: String, on: Option<Message>, k: f32) -> Element<'a, Message> {
    let made = button(container(tight(said, theme::BODY * k, Color::WHITE)).center_x(Length::Fill).center_y((theme::CONTROL_HEIGHT + 6.0) * k))
        .width(Length::Fill)
        .padding([0.0, 14.0 * k])
        .style(ui::button_faded(theme::primary));
    match on {
        Some(message) => made.on_press(message).into(),
        None => made.into(),
    }
}

pub fn map_page(beatmap: i64) -> String {
    format!("https://osu.ppy.sh/b/{beatmap}")
}

fn link_button<'a>(beatmap: i64, side: f32) -> Element<'a, Message> {
    button(container(glyph(Icon::External, 16.0 * (side / 38.0).max(32.0 / 38.0), INK)).center_x(side).center_y(side))
        .padding(0)
        .style(quiet(false, theme::CONTROL_RADIUS))
        .on_press(Message::Open(beatmap))
        .into()
}

fn listen_button<'a>(w: &Words, state: &State, beatmap: i64, high: f32) -> Element<'a, Message> {
    let on = state.hearing == Some(beatmap);
    let said = if state.silent == Some(beatmap) { w.t("desk-no-preview") } else { w.t("desk-listen") };
    let k = (high / 38.0).max(32.0 / 38.0);
    let inside = row![glyph(Icon::Volume, 16.0 * k, INK), tight(said, theme::BODY * k, INK)].spacing(8).align_y(Alignment::Center);
    button(container(inside).center_x(Length::Fill).center_y(high))
        .width(Length::Fill)
        .padding([0, 12])
        .style(quiet(on, theme::CONTROL_RADIUS))
        .on_press(Message::Hear(beatmap))
        .into()
}

fn line_button_k<'a>(said: String, on: Option<Message>, k: f32) -> Element<'a, Message> {
    button(container(tight(said, theme::BODY * k, INK)).center_x(Length::Fill).center_y((theme::CONTROL_HEIGHT + 6.0) * k)).padding([0.0, 12.0 * k]).style(quiet(false, theme::CONTROL_RADIUS)).on_press_maybe(on).into()
}

fn quiet(on: bool, radius: f32) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |theme, status| {
        let mut style = workspace::choice(on)(theme, status);
        style.border.radius = radius.into();
        style
    }
}

fn field_k<'a>(hint: String, value: &str, on: fn(String) -> Message, k: f32) -> Element<'a, Message> {
    text_input(&hint, value).on_input(on).font(theme::SANS).size(15.0 * k).padding([11.0 * k, 14.0 * k]).width(Length::Fill).style(theme::field).into()
}

fn heading_k<'a>(k: f32, said: String) -> Element<'a, Message> {
    strong(said, 17.0 * k, INK)
}

fn main_button_kk<'a>(k: f32, said: String, on: Option<Message>) -> Element<'a, Message> {
    main_button_k(said, on, k)
}

fn field_kk<'a>(k: f32, hint: String, value: &str, on: fn(String) -> Message) -> Element<'a, Message> {
    field_k(hint, value, on, k)
}

fn labelled_k<'a>(k: f32, caption: String, inside: Element<'a, Message>) -> Element<'a, Message> {
    column![words(caption, 13.0 * k, FAINT), inside].spacing(6).width(Length::Fill).into()
}

fn tile<'a>(fill: Color, frame: Option<(Color, f32)>, inside: Element<'a, Message>, press: Option<Message>) -> Element<'a, Message> {
    let made = container(inside).padding([9, 9]).width(Length::Fill).style(move |_| container::Style {
        background: Some(Background::Color(fill)),
        border: frame.map_or(Border { radius: 12.0.into(), ..Border::default() }, |(colour, width)| Border { color: colour, width, radius: 12.0.into() }),
        ..container::Style::default()
    });
    match press {
        Some(message) => mouse_area(made).on_press(message).interaction(mouse::Interaction::Pointer).into(),
        None => made.into(),
    }
}

fn cover<'a>(state: &State, set: Option<i64>, wide: f32, high: f32, radius: f32) -> Element<'a, Message> {
    let base = container(Space::new()).width(wide).height(high).style(move |_| container::Style {
        background: Some(Background::Color(ROW)),
        border: Border { radius: radius.into(), ..Border::default() },
        ..container::Style::default()
    });
    match set.and_then(|set| state.covers.get(&set)) {
        Some(handle) => stack![base, container(image(handle.clone()).width(wide - 2.0).height(high - 2.0).content_fit(iced::ContentFit::Cover).border_radius((radius - 1.0).max(0.0))).padding(1)].into(),
        None => base.into(),
    }
}

fn one_line<'a>(said: String, size: f32, colour: Color, font: iced::Font) -> Element<'a, Message> {
    container(text(said).size(size).font(font).color(colour).wrapping(iced::widget::text::Wrapping::None)).width(Length::Fill).clip(true).into()
}

fn titled_map<'a>(title: Option<&str>, beatmap: i64, size: f32) -> Element<'a, Message> {
    one_line(title.map(str::to_owned).unwrap_or_else(|| beatmap.to_string()), size, INK, theme::SANS_SEMI)
}

fn stars_of<'a>(stars: Option<f32>, size: f32) -> Element<'a, Message> {
    match stars {
        Some(value) => crate::dossier::star_pill(format!("{value:.2}").replace('.', ","), value, size),
        None => Space::new().into(),
    }
}

fn tabs<'a>(w: &Words, state: &State, roles: &[String], k: f32) -> Element<'a, Message> {
    let mut line = row![].spacing(4).align_y(Alignment::Center);
    for tab in Tab::ALL.into_iter().filter(|tab| tab.open(roles)) {
        let on = tab == state.tab;
        let mut inside = row![strong(w.t(tab.key()), 15.0 * k, if on { INK } else { MUTED })].spacing(8).align_y(Alignment::Center);
        let count = match tab {
            Tab::Queue => state.tasks().len(),
            Tab::Duels => state.desk.stuck.len(),
            _ => 0,
        };
        if count > 0 {
            inside = inside.push(container(strong(count.to_string(), 12.0 * k, Color::WHITE)).padding([1.0 * k, 7.0 * k]).style(|_| container::Style {
                background: Some(Background::Color(ACCENT)),
                border: Border { radius: 9.0.into(), ..Border::default() },
                ..container::Style::default()
            }));
        }
        let made = button(container(inside).center_y(40.0 * k)).padding([0.0, 14.0 * k]).on_press(Message::Tab(tab)).style(move |theme, status| { let mut style = workspace::choice(on)(theme, status); style.border.width = 0.0; style });
        line = line.push(occupancy::mark(state, made.into(), on, 8.0, |person| person.section == tab.key().trim_start_matches("desk-tab-")));
    }
    container(container(line.wrap().vertical_spacing(6)).id(iced::widget::Id::new("desk-tabs"))).center_x(Length::Fill).into()
}

fn reason_colour(task: &Task) -> Color {
    match task.reason.as_str() {
        "uncertain_revealed" | "uncertain" => UNSURE,
        "unassigned" => MUTED,
        "manual" => MUTED,
        _ => BAD,
    }
}

fn queue_view<'a>(w: &Words, state: &'a State, now: i64, k: f32, wide: f32) -> Element<'a, Message> {
    let mut chips = row![].spacing(6);
    for filter in Filter::ALL {
        let count = state.matching_tasks(filter).iter().map(|task| task.map.beatmapset_id.map(|id| (true, id)).unwrap_or((false, task.map.beatmap_id))).collect::<HashSet<_>>().len();
        chips = chips.push(chip_k(format!("{} {count}", w.t(filter.key())), state.filter == filter, Message::Filter(filter), k));
    }
    let picked = state.task().map(|task| task.map.beatmap_id);
    let mut list = column![].spacing(6);
    for task in state.tasks() {
        let map = state.shown_map(&task.map);
        let on = picked == Some(task.map.beatmap_id) || state.task().is_some_and(|chosen| task.map.beatmapset_id.is_some() && task.map.beatmapset_id == chosen.map.beatmapset_id);
        let mut tags = row![].spacing(6).align_y(Alignment::Center);
        if let Some(slot) = task.slot.as_deref().filter(|_| task.kind != "unassigned") {
            tags = tags.push(tag(slot, k));
        }
        tags = tags.push(strong(w.t(&format!("desk-reason-{}", task.reason.replace('_', "-"))), 12.0 * k, reason_colour(task)));
        let inside = row![
            cover(state, map.beatmapset_id, 60.0 * k, 38.0 * k, 7.0),
            column![titled_map(map.title.as_deref(), map.beatmap_id, 14.0 * k), one_line(map.version.clone().unwrap_or_default(), 12.0 * k, MUTED, theme::SANS), tags].spacing(3).width(Length::Fill),
            tight(ago(w, task.since.as_deref(), now), 12.0 * k, FAINT),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        let fill = if on { LIT } else { ROW };
        list = list.push(occupancy::mark(state, tile(fill, None, inside.into(), Some(Message::Pick(task.map.beatmap_id))), on, 12.0, |person| person.beatmap == Some(map.beatmap_id)));
    }
    if state.tasks().is_empty() {
        list = list.push(words(w.t(if state.loading { "desk-loading" } else { "desk-queue-empty" }), 14.0, FAINT));
    }
    let (low, high) = state.stars.unwrap_or((0.0, crate::carousel::STARS_TOP));
    let edge = |value| if value >= crate::carousel::STARS_TOP { "∞".to_owned() } else { w.decimal(value, 1) };
    let ranges = column![
        row![words(w.t("filter-stars"), 13.0 * k, MUTED), Space::new().width(Length::Fill), tight(format!("{}–{}", edge(low), edge(high)), 13.0 * k, INK)],
        iced::widget::canvas(crate::main_screen::select::StarBar { low, high, alpha: 1.0, on_at: Message::StarsAt, on_done: || Message::StarsDone }).width(Length::Fill).height(crate::main_screen::select::BAR_H),
    ].spacing(4);
    let ranges = container(ranges).id(iced::widget::Id::new("desk-star-range"));
    let aside = container(column![chips.wrap().vertical_spacing(6), ranges, scrollable(list).direction(ui::hidden_bar()).height(Length::Fill)].spacing(10)).padding(14).width(330.0 * k).height(Length::Fill).style(theme::slab);
    let detail: Element<'a, Message> = match state.task() {
        Some(task) => task_detail(w, state, task, k, wide),
        None => container(words(w.t(if state.loading { "desk-loading" } else { "desk-queue-empty" }), 14.0, FAINT)).padding(20).width(Length::Fill).into(),
    };
    row![aside, container(detail).max_width(1040.0)].spacing(14).height(Length::Fill).into()
}

fn task_detail<'a>(w: &Words, state: &'a State, task: &Task, k: f32, wide: f32) -> Element<'a, Message> {
    let map = &state.shown_map(&task.map);
    let card = stack![
        cover(state, map.beatmapset_id, 280.0 * k, 160.0 * k, 14.0),
        container(column![
            row![task.slot.as_deref().filter(|_| task.kind != "unassigned").map(|slot| tag(slot, k)).unwrap_or_else(|| Space::new().into()), Space::new().width(Length::Fill), container(stars_of(map.stars, 12.0 * k)).id(iced::widget::Id::new("desk-map-stars"))].align_y(Alignment::Center),
            Space::new().height(Length::Fill),
            one_line(map.version.clone().unwrap_or_default(), 15.0 * k, INK, theme::SANS_SEMI),
            titled_map(map.title.as_deref(), map.beatmap_id, 17.0 * k),
            one_line(map.artist.clone().unwrap_or_default(), 13.0 * k, MUTED, theme::SANS),
            row![crate::duels_screen::mapper_face(map.mapper.as_deref().unwrap_or_default(), map.beatmap_id, 18.0 * k), words(map.mapper.clone().unwrap_or_default(), 13.0 * k, MUTED)].spacing(6).align_y(Alignment::Center),
        ].spacing(1))
        .padding(12.0 * k)
        .width(280.0 * k)
        .height(160.0 * k)
        .align_y(iced::alignment::Vertical::Bottom)
        .style(|_| container::Style { background: Some(crate::duel_view::veil_fill()), border: Border { radius: 14.0.into(), ..Border::default() }, ..container::Style::default() }),
    ];
    let card = occupancy::mark(state, card.into(), true, 14.0, |person| person.beatmap == Some(map.beatmap_id));
    let mut difficulties = row![].spacing(6);
    if let Some(set) = map.beatmapset_id {
        let versions: Vec<_> = state.matching_tasks(state.filter).into_iter().filter(|other| other.map.beatmapset_id == Some(set)).collect();
        for other in versions.iter().filter(|_| versions.len() > 1) {
            difficulties = difficulties.push(chip_k(other.map.version.clone().unwrap_or_else(|| other.map.beatmap_id.to_string()), other.map.beatmap_id == map.beatmap_id, Message::Pick(other.map.beatmap_id), k));
        }
    }
    let ready = !state.posting;
    let ask = |message: Message| ready.then_some(message);
    let (caption, question, answers): (String, String, Element<'a, Message>) = match (task.kind.as_str(), task.slot.clone()) {
        ("review", Some(slot)) if state.moving => {
            let mut buttons = row![].spacing(6).align_y(Alignment::Center);
            for other in desk::SLOTS.into_iter().filter(|other| *other != slot) {
                let made = button(container(tag(other, k)).center_y(36.0 * k)).padding([0.0, 6.0 * k]).style(quiet(false, 9.0));
                buttons = buttons.push(match ask(Message::Decide("MOVE", Some(other.to_owned()))) {
                    Some(message) => made.on_press(message),
                    None => made,
                });
            }
            buttons = buttons.push(container(line_button_k(w.t("desk-fits-none"), ask(Message::Decide("REJECT", None)), k)).width(Length::Shrink));
            buttons = buttons.push(container(line_button_k(w.t("desk-back"), Some(Message::NotThis(false)), k)).width(Length::Shrink));
            let answers = column![buttons.wrap().vertical_spacing(6), spare_slots(w, state, "MOVE", ready, k)].spacing(10);
            (w.with("desk-not-slot", &[("slot", slot.clone())]), w.t("desk-which-then"), answers.into())
        }
        ("review", Some(slot)) => {
            let caption = if task.reason == "manual" { w.t("desk-edit-map") } else if task.reason.starts_with("uncertain") { w.with("desk-ask-uncertain", &[("slot", slot.clone())]) } else { w.with("desk-ask-feedback", &[("slot", slot.clone())]) };
            let buttons = row![
                container(process_button(w, state, w.with("desk-yes", &[("slot", slot.clone())]), ask(Message::Decide("APPROVE", None)), k)).width(Length::Shrink),
                container(line_button_k(w.with("desk-no", &[("slot", slot.clone())]), (!state.posting).then_some(Message::NotThis(true)), k)).width(Length::Shrink),
                container(line_button_k(w.t("desk-quarantine"), ask(Message::Decide("QUARANTINE", None)), k)).width(Length::Shrink),
                container(line_button_k(w.t("desk-remove-map"), ask(Message::Decide("REJECT", None)), k)).width(Length::Shrink),
            ]
            .spacing(8)
            .wrap()
            .vertical_spacing(8);
            (caption, w.with("desk-is-it", &[("skill", first_lower(&skill(w, &slot)))]), buttons.into())
        }
        _ => {
            let mut buttons = row![].spacing(6).align_y(Alignment::Center);
            for slot in desk::SLOTS {
                let made = button(container(tag(slot, k)).center_y(36.0 * k)).padding([0.0, 6.0 * k]).style(quiet(false, 9.0));
                buttons = buttons.push(match ask(Message::Decide("APPROVE", Some(slot.to_owned()))) {
                    Some(message) => made.on_press(message),
                    None => made,
                });
            }
            buttons = buttons.push(container(line_button_k(w.t("desk-quarantine"), ask(Message::Decide("QUARANTINE", None)), k)).width(Length::Shrink));
            let answers = column![buttons.wrap().vertical_spacing(6), spare_slots(w, state, "APPROVE", ready, k)].spacing(10);
            (w.t("desk-ask-unassigned"), w.t("desk-which-slot"), answers.into())
        }
    };
    let under = row![listen_button(w, state, map.beatmap_id, 38.0 * k), link_button(map.beatmap_id, 38.0 * k)].spacing(8).width(280.0 * k);
    let mut facts = row![].spacing(6).width(280.0 * k);
    for (key, name) in [("cs", "CS"), ("ar", "AR"), ("od", "OD"), ("hp", "HP")] {
        if let Some(value) = map.settings.get(key) {
            facts = facts.push(fact(name.to_owned(), w.decimal(f64::from(*value), 1), k));
        }
    }
    if let Some(bpm) = map.bpm.filter(|bpm| bpm.is_finite() && *bpm > 0.0) {
        facts = facts.push(fact("BPM".into(), w.decimal(f64::from(bpm), 0), k));
    }
    if let Some(length) = map.length.filter(|length| *length > 0.0) {
        let seconds = length.round() as u32;
        facts = facts.push(fact(w.t("desk-length"), format!("{}:{:02}", seconds / 60, seconds % 60), k));
    }
    let reason = column![words(w.t("desk-reason"), 13.0 * k, MUTED), field_k(String::new(), &state.reason, Message::Reason, k)].spacing(6);
    let decision = column![words(caption, 14.0 * k, FAINT), bold(question, 26.0 * k, INK), container(answers).padding(Padding { top: 6.0 * k, bottom: 10.0 * k, ..Padding::ZERO }), secondary_row(w, state, task, k), reason]
        .spacing(8)
        .width(Length::Fill);
    let played: Element<'a, Message> = match (map.mods.is_empty(), map.plain_stars) {
        (false, Some(plain)) => row![
            container(strong(map.mods.join(""), 12.0 * k, INK)).padding([2.0 * k, 8.0 * k]).style(|_| container::Style { background: Some(Background::Color(LIT)), border: Border { radius: 6.0.into(), ..Border::default() }, ..container::Style::default() }),
            meta_pill("NM".into(), k), stars_of(Some(plain), 12.0 * k),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into(),
        _ => Space::new().into(),
    };
    let status = map.ranked_status.as_deref().filter(|status| !status.is_empty()).map(|status| status_pill(w, status, k)).unwrap_or_else(|| Space::new().into());
    let summary = column![card, row![played, status].spacing(8).align_y(Alignment::Center), facts.wrap().vertical_spacing(6), under].spacing(8);
    let top: Element<'a, Message> = if wide < 800.0 {
        column![summary, decision].spacing(16).into()
    } else { row![summary, decision].spacing(22.0 * k).into() };
    let mut why = column![].spacing(8);
    if let Some(guess) = &map.model {
        for odds in guess.top.iter().filter(|odds| odds.slot != desk::OTHER) {
            let chosen = Some(&odds.slot) == task.slot.as_ref() || (task.slot.is_none() && Some(&odds.slot) == guess.slot.as_ref());
            why = why.push(row![container(tag(&odds.slot, k)).width(56.0 * k), bar(odds.probability, if chosen { ACCENT } else { MUTED }), container(strong(format!("{:.0}%", odds.probability * 100.0), 14.0 * k, INK)).width(44.0 * k).align_x(iced::alignment::Horizontal::Right)].spacing(10).align_y(Alignment::Center));
        }
        let mut reasons = column![].spacing(5).padding(Padding { top: 6.0 * k, ..Padding::ZERO });
        for reason in guess.reasons.iter().take(3) {
            let (sign, colour) = if reason.weight >= 0.0 { ("+", GOOD) } else { ("−", BAD) };
            let level = match reason.level {
                Some(level) if level > 0.5 => w.t("desk-level-above"),
                Some(level) if level < -0.5 => w.t("desk-level-below"),
                Some(_) => w.t("desk-level-usual"),
                None => String::new(),
            };
            reasons = reasons.push(row![container(strong(sign.to_owned(), 14.0 * k, colour)).width(14.0 * k), words(feature(w, &reason.feature), 14.0 * k, MUTED), words(level, 13.0 * k, FAINT)].spacing(10).align_y(Alignment::Center));
        }
        why = why.push(reasons);
    } else {
        why = why.push(words(w.t("desk-no-model"), 14.0 * k, FAINT));
    }
    let version = map.model.as_ref().map(|guess| version_name(w, &guess.version)).unwrap_or_default();
    let model_side = block(row![strong(w.t("desk-why"), 17.0 * k, INK), container(tight(version, 13.0 * k, FAINT)).width(Length::Fill).align_x(iced::alignment::Horizontal::Right).clip(true)].spacing(12).align_y(Alignment::Center).into(), why.into(), k);
    let said: Element<'a, Message> = match &task.opinions {
        Some(opinions) => {
            let voted = opinions.fits + opinions.off;
            let difficulty = opinions.difficulty.iter().max_by_key(|(_, count)| **count).map(|(label, _)| w.t(&format!("desk-difficulty-{label}"))).unwrap_or_else(|| "—".to_owned());
            let slot = task.slot.clone().unwrap_or_default();
            let stat = |value: String, label: String, colour: Color| -> Element<'a, Message> { row![words(label, 14.0 * k, MUTED), Space::new().width(Length::Fill), bold(value, 18.0 * k, colour)].spacing(12).align_y(Alignment::Center).into() };
            column![
                stat(opinions.played.to_string(), w.t("desk-played"), INK),
                stat(w.with("desk-of", &[("count", opinions.off.to_string()), ("total", voted.to_string())]), w.with("desk-off", &[("slot", slot)]), if opinions.off > 0 { BAD } else { INK }),
                stat(difficulty, w.t("desk-difficulty"), INK),
            ]
            .spacing(12.0 * k)
            .into()
        }
        None => words(w.t("desk-no-opinions"), 14.0 * k, FAINT),
    };
    let players = block(strong(w.t("desk-players-say"), 17.0 * k, INK), said, k);
    let info: Element<'a, Message> = if wide >= 1300.0 { row![container(model_side).width(Length::FillPortion(2)), container(players).width(Length::Fill)].spacing(14).into() } else { column![model_side, players].spacing(14).into() };
    container(scrollable(column![difficulties.wrap(), top, info].spacing(18.0 * k)).direction(ui::hidden_bar()))
        .padding(workspace::INSET).width(Length::Fill).style(workspace::surface).into()
}

fn secondary_row<'a>(w: &Words, state: &State, task: &Task, k: f32) -> Element<'a, Message> {
    let primary = task.slot.clone().or_else(|| task.map.model.as_ref().and_then(|guess| guess.slot.clone()));
    let chosen = state.secondary_for(task);
    let hint = task.map.model.as_ref().and_then(|guess| {
        guess.top.iter().filter(|odds| odds.slot != desk::OTHER && Some(&odds.slot) != primary.as_ref())
            .max_by(|a, b| a.probability.total_cmp(&b.probability)).filter(|odds| odds.probability >= 0.25).cloned()
    });
    let mut chips = row![].spacing(6).align_y(Alignment::Center);
    for slot in desk::SLOTS.iter().map(|slot| (*slot).to_owned()).chain(state.reserve()) {
        if Some(&slot) == primary.as_ref() {
            continue;
        }
        let on = chosen.as_ref() == Some(&slot);
        let suggested = hint.as_ref().is_some_and(|odds| odds.slot == slot);
        let made = button(container(tag(&slot, k)).center_y(30.0 * k)).padding([0.0, 5.0 * k]).style(quiet(on || suggested, 9.0));
        chips = chips.push(if state.posting { made } else { made.on_press(Message::Secondary(Some(slot))) });
    }
    chips = chips.push(chip_k(w.t("desk-secondary-none"), chosen.is_none(), Message::Secondary(None), k));
    let mut body = column![words(w.t("desk-secondary"), 13.0 * k, FAINT), chips.wrap().vertical_spacing(6)].spacing(6);
    if let Some(odds) = hint {
        body = body.push(words(w.with("desk-secondary-hint", &[("slot", odds.slot.clone()), ("share", format!("{:.0}", odds.probability * 100.0))]), 12.0 * k, UNSURE));
    }
    container(body).padding(Padding { bottom: 6.0 * k, ..Padding::ZERO }).into()
}

fn spare_slots<'a>(w: &Words, state: &State, decision: &'static str, ready: bool, k: f32) -> Element<'a, Message> {
    let mut chips = row![].spacing(6).align_y(Alignment::Center);
    for slot in state.reserve() {
        let made = button(container(tag(&slot, k)).center_y(32.0 * k)).padding([0.0, 6.0 * k]).style(quiet(false, 9.0));
        chips = chips.push(if ready { made.on_press(Message::Decide(decision, Some(slot))) } else { made });
    }
    column![words(w.t("desk-reserve"), 13.0 * k, FAINT), chips.wrap().vertical_spacing(6)].spacing(6).into()
}

fn first_upper(said: &str) -> String {
    let mut letters = said.chars();
    letters.next().map(|first| first.to_uppercase().chain(letters).collect()).unwrap_or_default()
}

fn first_lower(said: &str) -> String {
    let mut letters = said.chars();
    letters.next().map(|first| first.to_lowercase().chain(letters).collect()).unwrap_or_default()
}

fn fact<'a>(name: String, value: String, k: f32) -> Element<'a, Message> {
    container(column![tight(value, 15.0 * k, INK), tight(name, 11.0 * k, FAINT)].spacing(1).align_x(iced::alignment::Horizontal::Center))
        .padding([6.0 * k, 0.0])
        .center_x(88.0 * k)
        .style(|_| container::Style { background: Some(Background::Color(ROW)), border: Border { radius: 9.0.into(), ..Border::default() }, ..container::Style::default() })
        .into()
}

fn block<'a>(title: Element<'a, Message>, inside: Element<'a, Message>, k: f32) -> Element<'a, Message> {
    container(column![title, inside].spacing(12.0 * k))
        .padding(18.0 * k)
        .width(Length::Fill)
        .style(|_| container::Style { background: Some(Background::Color(ROW)), border: Border { radius: 14.0.into(), ..Border::default() }, ..container::Style::default() })
        .into()
}

fn tag<'a>(slot: &str, k: f32) -> Element<'a, Message> {
    let base = crate::duel_view::slot_colour(slot);
    let fill = ui::mix(base, theme::GROUND, 0.74);
    container(strong(slot.to_owned(), 12.0 * k, ui::mix(base, Color::WHITE, 0.45)))
        .center_x(42.0 * k)
        .padding([3.0 * k, 0.0])
        .style(move |_| container::Style { background: Some(Background::Color(fill)), border: Border { radius: 6.0.into(), ..Border::default() }, ..container::Style::default() })
        .into()
}

fn bar<'a>(share: f32, colour: Color) -> Element<'a, Message> {
    let share = share.clamp(0.0, 1.0);
    let filled = (share * 1000.0).round() as u16;
    let track = row![
        container(Space::new()).width(Length::FillPortion(filled.max(1))).height(8).style(move |_| container::Style { background: Some(Background::Color(colour)), border: Border { radius: 4.0.into(), ..Border::default() }, ..container::Style::default() }),
        Space::new().width(Length::FillPortion((1000 - filled).max(1))),
    ];
    container(track).width(Length::Fill).style(|_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))), border: Border { radius: 4.0.into(), ..Border::default() }, ..container::Style::default() }).into()
}

fn note<'a>(value: String, label: String, colour: Color, k: f32) -> Element<'a, Message> {
    container(column![bold(value, 19.0 * k, colour), words(label, 13.0 * k, FAINT)].spacing(2))
        .padding([10.0 * k, 14.0 * k])
        .width(Length::Fill)
        .style(|_| container::Style { background: Some(Background::Color(ROW)), border: Border { radius: 12.0.into(), ..Border::default() }, ..container::Style::default() })
        .into()
}

fn tournament_name(w: &Words, tournament: &crate::desk_screen::Tournament) -> String {
    match &tournament.stage {
        Some(stage) if tournament.year == 0 => stage.clone(),
        Some(stage) => format!("{stage} {}", tournament.year),
        None => w.with("desk-owc-year", &[("year", tournament.year.to_string())]),
    }
}

fn minutes(seconds: u32) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn label_view<'a>(w: &Words, state: &'a State, k: f32) -> Element<'a, Message> {
    if state.adding {
        return tournament_view(w, state, k);
    }
    let tournaments = state.tournaments();
    let at = state.tournament.unwrap_or(0);
    let Some(tournament) = tournaments.get(at) else {
        let content = column![strong(w.t("desk-batches"), 22.0 * k, INK),
            container(words(w.t(if state.loading { "desk-loading" } else { "desk-label-empty" }), 16.0 * k, MUTED)).center_x(Length::Fill),
            container(line_button_k(w.t("desk-add-tournament"), Some(Message::Adding(true)), k)).width(280.0 * k)]
            .spacing(24).align_x(Alignment::Center);
        let panel = container(content).padding(32).width(600.0 * k).center_y(260.0 * k)
            .id(iced::widget::Id::new("desk-empty-batches")).style(workspace::surface);
        return container(panel).center_x(Length::Fill).center_y(Length::Fill)
            .id(iced::widget::Id::new("desk-label-area")).into();
    };
    let held = state.held_by(tournament);
    let done = held.iter().filter(|(_, map)| map.verified || state.skipped.contains(&map.beatmap_id)).count();
    let main: Element<'a, Message> = match state.next_map() {
        Some((_, map)) => {
            let about = state.about(map.beatmap_id);
            let pick = |own: &Option<String>, other: Option<&String>| own.clone().filter(|said| !said.is_empty()).or_else(|| other.cloned()).unwrap_or_default();
            let title = pick(&map.title, about.map(|found| &found.title));
            let version = pick(&map.version, about.map(|found| &found.version));
            let artist = pick(&map.artist, about.map(|found| &found.artist));
            let mapper = pick(&map.mapper, about.map(|found| &found.mapper));
            let listen = row![container(listen_button(w, state, map.beatmap_id, theme::CONTROL_HEIGHT)).width(Length::Shrink), link_button(map.beatmap_id, theme::CONTROL_HEIGHT)].spacing(8);
            let mut facts = row![].spacing(10).align_y(Alignment::Center);
            if desk::SLOTS.contains(&map.label.as_str()) || RESERVE.contains(&map.label.as_str()) { facts = facts.push(tag(&map.label, k)); }
            if let Some(found) = about {
                facts = facts.push(stars_of(found.stars, 13.0 * k));
                if let Some(bpm) = found.bpm {
                    facts = facts.push(words(w.with("desk-bpm", &[("bpm", format!("{bpm:.0}"))]), 14.0 * k, MUTED));
                }
                if let Some(seconds) = found.seconds {
                    facts = facts.push(words(minutes(seconds), 14.0 * k, MUTED));
                }
            }
            let shown_title = if title.is_empty() { map.beatmap_id.to_string() } else { title };
            let words_block = column![
                one_line(version, 16.0 * k, INK, theme::SANS_SEMI),
                one_line(shown_title, 34.0 * k, INK, theme::SANS_SEMI),
                words(artist, 17.0 * k, MUTED),
                row![crate::duels_screen::mapper_face(&mapper, map.beatmap_id, 24.0 * k), words(mapper.clone(), 15.0 * k, MUTED)].spacing(8).align_y(Alignment::Center),
            ]
            .spacing(4);
            let shown = container(column![row![facts, Space::new().width(Length::Fill), listen].align_y(Alignment::Center), Space::new().height(Length::Fill), words_block])
                .padding([20, 24])
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_| container::Style { background: Some(crate::duel_view::veil_fill()), border: Border { radius: 18.0.into(), ..Border::default() }, ..container::Style::default() });
            let set = map.beatmapset_id.or_else(|| about.and_then(|found| found.set));
            let picture: Element<'a, Message> = match set.and_then(|set| state.covers.get(&set)) {
                Some(handle) => container(image(handle.clone()).width(Length::Fill).height(Length::Fill).content_fit(iced::ContentFit::Cover).border_radius(17.0)).padding(1).style(|_| container::Style { background: Some(Background::Color(ROW)), border: Border { radius: 18.0.into(), ..Border::default() }, ..Default::default() }).into(),
                None => container(Space::new()).width(Length::Fill).height(Length::Fill).style(|_| container::Style { background: Some(Background::Color(ROW)), border: Border { radius: 18.0.into(), ..Border::default() }, ..container::Style::default() }).into(),
            };
            let mut keys = row![].spacing(8);
            for slot in desk::SLOTS.into_iter().chain(RESERVE) {
                let made = button(container(tag(slot, k)).center_x(64.0 * k).center_y(theme::workspace::CONTROL * k)).padding(0).style(quiet(false, theme::CONTROL_RADIUS));
                let made = if state.posting { made } else { made.on_press(Message::MapLabel(slot.to_owned())) };
                keys = keys.push(occupancy::mark(state, made.into(), map.verified && map.label == slot, 8.0, |_| false));
            }
            let revisiting = state.revisiting.is_some();
            let rest = row![container(line_button_k(w.t("desk-fits-none"), (!state.posting).then(|| Message::MapLabel(desk::OTHER.to_owned())), k)).width(240.0 * k), container(line_button_k(w.t(if revisiting { "desk-resume-labels" } else { "desk-skip" }), (!state.posting).then_some(if revisiting { Message::ResumeLabels } else { Message::Skip }), k)).width(160.0 * k)].spacing(8).wrap();
            let hero = occupancy::mark(state, stack![picture, shown].height(300.0 * k).into(), true, 18.0, |person| person.beatmap == Some(map.beatmap_id));
            let mut body = column![hero, keys.wrap(), busy(w, state, k), rest].spacing(12).width(Length::Fill);
            if revisiting {
                body = body.push(words(w.t("desk-relabel-note"), 12.0 * k, MUTED));
            }
            body.into()
        }
        None => container(column![heading_k(k, tournament_name(w, tournament)), words(w.t("desk-label-done"), 15.0 * k, MUTED)].spacing(10)).padding(workspace::INSET).width(Length::Fill).style(workspace::surface).into(),
    };
    let title = mouse_area(row![container(heading_k(k, tournament_name(w, tournament))).width(Length::Fill), glyph(Icon::Chevron, 16.0 * k, MUTED)].spacing(8).align_y(Alignment::Center))
        .on_press(Message::Picking(!state.picking))
        .interaction(mouse::Interaction::Pointer);
    let mut aside = column![title, line_button_k(w.t("desk-add-tournament"), (!state.posting).then_some(Message::Adding(true)), k)].spacing(10);
    if state.picking {
        let mut list = column![].spacing(2);
        for (index, each) in tournaments.iter().enumerate() {
            let on = index == at;
            list = list.push(tile(if on { LIT } else { Color::TRANSPARENT }, None, words(tournament_name(w, each), 14.0 * k, if on { INK } else { MUTED }), Some(Message::Tournament(index))));
        }
        list = list.push(tile(Color::TRANSPARENT, None, strong(w.t("desk-add-tournament"), 14.0 * k, INK), Some(Message::Adding(true))));
        aside = aside.push(list);
    } else {
        let total = held.len().max(1);
        aside = aside
            .push(row![bold((done + 1).min(held.len()).to_string(), 26.0 * k, INK), words(w.with("desk-of-total", &[("total", held.len().to_string())]), 15.0 * k, FAINT)].spacing(10).align_y(Alignment::End))
            .push(bar(done as f32 / total as f32, ACCENT));
        let mut saved = column![words(w.t("desk-saved-labels"), 13.0 * k, MUTED)].spacing(8);
        for (cell, map) in held.iter().filter(|(_, map)| map.verified) {
            let title = map.title.clone().unwrap_or_else(|| map.beatmap_id.to_string());
            let on = state.revisiting.as_ref().is_some_and(|(target, id)| *id == map.beatmap_id && target.stage == cell.stage && target.slot == cell.slot);
            let inside = row![one_line(title, 14.0 * k, INK, theme::SANS), tag(&map.label, k)].spacing(10).align_y(Alignment::Center);
            saved = saved.push(occupancy::mark(state, button(container(inside).padding(8).width(Length::Fill)).padding(0).style(quiet(false, theme::CONTROL_RADIUS)).on_press_maybe((!state.posting).then(|| Message::Revisit((*cell).clone(), map.beatmap_id))).into(), on, 8.0, |person| person.beatmap == Some(map.beatmap_id)));
        }
        aside = aside.push(scrollable(saved).direction(ui::hidden_bar()).height(Length::Fill));
    }
    let aside = container(aside).padding(18).width(260.0 * k).style(theme::slab);
    container(row![container(scrollable(main).direction(ui::hidden_bar())).width(Length::Fill).height(Length::Fill), aside].spacing(14).height(Length::Fill)).max_width(1180.0).height(Length::Fill).into()
}

fn archive_picker<'a>(w: &Words, state: &'a State, k: f32) -> iced::widget::Column<'a, Message> {
    let mut list = column![heading_k(k, w.t("desk-archive-title")), words(w.t("desk-archive-hint"), 12.0 * k, FAINT)].spacing(8);
    for packed in &state.archive {
        let on = state.archive_picked.contains(&packed.beatmap);
        let note = if packed.standard { format!("#{}", packed.beatmap) } else { w.t("desk-archive-other-mode") };
        let inside = row![
            glyph(if on { Icon::Check } else { Icon::Circle }, 16.0 * k, if on { ACCENT } else { FAINT }),
            column![one_line(packed.version.clone(), 14.0 * k, if packed.standard { INK } else { FAINT }, theme::SANS_SEMI), words(note, 12.0 * k, FAINT)].spacing(1).width(Length::Fill),
            stars_of(packed.stars, 12.0 * k),
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        let made = button(container(inside).padding(8).width(Length::Fill)).padding(0).style(quiet(on, theme::CONTROL_RADIUS));
        list = list.push(if packed.standard { made.on_press(Message::ArchivePick(packed.beatmap)) } else { made });
    }
    let count = state.archive_picked.len();
    list.push(main_button_k(w.with("desk-archive-take", &[("count", count.to_string())]), (count > 0).then_some(Message::ArchiveTake), k))
        .push(line_button_k(w.t("desk-archive-drop"), Some(Message::ArchiveDrop), k))
}

fn tournament_view<'a>(w: &Words, state: &'a State, k: f32) -> Element<'a, Message> {
    let found = read_pool(&state.pool.0.text());
    let lines = state.pool.0.text().lines().filter(|line| !line.trim().is_empty()).count();
    let ready = !state.name.trim().is_empty() && (state.year_text.is_empty() || state.year_text.len() == 4) && !found.is_empty() && !state.posting;
    let editor = text_editor(&state.pool.0).on_action(Message::Pool).placeholder(w.t("desk-pool-hint")).font(theme::SANS).size(14.0 * k).line_height(1.7).padding([12, 14]).height(Length::Fill).style(|theme, status| {
        let field = theme::field(theme, match status {
            text_editor::Status::Focused { .. } => text_input::Status::Focused { is_hovered: false },
            text_editor::Status::Hovered => text_input::Status::Hovered,
            _ => text_input::Status::Active,
        });
        text_editor::Style { background: field.background, border: field.border, placeholder: field.placeholder, value: field.value, selection: field.selection }
    });
    let archive = button(container(row![glyph(Icon::Download, 16.0 * k, INK), strong(w.t("desk-from-archive"), theme::BODY, INK)].spacing(8).align_y(Alignment::Center)).center_y(theme::CONTROL_HEIGHT - 2.0)).padding([0, 12]).style(quiet(false, theme::CONTROL_RADIUS)).on_press(Message::Archive);
    let form = panel(column![
        row![heading_k(k, w.t("desk-add-tournament")), Space::new().width(Length::Fill),
            container(button(container(strong(w.t("desk-back"), 14.0 * k, INK)).center_y(34.0 * k)).padding([0, 14]).style(quiet(false, theme::CONTROL_RADIUS)).on_press_maybe((!state.posting).then_some(Message::Adding(false)))).id(iced::widget::Id::new("desk-batch-back"))].spacing(12).align_y(Alignment::Center),
        row![labelled_k(k, w.t("desk-name"), field_kk(k, String::new(), &state.name, Message::Name)), container(labelled_k(k, w.t("desk-year"), field_kk(k, String::new(), &state.year_text, Message::YearText))).width(180.0 * k)].spacing(10),
        column![words(w.t("desk-pool"), 13.0 * k, FAINT), editor].spacing(6).height(Length::Fill),
        row![archive, Space::new().width(Length::Fill), container(process_button(w, state, w.with("desk-add-maps", &[("count", found.len().to_string())]), ready.then_some(Message::AddTournament), k)).width(Length::Shrink)].align_y(Alignment::Center),
    ].spacing(14).into());
    let mut parsed = column![row![heading_k(k, w.t("desk-parsed")), Space::new().width(Length::Fill), words(w.with("desk-of", &[("count", found.len().to_string()), ("total", lines.to_string())]), 13.0 * k, FAINT)].align_y(Alignment::Center)].spacing(4);
    for (slot, beatmap) in &found {
        let about = state.about(*beatmap);
        let title = about.map(|found| found.title.clone()).filter(|title| !title.is_empty()).unwrap_or_else(|| beatmap.to_string());
        parsed = parsed.push(row![
            cover(state, about.and_then(|found| found.set), 72.0 * k, 40.0 * k, 7.0),
            if slot == "PENDING" { words(w.t("desk-filter-unassigned"), 12.0 * k, MUTED) } else { tag(slot, k) },
            one_line(title, 14.0 * k, INK, theme::SANS_SEMI),
            stars_of(about.and_then(|found| found.stars), 12.0 * k),
        ].spacing(10).height(46).align_y(Alignment::Center));
    }
    let aside = if state.archive.is_empty() { parsed } else { archive_picker(w, state, k) };
    let aside = container(scrollable(aside).direction(ui::hidden_bar())).padding(18).width(300.0 * k).height(Length::Fill).style(theme::slab);
    container(row![form, aside].spacing(14).height(Length::Fill)).max_width(1180.0).max_height(680.0).height(Length::Fill).into()
}

pub use crate::desk::read_pool;

fn owc_order(slot: &str) -> usize {
    ["NM", "HD", "HR", "DT", "FM", "EZ", "FL", "TB"].iter().position(|prefix| slot.starts_with(prefix)).unwrap_or(6)
}

const ENOUGH: u32 = 5;
const PLENTY: u32 = 20;

fn tone(count: u32) -> (Color, Color) {
    match count {
        0 => (iced::color!(0x442224), BAD),
        1..ENOUGH => (ui::mix(iced::color!(0x30261a), iced::color!(0x4a3819), count as f32 / ENOUGH as f32), UNSURE),
        _ => (ui::mix(iced::color!(0x28331f), iced::color!(0x2f5426), (count - ENOUGH) as f32 / (PLENTY - ENOUGH) as f32), iced::color!(0xb6e59a)),
    }
}

fn band_name(w: &Words, band: usize) -> String {
    let edge = |value: f32| w.decimal(f64::from(value), 1);
    let next = desk::BANDS.get(band + 1).copied().unwrap_or(desk::TOP);
    format!("{}–{}", edge(desk::BANDS[band]), if next == desk::TOP { w.decimal(f64::from(next), 0) } else { edge(next) })
}

fn band_pill<'a>(said: String, on: bool, k: f32) -> Element<'a, Message> {
    container(strong(said, 12.0 * k, if on { INK } else { MUTED })).padding([4.0 * k, 6.0 * k]).style(move |_| container::Style {
        background: Some(Background::Color(if on { ui::mix(ACCENT, theme::GROUND, 0.7) } else { iced::color!(0x201319) })),
        border: Border { radius: 12.0.into(), ..Border::default() }, ..container::Style::default()
    }).into()
}

fn cell_style(fill: Color, ink: Color, chosen: bool, crossed: bool) -> impl Fn(&iced::Theme, button::Status) -> button::Style {
    move |_, status| {
        let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let shown = if lit { ui::mix(fill, Color::WHITE, 0.1) } else if crossed { ui::mix(fill, Color::WHITE, 0.04) } else { fill };
        let (edge, width) = if chosen { (INK, 2.0) } else if lit { (Color { a: 0.45, ..ink }, 1.0) } else { (Color::TRANSPARENT, 0.0) };
        button::Style { background: Some(Background::Color(shown)), text_color: ink, border: Border { color: edge, width, radius: 8.0.into() }, ..button::Style::default() }
    }
}

fn count_of(coverage: &desk::Coverage, slot: &str, band: usize) -> u32 {
    coverage.grid.get(slot).and_then(|counts| counts.get(band)).copied().unwrap_or_default()
}

fn status_pill<'a>(w: &Words, status: &str, k: f32) -> Element<'a, Message> {
    let (colour, key) = match status {
        "ranked" | "approved" => (GOOD, status),
        "loved" => (iced::color!(0xff7eb6), status),
        "qualified" => (UNSURE, status),
        "graveyard" | "wip" | "pending" => (MUTED, status),
        _ => (FAINT, "unknown"),
    };
    container(strong(w.t(&format!("desk-status-{key}")), 12.0 * k, colour)).padding([3.0 * k, 8.0 * k]).style(|_| container::Style {
        background: Some(Background::Color(iced::color!(0x201319))),
        border: Border { radius: 10.0.into(), ..Border::default() }, ..container::Style::default()
    }).into()
}

fn held_card<'a>(w: &Words, state: &'a State, map: &Held, k: f32) -> Element<'a, Message> {
    let shown = state.shown_map(&desk::Map { beatmap_id: map.beatmap_id, beatmapset_id: map.beatmapset_id, title: map.title.clone(), version: map.version.clone(), mapper: map.mapper.clone(), ..Default::default() });
    let colour = match map.level.as_str() { "human" => GOOD, "uncertain" => UNSURE, _ => MUTED };
    let fresh = state.added == Some(map.beatmap_id);
    let byline = [shown.version.clone(), shown.mapper.clone()].into_iter().flatten().filter(|said| !said.is_empty()).collect::<Vec<_>>().join(" · ");
    let top = row![
        cover(state, shown.beatmapset_id, 76.0 * k, 46.0 * k, 8.0),
        column![
            titled_map(shown.title.as_deref(), map.beatmap_id, 14.0 * k),
            one_line(byline, 12.0 * k, MUTED, theme::SANS),
            row![stars_of(Some(map.stars), 12.0 * k), strong(if map.level == "human" { map.reviewer.as_ref().map_or_else(|| w.t("desk-level-human"), |name| w.with("desk-reviewed-by", &[("name", name.clone())])) } else { w.t(&format!("desk-level-{}", map.level)) }, 12.0 * k, colour)].spacing(8).align_y(Alignment::Center),
        ].spacing(3).width(Length::Fill),
    ].spacing(10).align_y(Alignment::Center);
    let edit = button(container(row![glyph(Icon::Sliders, 15.0 * k, INK), tight(w.t("desk-edit-short"), 13.0 * k, INK)].spacing(6).align_y(Alignment::Center)).center_x(Length::Fill).center_y(32.0 * k))
        .width(Length::Fill)
        .padding([0, 10])
        .style(quiet(false, theme::CONTROL_RADIUS))
        .on_press_maybe((!state.posting).then_some(Message::Inspect(map.beatmap_id)));
    let actions = row![container(listen_button(w, state, map.beatmap_id, 32.0 * k)).width(Length::Fill), edit, link_button(map.beatmap_id, 32.0 * k)].spacing(6).align_y(Alignment::Center);
    let card = container(column![top, actions].spacing(8)).padding(10).width(Length::Fill).style(move |_| container::Style {
        background: Some(Background::Color(if fresh { LIT } else { ROW })),
        border: Border { radius: 12.0.into(), ..Border::default() },
        ..container::Style::default()
    }).into();
    occupancy::mark(state, card, fresh, 12.0, |person| person.beatmap == Some(map.beatmap_id))
}

fn cell_view<'a>(w: &Words, state: &'a State, coverage: &desk::Coverage, slot: &str, band: usize, k: f32) -> Element<'a, Message> {
    let count = count_of(coverage, slot, band);
    let (_, ink) = tone(count);
    let mut list = column![].spacing(8);
    for map in &state.held {
        list = list.push(held_card(w, state, map, k));
    }
    if state.held.is_empty() {
        list = list.push(words(w.t(if count == 0 { "desk-cell-empty" } else { "desk-loading" }), 13.0 * k, FAINT));
    }
    let range = format!("{}★", band_name(w, band));
    let search = button(container(if state.posting { busy(w, state, k) } else { row![glyph(Icon::Download, 16.0 * k, INK), strong(w.t("desk-search"), 14.0 * k, INK), meta_pill(range, k)].spacing(8).align_y(Alignment::Center).into() }).center_x(Length::Fill).center_y(40.0 * k))
        .padding(0)
        .style(theme::primary)
        .on_press_maybe((!state.posting).then_some(Message::Discover));
    let more = column![
        line_button_k(w.t("desk-add-map-here"), (!state.posting).then_some(Message::AddingMap(true)), k),
        line_button_k(w.t("desk-label-waiting"), Some(Message::Tab(Tab::Queue)), k),
    ].spacing(8);
    column![
        row![tag(slot, k), container(heading_k(k, skill(w, slot))).width(Length::Fill), band_pill(band_name(w, band), true, k)].spacing(8).align_y(Alignment::Center),
        column![
            row![strong(w.with("desk-band-count", &[("count", count.to_string())]), 14.0 * k, ink), Space::new().width(Length::Fill), words(w.with("desk-band-need", &[("count", ENOUGH.to_string())]), 12.0 * k, FAINT)].align_y(Alignment::Center),
            bar(count as f32 / ENOUGH as f32, ink),
        ].spacing(6),
        scrollable(list).direction(ui::hidden_bar()).height(Length::Fill),
        search,
        more,
    ].spacing(12).into()
}

fn gaps_view<'a>(w: &Words, state: &State, coverage: &desk::Coverage, k: f32) -> Element<'a, Message> {
    let mut gaps: Vec<(u32, usize, usize)> = Vec::new();
    for (at, slot) in desk::SLOTS.iter().enumerate() {
        for band in 0..desk::BANDS.len() {
            let count = count_of(coverage, slot, band);
            if count < ENOUGH { gaps.push((count, at, band)); }
        }
    }
    gaps.sort();
    let mut body = column![heading_k(k, w.t("desk-gaps-title"))].spacing(12);
    if gaps.is_empty() {
        body = body.push(words(w.t("desk-gaps-none"), 14.0 * k, GOOD));
    } else {
        let mut list = column![].spacing(4);
        for (count, at, band) in gaps.iter().take(6).copied() {
            let slot = desk::SLOTS[at];
            let (fill, ink) = tone(count);
            let amount = container(bold(count.to_string(), 14.0 * k, ink)).center_x(34.0 * k).center_y(26.0 * k).style(move |_| container::Style {
                background: Some(Background::Color(fill)), border: Border { radius: 7.0.into(), ..Border::default() }, ..container::Style::default()
            });
            let inside = row![tag(slot, k), band_pill(band_name(w, band), false, k), container(words(skill(w, slot), 13.0 * k, MUTED)).width(Length::Fill).clip(true), amount].spacing(8).align_y(Alignment::Center);
            list = list.push(button(container(inside).padding([6.0 * k, 8.0 * k]).width(Length::Fill)).padding(0).style(quiet(false, theme::CONTROL_RADIUS)).on_press(Message::Band(slot.to_owned(), band)));
        }
        body = body.push(words(w.t("desk-gaps-hint"), 13.0 * k, FAINT)).push(list);
    }
    body.push(line_button_k(w.t("desk-add-map"), (!state.posting).then_some(Message::AddingMap(true)), k))
        .push(words(w.t("desk-catalog-pick"), 13.0 * k, FAINT))
        .into()
}

fn map_preview<'a>(w: &Words, state: &State, found: &desk::About, k: f32) -> Element<'a, Message> {
    let high = 150.0 * k;
    let picture: Element<'a, Message> = match found.set.and_then(|set| state.covers.get(&set)) {
        Some(handle) => container(image(handle.clone()).width(Length::Fill).height(high - 2.0).content_fit(iced::ContentFit::Cover).border_radius(11.0)).padding(1).style(|_| container::Style { background: Some(Background::Color(ROW)), border: Border { radius: 12.0.into(), ..Border::default() }, ..Default::default() }).into(),
        None => container(Space::new()).width(Length::Fill).height(high).style(|_| container::Style { background: Some(Background::Color(ROW)), border: Border { radius: 12.0.into(), ..Border::default() }, ..container::Style::default() }).into(),
    };
    let mut facts = row![status_pill(w, &found.status, k), stars_of(found.stars, 12.0 * k)].spacing(8).align_y(Alignment::Center);
    if let Some(seconds) = found.seconds {
        facts = facts.push(words(minutes(seconds), 13.0 * k, MUTED));
    }
    if let Some(bpm) = found.bpm {
        facts = facts.push(words(w.with("desk-bpm", &[("bpm", format!("{bpm:.0}"))]), 13.0 * k, MUTED));
    }
    let byline = [found.artist.clone(), found.mapper.clone()].into_iter().filter(|said| !said.is_empty()).collect::<Vec<_>>().join(" · ");
    let shown = column![
        facts,
        Space::new().height(Length::Fill),
        one_line(found.version.clone(), 13.0 * k, INK, theme::SANS_SEMI),
        one_line(found.title.clone(), 17.0 * k, INK, theme::SANS_SEMI),
        one_line(byline, 12.0 * k, MUTED, theme::SANS),
    ].spacing(2);
    let veil = container(shown).padding(12.0 * k).width(Length::Fill).height(high).style(|_| container::Style {
        background: Some(crate::duel_view::veil_fill()), border: Border { radius: 12.0.into(), ..Border::default() }, ..container::Style::default()
    });
    stack![picture, veil].into()
}

fn add_map_view<'a>(w: &Words, state: &'a State, k: f32) -> Element<'a, Message> {
    let beatmap = desk::read_map(&state.map_link);
    let about = beatmap.and_then(|beatmap| state.about(beatmap));
    let foreign = about.is_some_and(|found| !found.standard);
    let unranked = about.is_some_and(|found| !found.status.is_empty() && found.status != "ranked");
    let nominal = about.and_then(|found| found.stars);
    let too_hard = nominal.is_some_and(|stars| stars > desk::TOP);
    let too_easy = nominal.is_some_and(|stars| stars < desk::BANDS[0]) && state.map_slot.as_deref().is_some_and(|slot| !slot.starts_with("DT"));
    let ready = beatmap.is_some() && !foreign && !too_hard && !state.posting;
    let field = text_input(&w.t("desk-map-link-hint"), &state.map_link)
        .on_input(Message::MapLink)
        .on_submit(Message::LookMap)
        .font(theme::SANS)
        .size(15.0 * k)
        .padding([11.0 * k, 14.0 * k])
        .width(Length::Fill)
        .style(theme::field);
    let asked = state.map_asked || state.map_link.contains('/');
    let preview: Element<'a, Message> = match (beatmap, about) {
        (None, _) if state.map_link.trim().is_empty() => words(w.t("desk-map-link-about"), 13.0 * k, FAINT),
        (None, _) => words(w.t("desk-map-link-bad"), 13.0 * k, BAD),
        (Some(_), Some(found)) => map_preview(w, state, found, k),
        (Some(id), None) if state.missing.contains(&id) => words(w.t("desk-map-missing"), 13.0 * k, UNSURE),
        (Some(_), None) if asked => words(w.t("desk-map-looking"), 13.0 * k, MUTED),
        (Some(_), None) => words(w.t("desk-map-press-enter"), 13.0 * k, FAINT),
    };
    let mut slots = row![].spacing(6).align_y(Alignment::Center);
    for slot in state.catalog_slots().iter().map(String::as_str) {
        let on = state.map_slot.as_deref() == Some(slot);
        slots = slots.push(button(container(tag(slot, k)).center_y(34.0 * k)).padding([0.0, 5.0 * k]).style(quiet(on, 9.0)).on_press(Message::MapSlot(Some(slot.to_owned()))));
    }
    slots = slots.push(chip_k(w.t("desk-map-slot-model"), state.map_slot.is_none(), Message::MapSlot(None), k));
    let close = button(container(glyph(Icon::Close, 16.0 * k, INK)).center_x(32.0 * k).center_y(32.0 * k))
        .padding(0)
        .style(quiet(false, theme::CONTROL_RADIUS))
        .on_press_maybe((!state.posting).then_some(Message::AddingMap(false)));
    let warning = |said: String, colour: Color| -> Element<'a, Message> {
        row![glyph(Icon::Warn, 16.0 * k, colour), container(words(said, 13.0 * k, colour)).width(Length::Fill)].spacing(8).align_y(Alignment::Center).into()
    };
    let mut body = column![
        row![glyph(Icon::Plus, 18.0 * k, ACCENT), container(heading_k(k, w.t("desk-add-map"))).width(Length::Fill), close].spacing(8).align_y(Alignment::Center),
        words(w.t("desk-add-map-about"), 13.0 * k, FAINT),
        labelled_k(k, w.t("desk-map-link"), field.into()),
        preview,
        labelled_k(k, w.t("desk-map-slot"), slots.wrap().vertical_spacing(6).into()),
    ]
    .spacing(14);
    if foreign {
        body = body.push(warning(w.t("desk-add-map-not-standard"), BAD));
    } else if too_hard || too_easy {
        body = body.push(warning(w.t("desk-map-stars-range"), if too_hard { BAD } else { UNSURE }));
    } else if unranked && state.map_slot.is_none() {
        body = body.push(warning(w.t("desk-map-unranked-note"), UNSURE));
    }
    body = body.push(labelled_k(k, w.t("desk-reason"), field_k(String::new(), &state.reason, Message::Reason, k)));
    if let Some(why) = &state.map_failed {
        body = body.push(warning(why.clone(), BAD));
    }
    let label = match &state.map_slot {
        Some(slot) => w.with("desk-add-map-to", &[("slot", slot.clone())]),
        None => w.t("desk-add-map-model"),
    };
    body = body.push(process_button(w, state, label, ready.then_some(Message::AddMap), k));
    scrollable(body).direction(ui::hidden_bar()).height(Length::Fill).into()
}

fn catalog_view<'a>(w: &Words, state: &'a State, _wide: f32, k: f32) -> Element<'a, Message> {
    let Some(coverage) = &state.desk.coverage else { return panel(words(w.t("desk-loading"), 15.0 * k, FAINT)) };
    let totals = &coverage.totals;
    let group = |value: u32| w.lang().group(u64::from(value));
    let tiles = row![
        note(group(totals.playable), w.t("desk-total-playable"), INK, k),
        note(group(totals.human), w.t("desk-total-human"), GOOD, k),
        note(group(totals.uncertain), w.t("desk-total-uncertain"), UNSURE, k),
        note(group(totals.quarantined), w.t("desk-total-quarantined"), BAD, k),
        note(group(totals.unassigned), w.t("desk-total-unassigned"), MUTED, k),
    ]
    .spacing(10);
    let chosen = state.band.as_ref().map(|(slot, band)| (slot.as_str(), *band));
    let (label_wide, cell_wide, cell_high, sum_wide) = (124.0 * k, 58.0 * k, 42.0 * k, 52.0 * k);
    let mut head = row![Space::new().width(label_wide)].spacing(5).align_y(Alignment::Center);
    for band in 0..desk::BANDS.len() {
        head = head.push(container(band_pill(band_name(w, band), chosen.is_some_and(|(_, at)| at == band), k)).center_x(cell_wide));
    }
    head = head.push(container(tight(w.t("desk-catalog-sum"), 13.0 * k, FAINT)).center_x(sum_wide));
    let mut grid = column![head].spacing(5);
    let mut columns = vec![0u32; desk::BANDS.len()];
    let slots = state.catalog_slots();
    for slot in slots.iter().map(String::as_str) {
        let on_row = chosen.is_some_and(|(held, _)| held == slot);
        let mut line = row![row![tag(slot, k), words(skill(w, slot), 13.0 * k, if on_row { INK } else { MUTED })].spacing(8).width(label_wide).align_y(Alignment::Center)].spacing(5).align_y(Alignment::Center);
        let mut sum = 0u32;
        for band in 0..desk::BANDS.len() {
            let count = count_of(coverage, slot, band);
            sum += count;
            columns[band] += count;
            let (fill, ink) = tone(count);
            let picked = chosen == Some((slot, band));
            let crossed = !picked && (on_row || chosen.is_some_and(|(_, at)| at == band));
            let more = coverage.secondary.get(slot).and_then(|counts| counts.get(band)).copied().unwrap_or_default();
            let shown: Element<'a, Message> = if more > 0 {
                row![bold(count.to_string(), 15.0 * k, ink), words(format!("+{more}"), 11.0 * k, ink)].spacing(2).align_y(Alignment::Center).into()
            } else {
                bold(count.to_string(), 15.0 * k, ink)
            };
            let made = button(container(shown).center_x(cell_wide).center_y(cell_high).id(iced::widget::Id::from(format!("desk-cell-{slot}-{band}"))))
                .padding(0)
                .style(cell_style(fill, ink, false, crossed))
                .on_press(Message::Band(slot.to_owned(), band));
            line = line.push(occupancy::mark(state, made.into(), picked, 8.0, |person| person.section == "catalog" && person.slot.as_deref() == Some(slot) && person.band == Some(band)));
        }
        line = line.push(container(strong(group(sum), 14.0 * k, if on_row { INK } else { MUTED })).center_x(sum_wide).center_y(cell_high));
        grid = grid.push(line);
    }
    let mut foot = row![container(words(w.t("desk-catalog-total"), 13.0 * k, FAINT)).width(label_wide)].spacing(5).align_y(Alignment::Center);
    for total in &columns {
        foot = foot.push(container(strong(group(*total), 13.0 * k, MUTED)).center_x(cell_wide).center_y(28.0 * k));
    }
    foot = foot.push(container(strong(group(columns.iter().sum::<u32>()), 13.0 * k, INK)).center_x(sum_wide).center_y(28.0 * k));
    grid = grid.push(container(foot).id(iced::widget::Id::new("desk-catalog-totals")));
    let table = container(scrollable(grid).direction(scrollable::Direction::Both { vertical: scrollable::Scrollbar::hidden(), horizontal: scrollable::Scrollbar::default().spacing(6.0) }).height(Length::Fill))
        .padding(workspace::INSET).width(Length::Fill).height(Length::Fill).max_height(620.0 * k).id(iced::widget::Id::new("desk-catalog-table")).style(workspace::surface);
    let (aside, tall): (Element<'a, Message>, bool) = if state.adding_map {
        (add_map_view(w, state, k), true)
    } else {
        match &state.band {
            Some((slot, band)) => (cell_view(w, state, coverage, slot, *band, k), true),
            None => (gaps_view(w, state, coverage, k), false),
        }
    };
    let aside: Element<'a, Message> = container(aside)
        .padding(18)
        .width(320.0 * k)
        .height(if tall { Length::Fill } else { Length::Shrink })
        .max_height(if state.adding_map { 760.0 } else { 620.0 } * k)
        .id(iced::widget::Id::new("desk-catalog-aside"))
        .style(workspace::surface)
        .into();
    let aside = container(occupancy::mark(state, aside, state.band.is_some(), 12.0, |person| {
        person.section == "catalog" && state.band.as_ref().is_some_and(|(slot, band)| person.slot.as_ref() == Some(slot) && person.band == Some(*band))
    }))
        .width(320.0 * k)
        .height(if tall { Length::Fill } else { Length::Shrink })
        .max_height(if state.adding_map { 760.0 } else { 620.0 } * k);
    let content = row![table, aside].spacing(workspace::GAP).height(Length::Fill);
    container(column![tiles, content].spacing(12).height(Length::Fill)).max_width(1200.0).height(Length::Fill).into()
}

fn percent(value: Option<f64>) -> (String, Color) {
    match value {
        Some(value) => (format!("{:.0}%", value * 100.0), if value >= 0.7 { GOOD } else if value >= 0.5 { UNSURE } else { BAD }),
        None => ("—".to_owned(), FAINT),
    }
}

fn state_colour(state: &str) -> Color {
    match state {
        "published" => GOOD,
        "shadow" => UNSURE,
        _ => FAINT,
    }
}

fn model_view<'a>(w: &Words, state: &'a State, roles: &[String], now: i64, k: f32) -> Element<'a, Message> {
    let chosen = state.version.clone().or_else(|| state.desk.versions.first().map(|found| found.version.clone()));
    let mut list = column![strong(w.t("desk-versions"), 17.0 * k, INK)].spacing(8);
    for version in &state.desk.versions {
        let on = Some(&version.version) == chosen.as_ref();
        let (score, colour) = percent(version.report.pointer("/test/macro_f1").and_then(serde_json::Value::as_f64));
        let inside = row![
            column![one_line(version_name(w, &version.version), 16.0 * k, INK, theme::SANS_SEMI), tight(w.t(&format!("desk-model-{}", version.state.to_lowercase())), 13.0 * k, state_colour(&version.state))].spacing(3).width(Length::Fill),
            column![tight(score, 18.0 * k, colour), tight(ago(w, version.created_at.as_deref(), now), 12.0 * k, FAINT)].spacing(3).align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        let fill = if on { LIT } else { ROW };
        list = list.push(tile(fill, on.then_some((PICKED, 1.5)), container(inside).padding([2.0 * k, 4.0 * k]).into(), Some(Message::Version(version.version.clone()))));
    }
    if state.desk.versions.is_empty() {
        list = list.push(words(w.t(if state.loading { "desk-loading" } else { "desk-no-versions" }), 14.0 * k, FAINT));
    }
    let may = desk::allows(roles, desk::PUBLISHER) && !state.posting;
    let published = state.desk.versions.iter().any(|version| version.state == "published");
    let mut general = column![row![
        container(line_button_k(w.t("desk-train"), (desk::allows(roles, desk::REVIEWER) && !state.posting && state.training.is_none()).then_some(Message::Train), k)).width(Length::Fill),
        container(line_button_k(w.t("desk-rollback"), (may && published).then_some(Message::Rollback), k)).width(Length::Fill),
    ]
    .spacing(8)]
    .spacing(8);
    if state.training.is_some() {
        general = general.push(words(w.t("desk-training"), 13.0 * k, FAINT));
    } else if let Some(error) = &state.training_failed {
        general = general.push(words(w.with("desk-training-failed", &[("error", error.clone())]), 13.0 * k, BAD));
    } else if state.training_lost {
        general = general.push(words(w.t("desk-training-lost"), 13.0 * k, BAD));
    } else if state.nothing_new {
        general = general.push(words(w.t("desk-nothing-new"), 13.0 * k, FAINT));
    }
    let aside = container(column![scrollable(list).direction(ui::hidden_bar()).height(Length::Fill), general].spacing(12)).padding(18).width(300.0 * k).height(Length::Fill).style(theme::slab);
    let Some(version) = state.desk.versions.iter().find(|version| Some(&version.version) == chosen.as_ref()) else {
        return row![aside, panel(Space::new().into())].spacing(14).height(Length::Fill).into();
    };
    let report = &version.report;
    let live = version.state == "published";
    let mut buttons = row![].spacing(8);
    if !live {
        buttons = buttons.push(container(line_button_k(w.t(if state.confirming { "desk-delete-sure" } else { "desk-delete" }), may.then_some(Message::Delete), k)).width(170.0 * k));
        buttons = buttons.push(container(process_button(w, state, w.t("desk-publish"), may.then_some(Message::Publish), k)).width(170.0 * k));
    }
    let header = row![
        column![
            bold(version_name(w, &version.version), 30.0 * k, INK),
            tight(w.t(&format!("desk-model-{}", version.state.to_lowercase())), 15.0 * k, state_colour(&version.state)),
        ]
        .spacing(4)
        .width(Length::Fill),
        buttons,
    ]
    .align_y(Alignment::Center);
    let number = |path: &str| report.pointer(path).and_then(serde_json::Value::as_u64);
    let maps = number("/maps").or_else(|| number("/train")).unwrap_or(u64::from(version.labels));
    let people = if version.version.starts_with("kamerton-") { u64::from(version.labels) } else { number("/verified").unwrap_or_default() };
    let figure = |value: String, colour: Color, label: String| -> Element<'a, Message> {
        container(column![bold(value, 30.0 * k, colour), words(label, 13.0 * k, FAINT)].spacing(2))
            .padding([14.0 * k, 18.0 * k])
            .width(Length::Fill)
            .style(|_| container::Style { background: Some(Background::Color(ROW)), border: Border { radius: 14.0.into(), ..Border::default() }, ..container::Style::default() })
            .into()
    };
    let (score, score_colour) = percent(report.pointer("/test/macro_f1").and_then(serde_json::Value::as_f64));
    let (sure, sure_colour) = percent(report.pointer("/test/confident/precision").and_then(serde_json::Value::as_f64));
    let figures = row![
        figure(score, score_colour, w.t("desk-figure-score")),
        figure(sure, sure_colour, w.t("desk-sure-right")),
        figure(w.lang().group(maps), INK, w.t("desk-figure-maps")),
        figure(w.lang().group(people), INK, w.t("desk-verified")),
    ]
    .spacing(10);
    let figures = match admitted_line(w, report) {
        Some(line) => column![figures, words(line, 13.0 * k, FAINT)].spacing(8),
        None => column![figures],
    };
    let cell = |said: String, colour: Color| -> Element<'a, Message> { container(tight(said, 16.0 * k, colour)).width(96.0 * k).align_x(iced::alignment::Horizontal::Right).into() };
    let mut slots = column![row![Space::new().width(Length::Fill), container(tight(w.t("desk-precision"), 13.0 * k, FAINT)).width(96.0 * k).align_x(iced::alignment::Horizontal::Right), container(tight(w.t("desk-recall"), 13.0 * k, FAINT)).width(96.0 * k).align_x(iced::alignment::Horizontal::Right)]].spacing(4);
    let line = |slot: &str| -> Element<'a, Message> {
        let held = report.pointer(&format!("/test/per_slot/{slot}"));
        let (precision, pc) = percent(held.and_then(|held| held.get("precision")).and_then(serde_json::Value::as_f64));
        let (recall, rc) = percent(held.and_then(|held| held.get("recall")).and_then(serde_json::Value::as_f64));
        row![container(row![tag(slot, k), one_line(skill(w, slot), 14.0 * k, MUTED, theme::SANS)].spacing(10).align_y(Alignment::Center)).width(Length::Fill).clip(true), cell(precision, pc), cell(recall, rc)].spacing(8).height(36.0 * k).align_y(Alignment::Center).into()
    };
    for slot in desk::SLOTS {
        slots = slots.push(line(slot));
    }
    let mut spare: Vec<String> = report.pointer("/test/per_slot").and_then(serde_json::Value::as_object)
        .map(|found| found.keys().filter(|slot| !desk::SLOTS.contains(&slot.as_str()) && slot.as_str() != desk::OTHER).cloned().collect())
        .unwrap_or_default();
    spare.extend(state.reserve());
    spare.sort_by_key(|slot| (owc_order(slot), slot.clone()));
    spare.dedup();
    if !spare.is_empty() {
        slots = slots.push(container(words(w.t("desk-reserve"), 13.0 * k, FAINT)).padding(Padding { top: 12.0 * k, ..Padding::ZERO }));
        for slot in &spare {
            slots = slots.push(line(slot));
        }
    }
    let mut years = column![].spacing(10);
    if let Some(found) = report.get("years").and_then(serde_json::Value::as_object) {
        for (fold, scored) in found {
            let value = scored.get("macro_f1").and_then(serde_json::Value::as_f64);
            let (said, colour) = percent(value);
            let name = if fold == "queue" { w.t("desk-fold-queue") } else { fold.clone() };
            years = years.push(row![container(tight(name, 14.0 * k, MUTED)).width(76.0 * k), bar(value.unwrap_or_default() as f32, colour), container(tight(said, 16.0 * k, colour)).width(56.0 * k).align_x(iced::alignment::Horizontal::Right)].spacing(12).height(36.0 * k).align_y(Alignment::Center));
        }
    }
    let body = column![
        column![strong(w.t("desk-slots"), 17.0 * k, INK), slots].spacing(10),
        column![strong(w.t("desk-by-year"), 17.0 * k, INK), years].spacing(10),
    ].spacing(24.0 * k);
    let reason = row![words(w.t("desk-reason"), 13.0 * k, FAINT), field_k(w.t("desk-reason-hint"), &state.reason, Message::Reason, k)].spacing(12).align_y(Alignment::Center);
    let page = column![header, figures, scrollable(body).direction(ui::hidden_bar()).height(Length::Fill), reason].spacing(22.0 * k);
    row![aside, panel(page.into())].spacing(14).height(Length::Fill).into()
}

fn role_name(w: &Words, role: &str) -> String {
    w.t(&format!("desk-role-{}", role.to_lowercase().replace('_', "-")))
}

fn people_list<'a>(w: &Words, state: &'a State, owner: bool, k: f32) -> Element<'a, Message> {
    let mut list = column![].spacing(8.0 * k);
    for held in &state.desk.people {
        let name = held.name.clone().unwrap_or_else(|| held.player.to_string());
        let mut line = row![player_face(&name, held.osu_id, 38.0 * k), container(strong(name.clone(), 17.0 * k, INK)).width(Length::Fill)].spacing(12.0 * k).align_y(Alignment::Center);
        for role in &held.roles {
            if !owner || role == desk::OWNER {
                line = line.push(container(strong(role_name(w, role), 14.0 * k, INK)).padding([6.0 * k, 12.0 * k]).style(|_| container::Style { background: Some(Background::Color(LIT)), border: Border { radius: 8.0.into(), ..Border::default() }, ..container::Style::default() }));
                continue;
            }
            let asking = state.revoking.as_ref() == Some(&(held.player, role.clone()));
            let said = if asking { w.t("desk-revoke-sure") } else { role_name(w, role) };
            let inside = row![strong(said, 14.0 * k, if asking { BAD } else { INK }), glyph(Icon::Close, 13.0 * k, if asking { BAD } else { MUTED })].spacing(8).align_y(Alignment::Center);
            line = line.push(button(inside).padding([6.0 * k, 12.0 * k]).style(move |_, status| {
                let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
                button::Style { background: Some(Background::Color(if asking || lit { ui::mix(ACCENT, theme::GROUND, 0.82) } else { LIT })), text_color: INK, border: Border { radius: 8.0.into(), ..Border::default() }, ..button::Style::default() }
            }).on_press_maybe((!state.posting).then(|| Message::Revoke(held.player, role.clone()))));
        }
        list = list.push(container(line).padding([10.0 * k, 14.0 * k]).width(Length::Fill).style(|_| container::Style { background: Some(Background::Color(ROW)), border: Border { radius: 12.0.into(), ..Border::default() }, ..container::Style::default() }));
    }
    list.into()
}

fn journal_map_id(entry: &desk::Entry) -> Option<i64> {
    let id = if entry.action.starts_with("map_") {
        entry.target.parse().ok()
    } else if matches!(entry.action.as_str(), "batch_delete" | "batch_exclude" | "batch_include") {
        let (_, rest) = entry.target.split_once(':')?;
        rest.rsplit_once(':')?.1.parse().ok()
    } else { None };
    id.filter(|id| *id > 0)
}

fn did(w: &Words, state: &State, entry: &desk::Entry) -> String {
    let key = format!("desk-did-{}", entry.action.replace('_', "-"));
    let after = |field: &str| entry.after.get(field).and_then(serde_json::Value::as_str).unwrap_or_default().to_owned();
    if entry.action.starts_with("map_") {
        let title = entry.target.parse::<i64>().ok().and_then(|beatmap| state.about(beatmap)).map(|found| found.title.clone()).filter(|title| !title.is_empty()).unwrap_or_else(|| entry.target.clone());
        return w.with(&key, &[("slot", after("slot")), ("title", title)]);
    }
    if entry.action == "corpus_label" {
        let mut parts = entry.target.split(':');
        let (year, stage, slot) = (parts.next().unwrap_or_default().to_owned(), parts.next().unwrap_or_default(), parts.next().unwrap_or_default().to_owned());
        let label = after("label");
        let label = if label == desk::EXCLUDED { w.t("desk-no-slot") } else { label };
        return w.with(&key, &[("year", year), ("stage", stage_name(w, stage).to_lowercase()), ("slot", slot), ("label", label)]);
    }
    if entry.action == "discover_result" {
        let outcomes = entry.after.get("outcomes").and_then(serde_json::Value::as_object);
        let count = |accept: fn(&str) -> bool| outcomes.map(|all| all.iter().filter(|(key, _)| accept(key)).filter_map(|(_, value)| value.as_u64()).sum::<u64>()).unwrap_or_default().to_string();
        let band = entry.after.get("band").and_then(serde_json::Value::as_u64).filter(|band| *band < desk::BANDS.len() as u64).map(|band| band_name(w, band as usize)).unwrap_or_default();
        let summary = w.with(&key, &[
            ("slot", after("slot")), ("band", band),
            ("assigned", count(|key| key.starts_with("proposed "))),
            ("waiting", count(|key| matches!(key, "no_slot" | "unassigned" | "below_threshold" | "reserve_slot" | "unknown_slot"))),
            ("known", count(|key| key == "known")),
            ("failed", count(|key| matches!(key, "failed" | "download_failed" | "no_file" | "parse_failed" | "mirrors_unavailable"))),
        ]);
        return summary;
    }
    if entry.action == "auto_discover" {
        let (slot, band) = entry.target.split_once(':').unwrap_or((entry.target.as_str(), ""));
        let band = band.parse::<usize>().ok().filter(|band| *band < desk::BANDS.len()).map(|band| band_name(w, band)).unwrap_or_default();
        let added = entry.after.get("added").and_then(serde_json::Value::as_u64).unwrap_or_default();
        return w.with(&key, &[("slot", slot.to_owned()), ("band", band), ("added", added.to_string())]);
    }
    if entry.action.starts_with("batch_") {
        let (year, rest) = entry.target.split_once(':').unwrap_or(("0", entry.target.as_str()));
        let (stage, beatmap) = match rest.rsplit_once(':') {
            Some((stage, tail)) if tail.parse::<i64>().is_ok() => (stage, tail.parse::<i64>().ok()),
            _ => (rest, None),
        };
        let batch = if year == "0" { stage.to_owned() } else { format!("{stage} {year}") };
        let target = match beatmap {
            Some(id) => {
                let title = state.about(id).map(|found| found.title.clone()).filter(|title| !title.is_empty()).unwrap_or_else(|| format!("#{id}"));
                w.with("desk-batch-map", &[("title", title), ("batch", batch)])
            }
            None => w.with("desk-batch-whole", &[("batch", batch)]),
        };
        let renamed = format!("{} {}", after("name"), entry.after.get("year").and_then(serde_json::Value::as_i64).unwrap_or_default());
        return w.with(&key, &[("target", target), ("name", renamed.trim_end_matches(" 0").to_owned())]);
    }
    if matches!(entry.action.as_str(), "grant" | "revoke") {
        let (player, role) = entry.target.split_once(':').unwrap_or((entry.target.as_str(), ""));
        let name = player.parse::<i64>().ok().and_then(|player| state.desk.people.iter().find(|held| held.player == player)).and_then(|held| held.name.clone()).unwrap_or_else(|| player.to_owned());
        return w.with(&key, &[("target", format!("{name}, {}", role_name(w, role).to_lowercase()))]);
    }
    let target = if matches!(entry.action.as_str(), "publish_model" | "delete_model") { version_name(w, &entry.target) } else { entry.target.clone() };
    w.with(&key, &[("target", target)])
}

fn batches_view<'a>(w: &Words, state: &'a State, now: i64, k: f32) -> Element<'a, Message> {
    let shown = state.batch_shown();
    let mut list = column![heading_k(k, w.t("desk-batches-all")), words(w.t("desk-batches-hint"), 12.0 * k, FAINT)].spacing(8);
    for batch in &state.desk.batches {
        let on = shown.is_some_and(|held| held.name == batch.name && held.year == batch.year);
        let title = if batch.year == 0 { batch.name.clone() } else { format!("{} {}", batch.name, batch.year) };
        let who = batch.author.as_ref().and_then(|author| author.name.clone()).unwrap_or_else(|| w.t("desk-batches-unknown"));
        let when = unix_of(batch.added_at.as_deref()).map(|at| w.day(at, now)).unwrap_or_default();
        let mut meta = row![player_face(&who, batch.author.as_ref().and_then(|author| author.osu_id), 20.0 * k), words(format!("{who} · {when}"), 12.0 * k, FAINT)].spacing(6).align_y(Alignment::Center);
        if batch.excluded > 0 {
            meta = meta.push(strong(w.with("desk-batches-excluded", &[("count", batch.excluded.to_string())]), 12.0 * k, UNSURE));
        }
        let progress = batch.labelled as f32 / batch.maps.max(1) as f32;
        let inside = column![
            row![container(strong(title, 15.0 * k, INK)).width(Length::Fill), words(w.with("desk-of", &[("count", batch.labelled.to_string()), ("total", batch.maps.to_string())]), 12.0 * k, FAINT)].align_y(Alignment::Center),
            meta,
            bar(progress, if batch.excluded == batch.maps { FAINT } else { GOOD }),
        ]
        .spacing(6);
        list = list.push(tile(if on { LIT } else { ROW }, on.then_some((PICKED, 1.5)), inside.into(), Some(Message::Batch(batch.name.clone(), batch.year))));
    }
    if state.desk.batches.is_empty() {
        list = list.push(words(w.t(if state.loading { "desk-loading" } else { "desk-batches-none" }), 14.0 * k, FAINT));
    }
    let aside = container(scrollable(list).direction(ui::hidden_bar())).padding(18).width(340.0 * k).height(Length::Fill).style(theme::slab);
    let Some(batch) = shown else { return row![aside, panel(Space::new().into())].spacing(14).height(Length::Fill).into() };
    let ready = !state.posting;
    let held = state.batch.is_some();
    let (name_shown, year_shown) = if held { (state.batch_name.clone(), state.batch_year.clone()) } else { (batch.name.clone(), batch.year.to_string()) };
    let renamed = held && state.batch_year.parse::<i32>().is_ok() && !state.batch_name.trim().is_empty()
        && (state.batch_name.trim() != batch.name || state.batch_year != batch.year.to_string());
    let rename = row![
        container(labelled_k(k, w.t("desk-name"), field_kk(k, String::new(), &name_shown, Message::BatchName))).width(Length::Fill),
        container(labelled_k(k, w.t("desk-year"), field_kk(k, String::new(), &year_shown, Message::BatchYear))).width(190.0 * k),
        container(line_button_k(w.t("desk-batches-rename"), (ready && renamed).then_some(Message::RenameBatch), k)).width(Length::Shrink),
    ]
    .spacing(10)
    .align_y(Alignment::End);
    let whole_out = batch.excluded >= batch.maps && batch.maps > 0;
    let deleting = state.batch_sure == Some(None);
    let actions = row![
        container(line_button_k(w.t(if whole_out { "desk-batches-include" } else { "desk-batches-exclude" }), ready.then_some(Message::ExcludeBatch(None, !whole_out)), k)).width(Length::Shrink),
        container(line_button_k(w.t(if deleting { "desk-batches-delete-sure" } else { "desk-batches-delete" }), ready.then_some(Message::DeleteBatch(None)), k)).width(Length::Shrink),
    ]
    .spacing(10);
    let mut maps = column![].spacing(6);
    for (_, map) in state.batch_maps() {
        let about = state.about(map.beatmap_id);
        let title = map.title.clone().filter(|title| !title.is_empty()).or_else(|| about.map(|found| found.title.clone()).filter(|title| !title.is_empty())).unwrap_or_else(|| format!("#{}", map.beatmap_id));
        let version = map.version.clone().filter(|version| !version.is_empty()).or_else(|| about.map(|found| found.version.clone())).unwrap_or_default();
        let set = map.beatmapset_id.or_else(|| about.and_then(|found| found.set));
        let shade = if map.excluded { FAINT } else { INK };
        let mut state_line = row![].spacing(6).align_y(Alignment::Center);
        if desk::SLOTS.contains(&map.label.as_str()) || RESERVE.contains(&map.label.as_str()) {
            state_line = state_line.push(tag(&map.label, k));
        }
        state_line = state_line.push(words(w.t(if map.excluded { "desk-batches-map-out" } else if map.verified { "desk-batches-map-checked" } else { "desk-batches-map-waiting" }), 12.0 * k, if map.excluded { UNSURE } else { FAINT }));
        let sure = state.batch_sure == Some(Some(map.beatmap_id));
        let toggle = button(container(glyph(if map.excluded { Icon::Undo } else { Icon::Ban }, 15.0 * k, INK)).center_x(32.0 * k).center_y(32.0 * k))
            .padding(0).style(quiet(false, theme::CONTROL_RADIUS)).on_press_maybe(ready.then_some(Message::ExcludeBatch(Some(map.beatmap_id), !map.excluded)));
        let remove = button(container(if sure { words(w.t("desk-batches-delete-map-sure"), 12.0 * k, BAD) } else { glyph(Icon::Close, 15.0 * k, INK) }).center_y(32.0 * k).padding([0.0, 9.0 * k]))
            .padding(0).style(quiet(sure, theme::CONTROL_RADIUS)).on_press_maybe(ready.then_some(Message::DeleteBatch(Some(map.beatmap_id))));
        let line = row![
            cover(state, set, 64.0 * k, 38.0 * k, 7.0),
            column![one_line(if version.is_empty() { title } else { format!("{title} [{version}]") }, 14.0 * k, shade, theme::SANS_SEMI), state_line].spacing(3).width(Length::Fill),
            toggle,
            remove,
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        maps = maps.push(container(line).padding([6.0 * k, 10.0 * k]).width(Length::Fill).style(|_| container::Style {
            background: Some(Background::Color(ROW)), border: Border { radius: 10.0.into(), ..Border::default() }, ..container::Style::default()
        }));
    }
    let author = batch.author.as_ref().and_then(|author| author.name.clone()).unwrap_or_else(|| w.t("desk-batches-unknown"));
    let head = row![
        player_face(&author, batch.author.as_ref().and_then(|author| author.osu_id), 42.0 * k),
        column![
            heading_k(k, if batch.year == 0 { batch.name.clone() } else { format!("{} {}", batch.name, batch.year) }),
            words(w.with("desk-batches-by", &[("name", author), ("when", unix_of(batch.added_at.as_deref()).map(|at| w.day(at, now)).unwrap_or_default())]), 13.0 * k, FAINT),
        ].spacing(2).width(Length::Fill),
        meta_pill(w.with("desk-of", &[("count", batch.labelled.to_string()), ("total", batch.maps.to_string())]), k),
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    let body = column![
        head,
        block(strong(w.t("desk-batches-rename-title"), 16.0 * k, INK), rename.into(), k),
        labelled_k(k, w.t("desk-reason"), field_kk(k, w.t("desk-reason-hint"), &state.reason, Message::Reason)),
        block(strong(w.t("desk-batches-whole"), 16.0 * k, INK), column![words(w.t("desk-batches-whole-about"), 13.0 * k, FAINT), actions].spacing(10).into(), k),
        block(strong(w.with("desk-batches-maps", &[("count", batch.maps.to_string())]), 16.0 * k, INK), column![words(w.t("desk-batches-maps-about"), 13.0 * k, FAINT), maps].spacing(10).into(), k),
        busy(w, state, k),
    ]
    .spacing(16);
    let detail = container(scrollable(body).direction(ui::hidden_bar())).padding(workspace::INSET).width(Length::Fill).height(Length::Fill).style(workspace::surface);
    row![aside, detail].spacing(14).height(Length::Fill).into()
}

fn spoken(w: &Words, key: &str, fallback: &str) -> String {
    let said = w.t(key);
    if said == format!("[{key}]") { fallback.to_owned() } else { said }
}

fn unix_of(stamp: Option<&str>) -> Option<i64> {
    stamp.and_then(|at| chrono::DateTime::parse_from_rfc3339(at).ok()).map(|at| at.timestamp())
}

fn mark_of(action: &str) -> (Icon, Color) {
    match action {
        "map_approve" => (Icon::Check, GOOD),
        "map_move" => (Icon::Right, UNSURE),
        "map_reject" => (Icon::Ban, BAD),
        "map_quarantine" => (Icon::Warn, UNSURE),
        "map_add" => (Icon::Plus, GOOD),
        "discover" | "discover_result" => (Icon::Search, MUTED),
        "auto_discover" => (Icon::Search, ACCENT),
        "corpus_label" | "add_tournament" | "batch_rename" => (Icon::Shapes, MUTED),
        "batch_delete" => (Icon::Ban, BAD),
        "batch_exclude" => (Icon::Warn, UNSURE),
        "batch_include" => (Icon::Undo, GOOD),
        "publish_model" | "rollback_model" | "train_model" | "delete_model" => (Icon::Chart, INK),
        "cancel" | "manual_finalize" => (Icon::Swords, BAD),
        "grant" | "revoke" => (Icon::People, MUTED),
        _ => (Icon::Circle, FAINT),
    }
}

fn badge<'a>(icon: Icon, colour: Color, k: f32) -> Element<'a, Message> {
    container(glyph(icon, 16.0 * k, colour)).center_x(34.0 * k).center_y(34.0 * k).style(move |_| container::Style {
        background: Some(Background::Color(ui::mix(colour, theme::GROUND, 0.82))),
        border: Border { radius: (17.0 * k).into(), ..Border::default() },
        ..container::Style::default()
    }).into()
}

fn journal_row<'a>(w: &Words, state: &'a State, entry: &desk::Entry, open: bool, k: f32) -> Element<'a, Message> {
    let (icon, colour) = mark_of(&entry.action);
    let who = if entry.role == "SYSTEM" { "Kamerton".to_owned() } else { entry.name.clone().unwrap_or_else(|| entry.actor.to_string()) };
    let mut meta = vec![who, role_name(w, &entry.role)];
    if let Some(at) = unix_of(entry.at.as_deref()) {
        meta.push(w.clock(at));
    }
    let beatmap = entry.action.starts_with("map_").then(|| entry.target.parse::<i64>().ok()).flatten();
    let about = beatmap.and_then(|id| state.about(id));
    let headline: Element<'a, Message> = match beatmap {
        Some(id) => {
            let slot_of = |value: &serde_json::Value| value.get("slot").and_then(serde_json::Value::as_str).map(str::to_owned);
            let mut line = row![strong(spoken(w, &format!("desk-verb-{}", entry.action.replace('_', "-")), &entry.action), 14.0 * k, INK)].spacing(8).align_y(Alignment::Center);
            match (slot_of(&entry.before), slot_of(&entry.after)) {
                (Some(from), Some(to)) if from != to => line = line.push(tag(&from, k)).push(glyph(Icon::Right, 14.0 * k, FAINT)).push(tag(&to, k)),
                (_, Some(to)) => line = line.push(tag(&to, k)),
                _ => {}
            }
            let title = about.map(|found| found.title.clone()).filter(|title| !title.is_empty()).unwrap_or_else(|| format!("#{id}"));
            let named = match about.map(|found| found.version.clone()).filter(|version| !version.is_empty()) {
                Some(version) => format!("{title} [{version}]"),
                None => title,
            };
            line.push(one_line(named, 14.0 * k, INK, theme::SANS)).into()
        }
        None => strong(did(w, state, entry), 14.0 * k, INK),
    };
    let mut text = column![headline, words(meta.join(" · "), 12.0 * k, FAINT)].spacing(3).width(Length::Fill);
    if !entry.reason.trim().is_empty() {
        text = text.push(words(w.with("desk-quoted", &[("text", entry.reason.trim().to_owned())]), 13.0 * k, MUTED));
    }
    let mut line = row![badge(icon, colour, k), text].spacing(12).align_y(Alignment::Center);
    if let Some(set) = about.and_then(|found| found.set) {
        line = line.push(cover(state, Some(set), 64.0 * k, 38.0 * k, 7.0));
    }
    let made = container(line).padding([10.0 * k, 12.0 * k]).width(Length::Fill);
    match beatmap.filter(|_| open && !state.posting) {
        Some(id) => button(made).padding(0).style(|_, status| button::Style {
            background: matches!(status, button::Status::Hovered | button::Status::Pressed).then_some(Background::Color(workspace::HOVER)),
            text_color: INK,
            border: Border { radius: theme::CONTROL_RADIUS.into(), ..Border::default() },
            ..button::Style::default()
        }).on_press(Message::Inspect(id)).into(),
        None => made.into(),
    }
}

fn journal_view<'a>(w: &Words, state: &'a State, roles: &[String], now: i64, k: f32) -> Element<'a, Message> {
    let open = desk::allows(roles, desk::REVIEWER);
    let mut chips = row![].spacing(6);
    for kind in Kind::ALL {
        let count = state.desk.journal.iter().filter(|entry| kind.holds(&entry.action) && state.journal_person.is_none_or(|player| entry.actor == player)).count();
        chips = chips.push(chip_k(format!("{} {count}", w.t(kind.key())), state.journal_kind == kind, Message::JournalKind(kind), k));
    }
    if let Some(player) = state.journal_person {
        let name = state.desk.people.iter().find(|person| person.player == player).and_then(|person| person.name.clone())
            .or_else(|| state.desk.journal.iter().find(|entry| entry.actor == player).and_then(|entry| entry.name.clone()))
            .unwrap_or_else(|| player.to_string());
        let inside = row![words(name, 13.0 * k, INK), glyph(Icon::Close, 12.0 * k, MUTED)].spacing(6).align_y(Alignment::Center);
        chips = chips.push(button(container(inside).center_y(30.0 * k)).padding([0.0, 12.0 * k]).style(workspace::choice(true)).on_press(Message::JournalPerson(None)));
    }
    let shown = state.journal_shown();
    let mut log = column![].spacing(2);
    let mut day: Option<String> = None;
    for entry in &shown {
        let label = unix_of(entry.at.as_deref()).map(|at| first_upper(&if w.days_back(at, now) == 1 { w.t("desk-yesterday") } else { w.day(at, now) }));
        if label != day {
            if let Some(said) = &label {
                log = log.push(container(strong(said.clone(), 13.0 * k, MUTED)).padding(Padding { top: 14.0 * k, bottom: 4.0 * k, left: 12.0 * k, right: 0.0 }));
            }
            day = label;
        }
        log = log.push(journal_row(w, state, entry, open, k));
    }
    if shown.is_empty() {
        let said = if state.loading { "desk-loading" } else if state.desk.journal.is_empty() { "desk-journal-empty" } else { "desk-journal-none-here" };
        log = log.push(container(words(w.t(said), 14.0 * k, FAINT)).padding(12));
    }
    if !state.desk.journal.is_empty() && !state.journal_end {
        let more = line_button_k(w.t(if state.journal_more { "desk-loading" } else { "desk-journal-more" }), (!state.journal_more).then_some(Message::JournalMore), k);
        log = log.push(container(container(more).width(240.0 * k)).center_x(Length::Fill).padding([12, 0]));
    }
    let main = panel(column![heading_k(k, w.t("desk-journal")), chips.wrap().vertical_spacing(6), scrollable(log).direction(ui::hidden_bar()).height(Length::Fill)].spacing(12).into());
    let mut people = column![heading_k(k, w.t("desk-journal-who")), words(w.t("desk-journal-who-hint"), 12.0 * k, FAINT)].spacing(8);
    for held in &state.desk.people {
        let name = held.name.clone().unwrap_or_else(|| held.player.to_string());
        let count = state.desk.journal.iter().filter(|entry| entry.actor == held.player).count();
        let on = state.journal_person == Some(held.player);
        let roles = held.roles.iter().map(|role| role_name(w, role)).collect::<Vec<_>>().join(", ");
        let inside = row![player_face(&name, held.osu_id, 30.0 * k), column![strong(name.clone(), 14.0 * k, INK), words(roles, 12.0 * k, FAINT)].spacing(1).width(Length::Fill), meta_pill(count.to_string(), k)]
            .spacing(10)
            .align_y(Alignment::Center);
        people = people.push(button(container(inside).padding(8).width(Length::Fill)).padding(0).style(quiet(on, theme::CONTROL_RADIUS)).on_press(Message::JournalPerson(if on { None } else { Some(held.player) })));
    }
    people = people.push(Space::new().height(4)).push(line_button_k(w.t("desk-grant"), Some(Message::Tab(Tab::People)), k));
    let aside = container(people).padding(18).width(320.0 * k).style(theme::slab);
    row![main, column![aside].height(Length::Fill)].spacing(14).height(Length::Fill).into()
}

fn happened(w: &Words, event: &desk::Happened) -> String {
    let reason = event.reason.clone().unwrap_or_default();
    if event.kind == "state" {
        return spoken(w, &format!("desk-state-{}", reason.to_lowercase().replace('_', "-")), &reason);
    }
    spoken(w, &format!("desk-event-{}", event.kind.replace('_', "-")), &event.kind)
}

fn due_text(w: &Words, due: &desk::Due, now: i64) -> String {
    let what = spoken(w, &format!("desk-due-{}", due.kind.replace('_', "-")), &due.kind);
    match unix_of(due.at.as_deref()) {
        Some(at) if at > now => w.with("desk-due-in", &[("what", what), ("minutes", ((at - now + 59) / 60).to_string())]),
        Some(_) => w.with("desk-due-passed", &[("what", what)]),
        None => what,
    }
}

fn presence_line<'a>(w: &Words, seat: Option<&crate::duels::Seat>, k: f32) -> Element<'a, Message> {
    let Some(on) = seat.and_then(|seat| seat.present) else { return Space::new().into() };
    let colour = if on { GOOD } else { BAD };
    let dot = container(Space::new()).width(8.0 * k).height(8.0 * k).style(move |_| container::Style {
        background: Some(Background::Color(colour)), border: Border { radius: 4.0.into(), ..Border::default() }, ..container::Style::default()
    });
    row![dot, words(w.t(if on { "desk-present" } else { "desk-absent" }), 12.0 * k, MUTED)].spacing(6).align_y(Alignment::Center).into()
}

fn duels_view<'a>(w: &Words, state: &'a State, now: i64, k: f32) -> Element<'a, Message> {
    let shown = state.stuck_shown().map(|found| found.id.clone());
    let mut list = column![heading_k(k, w.t("desk-stuck"))].spacing(8);
    for duel in &state.desk.stuck {
        let on = Some(&duel.id) == shown.as_ref();
        let names: Vec<String> = duel.players.iter().map(|seat| seat.name.clone().unwrap_or_default()).collect();
        let a = names.first().cloned().unwrap_or_default();
        let b = names.get(1).cloned().unwrap_or_default();
        let face_a = duel.players.first().and_then(|seat| seat.osu_id);
        let face_b = duel.players.get(1).and_then(|seat| seat.osu_id);
        let faces = stack![row![player_face(&a, face_a, 32.0 * k)], row![Space::new().width(24.0 * k), player_face(&b, face_b, 32.0 * k)]].width(56.0 * k).height(32);
        let inside = row![
            faces,
            column![strong(w.with("desk-pair", &[("a", a), ("b", b)]), 15.0 * k, INK), strong(w.with(&format!("desk-stuck-{}", duel.reason.replace('_', "-")), &[("waited", ago(w, duel.since.as_deref(), now))]), 13.0 * k, if duel.reason == "verifying" { UNSURE } else { BAD })].spacing(2).width(Length::Fill),
            bold(format!("{} : {}", duel.wins.a, duel.wins.b), 15.0 * k, INK),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        let fill = if on { LIT } else { ROW };
        list = list.push(tile(fill, on.then_some((PICKED, 1.5)), inside.into(), Some(Message::Stuck(duel.id.clone()))));
    }
    if state.desk.stuck.is_empty() {
        list = list.push(words(w.t(if state.loading { "desk-loading" } else { "desk-stuck-none" }), 14.0 * k, FAINT));
    }
    let aside = container(scrollable(list).direction(ui::hidden_bar())).padding(18).width(300.0 * k).height(Length::Fill).style(theme::slab);
    let Some(duel) = state.stuck_shown() else { return row![aside, panel(Space::new().into())].spacing(14).height(Length::Fill).into() };
    let seat_a = duel.players.first();
    let seat_b = duel.players.get(1);
    let a = seat_a.and_then(|seat| seat.name.clone()).unwrap_or_default();
    let b = seat_b.and_then(|seat| seat.name.clone()).unwrap_or_default();
    let ordinal = duel.round.as_ref().map_or(duel.wins.a + duel.wins.b + 1, |round| round.ordinal);
    let mut playing = row![words(w.with("desk-round", &[("round", ordinal.to_string())]), 13.0 * k, FAINT)].spacing(6).align_y(Alignment::Center);
    if let Some(round) = &duel.round {
        playing = playing.push(tag(&round.slot, k));
        if let Some(title) = &round.title {
            playing = playing.push(words(title.clone(), 13.0 * k, MUTED));
        }
    }
    let head = container(row![
        container(row![player_face(&a, seat_a.and_then(|seat| seat.osu_id), 42.0 * k), column![strong(a.clone(), 18.0 * k, crate::duels_screen::SIDE_A), presence_line(w, seat_a, k)].spacing(3)].spacing(14).align_y(Alignment::Center)).width(Length::Fill),
        column![bold(format!("{} : {}", duel.wins.a, duel.wins.b), 34.0 * k, INK), playing].spacing(2).align_x(Alignment::Center),
        container(row![column![strong(b.clone(), 18.0 * k, crate::duels_screen::SIDE_B), presence_line(w, seat_b, k)].spacing(3).align_x(Alignment::End), player_face(&b, seat_b.and_then(|seat| seat.osu_id), 42.0 * k)].spacing(14).align_y(Alignment::Center)).width(Length::Fill).align_x(iced::alignment::Horizontal::Right),
    ].align_y(Alignment::Center))
    .padding([12, 20])
    .width(Length::Fill)
    .style(|_| container::Style { background: Some(Background::Color(ROW)), border: Border { radius: 16.0.into(), ..Border::default() }, ..container::Style::default() });
    let mut about = column![words(w.with(&format!("desk-stuck-{}-note", duel.reason.replace('_', "-")), &[("round", ordinal.to_string()), ("waited", ago(w, duel.since.as_deref(), now))]), 15.0 * k, MUTED)].spacing(4);
    if let Some(due) = &duel.deadline {
        about = about.push(words(due_text(w, due, now), 13.0 * k, UNSURE));
    }
    let warning = container(row![glyph(Icon::Clock, 20.0 * k, UNSURE), about.width(Length::Fill)].spacing(12).align_y(Alignment::Center))
        .padding([12, 16])
        .width(Length::Fill)
        .style(|_| container::Style { background: Some(Background::Color(ROW)), border: Border { radius: 12.0.into(), ..Border::default() }, ..container::Style::default() });
    let mut story = column![].spacing(6);
    for event in &duel.events {
        let at = unix_of(event.at.as_deref()).map(|at| w.clock(at)).unwrap_or_default();
        story = story.push(row![container(words(at, 13.0 * k, FAINT)).width(52.0 * k), words(happened(w, event), 14.0 * k, INK)].spacing(10).align_y(Alignment::Center));
    }
    if duel.events.is_empty() {
        story = story.push(words(w.t("desk-happened-none"), 13.0 * k, FAINT));
    }
    let ready = !state.posting;
    let mut body = column![head, warning, block(strong(w.t("desk-happened"), 16.0 * k, INK), story.into(), k),
        labelled_k(k, w.t("desk-reason"), field_kk(k, w.t("desk-reason-hint"), &state.reason, Message::Reason))].spacing(16);
    let room_match = duel.room.as_ref().and_then(|room| room.match_id);
    if room_match.is_some() || matches!(duel.reason.as_str(), "verifying" | "absent" | "disconnect" | "aborting") {
        let numbers = !state.match_id.is_empty() && !state.game_id.is_empty();
        let mut matched = row![container(labelled_k(k, w.t("desk-match"), field_kk(k, String::new(), &state.match_id, Message::MatchId))).width(Length::Fill)].spacing(8).align_y(Alignment::End);
        if let Ok(found) = state.match_id.parse::<i64>() {
            let open = button(container(row![glyph(Icon::External, 16.0 * k, INK), words(w.t("desk-open-match"), 14.0 * k, INK)].spacing(8).align_y(Alignment::Center)).center_y((theme::CONTROL_HEIGHT + 12.0) * k))
                .padding([0.0, 12.0 * k])
                .style(quiet(false, theme::CONTROL_RADIUS))
                .on_press(Message::OpenMatch(found));
            matched = matched.push(open);
        }
        let label = if state.finalize_sure { w.with("desk-finalize-sure", &[("round", ordinal.to_string())]) } else { w.t("desk-finalize") };
        let count = column![
            words(w.t("desk-finalize-about"), 13.0 * k, FAINT),
            matched,
            labelled_k(k, w.t("desk-game"), field_kk(k, w.t("desk-game-hint"), &state.game_id, Message::GameId)),
            container(main_button_kk(k, label, (ready && numbers).then_some(Message::Finalize))).width(Length::Shrink),
        ]
        .spacing(10);
        body = body.push(block(strong(w.t("desk-finalize-title"), 16.0 * k, INK), count.into(), k));
    }
    let stop = column![
        words(w.t("desk-cancel-about"), 13.0 * k, FAINT),
        container(line_button_k(w.t(if state.confirming { "desk-cancel-sure" } else { "desk-cancel-duel" }), ready.then_some(Message::CancelDuel), k)).width(Length::Shrink),
    ]
    .spacing(10);
    body = body.push(block(strong(w.t("desk-cancel-title"), 16.0 * k, INK), stop.into(), k)).push(busy(w, state, k));
    let detail = container(scrollable(body).direction(ui::hidden_bar())).padding(workspace::INSET).width(Length::Fill).height(Length::Fill).style(workspace::surface);
    row![aside, detail].spacing(14).height(Length::Fill).into()
}

fn people_view<'a>(w: &Words, state: &'a State, roles: &[String], k: f32) -> Element<'a, Message> {
    let owner = roles.iter().any(|role| role == desk::OWNER);
    let mut choices = column![].spacing(8.0 * k);
    for role in desk::GRANTED {
        let on = state.role == Some(role);
        let inside = column![
            strong(role_name(w, role), 16.0 * k, INK),
            words(w.t(&format!("desk-role-{}-means", role.to_lowercase().replace('_', "-"))), 13.0 * k, if on { MUTED } else { FAINT }),
        ]
        .spacing(3);
        choices = choices.push(button(container(inside).padding([10.0 * k, 14.0 * k]).width(Length::Fill)).padding(0).width(Length::Fill).style(quiet(on, 12.0)).on_press(Message::Role(role)));
    }
    let ready = owner && !state.player.trim().is_empty() && state.role.is_some() && !state.posting;
    let form = container(column![
        strong(w.t("desk-grant"), 20.0 * k, INK),
        column![words(w.t("desk-player"), 13.0 * k, FAINT), field_k(w.t("desk-player-hint"), &state.player, Message::Player, k)].spacing(6),
        column![words(w.t("desk-role"), 13.0 * k, FAINT), choices].spacing(6),
        column![words(w.t("desk-reason"), 13.0 * k, FAINT), field_k(w.t("desk-reason-hint"), &state.reason, Message::Reason, k)].spacing(6),
        main_button_k(w.t("desk-grant"), ready.then_some(Message::Grant), k),
    ].spacing(16.0 * k))
    .padding(22.0 * k)
    .width(440.0 * k)
    .height(Length::Fill)
    .style(theme::slab);
    let list: Element<'a, Message> = if state.desk.people.is_empty() { words(w.t(if state.loading { "desk-loading" } else { "desk-people-none" }), 15.0 * k, FAINT) } else { people_list(w, state, owner, k) };
    row![panel(column![strong(w.t("desk-people"), 20.0 * k, INK), scrollable(list).direction(ui::hidden_bar())].spacing(14.0 * k).into()), form].spacing(14).height(Length::Fill).into()
}

fn meta_pill<'a>(value: String, k: f32) -> Element<'a, Message> {
    container(strong(value, 13.0 * k, INK)).padding([4.0 * k, 9.0 * k]).style(|_| container::Style {
        background: Some(Background::Color(iced::color!(0x201319))),
        border: Border { radius: 12.0.into(), ..Border::default() }, ..container::Style::default()
    }).into()
}

fn process_button<'a>(w: &Words, state: &State, label: String, press: Option<Message>, k: f32) -> Element<'a, Message> {
    if state.posting {
        button(container(busy(w, state, k)).center_x(Length::Fill).center_y(workspace::CONTROL * k)).padding([0, 12]).style(theme::primary).into()
    } else { main_button_k(label, press, k) }
}

fn busy<'a>(w: &Words, state: &State, k: f32) -> Element<'a, Message> {
    if !state.posting && !state.loading && state.inspecting.is_none() { return Space::new().into(); }
    row![iced::widget::canvas(crate::intro::Spinner { seconds: state.busy_frame as f32 * 0.06, alpha: 1.0 }).width(18.0 * k).height(18.0 * k), words(w.t("desk-loading"), 13.0 * k, MUTED)].spacing(8).align_y(Alignment::Center).into()
}

fn workers<'a>(w: &Words, state: &State, k: f32) -> Element<'a, Message> {
    let context = state.presence_context().0;
    let section = context.split(':').next().unwrap_or_default();
    let mut line = row![].spacing(8).align_y(Alignment::Center);
    for person in state.presence.iter().filter(|person| !person.mine && person.section == section) {
        let colour = occupancy::colour(person.player);
        let name = container(strong(person.name.clone(), 13.0 * k, colour)).padding([4, 9]).style(move |_| container::Style { border: Border { color: colour, width: 1.0, radius: 8.0.into() }, ..Default::default() });
        line = line.push(name);
    }
    if state.refresh_failed { line = line.push(words(w.t("desk-refresh-failed"), 12.0 * k, MUTED)); }
    if state.presence_failed { line = line.push(words(w.t("desk-live-offline"), 12.0 * k, MUTED)); }
    line.wrap().into()
}

pub fn view<'a>(state: &'a State, roles: &[String], w: &Words, now: i64, wide: f32) -> Element<'a, Message> {
    let k = 1.0;
    let body: Element<'a, Message> = match state.tab {
        Tab::Queue => queue_view(w, state, now, k, wide),
        Tab::Label => label_view(w, state, k),
        Tab::Batches => batches_view(w, state, now, k),
        Tab::Catalog => catalog_view(w, state, wide, k),
        Tab::Model => model_view(w, state, roles, now, k),
        Tab::Duels => duels_view(w, state, now, k),
        Tab::Journal => journal_view(w, state, roles, now, k),
        Tab::People => people_view(w, state, roles, k),
    };
    let mut page = column![tabs(w, state, roles, k), workers(w, state, k)].spacing(14).width(Length::Fill).height(Length::Fill);
    if let Some(notice) = &state.notice { page = page.push(words(notice.clone(), 13.0, GOOD)); }
    if let Some(why) = &state.failed {
        page = page.push(strong(w.with("desk-failed", &[("why", why.clone())]), 14.0, BAD));
    }
    if state.inspecting.is_some() {
        page = page.push(words(w.t("desk-loading"), 14.0, MUTED));
    }
    page = page.push(body);
    let limit = match state.tab { Tab::Catalog => 1200.0, Tab::Label => 1180.0, _ => 1400.0 };
    container(container(page).max_width(limit).id(iced::widget::Id::new("desk-workspace"))).padding(Padding { top: 8.0, right: 40.0, bottom: 24.0, left: 40.0 }).center_x(Length::Fill).height(Length::Fill).id(iced::widget::Id::new("desk-work-area")).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn presence(revision: &str) -> Message {
        Message::Presence(Ok(desk::PresenceUpdate { revision: Some(revision.into()), ..Default::default() }))
    }

    #[test]
    fn catalogue_presence_tracks_cells_and_navigation_without_changing_duel_slots() {
        let mut state = State::default();
        state.tab = Tab::Catalog;
        state.band = Some(("FM4".into(), 9));
        assert_eq!(state.presence_context(), ("catalog:FM4:9".into(), None));
        let slots = state.catalog_slots();
        assert_eq!(slots.len(), 27);
        for slot in ["HD4", "HR4", "DT4", "FM4", "EZ2", "FL2"] { assert!(slots.contains(&slot.to_owned())); }
        assert_eq!(desk::SLOTS.len(), 9);
        state.tab = Tab::Model;
        assert_eq!(state.presence_context(), ("model".into(), None));
        assert_ne!(occupancy::colour(1), occupancy::colour(2));
        assert_ne!(occupancy::colour(2), occupancy::colour(3));
    }

    #[test]
    fn refresh_keeps_selection_and_drafts_and_rejects_a_pre_save_response() {
        let w = Words::new(crate::lang::Lang::En);
        let task = |id| Task { map: desk::Map { beatmap_id: id, title: Some("Song".into()), stars: Some(5.5), ..Default::default() }, ..Default::default() };
        let mut state = State::default();
        state.desk.tasks = vec![task(1), task(2)];
        state.reason = "my explanation".into();
        state.name = "my batch".into();
        assert_eq!(state.update(presence("one"), &w), vec![Effect::Refresh(1)]);
        assert!(state.update(presence("two"), &w).is_empty(), "only one refresh may run at a time");
        let mut fresh = state.desk.clone();
        fresh.tasks.reverse();
        state.update(Message::Refreshed(1, Ok(fresh)), &w);
        assert_eq!(state.task().unwrap().map.beatmap_id, 1);
        assert_eq!(state.reason, "my explanation");
        assert_eq!(state.name, "my batch");
        assert!(state.update(presence("one"), &w).is_empty());
        assert_eq!(state.update(presence("two"), &w), vec![Effect::Refresh(2)]);
        state.update(Message::Decide("APPROVE", Some("NM1".into())), &w);
        assert!(state.update(Message::Refreshed(2, Ok(Desk::default())), &w).is_empty());
        assert_eq!(state.desk.tasks.len(), 2);
        state.posting = false;
        assert_eq!(state.update(presence("two"), &w), vec![Effect::Refresh(3)]);
        state.update(Message::Refreshed(3, Ok(Desk::default())), &w);
        assert!(state.task().is_none());
        assert!(state.reason.is_empty(), "a draft must not attach itself to a different map");
    }

    #[test]
    fn refresh_failure_retries_and_catalogue_replies_cannot_replace_another_cell() {
        let w = Words::new(crate::lang::Lang::En);
        let mut state = State::default();
        state.update(presence("one"), &w);
        state.update(Message::Refreshed(1, Err("offline".into())), &w);
        assert!(state.refresh_failed);
        assert_eq!(state.update(presence("one"), &w), vec![Effect::Refresh(2)]);
        state.band = Some(("NM1".into(), 0));
        state.update(Message::Held("DT1".into(), 0, Ok(vec![Held::default()])), &w);
        assert!(state.held.is_empty());
    }

    #[test]
    fn batches_exclude_reference_owc_and_survive_refresh_reordering() {
        let w = Words::new(crate::lang::Lang::En);
        let mut state = State::default();
        state.desk.cells = vec![Cell { corpus: "owc-2021-2025-v1".into(), year: 2025, ..Default::default() }, Cell { corpus: "own-v1".into(), stage: "My batch".into(), ..Default::default() }];
        assert_eq!(state.tournaments().len(), 1);
        assert_eq!(state.tournaments()[0].stage.as_deref(), Some("My batch"));
        state.update(presence("one"), &w);
        let mut fresh = state.desk.clone();
        fresh.cells.push(Cell { corpus: "own-v1".into(), stage: "New batch".into(), year: 2026, ..Default::default() });
        state.update(Message::Refreshed(1, Ok(fresh)), &w);
        assert_eq!(state.tournaments()[state.tournament.unwrap()].stage.as_deref(), Some("My batch"));
    }

    #[test]
    fn a_saved_batch_opens_after_reload_and_a_rejected_batch_keeps_input() {
        let w = Words::new(crate::lang::Lang::Ru);
        let mut state = State { adding: true, name: "Samples".into(), pool: Pool(text_editor::Content::with_text("123")), ..Default::default() };
        assert!(matches!(state.update(Message::AddTournament, &w).first(), Some(Effect::Post(Post::Tournament { year: 0, .. }))));
        state.update(Message::Posted(Err("BATCH_UNSUPPORTED".into())), &w);
        assert!(state.adding && state.pool.0.text().contains("123"));
        assert_eq!(state.failed.as_deref(), Some(w.t("desk-batch-unsupported").as_str()));
        state.update(Message::AddTournament, &w);
        state.update(Message::Posted(Ok(())), &w);
        state.update(Message::Loaded(Ok(Desk { cells: vec![Cell { corpus: "own-v1".into(), year: 0, stage: "Samples".into(), ..Default::default() }], ..Default::default() })), &w);
        assert!(!state.adding && state.tournament == Some(0) && state.notice.is_some());
    }

    #[test]
    fn queue_groups_sets_after_filtering_and_keeps_difficulty_selection() {
        let mut state = State::default();
        for (id, set, stars) in [(1, Some(10), 4.2), (2, Some(10), 5.2), (3, Some(20), 5.4), (4, None, 5.3), (5, None, 5.3)] {
            state.desk.tasks.push(Task { map: desk::Map { beatmap_id: id, beatmapset_id: set, stars: Some(stars), ..Default::default() }, ..Default::default() });
        }
        assert_eq!(state.tasks().len(), 4);
        state.stars = Some((5.0, 5.49));
        assert_eq!(state.tasks().iter().map(|task| task.map.beatmap_id).collect::<Vec<_>>(), [2, 3, 4, 5]);
        state.picked = Some(2);
        assert_eq!(state.task().unwrap().map.beatmap_id, 2);
        state.stars = None;
        assert_eq!(state.task().unwrap().map.beatmap_id, 2);
    }

    #[test]
    fn star_handles_stay_ordered_and_full_range_includes_unknown_ratings() {
        let w = Words::new(crate::lang::Lang::Ru);
        let mut state = State::default();
        state.update(Message::StarsAt(0.4, true), &w);
        assert_eq!(state.stars, Some((4.0, 10.0)));
        state.update(Message::StarsAt(0.8, true), &w);
        assert_eq!(state.stars, Some((4.0, 8.0)));
        state.update(Message::StarsAt(0.2, false), &w);
        assert_eq!(state.stars, Some((4.0, 4.0)));
        state.update(Message::StarsAt(f32::NAN, true), &w);
        assert_eq!(state.stars, Some((4.0, 4.0)));
        state.update(Message::StarsAt(-1.0, true), &w);
        state.update(Message::StarsAt(2.0, true), &w);
        assert_eq!(state.stars, None);
        state.desk.tasks.push(Task { map: desk::Map { beatmap_id: 1, ..Default::default() }, ..Default::default() });
        assert_eq!(state.tasks().len(), 1);
    }

    #[test]
    fn reviewed_card_stays_until_reload_and_a_failed_reload_releases_buttons() {
        let w = Words::new(crate::lang::Lang::Ru);
        let mut state = State::default();
        state.inspected = Some(Task { map: desk::Map { beatmap_id: 7, ..Default::default() }, ..Default::default() });
        state.update(Message::Decide("APPROVE", Some("TB".into())), &w);
        assert!(state.posting);
        state.update(Message::Posted(Ok(())), &w);
        assert_eq!(state.task().unwrap().map.beatmap_id, 7);
        assert!(state.posting && state.loading);
        state.update(Message::Loaded(Err("offline".into())), &w);
        assert!(!state.posting && !state.loading);
        assert_eq!(state.task().unwrap().map.beatmap_id, 7);
        state.update(Message::Decide("APPROVE", Some("TB".into())), &w);
        state.update(Message::Posted(Ok(())), &w);
        state.update(Message::Loaded(Ok(Desk::default())), &w);
        assert!(state.task().is_none() && !state.posting && !state.loading);
    }

    #[test]
    fn batches_accept_unlabelled_maps_but_not_set_ids_or_duplicates() {
        assert_eq!(read_pool("123\nhttps://osu.ppy.sh/beatmaps/456\nDT1 789\n123\nhttps://osu.ppy.sh/beatmapsets/999\n-2"),
            [("PENDING".into(), 123), ("PENDING".into(), 456), ("DT1".into(), 789)]);
    }

    #[test]
    fn a_model_version_is_shown_by_its_day_and_labels() {
        assert_eq!(short_version("owc-202610231200-606"), "owc-1023-606");
        assert_eq!(short_version("handmade"), "handmade");
    }

    #[test]
    fn a_pool_is_read_slot_by_slot_from_links_and_bare_numbers() {
        let found = read_pool("NM1 https://osu.ppy.sh/b/1222416\nhd2 https://osu.ppy.sh/beatmapsets/80#osu/8\nTB 5397135\nтекст без ссылки\n");
        assert_eq!(found, [("NM1".to_owned(), 1_222_416), ("HD2".to_owned(), 8), ("TB".to_owned(), 5_397_135)]);
    }

    #[test]
    fn a_decision_goes_without_a_reason_and_only_one_post_runs_at_a_time() {
        let w = Words::new(crate::lang::Lang::Ru);
        let mut state = State::default();
        state.desk.tasks = vec![Task { kind: "review".into(), reason: "uncertain_revealed".into(), slot: Some("NM3".into()), map: desk::Map { beatmap_id: 7, title: Some("Song".into()), stars: Some(5.5), ..desk::Map::default() }, ..Task::default() }];
        assert!(state.update(Message::NotThis(true), &w).is_empty() && state.moving);
        let effects = state.update(Message::Decide("MOVE", Some("NM2".into())), &w);
        assert_eq!(effects, [Effect::Post(Post::Review { beatmap: 7, decision: "MOVE", slot: Some("NM2".into()), secondary: Some(None), reason: String::new() })]);
        assert!(state.update(Message::Decide("REJECT", None), &w).is_empty());
        assert_eq!(state.update(Message::Posted(Ok(())), &w), [Effect::Load]);
        assert!(state.posting && !state.moving);
        assert_eq!(state.task().unwrap().map.beatmap_id, 7);
        assert!(state.update(Message::Decide("APPROVE", None), &w).is_empty());
        let desk = state.desk.clone();
        state.update(Message::Loaded(Ok(desk)), &w);
        assert!(!state.posting && state.pending_review.is_none());
        let _ = state.update(Message::Reason("короткие группы".into()), &w);
        let effects = state.update(Message::Decide("APPROVE", None), &w);
        assert_eq!(effects, [Effect::Post(Post::Review { beatmap: 7, decision: "APPROVE", slot: None, secondary: Some(None), reason: "короткие группы".into() })]);
    }

    #[test]
    fn a_role_is_given_by_osu_id_or_name_and_taken_back_with_a_second_press() {
        let w = Words::new(crate::lang::Lang::Ru);
        let mut state = State::default();
        let _ = state.update(Message::Player(" NaumRedlo ".into()), &w);
        let _ = state.update(Message::Role(desk::REVIEWER), &w);
        assert_eq!(state.update(Message::Grant, &w), [Effect::Post(Post::Grant { who: "NaumRedlo".into(), role: desk::REVIEWER.into(), reason: String::new() })]);
        let _ = state.update(Message::Posted(Ok(())), &w);
        assert!(state.update(Message::Revoke(2, desk::REVIEWER.into()), &w).is_empty());
        assert_eq!(state.update(Message::Revoke(2, desk::REVIEWER.into()), &w), [Effect::Post(Post::Revoke { player: 2, role: desk::REVIEWER.into(), reason: String::new() })]);
    }

    #[test]
    fn training_on_unchanged_labels_is_answered_without_waiting() {
        let w = Words::new(crate::lang::Lang::Ru);
        let mut state = State::default();
        assert_eq!(state.update(Message::Train, &w), [Effect::Train(String::new())]);
        assert!(state.training.is_some());
        let _ = state.update(Message::TrainStarted(Err("NOTHING_NEW".into())), &w);
        assert!(state.training.is_none() && state.nothing_new && state.failed.is_none());
    }

    #[test]
    fn a_training_is_followed_by_its_job_until_the_server_says_it_ended() {
        let w = Words::new(crate::lang::Lang::Ru);
        let mut state = State { reason: "новые метки".into(), ..State::default() };
        assert_eq!(state.update(Message::Train, &w), [Effect::Train("новые метки".into())]);
        assert!(state.update(Message::Train, &w).is_empty(), "one training at a time");
        assert_eq!(state.update(Message::TrainStarted(Ok(Some("j1".into()))), &w), [Effect::Load]);
        state.loading = false;
        let job = |state: &str, version: Option<&str>, error: Option<&str>| desk::Job { job: "j1".into(), state: state.into(), version: version.map(Into::into), error: error.map(Into::into) };
        assert_eq!(state.update(Message::PresenceTick, &w), [Effect::TrainingJob("j1".into())]);
        assert!(state.update(Message::PresenceTick, &w).is_empty(), "no second question while the first is unanswered");
        let _ = state.update(Message::Loaded(Ok(Desk { versions: vec![desk::Version::default(); 30], ..Desk::default() })), &w);
        assert!(state.training.is_some(), "a longer list of versions says nothing once there is a job");
        assert!(state.update(Message::TrainingState("j1".into(), Ok(job("running", None, None))), &w).is_empty());
        assert!(state.update(Message::TrainingState("j1".into(), Err("offline".into())), &w).is_empty() && state.training.is_some());
        assert_eq!(state.update(Message::PresenceTick, &w), [Effect::TrainingJob("j1".into())]);
        state.loading = false;
        assert_eq!(state.update(Message::TrainingState("j1".into(), Ok(job("done", Some("kamerton-21"), None))), &w), [Effect::Load]);
        assert!(state.training.is_none() && state.version.as_deref() == Some("kamerton-21"));
        let mut failing = State::default();
        let _ = failing.update(Message::Train, &w);
        let _ = failing.update(Message::TrainStarted(Ok(Some("j1".into()))), &w);
        let _ = failing.update(Message::TrainingState("j1".into(), Ok(job("failed", None, Some("ValueError: no maps")))), &w);
        assert!(failing.training.is_none() && failing.training_failed.as_deref() == Some("ValueError: no maps"));
        let mut forgotten = State::default();
        let _ = forgotten.update(Message::Train, &w);
        let _ = forgotten.update(Message::TrainStarted(Ok(Some("j2".into()))), &w);
        assert!(forgotten.update(Message::TrainingState("j1".into(), Err("UNKNOWN_JOB".into())), &w).is_empty() && forgotten.training.is_some(), "another job's answer is not ours");
        let _ = forgotten.update(Message::TrainingState("j2".into(), Err("UNKNOWN_JOB".into())), &w);
        assert!(forgotten.training.is_none() && forgotten.training_lost);
    }

    #[test]
    fn a_slot_without_a_name_shows_only_its_tag() {
        let w = Words::new(crate::lang::Lang::Ru);
        assert_eq!(skill(&w, "NM5"), "Скорость");
        assert_eq!(skill(&w, "FM4"), "Скорость во фримоде");
        assert_eq!(skill(&w, "EZ1"), "Easy");
        assert_eq!(skill(&w, "NM9"), "");
        assert_eq!(feature(&w, "no.such_path"), "no.such_path");
    }

    #[test]
    fn deleting_a_version_takes_a_second_press() {
        let w = Words::new(crate::lang::Lang::Ru);
        let mut state = State::default();
        state.version = Some("kamerton-2".into());
        assert!(state.update(Message::Delete, &w).is_empty() && state.confirming);
        assert_eq!(state.update(Message::Delete, &w), [Effect::Post(Post::Delete { version: "kamerton-2".into(), reason: String::new() })]);
    }


    #[test]
    fn a_training_that_never_lands_stops_being_awaited() {
        let w = Words::new(crate::lang::Lang::Ru);
        let mut state = State::default();
        state.training = Some(Training::new(0));
        assert_eq!(state.update(Message::Again, &w), [Effect::Load]);
        state.loading = false;
        state.training = std::time::Instant::now().checked_sub(TRAINING_WAIT * 2).map(|since| Training { since, ..Training::new(0) });
        if state.training.is_some() {
            assert!(state.update(Message::Again, &w).is_empty());
            assert!(state.training.is_none() && state.training_lost);
        }
    }

    #[test]
    fn labelling_walks_a_tournament_and_skips_what_is_checked() {
        let w = Words::new(crate::lang::Lang::Ru);
        let map = |id: i64, verified: bool| desk::CorpusMap { beatmap_id: id, verified, label: "NM1".into(), ..desk::CorpusMap::default() };
        let mut state = State::default();
        state.desk.cells = vec![Cell { corpus: "own-v1".into(), year: 2024, stage: "Russian Cup".into(), slot: "NM1".into(), label: None, maps: vec![map(1, true), map(2, false), map(3, false)] }];
        assert_eq!(state.next_map().map(|(_, map)| map.beatmap_id), Some(2));
        let effects = state.update(Message::MapLabel("NM2".into()), &w);
        assert!(matches!(&effects[0], Effect::Post(Post::Label { beatmap: Some(2), label, .. }) if label == "NM2"));
        assert_eq!(state.next_map().map(|(_, map)| map.beatmap_id), Some(2));
        let _ = state.update(Message::Posted(Ok(())), &w);
        assert_eq!(state.next_map().map(|(_, map)| map.beatmap_id), Some(3));
        let _ = state.update(Message::Skip, &w);
        assert!(state.next_map().is_none());
    }

    #[test]
    fn saved_labels_can_be_corrected_and_failed_saves_keep_the_original() {
        let w = Words::new(crate::lang::Lang::En);
        let mut state = State::default();
        let map = desk::CorpusMap { beatmap_id: 7, verified: true, label: "NM1".into(), ..desk::CorpusMap::default() };
        let cell = Cell { corpus: "own-v1".into(), year: 2024, stage: "Cup".into(), slot: "FM4".into(), maps: vec![map], ..Cell::default() };
        state.desk.cells.push(cell.clone());
        assert!(state.next_map().is_none());
        state.update(Message::Revisit(cell, 7), &w);
        let effects = state.update(Message::MapLabel("FM4".into()), &w);
        assert!(matches!(&effects[0], Effect::Post(Post::Label { beatmap: Some(7), label, .. }) if label == "FM4"));
        assert!(state.update(Message::MapLabel("DT2".into()), &w).is_empty());
        state.update(Message::Posted(Err("offline".into())), &w);
        assert_eq!(state.next_map().unwrap().1.label, "NM1");
        state.update(Message::MapLabel("FM4".into()), &w);
        state.update(Message::Posted(Ok(())), &w);
        assert_eq!(state.desk.cells[0].maps[0].label, "FM4");
        assert!(state.next_map().is_none());
    }

    #[test]
    fn catalogue_review_uses_the_requested_map_and_ignores_an_older_response() {
        let w = Words::new(crate::lang::Lang::En);
        let mut state = State::default();
        assert_eq!(state.update(Message::Inspect(7), &w), vec![Effect::Inspect(7)]);
        assert_eq!(state.presence_context(), ("queue".into(), Some(7)));
        state.update(Message::Inspect(8), &w);
        state.update(Message::Inspected(7, Ok(Task::default())), &w);
        assert!(state.inspected.is_none());
        let task = Task { kind: "review".into(), reason: "manual".into(), slot: Some("NM1".into()), map: desk::Map { beatmap_id: 8, ..Default::default() }, ..Default::default() };
        state.update(Message::Inspected(8, Ok(task)), &w);
        let effects = state.update(Message::Decide("MOVE", Some("FM4".into())), &w);
        assert!(matches!(&effects[0], Effect::Post(Post::Review { beatmap: 8, decision: "MOVE", slot: Some(slot), .. }) if slot == "FM4"));
    }

    #[test]
    fn presence_failure_discards_stale_map_occupancy() {
        let w = Words::new(crate::lang::Lang::En);
        let mut state = State::default();
        state.presence_busy = true;
        state.update(Message::Presence(Ok(desk::PresenceUpdate { people: vec![desk::Presence { name: "Reviewer".into(), beatmap: Some(7), ..Default::default() }], ..Default::default() })), &w);
        assert!(!state.presence_busy && !state.presence_failed && state.presence.len() == 1);
        state.update(Message::Presence(Err("offline".into())), &w);
        assert!(state.presence_failed && state.presence.is_empty());
    }

    #[test]
    fn the_journal_filters_by_kind_and_person_and_keeps_older_pages_through_refreshes() {
        let w = Words::new(crate::lang::Lang::En);
        let entry = |id: i64, actor: i64, action: &str| desk::Entry { id, actor, action: action.into(), ..Default::default() };
        let mut state = State { tab: Tab::Journal, ..State::default() };
        state.update(Message::Loaded(Ok(Desk { journal: vec![entry(12, 1, "map_move"), entry(11, 2, "grant"), entry(10, 1, "publish_model")], ..Desk::default() })), &w);
        state.update(Message::JournalKind(Kind::Maps), &w);
        assert_eq!(state.journal_shown().iter().map(|entry| entry.id).collect::<Vec<_>>(), [12]);
        state.update(Message::JournalKind(Kind::All), &w);
        state.update(Message::JournalPerson(Some(1)), &w);
        assert_eq!(state.journal_shown().iter().map(|entry| entry.id).collect::<Vec<_>>(), [12, 10]);
        assert_eq!(state.update(Message::JournalMore, &w), vec![Effect::Older(10)]);
        assert!(state.update(Message::JournalMore, &w).is_empty(), "one page at a time");
        state.update(Message::JournalOlder(Ok(vec![entry(9, 1, "map_add"), entry(8, 2, "discover")])), &w);
        state.update(Message::Loaded(Ok(Desk { journal: vec![entry(13, 2, "map_approve"), entry(12, 1, "map_move")], ..Desk::default() })), &w);
        assert_eq!(state.desk.journal.iter().map(|entry| entry.id).collect::<Vec<_>>(), [13, 12, 11, 10, 9, 8], "older pages survive a refresh");
        state.update(Message::JournalMore, &w);
        state.update(Message::JournalOlder(Ok(Vec::new())), &w);
        assert!(state.journal_end && state.update(Message::JournalMore, &w).is_empty());
        assert_eq!((Kind::of("corpus_label"), Kind::of("manual_finalize"), Kind::of("map_quarantine")), (Kind::Labels, Kind::Duels, Kind::Maps));
    }

    #[test]
    fn an_operator_gets_the_room_match_and_confirms_before_counting_a_round() {
        let w = Words::new(crate::lang::Lang::En);
        let stuck = desk::Stuck { id: "s1".into(), room: Some(desk::StuckRoom { match_id: Some(114_482_093), ..Default::default() }), ..Default::default() };
        let mut state = State { tab: Tab::Duels, ..State::default() };
        state.update(Message::Loaded(Ok(Desk { stuck: vec![stuck], ..Desk::default() })), &w);
        assert_eq!(state.match_id, "114482093");
        state.update(Message::MatchId("https://osu.ppy.sh/community/matches/120000001".into()), &w);
        assert_eq!(state.match_id, "120000001", "a pasted link keeps only the match id");
        state.update(Message::GameId("602118554".into()), &w);
        assert!(state.update(Message::Finalize, &w).is_empty() && state.finalize_sure, "the first press only asks");
        let effects = state.update(Message::Finalize, &w);
        assert!(matches!(&effects[0], Effect::Post(Post::Finalize { match_id: 120_000_001, game_id: 602_118_554, .. })));
        assert_eq!(state.update(Message::OpenMatch(5), &w), vec![Effect::Open("https://osu.ppy.sh/community/matches/5".into())]);
        assert_eq!((desk::match_of("https://osu.ppy.sh/mp/77/"), desk::match_of("77"), desk::match_of("https://example.com/77")), (Some(77), Some(77), None));
    }

    #[test]
    fn the_owner_renames_excludes_and_deletes_batches_and_single_maps_with_a_second_press_to_delete() {
        let w = Words::new(crate::lang::Lang::En);
        let batch = |name: &str, year: i32| desk::Batch { name: name.into(), year, maps: 2, ..Default::default() };
        let mut state = State { tab: Tab::Batches, ..State::default() };
        state.update(Message::Loaded(Ok(Desk { batches: vec![batch("Cup", 2024), batch("Other", 0)], ..Desk::default() })), &w);
        assert!(state.update(Message::RenameBatch, &w).is_empty(), "nothing changed yet");
        state.update(Message::BatchName("Cup Finals".into()), &w);
        assert_eq!((state.batch.clone(), state.batch_year.as_str()), (Some(("Cup".to_owned(), 2024)), "2024"), "typing holds the shown batch");
        let effects = state.update(Message::RenameBatch, &w);
        assert!(matches!(&effects[0], Effect::Post(Post::RenameBatch { name, year: 2024, new_name, new_year: 2024, .. }) if name == "Cup" && new_name == "Cup Finals"));
        state.posting = false;
        state.update(Message::Batch("Other".into(), 0), &w);
        assert!(state.update(Message::DeleteBatch(Some(7)), &w).is_empty() && state.batch_sure == Some(Some(7)));
        assert!(state.update(Message::DeleteBatch(None), &w).is_empty(), "another target asks again");
        let effects = state.update(Message::DeleteBatch(None), &w);
        assert!(matches!(&effects[0], Effect::Post(Post::DeleteBatch { year: 0, beatmap: None, .. })));
        state.posting = false;
        let effects = state.update(Message::ExcludeBatch(Some(7), true), &w);
        assert!(matches!(&effects[0], Effect::Post(Post::ExcludeBatch { beatmap: Some(7), excluded: true, .. })));
        assert_eq!(state.presence_context().0, "label", "the batches tab reports itself as labelling to older servers");
    }

    #[test]
    fn an_archive_inserts_only_the_ticked_difficulties_and_a_failed_lookup_is_tried_again_later() {
        let w = Words::new(crate::lang::Lang::En);
        let packed = |beatmap: i64, stars: Option<f32>, standard: bool| desk::Packed { beatmap, version: format!("v{beatmap}"), stars, standard };
        let mut state = State { tab: Tab::Label, adding: true, ..State::default() };
        state.update(Message::Archived(vec![packed(3, Some(5.9), true), packed(1, Some(3.2), true), packed(9, None, false)]), &w);
        assert_eq!(state.archive_picked.iter().copied().collect::<Vec<_>>(), [3], "the hardest standard difficulty is ticked");
        state.update(Message::ArchivePick(1), &w);
        state.update(Message::ArchiveTake, &w);
        assert_eq!(state.pool.0.text().trim_end(), "https://osu.ppy.sh/b/3\nhttps://osu.ppy.sh/b/1");
        assert!(state.archive.is_empty());
        state.update(Message::Archived(Vec::new()), &w);
        assert_eq!(state.notice.as_deref(), Some(w.t("desk-archive-empty").as_str()));
        let mut state = State { tab: Tab::Journal, ..State::default() };
        let entry = desk::Entry { id: 1, action: "map_add".into(), target: "42".into(), ..Default::default() };
        assert_eq!(state.update(Message::Loaded(Ok(Desk { journal: vec![entry], ..Desk::default() })), &w), vec![Effect::About(42)]);
        assert!(state.update(Message::About(42, None), &w).is_empty(), "a failed lookup waits before trying again");
        state.missed_at.insert(42, std::time::Instant::now() - ABOUT_RETRY - std::time::Duration::from_secs(1));
        assert_eq!(state.update(Message::PresenceTick, &w), vec![Effect::About(42)], "and is tried again after a minute");
    }

    #[test]
    fn a_secondary_slot_is_kept_unless_the_reviewer_changes_it_and_never_sent_when_removing_a_map() {
        let w = Words::new(crate::lang::Lang::En);
        let task = |id: i64, secondary: Option<&str>| Task { kind: "review".into(), reason: "manual".into(), slot: Some("NM1".into()),
            secondary: secondary.map(Into::into), map: desk::Map { beatmap_id: id, ..Default::default() }, ..Default::default() };
        let mut state = State::default();
        state.desk.tasks = vec![task(7, Some("HR1")), task(8, None)];
        let effects = state.update(Message::Decide("APPROVE", None), &w);
        assert!(matches!(&effects[0], Effect::Post(Post::Review { beatmap: 7, secondary: Some(Some(slot)), .. }) if slot == "HR1"), "an untouched choice keeps the map's own");
        state.posting = false;
        state.update(Message::Secondary(None), &w);
        let effects = state.update(Message::Decide("APPROVE", None), &w);
        assert!(matches!(&effects[0], Effect::Post(Post::Review { secondary: Some(None), .. })));
        state.posting = false;
        state.update(Message::Pick(8), &w);
        assert_eq!(state.secondary_for(state.task().unwrap()), None, "a choice belongs to its own map");
        state.update(Message::Secondary(Some("HD2".into())), &w);
        let effects = state.update(Message::Decide("MOVE", Some("HD2".into())), &w);
        assert!(matches!(&effects[0], Effect::Post(Post::Review { secondary: Some(None), .. })), "a secondary equal to the new primary is dropped");
        state.posting = false;
        let effects = state.update(Message::Decide("REJECT", None), &w);
        assert!(matches!(&effects[0], Effect::Post(Post::Review { decision: "REJECT", secondary: None, .. })));
    }

    #[test]
    fn one_map_joins_the_catalogue_in_the_chosen_cell_or_opens_for_a_slot() {
        let w = Words::new(crate::lang::Lang::En);
        let mut state = State { tab: Tab::Catalog, band: Some(("NM3".into(), 2)), ..State::default() };
        state.update(Message::AddingMap(true), &w);
        assert_eq!(state.map_slot.as_deref(), Some("NM3"), "the open cell lends its slot");
        assert!(state.update(Message::MapLink("568".into()), &w).is_empty(), "a bare number waits for Enter");
        assert_eq!(state.update(Message::MapLink("5687673".into()), &w), Vec::new());
        assert_eq!(state.update(Message::LookMap, &w), vec![Effect::About(5687673)]);
        assert_eq!(state.update(Message::MapLink("https://osu.ppy.sh/b/75".into()), &w), vec![Effect::About(75)]);
        state.update(Message::About(75, Some(desk::About { standard: false, ..Default::default() })), &w);
        assert!(state.update(Message::AddMap, &w).is_empty(), "another mode is never sent");
        state.update(Message::MapLink("https://osu.ppy.sh/beatmapsets/2612145#osu/5687673".into()), &w);
        state.update(Message::Reason("aim".into()), &w);
        let effects = state.update(Message::AddMap, &w);
        assert_eq!(effects, vec![Effect::AddMap { beatmap: 5687673, slot: Some("NM3".into()), reason: "aim".into() }]);
        assert!(state.update(Message::AddMap, &w).is_empty(), "one request at a time");
        state.update(Message::MapAdded(5687673, Err("NOT_STANDARD".into())), &w);
        assert_eq!(state.map_failed.as_deref(), Some(w.t("desk-add-map-not-standard").as_str()));
        assert!(state.adding_map && !state.map_link.is_empty(), "a refusal keeps the input");
        state.update(Message::AddMap, &w);
        let task = Task { slot: Some("NM3".into()), map: desk::Map { beatmap_id: 5687673, stars: Some(5.71), ..Default::default() }, ..Default::default() };
        let effects = state.update(Message::MapAdded(5687673, Ok(desk::Added { outcome: "unassigned".into(), status: None, task: Some(task) })), &w);
        assert_eq!(effects, vec![Effect::Load, Effect::Cell("NM3".into(), 1)]);
        assert_eq!((state.band.clone(), state.added, state.adding_map, state.tab), (Some(("NM3".into(), 1)), Some(5687673), false, Tab::Catalog));
        assert!(state.map_link.is_empty() && state.reason.is_empty());
        state.loading = false;
        state.update(Message::AddingMap(true), &w);
        state.update(Message::MapSlot(None), &w);
        state.update(Message::MapLink("2".into()), &w);
        assert_eq!(state.update(Message::AddMap, &w), vec![Effect::AddMap { beatmap: 2, slot: None, reason: String::new() }]);
        let task = Task { kind: "review".into(), reason: "manual".into(), map: desk::Map { beatmap_id: 2, ..Default::default() }, ..Default::default() };
        state.update(Message::MapAdded(2, Ok(desk::Added { task: Some(task), ..Default::default() })), &w);
        assert_eq!((state.tab, state.task().map(|task| task.map.beatmap_id)), (Tab::Queue, Some(2)), "without a slot the map opens for a decision");
        assert_eq!(add_failure(&w, "404"), w.t("desk-add-map-unsupported"));
    }
    #[test]
    fn linked_maps_restore_titles_and_keep_mod_stars_separate_across_refreshes() {
        let w = Words::new(crate::lang::Lang::En);
        let map = desk::Map { beatmap_id: 7, title: Some("7".into()), mods: vec!["DT".into()], revision: Some("old".into()), ..Default::default() };
        let mut state = State::default();
        state.desk.tasks.push(Task { map: map.clone(), ..Default::default() });
        let effects = state.wanted();
        assert!(effects.contains(&Effect::Details(7, "old".into())));
        assert!(effects.contains(&Effect::About(7)));
        assert!(state.wanted().is_empty(), "do not repeat requests on every frame");
        state.update(Message::About(7, Some(desk::About { title: "Voiceless Fish".into(), version: "Insane".into(), status: "graveyard".into(), stars: Some(5.08), ..Default::default() })), &w);
        state.update(Message::Details(7, "old".into(), Some(desk::details::Details { stars: [("DT".into(), 6.92)].into_iter().collect(), ..Default::default() })), &w);
        let shown = state.shown_map(&map);
        assert_eq!(shown.title.as_deref(), Some("Voiceless Fish"));
        assert_eq!((shown.stars, shown.plain_stars), (Some(6.92), Some(5.08)));
        assert_eq!(shown.ranked_status.as_deref(), Some("graveyard"));
        let mut changed = map.clone();
        changed.revision = Some("new".into());
        assert_eq!(state.shown_map(&changed).stars, None, "another revision needs its own calculation");
        changed.stars = Some(7.1);
        assert_eq!(state.shown_map(&changed).stars, Some(7.1), "preserve stars received from the server");
    }

    #[test]
    fn journal_resolves_batch_maps_and_retries_incomplete_names_without_flooding_mirrors() {
        let w = Words::new(crate::lang::Lang::En);
        let mut state = State { tab: Tab::Journal, ..Default::default() };
        state.desk.journal = (1..=8).map(|id| desk::Entry {
            id, action: "batch_exclude".into(), target: format!("0:Practice:{id}"), ..Default::default()
        }).collect();
        assert_eq!(state.wanted(), (1..=4).map(Effect::About).collect::<Vec<_>>());
        assert!(state.wanted().is_empty());
        let incomplete = desk::About { version: "Insane".into(), ..Default::default() };
        assert_eq!(state.update(Message::About(1, Some(incomplete)), &w), vec![Effect::About(5)]);
        assert!(!state.looked.contains(&1));
        assert_eq!(state.about(1).unwrap().version, "Insane", "retain useful partial metadata");
        assert_eq!(state.update(Message::About(2, None), &w), vec![Effect::About(6)], "cooling-down requests do not block later entries");
        for id in 3..=8 {
            state.update(Message::About(id, Some(desk::About { title: format!("Song {id}"), ..Default::default() })), &w);
        }
        assert!(state.about_pending.is_empty() && state.wanted().is_empty());
        for id in [1, 2] { state.missed_at.insert(id, std::time::Instant::now() - ABOUT_RETRY); }
        assert_eq!(state.wanted(), vec![Effect::About(1), Effect::About(2)]);
        state.update(Message::About(1, Some(desk::About { title: "Recovered song".into(), ..Default::default() })), &w);
        assert!(did(&w, &state, &state.desk.journal[0]).contains("Recovered song"));
        assert!(!did(&w, &state, &state.desk.journal[0]).contains("#1"));
        let whole = desk::Entry { action: "batch_delete".into(), target: "0:2026".into(), ..Default::default() };
        assert_eq!(journal_map_id(&whole), None, "a numeric batch name is not a map ID");
        let renamed = desk::Entry { action: "batch_rename".into(), target: "0:Practice:2026".into(), ..Default::default() };
        assert_eq!(journal_map_id(&renamed), None);
    }
}
