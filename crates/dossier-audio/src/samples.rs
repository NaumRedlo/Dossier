use std::collections::HashMap;
use std::path::Path;

use crate::synth::Voice;
use crate::SAMPLE_RATE;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SampleSet {
    Normal,
    Soft,
    Drum,
}

impl SampleSet {
    pub const ALL: [Self; 3] = [Self::Normal, Self::Soft, Self::Drum];

    pub fn name(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Soft => "soft",
            Self::Drum => "drum",
        }
    }
}

type Banks = HashMap<(SampleSet, Voice, u32), Vec<f32>>;

#[derive(Debug, Clone, Default)]
pub struct SamplePack {
    skin: Banks,

    unused: Vec<String>,

    beatmap: Banks,

    game: Banks,

    banked_spinner: HashMap<(SampleSet, Voice), Vec<f32>>,

    lazer: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Found {
    Beatmap(u32),

    SkinPlain,

    Game,

    SkinNormalBank,

    Blank,

    Synthesised,
}

impl Found {
    pub fn describe(self) -> String {
        match self {
            Self::Beatmap(at) => format!("the map's, index {at}"),
            Self::SkinPlain => "the skin's, unnumbered".to_owned(),
            Self::Game => "osu!'s own".to_owned(),
            Self::SkinNormalBank => "the skin's normal bank".to_owned(),
            Self::Blank => "blank — the skin removed it".to_owned(),
            Self::Synthesised => "nothing anywhere — synthesised".to_owned(),
        }
    }
}

const BANKED: [(Voice, &str); 7] = [
    (Voice::Normal, "hitnormal"),
    (Voice::Whistle, "hitwhistle"),
    (Voice::Finish, "hitfinish"),
    (Voice::Clap, "hitclap"),
    (Voice::Tick, "slidertick"),
    (Voice::Slide, "sliderslide"),
    (Voice::SlideWhistle, "sliderwhistle"),
];

const BANKLESS: [(Voice, &str); 5] = [
    (Voice::Bonus, "spinnerbonus"),
    (Voice::Spin, "spinnerspin"),
    (Voice::Miss, "combobreak"),
    (Voice::SectionPass, "sectionpass"),
    (Voice::SectionFail, "sectionfail"),
];

const SPINNER: [(Voice, &str); 2] = [(Voice::Bonus, "spinnerbonus"), (Voice::Spin, "spinnerspin")];

const SOUND_ENDINGS: [&str; 3] = ["wav", "ogg", "mp3"];

fn decode_through_ffmpeg(path: &Path) -> Option<Vec<f32>> {
    let done = crate::quiet("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args([
            "-f",
            "f32le",
            "-ac",
            "1",
            "-ar",
            &SAMPLE_RATE.to_string(),
            "-",
        ])
        .output()
        .ok()?;
    if !done.status.success() {
        return None;
    }
    Some(
        done.stdout
            .chunks_exact(4)
            .map(|four| f32::from_le_bytes([four[0], four[1], four[2], four[3]]))
            .collect(),
    )
}

impl SamplePack {
    fn read(path: &Path) -> Option<Vec<f32>> {
        let bytes = std::fs::read(path).ok()?;
        if bytes.is_empty() {
            return Some(Vec::new());
        }
        if let Some(samples) = decode_wav(&bytes) {
            return Some(samples);
        }
        decode_through_ffmpeg(path)
    }

    pub fn load(folder: &Path) -> Self {
        let files = index_of(folder);
        let mut skin = HashMap::new();
        let mut numbered: Vec<String> = Vec::new();
        for (stem, path) in &files {
            let Some(key) = parse_sample_name(stem) else {
                continue;
            };
            if key.2 != 1 {
                numbered.push(stem.clone());
                continue;
            }
            if let Some(samples) = Self::read(path) {
                skin.insert(key, samples);
            }
        }
        for (voice, name) in BANKLESS {
            let found = files.get(name).and_then(|path| Self::read(path));
            if let Some(samples) = found {
                skin.insert((SampleSet::Normal, voice, 1), samples);
            }
        }
        let mut banked_spinner = HashMap::new();
        for (voice, name) in SPINNER {
            for set in SampleSet::ALL {
                if let Some(samples) = files.get(&format!("{}-{name}", set.name())).and_then(|path| Self::read(path)) {
                    banked_spinner.insert((set, voice), samples);
                }
            }
        }

        let mut guessed = Vec::new();
        for name in unfiled_in(folder) {
            let Some((set, voice, index)) = guess_sample_name(&name) else {
                guessed.push(name);
                continue;
            };
            let key = (set, voice, index);
            if skin.contains_key(&key) {
                guessed.push(name);
                continue;
            }
            match files.get(&name).and_then(|path| Self::read(path)) {
                Some(samples) if !samples.is_empty() => {
                    skin.insert(key, samples);
                }
                _ => guessed.push(name),
            }
        }

        guessed.extend(numbered);
        guessed.sort();
        Self {
            unused: guessed,
            skin,
            beatmap: HashMap::new(),
            game: HashMap::new(),
            banked_spinner,
            lazer: false,
        }
    }

    #[must_use]
    pub fn looked_up_as_lazer(mut self) -> Self {
        self.lazer = true;
        self
    }

    fn lazer_first(&self, set: SampleSet, voice: Voice) -> Option<&Vec<f32>> {
        if !self.lazer {
            return None;
        }
        self.banked_spinner.get(&(set, voice))
    }

    pub fn unused(&self) -> &[String] {
        &self.unused
    }

    #[must_use]
    pub fn with_beatmap(mut self, folder: &Path) -> Self {
        for (name, samples) in banked_in(folder) {
            self.beatmap.insert(name, samples);
        }
        self
    }

    pub fn with_game_sounds(mut self, folder: &Path) -> Self {
        self.game = Self::load(folder).skin;
        self
    }

    pub fn from_game(&self) -> usize {
        self.game.len()
    }

    pub fn is_empty(&self) -> bool {
        self.skin.is_empty() && self.beatmap.is_empty() && self.banked_spinner.is_empty()
    }

    pub fn len(&self) -> usize {
        self.skin.len()
    }

    pub fn from_beatmap(&self) -> usize {
        self.beatmap.len()
    }

    fn ladder(
        &self,
        set: SampleSet,
        voice: Voice,
        index: u32,
    ) -> Vec<(&Banks, (SampleSet, Voice, u32), Found)> {
        vec![
            (&self.beatmap, (set, voice, index), Found::Beatmap(index)),
            (&self.skin, (set, voice, 1), Found::SkinPlain),
            (&self.game, (set, voice, 1), Found::Game),
            (
                &self.skin,
                (SampleSet::Normal, voice, 1),
                Found::SkinNormalBank,
            ),
        ]
    }

    pub fn trace(&self, set: SampleSet, voice: Voice, index: u32) -> Found {
        if let Some(sound) = self.lazer_first(set, voice) {
            return if sound.is_empty() { Found::Blank } else { Found::SkinPlain };
        }
        let index = index.max(1);
        for (store, at, step) in self.ladder(set, voice, index) {
            if let Some(sound) = store.get(&at) {
                return if sound.is_empty() { Found::Blank } else { step };
            }
        }
        Found::Synthesised
    }

    pub fn get(&self, set: SampleSet, voice: Voice, index: u32) -> Option<&[f32]> {
        if let Some(sound) = self.lazer_first(set, voice) {
            return Some(sound.as_slice());
        }
        let index = index.max(1);
        for (store, at, _) in self.ladder(set, voice, index) {
            if let Some(sound) = store.get(&at) {
                return Some(sound.as_slice());
            }
        }
        None
    }
}

fn index_of(folder: &Path) -> HashMap<String, std::path::PathBuf> {
    let mut best: HashMap<String, (usize, std::path::PathBuf)> = HashMap::new();
    let Ok(entries) = std::fs::read_dir(folder) else {
        return HashMap::new();
    };
    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|kind| kind.is_file()) {
            continue;
        }
        let leaf = entry.file_name().to_string_lossy().to_ascii_lowercase();
        let Some((stem, ending)) = leaf.rsplit_once('.') else {
            continue;
        };
        let Some(rank) = SOUND_ENDINGS.iter().position(|one| *one == ending) else {
            continue;
        };
        match best.get(stem) {
            Some((had, _)) if *had <= rank => {}
            _ => {
                best.insert(stem.to_owned(), (rank, entry.path()));
            }
        }
    }
    best.into_iter()
        .map(|(stem, (_, path))| (stem, path))
        .collect()
}

fn unfiled_in(folder: &Path) -> Vec<String> {
    let mut out: Vec<String> = index_of(folder)
        .into_keys()
        .filter(|stem| {
            let bankless = BANKLESS.iter().any(|(_, name)| name == stem);
            let banked_spinner = SPINNER.iter().any(|(_, name)| SampleSet::ALL.iter().any(|set| *stem == format!("{}-{name}", set.name())));
            parse_sample_name(stem).is_none() && !bankless && !banked_spinner
        })
        .collect();
    out.sort();
    out
}

fn banked_in(folder: &Path) -> Vec<((SampleSet, Voice, u32), Vec<f32>)> {
    index_of(folder)
        .into_iter()
        .filter_map(|(stem, path)| {
            let key = parse_sample_name(&stem)?;
            Some((key, SamplePack::read(&path)?))
        })
        .collect()
}

fn guess_sample_name(stem: &str) -> Option<(SampleSet, Voice, u32)> {
    let mut squeezed = String::with_capacity(stem.len());
    for ch in stem.chars() {
        if ch == '-' && squeezed.ends_with('-') {
            continue;
        }
        squeezed.push(ch);
    }
    let (bank, rest) = squeezed.split_once('-')?;
    let set = SampleSet::ALL.into_iter().find(|s| {
        bank == s.name() || bank.starts_with(s.name()) || one_edit_apart(bank, s.name())
    })?;

    let digits = rest.len() - rest.trim_end_matches(|c: char| c.is_ascii_digit()).len();
    let (word, tail) = rest.split_at(rest.len() - digits);
    let index = if tail.is_empty() {
        1
    } else {
        tail.parse().ok()?
    };
    let (voice, _) = BANKED
        .into_iter()
        .find(|(_, name)| word == *name || word.starts_with(name) || one_edit_apart(word, name))?;
    Some((set, voice, index))
}

fn one_edit_apart(a: &str, b: &str) -> bool {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a.len().abs_diff(b.len()) > 1 {
        return false;
    }

    let (long, short) = if a.len() >= b.len() {
        (&a, &b)
    } else {
        (&b, &a)
    };
    let mut skipped = false;
    let (mut i, mut j) = (0usize, 0usize);
    while i < long.len() && j < short.len() {
        if long[i] == short[j] {
            i += 1;
            j += 1;
            continue;
        }
        if skipped {
            return false;
        }
        skipped = true;
        i += 1;
        if long.len() == short.len() {
            j += 1;
        }
    }
    true
}

fn parse_sample_name(stem: &str) -> Option<(SampleSet, Voice, u32)> {
    let (bank, rest) = stem.split_once('-')?;
    let set = SampleSet::ALL.into_iter().find(|s| s.name() == bank)?;
    let (voice, name) = BANKED.into_iter().find(|(_, n)| rest.starts_with(n))?;
    let digits = &rest[name.len()..];

    let index = if digits.is_empty() {
        1
    } else {
        match digits.parse().ok()? {
            0 | 1 => return None,
            n => n,
        }
    };
    Some((set, voice, index))
}

pub fn decode_wav(bytes: &[u8]) -> Option<Vec<f32>> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }

    let mut channels = 0u16;
    let mut rate = 0u32;
    let mut bits = 0u16;
    let mut format = 0u16;
    let mut data: Option<&[u8]> = None;

    let mut cursor = 12usize;
    while cursor + 8 <= bytes.len() {
        let id = &bytes[cursor..cursor + 4];
        let size = u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().ok()?) as usize;
        let body = bytes.get(cursor + 8..cursor + 8 + size)?;

        match id {
            b"fmt " if size >= 16 => {
                format = u16::from_le_bytes(body[0..2].try_into().ok()?);
                channels = u16::from_le_bytes(body[2..4].try_into().ok()?);
                rate = u32::from_le_bytes(body[4..8].try_into().ok()?);
                bits = u16::from_le_bytes(body[14..16].try_into().ok()?);
            }
            b"data" => data = Some(body),
            _ => {}
        }

        cursor += 8 + size + (size & 1);
    }

    let data = data?;
    if format != 1 || channels == 0 {
        return None;
    }

    let mono: Vec<f32> = match bits {
        8 => data
            .chunks_exact(channels as usize)
            .map(|frame| {
                frame
                    .iter()
                    .map(|&s| (f32::from(s) - 128.0) / 128.0)
                    .sum::<f32>()
                    / f32::from(channels)
            })
            .collect(),
        16 => data
            .chunks_exact(2 * channels as usize)
            .map(|frame| {
                frame
                    .chunks_exact(2)
                    .map(|s| f32::from(i16::from_le_bytes([s[0], s[1]])) / 32_768.0)
                    .sum::<f32>()
                    / f32::from(channels)
            })
            .collect(),
        _ => return None,
    };

    Some(resample(mono, rate))
}

fn resample(samples: Vec<f32>, from_rate: u32) -> Vec<f32> {
    if from_rate == SAMPLE_RATE || from_rate == 0 || samples.is_empty() {
        return samples;
    }
    let ratio = f64::from(SAMPLE_RATE) / f64::from(from_rate);
    let out_len = (samples.len() as f64 * ratio) as usize;

    (0..out_len)
        .map(|i| {
            let source = i as f64 / ratio;
            let index = source as usize;
            let fraction = (source - index as f64) as f32;
            let a = samples.get(index).copied().unwrap_or(0.0);
            let b = samples.get(index + 1).copied().unwrap_or(a);
            a + (b - a) * fraction
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skin_that_ships_ogg_is_read_the_same_as_one_that_ships_wav() {
        let dir = std::env::temp_dir().join(format!("dossier-sounds-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a place to write");
        for leaf in [
            "normal-hitnormal.ogg",
            "soft-hitwhistle.mp3",
            "drum-hitclap.wav",
            "readme.txt",
        ] {
            std::fs::write(dir.join(leaf), b"").expect("written");
        }

        let found = index_of(&dir);
        let mut stems: Vec<&String> = found.keys().collect();
        stems.sort();
        assert_eq!(
            stems,
            ["drum-hitclap", "normal-hitnormal", "soft-hitwhistle"]
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_wav_wins_over_the_same_sound_in_another_container() {
        let dir = std::env::temp_dir().join(format!("dossier-pick-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a place to write");
        for leaf in ["normal-hitnormal.ogg", "normal-hitnormal.wav"] {
            std::fs::write(dir.join(leaf), b"").expect("written");
        }

        let found = index_of(&dir);
        assert_eq!(found.len(), 1);
        assert!(found["normal-hitnormal"]
            .to_string_lossy()
            .ends_with(".wav"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_lazer_replay_hears_the_banked_spinner_a_stable_one_does_not() {
        let dir = std::env::temp_dir().join(format!("dossier-spinner-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a place to write");
        let tone: Vec<i16> = (0..(SAMPLE_RATE / 8) as i16).map(|n| (n % 64) * 200).collect();
        std::fs::write(dir.join("spinnerspin.wav"), wav(2, 16, SAMPLE_RATE, &[])).expect("written");
        std::fs::write(dir.join("soft-spinnerspin.wav"), wav(2, 16, SAMPLE_RATE, &tone)).expect("written");
        std::fs::write(dir.join("normal-spinnerbonus.wav"), wav(2, 16, SAMPLE_RATE, &tone)).expect("written");

        let stable = SamplePack::load(&dir);
        assert_eq!(stable.get(SampleSet::Soft, Voice::Spin, 1).map(<[f32]>::len), Some(0), "stable reads only the bare name, and it is blank");
        assert_eq!(stable.trace(SampleSet::Soft, Voice::Spin, 1), Found::Blank);
        assert!(stable.unused().is_empty(), "a banked spinner sound is not an unknown file: {:?}", stable.unused());

        let lazer = SamplePack::load(&dir).looked_up_as_lazer();
        assert!(lazer.get(SampleSet::Soft, Voice::Spin, 1).is_some_and(|sound| !sound.is_empty()));
        assert_eq!(lazer.trace(SampleSet::Soft, Voice::Spin, 1), Found::SkinPlain);
        assert_eq!(lazer.get(SampleSet::Normal, Voice::Spin, 1).map(<[f32]>::len), Some(0), "no normal-spinnerspin: the bare blank again");
        assert!(lazer.get(SampleSet::Normal, Voice::Bonus, 1).is_some_and(|sound| !sound.is_empty()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    fn wav(channels: u16, bits: u16, rate: u32, frames: &[i16]) -> Vec<u8> {
        let mut data = Vec::new();
        for &frame in frames {
            for _ in 0..channels {
                match bits {
                    8 => data.push(((frame >> 8) as i32 + 128).clamp(0, 255) as u8),
                    _ => data.extend_from_slice(&frame.to_le_bytes()),
                }
            }
        }

        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&rate.to_le_bytes());
        out.extend_from_slice(&(rate * u32::from(channels) * u32::from(bits / 8)).to_le_bytes());
        out.extend_from_slice(&(channels * bits / 8).to_le_bytes());
        out.extend_from_slice(&bits.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&data);
        out
    }

    fn skin(files: &[(&str, Option<&[i16]>)]) -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "dossier-pack-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a folder");
        for (leaf, frames) in files {
            let body = frames.map_or_else(Vec::new, |f| wav(1, 16, SAMPLE_RATE, f));
            std::fs::write(dir.join(format!("{leaf}.wav")), body).expect("a file");
        }
        dir
    }

    const LOUD: &[i16] = &[16_384, -16_384, 16_384];

    #[test]
    fn sixteen_bit_mono_decodes_to_the_samples_that_went_in() {
        let decoded = decode_wav(&wav(1, 16, SAMPLE_RATE, &[0, 16_384, -16_384])).unwrap();
        assert_eq!(decoded.len(), 3);
        assert!((decoded[1] - 0.5).abs() < 0.001);
        assert!((decoded[2] + 0.5).abs() < 0.001);
    }

    #[test]
    fn stereo_is_folded_to_mono() {
        let decoded = decode_wav(&wav(2, 16, SAMPLE_RATE, &[16_384])).unwrap();
        assert_eq!(decoded.len(), 1);
        assert!((decoded[0] - 0.5).abs() < 0.001);
    }

    #[test]
    fn eight_bit_is_read_as_unsigned() {
        let decoded = decode_wav(&wav(1, 8, SAMPLE_RATE, &[0])).unwrap();
        assert!(
            decoded[0].abs() < 0.01,
            "silence came out as {}",
            decoded[0]
        );
    }

    #[test]
    fn a_different_rate_is_resampled_to_ours() {
        let decoded = decode_wav(&wav(1, 16, SAMPLE_RATE / 2, &[0; 100])).unwrap();
        assert!(
            (decoded.len() as i32 - 200).abs() <= 1,
            "got {} samples",
            decoded.len()
        );
    }

    #[test]
    fn nonsense_is_refused_rather_than_played() {
        assert!(decode_wav(b"not a wav at all").is_none());
        assert!(decode_wav(&[]).is_none());

        assert!(decode_wav(&wav(1, 24, SAMPLE_RATE, &[0])).is_none());
    }

    const QUIET: &[i16] = &[2_048, -2_048, 2_048];

    #[test]
    fn a_sound_the_skin_leaves_out_comes_from_osu_rather_than_another_bank() {
        let dressed = skin(&[("normal-hitwhistle", Some(LOUD))]);
        let osu = skin(&[
            ("normal-hitwhistle", Some(LOUD)),
            ("soft-hitwhistle", Some(QUIET)),
        ]);
        let pack = SamplePack::load(&dressed).with_game_sounds(&osu);
        let heard = pack
            .get(SampleSet::Soft, Voice::Whistle, 1)
            .expect("a sound");
        assert!(heard[0] < 0.2, "the skin's normal bank was used instead");
        assert_eq!(pack.trace(SampleSet::Soft, Voice::Whistle, 1), Found::Game);
    }

    #[test]
    fn a_sound_the_skin_removed_stays_removed() {
        let silenced = skin(&[("soft-hitwhistle", None)]);
        let osu = skin(&[("soft-hitwhistle", Some(LOUD))]);
        let pack = SamplePack::load(&silenced).with_game_sounds(&osu);
        assert_eq!(
            pack.get(SampleSet::Soft, Voice::Whistle, 1),
            Some(&[][..]),
            "a deliberate blank was overruled"
        );
        assert_eq!(pack.trace(SampleSet::Soft, Voice::Whistle, 1), Found::Blank);
    }

    #[test]
    fn the_skin_and_the_map_both_still_come_first() {
        let dressed = skin(&[("soft-hitwhistle", Some(LOUD))]);
        let osu = skin(&[("soft-hitwhistle", Some(QUIET))]);
        let pack = SamplePack::load(&dressed).with_game_sounds(&osu);
        assert_eq!(
            pack.trace(SampleSet::Soft, Voice::Whistle, 1),
            Found::SkinPlain
        );
        assert!(
            pack.get(SampleSet::Soft, Voice::Whistle, 1)
                .expect("a sound")[0]
                > 0.4
        );

        let map = skin(&[("soft-hitwhistle4", Some(&[8_192, -8_192, 8_192]))]);
        let pack = pack.with_beatmap(&map);
        assert_eq!(
            pack.trace(SampleSet::Soft, Voice::Whistle, 4),
            Found::Beatmap(4)
        );
    }

    #[test]
    fn without_that_folder_the_old_liberty_is_still_taken() {
        let pack = SamplePack::load(&skin(&[("normal-hitwhistle", Some(LOUD))]));
        assert_eq!(
            pack.trace(SampleSet::Soft, Voice::Whistle, 1),
            Found::SkinNormalBank
        );
    }

    #[test]
    fn a_file_written_with_a_one_is_not_the_plain_sound() {
        let dir = skin(&[
            ("soft-hitnormal", Some(LOUD)),
            ("soft-hitnormal1", Some(&[512, -512, 512])),
        ]);
        let pack = SamplePack::load(&dir);
        let heard = pack
            .get(SampleSet::Soft, Voice::Normal, 1)
            .expect("a sound");
        assert!(
            heard[0] > 0.4,
            "the numbered file was played as the plain one"
        );
        assert!(
            pack.unused().contains(&"soft-hitnormal1".to_owned()),
            "a name the game never asks for should be reported, not used"
        );
    }

    #[test]
    fn the_plain_sound_still_answers_when_it_is_the_only_one() {
        let pack = SamplePack::default().with_beatmap(&skin(&[
            ("soft-hitnormal", Some(LOUD)),
            ("soft-hitnormal2", Some(&[512, -512, 512])),
        ]));
        assert_eq!(
            pack.trace(SampleSet::Soft, Voice::Normal, 1),
            Found::Beatmap(1)
        );
        assert_eq!(
            pack.trace(SampleSet::Soft, Voice::Normal, 2),
            Found::Beatmap(2)
        );
    }

    #[test]
    fn a_skin_that_capitalises_a_name_is_still_read() {
        let dir = std::env::temp_dir().join(format!(
            "dossier-case-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos())
        ));
        std::fs::create_dir_all(&dir).expect("a folder");
        for name in ["Soft-HitClap.WAV", "NORMAL-HitWhistle.Wav"] {
            std::fs::write(
                dir.join(name),
                wav(1, 16, 44_100, &[1000, -1000, 1000, -1000]),
            )
            .expect("a file");
        }

        let pack = SamplePack::load(&dir);
        assert!(
            pack.get(SampleSet::Soft, Voice::Clap, 1).is_some(),
            "a capitalised clap was not found"
        );
        assert!(
            pack.get(SampleSet::Normal, Voice::Whistle, 1).is_some(),
            "a capitalised whistle was not found"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn loading_a_folder_that_is_not_there_gives_an_empty_pack() {
        let pack = SamplePack::load(Path::new("/nowhere/at/all"));
        assert!(pack.is_empty());
        assert!(pack.get(SampleSet::Normal, Voice::Normal, 1).is_none());
    }

    #[test]
    fn a_missing_set_falls_back_to_normal() {
        let pack = SamplePack::load(&skin(&[("normal-hitclap", Some(LOUD))]));
        assert!(pack.get(SampleSet::Drum, Voice::Clap, 1).is_some());
        assert!(pack.get(SampleSet::Drum, Voice::Finish, 1).is_none());
    }

    #[test]
    fn a_map_that_switches_banks_gets_the_bank_it_asked_for() {
        let dir = skin(&[
            ("soft-hitnormal", Some(&[4_000])),
            ("soft-hitnormal2", Some(&[4_000, 4_000])),
            (
                "soft-hitnormal6",
                Some(&[4_000, 4_000, 4_000, 4_000, 4_000, 4_000]),
            ),
        ]);
        let pack = SamplePack::default().with_beatmap(&dir);

        assert_eq!(
            pack.get(SampleSet::Soft, Voice::Normal, 1).unwrap().len(),
            1
        );
        assert_eq!(
            pack.get(SampleSet::Soft, Voice::Normal, 2).unwrap().len(),
            2
        );
        assert_eq!(
            pack.get(SampleSet::Soft, Voice::Normal, 6).unwrap().len(),
            6
        );

        assert_eq!(
            pack.get(SampleSet::Soft, Voice::Normal, 0).unwrap().len(),
            1
        );
    }

    #[test]
    fn a_skins_numbered_file_is_never_asked_for() {
        let dir = skin(&[
            ("soft-hitwhistle", Some(&[4_000])),
            ("soft-hitwhistle2", Some(LOUD)),
        ]);
        let pack = SamplePack::load(&dir);

        for asked in [1, 2, 4] {
            assert_eq!(
                pack.get(SampleSet::Soft, Voice::Whistle, asked)
                    .unwrap_or_else(|| panic!("silence at index {asked}"))
                    .len(),
                1,
                "index {asked} should be the skin's plain file"
            );
        }

        assert!(
            pack.unused().contains(&"soft-hitwhistle2".to_owned()),
            "only a beatmap may hold a numbered sound"
        );
    }

    #[test]
    fn the_map_is_asked_before_the_skin() {
        let map = skin(&[("soft-hitwhistle4", Some(&[4_000, 4_000, 4_000]))]);
        let dressed = skin(&[("soft-hitwhistle", Some(&[4_000]))]);
        let pack = SamplePack::load(&dressed).with_beatmap(&map);

        assert_eq!(
            pack.get(SampleSet::Soft, Voice::Whistle, 4).unwrap().len(),
            3,
            "the map's own numbered sound"
        );

        assert_eq!(
            pack.get(SampleSet::Soft, Voice::Whistle, 5).unwrap().len(),
            1,
            "the skin's plain sound"
        );
    }

    #[test]
    fn a_voice_nobody_carries_is_left_to_synthesis() {
        let pack = SamplePack::load(&skin(&[("soft-hitclap", Some(LOUD))]));
        assert!(pack.get(SampleSet::Soft, Voice::Whistle, 1).is_none());
    }

    #[test]
    fn a_slip_of_the_finger_fills_a_slot_nothing_else_speaks_for() {
        let dir = skin(&[
            ("normal-hitwistle", Some(LOUD)),
            ("soft-hitfinish", Some(LOUD)),
            ("softl-hitfinish", Some(&[99])),
        ]);
        let pack = SamplePack::load(&dir);

        assert!(
            pack.get(SampleSet::Normal, Voice::Whistle, 1)
                .is_some_and(|s| !s.is_empty()),
            "the misspelt whistle was left on the floor"
        );
        assert_eq!(
            pack.get(SampleSet::Soft, Voice::Finish, 1).unwrap().len(),
            LOUD.len(),
            "a guess overruled a name that was spelt right"
        );
        assert_eq!(pack.unused(), ["softl-hitfinish"]);
    }

    #[test]
    fn a_guess_does_not_undo_a_deliberate_blank() {
        let dir = skin(&[
            ("normal-hitwhistle", None),
            ("normal-hitwistle", Some(LOUD)),
        ]);
        let pack = SamplePack::load(&dir);
        assert_eq!(
            pack.get(SampleSet::Normal, Voice::Whistle, 1),
            Some(&[][..]),
            "a guess put back a sound the skin removed"
        );
        assert_eq!(
            pack.trace(SampleSet::Normal, Voice::Whistle, 1),
            Found::Blank
        );
        assert_eq!(pack.unused(), ["normal-hitwistle"]);
    }

    #[test]
    fn a_guess_never_silences_a_sound_that_is_there() {
        let dir = skin(&[("drum-hitwhistle", Some(LOUD)), ("drum--hitwhistle", None)]);
        let pack = SamplePack::load(&dir);
        assert_eq!(
            pack.get(SampleSet::Drum, Voice::Whistle, 1).unwrap().len(),
            LOUD.len(),
            "a blank with a doubled dash silenced the real file"
        );
    }

    #[test]
    fn a_name_no_amount_of_forgiveness_reaches_is_left_alone() {
        let dir = skin(&[("soft-hitnormal", Some(LOUD)), ("soft-hitsoft", Some(LOUD))]);
        let pack = SamplePack::load(&dir);
        assert_eq!(pack.unused(), ["soft-hitsoft"]);
    }

    #[test]
    fn what_no_voice_uses_is_remembered_so_it_can_be_named() {
        let dir = skin(&[
            ("soft-hitnormal", Some(LOUD)),
            ("menu-play-click", Some(LOUD)),
            ("combobreak", Some(LOUD)),
        ]);
        let pack = SamplePack::load(&dir);
        assert_eq!(
            pack.unused(),
            ["menu-play-click"],
            "the list is what it could neither file nor place"
        );
        assert!(
            pack.get(SampleSet::Normal, Voice::Miss, 1).is_some(),
            "combobreak is filed"
        );
    }

    #[test]
    fn a_name_that_is_not_a_sample_is_not_read_as_one() {
        assert_eq!(
            parse_sample_name("soft-hitwhistle4"),
            Some((SampleSet::Soft, Voice::Whistle, 4))
        );
        assert_eq!(
            parse_sample_name("drum-sliderslide"),
            Some((SampleSet::Drum, Voice::Slide, 1))
        );

        assert_eq!(parse_sample_name("audio"), None);
        assert_eq!(parse_sample_name("bg"), None);
        assert_eq!(parse_sample_name("taiko-normal-hitclap"), None);
    }
}
