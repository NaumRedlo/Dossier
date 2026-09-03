//! What a replay was worth, in the shape a timeline can draw.
//!
//! The engine already answers this — [`dossier_sim::Judge`] holds every
//! judgement with its time, its verdict, how far off the click was and what the
//! combo became. This puts that in front of somebody without rendering a single
//! frame: a play can be looked at in the time it takes to read the file.

use std::path::Path;

use dossier_produce::locate;
use dossier_sim::{GameState, Part};

/// One judgement, small on purpose: a map has a couple of thousand of these and
/// they all go through the bridge at once.
#[derive(serde::Serialize)]
pub struct Mark {
    pub ms: f64,
    /// `300`, `100`, `50` or `0`.
    pub worth: u32,
    /// How early or late the click was, where that means anything.
    pub error_ms: Option<f64>,
    pub combo: u32,
    /// `circle`, `slider` or `spinner`.
    pub kind: &'static str,
}

#[derive(serde::Serialize)]
pub struct Counts {
    pub great: u32,
    pub ok: u32,
    pub meh: u32,
    pub miss: u32,
}

#[derive(serde::Serialize)]
pub struct Judged {
    pub title: String,
    pub player: String,
    pub mods: String,
    pub from_ms: f64,
    pub to_ms: f64,
    pub combo: u32,
    pub max_combo: u32,
    pub accuracy: f64,
    pub counts: Counts,
    pub unstable_rate: Option<f64>,
    pub marks: Vec<Mark>,
}

/// Read the replay, find its map, judge it.
pub fn look(replay: &Path, songs: Option<&Path>) -> Result<Judged, String> {
    let (beatmap, replay, _origin, _text) = locate::load(replay, None, songs)?;
    let state = GameState::new(&beatmap, &replay);
    let judge = state
        .judge()
        .ok_or_else(|| "судить нечего: в реплее нет ни одного нажатия".to_owned())?;

    // Only whole objects. A slider's ticks are judgements too, and putting them
    // on the same strip would bury the circles under them.
    let marks: Vec<Mark> = judge
        .events()
        .iter()
        .filter(|event| event.part.counts_for_accuracy())
        .map(|event| Mark {
            ms: event.time_ms,
            worth: event.result.value(),
            error_ms: event.error_ms,
            combo: event.combo_after,
            kind: match event.part {
                Part::Circle => "circle",
                Part::Slider => "slider",
                _ => "spinner",
            },
        })
        .collect();

    let last = judge.final_state();
    let ends = marks.last().map_or(0.0, |mark| mark.ms);
    Ok(Judged {
        title: format!(
            "{} — {} [{}]",
            beatmap.metadata.artist, beatmap.metadata.title, beatmap.metadata.version
        ),
        player: replay.player.clone(),
        mods: replay.mods.to_string(),
        from_ms: marks.first().map_or(0.0, |mark| mark.ms),
        to_ms: ends,
        combo: last.max_combo,
        max_combo: u32::from(replay.max_combo),
        accuracy: last.accuracy(),
        counts: Counts {
            great: u32::from(last.counts.count_300),
            ok: u32::from(last.counts.count_100),
            meh: u32::from(last.counts.count_50),
            miss: u32::from(last.counts.count_miss),
        },
        unstable_rate: judge.unstable_rate(ends + 1.0),
        marks,
    })
}
