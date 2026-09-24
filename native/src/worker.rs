use std::path::{Path, PathBuf};
use std::process::Command;
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

const RESTING: Duration = Duration::from_secs(8);
const QUIET: Duration = Duration::from_secs(20);
const BEAT: Duration = Duration::from_secs(15);

pub fn stop() {
    ON.store(false, Ordering::SeqCst);
}

pub fn drawing() -> bool {
    DRAWING.load(Ordering::SeqCst)
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
                let title = job.title.clone();
                if !push(Step::Taken { title: title.clone() }) {
                    let _ = bot::give_back(&setup.server, &setup.token, &setup.name, &job.id, "the application closed");
                    break;
                }
                match one(setup, &job, push) {
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
        Some(render::Step::Saved(_)) => {}
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
    let hash: String = skin.hash.chars().filter(|c| c.is_ascii_hexdigit()).take(64).collect();
    if hash.is_empty() {
        return Err("the skin has no hash".to_owned());
    }
    let skins = crate::sources::own_root().join("cache").join("farm-skins");
    let unpacked = skins.join(&hash);
    if unpacked.is_dir() {
        return Ok(unpacked);
    }
    let archive = skins.join(format!("{hash}.osk"));
    bot::job_file(&setup.server, &setup.token, &setup.name, &job.id, "skin", &archive).map_err(|e| format!("the skin: {e}"))?;
    let opened = crate::maps::unpack(&archive, &unpacked);
    let _ = std::fs::remove_file(&archive);
    match opened {
        Ok(_) => Ok(unpacked),
        Err(why) => {
            let _ = std::fs::remove_dir_all(&unpacked);
            Err(format!("the skin: {why}"))
        }
    }
}

fn length_of(replay: &Path) -> Duration {
    std::fs::read(replay)
        .ok()
        .and_then(|bytes| dossier_replay::Replay::parse(&bytes).ok())
        .map_or(Duration::ZERO, |r| Duration::from_millis(r.duration_ms().max(0) as u64))
}

fn ffmpeg(binary: &Path, args: &[&str]) -> Result<(), String> {
    let mut command = Command::new(binary);
    command.args(["-hide_banner", "-loglevel", "error", "-y"]).args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
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
