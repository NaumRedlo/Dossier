use std::collections::HashMap;
use std::path::{Path, PathBuf};

use dossier_assay::performance::{performance, Score};

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Rating {
    pub stars: f64,
    pub pp: f64,
    pub max_combo: u32,
    #[serde(default)]
    pub pp_fc: f64,
    #[serde(default)]
    pub pp_ss: f64,
}

impl Rating {
    pub fn complete(&self) -> bool {
        self.pp_ss > 0.0
    }
}

fn worth(pp: f64) -> f64 {
    if pp.is_finite() { pp.max(0.0) } else { 0.0 }
}

pub fn rate(path: &Path, map: &crate::library::Map, hash: &str) -> Option<Rating> {
    let bytes = std::fs::read(path).ok()?;
    let replay = dossier_replay::Replay::parse(&bytes).ok()?;
    let found = dossier_produce::locate::load_map(&map.file, hash).ok()?;
    let beatmap = dossier_beatmap::Beatmap::parse(&found.text).ok()?;
    if beatmap.mode != 0 {
        return None;
    }
    let attributes = dossier_assay::attributes(&beatmap, replay.mods);
    let hits = replay.hits;
    let mut score = Score {
        max_combo: u32::from(replay.max_combo).min(attributes.max_combo),
        great: u32::from(hits.count_300),
        ok: u32::from(hits.count_100),
        meh: u32::from(hits.count_50),
        miss: u32::from(hits.count_miss),
        slider_tail_hit: attributes.slider_count,
        large_tick_miss: 0,
        classic: true,
        legacy_total_score: None,
        accuracy: None,
    };
    score.accuracy = Some(score.accuracy());
    let pp = performance(&score, &attributes, replay.mods).pp;
    let mut full = Score { great: score.great + score.miss, miss: 0, max_combo: attributes.max_combo, accuracy: None, ..score };
    full.accuracy = Some(full.accuracy());
    let pp_fc = performance(&full, &attributes, replay.mods).pp;
    let objects = score.great + score.ok + score.meh + score.miss;
    let mut perfect = Score { great: objects, ok: 0, meh: 0, miss: 0, max_combo: attributes.max_combo, accuracy: None, ..score };
    perfect.accuracy = Some(perfect.accuracy());
    let pp_ss = performance(&perfect, &attributes, replay.mods).pp;
    Some(Rating {
        stars: attributes.star_rating,
        pp: worth(pp),
        max_combo: attributes.max_combo,
        pp_fc: worth(pp_fc),
        pp_ss: worth(pp_ss),
    })
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Ratings {
    #[serde(default)]
    pub known: HashMap<String, Rating>,
}

impl Ratings {
    pub fn file() -> PathBuf {
        crate::sources::home().join(".dossier").join("ratings.json")
    }

    pub fn load(file: &Path) -> Ratings {
        std::fs::read_to_string(file).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
    }

    pub fn save(&self, file: &Path) {
        let Ok(text) = serde_json::to_string(self) else {
            return;
        };
        if let Some(folder) = file.parent() {
            let _ = std::fs::create_dir_all(folder);
        }
        let staged = file.with_extension("json.part");
        if std::fs::write(&staged, text).is_ok() && std::fs::rename(&staged, file).is_err() {
            let _ = std::fs::remove_file(&staged);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_cache_comes_back_and_a_broken_one_is_empty() {
        let folder = std::env::temp_dir().join(format!("dossier-ratings-{}", std::process::id()));
        let file = folder.join("ratings.json");
        let mut ratings = Ratings::default();
        ratings.known.insert("abc".to_owned(), Rating { stars: 6.24, pp: 274.5, max_combo: 1102, pp_fc: 301.0, pp_ss: 355.5 });
        ratings.save(&file);
        assert_eq!(Ratings::load(&file).known.get("abc"), Some(&Rating { stars: 6.24, pp: 274.5, max_combo: 1102, pp_fc: 301.0, pp_ss: 355.5 }));
        std::fs::write(&file, "not json").unwrap();
        assert!(Ratings::load(&file).known.is_empty());
        let _ = std::fs::remove_dir_all(folder);
    }
}
