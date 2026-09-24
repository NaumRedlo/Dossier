use std::path::PathBuf;

use crate::bot;
use crate::sources::Source;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Passed(String),
    Failed(String),
    Skipped(String),
}

pub fn quiet(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    let command = std::process::Command::new(program);
    #[cfg(windows)]
    let command = {
        use std::os::windows::process::CommandExt;
        let mut command = command;
        command.creation_flags(0x0800_0000);
        command
    };
    command
}

pub fn ffmpeg_on_path() -> Option<PathBuf> {
    let own = crate::ffmpeg::own();
    if own_works(&own) {
        return Some(own);
    }
    let name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    let on_path: Vec<PathBuf> = std::env::var_os("PATH").map(|paths| std::env::split_paths(&paths).collect()).unwrap_or_default();
    on_path.into_iter().chain(usual_places()).map(|dir| dir.join(name)).find(|candidate| runnable(candidate))
}

fn stamp(path: &std::path::Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
    Some(format!("{}:{modified}", meta.len()))
}

pub fn vouch_for(path: &std::path::Path) {
    if let Some(stamp) = stamp(path) {
        let _ = std::fs::write(path.with_extension("ok"), stamp);
    }
}

fn own_works(own: &std::path::Path) -> bool {
    if !runnable(own) {
        return false;
    }
    let known = stamp(own);
    if known.is_some() && std::fs::read_to_string(own.with_extension("ok")).ok() == known {
        return true;
    }
    if ffmpeg_version(own).is_some() {
        vouch_for(own);
        return true;
    }
    let _ = std::fs::remove_file(own);
    let _ = std::fs::remove_file(own.with_extension("ok"));
    false
}

pub fn runnable(path: &std::path::Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = |path: &std::path::Path| std::fs::metadata(path).map(|meta| meta.permissions().mode()).unwrap_or(0);
        if mode(path) & 0o111 == 0 {
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755));
            return mode(path) & 0o111 != 0;
        }
    }
    true
}

fn usual_places() -> Vec<PathBuf> {
    let home = crate::sources::home();
    let mut out: Vec<PathBuf> = Vec::new();
    if cfg!(target_os = "macos") {
        out.extend(["/opt/homebrew/bin", "/usr/local/bin", "/opt/local/bin"].map(PathBuf::from));
    }
    if cfg!(target_os = "linux") {
        out.extend(["/usr/bin", "/usr/local/bin", "/snap/bin"].map(PathBuf::from));
        out.push(home.join(".local").join("bin"));
    }
    if cfg!(windows) {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            out.push(PathBuf::from(local).join("Microsoft").join("WinGet").join("Links"));
        }
        out.push(PathBuf::from(r"C:\ProgramData\chocolatey\bin"));
        out.push(home.join("scoop").join("shims"));
        out.push(PathBuf::from(r"C:\ffmpeg\bin"));
    }
    out
}

pub fn lend_ffmpeg_to_path() {
    let Some(dir) = ffmpeg_on_path().and_then(|path| path.parent().map(std::path::Path::to_path_buf)) else {
        return;
    };
    let mut paths: Vec<PathBuf> = std::env::var_os("PATH").map(|paths| std::env::split_paths(&paths).collect()).unwrap_or_default();
    if paths.contains(&dir) {
        return;
    }
    paths.insert(0, dir);
    if let Ok(joined) = std::env::join_paths(paths) {
        std::env::set_var("PATH", joined);
    }
}

pub fn ffmpeg_version(path: &std::path::Path) -> Option<String> {
    let out = quiet(path).arg("-version").output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let first = text.lines().next()?;
    first
        .split_whitespace()
        .nth(2)
        .map(|v| v.trim_start_matches('n').split('-').next().unwrap_or(v).to_owned())
}

pub fn ffmpeg() -> Outcome {
    match ffmpeg_on_path() {
        Some(path) => Outcome::Passed(ffmpeg_version(&path).unwrap_or_else(|| "found".to_owned())),
        None => Outcome::Failed(String::new()),
    }
}

pub fn folder(sources: &[Source]) -> Outcome {
    let live: Vec<&Source> = sources.iter().filter(|s| s.on).collect();
    if live.is_empty() {
        return Outcome::Failed(String::new());
    }
    let maps: u64 = live.iter().filter_map(|s| s.maps).sum();
    let replays: u64 = live.iter().map(|s| crate::sources::counted(s)).map(|s| s.replay_count + s.scores).sum();
    Outcome::Passed(format!("{maps} maps, {replays} replays"))
}

pub fn engine(server: &str, token: &str, name: &str) -> Outcome {
    if token.is_empty() {
        return Outcome::Passed(bot::BUILD.to_owned());
    }
    match bot::hello(server, token, name) {
        Ok(hello) if hello.agree || hello.build.is_empty() => Outcome::Passed(bot::BUILD.to_owned()),
        Ok(hello) => Outcome::Failed(hello.build),
        Err(why) => Outcome::Failed(why.to_string()),
    }
}

pub fn bot(server: &str, token: &str, name: &str) -> Outcome {
    if token.is_empty() {
        return Outcome::Skipped(String::new());
    }
    let started = std::time::Instant::now();
    match bot::hello(server, token, name) {
        Ok(_) => Outcome::Passed(format!("{} ms", started.elapsed().as_millis())),
        Err(why) => Outcome::Failed(why.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ffmpeg_banner_gives_up_its_version() {
        let dir = std::env::temp_dir().join(format!("dossier-ffmpeg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let fake = dir.join("ffmpeg");
        std::fs::write(&fake, "#!/bin/sh\necho 'ffmpeg version 7.1.1 Copyright (c) 2000-2025 the FFmpeg developers'\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
            assert_eq!(ffmpeg_version(&fake).as_deref(), Some("7.1.1"));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_sources_on_means_the_folder_check_fails() {
        assert_eq!(folder(&[]), Outcome::Failed(String::new()));
    }
}
