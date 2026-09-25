use std::time::Duration;

pub const ENGINE: &str = concat!("dossier ", env!("CARGO_PKG_VERSION"));
pub const BUILD: &str = env!("CARGO_PKG_VERSION");
pub const PRERELEASE: bool = true;

const PATIENCE: Duration = Duration::from_secs(8);

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
    let response = client()?
        .post(format!("{server}/render/me/profile"))
        .header("X-Render-Worker", name)
        .bearer_auth(token)
        .json(card)
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
) -> Result<i64, Refused> {
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
    let answer: serde_json::Value = status(response)?.json().map_err(|e| Refused::Network(e.to_string()))?;
    Ok(answer.get("message_id").and_then(|m| m.as_i64()).unwrap_or(0))
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
    if let Some(folder) = into.parent() {
        std::fs::create_dir_all(folder).map_err(|e| Refused::Network(e.to_string()))?;
    }
    let part = into.with_extension("part");
    let mut out = std::fs::File::create(&part).map_err(|e| Refused::Network(e.to_string()))?;
    let written = std::io::copy(&mut response, &mut out).map_err(|e| Refused::Network(e.to_string()))?;
    drop(out);
    std::fs::rename(&part, into).map_err(|e| Refused::Network(e.to_string()))?;
    Ok(written)
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
