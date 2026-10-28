use std::collections::BTreeMap;
use std::io::Read;

use super::About;

const FILE_LIMIT: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Details {
    pub about: About,
    pub stars: BTreeMap<String, f32>,
}

impl Details {
    pub fn fill(&self, about: &mut About) {
        for (to, from) in [(&mut about.title, &self.about.title), (&mut about.artist, &self.about.artist),
            (&mut about.version, &self.about.version), (&mut about.mapper, &self.about.mapper)] {
            if to.trim().is_empty() { *to = from.clone(); }
        }
        about.set = about.set.or(self.about.set);
        about.stars = about.stars.or(self.about.stars);
        about.standard = self.about.standard;
    }
}

fn valid(text: &str, beatmap: i64, revision: &str) -> Option<dossier_beatmap::Beatmap> {
    if text.len() as u64 > FILE_LIMIT { return None; }
    if !revision.is_empty() && dossier_produce::locate::md5_hex(text.as_bytes()) != revision { return None; }
    let parsed = dossier_beatmap::Beatmap::parse(text).ok()?;
    (parsed.metadata.beatmap_id == Some(beatmap) && parsed.mode == 0 && parsed.objects.len() <= 20_000).then_some(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAP: &str = include_str!("../../../crates/dossier-assay/corpus/maps/1355247.osu");

    #[test]
    fn details_use_the_matching_file_and_calculate_each_mod_instead_of_reusing_nm() {
        let hash = dossier_produce::locate::md5_hex(MAP.as_bytes());
        let details = read(MAP, 1355247, &hash).unwrap();
        assert!(!details.about.title.is_empty() && !details.about.version.is_empty());
        assert!(details.stars["DT"] > details.stars["NM"]);
        assert_ne!(details.stars["HR"], details.stars["NM"]);
        assert!(read(MAP, 1, &hash).is_none());
        assert!(read(MAP, 1355247, "00000000000000000000000000000000").is_none());
        let mut about = About { status: "graveyard".into(), ..Default::default() };
        details.fill(&mut about);
        assert_eq!(about.title, details.about.title);
        assert_eq!(about.status, "graveyard");
    }

    #[test]
    #[ignore = "asks the real mirrors"]
    fn a_linked_map_can_be_measured_without_osu_api_attributes() {
        let found = load(4132505, "").expect("a mirror supplies the file");
        assert_eq!(found.about.title, "AugoEidEs");
        assert_eq!(found.about.version, "Distant light");
        assert!(found.stars["DT"] > found.stars["NM"]);
    }
}

pub fn read(text: &str, beatmap: i64, revision: &str) -> Option<Details> {
    let parsed = valid(text, beatmap, revision)?;
    let stars: BTreeMap<_, _> = [crate::pools::Mod::Nm, crate::pools::Mod::Hd, crate::pools::Mod::Hr, crate::pools::Mod::Dt].into_iter()
        .filter_map(|mods| {
            let stars = dossier_assay::attributes(&parsed, mods.applied()).star_rating;
            stars.is_finite().then(|| (mods.code().to_owned(), stars as f32))
        }).collect();
    Some(Details {
        about: About { set: parsed.metadata.beatmapset_id, title: parsed.metadata.title, artist: parsed.metadata.artist,
            version: parsed.metadata.version, mapper: parsed.metadata.creator, stars: stars.get("NM").copied(), standard: true, ..Default::default() },
        stars,
    })
}

pub fn load(beatmap: i64, revision: &str) -> Option<Details> {
    if !revision.is_empty() && (revision.len() != 32 || !revision.bytes().all(|b| b.is_ascii_hexdigit())) { return None; }
    let folder = crate::sources::own_root().join("cache").join("balance-maps");
    let path = folder.join(format!("{beatmap}-{}.osu", if revision.is_empty() { "latest" } else { revision }));
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Some(found) = read(&text, beatmap, revision) { return Some(found); }
    }
    let client = crate::bot::client().ok()?;
    for address in [format!("https://osu.direct/api/osu/{beatmap}"), format!("https://catboy.best/osu/{beatmap}")] {
        let Ok(response) = client.get(address).header("User-Agent", super::MIRROR_AGENT).send() else { continue };
        if !response.status().is_success() { continue; }
        let mut text = String::new();
        if response.take(FILE_LIMIT + 1).read_to_string(&mut text).is_err() { continue; }
        let Some(found) = read(&text, beatmap, revision) else { continue };
        if std::fs::create_dir_all(&folder).is_ok() { let _ = std::fs::write(&path, text); }
        return Some(found);
    }
    None
}
