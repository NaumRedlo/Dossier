use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use reqwest::blocking::Client;

const KEEP_ALIVE: Duration = Duration::from_secs(30);
const IDLE_KEPT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Kind {
    connect: u64,
    total: u64,
    agent: String,
    accept: Option<String>,
}

fn kept() -> &'static Mutex<HashMap<Kind, Client>> {
    static KEPT: OnceLock<Mutex<HashMap<Kind, Client>>> = OnceLock::new();
    KEPT.get_or_init(Mutex::default)
}

pub fn client(connect: u64, total: u64, agent: &str, accept: Option<&str>) -> Result<Client, String> {
    let kind = Kind { connect, total, agent: agent.to_owned(), accept: accept.map(str::to_owned) };
    let mut all = kept().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(found) = all.get(&kind) {
        return Ok(found.clone());
    }
    let mut builder = Client::builder()
        .connect_timeout(Duration::from_secs(connect))
        .timeout(Duration::from_secs(total))
        .tcp_keepalive(KEEP_ALIVE)
        .pool_idle_timeout(IDLE_KEPT)
        .user_agent(agent);
    if let Some(accept) = accept.and_then(|accept| reqwest::header::HeaderValue::from_str(accept).ok()) {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::ACCEPT, accept);
        builder = builder.default_headers(headers);
    }
    let made = builder.build().map_err(|e| e.to_string())?;
    all.insert(kind, made.clone());
    Ok(made)
}

#[cfg(test)]
fn is_kept(connect: u64, total: u64, agent: &str, accept: Option<&str>) -> bool {
    let kind = Kind { connect, total, agent: agent.to_owned(), accept: accept.map(str::to_owned) };
    kept().lock().unwrap_or_else(|poisoned| poisoned.into_inner()).contains_key(&kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_kind_of_client_is_built_once_and_another_kind_is_built_apart() {
        assert!(!is_kept(3, 4_001, "net-test/1", None));
        client(3, 4_001, "net-test/1", None).expect("a client");
        assert!(is_kept(3, 4_001, "net-test/1", None));
        client(3, 4_001, "net-test/1", None).expect("the same client again");
        assert!(!is_kept(3, 4_002, "net-test/1", None), "another total patience is another client");
        assert!(!is_kept(3, 4_001, "net-test/1", Some("text/html")), "another header is another client");
        client(3, 4_001, "net-test/1", Some("text/html")).expect("a client with a header");
        assert!(is_kept(3, 4_001, "net-test/1", Some("text/html")));
    }

    #[test]
    fn a_header_that_cannot_be_sent_does_not_stop_the_client() {
        client(3, 4_003, "net-test/2", Some("bad\nheader")).expect("a client without the header");
    }
}
