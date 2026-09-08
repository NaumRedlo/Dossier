use dossier_beatmap::Beatmap;
use dossier_replay::Replay;
use dossier_sim::{GameState, Part};

#[derive(serde::Serialize)]
pub struct Mark {
    pub ms: f64,

    pub object_index: usize,

    pub worth: u32,

    pub error_ms: Option<f64>,
    pub combo: u32,

    pub kind: &'static str,

    pub x: f64,
    pub y: f64,
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

    pub combo_recorded: u32,

    pub accuracy_percent: f64,
    pub counts: Counts,
    pub unstable_rate: Option<f64>,

    pub combo_possible: u32,

    pub outcome: Outcome,

    pub client: Client,

    pub presses: Vec<f32>,
    pub marks: Vec<Mark>,
}

#[derive(serde::Serialize)]
pub struct Outcome {
    pub kind: &'static str,

    pub share: f64,
    pub misses: u32,
}

#[derive(serde::Serialize)]
pub struct Client {
    pub name: &'static str,
    pub version: i32,

    pub build: String,

    pub played_at: String,
}

const TICKS_TO_UNIX: i64 = 62_135_596_800;

fn client_of(replay: &Replay) -> Client {
    let version = replay.game_version;
    let name = if version >= 30_000_000 {
        "lazer"
    } else if version >= 20_070_000 {
        "stable"
    } else {
        "неизвестно"
    };

    let build = if name == "osu!stable" {
        format!(
            "{:04}-{:02}-{:02}",
            version / 10_000,
            version / 100 % 100,
            version % 100
        )
    } else {
        String::new()
    };
    let seconds = replay.timestamp_ticks / 10_000_000 - TICKS_TO_UNIX;
    Client {
        name,
        version,
        build,
        played_at: if seconds > 0 {
            crate::logbook::stamp(seconds as u64)
        } else {
            String::new()
        },
    }
}

fn place(state: &GameState, index: usize) -> (f64, f64) {
    let Some(object) = state.timeline().objects.get(index) else {
        return (0.0, 0.0);
    };
    match object.ball_at(object.end_ms) {
        Some(end) => (end.x, end.y),
        None => (object.pos.x, object.pos.y),
    }
}

pub fn summarise(beatmap: &Beatmap, replay: &Replay, state: &GameState) -> Result<Judged, String> {
    let judge = state
        .judge()
        .ok_or_else(|| "Судить нечего: в реплее нет ни одного нажатия".to_owned())?;

    let presses: Vec<f32> = judge
        .events()
        .iter()
        .filter(|event| {
            matches!(
                event.part,
                Part::Circle | Part::SliderHead | Part::SliderTick | Part::SliderRepeat
            )
        })
        .filter_map(|event| event.error_ms)
        .map(|error| error as f32)
        .collect();

    let marks: Vec<Mark> = judge
        .events()
        .iter()
        .filter(|event| event.part.counts_for_accuracy())
        .map(|event| Mark {
            ms: event.time_ms,
            object_index: event.object_index,
            worth: event.result.value(),
            error_ms: event.error_ms,
            combo: event.combo_after,
            kind: match event.part {
                Part::Circle => "circle",
                Part::Slider => "slider",
                _ => "spinner",
            },

            x: place(state, event.object_index).0,
            y: place(state, event.object_index).1,
        })
        .collect();

    let last = judge.final_state();
    let ends = marks.last().map_or(0.0, |mark| mark.ms);

    let objects = state.timeline().objects.len().max(1);
    let possible = state.max_possible_combo();
    let misses = u32::from(last.counts.count_miss);

    let outcome = if state.ending().is_some() {
        Outcome {
            kind: "fail",
            share: state.objects_played() as f64 / objects as f64 * 100.0,
            misses,
        }
    } else if misses == 0 && last.max_combo >= possible {
        Outcome {
            kind: "fc",
            share: 100.0,
            misses: 0,
        }
    } else if misses == 0 {
        Outcome {
            kind: "break",
            share: 100.0,
            misses: 0,
        }
    } else {
        Outcome {
            kind: "miss",
            share: 100.0,
            misses,
        }
    };
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
        combo_recorded: u32::from(replay.max_combo),
        accuracy_percent: last.accuracy(),
        counts: Counts {
            great: u32::from(last.counts.count_300),
            ok: u32::from(last.counts.count_100),
            meh: u32::from(last.counts.count_50),
            miss: u32::from(last.counts.count_miss),
        },
        unstable_rate: judge.unstable_rate(ends + 1.0),
        combo_possible: possible,
        outcome,
        client: client_of(replay),
        presses,
        marks,
    })
}
