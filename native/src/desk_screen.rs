use std::collections::{HashMap, HashSet};

use iced::widget::{button, column, container, image, mouse_area, row, scrollable, stack, text, text_editor, text_input, Space};
use iced::{mouse, Alignment, Background, Border, Color, Element, Length, Padding};

use crate::desk::{self, Cell, Desk, Held, Task};
use crate::duels_screen::{bold, person, strong, words, ROW};
use crate::glyphs::{glyph, Icon};
use crate::lang::Words;
use crate::theme::workspace;
use crate::theme::{self, ACCENT, FAINT, INK, MUTED};
use crate::ui;

const OWC_CELL_WIDTH: f32 = 88.0;
const TRAINING_WAIT: std::time::Duration = std::time::Duration::from_secs(240);
const OWC_CELL_HEIGHT: f32 = 48.0;
const OWC_STAGE_WIDTH: f32 = 148.0;

const GOOD: Color = theme::GRADE_A;
const UNSURE: Color = theme::GRADE_S;
const BAD: Color = iced::color!(0xf38a8a);
const CHECKED: Color = iced::color!(0x2c3523);
const PICKED: Color = Color { a: 0.55, ..ACCENT };
pub(crate) const LIT: Color = Color::from_rgba(0.135, 0.086, 0.098, 0.92);
const STAGES: [&str; 9] = ["grand finals", "finals", "semifinals", "quarterfinals", "round of 16", "round of 32", "group stage", "qualifiers", "unknown"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Queue,
    Label,
    Owc,
    Catalog,
    Model,
    Duels,
    Journal,
    People,
}

impl Tab {
    const ALL: [Tab; 8] = [Tab::Queue, Tab::Label, Tab::Owc, Tab::Catalog, Tab::Model, Tab::Duels, Tab::Journal, Tab::People];

    fn key(self) -> &'static str {
        match self {
            Tab::Queue => "desk-tab-queue",
            Tab::Label => "desk-tab-label",
            Tab::Owc => "desk-tab-owc",
            Tab::Catalog => "desk-tab-catalog",
            Tab::Model => "desk-tab-model",
            Tab::Duels => "desk-tab-duels",
            Tab::Journal => "desk-tab-journal",
            Tab::People => "desk-tab-people",
        }
    }

    fn open(self, roles: &[String]) -> bool {
        match self {
            Tab::Queue | Tab::Label | Tab::Owc | Tab::Catalog => desk::allows(roles, desk::REVIEWER),
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
    Review { beatmap: i64, decision: &'static str, slot: Option<String>, reason: String },
    Label { cell: Cell, beatmap: Option<i64>, label: String, reason: String },
    Tournament { name: String, year: i32, pool: String, reason: String },
    Publish { version: String, reason: String },
    Rollback { reason: String },
    Finalize { duel: String, match_id: i64, game_id: i64, reason: String },
    Cancel { duel: String, reason: String },
    Grant { who: String, role: String, reason: String },
    Revoke { player: i64, role: String, reason: String },
    Discover { band: usize, reason: String },
    Train { reason: String },
    Delete { version: String, reason: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Load,
    Cover(i64),
    Hear(Option<i64>),
    Cell(String, usize),
    Post(Post),
    About(i64),
    Archive,
    Open(String),
}

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Result<Desk, String>),
    Tab(Tab),
    Filter(Filter),
    Pick(i64),
    Reason(String),
    Decide(&'static str, Option<String>),
    NotThis(bool),
    Revoke(i64, String),
    Posted(Result<(), String>),
    Cover(i64, Option<image::Handle>),
    Hear(i64),
    Heard,
    Year(i32),
    Cell(usize),
    Label(String),
    SaveCell,
    Band(String, usize),
    Held(Result<Vec<Held>, String>),
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
    Picking(bool),
    Archive,
    Archived(Vec<String>),
    Discover,
    Open(i64),
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pub desk: Desk,
    pub tab: Tab,
    pub loading: bool,
    pub posting: bool,
    pub failed: Option<String>,
    pub filter: Filter,
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
    pub recent: Vec<(String, String)>,
    pub adding: bool,
    pub name: String,
    pub year_text: String,
    pub pool: Pool,
    pub player: String,
    pub role: Option<&'static str>,
    pub abouts: HashMap<i64, desk::About>,
    pub looked: HashSet<i64>,
    pub picking: bool,
    pub training: Option<(usize, std::time::Instant)>,
    pub training_lost: bool,
    pub nothing_new: bool,
    pub moving: bool,
    pub confirming: bool,
    pub revoking: Option<(i64, String)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tournament {
    pub corpus: String,
    pub year: i32,
    pub stage: Option<String>,
}

const RESERVE: [&str; 11] = ["NM5", "HD1", "HD3", "HR1", "HR3", "DT3", "DT4", "FM1", "FM2", "FM3", "FM4"];

impl State {
    pub fn reserve(&self) -> Vec<String> {
        let mut found: Vec<String> = self.desk.cells.iter().map(|cell| cell.slot.clone())
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
        self.loading = true;
        vec![Effect::Load]
    }

    fn tasks(&self) -> Vec<&Task> {
        self.desk.tasks.iter().filter(|task| self.filter.holds(task)).collect()
    }

    fn task(&self) -> Option<&Task> {
        let shown = self.tasks();
        self.picked.and_then(|id| shown.iter().copied().find(|task| task.map.beatmap_id == id)).or_else(|| shown.first().copied())
    }

    fn years(&self) -> Vec<i32> {
        let mut years: Vec<i32> = self.desk.cells.iter().filter(|cell| cell.corpus.starts_with("owc")).map(|cell| cell.year).collect();
        years.sort_unstable();
        years.dedup();
        years
    }

    fn year_shown(&self) -> Option<i32> {
        self.year.or_else(|| self.years().last().copied())
    }

    pub fn tournaments(&self) -> Vec<Tournament> {
        let mut found: Vec<Tournament> = Vec::new();
        for cell in self.desk.cells.iter().rev() {
            let stage = (!cell.corpus.starts_with("owc")).then(|| cell.stage.clone());
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

    fn post(&mut self, post: Post) -> Vec<Effect> {
        if self.posting {
            return Vec::new();
        }
        self.posting = true;
        self.failed = None;
        vec![Effect::Post(post)]
    }

    pub fn update(&mut self, message: Message, w: &Words) -> Vec<Effect> {
        let mut effects = self.step(message, w);
        effects.extend(self.wanted());
        effects
    }

    fn wanted(&mut self) -> Vec<Effect> {
        if self.tab == Tab::Journal {
            let ids: Vec<i64> = self.desk.journal.iter().filter(|entry| entry.action.starts_with("map_")).filter_map(|entry| entry.target.parse().ok()).collect();
            return ids.into_iter().filter(|beatmap| self.looked.insert(*beatmap)).map(Effect::About).collect();
        }
        if self.tab != Tab::Label {
            return Vec::new();
        }
        let ids: Vec<i64> = if self.adding {
            read_pool(&self.pool.0.text()).into_iter().map(|(_, beatmap)| beatmap).take(40).collect()
        } else {
            self.next_map().map(|(_, map)| map.beatmap_id).into_iter().collect()
        };
        ids.into_iter().filter(|beatmap| self.looked.insert(*beatmap)).map(Effect::About).collect()
    }

    pub fn about(&self, beatmap: i64) -> Option<&desk::About> {
        self.abouts.get(&beatmap)
    }

    fn step(&mut self, message: Message, w: &Words) -> Vec<Effect> {
        match message {
            Message::About(beatmap, found) => {
                let Some(found) = found else { return Vec::new() };
                let set = found.set;
                self.abouts.insert(beatmap, found);
                self.covered([set])
            }
            Message::Picking(on) => {
                self.picking = on;
                Vec::new()
            }
            Message::Archive => vec![Effect::Archive],
            Message::Open(beatmap) => vec![Effect::Open(map_page(beatmap))],
            Message::Discover => {
                let Some((slot, band)) = self.band.clone() else { return Vec::new() };
                let reason = w.with("desk-discover-reason", &[("slot", slot), ("band", band_name(w, band))]);
                self.post(Post::Discover { band, reason })
            }
            Message::Archived(lines) => {
                if !lines.is_empty() {
                    let mut text = self.pool.0.text();
                    if !text.is_empty() && !text.ends_with('\n') {
                        text.push('\n');
                    }
                    text.push_str(&lines.join("\n"));
                    self.pool = Pool(text_editor::Content::with_text(&text));
                }
                Vec::new()
            }
            Message::Loaded(found) => {
                self.loading = false;
                match found {
                    Ok(desk) => {
                        if self.training.is_some_and(|(count, _)| desk.versions.len() > count) {
                            self.training = None;
                        }
                        self.desk = desk;
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
                self.tab = tab;
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
                let Some(task) = self.task() else { return Vec::new() };
                let post = Post::Review { beatmap: task.map.beatmap_id, decision, slot, reason: self.reason.trim().to_owned() };
                self.post(post)
            }
            Message::Posted(said) => {
                self.posting = false;
                if said.is_err() {
                    self.training = None;
                }
                match said {
                    Ok(()) => {
                        self.reason.clear();
                        self.moving = false;
                        self.confirming = false;
                        self.picked = None;
                        self.label = None;
                        self.match_id.clear();
                        self.game_id.clear();
                        self.loading = true;
                        vec![Effect::Load]
                    }
                    Err(why) if why == "NOTHING_NEW" => {
                        self.nothing_new = true;
                        Vec::new()
                    }
                    Err(why) => {
                        self.failed = Some(why);
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
            Message::Held(found) => {
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
                let reason = self.reason.trim().to_owned();
                self.training = Some((self.desk.versions.len(), std::time::Instant::now()));
                self.training_lost = false;
                self.nothing_new = false;
                self.post(Post::Train { reason })
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
                if self.training.is_some_and(|(_, since)| since.elapsed() > TRAINING_WAIT) {
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
                self.stuck = Some(duel);
                self.match_id.clear();
                self.game_id.clear();
                Vec::new()
            }
            Message::MatchId(said) => {
                self.match_id = said.chars().filter(char::is_ascii_digit).collect();
                Vec::new()
            }
            Message::GameId(said) => {
                self.game_id = said.chars().filter(char::is_ascii_digit).collect();
                Vec::new()
            }
            Message::Finalize => {
                let Some(duel) = self.stuck_shown().map(|found| found.id.clone()) else { return Vec::new() };
                let (Ok(match_id), Ok(game_id)) = (self.match_id.parse::<i64>(), self.game_id.parse::<i64>()) else { return Vec::new() };
                let reason = self.reason.trim().to_owned();
                self.post(Post::Finalize { duel, match_id, game_id, reason })
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
                self.tournament = Some(at);
                self.adding = false;
                self.picking = false;
                let sets: Vec<Option<i64>> = self.next_map().map(|(_, map)| map.beatmapset_id).into_iter().collect();
                self.covered(sets)
            }
            Message::MapLabel(label) => {
                let Some((cell, map)) = self.next_map().map(|(cell, map)| (cell.clone(), map.clone())) else { return Vec::new() };
                self.recent.push((map.title.clone().unwrap_or_else(|| map.beatmap_id.to_string()), label.clone()));
                if self.recent.len() > 4 {
                    self.recent.remove(0);
                }
                self.skipped.insert(map.beatmap_id);
                let mut effects = self.post(Post::Label { cell, beatmap: Some(map.beatmap_id), label, reason: w.t("desk-label-reason") });
                let sets: Vec<Option<i64>> = self.next_map().map(|(_, map)| map.beatmapset_id).into_iter().collect();
                effects.extend(self.covered(sets));
                effects
            }
            Message::Skip => {
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
                let Ok(year) = self.year_text.parse::<i32>() else { return Vec::new() };
                let pool = self.pool.0.text();
                if self.name.trim().is_empty() || pool.trim().is_empty() {
                    return Vec::new();
                }
                let post = Post::Tournament { name: self.name.trim().to_owned(), year, pool, reason: w.with("desk-tournament-reason", &[("name", self.name.trim().to_owned())]) };
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
    w.t(&format!("desk-skill-{}", slot.to_lowercase()))
}

fn stage_name(w: &Words, stage: &str) -> String {
    let key = format!("desk-stage-{}", stage.replace(' ', "-"));
    let said = w.t(&key);
    if said == key { stage.to_owned() } else { said }
}

fn feature(w: &Words, path: &str) -> String {
    let key = format!("desk-feature-{}", path.replace(['.', '_'], "-"));
    let said = w.t(&key);
    if said == key { path.to_owned() } else { said }
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
    button(container(tight(said, theme::BODY * k, if on { INK } else { MUTED })).center_y(workspace::CONTROL * k))
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

fn main_button_with_k<'a>(k: f32, icon: Icon, said: String, on: Option<Message>) -> Element<'a, Message> {
    let inside = row![glyph(icon, 17.0 * k, Color::WHITE), tight(said, theme::BODY * k, Color::WHITE)].spacing(8).align_y(Alignment::Center);
    let made = button(container(inside).center_x(Length::Fill).center_y((theme::CONTROL_HEIGHT + 6.0) * k)).width(Length::Fill).padding([0.0, 14.0 * k]).style(ui::button_faded(theme::primary));
    match on {
        Some(message) => made.on_press(message).into(),
        None => made.into(),
    }
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
        Some(handle) => stack![base, image(handle.clone()).width(wide).height(high).content_fit(iced::ContentFit::Cover).border_radius(radius)].into(),
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
            Tab::Queue => state.desk.tasks.len(),
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
        let made = button(container(inside).center_y(40.0 * k)).padding([0.0, 14.0 * k]).on_press(Message::Tab(tab)).style(workspace::choice(on));
        line = line.push(made);
    }
    line.wrap().vertical_spacing(6).into()
}

fn reason_colour(task: &Task) -> Color {
    match task.reason.as_str() {
        "uncertain_revealed" | "uncertain" => UNSURE,
        "unassigned" => MUTED,
        _ => BAD,
    }
}

fn queue_view<'a>(w: &Words, state: &'a State, now: i64, k: f32) -> Element<'a, Message> {
    let mut chips = row![].spacing(6);
    for filter in Filter::ALL {
        let count = state.desk.tasks.iter().filter(|task| filter.holds(task)).count();
        chips = chips.push(chip_k(format!("{} {count}", w.t(filter.key())), state.filter == filter, Message::Filter(filter), k));
    }
    let picked = state.task().map(|task| task.map.beatmap_id);
    let mut list = column![].spacing(6);
    for task in state.tasks() {
        let on = picked == Some(task.map.beatmap_id);
        let mut tags = row![].spacing(6).align_y(Alignment::Center);
        if let Some(slot) = task.slot.as_deref().filter(|_| task.kind != "unassigned") {
            tags = tags.push(tag(slot, k));
        }
        tags = tags.push(strong(w.t(&format!("desk-reason-{}", task.reason.replace('_', "-"))), 12.0 * k, reason_colour(task)));
        let inside = row![
            cover(state, task.map.beatmapset_id, 60.0 * k, 38.0 * k, 7.0),
            column![titled_map(task.map.title.as_deref(), task.map.beatmap_id, 14.0 * k), tags].spacing(3).width(Length::Fill),
            tight(ago(w, task.since.as_deref(), now), 12.0 * k, FAINT),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        let fill = if on { LIT } else { ROW };
        list = list.push(tile(fill, on.then_some((PICKED, 1.5)), inside.into(), Some(Message::Pick(task.map.beatmap_id))));
    }
    if state.tasks().is_empty() {
        list = list.push(words(w.t(if state.loading { "desk-loading" } else { "desk-queue-empty" }), 14.0, FAINT));
    }
    let aside = container(column![chips.wrap().vertical_spacing(6), scrollable(list).direction(ui::hidden_bar()).height(Length::Fill)].spacing(10)).padding(14).width(330.0 * k).height(Length::Fill).style(theme::slab);
    let detail: Element<'a, Message> = match state.task() {
        Some(task) => task_detail(w, state, task, k),
        None => panel(Space::new().into()),
    };
    row![aside, detail].spacing(14).height(Length::Fill).into()
}

fn task_detail<'a>(w: &Words, state: &'a State, task: &Task, k: f32) -> Element<'a, Message> {
    let map = &task.map;
    let card = stack![
        cover(state, map.beatmapset_id, 280.0 * k, 160.0 * k, 14.0),
        container(column![
            row![task.slot.as_deref().filter(|_| task.kind != "unassigned").map(|slot| tag(slot, k)).unwrap_or_else(|| Space::new().into()), Space::new().width(Length::Fill), stars_of(map.stars, 12.0 * k)].align_y(Alignment::Center),
            Space::new().height(Length::Fill),
            one_line(map.version.clone().unwrap_or_default(), 13.0 * k, MUTED, theme::SANS),
            titled_map(map.title.as_deref(), map.beatmap_id, 17.0 * k),
            one_line(map.artist.clone().unwrap_or_default(), 13.0 * k, MUTED, theme::SANS),
            row![crate::duels_screen::mapper_face(map.mapper.as_deref().unwrap_or_default(), 18.0 * k), words(map.mapper.clone().unwrap_or_default(), 13.0 * k, MUTED)].spacing(6).align_y(Alignment::Center),
        ].spacing(1))
        .padding(12.0 * k)
        .width(280.0 * k)
        .height(160.0 * k)
        .align_y(iced::alignment::Vertical::Bottom)
        .style(|_| container::Style { background: Some(crate::duel_view::veil_fill()), border: Border { radius: 14.0.into(), ..Border::default() }, ..container::Style::default() }),
    ];
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
            let caption = if task.reason.starts_with("uncertain") { w.with("desk-ask-uncertain", &[("slot", slot.clone())]) } else { w.with("desk-ask-feedback", &[("slot", slot.clone())]) };
            let buttons = row![
                container(main_button_k(w.with("desk-yes", &[("slot", slot.clone())]), ask(Message::Decide("APPROVE", None)), k)).width(Length::Shrink),
                container(line_button_k(w.with("desk-no", &[("slot", slot.clone())]), (!state.posting).then_some(Message::NotThis(true)), k)).width(Length::Shrink),
                container(line_button_k(w.t("desk-quarantine"), ask(Message::Decide("QUARANTINE", None)), k)).width(Length::Shrink),
            ]
            .spacing(8)
            .wrap()
            .vertical_spacing(8);
            (caption, w.with("desk-is-it", &[("skill", skill(w, &slot).to_lowercase())]), buttons.into())
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
    if let Some(length) = map.length.filter(|length| *length > 0.0) {
        let seconds = length.round() as u32;
        facts = facts.push(fact(w.t("desk-length"), format!("{}:{:02}", seconds / 60, seconds % 60), k));
    }
    let reason = column![words(w.t("desk-reason"), 13.0 * k, FAINT), field_k(w.t("desk-reason-hint"), &state.reason, Message::Reason, k)].spacing(6);
    let decision = column![words(caption, 14.0 * k, FAINT), bold(question, 26.0 * k, INK), container(answers).padding(Padding { top: 6.0 * k, bottom: 10.0 * k, ..Padding::ZERO }), reason]
        .spacing(8)
        .width(Length::Fill);
    let top = row![column![card, facts, under].spacing(8), decision].spacing(22.0 * k);
    let mut why = column![].spacing(8);
    if let Some(guess) = &map.model {
        for odds in guess.top.iter().filter(|odds| odds.slot != desk::OTHER) {
            let chosen = Some(&odds.slot) == task.slot.as_ref() || (task.slot.is_none() && Some(&odds.slot) == guess.slot.as_ref());
            why = why.push(row![container(tag(&odds.slot, k)).width(56.0 * k), bar(odds.probability, if chosen { ACCENT } else { MUTED }), container(strong(format!("{:.0}%", odds.probability * 100.0), 14.0 * k, INK)).width(44.0 * k).align_x(iced::alignment::Horizontal::Right)].spacing(10).align_y(Alignment::Center));
        }
        let mut reasons = column![].spacing(5).padding(Padding { top: 6.0 * k, ..Padding::ZERO });
        for reason in guess.reasons.iter().take(3) {
            let (sign, colour) = if reason.weight >= 0.0 { ("+", GOOD) } else { ("−", BAD) };
            let level = if reason.level > 0.5 { w.t("desk-level-above") } else if reason.level < -0.5 { w.t("desk-level-below") } else { w.t("desk-level-usual") };
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
    let info: Element<'a, Message> = if k >= 1.2 { row![model_side, players].spacing(14).into() } else { column![model_side, players].spacing(14).into() };
    panel(scrollable(column![top, info].spacing(18.0 * k)).direction(ui::hidden_bar()).into())
}

fn spare_slots<'a>(w: &Words, state: &State, decision: &'static str, ready: bool, k: f32) -> Element<'a, Message> {
    let mut chips = row![].spacing(6).align_y(Alignment::Center);
    for slot in state.reserve() {
        let made = button(container(tag(&slot, k)).center_y(32.0 * k)).padding([0.0, 6.0 * k]).style(quiet(false, 9.0));
        chips = chips.push(if ready { made.on_press(Message::Decide(decision, Some(slot))) } else { made });
    }
    column![words(w.t("desk-reserve"), 13.0 * k, FAINT), chips.wrap().vertical_spacing(6)].spacing(6).into()
}

fn fact<'a>(name: String, value: String, k: f32) -> Element<'a, Message> {
    container(column![tight(value, 15.0 * k, INK), tight(name, 11.0 * k, FAINT)].spacing(1).align_x(iced::alignment::Horizontal::Center))
        .padding([6.0 * k, 0.0])
        .center_x(Length::Fill)
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
        return row![panel(words(w.t(if state.loading { "desk-loading" } else { "desk-label-empty" }), 15.0 * k, FAINT)), container(line_button_k(w.t("desk-add-tournament"), Some(Message::Adding(true)), k)).width(260.0 * k)].spacing(14).into();
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
                words(version, 16.0 * k, MUTED),
                one_line(shown_title, 34.0 * k, INK, theme::SANS_SEMI),
                words(artist, 17.0 * k, MUTED),
                row![crate::duels_screen::mapper_face(&mapper, 24.0), words(mapper.clone(), 15.0 * k, MUTED)].spacing(8).align_y(Alignment::Center),
            ]
            .spacing(4);
            let shown = container(column![row![facts, Space::new().width(Length::Fill), listen].align_y(Alignment::Center), Space::new().height(Length::Fill), words_block])
                .padding([20, 24])
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_| container::Style { background: Some(crate::duel_view::veil_fill()), border: Border { radius: 18.0.into(), ..Border::default() }, ..container::Style::default() });
            let set = map.beatmapset_id.or_else(|| about.and_then(|found| found.set));
            let picture: Element<'a, Message> = match set.and_then(|set| state.covers.get(&set)) {
                Some(handle) => image(handle.clone()).width(Length::Fill).height(Length::Fill).content_fit(iced::ContentFit::Cover).border_radius(18.0).into(),
                None => container(Space::new()).width(Length::Fill).height(Length::Fill).style(|_| container::Style { background: Some(Background::Color(ROW)), border: Border { radius: 18.0.into(), ..Border::default() }, ..container::Style::default() }).into(),
            };
            let mut keys = row![].spacing(8);
            for slot in desk::SLOTS {
                let made = button(container(tag(slot, k)).center_x(Length::Fill).center_y(52.0 * k)).width(Length::Fill).padding(0).style(quiet(false, theme::CONTROL_RADIUS));
                keys = keys.push(if state.posting { made } else { made.on_press(Message::MapLabel(slot.to_owned())) });
            }
            let rest = row![line_button_k(w.t("desk-fits-none"), (!state.posting).then(|| Message::MapLabel(desk::OTHER.to_owned())), k), line_button_k(w.t("desk-skip"), Some(Message::Skip), k)].spacing(8);
            column![stack![picture, shown].height(Length::Fill), keys, rest].spacing(12).width(Length::Fill).into()
        }
        None => panel(words(w.t("desk-label-done"), 15.0 * k, FAINT)),
    };
    let title = mouse_area(row![container(heading_k(k, tournament_name(w, tournament))).width(Length::Fill), glyph(Icon::Chevron, 16.0 * k, MUTED)].spacing(8).align_y(Alignment::Center))
        .on_press(Message::Picking(!state.picking))
        .interaction(mouse::Interaction::Pointer);
    let mut aside = column![title].spacing(10);
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
        let mut recent = column![].spacing(2);
        for (title, label) in state.recent.iter().rev() {
            recent = recent.push(row![one_line(title.clone(), 14.0 * k, INK, theme::SANS), strong(label.clone(), 14.0 * k, INK)].spacing(10).height(32).align_y(Alignment::Center));
        }
        aside = aside.push(recent);
    }
    let aside = container(aside).padding(18).width(260.0 * k).style(theme::slab);
    row![main, aside].spacing(14).height(Length::Fill).into()
}

fn tournament_view<'a>(w: &Words, state: &'a State, k: f32) -> Element<'a, Message> {
    let found = read_pool(&state.pool.0.text());
    let lines = state.pool.0.text().lines().filter(|line| !line.trim().is_empty()).count();
    let ready = !state.name.trim().is_empty() && state.year_text.len() == 4 && !found.is_empty() && !state.posting;
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
        heading_k(k, w.t("desk-add-tournament")),
        row![labelled_k(k, w.t("desk-name"), field_kk(k, String::new(), &state.name, Message::Name)), container(labelled_k(k, w.t("desk-year"), field_kk(k, String::new(), &state.year_text, Message::YearText))).width(120.0 * k)].spacing(10),
        column![words(w.t("desk-pool"), 13.0 * k, FAINT), editor].spacing(6).height(Length::Fill),
        row![archive, Space::new().width(Length::Fill), container(main_button_kk(k, w.with("desk-add-maps", &[("count", found.len().to_string())]), ready.then_some(Message::AddTournament))).width(Length::Shrink)].align_y(Alignment::Center),
    ].spacing(14).into());
    let mut parsed = column![row![heading_k(k, w.t("desk-parsed")), Space::new().width(Length::Fill), words(w.with("desk-of", &[("count", found.len().to_string()), ("total", lines.to_string())]), 13.0 * k, FAINT)].align_y(Alignment::Center)].spacing(4);
    for (slot, beatmap) in &found {
        let about = state.about(*beatmap);
        let title = about.map(|found| found.title.clone()).filter(|title| !title.is_empty()).unwrap_or_else(|| beatmap.to_string());
        parsed = parsed.push(row![
            cover(state, about.and_then(|found| found.set), 72.0 * k, 40.0 * k, 7.0),
            tag(slot, k),
            one_line(title, 14.0 * k, INK, theme::SANS_SEMI),
            stars_of(about.and_then(|found| found.stars), 12.0 * k),
        ].spacing(10).height(46).align_y(Alignment::Center));
    }
    let aside = container(scrollable(parsed).direction(ui::hidden_bar())).padding(18).width(440.0 * k).height(Length::Fill).style(theme::slab);
    row![form, aside].spacing(14).height(Length::Fill).into()
}

pub fn read_pool(text: &str) -> Vec<(String, i64)> {
    let mut found = Vec::new();
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let (Some(slot), Some(link)) = (parts.next(), parts.next()) else { continue };
        let slot = slot.to_uppercase();
        if slot.len() > 4 || !slot.chars().next().is_some_and(|first| first.is_ascii_alphabetic()) {
            continue;
        }
        let tail = link.rsplit(['/', '#']).find(|part| !part.is_empty()).unwrap_or_default();
        if let Ok(beatmap) = tail.parse::<i64>() {
            found.push((slot, beatmap));
        }
    }
    found
}

fn owc_view<'a>(w: &Words, state: &'a State, wide: f32, k: f32) -> Element<'a, Message> {
    let years = state.years();
    let year = state.year_shown();
    let mut picker = row![].spacing(8).align_y(Alignment::Center);
    for each in &years {
        picker = picker.push(chip_k(each.to_string(), Some(*each) == year, Message::Year(*each), k));
    }
    let shown: Vec<(usize, &Cell)> = state.desk.cells.iter().enumerate().filter(|(_, cell)| cell.corpus.starts_with("owc") && Some(cell.year) == year).collect();
    let checked = shown.iter().map(|(_, cell)| cell.maps.iter().filter(|map| map.verified).count()).sum::<usize>();
    let total = shown.iter().map(|(_, cell)| cell.maps.len()).sum::<usize>();
    picker = picker.push(Space::new().width(Length::Fill)).push(words(w.with("desk-checked", &[("count", checked.to_string()), ("total", total.to_string())]), 14.0 * k, FAINT));
    let mut slots: Vec<String> = shown.iter().map(|(_, cell)| cell.slot.clone()).collect();
    slots.sort_by_key(|slot| (owc_order(slot), slot.clone()));
    slots.dedup();
    let mut stages: Vec<String> = shown.iter().map(|(_, cell)| cell.stage.clone()).collect();
    stages.sort_by_key(|stage| STAGES.iter().position(|known| known == stage).unwrap_or(STAGES.len()));
    stages.dedup();
    let mut groups = row![Space::new().width(OWC_STAGE_WIDTH * k)].spacing(4);
    let mut at = 0;
    while at < slots.len() {
        let order = owc_order(&slots[at]).min(4);
        let count = slots[at..].iter().take_while(|slot| owc_order(slot).min(4) == order).count();
        let name = match order {
            0 => "NoMod".to_owned(),
            1 => "Hidden".to_owned(),
            2 => "HardRock".to_owned(),
            3 => "DoubleTime".to_owned(),
            _ => w.t("desk-group-other"),
        };
        let line = container(Space::new()).width(Length::Fill).height(1).style(|_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.1))), ..container::Style::default() });
        groups = groups.push(container(column![container(words(name, 12.0 * k, FAINT)).center_x(Length::Fill), line].spacing(6)).padding([0, 4]).width(count as f32 * ((OWC_CELL_WIDTH * k) + 4.0) - 4.0));
        at += count;
    }
    let mut head = row![Space::new().width(OWC_STAGE_WIDTH * k)].spacing(4);
    for slot in &slots {
        head = head.push(container(strong(slot.clone(), 12.0 * k, pastel(slot))).center_x(OWC_CELL_WIDTH * k));
    }
    let mut grid = column![groups, head].spacing(4);
    for stage in &stages {
        let mut line = row![container(words(stage_name(w, stage), 13.0 * k, MUTED)).width(OWC_STAGE_WIDTH * k).center_y(OWC_CELL_HEIGHT * k)].spacing(4);
        for slot in &slots {
            let found = shown.iter().find(|(_, cell)| cell.stage == *stage && cell.slot == *slot);
            line = line.push(match found {
                Some((at, cell)) => owc_cell(*at, cell, state.cell == Some(*at), k),
                None => Space::new().width(OWC_CELL_WIDTH * k).height(OWC_CELL_HEIGHT * k).into(),
            });
        }
        grid = grid.push(line);
    }
    let legend = row![
        swatch(iced::color!(0x3e5130), None, w.t("desk-legend-checked"), k),
        swatch(ROW, None, w.t("desk-legend-number"), k),
        swatch(Color::TRANSPARENT, Some(UNSURE), w.t("desk-legend-moved"), k),
    ]
    .spacing(18);
    let table = panel(column![picker, scrollable(grid).direction(scrollable::Direction::Both { vertical: scrollable::Scrollbar::hidden(), horizontal: scrollable::Scrollbar::default() }).height(Length::Fill), legend].spacing(14).into());
    if wide < workspace::SPLIT_MIN {
        column![container(table).height(Length::FillPortion(3)), scrollable(owc_aside(w, state, true, k)).height(Length::FillPortion(2))].spacing(workspace::GAP).height(Length::Fill).into()
    } else {
        row![table, owc_aside(w, state, false, k)].spacing(workspace::GAP).height(Length::Fill).into()
    }
}

fn pastel(slot: &str) -> Color {
    match slot.get(..2).unwrap_or_default() {
        "NM" => iced::color!(0x8ccaee),
        "HD" => iced::color!(0xf0b56e),
        "HR" => iced::color!(0xf38a8a),
        "DT" => iced::color!(0xc3a3f8),
        "FM" => iced::color!(0x92d9a5),
        _ => iced::color!(0xece2de),
    }
}

fn owc_order(slot: &str) -> usize {
    ["NM", "HD", "HR", "DT", "FM", "TB"].iter().position(|prefix| slot.starts_with(prefix)).unwrap_or(6)
}

fn owc_cell<'a>(at: usize, cell: &Cell, chosen: bool, k: f32) -> Element<'a, Message> {
    let label = cell.shown();
    let checked = cell.checked();
    let moved = label != cell.slot && label != desk::EXCLUDED && label != desk::OTHER;
    let shown = if label == desk::EXCLUDED || label == desk::OTHER { "—".to_owned() } else { label.clone() };
    let fill = if checked { CHECKED } else { ROW };
    let frame = if chosen { Some((INK, 2.0)) } else if moved { Some((UNSURE, 1.5)) } else { None };
    let spare = label != desk::EXCLUDED && label != desk::OTHER && !desk::SLOTS.contains(&label.as_str());
    let made = container(strong(shown, 14.0 * k, if spare && checked { MUTED } else if spare { FAINT } else if checked { INK } else { MUTED })).center_x(OWC_CELL_WIDTH * k).center_y(OWC_CELL_HEIGHT * k).style(move |_| container::Style {
        background: Some(Background::Color(fill)),
        border: frame.map_or(Border { radius: 7.0.into(), ..Border::default() }, |(colour, width)| Border { color: colour, width, radius: 7.0.into() }),
        ..container::Style::default()
    });
    mouse_area(made).on_press(Message::Cell(at)).interaction(mouse::Interaction::Pointer).into()
}

fn swatch<'a>(fill: Color, frame: Option<Color>, said: String, k: f32) -> Element<'a, Message> {
    let mark = container(Space::new()).width(14.0 * k).height(14.0 * k).style(move |_| container::Style {
        background: Some(Background::Color(fill)),
        border: Border { color: frame.unwrap_or(Color::TRANSPARENT), width: if frame.is_some() { 1.5 } else { 0.0 }, radius: 4.0.into() },
        ..container::Style::default()
    });
    row![mark, words(said, 13.0 * k, FAINT)].spacing(6).align_y(Alignment::Center).into()
}

fn family(slot: &str) -> Vec<&'static str> {
    let prefix = &slot[..slot.len().min(2)];
    let found: Vec<&'static str> = desk::SLOTS.into_iter().filter(|known| known.starts_with(prefix)).collect();
    if found.is_empty() || prefix == "FM" { desk::SLOTS.to_vec() } else { found }
}

fn owc_aside<'a>(w: &Words, state: &'a State, stacked: bool, k: f32) -> Element<'a, Message> {
    let Some(cell) = state.cell.and_then(|at| state.desk.cells.get(at)) else {
        return container(words(w.t("desk-owc-pick"), 14.0 * k, FAINT)).padding(18).width(if stacked { Length::Fill } else { Length::Fixed(320.0 * k) }).height(if stacked { Length::Shrink } else { Length::Fill }).style(workspace::surface).into();
    };
    let current = state.label.clone().unwrap_or_else(|| cell.shown());
    let mut choices = row![].spacing(6).align_y(Alignment::Center);
    for slot in family(&cell.slot) {
        let on = current == slot;
        choices = choices.push(chip_with(tag(slot, k), skill(w, slot), on, Message::Label(slot.to_owned()), k));
    }
    let none = chip_with(Space::new().width(0.0 * k).into(), w.t("desk-no-slot"), current == desk::EXCLUDED, Message::Label(desk::EXCLUDED.to_owned()), k);
    let prefix = &cell.slot[..cell.slot.len().min(2)];
    let mut spare: Vec<String> = state.desk.cells.iter().map(|other| other.slot.clone()).chain([cell.slot.clone()])
        .filter(|slot| slot.starts_with(prefix) && slot != "TB" && !desk::SLOTS.contains(&slot.as_str()))
        .collect();
    spare.sort();
    spare.dedup();
    let reserve: Element<'a, Message> = if spare.is_empty() {
        Space::new().into()
    } else {
        let mut chips = row![].spacing(6).align_y(Alignment::Center);
        for slot in spare {
            let on = current == slot;
            chips = chips.push(button(container(tag(&slot, k)).center_y(38.0 * k)).padding([0, 10]).style(quiet(on, 9.0)).on_press(Message::Label(slot)));
        }
        column![words(w.t("desk-reserve"), 13.0 * k, FAINT), chips.wrap().vertical_spacing(6)].spacing(8).into()
    };
    let mut maps = column![heading_k(k, w.t("desk-cell-maps"))].spacing(8);
    for map in &cell.maps {
        maps = maps.push(row![cover(state, map.beatmapset_id, 66.0 * k, 40.0 * k, 7.0), container(titled_map(map.title.as_deref(), map.beatmap_id, 14.0 * k)).width(Length::Fill), if map.label == desk::EXCLUDED || map.label == desk::OTHER { words("—".to_owned(), 13.0 * k, FAINT) } else { tag(&map.label, k) }].spacing(10).align_y(Alignment::Center));
    }
    let ready = !state.posting;
    container(column![
        column![words(w.with("desk-owc-cell", &[("year", cell.year.to_string()), ("stage", stage_name(w, &cell.stage).to_lowercase())]), 14.0 * k, FAINT), bold(w.with("desk-owc-question", &[("slot", cell.slot.clone())]), 22.0 * k, INK)].spacing(4),
        choices.push(none).wrap().vertical_spacing(6),
        reserve,
        maps,
        Space::new().height(if stacked { Length::Shrink } else { Length::Fill }),
        labelled_k(k, w.t("desk-reason"), field_kk(k, w.t("desk-reason-hint"), &state.reason, Message::Reason)),
        main_button_kk(k, w.t("desk-save"), ready.then_some(Message::SaveCell)),
    ].spacing(12))
    .padding(18)
    .width(if stacked { Length::Fill } else { Length::Fixed(320.0 * k) })
    .height(if stacked { Length::Shrink } else { Length::Fill })
    .style(workspace::surface)
    .into()
}

fn chip_with<'a>(tag: Element<'a, Message>, said: String, on: bool, press: Message, k: f32) -> Element<'a, Message> {
    button(container(row![tag, strong(said, 13.0 * k, INK)].spacing(6).align_y(Alignment::Center)).center_y(38.0 * k))
        .padding([0.0, 10.0 * k])
        .style(quiet(on, 9.0))
        .on_press(press)
        .into()
}

fn tone(count: u32) -> (Color, Color) {
    match count {
        0 => (iced::color!(0x442224), BAD),
        1..5 => (iced::color!(0x3c2e19), UNSURE),
        _ => (iced::color!(0x2e3925), iced::color!(0xb6e59a)),
    }
}

fn band_name(w: &Words, band: usize) -> String {
    let edge = |value: f32| w.decimal(f64::from(value), 1);
    match desk::BANDS.get(band + 1) {
        Some(next) => format!("{}–{}", edge(desk::BANDS[band]), edge(*next)),
        None => format!("{}+", edge(desk::BANDS[band])),
    }
}

fn catalog_view<'a>(w: &Words, state: &'a State, wide: f32, k: f32) -> Element<'a, Message> {
    let Some(coverage) = &state.desk.coverage else { return panel(words(w.t("desk-loading"), 15.0 * k, FAINT)) };
    let stacked = wide < workspace::SPLIT_MIN;
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
    let mut head = row![Space::new().width(150.0 * k)].spacing(5);
    for band in 0..desk::BANDS.len() {
        head = head.push(container(words(band_name(w, band), 13.0 * k, FAINT)).center_x(72.0 * k));
    }
    let mut grid = column![head].spacing(5);
    for slot in desk::SLOTS {
        let counts = coverage.grid.get(slot).cloned().unwrap_or_default();
        let mut line = row![row![tag(slot, k), words(skill(w, slot), 13.0 * k, MUTED)].spacing(8).width(150.0 * k).align_y(Alignment::Center)].spacing(5).align_y(Alignment::Center);
        for band in 0..desk::BANDS.len() {
            let count = counts.get(band).copied().unwrap_or_default();
            let (fill, ink) = tone(count);
            let chosen = state.band.as_ref().is_some_and(|(held, at)| held == slot && *at == band);
            let made = container(bold(count.to_string(), 15.0 * k, ink)).center_x(72.0 * k).center_y(48.0 * k).style(move |_| container::Style {
                background: Some(Background::Color(fill)),
                border: if chosen { Border { color: INK, width: 2.0, radius: 8.0.into() } } else { Border { radius: 8.0.into(), ..Border::default() } },
                ..container::Style::default()
            });
            line = line.push(mouse_area(made).on_press(Message::Band(slot.to_owned(), band)).interaction(mouse::Interaction::Pointer));
        }
        grid = grid.push(line);
    }
    let legend = row![
        swatch(tone(5).0, None, w.t("desk-legend-enough"), k),
        swatch(tone(2).0, None, w.t("desk-legend-few"), k),
        swatch(tone(0).0, None, w.t("desk-legend-none"), k),
    ]
    .spacing(18);
    let table = panel(column![row![container(heading_k(k, w.t("desk-catalog-title"))).width(Length::Fill), legend].align_y(Alignment::Center), scrollable(grid).direction(scrollable::Direction::Both { vertical: scrollable::Scrollbar::default(), horizontal: scrollable::Scrollbar::default() }).height(Length::Fill)].spacing(12).into());
    let aside: Element<'a, Message> = match &state.band {
        Some((slot, band)) => {
            let count = coverage.grid.get(slot).and_then(|counts| counts.get(*band)).copied().unwrap_or_default();
            let (_, ink) = tone(count);
            let mut list = column![].spacing(10);
            for map in &state.held {
                let colour = if map.level == "human" { GOOD } else if map.level == "uncertain" { UNSURE } else { MUTED };
                list = list.push(column![
                    row![cover(state, map.beatmapset_id, 72.0 * k, 44.0 * k, 7.0),
                        column![titled_map(map.title.as_deref(), map.beatmap_id, 14.0 * k), one_line(map.version.clone().unwrap_or_default(), 12.0 * k, MUTED, theme::SANS), strong(w.t(&format!("desk-level-{}", map.level)), 12.0 * k, colour)].spacing(2).width(Length::Fill)
                    ].spacing(10).align_y(Alignment::Center),
                    row![stars_of(Some(map.stars), 12.0 * k), Space::new().width(Length::Fill),
                        container(listen_button(w, state, map.beatmap_id, 32.0 * k)).width(130.0 * k),
                        link_button(map.beatmap_id, 32.0 * k)
                    ].spacing(8).align_y(Alignment::Center)
                ].spacing(8));
            }
            container(column![
                tag(slot, k),
                heading_k(k, format!("{}, {}", skill(w, slot), band_name(w, *band))),
                strong(w.with("desk-band-count", &[("count", count.to_string())]), 14.0 * k, ink),
                scrollable(list).direction(ui::hidden_bar()).height(Length::Fill),
                main_button_with_k(k, Icon::Download, w.t("desk-discover"), (!state.posting).then_some(Message::Discover)),
                line_button_k(w.t("desk-label-waiting"), Some(Message::Tab(Tab::Queue)), k),
            ].spacing(10))
            .padding(18)
            .width(if stacked { Length::Fill } else { Length::Fixed(320.0 * k) })
            .height(Length::Fill)
            .style(workspace::surface)
            .into()
        }
        None => container(words(w.t("desk-catalog-pick"), 14.0 * k, FAINT)).padding(18).width(if stacked { Length::Fill } else { Length::Fixed(320.0 * k) }).height(Length::Fill).style(workspace::surface).into(),
    };
    let content: Element<'a, Message> = if stacked {
        column![container(table).height(Length::FillPortion(3)), container(aside).height(Length::FillPortion(2))].spacing(workspace::GAP).height(Length::Fill).into()
    } else {
        row![table, aside].spacing(workspace::GAP).height(Length::Fill).into()
    };
    column![tiles, content].spacing(12).height(Length::Fill).into()
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
    } else if state.training_lost {
        general = general.push(words(w.t("desk-training-lost"), 13.0 * k, BAD));
    } else if state.nothing_new {
        general = general.push(words(w.t("desk-nothing-new"), 13.0 * k, FAINT));
    }
    let aside = container(column![scrollable(list).direction(ui::hidden_bar()).height(Length::Fill), general].spacing(12)).padding(18).width(400.0 * k).height(Length::Fill).style(theme::slab);
    let Some(version) = state.desk.versions.iter().find(|version| Some(&version.version) == chosen.as_ref()) else {
        return row![aside, panel(Space::new().into())].spacing(14).height(Length::Fill).into();
    };
    let report = &version.report;
    let live = version.state == "published";
    let mut buttons = row![].spacing(8);
    if !live {
        buttons = buttons.push(container(line_button_k(w.t(if state.confirming { "desk-delete-sure" } else { "desk-delete" }), may.then_some(Message::Delete), k)).width(170.0 * k));
        buttons = buttons.push(container(main_button_k(w.t("desk-publish"), may.then_some(Message::Publish), k)).width(170.0 * k));
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
    let cell = |said: String, colour: Color| -> Element<'a, Message> { container(tight(said, 16.0 * k, colour)).width(96.0 * k).align_x(iced::alignment::Horizontal::Right).into() };
    let mut slots = column![row![Space::new().width(Length::Fill), container(tight(w.t("desk-precision"), 13.0 * k, FAINT)).width(96.0 * k).align_x(iced::alignment::Horizontal::Right), container(tight(w.t("desk-recall"), 13.0 * k, FAINT)).width(96.0 * k).align_x(iced::alignment::Horizontal::Right)]].spacing(4);
    for slot in desk::SLOTS {
        let held = report.pointer(&format!("/test/per_slot/{slot}"));
        let (precision, pc) = percent(held.and_then(|held| held.get("precision")).and_then(serde_json::Value::as_f64));
        let (recall, rc) = percent(held.and_then(|held| held.get("recall")).and_then(serde_json::Value::as_f64));
        slots = slots.push(row![row![tag(slot, k), tight(skill(w, slot), 14.0 * k, MUTED)].spacing(10).align_y(Alignment::Center).width(Length::Fill), cell(precision, pc), cell(recall, rc)].height(36.0 * k).align_y(Alignment::Center));
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
    let body = row![
        column![strong(w.t("desk-slots"), 17.0 * k, INK), slots].spacing(10).width(Length::FillPortion(3)),
        column![strong(w.t("desk-by-year"), 17.0 * k, INK), years].spacing(10).width(Length::FillPortion(2)),
    ]
    .spacing(36.0 * k);
    let reason = row![words(w.t("desk-reason"), 13.0 * k, FAINT), field_k(w.t("desk-reason-hint"), &state.reason, Message::Reason, k)].spacing(12).align_y(Alignment::Center);
    let page = column![header, figures, scrollable(body).direction(ui::hidden_bar()).height(Length::Fill), reason].spacing(22.0 * k);
    row![aside, panel(page.into())].spacing(14).height(Length::Fill).into()
}

fn duels_view<'a>(w: &Words, state: &'a State, now: i64, k: f32) -> Element<'a, Message> {
    let shown = state.stuck_shown().map(|found| found.id.clone());
    let mut list = column![heading_k(k, w.t("desk-stuck"))].spacing(8);
    for duel in &state.desk.stuck {
        let on = Some(&duel.id) == shown.as_ref();
        let names: Vec<String> = duel.players.iter().map(|seat| seat.name.clone().unwrap_or_default()).collect();
        let a = names.first().cloned().unwrap_or_default();
        let b = names.get(1).cloned().unwrap_or_default();
        let faces = stack![row![person(&a, 32.0 * k)], row![Space::new().width(24.0 * k), person(&b, 32.0 * k)]].width(56.0 * k).height(32);
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
    let aside = container(scrollable(list).direction(ui::hidden_bar())).padding(18).width(400.0 * k).height(Length::Fill).style(theme::slab);
    let Some(duel) = state.stuck_shown() else { return row![aside, panel(Space::new().into())].spacing(14).height(Length::Fill).into() };
    let a = duel.players.first().and_then(|seat| seat.name.clone()).unwrap_or_default();
    let b = duel.players.get(1).and_then(|seat| seat.name.clone()).unwrap_or_default();
    let head = container(row![
        container(row![person(&a, 42.0 * k), strong(a.clone(), 18.0 * k, crate::duels_screen::SIDE_A)].spacing(14).align_y(Alignment::Center)).width(Length::Fill),
        column![bold(format!("{} : {}", duel.wins.a, duel.wins.b), 34.0 * k, INK), words(w.with("desk-round", &[("round", (duel.wins.a + duel.wins.b + 1).to_string())]), 13.0 * k, FAINT)].spacing(2).align_x(Alignment::Center),
        container(row![strong(b.clone(), 18.0 * k, crate::duels_screen::SIDE_B), person(&b, 42.0 * k)].spacing(14).align_y(Alignment::Center)).width(Length::Fill).align_x(iced::alignment::Horizontal::Right),
    ].align_y(Alignment::Center))
    .padding([12, 20])
    .width(Length::Fill)
    .style(|_| container::Style { background: Some(Background::Color(ROW)), border: Border { radius: 16.0.into(), ..Border::default() }, ..container::Style::default() });
    let warning = container(row![glyph(Icon::Clock, 20.0 * k, UNSURE), words(w.with(&format!("desk-stuck-{}-note", duel.reason.replace('_', "-")), &[("round", (duel.wins.a + duel.wins.b + 1).to_string()), ("waited", ago(w, duel.since.as_deref(), now))]), 15.0 * k, MUTED)].spacing(12).align_y(Alignment::Center))
        .padding([12, 16])
        .width(Length::Fill)
        .style(|_| container::Style { background: Some(Background::Color(ROW)), border: Border { radius: 12.0.into(), ..Border::default() }, ..container::Style::default() });
    let ready = !state.posting;
    let numbers = !state.match_id.is_empty() && !state.game_id.is_empty();
    let detail = panel(column![
        head,
        warning,
        row![labelled_k(k, w.t("desk-match"), field_kk(k, String::new(), &state.match_id, Message::MatchId)), labelled_k(k, w.t("desk-game"), field_kk(k, String::new(), &state.game_id, Message::GameId))].spacing(10),
        labelled_k(k, w.t("desk-reason"), field_kk(k, w.t("desk-reason-hint"), &state.reason, Message::Reason)),
        Space::new().height(Length::Fill),
        row![container(main_button_kk(k, w.t("desk-finalize"), (ready && numbers).then_some(Message::Finalize))).width(Length::Shrink), container(line_button_k(w.t(if state.confirming { "desk-cancel-sure" } else { "desk-cancel-duel" }), ready.then_some(Message::CancelDuel), k)).width(Length::Shrink)].spacing(10),
    ].spacing(16).into());
    row![aside, detail].spacing(14).height(Length::Fill).into()
}

fn role_name(w: &Words, role: &str) -> String {
    w.t(&format!("desk-role-{}", role.to_lowercase().replace('_', "-")))
}

fn people_list<'a>(w: &Words, state: &'a State, owner: bool, k: f32) -> Element<'a, Message> {
    let mut list = column![].spacing(8.0 * k);
    for held in &state.desk.people {
        let name = held.name.clone().unwrap_or_else(|| held.player.to_string());
        let mut line = row![person(&name, 38.0 * k), container(strong(name.clone(), 17.0 * k, INK)).width(Length::Fill)].spacing(12.0 * k).align_y(Alignment::Center);
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
    if matches!(entry.action.as_str(), "grant" | "revoke") {
        let (player, role) = entry.target.split_once(':').unwrap_or((entry.target.as_str(), ""));
        let name = player.parse::<i64>().ok().and_then(|player| state.desk.people.iter().find(|held| held.player == player)).and_then(|held| held.name.clone()).unwrap_or_else(|| player.to_owned());
        return w.with(&key, &[("target", format!("{name}, {}", role_name(w, role).to_lowercase()))]);
    }
    let target = if matches!(entry.action.as_str(), "publish_model" | "delete_model") { version_name(w, &entry.target) } else { entry.target.clone() };
    w.with(&key, &[("target", target)])
}

fn journal_view<'a>(w: &Words, state: &'a State, now: i64, k: f32) -> Element<'a, Message> {
    let mut log = column![container(heading_k(k, w.t("desk-journal"))).padding(Padding { bottom: 8.0, ..Padding::ZERO })].spacing(0);
    for entry in &state.desk.journal {
        let name = entry.name.clone().unwrap_or_else(|| entry.actor.to_string());
        let what = did(w, state, entry);
        let line = row![
            row![person(&name, 28.0 * k), strong(name.clone(), 14.0 * k, INK)].spacing(8).width(150.0 * k).align_y(Alignment::Center),
            container(words(role_name(w, &entry.role), 13.0 * k, FAINT)).width(110.0 * k),
            column![strong(what, 14.0 * k, INK), words(entry.reason.clone(), 13.0 * k, FAINT)].spacing(2).width(Length::Fill),
            container(words(ago(w, entry.at.as_deref(), now), 13.0 * k, FAINT)).width(80.0 * k).align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        log = log.push(container(Space::new()).width(Length::Fill).height(1).style(|_| container::Style { background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.06))), ..container::Style::default() }));
        log = log.push(container(line).padding([10, 0]));
    }
    if state.desk.journal.is_empty() {
        log = log.push(words(w.t(if state.loading { "desk-loading" } else { "desk-journal-empty" }), 14.0 * k, FAINT));
    }
    let aside = container(column![heading_k(k, w.t("desk-people")), people_list(w, state, false, k), Space::new().height(Length::Fill), main_button_kk(k, w.t("desk-grant"), Some(Message::Tab(Tab::People)))].spacing(8)).padding(18).width(360.0 * k).height(Length::Fill).style(theme::slab);
    row![panel(scrollable(log).direction(ui::hidden_bar()).into()), aside].spacing(14).height(Length::Fill).into()
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

pub fn view<'a>(state: &'a State, roles: &[String], w: &Words, now: i64, wide: f32) -> Element<'a, Message> {
    let k = ((wide - 80.0) / 1050.0).clamp(1.0, 1.5);
    let body: Element<'a, Message> = match state.tab {
        Tab::Queue => queue_view(w, state, now, k),
        Tab::Label => label_view(w, state, k),
        Tab::Owc => owc_view(w, state, wide, k),
        Tab::Catalog => catalog_view(w, state, wide, k),
        Tab::Model => model_view(w, state, roles, now, k),
        Tab::Duels => duels_view(w, state, now, k),
        Tab::Journal => journal_view(w, state, now, k),
        Tab::People => people_view(w, state, roles, k),
    };
    let mut page = column![tabs(w, state, roles, k)].spacing(14).width(Length::Fill).height(Length::Fill);
    if let Some(why) = &state.failed {
        page = page.push(strong(w.with("desk-failed", &[("why", why.clone())]), 14.0, BAD));
    }
    page = page.push(body);
    container(page).padding(Padding { top: 8.0, right: 40.0, bottom: 24.0, left: 40.0 }).width(Length::Fill).height(Length::Fill).into()
}

#[cfg(test)]
mod tests {
    use super::*;

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
        state.desk.tasks = vec![Task { kind: "review".into(), reason: "uncertain_revealed".into(), slot: Some("NM3".into()), map: desk::Map { beatmap_id: 7, ..desk::Map::default() }, ..Task::default() }];
        assert!(state.update(Message::NotThis(true), &w).is_empty() && state.moving);
        let effects = state.update(Message::Decide("MOVE", Some("NM2".into())), &w);
        assert_eq!(effects, [Effect::Post(Post::Review { beatmap: 7, decision: "MOVE", slot: Some("NM2".into()), reason: String::new() })]);
        assert!(state.update(Message::Decide("REJECT", None), &w).is_empty());
        assert_eq!(state.update(Message::Posted(Ok(())), &w), [Effect::Load]);
        assert!(!state.posting && !state.moving);
        let _ = state.update(Message::Reason("короткие группы".into()), &w);
        let effects = state.update(Message::Decide("APPROVE", None), &w);
        assert_eq!(effects, [Effect::Post(Post::Review { beatmap: 7, decision: "APPROVE", slot: None, reason: "короткие группы".into() })]);
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
        let _ = state.update(Message::Train, &w);
        assert!(state.training.is_some());
        let _ = state.update(Message::Posted(Err("NOTHING_NEW".into())), &w);
        assert!(state.training.is_none() && state.nothing_new && state.failed.is_none());
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
        state.training = Some((0, std::time::Instant::now()));
        assert_eq!(state.update(Message::Again, &w), [Effect::Load]);
        state.loading = false;
        state.training = std::time::Instant::now().checked_sub(TRAINING_WAIT * 2).map(|since| (0, since));
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
        assert_eq!(state.next_map().map(|(_, map)| map.beatmap_id), Some(3));
        let _ = state.update(Message::Posted(Ok(())), &w);
        let _ = state.update(Message::Skip, &w);
        assert!(state.next_map().is_none());
    }
}
