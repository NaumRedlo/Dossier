//! One job, start to finish.
//!
//! Ask for work; if there is any, fetch what it needs, draw it, hand the file
//! over. If anything goes wrong, give the job back saying why — a job nobody
//! hands back sits until its lease runs out, and whoever asked for it waits
//! that long for nothing.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Deserialize;

use crate::bot::{Bot, Capacity, Job, Refused};

/// How often to say we are still here while the engine has nothing to report.
const HEARTBEAT: Duration = Duration::from_secs(10);

/// What became of one turn of the loop.
#[derive(Debug)]
pub enum Did {
    /// Nobody had anything. Not a failure.
    Nothing,
    Delivered {
        title: String,
    },
    /// Handed back, with what to tell whoever asked.
    GaveBack {
        why: String,
    },
}

/// How far along a render is, for a window to show and a heartbeat to carry.
#[derive(Debug, Clone, Copy, Default)]
pub struct Along {
    pub frames: u64,
    pub of: u64,
    pub per_second: f64,
    pub left_seconds: f64,
}

/// The settings a job carries, as far as this understands them.
///
/// Absent means a bot that has never been asked about it, and the engine's own
/// default is the right answer to that — so every field is optional and nothing
/// is coerced.
#[derive(Debug, Default, Deserialize)]
struct Asked {
    size: Option<String>,
    fps: Option<f64>,
    #[serde(default)]
    mute: bool,
    #[serde(default)]
    background: bool,
    #[serde(default)]
    storyboard: bool,
    #[serde(default)]
    bare: bool,
    /// Either a name the engine knows or `{{a0}}`, which is an archive to fetch.
    skin: Option<String>,
}

/// Ask for a job and do it.
pub fn once(
    bot: &Bot,
    engine: &str,
    capacity: &Capacity,
    songs: &Path,
    along: &Arc<Mutex<Along>>,
) -> Result<Did, Refused> {
    let Some(job) = bot.claim(engine, capacity)? else {
        return Ok(Did::Nothing);
    };
    let workdir = std::env::temp_dir().join(format!("dossier-job-{}", job.id));
    let done = do_job(bot, &job, songs, &workdir, along);
    // Whatever happened: the replay, the skin and the video are megabytes and
    // this machine belongs to somebody else.
    let _ = std::fs::remove_dir_all(&workdir);
    match done {
        Ok(title) => Ok(Did::Delivered { title }),
        Err(why) => {
            bot.give_back(&job.id, &why);
            Ok(Did::GaveBack { why })
        }
    }
}

fn do_job(
    bot: &Bot,
    job: &Job,
    songs: &Path,
    workdir: &Path,
    along: &Arc<Mutex<Along>>,
) -> Result<String, String> {
    std::fs::create_dir_all(workdir).map_err(|e| e.to_string())?;
    let replay = workdir.join("replay.osr");
    std::fs::write(&replay, bot.replay(&job.id).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    // The settings name their files as `{{a0}}`; each is fetched by that name
    // and the name is swapped for where it landed. Names are the server's own
    // and checked all the same, since they end up in a filename.
    let mut here = Vec::new();
    for name in &job.assets {
        if !name.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(format!("задача предложила странное имя файла: {name}"));
        }
        let bytes = bot.asset(&job.id, name).map_err(|e| e.to_string())?;
        here.push((name.clone(), bytes));
    }

    let asked: Asked = serde_json::from_value(job.settings.clone()).unwrap_or_default();
    let skin = match asked.skin.as_deref() {
        // A skin arrives as one archive rather than as its files.
        Some(named) if named.starts_with("{{") => {
            let name = named.trim_matches(['{', '}']);
            let archive = here
                .iter()
                .find(|(had, _)| had == name)
                .map(|(_, bytes)| bytes)
                .ok_or_else(|| format!("задача назвала скин {named}, которого не прислала"))?;
            let into = workdir.join("skin");
            std::fs::create_dir_all(&into).map_err(|e| e.to_string())?;
            dossier_produce::skin::unpack(archive, &into)?;
            Some(into)
        }
        // A name the engine knows, or nothing.
        _ => None,
    };

    let out = workdir.join("render.mp4");
    let size = asked
        .size
        .as_deref()
        .and_then(|text| text.split_once('x'))
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
        .unwrap_or((1280, 720));

    // Two threads say we are here: the render's own events, which arrive when
    // frames do, and a slow tick for the stretches when they do not — muxing,
    // or a map being fetched.
    let ours = Arc::new(AtomicBool::new(true));
    let seen = Arc::clone(along);
    dossier_produce::events::listen(move |line| {
        if let Ok(said) = serde_json::from_str::<serde_json::Value>(line) {
            if said["event"] == "progress" {
                *seen.lock().expect("how far along") = Along {
                    frames: said["frames"].as_u64().unwrap_or(0),
                    of: said["of"].as_u64().unwrap_or(0),
                    per_second: said["per_second"].as_f64().unwrap_or(0.0),
                    left_seconds: said["left_seconds"].as_f64().unwrap_or(0.0),
                };
            }
        }
    });

    let drawn = std::thread::scope(|threads| {
        let beating = {
            let ours = Arc::clone(&ours);
            let along = Arc::clone(along);
            threads.spawn(move || {
                while ours.load(Ordering::Relaxed) {
                    std::thread::sleep(HEARTBEAT);
                    if !ours.load(Ordering::Relaxed) {
                        break;
                    }
                    let now = *along.lock().expect("how far along");
                    let told = serde_json::json!({
                        "done": now.frames, "total": now.of,
                        "fps": now.per_second, "seconds_left": now.left_seconds,
                    });
                    if !bot.heartbeat(&job.id, Some(told)) {
                        // It stopped being ours. Anything still drawing for it
                        // is work nobody will collect.
                        ours.store(false, Ordering::Relaxed);
                    }
                }
            })
        };
        let drawn = crate::draw::draw(
            &crate::draw::Asked {
                replay: &replay,
                songs: Some(songs),
                map: None,
                out: out.clone(),
                size,
                fps: asked.fps.unwrap_or(60.0),
                from_ms: None,
                to_ms: None,
                background: asked.background,
                storyboard: asked.storyboard,
                bare: asked.bare,
                mute: asked.mute,
                skin: skin.clone(),
                events: true,
            },
            &crate::draw::Told::default(),
        );
        ours.store(false, Ordering::Relaxed);
        let _ = beating.join();
        drawn
    });
    dossier_produce::events::unlisten();
    let written = drawn?;

    let meta = serde_json::json!({
        "title": job.title,
        "bytes": std::fs::metadata(&written).map(|m| m.len()).unwrap_or(0),
    });
    bot.deliver(&job.id, &written, &meta)
        .map_err(|e| e.to_string())?;
    Ok(job.title.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_size_that_makes_no_sense_falls_back_rather_than_failing() {
        for (text, want) in [
            (Some("1920x1080"), (1920u32, 1080u32)),
            (Some("nonsense"), (1280, 720)),
            (Some("1920x"), (1280, 720)),
            (None, (1280, 720)),
        ] {
            let asked = Asked {
                size: text.map(str::to_owned),
                ..Asked::default()
            };
            let got = asked
                .size
                .as_deref()
                .and_then(|t| t.split_once('x'))
                .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
                .unwrap_or((1280, 720));
            assert_eq!(got, want, "{text:?}");
        }
    }

    /// Everything is optional: a job from a bot that has never been asked about
    /// a setting should get the engine's own answer, not a zero.
    #[test]
    fn a_job_that_says_almost_nothing_still_parses() {
        let asked: Asked = serde_json::from_value(serde_json::json!({"fps": 30})).expect("read");
        assert_eq!(asked.fps, Some(30.0));
        assert_eq!(asked.size, None);
        assert!(!asked.mute && !asked.bare);
    }

    #[test]
    fn a_skin_named_as_an_asset_is_told_apart_from_one_named_outright() {
        let templated: Asked =
            serde_json::from_value(serde_json::json!({"skin": "{{a0}}"})).expect("read");
        assert!(templated
            .skin
            .as_deref()
            .is_some_and(|s| s.starts_with("{{")));
        let named: Asked =
            serde_json::from_value(serde_json::json!({"skin": "classic"})).expect("read");
        assert!(!named.skin.as_deref().is_some_and(|s| s.starts_with("{{")));
    }
}
