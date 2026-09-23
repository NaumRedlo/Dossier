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

fn plain_sounds(beatmap: &dossier_beatmap::Beatmap) -> dossier_beatmap::Beatmap {
    let mut plain = beatmap.clone();
    plain.sample_set = dossier_beatmap::SampleSet::Normal;
    for point in &mut plain.timing.samples {
        point.set = dossier_beatmap::SampleSet::Normal;
        point.set_given = true;
        point.index = 1;
    }
    for object in &mut plain.objects {
        object.hit_sound = 0;
        object.hit_sample = dossier_beatmap::HitSample { volume: object.hit_sample.volume, ..dossier_beatmap::HitSample::default() };
        if let dossier_beatmap::ObjectKind::Slider(slider) = &mut object.kind {
            slider.edge_sounds.iter_mut().for_each(|sound| *sound = 0);
            slider.edge_sets.iter_mut().for_each(|set| *set = (0, 0));
        }
    }
    plain
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

pub fn perform(ask: Ask, push: &mut dyn FnMut(Step) -> bool) {
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
    let heard = match ask.play.map_sounds {
        true => beatmap.clone(),
        false => plain_sounds(&beatmap),
    };

    let mut skin = dossier_render::Skin::with_combo_colours(beatmap.combo_colours());
    if let Some(font) = dossier_produce::font::find(None)? {
        skin = skin.with_font(font);
    }
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
        threads: None,
        encoder_threads: None,
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
    let samples = {
        let mut pack = match ask.skin.as_ref().filter(|folder| folder.is_dir() && ask.play.skin_sounds) {
            Some(folder) => dossier_audio::SamplePack::load(folder),
            None => dossier_audio::SamplePack::load(Path::new("")),
        };
        let from_map = scratch.join("map-samples");
        if ask.play.map_sounds && std::fs::create_dir_all(&from_map).is_ok() && locate::extract_samples(&found.origin, &from_map, &ffmpeg) > 0 {
            pack = pack.with_beatmap(&from_map);
        }
        pack
    };
    let sounds = |plan: &video::Plan| -> Option<PathBuf> {
        let track = dossier_produce::hitsounds::build(
            &state,
            &heard,
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
    fn progress_lines_become_steps() {
        assert_eq!(
            step_of(r#"{"event":"progress","frames":4512,"of":7280,"per_second":341.0,"left_seconds":18.2}"#),
            Some(Step::Drawing { frames: 4512, of: 7280, left_seconds: 18.2 })
        );
        assert_eq!(step_of(r#"{"event":"wrote","path":"a.mp4","bytes":10}"#), Some(Step::Encoded));
        assert_eq!(step_of(r#"{"event":"video","width":1920,"height":1080,"seconds":231.4}"#), None);
        assert_eq!(step_of("not json"), None);
    }

    #[test]
    fn without_the_maps_sounds_every_hit_is_the_skins_plain_one() {
        let text = "osu file format v14

[General]
AudioFilename: audio.mp3
SampleSet: Soft

[Difficulty]
HPDrainRate:5
CircleSize:4
OverallDifficulty:8
ApproachRate:9
SliderMultiplier:1.4
SliderTickRate:1

[TimingPoints]
0,500,4,2,3,70,1,0

[HitObjects]
256,192,1000,1,10,2:3:4:60:
100,100,1500,2,8,L|200:100,1,100,2|10,1:2|3:0,0:0:0:0:
";
        let beatmap = dossier_beatmap::Beatmap::parse(text).expect("a map");
        let plain = plain_sounds(&beatmap);
        assert_eq!(plain.sample_set, dossier_beatmap::SampleSet::Normal);
        assert!(plain.timing.samples.iter().all(|p| p.set == dossier_beatmap::SampleSet::Normal && p.index == 1));
        assert!(plain.timing.samples.iter().zip(&beatmap.timing.samples).all(|(a, b)| a.volume == b.volume), "the map's loudness stays");
        for object in &plain.objects {
            assert_eq!(object.hit_sound, 0);
            assert_eq!((object.hit_sample.normal_set, object.hit_sample.addition_set, object.hit_sample.index), (0, 0, 0));
            if let dossier_beatmap::ObjectKind::Slider(slider) = &object.kind {
                assert!(slider.edge_sounds.iter().all(|sound| *sound == 0));
                assert!(slider.edge_sets.iter().all(|set| *set == (0, 0)));
            }
        }
    }

    #[test]
    fn a_file_name_keeps_the_player_and_the_map_and_nothing_the_disk_rejects() {
        assert_eq!(file_name("Naum", "xi — Blue Zenith [FOUR DIMENSIONS]"), "Naum - xi — Blue Zenith [FOUR DIMENSIONS].mp4");
        assert_eq!(file_name("a/b", "c: d?"), "a_b - c_ d_.mp4");
    }
}
