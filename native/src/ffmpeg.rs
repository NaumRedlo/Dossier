use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub const HOME: &str = "https://ffmpeg.org/download.html";

pub const VERSION: &str = "9.0.2";

pub struct Build {
    pub from: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
}

const RIEDL_MAC_ARM: Build = Build {
    from: "martin-riedl.de",
    url: "https://ffmpeg.martin-riedl.de/download/macos/arm64/1789931890_9.0.2/ffmpeg.zip",
    sha256: "c8ed4c4e6978a03c485edbfe4e0a5dc2380f8a30bba5150531b31b094492d924",
};
const OSXEXPERTS_MAC_ARM: Build = Build {
    from: "osxexperts.net",
    url: "https://www.osxexperts.net/ffmpeg71arm.zip",
    sha256: "0878f3313311c2c1b2c818e7c955c0bd828c97b357fa86211b42a5c36d01e36f",
};
const RIEDL_MAC_INTEL: Build = Build {
    from: "martin-riedl.de",
    url: "https://ffmpeg.martin-riedl.de/download/macos/amd64/1789931006_9.0.2/ffmpeg.zip",
    sha256: "7c6b4125b191cbf773832dc51f424cf2b6bb7da43007d1e066f95909e47cacd4",
};
const RIEDL_LINUX: Build = Build {
    from: "martin-riedl.de",
    url: "https://ffmpeg.martin-riedl.de/download/linux/amd64/1789931100_9.0.2/ffmpeg.zip",
    sha256: "fa8ecf4abbd290d98f7d188b8649cc6b391ae209a98452be955a15aab1909d7f",
};
const RIEDL_LINUX_ARM: Build = Build {
    from: "martin-riedl.de",
    url: "https://ffmpeg.martin-riedl.de/download/linux/arm64/1789931697_9.0.2/ffmpeg.zip",
    sha256: "93a76ae90db5474eecdf951a729857c64f3de23567228d6a7d5e6e8e3cd1021b",
};
const WINDOWS: &str = "60f467265b1e312373dbcd92200c2618a74850f98d3d078e94296bb3fa2047ba";
const GYAN: Build = Build {
    from: "gyan.dev",
    url: "https://www.gyan.dev/ffmpeg/builds/packages/ffmpeg-9.0.2-essentials_build.zip",
    sha256: WINDOWS,
};
const GYAN_GITHUB: Build = Build {
    from: "github.com/GyanD",
    url: "https://github.com/GyanD/codexffmpeg/releases/download/9.0.2/ffmpeg-9.0.2-essentials_build.zip",
    sha256: WINDOWS,
};

pub fn builds() -> &'static [Build] {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => &[GYAN, GYAN_GITHUB],
        ("macos", "aarch64") => &[RIEDL_MAC_ARM, OSXEXPERTS_MAC_ARM],
        ("macos", "x86_64") => &[RIEDL_MAC_INTEL],
        ("linux", "x86_64") => &[RIEDL_LINUX],
        ("linux", "aarch64") => &[RIEDL_LINUX_ARM],
        _ => &[],
    }
}

pub fn binary_name() -> &'static str {
    if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" }
}

pub fn own_dir() -> PathBuf {
    crate::sources::own_root().join("bin")
}

pub fn own() -> PathBuf {
    own_dir().join(binary_name())
}

#[derive(Debug, Clone)]
pub enum Step {
    Downloading { from: &'static str, done: u64, total: Option<u64> },
    Unpacking { from: &'static str },
    Done(PathBuf),
    Failed(String),
}

const BIGGEST: u64 = 400 * 1024 * 1024;

pub const NOT_PINNED: &str = "the download is not the pinned build";

const EVERY: Duration = Duration::from_millis(120);

pub fn fetch(report: impl FnMut(Step)) {
    fetch_into(&own_dir(), report)
}

pub fn fetch_into(dir: &std::path::Path, mut report: impl FnMut(Step)) {
    let mut last = String::new();
    for build in builds() {
        match fetch_from(build, dir, &mut report) {
            Ok(path) => {
                report(Step::Done(path));
                return;
            }
            Err(why) => last = format!("{}: {}", build.from, why),
        }
    }
    report(Step::Failed(if last.is_empty() { "no build for this system".to_owned() } else { last }));
}

fn fetch_from(build: &Build, dir: &std::path::Path, report: &mut impl FnMut(Step)) -> Result<PathBuf, String> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(1800))
        .user_agent(format!("Dossier/{}", crate::bot::BUILD))
        .build()
        .map_err(|e| e.to_string())?;
    let mut response = client
        .get(build.url)
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let total = response.content_length();
    if total.is_some_and(|t| t > BIGGEST) {
        return Err("too big".to_owned());
    }
    let target = dir.join(binary_name());
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let archive = dir.join("ffmpeg.download");
    let _ = std::fs::remove_file(&archive);
    let mut file = std::fs::File::create(&archive).map_err(|e| e.to_string())?;
    let mut done = 0u64;
    let mut hasher = <sha2::Sha256 as sha2::Digest>::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut said = Instant::now() - EVERY;
    report(Step::Downloading { from: build.from, done, total });
    loop {
        let n = response.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
        sha2::Digest::update(&mut hasher, &buffer[..n]);
        done += n as u64;
        if done > BIGGEST {
            let _ = std::fs::remove_file(&archive);
            return Err("too big".to_owned());
        }
        if said.elapsed() >= EVERY {
            said = Instant::now();
            report(Step::Downloading { from: build.from, done, total });
        }
    }
    file.flush().map_err(|e| e.to_string())?;
    drop(file);
    let sum: String = sha2::Digest::finalize(hasher).iter().map(|b| format!("{b:02x}")).collect();
    if sum != build.sha256 {
        let _ = std::fs::remove_file(&archive);
        return Err(NOT_PINNED.to_owned());
    }
    report(Step::Unpacking { from: build.from });
    let fresh = dir.join(format!("{}.new", binary_name()));
    let unpacked = unpack(&archive, &fresh);
    let _ = std::fs::remove_file(&archive);
    unpacked?;
    if crate::checks::ffmpeg_version(&fresh).is_none() {
        let _ = std::fs::remove_file(&fresh);
        return Err("the file does not run".to_owned());
    }
    std::fs::rename(&fresh, &target).map_err(|e| e.to_string())?;
    crate::checks::vouch_for(&target);
    Ok(target)
}

fn unpack(archive: &std::path::Path, target: &std::path::Path) -> Result<(), String> {
    let file = std::fs::File::open(archive).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let wanted = binary_name();
    let at = (0..zip.len())
        .find(|i| {
            zip.by_index(*i).ok().is_some_and(|entry| {
                !entry.is_dir()
                    && entry
                        .enclosed_name()
                        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                        .is_some_and(|name| name == wanted)
            })
        })
        .ok_or("no ffmpeg inside")?;
    let mut entry = zip.by_index(at).map_err(|e| e.to_string())?;
    let mut out = std::fs::File::create(target).map_err(|e| e.to_string())?;
    std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
    drop(out);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(target, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn this_system_has_a_build_to_offer() {
        assert!(!builds().is_empty());
        for build in builds() {
            assert!(build.url.starts_with("https://"));
            assert!(build.url.ends_with(".zip"));
        }
    }

    #[test]
    fn every_build_is_pinned_to_a_version_and_a_sha256() {
        for build in [RIEDL_MAC_ARM, OSXEXPERTS_MAC_ARM, RIEDL_MAC_INTEL, RIEDL_LINUX, RIEDL_LINUX_ARM, GYAN, GYAN_GITHUB] {
            assert!(!build.url.contains("latest") && !build.url.contains("release-essentials"), "{}", build.url);
            assert!(build.url.contains(VERSION) || build.from == "osxexperts.net", "{}", build.url);
            assert_eq!(build.sha256.len(), 64, "{}", build.url);
            assert!(build.sha256.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()), "{}", build.url);
        }
        assert_eq!(GYAN.sha256, GYAN_GITHUB.sha256, "both Windows sources serve the same archive");
    }

    #[test]
    #[ignore = "downloads tens of megabytes from the network"]
    fn a_build_downloads_unpacks_and_runs() {
        let dir = std::env::temp_dir().join(format!("dossier-ffmpeg-{}", std::process::id()));
        let mut steps = Vec::new();
        fetch_into(&dir, |step| steps.push(step));
        let last = steps.last().expect("a last step");
        assert!(matches!(last, Step::Done(_)), "{last:?}");
        assert!(steps.iter().any(|s| matches!(s, Step::Downloading { done, .. } if *done > 0)));
        assert!(steps.iter().any(|s| matches!(s, Step::Unpacking { .. })));
        assert!(crate::checks::ffmpeg_version(&dir.join(binary_name())).is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_own_binary_lives_in_the_own_folder() {
        let path = own();
        assert!(path.starts_with(crate::sources::own_root()));
        assert_eq!(path.parent().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()), Some("bin".to_owned()));
    }
}
