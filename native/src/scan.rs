use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::sources::{self, Kind, Source};

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Looking { files: u64, found: u64, seconds: u64 },
    Done(Vec<PathBuf>),
    Stopped,
}

const SKIPPED: &[&str] = &[
    "Library",
    "Applications",
    "AppData",
    "node_modules",
    "target",
    "Music",
    "Movies",
    "Pictures",
    "Photos",
    "snap",
    "Steam",
    "steamapps",
];

const EVERY: Duration = Duration::from_millis(150);

static STOP: AtomicBool = AtomicBool::new(false);

pub fn stop() {
    STOP.store(true, Ordering::SeqCst);
}

pub fn skipped(name: &str) -> bool {
    (name.starts_with('.') && name != ".dossier") || SKIPPED.iter().any(|s| s.eq_ignore_ascii_case(name))
}

pub fn likely_first(home: &Path) -> Vec<PathBuf> {
    ["Downloads", "Desktop", "Documents", "Загрузки", "Рабочий стол", "Документы"]
        .iter()
        .map(|name| home.join(name))
        .filter(|dir| dir.is_dir())
        .collect()
}

pub fn roots() -> Vec<PathBuf> {
    let home = sources::home();
    let mut out = likely_first(&home);
    out.push(home);
    out
}

fn is_replay(path: &Path) -> bool {
    if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("osr")) {
        return false;
    }
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    use std::io::Read;
    let mut head = [0u8; 40];
    let Ok(n) = file.read(&mut head) else {
        return false;
    };
    sources::is_osr(&head[..n])
}

pub fn found_list() -> PathBuf {
    sources::own_root().join("found.json")
}

pub fn remembered() -> Vec<PathBuf> {
    std::fs::read_to_string(found_list())
        .ok()
        .and_then(|text| serde_json::from_str::<Vec<PathBuf>>(&text).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|p| p.is_file())
        .collect()
}

pub fn remember(paths: &[PathBuf]) -> Result<(), String> {
    let list = found_list();
    if let Some(dir) = list.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(paths).map_err(|e| e.to_string())?;
    std::fs::write(list, text).map_err(|e| e.to_string())
}

pub fn songs_beside(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for path in paths {
        let Some(folder) = path.parent() else {
            continue;
        };
        for near in [Some(folder), folder.parent()].into_iter().flatten() {
            for name in ["Songs", "Beatmap", "Beatmaps"] {
                let dir = near.join(name);
                if dir.is_dir() && !out.contains(&dir) {
                    out.push(dir);
                }
            }
        }
    }
    out
}

pub fn source(paths: &[PathBuf]) -> Source {
    Source {
        kind: Kind::Found,
        root: sources::home(),
        songs: None,
        skins: None,
        replays: None,
        maps: None,
        skin_count: 0,
        replay_count: paths.len() as u64,
        scores: 0,
        on: true,
    }
}

pub fn walk(roots: &[PathBuf], known: &[PathBuf], report: &mut dyn FnMut(Step) -> bool) -> Step {
    STOP.store(false, Ordering::SeqCst);
    let started = Instant::now();
    let mut told = Instant::now() - EVERY;
    let mut files = 0u64;
    let mut found: Vec<PathBuf> = Vec::new();
    let mut seen_dirs: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
    for root in roots {
        let mut stack = vec![root.clone()];
        while let Some(dir) = stack.pop() {
            if STOP.load(Ordering::SeqCst) {
                return Step::Stopped;
            }
            if !seen_dirs.insert(dir.clone()) {
                continue;
            }
            if known.iter().any(|k| dir.starts_with(k)) {
                continue;
            }
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let Ok(kind) = entry.file_type() else {
                    continue;
                };
                if kind.is_symlink() {
                    continue;
                }
                if kind.is_dir() {
                    let name = entry.file_name();
                    if !skipped(&name.to_string_lossy()) {
                        stack.push(path);
                    }
                    continue;
                }
                files += 1;
                if is_replay(&path) {
                    found.push(path);
                }
                if told.elapsed() >= EVERY {
                    told = Instant::now();
                    if !report(Step::Looking { files, found: found.len() as u64, seconds: started.elapsed().as_secs() }) {
                        return Step::Stopped;
                    }
                }
            }
        }
    }
    found.sort();
    found.dedup();
    Step::Done(found)
}

pub fn run(known: Vec<PathBuf>) -> iced::Task<Step> {
    crate::ui::streamed(move |push| {
        let roots = roots();
        let last = walk(&roots, &known, push);
        push(last);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_walk_finds_replays_by_their_signature_and_skips_what_it_should() {
        let dir = std::env::temp_dir().join(format!("dossier-scan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Downloads/Telegram Desktop")).unwrap();
        std::fs::create_dir_all(dir.join("node_modules/x")).unwrap();
        std::fs::create_dir_all(dir.join(".hidden")).unwrap();
        let mut real = vec![0u8; 48];
        real[0] = 0;
        real[1..5].copy_from_slice(&20240101i32.to_le_bytes());
        real[5] = 0x0b;
        real[6] = 0x20;
        for b in &mut real[7..39] {
            *b = b'a';
        }
        std::fs::write(dir.join("Downloads/Telegram Desktop/one.osr"), &real).unwrap();
        std::fs::write(dir.join("Downloads/fake.osr"), b"not a replay at all, just bytes").unwrap();
        std::fs::write(dir.join("node_modules/x/two.osr"), &real).unwrap();
        std::fs::write(dir.join(".hidden/three.osr"), &real).unwrap();
        std::fs::write(dir.join("Downloads/song.mp3"), b"mp3").unwrap();
        let mut steps = Vec::new();
        let last = walk(&[dir.clone()], &[], &mut |s| {
            steps.push(s);
            true
        });
        match last {
            Step::Done(found) => assert_eq!(found, vec![dir.join("Downloads/Telegram Desktop/one.osr")]),
            other => panic!("{other:?}"),
        }
        assert!(skipped("Library") && skipped(".git") && !skipped(".dossier") && !skipped("Downloads"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_folder_already_read_as_a_source_is_not_walked_again() {
        let dir = std::env::temp_dir().join(format!("dossier-scan-known-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("osu/Replays")).unwrap();
        let mut real = vec![0u8; 48];
        real[1..5].copy_from_slice(&20240101i32.to_le_bytes());
        real[5] = 0x0b;
        real[6] = 0x20;
        for b in &mut real[7..39] {
            *b = b'0';
        }
        std::fs::write(dir.join("osu/Replays/one.osr"), &real).unwrap();
        let last = walk(&[dir.clone()], &[dir.join("osu")], &mut |_| true);
        assert_eq!(last, Step::Done(vec![]));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
