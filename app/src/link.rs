fn allowed(url: &str) -> bool {
    ["https://", "http://", "mailto:"]
        .iter()
        .any(|scheme| url.starts_with(scheme))
}

use std::path::Path;

pub fn open(url: &str) -> Result<(), String> {
    if !allowed(url) {
        return Err("Такие ссылки не открываются".to_owned());
    }
    let started = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).spawn()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
    } else {
        std::process::Command::new("xdg-open").arg(url).spawn()
    };
    started.map(|_| ()).map_err(|why| why.to_string())
}

const PLAYABLE: [&str; 5] = ["mp4", "mkv", "mov", "webm", "m4v"];

fn playable(path: &Path) -> bool {
    path.extension()
        .and_then(|end| end.to_str())
        .is_some_and(|end| PLAYABLE.contains(&end.to_ascii_lowercase().as_str()))
}

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
        let folder = path.parent().unwrap_or(path);
        std::process::Command::new("xdg-open").arg(folder).spawn()
    };
    started.map(|_| ()).map_err(|why| why.to_string())
}

pub fn play(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err(format!("{} — такого файла нет", path.display()));
    }
    if !playable(path) {
        return Err("Открывается только готовое видео".to_owned());
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
