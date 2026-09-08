use dossier_beatmap::Beatmap;
use dossier_replay::Replay;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

use md5::{Digest, Md5};

#[derive(Debug, Clone)]
pub struct FoundMap {
    pub text: String,
    pub source: String,
    pub origin: Origin,
}

#[derive(Debug, Clone)]
pub enum Origin {
    Archive(PathBuf),

    Folder(PathBuf),
}

impl Origin {
    fn of_file(path: &Path) -> Self {
        Self::Folder(path.parent().unwrap_or(Path::new(".")).to_path_buf())
    }
}

pub fn md5_hex(bytes: &[u8]) -> String {
    let digest = Md5::digest(bytes);
    let mut out = String::with_capacity(32);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

pub fn load_map(path: &Path, want_hash: &str) -> Result<FoundMap, String> {
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let name = path.display().to_string();

    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("osz"))
    {
        return match search_osz(&bytes, want_hash) {
            Ok(Some(found)) => Ok(FoundMap {
                text: found.0,
                source: format!("{name} → {}", found.1),
                origin: Origin::Archive(path.to_path_buf()),
            }),
            Ok(None) => Err(format!(
                "{name} contains no difficulty with hash {want_hash}"
            )),
            Err(e) => Err(format!("{name}: {e}")),
        };
    }

    Ok(FoundMap {
        text: decode(&bytes),
        source: name,
        origin: Origin::of_file(path),
    })
}

type Known = std::collections::HashMap<String, PathBuf>;

static KNOWN: std::sync::OnceLock<std::sync::Mutex<Known>> = std::sync::OnceLock::new();

fn known() -> &'static std::sync::Mutex<Known> {
    KNOWN.get_or_init(|| std::sync::Mutex::new(Known::new()))
}

pub fn remember(hash: &str, path: &Path) {
    if let Ok(mut held) = known().lock() {
        held.insert(hash.to_ascii_lowercase(), path.to_path_buf());
    }
}

pub fn remember_all(index: &Known) {
    if let Ok(mut held) = known().lock() {
        for (hash, path) in index {
            held.insert(hash.to_ascii_lowercase(), path.clone());
        }
    }
}

pub fn forget() {
    if let Ok(mut held) = known().lock() {
        held.clear();
    }
}

fn recall(want_hash: &str) -> Option<FoundMap> {
    let path = known().lock().ok()?.get(want_hash)?.clone();
    let named = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("osz"));
    if !named && fs::read(&path).ok().map(|b| md5_hex(&b)).as_deref() != Some(want_hash) {
        return None;
    }
    load_map(&path, want_hash).ok()
}

pub fn search_dir(root: &Path, want_hash: &str) -> Result<Option<FoundMap>, String> {
    let want_hash = &want_hash.to_ascii_lowercase();
    if let Some(found) = recall(want_hash) {
        return Ok(Some(found));
    }
    let mut stack = vec![root.to_path_buf()];
    let mut archives: Vec<PathBuf> = Vec::new();

    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,

            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            match path.extension().and_then(|e| e.to_str()) {
                Some(ext) if ext.eq_ignore_ascii_case("osu") => {
                    let Ok(bytes) = fs::read(&path) else { continue };
                    if md5_hex(&bytes) == *want_hash {
                        remember(want_hash, &path);
                        return Ok(Some(FoundMap {
                            text: decode(&bytes),
                            source: path.display().to_string(),
                            origin: Origin::of_file(&path),
                        }));
                    }
                }
                Some(ext) if ext.eq_ignore_ascii_case("osz") => archives.push(path),
                _ => {}
            }
        }
    }

    for archive in archives {
        let Ok(bytes) = fs::read(&archive) else {
            continue;
        };
        if let Ok(Some((text, inner))) = search_osz(&bytes, want_hash) {
            remember(want_hash, &archive);
            return Ok(Some(FoundMap {
                text,
                source: format!("{} → {inner}", archive.display()),
                origin: Origin::Archive(archive.clone()),
            }));
        }
    }

    Ok(None)
}

pub fn hashes(root: &Path) -> std::collections::HashSet<String> {
    index(root).into_keys().collect()
}

pub fn index(root: &Path) -> Known {
    let mut found = Known::new();
    let mut stack = vec![root.to_path_buf()];
    let mut archives: Vec<PathBuf> = Vec::new();

    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            match path.extension().and_then(|e| e.to_str()) {
                Some(ext) if ext.eq_ignore_ascii_case("osu") => {
                    if let Ok(bytes) = fs::read(&path) {
                        found.insert(md5_hex(&bytes), path.clone());
                    }
                }
                Some(ext) if ext.eq_ignore_ascii_case("osz") => archives.push(path),
                _ => {}
            }
        }
    }

    for archive in archives {
        let Ok(bytes) = fs::read(&archive) else {
            continue;
        };
        let Ok(mut zip) = zip::ZipArchive::new(Cursor::new(bytes)) else {
            continue;
        };
        for index in 0..zip.len() {
            let Ok(mut file) = zip.by_index(index) else {
                continue;
            };
            if !file.name().to_ascii_lowercase().ends_with(".osu") {
                continue;
            }
            let mut inside = Vec::new();
            if file.read_to_end(&mut inside).is_ok() {
                found.insert(md5_hex(&inside), archive.clone());
            }
        }
    }
    found
}

fn search_osz(bytes: &[u8], want_hash: &str) -> Result<Option<(String, String)>, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| format!("not a readable .osz: {e}"))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        if !file.name().to_ascii_lowercase().ends_with(".osu") {
            continue;
        }
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).map_err(|e| e.to_string())?;
        if md5_hex(&contents) == want_hash {
            let name = file.name().to_owned();
            return Ok(Some((decode(&contents), name)));
        }
    }
    Ok(None)
}

fn decode(bytes: &[u8]) -> String {
    let body = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    String::from_utf8_lossy(body).into_owned()
}

pub fn extract_audio(origin: &Origin, filename: &str, into: &Path) -> Option<PathBuf> {
    if filename.trim().is_empty() {
        return None;
    }
    match origin {
        Origin::Folder(folder) => {
            let path = folder.join(filename);
            path.is_file().then_some(path)
        }
        Origin::Archive(archive) => {
            let bytes = fs::read(archive).ok()?;
            let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).ok()?;

            let wanted = normalise(filename);
            let index = (0..zip.len()).find(|&i| {
                zip.by_index(i)
                    .map(|f| normalise(f.name()) == wanted)
                    .unwrap_or(false)
            })?;

            let mut file = zip.by_index(index).ok()?;
            let mut contents = Vec::new();
            file.read_to_end(&mut contents).ok()?;

            let extension = Path::new(filename).extension().and_then(|e| e.to_str());
            let out = into.join(format!("audio.{}", extension.unwrap_or("mp3")));
            fs::write(&out, contents).ok()?;
            Some(out)
        }
    }
}

pub fn extract_video(origin: &Origin, filename: &str, into: &Path) -> Option<PathBuf> {
    if filename.trim().is_empty() {
        return None;
    }
    match origin {
        Origin::Folder(folder) => {
            let path = folder.join(filename);
            path.is_file().then_some(path)
        }
        Origin::Archive(_) => {
            let mut assets = Assets::open(origin);
            let bytes = assets.read(filename)?;
            let extension = Path::new(filename).extension().and_then(|e| e.to_str());
            let out = into.join(format!("video.{}", extension.unwrap_or("mp4")));
            fs::write(&out, bytes).ok()?;
            Some(out)
        }
    }
}

pub fn extract_samples(origin: &Origin, into: &Path, ffmpeg: &str) -> usize {
    let named: Vec<(String, Vec<u8>)> = match origin {
        Origin::Folder(folder) => fs::read_dir(folder)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| is_sample(&entry.file_name().to_string_lossy()))
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                Some((name, fs::read(entry.path()).ok()?))
            })
            .collect(),
        Origin::Archive(archive) => {
            let Ok(bytes) = fs::read(archive) else {
                return 0;
            };
            let Ok(mut zip) = zip::ZipArchive::new(Cursor::new(bytes)) else {
                return 0;
            };
            let mut out = Vec::new();
            for index in 0..zip.len() {
                let Ok(mut file) = zip.by_index(index) else {
                    continue;
                };

                let leaf = Path::new(file.name())
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if !is_sample(&leaf) {
                    continue;
                }
                let mut contents = Vec::new();
                if file.read_to_end(&mut contents).is_ok() {
                    out.push((leaf, contents));
                }
            }
            out
        }
    };

    let mut written = 0;
    for (name, bytes) in named {
        let stem = Path::new(&name).file_stem().unwrap_or_default();
        let target = into.join(stem).with_extension("wav");

        if bytes.is_empty() {
            if fs::write(&target, []).is_ok() {
                written += 1;
            }
            continue;
        }
        let source = into.join(&name);
        if fs::write(&source, &bytes).is_err() {
            continue;
        }
        if source == target {
            written += 1;
            continue;
        }
        let done = Command::new(ffmpeg)
            .args(["-nostdin", "-v", "error", "-y", "-i"])
            .arg(&source)
            .args(["-ac", "2", "-ar", "44100", "-c:a", "pcm_s16le", "-f", "wav"])
            .arg(&target)
            .output();
        let _ = fs::remove_file(&source);
        if done.is_ok_and(|d| d.status.success()) {
            written += 1;
        }
    }
    written
}

fn is_sample(leaf: &str) -> bool {
    let lower = leaf.to_ascii_lowercase();
    let Some(stem) = lower
        .strip_suffix(".wav")
        .or_else(|| lower.strip_suffix(".ogg"))
        .or_else(|| lower.strip_suffix(".mp3"))
    else {
        return false;
    };
    let Some((bank, rest)) = stem.split_once('-') else {
        return false;
    };
    if !matches!(bank, "normal" | "soft" | "drum") {
        return false;
    }
    [
        "hitnormal",
        "hitwhistle",
        "hitfinish",
        "hitclap",
        "slidertick",
        "sliderslide",
        "sliderwhistle",
    ]
    .iter()
    .any(|voice| {
        rest.strip_prefix(voice)
            .is_some_and(|digits| digits.is_empty() || digits.chars().all(|c| c.is_ascii_digit()))
    })
}

pub fn read_background(origin: &Origin, filename: &str) -> Option<Vec<u8>> {
    if filename.trim().is_empty() {
        return None;
    }
    match origin {
        Origin::Folder(folder) => fs::read(folder.join(filename)).ok(),
        Origin::Archive(archive) => {
            let bytes = fs::read(archive).ok()?;
            let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).ok()?;
            let wanted = normalise(filename);
            let index = (0..zip.len()).find(|&i| {
                zip.by_index(i)
                    .map(|f| normalise(f.name()) == wanted)
                    .unwrap_or(false)
            })?;
            let mut file = zip.by_index(index).ok()?;
            let mut out = Vec::new();
            std::io::Read::read_to_end(&mut file, &mut out).ok()?;
            Some(out)
        }
    }
}

fn normalise(name: &str) -> String {
    name.replace('\\', "/")
        .rsplit('/')
        .next()
        .unwrap_or(name)
        .to_ascii_lowercase()
}

pub struct Assets {
    inside: Option<zip::ZipArchive<Cursor<Vec<u8>>>>,

    index: std::collections::HashMap<String, Entry>,
}

enum Entry {
    Inside(usize),
    OnDisk(PathBuf),
}

impl Assets {
    #[must_use]
    pub fn open(origin: &Origin) -> Self {
        let mut index = std::collections::HashMap::new();
        match origin {
            Origin::Archive(path) => {
                let Some(zip) = fs::read(path)
                    .ok()
                    .and_then(|bytes| zip::ZipArchive::new(Cursor::new(bytes)).ok())
                else {
                    return Self {
                        inside: None,
                        index,
                    };
                };
                let mut zip = zip;
                for at in 0..zip.len() {
                    if let Ok(file) = zip.by_index(at) {
                        if !file.is_dir() {
                            index.insert(flatten(file.name()), Entry::Inside(at));
                        }
                    }
                }
                Self {
                    inside: Some(zip),
                    index,
                }
            }
            Origin::Folder(folder) => {
                walk(folder, folder, &mut index, 0);
                Self {
                    inside: None,
                    index,
                }
            }
        }
    }

    pub fn read(&mut self, name: &str) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        match self.index.get(&flatten(name))? {
            Entry::OnDisk(path) => fs::read(path).ok(),
            Entry::Inside(at) => {
                let mut file = self.inside.as_mut()?.by_index(*at).ok()?;
                file.read_to_end(&mut out).ok()?;
                Some(out)
            }
        }
    }

    #[must_use]
    pub fn osb(&self) -> Option<String> {
        let mut found: Vec<&String> = self
            .index
            .keys()
            .filter(|name| name.ends_with(".osb"))
            .collect();

        found.sort();
        found.first().map(|name| (*name).clone())
    }
}

fn flatten(name: &str) -> String {
    name.replace('\\', "/")
        .trim_start_matches("./")
        .to_ascii_lowercase()
}

fn walk(
    root: &Path,
    at: &Path,
    index: &mut std::collections::HashMap<String, Entry>,
    depth: usize,
) {
    if depth > 4 {
        return;
    }
    let Ok(entries) = fs::read_dir(at) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(root, &path, index, depth + 1);
        } else if let Ok(relative) = path.strip_prefix(root) {
            index.insert(
                flatten(&relative.to_string_lossy()),
                Entry::OnDisk(path.clone()),
            );
        }
    }
}

pub fn load(
    replay_path: &Path,
    map: Option<&Path>,
    songs: Option<&Path>,
) -> Result<(Beatmap, Replay, Origin, String), String> {
    let bytes = std::fs::read(replay_path).map_err(|e| format!("{e}"))?;
    let replay = Replay::parse(&bytes).map_err(|e| format!("{e}"))?;
    let found = match map {
        Some(path) => load_map(path, &replay.beatmap_hash)?,
        None => {
            let songs = songs.ok_or("no map given and nowhere to search for one")?;
            search_dir(songs, &replay.beatmap_hash)?
                .ok_or_else(|| format!("map {} not found", replay.beatmap_hash))?
        }
    };
    let beatmap = Beatmap::parse(&found.text).map_err(|e| format!("{e}"))?;

    Ok((beatmap, replay, found.origin, found.text))
}
