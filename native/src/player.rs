use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use iced::widget::image;

pub const WIDTH: u32 = 960;
pub const HEIGHT: u32 = 540;

pub struct Player {
    pub path: PathBuf,
    pub level: f32,
    pub length_ms: i64,
    pub frame: Option<image::Handle>,
    pub paused: bool,
    at_ms: Arc<AtomicI64>,
    stop: Arc<AtomicBool>,
    hold: Arc<AtomicBool>,
    frames: Receiver<(i64, Vec<u8>)>,
    ffmpeg: PathBuf,
    fps: u32,
    sound: Option<Sound>,
    procs: Arc<Mutex<Vec<Child>>>,
    started: Instant,
    from_ms: i64,
    ended: bool,
}

struct Sound {
    _stream: cpal::Stream,
}

impl Player {
    pub fn open(ffmpeg: &Path, path: &Path, length_ms: i64, fps: u32, level: f32) -> Player {
        let mut player = Player {
            path: path.to_path_buf(),
            level,
            length_ms,
            frame: None,
            paused: false,
            at_ms: Arc::new(AtomicI64::new(0)),
            stop: Arc::new(AtomicBool::new(false)),
            hold: Arc::new(AtomicBool::new(false)),
            frames: sync_channel(1).1,
            ffmpeg: ffmpeg.to_path_buf(),
            fps: fps.max(1),
            sound: None,
            procs: Arc::new(Mutex::new(Vec::new())),
            started: Instant::now(),
            from_ms: 0,
            ended: false,
        };
        player.start(0);
        player
    }

    pub fn still(path: &Path, length_ms: i64, at_ms: i64) -> Player {
        let mut player = Player {
            path: path.to_path_buf(),
            level: 1.0,
            length_ms,
            frame: None,
            paused: true,
            at_ms: Arc::new(AtomicI64::new(at_ms)),
            stop: Arc::new(AtomicBool::new(true)),
            hold: Arc::new(AtomicBool::new(true)),
            frames: sync_channel(1).1,
            ffmpeg: PathBuf::new(),
            fps: 60,
            sound: None,
            procs: Arc::new(Mutex::new(Vec::new())),
            started: Instant::now(),
            from_ms: at_ms,
            ended: false,
        };
        player.paused = true;
        player
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

    pub fn pull(&mut self) {
        loop {
            match self.frames.try_recv() {
                Ok((at, rgba)) => {
                    self.at_ms.store(at, Ordering::Relaxed);
                    self.frame = Some(image::Handle::from_rgba(WIDTH, HEIGHT, rgba));
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if !self.paused && self.frame.is_some() && self.at_ms() >= self.length_ms.saturating_sub(120) {
                        self.ended = true;
                        self.paused = true;
                        self.hold.store(true, Ordering::Relaxed);
                    }
                    break;
                }
            }
        }
    }

    pub fn toggle(&mut self) {
        if self.ended {
            self.seek(0);
            return;
        }
        self.paused = !self.paused;
        self.hold.store(self.paused, Ordering::Relaxed);
        if let Some(sound) = &self.sound {
            if self.paused {
                let _ = sound._stream.pause();
            } else {
                let _ = sound._stream.play();
            }
        }
    }

    pub fn seek(&mut self, to_ms: i64) {
        let to = to_ms.clamp(0, self.length_ms.max(0));
        self.stop_streams();
        self.ended = false;
        self.paused = false;
        self.at_ms.store(to, Ordering::Relaxed);
        self.start(to);
    }

    pub fn seek_by(&mut self, delta_ms: i64) {
        self.seek(self.at_ms() + delta_ms);
    }

    pub fn close(&mut self) {
        self.stop_streams();
    }

    fn stop_streams(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.hold.store(false, Ordering::Relaxed);
        self.sound = None;
        if let Ok(mut procs) = self.procs.lock() {
            for child in procs.iter_mut() {
                let _ = child.kill();
                let _ = child.wait();
            }
            procs.clear();
        }
    }

    fn start(&mut self, from_ms: i64) {
        self.stop = Arc::new(AtomicBool::new(false));
        self.hold = Arc::new(AtomicBool::new(false));
        self.from_ms = from_ms;
        self.started = Instant::now();
        let (tx, rx) = sync_channel(2);
        self.frames = rx;
        self.spawn_video(from_ms, tx);
        self.sound = self.spawn_sound(from_ms);
    }

    fn spawn_video(&self, from_ms: i64, tx: SyncSender<(i64, Vec<u8>)>) {
        let mut child = match Command::new(&self.ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-ss", &seconds(from_ms), "-i"])
            .arg(&self.path)
            .args(["-an", "-f", "rawvideo", "-pix_fmt", "rgba", "-vf", &format!("scale={WIDTH}:{HEIGHT}"), "-"])
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
        let hold = self.hold.clone();
        let fps = self.fps;
        let frame_bytes = (WIDTH * HEIGHT * 4) as usize;
        thread::spawn(move || {
            let started = Instant::now();
            let mut held = Duration::ZERO;
            let mut index: u64 = 0;
            let mut buffer = vec![0u8; frame_bytes];
            loop {
                if stop.load(Ordering::Relaxed) {
                    return;
                }
                if out.read_exact(&mut buffer).is_err() {
                    return;
                }
                let due = started + held + Duration::from_secs_f64(index as f64 / fps as f64);
                let now = Instant::now();
                if due > now {
                    thread::sleep(due - now);
                }
                while hold.load(Ordering::Relaxed) {
                    let paused_at = Instant::now();
                    while hold.load(Ordering::Relaxed) {
                        if stop.load(Ordering::Relaxed) {
                            return;
                        }
                        thread::sleep(Duration::from_millis(8));
                    }
                    held += paused_at.elapsed();
                }
                let at = from_ms + (index as f64 * 1000.0 / fps as f64) as i64;
                if tx.send((at, buffer.clone())).is_err() {
                    return;
                }
                index += 1;
            }
        });
    }

    fn spawn_sound(&self, from_ms: i64) -> Option<Sound> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let config = device.default_output_config().ok()?;
        let rate = config.sample_rate();
        let channels = config.channels() as usize;
        let mut child = Command::new(&self.ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-ss", &seconds(from_ms), "-i"])
            .arg(&self.path)
            .args(["-vn", "-f", "f32le", "-acodec", "pcm_f32le", "-ac", &channels.to_string(), "-ar", &rate.0.to_string(), "-"])
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
        let level = self.level.clamp(0.0, 1.0);
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
        let stream = device
            .build_output_stream(
                &config.into(),
                move |data: &mut [f32], _| {
                    let mut left = leftover.lock().unwrap();
                    let rx = rx.lock().unwrap();
                    for sample in data.iter_mut() {
                        if left.1 >= left.0.len() {
                            match rx.try_recv() {
                                Ok(chunk) => {
                                    left.0 = chunk;
                                    left.1 = 0;
                                }
                                Err(_) => {
                                    *sample = 0.0;
                                    continue;
                                }
                            }
                        }
                        *sample = left.0[left.1] * level;
                        left.1 += 1;
                    }
                },
                |_| {},
                None,
            )
            .ok()?;
        stream.play().ok()?;
        Some(Sound { _stream: stream })
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.stop_streams();
    }
}

fn seconds(ms: i64) -> String {
    format!("{:.3}", ms.max(0) as f64 / 1000.0)
}
