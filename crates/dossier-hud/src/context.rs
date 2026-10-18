use crate::{Card, Context, Day, Lang};
use dossier_overlay::View;
use serde::{Deserialize, Serialize};
use std::{
    io::{self, Read},
    path::Path,
};

pub const MAX_BYTES: usize = 64 * 1024;
pub const TTL_MS: u64 = 15_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Packet {
    pub version: u32,
    pub pid: u32,
    pub at_ms: u64,
    pub lang: Lang,
    pub map_md5: Option<String>,
    pub card: Option<Card>,
    pub day: Option<Day>,
}

impl Packet {
    pub fn context(&self, view: &View, now_ms: u64) -> Option<Context> {
        if self.version != 1
            || self.pid != view.pid
            || self.at_ms > now_ms
            || now_ms - self.at_ms >= TTL_MS
        {
            return None;
        }
        let matches = self
            .map_md5
            .as_ref()
            .zip(view.snapshot.beatmap.as_ref())
            .is_some_and(|(hash, map)| !hash.is_empty() && hash.eq_ignore_ascii_case(&map.md5));
        Some(Context {
            lang: self.lang,
            day: self.day.clone(),
            card: if matches { self.card.clone() } else { None },
            ..Context::default()
        })
    }
}

pub fn read(path: &Path) -> io::Result<Packet> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "overlay context too large",
        ));
    }
    serde_json::from_slice(&bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

pub fn write(path: &Path, packet: &Packet) -> io::Result<()> {
    let bytes = serde_json::to_vec(packet)?;
    if bytes.len() > MAX_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "overlay context too large",
        ));
    }
    let temporary = path.with_extension("pending");
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(temporary, path)
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_transport_round_trips_and_rejects_partial_or_oversized_input() {
        let path = std::env::temp_dir().join(format!(
            "dossier-context-{}-{}.json",
            std::process::id(),
            now_ms()
        ));
        let packet = Packet {
            version: 1,
            pid: 7,
            at_ms: 1000,
            lang: Lang::Ru,
            map_md5: None,
            card: None,
            day: None,
        };
        write(&path, &packet).unwrap();
        assert_eq!(read(&path).unwrap(), packet);
        let mut next = packet.clone();
        next.pid = 8;
        write(&path, &next).unwrap();
        assert_eq!(read(&path).unwrap(), next);
        std::fs::write(&path, b"{").unwrap();
        assert!(read(&path).is_err());
        std::fs::write(&path, vec![b' '; MAX_BYTES + 1]).unwrap();
        assert!(read(&path).is_err());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn expired_foreign_and_future_context_is_ignored_and_cards_follow_the_map() {
        let (_, mut view, _) = crate::sample::frames(Lang::En).remove(1);
        view.snapshot.beatmap = Some(dossier_overlay::Beatmap {
            md5: "a".repeat(32),
            title: String::new(),
            artist: String::new(),
            difficulty: String::new(),
        });
        let mut packet = Packet {
            version: 1,
            pid: view.pid,
            at_ms: 1000,
            lang: Lang::En,
            map_md5: Some("a".repeat(32)),
            card: Some(Card {
                pool: Some("Cup".into()),
                ..Card::default()
            }),
            day: None,
        };
        assert!(packet.context(&view, 1001).unwrap().card.is_some());
        packet.map_md5 = Some("b".repeat(32));
        assert!(packet.context(&view, 1001).unwrap().card.is_none());
        assert!(packet.context(&view, 1000 + TTL_MS).is_none());
        assert!(packet.context(&view, 999).is_none());
        packet.pid += 1;
        assert!(packet.context(&view, 1001).is_none());
    }
}
