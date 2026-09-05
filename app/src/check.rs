use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub name: String,

    pub ok: Option<bool>,
    pub said: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub fix: String,
}

impl Row {
    fn new(name: &str, ok: Option<bool>, said: impl Into<String>, fix: &str) -> Self {
        Self {
            name: name.to_owned(),
            ok,
            said: said.into(),
            fix: fix.to_owned(),
        }
    }
}

pub fn fingerprint(secret: &str) -> String {
    if secret.is_empty() {
        return "нет".to_owned();
    }
    use sha2::{Digest, Sha256};
    let short = hex(&Sha256::digest(secret.as_bytes())[..4]);
    format!(
        "{} {}, {short}",
        secret.chars().count(),
        plural(secret.chars().count(), "знак", "знака", "знаков")
    )
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn plural<'a>(n: usize, one: &'a str, few: &'a str, many: &'a str) -> &'a str {
    if n % 10 == 1 && n % 100 != 11 {
        one
    } else if (2..=4).contains(&(n % 10)) && !(12..=14).contains(&(n % 100)) {
        few
    } else {
        many
    }
}

pub fn config_path() -> PathBuf {
    home().join(".dossier").join("worker.env")
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
}

pub fn read_pairs(path: &Path) -> Vec<(String, String)> {
    let Ok(text) = std::fs::read_to_string(path) else {
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

pub(crate) fn on_path(name: &str) -> Option<PathBuf> {
    let sep = if cfg!(windows) { ';' } else { ':' };
    let names: Vec<String> = if cfg!(windows) {
        vec![format!("{name}.exe"), name.to_owned()]
    } else {
        vec![name.to_owned()]
    };
    std::env::var("PATH").ok().and_then(|path| {
        path.split(sep)
            .flat_map(|dir| names.iter().map(move |n| Path::new(dir).join(n)))
            .find(|candidate| candidate.is_file())
    })
}

pub fn handshake(said: &crate::settings::Settings) -> Row {
    let engine = format!("dossier {}", env!("CARGO_PKG_VERSION"));
    let asked = crate::bot::Bot::new(&said.server, &said.token, &said.name)
        .and_then(|bot| bot.hello(&engine));
    match asked {
        Ok(hello) if hello.agree => Row::new(
            "Сборка",
            Some(true),
            if hello.waiting > 0 {
                format!("Сборки сходятся · в очереди {}", hello.waiting)
            } else {
                "Сборки сходятся · очередь пуста".to_owned()
            },
            "",
        ),
        Ok(hello) => Row::new(
            "Сборка",
            Some(false),
            if hello.reason.is_empty() {
                format!("Бот рисует сборкой {}, а здесь {engine}", hello.build)
            } else {
                hello.reason
            },
            &if hello.release.is_empty() {
                "Работа не берётся, пока сборки разные".to_owned()
            } else {
                format!("Всем нужен релиз {}", hello.release)
            },
        ),
        Err(refused) => Row::new(
            "Сборка",
            Some(false),
            refused.to_string(),
            "Адрес и токен двумя строками выше",
        ),
    }
}

pub fn ready() -> Vec<Row> {
    let mut rows = Vec::new();

    let config = config_path();
    let pairs = read_pairs(&config);
    rows.push(Row::new(
        "Настройки",
        if pairs.is_empty() { None } else { Some(true) },
        if pairs.is_empty() {
            format!("Нет в {}", config.display())
        } else {
            config.display().to_string()
        },
        "Их можно держать там, а не в переменных оболочки",
    ));

    let value = |key: &str| {
        std::env::var(key)
            .ok()
            .filter(|v| !v.is_empty())
            .or_else(|| pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone()))
    };

    let token = value("RENDER_WORKER_TOKEN").unwrap_or_default();
    rows.push(Row::new(
        "Токен",
        Some(!token.is_empty()),
        fingerprint(&token),
        "RENDER_WORKER_TOKEN, тот же самый, что у бота",
    ));

    let server = value("RENDER_SERVER").unwrap_or_default();
    rows.push(Row::new(
        "Адрес бота",
        Some(!server.is_empty()),
        if server.is_empty() {
            "Не задан".to_owned()
        } else {
            server.clone()
        },
        "RENDER_SERVER — куда ходить за работой",
    ));

    let ffmpeg = on_path("ffmpeg");
    rows.push(Row::new(
        "ffmpeg",
        Some(ffmpeg.is_some()),
        ffmpeg.map_or_else(
            || "Нет в PATH".to_owned(),
            |path| path.display().to_string(),
        ),
        "Нужен, чтобы перегнать звуки скина и склеить дорожку",
    ));

    let songs =
        value("DOSSIER_SONGS_DIR").map_or_else(|| home().join(".osu").join("Songs"), PathBuf::from);
    let usable = songs.is_dir() || std::fs::create_dir_all(&songs).is_ok();
    rows.push(Row::new(
        "Склад карт",
        Some(usable),
        songs.display().to_string(),
        "Приложение качает карты сюда и не смогло создать эту папку",
    ));

    let can = crate::machine::capacity();
    rows.push(Row::new(
        "Текущее устройство",
        Some(can.take),
        if can.take {
            format!(
                "{}, {} {}",
                can.reason,
                can.threads,
                plural(can.threads as usize, "поток", "потока", "потоков")
            )
        } else {
            can.reason.clone()
        },
        "Работа не берётся, пока это так",
    ));

    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_is_named_by_its_length_and_a_hash_of_it() {
        let one = fingerprint("abc");
        assert!(one.starts_with("3 знака"), "{one}");

        assert!(fingerprint("abc\n").starts_with("4 знака"));
        assert!(fingerprint("\"abc\"").starts_with("5 знаков"));
        assert_ne!(one, fingerprint("abd"), "two secrets shared a name");
        assert_eq!(fingerprint(""), "нет");
    }

    #[test]
    fn russian_counts_in_three_ways() {
        for (n, want) in [
            (1, "знак"),
            (2, "знака"),
            (5, "знаков"),
            (11, "знаков"),
            (21, "знак"),
            (104, "знака"),
            (112, "знаков"),
        ] {
            assert_eq!(plural(n, "знак", "знака", "знаков"), want, "{n}");
        }
    }

    #[test]
    fn settings_are_read_past_comments_quotes_and_blank_lines() {
        let dir = std::env::temp_dir().join(format!("dossier-check-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a place to write");
        let file = dir.join("worker.env");
        std::fs::write(
            &file,
            "# a comment\n\nRENDER_SERVER = \"https://example\"\nRENDER_WORKER_TOKEN=abc\nbroken\n",
        )
        .expect("written");
        let pairs = read_pairs(&file);
        assert_eq!(
            pairs,
            vec![
                ("RENDER_SERVER".to_owned(), "https://example".to_owned()),
                ("RENDER_WORKER_TOKEN".to_owned(), "abc".to_owned()),
            ]
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_file_that_is_not_there_is_no_settings_rather_than_a_failure() {
        assert!(read_pairs(std::path::Path::new("/nowhere/at/all.env")).is_empty());
    }

    #[test]
    fn every_row_can_be_read_by_somebody_who_is_stuck() {
        for row in ready() {
            assert!(!row.name.is_empty(), "a row with no name");
            assert!(!row.said.is_empty(), "{} said nothing", row.name);
            if row.ok == Some(false) {
                assert!(!row.fix.is_empty(), "{} failed without a remedy", row.name);
            }
        }
    }
}
