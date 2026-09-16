use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use dossier_produce::{locate, scenery};
use iced::widget::image;

pub const SIZE: (u32, u32) = (960, 540);
pub const FPS: f64 = 30.0;

#[derive(Debug, Default)]
pub struct Control {
    stop: AtomicBool,
    paused: AtomicBool,
    seek: Mutex<Option<f64>>,
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

    fn take_seek(&self) -> Option<f64> {
        self.seek.lock().expect("the seek").take()
    }
}

#[derive(Debug, Clone)]
pub enum Frame {
    Picture { handle: image::Handle, at_ms: f64, from_ms: f64, to_ms: f64 },
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct Ask {
    pub replay: PathBuf,
    pub map: PathBuf,
    pub map_hash: String,
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
    if let Some(font) = dossier_produce::font::find(None)? {
        skin = skin.with_font(font);
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
    let mut clock = Instant::now();
    let mut base_ms = from_ms;
    let mut shown_ms = f64::NAN;
    loop {
        if control.stopped() {
            return Ok(());
        }
        if let Some(delta) = control.take_seek() {
            let now_ms = base_ms + clock.elapsed().as_secs_f64() * 1000.0 * rate;
            base_ms = (now_ms + delta).clamp(from_ms, to_ms);
            clock = Instant::now();
        }
        let paused = control.paused();
        let at_ms = if paused {
            base_ms
        } else {
            base_ms + clock.elapsed().as_secs_f64() * 1000.0 * rate
        };
        if paused {
            base_ms = at_ms;
            clock = Instant::now();
        }
        let at_ms = if at_ms > to_ms {
            base_ms = from_ms;
            clock = Instant::now();
            from_ms
        } else {
            at_ms
        };
        if (at_ms - shown_ms).abs() < 0.5 {
            std::thread::sleep(step);
            continue;
        }
        let drawn = Instant::now();
        let pixmap = scene.frame(at_ms, &layout);
        let mut rgba = pixmap.take();
        dim_rows(&mut rgba, SIZE.0 as usize, SIZE.1 as usize);
        let handle = image::Handle::from_rgba(SIZE.0, SIZE.1, rgba);
        shown_ms = at_ms;
        if !push(Frame::Picture { handle, at_ms, from_ms, to_ms }) {
            return Ok(());
        }
        let spent = drawn.elapsed();
        if spent < step {
            std::thread::sleep(step - spent);
        }
    }
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
    let ground = [
        (crate::theme::GROUND.r * 255.0) as f32,
        (crate::theme::GROUND.g * 255.0) as f32,
        (crate::theme::GROUND.b * 255.0) as f32,
    ];
    for y in 0..height {
        let a = dim_at(y as f32 / (height.max(2) - 1) as f32);
        let keep = 1.0 - a;
        let row = &mut rgba[y * width * 4..(y + 1) * width * 4];
        for pixel in row.chunks_exact_mut(4) {
            for c in 0..3 {
                pixel[c] = (pixel[c] as f32 * keep + ground[c] * a).round().clamp(0.0, 255.0) as u8;
            }
            pixel[3] = 255;
        }
    }
}
