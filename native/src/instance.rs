use std::hash::{BuildHasher, RandomState};
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use iced::futures::channel::mpsc::Sender;

const KNOCK: Duration = Duration::from_millis(300);
const ANSWER: Duration = Duration::from_secs(2);
const LEAVING: Duration = Duration::from_secs(6);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Said {
    Show,
    Quit,
}

impl Said {
    fn word(self) -> &'static str {
        match self {
            Said::Show => "show",
            Said::Quit => "quit",
        }
    }

    fn read(word: &str) -> Option<Said> {
        match word {
            "show" => Some(Said::Show),
            "quit" => Some(Said::Quit),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Card {
    port: u16,
    key: String,
    version: String,
    program: String,
}

pub struct Guard {
    file: PathBuf,
    card: Card,
    closed: Arc<AtomicBool>,
}

impl Drop for Guard {
    fn drop(&mut self) {
        self.closed.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect_timeout(&at(self.card.port), KNOCK);
        if held(&self.file).is_some_and(|card| card.key == self.card.key) {
            let _ = std::fs::remove_file(&self.file);
        }
    }
}

pub enum Claim {
    Ours(Guard),
    Theirs,
}

fn at(port: u16) -> SocketAddr {
    SocketAddr::from((Ipv4Addr::LOCALHOST, port))
}

fn held(file: &Path) -> Option<Card> {
    serde_json::from_str(&std::fs::read_to_string(file).ok()?).ok()
}

fn key() -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_nanos());
    format!("{:016x}{:016x}", RandomState::new().hash_one(std::process::id()), RandomState::new().hash_one(nanos))
}

fn ask(card: &Card, said: Said) -> bool {
    let Ok(mut stream) = TcpStream::connect_timeout(&at(card.port), KNOCK) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(ANSWER));
    let _ = stream.set_write_timeout(Some(ANSWER));
    if writeln!(stream, "{} {}", card.key, said.word()).is_err() {
        return false;
    }
    let mut answer = String::new();
    BufReader::new(stream).read_line(&mut answer).is_ok() && answer.trim() == "ok"
}

fn gone(card: &Card, within: Duration) -> bool {
    let until = Instant::now() + within;
    loop {
        if TcpStream::connect_timeout(&at(card.port), KNOCK).is_err() {
            return true;
        }
        if Instant::now() >= until {
            return false;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
}

fn serve(listener: TcpListener, key: String, closed: Arc<AtomicBool>, tell: Box<dyn Fn(Said) + Send>) {
    for stream in listener.incoming() {
        if closed.load(Ordering::SeqCst) {
            return;
        }
        let Ok(mut stream) = stream else {
            continue;
        };
        let _ = stream.set_read_timeout(Some(ANSWER));
        let _ = stream.set_write_timeout(Some(ANSWER));
        let mut line = String::new();
        let Ok(reader) = stream.try_clone() else {
            continue;
        };
        if BufReader::new(reader).read_line(&mut line).is_err() {
            continue;
        }
        let Some(said) = line.trim().split_once(' ').filter(|(given, _)| *given == key).and_then(|(_, word)| Said::read(word)) else {
            continue;
        };
        tell(said);
        let _ = writeln!(stream, "ok");
    }
}

pub fn claim_at(file: &Path, version: &str, program: &str, leaving: Duration, tell: Box<dyn Fn(Said) + Send>) -> Claim {
    if let Some(card) = held(file) {
        let same = card.version == version && card.program == program;
        if ask(&card, if same { Said::Show } else { Said::Quit }) && (same || !gone(&card, leaving)) {
            return Claim::Theirs;
        }
    }
    let Ok(listener) = TcpListener::bind(at(0)) else {
        return Claim::Ours(Guard { file: file.to_path_buf(), card: Card { port: 0, key: String::new(), version: version.to_owned(), program: program.to_owned() }, closed: Arc::new(AtomicBool::new(true)) });
    };
    let card = Card { port: listener.local_addr().map_or(0, |addr| addr.port()), key: key(), version: version.to_owned(), program: program.to_owned() };
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(file, serde_json::to_string(&card).unwrap_or_default());
    std::thread::sleep(Duration::from_millis(60));
    if let Some(other) = held(file).filter(|other| other.key != card.key) {
        if ask(&other, Said::Show) {
            return Claim::Theirs;
        }
        let _ = std::fs::write(file, serde_json::to_string(&card).unwrap_or_default());
    }
    let closed = Arc::new(AtomicBool::new(false));
    let (heard, wanted) = (closed.clone(), card.key.clone());
    std::thread::spawn(move || serve(listener, wanted, heard, tell));
    Claim::Ours(Guard { file: file.to_path_buf(), card, closed })
}

static OUT: Mutex<Option<Sender<Said>>> = Mutex::new(None);
static WAITING: Mutex<Option<Said>> = Mutex::new(None);

fn tell(said: Said) {
    if let Some(out) = OUT.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).as_mut() {
        if out.try_send(said).is_ok() {
            return;
        }
    }
    *WAITING.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(said);
}

pub fn file() -> PathBuf {
    crate::sources::home().join(".dossier").join("instance.json")
}

pub fn claim() -> Claim {
    let program = std::env::current_exe().map(|path| path.to_string_lossy().into_owned()).unwrap_or_default();
    claim_at(&file(), crate::bot::BUILD, &program, LEAVING, Box::new(tell))
}

pub fn events() -> impl iced::futures::Stream<Item = Said> {
    iced::stream::channel(8, async |mut out: Sender<Said>| {
        if let Some(said) = WAITING.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take() {
            let _ = out.try_send(said);
        }
        *OUT.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(out);
        std::future::pending::<()>().await;
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn room(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dossier-instance-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("instance.json")
    }

    fn listening(file: &Path, version: &str, program: &str) -> (Claim, mpsc::Receiver<Said>) {
        let (sender, receiver) = mpsc::channel();
        let sender = Mutex::new(sender);
        let claim = claim_at(file, version, program, Duration::from_millis(1500), Box::new(move |said| {
            let _ = sender.lock().unwrap().send(said);
        }));
        (claim, receiver)
    }

    #[test]
    fn a_second_start_of_the_same_program_shows_the_first_and_does_not_open() {
        let file = room("same");
        let (first, heard) = listening(&file, "0.95.0", "C:/Dossier/dossier.exe");
        assert!(matches!(first, Claim::Ours(_)));
        let (second, _) = listening(&file, "0.95.0", "C:/Dossier/dossier.exe");
        assert!(matches!(second, Claim::Theirs), "the second one must not open");
        assert_eq!(heard.recv_timeout(Duration::from_secs(2)), Ok(Said::Show));
        let (third, _) = listening(&file, "0.95.0", "C:/Dossier/dossier.exe");
        assert!(matches!(third, Claim::Theirs));
        assert_eq!(heard.recv_timeout(Duration::from_secs(2)), Ok(Said::Show));
        drop(first);
        assert!(held(&file).is_none(), "the one that leaves takes its card away");
    }

    #[test]
    fn another_version_or_another_copy_asks_the_running_one_to_leave_and_takes_its_place() {
        for (name, version, program) in [("version", "0.96.0", "C:/Dossier/dossier.exe"), ("copy", "0.95.0", "D:/New/dossier.exe")] {
            let file = room(name);
            let (first, heard) = listening(&file, "0.95.0", "C:/Dossier/dossier.exe");
            let leaving = std::thread::spawn(move || {
                let said = heard.recv_timeout(Duration::from_secs(3));
                drop(first);
                said
            });
            let (second, _) = listening(&file, version, program);
            assert_eq!(leaving.join().unwrap(), Ok(Said::Quit), "{name}");
            let Claim::Ours(guard) = second else {
                panic!("{name}: the new one opens once the old one has left");
            };
            assert_eq!(held(&file).map(|card| (card.version, card.program)), Some((version.to_owned(), program.to_owned())));
            drop(guard);
        }
    }

    #[test]
    fn a_running_one_that_will_not_leave_keeps_the_new_one_out() {
        let file = room("stays");
        let (first, heard) = listening(&file, "0.95.0", "C:/Dossier/dossier.exe");
        let (second, _) = listening(&file, "0.96.0", "C:/Dossier/dossier.exe");
        assert_eq!(heard.recv_timeout(Duration::from_secs(2)), Ok(Said::Quit));
        assert!(matches!(second, Claim::Theirs), "two are never open together");
        drop(first);
    }

    #[test]
    fn a_card_left_by_a_program_that_is_gone_does_not_stop_a_start() {
        let file = room("stale");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        let free = TcpListener::bind(at(0)).unwrap().local_addr().unwrap().port();
        std::fs::write(&file, serde_json::to_string(&Card { port: free, key: "old".into(), version: "0.95.0".into(), program: "C:/Dossier/dossier.exe".into() }).unwrap()).unwrap();
        let (claim, _) = listening(&file, "0.95.0", "C:/Dossier/dossier.exe");
        assert!(matches!(claim, Claim::Ours(_)));
        std::fs::write(&file, "not a card").unwrap();
        drop(claim);
        let (claim, _) = listening(&file, "0.95.0", "C:/Dossier/dossier.exe");
        assert!(matches!(claim, Claim::Ours(_)));
    }

    #[test]
    fn a_stranger_without_the_key_is_not_obeyed() {
        let file = room("stranger");
        let (claim, heard) = listening(&file, "0.95.0", "C:/Dossier/dossier.exe");
        let card = held(&file).unwrap();
        assert!(!ask(&Card { key: "guess".into(), ..card.clone() }, Said::Quit));
        assert!(heard.recv_timeout(Duration::from_millis(300)).is_err());
        assert!(ask(&card, Said::Show));
        assert_eq!(heard.recv_timeout(Duration::from_secs(2)), Ok(Said::Show));
        drop(claim);
    }
}
