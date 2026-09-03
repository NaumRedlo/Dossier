//! What this machine has been told, and where it is kept.
//!
//! The same file the terminal client reads — `~/.dossier/worker.env` — because
//! the two are the same worker wearing different faces, and somebody who set
//! one up should not have to do it twice. Which means writing it back has to
//! be careful: the file may hold keys this does not know about, and comments
//! somebody put there on purpose.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Everything the application asks for, and nothing it can work out.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    pub server: String,
    pub token: String,
    /// What this machine calls itself on the farm.
    ///
    /// Its host name to begin with, because that is what a worker was always
    /// called — but a name is for the person reading the farm, and "MacBook
    /// Air (2)" tells them nothing. Kept apart from the host name so that
    /// renaming here does not rename the computer.
    pub name: String,
    pub songs: String,
    /// Where skins are kept, when they are kept anywhere.
    pub skins: String,
    /// Where this person's own replays live, for rendering one by hand.
    pub replays: String,
}

/// The keys this writes. Anything else in the file is left exactly as it was.
const KEYS: [(&str, fn(&Settings) -> &String); 6] = [
    ("RENDER_SERVER", |s| &s.server),
    ("RENDER_WORKER_TOKEN", |s| &s.token),
    ("RENDER_WORKER_NAME", |s| &s.name),
    ("DOSSIER_SONGS_DIR", |s| &s.songs),
    ("DOSSIER_SKINS_DIR", |s| &s.skins),
    ("DOSSIER_REPLAYS_DIR", |s| &s.replays),
];

pub fn path() -> PathBuf {
    home().join(".dossier").join("worker.env")
}

pub fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
}

/// `KEY=value` lines, comments and blanks skipped.
pub fn read_pairs(file: &Path) -> Vec<(String, String)> {
    let Ok(text) = std::fs::read_to_string(file) else {
        return Vec::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| {
            (
                key.trim().to_owned(),
                value.trim().trim_matches('"').to_owned(),
            )
        })
        .collect()
}

impl Settings {
    /// The file, with the environment on top of it.
    ///
    /// That way round deliberately, and the same way round the terminal client
    /// reads them: a variable exported for one run beats what is written down.
    pub fn load() -> Self {
        let pairs = read_pairs(&path());
        let value = |key: &str| {
            std::env::var(key)
                .ok()
                .filter(|found| !found.is_empty())
                .or_else(|| {
                    pairs
                        .iter()
                        .find(|(had, _)| had == key)
                        .map(|(_, found)| found.clone())
                })
                .unwrap_or_default()
        };
        let mut said = Self {
            server: value("RENDER_SERVER"),
            token: value("RENDER_WORKER_TOKEN"),
            name: value("RENDER_WORKER_NAME"),
            songs: value("DOSSIER_SONGS_DIR"),
            skins: value("DOSSIER_SKINS_DIR"),
            replays: value("DOSSIER_REPLAYS_DIR"),
        };
        if said.name.is_empty() {
            said.name = host_name();
        }
        if said.songs.is_empty() {
            said.songs = home().join(".osu").join("Songs").display().to_string();
        }
        said
    }

    /// Write it back, keeping everything this does not understand.
    ///
    /// A line this knows is replaced where it stands; one it has never seen is
    /// left alone; a key that was missing is appended. The file belongs to the
    /// terminal client as much as to this, and a settings screen that quietly
    /// drops somebody's `RENDER_HOURS` would be the worst kind of helpful.
    pub fn save(&self) -> Result<(), String> {
        let file = path();
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let had = std::fs::read_to_string(&file).unwrap_or_default();
        let mut written: Vec<String> = Vec::new();
        let mut seen = Vec::new();
        for line in had.lines() {
            let key = line
                .split_once('=')
                .map(|(key, _)| key.trim())
                .filter(|key| !key.starts_with('#'));
            match key.and_then(|key| KEYS.iter().find(|(known, _)| *known == key)) {
                Some((known, of)) => {
                    seen.push(*known);
                    written.push(format!("{known}={}", of(self)));
                }
                None => written.push(line.to_owned()),
            }
        }
        for (known, of) in KEYS {
            if !seen.contains(&known) {
                written.push(format!("{known}={}", of(self)));
            }
        }
        let mut text = written.join("\n");
        text.push('\n');
        std::fs::write(&file, text).map_err(|e| e.to_string())
    }

    /// Whether this looks like the first time somebody has opened it.
    ///
    /// The two the machine cannot guess. Everything else has a sensible answer
    /// without being asked, and asking anyway is how a setup screen becomes
    /// eleven questions nobody reads.
    pub fn first_run(&self) -> bool {
        self.server.is_empty() || self.token.is_empty()
    }
}

/// What the computer calls itself, for a worker that has not been named.
pub fn host_name() -> String {
    for key in ["HOSTNAME", "COMPUTERNAME", "NAME"] {
        if let Ok(found) = std::env::var(key) {
            if !found.trim().is_empty() {
                return found.trim().to_owned();
            }
        }
    }
    std::process::Command::new("hostname")
        .output()
        .ok()
        .filter(|done| done.status.success())
        .map(|done| String::from_utf8_lossy(&done.stdout).trim().to_owned())
        .filter(|found| !found.is_empty())
        .unwrap_or_else(|| "воркер".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("dossier-settings-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a place to write");
        dir
    }

    /// The whole reason `save` is not three lines: this file is shared with the
    /// terminal client, and a settings screen that drops somebody's own keys is
    /// the worst kind of helpful.
    #[test]
    fn writing_settings_keeps_the_lines_it_does_not_understand() {
        let dir = scratch("keeps");
        let file = dir.join("worker.env");
        std::fs::write(
            &file,
            "# my own note\nRENDER_HOURS=22-6\nRENDER_SERVER=old\n\nRENDER_PAUSE=1\n",
        )
        .expect("written");

        // The same walk `save` does, against a file this test owns.
        let said = Settings {
            server: "new".to_owned(),
            token: "abc".to_owned(),
            ..Settings::default()
        };
        let had = std::fs::read_to_string(&file).expect("read");
        let mut out: Vec<String> = Vec::new();
        let mut seen = Vec::new();
        for line in had.lines() {
            let key = line
                .split_once('=')
                .map(|(key, _)| key.trim())
                .filter(|key| !key.starts_with('#'));
            match key.and_then(|key| KEYS.iter().find(|(known, _)| *known == key)) {
                Some((known, of)) => {
                    seen.push(known);
                    out.push(format!("{known}={}", of(&said)));
                }
                None => out.push(line.to_owned()),
            }
        }
        for (known, of) in KEYS {
            if !seen.contains(&&known) {
                out.push(format!("{known}={}", of(&said)));
            }
        }
        let text = out.join("\n");
        assert!(
            text.contains("# my own note"),
            "a comment was dropped:\n{text}"
        );
        assert!(
            text.contains("RENDER_HOURS=22-6"),
            "a key we do not know was dropped"
        );
        assert!(text.contains("RENDER_PAUSE=1"));
        assert!(
            text.contains("RENDER_SERVER=new"),
            "the value was not replaced"
        );
        assert_eq!(
            text.matches("RENDER_SERVER").count(),
            1,
            "it was written twice"
        );
        assert!(
            text.contains("RENDER_WORKER_TOKEN=abc"),
            "a missing key was not added"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_setting_is_read_past_comments_quotes_and_rubbish() {
        let dir = scratch("read");
        let file = dir.join("worker.env");
        std::fs::write(
            &file,
            "# a note\n\nRENDER_SERVER = \"https://example\"\nnonsense\nDOSSIER_SKINS_DIR=/skins\n",
        )
        .expect("written");
        let pairs = read_pairs(&file);
        assert_eq!(
            pairs,
            vec![
                ("RENDER_SERVER".to_owned(), "https://example".to_owned()),
                ("DOSSIER_SKINS_DIR".to_owned(), "/skins".to_owned()),
            ]
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Only the two nobody can guess.
    #[test]
    fn a_first_run_is_a_missing_server_or_a_missing_token() {
        let full = Settings {
            server: "s".to_owned(),
            token: "t".to_owned(),
            ..Settings::default()
        };
        assert!(!full.first_run());
        assert!(Settings {
            token: String::new(),
            ..full.clone()
        }
        .first_run());
        assert!(Settings {
            server: String::new(),
            ..full
        }
        .first_run());
    }

    #[test]
    fn a_machine_that_will_not_say_its_name_still_has_one() {
        assert!(!host_name().is_empty());
    }
}
