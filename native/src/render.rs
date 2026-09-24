use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use dossier_produce::{events, halt, locate, notes, render, scenery, video};

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    ReplayRead,
    MapOnDisk,
    Judged,
    Drawing { frames: u64, of: u64, left_seconds: f64 },
    Encoded,
    Saved(PathBuf),
    Stopped,
    Failed(String),
}

impl Step {
    pub fn is_last(&self) -> bool {
        matches!(self, Step::Saved(_) | Step::Stopped | Step::Failed(_))
    }
}

#[derive(Debug, Clone)]
pub struct Ask {
    pub replay: PathBuf,
    pub map: PathBuf,
    pub map_hash: String,
    pub ffmpeg: PathBuf,
    pub out: PathBuf,
    pub size: (u32, u32),
    pub fps: u32,
    pub crf: u32,
    pub skin: Option<PathBuf>,
    pub music_level: f32,
    pub hitsound_level: f32,
    pub play: Play,
}

#[derive(Debug, Clone, Copy)]
pub struct Play {
    pub hud: bool,
    pub cursor_grows: bool,
    pub dim: u32,
    pub blur: u32,
    pub map_sounds: bool,
    pub skin_sounds: bool,
}

impl Default for Play {
    fn default() -> Play {
        Play { hud: true, cursor_grows: false, dim: 82, blur: 100, map_sounds: true, skin_sounds: true }
    }
}

pub fn verdicts(replay: &Path, map: &Path, map_hash: &str) -> Result<Vec<(f64, String)>, String> {
    let bytes = std::fs::read(replay).map_err(|e| e.to_string())?;
    let replay = dossier_replay::Replay::parse(&bytes).map_err(|e| e.to_string())?;
    let found = locate::load_map(map, map_hash)?;
    let beatmap = dossier_beatmap::Beatmap::parse(&found.text).map_err(|e| e.to_string())?;
    let state = dossier_sim::GameState::new(&beatmap, &replay);
    let Some(judge) = state.judge() else {
        return Ok(Vec::new());
    };
    Ok(judge
        .events()
        .iter()
        .filter(|event| event.result != dossier_sim::Judgement::Great)
        .map(|event| (event.time_ms, format!("{:?}", event.result)))
        .collect())
}

const HUD_FACES: [&[u8]; 4] = [
    include_bytes!("../../assets/fonts/VarelaRound-Regular.ttf"),
    include_bytes!("../../assets/fonts/Commissioner-Regular.ttf"),
    include_bytes!("../../assets/fonts/MPLUSRounded1c-Regular.ttf"),
    include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf"),
];

pub fn hud_font() -> dossier_render::Font {
    static CHAIN: std::sync::OnceLock<dossier_render::Font> = std::sync::OnceLock::new();
    CHAIN
        .get_or_init(|| {
            let mut faces = HUD_FACES.iter().filter_map(|bytes| dossier_render::Font::from_bytes(bytes).ok());
            let front = faces.next().expect("the HUD's first face is built into the application");
            faces.fold(front, |built, behind| built.behind(&behind))
        })
        .clone()
}

pub fn still(replay: &Path, map: &Path, map_hash: &str, skin_folder: Option<&Path>, hud: bool, at_ms: f64, size: (u32, u32)) -> Result<Vec<u8>, String> {
    let bytes = std::fs::read(replay).map_err(|e| e.to_string())?;
    let replay = dossier_replay::Replay::parse(&bytes).map_err(|e| e.to_string())?;
    let found = locate::load_map(map, map_hash)?;
    let beatmap = dossier_beatmap::Beatmap::parse(&found.text).map_err(|e| e.to_string())?;
    let state = dossier_sim::GameState::new(&beatmap, &replay);
    let mut skin = dossier_render::Skin::with_combo_colours(beatmap.combo_colours());
    skin = skin.with_font(hud_font());
    if let Some(folder) = skin_folder.filter(|folder| folder.is_dir()) {
        skin = dossier_produce::skin::from_folder(skin, folder, None);
    }
    let scene = dossier_render::Scene::new(&state, skin).signed_by(&replay);
    let scene = if hud { scene } else { scene.bare() };
    let frame = scene.frame(at_ms, &dossier_render::Layout::new(size.0, size.1));
    frame.encode_png().map_err(|e| e.to_string())
}

fn sample_pack(play: &Play, skin: Option<&Path>, map_samples: Option<&Path>) -> dossier_audio::SamplePack {
    let pack = match skin.filter(|folder| play.skin_sounds && folder.is_dir()) {
        Some(folder) => dossier_audio::SamplePack::load(folder),
        None => dossier_audio::SamplePack::default(),
    };
    match map_samples.filter(|_| play.map_sounds) {
        Some(folder) => pack.with_beatmap(folder),
        None => pack,
    }
}

pub const SIZE: (u32, u32) = (1920, 1080);
pub const FPS: f64 = 60.0;

pub fn renders_dir() -> PathBuf {
    crate::sources::own_root().join("Renders")
}

pub fn file_name(player: &str, map_line: &str) -> String {
    let raw = format!("{player} - {map_line}");
    let clean: String = raw
        .chars()
        .map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { '_' } else { c })
        .collect();
    format!("{}.mp4", clean.trim())
}

pub fn stop() {
    halt::ask();
}

fn step_of(line: &str) -> Option<Step> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    match value.get("event")?.as_str()? {
        "progress" => Some(Step::Drawing {
            frames: value.get("frames")?.as_u64()?,
            of: value.get("of")?.as_u64()?,
            left_seconds: value.get("left_seconds")?.as_f64()?,
        }),
        "wrote" => Some(Step::Encoded),
        _ => None,
    }
}

pub fn run(ask: Ask) -> iced::Task<Step> {
    crate::ui::streamed(move |push| perform(ask, push))
}

static BUSY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static CPU: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(100);

pub fn share_cpu(percent: u32) {
    CPU.store(percent.clamp(10, 100), std::sync::atomic::Ordering::SeqCst);
}

pub fn threads() -> (Option<usize>, Option<usize>) {
    threads_of(CPU.load(std::sync::atomic::Ordering::SeqCst), std::thread::available_parallelism().map_or(1, |n| n.get()))
}

pub fn threads_of(share: u32, cores: usize) -> (Option<usize>, Option<usize>) {
    if share >= 100 {
        return (None, None);
    }
    let allowed = ((cores * share as usize + 50) / 100).clamp(1, cores.max(1));
    let draw = allowed.div_ceil(2);
    (Some(draw), Some(allowed.saturating_sub(draw).max(1)))
}

pub fn busy() -> bool {
    BUSY.load(std::sync::atomic::Ordering::SeqCst)
}

pub fn perform(ask: Ask, push: &mut dyn FnMut(Step) -> bool) {
    BUSY.store(true, std::sync::atomic::Ordering::SeqCst);
    {
        let (tell, heard) = std::sync::mpsc::channel::<Step>();
        let worker = std::thread::spawn(move || {
            halt::clear();
            let sink = Mutex::new(tell.clone());
            events::listen(move |line| {
                if let Some(step) = step_of(line) {
                    let _ = sink.lock().expect("the render's steps").send(step);
                }
            });
            notes::listen(|_| {});
            let outcome = draw(&ask, &tell);
            events::unlisten();
            notes::unlisten();
            let last = match outcome {
                Ok(path) => Step::Saved(path),
                Err(why) if halt::was_it(&why) || halt::asked() => {
                    let _ = std::fs::remove_file(&ask.out);
                    Step::Stopped
                }
                Err(why) => {
                    let _ = std::fs::remove_file(&ask.out);
                    Step::Failed(why)
                }
            };
            halt::clear();
            let _ = tell.send(last);
        });
        forward(heard, push);
        let _ = worker.join();
    }
    BUSY.store(false, std::sync::atomic::Ordering::SeqCst);
}

const EVERY: Duration = Duration::from_millis(120);

fn forward(heard: Receiver<Step>, push: &mut dyn FnMut(Step) -> bool) {
    let mut said = Instant::now() - EVERY;
    let mut held: Option<Step> = None;
    loop {
        match heard.recv_timeout(Duration::from_millis(60)) {
            Ok(step @ Step::Drawing { .. }) => {
                if said.elapsed() >= EVERY {
                    said = Instant::now();
                    held = None;
                    if !push(step) {
                        return;
                    }
                } else {
                    held = Some(step);
                }
            }
            Ok(step) => {
                let last = step.is_last();
                if !push(step) || last {
                    return;
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if let Some(step) = held.take() {
                    said = Instant::now();
                    if !push(step) {
                        return;
                    }
                }
            }
            Err(_) => return,
        }
    }
}

fn draw(ask: &Ask, tell: &Sender<Step>) -> Result<PathBuf, String> {
    let bytes = std::fs::read(&ask.replay).map_err(|e| e.to_string())?;
    let replay = dossier_replay::Replay::parse(&bytes).map_err(|e| e.to_string())?;
    let _ = tell.send(Step::ReplayRead);

    let found = locate::load_map(&ask.map, &ask.map_hash)?;
    let beatmap = dossier_beatmap::Beatmap::parse(&found.text).map_err(|e| e.to_string())?;
    let _ = tell.send(Step::MapOnDisk);

    let state = dossier_sim::GameState::new(&beatmap, &replay);
    let _ = tell.send(Step::Judged);
    let mut skin = dossier_render::Skin::with_combo_colours(beatmap.combo_colours());
    skin = skin.with_font(hud_font());
    if let Some(folder) = ask.skin.as_ref().filter(|folder| folder.is_dir()) {
        skin = dossier_produce::skin::from_folder(skin, folder, None);
    }
    skin.cursor_expand = ask.play.cursor_grows;
    let layering = skin.sprites.as_ref().is_none_or(|s| s.ini().layered_hit_sounds);

    let scratch = std::env::temp_dir().join(format!("dossier-native-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).map_err(|e| e.to_string())?;
    if let Some(dir) = ask.out.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let ffmpeg = ask.ffmpeg.display().to_string();

    let settings = video::Settings {
        out: ask.out.clone(),
        fps: ask.fps as f64,
        size: ask.size,
        from_ms: None,
        to_ms: None,
        ffmpeg: ffmpeg.clone(),
        crf: ask.crf,
        preset: "medium".to_owned(),
        music_level: ask.music_level,
        hitsound_level: ask.hitsound_level,
        threads: threads().0,
        encoder_threads: threads().1,
        audio: locate::extract_audio(&found.origin, &beatmap.audio_filename, &scratch),
        video: None,
        hitsounds: None,
        events: events::Events::wanted(true),
        slow_at_ms: None,
        slow_focus: None,
    };
    let job = render::Job {
        state: &state,
        beatmap: &beatmap,
        replay: &replay,
        map_text: &found.text,
        origin: &found.origin,
        skin,
        leaderboard: dossier_render::Leaderboard::default(),
        bare: !ask.play.hud,
        layering,
        behind: scenery::Behind {
            background: true,
            storyboard: false,
            video: false,
            dim: Some(ask.play.dim.min(100)),
            blur: Some(ask.play.blur.min(100)),
            ffmpeg: &ffmpeg,
            size: SIZE,
            at_ms: None,
            scratch: Some(&scratch),
        },
        settings,
    };

    let kit = dossier_audio::Kit::by_name("click").unwrap_or_else(dossier_audio::Kit::plain);
    let from_map = scratch.join("map-samples");
    let map_samples = ask.play.map_sounds
        && std::fs::create_dir_all(&from_map).is_ok()
        && locate::extract_samples(&found.origin, &from_map, &ffmpeg) > 0;
    let samples = sample_pack(&ask.play, ask.skin.as_deref(), map_samples.then_some(from_map.as_path()));
    let sounds = |plan: &video::Plan| -> Option<PathBuf> {
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

    let drawn = render::render(job, &sounds);
    let _ = std::fs::remove_dir_all(&scratch);
    drawn
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_share_of_the_processor_leaves_the_rest_alone() {
        assert_eq!(threads_of(100, 8), (None, None));
        assert_eq!(threads_of(50, 8), (Some(2), Some(2)));
        assert_eq!(threads_of(25, 12), (Some(2), Some(1)));
        assert_eq!(threads_of(75, 16), (Some(6), Some(6)));
        assert_eq!(threads_of(25, 2), (Some(1), Some(1)));
        assert_eq!(threads_of(75, 1), (Some(1), Some(1)));
    }

    #[test]
    fn progress_lines_become_steps() {
        assert_eq!(
            step_of(r#"{"event":"progress","frames":4512,"of":7280,"per_second":341.0,"left_seconds":18.2}"#),
            Some(Step::Drawing { frames: 4512, of: 7280, left_seconds: 18.2 })
        );
        assert_eq!(step_of(r#"{"event":"wrote","path":"a.mp4","bytes":10}"#), Some(Step::Encoded));
        assert_eq!(step_of(r#"{"event":"video","width":1920,"height":1080,"seconds":231.4}"#), None);
        assert_eq!(step_of("not json"), None);
    }

    fn folder_of_sounds(name: &str, leaves: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dossier-native-sounds-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a folder");
        let frames: Vec<u8> = (0..480i16).flat_map(|n| (n * 40).to_le_bytes()).collect();
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + frames.len() as u32).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&48_000u32.to_le_bytes());
        wav.extend_from_slice(&96_000u32.to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(frames.len() as u32).to_le_bytes());
        wav.extend_from_slice(&frames);
        for leaf in leaves {
            std::fs::write(dir.join(leaf), &wav).expect("a sound");
        }
        dir
    }

    #[test]
    fn without_the_maps_sounds_its_hitsounds_are_played_by_the_skin() {
        use dossier_audio::{Found, SampleSet, Voice};
        let skin = folder_of_sounds("skin", &["soft-hitwhistle.wav", "drum-hitclap.wav"]);
        let map = folder_of_sounds("map", &["soft-hitwhistle3.wav"]);
        let both = Play::default();
        let skin_only = Play { map_sounds: false, ..Play::default() };
        let neither = Play { map_sounds: false, skin_sounds: false, ..Play::default() };

        let pack = sample_pack(&both, Some(&skin), Some(&map));
        assert_eq!(pack.trace(SampleSet::Soft, Voice::Whistle, 3), Found::Beatmap(3));
        assert_eq!(pack.trace(SampleSet::Drum, Voice::Clap, 3), Found::SkinPlain);

        let pack = sample_pack(&skin_only, Some(&skin), Some(&map));
        assert_eq!(pack.trace(SampleSet::Soft, Voice::Whistle, 3), Found::SkinPlain, "the map's whistle is still heard, in the skin's voice");
        assert_eq!(pack.trace(SampleSet::Drum, Voice::Clap, 1), Found::SkinPlain);

        let pack = sample_pack(&neither, Some(&skin), Some(&map));
        assert_eq!(pack.trace(SampleSet::Soft, Voice::Whistle, 3), Found::Synthesised);

        let _ = std::fs::remove_dir_all(&skin);
        let _ = std::fs::remove_dir_all(&map);
    }

    #[test]
    fn the_hud_font_travels_inside_the_application() {
        let font = hud_font();
        assert_eq!(font.faces(), HUD_FACES.len());
        assert!(font.width("1984", 24.0) > 0.0);
        assert!(font.width("Съешь", 24.0) > 0.0, "no width for Cyrillic");
        assert!(font.width("結界", 24.0) > 0.0, "no width for a CJK title");
    }

    #[test]
    fn a_file_name_keeps_the_player_and_the_map_and_nothing_the_disk_rejects() {
        assert_eq!(file_name("Naum", "xi — Blue Zenith [FOUR DIMENSIONS]"), "Naum - xi — Blue Zenith [FOUR DIMENSIONS].mp4");
        assert_eq!(file_name("a/b", "c: d?"), "a_b - c_ d_.mp4");
    }
}
