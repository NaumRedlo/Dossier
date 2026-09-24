use std::path::PathBuf;

use crate::bot;
use crate::sources::Source;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Passed(String),
    Failed(String),
    Skipped(String),
}

pub fn ffmpeg_on_path() -> Option<PathBuf> {
    let own = crate::ffmpeg::own();
    if own.is_file() {
        return Some(own);
    }
    let name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths)
                .map(|dir| dir.join(name))
                .find(|candidate| candidate.is_file())
        })
        .flatten()
}

pub fn ffmpeg_version(path: &std::path::Path) -> Option<String> {
    let out = std::process::Command::new(path).arg("-version").output().ok()?;
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
    let replays: u64 = live.iter().map(|s| crate::sources::counted(s).replay_count).sum();
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
