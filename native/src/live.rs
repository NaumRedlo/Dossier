use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use dossier_produce::{locate, scenery};

pub const SIZE: (u32, u32) = (960, 540);
pub const FPS: f64 = 60.0;
pub const EASE: Duration = Duration::from_millis(700);

#[derive(Debug, Default)]
pub struct Control {
    stop: AtomicBool,
    paused: AtomicBool,
    settled: AtomicBool,
    seek: Mutex<Option<f64>>,
    wanted: Mutex<bool>,
    knock: std::sync::Condvar,
}

fn ease(k: f64) -> f64 {
    let k = k.clamp(0.0, 1.0);
    k * k * (3.0 - 2.0 * k)
}

impl Control {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    pub fn pause(&self, paused: bool) {
        self.paused.store(paused, Ordering::SeqCst);
    }

    pub fn paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    pub fn seek_by(&self, delta_ms: f64) {
        let mut seek = self.seek.lock().expect("the seek");
        *seek = Some(seek.unwrap_or(0.0) + delta_ms);
    }

    fn stopped(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    pub fn settled(&self) -> bool {
        self.settled.load(Ordering::SeqCst)
    }

    pub fn request(&self) {
        *self.wanted.lock().expect("the request") = true;
        self.knock.notify_one();
    }

    fn await_request(&self, at_most: Duration) -> bool {
        let mut wanted = self.wanted.lock().expect("the request");
        if !*wanted {
            let (guard, _) = self.knock.wait_timeout(wanted, at_most).expect("the request");
            wanted = guard;
        }
        std::mem::replace(&mut *wanted, false)
    }

    fn take_seek(&self) -> Option<f64> {
        self.seek.lock().expect("the seek").take()
    }
}

#[derive(Debug, Clone)]
pub enum Frame {
    Picture { frame: crate::film::Frame, at_ms: f64, from_ms: f64, to_ms: f64 },
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct Ask {
    pub replay: PathBuf,
    pub map: PathBuf,
    pub map_hash: String,
    pub skin: Option<PathBuf>,
}

pub fn play(ask: Ask, control: Arc<Control>) -> iced::Task<Frame> {
    crate::ui::streamed(move |push| {
        if let Err(why) = run(&ask, &control, push) {
            push(Frame::Failed(why));
        }
    })
}

fn run(ask: &Ask, control: &Control, push: &mut dyn FnMut(Frame) -> bool) -> Result<(), String> {
    let bytes = std::fs::read(&ask.replay).map_err(|e| e.to_string())?;
    let replay = dossier_replay::Replay::parse(&bytes).map_err(|e| e.to_string())?;
    let found = locate::load_map(&ask.map, &ask.map_hash)?;
    let beatmap = dossier_beatmap::Beatmap::parse(&found.text).map_err(|e| e.to_string())?;
    let state = dossier_sim::GameState::new(&beatmap, &replay);

    let mut skin = dossier_render::Skin::with_combo_colours(beatmap.combo_colours());
    skin = skin.with_font(crate::render::hud_font());
    if let Some(folder) = ask.skin.as_ref().filter(|folder| folder.is_dir()) {
        skin = dossier_produce::skin::from_folder(skin, folder, None);
    }
    let layering = skin.sprites.as_ref().is_none_or(|s| s.ini().layered_hit_sounds);
    let scene = dossier_render::Scene::new(&state, skin).signed_by(&replay).bare();
    let fired = dossier_produce::hitsounds::sounded(&state, &beatmap, layering);
    let behind = scenery::Behind {
        background: true,
        storyboard: false,
        video: false,
        dim: None,
        blur: None,
        ffmpeg: "",
        size: SIZE,
        at_ms: None,
        scratch: None,
    };
    let scene = scenery::dress(scene, &behind, &beatmap, (&found.text, &found.origin), &fired);
    let layout = dossier_render::Layout::new(SIZE.0, SIZE.1);

    let (from_ms, to_ms) = state.span_ms();
    let rate = state.playback_rate().max(0.01);
    let step = Duration::from_secs_f64(1.0 / FPS);
    let mut at_ms = from_ms;
    let mut tick = Instant::now();
    let mut speed = 1.0f64;
    let mut eased_since: Option<(Instant, f64, bool)> = None;
    let mut was_paused = false;
    let mut first = true;
    let mut drawn: Option<Instant> = None;
    let reel = crate::film::reel();
    loop {
        if control.stopped() {
            return Ok(());
        }
        let asked = first || control.await_request(step * 4);
        first = false;
        if control.stopped() {
            return Ok(());
        }
        if let Some(delta) = control.take_seek() {
            at_ms = (at_ms + delta).clamp(from_ms, to_ms);
        }
        let paused = control.paused();
        if paused != was_paused {
            eased_since = Some((Instant::now(), speed, paused));
            was_paused = paused;
            control.settled.store(false, Ordering::SeqCst);
        }
        if let Some((since, from_speed, to_pause)) = eased_since {
            let k = ease(since.elapsed().as_secs_f64() / EASE.as_secs_f64());
            let target = if to_pause { 0.0 } else { 1.0 };
            speed = from_speed + (target - from_speed) * k;
            if k >= 1.0 {
                eased_since = None;
                speed = target;
            }
        }
        let now = Instant::now();
        at_ms += now.duration_since(tick).as_secs_f64() * 1000.0 * rate * speed;
        tick = now;
        if at_ms > to_ms {
            at_ms = from_ms;
        }
        if speed <= 0.0 {
            control.settled.store(true, Ordering::SeqCst);
            continue;
        }
        if !asked || drawn.is_some_and(|at| now.duration_since(at) < step.mul_f64(0.9)) {
            continue;
        }
        drawn = Some(now);
        let pixmap = scene.frame(at_ms, &layout);
        let mut rgba = pixmap.take();
        dim_rows(&mut rgba, SIZE.0 as usize, SIZE.1 as usize);
        let frame = crate::film::Frame::new(reel, SIZE.0, SIZE.1, rgba);
        if !push(Frame::Picture { frame, at_ms, from_ms, to_ms }) {
            return Ok(());
        }
    }
}

pub fn trace(replay: &std::path::Path) -> Vec<(f64, f32, f32)> {
    let Ok(bytes) = std::fs::read(replay) else {
        return Vec::new();
    };
    let Ok(parsed) = dossier_replay::Replay::parse(&bytes) else {
        return Vec::new();
    };
    parsed
        .frames
        .iter()
        .filter(|f| f.time_ms >= 0)
        .map(|f| (f.time_ms as f64, f.x, f.y))
        .collect()
}

const DIM: [(f32, f32); 5] = [(0.0, 0.96), (0.28, 0.4), (0.5, 0.4), (0.7, 0.93), (1.0, 0.99)];

pub fn dim_at(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    for pair in DIM.windows(2) {
        let ((t0, a0), (t1, a1)) = (pair[0], pair[1]);
        if t <= t1 {
            let k = if t1 > t0 { (t - t0) / (t1 - t0) } else { 1.0 };
            let soft = k * k * (3.0 - 2.0 * k);
            return a0 + (a1 - a0) * soft;
        }
    }
    DIM[DIM.len() - 1].1
}

fn dim_rows(rgba: &mut [u8], width: usize, height: usize) {
    let ground = [crate::theme::GROUND.r, crate::theme::GROUND.g, crate::theme::GROUND.b].map(|c| c * 255.0);
    for y in 0..height {
        let a = dim_at(y as f32 / (height.max(2) - 1) as f32);
        let keep = ((1.0 - a) * 65536.0).round() as u32;
        let lift = ground.map(|c| (c * a * 65536.0 + 32768.0) as u32);
        let row = &mut rgba[y * width * 4..(y + 1) * width * 4];
        for pixel in row.chunks_exact_mut(4) {
            pixel[0] = ((u32::from(pixel[0]) * keep + lift[0]) >> 16).min(255) as u8;
            pixel[1] = ((u32::from(pixel[1]) * keep + lift[1]) >> 16).min(255) as u8;
            pixel[2] = ((u32::from(pixel[2]) * keep + lift[2]) >> 16).min(255) as u8;
            pixel[3] = 255;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dim_follows_its_curve_to_within_a_step() {
        let (width, height) = (3, 64);
        let ground = [crate::theme::GROUND.r, crate::theme::GROUND.g, crate::theme::GROUND.b].map(|c| c * 255.0);
        for shade in [0u8, 1, 17, 128, 200, 254, 255] {
            let mut rgba = vec![shade; width * height * 4];
            dim_rows(&mut rgba, width, height);
            for y in 0..height {
                let a = dim_at(y as f32 / (height - 1) as f32);
                for c in 0..3 {
                    let wanted = f32::from(shade) * (1.0 - a) + ground[c] * a;
                    let got = f32::from(rgba[y * width * 4 + c]);
                    assert!((got - wanted).abs() <= 1.0, "row {y}, channel {c}: {got} against {wanted}");
                }
                assert_eq!(rgba[y * width * 4 + 3], 255);
            }
        }
    }
}
