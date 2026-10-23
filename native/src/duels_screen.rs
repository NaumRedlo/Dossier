use iced::widget::{column, container, row, text, Space};
use iced::{color, Alignment, Color, Element, Length, Padding};

use crate::duels::{Duel, Live, Me, Running};
use crate::glyphs::{glyph, Icon};
use crate::lang::Words;
use crate::theme::{self, FAINT, INK, MUTED};
use crate::ui;

pub const SIDE_A: Color = color!(0xf05a5a);
pub const SIDE_B: Color = color!(0x5b9bf0);
const WON: Color = theme::GRADE_A;
const LOST: Color = color!(0xf06868);
const CLOCK: Color = theme::GRADE_S;
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
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub me: Me,
    pub live: Live,
    pub mine: Vec<Duel>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Result<Snapshot, String>),
    Search,
    Cancel,
    Queued(bool, Result<(), String>),
    Answer(String, u32, bool),
    Answered(Result<Duel, String>),
    Again,
    Watch(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Load,
    Queue(bool),
    Answer(String, u32, bool),
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
        if self.loading {
            return Vec::new();
        }
        self.loading = true;
        vec![Effect::Load]
    }

    pub fn update(&mut self, message: Message, now: i64) -> Vec<Effect> {
        match message {
            Message::Loaded(Ok(snapshot)) => {
                self.loading = false;
                self.failed = None;
                self.me = Some(snapshot.me);
                self.live = snapshot.live;
                self.mine = snapshot.mine;
                if self.current().is_some() {
                    self.searching = None;
                }
                Vec::new()
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
                if self.loading {
                    return Vec::new();
                }
                self.loading = true;
                vec![Effect::Load]
            }
            _ => Vec::new(),
        }
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

    pub fn polling(&self) -> bool {
        self.searching.is_some() || self.current().is_some()
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

fn initial(name: &str) -> String {
    name.chars().next().map(|first| first.to_uppercase().to_string()).unwrap_or_default()
}

fn words<'a, M: 'a>(said: String, size: f32, colour: Color) -> Element<'a, M> {
    text(said).size(size).font(theme::SANS).color(colour).into()
}

fn strong<'a, M: 'a>(said: String, size: f32, colour: Color) -> Element<'a, M> {
    text(said).size(size).font(theme::SANS_SEMI).color(colour).into()
}

fn bold<'a, M: 'a>(said: String, size: f32, colour: Color) -> Element<'a, M> {
    text(said).size(size).font(theme::SANS_SEMI).color(colour).into()
}

const FACES: [Color; 8] = [color!(0x4f72c4), color!(0x2f8a8a), color!(0xa04878), color!(0xb0682c), color!(0x7a5ac8), color!(0x4a8a3a), color!(0x8a6a2a), color!(0xc0504a)];
const NOTE_ROOM: f32 = 140.0;
const WATCH_WIDTH: f32 = 46.0;
const ROW: Color = Color::from_rgba(0.085, 0.078, 0.082, 0.88);
const OUTLINE: Color = Color { a: 0.45, ..FAINT };

fn face_colour(name: &str) -> Color {
    let sum = name.bytes().fold(2_166_136_261u32, |acc, byte| (acc ^ byte as u32).wrapping_mul(16_777_619));
    FACES[(sum % FACES.len() as u32) as usize]
}

fn person<'a>(name: &str, side: f32) -> Element<'a, Message> {
    let colour = face_colour(name);
    container(text(initial(name)).size(side * 0.42).font(theme::SANS_SEMI).color(Color::WHITE))
        .center_x(side)
        .center_y(side)
        .style(move |_| container::Style {
            background: Some(iced::Background::Color(colour)),
            border: iced::Border { radius: (side / 2.0).into(), width: 2.0, color: theme::GROUND },
            ..container::Style::default()
        })
        .into()
}

fn slab<'a>(inside: Element<'a, Message>, padding: f32) -> Element<'a, Message> {
    container(inside).padding(padding).width(Length::Fill).height(Length::Fill).style(theme::slab).into()
}

fn strip<'a>(parts: Vec<Element<'a, Message>>, lit: Option<Color>) -> Element<'a, Message> {
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

fn outlined<'a>(inside: Element<'a, Message>, on: Option<Message>) -> Element<'a, Message> {
    outlined_wide(container(inside).center_y(theme::CONTROL_HEIGHT - 2.0).into(), on)
}

fn outlined_wide<'a>(inside: Element<'a, Message>, on: Option<Message>) -> Element<'a, Message> {
    let made = iced::widget::button(inside).padding([0, 12]).style(|_, status| {
        let lit = matches!(status, iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed);
        iced::widget::button::Style {
            background: Some(iced::Background::Color(if lit { ROW } else { Color::TRANSPARENT })),
            text_color: INK,
            border: iced::Border { color: OUTLINE, width: 1.0, radius: theme::CONTROL_RADIUS.into() },
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

fn tally<'a>(value: u32, label: String, colour: Color) -> Element<'a, Message> {
    column![bold(value.to_string(), 28.0, colour), words(label, 14.0, FAINT)].spacing(2).into()
}

fn standing<'a>(w: &Words, me: Option<&Me>) -> Element<'a, Message> {
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

fn pair<'a>(duel: &Running, size: f32) -> Element<'a, Message> {
    let people = seats(duel);
    let mut faces: Vec<Element<'a, Message>> = Vec::new();
    for (at, (name, _)) in people.iter().enumerate() {
        faces.push(row![Space::new().width(at as f32 * size * 0.72), person(name, size)].into());
    }
    iced::widget::Stack::with_children(faces).width(size * (1.0 + 0.72 * (people.len().max(1) - 1) as f32)).height(size).into()
}

fn names<'a>(w: &Words, duel: &Running, size: f32) -> Element<'a, Message> {
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

pub fn view<'a>(state: &State, w: &Words, now: i64) -> Element<'a, Message> {
    let mut left = column![standing(w, state.me.as_ref()), action(w, state, now)].spacing(26).width(Length::Fill).padding(Padding { top: 24.0, ..Padding::ZERO });
    if let Some(why) = &state.failed {
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
    fn answering_reloads_and_failures_are_kept() {
        let mut state = State { player: Some(1), ..State::default() };
        assert_eq!(state.update(Message::Answer("d".into(), 3, true), 0), vec![Effect::Answer("d".into(), 3, true)]);
        assert_eq!(state.update(Message::Answered(Err("busy".into())), 0), vec![Effect::Load]);
        assert_eq!(state.failed.as_deref(), Some("busy"));
        state.update(Message::Loaded(Err("offline".into())), 0);
        assert!(!state.loading && state.failed.as_deref() == Some("offline"));
    }

    #[test]
    fn clocks_counts_and_sides_read_right() {
        let at = chrono::DateTime::parse_from_rfc3339("2026-10-23T12:00:00Z").unwrap().timestamp();
        assert_eq!(seconds_left(&duel("MATCH_OFFERED", &[]), at), Some(30));
        assert_eq!(clock(107), "1:47");
        assert_eq!((side_colour(Some("A")), side_colour(Some("B"))), (SIDE_A, SIDE_B));
    }
}
