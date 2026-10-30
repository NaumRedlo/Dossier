use std::path::{Path, PathBuf};

use dossier_beatmap::Beatmap;
use dossier_render::{Leaderboard, Scene, Skin};
use dossier_replay::Replay;
use dossier_sim::GameState;

use crate::{events, hitsounds, locate, scenery, video};

pub type Hitsounds<'a> = dyn Fn(&video::Plan) -> Option<PathBuf> + 'a;

pub struct Job<'a> {
    pub state: &'a GameState,
    pub beatmap: &'a Beatmap,
    pub replay: &'a Replay,
    pub map_text: &'a str,
    pub origin: &'a locate::Origin,

    pub skin: Skin,
    pub leaderboard: Leaderboard,

    pub bare: bool,

    pub layering: bool,
    pub behind: scenery::Behind<'a>,
    pub settings: video::Settings,
}

pub fn render(job: Job<'_>, hitsounds: &Hitsounds<'_>) -> Result<PathBuf, String> {
    let Job {
        state,
        beatmap,
        replay,
        map_text,
        origin,
        skin,
        leaderboard,
        bare,
        layering,
        behind,
        mut settings,
    } = job;

    let probe = video::Settings {
        encoder: video::Encoder::Software,
        out: settings.out.clone(),
        ffmpeg: settings.ffmpeg.clone(),
        preset: settings.preset.clone(),
        audio: settings.audio.clone(),
        video: None,
        hitsounds: None,
        events: events::Events::wanted(false),

        slow_focus: None,
        ..settings
    };
    settings.hitsounds = video::Plan::new(
        state.span_ms(),
        state.playback_rate(),
        &probe,
        state.ending().map(|end| end.time_ms),
    )
    .ok()
    .and_then(|plan| hitsounds(&plan));

    let scene = Scene::new(state, skin)
        .signed_by(replay)
        .with_leaderboard(leaderboard);
    let scene = if bare { scene.bare() } else { scene };

    let fired = hitsounds::sounded(state, beatmap, layering);
    let scene = scenery::dress(scene, &behind, beatmap, (map_text, origin), &fired);

    settings.video = scenery::film(&behind, map_text, origin, scene.skin());
    let scene = if settings.video.is_some() {
        scene.over_video()
    } else {
        scene
    };

    settings.slow_focus = settings
        .slow_at_ms
        .and_then(|at| state.cursor_track().sample(at))
        .map(|cursor| cursor.pos);

    video::encode(
        &scene,
        state.span_ms(),
        state.playback_rate(),
        &settings,
        state.ending().map(|end| end.time_ms),
    )?;
    let size = std::fs::metadata(&settings.out)
        .map(|written| written.len())
        .unwrap_or(0);
    settings.events.wrote(&settings.out, size);
    Ok(settings.out)
}

pub const LAZER_FROM: i32 = 30_000_000;

pub struct ManiaJob<'a> {
    pub beatmap: &'a Beatmap,
    pub replay: &'a Replay,
    pub origin: &'a locate::Origin,
    pub font: Option<dossier_render::Font>,
    pub behind: scenery::Behind<'a>,
    pub settings: video::Settings,
}

pub struct ManiaJudged {
    pub playback: dossier_sim::mania::Playback,
    pub judge: dossier_sim::mania::judge::Judge,
    pub trace: Option<Vec<dossier_sim::mania::stable::TraceEvent>>,
}

fn judged_by(playback: dossier_sim::mania::Playback, od: f64, stable: bool) -> Result<ManiaJudged, dossier_sim::mania::Error> {
    if stable {
        let (judge, trace) = playback.trace_stable_with(playback.stable_rules(od)?)?;
        Ok(ManiaJudged { playback, judge, trace: Some(trace) })
    } else {
        let judge = playback.judge_with(playback.lazer_rules(od)?)?;
        Ok(ManiaJudged { playback, judge, trace: None })
    }
}

pub fn judge_mania(beatmap: &Beatmap, replay: &Replay) -> Result<ManiaJudged, String> {
    if beatmap.mode != 3 {
        return Err("this osu!mania replay was played on a map converted from another mode, which cannot be shown yet".to_owned());
    }
    let unshown = |why: dossier_sim::mania::Error| format!("this osu!mania replay cannot be shown yet: {why}");
    let playback = dossier_sim::mania::Playback::new(beatmap, replay).map_err(unshown)?;
    judged_by(playback, beatmap.difficulty.overall_difficulty, replay.game_version < LAZER_FROM).map_err(unshown)
}

pub fn mania_max_combo(beatmap: &Beatmap, replay: &Replay) -> Option<u32> {
    let playback = dossier_sim::mania::Playback::new(beatmap, replay).ok()?.perfect().ok()?;
    let judged = judged_by(playback, beatmap.difficulty.overall_difficulty, replay.game_version < LAZER_FROM).ok()?;
    judged.judge.final_combo().map(|combo| combo.maximum)
}

pub fn render_mania(job: ManiaJob<'_>) -> Result<PathBuf, String> {
    let ManiaJob { beatmap, replay, origin, font, behind, settings } = job;
    let ManiaJudged { playback, judge, trace } = judge_mania(beatmap, replay)?;
    let background = scenery::backdrop(&behind, beatmap, origin, &Skin::default());
    let mut scene = dossier_render::mania::Mania::new(&playback, &judge, trace.as_deref())
        .titled(format!("{} - {}", beatmap.metadata.artist, beatmap.metadata.title), beatmap.metadata.version.clone(), replay.player.clone())
        .over(background);
    if let Some(font) = font {
        scene = scene.with_font(font);
    }
    video::encode(&scene, scene.span_ms(), playback.clock_rate(), &settings, None)?;
    let size = std::fs::metadata(&settings.out).map(|written| written.len()).unwrap_or(0);
    settings.events.wrote(&settings.out, size);
    Ok(settings.out)
}

pub fn check_output(path: &Path) -> Result<(), String> {
    video::check_output(path)
}
