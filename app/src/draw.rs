//! Drawing a replay, with the engine linked in.
//!
//! The terminal client runs `dossier` as a program and reads its stderr. This
//! calls it. Everything the pipeline has to say arrives through
//! [`dossier_produce::notes`] instead of a stream nobody in a window is
//! watching — see [`Told`].

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use dossier_produce::{locate, render, scenery, video};
use dossier_render::Skin;
use dossier_sim::GameState;

/// What was said while a render happened.
///
/// Collected rather than printed. A window shows these; a terminal would have
/// let them scroll past.
#[derive(Default)]
pub struct Told(Arc<Mutex<Vec<String>>>);

impl Told {
    fn listen(&self) {
        let mine = Arc::clone(&self.0);
        dossier_produce::notes::listen(move |text| {
            mine.lock().expect("what was said").push(text.to_owned());
        });
    }

    pub fn said(&self) -> Vec<String> {
        self.0.lock().expect("what was said").clone()
    }
}

/// What a render was asked for, in the shape a window can fill in.
pub struct Asked<'a> {
    pub replay: &'a Path,
    pub songs: Option<&'a Path>,
    pub map: Option<&'a Path>,
    pub out: PathBuf,
    pub size: (u32, u32),
    pub fps: f64,
    /// The span to draw, in map time. `None` either side means the whole play.
    pub from_ms: Option<f64>,
    pub to_ms: Option<f64>,
    pub background: bool,
    pub storyboard: bool,
}

/// Draw it, and say what happened on the way.
pub fn draw(asked: &Asked<'_>, told: &Told) -> Result<PathBuf, String> {
    told.listen();
    let (beatmap, replay, origin, map_text) = locate::load(asked.replay, asked.map, asked.songs)?;
    let state = GameState::new(&beatmap, &replay);

    let mut skin = Skin::with_combo_colours(beatmap.combo_colours());
    match dossier_produce::font::find(None)? {
        Some(font) => skin = skin.with_font(font),
        None => dossier_produce::note!("no font found — drawing without numbers"),
    }
    let layering = skin
        .sprites
        .as_ref()
        .is_none_or(|s| s.ini().layered_hit_sounds);

    let scratch = std::env::temp_dir().join(format!("dossier-app-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).map_err(|e| format!("{e}"))?;

    let settings = video::Settings {
        out: asked.out.clone(),
        fps: asked.fps,
        size: asked.size,
        from_ms: asked.from_ms,
        to_ms: asked.to_ms,
        ffmpeg: "ffmpeg".to_owned(),
        crf: 20,
        preset: "medium".to_owned(),
        music_level: 1.0,
        hitsound_level: 1.0,
        threads: None,
        encoder_threads: None,
        audio: locate::extract_audio(&origin, &beatmap.audio_filename, &scratch),
        video: None,
        hitsounds: None,
        events: dossier_produce::events::Events::wanted(false),
        slow_at_ms: None,
        slow_focus: None,
    };
    let job = render::Job {
        state: &state,
        beatmap: &beatmap,
        replay: &replay,
        map_text: &map_text,
        origin: &origin,
        skin,
        leaderboard: dossier_render::Leaderboard::default(),
        bare: false,
        layering,
        behind: scenery::Behind {
            background: asked.background,
            storyboard: asked.storyboard,
            video: false,
            dim: None,
            blur: None,
            ffmpeg: "ffmpeg",
            size: asked.size,
            at_ms: None,
            scratch: Some(&scratch),
        },
        settings,
    };
    let nothing = |_: &video::Plan| None;
    let done = render::render(job, &nothing);
    dossier_produce::notes::unlisten();
    done
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Draw something, with the engine linked in rather than run.
    ///
    /// Ignored by default and driven by the environment: it wants a real replay
    /// and the map it was played on, and neither belongs in this repository —
    /// a replay carries somebody's name and the maps are not ours. Run it by
    /// hand when the pipeline has been moved about:
    ///
    /// ```text
    /// DOSSIER_TEST_REPLAY=… DOSSIER_TEST_SONGS=… cargo test -- --ignored
    /// ```
    #[test]
    #[ignore = "wants a replay and a map that this repository does not carry"]
    fn a_replay_becomes_a_file() {
        let (Some(replay), Some(songs)) = (
            std::env::var_os("DOSSIER_TEST_REPLAY"),
            std::env::var_os("DOSSIER_TEST_SONGS"),
        ) else {
            panic!("set DOSSIER_TEST_REPLAY and DOSSIER_TEST_SONGS");
        };
        let out = std::env::temp_dir().join("dossier-app-test.mp4");
        let _ = std::fs::remove_file(&out);
        let told = Told::default();
        let asked = Asked {
            replay: Path::new(&replay),
            songs: Some(Path::new(&songs)),
            map: None,
            out: out.clone(),
            size: (640, 360),
            fps: 24.0,
            // Three seconds: this is asking whether the pipeline is wired up,
            // not whether it can draw a whole play.
            from_ms: Some(30_000.0),
            to_ms: Some(33_000.0),
            background: false,
            storyboard: false,
        };
        let written = draw(&asked, &told).expect("it drew");
        let size = std::fs::metadata(&written).expect("a file").len();
        assert!(
            size > 10_000,
            "a file too small to be a video: {size} bytes"
        );
        for line in told.said() {
            println!("said: {line}");
        }
    }
}
