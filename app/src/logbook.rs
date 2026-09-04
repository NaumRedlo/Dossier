//! The session's log: what the window did, kept where it can be read later.
//!
//! Everything the engine says about a render used to end up in one paragraph
//! under the list — the hit sound counts, the frame size, the length of the
//! file. That is the right information and the wrong place: it is read once
//! ever, usually when something looks wrong, and until then it sits between
//! somebody and the button they came for. So it goes to a file, and the window
//! says where.
//!
//! One file per run, named for when the run started. A single rolling log would
//! need trimming, and a log that trims itself throws away the session somebody
//! is about to ask about.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static FILE: OnceLock<PathBuf> = OnceLock::new();

/// Where this session writes. Decided once, on the first line written.
pub fn path() -> &'static Path {
    FILE.get_or_init(|| {
        let folder = crate::settings::home().join(".dossier").join("logs");
        let _ = std::fs::create_dir_all(&folder);
        folder.join(format!(
            "{}.log",
            stamp(seconds_now()).replace([':', ' '], "-")
        ))
    })
}

/// Seconds since the epoch, or zero on a machine whose clock is before it.
fn seconds_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// `2026-09-04 18:22:07` from a count of seconds.
///
/// The application's one date formatter — the replay list borrows it for the
/// moment a play was recorded. Two of these would be two answers about when
/// something happened.
///
/// The civil-from-days arithmetic is Howard Hinnant's, which is the standard
/// answer to this and shorter than the dependency that would otherwise arrive
/// to do it. UTC: a log is read beside other logs and beside the bot's, and a
/// local time makes two machines disagree about the order of events.
pub fn stamp(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let rest = seconds % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = era * 400 + yoe + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}",
        rest / 3600,
        (rest % 3600) / 60,
        rest % 60
    )
}

/// Put a headed block in the log. Nothing is ever taken out of one.
///
/// A failure to write is not reported: a log that interrupts the work it is
/// logging is worse than a missing log, and the window says where the file is
/// rather than promising it exists.
pub fn note(head: &str, lines: &[String]) {
    append(path(), &stamp(seconds_now()), head, lines);
}

/// The same, against a named file — which is the whole of it, so that the
/// shape of a block is something a test can read back rather than something
/// only a running window ever sees.
fn append(file: &Path, when: &str, head: &str, lines: &[String]) {
    let Ok(mut out) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)
    else {
        return;
    };
    let _ = writeln!(out, "[{when}] {head}");
    for line in lines {
        let _ = writeln!(out, "    {line}");
    }
    let _ = writeln!(out);
}

/// Hand the log to whatever this system opens text with.
///
/// No path is taken from anywhere: this one is ours, built here, and that is
/// the whole reason it may be opened at all — see the list in [`crate::link`],
/// which exists because `open` will launch an application as readily as it
/// shows a file.
pub fn open() -> Result<(), String> {
    let file = path();
    if !file.is_file() {
        // Nothing has happened yet worth writing down. An empty file to open is
        // a better answer than an error about a file the window just named.
        note("окно открыто", &[]);
    }
    let started = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(file).spawn()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(file)
            .spawn()
    } else {
        std::process::Command::new("xdg-open").arg(file).spawn()
    };
    started.map(|_| ()).map_err(|why| why.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Dates a person can check by hand, including the two the arithmetic is
    /// there for: a leap day, and the turn of a century that is not a leap year
    /// by the hundred rule but is one by the four-hundred.
    #[test]
    fn a_count_of_seconds_reads_as_a_date() {
        assert_eq!(stamp(0), "1970-01-01 00:00:00");
        assert_eq!(stamp(1), "1970-01-01 00:00:01");
        assert_eq!(stamp(951_782_400), "2000-02-29 00:00:00");
        assert_eq!(stamp(1_609_459_199), "2020-12-31 23:59:59");
        assert_eq!(stamp(1_756_944_000), "2025-09-04 00:00:00");
    }

    /// Two renders in one session are two blocks, and neither takes anything
    /// out of the other: a log that rewrites itself throws away the session
    /// somebody is about to ask about.
    #[test]
    fn every_block_is_added_and_none_replaces_another() {
        let file = std::env::temp_dir().join(format!("dossier-log-{}.log", std::process::id()));
        let _ = std::fs::remove_file(&file);

        append(
            &file,
            "2025-09-04 10:00:00",
            "рендер: /tmp/a.mp4",
            &[
                "hit sounds: 23 from the skin".to_owned(),
                "video 1920x1080 121.193s".to_owned(),
            ],
        );
        append(&file, "2025-09-04 10:04:11", "монтаж: /tmp/b.mp4", &[]);

        let text = std::fs::read_to_string(&file).expect("written");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines,
            [
                "[2025-09-04 10:00:00] рендер: /tmp/a.mp4",
                "    hit sounds: 23 from the skin",
                "    video 1920x1080 121.193s",
                "",
                "[2025-09-04 10:04:11] монтаж: /tmp/b.mp4",
                "",
            ]
        );
        let _ = std::fs::remove_file(&file);
    }

    /// The name goes into a file name, so it may not carry a colon: Windows
    /// refuses one outright and macOS shows it back as a slash.
    #[test]
    fn the_file_is_named_by_something_every_system_will_accept() {
        let named = stamp(1_756_944_000).replace([':', ' '], "-");
        assert_eq!(named, "2025-09-04-00-00-00");
        assert!(!named.contains(':'));
    }
}
