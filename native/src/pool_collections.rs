use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::sources::Source;

const MAX_FILE: u64 = 64 * 1024 * 1024;
const MAX_COLLECTIONS: usize = 10_000;
const MAX_MAPS: usize = 100_000;
const MAX_NAME: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Collection {
    pub name: String,
    pub hashes: Vec<String>,
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self.at.checked_add(count).ok_or("length")?;
        let bytes = self.bytes.get(self.at..end).ok_or("end of file")?;
        self.at = end;
        Ok(bytes)
    }

    fn byte(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn count(&mut self, most: usize) -> Result<usize, String> {
        let bytes: [u8; 4] = self.take(4)?.try_into().map_err(|_| "count")?;
        let count = i32::from_le_bytes(bytes);
        usize::try_from(count).ok().filter(|count| *count <= most).ok_or_else(|| "count".to_owned())
    }

    fn string(&mut self, most: usize) -> Result<String, String> {
        match self.byte()? {
            0 => Ok(String::new()),
            11 => {
                let mut length = 0usize;
                for shift in (0..35).step_by(7) {
                    let byte = self.byte()?;
                    length |= usize::from(byte & 127).checked_shl(shift).ok_or("length")?;
                    if byte & 128 == 0 {
                        if length > most {
                            return Err("length".to_owned());
                        }
                        return std::str::from_utf8(self.take(length)?).map(str::to_owned).map_err(|_| "text".to_owned());
                    }
                }
                Err("length".to_owned())
            }
            _ => Err("string".to_owned()),
        }
    }
}

pub fn parse(bytes: &[u8]) -> Result<Vec<Collection>, String> {
    let mut reader = Reader { bytes, at: 0 };
    reader.take(4)?;
    let count = reader.count(MAX_COLLECTIONS)?;
    let mut collections = Vec::with_capacity(count);
    for _ in 0..count {
        let name = reader.string(MAX_NAME)?;
        let total = reader.count(MAX_MAPS)?;
        let mut hashes = Vec::with_capacity(total);
        let mut seen = HashSet::new();
        for _ in 0..total {
            let hash = reader.string(32)?;
            if hash.len() == 32 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                let hash = hash.to_ascii_lowercase();
                if seen.insert(hash.clone()) {
                    hashes.push(hash);
                }
            }
        }
        collections.push(Collection { name, hashes });
    }
    Ok(collections)
}

fn put_string(out: &mut Vec<u8>, value: &str) {
    if value.is_empty() {
        out.push(0);
        return;
    }
    out.push(11);
    let mut length = value.len();
    loop {
        let byte = (length & 127) as u8;
        length >>= 7;
        if length == 0 {
            out.push(byte);
            break;
        }
        out.push(byte | 128);
    }
    out.extend_from_slice(value.as_bytes());
}

pub fn compose(version: [u8; 4], collections: &[Collection]) -> Vec<u8> {
    let mut out = version.to_vec();
    out.extend_from_slice(&(collections.len() as i32).to_le_bytes());
    for collection in collections {
        put_string(&mut out, &collection.name);
        out.extend_from_slice(&(collection.hashes.len() as i32).to_le_bytes());
        for hash in &collection.hashes {
            put_string(&mut out, hash);
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    NoGame,
    Running,
    Failed(String),
}

impl Refused {
    pub fn code(&self) -> String {
        match self {
            Refused::NoGame => "none".to_owned(),
            Refused::Running => "running".to_owned(),
            Refused::Failed(why) => format!("failed: {why}"),
        }
    }
}

pub fn add(root: &Path, name: &str, hashes: &[String]) -> Result<usize, String> {
    let file = root.join("collection.db");
    if file.metadata().map_err(|why| why.to_string())?.len() > MAX_FILE {
        return Err("collection.db is too large".to_owned());
    }
    let bytes = std::fs::read(&file).map_err(|why| why.to_string())?;
    let version: [u8; 4] = bytes.get(..4).and_then(|head| head.try_into().ok()).ok_or("collection.db is damaged")?;
    let mut collections = parse(&bytes)?;
    let mut seen = HashSet::new();
    let wanted: Vec<String> = hashes.iter().map(|hash| hash.to_ascii_lowercase()).filter(|hash| hash.len() == 32 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()) && seen.insert(hash.clone())).collect();
    if wanted.is_empty() || name.trim().is_empty() || name.len() > MAX_NAME {
        return Err("nothing to write".to_owned());
    }
    match collections.iter_mut().find(|collection| collection.name == name) {
        Some(held) => held.hashes = wanted.clone(),
        None => {
            if collections.len() >= MAX_COLLECTIONS {
                return Err("too many collections".to_owned());
            }
            collections.push(Collection { name: name.to_owned(), hashes: wanted.clone() });
        }
    }
    let composed = compose(version, &collections);
    if parse(&composed)? != collections {
        return Err("the new file does not read back".to_owned());
    }
    let backup = root.join("collection.db.dossier-backup");
    if !backup.exists() {
        std::fs::copy(&file, &backup).map_err(|why| why.to_string())?;
    }
    let staging = root.join(format!("collection.db.{}.part", std::process::id()));
    let written = std::fs::write(&staging, &composed).and_then(|_| std::fs::rename(&staging, &file));
    if written.is_err() {
        let _ = std::fs::remove_file(&staging);
    }
    written.map_err(|why| why.to_string())?;
    Ok(wanted.len())
}

pub fn game_runs(listed: &str) -> bool {
    listed.lines().any(|line| {
        let line = line.to_lowercase();
        line.contains("osu!.exe") && !line.contains("witness")
    })
}

pub fn game_running() -> bool {
    let listed = if cfg!(windows) {
        std::process::Command::new("tasklist").args(["/FI", "IMAGENAME eq osu!.exe", "/NH"]).output()
    } else {
        std::process::Command::new("/bin/ps").args(["-axo", "pid=,command="]).output()
    };
    listed.ok().is_some_and(|out| game_runs(&String::from_utf8_lossy(&out.stdout)))
}

pub fn add_to_game(sources: &[Source], name: &str, hashes: &[String]) -> Result<usize, Refused> {
    let root = roots(sources).into_iter().next().ok_or(Refused::NoGame)?;
    if game_running() {
        return Err(Refused::Running);
    }
    add(&root, name, hashes).map_err(Refused::Failed)
}

fn from_file(path: &Path) -> Option<Vec<Collection>> {
    if path.metadata().ok()?.len() > MAX_FILE {
        return None;
    }
    parse(&std::fs::read(path).ok()?).ok()
}

pub fn roots(sources: &[Source]) -> Vec<PathBuf> {
    roots_with(sources, crate::sources::stable_roots())
}

fn roots_with(sources: &[Source], discovered: Vec<PathBuf>) -> Vec<PathBuf> {
    let normalize = |path: &Path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut candidates = discovered;
    let mut disabled = HashSet::new();
    for source in sources {
        let mut paths = vec![source.root.clone()];
        for path in [Some(&source.root), source.songs.as_ref(), source.replays.as_ref()].into_iter().flatten() {
            if path.file_name().is_some_and(|name| ["songs", "replays"].iter().any(|folder| name.to_string_lossy().eq_ignore_ascii_case(folder))) {
                if let Some(parent) = path.parent() { paths.push(parent.to_path_buf()); }
            }
        }
        if source.on { candidates.extend(paths); } else { disabled.extend(paths.iter().map(|path| normalize(path))); }
    }
    let mut seen = HashSet::new();
    candidates.into_iter().map(|path| normalize(&path)).filter(|root| {
        !disabled.contains(root) && root.join("collection.db").is_file() && seen.insert(root.clone())
    }).collect()
}

pub fn read(sources: &[Source]) -> Vec<Collection> {
    read_roots(roots(sources))
}

fn read_roots(roots: Vec<PathBuf>) -> Vec<Collection> {
    roots.into_iter().filter_map(|root| from_file(&root.join("collection.db"))).flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::Kind;

    fn string(out: &mut Vec<u8>, value: &str) {
        out.push(11);
        let mut length = value.len();
        loop {
            let mut byte = (length & 127) as u8;
            length >>= 7;
            if length > 0 {
                byte |= 128;
            }
            out.push(byte);
            if length == 0 {
                break;
            }
        }
        out.extend(value.as_bytes());
    }

    #[test]
    fn collections_keep_order_and_normalize_hashes() {
        let mut bytes = Vec::new();
        bytes.extend(2024i32.to_le_bytes());
        bytes.extend(1i32.to_le_bytes());
        string(&mut bytes, "Турнир");
        bytes.extend(3i32.to_le_bytes());
        string(&mut bytes, "ABCDEF0123456789ABCDEF0123456789");
        string(&mut bytes, "abcdef0123456789abcdef0123456789");
        string(&mut bytes, "00112233445566778899AABBCCDDEEFF");
        assert_eq!(parse(&bytes).unwrap(), vec![Collection {
            name: "Турнир".to_owned(),
            hashes: vec!["abcdef0123456789abcdef0123456789".to_owned(), "00112233445566778899aabbccddeeff".to_owned()],
        }]);
    }

    #[test]
    fn truncated_and_excessive_counts_are_rejected() {
        let mut bytes = Vec::new();
        bytes.extend(2024i32.to_le_bytes());
        bytes.extend(1i32.to_le_bytes());
        assert!(parse(&bytes).is_err());
        bytes[4..8].copy_from_slice(&i32::MAX.to_le_bytes());
        assert!(parse(&bytes).is_err());
    }

    #[test]
    fn collections_are_found_in_game_and_songs_folders_without_a_running_client() {
        let root = std::env::temp_dir().join(format!("dossier-collections-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let mut bytes = Vec::new();
        bytes.extend(2024i32.to_le_bytes());
        bytes.extend(1i32.to_le_bytes());
        string(&mut bytes, "Favorites");
        bytes.extend(1i32.to_le_bytes());
        string(&mut bytes, "0123456789abcdef0123456789abcdef");
        std::fs::write(root.join("collection.db"), bytes).unwrap();
        let source = Source { kind: Kind::Stable, root: root.clone(), ..crate::sources::shared(true) };
        assert_eq!(read_roots(roots_with(&[source.clone(), source.clone()], vec![root.clone()])).len(), 1);
        assert!(roots_with(&[Source { on: false, ..source.clone() }], vec![root.clone()]).is_empty());
        std::fs::create_dir_all(root.join("Songs")).unwrap();
        let folder = Source { kind: Kind::Folder, root: root.join("Songs"), ..source.clone() };
        assert_eq!(read_roots(roots_with(&[folder], Vec::new())).len(), 1);
        assert_eq!(read_roots(roots_with(&[], vec![root.clone()])).len(), 1);
        assert_eq!(read_roots(roots_with(&[Source { kind: Kind::Folder, ..source }], Vec::new())).len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }
    #[test]
    fn a_pool_is_written_into_the_game_file_beside_what_is_there_and_reads_back() {
        let root = std::env::temp_dir().join(format!("dossier-collection-write-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let (first, second, third) = ("a".repeat(32), "b".repeat(32), "c".repeat(32));
        let held = vec![Collection { name: "Избранное".into(), hashes: vec![first.clone()] }, Collection { name: String::new(), hashes: Vec::new() }];
        let version = 20_150_203_i32.to_le_bytes();
        let original = compose(version, &held);
        assert_eq!(parse(&original).unwrap(), held);
        std::fs::write(root.join("collection.db"), &original).unwrap();
        assert_eq!(add(&root, "Winter Cup", &[second.to_uppercase(), second.clone(), "junk".into(), third.clone()]), Ok(2));
        let bytes = std::fs::read(root.join("collection.db")).unwrap();
        assert_eq!(&bytes[..4], &version, "the version of the file is kept");
        let read = parse(&bytes).unwrap();
        assert_eq!(read.len(), 3);
        assert_eq!(read[..2], held[..], "what was there is untouched");
        assert_eq!(read[2], Collection { name: "Winter Cup".into(), hashes: vec![second.clone(), third.clone()] });
        assert_eq!(std::fs::read(root.join("collection.db.dossier-backup")).unwrap(), original, "the first state is kept aside");
        assert_eq!(add(&root, "Winter Cup", &[first.clone()]), Ok(1));
        let again = parse(&std::fs::read(root.join("collection.db")).unwrap()).unwrap();
        assert_eq!((again.len(), again[2].hashes.clone()), (3, vec![first.clone()]), "the same name is replaced, not doubled");
        assert_eq!(std::fs::read(root.join("collection.db.dossier-backup")).unwrap(), original, "the backup is the state before Dossier touched the file");
        assert!(add(&root, "Empty", &["junk".into()]).is_err() && add(&root, " ", &[first.clone()]).is_err());
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2, "no staging file is left");
        std::fs::write(root.join("collection.db"), b"\x01\x02").unwrap();
        assert!(add(&root, "Winter Cup", &[first]).is_err());
        assert_eq!(std::fs::read(root.join("collection.db")).unwrap(), b"\x01\x02", "a damaged file is left as it is");
        let long = "я".repeat(200);
        assert_eq!(parse(&compose(version, &[Collection { name: long.clone(), hashes: Vec::new() }])).unwrap()[0].name, long);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_running_game_is_told_from_the_process_list() {
        assert!(game_runs("10625 C:\\users\\crossover\\AppData\\Local\\osu!\\osu!.exe \n"));
        assert!(game_runs("osu!.exe                     10625 Console                    1    512,000 K"));
        assert!(!game_runs("  501 /usr/bin/something\n10700 Z:\\tmp\\witness.exe --client osu!.exe\n"));
        assert!(!game_runs("INFO: No tasks are running which match the specified criteria."));
    }

}
