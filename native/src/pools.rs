use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use dossier_replay::{bits, Mods};

use crate::library::{self, Map};

pub const CALC_VERSION: u32 = 2;
pub const OUTWEIGHS: f64 = 0.4;
const STAMINA_SCALE: f64 = 60.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Mod {
    Nm,
    Hd,
    Hr,
    Dt,
    Fm,
    Tb,
}

impl Mod {
    pub const ALL: [Mod; 6] = [Mod::Nm, Mod::Hd, Mod::Hr, Mod::Dt, Mod::Fm, Mod::Tb];

    pub fn code(self) -> &'static str {
        match self {
            Mod::Nm => "NM",
            Mod::Hd => "HD",
            Mod::Hr => "HR",
            Mod::Dt => "DT",
            Mod::Fm => "FM",
            Mod::Tb => "TB",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Mod::Nm => "pool-mod-nm",
            Mod::Hd => "pool-mod-hd",
            Mod::Hr => "pool-mod-hr",
            Mod::Dt => "pool-mod-dt",
            Mod::Fm => "pool-mod-fm",
            Mod::Tb => "pool-mod-tb",
        }
    }

    pub fn applied(self) -> Mods {
        Mods(match self {
            Mod::Nm | Mod::Fm | Mod::Tb => 0,
            Mod::Hd => bits::HIDDEN,
            Mod::Hr => bits::HARD_ROCK,
            Mod::Dt => bits::DOUBLE_TIME,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Frame {
    Duel,
    Stage,
    Free,
}

impl Frame {
    pub const ALL: [Frame; 3] = [Frame::Duel, Frame::Stage, Frame::Free];

    pub fn mods(self) -> Vec<Mod> {
        use Mod::*;
        match self {
            Frame::Duel => vec![Nm, Nm, Hd, Hr, Dt, Fm, Tb],
            Frame::Stage => vec![Nm, Nm, Nm, Nm, Hd, Hd, Hr, Hr, Dt, Dt, Fm, Tb],
            Frame::Free => Vec::new(),
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Frame::Duel => "pool-frame-duel",
            Frame::Stage => "pool-frame-stage",
            Frame::Free => "pool-frame-free",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Measure {
    pub stars: f64,
    pub bpm: f64,
    pub length_ms: i64,
    pub ar: f64,
    pub od: f64,
    pub cs: f64,
    pub hp: f64,
    pub max_combo: u32,
    pub aim: f64,
    pub speed: f64,
    #[serde(default)]
    pub reading: f64,
    #[serde(default)]
    pub stamina: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Skill {
    Aim,
    Speed,
    Reading,
    Stamina,
}

impl Skill {
    pub const ALL: [Skill; 4] = [Skill::Aim, Skill::Speed, Skill::Reading, Skill::Stamina];

    pub fn key(self) -> &'static str {
        match self {
            Skill::Aim => "pool-skill-aim",
            Skill::Speed => "pool-skill-speed",
            Skill::Reading => "pool-skill-reading",
            Skill::Stamina => "pool-skill-stamina",
        }
    }

    pub fn outweighs_key(self) -> &'static str {
        match self {
            Skill::Aim => "pool-heavy-aim",
            Skill::Speed => "pool-heavy-speed",
            Skill::Reading => "pool-heavy-reading",
            Skill::Stamina => "pool-heavy-stamina",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Balance {
    Even,
    Heavy(Skill, u32),
}

impl Measure {
    pub fn skills(&self) -> [f64; 4] {
        [self.aim.max(0.0), self.speed.max(0.0), self.reading.max(0.0), self.stamina.max(0.0)]
    }

    pub fn shares(&self) -> [f64; 4] {
        let skills = self.skills();
        let sum: f64 = skills.iter().sum();
        if sum <= 0.0 {
            return [0.0; 4];
        }
        skills.map(|value| value / sum)
    }

    pub fn percents(&self) -> [u32; 4] {
        whole_percents(self.shares())
    }
}

pub fn whole_percents(shares: [f64; 4]) -> [u32; 4] {
    let total: f64 = shares.iter().sum();
    if total <= 0.0 {
        return [0; 4];
    }
    let mut floors = [0u32; 4];
    let mut rests = [(0usize, 0.0f64); 4];
    for (at, share) in shares.iter().enumerate() {
        let exact = share / total * 100.0;
        floors[at] = exact.floor() as u32;
        rests[at] = (at, exact - exact.floor());
    }
    let mut left = 100 - floors.iter().sum::<u32>().min(100);
    rests.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    for (at, _) in rests {
        if left == 0 {
            break;
        }
        floors[at] += 1;
        left -= 1;
    }
    floors
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Slot {
    pub hash: Option<String>,
    pub mods: Mod,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub colour: Option<[u8; 3]>,
    #[serde(default)]
    pub artist: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub set: Option<u64>,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub measure: Option<Measure>,
}

impl Slot {
    pub fn empty(mods: Mod) -> Slot {
        Slot { hash: None, mods, category: String::new(), colour: None, artist: String::new(), title: String::new(), version: String::new(), set: None, note: String::new(), measure: None }
    }

    pub fn is_empty(&self) -> bool {
        self.hash.is_none()
    }

    pub fn fill(&mut self, hash: &str, map: &Map) {
        if self.hash.as_deref() != Some(hash) {
            self.measure = None;
        }
        self.hash = Some(hash.to_owned());
        self.artist = map.artist.clone();
        self.title = map.title.clone();
        self.version = map.version.clone();
    }

    pub fn clear(&mut self) {
        let category = self.category.clone();
        let colour = self.colour;
        *self = Slot::empty(self.mods);
        self.category = category;
        self.colour = colour;
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Pool {
    pub id: String,
    #[serde(default)]
    pub collection: bool,
    #[serde(default)]
    pub published_revision: u64,
    #[serde(default)]
    pub categories: std::collections::BTreeMap<String, [u8; 3]>,
    #[serde(default)]
    pub category_order: Vec<String>,
    #[serde(default)]
    pub compiler: String,
    pub name: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub note: String,
    pub frame: Frame,
    pub slots: Vec<Slot>,
    pub calc: u32,
    pub made_at: i64,
    pub changed_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Facts {
    pub cards: usize,
    pub minutes: i64,
    pub low: Option<f64>,
    pub high: Option<f64>,
}

impl Pool {
    pub fn new(frame: Frame, name: &str, now: i64) -> Pool {
        Pool {
            id: fresh_id(now),
            collection: false,
            published_revision: 0,
            categories: Default::default(),
            category_order: Vec::new(),
            compiler: String::new(),
            name: name.to_owned(),
            authors: Vec::new(),
            note: String::new(),
            frame,
            slots: frame.mods().into_iter().map(Slot::empty).collect(),
            calc: CALC_VERSION,
            made_at: now,
            changed_at: now,
        }
    }

    pub fn filled(&self) -> usize {
        self.slots.iter().filter(|slot| !slot.is_empty()).count()
    }

    pub fn first_empty(&self) -> Option<usize> {
        self.slots.iter().position(Slot::is_empty)
    }

    pub fn facts(&self) -> Facts {
        let measured: Vec<&Measure> = self.slots.iter().filter_map(|slot| slot.measure.as_ref()).collect();
        let milliseconds: i64 = measured.iter().map(|measure| measure.length_ms).sum();
        let low = measured.iter().map(|measure| measure.stars).reduce(f64::min);
        let high = measured.iter().map(|measure| measure.stars).reduce(f64::max);
        Facts { cards: self.filled(), minutes: (milliseconds as f64 / 60_000.0).round() as i64, low, high }
    }

    pub fn profile(&self) -> Option<[f64; 4]> {
        let measured: Vec<[f64; 4]> = self.slots.iter().filter_map(|slot| slot.measure.as_ref()).map(Measure::shares).collect();
        if measured.is_empty() {
            return None;
        }
        let mut mean = [0.0; 4];
        for shares in &measured {
            for at in 0..4 {
                mean[at] += shares[at] / measured.len() as f64;
            }
        }
        Some(mean)
    }

    pub fn balance(&self) -> Option<Balance> {
        let profile = self.profile()?;
        let percents = whole_percents(profile);
        let (at, top) = profile.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1))?;
        Some(if *top >= OUTWEIGHS { Balance::Heavy(Skill::ALL[at], percents[at]) } else { Balance::Even })
    }

    pub fn fingerprint(&self) -> String {
        let mut text = format!("{:?}", self.frame);
        for slot in &self.slots {
            text.push('|');
            text.push_str(&slot.hash.as_deref().unwrap_or("-").to_ascii_lowercase());
            text.push(':');
            text.push_str(slot.mods.code());
        }
        let digest = library::md5_hex(text.as_bytes());
        const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
        let bytes: Vec<u8> = (0..digest.len() / 2).filter_map(|at| u8::from_str_radix(&digest[at * 2..at * 2 + 2], 16).ok()).collect();
        let mut number = bytes.iter().take(5).fold(0u64, |acc, byte| (acc << 8) | u64::from(*byte));
        let mut out = [b'0'; 7];
        for place in out.iter_mut().rev() {
            *place = ALPHABET[(number & 31) as usize];
            number >>= 5;
        }
        String::from_utf8_lossy(&out).into_owned()
    }
}

fn fresh_id(now: i64) -> String {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |since| since.as_nanos());
    let seed = format!("{now}:{nanos}:{}:{}", std::process::id(), COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    library::md5_hex(seed.as_bytes())[..12].to_owned()
}

pub fn pools_dir() -> PathBuf {
    crate::sources::own_root().join("pools")
}

fn file_in(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.pool"))
}

pub fn load_all(dir: &Path) -> Vec<Pool> {
    let mut pools: Vec<Pool> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "pool"))
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .filter_map(|text| serde_json::from_str::<Pool>(&text).ok())
        .collect();
    pools.sort_by(|a, b| b.changed_at.cmp(&a.changed_at).then_with(|| a.id.cmp(&b.id)));
    pools
}

pub fn save(dir: &Path, pool: &Pool) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|why| format!("{}: {why}", dir.display()))?;
    let text = serde_json::to_string_pretty(pool).map_err(|why| why.to_string())?;
    let target = file_in(dir, &pool.id);
    let staging = dir.join(format!("{}.pool.part", pool.id));
    std::fs::write(&staging, text).map_err(|why| format!("{}: {why}", staging.display()))?;
    std::fs::rename(&staging, &target).map_err(|why| format!("{}: {why}", target.display()))
}

pub fn remove(dir: &Path, id: &str) -> Result<(), String> {
    let target = file_in(dir, id);
    match std::fs::remove_file(&target) {
        Ok(()) => Ok(()),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(why) => Err(format!("{}: {why}", target.display())),
    }
}

fn clamp10(value: f64) -> f64 {
    value.clamp(0.0, 10.0)
}

fn approach_rate_with(ar: f64, hard_rock: bool, rate: f64) -> f64 {
    let ar = if hard_rock { clamp10(ar * 1.4) } else { ar };
    let preempt = dossier_beatmap::difficulty_range(ar, 1800.0, 1200.0, 450.0) / rate;
    if preempt > 1200.0 {
        (1800.0 - preempt) / 120.0
    } else {
        5.0 + (1200.0 - preempt) / 150.0
    }
}

fn overall_difficulty_with(od: f64, hard_rock: bool, rate: f64) -> f64 {
    let od = if hard_rock { clamp10(od * 1.4) } else { od };
    let window = dossier_beatmap::difficulty_range(od, 80.0, 50.0, 20.0) / rate;
    (80.0 - window) / 6.0
}

pub fn measure_text(text: &str, mods: Mod) -> Result<Measure, String> {
    let beatmap = dossier_beatmap::Beatmap::parse(text).map_err(|why| why.to_string())?;
    if beatmap.mode != 0 {
        return Err("mode".to_owned());
    }
    let applied = mods.applied();
    let rate = applied.speed_multiplier();
    let hard_rock = applied.contains(bits::HARD_ROCK);
    let attributes = dossier_assay::attributes(&beatmap, applied);
    let tempo = beatmap.timing.bpm_at(beatmap.objects.first().map_or(0.0, |first| first.time_ms));
    Ok(Measure {
        stars: attributes.star_rating,
        bpm: tempo * rate,
        length_ms: (beatmap.drain_time_ms() / rate).round() as i64,
        ar: approach_rate_with(beatmap.difficulty.approach_rate, hard_rock, rate),
        od: overall_difficulty_with(beatmap.difficulty.overall_difficulty, hard_rock, rate),
        cs: if hard_rock { clamp10(beatmap.difficulty.circle_size * 1.3) } else { beatmap.difficulty.circle_size },
        hp: if hard_rock { clamp10(beatmap.difficulty.hp_drain * 1.4) } else { beatmap.difficulty.hp_drain },
        max_combo: attributes.max_combo,
        aim: attributes.aim_difficulty,
        speed: attributes.speed_difficulty,
        reading: attributes.reading_difficulty,
        stamina: ((attributes.aim_difficult_strain_count + attributes.speed_difficult_strain_count) / 2.0 / STAMINA_SCALE).max(0.0).sqrt(),
    })
}

pub fn measure(map: &Map, hash: &str, mods: Mod) -> Result<Measure, String> {
    let found = dossier_produce::locate::load_map(&map.file, hash)?;
    measure_text(&found.text, mods)
}

pub fn suggest(songs: &HashMap<String, Map>, mods: Mod, target: f64, excluded: &HashSet<String>, stop: &AtomicBool) -> Vec<(String, Measure)> {
    let mut best: Vec<(String, Measure)> = Vec::new();
    for (hash, map) in songs {
        if stop.load(Ordering::Relaxed) {
            return Vec::new();
        }
        if excluded.contains(hash) {
            continue;
        }
        let Ok(measure) = measure(map, hash, mods) else { continue };
        if !measure.stars.is_finite() {
            continue;
        }
        keep_closest(&mut best, hash.clone(), measure, target);
    }
    best
}

fn keep_closest(best: &mut Vec<(String, Measure)>, hash: String, measure: Measure, target: f64) {
    let place = best.partition_point(|(other_hash, other)| {
        (other.stars - target).abs().total_cmp(&(measure.stars - target).abs()).then_with(|| other_hash.cmp(&hash)).is_lt()
    });
    if place < 8 {
        best.insert(place, (hash, measure));
        best.truncate(8);
    }
}

#[derive(Debug, Default, Clone)]
pub struct Measures {
    done: HashMap<(String, Mod), Result<Measure, String>>,
}

impl Measures {
    pub fn get(&self, hash: &str, mods: Mod) -> Option<&Result<Measure, String>> {
        self.done.get(&(hash.to_owned(), mods))
    }

    pub fn put(&mut self, hash: &str, mods: Mod, said: Result<Measure, String>) {
        self.done.insert((hash.to_owned(), mods), said);
    }

    pub fn has(&self, hash: &str, mods: Mod) -> bool {
        self.done.contains_key(&(hash.to_owned(), mods))
    }
}

pub fn clock(length_ms: i64) -> String {
    let seconds = (length_ms.max(0) + 500) / 1000;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> String {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates/dossier-assay/corpus/maps/5114204.osu");
        let bytes = std::fs::read(path).expect("corpus map");
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dossier-pools-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn frames_give_the_known_slot_rows() {
        let codes = |frame: Frame| frame.mods().into_iter().map(Mod::code).collect::<Vec<_>>().join(" ");
        assert_eq!(codes(Frame::Duel), "NM NM HD HR DT FM TB");
        assert_eq!(Frame::Stage.mods().len(), 12);
        assert_eq!(Frame::Stage.mods().last(), Some(&Mod::Tb));
        assert!(Frame::Free.mods().is_empty());
    }

    #[test]
    fn a_new_pool_has_empty_slots_in_frame_order_and_a_unique_id() {
        let a = Pool::new(Frame::Duel, "A", 1_790_000_000);
        let b = Pool::new(Frame::Duel, "A", 1_790_000_000);
        assert_eq!(a.slots.len(), 7);
        assert!(a.slots.iter().all(Slot::is_empty));
        assert_eq!(a.first_empty(), Some(0));
        assert_ne!(a.id, b.id);
        assert_eq!(a.facts(), Facts { cards: 0, minutes: 0, low: None, high: None });
    }

    #[test]
    fn closest_candidates_stay_sorted_and_bounded() {
        let base = measure_text(&corpus(), Mod::Nm).unwrap();
        let mut best = Vec::new();
        for (hash, stars) in [("far", 8.0), ("b", 5.2), ("a", 4.8), ("near", 5.01), ("c", 5.4), ("d", 5.5), ("e", 5.6), ("f", 5.7), ("g", 5.8), ("h", 5.9)] {
            keep_closest(&mut best, hash.to_owned(), Measure { stars, ..base }, 5.0);
        }
        assert_eq!(best.len(), 8);
        assert_eq!(best[0].0, "near");
        assert_eq!((best[1].0.as_str(), best[2].0.as_str()), ("a", "b"));
        assert!(!best.iter().any(|(hash, _)| hash == "far"));
    }

    #[test]
    fn suggestion_scan_measures_local_maps_and_honours_exclusion_and_cancellation() {
        let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates/dossier-assay/corpus/maps/5114204.osu");
        let hash = library::md5_hex(&std::fs::read(&file).unwrap());
        let songs = HashMap::from([(hash.clone(), Map { file, artist: "A".into(), title: "B".into(), version: "C".into(), background: None })]);
        let stop = AtomicBool::new(false);
        let found = suggest(&songs, Mod::Nm, 5.0, &HashSet::new(), &stop);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, hash);
        assert!(suggest(&songs, Mod::Nm, 5.0, &HashSet::from([hash]), &stop).is_empty());
        stop.store(true, Ordering::Relaxed);
        assert!(suggest(&songs, Mod::Nm, 5.0, &HashSet::new(), &stop).is_empty());
    }

    #[test]
    fn a_pool_survives_the_disk_and_the_newest_comes_first() {
        let dir = scratch("disk");
        let mut older = Pool::new(Frame::Duel, "Older", 1_790_000_000);
        older.slots[0].hash = Some("abc".into());
        older.slots[0].measure = Some(Measure { stars: 4.5, bpm: 180.0, length_ms: 90_000, ar: 9.0, od: 8.0, cs: 4.0, hp: 5.0, max_combo: 500, aim: 2.0, speed: 2.0, reading: 0.5, stamina: 1.0 });
        let mut newer = Pool::new(Frame::Free, "Newer", 1_790_000_100);
        newer.changed_at = 1_790_000_200;
        save(&dir, &older).unwrap();
        save(&dir, &newer).unwrap();
        let loaded = load_all(&dir);
        assert_eq!(loaded.iter().map(|pool| pool.name.as_str()).collect::<Vec<_>>(), ["Newer", "Older"]);
        assert_eq!(loaded[1], older);
        remove(&dir, &older.id).unwrap();
        remove(&dir, &older.id).unwrap();
        assert_eq!(load_all(&dir).len(), 1);
        assert!(std::fs::read_dir(&dir).unwrap().flatten().all(|entry| entry.path().extension().is_some_and(|ext| ext == "pool")), "no staging file is left");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_damaged_file_does_not_hide_the_good_ones() {
        let dir = scratch("damaged");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("broken.pool"), b"{ not json").unwrap();
        std::fs::write(dir.join("note.txt"), b"hello").unwrap();
        save(&dir, &Pool::new(Frame::Duel, "Fine", 1_790_000_000)).unwrap();
        assert_eq!(load_all(&dir).len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn facts_count_cards_minutes_and_the_star_range() {
        let mut pool = Pool::new(Frame::Duel, "P", 1);
        let measure = |stars: f64, length_ms: i64| Measure { stars, bpm: 180.0, length_ms, ar: 9.0, od: 8.0, cs: 4.0, hp: 5.0, max_combo: 1, aim: 1.0, speed: 1.0, reading: 0.5, stamina: 1.0 };
        for (at, (stars, length)) in [(4.52, 132_000), (6.35, 210_000), (5.05, 160_000)].into_iter().enumerate() {
            pool.slots[at].hash = Some(format!("h{at}"));
            pool.slots[at].measure = Some(measure(stars, length));
        }
        let facts = pool.facts();
        assert_eq!((facts.cards, facts.minutes), (3, 8));
        assert_eq!((facts.low, facts.high), (Some(4.52), Some(6.35)));
    }

    #[test]
    fn the_fingerprint_is_seven_symbols_and_follows_the_content() {
        let mut pool = Pool::new(Frame::Duel, "P", 1);
        let empty = pool.fingerprint();
        assert_eq!(empty.len(), 7);
        assert!(empty.chars().all(|c| c.is_ascii_alphanumeric()));
        assert_eq!(pool.fingerprint(), empty);
        pool.slots[0].hash = Some("abc".into());
        assert_ne!(pool.fingerprint(), empty);
        let named = Pool { name: "Another name".into(), ..pool.clone() };
        assert_eq!(named.fingerprint(), pool.fingerprint(), "the name is not part of the fingerprint");
    }

    #[test]
    fn a_map_is_measured_with_the_slot_mod() {
        let text = corpus();
        let plain = measure_text(&text, Mod::Nm).unwrap();
        let double = measure_text(&text, Mod::Dt).unwrap();
        let rock = measure_text(&text, Mod::Hr).unwrap();
        assert!(plain.stars > 1.0 && plain.stars < 12.0, "{}", plain.stars);
        assert!(double.stars > plain.stars, "DT makes the map harder: {} vs {}", double.stars, plain.stars);
        assert!((double.bpm / plain.bpm - 1.5).abs() < 1e-6);
        assert!((plain.length_ms as f64 / double.length_ms as f64 - 1.5).abs() < 0.01);
        assert!(double.ar > plain.ar);
        assert!(rock.ar >= plain.ar && rock.od >= plain.od);
        assert!(rock.cs >= plain.cs);
        assert_eq!(measure_text(&text, Mod::Fm).unwrap(), plain, "free mod and tiebreaker are measured plain");
        assert_eq!(measure_text(&text, Mod::Tb).unwrap(), plain);
    }

    #[test]
    fn another_mode_is_refused_with_the_word_mode() {
        let text = corpus().replace("Mode: 0", "Mode: 1");
        assert_eq!(measure_text(&text, Mod::Nm), Err("mode".to_owned()));
    }

    #[test]
    fn the_clock_rounds_to_seconds() {
        assert_eq!(clock(83_400), "1:23");
        assert_eq!(clock(59_600), "1:00");
        assert_eq!(clock(-5), "0:00");
    }

    fn measure_of(aim: f64, speed: f64, reading: f64, stamina: f64) -> Measure {
        Measure { stars: 5.0, bpm: 180.0, length_ms: 100_000, ar: 9.0, od: 8.0, cs: 4.0, hp: 5.0, max_combo: 500, aim, speed, reading, stamina }
    }

    #[test]
    fn shares_add_up_to_one_hundred_whole_percents() {
        let measure = measure_of(2.93, 1.94, 0.94, 0.92);
        let percents = measure.percents();
        assert_eq!(percents.iter().sum::<u32>(), 100);
        assert_eq!(percents, [43, 29, 14, 14]);
        assert_eq!(whole_percents([1.0, 1.0, 1.0, 1.0]), [25, 25, 25, 25]);
        assert_eq!(whole_percents([1.0, 1.0, 1.0, 0.0]).iter().sum::<u32>(), 100);
        assert_eq!(whole_percents([0.0; 4]), [0; 4]);
        assert_eq!(measure_of(0.0, 0.0, 0.0, 0.0).shares(), [0.0; 4]);
        assert_eq!(measure_of(-1.0, 2.0, 0.0, 0.0).percents(), [0, 100, 0, 0], "a negative skill counts as nothing");
    }

    #[test]
    fn the_pool_profile_is_the_mean_of_the_slot_shares_and_the_balance_names_the_leader() {
        let mut pool = Pool::new(Frame::Duel, "P", 1);
        assert_eq!(pool.profile(), None);
        assert_eq!(pool.balance(), None);
        pool.slots[0].measure = Some(measure_of(1.0, 1.0, 1.0, 1.0));
        assert_eq!(pool.balance(), Some(Balance::Even));
        pool.slots[1].measure = Some(measure_of(6.0, 1.0, 1.0, 1.0));
        let profile = pool.profile().unwrap();
        assert!((profile.iter().sum::<f64>() - 1.0).abs() < 1e-9);
        assert_eq!(pool.balance(), Some(Balance::Heavy(Skill::Aim, 46)), "{profile:?}");
        pool.slots[2].measure = Some(measure_of(1.0, 5.0, 0.2, 0.2));
        pool.slots[3].measure = Some(measure_of(0.5, 7.0, 0.2, 0.3));
        assert!(matches!(pool.balance(), Some(Balance::Heavy(Skill::Speed, _)) | Some(Balance::Even)));
    }

    #[test]
    fn a_measure_saved_before_reading_and_stamina_existed_still_loads() {
        let old = r#"{"stars":4.5,"bpm":180.0,"length_ms":90000,"ar":9.0,"od":8.0,"cs":4.0,"hp":5.0,"max_combo":500,"aim":2.0,"speed":2.0}"#;
        let measure: Measure = serde_json::from_str(old).unwrap();
        assert_eq!((measure.reading, measure.stamina), (0.0, 0.0));
    }

    #[test]
    fn a_real_map_has_all_four_skills() {
        let measured = measure_text(&corpus(), Mod::Nm).unwrap();
        assert!(measured.aim > 0.0 && measured.speed > 0.0, "{measured:?}");
        assert!(measured.reading >= 0.0);
        assert!(measured.stamina > 0.0, "{measured:?}");
        let percents = measured.percents();
        assert_eq!(percents.iter().sum::<u32>(), 100);
        let double = measure_text(&corpus(), Mod::Dt).unwrap();
        assert!(double.speed > measured.speed, "speed grows with DT");
    }
}
