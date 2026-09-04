//! Handing a link to whatever this system opens links with.
//!
//! Not `tauri-plugin-opener`: that is another dependency, another permission
//! file and another thing to keep in step across three systems, for a call each
//! of them already spells in one line.

/// The schemes that may be handed over.
///
/// Short on purpose. The window's content is ours, but `file://` would open
/// anything on this machine and a shell scheme would run it — and the day
/// somebody builds a link out of something a server said, this is the line that
/// will already have been drawn.
fn allowed(url: &str) -> bool {
    ["https://", "http://", "mailto:"]
        .iter()
        .any(|scheme| url.starts_with(scheme))
}

use std::path::Path;

/// Open a link in the browser, the mail client, whatever claims it.
pub fn open(url: &str) -> Result<(), String> {
    if !allowed(url) {
        return Err("такие ссылки не открываются".to_owned());
    }
    let started = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).spawn()
    } else if cfg!(target_os = "windows") {
        // The empty argument is `start`'s title, which it otherwise takes from
        // the URL and then has nothing left to open.
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
    } else {
        std::process::Command::new("xdg-open").arg(url).spawn()
    };
    started.map(|_| ()).map_err(|why| why.to_string())
}

/// The endings a finished render can have, and the only ones this will hand to
/// the system.
///
/// `open` on macOS and `start` on Windows will launch whatever a path turns out
/// to be, an application bundle included. The path this is called with is one
/// we wrote ourselves a moment ago — but "we wrote it" is a claim about today's
/// code, and this list is a claim about every day after.
const PLAYABLE: [&str; 5] = ["mp4", "mkv", "mov", "webm", "m4v"];

fn playable(path: &Path) -> bool {
    path.extension()
        .and_then(|end| end.to_str())
        .is_some_and(|end| PLAYABLE.contains(&end.to_ascii_lowercase().as_str()))
}

/// Show a finished file where it lives, selected in the system's file manager.
///
/// Not the same as opening it: this is the answer to "where did it go", which
/// is the question a person asks first about a render that has just finished.
pub fn reveal(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err(format!("{} — такого файла нет", path.display()));
    }
    let started = if cfg!(target_os = "macos") {
        std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("explorer")
            .arg(format!("/select,{}", path.display()))
            .spawn()
    } else {
        // No desktop-independent way to select a file; the folder is the part
        // every file manager understands.
        let folder = path.parent().unwrap_or(path);
        std::process::Command::new("xdg-open").arg(folder).spawn()
    };
    started.map(|_| ()).map_err(|why| why.to_string())
}

/// Play a finished render in whatever this system plays videos with.
pub fn play(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err(format!("{} — такого файла нет", path.display()));
    }
    if !playable(path) {
        return Err("открывается только готовое видео".to_owned());
    }
    let started = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(path).spawn()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(path)
            .spawn()
    } else {
        std::process::Command::new("xdg-open").arg(path).spawn()
    };
    started.map(|_| ()).map_err(|why| why.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_links_pass() {
        assert!(allowed("https://github.com/NaumRedlo/Dossier"));
        assert!(allowed("mailto:someone@example.com?subject=hi"));
    }

    #[test]
    fn the_rest_do_not() {
        assert!(!allowed("file:///etc/passwd"));
        assert!(!allowed("javascript:alert(1)"));
        assert!(!allowed("/Users/somebody/secrets"));
        assert!(!allowed(" https://sneaky"));
    }

    #[test]
    fn a_refused_link_says_so_rather_than_opening() {
        assert!(open("file:///etc/passwd").is_err());
    }

    /// The whole reason `play` is not `open` with a path: `open` on macOS
    /// launches an application bundle as readily as it plays a video, and the
    /// day a path comes from somewhere less trustworthy than our own render
    /// this is the line that will already have been drawn.
    #[test]
    fn only_a_finished_video_is_played() {
        assert!(playable(Path::new("/tmp/a.mp4")));
        assert!(playable(Path::new("/tmp/a.MKV")));
        assert!(!playable(Path::new("/tmp/Something.app")));
        assert!(!playable(Path::new("/tmp/a.sh")));
        assert!(!playable(Path::new("/tmp/a")));
    }

    #[test]
    fn a_file_that_is_not_there_is_said_so_rather_than_handed_over() {
        let missing = std::env::temp_dir().join("dossier-nothing-here.mp4");
        let _ = std::fs::remove_file(&missing);
        assert!(reveal(&missing).is_err());
        assert!(play(&missing).is_err());
    }
}
