use std::path::Path;

use crate::stable::Counts;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Held {
    pub md5: String,
    pub player: String,
    pub counts: Counts,
    pub max_combo: u16,
    pub mods: u32,
}

const TARGET: u32 = 1 << 23;

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

fn score(r: &mut Reader) -> Option<(u8, Held)> {
    let mode = r.u8()?;
    r.i32()?;
    let md5 = r.string()?;
    let player = r.string()?;
    r.string()?;
    let (n300, n100, n50, geki, katu, miss) = (r.u16()?, r.u16()?, r.u16()?, r.u16()?, r.u16()?, r.u16()?);
    r.i32()?;
    let max_combo = r.u16()?;
    r.u8()?;
    let mods = r.i32()? as u32;
    r.string()?;
    r.take(8)?;
    r.i32()?;
    r.take(8)?;
    if mods & TARGET != 0 {
        r.take(8)?;
    }
    Some((mode, Held { md5, player, counts: Counts { n300, n100, n50, geki, katu, miss }, max_combo, mods }))
}

pub fn read(bytes: &[u8]) -> Option<Vec<Held>> {
    let mut r = Reader { bytes, at: 0 };
    r.i32()?;
    let maps = r.i32()?;
    let mut out = Vec::new();
    for _ in 0..maps.max(0) {
        r.string()?;
        let count = r.i32()?;
        for _ in 0..count.max(0) {
            let (mode, held) = score(&mut r)?;
            if mode == 0 {
                out.push(held);
            }
        }
    }
    Some(out)
}

pub fn on_map(file: &Path, md5: &str, player: &str) -> Vec<Held> {
    let all = std::fs::read(file).ok().and_then(|bytes| read(&bytes)).unwrap_or_default();
    all.into_iter().filter(|held| held.md5.eq_ignore_ascii_case(md5) && (player.is_empty() || held.player.eq_ignore_ascii_case(player))).collect()
}

#[cfg(test)]
pub fn written(scores: &[Held]) -> Vec<u8> {
    let string = |out: &mut Vec<u8>, said: &str| {
        if said.is_empty() {
            out.push(0);
        } else {
            out.push(0x0b);
            out.push(said.len() as u8);
            out.extend_from_slice(said.as_bytes());
        }
    };
    let mut out = Vec::new();
    out.extend_from_slice(&20_260_924i32.to_le_bytes());
    out.extend_from_slice(&(scores.len() as i32).to_le_bytes());
    for held in scores {
        string(&mut out, &held.md5);
        out.extend_from_slice(&1i32.to_le_bytes());
        out.push(0);
        out.extend_from_slice(&20_260_924i32.to_le_bytes());
        string(&mut out, &held.md5);
        string(&mut out, &held.player);
        string(&mut out, "replay");
        for count in [held.counts.n300, held.counts.n100, held.counts.n50, held.counts.geki, held.counts.katu, held.counts.miss] {
            out.extend_from_slice(&count.to_le_bytes());
        }
        out.extend_from_slice(&1_000_000i32.to_le_bytes());
        out.extend_from_slice(&held.max_combo.to_le_bytes());
        out.push(0);
        out.extend_from_slice(&(held.mods as i32).to_le_bytes());
        string(&mut out, "");
        out.extend_from_slice(&0i64.to_le_bytes());
        out.extend_from_slice(&(-1i32).to_le_bytes());
        out.extend_from_slice(&0i64.to_le_bytes());
        if held.mods & TARGET != 0 {
            out.extend_from_slice(&0f64.to_le_bytes());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_scores_are_read_and_picked_by_map_and_player() {
        let held = |md5: &str, player: &str, mods: u32| Held { md5: md5.repeat(32), player: player.to_owned(), counts: Counts { n300: 300, n100: 4, miss: 1, ..Counts::default() }, max_combo: 250, mods };
        let all = [held("a", "NaumRedlo", 0), held("a", "Guest", 8), held("b", "NaumRedlo", 64 | TARGET), held("a", "naumredlo", 72)];
        let bytes = written(&all);
        assert_eq!(read(&bytes).unwrap(), all);
        assert!(read(&bytes[..bytes.len() - 3]).is_none(), "a cut file is not half believed");
        let file = std::env::temp_dir().join(format!("dossier-witness-scores-{}.db", std::process::id()));
        std::fs::write(&file, &bytes).unwrap();
        assert_eq!(on_map(&file, &"A".repeat(32), "NaumRedlo").iter().map(|held| held.mods).collect::<Vec<_>>(), [0, 72]);
        assert_eq!(on_map(&file, &"a".repeat(32), "").len(), 3, "without a name every local score counts");
        assert!(on_map(&file.with_extension("none"), &"a".repeat(32), "").is_empty());
        let _ = std::fs::remove_file(&file);
    }
}
