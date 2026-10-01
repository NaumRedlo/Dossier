use std::time::Duration;

pub const ENGINE: &str = concat!("dossier ", env!("CARGO_PKG_VERSION"));
pub const BUILD: &str = env!("CARGO_PKG_VERSION");
pub const PRERELEASE: bool = true;

const PATIENCE: Duration = Duration::from_secs(8);
const DONATE_PATIENCE: Duration = Duration::from_secs(60);
const BOARD_PATIENCE: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    Network(String),
    NotThere,
    Said(String),
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refused::Network(said) | Refused::Said(said) => write!(f, "{said}"),
            Refused::NotThere => write!(f, "not there"),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Hello {
    #[serde(default)]
    pub build: String,
    #[serde(default)]
    pub agree: bool,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub waiting: u32,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Pairing {
    pub code: String,
    #[serde(default)]
    pub link: String,
    #[serde(default)]
    pub expires_in: u64,
    #[serde(default)]
    pub osu: bool,
}

pub fn osu_link(server: &str, code: &str) -> String {
    format!("{server}/render/pair/{}/osu", tidy(code))
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Chat {
    pub id: i64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub private: bool,
    #[serde(default)]
    pub photo: bool,
}

pub fn chat_avatar(server: &str, token: &str, name: &str, chat: i64) -> Result<Vec<u8>, Refused> {
    let response = client()?
        .get(format!("{server}/render/chat/{chat}/avatar"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?.bytes().map(|b| b.to_vec()).map_err(|e| Refused::Network(e.to_string()))
}

pub fn chats(server: &str, token: &str, name: &str) -> Result<Vec<Chat>, Refused> {
    let response = client()?
        .get(format!("{server}/render/me/chats"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?.json::<Vec<Chat>>().map_err(|e| Refused::Network(e.to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum Paired {
    Waiting,
    Linked {
        token: String,
        #[serde(default)]
        who: String,
    },
    Gone,
    Throttled,
}

fn client() -> Result<reqwest::blocking::Client, Refused> {
    reqwest::blocking::Client::builder()
        .timeout(PATIENCE)
        .user_agent(ENGINE)
        .build()
        .map_err(|e| Refused::Network(e.to_string()))
}

fn status(response: reqwest::blocking::Response) -> Result<reqwest::blocking::Response, Refused> {
    match response.status().as_u16() {
        200..=299 => Ok(response),
        404 => Err(Refused::NotThere),
        code => Err(Refused::Said(format!("{code}"))),
    }
}

pub fn hello(server: &str, token: &str, name: &str) -> Result<Hello, Refused> {
    let mut request = client()?
        .get(format!("{server}/render/hello"))
        .header("X-Render-Worker", name)
        .query(&[("engine", ENGINE)]);
    if !token.is_empty() {
        request = request.bearer_auth(token);
    }
    let response = request.send().map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?
        .json::<Hello>()
        .map_err(|e| Refused::Network(e.to_string()))
}

pub fn pair(server: &str, name: &str) -> Result<Pairing, Refused> {
    let response = client()?
        .post(format!("{server}/render/pair"))
        .json(&serde_json::json!({
            "name": name,
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "cores": std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
            "build": BUILD,
        }))
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?
        .json::<Pairing>()
        .map_err(|e| Refused::Network(e.to_string()))
}

pub fn paired(server: &str, code: &str) -> Result<Paired, Refused> {
    let response = client()?
        .get(format!("{server}/render/pair/{code}"))
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    match response.status().as_u16() {
        404 => return Ok(Paired::Gone),
        429 => return Ok(Paired::Throttled),
        _ => {}
    }
    status(response)?
        .json::<Paired>()
        .map_err(|e| Refused::Network(e.to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Me {
    pub telegram_id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub avatar: bool,
    #[serde(default = "yes")]
    pub telegram: bool,
    #[serde(default)]
    pub player: Option<i64>,
}

fn yes() -> bool {
    true
}

pub fn link_telegram(server: &str, token: &str, name: &str) -> Result<Pairing, Refused> {
    let response = client()?
        .post(format!("{server}/render/me/telegram"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?.json::<Pairing>().map_err(|e| Refused::Network(e.to_string()))
}

pub fn me(server: &str, token: &str, name: &str) -> Result<Me, Refused> {
    let response = client()?
        .get(format!("{server}/render/me"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?.json::<Me>().map_err(|e| Refused::Network(e.to_string()))
}

pub fn avatar(server: &str, token: &str, name: &str) -> Result<Vec<u8>, Refused> {
    let response = client()?
        .get(format!("{server}/render/me/avatar"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?.bytes().map(|b| b.to_vec()).map_err(|e| Refused::Network(e.to_string()))
}

pub fn community(server: &str, token: &str, name: &str, chat: Option<i64>) -> Result<crate::community::wire::Community, Refused> {
    let mut request = client()?.get(format!("{server}/render/community")).header("X-Render-Worker", name).bearer_auth(token);
    if let Some(chat) = chat {
        request = request.query(&[("chat", chat)]);
    }
    let response = request.send().map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?.json().map_err(|e| Refused::Network(e.to_string()))
}

pub fn map_board(server: &str, token: &str, name: &str, chat: Option<i64>, beatmap: u64, fresh: bool) -> Result<crate::community::wire::MapBoard, Refused> {
    let asking = if fresh { long(BOARD_PATIENCE)? } else { client()? };
    let mut request = asking.get(format!("{server}/render/maps/{beatmap}/board")).header("X-Render-Worker", name).bearer_auth(token).query(&[("sync", if fresh { "1" } else { "0" })]);
    if let Some(chat) = chat {
        request = request.query(&[("chat", chat)]);
    }
    let response = request.send().map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?.json().map_err(|e| Refused::Network(e.to_string()))
}

pub fn donate(server: &str, token: &str, name: &str, replay: Vec<u8>) -> Result<bool, Refused> {
    let response = long(DONATE_PATIENCE)?
        .post(format!("{server}/render/me/replay"))
        .header("X-Render-Worker", name)
        .header("Content-Type", "application/octet-stream")
        .bearer_auth(token)
        .body(replay)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    let answer: serde_json::Value = status(response)?.json().map_err(|e| Refused::Network(e.to_string()))?;
    Ok(answer.get("known").and_then(|known| known.as_bool()).unwrap_or(false))
}

pub fn played(server: &str, token: &str, name: &str) -> Result<(), Refused> {
    let response = client()?
        .post(format!("{server}/render/me/played"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response).map(|_| ())
}

pub fn wear_title(server: &str, token: &str, name: &str, chat: Option<i64>, code: Option<&str>) -> Result<(), Refused> {
    let response = client()?
        .post(format!("{server}/render/me/title"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .json(&serde_json::json!({ "chat": chat, "code": code }))
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response).map(|_| ())
}

pub fn card(server: &str, token: &str, name: &str, chat: Option<i64>) -> Result<crate::community::wire::Card, Refused> {
    let mut request = client()?.get(format!("{server}/render/me/card")).header("X-Render-Worker", name).bearer_auth(token);
    if let Some(chat) = chat {
        request = request.query(&[("chat", chat)]);
    }
    let response = request.send().map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?.json().map_err(|e| Refused::Network(e.to_string()))
}

pub fn share_card(server: &str, token: &str, name: &str, card: &crate::community::wire::Card) -> Result<(), Refused> {
    let mut payload = serde_json::to_value(card).map_err(|error| Refused::Network(error.to_string()))?;
    if let Some(fields) = payload.as_object_mut() {
        fields.remove("country_rank_samples");
        fields.remove("country_rank_history");
    }
    let response = client()?
        .post(format!("{server}/render/me/profile"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response).map(|_| ())
}

pub fn person(server: &str, token: &str, name: &str, chat: i64, id: i64) -> Result<crate::community::wire::Me, Refused> {
    let response = client()?
        .get(format!("{server}/render/community/person"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .query(&[("chat", chat), ("id", id)])
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?.json().map_err(|e| Refused::Network(e.to_string()))
}

pub fn players(server: &str, token: &str, name: &str) -> Result<crate::community::wire::Everyone, Refused> {
    let response = client()?
        .get(format!("{server}/render/players"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?.json().map_err(|e| Refused::Network(e.to_string()))
}

pub fn player(server: &str, token: &str, name: &str, id: i64) -> Result<crate::community::wire::Me, Refused> {
    let response = client()?
        .get(format!("{server}/render/players/{id}"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?.json().map_err(|e| Refused::Network(e.to_string()))
}

pub fn pinned(server: &str, token: &str, name: &str) -> Result<crate::community::wire::Pin, Refused> {
    let response = client()?
        .get(format!("{server}/render/me/pin"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?.json().map_err(|e| Refused::Network(e.to_string()))
}

pub fn pin(server: &str, token: &str, name: &str, chat: i64) -> Result<crate::community::wire::Pin, Refused> {
    let response = client()?
        .post(format!("{server}/render/me/pin"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .json(&serde_json::json!({"chat": chat}))
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    match response.status().as_u16() {
        200..=299 | 409 => response.json().map_err(|e| Refused::Network(e.to_string())),
        _ => status(response).and_then(|response| response.json().map_err(|e| Refused::Network(e.to_string()))),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Friends {
    Listed(Vec<crate::community::wire::Friend>),
    Need(String),
}

pub fn friends(server: &str, token: &str, name: &str) -> Result<Friends, Refused> {
    let response = client()?
        .get(format!("{server}/render/me/friends"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    if response.status().as_u16() == 409 {
        let said: serde_json::Value = response.json().map_err(|e| Refused::Network(e.to_string()))?;
        return Ok(Friends::Need(said.get("need").and_then(|need| need.as_str()).unwrap_or("link").to_owned()));
    }
    let said: crate::community::wire::Friends = status(response)?.json().map_err(|e| Refused::Network(e.to_string()))?;
    Ok(Friends::Listed(said.friends))
}

struct Counted<R> {
    inner: R,
    done: u64,
    tell: Box<dyn FnMut(u64) + Send>,
}

impl<R: std::io::Read> std::io::Read for Counted<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.done += n as u64;
        (self.tell)(self.done);
        Ok(n)
    }
}

pub fn send(
    server: &str,
    token: &str,
    name: &str,
    file: &std::path::Path,
    meta: &serde_json::Value,
    tell: impl FnMut(u64) + Send + 'static,
) -> Result<Sent, Refused> {
    let opened = std::fs::File::open(file).map_err(|e| Refused::Network(e.to_string()))?;
    let size = opened.metadata().map(|m| m.len()).unwrap_or(0);
    let body = reqwest::blocking::Body::sized(Counted { inner: opened, done: 0, tell: Box::new(tell) }, size);
    let response = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(1800))
        .user_agent(ENGINE)
        .build()
        .map_err(|e| Refused::Network(e.to_string()))?
        .post(format!("{server}/render/send"))
        .header("X-Render-Worker", name)
        .header("X-Render-Meta", meta.to_string())
        .header("Content-Type", "video/mp4")
        .bearer_auth(token)
        .body(body)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    if response.status().as_u16() == 413 {
        return Err(Refused::Said("too large".to_owned()));
    }
    status(response)?.json().map_err(|e| Refused::Network(e.to_string()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
pub struct Sent {
    #[serde(default, rename = "message_id")]
    pub message: i64,
    #[serde(default)]
    pub video: Option<u64>,
}

const INBOX_PATIENCE: Duration = Duration::from_secs(20);
const VIDEO_PATIENCE: Duration = Duration::from_secs(3600);

pub fn receivers(server: &str, token: &str, name: &str) -> Result<Vec<crate::inbox::Face>, Refused> {
    let response = long(INBOX_PATIENCE)?
        .get(format!("{server}/render/videos/receivers"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?.json::<crate::inbox::Receivers>().map(|found| found.people).map_err(|e| Refused::Network(e.to_string()))
}

pub fn video_replay(server: &str, token: &str, name: &str, video: u64, replay: Vec<u8>) -> Result<(), Refused> {
    let response = long(DONATE_PATIENCE)?
        .put(format!("{server}/render/videos/{video}/replay"))
        .header("X-Render-Worker", name)
        .header("Content-Type", "application/octet-stream")
        .bearer_auth(token)
        .body(replay)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response).map(|_| ())
}

pub fn share(server: &str, token: &str, name: &str, video: u64, to: &[i64]) -> Result<crate::inbox::Shared, Refused> {
    let response = long(INBOX_PATIENCE)?
        .post(format!("{server}/render/videos/{video}/share"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .json(&serde_json::json!({ "to": to }))
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?.json().map_err(|e| Refused::Network(e.to_string()))
}

pub fn inbox(server: &str, token: &str, name: &str) -> Result<crate::inbox::Inbox, Refused> {
    let response = long(INBOX_PATIENCE)?
        .get(format!("{server}/render/me/inbox"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?.json().map_err(|e| Refused::Network(e.to_string()))
}

pub fn accept(server: &str, token: &str, name: &str, from: &str) -> Result<String, Refused> {
    let response = long(INBOX_PATIENCE)?
        .post(format!("{server}/render/me/accept"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .json(&serde_json::json!({ "from": from }))
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    let answer: serde_json::Value = status(response)?.json().map_err(|e| Refused::Network(e.to_string()))?;
    Ok(answer.get("accept").and_then(|said| said.as_str()).unwrap_or(from).to_owned())
}

fn inbox_bytes(server: &str, token: &str, name: &str, id: u64, what: &str) -> Result<Vec<u8>, Refused> {
    let response = long(DONATE_PATIENCE)?
        .get(format!("{server}/render/inbox/{id}/{what}"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?.bytes().map(|b| b.to_vec()).map_err(|e| Refused::Network(e.to_string()))
}

pub fn inbox_thumb(server: &str, token: &str, name: &str, id: u64) -> Result<Vec<u8>, Refused> {
    inbox_bytes(server, token, name, id, "thumb")
}

pub fn inbox_replay(server: &str, token: &str, name: &str, id: u64) -> Result<Vec<u8>, Refused> {
    inbox_bytes(server, token, name, id, "replay")
}

pub fn inbox_video(server: &str, token: &str, name: &str, id: u64, into: &std::path::Path, mut tell: impl FnMut(u64, u64) -> bool) -> Result<u64, Refused> {
    let mut response = status(
        long(VIDEO_PATIENCE)?
            .get(format!("{server}/render/inbox/{id}/video"))
            .header("X-Render-Worker", name)
            .bearer_auth(token)
            .send()
            .map_err(|e| Refused::Network(e.to_string()))?,
    )?;
    let total = response.content_length().unwrap_or(0);
    if let Some(folder) = into.parent() {
        std::fs::create_dir_all(folder).map_err(|e| Refused::Network(e.to_string()))?;
    }
    let part = into.with_extension("part");
    let copied = (|| {
        use std::io::{Read, Write};
        let mut out = std::fs::File::create(&part).map_err(|e| Refused::Network(e.to_string()))?;
        let mut written = 0u64;
        let mut bytes = vec![0u8; 256 * 1024];
        loop {
            let count = response.read(&mut bytes).map_err(|e| Refused::Network(e.to_string()))?;
            if count == 0 {
                break;
            }
            out.write_all(&bytes[..count]).map_err(|e| Refused::Network(e.to_string()))?;
            written += count as u64;
            if !tell(written, total) {
                return Err(Refused::Said("stopped".to_owned()));
            }
        }
        if written == 0 || (total > 0 && written < total) {
            return Err(Refused::Network("the video came short".to_owned()));
        }
        out.sync_all().map_err(|e| Refused::Network(e.to_string()))?;
        drop(out);
        std::fs::rename(&part, into).map_err(|e| Refused::Network(e.to_string()))?;
        Ok(written)
    })();
    if copied.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    copied
}

pub fn replays_state(server: &str, token: &str, name: &str) -> Result<crate::mixed::State, Refused> {
    let response = long(INBOX_PATIENCE)?
        .get(format!("{server}/render/me/replays"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?.json().map_err(|e| Refused::Network(e.to_string()))
}

pub fn replays_switch(server: &str, token: &str, name: &str, on: bool) -> Result<crate::mixed::State, Refused> {
    let response = long(INBOX_PATIENCE)?
        .post(format!("{server}/render/me/replays"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .json(&serde_json::json!({ "on": on }))
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?.json().map_err(|e| Refused::Network(e.to_string()))
}

pub fn replay_share(server: &str, token: &str, name: &str, replay: Vec<u8>, meta: &serde_json::Value) -> Result<bool, Refused> {
    let response = long(DONATE_PATIENCE)?
        .post(format!("{server}/render/replays"))
        .header("X-Render-Worker", name)
        .header("X-Render-Meta", meta.to_string())
        .header("Content-Type", "application/octet-stream")
        .bearer_auth(token)
        .body(replay)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    let answer: serde_json::Value = status(response)?.json().map_err(|e| Refused::Network(e.to_string()))?;
    Ok(answer.get("known").and_then(|known| known.as_bool()).unwrap_or(false))
}

pub fn replays_listed(server: &str, token: &str, name: &str, everyone: bool) -> Result<Vec<crate::mixed::Listed>, Refused> {
    let response = long(INBOX_PATIENCE)?
        .get(format!("{server}/render/replays"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .query(&[("scope", if everyone { "all" } else { "chat" })])
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?.json::<crate::mixed::List>().map(|list| list.replays).map_err(|e| Refused::Network(e.to_string()))
}

pub fn replay_file(server: &str, token: &str, name: &str, hash: &str) -> Result<Vec<u8>, Refused> {
    let response = long(DONATE_PATIENCE)?
        .get(format!("{server}/render/replays/{hash}"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?.bytes().map(|b| b.to_vec()).map_err(|e| Refused::Network(e.to_string()))
}

fn inbox_deed(server: &str, token: &str, name: &str, id: u64, what: &str, patience: Duration) -> Result<(), Refused> {
    let response = long(patience)?
        .post(format!("{server}/render/inbox/{id}/{what}"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response).map(|_| ())
}

pub fn inbox_telegram(server: &str, token: &str, name: &str, id: u64) -> Result<(), Refused> {
    inbox_deed(server, token, name, id, "telegram", DONATE_PATIENCE)
}

pub fn inbox_seen(server: &str, token: &str, name: &str, id: u64) -> Result<(), Refused> {
    inbox_deed(server, token, name, id, "seen", INBOX_PATIENCE)
}

pub fn inbox_drop(server: &str, token: &str, name: &str, id: u64) -> Result<(), Refused> {
    let response = long(INBOX_PATIENCE)?
        .delete(format!("{server}/render/inbox/{id}"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response).map(|_| ())
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct Job {
    pub id: String,
    #[serde(default)]
    pub title: String,
    pub beatmap_md5: String,
    #[serde(default)]
    pub beatmapset_id: Option<u64>,
    pub settings: JobLook,
    #[serde(default)]
    pub skin: Option<JobSkin>,
    #[serde(default)]
    pub most: u64,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct JobLook {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    #[serde(default)]
    pub loudness: f64,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct JobSkin {
    pub name: String,
    pub hash: String,
    #[serde(default)]
    pub size: u64,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
pub struct Farm {
    #[serde(default)]
    pub waiting: u32,
    #[serde(default)]
    pub workers: Vec<FarmWorker>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct FarmWorker {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub delivered: u32,
    #[serde(default)]
    pub handed_back: u32,
    #[serde(default)]
    pub mine: bool,
}

fn long(timeout: Duration) -> Result<reqwest::blocking::Client, Refused> {
    reqwest::blocking::Client::builder().timeout(timeout).user_agent(ENGINE).build().map_err(|e| Refused::Network(e.to_string()))
}

pub fn claim(server: &str, token: &str, name: &str, take: bool) -> Result<Option<Job>, Refused> {
    let response = client()?
        .post(format!("{server}/render/claim"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .json(&serde_json::json!({"build": BUILD, "take": take}))
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    if response.status().as_u16() == 204 {
        return Ok(None);
    }
    status(response)?.json::<Job>().map(Some).map_err(|e| Refused::Network(e.to_string()))
}

pub fn job_file(server: &str, token: &str, name: &str, job: &str, what: &str, into: &std::path::Path) -> Result<u64, Refused> {
    job_file_limited(server, token, name, job, what, into, u64::MAX)
}

pub fn job_file_limited(server: &str, token: &str, name: &str, job: &str, what: &str, into: &std::path::Path, most: u64) -> Result<u64, Refused> {
    let mut response = long(Duration::from_secs(600))?
        .get(format!("{server}/render/job/{job}/{what}"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    if response.status().as_u16() == 409 {
        return Err(Refused::Said("not yours".to_owned()));
    }
    response = status(response)?;
    if response.content_length().is_some_and(|size| size > most) {
        return Err(Refused::Said("the job file is too large".to_owned()));
    }
    if let Some(folder) = into.parent() {
        std::fs::create_dir_all(folder).map_err(|e| Refused::Network(e.to_string()))?;
    }
    let part = into.with_extension("part");
    let copied = (|| {
        let mut out = std::fs::File::create(&part).map_err(|e| Refused::Network(e.to_string()))?;
        let written = copy_job_file(&mut response, &mut out, most).map_err(|e| Refused::Network(e.to_string()))?;
        out.sync_all().map_err(|e| Refused::Network(e.to_string()))?;
        drop(out);
        std::fs::rename(&part, into).map_err(|e| Refused::Network(e.to_string()))?;
        Ok(written)
    })();
    if copied.is_err() { let _ = std::fs::remove_file(&part); }
    copied
}

fn copy_job_file(from: &mut impl std::io::Read, into: &mut impl std::io::Write, most: u64) -> std::io::Result<u64> {
    let mut written = 0u64;
    let mut bytes = [0u8; 64 * 1024];
    loop {
        let count = from.read(&mut bytes)?;
        if count == 0 { return Ok(written); }
        written = written.checked_add(count as u64).filter(|size| *size <= most)
            .ok_or_else(|| std::io::Error::other("the job file is too large"))?;
        into.write_all(&bytes[..count])?;
    }
}

pub fn heartbeat(server: &str, token: &str, name: &str, job: &str, progress: &serde_json::Value) -> Result<bool, Refused> {
    let response = client()?
        .post(format!("{server}/render/job/{job}/heartbeat"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .json(&serde_json::json!({"progress": progress}))
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    match response.status().as_u16() {
        200..=299 => Ok(true),
        409 => Ok(false),
        code => Err(Refused::Said(format!("{code}"))),
    }
}

pub fn deliver(
    server: &str,
    token: &str,
    name: &str,
    job: &str,
    file: &std::path::Path,
    meta: &serde_json::Value,
    tell: impl FnMut(u64) + Send + 'static,
) -> Result<(), Refused> {
    let opened = std::fs::File::open(file).map_err(|e| Refused::Network(e.to_string()))?;
    let size = opened.metadata().map(|m| m.len()).unwrap_or(0);
    let body = reqwest::blocking::Body::sized(Counted { inner: opened, done: 0, tell: Box::new(tell) }, size);
    let response = long(Duration::from_secs(1800))?
        .post(format!("{server}/render/job/{job}/result"))
        .header("X-Render-Worker", name)
        .header("X-Render-Meta", meta.to_string())
        .header("Content-Type", "video/mp4")
        .bearer_auth(token)
        .body(body)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    match response.status().as_u16() {
        200..=299 => Ok(()),
        413 => Err(Refused::Said("too large".to_owned())),
        409 => Err(Refused::Said("not yours".to_owned())),
        code => Err(Refused::Said(format!("{code}"))),
    }
}

pub fn give_back(server: &str, token: &str, name: &str, job: &str, reason: &str) -> Result<(), Refused> {
    let response = client()?
        .post(format!("{server}/render/job/{job}/give-back"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .json(&serde_json::json!({"reason": reason}))
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response).map(|_| ())
}

pub fn farm(server: &str, token: &str, name: &str) -> Result<Farm, Refused> {
    let response = client()?
        .get(format!("{server}/render/farm"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?.json::<Farm>().map_err(|e| Refused::Network(e.to_string()))
}

pub fn pretty(code: &str) -> String {
    let clean: String = code.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    if clean.len() == 8 {
        format!("{}-{}", &clean[..4], &clean[4..])
    } else {
        clean
    }
}

pub fn tidy(code: &str) -> String {
    code.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_download_limits_also_apply_without_a_content_length_header() {
        let mut source = std::io::Cursor::new(b"12345");
        let mut output = Vec::new();
        assert!(copy_job_file(&mut source, &mut output, 4).is_err());
        assert!(output.is_empty());
        source.set_position(0);
        assert_eq!(copy_job_file(&mut source, &mut output, 5).unwrap(), 5);
        assert_eq!(output, b"12345");
    }

    #[test]
    fn a_code_is_shown_in_two_halves_and_read_back_as_one() {
        assert_eq!(pretty("K7QNM4XZ"), "K7QN-M4XZ");
        assert_eq!(tidy("k7qn-m4xz "), "K7QNM4XZ");
        assert_eq!(pretty("abc"), "abc");
    }

    #[test]
    fn the_pairing_answer_is_read_the_way_the_bot_writes_it() {
        let waiting: Paired = serde_json::from_str(r#"{"status": "waiting"}"#).unwrap();
        assert_eq!(waiting, Paired::Waiting);
        let linked: Paired = serde_json::from_str(r#"{"status": "linked", "token": "t"}"#).unwrap();
        assert_eq!(linked, Paired::Linked { token: "t".into(), who: String::new() });
        let gone: Paired = serde_json::from_str(r#"{"status": "gone"}"#).unwrap();
        assert_eq!(gone, Paired::Gone);
    }

    #[test]
    fn the_pairing_offer_is_read_the_way_the_bot_writes_it() {
        let offered: Pairing = serde_json::from_str(
            r#"{"code": "BH5D-W8PR", "link": "https://t.me/OneNineEightFourGlobalBot?start=pair-BH5DW8PR", "expires_in": 600}"#,
        )
        .unwrap();
        assert_eq!(offered.code, "BH5D-W8PR");
        assert!(offered.link.ends_with("pair-BH5DW8PR"));
        assert_eq!(offered.expires_in, 600);
    }
}
