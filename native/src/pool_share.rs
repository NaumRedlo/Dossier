use std::io::{Read, Write};

use crate::pools::{Frame, Mod, Pool, Slot, CALC_VERSION};

pub const PREFIX: &str = "pool1.";
const FORMAT: &str = "dossier-pool";
const VERSION: u32 = 1;
const BIGGEST: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    NotAPool,
    Newer,
    Damaged,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Shared {
    format: String,
    version: u32,
    name: String,
    #[serde(default)]
    note: String,
    frame: Frame,
    calc: u32,
    slots: Vec<SharedSlot>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SharedSlot {
    hash: Option<String>,
    mods: Mod,
    #[serde(default)]
    artist: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    set: Option<u64>,
    #[serde(default)]
    note: String,
}

fn shared(pool: &Pool) -> Shared {
    Shared {
        format: FORMAT.to_owned(),
        version: VERSION,
        name: pool.name.clone(),
        note: pool.note.clone(),
        frame: pool.frame,
        calc: pool.calc,
        slots: pool
            .slots
            .iter()
            .map(|slot| SharedSlot { hash: slot.hash.clone(), mods: slot.mods, artist: slot.artist.clone(), title: slot.title.clone(), version: slot.version.clone(), set: slot.set, note: slot.note.clone() })
            .collect(),
    }
}

fn pool_of(shared: Shared, now: i64) -> Result<Pool, Refused> {
    if shared.format != FORMAT {
        return Err(Refused::NotAPool);
    }
    if shared.version > VERSION {
        return Err(Refused::Newer);
    }
    let mut pool = Pool::new(shared.frame, &shared.name, now);
    pool.note = shared.note;
    pool.calc = CALC_VERSION;
    pool.slots = shared
        .slots
        .into_iter()
        .map(|slot| {
            let hash = slot.hash.map(|hash| hash.to_ascii_lowercase()).filter(|hash| hash.len() == 32 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()));
            Slot { hash, mods: slot.mods, artist: slot.artist, title: slot.title, version: slot.version, set: slot.set, note: slot.note, measure: None }
        })
        .collect();
    Ok(pool)
}

pub fn to_file(pool: &Pool) -> Vec<u8> {
    serde_json::to_vec_pretty(&shared(pool)).unwrap_or_default()
}

pub fn from_file(bytes: &[u8], now: i64) -> Result<Pool, Refused> {
    if bytes.len() > BIGGEST {
        return Err(Refused::NotAPool);
    }
    let shared: Shared = serde_json::from_slice(bytes).map_err(|_| Refused::NotAPool)?;
    pool_of(shared, now)
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 4 / 3 + 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16) | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8) | u32::from(*chunk.get(2).unwrap_or(&0));
        let symbols = chunk.len() + 1;
        for at in 0..symbols {
            out.push(ALPHABET[((n >> (18 - 6 * at)) & 63) as usize] as char);
        }
    }
    out
}

pub fn decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let mut bits = 0u32;
    let mut held = 0u32;
    for symbol in text.bytes() {
        let value = ALPHABET.iter().position(|candidate| *candidate == symbol)? as u32;
        bits = (bits << 6) | value;
        held += 6;
        if held >= 8 {
            held -= 8;
            out.push(((bits >> held) & 0xff) as u8);
        }
    }
    (held < 6).then_some(out)
}

pub fn to_text(pool: &Pool) -> String {
    let json = serde_json::to_vec(&shared(pool)).unwrap_or_default();
    let mut packer = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
    let _ = packer.write_all(&json);
    let packed = packer.finish().unwrap_or_default();
    format!("{PREFIX}{}", encode(&packed))
}

pub fn is_text(text: &str) -> bool {
    text.trim().starts_with(PREFIX)
}

pub fn from_text(text: &str, now: i64) -> Result<Pool, Refused> {
    let body = text.trim().strip_prefix(PREFIX).ok_or(Refused::NotAPool)?;
    let packed = decode(body).ok_or(Refused::Damaged)?;
    let mut json = Vec::new();
    flate2::read::ZlibDecoder::new(packed.as_slice()).take(BIGGEST as u64 + 1).read_to_end(&mut json).map_err(|_| Refused::Damaged)?;
    if json.len() > BIGGEST {
        return Err(Refused::NotAPool);
    }
    let shared: Shared = serde_json::from_slice(&json).map_err(|_| Refused::Damaged)?;
    pool_of(shared, now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pools::Measure;

    fn sample() -> Pool {
        let mut pool = Pool::new(Frame::Duel, "Spring duel", 1_790_000_000);
        pool.note = "for the friendly cup".into();
        for (at, hash) in ["6d1a1ffcff39147ffa907a0683514135", "2840EE0A5A7DE95B74A40FA282A0C9FE"].into_iter().enumerate() {
            pool.slots[at].hash = Some(hash.to_owned());
            pool.slots[at].artist = "Batushka".into();
            pool.slots[at].title = "Pesn' 8".into();
            pool.slots[at].version = format!("Diff {at}");
            pool.slots[at].set = Some(2370103);
            pool.slots[at].note = "watch the end".into();
            pool.slots[at].measure = Some(Measure { stars: 6.2, bpm: 180.0, length_ms: 336_000, ar: 9.0, od: 8.0, cs: 4.0, hp: 5.0, max_combo: 1500, aim: 3.0, speed: 3.0, reading: 1.0, stamina: 1.5 });
        }
        pool.slots[1].mods = Mod::Dt;
        pool
    }

    #[test]
    fn base64_round_trips_every_length() {
        for length in 0..40usize {
            let bytes: Vec<u8> = (0..length).map(|at| (at * 37 + 11) as u8).collect();
            assert_eq!(decode(&encode(&bytes)), Some(bytes), "{length}");
        }
        assert_eq!(encode(b"Man"), "TWFu");
        assert_eq!(encode(b"Ma"), "TWE");
        assert_eq!(decode("TW"), Some(b"M".to_vec()));
        assert_eq!(decode("T W"), None, "a space is not a symbol");
        assert_eq!(decode("T"), None, "one symbol is not a whole byte");
    }

    #[test]
    fn a_string_brings_the_pool_back_without_the_numbers_and_with_a_new_id() {
        let pool = sample();
        let text = to_text(&pool);
        assert!(text.starts_with("pool1.") && is_text(&text));
        assert!(text.len() < 1500, "{}", text.len());
        let back = from_text(&format!("  {text}\n"), 1_790_000_500).unwrap();
        assert_ne!(back.id, pool.id);
        assert_eq!((back.name.as_str(), back.note.as_str(), back.frame), ("Spring duel", "for the friendly cup", Frame::Duel));
        assert_eq!(back.slots.len(), 7);
        assert_eq!(back.slots[0].hash.as_deref(), Some("6d1a1ffcff39147ffa907a0683514135"));
        assert_eq!(back.slots[1].hash.as_deref(), Some("2840ee0a5a7de95b74a40fa282a0c9fe"), "a hash is kept in lower case");
        assert_eq!(back.slots[1].mods, Mod::Dt);
        assert_eq!((back.slots[0].artist.as_str(), back.slots[0].note.as_str(), back.slots[0].set), ("Batushka", "watch the end", Some(2370103)));
        assert!(back.slots.iter().all(|slot| slot.measure.is_none()), "numbers are measured again by whoever opens it");
        assert!(back.slots[2].is_empty());
        assert_eq!(back.made_at, 1_790_000_500);
        assert_eq!(back.fingerprint(), pool.fingerprint(), "the same pool has the same fingerprint");
    }

    #[test]
    fn a_file_brings_the_pool_back_the_same_way() {
        let pool = sample();
        let bytes = to_file(&pool);
        assert!(String::from_utf8(bytes.clone()).unwrap().contains("\"dossier-pool\""));
        let back = from_file(&bytes, 5).unwrap();
        assert_eq!(back.fingerprint(), pool.fingerprint());
        assert_eq!(back.slots[0].version, "Diff 0");
    }

    #[test]
    fn something_else_is_not_a_pool_and_a_cut_string_is_damaged() {
        assert_eq!(from_file(b"{\"hello\":1}", 1), Err(Refused::NotAPool));
        assert_eq!(from_file(b"not json", 1), Err(Refused::NotAPool));
        assert_eq!(from_file(&vec![b' '; BIGGEST + 1], 1), Err(Refused::NotAPool));
        assert_eq!(from_text("hello", 1), Err(Refused::NotAPool));
        let text = to_text(&sample());
        assert_eq!(from_text(&text[..text.len() / 2], 1), Err(Refused::Damaged));
        assert_eq!(from_text("pool1.!!!", 1), Err(Refused::Damaged));
    }

    #[test]
    fn a_file_from_a_newer_version_is_refused_as_newer_and_bad_hashes_are_dropped() {
        let mut newer = String::from_utf8(to_file(&sample())).unwrap();
        newer = newer.replace("\"version\": 1,", "\"version\": 9,");
        assert_eq!(from_file(newer.as_bytes(), 1), Err(Refused::Newer));
        let broken = String::from_utf8(to_file(&sample())).unwrap().replace("6d1a1ffcff39147ffa907a0683514135", "short");
        let back = from_file(broken.as_bytes(), 1).unwrap();
        assert!(back.slots[0].hash.is_none(), "a hash that is not an md5 leaves the slot empty");
    }
}
