//! A render, from a play that has been judged to a file somebody can watch.
//!
//! What was spread through the command line's `video` command until 2026-09-02,
//! where nothing else could reach it. The parts that stayed behind are the ones
//! that are genuinely a caller's business: where a skin comes from, where a
//! font lives, which directory is scratch, and what to say about any of it.

use std::path::{Path, PathBuf};

use dossier_beatmap::Beatmap;
use dossier_render::{Leaderboard, Scene, Skin};
use dossier_replay::Replay;
use dossier_sim::GameState;

use crate::{events, hitsounds, locate, scenery, video};

/// The hit-sound track, built once the span is known.
///
/// A callback because building it is where the command line reports what a skin
/// resolved to and what it had to leave silent — tables that belong to a
/// terminal and not to a pipeline. The same shape [`crate::reel`] already uses.
pub type Hitsounds<'a> = dyn Fn(&video::Plan) -> Option<PathBuf> + 'a;

/// Everything a render was asked for, once the paths have been resolved.
///
/// `settings` carries what the encoder was told; the four fields of it that are
/// *worked out* rather than chosen — the music, the film behind, the hit-sound
/// track and where the camera draws in to — are filled in by [`render`].
pub struct Job<'a> {
    pub state: &'a GameState,
    pub beatmap: &'a Beatmap,
    pub replay: &'a Replay,
    pub map_text: &'a str,
    pub origin: &'a locate::Origin,
    /// Assembled, with its font already attached.
    pub skin: Skin,
    pub leaderboard: Leaderboard,
    /// Draw the play and nothing that talks about it.
    pub bare: bool,
    /// Whether a plain hit still sounds under a whistle — the skin's answer.
    pub layering: bool,
    pub behind: scenery::Behind<'a>,
    pub settings: video::Settings,
}

/// Draw it.
///
/// Returns where the file was written. The only failure is the encoder's:
/// everything else here is decoration, and decoration that cannot be found is
/// said out loud — see [`crate::notes`] — and then done without.
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

    // The hit sounds are laid on the video's own timebase, so the span has to
    // be worked out before anything is drawn — and worked out from settings
    // that draw nothing and report nothing.
    let probe = video::Settings {
        out: settings.out.clone(),
        ffmpeg: settings.ffmpeg.clone(),
        preset: settings.preset.clone(),
        audio: settings.audio.clone(),
        video: None,
        hitsounds: None,
        events: events::Events::wanted(false),
        // The probe draws nothing, so it has no camera to place.
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
    // What the play sounded, for the storyboard's triggers to answer to.
    let fired = hitsounds::sounded(state, beatmap, layering);
    let scene = scenery::dress(scene, &behind, beatmap, (map_text, origin), &fired);
    // Settled before the scene is finished with, because it decides what the
    // scene stands on: over a video the play is drawn on nothing.
    settings.video = scenery::film(&behind, map_text, origin, scene.skin());
    let scene = if settings.video.is_some() {
        scene.over_video()
    } else {
        scene
    };
    // Where the camera draws in to: where the cursor is at the moment being
    // slowed into — the place on the field the play is at, which is where the
    // eye already is.
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

/// The file a render would write to, checked before four minutes are spent on
/// finding out it cannot be.
pub fn check_output(path: &Path) -> Result<(), String> {
    video::check_output(path)
}
