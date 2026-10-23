use std::collections::BTreeSet;
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::{json, Value};
use tungstenite::client::IntoClientRequest;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

use crate::witness::Cursor;

const HEARTBEAT: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(100);
const FIRST_RETRY: Duration = Duration::from_secs(1);
const LONGEST_RETRY: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct Account {
    pub server: String,
    pub token: String,
    pub device: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Participant {
    #[serde(default)]
    pub player: i64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub osu_user_id: i64,
    #[serde(default)]
    pub side: Option<String>,
    #[serde(default)]
    pub invited: bool,
    #[serde(default)]
    pub accepted: bool,
    #[serde(default)]
    pub connection: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Wins {
    #[serde(default, rename = "A")]
    pub a: u32,
    #[serde(default, rename = "B")]
    pub b: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Deadline {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub at: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Entry {
    #[serde(default)]
    pub slot: String,
    #[serde(default)]
    pub beatmap_id: i64,
    #[serde(default)]
    pub mods: Vec<String>,
    #[serde(default)]
    pub banned_by: Option<i64>,
    #[serde(default)]
    pub used: bool,
    #[serde(default)]
    pub stars: Option<f32>,
    #[serde(default)]
    pub review: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct PoolView {
    #[serde(default)]
    pub manifest: String,
    #[serde(default)]
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Room {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub match_id: Option<i64>,
    #[serde(default)]
    pub password: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Turn {
    #[serde(default)]
    pub turn: u32,
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub slot: String,
    #[serde(default)]
    pub by: Option<i64>,
    #[serde(default)]
    pub automatic: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Round {
    #[serde(default)]
    pub ordinal: u32,
    #[serde(default)]
    pub slot: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub winner: Option<i64>,
    #[serde(default)]
    pub ties: u32,
    #[serde(default)]
    pub picked_by: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Bonus {
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub consents: Vec<i64>,
    #[serde(default)]
    pub winner: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Duel {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub participants: Vec<Participant>,
    #[serde(default)]
    pub wins: Wins,
    #[serde(default)]
    pub winner: Option<i64>,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default)]
    pub deadlines: Vec<Deadline>,
    #[serde(default)]
    pub actions: Vec<String>,
    #[serde(default)]
    pub pool: Option<PoolView>,
    #[serde(default)]
    pub room: Option<Room>,
    #[serde(default)]
    pub draft: Vec<Turn>,
    #[serde(default)]
    pub rounds: Vec<Round>,
    #[serde(default)]
    pub bonus: Option<Bonus>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Notice {
    pub id: i64,
    pub duel: String,
    #[serde(default)]
    pub sequence: u32,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Failure {
    Network(String),
    Refused { code: String, message: String, retryable: bool, duel: Option<Box<Duel>> },
    Status(u16),
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Failure::Network(said) => write!(f, "{said}"),
            Failure::Refused { message, .. } => write!(f, "{message}"),
            Failure::Status(code) => write!(f, "{code}"),
        }
    }
}

pub fn key() -> String {
    static COUNT: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_nanos());
    format!("dossier-{:x}-{:x}-{:x}", nanos, std::process::id(), COUNT.fetch_add(1, Ordering::Relaxed))
}

fn refused(code: u16, body: &str) -> Failure {
    #[derive(Deserialize)]
    struct Said {
        #[serde(default)]
        error: String,
        #[serde(default)]
        message: String,
        #[serde(default)]
        retryable: bool,
        #[serde(default)]
        duel: Option<Duel>,
    }
    match serde_json::from_str::<Said>(body) {
        Ok(said) if !said.error.is_empty() => Failure::Refused { code: said.error, message: said.message, retryable: said.retryable, duel: said.duel.map(Box::new) },
        _ => Failure::Status(code),
    }
}

fn call<T: serde::de::DeserializeOwned>(account: &Account, method: reqwest::Method, path: &str, body: Option<Value>, operation: Option<&str>) -> Result<T, Failure> {
    let client = crate::bot::client().map_err(|why| Failure::Network(why.to_string()))?;
    let mut request = client
        .request(method, format!("{}{path}", account.server))
        .bearer_auth(&account.token)
        .header("X-Render-Worker", crate::bot::worker(&account.device));
    if let Some(operation) = operation {
        request = request.header("Idempotency-Key", operation);
    }
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request.send().map_err(|why| Failure::Network(why.to_string()))?;
    let code = response.status().as_u16();
    let text = response.text().map_err(|why| Failure::Network(why.to_string()))?;
    if !(200..300).contains(&code) {
        return Err(refused(code, &text));
    }
    serde_json::from_str(&text).map_err(|why| Failure::Network(why.to_string()))
}

pub fn invite(account: &Account, opponent: i64, operation: &str) -> Result<Duel, Failure> {
    call(account, reqwest::Method::POST, "/render/duels/invitations", Some(json!({ "opponent_id": opponent })), Some(operation))
}

pub fn act(account: &Account, duel: &str, action: &str, body: Value, version: Option<u32>, operation: &str) -> Result<Duel, Failure> {
    let mut body = body;
    if let (Some(version), Some(map)) = (version, body.as_object_mut()) {
        map.insert("expected_version".into(), json!(version));
    }
    call(account, reqwest::Method::POST, &format!("/render/duels/{duel}/{action}"), Some(body), Some(operation))
}

pub fn read(account: &Account, duel: &str) -> Result<Duel, Failure> {
    call(account, reqwest::Method::GET, &format!("/render/duels/{duel}"), None, None)
}

pub fn mine(account: &Account) -> Result<Vec<Duel>, Failure> {
    #[derive(Deserialize)]
    struct Listed {
        #[serde(default)]
        duels: Vec<Duel>,
    }
    call::<Listed>(account, reqwest::Method::GET, "/render/duels", None, None).map(|listed| listed.duels)
}

pub fn queue(account: &Account, enter: bool, operation: &str) -> Result<Value, Failure> {
    call(account, if enter { reqwest::Method::POST } else { reqwest::Method::DELETE }, "/render/duels/queue", None, Some(operation))
}

pub fn notices(account: &Account) -> Result<Vec<Notice>, Failure> {
    #[derive(Deserialize)]
    struct Listed {
        #[serde(default)]
        notifications: Vec<Notice>,
    }
    call::<Listed>(account, reqwest::Method::GET, "/render/duels/notifications", None, None).map(|listed| listed.notifications)
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Standing {
    #[serde(default)]
    pub calibrating: bool,
    #[serde(default)]
    pub played: u32,
    #[serde(default)]
    pub needed: u32,
    #[serde(default)]
    pub rating: Option<i64>,
    #[serde(default)]
    pub rd: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Me {
    #[serde(default)]
    pub rating: Standing,
    #[serde(default)]
    pub observations: u32,
    #[serde(default)]
    pub won: u32,
    #[serde(default)]
    pub lost: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Seat {
    #[serde(default)]
    pub player: i64,
    #[serde(default)]
    pub side: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Running {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub wins: Wins,
    #[serde(default)]
    pub players: Vec<Seat>,
    #[serde(default)]
    pub watching: u32,
    #[serde(default)]
    pub slot: Option<String>,
    #[serde(default)]
    pub beatmap_id: Option<i64>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub seconds_ago: Option<i64>,
    #[serde(default)]
    pub finished_at: Option<String>,
    #[serde(default)]
    pub winner: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Live {
    #[serde(default)]
    pub running: Vec<Running>,
    #[serde(default)]
    pub finished: Vec<Running>,
}

pub fn me(account: &Account) -> Result<Me, Failure> {
    call(account, reqwest::Method::GET, "/render/duels/profile/me", None, None)
}

pub fn live(account: &Account) -> Result<Live, Failure> {
    call(account, reqwest::Method::GET, "/render/duels/running", None, None)
}

pub fn roles(account: &Account) -> Result<Vec<String>, Failure> {
    #[derive(Deserialize)]
    struct Held {
        #[serde(default)]
        roles: Vec<String>,
    }
    call::<Held>(account, reqwest::Method::GET, "/render/duels/roles/me", None, None).map(|held| held.roles)
}

pub fn seen(account: &Account, ids: &[i64]) -> Result<Value, Failure> {
    call(account, reqwest::Method::POST, "/render/duels/notifications/seen", Some(json!({ "ids": ids })), None)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Heard {
    Connected,
    Lost(String),
    Watching(String),
    Event { duel: String, sequence: u32, event: String, payload: Value, version: u32 },
    Frames { duel: String, player: i64, round: Option<u32>, frames: Vec<(i32, f32, f32, u32)> },
    Witness { duel: String, player: i64, state: Value },
    Refused { duel: String, code: String },
}

pub fn heard_of(text: &str) -> Option<Heard> {
    let said: Value = serde_json::from_str(text).ok()?;
    let duel = said.get("duel").and_then(Value::as_str).unwrap_or_default().to_owned();
    let player = said.get("player").and_then(Value::as_i64).unwrap_or_default();
    match said.get("type")?.as_str()? {
        "watching" => Some(Heard::Watching(duel)),
        "event" => Some(Heard::Event {
            duel,
            sequence: said.get("sequence").and_then(Value::as_u64).unwrap_or_default() as u32,
            event: said.get("event").and_then(Value::as_str).unwrap_or_default().to_owned(),
            payload: said.get("payload").cloned().unwrap_or(Value::Null),
            version: said.get("version").and_then(Value::as_u64).unwrap_or_default() as u32,
        }),
        "frames" => Some(Heard::Frames {
            duel,
            player,
            round: said.get("round").and_then(Value::as_u64).map(|round| round as u32),
            frames: serde_json::from_value(said.get("frames")?.clone()).ok()?,
        }),
        "witness" => Some(Heard::Witness { duel, player, state: said.get("state").cloned().unwrap_or(Value::Null) }),
        "error" => Some(Heard::Refused { duel, code: said.get("error").and_then(Value::as_str).unwrap_or_default().to_owned() }),
        _ => None,
    }
}

pub fn socket_address(server: &str) -> String {
    let base = server.trim_end_matches('/');
    let swapped = if let Some(rest) = base.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = base.strip_prefix("http://") {
        format!("ws://{rest}")
    } else {
        base.to_owned()
    };
    format!("{swapped}/render/duels/live")
}

#[derive(Debug, Clone, PartialEq)]
pub enum Order {
    Watch(String),
    Unwatch(String),
    Playing(Option<(String, u32)>),
    Witness(String, Value),
    Bye(String),
    Stop,
}

pub struct Link {
    orders: Sender<Order>,
}

impl Link {
    pub fn send(&self, order: Order) -> bool {
        self.orders.send(order).is_ok()
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        let _ = self.orders.send(Order::Stop);
    }
}

type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

fn open(account: &Account) -> Result<Socket, String> {
    let mut request = socket_address(&account.server).into_client_request().map_err(|why| why.to_string())?;
    let headers = request.headers_mut();
    headers.insert("Authorization", format!("Bearer {}", account.token).parse().map_err(|_| "bad token".to_owned())?);
    headers.insert("X-Render-Worker", crate::bot::worker(&account.device).parse().map_err(|_| "bad device name".to_owned())?);
    let (socket, _) = tungstenite::connect(request).map_err(|why| why.to_string())?;
    let stream = match socket.get_ref() {
        MaybeTlsStream::Plain(stream) => Some(stream),
        MaybeTlsStream::Rustls(stream) => Some(stream.get_ref()),
        _ => None,
    };
    if let Some(stream) = stream {
        let _ = stream.set_read_timeout(Some(POLL));
    }
    Ok(socket)
}

fn say(socket: &mut Socket, body: Value) -> Result<(), String> {
    socket.send(Message::text(body.to_string())).map_err(|why| why.to_string())
}

fn frames_of(cursor: &Cursor) -> Value {
    Value::Array(cursor.frames.iter().map(|(time, x, y, keys)| json!([time, x, y, keys])).collect())
}

struct Held {
    watched: BTreeSet<String>,
    playing: Option<(String, u32)>,
}

fn session(account: &Account, held: &mut Held, orders: &Receiver<Order>, cursor: Option<&Receiver<Cursor>>, heard: &mut dyn FnMut(Heard) -> bool) -> Result<bool, String> {
    let mut socket = open(account)?;
    if !heard(Heard::Connected) {
        return Ok(false);
    }
    for duel in &held.watched {
        say(&mut socket, json!({ "type": "watch", "duel": duel }))?;
    }
    let mut beat = Instant::now() - HEARTBEAT;
    loop {
        loop {
            match orders.try_recv() {
                Ok(Order::Stop) | Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    let _ = socket.close(None);
                    return Ok(false);
                }
                Ok(Order::Watch(duel)) => {
                    say(&mut socket, json!({ "type": "watch", "duel": duel }))?;
                    held.watched.insert(duel);
                }
                Ok(Order::Unwatch(duel)) => {
                    say(&mut socket, json!({ "type": "unwatch", "duel": duel }))?;
                    held.watched.remove(&duel);
                }
                Ok(Order::Playing(now)) => held.playing = now,
                Ok(Order::Witness(duel, state)) => say(&mut socket, json!({ "type": "witness", "duel": duel, "state": state }))?,
                Ok(Order::Bye(duel)) => say(&mut socket, json!({ "type": "bye", "duel": duel }))?,
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
            }
        }
        if let Some(cursor) = cursor {
            while let Ok(fresh) = cursor.try_recv() {
                if let Some((duel, round)) = &held.playing {
                    say(&mut socket, json!({ "type": "frames", "duel": duel, "round": round, "frames": frames_of(&fresh) }))?;
                }
            }
        }
        if beat.elapsed() >= HEARTBEAT {
            say(&mut socket, json!({ "type": "heartbeat" }))?;
            beat = Instant::now();
        }
        match socket.read() {
            Ok(Message::Text(text)) => {
                if let Some(said) = heard_of(text.as_str()) {
                    if !heard(said) {
                        let _ = socket.close(None);
                        return Ok(false);
                    }
                }
            }
            Ok(Message::Close(_)) => return Err("closed".into()),
            Ok(_) => {}
            Err(tungstenite::Error::Io(why)) if matches!(why.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
            Err(why) => return Err(why.to_string()),
        }
    }
}

pub fn connect(account: Account, cursor: Option<Receiver<Cursor>>, mut heard: impl FnMut(Heard) -> bool + Send + 'static) -> Link {
    let (orders, inbox) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut held = Held { watched: BTreeSet::new(), playing: None };
        let mut pause = FIRST_RETRY;
        loop {
            let started = Instant::now();
            match session(&account, &mut held, &inbox, cursor.as_ref(), &mut heard) {
                Ok(false) => return,
                Ok(true) => {}
                Err(why) => {
                    if !heard(Heard::Lost(why)) {
                        return;
                    }
                }
            }
            if started.elapsed() > Duration::from_secs(60) {
                pause = FIRST_RETRY;
            }
            match inbox.recv_timeout(pause) {
                Ok(Order::Stop) | Err(RecvTimeoutError::Disconnected) => return,
                Ok(Order::Watch(duel)) => {
                    held.watched.insert(duel);
                }
                Ok(Order::Unwatch(duel)) => {
                    held.watched.remove(&duel);
                }
                Ok(Order::Playing(now)) => held.playing = now,
                Ok(_) | Err(RecvTimeoutError::Timeout) => {}
            }
            pause = (pause * 2).min(LONGEST_RETRY);
        }
    });
    Link { orders }
}

pub const WATCH_DELAY_MS: i64 = 3000;

#[derive(Debug, Clone)]
struct Track {
    frames: Vec<dossier_replay::ReplayFrame>,
    anchor: Option<(i64, Instant)>,
}

#[derive(Debug, Clone, Default)]
pub struct Broadcast {
    tracks: std::collections::BTreeMap<(i64, u32), Track>,
}

impl Broadcast {
    pub fn take(&mut self, player: i64, round: u32, frames: &[(i32, f32, f32, u32)], now: Instant) -> usize {
        let track = self.tracks.entry((player, round)).or_insert(Track { frames: Vec::new(), anchor: None });
        let before = track.frames.len();
        for &(time, x, y, keys) in frames {
            let time = i64::from(time);
            if track.frames.last().is_some_and(|last| time <= last.time_ms) || !x.is_finite() || !y.is_finite() {
                continue;
            }
            track.frames.push(dossier_replay::ReplayFrame { time_ms: time, x, y, keys: dossier_replay::Keys((keys & 0xFF) as u8) });
            if track.anchor.is_none() {
                track.anchor = Some((time, now));
            }
        }
        track.frames.len() - before
    }

    pub fn until(&self, player: i64, round: u32, now: Instant) -> Option<i64> {
        let (time, heard) = self.tracks.get(&(player, round))?.anchor?;
        Some(time + now.saturating_duration_since(heard).as_millis() as i64 - WATCH_DELAY_MS)
    }

    pub fn visible(&self, player: i64, round: u32, now: Instant) -> &[dossier_replay::ReplayFrame] {
        let (Some(track), Some(until)) = (self.tracks.get(&(player, round)), self.until(player, round, now)) else {
            return &[];
        };
        &track.frames[..track.frames.partition_point(|frame| frame.time_ms <= until)]
    }

    pub fn forget(&mut self, round_before: u32) {
        self.tracks.retain(|(_, round), _| *round >= round_before);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_broadcast_holds_its_frames_back_by_the_delay_and_drops_repeats() {
        let start = Instant::now();
        let mut seen = Broadcast::default();
        assert_eq!(seen.take(2, 1, &[(1000, 10.0, 20.0, 1), (1016, 11.0, 21.0, 0), (1032, 12.0, 22.0, 5)], start), 3);
        assert_eq!(seen.take(2, 1, &[(1016, 11.0, 21.0, 0), (1048, f32::NAN, 1.0, 0), (1064, 13.0, 23.0, 0)], start), 1);
        assert!(seen.visible(2, 1, start).is_empty());
        let later = start + Duration::from_millis(WATCH_DELAY_MS as u64 + 20);
        let shown = seen.visible(2, 1, later);
        assert_eq!(shown.iter().map(|frame| frame.time_ms).collect::<Vec<_>>(), [1000, 1016]);
        assert_eq!(shown[0].keys, dossier_replay::Keys(1));
        assert_eq!(seen.visible(2, 1, later + Duration::from_secs(10)).len(), 4);
        assert!(seen.visible(3, 1, later).is_empty());
        seen.take(2, 2, &[(500, 1.0, 1.0, 0)], later);
        seen.forget(2);
        assert!(seen.visible(2, 1, later + Duration::from_secs(10)).is_empty());
        assert_eq!(seen.visible(2, 2, later + Duration::from_secs(10)).len(), 1);
    }

    #[test]
    fn a_server_snapshot_reads_into_a_duel() {
        let said = r#"{"id":"d1","mode":"direct","state":"SELECTING","version":12,"participants":[{"player":1,"osu_user_id":101,"side":"A","invited":false,"accepted":true,"connection":"present"}],
            "wins":{"A":1,"B":0},"winner":null,"outcome":null,"deadlines":[{"kind":"draft","at":"2026-10-23T12:00:30Z"}],"actions":["pick","surrender"],
            "pool":{"manifest":"abc","entries":[{"slot":"NM1","beatmap_id":5,"mods":["NF"],"banned_by":2,"used":false,"stars":5.1,"review":"human"}]},
            "room":{"name":"Dossier x","match_id":500,"password":"pw"},"draft":[{"turn":0,"type":"ban","slot":"NM1","by":2,"automatic":false}],
            "rounds":[{"ordinal":1,"slot":"NM2","state":"done","winner":1,"ties":0,"picked_by":2}],"bonus":null,"extra":"ignored"}"#;
        let duel: Duel = serde_json::from_str(said).unwrap();
        assert_eq!((duel.state.as_str(), duel.version, duel.wins.a, duel.actions.len()), ("SELECTING", 12, 1, 2));
        assert_eq!(duel.pool.unwrap().entries[0].banned_by, Some(2));
        assert_eq!(duel.room.unwrap().match_id, Some(500));
        assert_eq!((duel.draft[0].kind.as_str(), duel.rounds[0].picked_by), ("ban", Some(2)));
    }

    #[test]
    fn a_refusal_keeps_its_code_and_the_current_duel() {
        let failure = refused(409, r#"{"error":"VERSION_CONFLICT","message":"Дуэль уже изменилась","retryable":true,"duel":{"id":"d1","version":3}}"#);
        match failure {
            Failure::Refused { code, retryable, duel, .. } => assert_eq!((code.as_str(), retryable, duel.unwrap().version), ("VERSION_CONFLICT", true, 3)),
            other => panic!("{other:?}"),
        }
        assert_eq!(refused(502, "<html>"), Failure::Status(502));
    }

    #[test]
    fn keys_never_repeat() {
        let keys: BTreeSet<String> = (0..1000).map(|_| key()).collect();
        assert_eq!(keys.len(), 1000);
        assert!(keys.iter().all(|key| (8..=64).contains(&key.len())));
    }

    #[test]
    fn the_socket_lives_beside_the_server() {
        assert_eq!(socket_address("https://bot.example/"), "wss://bot.example/render/duels/live");
        assert_eq!(socket_address("http://127.0.0.1:8080"), "ws://127.0.0.1:8080/render/duels/live");
    }

    #[test]
    fn stream_lines_become_what_was_heard() {
        assert_eq!(heard_of(r#"{"type":"watching","duel":"d1"}"#), Some(Heard::Watching("d1".into())));
        assert_eq!(heard_of(r#"{"type":"frames","duel":"d1","player":2,"round":3,"frames":[[1000,256.5,192,1]]}"#),
                   Some(Heard::Frames { duel: "d1".into(), player: 2, round: Some(3), frames: vec![(1000, 256.5, 192.0, 1)] }));
        match heard_of(r#"{"type":"event","duel":"d1","sequence":4,"event":"state","payload":{"to":"PLAYING"},"version":9}"#) {
            Some(Heard::Event { sequence, event, payload, version, .. }) => assert_eq!((sequence, event.as_str(), payload["to"].as_str(), version), (4, "state", Some("PLAYING"), 9)),
            other => panic!("{other:?}"),
        }
        assert_eq!(heard_of("not json"), None);
        assert_eq!(heard_of(r#"{"type":"frames","duel":"d1","frames":"bad"}"#), None);
    }

    #[test]
    fn the_link_watches_beats_forwards_frames_and_hears_the_server() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut seen_headers = Vec::new();
            let callback = |request: &tungstenite::handshake::server::Request, response| {
                seen_headers.push(request.headers().get("Authorization").and_then(|v| v.to_str().ok()).unwrap_or_default().to_owned());
                Ok(response)
            };
            let mut socket = tungstenite::accept_hdr(stream, callback).unwrap();
            let mut said = Vec::new();
            while said.len() < 3 {
                if let Ok(Message::Text(text)) = socket.read() {
                    let value: Value = serde_json::from_str(text.as_str()).unwrap();
                    if value["type"] == "watch" {
                        socket.send(Message::text(r#"{"type":"event","duel":"d1","sequence":2,"event":"state","payload":{"to":"PLAYING"},"version":5}"#)).unwrap();
                    }
                    said.push(value);
                }
            }
            (said, seen_headers)
        });
        let (sink, cursor) = std::sync::mpsc::channel();
        let (tell, heard) = std::sync::mpsc::channel();
        let link = connect(Account { server: format!("http://127.0.0.1:{port}"), token: "secret".into(), device: "дрейк".into() }, Some(cursor), move |said| tell.send(said).is_ok());
        assert_eq!(heard.recv_timeout(Duration::from_secs(5)).unwrap(), Heard::Connected);
        link.send(Order::Watch("d1".into()));
        link.send(Order::Playing(Some(("d1".into(), 1))));
        match heard.recv_timeout(Duration::from_secs(5)).unwrap() {
            Heard::Event { sequence, .. } => assert_eq!(sequence, 2),
            other => panic!("{other:?}"),
        }
        sink.send(Cursor { from: 0, frames: vec![(1000, 256.0, 192.0, 1)] }).unwrap();
        let (said, headers) = server.join().unwrap();
        let mut first: Vec<&str> = said[..2].iter().map(|value| value["type"].as_str().unwrap()).collect();
        first.sort_unstable();
        assert_eq!(first, ["heartbeat", "watch"]);
        assert_eq!(said[2]["type"], "frames");
        assert_eq!(said[2]["frames"], json!([[1000, 256.0, 192.0, 1]]));
        assert_eq!(said[2]["round"], json!(1));
        assert_eq!(headers, ["Bearer secret"]);
    }
}
