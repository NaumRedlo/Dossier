use std::collections::HashMap;
use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use iced::widget::image;
use serde_json::Value;

const FACE_MOST: u64 = 2 * 1024 * 1024;

static QUIET: AtomicBool = AtomicBool::new(false);
static WAITING: AtomicUsize = AtomicUsize::new(0);

fn store() -> &'static Mutex<HashMap<i64, Option<image::Handle>>> {
    static STORE: OnceLock<Mutex<HashMap<i64, Option<image::Handle>>>> = OnceLock::new();
    STORE.get_or_init(Mutex::default)
}

pub fn quiet() {
    QUIET.store(true, Ordering::Relaxed);
}

pub fn waiting() -> bool {
    WAITING.load(Ordering::Relaxed) > 0
}

pub fn of(beatmap: i64) -> Option<image::Handle> {
    if beatmap <= 0 {
        return None;
    }
    let mut held = store().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(found) = held.get(&beatmap) {
        return found.clone();
    }
    held.insert(beatmap, None);
    if QUIET.load(Ordering::Relaxed) {
        return None;
    }
    WAITING.fetch_add(1, Ordering::Relaxed);
    std::thread::spawn(move || {
        let found = fetched(beatmap);
        store().lock().unwrap_or_else(std::sync::PoisonError::into_inner).insert(beatmap, found);
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

fn fetched(beatmap: i64) -> Option<image::Handle> {
    let client = crate::bot::client().ok()?;
    let about = client.get(crate::desk::about_address(beatmap)).header("User-Agent", "Dossier").send().ok()?;
    if !about.status().is_success() {
        return None;
    }
    let mapper = owner(&about.json::<Value>().ok()?)?;
    let picture = client.get(format!("https://a.ppy.sh/{mapper}")).header("User-Agent", "Dossier").send().ok()?;
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
