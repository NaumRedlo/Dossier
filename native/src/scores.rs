use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use dossier_replay::{bits, GameMode, HitCounts, Mods, Replay};

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.at.checked_add(n)?;
        let out = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(out)
    }

    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }

    fn u16(&mut self) -> Option<u16> {
        self.take(2).map(|b| u16::from_le_bytes([b[0], b[1]]))
    }

    fn i32(&mut self) -> Option<i32> {
        self.take(4).map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn i64(&mut self) -> Option<i64> {
        self.take(8).map(|b| i64::from_le_bytes(b.try_into().unwrap_or([0; 8])))
    }

    fn f64(&mut self) -> Option<f64> {
        self.take(8).map(|b| f64::from_le_bytes(b.try_into().unwrap_or([0; 8])))
    }

    fn string(&mut self) -> Option<String> {
        match self.u8()? {
            0x00 => Some(String::new()),
            0x0b => {
                let mut length = 0usize;
                let mut shift = 0;
                loop {
                    let byte = self.u8()?;
                    length |= ((byte & 0x7f) as usize) << shift;
                    if byte & 0x80 == 0 {
                        break;
                    }
                    shift += 7;
                    if shift > 28 {
                        return None;
                    }
                }
                self.take(length).map(|b| String::from_utf8_lossy(b).into_owned())
            }
            _ => None,
        }
    }
}

fn score(r: &mut Reader) -> Option<(u8, Replay)> {
    let mode = r.u8()?;
    let game_version = r.i32()?;
    let beatmap_hash = r.string()?;
    let player = r.string()?;
    let replay_hash = r.string()?;
    let hits = HitCounts { count_300: r.u16()?, count_100: r.u16()?, count_50: r.u16()?, count_geki: r.u16()?, count_katu: r.u16()?, count_miss: r.u16()? };
    let score = r.i32()?;
    let max_combo = r.u16()?;
    let perfect_combo = r.u8()? != 0;
    let raw_mods = r.i32()? as u32;
    let life_bar = r.string()?;
    let timestamp_ticks = r.i64()?;
    let _ = r.i32()?;
    let online_score_id = r.i64()?;
    let target_practice_accuracy = if raw_mods & bits::TARGET != 0 { Some(r.f64()?) } else { None };
    Some((
        mode,
        Replay {
            mode: GameMode::Standard,
            game_version,
            beatmap_hash,
            player,
            replay_hash,
            hits,
            score,
            max_combo,
            perfect_combo,
            mods: Mods::new(raw_mods),
            life_bar,
            timestamp_ticks,
            online_score_id,
            target_practice_accuracy,
            frames: Vec::new(),
            rng_seed: None,
            score_info: None,
        },
    ))
}

pub fn read(bytes: &[u8]) -> Option<Vec<Replay>> {
    let mut r = Reader { bytes, at: 0 };
    let _version = r.i32()?;
    let maps = r.i32()?;
    let mut out = Vec::new();
    for _ in 0..maps.max(0) {
        let _map = r.string()?;
        let count = r.i32()?;
        for _ in 0..count.max(0) {
            let (mode, replay) = score(&mut r)?;
            if mode == 0 {
                out.push(replay);
            }
        }
    }
    Some(out)
}

pub fn in_folder(root: &Path) -> Vec<Replay> {
    std::fs::read(root.join("scores.db")).ok().and_then(|bytes| read(&bytes)).unwrap_or_default()
}

pub fn with_replays(root: &Path) -> Vec<(PathBuf, Replay)> {
    let scores = in_folder(root);
    if scores.is_empty() {
        return Vec::new();
    }
    let dir = root.join("Data").join("r");
    let files: HashSet<String> = std::fs::read_dir(&dir)
        .map(|entries| entries.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|name| name.to_ascii_lowercase().ends_with(".osr")).collect())
        .unwrap_or_default();
    let mut taken: HashSet<String> = HashSet::new();
    let mut found = Vec::new();
    let mut unmatched = Vec::new();
    for replay in scores {
        let name = format!("{}-{}.osr", replay.beatmap_hash, replay.timestamp_ticks);
        if files.contains(&name) {
            taken.insert(name.clone());
            found.push((dir.join(name), replay));
        } else {
            unmatched.push(replay);
        }
    }
    if !unmatched.is_empty() {
        let mut by_hash: HashMap<String, PathBuf> = HashMap::new();
        for name in files.iter().filter(|name| !taken.contains(*name)) {
            let path = dir.join(name);
            let mut head = Vec::with_capacity(512);
            if let Ok(file) = std::fs::File::open(&path) {
                use std::io::Read;
                let _ = file.take(512).read_to_end(&mut head);
            }
            let mut r = Reader { bytes: &head, at: 0 };
            let hash = (|| {
                r.u8()?;
                r.i32()?;
                r.string()?;
                r.string()?;
                r.string()
            })();
            if let Some(hash) = hash.filter(|h| !h.is_empty()) {
                by_hash.insert(hash, path);
            }
        }
        for replay in unmatched {
            if let Some(path) = by_hash.remove(&replay.replay_hash) {
                found.push((path, replay));
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn string(out: &mut Vec<u8>, text: &str) {
        if text.is_empty() {
            out.push(0);
            return;
        }
        out.push(0x0b);
        let mut n = text.len();
        loop {
            let byte = (n & 0x7f) as u8;
            n >>= 7;
            if n == 0 {
                out.push(byte);
                break;
            }
            out.push(byte | 0x80);
        }
        out.extend(text.as_bytes());
    }

    fn scored(out: &mut Vec<u8>, mode: u8, map: &str, player: &str, replay: &str, mods: u32, ticks: i64) {
        out.push(mode);
        out.extend(20250101i32.to_le_bytes());
        string(out, map);
        string(out, player);
        string(out, replay);
        for count in [300u16, 20, 3, 40, 10, 2] {
            out.extend(count.to_le_bytes());
        }
        out.extend(1_234_567i32.to_le_bytes());
        out.extend(512u16.to_le_bytes());
        out.push(0);
        out.extend((mods as i32).to_le_bytes());
        string(out, "");
        out.extend(ticks.to_le_bytes());
        out.extend((-1i32).to_le_bytes());
        out.extend(99i64.to_le_bytes());
        if mods & bits::TARGET != 0 {
            out.extend(0.97f64.to_le_bytes());
        }
    }

    fn database() -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(20250101i32.to_le_bytes());
        out.extend(2i32.to_le_bytes());
        string(&mut out, "aaaa");
        out.extend(2i32.to_le_bytes());
        scored(&mut out, 0, "aaaa", "NaumRedlo", "r1", 8 | 64, 638_000_000_000_000_001);
        scored(&mut out, 3, "aaaa", "NaumRedlo", "r2", 0, 638_000_000_000_000_002);
        string(&mut out, "bbbb");
        out.extend(1i32.to_le_bytes());
        scored(&mut out, 0, "bbbb", "Guest", "r3", bits::TARGET, 638_000_000_000_000_003);
        out
    }

    #[test]
    fn the_database_gives_the_standard_scores_with_their_headers() {
        let scores = read(&database()).expect("a database");
        assert_eq!(scores.len(), 2, "mania is left out");
        assert_eq!(scores[0].player, "NaumRedlo");
        assert_eq!(scores[0].beatmap_hash, "aaaa");
        assert_eq!(scores[0].replay_hash, "r1");
        assert_eq!(scores[0].hits.count_miss, 2);
        assert_eq!(scores[0].max_combo, 512);
        assert_eq!(scores[0].mods.acronyms(), vec!["HD", "DT"]);
        assert_eq!(scores[1].target_practice_accuracy, Some(0.97));
    }

    #[test]
    fn a_truncated_database_is_refused_rather_than_misread() {
        let whole = database();
        assert!(read(&whole[..whole.len() - 3]).is_none());
    }

    #[test]
    fn replays_are_found_by_name_and_otherwise_by_their_own_hash() {
        let root = std::env::temp_dir().join(format!("dossier-scores-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Data").join("r")).unwrap();
        std::fs::write(root.join("scores.db"), database()).unwrap();
        std::fs::write(root.join("Data").join("r").join("aaaa-638000000000000001.osr"), b"").unwrap();
        let mut elsewhere = Vec::new();
        elsewhere.push(0u8);
        elsewhere.extend(20250101i32.to_le_bytes());
        string(&mut elsewhere, "bbbb");
        string(&mut elsewhere, "Guest");
        string(&mut elsewhere, "r3");
        std::fs::write(root.join("Data").join("r").join("renamed.osr"), &elsewhere).unwrap();
        let found = with_replays(&root);
        let names: Vec<String> = found.iter().map(|(path, _)| path.file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"aaaa-638000000000000001.osr".to_owned()));
        assert!(names.contains(&"renamed.osr".to_owned()));
        let _ = std::fs::remove_dir_all(&root);
    }
}
