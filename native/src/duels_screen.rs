use iced::widget::{column, container, row, text, Space};
use iced::{color, Alignment, Color, Element, Length, Padding};

use crate::duels::{Duel, Live, Me, Running};
use crate::glyphs::{glyph, Icon};
use crate::lang::Words;
use crate::theme::{self, FAINT, INK, MUTED};
use crate::ui;

pub const SIDE_A: Color = color!(0xf05a5a);
pub const SIDE_B: Color = color!(0x5b9bf0);
pub(crate) const WON: Color = theme::GRADE_A;
pub(crate) const LOST: Color = color!(0xf06868);
pub(crate) const CLOCK: Color = theme::GRADE_S;
const LIVE_WIDTH: f32 = 420.0;

#[derive(Debug, Clone, Default)]
pub struct State {
    pub me: Option<Me>,
    pub live: Live,
    pub mine: Vec<Duel>,
    pub player: Option<i64>,
    pub searching: Option<i64>,
    pub loading: bool,
    pub acting: bool,
    pub failed: Option<String>,
    pub roles_failed: Option<String>,
    pub duel: Option<Duel>,
    pub reading: bool,
    pub covers: std::collections::HashMap<i64, iced::widget::image::Handle>,
    pub asked: std::collections::HashSet<i64>,
    pub acked: Option<String>,
    pub linked: bool,
    pub hearing: Option<i64>,
    pub rated: std::collections::HashMap<String, (Option<String>, Option<String>)>,
    pub closed: Option<String>,
    pub fetching: bool,
    pub watching: Option<Running>,
    pub broadcast: crate::duels::Broadcast,
    pub rounds: std::collections::HashMap<i64, u32>,
    pub heard_states: std::collections::HashMap<i64, serde_json::Value>,
    pub roles: Vec<String>,
    pub played_ms: Option<i64>,
    pub hovered: Option<(i64, std::time::Instant)>,
    pub heard_at: Option<std::time::Instant>,
    pub rested: Option<i64>,
    pub rested_at: Option<std::time::Instant>,
    pub tempos: std::collections::HashMap<i64, f32>,
    pub timed: std::collections::HashSet<i64>,
    pub moments: Moments,
}

pub const DWELL: std::time::Duration = std::time::Duration::from_millis(500);
pub const MEET: f32 = 4.7;
pub const REVEAL: f32 = 1.9;
pub const TURN: f32 = 1.0;
pub const ROUND: f32 = 2.4;
pub const RESULT: f32 = 3.3;
pub const HOLD: f32 = 2.5;
pub const LET_GO: f32 = 0.55;

#[derive(Debug, Clone, Default)]
pub struct Moments {
    pub duel: Option<String>,
    pub phase: Option<Phase>,
    pub from: Option<Phase>,
    pub phase_at: Option<std::time::Instant>,
    pub met_at: Option<std::time::Instant>,
    pub turns: usize,
    pub turn_at: Option<std::time::Instant>,
    pub hold: Option<(String, std::time::Instant)>,
    pub let_go: Option<(String, f32, std::time::Instant)>,
    pub sent: Option<String>,
}

pub fn since(at: Option<std::time::Instant>) -> f32 {
    at.map_or(f32::MAX, |at| at.elapsed().as_secs_f32())
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub me: Me,
    pub live: Live,
    pub mine: Vec<Duel>,
    pub notices: Vec<crate::duels::Notice>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Result<Snapshot, String>),
    Roles(Result<Vec<String>, String>),
    Search,
    Cancel,
    Queued(bool, Result<(), String>),
    Answer(String, u32, bool),
    Answered(Result<Duel, String>),
    Again,
    Watch(String),
    Read(Result<Duel, String>),
    Heard(crate::duels::Heard),
    Act(&'static str, serde_json::Value),
    Acted(Result<Duel, String>),
    Cover(i64, Option<iced::widget::image::Handle>),
    Hover(Option<i64>),
    Copy(String),
    Rate(String, Option<String>, Option<String>),
    Close,
    Download(Vec<i64>),
    Downloaded(Result<usize, String>),
    Open,
    Stay,
    Leave,
    Challenge(i64),
    Unwatch,
    Frame,
    Profile(i64),
    Tempo(i64, Option<f32>),
    PreviewEnded,
    Grip(String),
    Release,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Load,
    Queue(bool),
    Answer(String, u32, bool),
    Read(String),
    Act(String, &'static str, serde_json::Value, Option<u32>),
    Cover(i64),
    Hear(Option<i64>),
    Copy(String),
    Link(bool),
    Playing(Option<(String, u32)>),
    Download(Vec<i64>),
    Invite(i64),
    Tell(Vec<(&'static str, String)>, Vec<i64>),
    Order(crate::duels::Order),
    Tempo(i64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Room,
    Reveal,
    Draft,
    Ready,
    Playing,
    Result,
}

pub fn phase_of(state: &str) -> Phase {
    match state {
        "PREPARING" => Phase::Reveal,
        "DRAFTING" | "SELECTING" => Phase::Draft,
        "ROUND_SETUP" | "WAITING_READY" | "STARTING" => Phase::Ready,
        "PLAYING" | "VERIFYING" | "ABORTING" | "PAUSED" | "RECOVERING" | "FINALIZING" => Phase::Playing,
        "COMPLETED" | "CANCELLED" | "DECLINED" | "EXPIRED" => Phase::Result,
        _ => Phase::Room,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Idle,
    Searching,
    Offer,
    Waiting,
    Challenged,
    Challenging,
    Busy,
}

impl State {
    pub fn open(&mut self, player: Option<i64>) -> Vec<Effect> {
        self.player = player;
        if self.watching.is_some() {
            return self.update(Message::Unwatch, 0);
        }
        if self.duel.as_ref().is_some_and(|duel| phase_of(&duel.state) == Phase::Result) {
            return self.update(Message::Close, 0);
        }
        if self.loading {
            return Vec::new();
        }
        self.loading = true;
        vec![Effect::Load]
    }

    pub fn update(&mut self, message: Message, now: i64) -> Vec<Effect> {
        let effects = self.step(message, now);
        self.observe();
        effects
    }

    fn observe(&mut self) {
        let instant = std::time::Instant::now();
        let Some(duel) = self.shown() else {
            self.moments = Moments::default();
            return;
        };
        let phase = phase_of(&duel.state);
        let turns = duel.draft.len();
        if self.moments.duel.as_deref() != Some(duel.id.as_str()) {
            self.moments = Moments {
                duel: Some(duel.id.clone()),
                phase: Some(phase),
                phase_at: Some(instant),
                met_at: (phase == Phase::Room).then_some(instant),
                turns,
                ..Moments::default()
            };
            return;
        }
        if self.moments.phase != Some(phase) {
            self.moments.from = self.moments.phase;
            self.moments.phase = Some(phase);
            self.moments.phase_at = Some(instant);
        }
        if turns > self.moments.turns {
            self.moments.turns = turns;
            self.moments.turn_at = Some(instant);
            self.moments.sent = None;
        }
    }

    pub fn animating(&self) -> bool {
        let moments = &self.moments;
        let phase_time = since(moments.phase_at);
        since(moments.met_at) < MEET
            || (moments.phase == Some(Phase::Reveal) && phase_time < REVEAL)
            || (moments.phase == Some(Phase::Ready) && moments.from == Some(Phase::Draft) && phase_time < ROUND)
            || (moments.phase == Some(Phase::Result) && moments.from.is_some_and(|from| from != Phase::Result) && phase_time < RESULT)
            || since(moments.turn_at) < TURN
            || moments.hold.is_some()
            || moments.let_go.as_ref().is_some_and(|(_, _, at)| at.elapsed().as_secs_f32() < LET_GO)
    }

    fn let_go(&mut self) {
        if let Some((slot, at)) = self.moments.hold.take() {
            let share = (at.elapsed().as_secs_f32() / HOLD).min(1.0);
            self.moments.let_go = Some((slot, share, std::time::Instant::now()));
        }
    }

    fn step(&mut self, message: Message, now: i64) -> Vec<Effect> {
        match message {
            Message::Roles(Ok(roles)) => {
                self.roles = roles;
                self.roles_failed = None;
                Vec::new()
            }
            Message::Roles(Err(why)) => {
                self.roles_failed = Some(why);
                Vec::new()
            }
            Message::Loaded(Ok(snapshot)) => {
                self.loading = false;
                self.failed = None;
                self.me = Some(snapshot.me);
                self.live = snapshot.live;
                self.mine = snapshot.mine;
                if self.current().is_some() {
                    self.searching = None;
                }
                let mut effects = self.follow();
                if !snapshot.notices.is_empty() {
                    let ids = snapshot.notices.iter().map(|notice| notice.id).collect();
                    let told = snapshot.notices.iter().filter_map(|notice| self.told(notice)).collect();
                    effects.push(Effect::Tell(told, ids));
                }
                effects
            }
            Message::Loaded(Err(why)) => {
                self.loading = false;
                self.failed = Some(why);
                Vec::new()
            }
            Message::Search if !self.acting && self.searching.is_none() => {
                self.acting = true;
                vec![Effect::Queue(true)]
            }
            Message::Cancel if !self.acting && self.searching.is_some() => {
                self.acting = true;
                vec![Effect::Queue(false)]
            }
            Message::Queued(entered, said) => {
                self.acting = false;
                match said {
                    Ok(()) => {
                        self.searching = entered.then_some(now);
                        self.failed = None;
                    }
                    Err(why) => self.failed = Some(why),
                }
                Vec::new()
            }
            Message::Answer(duel, version, accept) if !self.acting => {
                self.acting = true;
                vec![Effect::Answer(duel, version, accept)]
            }
            Message::Answered(said) => {
                self.acting = false;
                match said {
                    Ok(duel) => {
                        if let Some(held) = self.mine.iter_mut().find(|held| held.id == duel.id) {
                            *held = duel;
                        } else {
                            self.mine.push(duel);
                        }
                        self.failed = None;
                    }
                    Err(why) => self.failed = Some(why),
                }
                self.loading = true;
                vec![Effect::Load]
            }
            Message::Again => {
                if let Some(id) = self.duel.as_ref().filter(|duel| phase_of(&duel.state) != Phase::Result).map(|duel| duel.id.clone()) {
                    return self.read(id);
                }
                if self.loading {
                    return Vec::new();
                }
                self.loading = true;
                vec![Effect::Load]
            }
            Message::Read(Ok(duel)) => {
                self.reading = false;
                self.failed = None;
                let mut effects = Vec::new();
                if let Some(pool) = &duel.pool {
                    for set in pool.entries.iter().filter_map(|entry| entry.beatmapset_id) {
                        if self.asked.insert(set) {
                            effects.push(Effect::Cover(set));
                        }
                    }
                    if duel.actions.iter().any(|action| action == "pool_ack") && self.acked.as_deref() != Some(pool.manifest.as_str()) {
                        self.acked = Some(pool.manifest.clone());
                        effects.push(Effect::Act(duel.id.clone(), "pool-ack", serde_json::json!({ "manifest_hash": pool.manifest }), Some(duel.version)));
                    }
                }
                let playing = duel.rounds.last().filter(|_| duel.state == "PLAYING").map(|round| (duel.id.clone(), round.ordinal));
                effects.push(Effect::Playing(playing));
                if phase_of(&duel.state) == Phase::Result {
                    effects.push(Effect::Link(false));
                    self.linked = false;
                }
                if let Some(held) = self.mine.iter_mut().find(|held| held.id == duel.id) {
                    *held = duel.clone();
                }
                self.duel = Some(duel);
                effects
            }
            Message::Read(Err(why)) => {
                self.reading = false;
                self.failed = Some(why);
                Vec::new()
            }
            Message::Heard(crate::duels::Heard::Event { duel, .. }) if self.duel.as_ref().is_some_and(|held| held.id == duel) => self.read(duel),
            Message::Heard(crate::duels::Heard::Connected) => {
                self.linked = true;
                Vec::new()
            }
            Message::Heard(crate::duels::Heard::Lost(_)) => {
                self.linked = false;
                Vec::new()
            }
            Message::Heard(crate::duels::Heard::Frames { duel, player, round, frames }) if self.watching.as_ref().is_some_and(|held| held.id == duel) => {
                let round = round.unwrap_or_default();
                let seen = self.rounds.entry(player).or_default();
                if round > *seen {
                    *seen = round;
                }
                self.broadcast.take(player, round, &frames, std::time::Instant::now());
                Vec::new()
            }
            Message::Heard(crate::duels::Heard::Witness { duel, player, state }) if self.watching.as_ref().is_some_and(|held| held.id == duel) => {
                self.heard_states.insert(player, state);
                Vec::new()
            }
            Message::Heard(_) => Vec::new(),
            Message::Watch(id) => {
                let Some(running) = self.live.running.iter().find(|duel| duel.id == id).cloned() else { return Vec::new() };
                let mut effects = Vec::new();
                if let Some(previous) = self.watching.take() {
                    effects.push(Effect::Order(crate::duels::Order::Unwatch(previous.id)));
                }
                self.broadcast = crate::duels::Broadcast::default();
                self.rounds.clear();
                self.heard_states.clear();
                self.watching = Some(running);
                if !self.linked {
                    self.linked = true;
                    effects.push(Effect::Link(true));
                }
                effects.push(Effect::Order(crate::duels::Order::Watch(id)));
                effects
            }
            Message::Unwatch => {
                let Some(previous) = self.watching.take() else { return Vec::new() };
                let mut effects = vec![Effect::Order(crate::duels::Order::Unwatch(previous.id))];
                if self.linked && self.current().filter(|_| self.stage() == Stage::Busy).is_none() {
                    self.linked = false;
                    effects.push(Effect::Link(false));
                }
                effects
            }
            Message::Frame if self.moments.hold.as_ref().is_some_and(|(_, at)| at.elapsed().as_secs_f32() >= HOLD) => {
                let Some((slot, _)) = self.moments.hold.take() else { return Vec::new() };
                self.moments.sent = Some(slot.clone());
                self.step(Message::Act("draft-actions", serde_json::json!({ "slot": slot })), now)
            }
            Message::Frame => {
                if let Some((beatmap, since)) = self.hovered {
                    if self.hearing != Some(beatmap) && self.rested != Some(beatmap) && since.elapsed() >= DWELL {
                        self.hearing = Some(beatmap);
                        self.heard_at = Some(std::time::Instant::now());
                        return vec![Effect::Hear(Some(beatmap))];
                    }
                }
                if let Some(watching) = &mut self.watching {
                    if let Some(fresh) = self.live.running.iter().find(|duel| duel.id == watching.id) {
                        *watching = fresh.clone();
                    }
                }
                Vec::new()
            }
            Message::Act(action, body) if !self.acting => {
                let Some(duel) = &self.duel else { return Vec::new() };
                self.acting = true;
                vec![Effect::Act(duel.id.clone(), action, body, Some(duel.version))]
            }
            Message::Acted(said) => {
                self.acting = false;
                if said.is_err() {
                    self.moments.sent = None;
                }
                match said {
                    Ok(duel) => {
                        let id = duel.id.clone();
                        self.duel = Some(duel);
                        self.read(id)
                    }
                    Err(why) => {
                        self.failed = Some(why);
                        self.duel.as_ref().map(|duel| duel.id.clone()).map(|id| self.read(id)).unwrap_or_default()
                    }
                }
            }
            Message::Cover(set, handle) => {
                if let Some(handle) = handle {
                    self.covers.insert(set, handle);
                }
                Vec::new()
            }
            Message::Hover(Some(beatmap)) => {
                if self.hovered.is_some_and(|(held, _)| held == beatmap) {
                    return Vec::new();
                }
                self.hovered = Some((beatmap, std::time::Instant::now()));
                self.rested = None;
                let mut effects = Vec::new();
                if self.hearing.is_some_and(|heard| heard != beatmap) {
                    self.hearing = None;
                    effects.push(Effect::Hear(None));
                }
                if self.timed.insert(beatmap) {
                    effects.push(Effect::Tempo(beatmap));
                }
                effects
            }
            Message::Grip(slot) => {
                if self.moments.hold.is_none() && self.moments.sent.is_none() && !self.acting {
                    self.moments.hold = Some((slot, std::time::Instant::now()));
                    self.moments.let_go = None;
                }
                Vec::new()
            }
            Message::Release => {
                self.let_go();
                Vec::new()
            }
            Message::Hover(None) => {
                self.let_go();
                self.hovered = None;
                self.rested = None;
                if self.hearing.take().is_some() {
                    return vec![Effect::Hear(None)];
                }
                Vec::new()
            }
            Message::Tempo(beatmap, bpm) => {
                if let Some(bpm) = bpm.filter(|bpm| *bpm > 20.0 && *bpm < 400.0) {
                    self.tempos.insert(beatmap, bpm);
                }
                Vec::new()
            }
            Message::PreviewEnded => {
                self.rested = self.hearing.take();
                self.rested_at = Some(std::time::Instant::now());
                self.heard_at = None;
                Vec::new()
            }
            Message::Copy(text) => vec![Effect::Copy(text)],
            Message::Rate(slot, fit, difficulty) => {
                let held = self.rated.entry(slot.clone()).or_default();
                if fit.is_some() {
                    held.0 = fit;
                }
                if difficulty.is_some() {
                    held.1 = difficulty;
                }
                let (fit, difficulty) = held.clone();
                let Some(duel) = &self.duel else { return Vec::new() };
                let mut body = serde_json::json!({ "slot": slot });
                if let Some(fit) = fit {
                    body["fit"] = serde_json::json!(fit);
                }
                if let Some(difficulty) = difficulty {
                    body["difficulty"] = serde_json::json!(difficulty);
                }
                vec![Effect::Act(duel.id.clone(), "feedback", body, None)]
            }
            Message::Challenge(player) if !self.acting => {
                self.acting = true;
                vec![Effect::Invite(player)]
            }
            Message::Download(sets) if !self.fetching => {
                self.fetching = true;
                vec![Effect::Download(sets)]
            }
            Message::Downloaded(said) => {
                self.fetching = false;
                if let Err(why) = said {
                    self.failed = Some(why);
                }
                Vec::new()
            }
            Message::Close => {
                self.closed = self.duel.as_ref().map(|duel| duel.id.clone());
                self.duel = None;
                self.rated.clear();
                self.loading = true;
                vec![Effect::Hear(None), Effect::Link(false), Effect::Load]
            }
            _ => Vec::new(),
        }
    }

    fn read(&mut self, id: String) -> Vec<Effect> {
        if self.reading {
            return Vec::new();
        }
        self.reading = true;
        vec![Effect::Read(id)]
    }

    fn follow(&mut self) -> Vec<Effect> {
        let busy = self.current().filter(|_| self.stage() == Stage::Busy).map(|duel| duel.id.clone());
        let mut effects = Vec::new();
        match busy {
            Some(id) => {
                if !self.linked {
                    self.linked = true;
                    effects.push(Effect::Link(true));
                }
                effects.extend(self.read(id));
            }
            None => {
                if self.linked && self.watching.is_none() {
                    self.linked = false;
                    effects.push(Effect::Link(false));
                }
                if let Some(id) = self.duel.as_ref().filter(|duel| phase_of(&duel.state) != Phase::Result).map(|duel| duel.id.clone()) {
                    effects.extend(self.read(id));
                }
            }
        }
        effects
    }

    fn told(&self, notice: &crate::duels::Notice) -> Option<(&'static str, String)> {
        let kind = notice.payload.get("kind").and_then(serde_json::Value::as_str)?;
        let by = notice.payload.get("by").and_then(serde_json::Value::as_i64);
        let name = self.mine.iter().flat_map(|duel| duel.participants.iter())
            .find(|person| Some(person.player) == by || (by.is_none() && Some(person.player) != self.player))
            .and_then(|person| person.name.clone())
            .unwrap_or_default();
        let key = match kind {
            "invitation" => "duels-told-invitation",
            "accepted" => "duels-told-accepted",
            "declined" => "duels-told-declined",
            "invitation_expired" => "duels-told-expired",
            "matched" => "duels-told-matched",
            "room_ready" => "duels-told-room",
            "finished" => "duels-told-finished",
            "cancelled" => "duels-told-cancelled",
            _ => return None,
        };
        Some((key, name))
    }

    pub fn shown(&self) -> Option<&Duel> {
        self.duel.as_ref().filter(|duel| self.closed.as_deref() != Some(duel.id.as_str()))
    }

    pub fn side_of(&self, duel: &Duel) -> Option<String> {
        duel.participants.iter().find(|person| Some(person.player) == self.player).and_then(|person| person.side.clone())
    }

    pub fn current(&self) -> Option<&Duel> {
        self.mine.iter().find(|duel| !matches!(duel.state.as_str(), "COMPLETED" | "CANCELLED" | "DECLINED" | "EXPIRED"))
    }

    pub fn stage(&self) -> Stage {
        let Some(duel) = self.current() else {
            return if self.searching.is_some() { Stage::Searching } else { Stage::Idle };
        };
        let answers = duel.actions.iter().any(|action| action == "accept");
        match (duel.state.as_str(), answers) {
            ("MATCH_OFFERED", true) => Stage::Offer,
            ("MATCH_OFFERED", false) => Stage::Waiting,
            ("INVITED", true) => Stage::Challenged,
            ("INVITED", false) => Stage::Challenging,
            _ => Stage::Busy,
        }
    }

    pub fn opponent<'a>(&self, duel: &'a Duel) -> Option<&'a crate::duels::Participant> {
        duel.participants.iter().find(|person| Some(person.player) != self.player)
    }

    pub fn lively(&self) -> bool {
        self.hovered.is_some() || self.hearing.is_some()
    }

    pub fn polling(&self) -> bool {
        self.searching.is_some() || self.current().is_some() || self.duel.is_some()
    }
}

pub fn seconds_left(duel: &Duel, now: i64) -> Option<i64> {
    duel.deadlines
        .iter()
        .filter_map(|deadline| deadline.at.as_deref())
        .filter_map(|at| chrono::DateTime::parse_from_rfc3339(at).ok())
        .map(|at| (at.timestamp() - now).max(0))
        .min()
}

pub fn clock(seconds: i64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

pub fn side_colour(side: Option<&str>) -> Color {
    match side {
        Some("B") => SIDE_B,
        _ => SIDE_A,
    }
}

pub(crate) fn initial(name: &str) -> String {
    name.chars().find(|letter| letter.is_alphanumeric()).map(|first| first.to_uppercase().to_string()).unwrap_or_default()
}

pub(crate) fn fa(colour: Color) -> Color {
    Color { a: colour.a * ui::fade(), ..colour }
}

pub(crate) fn words<'a, M: 'a>(said: String, size: f32, colour: Color) -> Element<'a, M> {
    text(said).size(size).font(theme::SANS).color(fa(colour)).into()
}

pub(crate) fn strong<'a, M: 'a>(said: String, size: f32, colour: Color) -> Element<'a, M> {
    text(said).size(size).font(theme::SANS_SEMI).color(fa(colour)).into()
}

pub(crate) fn bold<'a, M: 'a>(said: String, size: f32, colour: Color) -> Element<'a, M> {
    text(said).size(size).font(theme::SANS_SEMI).color(fa(colour)).into()
}

const FACES: [Color; 8] = [color!(0x4f72c4), color!(0x2f8a8a), color!(0xa04878), color!(0xb0682c), color!(0x7a5ac8), color!(0x4a8a3a), color!(0x8a6a2a), color!(0xc0504a)];
const NOTE_ROOM: f32 = 140.0;
const WATCH_WIDTH: f32 = 46.0;
pub(crate) const ROW: Color = Color::from_rgba(0.085, 0.078, 0.082, 0.88);
pub(crate) const OUTLINE: Color = Color { a: 0.45, ..FAINT };

fn face_colour(name: &str) -> Color {
    let sum = name.bytes().fold(2_166_136_261u32, |acc, byte| (acc ^ byte as u32).wrapping_mul(16_777_619));
    FACES[(sum % FACES.len() as u32) as usize]
}

pub(crate) fn person<'a, M: 'a>(name: &str, side: f32) -> Element<'a, M> {
    face(name, side, 2.0)
}

pub(crate) fn mapper_face<'a, M: 'a>(name: &str, beatmap: i64, side: f32) -> Element<'a, M> {
    match crate::mapper_faces::of(beatmap) {
        Some(handle) => iced::widget::image(handle).content_fit(iced::ContentFit::Cover).width(side).height(side).border_radius(side / 2.0).opacity(ui::fade()).into(),
        None => face(name, side, 0.0),
    }
}

fn face<'a, M: 'a>(name: &str, side: f32, ring: f32) -> Element<'a, M> {
    let colour = fa(face_colour(name));
    let edge = fa(theme::GROUND);
    container(text(initial(name)).size(side * 0.42).font(theme::SANS_SEMI).color(fa(Color::WHITE)))
        .center_x(side)
        .center_y(side)
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(colour)),
            border: iced::Border { radius: (side / 2.0).into(), width: ring, color: edge },
            ..container::Style::default()
        })
        .into()
}

pub(crate) fn slab<'a>(inside: Element<'a, Message>, padding: f32) -> Element<'a, Message> {
    container(inside).padding(padding).width(Length::Fill).height(Length::Fill).style(theme::slab).into()
}

pub(crate) fn strip<'a>(parts: Vec<Element<'a, Message>>, lit: Option<Color>) -> Element<'a, Message> {
    let line = row(parts).spacing(18).align_y(Alignment::Center);
    container(line)
        .padding(Padding { top: 16.0, right: 20.0, bottom: 16.0, left: 20.0 })
        .max_width(640.0)
        .style(move |theme| {
            let mut style = theme::slab(theme);
            if let Some(colour) = lit {
                style.border = iced::Border { color: Color { a: 0.6, ..colour }, width: 1.5, radius: 16.0.into() };
            }
            style
        })
        .into()
}

pub(crate) fn outlined<'a, M: Clone + 'a>(inside: Element<'a, M>, on: Option<M>) -> Element<'a, M> {
    outlined_wide(container(inside).center_y(theme::CONTROL_HEIGHT - 2.0).into(), on)
}

pub(crate) fn outlined_wide<'a, M: Clone + 'a>(inside: Element<'a, M>, on: Option<M>) -> Element<'a, M> {
    let (row_fill, edge) = (fa(ROW), fa(OUTLINE));
    let made = iced::widget::button(inside).padding([0, 12]).style(move |_, status| {
        let lit = matches!(status, iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed);
        iced::widget::button::Style {
            background: Some(iced::Background::Color(if lit { row_fill } else { Color::TRANSPARENT })),
            text_color: INK,
            border: iced::Border { color: edge, width: 1.0, radius: theme::CONTROL_RADIUS.into() },
            ..iced::widget::button::Style::default()
        }
    });
    match on {
        Some(message) => made.on_press(message).into(),
        None => made.into(),
    }
}

fn find_button<'a>(w: &Words, on: Option<Message>) -> Element<'a, Message> {
    let inside = row![glyph(Icon::Swords, 16.0, Color::WHITE), strong(w.t("duels-find"), theme::BODY, Color::WHITE)].spacing(8).align_y(Alignment::Center);
    let made = iced::widget::button(container(inside).center_y(theme::CONTROL_HEIGHT - 2.0)).padding([0, 14]).style(ui::button_faded(theme::primary));
    match on {
        Some(message) => made.on_press(message).into(),
        None => made.into(),
    }
}

fn action<'a>(w: &Words, state: &State, now: i64) -> Element<'a, Message> {
    let busy = state.acting;
    match state.stage() {
        Stage::Idle => row![find_button(w, (!busy).then_some(Message::Search))].into(),
        Stage::Searching => {
            let since = state.searching.map_or(0, |at| (now - at).max(0));
            strip(vec![
                bold(clock(since), 28.0, INK),
                column![strong(w.t("duels-searching"), 17.0, INK), words(w.t("duels-searching-range"), 14.0, FAINT)].spacing(3).width(Length::Fill).into(),
                outlined(strong(w.t("duels-cancel"), theme::BODY, INK), (!busy).then_some(Message::Cancel)),
            ], None)
        }
        Stage::Offer | Stage::Challenged | Stage::Waiting | Stage::Challenging | Stage::Busy => {
            let duel = state.current().expect("a current duel");
            let them = state.opponent(duel);
            let name = them.and_then(|person| person.name.clone()).unwrap_or_default();
            let left = seconds_left(duel, now);
            let stage = state.stage();
            let (heading, note) = match stage {
                Stage::Offer => (name.clone(), w.t("duels-offer")),
                Stage::Waiting => (w.with("duels-waiting-for", &[("name", name.clone())]), w.t("duels-waiting-note")),
                Stage::Challenged => (w.with("duels-challenged-by", &[("name", name.clone())]), w.t("duels-friendly")),
                Stage::Challenging => (w.with("duels-challenge-sent", &[("name", name.clone())]), w.t("duels-friendly")),
                _ => (w.with("duels-busy", &[("name", name.clone())]), format!("{}:{}", duel.wins.a, duel.wins.b)),
            };
            let mut parts: Vec<Element<'a, Message>> = vec![
                person(&name, 48.0),
                column![strong(heading, 18.0, INK), words(note, 14.0, FAINT)].spacing(3).width(Length::Fill).into(),
            ];
            if let Some(seconds) = left.filter(|_| stage != Stage::Busy) {
                parts.push(bold(clock(seconds), 22.0, CLOCK));
            }
            match stage {
                Stage::Offer | Stage::Challenged => {
                    parts.push(outlined(strong(w.t("duels-decline"), theme::BODY, INK), (!busy).then(|| Message::Answer(duel.id.clone(), duel.version, false))));
                    parts.push(ui::primary(w.t("duels-accept"), (!busy).then(|| Message::Answer(duel.id.clone(), duel.version, true))));
                }
                Stage::Busy => parts.push(ui::primary(w.t("duels-return"), None)),
                _ => {}
            }
            let lit = matches!(stage, Stage::Offer | Stage::Challenged).then_some(theme::ACCENT);
            strip(parts, lit)
        }
    }
}

pub(crate) fn tally<'a>(value: u32, label: String, colour: Color) -> Element<'a, Message> {
    column![bold(value.to_string(), 28.0, colour), words(label, 14.0, FAINT)].spacing(2).into()
}

pub(crate) fn standing<'a>(w: &Words, me: Option<&Me>) -> Element<'a, Message> {
    let Some(me) = me else {
        return column![words(w.t("duels-rating"), 15.0, FAINT), bold("…".to_owned(), 96.0, INK)].spacing(8).into();
    };
    let tallies = row![tally(me.won, w.t("duels-won"), WON), tally(me.lost, w.t("duels-lost"), LOST)].spacing(30);
    if me.rating.calibrating {
        let filled = me.rating.played.min(me.rating.needed.max(1));
        let marks = (0..me.rating.needed.max(1)).map(|at| {
            let lit = at < filled;
            container(Space::new().width(Length::Fill).height(10))
                .style(move |_| container::Style { background: Some(iced::Background::Color(if lit { theme::ACCENT } else { ROW })), border: iced::Border { radius: 5.0.into(), ..iced::Border::default() }, ..container::Style::default() })
                .width(Length::Fill)
                .into()
        });
        let line = row(marks.collect::<Vec<Element<'a, Message>>>()).spacing(6);
        return column![
            words(w.t("duels-rating-hidden"), 15.0, FAINT),
            bold("?".to_owned(), 96.0, INK),
            container(column![
                row![strong(w.t("duels-calibration"), 17.0, INK), Space::new().width(Length::Fill), strong(w.with("duels-calibration-of", &[("played", filled.to_string()), ("needed", me.rating.needed.to_string())]), 16.0, MUTED)].align_y(Alignment::Center),
                line,
            ].spacing(14)).padding(20).max_width(560.0).style(theme::slab),
        ].spacing(12).into();
    }
    let rating = me.rating.rating.map(|value| w.lang().group(value.max(0) as u64)).unwrap_or_default();
    let spread = me.rating.rd.map(|rd| format!("± {rd}")).unwrap_or_default();
    column![
        words(w.t("duels-rating"), 15.0, FAINT),
        row![bold(rating, 96.0, INK), container(strong(spread, 24.0, MUTED)).padding(Padding { bottom: 16.0, ..Padding::ZERO })].spacing(16).align_y(Alignment::End),
        tallies,
    ].spacing(14).into()
}

fn seats(duel: &Running) -> Vec<(String, Option<String>)> {
    duel.players.iter().map(|seat| (seat.name.clone().unwrap_or_default(), seat.side.clone())).collect()
}

pub(crate) fn pair<'a>(duel: &Running, size: f32) -> Element<'a, Message> {
    let people = seats(duel);
    let mut faces: Vec<Element<'a, Message>> = Vec::new();
    for (at, (name, _)) in people.iter().enumerate() {
        faces.push(row![Space::new().width(at as f32 * size * 0.72), person(name, size)].into());
    }
    iced::widget::Stack::with_children(faces).width(size * (1.0 + 0.72 * (people.len().max(1) - 1) as f32)).height(size).into()
}

pub(crate) fn names<'a>(w: &Words, duel: &Running, size: f32) -> Element<'a, Message> {
    let people = seats(duel);
    let mut line = row![].spacing(5).align_y(Alignment::Center);
    for (at, (name, _)) in people.iter().enumerate() {
        if at > 0 {
            line = line.push(words(w.t("duels-and"), size - 1.0, FAINT));
        }
        line = line.push(strong(name.clone(), size, INK));
    }
    line.into()
}

fn ago(w: &Words, seconds: Option<i64>) -> String {
    match seconds {
        Some(seconds) if seconds < 120 => w.t("duels-just-now"),
        Some(seconds) if seconds < 3600 => w.with("duels-minutes-ago", &[("count", (seconds / 60).to_string())]),
        Some(seconds) => w.with("duels-hours-ago", &[("count", (seconds / 3600).to_string())]),
        None => String::new(),
    }
}

fn live_panel<'a>(w: &Words, live: &Live) -> Element<'a, Message> {
    let header = row![
        strong(w.t("duels-running"), 20.0, INK),
        Space::new().width(Length::Fill),
        container(Space::new().width(8).height(8)).style(|_| container::Style { background: Some(iced::Background::Color(LOST)), border: iced::Border { radius: 4.0.into(), ..iced::Border::default() }, ..container::Style::default() }),
        words(live.running.len().to_string(), 14.0, FAINT),
    ].spacing(6).align_y(Alignment::Center);
    let mut list = column![header].spacing(10);
    if live.running.is_empty() {
        list = list.push(words(w.t("duels-running-none"), 15.0, FAINT));
    }
    for duel in live.running.iter().take(6) {
        let note = duel.title.clone().filter(|title| !title.is_empty()).or_else(|| duel.slot.clone()).unwrap_or_else(|| w.t("duels-drafting"));
        let watch = outlined_wide(container(row![glyph(Icon::Eye, 15.0, INK), words(duel.watching.to_string(), 14.0, INK)].spacing(6).align_y(Alignment::Center)).center_x(WATCH_WIDTH).center_y(theme::CONTROL_HEIGHT - 2.0).into(), Some(Message::Watch(duel.id.clone())));
        let line = row![
            container(pair(duel, 32.0)).width(56),
            column![names(w, duel, 15.0), words(ui::fitted(&note, theme::SANS, 13.0, NOTE_ROOM), 13.0, FAINT)].spacing(2).width(Length::Fill),
            bold(format!("{}:{}", duel.wins.a, duel.wins.b), 17.0, INK),
            watch,
        ].spacing(12).align_y(Alignment::Center);
        list = list.push(container(line).padding(Padding { top: 10.0, right: 12.0, bottom: 10.0, left: 12.0 }).clip(true).style(|_| container::Style {
            background: Some(iced::Background::Color(ROW)),
            border: iced::Border { radius: 12.0.into(), ..iced::Border::default() },
            ..container::Style::default()
        }));
    }
    list = list.push(Space::new().height(14)).push(strong(w.t("duels-finished"), 20.0, INK));
    if live.finished.is_empty() {
        list = list.push(words(w.t("duels-finished-none"), 15.0, FAINT));
    }
    for duel in live.finished.iter().take(6) {
        list = list.push(row![
            container(names(w, duel, 15.0)).width(Length::Fill),
            bold(format!("{}:{}", duel.wins.a, duel.wins.b), 16.0, INK),
            container(words(ago(w, duel.seconds_ago), 13.0, FAINT)).width(64).align_x(iced::alignment::Horizontal::Right),
        ].spacing(12).align_y(Alignment::Center).height(40));
    }
    container(slab(list.into(), 22.0)).width(LIVE_WIDTH).height(Length::Fill).into()
}

pub fn view<'a>(state: &State, w: &Words, now: i64, wide: f32) -> Element<'a, Message> {
    if let Some(duel) = state.shown() {
        return crate::duel_view::view(state, duel, w, now, wide);
    }
    if let Some(running) = &state.watching {
        return crate::duel_watch::view(state, running, w, wide);
    }
    let mut left = column![standing(w, state.me.as_ref()), action(w, state, now)].spacing(26).width(Length::Fill).padding(Padding { top: 24.0, ..Padding::ZERO });
    if let Some(why) = state.failed.as_ref().or(state.roles_failed.as_ref()) {
        left = left.push(row![glyph(Icon::Warn, 16.0, LOST), words(w.with("duels-failed", &[("reason", why.clone())]), 14.0, MUTED)].spacing(8).align_y(Alignment::Center));
    }
    let page = row![left, live_panel(w, &state.live)].spacing(28).height(Length::Fill);
    container(page).padding(Padding { top: 8.0, right: 40.0, bottom: 28.0, left: 40.0 }).width(Length::Fill).height(Length::Fill).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duels::{Deadline, Participant};

    fn duel(state: &str, actions: &[&str]) -> Duel {
        Duel {
            id: "d".into(),
            state: state.into(),
            version: 3,
            participants: vec![Participant { player: 1, name: Some("NaumRedlo".into()), ..Participant::default() },
                               Participant { player: 2, name: Some("ssnowy".into()), ..Participant::default() }],
            actions: actions.iter().map(|action| (*action).to_owned()).collect(),
            deadlines: vec![Deadline { kind: "pair_confirm".into(), at: Some("2026-10-23T12:00:30Z".into()) }],
            ..Duel::default()
        }
    }

    #[test]
    fn the_stage_follows_the_current_duel_and_the_queue() {
        let mut state = State { player: Some(1), ..State::default() };
        assert_eq!(state.stage(), Stage::Idle);
        assert_eq!(state.update(Message::Search, 100), vec![Effect::Queue(true)]);
        assert!(state.update(Message::Search, 100).is_empty());
        state.update(Message::Queued(true, Ok(())), 100);
        assert_eq!((state.stage(), state.searching), (Stage::Searching, Some(100)));
        state.mine = vec![duel("MATCH_OFFERED", &["accept", "decline"])];
        assert_eq!(state.stage(), Stage::Offer);
        assert_eq!(state.opponent(state.current().unwrap()).and_then(|p| p.name.clone()), Some("ssnowy".into()));
        state.mine = vec![duel("MATCH_OFFERED", &[])];
        assert_eq!(state.stage(), Stage::Waiting);
        state.mine = vec![duel("INVITED", &["accept", "decline"])];
        assert_eq!(state.stage(), Stage::Challenged);
        state.mine = vec![duel("DRAFTING", &["draft"])];
        assert_eq!(state.stage(), Stage::Busy);
        state.mine = vec![duel("COMPLETED", &[])];
        state.searching = None;
        assert_eq!(state.stage(), Stage::Idle);
    }

    #[test]
    fn permissions_load_even_when_duel_data_fails() {
        let mut state = State::default();
        state.update(Message::Loaded(Err("profile failed".into())), 0);
        state.update(Message::Roles(Ok(vec!["OWNER".into()])), 0);
        assert_eq!(state.roles, vec!["OWNER"]);
        assert_eq!(state.failed.as_deref(), Some("profile failed"));
        state.update(Message::Roles(Err("roles failed".into())), 0);
        assert_eq!(state.roles_failed.as_deref(), Some("roles failed"));
        state.update(Message::Roles(Ok(Vec::new())), 0);
        assert!(state.roles.is_empty() && state.roles_failed.is_none());
    }

    #[test]
    fn answering_reloads_and_failures_are_kept() {
        let mut state = State { player: Some(1), ..State::default() };
        assert_eq!(state.update(Message::Answer("d".into(), 3, true), 0), vec![Effect::Answer("d".into(), 3, true)]);
        assert_eq!(state.update(Message::Answered(Err("busy".into())), 0), vec![Effect::Load]);
        assert_eq!(state.failed.as_deref(), Some("busy"));
        state.update(Message::Loaded(Err("offline".into())), 0);
        assert!(!state.loading && state.failed.as_deref() == Some("offline"));
    }

    #[test]
    fn a_running_duel_is_followed_linked_read_acknowledged_and_closed() {
        use crate::duels::{Entry, PoolView};
        let mut state = State { player: Some(1), ..State::default() };
        let snapshot = |mine: Vec<Duel>, notices: Vec<crate::duels::Notice>| Snapshot { me: Me::default(), live: Live::default(), mine, notices };
        let effects = state.update(Message::Loaded(Ok(snapshot(vec![duel("PREPARING", &[])], Vec::new()))), 0);
        assert_eq!(effects, vec![Effect::Link(true), Effect::Read("d".into())]);
        let mut full = duel("PREPARING", &["pool_ack"]);
        full.pool = Some(PoolView { manifest: "m1".into(), entries: vec![Entry { slot: "NM1".into(), beatmap_id: 5, beatmapset_id: Some(50), ..Entry::default() }] });
        let effects = state.update(Message::Read(Ok(full.clone())), 0);
        assert!(effects.contains(&Effect::Cover(50)));
        assert!(effects.contains(&Effect::Act("d".into(), "pool-ack", serde_json::json!({ "manifest_hash": "m1" }), Some(3))));
        let again = state.update(Message::Read(Ok(full)), 0);
        assert!(!again.iter().any(|effect| matches!(effect, Effect::Cover(_) | Effect::Act(..))));
        assert_eq!(phase_of("DRAFTING"), Phase::Draft);
        let mut done = duel("COMPLETED", &["bonus"]);
        done.id = "d".into();
        let effects = state.update(Message::Read(Ok(done)), 0);
        assert!(effects.contains(&Effect::Link(false)) && state.shown().is_some());
        assert_eq!(state.update(Message::Close, 0), vec![Effect::Hear(None), Effect::Link(false), Effect::Load]);
        assert!(state.shown().is_none());
    }

    #[test]
    fn challenges_and_notices_become_effects() {
        let mut state = State { player: Some(1), ..State::default() };
        assert_eq!(state.update(Message::Challenge(3), 0), vec![Effect::Invite(3)]);
        state.acting = false;
        let notice = crate::duels::Notice { id: 9, duel: "d".into(), sequence: 1, payload: serde_json::json!({ "kind": "invitation", "by": 2 }) };
        let effects = state.update(Message::Loaded(Ok(Snapshot { me: Me::default(), live: Live::default(), mine: vec![duel("INVITED", &["accept"])], notices: vec![notice] })), 0);
        assert_eq!(effects.last(), Some(&Effect::Tell(vec![("duels-told-invitation", "ssnowy".into())], vec![9])));
    }

    #[test]
    fn clocks_counts_and_sides_read_right() {
        let at = chrono::DateTime::parse_from_rfc3339("2026-10-23T12:00:00Z").unwrap().timestamp();
        assert_eq!(seconds_left(&duel("MATCH_OFFERED", &[]), at), Some(30));
        assert_eq!(clock(107), "1:47");
        assert_eq!((side_colour(Some("A")), side_colour(Some("B"))), (SIDE_A, SIDE_B));
    }
}
