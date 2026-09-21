use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::library::Entry;
use crate::render;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Video {
    pub path: PathBuf,
    pub replay: PathBuf,
    pub replay_hash: String,
    pub map_hash: String,
    pub player: String,
    pub song: String,
    pub version: String,
    pub mods: Vec<String>,
    pub length_ms: i64,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub size: u64,
    pub made_at: i64,
    pub sent_at: Option<i64>,
    pub background: Option<PathBuf>,
}

impl Video {
    pub fn from_render(entry: &Entry, path: PathBuf, length_ms: i64, width: u32, height: u32, fps: u32) -> Video {
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let (song, version, background) = match &entry.map {
            Some(map) => (format!("{} — {}", map.artist, map.title), map.version.clone(), map.background.clone()),
            None => match &entry.named {
                Some(named) => (format!("{} — {}", named.artist, named.title), named.version.clone(), None),
                None => (String::new(), String::new(), None),
            },
        };
        Video {
            path,
            replay: entry.path.clone(),
            replay_hash: entry.replay_hash.clone(),
            map_hash: entry.map_hash.clone(),
            player: entry.player.clone(),
            song,
            version,
            mods: entry.mods.clone(),
            length_ms,
            width,
            height,
            fps,
            size,
            made_at: chrono::Utc::now().timestamp(),
            sent_at: None,
            background,
        }
    }

    pub fn map_line(&self) -> String {
        if self.version.is_empty() {
            self.song.clone()
        } else {
            format!("{} [{}]", self.song, self.version)
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Store {
    pub videos: Vec<Video>,
    #[serde(skip)]
    at: PathBuf,
}

pub fn index_path() -> PathBuf {
    render::renders_dir().join("videos.json")
}

impl Store {
    pub fn load() -> Store {
        Store::at(index_path())
    }

    pub fn at(index: PathBuf) -> Store {
        let mut store: Store = std::fs::read(&index)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        store.at = index;
        store.videos.retain(|v| v.path.exists());
        store.videos.sort_by_key(|v| std::cmp::Reverse(v.made_at));
        store
    }

    pub fn save(&self) {
        if self.at.as_os_str().is_empty() {
            return;
        }
        if let Some(dir) = self.at.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&self.at, text);
        }
    }

    pub fn add(&mut self, video: Video) {
        self.videos.retain(|v| v.path != video.path);
        self.videos.insert(0, video);
        self.save();
    }

    pub fn forget(&mut self, path: &Path) {
        self.videos.retain(|v| v.path != path);
        self.save();
    }

    pub fn mark_sent(&mut self, path: &Path, at: i64) {
        if let Some(video) = self.videos.iter_mut().find(|v| v.path == path) {
            video.sent_at = Some(at);
            self.save();
        }
    }

    pub fn marry(&mut self, entries: &[Entry]) {
        let mut changed = false;
        for video in &mut self.videos {
            if !video.replay_hash.is_empty() {
                continue;
            }
            let Some(entry) = entries.iter().find(|e| e.player == video.player && e.song().as_deref() == Some(video.song.as_str())) else {
                continue;
            };
            video.replay = entry.path.clone();
            video.replay_hash = entry.replay_hash.clone();
            video.map_hash = entry.map_hash.clone();
            video.mods = entry.mods.clone();
            video.background = entry.map.as_ref().and_then(|m| m.background.clone());
            changed = true;
        }
        if changed {
            self.save();
        }
    }

    pub fn total_size(&self) -> u64 {
        self.videos.iter().map(|v| v.size).sum()
    }
}

pub struct Probe {
    pub length_ms: i64,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

pub fn probe(ffmpeg: &Path, path: &Path) -> Option<Probe> {
    let out = std::process::Command::new(ffmpeg).args(["-hide_banner", "-i"]).arg(path).output().ok()?;
    let text = String::from_utf8_lossy(&out.stderr);
    read_probe(&text)
}

pub fn read_probe(text: &str) -> Option<Probe> {
    let mut length_ms = None;
    let mut size = None;
    let mut fps = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("Duration: ") {
            let stamp = rest.split(',').next()?.trim();
            let mut parts = stamp.split(':');
            let h: f64 = parts.next()?.parse().ok()?;
            let m: f64 = parts.next()?.parse().ok()?;
            let s: f64 = parts.next()?.parse().ok()?;
            length_ms = Some(((h * 3600.0 + m * 60.0 + s) * 1000.0) as i64);
        }
        if line.contains("Video:") {
            for part in line.split(',') {
                let part = part.trim();
                if let Some(rate) = part.strip_suffix(" fps") {
                    fps = rate.trim().parse::<f64>().ok().map(|f| f.round() as u32);
                }
                let dims = part.split(' ').next().unwrap_or("");
                if let Some((w, h)) = dims.split_once('x') {
                    if let (Ok(w), Ok(h)) = (w.parse::<u32>(), h.parse::<u32>()) {
                        if size.is_none() && w > 0 && h > 0 {
                            size = Some((w, h));
                        }
                    }
                }
            }
        }
    }
    let (width, height) = size?;
    Some(Probe { length_ms: length_ms?, width, height, fps: fps.unwrap_or(60) })
}

pub fn named_from_file(stem: &str) -> (String, String, String) {
    let (player, rest) = match stem.split_once(" - ") {
        Some((p, r)) => (p.trim().to_owned(), r.trim()),
        None => (stem.to_owned(), ""),
    };
    let (song, version) = match (rest.rfind(" ["), rest.ends_with(']')) {
        (Some(open), true) => (rest[..open].trim().to_owned(), rest[open + 2..rest.len() - 1].to_owned()),
        _ => (rest.to_owned(), String::new()),
    };
    (player, song, version)
}

pub fn strays(known: &[Video]) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(render::renders_dir()) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = read
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("mp4")))
        .filter(|p| !known.iter().any(|v| &v.path == p))
        .collect();
    found.sort();
    found
}

pub fn adopt(ffmpeg: &Path, path: &Path) -> Option<Video> {
    let probe = probe(ffmpeg, path)?;
    let stem = path.file_stem()?.to_string_lossy();
    let (player, song, version) = named_from_file(&stem);
    let meta = std::fs::metadata(path).ok()?;
    let made_at = meta
        .modified()
        .ok()
        .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs() as i64);
    Some(Video {
        path: path.to_path_buf(),
        replay: PathBuf::new(),
        replay_hash: String::new(),
        map_hash: String::new(),
        player,
        song,
        version,
        mods: Vec::new(),
        length_ms: probe.length_ms,
        width: probe.width,
        height: probe.height,
        fps: probe.fps,
        size: meta.len(),
        made_at,
        sent_at: None,
        background: None,
    })
}

pub fn to_bin(path: &Path) -> Result<(), String> {
    trash::delete(path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffmpeg_banner_gives_length_size_and_rate() {
        let text = "  Duration: 00:03:51.20, start: 0.000000, bitrate: 3373 kb/s\n  Stream #0:0[0x1](und): Video: h264 (High) (avc1 / 0x31637661), yuv420p(tv, bt709/unknown/unknown, progressive), 1920x1080, 3153 kb/s, 60 fps, 60 tbr, 15360 tbn (default)\n";
        let probe = read_probe(text).unwrap();
        assert_eq!((probe.length_ms, probe.width, probe.height, probe.fps), (231_200, 1920, 1080, 60));
    }

    #[test]
    fn a_render_file_name_splits_into_player_song_and_version() {
        let (p, s, v) = named_from_file("NaumRedlo - Y&Co. — Daisuke [moph's Expert]");
        assert_eq!((p.as_str(), s.as_str(), v.as_str()), ("NaumRedlo", "Y&Co. — Daisuke", "moph's Expert"));
        let (p, s, v) = named_from_file("just a file");
        assert_eq!((p.as_str(), s.as_str(), v.as_str()), ("just a file", "", ""));
    }

    #[test]
    fn the_store_keeps_newest_first_and_drops_missing_files() {
        let dir = std::env::temp_dir().join(format!("dossier-videos-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let here = dir.join("a.mp4");
        std::fs::write(&here, b"x").unwrap();
        let gone = dir.join("b.mp4");
        let video = |path: PathBuf, made_at: i64| Video {
            path,
            replay: PathBuf::new(),
            replay_hash: String::new(),
            map_hash: String::new(),
            player: "p".into(),
            song: "s".into(),
            version: String::new(),
            mods: vec![],
            length_ms: 0,
            width: 1920,
            height: 1080,
            fps: 60,
            size: 1,
            made_at,
            sent_at: None,
            background: None,
        };
        let index = dir.join("videos.json");
        let mut first = Store::at(index.clone());
        first.add(video(here.clone(), 1));
        first.add(video(gone, 2));
        assert_eq!(first.videos[0].made_at, 2);
        let store = Store::at(index.clone());
        assert_eq!(store.videos.len(), 1);
        assert_eq!(store.videos[0].path, here);
        let mut again = Store::at(index);
        again.add(video(here.clone(), 3));
        assert_eq!(again.videos.len(), 1);
        assert_eq!(again.videos[0].made_at, 3);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
