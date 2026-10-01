use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::Deserialize;

use crate::bot::Refused;

const PAUSE: Duration = Duration::from_millis(150);

static ALLOWED: AtomicBool = AtomicBool::new(false);
static GIVING: AtomicBool = AtomicBool::new(false);
static SYNCING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct State {
    #[serde(default)]
    pub on: bool,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub count: u32,
    #[serde(default)]
    pub most: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct Listed {
    pub hash: String,
    #[serde(default)]
    pub player: String,
    #[serde(default)]
    pub map_hash: String,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub played_at: Option<i64>,
    #[serde(default)]
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct List {
    #[serde(default)]
    pub replays: Vec<Listed>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Change {
    pub added: usize,
    pub removed: usize,
}

pub struct Play {
    pub path: PathBuf,
    pub hash: String,
    pub artist: String,
    pub title: String,
    pub version: String,
}

pub fn ledger() -> PathBuf {
    crate::sources::home().join(".dossier").join("shared.txt")
}

pub fn allow(on: bool) {
    ALLOWED.store(on, Ordering::SeqCst);
}

pub fn is_shared(path: &Path) -> bool {
    path.starts_with(crate::sources::shared_root())
}

fn keyed(hash: &str) -> bool {
    hash.len() == 32 && hash.bytes().all(|b| b.is_ascii_hexdigit())
}

fn clean(words: &str) -> String {
    words.chars().map(|c| if c.is_control() || "/\\:*?\"<>|".contains(c) { '_' } else { c }).collect::<String>().trim().to_owned()
}

pub fn file_name(listed: &Listed) -> String {
    let player = clean(&listed.player);
    let short: String = match listed.artist.is_empty() || listed.title.is_empty() || listed.version.is_empty() {
        true => player,
        false => format!("{player} - {} - {} [{}]", clean(&listed.artist), clean(&listed.title), clean(&listed.version)),
    };
    format!("{} ({}).osr", short.chars().take(150).collect::<String>(), listed.hash.to_ascii_lowercase())
}

pub fn hash_of(file: &str) -> Option<String> {
    let stem = file.strip_suffix(".osr")?;
    let open = stem.rfind('(')?;
    let hash = stem[open + 1..].strip_suffix(')')?;
    keyed(hash).then(|| hash.to_ascii_lowercase())
}

fn kept(folder: &Path) -> HashMap<String, PathBuf> {
    let Ok(read) = std::fs::read_dir(folder) else {
        return HashMap::new();
    };
    read.flatten().filter_map(|entry| Some((hash_of(entry.file_name().to_str()?)?, entry.path()))).collect()
}

pub fn settle(folder: &Path, listed: &[Listed], mut fetch: impl FnMut(&str) -> Result<Vec<u8>, Refused>) -> Result<Change, Refused> {
    std::fs::create_dir_all(folder).map_err(|e| Refused::Network(e.to_string()))?;
    let mut have = kept(folder);
    let mut change = Change::default();
    for one in listed.iter().filter(|one| keyed(&one.hash)) {
        let hash = one.hash.to_ascii_lowercase();
        if have.remove(&hash).is_some() {
            continue;
        }
        let bytes = match fetch(&hash) {
            Ok(bytes) => bytes,
            Err(Refused::NotThere) => continue,
            Err(why) => return Err(why),
        };
        let path = folder.join(file_name(one));
        let part = path.with_extension("part");
        std::fs::write(&part, bytes).map_err(|e| Refused::Network(e.to_string()))?;
        std::fs::rename(&part, &path).map_err(|e| Refused::Network(e.to_string()))?;
        change.added += 1;
    }
    for path in have.into_values() {
        if std::fs::remove_file(&path).is_ok() {
            change.removed += 1;
        }
    }
    Ok(change)
}

pub fn sync(server: &str, token: &str, name: &str, everyone: bool) -> Result<Change, Refused> {
    if SYNCING.swap(true, Ordering::SeqCst) {
        return Ok(Change::default());
    }
    let outcome = crate::bot::replays_listed(server, token, name, everyone)
        .and_then(|listed| settle(&crate::sources::shared_root(), &listed, |hash| crate::bot::replay_file(server, token, name, hash)));
    SYNCING.store(false, Ordering::SeqCst);
    outcome
}

pub fn give(server: &str, token: &str, name: &str, plays: &[Play], at: &Path, told: impl FnMut(usize)) -> Result<usize, Refused> {
    if GIVING.swap(true, Ordering::SeqCst) {
        return Ok(0);
    }
    let outcome = give_all(
        plays,
        at,
        || ALLOWED.load(Ordering::SeqCst),
        |play, bytes| {
            let meta = serde_json::json!({ "artist": play.artist, "title": play.title, "version": play.version });
            crate::bot::replay_share(server, token, name, bytes, &meta)
        },
        told,
    );
    GIVING.store(false, Ordering::SeqCst);
    outcome
}

fn give_all(
    plays: &[Play],
    at: &Path,
    allowed: impl Fn() -> bool,
    mut send: impl FnMut(&Play, Vec<u8>) -> Result<bool, Refused>,
    mut told: impl FnMut(usize),
) -> Result<usize, Refused> {
    let mut done = crate::donate::given(at);
    let mut sent = 0;
    for play in plays {
        if !allowed() {
            break;
        }
        let Some((hash, bytes)) = crate::donate::waiting(&crate::donate::Play { path: play.path.clone(), hash: play.hash.clone() }, &done) else {
            continue;
        };
        match send(play, bytes) {
            Ok(false) => sent += 1,
            Ok(true) => {}
            Err(Refused::Said(code)) if code == "400" || code == "403" || code == "413" => {}
            Err(refused) => return Err(refused),
        }
        crate::donate::note(at, &hash);
        done.insert(hash);
        told(sent);
        std::thread::sleep(PAUSE);
    }
    Ok(sent)
}

pub fn forget_given() {
    let _ = std::fs::remove_file(ledger());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dossier-mixed-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn listed(hash: &str, player: &str, title: &str) -> Listed {
        Listed { hash: hash.to_owned(), player: player.to_owned(), artist: "xi".into(), title: title.to_owned(), version: "Extra".into(), ..Listed::default() }
    }

    #[test]
    fn a_shared_replay_is_named_so_the_journal_can_read_its_map() {
        let one = listed(&"a".repeat(32), "kotofey", "FREEDOM DiVE");
        let name = file_name(&one);
        assert_eq!(name, format!("kotofey - xi - FREEDOM DiVE [Extra] ({}).osr", "a".repeat(32)));
        assert_eq!(hash_of(&name), Some("a".repeat(32)));
        let named = crate::library::named(name.strip_suffix(".osr").unwrap()).unwrap();
        assert_eq!((named.artist.as_str(), named.title.as_str(), named.version.as_str()), ("xi", "FREEDOM DiVE", "Extra"));
        let bare = Listed { hash: "B".repeat(32), player: "a/b".into(), ..Listed::default() };
        assert_eq!(file_name(&bare), format!("a_b ({}).osr", "b".repeat(32)));
        assert_eq!(hash_of("someone - song [x] (2026-08-03) Osu.osr"), None);
    }

    #[test]
    fn the_folder_follows_the_list_and_fetches_each_replay_once() {
        let dir = folder("settle");
        let (a, b, c) = ("a".repeat(32), "b".repeat(32), "c".repeat(32));
        let mut asked = Vec::new();
        let first = settle(&dir, &[listed(&a, "kotofey", "one"), listed(&b, "lumen", "two")], |hash| {
            asked.push(hash.to_owned());
            Ok(hash.as_bytes().to_vec())
        })
        .unwrap();
        assert_eq!(first, Change { added: 2, removed: 0 });
        let again = settle(&dir, &[listed(&b, "lumen", "two"), listed(&c, "lumen", "three")], |hash| {
            asked.push(hash.to_owned());
            Ok(hash.as_bytes().to_vec())
        })
        .unwrap();
        assert_eq!(again, Change { added: 1, removed: 1 });
        assert_eq!(asked, vec![a.clone(), b.clone(), c.clone()]);
        let mut left: Vec<String> = kept(&dir).into_keys().collect();
        left.sort();
        assert_eq!(left, vec![b, c]);
    }

    #[test]
    fn a_replay_the_server_no_longer_has_is_passed_over_and_a_broken_link_stops_the_sync() {
        let dir = folder("gone");
        let (a, b) = ("a".repeat(32), "b".repeat(32));
        let passed = settle(&dir, &[listed(&a, "kotofey", "one"), listed(&b, "lumen", "two")], |hash| if hash == a { Err(Refused::NotThere) } else { Ok(vec![1]) }).unwrap();
        assert_eq!(passed, Change { added: 1, removed: 0 });
        let stopped = settle(&dir, &[listed(&a, "kotofey", "one")], |_| Err(Refused::Network("offline".into())));
        assert_eq!(stopped, Err(Refused::Network("offline".into())));
        assert!(kept(&dir).contains_key(&b), "a failed sync must not take replays away");
    }

    #[test]
    fn own_plays_go_once_and_stop_when_sharing_is_switched_off() {
        let dir = folder("give");
        let (one, two) = (dir.join("one.osr"), dir.join("two.osr"));
        std::fs::write(&one, b"first replay").unwrap();
        std::fs::write(&two, b"second replay").unwrap();
        let at = dir.join("shared.txt");
        let play = |path: &Path| Play { path: path.to_path_buf(), hash: String::new(), artist: "xi".into(), title: "t".into(), version: "v".into() };
        let plays = vec![play(&one), play(&two)];
        let mut asked = 0;
        assert_eq!(give_all(&plays, &at, || false, |_, _| { asked += 1; Ok(false) }, |_| {}).unwrap(), 0);
        assert_eq!(asked, 0);
        assert_eq!(give_all(&plays, &at, || true, |_, _| { asked += 1; Ok(asked == 2) }, |_| {}).unwrap(), 1);
        assert_eq!(give_all(&plays, &at, || true, |_, _| { asked += 1; Ok(false) }, |_| {}).unwrap(), 0);
        assert_eq!(asked, 2);
    }
}
