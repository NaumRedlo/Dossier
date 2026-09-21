use std::path::PathBuf;

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
