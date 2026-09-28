use dossier_beatmap::Point;
use dossier_sim::{CursorTrack, GameState, Part};
use tiny_skia::Pixmap;

pub(super) const SIZE: f64 = 168.0;
const CHANGE_MS: f64 = 800.0;
const DIM_MS: f64 = 50.0;
const SLIDING_DIM: f32 = 0.8;
const FOLLOW_MS: f64 = 56.0;
const FOLLOW_STEP_MS: f64 = 4.0;
const FOLLOW_REACH: f64 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Change {
    from_ms: f64,
    from: f64,
    to_ms: f64,
    to: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Flashlight {
    sizes: Vec<Change>,
    sliding: Vec<(f64, f64)>,
    rate: f64,
}

pub(super) fn combo_scale(combo: u32) -> f64 {
    match combo {
        c if c > 200 => 0.625,
        c if c > 100 => 0.8125,
        _ => 1.0,
    }
}

fn value_of(changes: &[Change], time_ms: f64) -> f64 {
    let at = changes.partition_point(|change| change.from_ms <= time_ms);
    let Some(change) = at.checked_sub(1).map(|at| changes[at]) else {
        return changes.first().map_or(SIZE, |change| change.from);
    };
    if time_ms >= change.to_ms || change.to_ms <= change.from_ms {
        return change.to;
    }
    change.from + (change.to - change.from) * (time_ms - change.from_ms) / (change.to_ms - change.from_ms)
}

fn glided(start: f64, mut events: Vec<(f64, f64, f64)>) -> Vec<Change> {
    events.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut changes = vec![Change { from_ms: f64::NEG_INFINITY, from: start, to_ms: f64::NEG_INFINITY, to: start }];
    for (from_ms, to_ms, to) in events {
        let from = value_of(&changes, from_ms);
        changes.push(Change { from_ms, from, to_ms, to });
    }
    changes
}

fn merged(mut spans: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    spans.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out: Vec<(f64, f64)> = Vec::with_capacity(spans.len());
    for (from, to) in spans {
        match out.last_mut() {
            Some(last) if from <= last.1 => last.1 = last.1.max(to),
            _ => out.push((from, to)),
        }
    }
    out
}

impl Flashlight {
    pub(super) fn of(state: &GameState) -> Option<Flashlight> {
        if !state.mods().contains(dossier_replay::bits::FLASHLIGHT) {
            return None;
        }
        let objects = &state.timeline().objects;
        let first = objects.first()?.start_ms;
        let last = objects.iter().map(|object| object.end_ms).fold(f64::NEG_INFINITY, f64::max) + state.difficulty().hit_window_50() + 5.0;
        let rate = state.playback_rate().max(0.01);
        let change = CHANGE_MS * rate;
        let judged: Vec<(f64, u32)> = state.judge().map(|judge| judge.events().iter().map(|event| (event.time_ms, event.combo_after)).collect()).unwrap_or_default();
        let target_at = |time_ms: f64| {
            let at = judged.partition_point(|(when, _)| *when <= time_ms);
            SIZE * at.checked_sub(1).map_or(1.0, |at| combo_scale(judged[at].1))
        };
        let mut events = vec![(first - change, first, SIZE), (last, last + change, SIZE * 8.0)];
        events.extend(judged.iter().map(|(when, combo)| (*when, *when + change, SIZE * combo_scale(*combo))));
        for &(from, to) in &state.timeline().breaks {
            if to - from > change * 2.0 {
                events.push((from, from + change, SIZE * 2.5));
                events.push((to - change, to, target_at(from)));
            }
        }
        let mut held: std::collections::HashMap<usize, (Option<f64>, Option<f64>)> = std::collections::HashMap::new();
        if let Some(judge) = state.judge() {
            for event in judge.events() {
                if !objects.get(event.object_index).is_some_and(|object| object.is_slider()) {
                    continue;
                }
                let (pressed, stopped) = held.entry(event.object_index).or_insert((None, None));
                let missed = event.result.is_miss();
                match event.part {
                    Part::SliderHead | Part::SliderTick | Part::SliderRepeat if !missed => {
                        pressed.get_or_insert(event.time_ms);
                    }
                    Part::SliderTick | Part::SliderRepeat | Part::SliderTail => {
                        stopped.get_or_insert(event.time_ms);
                    }
                    _ => {}
                }
            }
        }
        let sliding = merged(
            held.into_iter()
                .filter_map(|(index, (pressed, stopped))| {
                    let from = pressed?;
                    let to = stopped.unwrap_or(objects[index].end_ms);
                    (to > from).then_some((from, to))
                })
                .collect(),
        );
        Some(Flashlight { sizes: glided(SIZE * 8.0, events), sliding, rate })
    }

    pub(super) fn size_at(&self, time_ms: f64) -> f64 {
        value_of(&self.sizes, time_ms)
    }

    pub(super) fn dim_at(&self, time_ms: f64) -> f32 {
        let span = DIM_MS * self.rate;
        let at = self.sliding.partition_point(|(from, _)| *from <= time_ms);
        let Some((from, to)) = at.checked_sub(1).map(|at| self.sliding[at]) else {
            return 0.0;
        };
        let share = if time_ms < to { (time_ms - from) / span } else { 1.0 - (time_ms - to) / span };
        SLIDING_DIM * share.clamp(0.0, 1.0) as f32
    }

    pub(super) fn centre(&self, track: &CursorTrack, time_ms: f64) -> Option<Point> {
        follow(track, time_ms, self.rate)
    }
}

pub(super) fn follow(track: &CursorTrack, time_ms: f64, rate: f64) -> Option<Point> {
    let reach = FOLLOW_MS * rate;
    let step = FOLLOW_STEP_MS * rate;
    let steps = (FOLLOW_REACH * reach / step).ceil() as usize;
    let (mut x, mut y, mut weight) = (0.0, 0.0, 0.0);
    for k in 0..=steps {
        let age = k as f64 * step;
        let Some(sample) = track.sample(time_ms - age) else {
            continue;
        };
        let w = (-age / reach).exp();
        x += sample.pos.x * w;
        y += sample.pos.y * w;
        weight += w;
    }
    (weight > 0.0).then(|| Point { x: x / weight, y: y / weight })
}

pub(super) fn shade(pixmap: &mut Pixmap, centre: (f32, f32), radius: f32, dim: f32) {
    let (width, height) = (pixmap.width() as i32, pixmap.height() as i32);
    let (cx, cy) = centre;
    let radius = radius.max(1.0);
    let left = ((cx - radius).floor() as i32).clamp(0, width);
    let right = ((cx + radius).ceil() as i32).clamp(0, width);
    let top = ((cy - radius).floor() as i32).clamp(0, height);
    let bottom = ((cy + radius).ceil() as i32).clamp(0, height);
    let data = pixmap.data_mut();
    let darken = |pixel: &mut [u8], a: f32| {
        let keep = 1.0 - a.clamp(0.0, 1.0);
        for channel in &mut pixel[..3] {
            *channel = (f32::from(*channel) * keep).round() as u8;
        }
        pixel[3] = (f32::from(pixel[3]) + (255.0 - f32::from(pixel[3])) * (1.0 - keep)).round() as u8;
    };
    let black = |pixels: &mut [u8]| {
        for pixel in pixels.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[0, 0, 0, 255]);
        }
    };
    for y in 0..height {
        let row = &mut data[(y * width * 4) as usize..((y + 1) * width * 4) as usize];
        if y < top || y >= bottom {
            black(row);
            continue;
        }
        let dy = y as f32 + 0.5 - cy;
        black(&mut row[..(left * 4) as usize]);
        black(&mut row[(right * 4) as usize..]);
        for (x, pixel) in row[(left * 4) as usize..(right * 4) as usize].chunks_exact_mut(4).enumerate() {
            let dx = (left + x as i32) as f32 + 0.5 - cx;
            let near = ((dx * dx + dy * dy).sqrt() / radius).min(1.0);
            let t = near.powi(5);
            darken(pixel, t + (1.0 - t) * dim);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_light_narrows_past_a_hundred_and_two_hundred_combo() {
        assert_eq!(combo_scale(0), 1.0);
        assert_eq!(combo_scale(100), 1.0);
        assert_eq!(combo_scale(101), 0.8125);
        assert_eq!(combo_scale(201), 0.625);
    }

    #[test]
    fn a_change_glides_from_where_the_last_one_had_got_to() {
        let changes = glided(1000.0, vec![(0.0, 800.0, 200.0), (400.0, 1200.0, 100.0)]);
        assert_eq!(value_of(&changes, -10.0), 1000.0);
        assert_eq!(value_of(&changes, 0.0), 1000.0);
        assert!((value_of(&changes, 400.0) - 600.0).abs() < 1e-9);
        assert!((value_of(&changes, 800.0) - 350.0).abs() < 1e-9);
        assert_eq!(value_of(&changes, 5000.0), 100.0);
    }

    #[test]
    fn held_sliders_join_when_they_touch() {
        assert_eq!(merged(vec![(500.0, 900.0), (100.0, 300.0), (250.0, 400.0)]), vec![(100.0, 400.0), (500.0, 900.0)]);
    }

    #[test]
    fn the_centre_is_lit_and_everything_past_the_edge_is_dark() {
        let mut pixmap = Pixmap::new(100, 60).unwrap();
        pixmap.fill(tiny_skia::Color::from_rgba8(200, 100, 50, 255));
        shade(&mut pixmap, (50.0, 30.0), 20.0, 0.0);
        let at = |x: u32, y: u32| pixmap.pixel(x, y).unwrap();
        assert_eq!((at(50, 30).red(), at(50, 30).alpha()), (200, 255), "the middle of the light is untouched");
        assert_eq!((at(5, 5).red(), at(5, 5).green(), at(5, 5).blue(), at(5, 5).alpha()), (0, 0, 0, 255), "outside it is black");
        assert!(at(66, 30).red() < at(58, 30).red(), "the light fades towards its rim");
        let mut dimmed = Pixmap::new(100, 60).unwrap();
        dimmed.fill(tiny_skia::Color::from_rgba8(200, 100, 50, 255));
        shade(&mut dimmed, (50.0, 30.0), 20.0, 0.8);
        assert_eq!(dimmed.pixel(50, 30).unwrap().red(), 40, "a held slider darkens even the middle");
    }

    #[test]
    fn a_clear_frame_is_blacked_out_not_left_see_through() {
        let mut pixmap = Pixmap::new(10, 10).unwrap();
        shade(&mut pixmap, (100.0, 100.0), 5.0, 0.0);
        assert_eq!(pixmap.pixel(0, 0).unwrap().alpha(), 255);
    }
}
