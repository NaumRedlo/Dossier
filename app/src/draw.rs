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
    /// Draw the play and nothing that talks about it.
    pub bare: bool,
    pub mute: bool,
    /// A skin folder, when one came with the job.
    pub skin: Option<PathBuf>,
    /// Say what is happening in a form a program can read — see
    /// [`dossier_produce::events`].
    pub events: bool,
    /// The knobs the engine has always had and the window never offered.
    pub fine: Fine,
}

/// What a render can be told beyond "draw this replay".
///
/// Every one of these was already a flag on the command line; none of them was
/// reachable from the window, which meant the application could draw exactly
/// one way and the terminal could draw twelve. Defaults are the engine's own,
/// so leaving the whole thing alone changes nothing.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct Fine {
    /// 0–51, lower is better and bigger. 20 is the engine's own.
    pub crf: u32,
    /// x264's speed/размер trade: `ultrafast` … `veryslow`.
    pub preset: String,
    /// How loud the song and the hit sounds are against each other.
    pub music_level: f32,
    pub hitsound_level: f32,
    /// Drawing threads, and the encoder's. `0` means "as many as sensible".
    pub threads: u32,
    pub encoder_threads: u32,
    /// How far the background is darkened and blurred, in per cent.
    pub dim: u32,
    pub blur: u32,
    /// Play the map's own video behind, when it has one.
    pub video: bool,
    /// Draw the play and nothing that talks about it — no HUD at all.
    pub bare: bool,
    /// Whether the cursor turns as it moves. `None` leaves the skin's answer.
    pub cursor_rotate: Option<bool>,
    /// The flash a struck note leaves on the field, from the skin's own
    /// `lighting.png`. osu! calls this Hit Lighting and ships it off; a render
    /// is watched rather than played, and what a player would call clutter is
    /// most of what a viewer came to see — so this one is on here.
    pub hit_lighting: bool,
    /// Whether a slider body grows into the field and retracts behind the ball.
    /// Off by default for the same reason it is on in the game and off here:
    /// snaking tells a *player* where the ball has yet to go, and a viewer can
    /// already see the whole shape.
    pub snake: bool,
    /// Whether the cursor swells under a click. The skin may still refuse.
    pub cursor_expand: bool,
    /// The map's own hit sounds — the folder beside the `.osu`, where a custom
    /// sample index means something. Most of what a hitsounded map sounds like.
    pub map_hitsounds: bool,
    /// The skin's, asked only by plain name. A skin without a given sound
    /// leaves it to the map, and both missing leaves it to synthesis.
    pub skin_hitsounds: bool,
    /// What synthesis sounds like when neither of the two covers a voice:
    /// `click`, `soft`, `drum`, `glass` or `wood`.
    pub kit: String,
    /// And how that synthesis is tuned — every frequency, every decay and the
    /// level, each as a multiplier on the pack's own.
    pub pitch: f32,
    pub decay: f32,
    pub kit_level: f32,
}

impl Default for Fine {
    fn default() -> Self {
        Self {
            crf: 20,
            preset: "medium".to_owned(),
            music_level: 1.0,
            hitsound_level: 1.0,
            threads: 0,
            encoder_threads: 0,
            dim: 0,
            blur: 0,
            video: false,
            bare: false,
            cursor_rotate: None,
            hit_lighting: true,
            snake: false,
            cursor_expand: true,
            map_hitsounds: true,
            skin_hitsounds: true,
            kit: "click".to_owned(),
            pitch: 1.0,
            decay: 1.0,
            kit_level: 1.0,
        }
    }
}

/// Draw it, and say what happened on the way.
pub fn draw(asked: &Asked<'_>, told: &Told) -> Result<PathBuf, String> {
    told.listen();
    let (beatmap, replay, origin, map_text) = locate::load(asked.replay, asked.map, asked.songs)?;
    let state = GameState::new(&beatmap, &replay);

    let mut skin = Skin::with_combo_colours(beatmap.combo_colours());
    if let Some(folder) = &asked.skin {
        skin = dossier_produce::skin::from_folder(skin, folder, None);
    }
    skin.cursor_rotate = asked.fine.cursor_rotate;
    skin.hit_lighting = asked.fine.hit_lighting;
    skin.snake_in = asked.fine.snake;
    skin.snake_out = asked.fine.snake;
    skin.cursor_expand = asked.fine.cursor_expand;
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
        crf: asked.fine.crf,
        preset: asked.fine.preset.clone(),
        music_level: asked.fine.music_level,
        hitsound_level: asked.fine.hitsound_level,
        threads: (asked.fine.threads > 0).then_some(asked.fine.threads as usize),
        encoder_threads: (asked.fine.encoder_threads > 0)
            .then_some(asked.fine.encoder_threads as usize),
        audio: if asked.mute {
            None
        } else {
            locate::extract_audio(&origin, &beatmap.audio_filename, &scratch)
        },
        video: None,
        hitsounds: None,
        events: dossier_produce::events::Events::wanted(asked.events),
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
        bare: asked.bare || asked.fine.bare,
        layering,
        behind: scenery::Behind {
            background: asked.background,
            storyboard: asked.storyboard,
            video: asked.fine.video,
            dim: (asked.fine.dim > 0).then_some(asked.fine.dim),
            blur: (asked.fine.blur > 0).then_some(asked.fine.blur),
            ffmpeg: "ffmpeg",
            size: asked.size,
            at_ms: None,
            scratch: Some(&scratch),
        },
        settings,
    };
    // The hit sounds. Without these the video is silent exactly where the play
    // was loud — the click of every note — which is most of what a hitsounded
    // map sounds like. The terminal client built this track and the window did
    // not, and a render that came out of the two was not the same render.
    //
    // Two places are asked, because osu! asks two and does not treat them
    // alike: the map's own folder first, where a custom sample index means
    // something, and the skin only ever by plain name.
    let kit = {
        let mut kit =
            dossier_audio::Kit::by_name(&asked.fine.kit).unwrap_or_else(dossier_audio::Kit::plain);
        kit.pitch *= asked.fine.pitch;
        kit.decay *= asked.fine.decay;
        kit.level *= asked.fine.kit_level;
        kit
    };
    let samples = {
        // Две стопки, и их можно брать по отдельности: скин отвечает на простое
        // имя, карта — на своё с номером набора, и звучат они по-разному. Кто
        // хочет слышать карту такой, какой её задумали, глушит скин; кому
        // важнее свой набор — наоборот.
        let mut pack = match &asked.skin {
            Some(folder) if asked.fine.skin_hitsounds => dossier_audio::SamplePack::load(folder),
            _ => dossier_audio::SamplePack::load(Path::new("")),
        };
        if asked.fine.map_hitsounds {
            let from_map = scratch.join("map-samples");
            if std::fs::create_dir_all(&from_map).is_ok()
                && locate::extract_samples(&origin, &from_map, "ffmpeg") > 0
            {
                pack = pack.with_beatmap(&from_map);
            }
        }
        pack
    };
    // `len` is the skin's alone and `from_beatmap` the map's — two counts and
    // not a total, which is why they are said apart.
    if samples.is_empty() {
        dossier_produce::note!("no hit-sound samples — the notes will be synthesised");
    } else {
        dossier_produce::note!(
            "hit sounds: {} from the skin, {} from the map",
            samples.len(),
            samples.from_beatmap()
        );
    }

    let muted = asked.mute;
    let sounds = |plan: &video::Plan| -> Option<PathBuf> {
        if muted {
            return None;
        }
        let track = dossier_produce::hitsounds::build(
            &state,
            &beatmap,
            |map_ms| plan.video_time_of(map_ms),
            plan.video_seconds,
            kit,
            samples.clone(),
            layering,
        );
        if track.is_empty() {
            return None;
        }
        let path = scratch.join("hitsounds.pcm");
        std::fs::write(&path, track.to_pcm()).ok()?;
        Some(path)
    };
    let done = render::render(job, &sounds);
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
            bare: false,
            // Unmuted on purpose: a test that mutes the render never asks
            // whether the hit sounds were built, which is how the window
            // shipped without them.
            mute: false,
            skin: None,
            events: false,
            background: false,
            storyboard: false,
            fine: Fine::default(),
        };
        let written = draw(&asked, &told).expect("it drew");
        let size = std::fs::metadata(&written).expect("a file").len();
        assert!(
            size > 10_000,
            "a file too small to be a video: {size} bytes"
        );

        // And it has sound. The window wrote silent videos for a while — the
        // hit-sound track was never built — and nothing here noticed, because
        // the only test of the pipeline muted the render it was checking.
        let streams = std::process::Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-show_entries",
                "stream=codec_type",
                "-of",
                "csv=p=0",
            ])
            .arg(&written)
            .output()
            .expect("ffprobe, which a render needs anyway");
        let said = String::from_utf8_lossy(&streams.stdout);
        assert!(said.contains("audio"), "the render came out silent: {said}");
        for line in told.said() {
            println!("said: {line}");
        }
    }
}
