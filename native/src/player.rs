use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub const WIDTH: u32 = 960;
pub const HEIGHT: u32 = 540;

pub const RATES: [f32; 6] = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0];

const AHEAD: usize = 4;
const SOUND_WAIT: Duration = Duration::from_millis(500);
const HEARD_FOR: f64 = 150.0;
const SNAP: f64 = 200.0;
const SETTLE: Duration = Duration::from_millis(700);
const FOLLOW: f64 = 0.02;
const SLEW: f64 = 0.08;

fn epoch() -> Instant {
    static EPOCH: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    *EPOCH.get_or_init(Instant::now)
}

fn micros(at: Instant) -> i64 {
    at.saturating_duration_since(epoch()).as_micros() as i64
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Pace {
    from_ms: f64,
    rate: f64,
    frame_ms: f64,
    began: Option<Instant>,
    asked: Option<Instant>,
    paused_at: Option<Instant>,
    resumed_at: Option<Instant>,
    held: Duration,
    offset: f64,
    synced: bool,
    far_since: Option<Instant>,
    bias: f64,
    last: Option<Instant>,
    gap_ms: f64,
    tick_ms: f64,
}

impl Pace {
    pub fn new(from_ms: i64, rate: f32, shown_fps: f64, held: bool, now: Instant) -> Pace {
        Pace { from_ms: from_ms as f64, rate: rate as f64, frame_ms: 1000.0 / shown_fps.max(1.0), paused_at: held.then_some(now), ..Pace::default() }
    }

    pub fn pause(&mut self, now: Instant) {
        if self.paused_at.is_none() {
            self.paused_at = Some(now);
        }
    }

    pub fn resume(&mut self, now: Instant) {
        if let Some(since) = self.paused_at.take() {
            if self.began.is_some_and(|began| since >= began) {
                self.held += now.saturating_duration_since(since);
            }
            self.resumed_at = Some(now);
            self.last = None;
        }
    }

    fn wall(&self, now: Instant) -> Option<f64> {
        let began = self.began?;
        let now = self.paused_at.map_or(now, |paused| paused.max(began));
        Some(self.from_ms + now.saturating_duration_since(began).saturating_sub(self.held).as_secs_f64() * 1000.0 * self.rate)
    }

    pub fn at(&mut self, now: Instant, heard: Option<(f64, Instant)>, sound: bool, ready: bool) -> Option<f64> {
        if self.paused_at.is_some() {
            return self.wall(now).map(|wall| wall + self.offset + self.bias);
        }
        let asked = *self.asked.get_or_insert(now);
        if self.began.is_none() {
            let waited = now.saturating_duration_since(asked) >= SOUND_WAIT;
            if !ready || (sound && heard.is_none() && !waited) {
                return None;
            }
            self.began = Some(now);
        }
        if let Some(before) = self.last.replace(now) {
            let gap = now.saturating_duration_since(before).as_secs_f64() * 1000.0;
            if gap > 0.5 && gap < 50.0 {
                if (gap - self.gap_ms).abs() < 1.5 {
                    self.tick_ms = (gap + self.gap_ms) / 2.0;
                }
                self.gap_ms = gap;
            }
        }
        let wall = self.wall(now)?;
        if let Some((at, stamp)) = heard.filter(|(_, stamp)| self.resumed_at.is_none_or(|resumed| *stamp >= resumed)) {
            let since = now.saturating_duration_since(stamp).as_secs_f64() * 1000.0 * self.rate;
            if since <= HEARD_FOR {
                let off = at + since - (wall + self.offset);
                if !self.synced {
                    self.offset += off;
                    self.synced = true;
                } else if off.abs() <= SNAP {
                    self.far_since = None;
                    self.offset += (off * FOLLOW).clamp(-SLEW, SLEW);
                } else if now.saturating_duration_since(*self.far_since.get_or_insert(now)) >= SETTLE {
                    self.offset += off;
                    self.far_since = None;
                }
            }
        }
        Some(wall + self.offset + self.bias)
    }

    pub fn shown(&mut self, at: f64, frame_at: f64) {
        let tick = self.tick_ms;
        let shown_for = self.frame_ms / self.rate.max(0.01);
        if tick <= 0.0 || tick > shown_for * 1.03 {
            return;
        }
        let ticks = shown_for / tick;
        if (ticks - ticks.round()).abs() > 0.03 {
            return;
        }
        let late = (at - frame_at) / self.rate.max(0.01);
        if late < tick * 0.25 || late > tick * 0.75 {
            self.bias = (self.bias + (tick * 0.5 - late) * self.rate).clamp(-tick * self.rate, tick * self.rate);
        }
    }
}

#[derive(Debug)]
struct Heard {
    at_us: AtomicI64,
    stamp_us: AtomicI64,
}

const NOT_YET: i64 = -1;
const OVER: i64 = -2;

impl Heard {
    fn said(&self) -> Option<(f64, Instant)> {
        let at = self.at_us.load(Ordering::Relaxed);
        (at >= 0).then(|| (at as f64 / 1000.0, epoch() + Duration::from_micros(self.stamp_us.load(Ordering::Relaxed).max(0) as u64)))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Manner {
    pub level: f32,
    pub muted: bool,
    pub rate: f32,
}

impl Default for Manner {
    fn default() -> Manner {
        Manner { level: 1.0, muted: false, rate: 1.0 }
    }
}

pub struct Player {
    pub path: PathBuf,
    pub length_ms: i64,
    pub frame: Option<crate::film::Frame>,
    reel: u64,
    pub paused: bool,
    level: Arc<AtomicU32>,
    muted: Arc<AtomicBool>,
    rate: f32,
    at_ms: Arc<AtomicI64>,
    stop: Arc<AtomicBool>,
    frames: Receiver<(i64, Vec<u8>)>,
    next: Option<(i64, Vec<u8>)>,
    pace: Pace,
    fresh: bool,
    ffmpeg: PathBuf,
    fps: f64,
    size: (u32, u32),
    sound: Option<Sound>,
    procs: Arc<Mutex<Vec<Child>>>,
    ended: bool,
    wake: Option<crate::playback_wake::PlaybackWake>,
}

struct Sound {
    _stream: cpal::Stream,
    heard: Arc<Heard>,
}

impl Player {
    pub fn open(ffmpeg: &Path, path: &Path, media: crate::videos::Probe, manner: Manner) -> Player {
        let mut player = Player {
            path: path.to_path_buf(),
            length_ms: media.length_ms,
            frame: None,
            reel: crate::film::reel(),
            paused: false,
            level: Arc::new(AtomicU32::new(manner.level.clamp(0.0, 1.0).to_bits())),
            muted: Arc::new(AtomicBool::new(manner.muted)),
            rate: steady(manner.rate),
            at_ms: Arc::new(AtomicI64::new(0)),
            stop: Arc::new(AtomicBool::new(false)),
            frames: sync_channel(1).1,
            next: None,
            pace: Pace::default(),
            fresh: true,
            ffmpeg: ffmpeg.to_path_buf(),
            fps: media.fps.max(1.0),
            size: display_size(media.width, media.height),
            sound: None,
            procs: Arc::new(Mutex::new(Vec::new())),
            ended: false,
            wake: Some(crate::playback_wake::PlaybackWake::new()),
        };
        player.start(0, false);
        player
    }

    pub fn still(path: &Path, length_ms: i64, at_ms: i64) -> Player {
        Player {
            path: path.to_path_buf(),
            length_ms,
            frame: None,
            reel: crate::film::reel(),
            paused: true,
            level: Arc::new(AtomicU32::new(1.0f32.to_bits())),
            muted: Arc::new(AtomicBool::new(false)),
            rate: 1.0,
            at_ms: Arc::new(AtomicI64::new(at_ms)),
            stop: Arc::new(AtomicBool::new(true)),
            frames: sync_channel(1).1,
            next: None,
            pace: Pace::default(),
            fresh: false,
            ffmpeg: PathBuf::new(),
            fps: 60.0,
            size: (WIDTH, HEIGHT),
            sound: None,
            procs: Arc::new(Mutex::new(Vec::new())),
            ended: false,
            wake: None,
        }
    }

    pub fn at_ms(&self) -> i64 {
        self.at_ms.load(Ordering::Relaxed).clamp(0, self.length_ms.max(0))
    }

    pub fn fraction(&self) -> f32 {
        if self.length_ms <= 0 {
            return 0.0;
        }
        (self.at_ms() as f32 / self.length_ms as f32).clamp(0.0, 1.0)
    }

    pub fn ended(&self) -> bool {
        self.ended
    }

    pub fn ready(&self) -> bool {
        self.frame.is_some()
    }

    pub fn level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }

    pub fn set_level(&mut self, level: f32) {
        self.level.store(level.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
        if level > 0.0 {
            self.muted.store(false, Ordering::Relaxed);
        }
    }

    pub fn muted(&self) -> bool {
        self.muted.load(Ordering::Relaxed)
    }

    pub fn set_muted(&mut self, muted: bool) {
        self.muted.store(muted, Ordering::Relaxed);
    }

    pub fn rate(&self) -> f32 {
        self.rate
    }

    fn shown_fps(&self) -> f64 {
        self.fps.min(60.0).min(60.0 / self.rate as f64)
    }

    pub fn set_rate(&mut self, rate: f32) {
        let rate = steady(rate);
        if (rate - self.rate).abs() < 0.001 {
            return;
        }
        let at = self.at_ms();
        let held = self.paused;
        self.rate = rate;
        self.stop_streams();
        self.start(at, held);
    }

    pub fn pull(&mut self, now: Instant) -> bool {
        let heard = self.sound.as_ref().and_then(|sound| sound.heard.said());
        if self.next.is_none() {
            self.next = self.frames.try_recv().ok();
        }
        let at = self.pace.at(now, heard, self.sound.is_some(), self.next.is_some() || !self.fresh);
        let mut latest: Option<(i64, Vec<u8>)> = None;
        loop {
            if self.next.is_none() {
                match self.frames.try_recv() {
                    Ok(frame) => self.next = Some(frame),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        if latest.is_none() {
                            self.over();
                        }
                        break;
                    }
                }
            }
            let due = self.next.as_ref().is_some_and(|(frame_at, _)| (self.fresh && latest.is_none()) || at.is_some_and(|at| *frame_at as f64 <= at));
            if !due || (self.paused && !self.fresh) {
                break;
            }
            latest = self.next.take();
            self.fresh = false;
        }
        if crate::frames::on() {
            let heard_said = heard.map(|(heard_at, stamp)| format!("{:.1}/{:.1}", heard_at, now.saturating_duration_since(stamp).as_secs_f64() * 1000.0)).unwrap_or_default();
            crate::frames::mark("pace", now, &format!("at={:.1} offset={:.2} bias={:.2} heard={heard_said} took={:?} waiting={}", at.unwrap_or(-1.0), self.pace.offset, self.pace.bias, latest.as_ref().map(|(frame_at, _)| *frame_at), self.next.is_some()));
        }
        let Some((frame_at, rgba)) = latest else {
            return false;
        };
        if let Some(at) = at {
            self.pace.shown(at, frame_at as f64);
        }
        self.at_ms.store(frame_at, Ordering::Relaxed);
        self.frame = Some(crate::film::Frame::new(self.reel, self.size.0, self.size.1, rgba));
        true
    }

    fn over(&mut self) {
        self.keep_awake(false);
        if !self.paused && self.frame.is_some() {
            self.ended = true;
            self.paused = true;
            self.pace.pause(Instant::now());
            if let Some(sound) = &self.sound {
                let _ = sound._stream.pause();
            }
        }
    }

    pub fn toggle(&mut self) {
        crate::frames::mark("player", Instant::now(), if self.ended { "again" } else if self.paused { "play" } else { "pause" });
        if self.ended {
            self.go(0, false);
            return;
        }
        self.paused = !self.paused;
        match self.paused {
            true => self.pace.pause(Instant::now()),
            false => self.pace.resume(Instant::now()),
        }
        self.keep_awake(!self.paused);
        if let Some(sound) = &self.sound {
            if self.paused {
                let _ = sound._stream.pause();
            } else {
                let _ = sound._stream.play();
            }
        }
    }

    pub fn seek(&mut self, to_ms: i64) {
        self.go(to_ms, self.paused && !self.ended);
    }

    pub fn seek_by(&mut self, delta_ms: i64) {
        self.seek(self.at_ms() + delta_ms);
    }

    pub fn step(&mut self, frames: i64) {
        let frame = (self.at_ms() as f64 * self.fps / 1000.0).round() + frames as f64;
        self.go((frame * 1000.0 / self.fps).round() as i64, true);
    }

    fn go(&mut self, to_ms: i64, held: bool) {
        crate::frames::mark("player", Instant::now(), &format!("go {to_ms}"));
        let to = to_ms.clamp(0, self.length_ms.max(0));
        self.stop_streams();
        self.ended = false;
        self.at_ms.store(to, Ordering::Relaxed);
        self.start(to, held);
    }

    pub fn close(&mut self) {
        self.keep_awake(false);
        self.stop_streams();
    }

    fn keep_awake(&mut self, playing: bool) {
        if let Some(wake) = &mut self.wake { wake.playing(playing); }
    }

    fn stop_streams(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.sound = None;
        self.next = None;
        if let Ok(mut procs) = self.procs.lock() {
            for child in procs.iter_mut() {
                let _ = child.kill();
                let _ = child.wait();
            }
            procs.clear();
        }
    }

    fn start(&mut self, from_ms: i64, held: bool) {
        self.stop = Arc::new(AtomicBool::new(false));
        self.paused = held;
        self.keep_awake(!held);
        let (tx, rx) = sync_channel(AHEAD);
        self.frames = rx;
        self.next = None;
        self.fresh = true;
        self.pace = Pace::new(from_ms, self.rate, self.shown_fps(), held, Instant::now());
        self.sound = self.spawn_sound(from_ms, held);
        self.spawn_video(from_ms, tx);
    }

    fn spawn_video(&self, from_ms: i64, tx: SyncSender<(i64, Vec<u8>)>) {
        let shown = self.shown_fps();
        let (width, height) = self.size;
        let mut child = match crate::checks::quiet(&self.ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-ss", &seconds(from_ms), "-i"])
            .arg(&self.path)
            .args(["-an", "-f", "rawvideo", "-pix_fmt", "rgba", "-vf", &format!("scale={width}:{height},fps={shown:.6}"), "-"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(_) => return,
        };
        let Some(mut out) = child.stdout.take() else {
            return;
        };
        if let Ok(mut procs) = self.procs.lock() {
            procs.push(child);
        }
        let stop = self.stop.clone();
        let frame_bytes = width as usize * height as usize * 4;
        thread::spawn(move || {
            let mut index: u64 = 0;
            loop {
                let mut buffer = vec![0u8; frame_bytes];
                if stop.load(Ordering::Relaxed) || out.read_exact(&mut buffer).is_err() {
                    return;
                }
                let mut ready = (from_ms + (index as f64 * 1000.0 / shown) as i64, buffer);
                loop {
                    if stop.load(Ordering::Relaxed) {
                        return;
                    }
                    match tx.try_send(ready) {
                        Ok(()) => break,
                        Err(TrySendError::Full(back)) => {
                            ready = back;
                            thread::sleep(Duration::from_millis(2));
                        }
                        Err(TrySendError::Disconnected(_)) => return,
                    }
                }
                index += 1;
            }
        });
    }

    fn spawn_sound(&self, from_ms: i64, held: bool) -> Option<Sound> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let config = device.default_output_config().ok()?;
        let rate = config.sample_rate();
        let channels = config.channels() as usize;
        let mut args: Vec<String> = ["-hide_banner", "-loglevel", "error", "-ss"].iter().map(|s| s.to_string()).collect();
        args.push(seconds(from_ms));
        args.push("-i".to_owned());
        args.push(self.path.to_string_lossy().into_owned());
        if (self.rate - 1.0).abs() > 0.001 {
            args.push("-af".to_owned());
            args.push(format!("atempo={:.3}", self.rate));
        }
        for part in ["-vn", "-f", "f32le", "-acodec", "pcm_f32le", "-ac"] {
            args.push(part.to_owned());
        }
        args.push(channels.to_string());
        args.push("-ar".to_owned());
        args.push(rate.0.to_string());
        args.push("-".to_owned());
        let mut child = crate::checks::quiet(&self.ffmpeg)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
            .ok()?;
        let mut out = child.stdout.take()?;
        if let Ok(mut procs) = self.procs.lock() {
            procs.push(child);
        }
        let (tx, rx) = sync_channel::<Vec<f32>>(16);
        let stop = self.stop.clone();
        thread::spawn(move || {
            let mut bytes = vec![0u8; 4096 * 4];
            loop {
                if stop.load(Ordering::Relaxed) {
                    return;
                }
                let read = match out.read(&mut bytes) {
                    Ok(0) | Err(_) => return,
                    Ok(n) => n,
                };
                let samples: Vec<f32> = bytes[..read - read % 4].chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
                if tx.send(samples).is_err() {
                    return;
                }
            }
        });
        let leftover: Arc<Mutex<(Vec<f32>, usize)>> = Arc::new(Mutex::new((Vec::new(), 0)));
        let rx = Mutex::new(rx);
        let level = self.level.clone();
        let muted = self.muted.clone();
        let heard = Arc::new(Heard { at_us: AtomicI64::new(NOT_YET), stamp_us: AtomicI64::new(0) });
        let output_clock = heard.clone();
        let speed = self.rate as f64;
        let mut rendered_samples = 0i64;
        let stream = device
            .build_output_stream(
                &config.into(),
                move |data: &mut [f32], _| {
                    let gain = if muted.load(Ordering::Relaxed) { 0.0 } else { f32::from_bits(level.load(Ordering::Relaxed)) };
                    let mut left = leftover.lock().unwrap();
                    let rx = rx.lock().unwrap();
                    for sample in data.iter_mut() {
                        if left.1 >= left.0.len() {
                            match rx.try_recv() {
                                Ok(chunk) => {
                                    left.0 = chunk;
                                    left.1 = 0;
                                }
                                Err(TryRecvError::Disconnected) => {
                                    output_clock.at_us.store(OVER, Ordering::Relaxed);
                                    *sample = 0.0;
                                    continue;
                                }
                                Err(TryRecvError::Empty) => {
                                    *sample = 0.0;
                                    continue;
                                }
                            }
                        }
                        *sample = left.0[left.1] * gain;
                        left.1 += 1;
                        rendered_samples += 1;
                    }
                    if rendered_samples > 0 && output_clock.at_us.load(Ordering::Relaxed) != OVER {
                        let before = (data.len() / channels.max(1)) as f64 / rate.0 as f64;
                        let at = from_ms as f64 + (rendered_samples as f64 / channels as f64 / rate.0 as f64 - before).max(0.0) * speed * 1000.0;
                        output_clock.stamp_us.store(micros(Instant::now()), Ordering::Relaxed);
                        output_clock.at_us.store((at * 1000.0) as i64, Ordering::Relaxed);
                    }
                },
                |_| {},
                None,
            )
            .ok()?;
        match held {
            true => stream.pause().ok()?,
            false => stream.play().ok()?,
        }
        Some(Sound { _stream: stream, heard })
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.stop_streams();
    }
}

fn steady(rate: f32) -> f32 {
    RATES
        .iter()
        .copied()
        .min_by(|a, b| (a - rate).abs().total_cmp(&(b - rate).abs()))
        .unwrap_or(1.0)
}

fn display_size(width: u32, height: u32) -> (u32, u32) {
    let width = width.max(1);
    let height = height.max(1);
    let scale = (1280.0 / width as f64).min(720.0 / height as f64).min(1.0);
    let even = |side: u32| ((side as f64 * scale).round() as u32).max(2) & !1;
    (even(width), even(height))
}

pub fn next_rate(rate: f32, by: i32) -> f32 {
    let many = RATES.len() as i32;
    let at = RATES.iter().position(|r| (r - steady(rate)).abs() < 0.001).unwrap_or(2) as i32;
    RATES[(at + by).rem_euclid(many) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires ffmpeg with the libx264 encoder"]
    fn decoded_frames_keep_the_playback_resolution_and_cadence() {
        let ffmpeg = crate::checks::ffmpeg_on_path().expect("ffmpeg installed");
        let path = std::env::temp_dir().join(format!("dossier-player-{}.mp4", std::process::id()));
        let status = crate::checks::quiet(&ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-y", "-f", "lavfi", "-i", "testsrc2=size=1200x800:rate=120", "-t", "0.2", "-c:v", "libx264", "-preset", "ultrafast"])
            .arg(&path).status().unwrap();
        assert!(status.success());
        let media = crate::videos::probe(&ffmpeg, &path).unwrap();
        assert_eq!((media.width, media.height, media.fps), (1200, 800, 120.0));
        let mut player = Player::still(&path, media.length_ms, 0);
        player.ffmpeg = ffmpeg;
        player.size = display_size(media.width, media.height);
        player.fps = media.fps;
        player.stop.store(false, Ordering::Relaxed);
        let (tx, rx) = sync_channel(2);
        player.spawn_video(0, tx);
        let frames: Vec<_> = rx.into_iter().collect();
        assert_eq!(frames.len(), 12, "the reader waits for room and loses no frame");
        assert!(frames.iter().all(|(_, rgba)| rgba.len() == 1080 * 720 * 4));
        assert_eq!(frames[1].0, 16);
        assert_eq!(frames.last().unwrap().0, 183);
        player.close();
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn a_burst_from_the_sound_device_does_not_throw_the_picture_ahead() {
        let began = Instant::now();
        let ms = |at: f64| began + Duration::from_secs_f64(at / 1000.0);
        let mut pace = Pace::new(0, 1.0, 60.0, false, began);
        let heard = |wall: f64| -> (f64, Instant) {
            let step = 10.667;
            let told = if (3000.0..3352.0).contains(&wall) { 3352.0 } else { (wall / step).floor() * step };
            let stamp = if (3000.0..3352.0).contains(&wall) { wall } else { (wall / step).floor() * step };
            (told, ms(stamp))
        };
        let mut worst: f64 = 0.0;
        for tick in 0..720 {
            let wall = tick as f64 * 1000.0 / 120.0;
            let at = pace.at(ms(wall), Some(heard(wall)), true, true).unwrap();
            if wall > 500.0 {
                worst = worst.max((at - wall).abs());
            }
        }
        assert!(worst < 20.0, "the picture left the wall clock by {worst:.1} ms when the sound device took a third of a second at once");
        let mut moved = Pace::new(0, 1.0, 60.0, false, began);
        let mut last = 0.0;
        for tick in 0..480 {
            let wall = tick as f64 * 1000.0 / 120.0;
            let told = if wall < 1000.0 { wall } else { wall + 900.0 };
            last = moved.at(ms(wall), Some((told, ms(wall))), true, true).unwrap() - wall;
        }
        assert!((last - 900.0).abs() < 30.0, "a sound that really moved was not followed: {last:.1}");
    }

    #[test]
    fn a_screen_that_changes_its_rate_costs_the_picture_a_frame_or_two_and_no_more() {
        let began = Instant::now();
        let mut pace = Pace::new(0, 1.0, 60.0, false, began);
        let frame_ms = 1000.0 / 60.0;
        let (mut wall, mut shown, mut last_wall, mut odd, mut seen) = (0.0f64, -1i64, 0.0f64, 0usize, 0usize);
        for tick in 0..2_400 {
            let slow = (600..1_200).contains(&tick) || tick >= 1_800;
            wall += if slow { 1000.0 / 60.0 } else { 1000.0 / 120.0 };
            let now = began + Duration::from_secs_f64(wall / 1000.0);
            let at = pace.at(now, None, false, true).unwrap();
            let due = (at / frame_ms).floor() as i64;
            if due > shown {
                pace.shown(at, due as f64 * frame_ms);
                if shown >= 0 && tick > 60 {
                    seen += 1;
                    if due - shown != 1 || (wall - last_wall - frame_ms).abs() > 1.0 {
                        odd += 1;
                    }
                }
                shown = due;
                last_wall = wall;
            }
        }
        assert!(seen > 1_500 && odd <= 8, "{odd} of {seen} frames were skipped or held when the screen went between 120 and 60 refreshes a second");
    }

    fn played(rate: f32, shown_fps: f64, tick_ms: f64, sound_every_ms: Option<f64>, ticks: usize) -> Vec<usize> {
        let began = Instant::now();
        let mut pace = Pace::new(0, rate, shown_fps, false, began);
        let frame_ms = 1000.0 / shown_fps;
        let (mut shown, mut gaps, mut last_tick) = (-1i64, Vec::new(), 0usize);
        let mut seed = 7u64;
        for tick in 0..ticks {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            let shake = ((seed >> 33) % 800) as f64 / 1000.0 - 0.4;
            let wall = tick as f64 * tick_ms + 1.0 + shake;
            let now = began + Duration::from_secs_f64(wall / 1000.0);
            let heard = sound_every_ms.map(|every| {
                let calls = ((wall - 0.7) / every).floor().max(0.0);
                (calls * every * rate as f64 * 1.000_03, began + Duration::from_secs_f64((calls * every + 0.7) / 1000.0))
            });
            let Some(at) = pace.at(now, heard, sound_every_ms.is_some(), true) else {
                continue;
            };
            let due = (at / frame_ms).floor() as i64;
            if due > shown {
                pace.shown(at, due as f64 * frame_ms);
                if shown >= 0 && tick > ticks / 10 {
                    gaps.push(tick - last_tick);
                }
                shown = due;
                last_tick = tick;
            }
        }
        gaps
    }

    #[test]
    fn every_frame_stays_on_screen_for_the_same_number_of_refreshes() {
        for (rate, shown, tick, sound, each) in [(1.0, 60.0, 1000.0 / 120.0, Some(10.667), 2), (1.0, 60.0, 1000.0 / 120.0, Some(85.33), 2), (1.0, 60.0, 1000.0 / 120.0, None, 2), (1.0, 60.0, 1000.0 / 60.0, Some(10.667), 1), (2.0, 30.0, 1000.0 / 120.0, Some(10.667), 2), (0.5, 60.0, 1000.0 / 120.0, Some(10.667), 4), (1.0, 30.0, 1000.0 / 120.0, Some(21.33), 4)] {
            let gaps = played(rate, shown, tick, sound, 3_600);
            let odd = gaps.iter().filter(|gap| **gap != each).count();
            assert!(gaps.len() > 500 && odd <= 2, "rate {rate}, {shown} fps, tick {tick:.2}, sound {sound:?}: {odd} of {} frames stayed for another number of refreshes", gaps.len());
        }
    }

    #[test]
    fn a_pause_holds_the_picture_and_playing_goes_on_from_the_same_place() {
        let began = Instant::now();
        let ms = |at: f64| began + Duration::from_secs_f64(at / 1000.0);
        let mut pace = Pace::new(5_000, 1.0, 60.0, false, began);
        assert_eq!(pace.at(ms(0.0), None, true, false), None, "nothing is shown before the first frame");
        assert_eq!(pace.at(ms(10.0), None, true, true), None, "the picture waits for the sound to begin");
        let first = pace.at(ms(20.0), Some((5_000.0, ms(20.0))), true, true).unwrap();
        assert!((first - 5_000.0).abs() < 0.01);
        let later = pace.at(ms(120.0), Some((5_096.0, ms(116.0))), true, true).unwrap();
        assert!((later - 5_100.0).abs() < 1.0, "{later}");
        pace.pause(ms(130.0));
        let held = pace.at(ms(900.0), Some((5_096.0, ms(116.0))), true, true).unwrap();
        assert!((held - 5_110.0).abs() < 1.0, "{held}");
        pace.resume(ms(1_000.0));
        let on = pace.at(ms(1_050.0), Some((5_096.0, ms(116.0))), true, true).unwrap();
        assert!((on - 5_160.0).abs() < 1.0, "a stale word from the sound moved the picture: {on}");
        let silent = Pace::new(0, 1.0, 60.0, false, began).at(ms(0.0), None, false, true);
        assert_eq!(silent, Some(0.0), "a video without sound starts at once");
        let mut deaf = Pace::new(0, 1.0, 60.0, false, began);
        assert_eq!(deaf.at(ms(0.0), None, true, true), None);
        assert!(deaf.at(ms(600.0), None, true, true).is_some(), "a sound that never begins does not hold the picture for good");
        let mut still = Pace::new(700, 1.0, 60.0, true, began);
        assert_eq!(still.at(ms(50.0), None, true, true), None, "a video opened paused does not run");
    }

    #[test]
    fn a_speed_settles_on_one_of_the_steps() {
        assert_eq!(steady(1.1), 1.0);
        assert_eq!(steady(1.9), 2.0);
        assert_eq!(steady(0.1), 0.5);
    }

    #[test]
    fn high_frame_rate_video_stays_within_the_display_budget_at_double_speed() {
        let mut player = Player::still(Path::new("sample.mp4"), 1_000, 0);
        player.fps = 120.0;
        player.rate = 2.0;
        assert_eq!(player.shown_fps(), 30.0);
        assert_eq!(display_size(1920, 1080), (1280, 720));
        assert_eq!(display_size(640, 480), (640, 480));
    }

    #[test]
    fn the_speed_walks_the_steps_and_comes_round_again() {
        assert_eq!(next_rate(1.0, 1), 1.25);
        assert_eq!(next_rate(1.0, -1), 0.75);
        assert_eq!(next_rate(2.0, 1), 0.5, "the last step leads back to the first");
        assert_eq!(next_rate(0.5, -1), 2.0);
    }
}

fn seconds(ms: i64) -> String {
    format!("{:.3}", ms.max(0) as f64 / 1000.0)
}
