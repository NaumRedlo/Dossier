use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::library::{self, Map};

pub struct Mirror {
    pub name: &'static str,
    pub look_up: &'static str,
    pub download: &'static str,
}

pub const MIRRORS: [Mirror; 2] = [
    Mirror { name: "osu.direct", look_up: "https://osu.direct/api/v2/md5/", download: "https://osu.direct/api/d/" },
    Mirror { name: "catboy.best", look_up: "https://catboy.best/api/v2/md5/", download: "https://catboy.best/d/" },
];

#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub from: &'static str,
    pub set: u64,
    pub artist: String,
    pub title: String,
}

impl Found {
    pub fn line(&self) -> String {
        match (self.artist.is_empty(), self.title.is_empty()) {
            (false, false) => format!("{} — {}", self.artist, self.title),
            (true, false) => self.title.clone(),
            _ => format!("#{}", self.set),
        }
    }

    pub fn folder(&self) -> String {
        let name = if self.artist.is_empty() && self.title.is_empty() {
            format!("{}", self.set)
        } else {
            format!("{} {} - {}", self.set, self.artist, self.title)
        };
        tidy(&name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Looking,
    Found(Found),
    Downloading { from: &'static str, done: u64, total: Option<u64> },
    Unpacking,
    Checking,
    Done(Map),
    Nowhere,
    Stopped,
    Failed(String),
}

impl Step {
    pub fn is_last(&self) -> bool {
        matches!(self, Step::Done(_) | Step::Nowhere | Step::Stopped | Step::Failed(_))
    }
}

const BIGGEST: u64 = 512 * 1024 * 1024;
const EVERY: Duration = Duration::from_millis(120);

static STOP: AtomicBool = AtomicBool::new(false);

pub fn stop() {
    STOP.store(true, Ordering::SeqCst);
}

fn stopped() -> bool {
    STOP.load(Ordering::SeqCst)
}

pub fn tidy(text: &str) -> String {
    let cleaned: String = text
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            other if other.is_control() => ' ',
            other => other,
        })
        .collect();
    let squeezed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if squeezed.chars().all(|c| c == '.') {
        String::new()
    } else {
        squeezed
    }
}

fn hex(hash: &str) -> Result<String, String> {
    let clean = hash.trim().to_ascii_lowercase();
    if clean.len() == 32 && clean.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(clean)
    } else {
        Err("the replay names no map".to_owned())
    }
}

fn client(seconds: u64) -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(seconds))
        .user_agent(format!("Dossier/{}", crate::bot::BUILD))
        .build()
        .map_err(|e| e.to_string())
}

fn said(value: &serde_json::Value, paths: &[&[&str]]) -> String {
    for path in paths {
        let mut at = value;
        let mut whole = true;
        for step in *path {
            match at.get(step) {
                Some(next) => at = next,
                None => {
                    whole = false;
                    break;
                }
            }
        }
        if whole {
            if let Some(text) = at.as_str() {
                if !text.is_empty() {
                    return text.to_owned();
                }
            }
        }
    }
    String::new()
}

pub fn parse_look_up(mirror: &'static Mirror, body: &serde_json::Value) -> Option<Found> {
    let set = body.get("beatmapset_id").and_then(serde_json::Value::as_u64)?;
    Some(Found {
        from: mirror.name,
        set,
        artist: tidy(&said(body, &[&["beatmapset", "artist"], &["set", "artist"], &["artist"]])),
        title: tidy(&said(body, &[&["beatmapset", "title"], &["set", "title"], &["title"]])),
    })
}

enum Answer {
    Found(Found),
    Unknown,
    Silent(String),
}

fn ask(mirror: &'static Mirror, hash: &str) -> Answer {
    let Ok(http) = client(30) else {
        return Answer::Silent("no way to the network".to_owned());
    };
    let reply = match http.get(format!("{}{hash}", mirror.look_up)).send() {
        Ok(reply) => reply,
        Err(why) => return Answer::Silent(why.to_string()),
    };
    if reply.status() == reqwest::StatusCode::NOT_FOUND {
        return Answer::Unknown;
    }
    if !reply.status().is_success() {
        return Answer::Silent(format!("{} answered {}", mirror.name, reply.status()));
    }
    match reply.json::<serde_json::Value>() {
        Ok(body) => match parse_look_up(mirror, &body) {
            Some(found) => Answer::Found(found),
            None => Answer::Silent(format!("{} answered without a set number", mirror.name)),
        },
        Err(why) => Answer::Silent(why.to_string()),
    }
}

fn download(mirror: &'static Mirror, set: u64, into: &Path, report: &mut dyn FnMut(Step) -> bool) -> Result<(), String> {
    let http = client(1800)?;
    let mut reply = http.get(format!("{}{set}", mirror.download)).send().map_err(|e| e.to_string())?;
    if !reply.status().is_success() {
        return Err(format!("{} answered {}", mirror.name, reply.status()));
    }
    let total = reply.content_length();
    if total.is_some_and(|t| t > BIGGEST) {
        return Err("too big".to_owned());
    }
    let mut file = std::fs::File::create(into).map_err(|e| e.to_string())?;
    let mut done = 0u64;
    let mut chunk = [0u8; 64 * 1024];
    let mut told = Instant::now() - EVERY;
    report(Step::Downloading { from: mirror.name, done, total });
    loop {
        if stopped() {
            return Err("stopped".to_owned());
        }
        let n = reply.read(&mut chunk).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        file.write_all(&chunk[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        if done > BIGGEST {
            return Err("too big".to_owned());
        }
        if told.elapsed() >= EVERY {
            told = Instant::now();
            report(Step::Downloading { from: mirror.name, done, total });
        }
    }
    file.flush().map_err(|e| e.to_string())?;
    if done < 1024 {
        return Err(format!("{} gave an empty file", mirror.name));
    }
    Ok(())
}

fn free_folder(songs: &Path, wanted: &str) -> PathBuf {
    let first = songs.join(wanted);
    if !first.exists() {
        return first;
    }
    (2..100)
        .map(|n| songs.join(format!("{wanted} ({n})")))
        .find(|p| !p.exists())
        .unwrap_or_else(|| songs.join(format!("{wanted} ({})", std::process::id())))
}

pub fn unpack(archive: &Path, into: &Path) -> Result<usize, String> {
    let file = std::fs::File::open(archive).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(into).map_err(|e| e.to_string())?;
    let mut written = 0;
    for at in 0..zip.len() {
        let mut entry = zip.by_index(at).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let Some(relative) = entry.enclosed_name() else {
            continue;
        };
        let target = into.join(relative);
        if let Some(dir) = target.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let mut out = std::fs::File::create(&target).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
        written += 1;
    }
    Ok(written)
}

fn difficulty_in(folder: &Path, hash: &str) -> Option<Map> {
    let mut stack = vec![folder.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("osu")) {
                continue;
            }
            if let Some((found, map)) = library::describe(&path) {
                if found == hash {
                    return Some(map);
                }
            }
        }
    }
    None
}

pub fn fetch(hash: String, songs: PathBuf) -> iced::Task<Step> {
    crate::ui::streamed(move |push| {
        STOP.store(false, Ordering::SeqCst);
        let last = bring(&hash, &songs, push);
        push(last);
    })
}

fn bring(hash: &str, songs: &Path, report: &mut dyn FnMut(Step) -> bool) -> Step {
    let hash = match hex(hash) {
        Ok(hash) => hash,
        Err(why) => return Step::Failed(why),
    };
    report(Step::Looking);
    let mut found = None;
    let mut silence = None;
    for mirror in MIRRORS.iter() {
        if stopped() {
            return Step::Stopped;
        }
        match ask(mirror, &hash) {
            Answer::Found(got) => {
                found = Some(got);
                break;
            }
            Answer::Unknown => {}
            Answer::Silent(why) => silence = Some(why),
        }
    }
    let Some(found) = found else {
        return match silence {
            Some(why) => Step::Failed(why),
            None => Step::Nowhere,
        };
    };
    report(Step::Found(found.clone()));

    if std::fs::create_dir_all(songs).is_err() {
        return Step::Failed(format!("cannot write to {}", songs.display()));
    }
    let archive = songs.join(format!(".{}.osz.download", found.set));
    let mut order: Vec<&'static Mirror> = MIRRORS.iter().filter(|m| m.name == found.from).collect();
    order.extend(MIRRORS.iter().filter(|m| m.name != found.from));
    let mut fetched = Err("no mirror gave the map".to_owned());
    for mirror in order {
        if stopped() {
            let _ = std::fs::remove_file(&archive);
            return Step::Stopped;
        }
        fetched = download(mirror, found.set, &archive, report);
        if fetched.is_ok() {
            break;
        }
    }
    if let Err(why) = fetched {
        let _ = std::fs::remove_file(&archive);
        return if stopped() { Step::Stopped } else { Step::Failed(why) };
    }

    report(Step::Unpacking);
    let into = free_folder(songs, &found.folder());
    let unpacked = unpack(&archive, &into);
    let _ = std::fs::remove_file(&archive);
    if let Err(why) = unpacked {
        let _ = std::fs::remove_dir_all(&into);
        return Step::Failed(why);
    }

    report(Step::Checking);
    match difficulty_in(&into, &hash) {
        Some(map) => Step::Done(map),
        None => {
            let _ = std::fs::remove_dir_all(&into);
            Step::Failed("the set has no difficulty with the replay's hash".to_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mirror_s_answer_names_the_set_and_the_song() {
        let body: serde_json::Value = serde_json::from_str(
            r#"{"beatmapset_id": 292301, "version": "FOUR DIMENSIONS", "beatmapset": {"artist": "xi", "title": "Blue Zenith"}}"#,
        )
        .unwrap();
        let found = parse_look_up(&MIRRORS[0], &body).expect("found");
        assert_eq!(found.set, 292301);
        assert_eq!(found.line(), "xi — Blue Zenith");
        assert_eq!(found.folder(), "292301 xi - Blue Zenith");
        assert_eq!(found.from, "osu.direct");
        let bare: serde_json::Value = serde_json::from_str(r#"{"beatmapset_id": 7}"#).unwrap();
        assert_eq!(parse_look_up(&MIRRORS[1], &bare).unwrap().folder(), "7");
        assert!(parse_look_up(&MIRRORS[1], &serde_json::json!({"nothing": 1})).is_none());
    }

    #[test]
    fn names_are_kept_off_the_disk_s_forbidden_list() {
        assert_eq!(tidy("a/b:c*d?e\"f<g>h|i"), "a b c d e f g h i");
        assert_eq!(tidy("..."), "");
        assert!(hex("ZZ").is_err());
        assert_eq!(hex("0123456789ABCDEF0123456789abcdef").unwrap(), "0123456789abcdef0123456789abcdef");
    }

    #[test]
    fn an_archive_unpacks_into_the_folder_and_the_difficulty_is_found_by_hash() {
        let dir = std::env::temp_dir().join(format!("dossier-maps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let osu = "osu file format v14\n\n[Metadata]\nTitle:Blue Zenith\nArtist:xi\nVersion:Hard\n\n[HitObjects]\n1,2,3\n";
        let archive = dir.join("set.osz");
        {
            let file = std::fs::File::create(&archive).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();
            zip.start_file("xi - Blue Zenith (Asphyxia) [Hard].osu", options).unwrap();
            zip.write_all(osu.as_bytes()).unwrap();
            zip.start_file("audio.mp3", options).unwrap();
            zip.write_all(b"mp3").unwrap();
            zip.finish().unwrap();
        }
        let into = dir.join("292301 xi - Blue Zenith");
        assert_eq!(unpack(&archive, &into).unwrap(), 2);
        let hash = library::md5_hex(osu.as_bytes());
        let map = difficulty_in(&into, &hash).expect("the difficulty");
        assert_eq!(map.line(), "xi — Blue Zenith [Hard]");
        assert!(difficulty_in(&into, "0000").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
