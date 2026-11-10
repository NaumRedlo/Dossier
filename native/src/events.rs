use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::time::{Duration, Instant};

use serde_json::Value;
use tungstenite::Message;

use crate::duels::Account;

const PATH: &str = "/render/events";
const PING_EVERY: Duration = Duration::from_secs(10);
const SILENCE: Duration = if cfg!(test) { Duration::from_secs(1) } else { Duration::from_secs(25) };
const GREETING_PATIENCE: Duration = Duration::from_secs(5);
const FIRST_RETRY: Duration = Duration::from_secs(1);
const LONGEST_RETRY: Duration = Duration::from_secs(30);
const STEADY_AFTER: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Topic {
    Duels,
    Live,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Heard {
    Connected(u32),
    Lost(String),
    Changed(Topic),
    Online(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Said {
    Hello(u32),
    Changed(Option<Topic>),
    Online(u32),
    Other,
}

pub fn said_of(text: &str) -> Result<Said, String> {
    let said: Value = serde_json::from_str(text).map_err(|why| why.to_string())?;
    let kind = said.get("type").and_then(Value::as_str).ok_or("a message without a type")?;
    let online = || said.get("online").and_then(Value::as_u64).map(|online| online as u32).ok_or_else(|| format!("{kind} without the online count"));
    match kind {
        "hello" => online().map(Said::Hello),
        "online" => online().map(Said::Online),
        "changed" => match said.get("topic").and_then(Value::as_str).ok_or("a change without a topic")? {
            "duels" => Ok(Said::Changed(Some(Topic::Duels))),
            "live" => Ok(Said::Changed(Some(Topic::Live))),
            _ => Ok(Said::Changed(None)),
        },
        _ => Ok(Said::Other),
    }
}

pub struct Link {
    stop: Sender<()>,
}

impl Drop for Link {
    fn drop(&mut self) {
        let _ = self.stop.send(());
    }
}

fn stopped(stop: &Receiver<()>) -> bool {
    !matches!(stop.try_recv(), Err(TryRecvError::Empty))
}

fn waiting(why: &tungstenite::Error) -> bool {
    matches!(why, tungstenite::Error::Io(why) if matches!(why.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut))
}

fn session(account: &Account, stop: &Receiver<()>, heard: &mut dyn FnMut(Heard) -> bool) -> Result<(), String> {
    let mut socket = crate::duels::open_at(account, PATH)?;
    let opened = Instant::now();
    let mut greeted = false;
    let mut pinged = Instant::now();
    let mut last_heard = Instant::now();
    loop {
        if stopped(stop) {
            let _ = socket.send(Message::text("bye"));
            let _ = socket.close(None);
            return Ok(());
        }
        if !greeted && opened.elapsed() > GREETING_PATIENCE {
            return Err("the server did not greet".into());
        }
        if pinged.elapsed() >= PING_EVERY {
            socket.send(Message::Ping(Vec::new().into())).map_err(|why| why.to_string())?;
            pinged = Instant::now();
        }
        if last_heard.elapsed() > SILENCE {
            return Err("the server went silent".into());
        }
        let read = socket.read();
        if read.is_ok() {
            last_heard = Instant::now();
        }
        let told = match read {
            Ok(Message::Text(text)) => match said_of(text.as_str())? {
                Said::Hello(online) => {
                    greeted = true;
                    Some(Heard::Connected(online))
                }
                Said::Changed(Some(topic)) if greeted => Some(Heard::Changed(topic)),
                Said::Online(online) if greeted => Some(Heard::Online(online)),
                _ => None,
            },
            Ok(Message::Ping(_)) => {
                let _ = socket.flush();
                None
            }
            Ok(Message::Close(_)) => return Err("closed".into()),
            Ok(_) => None,
            Err(why) if waiting(&why) => None,
            Err(why) => return Err(why.to_string()),
        };
        if let Some(told) = told {
            if !heard(told) {
                let _ = socket.close(None);
                return Ok(());
            }
        }
    }
}

pub fn link() -> (Link, Receiver<()>) {
    let (stop, stopping) = std::sync::mpsc::channel();
    (Link { stop }, stopping)
}

pub fn run(account: &Account, stopping: &Receiver<()>, heard: &mut dyn FnMut(Heard) -> bool) {
    let mut pause = FIRST_RETRY;
    loop {
        let started = Instant::now();
        match session(account, stopping, heard) {
            Ok(()) => return,
            Err(why) => {
                if !heard(Heard::Lost(why)) {
                    return;
                }
            }
        }
        if started.elapsed() > STEADY_AFTER {
            pause = FIRST_RETRY;
        }
        if !matches!(stopping.recv_timeout(pause), Err(RecvTimeoutError::Timeout)) {
            return;
        }
        pause = (pause * 2).min(LONGEST_RETRY);
    }
}

pub fn connect(account: Account, mut heard: impl FnMut(Heard) -> bool + Send + 'static) -> Link {
    let (link, stopping) = link();
    std::thread::spawn(move || run(&account, &stopping, &mut heard));
    link
}

pub fn duel_poll(busy: bool, pushed: bool, here: bool) -> Duration {
    Duration::from_secs(match (busy, pushed, here) {
        (true, false, _) => 2,
        (true, true, _) => 6,
        (false, false, true) => 10,
        (false, false, false) => 15,
        (false, true, _) => 60,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn the_server_s_messages_are_read_and_a_message_missing_its_value_is_an_error() {
        assert_eq!(said_of(r#"{"type":"hello","topics":["duels","live","online"],"online":3}"#), Ok(Said::Hello(3)));
        assert_eq!(said_of(r#"{"type":"online","online":0}"#), Ok(Said::Online(0)));
        assert_eq!(said_of(r#"{"type":"changed","topic":"duels"}"#), Ok(Said::Changed(Some(Topic::Duels))));
        assert_eq!(said_of(r#"{"type":"changed","topic":"live"}"#), Ok(Said::Changed(Some(Topic::Live))));
        assert_eq!(said_of(r#"{"type":"changed","topic":"news"}"#), Ok(Said::Changed(None)), "a topic of a later server is passed over");
        assert_eq!(said_of(r#"{"type":"later"}"#), Ok(Said::Other));
        assert!(said_of(r#"{"type":"hello"}"#).is_err(), "a greeting without the count is never read as zero people");
        assert!(said_of(r#"{"type":"changed"}"#).is_err());
        assert!(said_of("[]").is_err());
    }

    #[test]
    fn polling_slows_down_only_while_the_server_pushes_changes() {
        assert_eq!(duel_poll(true, false, true), Duration::from_secs(2));
        assert_eq!(duel_poll(true, true, false), Duration::from_secs(6));
        assert_eq!(duel_poll(false, false, true), Duration::from_secs(10));
        assert_eq!(duel_poll(false, false, false), Duration::from_secs(15));
        assert_eq!(duel_poll(false, true, true), Duration::from_secs(60));
    }

    fn served(script: impl FnOnce(&mut tungstenite::WebSocket<std::net::TcpStream>, String) + Send + 'static) -> (Account, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut asked = String::new();
            let mut socket = tungstenite::accept_hdr(stream, |request: &tungstenite::handshake::server::Request, response| {
                asked = format!("{} {}", request.uri().path(), request.headers().get("Authorization").and_then(|value| value.to_str().ok()).unwrap_or_default());
                Ok(response)
            }).unwrap();
            script(&mut socket, asked);
        });
        (Account { server: format!("http://127.0.0.1:{port}"), token: "secret".into(), device: "Mac".into() }, server)
    }

    #[test]
    fn a_link_greets_passes_changes_on_and_says_bye_when_dropped() {
        let (account, server) = served(|socket, asked| {
            assert_eq!(asked, "/render/events Bearer secret");
            socket.send(Message::text(r#"{"type":"changed","topic":"duels"}"#)).unwrap();
            socket.send(Message::text(r#"{"type":"hello","topics":["duels"],"online":2}"#)).unwrap();
            socket.send(Message::text(r#"{"type":"changed","topic":"live"}"#)).unwrap();
            socket.send(Message::text(r#"{"type":"changed","topic":"news"}"#)).unwrap();
            socket.send(Message::text(r#"{"type":"online","online":5}"#)).unwrap();
            loop {
                match socket.read() {
                    Ok(Message::Text(text)) => {
                        assert_eq!(text.as_str(), "bye");
                        return;
                    }
                    Ok(_) => {}
                    Err(why) => panic!("the link went away without a word: {why}"),
                }
            }
        });
        let (told, hearing) = std::sync::mpsc::channel();
        let link = connect(account, move |heard| told.send(heard).is_ok());
        let patience = Duration::from_secs(5);
        assert_eq!(hearing.recv_timeout(patience), Ok(Heard::Connected(2)), "a change before the greeting is not passed on");
        assert_eq!(hearing.recv_timeout(patience), Ok(Heard::Changed(Topic::Live)));
        assert_eq!(hearing.recv_timeout(patience), Ok(Heard::Online(5)));
        drop(link);
        server.join().unwrap();
    }

    #[test]
    fn a_silent_server_is_reported_as_lost() {
        let (account, server) = served(|socket, _| {
            socket.send(Message::text(r#"{"type":"hello","topics":[],"online":1}"#)).unwrap();
            std::thread::sleep(Duration::from_millis(2500));
        });
        let (told, hearing) = std::sync::mpsc::channel();
        let _link = connect(account, move |heard| told.send(heard).is_ok());
        assert_eq!(hearing.recv_timeout(Duration::from_secs(5)), Ok(Heard::Connected(1)));
        assert!(matches!(hearing.recv_timeout(Duration::from_secs(5)), Ok(Heard::Lost(_))));
        server.join().unwrap();
    }

    #[test]
    fn an_unreadable_message_ends_the_session_instead_of_being_guessed() {
        let (account, server) = served(|socket, _| {
            socket.send(Message::text(r#"{"type":"hello"}"#)).unwrap();
            std::thread::sleep(Duration::from_millis(300));
        });
        let (told, hearing) = std::sync::mpsc::channel();
        let _link = connect(account, move |heard| told.send(heard).is_ok());
        assert!(matches!(hearing.recv_timeout(Duration::from_secs(5)), Ok(Heard::Lost(why)) if why.contains("online")));
        server.join().unwrap();
    }
}
