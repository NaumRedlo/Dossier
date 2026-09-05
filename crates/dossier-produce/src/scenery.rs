use std::path::Path;

use dossier_beatmap::Beatmap;
use dossier_render::{Scene, Skin};

use crate::{locate, video};

pub struct Behind<'a> {
    pub background: bool,
    pub storyboard: bool,
    pub video: bool,
    pub dim: Option<u32>,
    pub blur: Option<u32>,

    pub ffmpeg: &'a str,

    pub size: (u32, u32),

    pub at_ms: Option<f64>,

    pub scratch: Option<&'a Path>,
}

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
        .map(|bytes| storyboard::parse(&String::from_utf8_lossy(&bytes)))
        .unwrap_or_default();
    board.absorb(storyboard::parse(map_text));
    if board.sprites.is_empty() {
        return None;
    }

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
