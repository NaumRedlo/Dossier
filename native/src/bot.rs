use std::time::Duration;

pub const ENGINE: &str = concat!("dossier ", env!("CARGO_PKG_VERSION"));
pub const BUILD: &str = env!("CARGO_PKG_VERSION");

const PATIENCE: Duration = Duration::from_secs(8);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    Network(String),
    NoSuchCode,
    NotThere,
    Said(String),
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refused::Network(said) | Refused::Said(said) => write!(f, "{said}"),
            Refused::NoSuchCode => write!(f, "no such code"),
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
    pub seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum Paired {
    Waiting,
    Linked {
        token: String,
        #[serde(default)]
        who: String,
    },
    Expired,
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
        403 => Err(Refused::NoSuchCode),
        404 => Err(Refused::NotThere),
        code => Err(Refused::Said(format!("{code}"))),
    }
}

pub fn hello(server: &str, token: &str) -> Result<Hello, Refused> {
    let mut request = client()?
        .get(format!("{server}/render/hello"))
        .query(&[("engine", ENGINE)]);
    if !token.is_empty() {
        request = request.bearer_auth(token);
    }
    let response = request.send().map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?
        .json::<Hello>()
        .map_err(|e| Refused::Network(e.to_string()))
}

pub fn join(server: &str, code: &str, name: &str) -> Result<String, Refused> {
    #[derive(serde::Deserialize)]
    struct Issued {
        token: String,
    }
    let response = client()?
        .post(format!("{server}/render/join"))
        .json(&serde_json::json!({ "code": code, "name": name }))
        .send()
        .map_err(|e| Refused::Network(e.to_string()))?;
    status(response)?
        .json::<Issued>()
        .map(|issued| issued.token)
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
    status(response)?
        .json::<Paired>()
        .map_err(|e| Refused::Network(e.to_string()))
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
    fn the_pairing_answer_is_read_by_its_state() {
        let waiting: Paired = serde_json::from_str(r#"{"state":"waiting"}"#).unwrap();
        assert_eq!(waiting, Paired::Waiting);
        let linked: Paired = serde_json::from_str(r#"{"state":"linked","token":"t","who":"naum"}"#).unwrap();
        assert_eq!(linked, Paired::Linked { token: "t".into(), who: "naum".into() });
    }
}
