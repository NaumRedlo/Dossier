use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

const OPPONENT_FOUND: &[u8] = include_bytes!("../assets/sounds/opponent-found.wav");

pub fn opponent_found(level: f32) {
    if level <= 0.0 {
        return;
    }
    thread::spawn(move || play(OPPONENT_FOUND, level));
}

pub(crate) fn spread(mono: &[f32], from_rate: u32, to_rate: u32, channels: usize) -> Vec<f32> {
    if mono.is_empty() || from_rate == 0 || to_rate == 0 || channels == 0 {
        return Vec::new();
    }
    let ratio = f64::from(from_rate) / f64::from(to_rate);
    let frames = (mono.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(frames * channels);
    for frame in 0..frames {
        let at = frame as f64 * ratio;
        let left = at.floor() as usize;
        let right = (left + 1).min(mono.len() - 1);
        let part = (at - left as f64) as f32;
        let sample = mono[left] * (1.0 - part) + mono[right] * part;
        out.extend(std::iter::repeat_n(sample, channels));
    }
    out
}

fn play(bytes: &'static [u8], level: f32) -> Option<()> {
    let mono = dossier_audio::decode_wav(bytes)?;
    let device = cpal::default_host().default_output_device()?;
    let config = device.default_output_config().ok()?;
    let samples = spread(&mono, dossier_audio::SAMPLE_RATE, config.sample_rate().0, config.channels() as usize);
    let total = samples.len();
    if total == 0 {
        return None;
    }
    let gain = level.clamp(0.0, 1.0) * crate::preview::steadied(&samples);
    let at = Arc::new(AtomicUsize::new(0));
    let played = at.clone();
    let stream = device
        .build_output_stream(
            &config.into(),
            move |data: &mut [f32], _| {
                let mut next = played.load(Ordering::Relaxed);
                for sample in data.iter_mut() {
                    *sample = samples.get(next).map_or(0.0, |value| value * gain);
                    next = (next + 1).min(total);
                }
                played.store(next, Ordering::Relaxed);
            },
            |_| {},
            None,
        )
        .ok()?;
    stream.play().ok()?;
    while at.load(Ordering::Relaxed) < total {
        thread::sleep(Duration::from_millis(30));
    }
    thread::sleep(Duration::from_millis(60));
    Some(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_sound_for_a_found_opponent_is_a_short_wav_the_app_can_read() {
        let mono = dossier_audio::decode_wav(super::OPPONENT_FOUND).expect("a readable wav");
        let seconds = mono.len() as f32 / dossier_audio::SAMPLE_RATE as f32;
        assert!((0.5..3.0).contains(&seconds), "{seconds}");
        assert!(mono.iter().any(|sample| sample.abs() > 0.05));
    }

    #[test]
    fn spreading_keeps_the_length_in_time_and_fills_every_channel() {
        let mono = vec![0.0, 1.0, 0.0, -1.0];
        let out = super::spread(&mono, 4, 8, 2);
        assert_eq!(out.len(), 8 * 2);
        assert_eq!(out[0], out[1]);
        assert_eq!(out[2], 0.5);
        assert!(super::spread(&[], 44_100, 48_000, 2).is_empty());
    }
}
