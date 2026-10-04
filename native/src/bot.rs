use std::collections::HashMap;
use std::time::Duration;

pub const ENGINE: &str = concat!("dossier ", env!("CARGO_PKG_VERSION"));
pub const BUILD: &str = env!("CARGO_PKG_VERSION");
pub const PRERELEASE: bool = true;

const PATIENCE: Duration = Duration::from_secs(12);
const CONNECT_PATIENCE: Duration = Duration::from_secs(5);
const KEEP_ALIVE: Duration = Duration::from_secs(30);
const IDLE_KEPT: Duration = Duration::from_secs(60);
const RETRY_AFTER: Duration = if cfg!(test) { Duration::from_millis(20) } else { Duration::from_millis(1200) };
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
    twice(|| {
        let response = client()?
            .get(format!("{server}/render/me/chats"))
            .header("X-Render-Worker", name)
            .bearer_auth(token)
            .send()
            .map_err(|e| Refused::Network(e.to_string()))?;
        status(response)?.json::<Vec<Chat>>().map_err(|e| Refused::Network(e.to_string()))
    })
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
    static SHARED: std::sync::OnceLock<Result<reqwest::blocking::Client, String>> = std::sync::OnceLock::new();
    SHARED
        .get_or_init(|| {
            reqwest::blocking::Client::builder()
                .connect_timeout(CONNECT_PATIENCE)
                .timeout(PATIENCE)
                .tcp_keepalive(KEEP_ALIVE)
                .pool_idle_timeout(IDLE_KEPT)
                .user_agent(ENGINE)
                .build()
                .map_err(|e| e.to_string())
        })
        .clone()
        .map_err(Refused::Network)
}

fn transient(refused: &Refused) -> bool {
    match refused {
        Refused::Network(_) => true,
        Refused::Said(code) => matches!(code.as_str(), "502" | "503" | "504"),
        Refused::NotThere => false,
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Link {
    pub answered: Option<i64>,
    pub refused: Option<(i64, Refused)>,
    pub retries: u32,
    pub asked: u32,
}

fn link() -> &'static std::sync::Mutex<Link> {
    static LINK: std::sync::OnceLock<std::sync::Mutex<Link>> = std::sync::OnceLock::new();
    LINK.get_or_init(std::sync::Mutex::default)
}

pub fn link_state() -> Link {
    link().lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone()
}

fn noted(change: impl FnOnce(&mut Link)) {
    change(&mut link().lock().unwrap_or_else(|poisoned| poisoned.into_inner()));
}

fn clock() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_secs() as i64)
}

fn twice<T>(mut call: impl FnMut() -> Result<T, Refused>) -> Result<T, Refused> {
    noted(|link| link.asked += 1);
    let outcome = match call() {
        Err(first) if transient(&first) => {
            noted(|link| link.retries += 1);
            std::thread::sleep(RETRY_AFTER);
            call()
        }
        done => done,
    };
    match &outcome {
        Ok(_) | Err(Refused::NotThere) => noted(|link| link.answered = Some(clock())),
        Err(Refused::Said(code)) => noted(|link| {
            link.answered = Some(clock());
            link.refused = Some((clock(), Refused::Said(code.clone())));
        }),
        Err(refused) => noted(|link| link.refused = Some((clock(), refused.clone()))),
    }
    outcome
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

pub fn presence(server: &str, token: &str, name: &str) -> Result<(), Refused> {
    let response = client()?
        .post(format!("{server}/render/me/presence"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .timeout(Duration::from_secs(4))
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response).map(|_| ())
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

fn kept_community() -> &'static std::sync::Mutex<HashMap<String, (String, crate::community::wire::Community)>> {
    static KEPT: std::sync::OnceLock<std::sync::Mutex<HashMap<String, (String, crate::community::wire::Community)>>> = std::sync::OnceLock::new();
    KEPT.get_or_init(std::sync::Mutex::default)
}

pub fn community(server: &str, token: &str, name: &str, chat: Option<i64>) -> Result<crate::community::wire::Community, Refused> {
    let whose = format!("{server}\n{token}\n{chat:?}");
    twice(|| {
        let mut request = client()?.get(format!("{server}/render/community")).header("X-Render-Worker", name).bearer_auth(token);
        if let Some(chat) = chat {
            request = request.query(&[("chat", chat)]);
        }
        let had = kept_community().lock().unwrap_or_else(|poisoned| poisoned.into_inner()).get(&whose).cloned();
        if let Some((tag, _)) = &had {
            request = request.header(reqwest::header::IF_NONE_MATCH, tag);
        }
        let response = request.send().map_err(|e| Refused::Network(e.to_string()))?;
        if response.status() == reqwest::StatusCode::NOT_MODIFIED {
            return had.map(|(_, body)| body).ok_or_else(|| Refused::Said("304".to_owned()));
        }
        let tag = response.headers().get(reqwest::header::ETAG).and_then(|tag| tag.to_str().ok()).map(str::to_owned);
        let body: crate::community::wire::Community = status(response)?.json().map_err(|e| Refused::Network(e.to_string()))?;
        let mut kept = kept_community().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        match tag {
            Some(tag) => kept.insert(whose.clone(), (tag, body.clone())),
            None => kept.remove(&whose),
        };
        Ok(body)
    })
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

pub fn witnessed(server: &str, token: &str, name: &str, play: &crate::witness::Told) -> Result<(), Refused> {
    let response = client()?
        .post(format!("{server}/render/me/play"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .json(play)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response).map(|_| ())
}

pub fn local_scores(server: &str, token: &str, name: &str, scores: &[crate::history::LocalScore]) -> Result<u32, Refused> {
    #[derive(serde::Deserialize)]
    struct Kept {
        kept: u32,
    }
    let response = client()?
        .post(format!("{server}/render/me/history"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .json(&serde_json::json!({ "scores": scores }))
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    let kept: Kept = status(response)?.json().map_err(|e| Refused::Network(e.to_string()))?;
    Ok(kept.kept)
}

pub fn witness_session(server: &str, token: &str, name: &str, sitting: &crate::witness::Sitting) -> Result<(), Refused> {
    let response = client()?
        .post(format!("{server}/render/me/session"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .json(sitting)
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
    twice(|| {
        let mut request = client()?.get(format!("{server}/render/me/card")).header("X-Render-Worker", name).bearer_auth(token);
        if let Some(chat) = chat {
            request = request.query(&[("chat", chat)]);
        }
        let response = request.send().map_err(|e| Refused::Network(e.to_string()))?;
        status(response)?.json().map_err(|e| Refused::Network(e.to_string()))
    })
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
    twice(|| {
        let response = client()?
            .get(format!("{server}/render/community/person"))
            .header("X-Render-Worker", name)
            .bearer_auth(token)
            .query(&[("chat", chat), ("id", id)])
            .send()
            .map_err(|e| Refused::Network(e.to_string()))?;
        status(response)?.json().map_err(|e| Refused::Network(e.to_string()))
    })
}

pub fn players(server: &str, token: &str, name: &str) -> Result<crate::community::wire::Everyone, Refused> {
    twice(|| {
        let response = client()?
            .get(format!("{server}/render/players"))
            .header("X-Render-Worker", name)
            .bearer_auth(token)
            .send()
            .map_err(|e| Refused::Network(e.to_string()))?;
        status(response)?.json().map_err(|e| Refused::Network(e.to_string()))
    })
}

pub fn player(server: &str, token: &str, name: &str, id: i64) -> Result<crate::community::wire::Me, Refused> {
    twice(|| {
        let response = client()?
            .get(format!("{server}/render/players/{id}"))
            .header("X-Render-Worker", name)
            .bearer_auth(token)
            .send()
            .map_err(|e| Refused::Network(e.to_string()))?;
        status(response)?.json().map_err(|e| Refused::Network(e.to_string()))
    })
}

pub fn pinned(server: &str, token: &str, name: &str) -> Result<crate::community::wire::Pin, Refused> {
    twice(|| {
        let response = client()?
            .get(format!("{server}/render/me/pin"))
            .header("X-Render-Worker", name)
            .bearer_auth(token)
            .send()
            .map_err(|e| Refused::Network(e.to_string()))?;
        status(response)?.json().map_err(|e| Refused::Network(e.to_string()))
    })
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
    twice(|| {
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
    })
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

fn upload(
    route: &str,
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
        .post(format!("{server}/render/{route}"))
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

pub fn send(server: &str, token: &str, name: &str, file: &std::path::Path, meta: &serde_json::Value, tell: impl FnMut(u64) + Send + 'static) -> Result<Sent, Refused> {
    upload("send", server, token, name, file, meta, tell)
}

pub fn share_upload(server: &str, token: &str, name: &str, file: &std::path::Path, meta: &serde_json::Value, tell: impl FnMut(u64) + Send + 'static) -> Result<Sent, Refused> {
    upload("videos", server, token, name, file, meta, tell)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
pub struct Sent {
    #[serde(default, rename = "message_id")]
    pub message: i64,
    #[serde(default)]
    pub video: Option<u64>,
}

pub fn video_ready(server: &str, token: &str, name: &str, video: u64) -> Result<bool, Refused> {
    let response = long(INBOX_PATIENCE)?
        .get(format!("{server}/render/videos/{video}"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    match response.status().as_u16() {
        200 => Ok(true),
        404 | 410 => Ok(false),
        _ => Err(Refused::Said(response.status().to_string())),
    }
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
    use sha2::{Digest, Sha256};

    let mut response = status(
        long(VIDEO_PATIENCE)?
            .get(format!("{server}/render/inbox/{id}/video"))
            .header("X-Render-Worker", name)
            .bearer_auth(token)
            .send()
            .map_err(|e| Refused::Network(e.to_string()))?,
    )?;
    let total = response.content_length().unwrap_or(0);
    let expected = response.headers().get("x-content-sha256").and_then(|value| value.to_str().ok()).unwrap_or("").to_owned();
    if let Some(folder) = into.parent() {
        std::fs::create_dir_all(folder).map_err(|e| Refused::Network(e.to_string()))?;
    }
    let part = into.with_extension("part");
    let copied = (|| {
        use std::io::{Read, Write};
        let mut out = std::fs::File::create(&part).map_err(|e| Refused::Network(e.to_string()))?;
        let mut written = 0u64;
        let mut digest = Sha256::new();
        let mut bytes = vec![0u8; 256 * 1024];
        loop {
            let count = response.read(&mut bytes).map_err(|e| Refused::Network(e.to_string()))?;
            if count == 0 {
                break;
            }
            out.write_all(&bytes[..count]).map_err(|e| Refused::Network(e.to_string()))?;
            digest.update(&bytes[..count]);
            written += count as u64;
            if !tell(written, total) {
                return Err(Refused::Said("stopped".to_owned()));
            }
        }
        if written == 0 || (total > 0 && written < total) {
            return Err(Refused::Network("the video came short".to_owned()));
        }
        if !expected.is_empty() && format!("{:x}", digest.finalize()) != expected {
            return Err(Refused::Network("the video checksum does not match".to_owned()));
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

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Restore {
    pub state: String,
}

pub fn inbox_restore(server: &str, token: &str, name: &str, id: u64, start: bool) -> Result<Restore, Refused> {
    let request = if start {
        long(INBOX_PATIENCE)?.post(format!("{server}/render/inbox/{id}/restore"))
    } else {
        long(INBOX_PATIENCE)?.get(format!("{server}/render/inbox/{id}/restore"))
    };
    let response = request.header("X-Render-Worker", name)
        .bearer_auth(token)
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?.json().map_err(|e| Refused::Network(e.to_string()))
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
    #[serde(default = "one")]
    pub music: f32,
    #[serde(default = "one")]
    pub hitsounds: f32,
    #[serde(default)]
    pub play: Option<crate::render::Play>,
}

fn one() -> f32 { 1.0 }

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
    use std::io::{Read, Write};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    const BUSY: &str = "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    const EMPTY_LIST: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n[]";
    const NOT_KNOWN: &str = "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    const NO_ONE: &str = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

    fn serve(answers: Vec<&'static str>) -> (String, Arc<AtomicUsize>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let count = Arc::new(AtomicUsize::new(0));
        let seen = count.clone();
        std::thread::spawn(move || {
            for answer in answers {
                let Ok((mut stream, _)) = listener.accept() else { return };
                let mut buffer = [0u8; 4096];
                let _ = stream.read(&mut buffer);
                seen.fetch_add(1, Ordering::SeqCst);
                let _ = stream.write_all(answer.as_bytes());
            }
        });
        (address, count)
    }

    #[test]
    fn presence_accepts_an_empty_acknowledgement_and_does_not_retry_a_failed_beat() {
        let (server, asked) = serve(vec!["HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n"]);
        assert_eq!(presence(&server, "token", "device"), Ok(()));
        assert_eq!(asked.load(Ordering::SeqCst), 1);
        let (server, asked) = serve(vec![BUSY]);
        assert_eq!(presence(&server, "token", "device"), Err(Refused::Said("503".into())));
        assert_eq!(asked.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_busy_server_is_asked_once_more_and_the_second_answer_is_taken() {
        let (server, asked) = serve(vec![BUSY, EMPTY_LIST]);
        let chats = chats(&server, "token", "device").unwrap();
        assert!(chats.is_empty());
        assert_eq!(asked.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn a_server_that_stays_busy_gives_up_after_the_second_try() {
        let (server, asked) = serve(vec![BUSY, BUSY, EMPTY_LIST]);
        assert_eq!(chats(&server, "token", "device"), Err(Refused::Said("503".into())));
        assert_eq!(asked.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn an_answer_that_will_not_change_is_not_asked_for_again() {
        let (server, asked) = serve(vec![NOT_KNOWN, EMPTY_LIST]);
        assert_eq!(chats(&server, "token", "device"), Err(Refused::Said("401".into())));
        assert_eq!(asked.load(Ordering::SeqCst), 1);
        let (server, asked) = serve(vec![NO_ONE, EMPTY_LIST]);
        assert_eq!(chats(&server, "token", "device"), Err(Refused::NotThere));
        assert_eq!(asked.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn nobody_listening_is_a_network_failure_that_is_tried_twice_and_comes_back_quickly() {
        let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let server = format!("http://{}", closed.local_addr().unwrap());
        drop(closed);
        let started = std::time::Instant::now();
        assert!(matches!(chats(&server, "token", "device"), Err(Refused::Network(_))));
        assert!(started.elapsed() < Duration::from_secs(9), "two refused connections must not run into the twelve seconds of patience");
    }

    fn serve_seen(answers: Vec<String>) -> (String, Arc<std::sync::Mutex<Vec<String>>>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let heard = seen.clone();
        std::thread::spawn(move || {
            for answer in answers {
                let Ok((mut stream, _)) = listener.accept() else { return };
                let mut head = Vec::new();
                let mut buffer = [0u8; 2048];
                while !head.windows(4).any(|window| window == b"\r\n\r\n") {
                    match stream.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(read) => head.extend_from_slice(&buffer[..read]),
                    }
                }
                heard.lock().unwrap().push(String::from_utf8_lossy(&head).to_string());
                let _ = stream.write_all(answer.as_bytes());
            }
        });
        (address, seen)
    }

    #[test]
    fn an_unchanged_feed_is_asked_for_with_its_tag_and_the_kept_body_is_used_on_not_modified() {
        let body = r#"{"week":7,"group":"Osu Squad"}"#;
        let full = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nETag: \"abc\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        let same = "HTTP/1.1 304 Not Modified\r\nETag: \"abc\"\r\nConnection: close\r\n\r\n".to_owned();
        let changed_body = r#"{"week":8,"group":"Osu Squad"}"#;
        let changed = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nETag: \"def\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{changed_body}", changed_body.len());
        let (server, seen) = serve_seen(vec![full, same, changed]);
        let first = community(&server, "token", "device", Some(-100)).unwrap();
        assert_eq!(first.week, 7);
        let second = community(&server, "token", "device", Some(-100)).unwrap();
        assert_eq!(second, first, "a 304 gives back the body that was kept");
        let third = community(&server, "token", "device", Some(-100)).unwrap();
        assert_eq!(third.week, 8);
        let seen = seen.lock().unwrap();
        assert!(!seen[0].to_ascii_lowercase().contains("if-none-match"), "the first ask carries no tag: {}", seen[0]);
        assert!(seen[1].to_ascii_lowercase().contains("if-none-match: \"abc\""), "{}", seen[1]);
        assert!(seen[2].to_ascii_lowercase().contains("if-none-match: \"abc\""), "{}", seen[2]);
    }

    #[test]
    fn a_tag_kept_for_one_chat_is_not_sent_for_another() {
        let body = r#"{"week":1}"#;
        let full = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nETag: \"one\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        let (server, seen) = serve_seen(vec![full.clone(), full]);
        community(&server, "token", "device", Some(-1)).unwrap();
        community(&server, "token", "device", Some(-2)).unwrap();
        assert!(!seen.lock().unwrap()[1].to_ascii_lowercase().contains("if-none-match"));
    }

    #[test]
    fn the_link_remembers_the_last_answer_the_last_refusal_and_the_retries() {
        let before = link_state();
        let (server, _) = serve(vec![BUSY, BUSY]);
        let _ = chats(&server, "token", "device");
        let after = link_state();
        assert!(after.retries >= before.retries + 1);
        assert!(after.asked >= before.asked + 1);
        assert!(matches!(after.refused, Some((_, Refused::Said(ref code))) if code == "503"));
        assert!(after.answered.is_some(), "a 503 is still an answer from the server");
        let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let gone = format!("http://{}", closed.local_addr().unwrap());
        drop(closed);
        let _ = chats(&gone, "token", "device");
        assert!(matches!(link_state().refused, Some((_, Refused::Network(_)))));
    }

    #[test]
    fn the_one_client_answers_request_after_request() {
        let (server, asked) = serve(vec![EMPTY_LIST, EMPTY_LIST, EMPTY_LIST]);
        for _ in 0..3 {
            assert!(chats(&server, "token", "device").unwrap().is_empty());
        }
        assert_eq!(asked.load(Ordering::SeqCst), 3);
    }

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
