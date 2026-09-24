use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    Stable,
    Lazer,
    Folder,
    Own,
    Found,
}

impl Kind {
    pub fn tag(self) -> &'static str {
        match self {
            Kind::Stable => "stable",
            Kind::Lazer => "lazer",
            Kind::Folder => "folder",
            Kind::Own => "dossier",
            Kind::Found => "found",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Source {
    pub kind: Kind,
    pub root: PathBuf,
    pub songs: Option<PathBuf>,
    pub skins: Option<PathBuf>,
    pub replays: Option<PathBuf>,
    pub maps: Option<u64>,
    pub skin_count: u64,
    pub replay_count: u64,
    pub on: bool,
}

impl Source {
    pub fn shown(&self) -> String {
        shortened(&self.root)
    }
}

pub fn shortened(path: &Path) -> String {
    let home = home();
    let shown = path.display().to_string();
    match home.to_str() {
        Some(prefix) if shown.starts_with(prefix) => format!("~{}", &shown[prefix.len()..]),
        _ => shown,
    }
}

pub fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn count_dirs(dir: &Path) -> u64 {
    std::fs::read_dir(dir)
        .map(|entries| entries.flatten().filter(|e| e.file_type().is_ok_and(|kind| kind.is_dir())).count() as u64)
        .unwrap_or(0)
}

fn count_files(dir: &Path, extension: &str) -> u64 {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    e.path()
                        .extension()
                        .and_then(|x| x.to_str())
                        .map(|x| x.eq_ignore_ascii_case(extension))
                        .unwrap_or(false)
                })
                .count() as u64
        })
        .unwrap_or(0)
}

fn stable_config(root: &Path) -> Option<String> {
    let entries = std::fs::read_dir(root).ok()?;
    let mut best: Option<(u64, PathBuf)> = None;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !(name.starts_with("osu!.") && name.ends_with(".cfg")) {
            continue;
        }
        let stamp = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if best.as_ref().map_or(true, |(when, _)| stamp >= *when) {
            best = Some((stamp, entry.path()));
        }
    }
    best.and_then(|(_, path)| std::fs::read_to_string(path).ok())
}

pub fn beatmap_directory(config: &str) -> Option<String> {
    config.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        (key.trim() == "BeatmapDirectory").then(|| value.trim().to_owned())
    })
}

fn holds_beatmaps(songs: &Path) -> bool {
    std::fs::read_dir(songs)
        .map(|entries| {
            entries.flatten().take(64).any(|set| {
                std::fs::read_dir(set.path())
                    .map(|inner| {
                        inner.flatten().any(|f| {
                            f.path()
                                .extension()
                                .and_then(|x| x.to_str())
                                .map(|x| x.eq_ignore_ascii_case("osu"))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

pub fn stable_at(root: &Path) -> Option<Source> {
    let evidence = root.join("osu!.exe").is_file() || stable_config(root).is_some();
    if !evidence && !holds_beatmaps(&root.join("Songs")) {
        return None;
    }
    let songs = stable_config(root)
        .and_then(|config| beatmap_directory(&config))
        .map(|dir| {
            let path = PathBuf::from(&dir);
            if path.is_absolute() {
                path
            } else {
                root.join(dir)
            }
        })
        .filter(|path| path.is_dir())
        .unwrap_or_else(|| root.join("Songs"));
    let skins = root.join("Skins");
    let replays = root.join("Replays");
    Some(Source {
        kind: Kind::Stable,
        maps: Some(count_dirs(&songs)),
        skin_count: count_dirs(&skins),
        replay_count: count_files(&replays, "osr"),
        songs: Some(songs),
        skins: skins.is_dir().then_some(skins),
        replays: replays.is_dir().then_some(replays),
        root: root.to_path_buf(),
        on: true,
    })
}

pub fn storage_redirect(ini: &str) -> Option<PathBuf> {
    ini.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        (key.trim() == "FullPath").then(|| PathBuf::from(value.trim()))
    })
}

pub fn is_osr(head: &[u8]) -> bool {
    head.len() >= 39
        && head[0] <= 3
        && head[5] == 0x0b
        && head[6] == 0x20
        && head[7..39].iter().all(|b| b.is_ascii_hexdigit())
        && {
            let version = i32::from_le_bytes([head[1], head[2], head[3], head[4]]);
            (2007_00_00..=2099_12_31).contains(&version)
        }
}

const SMALLEST_REPLAY: u64 = 100;
const BIGGEST_REPLAY: u64 = 64 * 1024 * 1024;

static STORE: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn key(name: &str) -> u64 {
    name.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| (hash ^ byte as u64).wrapping_mul(0x0100_0000_01b3))
}

fn store_memory(files: &Path) -> PathBuf {
    own_root().join("cache").join(format!("store-{:016x}.bin", key(&files.to_string_lossy())))
}

fn recall(at: &Path) -> Option<(std::collections::HashSet<u64>, std::collections::HashSet<u64>)> {
    let bytes = std::fs::read(at).ok()?;
    let words: Vec<u64> = bytes.chunks_exact(8).map(|chunk| u64::from_le_bytes(chunk.try_into().unwrap_or([0; 8]))).collect();
    let (&count, rest) = words.split_first()?;
    let count = (count as usize).min(rest.len());
    Some((rest[..count].iter().copied().collect(), rest[count..].iter().copied().collect()))
}

fn remember(at: &Path, replays: &[u64], others: &[u64]) {
    let mut bytes = Vec::with_capacity(8 * (1 + replays.len() + others.len()));
    bytes.extend((replays.len() as u64).to_le_bytes());
    for word in replays.iter().chain(others) {
        bytes.extend(word.to_le_bytes());
    }
    if let Some(dir) = at.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let fresh = at.with_extension("new");
    if std::fs::write(&fresh, bytes).is_ok() {
        let _ = std::fs::rename(&fresh, at);
    }
}

fn looks_like_replay(path: &Path) -> Option<bool> {
    use std::io::Read;
    let file = std::fs::File::open(path).ok()?;
    let mut head = Vec::with_capacity(40);
    file.take(40).read_to_end(&mut head).ok()?;
    Some(is_osr(&head))
}

fn judge(unknown: Vec<(PathBuf, u64)>) -> Vec<(PathBuf, u64, Option<bool>)> {
    if unknown.is_empty() {
        return Vec::new();
    }
    let lanes = std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(2, 8);
    let share = unknown.len().div_ceil(lanes);
    std::thread::scope(|scope| {
        let lanes: Vec<_> = unknown
            .chunks(share)
            .map(|chunk| scope.spawn(move || chunk.iter().map(|(path, word)| (path.clone(), *word, looks_like_replay(path))).collect::<Vec<_>>()))
            .collect();
        lanes.into_iter().flat_map(|lane| lane.join().unwrap_or_default()).collect()
    })
}

pub fn store_known(files: &Path) -> Option<u64> {
    recall(&store_memory(files)).map(|(replays, _)| replays.len() as u64)
}

pub fn store_replays(files: &Path) -> Vec<PathBuf> {
    store_replays_minding(files, &store_memory(files))
}

fn store_replays_minding(files: &Path, memory: &Path) -> Vec<PathBuf> {
    let _walking = STORE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let (replays, others) = recall(memory).unwrap_or_default();
    let mut found = Vec::new();
    let mut unknown = Vec::new();
    let (mut kept_replays, mut kept_others) = (Vec::new(), Vec::new());
    let mut stack = vec![files.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                stack.push(entry.path());
                continue;
            }
            if !kind.is_file() {
                continue;
            }
            let word = key(&entry.file_name().to_string_lossy());
            if replays.contains(&word) {
                found.push(entry.path());
                kept_replays.push(word);
            } else if others.contains(&word) {
                kept_others.push(word);
            } else if entry.metadata().is_ok_and(|m| (SMALLEST_REPLAY..=BIGGEST_REPLAY).contains(&m.len())) {
                unknown.push((entry.path(), word));
            } else {
                kept_others.push(word);
            }
        }
    }
    for (path, word, verdict) in judge(unknown) {
        match verdict {
            Some(true) => {
                found.push(path);
                kept_replays.push(word);
            }
            Some(false) => kept_others.push(word),
            None => {}
        }
    }
    remember(memory, &kept_replays, &kept_others);
    found
}

pub fn counted(source: &Source) -> Source {
    let mut out = source.clone();
    if source.kind == Kind::Lazer {
        out.replay_count = count_files(&source.root.join("exports"), "osr") + store_replays(&source.root.join("files")).len() as u64;
    }
    out
}

pub fn lazer_at(root: &Path) -> Option<Source> {
    let root = std::fs::read_to_string(root.join("storage.ini"))
        .ok()
        .and_then(|ini| storage_redirect(&ini))
        .filter(|redirect| redirect.is_dir())
        .unwrap_or_else(|| root.to_path_buf());
    if !(root.join("client.realm").is_file() && root.join("files").is_dir()) {
        return None;
    }
    let exports = root.join("exports");
    Some(Source {
        kind: Kind::Lazer,
        maps: None,
        skin_count: count_files(&exports, "osk"),
        replay_count: count_files(&exports, "osr") + store_known(&root.join("files")).unwrap_or(0),
        songs: None,
        skins: exports.is_dir().then_some(exports.clone()),
        replays: exports.is_dir().then_some(exports),
        root,
        on: true,
    })
}

pub fn folder_at(root: &Path) -> Option<Source> {
    let replays = count_files(root, "osr");
    let songs = ["Songs", "Beatmap", "Beatmaps"]
        .iter()
        .map(|name| root.join(name))
        .find(|dir| dir.is_dir());
    (replays > 0).then(|| Source {
        kind: Kind::Folder,
        maps: songs.as_deref().map(count_dirs),
        skin_count: 0,
        replay_count: replays,
        songs,
        skins: None,
        replays: Some(root.to_path_buf()),
        root: root.to_path_buf(),
        on: true,
    })
}

pub fn own_root() -> PathBuf {
    home().join(".dossier")
}

pub fn own() -> Result<Source, String> {
    let root = own_root();
    let songs = root.join("Songs");
    let skins = root.join("Skins");
    let replays = root.join("Replays");
    for dir in [&songs, &skins, &replays] {
        std::fs::create_dir_all(dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    }
    Ok(Source {
        kind: Kind::Own,
        maps: Some(count_dirs(&songs)),
        skin_count: count_dirs(&skins),
        replay_count: count_files(&replays, "osr"),
        songs: Some(songs),
        skins: Some(skins),
        replays: Some(replays),
        root,
        on: true,
    })
}

pub fn read(root: &Path) -> Option<Source> {
    stable_at(root).or_else(|| lazer_at(root)).or_else(|| folder_at(root))
}

fn stable_roots() -> Vec<PathBuf> {
    let home = home();
    let mut out = vec![
        home.join("osu!"),
        home.join("osu"),
        home.join("Games").join("osu!"),
        home.join(".local").join("share").join("osu-stable"),
    ];
    for key in ["LOCALAPPDATA", "PROGRAMFILES", "PROGRAMFILES(X86)"] {
        if let Some(base) = std::env::var_os(key) {
            out.push(PathBuf::from(base).join("osu!"));
        }
    }
    for wine in [".wine", ".local/share/wineprefixes/osu"] {
        let users = home.join(wine).join("drive_c").join("users");
        if let Ok(names) = std::fs::read_dir(&users) {
            for name in names.flatten() {
                out.push(name.path().join("AppData").join("Local").join("osu!"));
            }
        }
    }
    out
}

fn lazer_roots() -> Vec<PathBuf> {
    let home = home();
    let mut out = vec![home.join(".local").join("share").join("osu")];
    if let Some(appdata) = std::env::var_os("APPDATA") {
        out.push(PathBuf::from(appdata).join("osu"));
    }
    out
}

pub fn find() -> Vec<Source> {
    let mut found = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for root in stable_roots() {
        if let Some(source) = stable_at(&root) {
            if seen.insert(source.root.clone()) {
                found.push(source);
            }
        }
    }
    for root in lazer_roots() {
        if let Some(source) = lazer_at(&root) {
            if seen.insert(source.root.clone()) {
                found.push(source);
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dossier-native-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_stable_install_is_read_through_its_own_config() {
        let root = scratch("stable");
        std::fs::create_dir_all(root.join("Elsewhere")).unwrap();
        std::fs::create_dir_all(root.join("Elsewhere").join("1 - a")).unwrap();
        std::fs::create_dir_all(root.join("Elsewhere").join("2 - b")).unwrap();
        std::fs::create_dir_all(root.join("Songs")).unwrap();
        std::fs::create_dir_all(root.join("Skins").join("one")).unwrap();
        std::fs::create_dir_all(root.join("Replays")).unwrap();
        std::fs::write(root.join("Replays").join("x.osr"), b"").unwrap();
        std::fs::write(root.join("osu!.naum.cfg"), "Skin = one\nBeatmapDirectory = Elsewhere\n").unwrap();
        let source = stable_at(&root).expect("a stable install");
        assert_eq!(source.kind, Kind::Stable);
        assert_eq!(source.songs, Some(root.join("Elsewhere")));
        assert_eq!(source.maps, Some(2));
        assert_eq!(source.skin_count, 1);
        assert_eq!(source.replay_count, 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    fn replay_head() -> Vec<u8> {
        let mut head = vec![0u8, 0x1d, 0x2d, 0x33, 0x01, 0x0b, 0x20];
        head.extend(b"0123456789abcdef0123456789abcdef");
        head.resize(160, 7);
        head
    }

    fn shelve(files: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let dir = files.join(&name[..1]).join(&name[..2]);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn the_store_is_walked_once_and_then_trusted_by_name() {
        let root = scratch("store");
        let files = root.join("files");
        let memory = root.join("memory.bin");
        assert!(is_osr(&replay_head()));
        let replay = shelve(&files, "a1replay", &replay_head());
        let audio = shelve(&files, "b2audio", &[9u8; 4096]);
        shelve(&files, "c3tiny", b"[General]");
        let first = store_replays_minding(&files, &memory);
        assert_eq!(first, vec![replay.clone()]);
        std::fs::write(&audio, replay_head()).unwrap();
        let later = shelve(&files, "d4later", &replay_head());
        let mut second = store_replays_minding(&files, &memory);
        second.sort();
        assert_eq!(second, vec![replay.clone(), later.clone()], "a file judged once keeps its verdict, a new one is read");
        std::fs::remove_file(&replay).unwrap();
        assert_eq!(store_replays_minding(&files, &memory), vec![later]);
        assert_eq!(recall(&memory).map(|(replays, _)| replays.len()), Some(1), "a file gone from the store leaves the memory");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    #[ignore = "writes a hundred thousand files"]
    fn a_large_store_is_quick_the_second_time() {
        let root = scratch("store-large");
        let files = root.join("files");
        let memory = root.join("memory.bin");
        let head = replay_head();
        for n in 0..100_000u32 {
            let name = format!("{:08x}{n}", n.wrapping_mul(2_654_435_761));
            shelve(&files, &name, if n % 500 == 0 { &head } else { b"[General]\nAudioFilename: a.mp3\n..................................................................................................." });
        }
        let started = std::time::Instant::now();
        let first = store_replays_minding(&files, &memory).len();
        let cold = started.elapsed();
        let started = std::time::Instant::now();
        let second = store_replays_minding(&files, &memory).len();
        let warm = started.elapsed();
        println!("{first} replays, first walk {cold:.2?}, second {warm:.2?}");
        assert_eq!(first, 200);
        assert_eq!(second, 200);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_missing_replays_folder_is_none_rather_than_a_failure() {
        let root = scratch("bare");
        std::fs::create_dir_all(root.join("Songs")).unwrap();
        std::fs::write(root.join("osu!.exe"), b"").unwrap();
        let source = stable_at(&root).expect("a stable install");
        assert_eq!(source.replays, None);
        assert_eq!(source.replay_count, 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn lazer_is_found_at_once_through_its_redirect_and_its_replays_are_found_by_content() {
        let old = scratch("lazer-old");
        let new = scratch("lazer-new");
        std::fs::write(old.join("storage.ini"), format!("FullPath = {}\n", new.display())).unwrap();
        std::fs::write(new.join("client.realm"), b"").unwrap();
        std::fs::create_dir_all(new.join("files").join("ab").join("abc")).unwrap();
        std::fs::create_dir_all(new.join("exports")).unwrap();
        std::fs::write(new.join("exports").join("my.osk"), b"").unwrap();
        let mut replay = vec![0u8, 0x8a, 0x1e, 0x35, 0x01, 0x0b, 0x20];
        replay.extend_from_slice(b"0123456789abcdef0123456789abcdef");
        replay.extend_from_slice(&[0x0b, 0x04]);
        replay.resize(200, 0);
        std::fs::write(new.join("files").join("ab").join("abc").join("abcdef"), &replay).unwrap();
        std::fs::write(new.join("files").join("ab").join("song.mp3"), [b'I'; 300]).unwrap();
        let source = lazer_at(&old).expect("a lazer install");
        assert_eq!(source.kind, Kind::Lazer);
        assert_eq!(source.root, new);
        assert_eq!(source.maps, None);
        assert_eq!(source.skin_count, 1);
        assert_eq!(source.replay_count, 0, "finding does not walk the store");
        let found = store_replays_minding(&new.join("files"), &old.join("memory.bin"));
        assert_eq!(found, vec![new.join("files").join("ab").join("abc").join("abcdef")]);
        let _ = std::fs::remove_dir_all(&old);
        let _ = std::fs::remove_dir_all(&new);
    }

    #[test]
    fn the_replay_signature_is_strict_about_its_shape() {
        let mut good = vec![0u8, 0x8a, 0x1e, 0x35, 0x01, 0x0b, 0x20];
        good.extend_from_slice(b"0123456789abcdef0123456789abcdef");
        assert!(is_osr(&good));
        let mut bad_mode = good.clone();
        bad_mode[0] = 7;
        assert!(!is_osr(&bad_mode));
        let mut bad_hash = good.clone();
        bad_hash[10] = b'z';
        assert!(!is_osr(&bad_hash));
        assert!(!is_osr(&good[..20]));
    }

    #[test]
    fn an_empty_songs_folder_alone_is_not_a_game() {
        let root = scratch("not-a-game");
        std::fs::create_dir_all(root.join("Songs")).unwrap();
        assert!(stable_at(&root).is_none());
        std::fs::create_dir_all(root.join("Songs").join("1 - a")).unwrap();
        assert!(stable_at(&root).is_none());
        std::fs::write(root.join("Songs").join("1 - a").join("a.osu"), b"osu file format v14").unwrap();
        assert_eq!(stable_at(&root).and_then(|s| s.maps), Some(1));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_plain_folder_counts_only_if_it_holds_replays() {
        let root = scratch("folder");
        assert!(folder_at(&root).is_none());
        std::fs::write(root.join("a.OSR"), b"").unwrap();
        assert_eq!(folder_at(&root).map(|s| s.replay_count), Some(1));
        let _ = std::fs::remove_dir_all(&root);
    }
}
