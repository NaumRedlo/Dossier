use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::sources::{Kind, Source};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Grade {
    Ss,
    S,
    A,
    B,
    C,
    D,
    F,
}

impl Grade {
    pub fn letter(self) -> &'static str {
        match self {
            Grade::Ss => "SS",
            Grade::S => "S",
            Grade::A => "A",
            Grade::B => "B",
            Grade::C => "C",
            Grade::D => "D",
            Grade::F => "F",
        }
    }

    pub fn of(counts: [u16; 4], failed: bool) -> Grade {
        if failed {
            return Grade::F;
        }
        let [c300, _c100, c50, miss] = counts;
        let total = counts.iter().map(|c| *c as u32).sum::<u32>().max(1) as f64;
        let r300 = c300 as f64 / total;
        let r50 = c50 as f64 / total;
        if c300 as f64 == total {
            Grade::Ss
        } else if r300 > 0.9 && r50 <= 0.01 && miss == 0 {
            Grade::S
        } else if (r300 > 0.8 && miss == 0) || r300 > 0.9 {
            Grade::A
        } else if (r300 > 0.7 && miss == 0) || r300 > 0.8 {
            Grade::B
        } else if r300 > 0.6 {
            Grade::C
        } else {
            Grade::D
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    FullCombo,
    SliderBreak,
    Misses(u16),
    Fail,
}

impl Outcome {
    pub fn mark(self) -> String {
        match self {
            Outcome::FullCombo => "FC".to_owned(),
            Outcome::SliderBreak => "SB".to_owned(),
            Outcome::Misses(n) => format!("×{n}"),
            Outcome::Fail => "F".to_owned(),
        }
    }

    pub fn is_bad(self) -> bool {
        matches!(self, Outcome::Misses(_) | Outcome::Fail)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Map {
    pub file: PathBuf,
    pub artist: String,
    pub title: String,
    pub version: String,
    pub background: Option<PathBuf>,
}

impl Map {
    pub fn folder(&self) -> PathBuf {
        self.file.parent().map(Path::to_path_buf).unwrap_or_default()
    }

    pub fn line(&self) -> String {
        format!("{} — {} [{}]", self.artist, self.title, self.version)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Named {
    pub artist: String,
    pub title: String,
    pub version: String,
}

impl Named {
    pub fn line(&self) -> String {
        format!("{} — {} [{}]", self.artist, self.title, self.version)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Client {
    Stable,
    Lazer,
}

impl Client {
    pub fn tag(self) -> &'static str {
        match self {
            Client::Stable => "stable",
            Client::Lazer => "lazer",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub path: PathBuf,
    pub kind: Kind,
    pub client: Client,
    pub player: String,
    pub replay_hash: String,
    pub map_hash: String,
    pub mods: Vec<String>,
    pub combo: u16,
    pub counts: [u16; 4],
    pub accuracy: f64,
    pub grade: Grade,
    pub outcome: Outcome,
    pub played_at: i64,
    pub named: Option<Named>,
    pub map: Option<Map>,
}

impl Entry {
    pub fn map_line(&self) -> Option<String> {
        self.map.as_ref().map(Map::line).or_else(|| self.named.as_ref().map(Named::line))
    }

    pub fn title(&self) -> Option<String> {
        self.map.as_ref().map(|m| m.title.clone()).or_else(|| self.named.as_ref().map(|n| n.title.clone()))
    }

    pub fn song(&self) -> Option<String> {
        self.map
            .as_ref()
            .map(|m| format!("{} — {}", m.artist, m.title))
            .or_else(|| self.named.as_ref().map(|n| format!("{} — {}", n.artist, n.title)))
    }

    pub fn matches(&self, query: &str) -> bool {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return true;
        }
        let mods: Vec<String> = self.mods.iter().map(|m| m.to_lowercase()).collect();
        let mut hay = vec![self.player.to_lowercase()];
        if let Some(line) = self.map_line() {
            hay.push(line.to_lowercase());
        }
        hay.extend(mods.iter().cloned());
        match self.outcome {
            Outcome::FullCombo => hay.push("fc".to_owned()),
            Outcome::Misses(_) => hay.push("miss".to_owned()),
            _ => {}
        }
        query.split_whitespace().all(|word| match word.strip_prefix('+') {
            Some("nm") => mods.is_empty(),
            Some(wanted) if !wanted.is_empty() => {
                let letters: Vec<char> = wanted.chars().collect();
                letters.len() % 2 == 0 && letters.chunks(2).all(|pair| mods.iter().any(|m| m.chars().eq(pair.iter().copied())))
            }
            _ => hay.iter().any(|h| h.contains(word)),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Library {
    pub entries: Vec<Entry>,
    pub maps: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Reading {
    pub maps: (usize, usize),
    pub replays: (usize, usize),
}

pub fn read(sources: &[Source]) -> Library {
    read_with(sources, &mut |_| {})
}

pub fn read_with(sources: &[Source], report: &mut dyn FnMut(Reading)) -> Library {
    let live: Vec<&Source> = sources.iter().filter(|s| s.on).collect();
    let mut songs: Vec<PathBuf> = live.iter().filter_map(|s| s.songs.clone()).collect();
    if live.iter().any(|s| s.kind == Kind::Found) {
        for dir in crate::scan::songs_beside(&crate::scan::remembered()) {
            if !songs.contains(&dir) {
                songs.push(dir);
            }
        }
    }
    let mut maps = (0, 0);
    let index = Index::load_with(&songs, &mut |done, total| {
        maps = (done, total);
        report(Reading { maps, replays: (0, 0) });
    });
    let mut entries: Vec<Entry> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let files: Vec<(Kind, Vec<PathBuf>)> = live.iter().map(|source| (source.kind, replay_files(source))).collect();
    let scores: Vec<(Kind, Vec<(PathBuf, dossier_replay::Replay)>)> =
        live.iter().filter(|source| source.kind == Kind::Stable).map(|source| (source.kind, crate::scores::with_replays(&source.root))).collect();
    let total = files.iter().map(|(_, list)| list.len()).sum::<usize>() + scores.iter().map(|(_, list)| list.len()).sum::<usize>();
    let mut done = 0;
    let mut told = std::time::Instant::now();
    let mut tell = |done: usize, report: &mut dyn FnMut(Reading)| {
        if done == total || told.elapsed() >= std::time::Duration::from_millis(100) {
            told = std::time::Instant::now();
            report(Reading { maps, replays: (done, total) });
        }
    };
    for (kind, list) in &files {
        for path in list {
            if let Some(entry) = entry(path, *kind, &index) {
                if seen.insert(entry.replay_hash.clone()) {
                    entries.push(entry);
                }
            }
            done += 1;
            tell(done, report);
        }
    }
    for (kind, list) in &scores {
        for (path, replay) in list {
            if let Some(entry) = entry_of(replay, path, *kind, &index) {
                if seen.insert(entry.replay_hash.clone()) {
                    entries.push(entry);
                }
            }
            done += 1;
            tell(done, report);
        }
    }
    entries.sort_by(|a, b| b.played_at.cmp(&a.played_at).then_with(|| a.path.cmp(&b.path)));
    Library { entries, maps: index.by_hash.len() }
}

fn replay_files(source: &Source) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(dir) = &source.replays {
        if let Ok(read) = std::fs::read_dir(dir) {
            out.extend(
                read.flatten()
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("osr"))),
            );
        }
    }
    if source.kind == Kind::Lazer {
        out.extend(crate::sources::store_replays(&source.root.join("files")));
    }
    if source.kind == Kind::Found {
        out.extend(crate::scan::remembered());
    }
    out
}

fn entry(path: &Path, kind: Kind, index: &Index) -> Option<Entry> {
    let bytes = std::fs::read(path).ok()?;
    let replay = dossier_replay::Replay::heading(&bytes).ok()?;
    entry_of(&replay, path, kind, index)
}

fn entry_of(replay: &dossier_replay::Replay, path: &Path, kind: Kind, index: &Index) -> Option<Entry> {
    if replay.mode != dossier_replay::GameMode::Standard {
        return None;
    }
    let counts = [replay.hits.count_300, replay.hits.count_100, replay.hits.count_50, replay.hits.count_miss];
    let failed = failed(&replay.life_bar);
    let outcome = if failed {
        Outcome::Fail
    } else if replay.hits.count_miss > 0 {
        Outcome::Misses(replay.hits.count_miss)
    } else if replay.perfect_combo {
        Outcome::FullCombo
    } else {
        Outcome::SliderBreak
    };
    let mut mods: Vec<String> = replay.mods.acronyms().into_iter().map(str::to_owned).collect();
    if mods.is_empty() {
        mods = replay.lazer_mods().iter().map(|m| m.acronym.clone()).collect();
    }
    let played_at = match replay.played_at_unix() {
        t if t > 0 => t,
        _ => std::fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs() as i64),
    };
    Some(Entry {
        path: path.to_path_buf(),
        kind,
        client: if kind == Kind::Lazer || replay.game_version >= crate::sources::LAZER_VERSIONS || replay.score_info.is_some() { Client::Lazer } else { Client::Stable },
        player: replay.player.clone(),
        replay_hash: if replay.replay_hash.is_empty() { path.display().to_string() } else { replay.replay_hash.clone() },
        map_hash: replay.beatmap_hash.clone(),
        mods,
        combo: replay.max_combo,
        counts,
        accuracy: replay.hits.accuracy_std(),
        grade: Grade::of(counts, failed),
        outcome,
        played_at,
        named: path.file_stem().and_then(|s| named(&s.to_string_lossy())),
        map: index.by_hash.get(&replay.beatmap_hash).cloned(),
    })
}

fn failed(life_bar: &str) -> bool {
    let last = life_bar.trim_end_matches(',').rsplit(',').next().unwrap_or("");
    let value = last.split('|').nth(1).and_then(|v| v.trim().parse::<f32>().ok());
    !life_bar.is_empty() && value.is_some_and(|v| v <= 0.0)
}

pub fn named(stem: &str) -> Option<Named> {
    let stem = stem.trim();
    let before_date = match stem.rfind(" (") {
        Some(at) if stem[at..].contains(')') => &stem[..at],
        _ => stem,
    };
    let (_player, rest) = before_date.split_once(" - ")?;
    let open = rest.rfind(" [")?;
    let version = rest[open + 2..].trim_end_matches(']').trim().to_owned();
    let (artist, title) = rest[..open].split_once(" - ")?;
    if artist.trim().is_empty() || title.trim().is_empty() || version.is_empty() {
        return None;
    }
    Some(Named { artist: artist.trim().to_owned(), title: title.trim().to_owned(), version })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Remembered {
    modified: u64,
    hash: String,
    map: Map,
}

#[derive(Debug, Default)]
pub struct Index {
    pub by_hash: HashMap<String, Map>,
}

fn cache_path() -> PathBuf {
    crate::sources::own_root().join("maps.json")
}

impl Index {
    pub fn load(songs: &[PathBuf]) -> Index {
        Index::load_with(songs, &mut |_, _| {})
    }

    pub fn load_with(songs: &[PathBuf], report: &mut dyn FnMut(usize, usize)) -> Index {
        let mut remembered: HashMap<PathBuf, Remembered> = std::fs::read_to_string(cache_path())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        let mut fresh: HashMap<PathBuf, Remembered> = HashMap::new();
        let mut by_hash = HashMap::new();
        let mut unknown = Vec::new();
        let mut known = 0;
        let found: Vec<(PathBuf, u64)> = songs.iter().flat_map(|root| osu_files(root)).collect();
        let total = found.len();
        report(0, total);
        {
            for (file, modified) in found {
                match remembered.remove(&file) {
                    Some(seen) if seen.modified == modified => {
                        by_hash.insert(seen.hash.clone(), seen.map.clone());
                        fresh.insert(file, seen);
                        known += 1;
                    }
                    _ => unknown.push((file, modified)),
                }
            }
        }
        report(known, total);
        for (file, modified, described) in described_all(unknown, &mut |done| report(known + done, total)) {
            if let Some((hash, map)) = described {
                by_hash.insert(hash.clone(), map.clone());
                fresh.insert(file, Remembered { modified, hash, map });
            }
        }
        if let Ok(text) = serde_json::to_string(&fresh) {
            if let Some(dir) = cache_path().parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(cache_path(), text);
        }
        Index { by_hash }
    }
}

fn osu_files(root: &Path) -> Vec<(PathBuf, u64)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if kind.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("osu")) {
                let modified = entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_secs());
                out.push((path, modified));
            }
        }
    }
    out
}

fn described_all(unknown: Vec<(PathBuf, u64)>, report: &mut dyn FnMut(usize)) -> Vec<(PathBuf, u64, Option<(String, Map)>)> {
    if unknown.is_empty() {
        return Vec::new();
    }
    let lanes = std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(2, 8);
    let share = unknown.len().div_ceil(lanes);
    let done = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let lanes: Vec<_> = unknown
            .chunks(share)
            .map(|chunk| {
                let done = &done;
                scope.spawn(move || {
                    chunk
                        .iter()
                        .map(|(file, modified)| {
                            let described = describe(file);
                            done.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            (file.clone(), *modified, described)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        while lanes.iter().any(|lane| !lane.is_finished()) {
            report(done.load(std::sync::atomic::Ordering::Relaxed));
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        report(done.load(std::sync::atomic::Ordering::Relaxed));
        lanes.into_iter().flat_map(|lane| lane.join().unwrap_or_default()).collect()
    })
}

pub fn describe(file: &Path) -> Option<(String, Map)> {
    let bytes = std::fs::read(file).ok()?;
    let hash = md5_hex(&bytes);
    let text = String::from_utf8_lossy(bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes));
    let (mut artist, mut title, mut version, mut background) = (String::new(), String::new(), String::new(), None);
    let mut section = "";
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = line;
            if section == "[HitObjects]" {
                break;
            }
            continue;
        }
        match section {
            "[Metadata]" => {
                if let Some((key, value)) = line.split_once(':') {
                    match key.trim() {
                        "Artist" if artist.is_empty() => artist = value.trim().to_owned(),
                        "Title" if title.is_empty() => title = value.trim().to_owned(),
                        "Version" => version = value.trim().to_owned(),
                        _ => {}
                    }
                }
            }
            "[Events]" if background.is_none() => {
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() >= 3 && (parts[0].trim() == "0" || parts[0].trim() == "Background") {
                    let name = parts[2].trim().trim_matches('"');
                    if !name.is_empty() {
                        background = file.parent().map(|dir| dir.join(name)).filter(|p| p.is_file());
                    }
                }
            }
            _ => {}
        }
    }
    if title.is_empty() {
        return None;
    }
    Some((hash, Map { file: file.to_path_buf(), artist, title, version, background }))
}

pub fn md5_hex(bytes: &[u8]) -> String {
    use md5::Digest;
    let digest = md5::Md5::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn grades_follow_the_game_s_table() {
        assert_eq!(Grade::of([100, 0, 0, 0], false), Grade::Ss);
        assert_eq!(Grade::of([95, 5, 0, 0], false), Grade::S);
        assert_eq!(Grade::of([95, 3, 2, 0], false), Grade::A);
        assert_eq!(Grade::of([85, 15, 0, 0], false), Grade::A);
        assert_eq!(Grade::of([85, 10, 0, 5], false), Grade::B);
        assert_eq!(Grade::of([65, 35, 0, 0], false), Grade::C);
        assert_eq!(Grade::of([50, 50, 0, 0], false), Grade::D);
        assert_eq!(Grade::of([100, 0, 0, 0], true), Grade::F);
    }

    #[test]
    fn a_stable_file_name_names_its_map() {
        let got = named("-legusshhka- - xi - FREEDOM DiVE [Extra] (2026-08-02) Osu").expect("named");
        assert_eq!(got.artist, "xi");
        assert_eq!(got.title, "FREEDOM DiVE");
        assert_eq!(got.version, "Extra");
        assert!(named("Deeo_XD_15_Voices_Non_breath_oblige_2026").is_none());
        assert!(named("a1b2c3d4").is_none());
    }

    #[test]
    fn a_life_bar_that_ends_at_zero_is_a_fail() {
        assert!(failed("1000|1,2000|0.5,3000|0,"));
        assert!(!failed("1000|1,2000|0.5,"));
        assert!(!failed(""));
    }

    #[test]
    fn a_map_is_described_by_its_header_alone() {
        let dir = std::env::temp_dir().join(format!("dossier-lib-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        std::fs::write(dir.join("bg.jpg"), b"jpg").expect("written");
        let file = dir.join("x.osu");
        std::fs::write(&file, "osu file format v14\n\n[Metadata]\nTitle:Blue Zenith\nArtist:xi\nVersion:FOUR DIMENSIONS\n\n[Events]\n//Background and Video events\n0,0,\"bg.jpg\",0,0\n\n[HitObjects]\n1,2,3\n").expect("written");
        let (hash, map) = describe(&file).expect("described");
        assert_eq!(hash.len(), 32);
        assert_eq!(map.line(), "xi — Blue Zenith [FOUR DIMENSIONS]");
        assert_eq!(map.background, Some(dir.join("bg.jpg")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn search_reads_the_player_the_map_the_mods_and_the_outcome() {
        let entry = Entry {
            path: PathBuf::from("a.osr"),
            kind: Kind::Own,
            client: Client::Stable,
            player: "Guest".into(),
            replay_hash: "r".into(),
            map_hash: String::new(),
            mods: vec!["HD".into(), "DT".into()],
            combo: 1,
            counts: [1, 0, 0, 0],
            accuracy: 1.0,
            grade: Grade::Ss,
            outcome: Outcome::FullCombo,
            played_at: 0,
            named: named("Guest - xi - Blue Zenith [Hard] (2026-08-14) Osu"),
            map: None,
        };
        assert!(entry.matches("zenith"));
        assert!(entry.matches("guest hd"));
        assert!(entry.matches("fc"));
        assert!(!entry.matches("miss"));
        assert!(!entry.matches("freedom"));
        assert!(entry.matches(""));
        assert!(entry.matches("+hddt"));
        assert!(entry.matches("+DT"));
        assert!(entry.matches("guest +hd"));
        assert!(!entry.matches("+hdhr"));
        assert!(!entry.matches("+hdd"));
        assert!(!entry.matches("+nm"));
        let plain = Entry { mods: Vec::new(), ..entry.clone() };
        assert!(plain.matches("+nm"));
    }
}
