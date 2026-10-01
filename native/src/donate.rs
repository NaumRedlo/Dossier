use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::bot::Refused;

const PAUSE: Duration = Duration::from_millis(250);
const LARGEST: u64 = 8 * 1024 * 1024;

static ALLOWED: AtomicBool = AtomicBool::new(false);
static RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    pub sent: usize,
    pub known: usize,
    pub skipped: usize,
}

pub struct Play {
    pub path: PathBuf,
    pub hash: String,
}

pub fn ledger() -> PathBuf {
    crate::sources::home().join(".dossier").join("donated.txt")
}

pub fn allow(on: bool) {
    ALLOWED.store(on, Ordering::SeqCst);
}

fn keyed(hash: &str) -> bool {
    hash.len() == 32 && hash.bytes().all(|b| b.is_ascii_hexdigit())
}

pub fn given(at: &Path) -> HashSet<String> {
    std::fs::read_to_string(at)
        .map(|text| text.lines().map(str::trim).filter(|line| keyed(line)).map(str::to_ascii_lowercase).collect())
        .unwrap_or_default()
}

pub(crate) fn note(at: &Path, hash: &str) {
    if let Some(folder) = at.parent() {
        let _ = std::fs::create_dir_all(folder);
    }
    if let Ok(mut out) = std::fs::OpenOptions::new().create(true).append(true).open(at) {
        let _ = writeln!(out, "{hash}");
    }
}

pub(crate) fn waiting(play: &Play, done: &HashSet<String>) -> Option<(String, Vec<u8>)> {
    let named = keyed(&play.hash).then(|| play.hash.to_ascii_lowercase());
    if named.as_ref().is_some_and(|hash| done.contains(hash)) {
        return None;
    }
    if std::fs::metadata(&play.path).ok()?.len() > LARGEST {
        return None;
    }
    let bytes = std::fs::read(&play.path).ok()?;
    let hash = named.unwrap_or_else(|| crate::library::md5_hex(&bytes));
    (!done.contains(&hash)).then_some((hash, bytes))
}

pub fn give(server: &str, token: &str, name: &str, plays: &[Play], at: &Path, told: impl FnMut(usize)) -> Result<Tally, Refused> {
    if RUNNING.swap(true, Ordering::SeqCst) {
        return Ok(Tally::default());
    }
    let result = give_all(plays, at, || ALLOWED.load(Ordering::SeqCst), |bytes| crate::bot::donate(server, token, name, bytes), told);
    RUNNING.store(false, Ordering::SeqCst);
    result
}

fn give_all(
    plays: &[Play],
    at: &Path,
    allowed: impl Fn() -> bool,
    mut send: impl FnMut(Vec<u8>) -> Result<bool, Refused>,
    mut told: impl FnMut(usize),
) -> Result<Tally, Refused> {
    let mut done = given(at);
    let mut tally = Tally::default();
    for play in plays {
        if !allowed() {
            break;
        }
        let Some((hash, bytes)) = waiting(play, &done) else {
            continue;
        };
        match send(bytes) {
            Ok(true) => tally.known += 1,
            Ok(false) => tally.sent += 1,
            Err(Refused::Said(code)) if code == "400" || code == "413" => tally.skipped += 1,
            Err(refused) => return Err(refused),
        }
        note(at, &hash);
        done.insert(hash);
        told(done.len());
        std::thread::sleep(PAUSE);
    }
    Ok(tally)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dossier-donate-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn play(path: &Path, hash: &str) -> Play {
        Play { path: path.to_path_buf(), hash: hash.to_owned() }
    }

    #[test]
    fn a_replay_goes_once_and_nothing_goes_while_it_is_off() {
        let dir = folder("once");
        let (one, two) = (dir.join("one.osr"), dir.join("two.osr"));
        std::fs::write(&one, b"first replay").unwrap();
        std::fs::write(&two, b"second replay").unwrap();
        let at = dir.join("donated.txt");
        let plays = vec![play(&one, ""), play(&two, ""), play(&one, "")];

        let mut asked = 0;
        assert_eq!(give_all(&plays, &at, || false, |_| { asked += 1; Ok(false) }, |_| {}).unwrap(), Tally::default());
        assert_eq!(asked, 0, "sent while donation was off");

        let mut counted = Vec::new();
        let tally = give_all(&plays, &at, || true, |_| { asked += 1; Ok(false) }, |n| counted.push(n)).unwrap();
        assert_eq!(tally.sent, 2);
        assert_eq!(asked, 2, "the same file went twice");
        assert_eq!(counted, vec![1, 2], "the count did not follow each replay");
        assert_eq!(given(&at).len(), 2);

        let again = give_all(&plays, &at, || true, |_| { asked += 1; Ok(false) }, |_| {}).unwrap();
        assert_eq!(again, Tally::default());
        assert_eq!(asked, 2, "a replay already given went again");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_replay_known_by_its_hash_is_not_read_again() {
        let dir = folder("hash");
        let one = dir.join("one.osr");
        std::fs::write(&one, b"a replay").unwrap();
        let at = dir.join("donated.txt");
        let hash = "0123456789ABCDEF0123456789abcdef";
        let mut asked = 0;
        give_all(&[play(&one, hash)], &at, || true, |_| { asked += 1; Ok(false) }, |_| {}).unwrap();
        assert!(given(&at).contains(&hash.to_ascii_lowercase()), "the ledger did not keep the replay's own hash");

        std::fs::remove_file(&one).unwrap();
        let copy = dir.join("copy.osr");
        std::fs::write(&copy, b"the same play, exported again").unwrap();
        give_all(&[play(&one, hash), play(&copy, hash)], &at, || true, |_| { asked += 1; Ok(false) }, |_| {}).unwrap();
        assert_eq!(asked, 1, "a play already given went again under another file");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_server_that_is_down_is_tried_again_later() {
        let dir = folder("down");
        let one = dir.join("one.osr");
        std::fs::write(&one, b"a replay").unwrap();
        let at = dir.join("donated.txt");
        let failed = give_all(&[play(&one, "")], &at, || true, |_| Err(Refused::Network("down".into())), |_| {});
        assert!(failed.is_err());
        assert!(given(&at).is_empty(), "a replay the server never took was marked as given");
        let refused = give_all(&[play(&one, "")], &at, || true, |_| Err(Refused::Said("400".into())), |_| {}).unwrap();
        assert_eq!(refused.skipped, 1);
        assert_eq!(given(&at).len(), 1, "a replay the server turned away is not offered again");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
