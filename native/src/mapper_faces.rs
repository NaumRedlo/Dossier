use std::collections::HashMap;
use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use iced::widget::image;
use serde_json::Value;

const FACE_MOST: u64 = 2 * 1024 * 1024;
const RETRY: Duration = Duration::from_secs(120);

static QUIET: AtomicBool = AtomicBool::new(false);
static WAITING: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Face {
    Mapper(i64),
    Player(i64),
}

fn store() -> &'static Mutex<HashMap<Face, (Option<image::Handle>, Instant)>> {
    static STORE: OnceLock<Mutex<HashMap<Face, (Option<image::Handle>, Instant)>>> = OnceLock::new();
    STORE.get_or_init(Mutex::default)
}

pub fn quiet() {
    QUIET.store(true, Ordering::Relaxed);
}

pub fn waiting() -> bool {
    WAITING.load(Ordering::Relaxed) > 0
}

pub fn of(beatmap: i64) -> Option<image::Handle> {
    held(Face::Mapper(beatmap), beatmap)
}

pub fn of_player(osu_id: i64) -> Option<image::Handle> {
    held(Face::Player(osu_id), osu_id)
}

fn held(face: Face, id: i64) -> Option<image::Handle> {
    if id <= 0 {
        return None;
    }
    let mut held = store().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some((found, at)) = held.get(&face) {
        if found.is_some() || at.elapsed() < RETRY {
            return found.clone();
        }
    }
    held.insert(face, (None, Instant::now()));
    if QUIET.load(Ordering::Relaxed) {
        return None;
    }
    WAITING.fetch_add(1, Ordering::Relaxed);
    std::thread::spawn(move || {
        let found = match face {
            Face::Mapper(beatmap) => crate::desk::looked_up(beatmap).as_ref().and_then(owner).and_then(picture),
            Face::Player(osu_id) => picture(osu_id),
        };
        store().lock().unwrap_or_else(std::sync::PoisonError::into_inner).insert(face, (found, Instant::now()));
        WAITING.fetch_sub(1, Ordering::Relaxed);
    });
    None
}

pub fn owner(found: &Value) -> Option<i64> {
    found.get("owners").and_then(Value::as_array).and_then(|owners| owners.first()).and_then(|owner| owner.get("id")).and_then(Value::as_i64)
        .or_else(|| found.get("user_id").and_then(Value::as_i64))
        .or_else(|| found.get("set").or_else(|| found.get("beatmapset")).and_then(|set| set.get("user_id")).and_then(Value::as_i64))
        .filter(|id| *id > 0)
}

fn picture(user: i64) -> Option<image::Handle> {
    let client = crate::bot::client().ok()?;
    let picture = client.get(format!("https://a.ppy.sh/{user}")).header("User-Agent", crate::desk::MIRROR_AGENT).send().ok()?;
    if !picture.status().is_success() {
        return None;
    }
    let mut bytes = Vec::new();
    picture.take(FACE_MOST).read_to_end(&mut bytes).ok()?;
    crate::main_screen::covered_bytes(&bytes, 96, 96)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_mapper_is_read_from_the_owner_then_the_map_then_the_set() {
        let owners = serde_json::json!({ "owners": [{ "id": 4116573, "username": "Nevo" }], "user_id": 1, "set": { "user_id": 2 } });
        assert_eq!(super::owner(&owners), Some(4116573));
        assert_eq!(super::owner(&serde_json::json!({ "user_id": 7, "set": { "user_id": 2 } })), Some(7));
        assert_eq!(super::owner(&serde_json::json!({ "beatmapset": { "user_id": 2 } })), Some(2));
        assert_eq!(super::owner(&serde_json::json!({ "user_id": 0 })), None);
    }

    #[test]
    fn an_animated_avatar_is_drawn_from_its_first_frame() {
        let mut bytes = Vec::new();
        ::image::DynamicImage::new_rgba8(4, 4).write_to(&mut std::io::Cursor::new(&mut bytes), ::image::ImageFormat::Gif).unwrap();
        assert!(crate::main_screen::covered_bytes(&bytes, 96, 96).is_some());
    }
}
