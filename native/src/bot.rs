use std::time::Duration;

pub const ENGINE: &str = concat!("dossier ", env!("CARGO_PKG_VERSION"));
pub const BUILD: &str = env!("CARGO_PKG_VERSION");

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
