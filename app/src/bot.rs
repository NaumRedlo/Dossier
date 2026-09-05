use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum Refused {
    Token,

    Build { reason: String, release: String },

    Network(String),
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Token => write!(f, "Сервер отверг токен"),
            Self::Build { reason, release } if release.is_empty() => write!(f, "{reason}"),

            Self::Build { reason, release } => write!(f, "{reason} · нужен релиз {release}"),
            Self::Network(said) => write!(f, "{said}"),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Hello {
    #[serde(default)]
    pub build: String,
    #[serde(default)]
    pub agree: bool,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub waiting: u32,
    #[serde(default)]
    pub release: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Farm {
    #[serde(default)]
    pub waiting: u32,
    #[serde(default)]
    pub workers: Vec<Worker>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Worker {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub build: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub threads: u32,
    #[serde(default)]
    pub polite: bool,
    #[serde(default)]
    pub delivered: u32,
    #[serde(default)]
    pub handed_back: u32,
}

#[derive(Debug, Deserialize)]
pub struct Job {
    pub id: String,
    #[serde(default)]
    pub title: String,

    #[serde(default)]
    pub assets: Vec<String>,
    #[serde(default)]
    pub settings: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct Capacity {
    pub take: bool,
    pub reason: String,

    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub threads: u32,
    pub polite: bool,
}

pub struct Bot {
    base: String,
    http: reqwest::blocking::Client,
}

impl Bot {
    pub fn new(base: &str, token: &str, name: &str) -> Result<Self, Refused> {
        use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

        let mut headers = HeaderMap::new();
        let bearer = format!("Bearer {token}");
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&bearer).map_err(|_| Refused::Token)?,
        );
        headers.insert(
            "X-Render-Worker",
            HeaderValue::from_str(name).unwrap_or_else(|_| HeaderValue::from_static("worker")),
        );
        let http = reqwest::blocking::Client::builder()
            .default_headers(headers)

            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| Refused::Network(e.to_string()))?;
        Ok(Self {
            base: base.trim_end_matches('/').to_owned(),
            http,
        })
    }

    pub fn hello(&self, engine: &str) -> Result<Hello, Refused> {
        let reply = self
            .http
            .get(format!("{}/render/hello", self.base))
            .query(&[("engine", engine)])
            .send()
            .map_err(|e| Refused::Network(e.to_string()))?;
        if reply.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(Refused::Token);
        }
        reply
            .error_for_status()
            .map_err(|e| Refused::Network(e.to_string()))?
            .json()
            .map_err(|e| Refused::Network(e.to_string()))
    }

    pub fn farm(&self) -> Result<Farm, Refused> {
        let reply = self
            .http
            .get(format!("{}/render/farm", self.base))
            .send()
            .map_err(|e| Refused::Network(e.to_string()))?;
        if reply.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(Refused::Token);
        }
        reply
            .error_for_status()
            .map_err(|e| Refused::Network(e.to_string()))?
            .json()
            .map_err(|e| Refused::Network(e.to_string()))
    }

    pub fn claim(&self, engine: &str, capacity: &Capacity) -> Result<Option<Job>, Refused> {
        #[derive(Serialize)]
        struct Asking<'a> {
            engine: &'a str,
            capacity: &'a Capacity,
        }

        let reply = self
            .http
            .post(format!("{}/render/claim", self.base))
            .json(&Asking { engine, capacity })
            .send()
            .map_err(|e| Refused::Network(e.to_string()))?;
        match reply.status().as_u16() {
            204 => Ok(None),
            401 => Err(Refused::Token),
            409 => {
                let said: serde_json::Value = reply.json().unwrap_or_default();
                Err(Refused::Build {
                    reason: said
                        .get("reason")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Сборки не совпадают")
                        .to_owned(),
                    release: said
                        .get("release")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_owned(),
                })
            }
            _ => reply
                .error_for_status()
                .map_err(|e| Refused::Network(e.to_string()))?
                .json()
                .map(Some)
                .map_err(|e| Refused::Network(e.to_string())),
        }
    }

    pub fn replay(&self, job: &str) -> Result<Vec<u8>, Refused> {
        self.fetch(&format!("{}/render/job/{job}/replay", self.base))
    }

    pub fn asset(&self, job: &str, name: &str) -> Result<Vec<u8>, Refused> {
        self.fetch(&format!("{}/render/job/{job}/file/{name}", self.base))
    }

    fn fetch(&self, url: &str) -> Result<Vec<u8>, Refused> {
        let reply = self
            .http
            .get(url)
            .send()
            .map_err(|e| Refused::Network(e.to_string()))?
            .error_for_status()
            .map_err(|e| Refused::Network(e.to_string()))?;
        reply
            .bytes()
            .map(|b| b.to_vec())
            .map_err(|e| Refused::Network(e.to_string()))
    }

    pub fn report(&self, what: &serde_json::Value) -> Result<(), Refused> {
        let sent = self
            .http
            .post(format!("{}/render/report", self.base))
            .json(what)
            .send()
            .map_err(|why| Refused::Network(why.to_string()))?;
        if sent.status().is_success() {
            Ok(())
        } else {
            Err(Refused::Build {
                reason: format!("Сервер ответил {}", sent.status().as_u16()),
                release: String::new(),
            })
        }
    }

    pub fn heartbeat(&self, job: &str, progress: Option<serde_json::Value>) -> bool {
        let sent = self
            .http
            .post(format!("{}/render/job/{job}/heartbeat", self.base))
            .json(&serde_json::json!({ "progress": progress }))
            .send();
        match sent {
            Ok(reply) => reply.status() == reqwest::StatusCode::OK,
            Err(_) => true,
        }
    }

    pub fn deliver(&self, job: &str, file: &Path, meta: &serde_json::Value) -> Result<(), Refused> {
        let body = std::fs::read(file).map_err(|e| Refused::Network(e.to_string()))?;
        self.http
            .post(format!("{}/render/job/{job}/result", self.base))
            .header("Content-Type", "application/octet-stream")
            .header("X-Render-Meta", meta.to_string())
            .body(body)
            .send()
            .map_err(|e| Refused::Network(e.to_string()))?
            .error_for_status()
            .map_err(|e| Refused::Network(e.to_string()))?;
        Ok(())
    }

    pub fn give_back(&self, job: &str, why: &str) {
        let _ = self
            .http
            .post(format!("{}/render/job/{job}/give-back", self.base))
            .json(&serde_json::json!({ "reason": why }))
            .send();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    #[test]
    fn a_refusal_over_the_build_names_the_release_everybody_needs() {
        let said = Refused::Build {
            reason: "Сборки разные".to_owned(),
            release: "v0.12.0".to_owned(),
        };
        assert_eq!(said.to_string(), "Сборки разные · нужен релиз v0.12.0");
        let plain = Refused::Build {
            reason: "Сборки разные".to_owned(),
            release: String::new(),
        };
        assert_eq!(plain.to_string(), "Сборки разные");
    }

    #[test]
    fn a_hello_says_whether_this_machine_could_work_without_claiming_anything() {
        let (base, heard) = one_request(concat!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 89\r\n\r\n",
            r#"{"build":"dossier 0.12.0","agree":false,"reason":"stale","waiting":3,"release":"v0.12.0"}"#
        ));
        let bot = Bot::new(&base, "token", "machine").expect("a bot");
        let said = bot.hello("dossier 0.11.0").expect("an answer");
        let asked = heard.recv().expect("a request");
        assert!(
            asked.contains("GET /render/hello?engine=dossier"),
            "{asked}"
        );
        assert!(!said.agree);
        assert_eq!(said.release, "v0.12.0");
        assert_eq!(said.reason, "stale");
        assert_eq!(said.waiting, 3);
    }

    fn one_request(answer: &'static str) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
        let base = format!("http://{}", listener.local_addr().expect("an address"));
        let (tell, heard) = mpsc::channel();
        std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().expect("a caller");
            let mut got = vec![0u8; 8192];
            let read = socket.read(&mut got).unwrap_or(0);
            tell.send(String::from_utf8_lossy(&got[..read]).into_owned())
                .ok();
            socket.write_all(answer.as_bytes()).ok();
            socket.flush().ok();
        });
        (base, heard)
    }

    fn capacity() -> Capacity {
        Capacity {
            take: true,
            reason: "Устройство свободно".to_owned(),
            code: "idle".to_owned(),
            detail: None,
            threads: 6,
            polite: false,
        }
    }

    #[test]
    fn a_claim_carries_the_token_the_name_and_what_the_machine_will_give() {
        let (base, heard) = one_request("HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n");
        let bot = Bot::new(&base, "sekrit", "drejk").expect("a line to the bot");
        assert!(
            bot.claim("dossier 0.11.0 (abc1234)", &capacity())
                .expect("asked")
                .is_none(),
            "204 is nothing to do, not a failure"
        );
        let asked = heard.recv().expect("the request");
        assert!(asked.starts_with("POST /render/claim "), "{asked}");
        assert!(asked.contains("authorization: Bearer sekrit"), "{asked}");
        assert!(asked.contains("x-render-worker: drejk"), "{asked}");

        assert!(asked.contains("\"threads\":6"), "{asked}");
        assert!(asked.contains("\"code\":\"idle\""), "{asked}");
    }

    #[test]
    fn a_rejected_token_is_not_weather() {
        let (base, _heard) = one_request("HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n");
        let bot = Bot::new(&base, "wrong", "drejk").expect("a line");
        assert!(matches!(
            bot.claim("engine", &capacity()),
            Err(Refused::Token)
        ));
    }

    #[test]
    fn a_build_that_does_not_match_says_what_to_do_and_which_release() {
        let body = r#"{"reason":"Бот рисует сборкой a, а воркер — b","release":"0.11.0"}"#;
        let answer: &'static str = Box::leak(
            format!(
                "HTTP/1.1 409 Conflict\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            )
            .into_boxed_str(),
        );
        let (base, _heard) = one_request(answer);
        let bot = Bot::new(&base, "sekrit", "drejk").expect("a line");
        match bot.claim("engine", &capacity()) {
            Err(Refused::Build { reason, release }) => {
                assert!(reason.contains("Воркер"), "{reason}");
                assert_eq!(release, "0.11.0");
            }
            other => panic!("expected a build mismatch, got {other:?}"),
        }
    }

    #[test]
    fn a_job_arrives_with_its_assets_and_its_settings() {
        let body = r#"{"id":"j1","title":"a play","assets":["a0"],"settings":{"skin":"{{a0}}"}}"#;
        let answer: &'static str = Box::leak(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            )
            .into_boxed_str(),
        );
        let (base, _heard) = one_request(answer);
        let bot = Bot::new(&base, "sekrit", "drejk").expect("a line");
        let job = bot
            .claim("engine", &capacity())
            .expect("asked")
            .expect("a job");
        assert_eq!(job.id, "j1");
        assert_eq!(job.assets, vec!["a0".to_owned()]);
        assert_eq!(job.settings["skin"], "{{a0}}");
    }

    #[test]
    fn a_heartbeat_that_cannot_be_sent_keeps_the_job() {
        let bot = Bot::new("http://127.0.0.1:1", "sekrit", "drejk").expect("a line");
        assert!(bot.heartbeat("j1", None), "a blip threw away a render");

        let (base, _heard) = one_request("HTTP/1.1 410 Gone\r\nContent-Length: 0\r\n\r\n");
        let bot = Bot::new(&base, "sekrit", "drejk").expect("a line");
        assert!(
            !bot.heartbeat("j1", None),
            "the job was taken and we held on"
        );
    }
}
