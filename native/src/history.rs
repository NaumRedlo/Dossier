use std::path::Path;

use dossier_replay::Replay;
use dossier_witness::beatmaps::Library;
use serde::Serialize;

pub const CHUNK: usize = 500;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LocalScore {
    pub md5: String,
    pub id: i64,
    pub set: i64,
    pub played: i64,
    pub score: i64,
    pub combo: u32,
    pub n300: u32,
    pub n100: u32,
    pub n50: u32,
    pub geki: u32,
    pub katu: u32,
    pub miss: u32,
    pub perfect: bool,
    pub mods: u32,
    pub stars: f64,
    pub base_stars: f64,
    pub ar: f64,
    pub cs: f64,
    pub od: f64,
    pub hp: f64,
    pub bpm: f64,
    pub length: i64,
    pub objects: u32,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Digest {
    pub scores: Vec<LocalScore>,
    pub unknown: usize,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sent {
    pub kept: u32,
    pub through: i64,
    pub failed: bool,
}

pub fn digest(replays: &[Replay], library: &Library, after: i64) -> Digest {
    let mut scores = Vec::new();
    let mut unknown = 0;
    for replay in replays {
        let played = replay.played_at_unix();
        if played <= 0 || (after > 0 && played < after) {
            continue;
        }
        let mods = replay.mods.raw();
        let Some(map) = library.get(&replay.beatmap_hash).filter(|map| map.mode == 0 && map.id > 0) else {
            unknown += 1;
            continue;
        };
        let facts = map.facts(mods);
        scores.push(LocalScore {
            md5: replay.beatmap_hash.to_ascii_lowercase(),
            id: i64::from(map.id),
            set: i64::from(map.set),
            played,
            score: i64::from(replay.score),
            combo: u32::from(replay.max_combo),
            n300: u32::from(replay.hits.count_300),
            n100: u32::from(replay.hits.count_100),
            n50: u32::from(replay.hits.count_50),
            geki: u32::from(replay.hits.count_geki),
            katu: u32::from(replay.hits.count_katu),
            miss: u32::from(replay.hits.count_miss),
            perfect: replay.perfect_combo,
            mods,
            stars: f64::from(facts.stars),
            base_stars: f64::from(facts.base_stars),
            ar: f64::from(facts.ar),
            cs: f64::from(facts.cs),
            od: f64::from(facts.od),
            hp: f64::from(facts.hp),
            bpm: f64::from(facts.bpm),
            length: i64::from(facts.length),
            objects: facts.objects,
            status: facts.status.to_owned(),
        });
    }
    scores.sort_by_key(|score| (score.played, score.md5.clone()));
    Digest { scores, unknown }
}

pub fn gather(root: &Path, after: i64) -> Digest {
    let replays = crate::scores::in_folder(root);
    if replays.is_empty() {
        return Digest::default();
    }
    match Library::open(&root.join("osu!.db")) {
        Some(library) => digest(&replays, &library, after),
        None => Digest { scores: Vec::new(), unknown: replays.len() },
    }
}

pub fn tell(digest: &Digest, mut send: impl FnMut(&[LocalScore]) -> Option<u32>) -> Sent {
    let mut sent = Sent::default();
    for chunk in digest.scores.chunks(CHUNK) {
        match send(chunk) {
            Some(kept) => {
                sent.kept += kept;
                sent.through = chunk.last().map_or(sent.through, |last| last.played);
            }
            None => {
                sent.failed = true;
                break;
            }
        }
    }
    sent
}

#[cfg(test)]
mod tests {
    use super::*;
    use dossier_replay::{GameMode, HitCounts, Mods};
    use dossier_witness::forge::{entry_bytes, library_bytes, spec};

    const KNOWN: &str = "0123456789abcdef0123456789abcdef";
    const LOCAL: &str = "fedcba9876543210fedcba9876543210";
    const OTHER: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn library() -> Library {
        let mut unsubmitted = spec(LOCAL);
        unsubmitted.id = 0;
        let mut mania = spec(OTHER);
        mania.mode = 3;
        let version = 20_260_924;
        Library::read(&library_bytes(version, &[entry_bytes(version, &spec(KNOWN), 0), entry_bytes(version, &unsubmitted, 0), entry_bytes(version, &mania, 0)])).expect("a library")
    }

    fn score(hash: &str, seconds: i64, mods: u32) -> Replay {
        Replay {
            mode: GameMode::Standard,
            game_version: 20_260_924,
            beatmap_hash: hash.to_owned(),
            player: "NaumRedlo".to_owned(),
            replay_hash: "r".to_owned(),
            hits: HitCounts { count_300: 500, count_100: 20, count_50: 3, count_geki: 90, count_katu: 8, count_miss: 1 },
            score: 1_234_567,
            max_combo: 640,
            perfect_combo: false,
            mods: Mods::new(mods),
            life_bar: String::new(),
            timestamp_ticks: (seconds + 62_135_596_800) * 10_000_000,
            online_score_id: 0,
            target_practice_accuracy: None,
            frames: Vec::new(),
            rng_seed: None,
            score_info: None,
        }
    }

    #[test]
    fn a_local_score_is_given_the_difficulty_of_the_map_its_hash_names() {
        let digest = digest(&[score(KNOWN, 1_700_000_100, 64)], &library(), 0);
        assert_eq!(digest.unknown, 0);
        let one = &digest.scores[0];
        assert_eq!((one.id, one.set, one.played, one.score, one.combo, one.mods), (11, 5, 1_700_000_100, 1_234_567, 640, 64));
        assert_eq!((one.n300, one.n100, one.n50, one.miss, one.perfect), (500, 20, 3, 1, false));
        assert_eq!((one.stars, one.base_stars, one.ar, one.bpm, one.length, one.objects, one.status.as_str()), (7.5, 5.0, 9.0, 120.0, 120, 152, "ranked"));
    }

    #[test]
    fn what_cannot_be_tied_to_a_map_osu_knows_is_counted_and_left_out() {
        let digest = digest(&[score("0000000000000000000000000000000f", 1_700_000_001, 0), score(LOCAL, 1_700_000_002, 0), score(OTHER, 1_700_000_003, 0), score(KNOWN, 1_700_000_004, 0)], &library(), 0);
        assert_eq!((digest.scores.len(), digest.unknown), (1, 3), "a map not in the library, a map osu! has no id for and a map of another mode");
    }

    #[test]
    fn only_what_is_newer_than_what_was_sent_is_told_and_in_the_order_it_was_played() {
        let replays = [score(KNOWN, 300, 0), score(KNOWN, 100, 0), score(KNOWN, 200, 0)];
        let all = digest(&replays, &library(), 0);
        assert_eq!(all.scores.iter().map(|s| s.played).collect::<Vec<_>>(), [100, 200, 300]);
        let later = digest(&replays, &library(), 200);
        assert_eq!(later.scores.iter().map(|s| s.played).collect::<Vec<_>>(), [200, 300], "the second the last one went at is told again, the server keeps one");
        assert!(digest(&[score(KNOWN, 0, 0)], &library(), 0).scores.is_empty(), "a score with no time is not believed");
    }

    #[test]
    fn a_digest_goes_in_chunks_and_a_failure_keeps_what_got_through() {
        let replays: Vec<Replay> = (1..=(CHUNK as i64 * 2 + 10)).map(|at| score(KNOWN, at, 0)).collect();
        let all = digest(&replays, &library(), 0);
        let mut chunks = 0;
        let whole = tell(&all, |chunk| {
            chunks += 1;
            Some(chunk.len() as u32)
        });
        assert_eq!((chunks, whole.kept, whole.through, whole.failed), (3, CHUNK as u32 * 2 + 10, CHUNK as i64 * 2 + 10, false));
        let mut seen = 0;
        let broken = tell(&all, |chunk| {
            seen += 1;
            (seen < 3).then_some(chunk.len() as u32)
        });
        assert_eq!((broken.kept, broken.through, broken.failed), (CHUNK as u32 * 2, CHUNK as i64 * 2, true));
    }

    #[test]
    fn a_folder_without_scores_gives_nothing_and_one_without_a_library_says_what_it_could_not_resolve() {
        let root = std::env::temp_dir().join(format!("dossier-history-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a folder");
        assert_eq!(gather(&root, 0), Digest::default());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_score_tells_itself_in_the_words_the_bot_reads() {
        let digest = digest(&[score(KNOWN, 1_700_000_100, 8)], &library(), 0);
        let said = serde_json::to_value(&digest.scores[0]).expect("it says itself");
        for key in ["md5", "id", "set", "played", "score", "combo", "n300", "n100", "n50", "geki", "katu", "miss", "perfect", "mods", "stars", "base_stars", "ar", "cs", "od", "hp", "bpm", "length", "objects", "status"] {
            assert!(said.get(key).is_some(), "{key}");
        }
        assert_eq!(said["md5"], KNOWN);
    }
}
