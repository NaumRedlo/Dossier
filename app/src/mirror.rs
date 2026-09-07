use std::io::Read;
use std::path::{Path, PathBuf};

const LOOK_UP: &str = "https://catboy.best/api/v2/md5/";
const DOWNLOAD: &str = "https://catboy.best/d/";

const BIGGEST: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, serde::Serialize)]
pub struct Found {
    pub set: u64,
    pub artist: String,
    pub title: String,
    pub version: String,
    pub folder: String,
}

fn hex(hash: &str) -> Result<String, String> {
    let clean = hash.trim().to_ascii_lowercase();
    if clean.len() == 32 && clean.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(clean)
    } else {
        Err("в реплее не записан хэш карты".to_owned())
    }
}

fn http() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .user_agent("Dossier")
        .build()
        .map_err(|why| format!("не с чем идти в сеть: {why}"))
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

pub fn look_up(hash: &str) -> Result<Found, String> {
    let hash = hex(hash)?;
    let reply = http()?
        .get(format!("{LOOK_UP}{hash}"))
        .send()
        .map_err(|why| format!("зеркало карт не ответило: {why}"))?;

    if reply.status() == reqwest::StatusCode::NOT_FOUND {
        return Err("зеркало не знает такой карты — возможно, её сняли с сайта".to_owned());
    }
    if !reply.status().is_success() {
        return Err(format!("зеркало ответило {}", reply.status()));
    }
    let body: serde_json::Value = reply
        .json()
        .map_err(|why| format!("зеркало ответило непонятным: {why}"))?;

    let set = body
        .get("beatmapset_id")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "в ответе зеркала нет номера набора".to_owned())?;

    let artist = tidy(&said(&body, &["set", "artist"]));
    let title = tidy(&said(&body, &["set", "title"]));
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
    })
}

fn download(set: u64, say: &dyn Fn(u64, u64)) -> Result<Vec<u8>, String> {
    let mut reply = http()?
        .get(format!("{DOWNLOAD}{set}"))
        .send()
        .map_err(|why| format!("карта не пошла: {why}"))?;
    if !reply.status().is_success() {
        return Err(format!("зеркало не отдало карту: {}", reply.status()));
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

pub fn bring(hash: &str, songs: &Path, say: &dyn Fn(&str, u64, u64)) -> Result<Found, String> {
    if !songs.is_dir() {
        return Err("папка карт не выбрана в настройках".to_owned());
    }
    say("Ищу карту на зеркале…", 0, 0);
    let found = look_up(hash)?;

    say("Качаю карту…", 0, 0);
    let bytes = download(found.set, &|got, of| say("Качаю карту…", got, of))?;

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
    if landed.is_none() {
        let _ = std::fs::remove_dir_all(&into);
        return Err("в наборе не оказалось той сложности, что в реплее".to_owned());
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
        let found = look_up("930b6fcc81c41a1c69d9abce11153b9c").expect("the mirror knows it");
        assert_eq!(found.set, 661_333);
        assert_eq!(found.artist, "Demetori");
        assert!(found.title.starts_with("Seijouki no Pierrot"));
        assert!(found.folder.starts_with("661333 Demetori - "));
        println!("{} → {}", found.set, found.folder);
    }

    #[test]
    #[ignore]
    fn a_map_comes_down_whole_and_carries_the_difficulty_that_was_played() {
        let songs = std::env::temp_dir().join(format!("dossier-songs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&songs);
        std::fs::create_dir_all(&songs).unwrap();

        let hash = "930b6fcc81c41a1c69d9abce11153b9c";
        let steps = std::sync::Mutex::new(Vec::new());
        let found = bring(hash, &songs, &|step, got, of| {
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
        let why = look_up("00000000000000000000000000000000").unwrap_err();
        assert!(why.contains("не знает"), "{why}");
    }
}
