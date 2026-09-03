//! Whether this machine could take work, and what is stopping it.
//!
//! The terminal client answers this with `--check` and a list of lines. The
//! same questions, in a window — and two of them have gone, which is the point
//! of the rewrite rather than a detail of it. "Is the engine there" and "do the
//! builds agree" were the two commonest ways a first evening was spent, and
//! they cannot be asked of an application that *is* the engine.

use std::path::{Path, PathBuf};

use serde::Serialize;

/// One row: what was asked, how it went, and what to do about it.
///
/// A remedy rather than a failure. Everything this catches is somebody's setup
/// and every one of them has a fix that fits on a line.
#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub name: String,
    /// `None` is neither pass nor fail: nobody has asked yet, or nobody can.
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

/// A short, shareable name for a secret, which is never the secret.
///
/// Eight hex characters of a hash and the length. Two sides that disagree about
/// a token cannot compare it by pasting it into a chat, and a length one longer
/// than expected is a quote or a newline that came along for the ride.
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

/// Russian's three-way plural — «1 знак», «2 знака», «5 знаков».
pub fn plural<'a>(n: usize, one: &'a str, few: &'a str, many: &'a str) -> &'a str {
    if n % 10 == 1 && n % 100 != 11 {
        one
    } else if (2..=4).contains(&(n % 10)) && !(12..=14).contains(&(n % 100)) {
        few
    } else {
        many
    }
}

/// Where the settings live when nobody has said otherwise.
pub fn config_path() -> PathBuf {
    home().join(".dossier").join("worker.env")
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
}

/// `KEY=value` lines, comments and blanks skipped.
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

/// The first of these on `PATH`, if any.
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

/// Every question, asked.
pub fn ready() -> Vec<Row> {
    let mut rows = Vec::new();

    let config = config_path();
    let pairs = read_pairs(&config);
    rows.push(Row::new(
        "настройки",
        if pairs.is_empty() { None } else { Some(true) },
        if pairs.is_empty() {
            format!("нет в {}", config.display())
        } else {
            config.display().to_string()
        },
        "их можно держать там, а не в переменных оболочки",
    ));

    let value = |key: &str| {
        std::env::var(key)
            .ok()
            .filter(|v| !v.is_empty())
            .or_else(|| pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone()))
    };

    let token = value("RENDER_WORKER_TOKEN").unwrap_or_default();
    rows.push(Row::new(
        "токен",
        Some(!token.is_empty()),
        fingerprint(&token),
        "RENDER_WORKER_TOKEN, тот же самый, что у бота",
    ));

    let server = value("RENDER_SERVER").unwrap_or_default();
    rows.push(Row::new(
        "адрес бота",
        Some(!server.is_empty()),
        if server.is_empty() {
            "не задан".to_owned()
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
            || "нет в PATH".to_owned(),
            |path| path.display().to_string(),
        ),
        "нужен, чтобы перегнать звуки скина и склеить дорожку",
    ));

    let songs =
        value("DOSSIER_SONGS_DIR").map_or_else(|| home().join(".osu").join("Songs"), PathBuf::from);
    let usable = songs.is_dir() || std::fs::create_dir_all(&songs).is_ok();
    rows.push(Row::new(
        "склад карт",
        Some(usable),
        songs.display().to_string(),
        "приложение качает карты сюда и не смогло создать эту папку",
    ));

    // The policy, asked of this machine — see `machine::decide`, which is the
    // same one the terminal client applies.
    let can = crate::machine::capacity();
    rows.push(Row::new(
        "эта машина",
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
        "работа не берётся, пока это так",
    ));

    rows.push(Row::new("бот", None, "ещё не спрашивали", ""));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_is_named_by_its_length_and_a_hash_of_it() {
        let one = fingerprint("abc");
        assert!(one.starts_with("3 знака"), "{one}");
        // A quote or a newline that came along for the ride shows up here and
        // nowhere else — which is the whole reason the length is in it.
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

    /// Every row says something, and the ones that can fail carry a remedy.
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
