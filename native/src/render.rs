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

    let mut skin = dossier_render::Skin::with_combo_colours(beatmap.combo_colours());
    if let Some(font) = dossier_produce::font::find(None)? {
        skin = skin.with_font(font);
    }
    let layering = skin.sprites.as_ref().is_none_or(|s| s.ini().layered_hit_sounds);

    let scratch = std::env::temp_dir().join(format!("dossier-native-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).map_err(|e| e.to_string())?;
    if let Some(dir) = ask.out.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let ffmpeg = ask.ffmpeg.display().to_string();

    let settings = video::Settings {
        out: ask.out.clone(),
        fps: FPS,
        size: SIZE,
        from_ms: None,
        to_ms: None,
        ffmpeg: ffmpeg.clone(),
        crf: 20,
        preset: "medium".to_owned(),
        music_level: 1.0,
        hitsound_level: 1.0,
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
        bare: false,
        layering,
        behind: scenery::Behind {
            background: true,
            storyboard: false,
            video: false,
            dim: None,
            blur: None,
            ffmpeg: &ffmpeg,
            size: SIZE,
            at_ms: None,
            scratch: Some(&scratch),
        },
        settings,
    };

    let kit = dossier_audio::Kit::by_name("click").unwrap_or_else(dossier_audio::Kit::plain);
    let samples = {
        let mut pack = dossier_audio::SamplePack::load(Path::new(""));
        let from_map = scratch.join("map-samples");
        if std::fs::create_dir_all(&from_map).is_ok() && locate::extract_samples(&found.origin, &from_map, &ffmpeg) > 0 {
            pack = pack.with_beatmap(&from_map);
        }
        pack
    };
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
    fn a_file_name_keeps_the_player_and_the_map_and_nothing_the_disk_rejects() {
        assert_eq!(file_name("Naum", "xi — Blue Zenith [FOUR DIMENSIONS]"), "Naum - xi — Blue Zenith [FOUR DIMENSIONS].mp4");
        assert_eq!(file_name("a/b", "c: d?"), "a_b - c_ d_.mp4");
    }
}
