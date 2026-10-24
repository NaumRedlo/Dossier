use std::io::Read;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

const LEVEL: f32 = 0.45;
const FADE_SECONDS: f32 = 0.25;
const TAIL_SECONDS: f32 = 2.5;
const STOP_SECONDS: f32 = 0.3;
const PREVIEW_MOST: usize = 4 * 1024 * 1024;

pub fn address(beatmap_id: i64) -> String {
    format!("https://catboy.best/preview/audio/{beatmap_id}")
}

fn cached(beatmap_id: i64) -> PathBuf {
    crate::sources::own_root().join("cache").join("previews").join(format!("{beatmap_id}.audio"))
}

fn fetched(beatmap_id: i64) -> Option<PathBuf> {
    let path = cached(beatmap_id);
    if std::fs::metadata(&path).is_ok_and(|meta| meta.len() > 0) {
        return Some(path);
    }
    let client = crate::bot::client().ok()?;
    let response = client.get(address(beatmap_id)).header("User-Agent", "Dossier").send().ok()?;
    if !response.status().is_success() {
        return None;
    }
    let mut body = Vec::new();
    response.take(PREVIEW_MOST as u64 + 1).read_to_end(&mut body).ok()?;
    if body.is_empty() || body.len() > PREVIEW_MOST {
        return None;
    }
    std::fs::create_dir_all(path.parent()?).ok()?;
    let part = path.with_extension("part");
    std::fs::write(&part, &body).ok()?;
    std::fs::rename(&part, &path).ok()?;
    Some(path)
}

fn decoded(path: &PathBuf, channels: usize, rate: u32) -> Option<Vec<f32>> {
    let ffmpeg = crate::checks::ffmpeg_on_path()?;
    let mut child = crate::checks::quiet(&ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-i"])
        .arg(path)
        .args(["-vn", "-f", "f32le", "-acodec", "pcm_f32le", "-ac", &channels.to_string(), "-ar", &rate.to_string(), "-"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .ok()?;
    let mut bytes = Vec::new();
    child.stdout.take()?.read_to_end(&mut bytes).ok()?;
    let _ = child.wait();
    Some(bytes.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect())
}

const ARCHIVE_MOST: u64 = 200 * 1024 * 1024;

pub fn archive_addresses(set: i64) -> [String; 3] {
    [format!("https://osu.direct/api/d/{set}?noVideo=1"), format!("https://catboy.best/d/{set}n"), format!("https://mirror.nekoha.moe/api4/download/{set}")]
}

fn archive(set: i64) -> Option<PathBuf> {
    let folder = crate::sources::own_root().join("cache").join("duel-maps");
    let path = folder.join(format!("{set}.osz"));
    if std::fs::metadata(&path).is_ok_and(|meta| meta.len() > 0) {
        return Some(path);
    }
    std::fs::create_dir_all(&folder).ok()?;
    let client = crate::bot::client().ok()?;
    for address in archive_addresses(set) {
        let Ok(response) = client.get(&address).header("User-Agent", "Dossier").send() else { continue };
        if !response.status().is_success() {
            continue;
        }
        let mut body = Vec::new();
        if response.take(ARCHIVE_MOST).read_to_end(&mut body).is_err() || body.len() < 4 || &body[..2] != b"PK" {
            continue;
        }
        let part = path.with_extension("part");
        if std::fs::write(&part, &body).is_ok() && std::fs::rename(&part, &path).is_ok() {
            return Some(path);
        }
    }
    None
}

pub fn fetch_sets(sets: &[i64]) -> Result<usize, String> {
    let mut opened = 0;
    let mut missing = Vec::new();
    for set in sets {
        match archive(*set) {
            Some(path) if open::that(&path).is_ok() => opened += 1,
            _ => missing.push(set.to_string()),
        }
    }
    if missing.is_empty() { Ok(opened) } else { Err(missing.join(", ")) }
}

pub struct Preview {
    stop: Arc<AtomicBool>,
    ended: Arc<AtomicBool>,
    pub beatmap_id: i64,
}

impl Preview {
    pub fn start(beatmap_id: i64) -> Preview {
        let stop = Arc::new(AtomicBool::new(false));
        let ended = Arc::new(AtomicBool::new(false));
        let quit = stop.clone();
        let over = ended.clone();
        thread::spawn(move || {
            let played = Self::play(beatmap_id, &quit);
            over.store(true, Ordering::Relaxed);
            played
        });
        Preview { stop, ended, beatmap_id }
    }

    pub fn ended(&self) -> bool {
        self.ended.load(Ordering::Relaxed)
    }

    fn play(beatmap_id: i64, quit: &Arc<AtomicBool>) -> Option<()> {
        {
            let path = fetched(beatmap_id)?;
            if quit.load(Ordering::Relaxed) {
                return None;
            }
            let host = cpal::default_host();
            let device = host.default_output_device()?;
            let config = device.default_output_config().ok()?;
            let rate = config.sample_rate().0;
            let channels = config.channels() as usize;
            let samples = decoded(&path, channels, rate)?;
            if quit.load(Ordering::Relaxed) || samples.is_empty() {
                return None;
            }
            let total = samples.len();
            let per_second = rate as usize * channels.max(1);
            let fade = (FADE_SECONDS * per_second as f32) as usize;
            let tail = (TAIL_SECONDS * per_second as f32) as usize;
            let stopping = (STOP_SECONDS * per_second as f32) as usize;
            let at = Arc::new(Mutex::new(0usize));
            let played = at.clone();
            let ear = quit.clone();
            let mut stopped_at: Option<usize> = None;
            let stream = device
                .build_output_stream(
                    &config.into(),
                    move |data: &mut [f32], _| {
                        let mut at = played.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                        if ear.load(Ordering::Relaxed) && stopped_at.is_none() {
                            stopped_at = Some(*at);
                        }
                        for sample in data.iter_mut() {
                            let leaving = stopped_at.map_or(1.0, |from| 1.0 - (*at - from) as f32 / stopping.max(1) as f32);
                            if *at >= total || leaving <= 0.0 {
                                *sample = 0.0;
                                *at = total;
                                continue;
                            }
                            let rise = (*at as f32 / fade.max(1) as f32).min(1.0);
                            let fall = ((total - *at) as f32 / tail.max(1) as f32).min(1.0);
                            *sample = samples[*at] * LEVEL * rise.min(fall * fall).min(leaving);
                            *at += 1;
                        }
                    },
                    |_| {},
                    None,
                )
                .ok()?;
            stream.play().ok()?;
            while *at.lock().unwrap_or_else(std::sync::PoisonError::into_inner) < total {
                thread::sleep(Duration::from_millis(30));
            }
            thread::sleep(Duration::from_millis(60));
            Some(())
        }
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl std::fmt::Debug for Preview {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Preview({})", self.beatmap_id)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn previews_come_from_the_mirror_and_never_from_osu() {
        let at = super::address(5379506);
        assert!(at.starts_with("https://catboy.best/") && !at.contains("ppy.sh"));
        assert!(super::archive_addresses(1).iter().all(|address| !address.contains("ppy.sh") && (address.contains("noVideo") || address.ends_with("1n") || address.contains("nekoha"))));
    }
}
