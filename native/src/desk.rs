use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::duels::{call, call_within, Account, Failure, Seat, Wins};

pub mod details;

pub const REVIEWER: &str = "BALANCE_REVIEWER";
pub const OPERATOR: &str = "TECHNICAL_OPERATOR";
pub const PUBLISHER: &str = "MODEL_PUBLISHER";
pub const OWNER: &str = "OWNER";
pub const GRANTED: [&str; 3] = [REVIEWER, PUBLISHER, OPERATOR];
pub const SLOTS: [&str; 9] = ["NM1", "NM2", "NM3", "NM4", "HD2", "HR2", "DT1", "DT2", "TB"];
pub const RESERVE: [&str; 13] = ["NM5", "NM6", "HD1", "HD3", "HD4", "HR1", "HR3", "HR4", "DT3", "DT4", "FM1", "FM2", "FM3"];
const READ_PATIENCE: std::time::Duration = std::time::Duration::from_secs(45);
pub const TIERS: [&str; 3] = ["exemplary", "fits", "borderline"];
pub const FITS: &str = "fits";
pub const BORDERLINE: &str = "borderline";
pub const MARKS: [&str; 10] = ["jumps", "turns", "streams", "bursts", "triplets", "rhythm", "sliders", "low_ar", "small_circles", "stamina"];
pub const MOST_MARKS: usize = 3;
pub const BANDS: [f32; 10] = [5.0, 5.5, 6.0, 6.5, 7.0, 7.5, 8.0, 8.5, 9.0, 9.5];
pub const TOP: f32 = 10.0;
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
    pub slot: Option<String>,
    pub band: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct PresenceUpdate {
    pub people: Vec<Presence>,
    pub revision: Option<String>,
    pub training_known: bool,
    pub training: Option<Job>,
}

fn read<T: serde::de::DeserializeOwned>(account: &Account, path: &str) -> Result<T, Failure> {
    call_within(account, reqwest::Method::GET, path, None, None, Some(READ_PATIENCE))
}

pub fn presence(account: &Account, section: Option<&str>, beatmap: Option<i64>) -> Result<PresenceUpdate, Failure> {
    let mut parts = section.unwrap_or_default().split(':');
    let page = parts.next().filter(|_| section.is_some());
    let slot = parts.next();
    let band = parts.next().and_then(|value| value.parse::<usize>().ok());
    call(account, reqwest::Method::POST, "/render/admin/duels/presence", Some(json!({"section": page, "beatmap": beatmap, "slot": slot, "band": band})), None)
}

pub fn inspect(account: &Account, beatmap: i64) -> Result<Task, Failure> {
    read(account, &format!("/render/admin/duels/maps/{beatmap}/inspect"))
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
    pub level: Option<f32>,
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
    pub revision: Option<String>,
    pub ranked_status: Option<String>,
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
    pub secondary: Option<String>,
    pub tier: Option<String>,
    pub reasons: Vec<String>,
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
    pub secondary: BTreeMap<String, Vec<u32>>,
    pub totals: Totals,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Held {
    pub reviewer: Option<String>,
    pub beatmap_id: i64,
    pub beatmapset_id: Option<i64>,
    pub stars: f32,
    pub level: String,
    pub tier: Option<String>,
    pub typical: Option<f32>,
    pub mapper: Option<String>,
    pub title: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct StuckRoom {
    pub match_id: Option<i64>,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct StuckRound {
    pub ordinal: u32,
    pub slot: String,
    pub state: String,
    pub beatmap_id: Option<i64>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Happened {
    #[serde(rename = "type")]
    pub kind: String,
    pub reason: Option<String>,
    pub at: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Due {
    pub kind: String,
    pub at: Option<String>,
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
    pub room: Option<StuckRoom>,
    pub round: Option<StuckRound>,
    pub events: Vec<Happened>,
    pub deadline: Option<Due>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Entry {
    pub id: i64,
    pub actor: i64,
    pub name: Option<String>,
    pub osu_id: Option<i64>,
    pub role: String,
    pub action: String,
    pub target: String,
    pub reason: String,
    pub at: Option<String>,
    pub before: Value,
    pub after: Value,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Person {
    pub player: i64,
    pub name: Option<String>,
    pub osu_id: Option<i64>,
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
    pub excluded: bool,
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
    pub batches: Vec<Batch>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Author {
    pub player: i64,
    pub name: Option<String>,
    pub osu_id: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Batch {
    pub name: String,
    pub year: i32,
    pub maps: u32,
    pub labelled: u32,
    pub excluded: u32,
    pub author: Option<Author>,
    pub added_at: Option<String>,
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
    pub status: String,
    pub standard: bool,
}

pub const MIRROR_AGENT: &str = concat!("Dossier/", env!("CARGO_PKG_VERSION"), " (+https://github.com/NaumRedlo/Dossier)");

pub fn about_address(beatmap: i64) -> String {
    format!("https://catboy.best/api/v2/b/{beatmap}")
}

pub fn spare_address(beatmap: i64) -> String {
    format!("https://osu.direct/api/v2/b/{beatmap}")
}

pub fn spare_set_address(set: i64) -> String {
    format!("https://osu.direct/api/v2/s/{set}")
}

pub fn looked_up(beatmap: i64) -> Option<Value> {
    let client = crate::bot::client().ok()?;
    let get = |address: String| -> Option<Value> {
        let response = client.get(address).header("User-Agent", MIRROR_AGENT).send().ok()?;
        if !response.status().is_success() {
            return None;
        }
        response.json::<Value>().ok()
    };
    let ours = |found: &Value| found.get("id").and_then(Value::as_i64) == Some(beatmap);
    let mut found = get(about_address(beatmap)).filter(ours).or_else(|| get(spare_address(beatmap)).filter(ours))?;
    if read_about(&found).is_none_or(|about| about.title.trim().is_empty()) {
        if let Some(set) = found.get("beatmapset_id").and_then(Value::as_i64).and_then(|set| get(spare_set_address(set)).or_else(|| get(format!("https://catboy.best/api/v2/s/{set}")))) {
            found["set"] = set;
        }
    }
    Some(found)
}

pub fn read_about(found: &Value) -> Option<About> {
    found.get("id").or_else(|| found.get("set")).or_else(|| found.get("beatmapset"))?;
    let none = Value::Object(serde_json::Map::new());
    let set = found.get("set").filter(|value| value.is_object()).or_else(|| found.get("beatmapset").filter(|value| value.is_object())).unwrap_or(&none);
    let text = |value: Option<&Value>| value.and_then(Value::as_str).unwrap_or_default().to_owned();
    let named = |key: &str| [set.get(key), found.get(key)].into_iter().flatten().filter_map(Value::as_str).find(|s| !s.trim().is_empty()).unwrap_or_default().to_owned();
    let mapper = found.get("owners").and_then(Value::as_array).and_then(|owners| owners.first()).and_then(|owner| owner.get("username")).and_then(Value::as_str).map(str::to_owned).unwrap_or_else(|| text(set.get("creator")));
    Some(About {
        set: found.get("beatmapset_id").and_then(Value::as_i64).or_else(|| set.get("id").and_then(Value::as_i64)),
        title: named("title"),
        artist: named("artist"),
        version: text(found.get("version")),
        mapper,
        stars: found.get("difficulty_rating").and_then(Value::as_f64).map(|value| value as f32),
        bpm: found.get("bpm").and_then(Value::as_f64).map(|value| value as f32),
        seconds: found.get("total_length").and_then(Value::as_u64).map(|value| value as u32),
        status: found.get("status").and_then(Value::as_str).or_else(|| set.get("status").and_then(Value::as_str)).unwrap_or_default().to_owned(),
        standard: found.get("mode").is_none_or(|mode| mode == "osu" || mode == 0) && found.get("convert").and_then(Value::as_bool) != Some(true),
    })
}

pub fn about(beatmap: i64) -> Option<About> {
    let mut found = looked_up(beatmap).and_then(|value| read_about(&value));
    if found.as_ref().is_none_or(|about| about.title.trim().is_empty()) {
        if let Some(details) = details::load(beatmap, "") {
            let about = found.get_or_insert_with(About::default);
            details.fill(about);
        }
    }
    found
}

#[derive(Debug, Clone, PartialEq)]
pub struct Packed {
    pub beatmap: i64,
    pub version: String,
    pub stars: Option<f32>,
    pub standard: bool,
}

pub fn read_packed(raw: &str) -> Option<Packed> {
    let field = |key: &str| raw.lines().find_map(|line| line.strip_prefix(key)).map(|value| value.trim().to_owned());
    let beatmap = field("BeatmapID:").and_then(|value| value.parse::<i64>().ok()).filter(|id| *id > 0)?;
    let standard = field("Mode:").is_none_or(|mode| mode == "0");
    let stars = standard.then(|| crate::pools::measure_text(raw, crate::pools::Mod::Nm).ok()).flatten().map(|measure| measure.stars as f32);
    Some(Packed { beatmap, version: field("Version:").unwrap_or_default(), stars, standard })
}

pub fn archive_maps(path: &std::path::Path) -> Vec<Packed> {
    let Ok(file) = std::fs::File::open(path) else { return Vec::new() };
    let Ok(mut bundle) = zip::ZipArchive::new(file) else { return Vec::new() };
    let mut found: Vec<Packed> = Vec::new();
    for at in 0..bundle.len() {
        let Ok(mut entry) = bundle.by_index(at) else { continue };
        if !entry.name().to_lowercase().ends_with(".osu") || entry.size() > 8 * 1024 * 1024 {
            continue;
        }
        let mut raw = String::new();
        if std::io::Read::read_to_string(&mut entry, &mut raw).is_err() {
            continue;
        }
        if let Some(packed) = read_packed(&raw).filter(|packed| found.iter().all(|held| held.beatmap != packed.beatmap)) {
            found.push(packed);
        }
    }
    found.sort_by(|a, b| b.standard.cmp(&a.standard).then(b.stars.unwrap_or(-1.0).total_cmp(&a.stars.unwrap_or(-1.0))));
    found
}

pub fn allows(roles: &[String], role: &str) -> bool {
    roles.iter().any(|held| held == role || held == OWNER)
}

pub fn band_of(stars: f32) -> usize {
    BANDS.iter().rposition(|edge| stars >= *edge).unwrap_or(0)
}

fn listed<T: serde::de::DeserializeOwned>(account: &Account, path: &str, key: &str) -> Result<Vec<T>, Failure> {
    let found: Value = read(account, path)?;
    let inner = if key.is_empty() { found } else { found.get(key).cloned().unwrap_or(Value::Array(Vec::new())) };
    serde_json::from_value(inner).map_err(|why| Failure::Network(why.to_string()))
}

pub fn load(account: &Account, roles: &[String]) -> Result<Desk, Failure> {
    let mut desk = Desk::default();
    if allows(roles, REVIEWER) {
        desk.tasks = listed(account, "/render/admin/duels/queue", "tasks")?;
        desk.coverage = Some(read(account, "/render/admin/duels/catalog")?);
        desk.cells = listed(account, "/render/admin/duels/corpus", "cells")?;
    }
    if allows(roles, REVIEWER) || allows(roles, PUBLISHER) {
        desk.versions = listed(account, "/render/admin/duels/models", "models")?;
    }
    if allows(roles, OPERATOR) {
        desk.stuck = listed(account, "/render/admin/duels/stuck", "duels")?;
    }
    if allows(roles, OWNER) {
        desk.batches = listed(account, "/render/admin/duels/corpus/batches", "batches").unwrap_or_default();
    }
    desk.journal = listed(account, "/render/admin/duels/journal", "entries")?;
    desk.people = listed(account, "/render/admin/duels/people", "people")?;
    Ok(desk)
}

pub fn match_of(said: &str) -> Option<i64> {
    let said = said.trim().trim_end_matches('/');
    let tail = said.rsplit('/').next()?;
    let known = said.contains("osu.ppy.sh/community/matches/") || said.contains("osu.ppy.sh/mp/") || said == tail;
    tail.parse::<i64>().ok().filter(|found| known && *found > 0)
}

pub fn journal_before(account: &Account, before: i64) -> Result<Vec<Entry>, Failure> {
    listed(account, &format!("/render/admin/duels/journal?before={before}"), "entries")
}

pub fn cell(account: &Account, slot: &str, band: usize) -> Result<Vec<Held>, Failure> {
    listed(account, &format!("/render/admin/duels/catalog/{slot}/{band}"), "maps")
}

pub fn review(account: &Account, beatmap: i64, decision: &str, slot: Option<&str>, secondary: Option<Option<&str>>, grade: Option<(&str, &[String])>, reason: &str) -> Result<Value, Failure> {
    let mut body = json!({ "decision": decision, "slot": slot, "reason": reason });
    if let Some(secondary) = secondary {
        body["secondary"] = json!(secondary);
    }
    if let Some((tier, marks)) = grade {
        body["tier"] = json!(tier);
        body["reasons"] = json!(marks);
    }
    call(account, reqwest::Method::POST, &format!("/render/admin/duels/maps/{beatmap}/review"), Some(body), None)
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
        let labelled = SLOTS.contains(&upper.as_str()) || RESERVE.contains(&upper.as_str());
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

pub fn read_map(text: &str) -> Option<i64> {
    let said = text.trim();
    let (path, part) = said.split_once('#').unwrap_or((said, ""));
    let path = path.split('?').next().unwrap_or_default().trim_end_matches('/');
    let found = if part.is_empty() {
        if path.contains("beatmapsets/") { return None; }
        path.rsplit('/').next()?
    } else {
        part.strip_prefix("osu/")?
    };
    found.parse::<i64>().ok().filter(|beatmap| *beatmap > 0)
}

const ADD_PATIENCE: std::time::Duration = std::time::Duration::from_secs(75);

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Added {
    pub outcome: String,
    pub status: Option<String>,
    pub task: Option<Task>,
}

pub fn add_map(account: &Account, beatmap: i64, slot: Option<&str>, reason: &str) -> Result<Added, Failure> {
    let body = json!({ "beatmap_id": beatmap, "slot": slot, "reason": reason });
    crate::duels::call_within(account, reqwest::Method::POST, "/render/admin/duels/catalog/maps", Some(body), None, Some(ADD_PATIENCE))
}

pub fn rename_batch(account: &Account, name: &str, year: i32, new_name: &str, new_year: i32, reason: &str) -> Result<Value, Failure> {
    let body = json!({ "name": name, "year": year, "new_name": new_name, "new_year": new_year, "reason": reason });
    call(account, reqwest::Method::POST, "/render/admin/duels/corpus/batches/rename", Some(body), None)
}

pub fn delete_batch(account: &Account, name: &str, year: i32, beatmap: Option<i64>, reason: &str) -> Result<Value, Failure> {
    let body = json!({ "name": name, "year": year, "beatmap_id": beatmap, "reason": reason });
    call(account, reqwest::Method::POST, "/render/admin/duels/corpus/batches/delete", Some(body), None)
}

pub fn exclude_batch(account: &Account, name: &str, year: i32, beatmap: Option<i64>, excluded: bool, reason: &str) -> Result<Value, Failure> {
    let body = json!({ "name": name, "year": year, "beatmap_id": beatmap, "excluded": excluded, "reason": reason });
    call(account, reqwest::Method::POST, "/render/admin/duels/corpus/batches/exclude", Some(body), None)
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

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Job {
    pub job: String,
    pub state: String,
    pub version: Option<String>,
    pub error: Option<String>,
    pub phase: String,
    pub started_at: Option<String>,
    pub completed: u64,
    pub total: u64,
    pub maps: u64,
    pub checks_done: u64,
    pub checks_total: u64,
    pub fold: Option<String>,
    pub elapsed_seconds: u64,
    pub revision: u64,

}

pub fn train(account: &Account, reason: &str) -> Result<Option<String>, Failure> {
    let said: Value = call(account, reqwest::Method::POST, "/render/admin/duels/models/train", Some(json!({ "reason": reason })), None)?;
    Ok(said.get("job").and_then(Value::as_str).map(str::to_owned))
}

pub fn training_job(account: &Account, job: &str) -> Result<Job, Failure> {
    call(account, reqwest::Method::GET, &format!("/render/admin/duels/models/training/{job}"), None, None)
}

pub fn discover(account: &Account, slot: &str, band: usize, reason: &str) -> Result<Value, Failure> {
    call(account, reqwest::Method::POST, "/render/admin/duels/catalog/discover", Some(json!({ "slot": slot, "band": band, "reason": reason })), None)
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
    fn a_reason_without_a_level_says_nothing_about_it() {
        let told: Reason = serde_json::from_value(json!({ "feature": "spacing_radii.p90", "weight": 0.4, "level": -1.2 })).expect("a reason");
        let quiet: Reason = serde_json::from_value(json!({ "feature": "skill.aim", "weight": 0.4 })).expect("a reason");
        assert_eq!((told.level, quiet.level), (Some(-1.2), None));
        let job: Job = serde_json::from_value(json!({ "job": "a1", "state": "done", "version": "kamerton-3", "error": null })).expect("a job");
        assert_eq!((job.state.as_str(), job.version.as_deref(), job.error), ("done", Some("kamerton-3"), None));
    }

    #[test]
    fn a_map_is_described_by_the_mirror_and_never_by_osu() {
        assert!(about_address(5687673).starts_with("https://catboy.best/") && !about_address(1).contains("ppy.sh"));
        let found = serde_json::json!({ "beatmapset_id": 2300330, "difficulty_rating": 5.86, "bpm": 137.8, "total_length": 216, "version": "YO SAY!", "owners": [{ "username": "Pata-Mon" }], "set": { "title": "HOT LIMIT", "artist": "T.M.Revolution", "creator": "Wanpachi" } });
        let about = read_about(&found).expect("an about");
        assert_eq!((about.set, about.title.as_str(), about.mapper.as_str(), about.seconds), (Some(2300330), "HOT LIMIT", "Pata-Mon", Some(216)));
        assert!(about.standard && about.status.is_empty());
        let loved = serde_json::json!({ "mode": "osu", "status": "loved", "set": { "title": "Old" } });
        assert_eq!(read_about(&loved).map(|about| (about.status, about.standard)), Some(("loved".to_owned(), true)));
        let mania = serde_json::json!({ "mode": "mania", "set": { "title": "Keys", "status": "graveyard" } });
        assert_eq!(read_about(&mania).map(|about| (about.status, about.standard)), Some(("graveyard".to_owned(), false)));
        let alternate = serde_json::json!({"id": 42, "set": null, "beatmapset": {"title": "Recovered"}});
        assert_eq!(read_about(&alternate).unwrap().title, "Recovered");
        let flat = serde_json::json!({"id": 42, "set": {"title": " "}, "title": "Flat title"});
        assert_eq!(read_about(&flat).unwrap().title, "Flat title");
    }

    #[test]
    #[ignore = "asks the real mirrors"]
    fn a_real_map_is_described_by_whichever_mirror_answers() {
        let about = about(5687673).expect("one of the mirrors answers");
        assert_eq!((about.title.as_str(), about.set), ("HOT LIMIT", Some(2300330)));
        assert!(crate::mapper_faces::owner(&looked_up(5687673).unwrap()).is_some());
    }

    #[test]
    fn a_difficulty_in_an_archive_is_read_with_its_mode_and_stars() {
        let standard = "osu file format v14\n\n[General]\nMode: 0\n\n[Metadata]\nVersion:Extra\nBeatmapID:77\n\n[Difficulty]\nHPDrainRate:5\nCircleSize:4\nOverallDifficulty:8\nApproachRate:9\nSliderMultiplier:1.4\nSliderTickRate:1\n\n[TimingPoints]\n0,250,4,2,0,60,1,0\n\n[HitObjects]\n100,100,1000,1,0,0:0:0:0:\n300,200,1250,1,0,0:0:0:0:\n100,300,1500,1,0,0:0:0:0:\n";
        let packed = read_packed(standard).expect("a difficulty");
        assert_eq!((packed.beatmap, packed.version.as_str(), packed.standard), (77, "Extra", true));
        assert!(packed.stars.is_some_and(|stars| stars > 0.0));
        let mania = read_packed(&standard.replace("Mode: 0", "Mode: 3")).expect("a difficulty");
        assert_eq!((mania.standard, mania.stars), (false, None));
        assert_eq!(read_packed(&standard.replace("BeatmapID:77", "BeatmapID:0")), None);
    }

    #[test]
    fn one_map_is_read_from_a_link_or_a_number_and_never_from_a_whole_set() {
        for (said, beatmap) in [("5687673", Some(5687673)), (" https://osu.ppy.sh/b/5687673 ", Some(5687673)), ("https://osu.ppy.sh/beatmaps/5687673?mode=osu", Some(5687673)),
            ("https://osu.ppy.sh/beatmapsets/2612145#osu/5687673", Some(5687673)), ("osu.ppy.sh/beatmaps/75/", Some(75)),
            ("https://osu.ppy.sh/beatmapsets/2612145", None), ("https://osu.ppy.sh/beatmapsets/2612145#mania/5687673", None), ("hot limit", None), ("0", None), ("", None)] {
            assert_eq!(read_map(said), beatmap, "{said}");
        }
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
        assert_eq!((band_of(3.2), band_of(5.6), band_of(9.0), band_of(10.0)), (0, 1, 8, 9));
        assert!(allows(&["OWNER".to_owned()], OPERATOR) && !allows(&[REVIEWER.to_owned()], OPERATOR));
    }
}
