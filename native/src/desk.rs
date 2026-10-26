use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::duels::{call, Account, Failure, Seat, Wins};

pub const REVIEWER: &str = "BALANCE_REVIEWER";
pub const OPERATOR: &str = "TECHNICAL_OPERATOR";
pub const PUBLISHER: &str = "MODEL_PUBLISHER";
pub const OWNER: &str = "OWNER";
pub const GRANTED: [&str; 3] = [REVIEWER, PUBLISHER, OPERATOR];
pub const SLOTS: [&str; 9] = ["NM1", "NM2", "NM3", "NM4", "HD2", "HR2", "DT1", "DT2", "TB"];
pub const BANDS: [f32; 8] = [4.0, 4.5, 5.0, 5.5, 6.0, 6.5, 7.0, 7.5];
pub const EXCLUDED: &str = "NONE";
pub const OTHER: &str = "OTHER";

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Presence {
    pub player: i64,
    pub name: String,
    pub section: String,
    pub beatmap: Option<i64>,
    pub mine: bool,
}

pub fn presence(account: &Account, section: Option<&str>, beatmap: Option<i64>) -> Result<Vec<Presence>, Failure> {
    #[derive(Deserialize)]
    struct Reply { people: Vec<Presence> }
    let found: Reply = call(account, reqwest::Method::POST, "/render/admin/duels/presence", Some(json!({"section": section, "beatmap": beatmap})), None)?;
    Ok(found.people)
}

pub fn inspect(account: &Account, beatmap: i64) -> Result<Task, Failure> {
    call(account, reqwest::Method::GET, &format!("/render/admin/duels/maps/{beatmap}/inspect"), None, None)
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Odds {
    pub slot: String,
    pub probability: f32,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Reason {
    pub feature: String,
    pub weight: f32,
    pub level: f32,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Guess {
    pub slot: Option<String>,
    pub probability: f32,
    pub margin: f32,
    pub top: Vec<Odds>,
    pub reasons: Vec<Reason>,
    pub outside: bool,
    pub version: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Map {
    pub beatmap_id: i64,
    pub beatmapset_id: Option<i64>,
    pub mapper: Option<String>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub version: Option<String>,
    pub stars: Option<f32>,
    pub model: Option<Guess>,
    pub settings: std::collections::BTreeMap<String, f32>,
    pub length: Option<f32>,
    pub plain_stars: Option<f32>,
    pub bpm: Option<f32>,
    pub mods: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Opinions {
    pub played: u32,
    pub fits: u32,
    pub off: u32,
    pub difficulty: BTreeMap<String, u32>,
    pub comments: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Task {
    pub kind: String,
    pub id: Option<i64>,
    pub reason: String,
    pub since: Option<String>,
    pub slot: Option<String>,
    pub state: Option<String>,
    pub map: Map,
    pub opinions: Option<Opinions>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Totals {
    pub playable: u32,
    pub human: u32,
    pub uncertain: u32,
    pub quarantined: u32,
    pub unassigned: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Coverage {
    pub slots: Vec<String>,
    pub grid: BTreeMap<String, Vec<u32>>,
    pub totals: Totals,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Held {
    pub beatmap_id: i64,
    pub beatmapset_id: Option<i64>,
    pub stars: f32,
    pub level: String,
    pub mapper: Option<String>,
    pub title: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Stuck {
    pub id: String,
    pub state: String,
    pub reason: String,
    pub since: Option<String>,
    pub wins: Wins,
    pub mode: String,
    pub players: Vec<Seat>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Entry {
    pub id: i64,
    pub actor: i64,
    pub name: Option<String>,
    pub role: String,
    pub action: String,
    pub target: String,
    pub reason: String,
    pub at: Option<String>,
    pub after: Value,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Person {
    pub player: i64,
    pub name: Option<String>,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Version {
    pub version: String,
    pub state: String,
    pub labels: u32,
    pub report: Value,
    pub created_at: Option<String>,
    pub published_at: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct CorpusMap {
    pub beatmap_id: i64,
    pub beatmapset_id: Option<i64>,
    pub label: String,
    pub verified: bool,
    pub file: bool,
    pub conflict: bool,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub version: Option<String>,
    pub mapper: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Cell {
    pub corpus: String,
    pub year: i32,
    pub stage: String,
    pub slot: String,
    pub label: Option<String>,
    pub maps: Vec<CorpusMap>,
}

impl Cell {
    pub fn shown(&self) -> String {
        if let Some(label) = &self.label {
            return label.clone();
        }
        let mut counted: BTreeMap<&str, usize> = BTreeMap::new();
        for map in &self.maps {
            *counted.entry(map.label.as_str()).or_default() += 1;
        }
        counted.into_iter().max_by_key(|(_, count)| *count).map(|(label, _)| label.to_owned()).unwrap_or_else(|| EXCLUDED.to_owned())
    }

    pub fn checked(&self) -> bool {
        self.label.is_some() || (!self.maps.is_empty() && self.maps.iter().all(|map| map.verified))
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Desk {
    pub tasks: Vec<Task>,
    pub coverage: Option<Coverage>,
    pub stuck: Vec<Stuck>,
    pub journal: Vec<Entry>,
    pub people: Vec<Person>,
    pub versions: Vec<Version>,
    pub cells: Vec<Cell>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct About {
    pub set: Option<i64>,
    pub title: String,
    pub artist: String,
    pub version: String,
    pub mapper: String,
    pub stars: Option<f32>,
    pub bpm: Option<f32>,
    pub seconds: Option<u32>,
}

pub fn about_address(beatmap: i64) -> String {
    format!("https://catboy.best/api/v2/b/{beatmap}")
}

pub fn read_about(found: &Value) -> Option<About> {
    let set = found.get("set").or_else(|| found.get("beatmapset"))?;
    let text = |value: Option<&Value>| value.and_then(Value::as_str).unwrap_or_default().to_owned();
    let mapper = found.get("owners").and_then(Value::as_array).and_then(|owners| owners.first()).and_then(|owner| owner.get("username")).and_then(Value::as_str).map(str::to_owned).unwrap_or_else(|| text(set.get("creator")));
    Some(About {
        set: found.get("beatmapset_id").and_then(Value::as_i64).or_else(|| set.get("id").and_then(Value::as_i64)),
        title: text(set.get("title")),
        artist: text(set.get("artist")),
        version: text(found.get("version")),
        mapper,
        stars: found.get("difficulty_rating").and_then(Value::as_f64).map(|value| value as f32),
        bpm: found.get("bpm").and_then(Value::as_f64).map(|value| value as f32),
        seconds: found.get("total_length").and_then(Value::as_u64).map(|value| value as u32),
    })
}

pub fn about(beatmap: i64) -> Option<About> {
    let client = crate::bot::client().ok()?;
    let response = client.get(about_address(beatmap)).header("User-Agent", "Dossier").send().ok()?;
    if !response.status().is_success() {
        return None;
    }
    read_about(&response.json::<Value>().ok()?)
}

pub fn archive_lines(path: &std::path::Path) -> Vec<String> {
    let Ok(file) = std::fs::File::open(path) else { return Vec::new() };
    let Ok(mut bundle) = zip::ZipArchive::new(file) else { return Vec::new() };
    let mut found = Vec::new();
    for at in 0..bundle.len() {
        let Ok(mut entry) = bundle.by_index(at) else { continue };
        if !entry.name().to_lowercase().ends_with(".osu") || entry.size() > 8 * 1024 * 1024 {
            continue;
        }
        let mut raw = String::new();
        if std::io::Read::read_to_string(&mut entry, &mut raw).is_err() {
            continue;
        }
        let field = |key: &str| raw.lines().find_map(|line| line.strip_prefix(key)).map(|value| value.trim().to_owned());
        if let Some(beatmap) = field("BeatmapID:").and_then(|value| value.parse::<i64>().ok()).filter(|id| *id > 0) {
            found.push(format!("https://osu.ppy.sh/b/{beatmap}"));
        }
    }
    found.sort();
    found
}

pub fn allows(roles: &[String], role: &str) -> bool {
    roles.iter().any(|held| held == role || held == OWNER)
}

pub fn band_of(stars: f32) -> usize {
    BANDS.iter().rposition(|edge| stars >= *edge).unwrap_or(0)
}

fn listed<T: serde::de::DeserializeOwned>(account: &Account, path: &str, key: &str) -> Result<Vec<T>, Failure> {
    let found: Value = call(account, reqwest::Method::GET, path, None, None)?;
    let inner = if key.is_empty() { found } else { found.get(key).cloned().unwrap_or(Value::Array(Vec::new())) };
    serde_json::from_value(inner).map_err(|why| Failure::Network(why.to_string()))
}

pub fn load(account: &Account, roles: &[String]) -> Result<Desk, Failure> {
    let mut desk = Desk::default();
    if allows(roles, REVIEWER) {
        desk.tasks = listed(account, "/render/admin/duels/queue", "tasks")?;
        desk.coverage = Some(call(account, reqwest::Method::GET, "/render/admin/duels/catalog", None, None)?);
        desk.cells = listed(account, "/render/admin/duels/corpus", "cells")?;
    }
    if allows(roles, REVIEWER) || allows(roles, PUBLISHER) {
        desk.versions = listed(account, "/render/admin/duels/models", "models")?;
    }
    if allows(roles, OPERATOR) {
        desk.stuck = listed(account, "/render/admin/duels/stuck", "duels")?;
    }
    desk.journal = listed(account, "/render/admin/duels/journal", "entries")?;
    desk.people = listed(account, "/render/admin/duels/people", "people")?;
    Ok(desk)
}

pub fn cell(account: &Account, slot: &str, band: usize) -> Result<Vec<Held>, Failure> {
    listed(account, &format!("/render/admin/duels/catalog/{slot}/{band}"), "maps")
}

pub fn review(account: &Account, beatmap: i64, decision: &str, slot: Option<&str>, reason: &str) -> Result<Value, Failure> {
    call(account, reqwest::Method::POST, &format!("/render/admin/duels/maps/{beatmap}/review"), Some(json!({ "decision": decision, "slot": slot, "reason": reason })), None)
}

pub fn label(account: &Account, cell: &Cell, beatmap: Option<i64>, label: &str, reason: &str) -> Result<Value, Failure> {
    let body = json!({ "corpus": cell.corpus, "year": cell.year, "stage": cell.stage, "slot": cell.slot, "beatmap_id": beatmap, "label": label, "reason": reason });
    call(account, reqwest::Method::POST, "/render/admin/duels/corpus/label", Some(body), None)
}

pub fn read_pool(text: &str) -> Vec<(String, i64)> {
    let mut found = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for line in text.lines() {
        let parts: Vec<_> = line.split_whitespace().collect();
        let Some(first) = parts.first() else { continue };
        let upper = first.to_uppercase();
        let labelled = SLOTS.contains(&upper.as_str()) || ["NM5", "NM6", "HD1", "HD3", "HR1", "HR3", "DT3", "DT4", "FM1", "FM2", "FM3", "FM4"].contains(&upper.as_str());
        let link = if labelled { parts.get(1).copied().unwrap_or_default() } else { first };
        if link.contains("beatmapsets/") && !link.contains("#osu/") { continue; }
        let tail = link.rsplit(['/', '#']).find(|part| !part.is_empty()).unwrap_or_default();
        if let Ok(beatmap) = tail.parse::<i64>() {
            if beatmap > 0 && seen.insert(beatmap) {
                found.push((if labelled { upper } else { "PENDING".into() }, beatmap));
            }
        }
    }
    found
}

pub fn tournament(account: &Account, name: &str, year: i32, pool: &str, reason: &str) -> Result<Value, Failure> {
    let result: Value = call(account, reqwest::Method::POST, "/render/admin/duels/corpus/tournaments", Some(json!({ "name": name, "year": year, "pool": pool, "reason": reason })), None)?;
    batch_reply(result, read_pool(pool).len())
}

fn batch_reply(result: Value, expected: usize) -> Result<Value, Failure> {
    if expected == 0 || result.get("read").and_then(Value::as_u64) != Some(expected as u64) {
        return Err(Failure::Refused { code: "BATCH_UNSUPPORTED".into(), message: String::new(), retryable: false, duel: None });
    }
    Ok(result)
}

pub fn publish(account: &Account, version: &str, reason: &str) -> Result<Value, Failure> {
    call(account, reqwest::Method::POST, &format!("/render/admin/duels/models/{version}/publish"), Some(json!({ "reason": reason })), None)
}

pub fn rollback(account: &Account, reason: &str) -> Result<Value, Failure> {
    call(account, reqwest::Method::POST, "/render/admin/duels/models/rollback", Some(json!({ "reason": reason })), None)
}

pub fn finalize(account: &Account, duel: &str, match_id: i64, game_id: i64, reason: &str) -> Result<Value, Failure> {
    call(account, reqwest::Method::POST, &format!("/render/admin/duels/{duel}/finalize"), Some(json!({ "match_id": match_id, "game_id": game_id, "reason": reason })), None)
}

pub fn delete_model(account: &Account, version: &str, reason: &str) -> Result<Value, Failure> {
    call(account, reqwest::Method::POST, &format!("/render/admin/duels/models/{version}/delete"), Some(json!({ "reason": reason })), None)
}

pub fn train(account: &Account, reason: &str) -> Result<Value, Failure> {
    call(account, reqwest::Method::POST, "/render/admin/duels/models/train", Some(json!({ "reason": reason })), None)
}

pub fn discover(account: &Account, band: usize, reason: &str) -> Result<Value, Failure> {
    call(account, reqwest::Method::POST, "/render/admin/duels/catalog/discover", Some(json!({ "band": band, "reason": reason })), None)
}

pub fn cancel(account: &Account, duel: &str, reason: &str) -> Result<Value, Failure> {
    call(account, reqwest::Method::POST, &format!("/render/admin/duels/{duel}/cancel"), Some(json!({ "reason": reason })), None)
}

pub fn grant(account: &Account, who: &str, role: &str, reason: &str) -> Result<Value, Failure> {
    call(account, reqwest::Method::POST, "/render/admin/duels/roles", Some(json!({ "who": who, "role": role, "reason": reason })), None)
}

pub fn revoke(account: &Account, player: i64, role: &str, reason: &str) -> Result<Value, Failure> {
    call(account, reqwest::Method::POST, "/render/admin/duels/roles/revoke", Some(json!({ "player_id": player, "role": role, "reason": reason })), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_acknowledgement_must_account_for_every_input_map() {
        assert!(batch_reply(json!({"read": 0, "added": 0}), 2).is_err());
        assert!(batch_reply(json!({"read": 1, "added": 1}), 2).is_err());
        assert!(batch_reply(json!({"read": 2, "added": 0}), 2).is_ok());
        assert!(batch_reply(json!({"read": 2, "added": 2}), 2).is_ok());
    }

    #[test]
    fn a_map_is_described_by_the_mirror_and_never_by_osu() {
        assert!(about_address(5687673).starts_with("https://catboy.best/") && !about_address(1).contains("ppy.sh"));
        let found = serde_json::json!({ "beatmapset_id": 2300330, "difficulty_rating": 5.86, "bpm": 137.8, "total_length": 216, "version": "YO SAY!", "owners": [{ "username": "Pata-Mon" }], "set": { "title": "HOT LIMIT", "artist": "T.M.Revolution", "creator": "Wanpachi" } });
        let about = read_about(&found).expect("an about");
        assert_eq!((about.set, about.title.as_str(), about.mapper.as_str(), about.seconds), (Some(2300330), "HOT LIMIT", "Pata-Mon", Some(216)));
    }

    #[test]
    fn a_cell_shows_its_own_label_before_the_maps_and_bands_follow_the_bot() {
        let map = |label: &str, verified: bool| CorpusMap { label: label.into(), verified, ..CorpusMap::default() };
        let mut cell = Cell { slot: "NM3".into(), maps: vec![map("NM2", true), map("NM2", false), map("NM3", true)], ..Cell::default() };
        assert_eq!(cell.shown(), "NM2");
        assert!(!cell.checked());
        cell.label = Some("NM4".into());
        assert_eq!(cell.shown(), "NM4");
        assert!(cell.checked());
        assert_eq!((band_of(3.2), band_of(5.6), band_of(9.0)), (0, 3, 7));
        assert!(allows(&["OWNER".to_owned()], OPERATOR) && !allows(&[REVIEWER.to_owned()], OPERATOR));
    }
}
