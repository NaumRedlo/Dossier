use std::path::Path;

use dossier_produce::locate;
use dossier_sim::{GameState, TimedKind};

const STEP_MS: f64 = 1000.0 / 60.0;

#[derive(serde::Serialize)]
pub struct Piece {
    pub kind: &'static str,
    pub x: f64,
    pub y: f64,
    pub start_ms: f64,
    pub end_ms: f64,

    pub combo: u32,

    pub run: usize,

    #[serde(skip_serializing_if = "is_one")]
    pub slides: u32,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<f32>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ball: Vec<f32>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ticks: Vec<f32>,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_one(slides: &u32) -> bool {
    *slides == 1
}

#[derive(serde::Serialize)]
pub struct Scene {
    pub radius: f64,
    pub preempt_ms: f64,
    pub fade_in_ms: f64,
    pub step_ms: f64,

    pub from_ms: f64,
    pub to_ms: f64,
    pub colours: Vec<String>,
    pub objects: Vec<Piece>,

    pub cursor: Vec<f32>,

    pub keys: Vec<u8>,
    pub summary: crate::look::Judged,
}

pub fn preview(replay: &Path, songs: Option<&Path>, seconds: f64) -> Result<Scene, String> {
    let mut scene = open(replay, songs)?;
    trim(&mut scene, seconds);
    Ok(scene)
}

fn busiest(objects: &[Piece], window: f64) -> f64 {
    let Some(first) = objects.first() else {
        return 0.0;
    };
    let mut best = (first.start_ms, 0usize);
    let mut low = 0;
    for high in 0..objects.len() {
        while objects[high].start_ms - objects[low].start_ms > window {
            low += 1;
        }
        if high - low + 1 > best.1 {
            best = (objects[low].start_ms, high - low + 1);
        }
    }
    best.0
}

fn trim(scene: &mut Scene, seconds: f64) {
    let window = seconds.max(1.0) * 1000.0;
    let busy = busiest(&scene.objects, window);

    let from = (busy - scene.preempt_ms).max(scene.from_ms);
    let to = busy + window;

    scene
        .objects
        .retain(|piece| piece.end_ms >= from && piece.start_ms <= to);
    scene
        .summary
        .marks
        .retain(|mark| mark.ms >= from && mark.ms <= to);

    let step = scene.step_ms.max(0.001);
    let held = scene.keys.len();
    let first = (((from - scene.from_ms) / step).floor().max(0.0) as usize).min(held);
    let last = ((((to - scene.from_ms) / step).ceil().max(0.0) as usize) + 1).min(held);
    if first < last {
        scene.cursor = scene.cursor[first * 2..last * 2].to_vec();
        scene.keys = scene.keys[first..last].to_vec();
        scene.from_ms += first as f64 * step;
    }
    scene.to_ms = to;
}

pub fn open(replay: &Path, songs: Option<&Path>) -> Result<Scene, String> {
    let (beatmap, replay, _origin, _text) = locate::load(replay, None, songs)?;
    let state = GameState::new(&beatmap, &replay);
    let summary = crate::look::summarise(&beatmap, &replay, &state)?;
    let timeline = state.timeline();
    let difficulty = &timeline.difficulty;

    let colours: Vec<String> = beatmap
        .combo_colours()
        .iter()
        .map(|c| format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b))
        .collect();

    let first = timeline.objects.first().map_or(0.0, |o| o.start_ms);
    let from_ms = (first - difficulty.preempt_ms()).max(0.0);
    let to_ms = timeline
        .objects
        .last()
        .map_or(from_ms, |o| o.end_ms + 1200.0);

    let mut number = 0;
    let mut run = 0;
    let objects = timeline
        .objects
        .iter()
        .map(|object| {
            if object.new_combo || number == 0 {
                if number > 0 {
                    run += 1;
                }
                number = 0;
            }
            number += 1;
            let (kind, slides, path, ball, ticks) = match &object.kind {
                TimedKind::Circle => ("circle", 1, Vec::new(), Vec::new(), Vec::new()),
                TimedKind::Spinner => ("spinner", 1, Vec::new(), Vec::new(), Vec::new()),
                TimedKind::Slider { path, slides, .. } => {
                    let line = path
                        .points()
                        .iter()
                        .flat_map(|point| [point.x as f32, point.y as f32])
                        .collect();

                    let mut balls = Vec::new();
                    let span = (object.end_ms - object.start_ms).max(1.0);
                    let mut at = 0.0;
                    while at <= span {
                        let progress = at / span * f64::from(*slides);
                        if let Some(point) = path.position_at_slide(progress, *slides) {
                            balls.push(point.x as f32);
                            balls.push(point.y as f32);
                        }
                        at += STEP_MS;
                    }

                    let mut ticks = Vec::new();
                    for at in object.tick_times() {
                        if let Some(point) = object.ball_at(at) {
                            ticks.push(at as f32);
                            ticks.push(point.x as f32);
                            ticks.push(point.y as f32);
                        }
                    }
                    ("slider", *slides, line, balls, ticks)
                }
            };
            Piece {
                kind,
                x: object.pos.x,
                y: object.pos.y,
                start_ms: object.start_ms,
                end_ms: object.end_ms,
                combo: number,
                run,
                slides,
                path,
                ball,
                ticks,
            }
        })
        .collect();

    let track = state.cursor_track();
    let steps = ((to_ms - from_ms) / STEP_MS).ceil().max(1.0) as usize;
    let mut cursor = Vec::with_capacity(steps * 2);
    let mut keys = Vec::with_capacity(steps);
    for step in 0..steps {
        let at = from_ms + step as f64 * STEP_MS;
        match track.sample(at) {
            Some(seen) => {
                cursor.push(seen.pos.x as f32);
                cursor.push(seen.pos.y as f32);
                keys.push(seen.keys.0);
            }
            None => {
                let last = cursor.len();
                cursor.push(if last >= 2 { cursor[last - 2] } else { 256.0 });
                cursor.push(if last >= 2 { cursor[last - 1] } else { 192.0 });
                keys.push(0);
            }
        }
    }

    Ok(Scene {
        radius: difficulty.circle_radius(),
        preempt_ms: difficulty.preempt_ms(),
        fade_in_ms: difficulty.fade_in_ms(),
        step_ms: STEP_MS,
        from_ms,
        to_ms,
        colours,
        objects,
        cursor,
        keys,
        summary,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(times: &[f64]) -> Vec<Piece> {
        times
            .iter()
            .map(|&start_ms| Piece {
                kind: "circle",
                x: 0.0,
                y: 0.0,
                start_ms,
                end_ms: start_ms,
                combo: 1,
                run: 0,
                slides: 1,
                path: Vec::new(),
                ball: Vec::new(),
                ticks: Vec::new(),
            })
            .collect()
    }

    #[test]
    fn the_busiest_window_is_found_rather_than_the_first_one() {
        let objects = at(&[
            0.0, 2000.0, 4000.0, 6000.0, 20_000.0, 20_200.0, 20_400.0, 20_600.0, 20_800.0,
        ]);
        assert_eq!(busiest(&objects, 3000.0), 20_000.0);
    }

    #[test]
    fn a_map_with_nothing_in_it_still_answers() {
        assert_eq!(busiest(&[], 3000.0), 0.0);
    }
}
