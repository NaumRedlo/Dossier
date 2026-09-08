use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use dossier_produce::{locate, render, scenery, video};
use dossier_render::Skin;
use dossier_sim::GameState;

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

pub struct Asked<'a> {
    pub replay: &'a Path,
    pub songs: Option<&'a Path>,
    pub map: Option<&'a Path>,
    pub out: PathBuf,
    pub size: (u32, u32),
    pub fps: f64,

    pub from_ms: Option<f64>,
    pub to_ms: Option<f64>,
    pub background: bool,
    pub storyboard: bool,

    pub bare: bool,
    pub mute: bool,

    pub skin: Option<PathBuf>,

    pub events: bool,

    pub fine: Fine,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct Fine {
    pub crf: u32,

    pub preset: String,

    pub music_level: f32,
    pub hitsound_level: f32,

    pub threads: u32,
    pub encoder_threads: u32,

    pub dim: u32,
    pub blur: u32,

    pub video: bool,

    pub bare: bool,

    pub cursor_rotate: Option<bool>,

    pub hit_lighting: bool,

    pub snake: bool,
    pub snake_in: bool,

    pub cursor_expand: bool,

    pub map_hitsounds: bool,

    pub skin_hitsounds: bool,


    pub kit: String,

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
            snake_in: false,
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
    skin.snake_in = asked.fine.snake_in;
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

    let kit = {
        let mut kit =
            dossier_audio::Kit::by_name(&asked.fine.kit).unwrap_or_else(dossier_audio::Kit::plain);
        kit.pitch *= asked.fine.pitch;
        kit.decay *= asked.fine.decay;
        kit.level *= asked.fine.kit_level;
        kit
    };
    let samples = {
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

            from_ms: Some(30_000.0),
            to_ms: Some(33_000.0),
            bare: false,

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
