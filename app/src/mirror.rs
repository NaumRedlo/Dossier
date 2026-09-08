use std::io::Read;
use std::path::{Path, PathBuf};

struct Mirror {
    name: &'static str,
    look_up: &'static str,
    download: &'static str,
}

const MIRRORS: &[Mirror] = &[
    Mirror {
        name: "catboy.best",
        look_up: "https://catboy.best/api/v2/md5/",
        download: "https://catboy.best/d/",
    },
    Mirror {
        name: "osu.direct",
        look_up: "https://osu.direct/api/v2/md5/",
        download: "https://osu.direct/api/d/",
    },
];

const BIGGEST: u64 = 512 * 1024 * 1024;

const TRIES: u32 = 3;

const ASKS: u32 = 2;

const SULK: std::time::Duration = std::time::Duration::from_secs(180);

const STRIKES: u32 = 2;

fn sulking(
) -> &'static std::sync::Mutex<std::collections::HashMap<&'static str, (u32, std::time::Instant)>> {
    static HELD: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<&'static str, (u32, std::time::Instant)>>,
    > = std::sync::OnceLock::new();
    HELD.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

fn resting(name: &'static str) -> bool {
    let held = sulking().lock().expect("зеркала");
    held.get(name)
        .is_some_and(|(strikes, since)| *strikes >= STRIKES && since.elapsed() < SULK)
}

fn struck(name: &'static str) {
    let mut held = sulking().lock().expect("зеркала");
    let seen = held.entry(name).or_insert((0, std::time::Instant::now()));
    seen.0 += 1;
    seen.1 = std::time::Instant::now();
}

fn answered(name: &'static str) {
    sulking().lock().expect("зеркала").remove(name);
}

const BREATH_MS: u64 = 350;

#[derive(Debug, Clone, serde::Serialize)]
pub struct Found {
    pub set: u64,
    pub artist: String,
    pub title: String,
    pub version: String,
    pub folder: String,

    pub from: String,
}

fn hex(hash: &str) -> Result<String, String> {
    let clean = hash.trim().to_ascii_lowercase();
    if clean.len() == 32 && clean.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(clean)
    } else {
        Err("в реплее не записан хэш карты".to_owned())
    }
}

fn http(seconds: u64) -> Result<&'static reqwest::blocking::Client, String> {
    static QUICK: std::sync::OnceLock<Option<reqwest::blocking::Client>> =
        std::sync::OnceLock::new();
    static PATIENT: std::sync::OnceLock<Option<reqwest::blocking::Client>> =
        std::sync::OnceLock::new();

    let made = |seconds: u64| {
        reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(seconds))
            .connect_timeout(std::time::Duration::from_secs(20))
            .pool_idle_timeout(std::time::Duration::from_secs(30))
            .user_agent("Dossier")
            .build()
            .ok()
    };
    let held = if seconds > 120 {
        PATIENT.get_or_init(|| made(900))
    } else {
        QUICK.get_or_init(|| made(12))
    };
    held.as_ref()
        .ok_or_else(|| "не с чем идти в сеть".to_owned())
}

fn said(value: &serde_json::Value, path: &[&str]) -> String {
    let mut at = value;
    for step in path {
        match at.get(step) {
            Some(next) => at = next,
            None => return String::new(),
        }
    }
    at.as_str().unwrap_or_default().to_owned()
}

fn tidy(text: &str) -> String {
    let cleaned: String = text
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            other if other.is_control() => ' ',
            other => other,
        })
        .collect();
    let squeezed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if squeezed.chars().all(|c| c == '.') {
        String::new()
    } else {
        squeezed
    }
}

pub fn look_up_at(hash: &str, only: Option<&str>) -> Result<Found, String> {
    let mut unknown = 0;
    let mut asked = 0;
    let mut last = String::new();
    for mirror in MIRRORS {
        if only.is_some_and(|name| name != mirror.name) {
            continue;
        }
        if only.is_none() && resting(mirror.name) {
            continue;
        }
        asked += 1;
        for attempt in 0..ASKS {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(400 * u64::from(attempt)));
            }
            match ask_about(mirror, hash) {
                Ok(found) => {
                    answered(mirror.name);
                    return Ok(found);
                }
                Err(why) if why.contains("не знает") => {
                    answered(mirror.name);
                    unknown += 1;
                    last = why;
                    break;
                }
                Err(why) => {
                    last = why;
                    if attempt + 1 == ASKS {
                        struck(mirror.name);
                    }
                }
            }
        }
    }
    if asked == 0 {
        return Err("все зеркала карт сейчас молчат — попробуйте позже".to_owned());
    }
    if unknown == asked {
        return Err("карту не знает ни одно зеркало — возможно, её сняли с сайта".to_owned());
    }
    Err(last)
}

fn ask_about(mirror: &Mirror, hash: &str) -> Result<Found, String> {
    let hash = hex(hash)?;
    let reply = http(45)?
        .get(format!("{}{hash}", mirror.look_up))
        .send()
        .map_err(|why| format!("{} не ответило: {why}", mirror.name))?;

    if reply.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(format!("{} не знает такой карты", mirror.name));
    }
    if !reply.status().is_success() {
        return Err(format!("{} ответило {}", mirror.name, reply.status()));
    }
    let body: serde_json::Value = reply
        .json()
        .map_err(|why| format!("{} ответило непонятным: {why}", mirror.name))?;

    let set = body
        .get("beatmapset_id")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "в ответе зеркала нет номера набора".to_owned())?;

    let artist = tidy(&first_of(
        &body,
        &[&["set", "artist"], &["beatmapset", "artist"], &["artist"]],
    ));
    let title = tidy(&first_of(
        &body,
        &[&["set", "title"], &["beatmapset", "title"], &["title"]],
    ));
    let version = said(&body, &["version"]);
    let folder = if artist.is_empty() && title.is_empty() {
        format!("{set}")
    } else {
        format!("{set} {artist} - {title}")
    };
    Ok(Found {
        set,
        artist,
        title,
        version,
        folder,
        from: mirror.name.to_owned(),
    })
}

fn first_of(value: &serde_json::Value, paths: &[&[&str]]) -> String {
    for path in paths {
        let found = said(value, path);
        if !found.is_empty() {
            return found;
        }
    }
    String::new()
}

fn download(set: u64, first: &str, only: bool, say: &dyn Fn(u64, u64)) -> Result<Vec<u8>, String> {
    let mut order: Vec<&Mirror> = MIRRORS.iter().filter(|m| m.name == first).collect();
    if !only {
        order.extend(MIRRORS.iter().filter(|m| m.name != first));
    }
    let mut last = "ни одно зеркало не отдало карту".to_owned();
    for mirror in order {
        if mirror.name != first && resting(mirror.name) {
            continue;
        }
        for attempt in 0..TRIES {
            if attempt > 0 {
                std::thread::sleep(std::time::Duration::from_millis(700 * u64::from(attempt)));
            }
            match once(mirror, set, say) {
                Ok(bytes) => {
                    answered(mirror.name);
                    return Ok(bytes);
                }
                Err(why) => {
                    last = why;
                    if attempt + 1 == TRIES {
                        struck(mirror.name);
                    }
                }
            }
        }
    }
    Err(last)
}

fn once(mirror: &Mirror, set: u64, say: &dyn Fn(u64, u64)) -> Result<Vec<u8>, String> {
    let mut reply = http(900)?
        .get(format!("{}{set}", mirror.download))
        .send()
        .map_err(|why| format!("{} не отдало карту: {why}", mirror.name))?;
    if !reply.status().is_success() {
        return Err(format!("{} отказало: {}", mirror.name, reply.status()));
    }
    let of = reply.content_length().unwrap_or(0);
    if of > BIGGEST {
        return Err("карта слишком большая, чтобы качать её отсюда".to_owned());
    }

    let mut bytes = Vec::with_capacity(of.min(64 * 1024 * 1024) as usize);
    let mut chunk = [0u8; 64 * 1024];
    loop {
        let read = reply
            .read(&mut chunk)
            .map_err(|why| format!("связь оборвалась: {why}"))?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..read]);
        if bytes.len() as u64 > BIGGEST {
            return Err("карта слишком большая, чтобы качать её отсюда".to_owned());
        }
        say(bytes.len() as u64, of);
    }
    if bytes.len() < 1024 {
        return Err("зеркало отдало пустой файл".to_owned());
    }
    Ok(bytes)
}

fn field_of(text: &str, key: &str) -> String {
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(key) {
            if let Some(value) = rest.trim_start().strip_prefix(':') {
                return value.trim().to_owned();
            }
        }
    }
    String::new()
}

fn free_folder(songs: &Path, wanted: &str) -> PathBuf {
    let first = songs.join(wanted);
    if !first.exists() {
        return first;
    }
    for n in 2..100 {
        let next = songs.join(format!("{wanted} ({n})"));
        if !next.exists() {
            return next;
        }
    }
    songs.join(format!("{wanted} ({})", std::process::id()))
}

pub fn bring(
    hash: &str,
    songs: &Path,
    only: Option<&str>,
    say: &dyn Fn(&str, u64, u64),
) -> Result<Found, String> {
    if !songs.is_dir() {
        return Err("папка карт не выбрана в настройках".to_owned());
    }
    say("Ищу карту на зеркале…", 0, 0);
    let found = look_up_at(hash, only)?;

    say("Качаю карту…", 0, 0);
    std::thread::sleep(std::time::Duration::from_millis(BREATH_MS));
    let bytes = download(found.set, &found.from, only.is_some(), &|got, of| {
        say("Качаю карту…", got, of)
    })?;

    say("Распаковываю…", 0, 0);
    let into = free_folder(songs, &found.folder);
    std::fs::create_dir_all(&into).map_err(|why| format!("не создать папку карты: {why}"))?;
    if let Err(why) = dossier_produce::skin::unpack(&bytes, &into) {
        let _ = std::fs::remove_dir_all(&into);
        return Err(format!("карта не распаковалась: {why}"));
    }

    let landed = dossier_produce::locate::search_dir(&into, &hex(hash)?)
        .ok()
        .flatten();
    let Some(landed) = landed else {
        let _ = std::fs::remove_dir_all(&into);
        return Err("в наборе не оказалось той сложности, что в реплее".to_owned());
    };

    let mut found = found;
    if found.artist.is_empty() || found.title.is_empty() {
        found.artist = tidy(&field_of(&landed.text, "Artist"));
        found.title = tidy(&field_of(&landed.text, "Title"));
        if !found.artist.is_empty() && !found.title.is_empty() {
            let wanted = format!("{} {} - {}", found.set, found.artist, found.title);
            let named = free_folder(songs, &wanted);
            if std::fs::rename(&into, &named).is_ok() {
                found.folder = named
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or(found.folder);
            }
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_real_md5_is_allowed_into_the_address() {
        assert_eq!(
            hex("930B6FCC81C41A1C69D9ABCE11153B9C").unwrap(),
            "930b6fcc81c41a1c69d9abce11153b9c"
        );
        assert!(hex("").is_err());
        assert!(hex("../../etc/passwd").is_err());
        assert!(hex("930b6fcc81c41a1c69d9abce11153b9").is_err());
        assert!(hex("930b6fcc81c41a1c69d9abce11153b9z").is_err());
    }

    #[test]
    fn a_folder_name_keeps_nothing_that_could_climb_out_of_the_songs_folder() {
        assert_eq!(tidy("Demetori"), "Demetori");
        assert_eq!(tidy("a/b\\c:d*e?f\"g<h>i|j"), "a b c d e f g h i j");
        assert_eq!(tidy("  spaced   out  "), "spaced out");
        assert_eq!(tidy(".."), "");
        assert_eq!(tidy("."), "");
        for nasty in ["..", ".", "../..", "..\\..", "/etc/passwd", "  ..  "] {
            let made = tidy(nasty);
            assert!(
                !made.contains('/') && !made.contains('\\'),
                "{nasty} → {made}"
            );
            assert!(made != "." && made != "..", "{nasty} → {made}");
        }
    }

    #[test]
    fn a_field_is_read_off_the_map_and_not_confused_with_its_neighbour() {
        let text = "[Metadata]\nTitle:Seijouki no Pierrot\nTitleUnicode:x\nArtist:Demetori\n";
        assert_eq!(field_of(text, "Title"), "Seijouki no Pierrot");
        assert_eq!(field_of(text, "Artist"), "Demetori");
        assert_eq!(field_of(text, "Creator"), "");
    }

    #[test]
    fn a_taken_name_gets_a_number_rather_than_overwriting_what_is_there() {
        let dir = std::env::temp_dir().join(format!("dossier-mirror-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(free_folder(&dir, "set"), dir.join("set"));
        std::fs::create_dir_all(dir.join("set")).unwrap();
        assert_eq!(free_folder(&dir, "set"), dir.join("set (2)"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod against_the_mirror {
    use super::*;

    #[test]
    #[ignore]
    fn a_known_map_is_found_by_its_hash() {
        let found =
            look_up_at("930b6fcc81c41a1c69d9abce11153b9c", None).expect("the mirror knows it");
        assert_eq!(found.set, 661_333);
        assert!(found.folder.starts_with("661333"));
        println!("{} → {} (с {})", found.set, found.folder, found.from);
    }

    #[test]
    #[ignore]
    fn a_map_comes_down_whole_and_carries_the_difficulty_that_was_played() {
        let songs = std::env::temp_dir().join(format!("dossier-songs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&songs);
        std::fs::create_dir_all(&songs).unwrap();

        let hash = "930b6fcc81c41a1c69d9abce11153b9c";
        let steps = std::sync::Mutex::new(Vec::new());
        let found = bring(hash, &songs, None, &|step, got, of| {
            let mut said = steps.lock().unwrap();
            if said.last().map(String::as_str) != Some(step) {
                said.push(step.to_owned());
            }
            let _ = (got, of);
        })
        .expect("the map came down");

        println!("{} → {}", found.set, found.folder);
        println!("steps: {:?}", steps.lock().unwrap());
        let landed = songs.join(&found.folder);
        assert!(landed.is_dir(), "no folder at {}", landed.display());
        assert!(
            dossier_produce::locate::search_dir(&songs, hash)
                .unwrap()
                .is_some(),
            "the played difficulty is not in what came down"
        );
        let _ = std::fs::remove_dir_all(&songs);
    }

    #[test]
    #[ignore]
    fn a_hash_nobody_has_is_refused_plainly() {
        let why = look_up_at("00000000000000000000000000000000", None).unwrap_err();
        assert!(why.contains("не знает"), "{why}");
    }
}

#[cfg(test)]
mod under_load {
    use super::*;

    #[test]
    #[ignore]
    fn a_burst_of_lookups_finds_a_mirror_that_answers() {
        let hashes = [
            "930b6fcc81c41a1c69d9abce11153b9c",
            "bd6aea9634c95136ddfe7c91cf03f32d",
        ];
        let mut good = 0;
        let mut said = Vec::new();
        for round in 0..6 {
            let hash = hashes[round % hashes.len()];
            match look_up_at(hash, None) {
                Ok(found) => {
                    good += 1;
                    println!("{round}: {hash} -> {} from {}", found.set, found.from);
                }
                Err(why) => {
                    println!("{round}: {hash} -> {why}");
                    said.push(why);
                }
            }
        }
        println!("{good} of 6 answered");
        assert!(good >= 4, "no mirror answered often enough: {said:?}");
    }
}
