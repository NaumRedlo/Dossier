use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use serde::Deserialize;
use sha2::{Digest, Sha256};

pub const PAGE: &str = "https://github.com/NaumRedlo/Dossier/releases";
const LIST: &str = "https://api.github.com/repos/NaumRedlo/Dossier/releases?per_page=20";
const SUMS: &str = "SHA256SUMS";
pub const EVERY: Duration = Duration::from_secs(6 * 60 * 60);
pub const IDLE: Duration = Duration::from_secs(30 * 60);
const BIGGEST: u64 = 300 * 1024 * 1024;
const TELL_EVERY: Duration = Duration::from_millis(120);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub pre: bool,
    pub page: String,
    pub archive: String,
    pub url: String,
    pub size: u64,
    pub sums: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Staged {
    pub version: String,
    pub payload: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    Bundle(PathBuf),
    Binary(PathBuf),
    Source,
}

#[derive(Debug, Clone)]
pub enum Step {
    Downloading { done: u64, total: Option<u64> },
    Unpacking,
    Ready(Staged),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum State {
    #[default]
    Unknown,
    Checking,
    Latest { at: i64 },
    Found(Release),
    Getting { release: Release, done: u64, total: Option<u64> },
    Ready { release: Release, staged: Staged },
    Failed { release: Option<Release>, why: String },
    Source,
}

impl State {
    pub fn release(&self) -> Option<&Release> {
        match self {
            State::Found(release) | State::Getting { release, .. } | State::Ready { release, .. } | State::Failed { release: Some(release), .. } => Some(release),
            _ => None,
        }
    }

    pub fn waiting(&self) -> bool {
        self.release().is_some()
    }
}

#[derive(Deserialize)]
struct Wire {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

pub fn version(said: &str) -> Option<(u64, u64, u64)> {
    let mut parts = said.trim().trim_start_matches('v').split('.');
    let out = (parts.next()?.parse().ok()?, parts.next()?.parse().ok()?, parts.next()?.parse().ok()?);
    parts.next().is_none().then_some(out)
}

pub fn platform() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Some("macos-arm64"),
        ("macos", "x86_64") => Some("macos-x64"),
        ("windows", "x86_64") => Some("windows-x64"),
        ("linux", "x86_64") => Some("linux-x64"),
        _ => None,
    }
}

pub fn newest(json: &str, current: &str, platform: &str) -> Result<Option<Release>, String> {
    let list: Vec<Wire> = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let now = version(current).unwrap_or((0, 0, 0));
    let best = list
        .into_iter()
        .filter(|release| !release.draft)
        .filter_map(|release| {
            let at = version(&release.tag_name)?;
            let shown = format!("{}.{}.{}", at.0, at.1, at.2);
            let wanted = format!("dossier-{shown}-{platform}.zip");
            let archive = release.assets.iter().find(|asset| asset.name == wanted)?;
            let sums = release.assets.iter().find(|asset| asset.name == SUMS)?;
            Some((
                at,
                Release {
                    version: shown,
                    pre: release.prerelease,
                    page: release.html_url.clone(),
                    archive: wanted,
                    url: archive.browser_download_url.clone(),
                    size: archive.size,
                    sums: sums.browser_download_url.clone(),
                },
            ))
        })
        .filter(|(at, _)| *at > now)
        .max_by_key(|(at, _)| *at);
    Ok(best.map(|(_, release)| release))
}

fn client(patience: Duration) -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(patience)
        .user_agent(format!("Dossier/{}", crate::bot::BUILD))
        .build()
        .map_err(|e| e.to_string())
}

pub fn check() -> Result<Option<Release>, String> {
    let platform = platform().ok_or("no build for this system")?;
    let text = client(Duration::from_secs(30))?
        .get(LIST)
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .text()
        .map_err(|e| e.to_string())?;
    newest(&text, crate::bot::BUILD, platform)
}

pub fn place() -> Place {
    match std::env::current_exe().and_then(|exe| exe.canonicalize()) {
        Ok(exe) => place_of(&exe),
        Err(_) => Place::Source,
    }
}

pub fn place_of(exe: &Path) -> Place {
    let names: Vec<String> = exe.components().map(|part| part.as_os_str().to_string_lossy().into_owned()).collect();
    if let Some(at) = names.iter().position(|name| name == "target") {
        if names[at + 1..].iter().any(|name| name == "release" || name == "debug") {
            return Place::Source;
        }
    }
    let named = |dir: &Path, name: &str| dir.file_name().is_some_and(|n| n == name);
    let bundle = exe
        .parent()
        .filter(|dir| named(dir, "MacOS"))
        .and_then(Path::parent)
        .filter(|dir| named(dir, "Contents"))
        .and_then(Path::parent)
        .filter(|app| app.extension().is_some_and(|e| e == "app"));
    match bundle {
        Some(app) => Place::Bundle(app.to_path_buf()),
        None => Place::Binary(exe.to_path_buf()),
    }
}

fn shelf() -> PathBuf {
    crate::sources::own_root().join("updates")
}

fn binary_name() -> &'static str {
    if cfg!(windows) { "dossier.exe" } else { "dossier" }
}

fn payload_in(unpacked: &Path, place: &Place) -> Option<PathBuf> {
    let mut stack = vec![(unpacked.to_path_buf(), 0)];
    while let Some((dir, depth)) = stack.pop() {
        for entry in std::fs::read_dir(&dir).ok()?.flatten() {
            let path = entry.path();
            let is_dir = entry.file_type().is_ok_and(|kind| kind.is_dir());
            match place {
                Place::Bundle(_) if is_dir && path.extension().is_some_and(|e| e == "app") => return Some(path),
                Place::Binary(_) | Place::Source if !is_dir && entry.file_name() == binary_name() => return Some(path),
                _ => {}
            }
            if is_dir && depth < 2 {
                stack.push((path, depth + 1));
            }
        }
    }
    None
}

fn runnable(payload: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let binary = if payload.is_dir() { payload.join("Contents").join("MacOS").join("dossier") } else { payload.to_path_buf() };
        let _ = std::fs::set_permissions(binary, std::fs::Permissions::from_mode(0o755));
    }
    #[cfg(not(unix))]
    let _ = payload;
}

fn expected_sum(client: &reqwest::blocking::Client, release: &Release) -> Result<String, String> {
    let text = client.get(&release.sums).send().and_then(|r| r.error_for_status()).and_then(|r| r.text()).map_err(|e| e.to_string())?;
    sum_for(&text, &release.archive).ok_or_else(|| "the archive is not in its checksums".to_owned())
}

pub fn sum_for(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let sum = parts.next()?;
        let file = parts.next()?.trim_start_matches('*');
        (file == name && sum.len() == 64).then(|| sum.to_ascii_lowercase())
    })
}

fn download(client: &reqwest::blocking::Client, release: &Release, into: &Path, report: &mut dyn FnMut(Step) -> bool) -> Result<String, String> {
    let mut response = client.get(&release.url).send().and_then(|r| r.error_for_status()).map_err(|e| e.to_string())?;
    let total = response.content_length().or((release.size > 0).then_some(release.size));
    if total.is_some_and(|t| t > BIGGEST) {
        return Err("too big".to_owned());
    }
    let mut file = std::fs::File::create(into).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut done = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    let mut told = Instant::now() - TELL_EVERY;
    loop {
        let n = response.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
        hasher.update(&buffer[..n]);
        done += n as u64;
        if done > BIGGEST {
            return Err("too big".to_owned());
        }
        if told.elapsed() >= TELL_EVERY {
            told = Instant::now();
            if !report(Step::Downloading { done, total }) {
                return Err("stopped".to_owned());
            }
        }
    }
    file.flush().map_err(|e| e.to_string())?;
    report(Step::Downloading { done, total: Some(done) });
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

fn unpack(archive: &Path, into: &Path) -> Result<(), String> {
    let file = std::fs::File::open(archive).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    zip.extract(into).map_err(|e| e.to_string())
}

pub fn fetch(release: &Release, place: &Place, report: &mut dyn FnMut(Step) -> bool) {
    match stage(&shelf(), release, place, report) {
        Ok(staged) => report(Step::Ready(staged)),
        Err(why) => report(Step::Failed(why)),
    };
}

fn stage(shelf: &Path, release: &Release, place: &Place, report: &mut dyn FnMut(Step) -> bool) -> Result<Staged, String> {
    if *place == Place::Source {
        return Err("a build from the source is updated with git".to_owned());
    }
    let dir = shelf.join(&release.version);
    let unpacked = dir.join("unpacked");
    if dir.join("ready").is_file() {
        if let Some(payload) = payload_in(&unpacked, place) {
            return Ok(Staged { version: release.version.clone(), payload });
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let client = client(Duration::from_secs(1800))?;
    let expected = expected_sum(&client, release)?;
    let archive = dir.join(&release.archive);
    let got = download(&client, release, &archive, report)?;
    if got != expected {
        let _ = std::fs::remove_dir_all(&dir);
        return Err("the archive does not match its checksum".to_owned());
    }
    report(Step::Unpacking);
    unpack(&archive, &unpacked)?;
    let _ = std::fs::remove_file(&archive);
    let payload = payload_in(&unpacked, place).ok_or("the archive holds no application")?;
    runnable(&payload);
    std::fs::write(dir.join("ready"), &release.version).map_err(|e| e.to_string())?;
    Ok(Staged { version: release.version.clone(), payload })
}

fn remove(path: &Path) -> std::io::Result<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}

fn copy_all(from: &Path, to: &Path) -> std::io::Result<()> {
    if from.is_dir() {
        std::fs::create_dir_all(to)?;
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            copy_all(&entry.path(), &to.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        std::fs::copy(from, to).map(|_| ())
    }
}

fn shift(from: &Path, to: &Path) -> Result<(), String> {
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    copy_all(from, to).map_err(|e| format!("{}: {e}", to.display()))
}

fn aside_for(target: &Path) -> PathBuf {
    let name = target.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let first = target.with_file_name(format!("{name}.old"));
    if !first.exists() || remove(&first).is_ok() {
        first
    } else {
        target.with_file_name(format!("{name}.{}.old", std::process::id()))
    }
}

pub fn apply(staged: &Staged, place: &Place) -> Result<PathBuf, String> {
    let target = match place {
        Place::Bundle(app) => app.clone(),
        Place::Binary(exe) => exe.clone(),
        Place::Source => return Err("a build from the source is updated with git".to_owned()),
    };
    let aside = aside_for(&target);
    std::fs::rename(&target, &aside).map_err(|e| format!("{}: {e}", target.display()))?;
    if let Err(why) = shift(&staged.payload, &target) {
        let _ = remove(&target);
        let _ = std::fs::rename(&aside, &target);
        return Err(why);
    }
    runnable(&target);
    let _ = remove(&aside);
    let _ = std::fs::remove_dir_all(shelf());
    Ok(target)
}

pub fn launch(what: &Path, quiet: bool) -> Result<(), String> {
    let mut command = if what.extension().is_some_and(|e| e == "app") {
        let mut open = std::process::Command::new("open");
        open.arg("-n");
        if quiet {
            open.arg("-g");
        }
        open.arg(what);
        open
    } else {
        std::process::Command::new(what)
    };
    command.spawn().map(|_| ()).map_err(|e| e.to_string())
}

pub fn leftover(name: &str, of: &str) -> bool {
    let Some(rest) = name.strip_prefix(of).and_then(|rest| rest.strip_suffix(".old")) else {
        return false;
    };
    rest.is_empty() || rest.strip_prefix('.').is_some_and(|pid| !pid.is_empty() && pid.chars().all(|c| c.is_ascii_digit()))
}

pub fn tidy(place: &Place) {
    if let Place::Bundle(target) | Place::Binary(target) = place {
        if let (Some(dir), Some(name)) = (target.parent(), target.file_name()) {
            let name = name.to_string_lossy();
            for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                if leftover(&entry.file_name().to_string_lossy(), &name) {
                    let _ = remove(&entry.path());
                }
            }
        }
    }
    let now = version(crate::bot::BUILD).unwrap_or((0, 0, 0));
    for entry in std::fs::read_dir(shelf()).into_iter().flatten().flatten() {
        if version(&entry.file_name().to_string_lossy()).is_none_or(|at| at <= now) {
            let _ = remove(&entry.path());
        }
    }
}

fn staged_newer(place: &Place) -> Option<Staged> {
    let now = version(crate::bot::BUILD)?;
    std::fs::read_dir(shelf())
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let at = version(&name).filter(|at| *at > now)?;
            if !entry.path().join("ready").is_file() {
                return None;
            }
            let payload = payload_in(&entry.path().join("unpacked"), place)?;
            Some((at, Staged { version: name, payload }))
        })
        .max_by_key(|(at, _)| *at)
        .map(|(_, staged)| staged)
}

pub fn on_launch(quiet: bool) -> bool {
    let place = place();
    if place == Place::Source {
        return false;
    }
    tidy(&place);
    if !quiet {
        return false;
    }
    let Some(staged) = staged_newer(&place) else {
        return false;
    };
    match apply(&staged, &place) {
        Ok(what) => launch(&what, false).is_ok(),
        Err(_) => false,
    }
}

fn epoch() -> Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    *START.get_or_init(Instant::now)
}

static TOUCHED: AtomicU64 = AtomicU64::new(0);

pub fn touched() {
    TOUCHED.store(epoch().elapsed().as_millis() as u64, Ordering::Relaxed);
}

pub fn idle() -> Duration {
    Duration::from_millis((epoch().elapsed().as_millis() as u64).saturating_sub(TOUCHED.load(Ordering::Relaxed)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTED: &str = r#"[
        {"tag_name": "v0.91.0", "draft": true, "prerelease": false, "html_url": "d", "assets": [
            {"name": "dossier-0.91.0-windows-x64.zip", "browser_download_url": "https://x/d.zip", "size": 9},
            {"name": "SHA256SUMS", "browser_download_url": "https://x/ds", "size": 1}]},
        {"tag_name": "v0.90.1", "prerelease": true, "html_url": "p", "assets": [
            {"name": "dossier-0.90.1-windows-x64.zip", "browser_download_url": "https://x/p.zip", "size": 7},
            {"name": "SHA256SUMS", "browser_download_url": "https://x/ps", "size": 1}]},
        {"tag_name": "v0.90.2", "prerelease": true, "html_url": "n", "assets": [
            {"name": "dossier-0.90.2-windows-x64.zip", "browser_download_url": "https://x/n.zip", "size": 7}]},
        {"tag_name": "v0.90.0", "prerelease": false, "html_url": "s", "assets": [
            {"name": "dossier-0.90.0-windows-x64.zip", "browser_download_url": "https://x/s.zip", "size": 5},
            {"name": "SHA256SUMS", "browser_download_url": "https://x/ss", "size": 1}]},
        {"tag_name": "v0.89.4", "prerelease": true, "html_url": "o", "assets": [
            {"name": "dossier-0.89.4-windows-x64.zip", "browser_download_url": "https://x/o.zip", "size": 5}]}
    ]"#;

    #[test]
    fn the_newest_release_with_this_system_and_its_checksums_is_offered_pre_releases_included() {
        let found = newest(LISTED, "0.89.5", "windows-x64").unwrap().expect("an update");
        assert_eq!(found.version, "0.90.1");
        assert!(found.pre);
        assert_eq!(found.url, "https://x/p.zip");
        assert_eq!(found.sums, "https://x/ps");
        assert_eq!(found.archive, "dossier-0.90.1-windows-x64.zip");
    }

    #[test]
    fn nothing_is_offered_to_the_newest_or_to_a_system_without_a_build() {
        assert_eq!(newest(LISTED, "0.90.1", "windows-x64").unwrap(), None);
        assert_eq!(newest(LISTED, "0.89.5", "linux-x64").unwrap(), None);
    }

    #[test]
    fn versions_compare_as_numbers() {
        assert!(version("v0.100.0") > version("0.99.9"));
        assert_eq!(version("0.89"), None);
        assert_eq!(version("0.89.4-rc"), None);
    }

    #[test]
    fn a_checksum_is_found_by_its_file() {
        let sums = "ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef0123456789  dossier-0.90.0-linux-x64.zip\n0000000000000000000000000000000000000000000000000000000000000000 *dossier-0.90.0-windows-x64.zip\n";
        assert_eq!(sum_for(sums, "dossier-0.90.0-linux-x64.zip").as_deref(), Some("abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"));
        assert_eq!(sum_for(sums, "dossier-0.90.0-windows-x64.zip").map(|s| s.len()), Some(64));
        assert_eq!(sum_for(sums, "dossier-0.90.0-macos-arm64.zip"), None);
    }

    #[test]
    fn where_the_application_lives_decides_what_is_replaced() {
        assert_eq!(place_of(Path::new("/Applications/Dossier.app/Contents/MacOS/dossier")), Place::Bundle(PathBuf::from("/Applications/Dossier.app")));
        assert_eq!(place_of(Path::new("/home/n/apps/dossier/dossier")), Place::Binary(PathBuf::from("/home/n/apps/dossier/dossier")));
        assert_eq!(place_of(Path::new("/home/n/Dossier/native/target/release/dossier")), Place::Source);
        assert_eq!(place_of(Path::new("/home/n/Dossier/native/target/x86_64-apple-darwin/debug/dossier")), Place::Source);
    }

    #[test]
    fn only_what_an_update_put_aside_is_cleared() {
        assert!(leftover("dossier.exe.old", "dossier.exe"));
        assert!(leftover("dossier.exe.4411.old", "dossier.exe"));
        assert!(leftover("Dossier.app.old", "Dossier.app"));
        assert!(!leftover("dossier-notes.old", "dossier"));
        assert!(!leftover("dossier.backup.old", "dossier"));
        assert!(!leftover("dossier.exe", "dossier.exe"));
    }

    #[test]
    fn an_update_swaps_the_binary_and_keeps_nothing_aside() {
        let root = std::env::temp_dir().join(format!("dossier-update-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("app")).unwrap();
        std::fs::create_dir_all(root.join("new")).unwrap();
        let exe = root.join("app").join("dossier-test");
        std::fs::write(&exe, b"old").unwrap();
        std::fs::write(root.join("new").join("dossier-test"), b"new").unwrap();
        let staged = Staged { version: "9.9.9".to_owned(), payload: root.join("new").join("dossier-test") };
        let place = Place::Binary(exe.clone());
        assert_eq!(apply(&staged, &place).unwrap(), exe);
        assert_eq!(std::fs::read(&exe).unwrap(), b"new");
        assert!(!root.join("app").join("dossier-test.old").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_zip_of_a_bundle_unpacks_to_the_bundle() {
        let root = std::env::temp_dir().join(format!("dossier-bundle-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let archive = root.join("a.zip");
        {
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
            let options = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
            zip.add_directory("dossier-9.9.9-macos-arm64/Dossier.app/Contents/MacOS/", options).unwrap();
            zip.start_file("dossier-9.9.9-macos-arm64/Dossier.app/Contents/MacOS/dossier", options).unwrap();
            zip.write_all(b"binary").unwrap();
            zip.start_file("dossier-9.9.9-macos-arm64/licenses/LICENSE.txt", options).unwrap();
            zip.write_all(b"text").unwrap();
            zip.finish().unwrap();
        }
        unpack(&archive, &root.join("unpacked")).unwrap();
        let bundle = payload_in(&root.join("unpacked"), &Place::Bundle(PathBuf::from("/Applications/Dossier.app"))).expect("the bundle");
        assert!(bundle.ends_with("Dossier.app"));
        assert_eq!(std::fs::read(bundle.join("Contents/MacOS/dossier")).unwrap(), b"binary");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    #[ignore = "downloads the newest release from GitHub"]
    fn the_newest_release_downloads_matches_its_checksum_and_unpacks() {
        let text = client(Duration::from_secs(30))
            .unwrap()
            .get(LIST)
            .header("Accept", "application/vnd.github+json")
            .send()
            .and_then(|r| r.error_for_status())
            .and_then(|r| r.text())
            .unwrap();
        let release = newest(&text, "0.0.1", platform().unwrap()).unwrap().expect("a release with this system and checksums");
        let shelf = std::env::temp_dir().join(format!("dossier-shelf-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&shelf);
        let place = if cfg!(target_os = "macos") { Place::Bundle(PathBuf::from("/Applications/Dossier.app")) } else { Place::Binary(PathBuf::from(binary_name())) };
        let mut steps = Vec::new();
        let staged = stage(&shelf, &release, &place, &mut |step| {
            steps.push(format!("{step:?}"));
            true
        })
        .unwrap();
        println!("{} → {}", release.version, staged.payload.display());
        assert!(staged.payload.exists());
        assert!(steps.iter().any(|s| s.starts_with("Downloading")));
        let again = stage(&shelf, &release, &place, &mut |_| true).unwrap();
        assert_eq!(again, staged, "a staged update is taken from the shelf, not downloaded again");
        let _ = std::fs::remove_dir_all(&shelf);
    }

    #[test]
    fn idleness_starts_over_at_a_touch() {
        touched();
        assert!(idle() < Duration::from_secs(1));
    }
}
