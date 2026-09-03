//! One play, in the shape a canvas can animate.
//!
//! The engine can already answer "where was everything at this instant" —
//! [`dossier_sim::GameState::update`] — but asking it sixty times a second
//! across the bridge is sixty round trips a second for a picture that is
//! already decided. So the whole play is handed over once, and the window
//! animates it from what it holds.
//!
//! What that costs is one big message. What it buys is a viewer that scrubs
//! instantly and does not stutter because a render thread was busy.

use std::path::Path;

use dossier_produce::locate;
use dossier_sim::{GameState, TimedKind};

/// How often the cursor and the slider balls are sampled, in map milliseconds.
///
/// Sixty a second: the same rate the picture is drawn at, so nothing is
/// interpolated that was not going to be looked at. Finer costs bytes for
/// movement nobody can see; coarser is visible on a fast stream.
const STEP_MS: f64 = 1000.0 / 60.0;

#[derive(serde::Serialize)]
pub struct Piece {
    pub kind: &'static str,
    pub x: f64,
    pub y: f64,
    pub start_ms: f64,
    pub end_ms: f64,
    /// Which number goes inside it, counting from one at every new combo.
    pub combo: u32,
    /// Index into `colours`.
    pub colour: usize,
    /// The slider's flattened path, `x, y, x, y…`, empty for anything else.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<f32>,
    /// Where the ball was, sampled from `start_ms` every `step_ms`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ball: Vec<f32>,
}

#[derive(serde::Serialize)]
pub struct Scene {
    pub radius: f64,
    pub preempt_ms: f64,
    pub fade_in_ms: f64,
    pub step_ms: f64,
    /// Where the sampled arrays start, in map time.
    pub from_ms: f64,
    pub to_ms: f64,
    pub colours: Vec<String>,
    pub objects: Vec<Piece>,
    /// `x, y` pairs, one per step from `from_ms`.
    pub cursor: Vec<f32>,
    /// What was held at each of those samples — the same bits the replay uses.
    pub keys: Vec<u8>,
    pub summary: crate::look::Judged,
}

/// Read the replay, find its map, and hand over everything at once.
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

    // The play starts one approach before the first object is due — that is
    // when it first appears — and ends where the judging did.
    let first = timeline.objects.first().map_or(0.0, |o| o.start_ms);
    let from_ms = (first - difficulty.preempt_ms()).max(0.0);
    let to_ms = timeline
        .objects
        .last()
        .map_or(from_ms, |o| o.end_ms + 1200.0);

    // The colour turns over at every new combo, so it starts one before the
    // first: the map's opening combo gets colour zero, which is what osu! shows.
    let mut number = 0;
    let mut colour = colours.len().saturating_sub(1);
    let objects = timeline
        .objects
        .iter()
        .map(|object| {
            if object.new_combo || number == 0 {
                number = 0;
                if !colours.is_empty() {
                    colour = (colour + 1) % colours.len();
                }
            }
            number += 1;
            let (kind, path, ball) = match &object.kind {
                TimedKind::Circle => ("circle", Vec::new(), Vec::new()),
                TimedKind::Spinner => ("spinner", Vec::new(), Vec::new()),
                TimedKind::Slider { path, slides, .. } => {
                    let line = path
                        .points()
                        .iter()
                        .flat_map(|point| [point.x as f32, point.y as f32])
                        .collect();
                    // The ping-pong across repeats is the path's own — asking
                    // it is the difference between the ball the engine draws
                    // and a second implementation of the same rule.
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
                    ("slider", line, balls)
                }
            };
            Piece {
                kind,
                x: object.pos.x,
                y: object.pos.y,
                start_ms: object.start_ms,
                end_ms: object.end_ms,
                combo: number,
                colour,
                path,
                ball,
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
                // Before the recording starts or after it stops. The last known
                // place, so the cursor does not jump to a corner.
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
