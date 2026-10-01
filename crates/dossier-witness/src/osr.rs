use crate::stable::{Frame, Take};

pub const WRITTEN_AS: i32 = 20_251_102;
const TICKS_TO_UNIX: i64 = 621_355_968_000_000_000;
const END_OF_FRAMES: i64 = -12_345;

fn uleb(out: &mut Vec<u8>, mut value: usize) {
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

fn string(out: &mut Vec<u8>, said: &str) {
    if said.is_empty() {
        out.push(0);
        return;
    }
    out.push(0x0B);
    uleb(out, said.len());
    out.extend_from_slice(said.as_bytes());
}

pub fn frames_text(frames: &[Frame]) -> String {
    let mut said = String::with_capacity(frames.len() * 24);
    let mut before = 0i64;
    for frame in frames {
        let at = i64::from(frame.time);
        said.push_str(&format!("{}|{}|{}|{},", at - before, frame.x, frame.y, frame.keys));
        before = at;
    }
    said.push_str(&format!("{END_OF_FRAMES}|0|0|0,"));
    said
}

pub fn life_text(life: &[(f32, f32)]) -> String {
    life.iter().map(|(at, hp)| format!("{}|{},", at.round() as i64, hp)).collect()
}

pub fn write(take: &Take, player: &str, unix_seconds: i64) -> Vec<u8> {
    let text = frames_text(&take.frames);
    let mut packed = Vec::new();
    lzma_rs::lzma_compress(&mut std::io::BufReader::new(text.as_bytes()), &mut packed).expect("frames compress into memory");
    let name = if take.play.player.is_empty() { player } else { take.play.player.as_str() };
    let counts = take.play.counts;
    let mut out = Vec::with_capacity(packed.len() + 256);
    out.push(take.play.ruleset.clamp(0, 3) as u8);
    out.extend_from_slice(&WRITTEN_AS.to_le_bytes());
    string(&mut out, &take.map.md5);
    string(&mut out, name);
    string(&mut out, &crate::md5::hex(text.as_bytes()));
    for count in [counts.n300, counts.n100, counts.n50, counts.geki, counts.katu, counts.miss] {
        out.extend_from_slice(&count.to_le_bytes());
    }
    out.extend_from_slice(&take.play.score.to_le_bytes());
    out.extend_from_slice(&take.play.max_combo.to_le_bytes());
    out.push(u8::from(take.passed && counts.miss == 0 && take.play.combo == take.play.max_combo));
    out.extend_from_slice(&take.play.mods.to_le_bytes());
    string(&mut out, &life_text(&take.life));
    out.extend_from_slice(&(unix_seconds * 10_000_000 + TICKS_TO_UNIX).to_le_bytes());
    out.extend_from_slice(&(packed.len() as i32).to_le_bytes());
    out.extend_from_slice(&packed);
    out.extend_from_slice(&0i64.to_le_bytes());
    out
}

pub fn file_name(take: &Take, player: &str, unix_seconds: i64) -> String {
    let name = if take.play.player.is_empty() { player } else { take.play.player.as_str() };
    let said = format!("{} - {} - {} [{}] ({})", if name.is_empty() { "Guest" } else { name }, take.map.artist, take.map.title, take.map.version, unix_seconds);
    let clean: String = said.chars().map(|c| if c.is_alphanumeric() || " -_[]().,!'".contains(c) { c } else { '_' }).collect();
    format!("{}.osr", clean.trim().chars().take(140).collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stable::{Counts, Map, Play};

    fn taken() -> Take {
        Take {
            score: 0x1000,
            map: Map { md5: "0123456789abcdef0123456789abcdef".into(), artist: "xi".into(), title: "FREEDOM DiVE".into(), version: "FOUR: DIMENSIONS".into(), ..Map::default() },
            play: Play { player: String::new(), ruleset: 0, mods: 24, score: 1_234_567, combo: 733, max_combo: 733, counts: Counts { n300: 1017, n100: 14, n50: 1, geki: 210, katu: 9, miss: 0 }, ..Play::default() },
            frames: vec![Frame { time: 0, x: 256.0, y: -500.0, keys: 0 }, Frame { time: -1, x: 256.0, y: -500.0, keys: 0 }, Frame { time: 1632, x: 245.73, y: 218.25, keys: 5 }, Frame { time: 1648, x: 240.5, y: 220.0, keys: 0 }],
            life: vec![(1632.0, 1.0), (3210.4, 0.87)],
            passed: true,
        }
    }

    #[test]
    fn what_was_written_reads_back_as_the_same_play() {
        let bytes = write(&taken(), "NaumRedlo", 1_790_000_000);
        let read = dossier_replay::Replay::parse(&bytes).expect("the replay reads");
        assert_eq!((read.beatmap_hash.as_str(), read.player.as_str(), read.score, read.max_combo, read.perfect_combo), ("0123456789abcdef0123456789abcdef", "NaumRedlo", 1_234_567, 733, true));
        assert_eq!((read.hits.count_300, read.hits.count_100, read.hits.count_50, read.hits.count_geki, read.hits.count_katu, read.hits.count_miss), (1017, 14, 1, 210, 9, 0));
        assert_eq!(read.mods.raw(), 24);
        assert_eq!(read.replay_hash.len(), 32);
        assert_eq!(read.played_at_unix(), 1_790_000_000);
        assert_eq!(read.life_bar, "1632|1,3210|0.87,");
        let times: Vec<i64> = read.frames.iter().map(|frame| frame.time_ms).collect();
        assert_eq!(times, vec![0, -1, 1632, 1648]);
        assert_eq!((read.frames[2].x, read.frames[2].y, read.frames[2].keys.0), (245.73, 218.25, 5));
    }

    #[test]
    fn the_frames_are_told_as_steps_and_end_with_the_marker() {
        assert_eq!(frames_text(&taken().frames), "0|256|-500|0,-1|256|-500|0,1633|245.73|218.25|5,16|240.5|220|0,-12345|0|0|0,");
    }

    #[test]
    fn a_file_is_named_by_who_played_what_and_keeps_to_what_a_disk_takes() {
        assert_eq!(file_name(&taken(), "", 7), "Guest - xi - FREEDOM DiVE [FOUR_ DIMENSIONS] (7).osr");
        assert!(file_name(&taken(), "NaumRedlo", 7).starts_with("NaumRedlo - xi"));
    }
}
