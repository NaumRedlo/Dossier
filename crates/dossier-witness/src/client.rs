use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Client {
    pub build: String,
    pub player: String,
}

const DATE_DIGITS: usize = 8;

impl Client {
    pub fn version(&self) -> Option<i32> {
        let digits: String = self.build.strip_prefix('b')?.chars().take_while(char::is_ascii_digit).collect();
        (digits.len() == DATE_DIGITS).then(|| digits.parse().ok()).flatten()
    }

    pub fn said_in(config: &str) -> Client {
        let mut found = Client::default();
        for line in config.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key.trim() {
                "LastVersion" => found.build = value.trim().to_owned(),
                "Username" => found.player = value.trim().to_owned(),
                _ => {}
            }
        }
        found
    }

    pub fn beside(folder: &Path, user: &str) -> Client {
        config_of(folder, user).and_then(|file| std::fs::read(file).ok()).map_or_else(Client::default, |bytes| Client::said_in(&String::from_utf8_lossy(&bytes)))
    }
}

const SHARED: &str = "osu!.cfg";

pub fn config_of(folder: &Path, user: &str) -> Option<PathBuf> {
    let own = folder.join(format!("osu!.{user}.cfg"));
    if !user.is_empty() && own.is_file() {
        return Some(own);
    }
    std::fs::read_dir(folder)
        .ok()?
        .flatten()
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            name.starts_with("osu!.") && name.ends_with(".cfg") && name != SHARED
        })
        .filter_map(|entry| Some((entry.metadata().ok()?.modified().ok()?, entry.path())))
        .max_by_key(|(written, _)| *written)
        .map(|(_, path)| path)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = "# osu! configuration for crossover\r\nLanguage = ru\r\nLastVersion = b20260924cuttingedge\r\nLastVersionPermissionsFailed = \r\nUsername = Naum Redlo\r\nPassword = not-for-us\r\nSaveUsername = 1\r\n";

    #[test]
    fn the_client_says_which_build_it_is_and_who_plays() {
        let client = Client::said_in(CONFIG);
        assert_eq!(client, Client { build: "b20260924cuttingedge".into(), player: "Naum Redlo".into() });
        assert_eq!(client.version(), Some(20_260_924));
    }

    #[test]
    fn a_build_is_dated_by_its_first_eight_digits_whatever_follows() {
        let of = |build: &str| Client { build: build.into(), player: String::new() }.version();
        assert_eq!(of("b20250401.2"), Some(20_250_401));
        assert_eq!(of("b20251102"), Some(20_251_102));
        assert_eq!(of("b2025"), None, "too short to be a date");
        assert_eq!(of("20251102"), None, "not a build name");
        assert_eq!(of(""), None);
    }

    #[test]
    fn a_configuration_with_neither_line_says_nothing() {
        assert_eq!(Client::said_in("Language = ru\nUsernameSuffix = x\n"), Client::default());
    }

    #[test]
    fn the_configuration_is_the_one_named_after_the_person_at_the_machine() {
        let folder = std::env::temp_dir().join(format!("dossier-witness-client-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("osu!.cfg"), "_ReleaseStream = Stable\n").unwrap();
        assert_eq!(config_of(&folder, "nobody"), None, "the shared file names no one");

        std::fs::write(folder.join("osu!.guest.cfg"), "Username = guest\nLastVersion = b20250101\n").unwrap();
        std::fs::write(folder.join("osu!.me.cfg"), CONFIG).unwrap();
        assert_eq!(config_of(&folder, "me"), Some(folder.join("osu!.me.cfg")));
        assert_eq!(Client::beside(&folder, "me").player, "Naum Redlo");
        assert!(config_of(&folder, "").is_some(), "with no name to go by the freshest one is taken");
        let _ = std::fs::remove_dir_all(&folder);
    }
}
