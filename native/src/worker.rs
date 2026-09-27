use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::bot;
use crate::render;

#[derive(Debug, Clone)]
pub struct Setup {
    pub server: String,
    pub token: String,
    pub name: String,
    pub ffmpeg: PathBuf,
    pub songs: Vec<PathBuf>,
    pub own_songs: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Getting {
    Replay,
    Map,
    Skin,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Waiting { waiting: u32 },
    Resting,
    Taken { title: String },
    Getting { title: String, what: Getting },
    Drawing { title: String, done: u64, of: u64, left_seconds: f64, fps: f64 },
    Polishing { title: String },
    Sending { title: String, done: u64, of: u64 },
    Delivered { title: String },
    HandedBack { title: String, reason: String },
    Offline(String),
    Stopped,
}

static ON: AtomicBool = AtomicBool::new(false);
static DRAWING: AtomicBool = AtomicBool::new(false);
static HOLDING: AtomicBool = AtomicBool::new(false);

const RESTING: Duration = Duration::from_secs(8);
const QUIET: Duration = Duration::from_secs(20);
const BEAT: Duration = Duration::from_secs(15);

pub fn stop() {
    ON.store(false, Ordering::SeqCst);
}

pub fn drawing() -> bool {
    DRAWING.load(Ordering::SeqCst)
}

pub fn holding() -> bool {
    HOLDING.load(Ordering::SeqCst)
}

pub fn run(setup: Setup) -> iced::Task<Step> {
    crate::ui::streamed(move |push| work(&setup, push))
}

fn nap(for_how_long: Duration) -> bool {
    let until = Instant::now() + for_how_long;
    while Instant::now() < until {
        if !ON.load(Ordering::SeqCst) {
            return false;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    ON.load(Ordering::SeqCst)
}

fn work(setup: &Setup, push: &mut dyn FnMut(Step) -> bool) {
    ON.store(true, Ordering::SeqCst);
    while ON.load(Ordering::SeqCst) {
        if render::busy() {
            let _ = bot::claim(&setup.server, &setup.token, &setup.name, false);
            if !push(Step::Resting) || !nap(RESTING) {
                break;
            }
            continue;
        }
        match bot::claim(&setup.server, &setup.token, &setup.name, true) {
            Ok(Some(job)) => {
                HOLDING.store(true, Ordering::SeqCst);
                let title = job.title.clone();
                if !push(Step::Taken { title: title.clone() }) {
                    let _ = bot::give_back(&setup.server, &setup.token, &setup.name, &job.id, "the application closed");
                    HOLDING.store(false, Ordering::SeqCst);
                    break;
                }
                let outcome = one(setup, &job, push);
                HOLDING.store(false, Ordering::SeqCst);
                match outcome {
                    Ok(()) => {
                        if !push(Step::Delivered { title }) {
                            break;
                        }
                    }
                    Err(reason) => {
                        let _ = bot::give_back(&setup.server, &setup.token, &setup.name, &job.id, &reason);
                        if !push(Step::HandedBack { title, reason }) {
                            break;
                        }
                    }
                }
            }
            Ok(None) => {
                let waiting = bot::hello(&setup.server, &setup.token, &setup.name).map_or(0, |hello| hello.waiting);
                if !push(Step::Waiting { waiting }) || !nap(RESTING) {
                    break;
                }
            }
            Err(why) => {
                if !push(Step::Offline(why.to_string())) || !nap(QUIET) {
                    break;
                }
            }
        }
    }
    let _ = bot::claim(&setup.server, &setup.token, &setup.name, false);
    ON.store(false, Ordering::SeqCst);
    push(Step::Stopped);
}

fn place() -> PathBuf {
    crate::sources::own_root().join("cache").join("farm")
}

fn one(setup: &Setup, job: &bot::Job, push: &mut dyn FnMut(Step) -> bool) -> Result<(), String> {
    let folder = place().join(&job.id);
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let outcome = draw_and_send(setup, job, &folder, push);
    let _ = std::fs::remove_dir_all(&folder);
    outcome
}

fn draw_and_send(setup: &Setup, job: &bot::Job, folder: &Path, push: &mut dyn FnMut(Step) -> bool) -> Result<(), String> {
    let title = job.title.clone();
    let progress = Arc::new(Mutex::new(serde_json::json!({"stage": "getting"})));
    let lost = Arc::new(AtomicBool::new(false));
    let finished = Arc::new(AtomicBool::new(false));
    let beating = {
        let (server, token, name, id) = (setup.server.clone(), setup.token.clone(), setup.name.clone(), job.id.clone());
        let (progress, lost, finished) = (progress.clone(), lost.clone(), finished.clone());
        std::thread::spawn(move || {
            let mut last = Instant::now();
            while !finished.load(Ordering::SeqCst) {
                if last.elapsed() >= BEAT {
                    last = Instant::now();
                    let said = progress.lock().map(|p| p.clone()).unwrap_or_default();
                    if let Ok(false) = bot::heartbeat(&server, &token, &name, &id, &said) {
                        lost.store(true, Ordering::SeqCst);
                        render::stop();
                        return;
                    }
                }
                std::thread::sleep(Duration::from_millis(250));
            }
        })
    };
    let outcome = steps(setup, job, folder, &title, &progress, &lost, push);
    finished.store(true, Ordering::SeqCst);
    let _ = beating.join();
    if lost.load(Ordering::SeqCst) {
        return Err("the bot gave the job to someone else".to_owned());
    }
    outcome
}

fn steps(
    setup: &Setup,
    job: &bot::Job,
    folder: &Path,
    title: &str,
    progress: &Arc<Mutex<serde_json::Value>>,
    lost: &Arc<AtomicBool>,
    push: &mut dyn FnMut(Step) -> bool,
) -> Result<(), String> {
    let gone = |push: &mut dyn FnMut(Step) -> bool, step: Step| -> Result<(), String> {
        if !push(step) || !ON.load(Ordering::SeqCst) {
            return Err("the worker was switched off".to_owned());
        }
        Ok(())
    };
    gone(push, Step::Getting { title: title.to_owned(), what: Getting::Replay })?;
    let replay = folder.join("replay.osr");
    bot::job_file(&setup.server, &setup.token, &setup.name, &job.id, "replay", &replay).map_err(|e| format!("the replay: {e}"))?;
    gone(push, Step::Getting { title: title.to_owned(), what: Getting::Map })?;
    let map = map_for(setup, &job.beatmap_md5)?;
    let skin = match &job.skin {
        Some(skin) => {
            gone(push, Step::Getting { title: title.to_owned(), what: Getting::Skin })?;
            Some(skin_for(setup, job, skin)?)
        }
        None => None,
    };
    let raw = folder.join("drawn.mp4");
    let ask = render::Ask {
        replay: replay.clone(),
        map,
        map_hash: job.beatmap_md5.clone(),
        ffmpeg: setup.ffmpeg.clone(),
        out: raw.clone(),
        size: (job.settings.width.max(640), job.settings.height.max(360)),
        fps: job.settings.fps.clamp(24, 240),
        crf: 20,
        skin,
        music_level: 1.0,
        hitsound_level: 1.0,
        play: render::Play::default(),
    };
    DRAWING.store(true, Ordering::SeqCst);
    let started = Instant::now();
    let mut last: Option<render::Step> = None;
    render::perform(ask, &mut |step| {
        if let render::Step::Drawing { frames, of, left_seconds } = &step {
            let fps = *frames as f64 / started.elapsed().as_secs_f64().max(0.001);
            if let Ok(mut said) = progress.lock() {
                *said = serde_json::json!({"stage": "drawing", "done": frames, "total": of, "seconds_left": left_seconds, "fps": fps});
            }
            let keep = push(Step::Drawing { title: title.to_owned(), done: *frames, of: *of, left_seconds: *left_seconds, fps });
            if !keep || !ON.load(Ordering::SeqCst) || lost.load(Ordering::SeqCst) {
                render::stop();
            }
        }
        last = Some(step);
        true
    });
    DRAWING.store(false, Ordering::SeqCst);
    match last {
        Some(render::Step::Saved(..)) => {}
        Some(render::Step::Failed(why)) => return Err(why),
        _ => return Err("the render was stopped".to_owned()),
    }
    gone(push, Step::Polishing { title: title.to_owned() })?;
    if let Ok(mut said) = progress.lock() {
        *said = serde_json::json!({"stage": "polishing"});
    }
    let level = if job.settings.loudness < 0.0 { job.settings.loudness } else { -14.0 };
    let even = folder.join("even.mp4");
    loudness(&setup.ffmpeg, &raw, &even, level)?;
    let length = length_of(&replay);
    let sent = if job.most > 0 && std::fs::metadata(&even).map(|m| m.len()).unwrap_or(0) > job.most {
        let fitted = folder.join("fitted.mp4");
        fit(&setup.ffmpeg, &even, &fitted, job.most, length)?;
        fitted
    } else {
        even
    };
    if let Ok(mut said) = progress.lock() {
        *said = serde_json::json!({"stage": "sending"});
    }
    let size = std::fs::metadata(&sent).map(|m| m.len()).unwrap_or(0);
    gone(push, Step::Sending { title: title.to_owned(), done: 0, of: size })?;
    let meta = serde_json::json!({"duration": length.as_secs(), "width": job.settings.width, "height": job.settings.height});
    bot::deliver(&setup.server, &setup.token, &setup.name, &job.id, &sent, &meta, |_| {}).map_err(|e| format!("sending: {e}"))
}

fn map_for(setup: &Setup, hash: &str) -> Result<PathBuf, String> {
    let index = crate::library::Index::load(&setup.songs);
    if let Some(map) = index.by_hash.get(&hash.to_ascii_lowercase()) {
        return Ok(map.file.clone());
    }
    match crate::maps::bring(hash, &setup.own_songs, &mut |_| ON.load(Ordering::SeqCst)) {
        crate::maps::Step::Done(map) => Ok(map.file),
        crate::maps::Step::Nowhere => Err("no mirror has the map".to_owned()),
        crate::maps::Step::Failed(why) => Err(format!("the map: {why}")),
        _ => Err("the map was not fetched".to_owned()),
    }
}

fn skin_for(setup: &Setup, job: &bot::Job, skin: &bot::JobSkin) -> Result<PathBuf, String> {
    let skins = crate::sources::own_root().join("cache").join("farm-skins");
    prepare_skin(&skins, skin, |archive| {
        bot::job_file_limited(&setup.server, &setup.token, &setup.name, &job.id, "skin", archive, SKIN_ARCHIVE_LIMIT)
            .map_err(|e| e.to_string())
    }).map_err(|why| format!("the skin: {why}"))
}

const SKIN_ARCHIVE_LIMIT: u64 = 256 * 1024 * 1024;
const SKIN_EXPANDED_LIMIT: u64 = 512 * 1024 * 1024;
const SKIN_FILES_LIMIT: usize = 8192;
const SKIN_READY: &str = ".dossier-ready-v1";

fn prepare_skin(cache: &Path, skin: &bot::JobSkin, fetch: impl FnOnce(&Path) -> Result<u64, String>) -> Result<PathBuf, String> {
    let hash = skin.hash.to_ascii_lowercase();
    if !matches!(hash.len(), 32 | 64) || !hash.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("expected the MD5 or SHA-256 of the skin archive".into());
    }
    if skin.size > SKIN_ARCHIVE_LIMIT { return Err("the skin archive is too large".into()); }
    let destination = cache.join(&hash);
    if std::fs::read_to_string(destination.join(SKIN_READY)).ok().as_deref() == Some(hash.as_str()) {
        if let Ok(root) = skin_root(&destination) { return Ok(root); }
    }
    std::fs::create_dir_all(cache).map_err(|e| e.to_string())?;
    let staging = cache.join(format!(".{hash}.part"));
    if staging.exists() { std::fs::remove_dir_all(&staging).map_err(|e| e.to_string())?; }
    std::fs::create_dir(&staging).map_err(|e| e.to_string())?;
    let result = (|| {
        let archive = staging.join("skin.osk");
        fetch(&archive)?;
        let size = std::fs::metadata(&archive).map_err(|e| e.to_string())?.len();
        if size == 0 || size > SKIN_ARCHIVE_LIMIT { return Err("the skin archive is empty or too large".into()); }
        if skin.size > 0 && size != skin.size { return Err("the skin archive size does not match the job".into()); }
        use sha2::Digest;
        let mut file = std::fs::File::open(&archive).map_err(|e| e.to_string())?;
        let actual = if hash.len() == 64 {
            let mut digest = sha2::Sha256::new();
            std::io::copy(&mut file, &mut digest).map_err(|e| e.to_string())?;
            format!("{:x}", digest.finalize())
        } else {
            let mut digest = md5::Md5::new();
            std::io::copy(&mut file, &mut digest).map_err(|e| e.to_string())?;
            format!("{:x}", digest.finalize())
        };
        if actual != hash { return Err("the skin archive checksum does not match the job".into()); }
        let extracted = staging.join("files");
        unpack_worker_skin(&archive, &extracted)?;
        let root = skin_root(&extracted)?;
        let relative = root.strip_prefix(&extracted).map_err(|e| e.to_string())?.to_path_buf();
        std::fs::write(extracted.join(SKIN_READY), &hash).map_err(|e| e.to_string())?;
        if destination.exists() { std::fs::remove_dir_all(&destination).map_err(|e| e.to_string())?; }
        std::fs::rename(&extracted, &destination).map_err(|e| e.to_string())?;
        Ok(destination.join(relative))
    })();
    let _ = std::fs::remove_dir_all(&staging);
    result
}

fn unpack_worker_skin(archive: &Path, into: &Path) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(archive).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    if zip.len() > SKIN_FILES_LIMIT { return Err("the skin has too many files".into()); }
    let mut remaining = SKIN_EXPANDED_LIMIT;
    for at in 0..zip.len() {
        let mut entry = zip.by_index(at).map_err(|e| e.to_string())?;
        let relative = entry.enclosed_name().ok_or("the skin contains an invalid path")?;
        if entry.name().contains('\\') || relative.components().any(|c| c.as_os_str().to_string_lossy().contains(':')) {
            return Err("the skin contains an invalid path".into());
        }
        if entry.unix_mode().is_some_and(|mode| mode & 0o170000 == 0o120000) { return Err("the skin contains a symbolic link".into()); }
        if entry.is_dir() { continue; }
        if entry.size() > remaining { return Err("the expanded skin is too large".into()); }
        let target = into.join(relative);
        std::fs::create_dir_all(target.parent().ok_or("the skin contains an invalid path")?).map_err(|e| e.to_string())?;
        let mut out = std::fs::File::create(&target).map_err(|e| e.to_string())?;
        let written = std::io::copy(&mut std::io::Read::take(&mut entry, remaining + 1), &mut out).map_err(|e| e.to_string())?;
        remaining = remaining.checked_sub(written).ok_or("the expanded skin is too large")?;
    }
    Ok(())
}

fn skin_root(folder: &Path) -> Result<PathBuf, String> {
    if crate::settings::looks_like_skin(folder) { return Ok(folder.to_path_buf()); }
    let mut pending = vec![folder.to_path_buf()];
    let mut found = None;
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if !entry.file_type().map_err(|e| e.to_string())?.is_dir() { continue; }
            let path = entry.path();
            if crate::settings::looks_like_skin(&path) {
                if found.is_some() { return Err("the archive contains several skins; send one skin".into()); }
                found = Some(path);
            } else { pending.push(path); }
        }
    }
    found.ok_or_else(|| "the archive does not contain a skin".into())
}

fn length_of(replay: &Path) -> Duration {
    std::fs::read(replay)
        .ok()
        .and_then(|bytes| dossier_replay::Replay::parse(&bytes).ok())
        .map_or(Duration::ZERO, |r| Duration::from_millis(r.duration_ms().max(0) as u64))
}

fn ffmpeg(binary: &Path, args: &[&str]) -> Result<(), String> {
    let mut command = crate::checks::quiet(binary);
    command.args(["-hide_banner", "-loglevel", "error", "-y"]);
    match (render::threads().1, args.split_last()) {
        (Some(n), Some((out, head))) => {
            command.args(head).args(["-threads", &n.to_string()]).arg(out);
        }
        _ => {
            command.args(args);
        }
    }
    let done = command.output().map_err(|e| e.to_string())?;
    if done.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&done.stderr).lines().last().unwrap_or("ffmpeg failed").to_owned())
    }
}

pub fn loudness_filter(level: f64) -> String {
    format!("loudnorm=I={level:.1}:TP=-1.5:LRA=11")
}

fn loudness(binary: &Path, from: &Path, into: &Path, level: f64) -> Result<(), String> {
    let filter = loudness_filter(level);
    ffmpeg(binary, &["-i", &from.to_string_lossy(), "-c:v", "copy", "-af", &filter, "-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart", &into.to_string_lossy()])
        .map_err(|why| format!("the loudness: {why}"))
}

pub fn bitrate_for(most: u64, length: Duration) -> u64 {
    let seconds = length.as_secs_f64().max(1.0);
    let room = (most as f64 * 0.92 * 8.0 / seconds) - 192_000.0;
    room.max(300_000.0) as u64
}

fn fit(binary: &Path, from: &Path, into: &Path, most: u64, length: Duration) -> Result<(), String> {
    let rate = bitrate_for(most, length);
    let rate_said = format!("{rate}");
    let buffer = format!("{}", rate * 2);
    ffmpeg(
        binary,
        &["-i", &from.to_string_lossy(), "-c:v", "libx264", "-preset", "veryfast", "-b:v", &rate_said, "-maxrate", &rate_said, "-bufsize", &buffer, "-c:a", "copy", "-movflags", "+faststart", &into.to_string_lossy()],
    )
    .map_err(|why| format!("fitting the video: {why}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skin_archive(files: &[(&str, &[u8])]) -> Vec<u8> {
        use std::io::Write;
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, bytes) in files {
            zip.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    fn skin_case(bytes: &[u8]) -> (PathBuf, bot::JobSkin) {
        use sha2::Digest;
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!("dossier-worker-skin-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
        (root, bot::JobSkin { name: "Player skin".into(), hash: format!("{:x}", sha2::Sha256::digest(bytes)), size: bytes.len() as u64 })
    }

    #[test]
    fn worker_skin_is_verified_cached_and_found_in_a_nested_folder() {
        let mut picture = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut picture, 16, 16);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&[12, 34, 56, 255].repeat(256)).unwrap();
        }
        let bytes = skin_archive(&[("My skin/skin.ini", b"[General]\nName: Player\n[Colours]\nCombo1: 3,17,99"), ("My skin/cursor.png", &picture)]);
        let (root, mut skin) = skin_case(&bytes);
        skin.hash = skin.hash.to_uppercase();
        let loaded = prepare_skin(&root, &skin, |path| { std::fs::write(path, &bytes).unwrap(); Ok(bytes.len() as u64) }).unwrap();
        assert_eq!(std::fs::read(loaded.join("cursor.png")).unwrap(), picture);
        assert_eq!(loaded.file_name().unwrap(), "My skin");
        assert_eq!(prepare_skin(&root, &skin, |_| panic!("verified skin should not be downloaded again")).unwrap(), loaded);
        let rendered_skin = dossier_produce::skin::from_folder(dossier_render::Skin::default(), &loaded, None);
        assert!((rendered_skin.combo_colours[0].red() - 3.0 / 255.0).abs() < 1e-6);
        assert!(rendered_skin.sprites.as_ref().unwrap().get(dossier_render::elements::Element::Cursor).is_some(), "the renderer must import the uploaded cursor");
        use sha2::Digest;
        skin.hash = format!("{:x}", md5::Md5::digest(&bytes));
        assert!(prepare_skin(&root, &skin, |path| { std::fs::write(path, &bytes).unwrap(); Ok(bytes.len() as u64) }).unwrap().join("skin.ini").is_file());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn an_interrupted_skin_cache_is_replaced_and_failed_downloads_are_cleaned() {
        let bytes = skin_archive(&[("skin.ini", b"[General]")]);
        let (root, skin) = skin_case(&bytes);
        let stale = root.join(&skin.hash);
        std::fs::create_dir_all(&stale).unwrap();
        std::fs::write(stale.join("partial.png"), b"partial").unwrap();
        assert!(prepare_skin(&root, &skin, |path| { std::fs::write(path, b"partial").unwrap(); Err("disconnected".into()) }).is_err());
        assert!(!root.join(format!(".{}.part", skin.hash)).exists());
        let loaded = prepare_skin(&root, &skin, |path| { std::fs::write(path, &bytes).unwrap(); Ok(bytes.len() as u64) }).unwrap();
        assert!(loaded.join("skin.ini").is_file());
        assert!(!loaded.join("partial.png").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn incorrect_hash_size_and_archives_never_become_ready_skins() {
        for files in [vec![("../escaped", &b"bad"[..])], vec![("notes.txt", &b"not a skin"[..])], vec![("A/skin.ini", &b"A"[..]), ("B/skin.ini", &b"B"[..])]] {
            let bytes = skin_archive(&files);
            let (root, skin) = skin_case(&bytes);
            assert!(prepare_skin(&root, &skin, |path| { std::fs::write(path, &bytes).unwrap(); Ok(bytes.len() as u64) }).is_err());
            assert!(!root.join(&skin.hash).exists());
            std::fs::remove_dir_all(root).unwrap();
        }
        let bytes = skin_archive(&[("skin.ini", b"[General]")]);
        let (root, mut skin) = skin_case(&bytes);
        skin.size += 1;
        assert!(prepare_skin(&root, &skin, |path| { std::fs::write(path, &bytes).unwrap(); Ok(0) }).unwrap_err().contains("size"));
        skin.size = 0;
        skin.hash = "a".repeat(64);
        assert!(prepare_skin(&root, &skin, |path| { std::fs::write(path, &bytes).unwrap(); Ok(0) }).unwrap_err().contains("checksum"));
        skin.hash = "../abc".into();
        assert!(prepare_skin(&root, &skin, |_| panic!("invalid hashes must not reach the downloader")).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_long_video_is_given_a_bitrate_that_fits_the_limit() {
        let most = 48 * 1024 * 1024;
        let rate = bitrate_for(most, Duration::from_secs(300));
        let size = (rate + 192_000) as f64 * 300.0 / 8.0;
        assert!(size <= most as f64);
        assert!(rate > 1_000_000);
        assert_eq!(bitrate_for(most, Duration::from_secs(100_000)), 300_000);
    }

    #[test]
    fn loudness_is_normalised_to_the_asked_level() {
        assert_eq!(loudness_filter(-14.0), "loudnorm=I=-14.0:TP=-1.5:LRA=11");
    }
}
