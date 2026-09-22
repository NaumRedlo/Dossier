use std::path::{Path, PathBuf};

use crate::lang::Lang;
use crate::sources::Source;

pub const DEFAULT_SERVER: &str = "https://onenineeightfour.ignorelist.com";

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    pub lang: Lang,
    pub sources: Vec<Source>,
    pub device: String,
    pub server: String,
    pub token: String,
    pub linked_as: String,
    #[serde(default)]
    pub menu_tab: String,
    #[serde(default = "yes")]
    pub live_scene: bool,
    #[serde(default = "yes")]
    pub pause_unfocused: bool,
    #[serde(default = "default_height")]
    pub render_height: u32,
    #[serde(default = "default_fps")]
    pub render_fps: u32,
    #[serde(default = "default_crf")]
    pub render_crf: u32,
    #[serde(default)]
    pub renders_dir: Option<PathBuf>,
    #[serde(default)]
    pub settings_tab: String,
    #[serde(default)]
    pub tiles_app: Vec<String>,
    #[serde(default)]
    pub tiles_bot: Vec<String>,
    #[serde(default)]
    pub chat_id: Option<i64>,
    #[serde(default)]
    pub tell: Tell,
    #[serde(default)]
    pub skin: Option<PathBuf>,
    #[serde(default = "full")]
    pub music_level: f32,
    #[serde(default = "full")]
    pub hitsound_level: f32,
    #[serde(default = "full")]
    pub player_level: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Tell {
    pub rendered: bool,
    pub errors: bool,
    pub maps: bool,
    pub worker: bool,
}

impl Default for Tell {
    fn default() -> Tell {
        Tell { rendered: true, errors: false, maps: false, worker: true }
    }
}

fn yes() -> bool {
    true
}

fn default_height() -> u32 {
    1080
}

fn default_fps() -> u32 {
    60
}

fn default_crf() -> u32 {
    20
}

fn full() -> f32 {
    1.0
}

pub const HEIGHTS: [u32; 3] = [720, 1080, 1440];
pub const RATES: [u32; 2] = [30, 60];
pub const CRFS: [u32; 3] = [23, 20, 17];

pub fn skins_in(sources: &[Source]) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = sources.iter().filter_map(|source| source.skins.clone()).collect();
    for found in crate::sources::find() {
        if let Some(skins) = found.skins {
            roots.push(skins);
        }
    }
    roots.push(crate::sources::own_root().join("Skins"));
    roots.sort();
    roots.dedup();
    let mut found: Vec<PathBuf> = Vec::new();
    for root in roots {
        let Ok(read) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in read.flatten() {
            if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) && !found.contains(&entry.path()) {
                found.push(entry.path());
            }
        }
    }
    found.sort_by_key(|path| path.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default());
    found.truncate(60);
    found
}

pub fn skin_face(folder: &Path) -> Option<PathBuf> {
    for name in ["hitcircle@2x.png", "hitcircle.png", "cursor@2x.png", "cursor.png", "menu-background.jpg", "menu-background.png"] {
        let file = folder.join(name);
        if file.is_file() {
            return Some(file);
        }
    }
    None
}

impl Settings {
    pub fn render_size(&self) -> (u32, u32) {
        let height = if HEIGHTS.contains(&self.render_height) { self.render_height } else { 1080 };
        (height * 16 / 9, height)
    }

    pub fn renders_dir(&self) -> PathBuf {
        self.renders_dir.clone().unwrap_or_else(|| crate::sources::own_root().join("Renders"))
    }
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            lang: Lang::from_system(),
            sources: Vec::new(),
            device: device_name(),
            server: DEFAULT_SERVER.to_owned(),
            token: String::new(),
            linked_as: String::new(),
            menu_tab: String::new(),
            live_scene: true,
            pause_unfocused: true,
            render_height: 1080,
            render_fps: 60,
            render_crf: 20,
            renders_dir: None,
            settings_tab: String::new(),
            tiles_app: Vec::new(),
            tiles_bot: Vec::new(),
            chat_id: None,
            tell: Tell::default(),
            skin: None,
            music_level: 1.0,
            hitsound_level: 1.0,
            player_level: 1.0,
        }
    }
}

pub fn path() -> PathBuf {
    crate::sources::home().join(".dossier").join("dossier.json")
}

pub fn first_run() -> bool {
    !path().is_file()
}

impl Settings {
    pub fn load() -> Settings {
        std::fs::read_to_string(path())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let file = path();
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&file, text).map_err(|e| e.to_string())
    }
}

pub fn device_name() -> String {
    let said = std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|s| s.trim().to_owned())
        .unwrap_or_default();
    let bare = said.split('.').next().unwrap_or("").trim();
    if bare.is_empty() {
        "Dossier".to_owned()
    } else {
        bare.replace('-', " ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_survive_a_round_trip_through_json() {
        let mut said = Settings::default();
        said.lang = Lang::Ru;
        said.device = "MacBook Pro".to_owned();
        said.token = "abc".to_owned();
        let text = serde_json::to_string(&said).unwrap();
        let back: Settings = serde_json::from_str(&text).unwrap();
        assert_eq!(said, back);
    }

    #[test]
    fn a_device_always_has_a_name() {
        assert!(!device_name().is_empty());
    }
}
