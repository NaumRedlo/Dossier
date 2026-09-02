//! What goes behind the play: the map's artwork, its storyboard, its video.
//!
//! None of it is the play, and none of it is a reason to fail. A background is
//! decoration — a map whose artwork is in a format nobody can read is still a
//! map worth watching — so everything here answers with `None` and says why on
//! stderr rather than stopping a render four minutes long.

use std::path::Path;

use dossier_beatmap::Beatmap;
use dossier_render::{Scene, Skin};

use crate::{locate, video};

/// What a render was told to put behind the play.
///
/// The command line reads these off its own options and an application off its
/// own settings; neither shape belongs in here. `dim` and `blur` are the
/// percentages a person types, and `None` leaves the skin's own answer alone.
pub struct Behind<'a> {
    pub background: bool,
    pub storyboard: bool,
    pub video: bool,
    pub dim: Option<u32>,
    pub blur: Option<u32>,
    /// The encoder to run, for the one frame a still needs out of a video.
    pub ffmpeg: &'a str,
    /// The frame being drawn, which the artwork is prepared for.
    pub size: (u32, u32),
    /// For a still: the moment its video frame is taken from. `None` for a
    /// render, which hands the whole film to the encoder instead and lets it
    /// composite in order — see [`film`].
    pub at_ms: Option<f64>,
    /// Somewhere to unpack a video to, when there is one.
    pub scratch: Option<&'a Path>,
}

/// The map's artwork, prepared for a frame of this size — or nothing, when it
/// was not asked for, the map names none, or the file will not decode.
///
/// Never a hard failure: a background is the one part of a render the play does
/// not depend on, and a map whose artwork is a format we cannot read is still a
/// map worth watching.
pub fn backdrop(
    behind: &Behind<'_>,
    beatmap: &Beatmap,
    origin: &locate::Origin,
    skin: &Skin,
) -> Option<dossier_render::Pixmap> {
    let size = behind.size;
    if !behind.background {
        return None;
    }
    let filename = beatmap.background.as_deref()?;
    let bytes = locate::read_background(origin, filename)?;
    let prepared = dossier_render::background::prepare(
        &bytes,
        size.0,
        size.1,
        behind
            .dim
            .map_or(skin.background_dim, |at| at as f32 / 100.0),
        // A share of what the skin blurs by, so zero is a sharp picture and a
        // hundred is what a render has always looked like — rather than a
        // figure in frame-heights that means nothing to anybody setting it.
        behind.blur.map_or(skin.background_blur, |at| {
            skin.background_blur * at as f32 / 100.0
        }),
        skin.background,
    );
    if prepared.is_none() {
        crate::note!("could not read the background `{filename}` — rendering without it");
    }
    prepared
}

/// One frame of the map's video, as a backdrop.
///
/// `video` hands the file to ffmpeg and lets it composite underneath; a single
/// picture has no pipeline to hand it to, so the frame is fetched instead —
/// one seek, one decode, and then it is a background like any other.
///
/// It takes the place of the artwork rather than sitting over it: a video is
/// what the map wanted behind the play at that moment, and drawing the still
/// picture on top of it would hide the thing that was asked for.
pub fn still(
    behind: &Behind<'_>,
    map_text: &str,
    origin: &locate::Origin,
    skin: &Skin,
) -> Option<dossier_render::Pixmap> {
    let (size, at_ms, scratch) = (behind.size, behind.at_ms?, behind.scratch);
    if !behind.video {
        return None;
    }
    let named = dossier_beatmap::storyboard::parse(map_text).video?;
    let Some(path) = scratch.and_then(|dir| locate::extract_video(origin, &named.path, dir)) else {
        crate::note!(
            "the map names a video `{}` that did not come with it — drawing without it",
            named.path
        );
        return None;
    };
    // Seek in the video's own time: a map states when the video starts, and it
    // is routinely negative.
    let seconds = ((at_ms - named.start_ms) / 1000.0).max(0.0);
    let shot = std::process::Command::new(behind.ffmpeg)
        .args(["-nostdin", "-loglevel", "error", "-ss"])
        .arg(format!("{seconds:.3}"))
        .arg("-i")
        .arg(&path)
        .args(["-frames:v", "1", "-f", "image2pipe", "-vcodec", "png", "-"])
        .output();
    let bytes = match shot {
        Ok(done) if done.status.success() && !done.stdout.is_empty() => done.stdout,
        Ok(done) => {
            crate::note!(
                "could not take a frame of `{}` at {seconds:.3}s — drawing without it{}",
                named.path,
                match String::from_utf8_lossy(&done.stderr).trim() {
                    "" => String::new(),
                    said => format!(": {said}"),
                }
            );
            return None;
        }
        Err(error) => {
            crate::note!("could not run `{}`: {error}", behind.ffmpeg);
            return None;
        }
    };
    // The dim the video gets under `video`, and no blur: ffmpeg does not blur
    // it there either, so the two commands light the same frame the same way.
    dossier_render::background::prepare(
        &bytes,
        size.0,
        size.1,
        behind
            .dim
            .map_or(skin.background_dim, |at| at as f32 / 100.0),
        0.0,
        skin.background,
    )
}

/// The map's background video, put where ffmpeg can open it.
///
/// The dim is the artwork's, so a render with `--video` and one with
/// `--background` are lit the same — turning one on should not change how
/// bright the play looks.
pub fn film(
    behind: &Behind<'_>,
    map_text: &str,
    origin: &locate::Origin,
    skin: &Skin,
) -> Option<video::Backdrop> {
    let scratch = behind.scratch;
    if !behind.video {
        return None;
    }
    let named = dossier_beatmap::storyboard::parse(map_text).video?;
    let Some(path) = scratch.and_then(|dir| locate::extract_video(origin, &named.path, dir)) else {
        crate::note!(
            "the map names a video `{}` that did not come with it — rendering without it",
            named.path
        );
        return None;
    };
    Some(video::Backdrop {
        path,
        start_ms: named.start_ms,
        dim: behind
            .dim
            .map_or(skin.background_dim, |at| at as f32 / 100.0),
    })
}

/// The map's own storyboard, with every picture it names.
///
/// Never a hard failure, for the same reason as the artwork: a storyboard is
/// the one part of a render the play does not depend on.
///
/// Both files are read. A `.osb` belongs to the whole set and is read first; a
/// difficulty's own `[Events]` is added over it, which is the order the game
/// draws them in.
pub fn show(
    behind: &Behind<'_>,
    map_text: &str,
    origin: &locate::Origin,
    sounds: &[dossier_beatmap::storyboard::Sounded],
) -> Option<dossier_render::storyboard::Show> {
    use dossier_beatmap::storyboard;

    if !behind.storyboard {
        return None;
    }
    let mut assets = locate::Assets::open(origin);
    let mut board = assets
        .osb()
        .and_then(|name| assets.read(&name))
        // Lossily: a stray byte in a comment somewhere should not cost the
        // whole storyboard, and every line this cares about is ASCII.
        .map(|bytes| storyboard::parse(&String::from_utf8_lossy(&bytes)))
        .unwrap_or_default();
    board.absorb(storyboard::parse(map_text));
    if board.sprites.is_empty() {
        return None;
    }
    // Triggers wait in the parsed board for somebody to say what happened. This
    // is that: the sounds the play actually made, laid down as ordinary
    // commands from the moment each one sounded.
    let board = board.fired(sounds);
    let sprites = board.sprites.len();
    let show = dossier_render::storyboard::Show::load(board, |path| assets.read(path));
    if show.is_empty() {
        crate::note!(
            "the storyboard names {sprites} sprite(s) and none of their \
             pictures came with the map — rendering without it"
        );
        return None;
    }
    Some(show)
}

/// Put behind the play everything the map asked for.
///
/// Three commands drew a frame and each did this itself, in the same order,
/// with the same two `match`es — and the fourth caller now is not a command at
/// all but an application. Somewhere for the four of them to agree.
///
/// The order is the order the game draws in: the artwork behind, the storyboard
/// over it, the play on top. A still takes one frame of the map's video in
/// place of its artwork when it was asked for both; a render hands the whole
/// film to the encoder instead — see [`film`] — because that is the one part of
/// this that reads frames in order.
pub fn dress<'a>(
    scene: Scene<'a>,
    behind: &Behind<'_>,
    beatmap: &Beatmap,
    map: (&str, &locate::Origin),
    sounded: &[dossier_beatmap::storyboard::Sounded],
) -> Scene<'a> {
    let (map_text, origin) = map;
    let art = still(behind, map_text, origin, scene.skin())
        .or_else(|| backdrop(behind, beatmap, origin, scene.skin()));
    let scene = match art {
        Some(art) => scene.with_backdrop(art),
        None => scene,
    };
    match show(behind, map_text, origin, sounded) {
        Some(show) => scene.with_storyboard(show),
        None => scene,
    }
}
