use std::time::Instant;

use dossier_beatmap::Beatmap;
use dossier_render::{Layout, Scene, Skin};
use dossier_replay::{GameMode, HitCounts, Mods, Replay, ReplayFrame};
use dossier_sim::GameState;

const MAP: &str = "osu file format v14

[General]
StackLeniency: 0.7
Mode: 0

[Difficulty]
HPDrainRate:5
CircleSize:4
OverallDifficulty:8
ApproachRate:9
SliderMultiplier:1.4
SliderTickRate:1

[TimingPoints]
0,500,4,2,0,60,1,0

[HitObjects]
100,100,1000,1,0,0:0:0:0:
200,150,1150,1,0,0:0:0:0:
300,200,1300,1,0,0:0:0:0:
400,250,1450,1,0,0:0:0:0:
256,192,1600,2,0,L|400:300,1,140,0|0,0:0|0:0,0:0:0:0:
";

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct Speed {
    pub per_thread: f64,

    pub estimated: f64,
    pub frames: u32,
    pub width: u32,
    pub height: u32,
}

fn pretend() -> Replay {
    let frames = (0..200)
        .map(|step| ReplayFrame {
            time_ms: 900 + step * 5,
            x: 100.0 + step as f32 * 1.5,
            y: 100.0 + step as f32,
            keys: dossier_replay::Keys(if step % 8 < 4 { 5 } else { 0 }),
        })
        .collect();
    Replay {
        mode: GameMode::Standard,
        game_version: 20_260_711,
        beatmap_hash: String::new(),
        player: "измерение".to_owned(),
        replay_hash: String::new(),
        hits: HitCounts::default(),
        score: 0,
        max_combo: 0,
        perfect_combo: false,
        mods: Mods::default(),
        life_bar: String::new(),
        timestamp_ticks: 0,
        online_score_id: 0,
        target_practice_accuracy: None,
        frames,
        rng_seed: None,
        score_info: None,
    }
}

pub fn draw_speed(size: (u32, u32), frames: u32, threads: u32) -> Result<Speed, String> {
    let beatmap = Beatmap::parse(MAP).map_err(|e| e.to_string())?;
    let replay = pretend();
    let state = GameState::new(&beatmap, &replay);
    let skin = Skin::with_combo_colours(beatmap.combo_colours());
    let scene = Scene::new(&state, skin);
    let layout = Layout::new(size.0, size.1);

    let _ = scene.frame(1000.0, &layout);

    let began = Instant::now();
    for frame in 0..frames {
        let at = 1000.0 + f64::from(frame) * 10.0;
        let _ = scene.frame(at, &layout);
    }
    let seconds = began.elapsed().as_secs_f64().max(f64::EPSILON);
    let per_thread = f64::from(frames) / seconds;
    Ok(Speed {
        per_thread,
        estimated: per_thread * f64::from(threads.max(1)),
        frames,
        width: size.0,
        height: size.1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_machine_can_say_how_fast_it_draws() {
        let got = draw_speed((640, 360), 12, 4).expect("it drew");
        assert!(got.per_thread > 0.0, "no frames were drawn");
        assert!(
            got.estimated >= got.per_thread,
            "four threads cannot be slower than one"
        );
        assert_eq!(got.frames, 12);
    }

    #[test]
    fn a_larger_frame_costs_more_than_a_smaller_one() {
        let small = draw_speed((320, 180), 8, 1).expect("drew");
        let large = draw_speed((1280, 720), 8, 1).expect("drew");
        assert!(
            small.per_thread > large.per_thread,
            "small {:.0}/s against large {:.0}/s",
            small.per_thread,
            large.per_thread
        );
    }
}
