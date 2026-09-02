//! Talking to the bot: asking for work, saying we are still on it, handing it back.
//!
//! The same seven requests the terminal worker makes, in the same order, with
//! the same meanings — see `client/dossier/worker.py`, which is what this
//! replaces. Nothing here decides anything; it asks and reports.

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Why the bot would not give us work.
///
/// Three of these are ordinary and one is not. A token the server rejects is
/// somebody's setup and stops the worker; a build that does not match is a
/// machine that would draw the wrong thing and also stops it; the rest is
/// weather.
#[derive(Debug)]
pub enum Refused {
    /// The server does not know this token.
    Token,
    /// The bot's engine and ours are not the same. Carries what to do about it
    /// and which release everybody should be on.
    Build { reason: String, release: String },
    /// A blip. The lease outlives several of these.
    Network(String),
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Token => write!(f, "сервер отверг токен"),
            Self::Build { reason, .. } => write!(f, "{reason}"),
            Self::Network(said) => write!(f, "{said}"),
        }
    }
}

/// What `/render/hello` says: enough to know whether this machine could work
/// without claiming anything.
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

/// A job, as the bot hands it over.
#[derive(Debug, Deserialize)]
pub struct Job {
    pub id: String,
    #[serde(default)]
    pub title: String,
    /// Names of files to fetch. The settings refer to them as `{{name}}`.
    #[serde(default)]
    pub assets: Vec<String>,
    #[serde(default)]
    pub settings: serde_json::Value,
}

/// What this machine is willing to give, sent with every claim.
///
/// Sent even when it says no. A worker that declines by going quiet is one
/// nobody can tell from a worker that is switched off, and "three machines are
/// here and all on battery" wants a different reaction from "nobody is here".
#[derive(Debug, Clone, Serialize)]
pub struct Capacity {
    pub take: bool,
    pub reason: String,
    /// The same thing as a word, so the bot can say it in whatever language the
    /// person looking at the farm reads.
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
    /// A worker's end of the line.
    ///
    /// The token and the name go on every request rather than being handed over
    /// once: there is no session here, and a request that arrives without them
    /// is not from a worker.
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
            // Long enough for a slow line, short enough that a machine which
            // has gone away is noticed within a poll rather than a lease.
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| Refused::Network(e.to_string()))?;
        Ok(Self {
            base: base.trim_end_matches('/').to_owned(),
            http,
        })
    }

    /// Is the token good, do the builds agree, is there anything waiting.
    ///
    /// Takes no job and claims nothing, which is what makes it safe to ask on a
    /// readiness screen.
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

    /// Ask for work. `Ok(None)` means there is none, which is not a failure.
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
                        .unwrap_or("сборки не совпадают")
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

    /// The replay a job is about.
    pub fn replay(&self, job: &str) -> Result<Vec<u8>, Refused> {
        self.fetch(&format!("{}/render/job/{job}/replay", self.base))
    }

    /// One of the files a job named — a skin, a picture.
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

    /// Say we are still on it. `false` means the job stopped being ours.
    ///
    /// A failed request is not a lost job: the lease outlives several of these,
    /// and abandoning a half-finished render over one bad moment would throw
    /// away minutes of work.
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

    /// Hand the finished file over.
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

    /// Give it back, so somebody else can have it.
    ///
    /// A courtesy rather than the mechanism: the lease expiring does the same
    /// thing a moment later.
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

    /// A server that answers one request and hands back what it was asked.
    ///
    /// Written out rather than mocked: what is being checked here is the shape
    /// of a request going over a socket — the path, the headers, the body — and
    /// a mock would only be able to confirm this file's own idea of it.
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
            reason: "машина свободна".to_owned(),
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
        // The capacity goes even when it says yes: the farm view is the reason
        // it is sent at all.
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
        let body = r#"{"reason":"бот рисует сборкой a, а воркер — b","release":"0.11.0"}"#;
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
                assert!(reason.contains("воркер"), "{reason}");
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

    /// A blip is not a lost job, and only a plain `200` means it is still ours.
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
