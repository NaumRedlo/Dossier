use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

pub const EZ: u32 = 2;
pub const HD: u32 = 8;
pub const HR: u32 = 16;
pub const DT: u32 = 64;
pub const HT: u32 = 256;
pub const NC: u32 = 512;
pub const FL: u32 = 1024;

const DIFFICULTY_MODS: u32 = EZ | HD | HR | DT | HT | FL;
pub(crate) const ENTRY_SIZED_BEFORE: i32 = 20_191_106;
pub(crate) const STARS_FROM: i32 = 20_140_609;
const STRING_MOST: usize = 1 << 20;
const POINTS_MOST: i32 = 1_000_000;
const PAIRS_MOST: i32 = 4096;
const STANDARD: u8 = 0;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Beatmap {
    pub md5: String,
    pub id: i32,
    pub set: i32,
    pub mode: u8,
    pub status: u8,
    pub version: String,
    pub ar: f32,
    pub cs: f32,
    pub od: f32,
    pub hp: f32,
    pub bpm: f32,
    pub length_ms: i32,
    pub drain_s: i32,
    pub objects: u32,
    pub stars: Vec<(u32, f32)>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Facts {
    pub id: i32,
    pub set: i32,
    pub status: &'static str,
    pub stars: f32,
    pub base_stars: f32,
    pub ar: f32,
    pub cs: f32,
    pub od: f32,
    pub hp: f32,
    pub bpm: f32,
    pub length: i32,
    pub objects: u32,
}

impl Beatmap {
    pub fn stars_with(&self, mods: u32) -> Option<f32> {
        if self.stars.is_empty() {
            return None;
        }
        let mut key = mods;
        if key & NC != 0 {
            key |= DT;
        }
        key &= DIFFICULTY_MODS;
        for tried in [key, key & !HD, key & !(HD | FL)] {
            if let Some((_, stars)) = self.stars.iter().find(|(held, _)| *held == tried) {
                return Some(*stars);
            }
        }
        self.stars.iter().find(|(held, _)| *held == 0).map(|(_, stars)| *stars)
    }

    pub fn facts(&self, mods: u32) -> Facts {
        let base = self.stars_with(0).unwrap_or(0.0);
        Facts {
            id: self.id,
            set: self.set,
            status: status_name(self.status),
            stars: self.stars_with(mods).unwrap_or(base),
            base_stars: base,
            ar: self.ar,
            cs: self.cs,
            od: self.od,
            hp: self.hp,
            bpm: self.bpm,
            length: self.length_ms / 1000,
            objects: self.objects,
        }
    }
}

pub fn status_name(raw: u8) -> &'static str {
    match raw {
        1 => "unsubmitted",
        2 => "pending",
        4 => "ranked",
        5 => "approved",
        6 => "qualified",
        7 => "loved",
        _ => "unknown",
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Library {
    pub version: i32,
    pub player: String,
    pub declared: usize,
    pub complete: bool,
    maps: HashMap<String, Beatmap>,
}

impl Library {
    pub fn get(&self, md5: &str) -> Option<&Beatmap> {
        self.maps.get(&md5.to_ascii_lowercase())
    }

    pub fn len(&self) -> usize {
        self.maps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.maps.is_empty()
    }

    pub fn maps(&self) -> impl Iterator<Item = &Beatmap> {
        self.maps.values()
    }

    pub fn open(path: &Path) -> Option<Library> {
        std::fs::read(path).ok().and_then(|bytes| Library::read(&bytes))
    }

    pub fn read(bytes: &[u8]) -> Option<Library> {
        let mut r = Cursor { bytes, at: 0 };
        let version = r.i32()?;
        let _folders = r.i32()?;
        let _unlocked = r.u8()?;
        let _unlock_time = r.i64()?;
        let player = r.string()?;
        let declared = usize::try_from(r.i32()?).ok()?;
        let mut maps = HashMap::with_capacity(declared.min(1 << 20));
        let mut complete = true;
        for _ in 0..declared {
            match entry(&mut r, version) {
                Some(map) => {
                    maps.insert(map.md5.clone(), map);
                }
                None => {
                    complete = false;
                    break;
                }
            }
        }
        Some(Library { version, player, declared, complete, maps })
    }
}

const SHELF_LOOKS_EVERY: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub struct Shelf {
    path: PathBuf,
    library: Option<Library>,
    stamp: Option<SystemTime>,
    looked: Option<Instant>,
    every: Duration,
}

impl Shelf {
    pub fn beside(folder: &Path) -> Shelf {
        Shelf::looking_every(folder, SHELF_LOOKS_EVERY)
    }

    pub fn looking_every(folder: &Path, every: Duration) -> Shelf {
        let mut shelf = Shelf { path: folder.join("osu!.db"), library: None, stamp: None, looked: None, every };
        shelf.refresh();
        shelf
    }

    pub fn map(&mut self, md5: &str) -> Option<&Beatmap> {
        self.refresh();
        self.library.as_ref()?.get(md5)
    }

    pub fn library(&self) -> Option<&Library> {
        self.library.as_ref()
    }

    fn refresh(&mut self) {
        if self.looked.is_some_and(|at| at.elapsed() < self.every) {
            return;
        }
        self.looked = Some(Instant::now());
        let stamp = std::fs::metadata(&self.path).and_then(|meta| meta.modified()).ok();
        if stamp == self.stamp && self.library.is_some() {
            return;
        }
        self.stamp = stamp;
        self.library = stamp.and_then(|_| Library::open(&self.path));
    }
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
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

    fn i16(&mut self) -> Option<i16> {
        self.take(2).map(|b| i16::from_le_bytes([b[0], b[1]]))
    }

    fn i32(&mut self) -> Option<i32> {
        self.take(4).map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn i64(&mut self) -> Option<i64> {
        self.take(8).map(|b| i64::from_le_bytes(b.try_into().unwrap_or([0; 8])))
    }

    fn f32(&mut self) -> Option<f32> {
        self.take(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
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
                if length > STRING_MOST {
                    return None;
                }
                self.take(length).map(|b| String::from_utf8_lossy(b).into_owned())
            }
            _ => None,
        }
    }
}

fn is_md5(said: &str) -> bool {
    said.len() == 32 && said.bytes().all(|b| b.is_ascii_hexdigit())
}

fn pairs(r: &mut Cursor) -> Option<Vec<(u32, f32)>> {
    let count = r.i32()?;
    if !(0..=PAIRS_MOST).contains(&count) {
        return None;
    }
    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let _ = r.u8()?;
        let mods = r.i32()? as u32;
        let stars = match r.u8()? {
            0x0d => r.f64()? as f32,
            0x0c => r.f32()?,
            _ => return None,
        };
        out.push((mods, stars));
    }
    Some(out)
}

fn dominant_bpm(points: &[(f64, f64, bool)], total_ms: i32) -> f32 {
    let mut timing: Vec<(f64, f64)> = points.iter().filter(|(length, _, uninherited)| *uninherited && *length > 0.0).map(|(length, offset, _)| (*offset, *length)).collect();
    timing.sort_by(|a, b| a.0.total_cmp(&b.0));
    let end = f64::from(total_ms.max(0));
    let mut best: Option<(f64, f64)> = None;
    for (at, (offset, length)) in timing.iter().enumerate() {
        let until = timing.get(at + 1).map_or(end, |next| next.0);
        let held = (until - offset).max(0.0);
        if best.is_none_or(|(most, _)| held > most) {
            best = Some((held, *length));
        }
    }
    best.map_or(0.0, |(_, length)| ((60_000.0 / length) * 100.0).round() as f32 / 100.0)
}

fn entry(r: &mut Cursor, version: i32) -> Option<Beatmap> {
    let sized = (version < ENTRY_SIZED_BEFORE).then(|| r.i32()).flatten();
    if version < ENTRY_SIZED_BEFORE && sized.is_none() {
        return None;
    }
    let body = r.at;
    for _ in 0..5 {
        r.string()?;
    }
    let difficulty = r.string()?;
    let _audio = r.string()?;
    let md5 = r.string()?.to_ascii_lowercase();
    let _file = r.string()?;
    if !is_md5(&md5) {
        return None;
    }
    let status = r.u8()?;
    let circles = r.u16()?;
    let sliders = r.u16()?;
    let spinners = r.u16()?;
    let _modified = r.i64()?;
    let (ar, cs, hp, od) = if version >= STARS_FROM {
        (r.f32()?, r.f32()?, r.f32()?, r.f32()?)
    } else {
        (f32::from(r.u8()?), f32::from(r.u8()?), f32::from(r.u8()?), f32::from(r.u8()?))
    };
    let _velocity = r.f64()?;
    let mut standard = Vec::new();
    if version >= STARS_FROM {
        for mode in 0..4 {
            let held = pairs(r)?;
            if mode == usize::from(STANDARD) {
                standard = held;
            }
        }
    }
    let drain = r.i32()?;
    let total = r.i32()?;
    let _preview = r.i32()?;
    let count = r.i32()?;
    if !(0..=POINTS_MOST).contains(&count) {
        return None;
    }
    let mut points = Vec::with_capacity(count as usize);
    for _ in 0..count {
        points.push((r.f64()?, r.f64()?, r.u8()? != 0));
    }
    let id = r.i32()?;
    let set = r.i32()?;
    let _thread = r.i32()?;
    r.take(4)?;
    let _local_offset = r.i16()?;
    let _stack = r.f32()?;
    let mode = r.u8()?;
    r.string()?;
    r.string()?;
    let _online_offset = r.i16()?;
    r.string()?;
    let _unplayed = r.u8()?;
    let _last_played = r.i64()?;
    let _osz2 = r.u8()?;
    r.string()?;
    let _checked = r.i64()?;
    r.take(5)?;
    if version < STARS_FROM {
        let _ = r.i16()?;
    }
    let _last_modified = r.i32()?;
    let _scroll = r.u8()?;
    if let Some(size) = sized {
        r.at = body.checked_add(usize::try_from(size).ok()?)?;
    }
    Some(Beatmap {
        md5,
        id,
        set,
        mode,
        status,
        version: difficulty,
        ar,
        cs,
        od,
        hp,
        bpm: dominant_bpm(&points, total),
        length_ms: total,
        drain_s: drain,
        objects: u32::from(circles) + u32::from(sliders) + u32::from(spinners),
        stars: if mode == STANDARD { standard } else { Vec::new() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::{entry_bytes, library_bytes, spec, Spec};

    const A: &str = "0123456789ABCDEF0123456789abcdef";
    const B: &str = "fedcba9876543210fedcba9876543210";

    #[test]
    fn a_current_library_gives_a_map_its_difficulty_its_numbers_and_its_stars_for_every_mod() {
        let mut mania = spec(B);
        mania.mode = 3;
        mania.stars = vec![(0, 3.0)];
        let bytes = library_bytes(20_260_924, &[entry_bytes(20_260_924, &spec(A), 0), entry_bytes(20_260_924, &mania, 0)]);
        let library = Library::read(&bytes).expect("a library");
        assert!(library.complete && library.len() == 2 && library.player == "Player" && library.version == 20_260_924);
        let map = library.get("0123456789abcdef0123456789abcdef").expect("found by its hash, whatever the case");
        assert_eq!((map.id, map.set, map.mode, map.status, map.version.as_str(), map.objects, map.length_ms, map.drain_s), (11, 5, 0, 4, "Insane", 152, 120_000, 90));
        assert_eq!((map.ar, map.cs, map.hp, map.od, map.bpm), (9.0, 4.0, 6.0, 8.0, 120.0));
        assert_eq!(map.stars_with(0), Some(5.0));
        assert_eq!(map.stars_with(DT), Some(7.5));
        assert_eq!(map.stars_with(NC | HD), Some(7.5), "Nightcore is Double Time and Hidden is not told apart when it was not stored");
        assert_eq!(map.stars_with(HR | HD), Some(5.6));
        assert_eq!(map.stars_with(FL | HD), Some(6.0));
        assert_eq!(map.stars_with(2 | 1 | 4096), Some(5.0), "no mod that changes difficulty, or one that is not stored, falls back to the plain stars");
        let facts = map.facts(DT);
        assert_eq!((facts.status, facts.stars, facts.base_stars, facts.length), ("ranked", 7.5, 5.0, 120));
        assert!(library.get(B).expect("the other map").stars.is_empty(), "stars are kept for osu!standard only");
        assert!(library.get("00000000000000000000000000000000").is_none());
    }

    #[test]
    fn an_older_library_with_sized_entries_and_double_stars_is_read_and_its_sizes_are_trusted() {
        let bytes = library_bytes(20_190_101, &[entry_bytes(20_190_101, &spec(A), 7), entry_bytes(20_190_101, &spec(B), 0)]);
        let library = Library::read(&bytes).expect("a library");
        assert!(library.complete && library.len() == 2, "the padding inside the first entry is skipped by its size");
        assert_eq!(library.get(B).and_then(|map| map.stars_with(64)), Some(7.5));
        assert_eq!(library.get(A).map(|map| map.id), Some(11));
    }

    #[test]
    fn the_bpm_is_the_one_the_map_spends_the_longest_in() {
        let mut long = spec(A);
        long.points = vec![(1000.0, 0.0, true), (-100.0, 500.0, false), (400.0, 10_000.0, true), (300.0, 100_000.0, true)];
        let library = Library::read(&library_bytes(20_260_924, &[entry_bytes(20_260_924, &long, 0)])).expect("a library");
        assert_eq!(library.get(A).map(|map| map.bpm), Some(150.0), "400 ms a beat lasts 90 s, the rest is shorter");
        let mut none = spec(B);
        none.points = Vec::new();
        let library = Library::read(&library_bytes(20_260_924, &[entry_bytes(20_260_924, &none, 0)])).expect("a library");
        assert_eq!(library.get(B).map(|map| map.bpm), Some(0.0));
    }

    #[test]
    fn a_library_cut_short_gives_what_was_read_and_says_it_is_not_whole() {
        let whole = library_bytes(20_260_924, &[entry_bytes(20_260_924, &spec(A), 0), entry_bytes(20_260_924, &spec(B), 0)]);
        let cut = &whole[..whole.len() - 60];
        let library = Library::read(cut).expect("the first map is there");
        assert!(!library.complete && library.len() == 1 && library.declared == 2);
        assert!(Library::read(&[1, 2, 3]).is_none());
        assert!(Library::read(&[]).is_none());
    }

    #[test]
    fn a_hash_that_is_not_one_ends_the_reading_instead_of_being_believed() {
        let broken = Spec { md5: "not-a-hash-not-a-hash-not-a-hash!", ..spec(A) };
        let library = Library::read(&library_bytes(20_260_924, &[entry_bytes(20_260_924, &spec(A), 0), entry_bytes(20_260_924, &broken, 0)])).expect("a library");
        assert!(!library.complete && library.len() == 1);
    }

    #[test]
    fn a_shelf_follows_the_file_beside_the_client_and_notices_when_it_is_written_again() {
        let folder = std::env::temp_dir().join(format!("dossier-witness-shelf-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("a folder");
        let mut shelf = Shelf::looking_every(&folder, Duration::ZERO);
        assert!(shelf.map(A).is_none() && shelf.library().is_none(), "no file, no maps");
        std::fs::write(folder.join("osu!.db"), library_bytes(20_260_924, &[entry_bytes(20_260_924, &spec(A), 0)])).expect("written");
        assert_eq!(shelf.map(A).map(|map| map.id), Some(11));
        assert!(shelf.map(B).is_none());
        std::thread::sleep(Duration::from_millis(20));
        std::fs::write(folder.join("osu!.db"), library_bytes(20_260_924, &[entry_bytes(20_260_924, &spec(A), 0), entry_bytes(20_260_924, &spec(B), 0)])).expect("written again");
        assert!(shelf.map(B).is_some(), "a library the client wrote anew is read anew");
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn the_status_is_said_in_the_words_the_bot_uses() {
        let said: Vec<&str> = [0u8, 1, 2, 4, 5, 6, 7, 9].iter().map(|raw| status_name(*raw)).collect();
        assert_eq!(said, ["unknown", "unsubmitted", "pending", "ranked", "approved", "qualified", "loved", "unknown"]);
    }
}
