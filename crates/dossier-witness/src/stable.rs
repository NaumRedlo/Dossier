use crate::memory::{Memory, Reads};
use crate::scan::{self, Pattern};

pub const BASE: &str = "F8 01 74 04 83 65";
pub const STATUS: &str = "48 83 F8 04 73 1E";
pub const PLAY_TIME: &str = "5E 5F 5D C3 A1 ?? ?? ?? ?? 89 ?? 04";
pub const RULESETS: &str = "7D 15 A1 ?? ?? ?? ?? 85 C0";
pub const REPLAY: &str = "55 8B EC 80 3D ?? ?? ?? ?? 00 75 26 80 3D";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchors {
    pub base: u64,
    pub status: u64,
    pub play_time: u64,
    pub rulesets: u64,
    pub replay: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lost {
    Base,
    Status,
    PlayTime,
    Rulesets,
}

pub fn anchors(memory: &dyn Memory) -> Result<Anchors, Lost> {
    let seek = |said: &str, lost: Lost| Pattern::parse(said).and_then(|pattern| scan::find(memory, &pattern)).ok_or(lost);
    Ok(Anchors { base: seek(BASE, Lost::Base)?, status: seek(STATUS, Lost::Status)?, play_time: seek(PLAY_TIME, Lost::PlayTime)?, rulesets: seek(RULESETS, Lost::Rulesets)?, replay: seek(REPLAY, Lost::Rulesets).ok() })
}

pub fn watching(memory: &dyn Memory, anchors: &Anchors) -> Option<bool> {
    match memory.u8(memory.pointer(anchors.replay? + 0x46)?)? {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

fn watched(before: Option<bool>, now: Option<bool>) -> Option<bool> {
    match (before, now) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), Some(false)) => Some(false),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Menu,
    Edit,
    Play,
    Exit,
    SelectEdit,
    SelectPlay,
    SelectDrawings,
    Rank,
    Update,
    Busy,
    Unknown,
    Lobby,
    MatchSetup,
    SelectMulti,
    RankingVs,
    OnlineSelection,
    OptionsOffsetWizard,
    RankingTagCoop,
    RankingTeam,
    BeatmapImport,
    PackageUpdater,
    Benchmark,
    Tourney,
    Charts,
}

impl Mode {
    pub const ALL: [Mode; 24] = [
        Mode::Menu,
        Mode::Edit,
        Mode::Play,
        Mode::Exit,
        Mode::SelectEdit,
        Mode::SelectPlay,
        Mode::SelectDrawings,
        Mode::Rank,
        Mode::Update,
        Mode::Busy,
        Mode::Unknown,
        Mode::Lobby,
        Mode::MatchSetup,
        Mode::SelectMulti,
        Mode::RankingVs,
        Mode::OnlineSelection,
        Mode::OptionsOffsetWizard,
        Mode::RankingTagCoop,
        Mode::RankingTeam,
        Mode::BeatmapImport,
        Mode::PackageUpdater,
        Mode::Benchmark,
        Mode::Tourney,
        Mode::Charts,
    ];

    pub fn of(raw: u32) -> Option<Mode> {
        Mode::ALL.get(raw as usize).copied()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Map {
    pub md5: String,
    pub id: i32,
    pub set: i32,
    pub artist: String,
    pub title: String,
    pub version: String,
    pub creator: String,
    pub file: String,
    pub folder: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    pub n300: u16,
    pub n100: u16,
    pub n50: u16,
    pub geki: u16,
    pub katu: u16,
    pub miss: u16,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Play {
    pub at: u64,
    pub player: String,
    pub ruleset: i32,
    pub mods: u32,
    pub score: i32,
    pub combo: u16,
    pub max_combo: u16,
    pub counts: Counts,
    pub hit_errors: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Glance {
    pub raw_mode: u32,
    pub mode: Option<Mode>,
    pub time_ms: i32,
    pub map: Option<Map>,
    pub play: Option<Play>,
    pub watching: Option<bool>,
}

fn is_md5(said: &str) -> bool {
    said.len() == 32 && said.bytes().all(|b| b.is_ascii_hexdigit())
}

pub fn map(memory: &dyn Memory, anchors: &Anchors) -> Option<Map> {
    let at = memory.pointer(memory.pointer(anchors.base - 0xC)?)?;
    let md5 = memory.string_at(at + 0x6C)?;
    if !is_md5(&md5) {
        return None;
    }
    let said = |offset: u64| memory.string_at(at + offset).unwrap_or_default();
    Some(Map {
        md5,
        id: memory.i32(at + 0xC8).unwrap_or(0),
        set: memory.i32(at + 0xCC).unwrap_or(0),
        artist: said(0x18),
        title: said(0x24),
        version: said(0xAC),
        creator: said(0x7C),
        file: said(0x90),
        folder: said(0x78),
    })
}

pub fn score_at(memory: &dyn Memory, anchors: &Anchors) -> Option<u64> {
    let rulesets = memory.pointer(memory.pointer(anchors.rulesets - 0xB)? + 0x4)?;
    let gameplay = memory.pointer(rulesets + 0x64)?;
    memory.pointer(gameplay + 0x38)
}

pub const HEALTH_MOST: f64 = 200.0;

pub fn health(memory: &dyn Memory, anchors: &Anchors) -> Option<f64> {
    let rulesets = memory.pointer(memory.pointer(anchors.rulesets - 0xB)? + 0x4)?;
    let gameplay = memory.pointer(rulesets + 0x64)?;
    let bar = memory.pointer(gameplay + 0x40)?;
    memory.f64(bar + 0x1C).filter(|health| health.is_finite() && (0.0..=HEALTH_MOST).contains(health))
}

pub const NEVER_FAILS: u32 = 1 | 1 << 7 | 1 << 13;

pub fn play(memory: &dyn Memory, anchors: &Anchors) -> Option<Play> {
    score_at(memory, anchors).map(|at| play_of(memory, at))
}

pub fn result_at(memory: &dyn Memory, anchors: &Anchors) -> Option<u64> {
    let rulesets = memory.pointer(memory.pointer(anchors.rulesets - 0xB)? + 0x4)?;
    memory.pointer(rulesets + 0x38)
}

pub fn play_of(memory: &dyn Memory, at: u64) -> Play {
    let mods = memory.pointer(at + 0x1C).and_then(|held| Some(memory.u32(held + 0xC)? ^ memory.u32(held + 0x8)?)).unwrap_or(0);
    let short = |offset: u64| memory.u16(at + offset).unwrap_or(0);
    Play {
        at,
        player: memory.string_at(at + 0x28).unwrap_or_default(),
        ruleset: memory.i32(at + 0x64).unwrap_or(0),
        mods,
        score: memory.i32(at + 0x78).unwrap_or(0),
        combo: short(0x94),
        max_combo: short(0x68),
        counts: Counts { n100: short(0x88), n300: short(0x8A), n50: short(0x8C), geki: short(0x8E), katu: short(0x90), miss: short(0x92) },
        hit_errors: memory.pointer(at + 0x38).and_then(|list| memory.list(list)).map_or(0, |(_, size)| size),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub time: i32,
    pub x: f32,
    pub y: f32,
    pub keys: u32,
}

pub const FRAME_TIME_MOST: i32 = 24 * 3600 * 1000;

pub fn frame_count(memory: &dyn Memory, score: u64) -> Option<usize> {
    memory.list(memory.pointer(score + 0x34)?).map(|(_, size)| size)
}

pub fn frames_from(memory: &dyn Memory, score: u64, from: usize) -> Option<Vec<Frame>> {
    let (items, size) = memory.list(memory.pointer(score + 0x34)?)?;
    if from >= size {
        return Some(Vec::new());
    }
    let held = memory.bytes(items + 4 * from as u64, (size - from) * 4)?;
    let mut found = Vec::with_capacity(size - from);
    for word in held.chunks_exact(4) {
        let at = u64::from(u32::from_le_bytes([word[0], word[1], word[2], word[3]]));
        let raw = memory.array::<16>(at + 4)?;
        let part = |n: usize| [raw[n], raw[n + 1], raw[n + 2], raw[n + 3]];
        let frame = Frame { x: f32::from_le_bytes(part(0)), y: f32::from_le_bytes(part(4)), keys: u32::from_le_bytes(part(8)), time: i32::from_le_bytes(part(12)) };
        if !frame.x.is_finite() || !frame.y.is_finite() || frame.time.abs() > FRAME_TIME_MOST || frame.keys > 0xFF {
            return None;
        }
        found.push(frame);
    }
    Some(found)
}

pub fn life(memory: &dyn Memory, score: u64) -> Vec<(f32, f32)> {
    let Some((items, size)) = memory.pointer(score + 0x24).and_then(|list| memory.list(list)) else {
        return Vec::new();
    };
    let Some(raw) = memory.bytes(items, size * 8) else {
        return Vec::new();
    };
    raw.chunks_exact(8).map(|pair| (f32::from_le_bytes([pair[0], pair[1], pair[2], pair[3]]), f32::from_le_bytes([pair[4], pair[5], pair[6], pair[7]]))).filter(|(at, hp)| at.is_finite() && hp.is_finite()).collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct Take {
    pub score: u64,
    pub map: Map,
    pub play: Play,
    pub frames: Vec<Frame>,
    pub life: Vec<(f32, f32)>,
    pub passed: bool,
    pub failed: bool,
    pub watched: Option<bool>,
}

#[derive(Debug, Default)]
pub struct Recorder {
    current: Option<Take>,
    filled: bool,
}

impl Recorder {
    pub fn recording(&self) -> Option<&Take> {
        self.current.as_ref()
    }

    fn continues(take: &Take, memory: &dyn Memory, score: u64) -> bool {
        let Some(last) = take.frames.len().checked_sub(1) else {
            return true;
        };
        let Some((items, _)) = memory.pointer(score + 0x34).and_then(|list| memory.list(list)) else {
            return false;
        };
        let held = memory.pointer(items + 4 * last as u64).and_then(|at| memory.array::<16>(at + 4));
        held.is_some_and(|raw| i32::from_le_bytes([raw[12], raw[13], raw[14], raw[15]]) == take.frames[last].time && f32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) == take.frames[last].x)
    }

    fn refresh(take: &mut Take, memory: &dyn Memory, score: u64) {
        take.play = play_of(memory, score);
        if let Some(fresh) = frames_from(memory, score, take.frames.len()) {
            take.frames.extend(fresh);
        }
        let seen = life(memory, score);
        if seen.len() >= take.life.len() {
            take.life = seen;
        }
    }

    pub fn poll(&mut self, memory: &dyn Memory, anchors: &Anchors) -> Option<Take> {
        let seen = glance(memory, anchors)?;
        match seen.mode {
            Some(Mode::Play) => {
                let score = score_at(memory, anchors)?;
                let count = frame_count(memory, score)?;
                let restarted = self.current.as_ref().is_some_and(|take| count < take.frames.len() || !Recorder::continues(take, memory, score));
                let finished = if restarted { self.current.take() } else { None };
                if self.current.is_none() {
                    self.current = Some(Take { score, map: seen.map.clone()?, play: Play::default(), frames: Vec::new(), life: Vec::new(), passed: false, failed: false, watched: seen.watching });
                    self.filled = false;
                }
                if let Some(take) = self.current.as_mut() {
                    take.score = score;
                    take.watched = watched(take.watched, seen.watching);
                    Recorder::refresh(take, memory, score);
                    let read = health(memory, anchors);
                    let emptied = self.filled && read.is_some_and(|health| health <= 0.0);
                    self.filled |= read.is_some_and(|health| health > 0.0);
                    take.failed |= emptied && take.play.mods & NEVER_FAILS == 0 && !take.frames.is_empty();
                }
                finished
            }
            Some(Mode::Rank) => {
                let mut take = self.current.take()?;
                if let Some(score) = result_at(memory, anchors).filter(|score| frame_count(memory, *score).is_some_and(|count| count >= take.frames.len())) {
                    Recorder::refresh(&mut take, memory, score);
                }
                take.passed = true;
                take.failed = false;
                Some(take)
            }
            _ => self.current.take(),
        }
    }
}

pub fn glance(memory: &dyn Memory, anchors: &Anchors) -> Option<Glance> {
    let raw_mode = memory.u32(memory.pointer(anchors.status - 0x4)?)?;
    let time_ms = memory.pointer(anchors.play_time + 0x5).and_then(|at| memory.i32(at)).unwrap_or(0);
    let mode = Mode::of(raw_mode);
    Some(Glance { raw_mode, mode, time_ms, map: map(memory, anchors), play: if mode == Some(Mode::Play) || mode == Some(Mode::Rank) { play(memory, anchors) } else { None }, watching: watching(memory, anchors) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::Fake;
    use crate::memory::Reads;

    const CODE: u64 = 0x0100_0000;
    const DATA: u64 = 0x0200_0000;

    fn staged() -> (Fake, Anchors) {
        let mut fake = Fake::default();
        let mut code = vec![0x90u8; 0x400];
        let place = |code: &mut Vec<u8>, at: usize, bytes: &[u8]| code[at..at + bytes.len()].copy_from_slice(bytes);
        place(&mut code, 0x100, &[0xF8, 0x01, 0x74, 0x04, 0x83, 0x65]);
        place(&mut code, 0x200, &[0x48, 0x83, 0xF8, 0x04, 0x73, 0x1E]);
        place(&mut code, 0x280, &[0x5E, 0x5F, 0x5D, 0xC3, 0xA1, 0, 0, 0, 0, 0x89, 0x46, 0x04]);
        place(&mut code, 0x300, &[0x7D, 0x15, 0xA1, 0, 0, 0, 0, 0x85, 0xC0]);
        place(&mut code, 0x340, &[0x55, 0x8B, 0xEC, 0x80, 0x3D, 0, 0, 0, 0, 0x00, 0x75, 0x26, 0x80, 0x3D]);
        fake.put(CODE, &code, true);
        fake.room(DATA, 0x4000);
        let anchors = Anchors { base: CODE + 0x100, status: CODE + 0x200, play_time: CODE + 0x280, rulesets: CODE + 0x300, replay: Some(CODE + 0x340) };
        fake.set_u32(CODE + 0x340 + 0x46, (DATA + 0x28) as u32);

        fake.set_u32(anchors.status - 0x4, (DATA + 0x10) as u32);
        fake.set_u32(DATA + 0x10, 2);
        fake.set_u32(anchors.play_time + 0x5, (DATA + 0x20) as u32);
        fake.set_u32(DATA + 0x20, 61_500);

        fake.set_u32(anchors.base - 0xC, (DATA + 0x30) as u32);
        let beatmap = DATA + 0x400;
        fake.set_u32(DATA + 0x30, beatmap as u32);
        for (offset, at, said) in [(0x6C, 0x800, "0123456789abcdef0123456789abcdef"), (0x18, 0x880, "xi"), (0x24, 0x8C0, "FREEDOM DiVE"), (0xAC, 0x900, "FOUR DIMENSIONS"), (0x7C, 0x940, "Nakagawa-Kanon"), (0x90, 0x980, "xi - FREEDOM DiVE (Nakagawa-Kanon) [FOUR DIMENSIONS].osu"), (0x78, 0xA40, "39804 xi - FREEDOM DiVE")] {
            fake.set_u32(beatmap + offset, (DATA + at) as u32);
            fake.set_string(DATA + at, said);
        }
        fake.set_u32(beatmap + 0xC8, 129_891);
        fake.set_u32(beatmap + 0xCC, 39_804);

        fake.set_u32(anchors.rulesets - 0xB, (DATA + 0x40) as u32);
        let (rulesets, gameplay, score) = (DATA + 0x1000, DATA + 0x1200, DATA + 0x1400);
        fake.set_u32(DATA + 0x44, rulesets as u32);
        fake.set_u32(rulesets + 0x64, gameplay as u32);
        fake.set_u32(gameplay + 0x38, score as u32);
        fake.set_u32(score + 0x28, (DATA + 0x1800) as u32);
        fake.set_string(DATA + 0x1800, "NaumRedlo");
        fake.set_u32(score + 0x1C, (DATA + 0x1900) as u32);
        fake.set_u32(DATA + 0x1908, 0x5A5A_0000);
        fake.set_u32(DATA + 0x190C, 0x5A5A_0000 ^ 24);
        fake.set_u32(score + 0x78, 1_234_567);
        fake.write(score + 0x68, &1800u16.to_le_bytes());
        for (offset, value) in [(0x88u64, 14u16), (0x8A, 1017), (0x8C, 1), (0x8E, 210), (0x90, 9), (0x92, 2), (0x94, 733)] {
            fake.write(score + offset, &value.to_le_bytes());
        }
        let (list, items) = (DATA + 0x1A00, DATA + 0x1B00);
        fake.set_u32(score + 0x38, list as u32);
        fake.set_u32(list + 0x4, items as u32);
        fake.set_u32(list + 0xC, 3);
        fake.set_u32(items + 0x4, 4);
        (fake, anchors)
    }

    #[test]
    fn the_anchors_are_found_by_their_signatures() {
        let (fake, expected) = staged();
        assert_eq!(anchors(&fake), Ok(expected));
        let mut bare = Fake::default();
        bare.put(CODE, &[0xF8, 0x01, 0x74, 0x04, 0x83, 0x65], true);
        assert_eq!(anchors(&bare), Err(Lost::Status));
    }

    #[test]
    fn a_glance_tells_the_screen_the_time_the_map_and_the_play() {
        let (fake, anchors) = staged();
        let seen = glance(&fake, &anchors).expect("a glance");
        assert_eq!((seen.mode, seen.time_ms), (Some(Mode::Play), 61_500));
        let map = seen.map.expect("the map");
        assert_eq!((map.md5.as_str(), map.id, map.set, map.title.as_str(), map.version.as_str()), ("0123456789abcdef0123456789abcdef", 129_891, 39_804, "FREEDOM DiVE", "FOUR DIMENSIONS"));
        let play = seen.play.expect("the play");
        assert_eq!((play.player.as_str(), play.mods, play.score, play.combo, play.max_combo, play.hit_errors), ("NaumRedlo", 24, 1_234_567, 733, 1800, 3));
        assert_eq!(play.counts, Counts { n300: 1017, n100: 14, n50: 1, geki: 210, katu: 9, miss: 2 });
    }

    const FRAMES: u64 = 0x0300_0000;

    fn put_frames(fake: &mut Fake, score: u64, frames: &[(i32, f32, f32, u32)]) {
        let (list, items, objects) = (FRAMES, FRAMES + 0x100, FRAMES + 0x1000);
        fake.set_u32(score + 0x34, list as u32);
        fake.set_u32(list + 0x4, items as u32);
        fake.set_u32(list + 0xC, frames.len() as u32);
        fake.set_u32(items + 0x4, 256);
        for (n, (time, x, y, keys)) in frames.iter().enumerate() {
            let at = objects + 0x20 * n as u64;
            fake.set_u32(items + 0x8 + 4 * n as u64, at as u32);
            fake.write(at + 0x4, &x.to_le_bytes());
            fake.write(at + 0x8, &y.to_le_bytes());
            fake.set_u32(at + 0xC, *keys);
            fake.write(at + 0x10, &time.to_le_bytes());
        }
    }

    fn with_frames() -> (Fake, Anchors, u64) {
        let (mut fake, anchors) = staged();
        fake.room(FRAMES, 0x4000);
        let score = DATA + 0x1400;
        put_frames(&mut fake, score, &[(0, 256.0, -500.0, 0), (-1, 256.0, -500.0, 0), (1632, 245.5, 218.25, 5)]);
        (fake, anchors, score)
    }

    #[test]
    fn frames_are_read_from_where_the_last_reading_stopped() {
        let (mut fake, _, score) = with_frames();
        assert_eq!(frame_count(&fake, score), Some(3));
        let all = frames_from(&fake, score, 0).expect("frames");
        assert_eq!(all[2], Frame { time: 1632, x: 245.5, y: 218.25, keys: 5 });
        put_frames(&mut fake, score, &[(0, 256.0, -500.0, 0), (-1, 256.0, -500.0, 0), (1632, 245.5, 218.25, 5), (1648, 240.0, 220.0, 0)]);
        assert_eq!(frames_from(&fake, score, 3), Some(vec![Frame { time: 1648, x: 240.0, y: 220.0, keys: 0 }]));
        assert_eq!(frames_from(&fake, score, 4), Some(Vec::new()));
        put_frames(&mut fake, score, &[(0, f32::NAN, 0.0, 0)]);
        assert_eq!(frames_from(&fake, score, 0), None, "a frame that makes no sense is not believed");
    }

    #[test]
    fn a_play_is_handed_over_when_it_ends_and_a_retry_begins_another() {
        let (mut fake, anchors, score) = with_frames();
        let mut recorder = Recorder::default();
        assert!(recorder.poll(&fake, &anchors).is_none());
        assert_eq!(recorder.recording().map(|take| take.frames.len()), Some(3));
        put_frames(&mut fake, score, &[(0, 256.0, -500.0, 0), (-1, 256.0, -500.0, 0), (1632, 245.5, 218.25, 5), (1648, 240.0, 220.0, 0)]);
        assert!(recorder.poll(&fake, &anchors).is_none());
        assert_eq!(recorder.recording().map(|take| take.frames.len()), Some(4));

        let moved = DATA + 0x2400;
        let held = fake.bytes(score, 0xA0).expect("the score");
        fake.write(moved, &held);
        fake.set_u32(DATA + 0x1200 + 0x38, moved as u32);
        assert!(recorder.poll(&fake, &anchors).is_none(), "the collector moved the score and the play was taken for a new one");
        assert_eq!(recorder.recording().map(|take| (take.frames.len(), take.score)), Some((4, moved)));
        let score = moved;

        put_frames(&mut fake, score, &[(0, 256.0, -500.0, 0)]);
        let dropped = recorder.poll(&fake, &anchors).expect("the play before the retry");
        assert_eq!((dropped.frames.len(), dropped.passed, dropped.play.score), (4, false, 1_234_567));
        assert_eq!(recorder.recording().map(|take| take.frames.len()), Some(1));

        fake.set_u32(DATA + 0x10, 5);
        let left = recorder.poll(&fake, &anchors).expect("the play that was left");
        assert!(!left.passed && left.map.title == "FREEDOM DiVE");
        assert!(recorder.recording().is_none() && recorder.poll(&fake, &anchors).is_none());
    }

    fn set_health(fake: &mut Fake, health: f64) {
        let bar = DATA + 0x2800;
        fake.set_u32(DATA + 0x1200 + 0x40, bar as u32);
        fake.write(bar + 0x1C, &health.to_le_bytes());
    }

    #[test]
    fn a_play_whose_health_ran_out_is_a_fail_and_one_that_was_left_is_not() {
        let (mut fake, anchors, _) = with_frames();
        assert_eq!(health(&fake, &anchors), None, "a client that does not show the bar is not guessed at");
        set_health(&mut fake, 140.0);
        assert_eq!(health(&fake, &anchors), Some(140.0));

        let mut recorder = Recorder::default();
        assert!(recorder.poll(&fake, &anchors).is_none());
        fake.set_u32(DATA + 0x10, 5);
        let left = recorder.poll(&fake, &anchors).expect("the play that was left");
        assert!(!left.passed && !left.failed, "health to spare when the screen was left");

        fake.set_u32(DATA + 0x10, 2);
        assert!(recorder.poll(&fake, &anchors).is_none());
        set_health(&mut fake, 0.0);
        assert!(recorder.poll(&fake, &anchors).is_none());
        set_health(&mut fake, 35.0);
        fake.set_u32(DATA + 0x10, 5);
        let failed = recorder.poll(&fake, &anchors).expect("the play that failed");
        assert!(failed.failed && !failed.passed, "it was seen at nothing once, whatever the bar did afterwards");

        set_health(&mut fake, 999.0);
        assert_eq!(health(&fake, &anchors), None, "a number the bar cannot hold is not a reading");
    }

    #[test]
    fn a_bar_that_was_never_seen_filled_has_not_run_out() {
        let (mut fake, anchors, _) = with_frames();
        set_health(&mut fake, 0.0);
        let mut recorder = Recorder::default();
        assert!(recorder.poll(&fake, &anchors).is_none());
        assert!(recorder.poll(&fake, &anchors).is_none());
        fake.set_u32(DATA + 0x10, 5);
        let left = recorder.poll(&fake, &anchors).expect("the play that was left");
        assert!(!left.failed, "a bar not yet set for the play reads nothing, and that is not a fail");

        fake.set_u32(DATA + 0x10, 2);
        assert!(recorder.poll(&fake, &anchors).is_none());
        set_health(&mut fake, 200.0);
        assert!(recorder.poll(&fake, &anchors).is_none());
        set_health(&mut fake, 0.0);
        assert!(recorder.poll(&fake, &anchors).is_none());
        fake.set_u32(DATA + 0x10, 5);
        assert!(recorder.poll(&fake, &anchors).expect("the play that failed").failed, "filled and then emptied is a fail");
    }

    #[test]
    fn health_at_nothing_under_no_fail_is_not_a_fail() {
        let (mut fake, anchors, score) = with_frames();
        fake.set_u32(DATA + 0x190C, 0x5A5A_0000 ^ 1);
        set_health(&mut fake, 0.0);
        let mut recorder = Recorder::default();
        assert!(recorder.poll(&fake, &anchors).is_none());
        fake.set_u32(DATA + 0x1000 + 0x38, score as u32);
        fake.set_u32(DATA + 0x10, 7);
        let done = recorder.poll(&fake, &anchors).expect("the finished play");
        assert!(done.passed && !done.failed);
    }

    #[test]
    fn a_play_that_reaches_the_results_is_passed_and_takes_its_last_frames_from_there() {
        let (mut fake, anchors, score) = with_frames();
        let mut recorder = Recorder::default();
        assert!(recorder.poll(&fake, &anchors).is_none());
        put_frames(&mut fake, score, &[(0, 256.0, -500.0, 0), (-1, 256.0, -500.0, 0), (1632, 245.5, 218.25, 5), (1648, 240.0, 220.0, 0), (1700, 1.0, 2.0, 1)]);
        fake.set_u32(DATA + 0x1000 + 0x38, score as u32);
        fake.set_u32(score + 0x78, 2_000_000);
        fake.set_u32(DATA + 0x10, 7);
        let done = recorder.poll(&fake, &anchors).expect("the finished play");
        assert_eq!((done.passed, done.frames.len(), done.play.score, done.watched), (true, 5, 2_000_000, Some(false)));
    }

    #[test]
    fn a_replay_that_was_watched_is_not_taken_for_a_play() {
        let (mut fake, anchors, _) = with_frames();
        let mut recorder = Recorder::default();
        assert_eq!(watching(&fake, &anchors), Some(false));
        assert!(recorder.poll(&fake, &anchors).is_none());
        fake.set_u32(DATA + 0x28, 1);
        assert_eq!(watching(&fake, &anchors), Some(true));
        assert!(recorder.poll(&fake, &anchors).is_none());
        fake.set_u32(DATA + 0x28, 0);
        fake.set_u32(DATA + 0x10, 0);
        assert_eq!(recorder.poll(&fake, &anchors).expect("what was left").watched, Some(true));
        fake.set_u32(DATA + 0x28, 7);
        assert_eq!(watching(&fake, &anchors), None);
        assert_eq!(watching(&fake, &Anchors { replay: None, ..anchors }), None);
        assert_eq!((watched(Some(false), None), watched(None, Some(false)), watched(Some(false), Some(false))), (None, None, Some(false)));
    }

    #[test]
    fn the_life_bar_is_read_as_pairs_of_time_and_health() {
        let (mut fake, _, score) = with_frames();
        let (list, items) = (FRAMES + 0x3000, FRAMES + 0x3100);
        fake.set_u32(score + 0x24, list as u32);
        fake.set_u32(list + 0x4, items as u32);
        fake.set_u32(list + 0xC, 2);
        fake.set_u32(items + 0x4, 4);
        for (n, value) in [1632.0f32, 1.0, 3210.0, 0.87].iter().enumerate() {
            fake.write(items + 0x8 + 4 * n as u64, &value.to_le_bytes());
        }
        assert_eq!(life(&fake, score), vec![(1632.0, 1.0), (3210.0, 0.87)]);
    }

    #[test]
    fn outside_a_play_nothing_is_said_about_one_and_a_broken_hash_is_no_map() {
        let (mut fake, anchors) = staged();
        fake.set_u32(DATA + 0x10, 5);
        let seen = glance(&fake, &anchors).expect("a glance");
        assert_eq!(seen.mode, Some(Mode::SelectPlay));
        assert!(seen.play.is_none() && seen.map.is_some());
        fake.set_string(DATA + 0x800, "not a hash");
        assert!(glance(&fake, &anchors).expect("a glance").map.is_none());
        fake.set_u32(DATA + 0x10, 77);
        assert_eq!(glance(&fake, &anchors).expect("a glance").mode, None);
    }
}
