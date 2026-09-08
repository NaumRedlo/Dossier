use std::collections::HashMap;

use crate::kit::Kit;
use crate::samples::{Found, SamplePack, SampleSet};
use crate::synth::Voice;
use crate::SAMPLE_RATE;

pub struct Track {
    left: Vec<f32>,
    right: Vec<f32>,

    sounding: HashMap<(SampleSet, Voice, u32), Vec<Sounding>>,
    voices: HashMap<(SampleSet, Voice, u32), Vec<f32>>,
    kit: Kit,

    pack: SamplePack,

    silenced: HashMap<(SampleSet, Voice), usize>,

    resolved: HashMap<(SampleSet, Voice, u32), (Found, usize)>,
}

const CONCURRENCY: usize = 6;

#[derive(Debug, Clone, Copy)]
struct Sounding {
    began: usize,
    gain: f32,
    balance: f32,
}

fn pan(balance: f32) -> (f32, f32) {
    let balance = balance.clamp(-1.0, 1.0);
    ((1.0 - balance).min(1.0), (1.0 + balance).min(1.0))
}

impl Track {
    pub fn new(seconds: f64, kit: Kit) -> Self {
        Self {
            left: vec![0.0; (seconds.max(0.0) * f64::from(SAMPLE_RATE)) as usize],
            right: vec![0.0; (seconds.max(0.0) * f64::from(SAMPLE_RATE)) as usize],
            sounding: HashMap::new(),
            voices: HashMap::new(),
            kit,
            pack: SamplePack::default(),
            silenced: HashMap::new(),
            resolved: HashMap::new(),
        }
    }

    pub fn resolved(&self) -> impl Iterator<Item = ((SampleSet, Voice, u32), Found, usize)> + '_ {
        self.resolved
            .iter()
            .map(|(&asked, &(found, count))| (asked, found, count))
    }

    pub fn silenced(&self) -> impl Iterator<Item = ((SampleSet, Voice), usize)> + '_ {
        self.silenced.iter().map(|(&key, &count)| (key, count))
    }

    pub fn with_samples(mut self, pack: SamplePack) -> Self {
        self.pack = pack;
        self
    }

    pub fn is_empty(&self) -> bool {
        self.left.is_empty()
    }

    pub fn seconds(&self) -> f64 {
        self.left.len() as f64 / f64::from(SAMPLE_RATE)
    }

    pub fn strike(&mut self, voice: Voice, at_seconds: f64) {
        self.strike_with(voice, at_seconds, SampleSet::Normal, 1.0);
    }

    pub fn strike_with(&mut self, voice: Voice, at_seconds: f64, set: SampleSet, volume: f32) {
        self.strike_indexed(voice, at_seconds, set, 1, volume);
    }

    pub fn strike_indexed(
        &mut self,
        voice: Voice,
        at_seconds: f64,
        set: SampleSet,
        index: u32,
        volume: f32,
    ) {
        self.strike_panned(voice, at_seconds, set, index, volume, 0.0);
    }

    pub fn strike_panned(
        &mut self,
        voice: Voice,
        at_seconds: f64,
        set: SampleSet,
        index: u32,
        volume: f32,
        balance: f32,
    ) {
        if at_seconds < 0.0 {
            return;
        }
        let start = (at_seconds * f64::from(SAMPLE_RATE)) as usize;
        if start >= self.left.len() {
            return;
        }

        if self
            .pack
            .get(set, voice, index)
            .is_some_and(<[f32]>::is_empty)
        {
            *self.silenced.entry((set, voice)).or_insert(0) += 1;
        }
        let found = self.pack.trace(set, voice, index);
        self.resolved
            .entry((set, voice, index))
            .or_insert((found, 0))
            .1 += 1;
        let gain = volume.clamp(0.0, 1.0);
        let kit = self.kit;
        let pack = &self.pack;
        let rendered = self.voices.entry((set, voice, index)).or_insert_with(|| {
            match pack.get(set, voice, index) {
                Some(sound) => sound.to_vec(),
                None => {
                    let level = voice.gain(&kit);
                    voice.render(&kit).into_iter().map(|s| s * level).collect()
                }
            }
        });
        let rendered = rendered.clone();

        let live = self.sounding.entry((set, voice, index)).or_default();
        live.retain(|old| start < old.began + rendered.len());
        let cut = (live.len() >= CONCURRENCY).then(|| live.remove(0));
        live.push(Sounding {
            began: start,
            gain,
            balance,
        });
        if let Some(old) = cut {
            let (l, r) = pan(old.balance);
            for (offset, value) in rendered.iter().enumerate().skip(start - old.began) {
                let at = old.began + offset;
                if let Some(slot) = self.left.get_mut(at) {
                    *slot -= value * old.gain * l;
                }
                if let Some(slot) = self.right.get_mut(at) {
                    *slot -= value * old.gain * r;
                }
            }
        }

        let (l, r) = pan(balance);
        for (offset, value) in rendered.iter().enumerate() {
            if let Some(slot) = self.left.get_mut(start + offset) {
                *slot += value * gain * l;
            }
            if let Some(slot) = self.right.get_mut(start + offset) {
                *slot += value * gain * r;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_arguments)]
    pub fn sustain(
        &mut self,
        voice: Voice,
        span: (f64, f64),
        set: SampleSet,
        index: u32,
        volume: f32,
        rate: impl Fn(f64) -> f32,
    ) {
        self.sustain_with(voice, span, set, index, |_| volume, rate);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn sustain_with(
        &mut self,
        voice: Voice,
        (from_seconds, to_seconds): (f64, f64),
        set: SampleSet,
        index: u32,
        level: impl Fn(f64) -> f32,
        rate: impl Fn(f64) -> f32,
    ) {
        let Some(source) = self.pack.get(set, voice, index) else {
            return;
        };
        if source.len() < 2 || to_seconds <= from_seconds {
            return;
        }
        let rate_hz = f64::from(SAMPLE_RATE);
        let start = (from_seconds.max(0.0) * rate_hz) as usize;
        let end = ((to_seconds * rate_hz) as usize).min(self.left.len());
        if start >= end {
            return;
        }

        let ramp = (((end - start) / 8).min((0.015 * rate_hz) as usize)).max(1);
        let mut read = 0.0f64;
        for (step, slot) in (start..end).enumerate() {
            let held = step as f64 / rate_hz;
            let fade = (step as f32 / ramp as f32)
                .min((end - start - step) as f32 / ramp as f32)
                .clamp(0.0, 1.0);

            let whole = read as usize % source.len();
            let next = (whole + 1) % source.len();
            let fraction = (read - read.floor()) as f32;
            let value = source[whole] + (source[next] - source[whole]) * fraction;

            let gain = level(held).clamp(0.0, 1.0);
            self.left[slot] += value * gain * fade;
            self.right[slot] += value * gain * fade;

            read += f64::from(rate(held).max(0.01));
            if read >= source.len() as f64 {
                read -= source.len() as f64;
            }
        }
    }

    pub fn to_pcm(&self) -> Vec<u8> {
        let peak = self
            .left
            .iter()
            .chain(&self.right)
            .fold(0.0f32, |worst, s| worst.max(s.abs()));
        let scale = if peak > 0.95 { 0.95 / peak } else { 1.0 };

        let mut out = Vec::with_capacity(self.left.len() * 4);
        for (l, r) in self.left.iter().zip(&self.right) {
            for channel in [l, r] {
                let value = (channel * scale * f32::from(i16::MAX)) as i16;
                out.extend_from_slice(&value.to_le_bytes());
            }
        }
        out
    }

    pub fn to_wav(&self) -> Vec<u8> {
        const CHANNELS: u16 = 2;
        const BITS: u16 = 16;
        let pcm = self.to_pcm();
        let byte_rate = SAMPLE_RATE * u32::from(CHANNELS) * u32::from(BITS / 8);

        let mut out = Vec::with_capacity(pcm.len() + 44);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + pcm.len() as u32).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&CHANNELS.to_le_bytes());
        out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
        out.extend_from_slice(&byte_rate.to_le_bytes());
        out.extend_from_slice(&(CHANNELS * BITS / 8).to_le_bytes());
        out.extend_from_slice(&BITS.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
        out.extend_from_slice(&pcm);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skin_keeps_the_balance_it_was_mixed_with() {
        let dir = std::env::temp_dir().join(format!("dossier-balance-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a folder");
        let wav = |peak: i16| {
            let mut out = Vec::new();
            let data: Vec<u8> = (0..8u32)
                .flat_map(|n| {
                    let v = if n % 2 == 0 { peak } else { -peak };
                    v.to_le_bytes()
                })
                .collect();
            out.extend_from_slice(b"RIFF");
            out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
            out.extend_from_slice(b"WAVEfmt ");
            out.extend_from_slice(&16u32.to_le_bytes());
            out.extend_from_slice(&1u16.to_le_bytes());
            out.extend_from_slice(&1u16.to_le_bytes());
            out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
            out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
            out.extend_from_slice(&2u16.to_le_bytes());
            out.extend_from_slice(&16u16.to_le_bytes());
            out.extend_from_slice(b"data");
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&data);
            out
        };
        std::fs::write(dir.join("soft-hitnormal.wav"), wav(16_000)).expect("a file");
        std::fs::write(dir.join("soft-hitclap.wav"), wav(4_000)).expect("a file");

        let mut track = Track::new(1.0, Kit::default()).with_samples(SamplePack::load(&dir));
        track.strike_with(Voice::Normal, 0.2, SampleSet::Soft, 1.0);
        track.strike_with(Voice::Clap, 0.6, SampleSet::Soft, 1.0);

        let pcm = track.to_pcm();
        let loudest = |seconds: f64| {
            let at = (seconds * f64::from(SAMPLE_RATE)) as usize * 4;
            pcm[at..at + 400]
                .chunks_exact(2)
                .map(|s| i16::from_le_bytes([s[0], s[1]]).abs())
                .max()
                .unwrap_or(0)
        };

        let (hit, clap) = (f64::from(loudest(0.2)), f64::from(loudest(0.6)));
        assert!(clap > 0.0, "the clap made no sound");
        let ratio = hit / clap;
        assert!(
            (ratio - 4.0).abs() < 0.15,
            "the skin's own balance was rewritten: {ratio:.2} against 4.00"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_blanked_voice_is_counted_so_somebody_can_be_told() {
        let dir = std::env::temp_dir().join(format!("dossier-silence-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a folder");
        std::fs::write(dir.join("soft-hitwhistle.wav"), []).expect("a blank");

        let mut track = Track::new(2.0, Kit::default()).with_samples(SamplePack::load(&dir));
        for at in [0.2, 0.4, 0.6] {
            track.strike_with(Voice::Whistle, at, SampleSet::Soft, 1.0);
        }

        track.strike_with(Voice::Clap, 0.8, SampleSet::Soft, 1.0);

        let counted: Vec<_> = track.silenced().collect();
        assert_eq!(counted, vec![((SampleSet::Soft, Voice::Whistle), 3)]);
        assert_eq!(Voice::Whistle.file_name(), "hitwhistle");
        assert!(Voice::Whistle.banked() && !Voice::Miss.banked());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_empty_track_is_silence_of_the_right_length() {
        let track = Track::new(2.0, Kit::default());
        assert!((track.seconds() - 2.0).abs() < 1e-9);

        assert_eq!(track.to_pcm().len(), 2 * 44_100 * 2 * 2);
        assert!(track.to_pcm().iter().all(|&b| b == 0));
    }

    #[test]
    fn a_hit_lands_where_it_was_asked_to() {
        let mut track = Track::new(1.0, Kit::default());
        track.strike(Voice::Normal, 0.5);
        let pcm = track.to_pcm();

        let loudness_around = |seconds: f64| {
            let frame = (seconds * 44_100.0) as usize * 4;
            pcm[frame..frame + 400]
                .chunks_exact(2)
                .map(|b| i16::from_le_bytes([b[0], b[1]]).unsigned_abs() as u32)
                .sum::<u32>()
        };
        assert_eq!(loudness_around(0.2), 0, "silent before the hit");
        assert!(loudness_around(0.5) > 0, "and not at it");
    }

    #[test]
    fn hits_outside_the_track_are_dropped_not_stretched_onto_it() {
        let mut track = Track::new(1.0, Kit::default());
        track.strike(Voice::Normal, 5.0);
        track.strike(Voice::Normal, -1.0);
        assert!(track.to_pcm().iter().all(|&b| b == 0));
        assert!((track.seconds() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn a_pile_of_hits_at_once_is_turned_down_rather_than_clipped() {
        let mut track = Track::new(1.0, Kit::default());
        for i in 0..12 {
            track.strike(Voice::Finish, 0.3 + f64::from(i) * 0.001);
        }
        let loudest = track
            .to_pcm()
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]).unsigned_abs())
            .max()
            .unwrap();
        assert!(loudest <= (0.96 * f32::from(i16::MAX)) as u16, "{loudest}");
    }

    #[test]
    fn every_voice_makes_a_sound_and_stays_in_range() {
        for voice in [
            Voice::Normal,
            Voice::Whistle,
            Voice::Finish,
            Voice::Clap,
            Voice::Tick,
        ] {
            let rendered = voice.render(&Kit::default());
            assert!(!rendered.is_empty(), "{voice:?} is silent");
            assert!(
                rendered.iter().all(|s| s.abs() <= 1.5),
                "{voice:?} runs far out of range"
            );
            assert!(
                rendered.iter().any(|s| s.abs() > 0.05),
                "{voice:?} is inaudible"
            );
        }
    }

    #[test]
    fn a_voice_starts_and_ends_quietly() {
        let rendered = Voice::Normal.render(&Kit::default());
        assert!(rendered[0].abs() < 0.05, "starts with a step");
        assert!(rendered[rendered.len() - 1].abs() < 0.05, "ends with one");
    }
}

#[cfg(test)]
mod levels {
    use super::*;

    #[test]
    fn every_kit_is_loud_enough_to_hear_over_music() {
        for (name, kit) in [("plain", Kit::plain())] {
            for voice in [Voice::Normal, Voice::Whistle, Voice::Finish, Voice::Clap] {
                let mut track = Track::new(0.5, kit);
                track.strike(voice, 0.1);
                let peak = track
                    .to_pcm()
                    .chunks_exact(2)
                    .map(|b| i16::from_le_bytes([b[0], b[1]]).unsigned_abs())
                    .max()
                    .unwrap_or(0);
                assert!(
                    peak > 20_000,
                    "{name}/{voice:?} peaks at {peak}, which the music will bury"
                );
            }
        }
    }

    fn folder_with(name: &str, hz: f32, seconds: f32) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("dossier-loop-{name}-{}-{hz}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a folder");

        let frames = (seconds * SAMPLE_RATE as f32) as usize;
        let mut data = Vec::with_capacity(frames * 2);
        for n in 0..frames {
            let phase = n as f32 / SAMPLE_RATE as f32 * hz * std::f32::consts::TAU;
            data.extend_from_slice(&((phase.sin() * 12_000.0) as i16).to_le_bytes());
        }
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
        out.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&data);
        std::fs::write(dir.join(format!("{name}.wav")), out).expect("written");
        dir
    }

    fn loudness(track: &Track, from: f64, to: f64) -> f32 {
        let rate = f64::from(SAMPLE_RATE);
        let (a, b) = ((from * rate) as usize, (to * rate) as usize);
        track.left[a..b.min(track.left.len())]
            .iter()
            .fold(0.0f32, |worst, s| worst.max(s.abs()))
    }

    #[test]
    fn a_held_sound_runs_for_its_whole_span_and_not_past_it() {
        let dir = folder_with("normal-sliderslide", 440.0, 0.05);
        let mut track = Track::new(1.0, Kit::default()).with_samples(SamplePack::load(&dir));
        track.sustain(Voice::Slide, (0.2, 0.8), SampleSet::Normal, 1, 1.0, |_| 1.0);

        assert!(
            loudness(&track, 0.0, 0.19) < 1e-6,
            "silent before it starts"
        );
        assert!(loudness(&track, 0.3, 0.4) > 0.01, "sounding at the start");

        assert!(
            loudness(&track, 0.7, 0.78) > 0.01,
            "still sounding at the end"
        );
        assert!(loudness(&track, 0.81, 1.0) < 1e-6, "silent after it stops");
    }

    #[test]
    fn a_held_sound_starts_and_ends_at_nothing() {
        let dir = folder_with("normal-sliderslide", 200.0, 0.05);
        let mut track = Track::new(1.0, Kit::default()).with_samples(SamplePack::load(&dir));
        track.sustain(Voice::Slide, (0.1, 0.9), SampleSet::Normal, 1, 1.0, |_| 1.0);

        let middle = loudness(&track, 0.4, 0.6);
        assert!(loudness(&track, 0.1, 0.105) < middle * 0.6, "it fades in");
        assert!(loudness(&track, 0.895, 0.9) < middle * 0.6, "and out");
    }

    #[test]
    fn a_spinner_climbs_in_pitch_as_it_is_turned() {
        let dir = folder_with("spinnerspin", 300.0, 0.2);
        let mut track = Track::new(2.0, Kit::default()).with_samples(SamplePack::load(&dir));

        track.sustain(Voice::Spin, (0.1, 1.9), SampleSet::Normal, 1, 1.0, |held| {
            0.5 + 1.5 * (held / 1.8) as f32
        });

        let crossings = |from: f64, to: f64| {
            let rate = f64::from(SAMPLE_RATE);
            let (a, b) = ((from * rate) as usize, (to * rate) as usize);
            track.left[a..b]
                .windows(2)
                .filter(|pair| (pair[0] < 0.0) != (pair[1] < 0.0))
                .count()
        };
        let early = crossings(0.3, 0.5);
        let late = crossings(1.5, 1.7);
        assert!(early > 0, "it is sounding at all: {early}");
        assert!(
            late > early * 2,
            "the pitch did not climb: {early} then {late}"
        );
    }

    #[test]
    fn a_skin_that_brought_no_loop_gets_silence() {
        let mut track = Track::new(1.0, Kit::default());
        track.sustain(Voice::Slide, (0.2, 0.8), SampleSet::Normal, 1, 1.0, |_| 1.0);
        assert!(track.to_pcm().iter().all(|&b| b == 0));
    }
}
