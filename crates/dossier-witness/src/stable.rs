use crate::memory::{Memory, Reads};
use crate::scan::{self, Pattern};

pub const BASE: &str = "F8 01 74 04 83 65";
pub const STATUS: &str = "48 83 F8 04 73 1E";
pub const PLAY_TIME: &str = "5E 5F 5D C3 A1 ?? ?? ?? ?? 89 ?? 04";
pub const RULESETS: &str = "7D 15 A1 ?? ?? ?? ?? 85 C0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchors {
    pub base: u64,
    pub status: u64,
    pub play_time: u64,
    pub rulesets: u64,
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
    Ok(Anchors { base: seek(BASE, Lost::Base)?, status: seek(STATUS, Lost::Status)?, play_time: seek(PLAY_TIME, Lost::PlayTime)?, rulesets: seek(RULESETS, Lost::Rulesets)? })
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

pub fn play(memory: &dyn Memory, anchors: &Anchors) -> Option<Play> {
    let at = score_at(memory, anchors)?;
    let mods = memory.pointer(at + 0x1C).and_then(|held| Some(memory.u32(held + 0xC)? ^ memory.u32(held + 0x8)?)).unwrap_or(0);
    let short = |offset: u64| memory.u16(at + offset).unwrap_or(0);
    Some(Play {
        at,
        player: memory.string_at(at + 0x28).unwrap_or_default(),
        ruleset: memory.i32(at + 0x64).unwrap_or(0),
        mods,
        score: memory.i32(at + 0x78).unwrap_or(0),
        combo: short(0x94),
        max_combo: short(0x68),
        counts: Counts { n100: short(0x88), n300: short(0x8A), n50: short(0x8C), geki: short(0x8E), katu: short(0x90), miss: short(0x92) },
        hit_errors: memory.pointer(at + 0x38).and_then(|list| memory.list(list)).map_or(0, |(_, size)| size),
    })
}

pub fn glance(memory: &dyn Memory, anchors: &Anchors) -> Option<Glance> {
    let raw_mode = memory.u32(memory.pointer(anchors.status - 0x4)?)?;
    let time_ms = memory.pointer(anchors.play_time + 0x5).and_then(|at| memory.i32(at)).unwrap_or(0);
    let mode = Mode::of(raw_mode);
    Some(Glance { raw_mode, mode, time_ms, map: map(memory, anchors), play: if mode == Some(Mode::Play) || mode == Some(Mode::Rank) { play(memory, anchors) } else { None } })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::Fake;

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
        fake.put(CODE, &code, true);
        fake.room(DATA, 0x4000);
        let anchors = Anchors { base: CODE + 0x100, status: CODE + 0x200, play_time: CODE + 0x280, rulesets: CODE + 0x300 };

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
