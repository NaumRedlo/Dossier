use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::settings::Settings;

#[derive(Debug, Clone, Serialize)]
pub struct Shelf {
    pub path: String,
    pub exists: bool,
    pub items: usize,
    pub bytes: u64,

    pub note: String,
}

impl Shelf {
    fn missing(path: &str, note: &str) -> Self {
        Self {
            path: path.to_owned(),
            exists: false,
            items: 0,
            bytes: 0,
            note: note.to_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Library {
    pub songs: Shelf,
    pub skins: Shelf,
    pub replays: Shelf,
}

#[derive(Debug, Clone, Serialize)]
pub struct Played {
    pub path: String,
    pub file: String,
    pub player: String,
    pub mods: String,
    pub score: i32,
    pub combo: u16,
    pub bytes: u64,

    pub have_map: bool,

    pub played_at: i64,
}

fn entries(path: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .map(|found| found.path())
        .collect()
}

fn plural(n: usize, one: &str, few: &str, many: &str) -> String {
    let word = crate::check::plural(n, one, few, many);
    format!("{n} {word}")
}

fn shelve_songs(path: &str) -> Shelf {
    let at = Path::new(path);
    if path.is_empty() || !at.is_dir() {
        return Shelf::missing(path, "Папки нет: некуда сохранять карты");
    }
    let found = entries(at);
    let archives = found
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("osz")))
        .count();
    let folders = found
        .iter()
        .filter(|p| p.is_dir() && !entries(p).is_empty())
        .count();
    let bytes = found
        .iter()
        .filter_map(|p| p.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum();
    Shelf {
        path: path.to_owned(),
        exists: true,
        items: archives + folders,
        bytes,
        note: format!(
            "{}, {}",
            plural(archives, "архив", "архива", "архивов"),
            plural(folders, "папка", "папки", "папок")
        ),
    }
}

fn shelve_skins(path: &str) -> Shelf {
    let at = Path::new(path);
    if path.is_empty() || !at.is_dir() {
        return Shelf::missing(path, "Не указана: рисуем своим скином");
    }
    let mut skins = 0;
    let mut bytes = 0;
    for folder in entries(at).into_iter().filter(|p| p.is_dir()) {
        let inside = entries(&folder);
        let looks_like = inside.iter().any(|p| {
            p.file_name()
                .is_some_and(|n| n.eq_ignore_ascii_case("skin.ini"))
                || p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png"))
        });
        if looks_like {
            skins += 1;
            bytes += inside
                .iter()
                .filter_map(|p| p.metadata().ok())
                .map(|m| m.len())
                .sum::<u64>();
        }
    }
    Shelf {
        path: path.to_owned(),
        exists: true,
        items: skins,
        bytes,
        note: plural(skins, "скин", "скина", "скинов"),
    }
}

fn shelve_replays(path: &str) -> Shelf {
    let at = Path::new(path);
    if path.is_empty() || !at.is_dir() {
        return Shelf::missing(path, "Не указана: негде брать реплеи");
    }
    let found: Vec<_> = entries(at)
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("osr")))
        .collect();
    let bytes = found
        .iter()
        .filter_map(|p| p.metadata().ok())
        .map(|m| m.len())
        .sum();
    Shelf {
        path: path.to_owned(),
        exists: true,
        items: found.len(),
        bytes,
        note: plural(found.len(), "реплей", "реплея", "реплеев"),
    }
}

pub fn look(said: &Settings) -> Library {
    Library {
        songs: shelve_songs(&said.songs),
        skins: shelve_skins(&said.skins),
        replays: shelve_replays(&said.replays),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Found {
    pub root: String,
    pub songs: String,
    pub skins: String,
    pub replays: String,
    pub maps: usize,
    pub note: String,
}

fn likely_roots() -> Vec<std::path::PathBuf> {
    let home = crate::settings::home();
    let mut out = vec![
        home.join("osu!"),
        home.join("osu"),
        home.join("Games").join("osu!"),
        home.join(".osu"),
        home.join(".local").join("share").join("osu-stable"),
    ];
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        out.push(std::path::PathBuf::from(local).join("osu!"));
    }
    for wine in [".wine", ".local/share/wineprefixes/osu"] {
        let users = home.join(wine).join("drive_c").join("users");
        if let Ok(names) = std::fs::read_dir(&users) {
            for name in names.flatten() {
                out.push(name.path().join("AppData").join("Local").join("osu!"));
            }
        }
    }
    out
}

pub fn find_osu() -> Vec<Found> {
    let mut seen = std::collections::HashSet::new();
    let mut found = Vec::new();
    for root in likely_roots() {
        let songs = root.join("Songs");
        if !songs.is_dir() {
            continue;
        }
        let Ok(canon) = root.canonicalize() else {
            continue;
        };
        if !seen.insert(canon) {
            continue;
        }
        let shelf = shelve_songs(&songs.display().to_string());
        let skins = root.join("Skins");
        let replays = root.join("Replays");
        found.push(Found {
            root: root.display().to_string(),
            songs: songs.display().to_string(),
            skins: if skins.is_dir() { skins.display().to_string() } else { String::new() },
            replays: if replays.is_dir() { replays.display().to_string() } else { String::new() },
            maps: shelf.items,
            note: shelf.note,
        });
    }
    found.sort_by(|a, b| b.maps.cmp(&a.maps));
    found
}

pub fn skins(said: &Settings) -> Vec<String> {
    let at = Path::new(&said.skins);
    if said.skins.is_empty() || !at.is_dir() {
        return Vec::new();
    }
    let mut names: Vec<String> = entries(at)
        .into_iter()
        .filter(|path| path.is_dir())
        .filter(|path| {
            entries(path).iter().any(|inside| {
                inside
                    .file_name()
                    .is_some_and(|n| n.eq_ignore_ascii_case("skin.ini"))
                    || inside
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("png"))
            })
        })
        .filter_map(|path| path.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect();
    names.sort_by_key(|name| name.to_lowercase());
    names
}

fn maps(songs: &Path) -> std::collections::HashSet<String> {
    static SEEN: std::sync::Mutex<Option<(String, u64, std::collections::HashSet<String>)>> =
        std::sync::Mutex::new(None);

    let mark = stamp(songs);
    let mut held = SEEN.lock().expect("the map index");
    if let Some((was, when, found)) = held.as_ref() {
        if was == &songs.display().to_string() && *when == mark {
            return found.clone();
        }
    }
    let index = dossier_produce::locate::index(songs);
    dossier_produce::locate::remember_all(&index);
    let found: std::collections::HashSet<String> = index.into_keys().collect();
    *held = Some((songs.display().to_string(), mark, found.clone()));
    found
}

fn stamp(folder: &Path) -> u64 {
    let mut count = 0u64;
    let mut newest = 0u64;
    let mut stack = vec![folder.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            count += 1;
            if let Ok(when) = entry.metadata().and_then(|m| m.modified()) {
                if let Ok(since) = when.duration_since(std::time::UNIX_EPOCH) {
                    newest = newest.max(since.as_secs());
                }
            }
        }
    }
    count.wrapping_mul(1_000_000_007).wrapping_add(newest)
}

pub fn played(said: &Settings, most: usize) -> Vec<Played> {
    let at = Path::new(&said.replays);
    if said.replays.is_empty() || !at.is_dir() {
        return Vec::new();
    }
    let files: Vec<_> = entries(at)
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("osr")))
        .filter_map(|p| {
            let when = p.metadata().ok()?.modified().ok()?;
            Some((when, p))
        })
        .collect();
    let known = maps(Path::new(&said.songs));
    let mut played: Vec<Played> = files
        .into_iter()
        .filter_map(|(when, path)| {
            let bytes = std::fs::read(&path).ok()?;
            let replay = dossier_replay::Replay::heading(&bytes).ok()?;
            let have_map = known.contains(&replay.beatmap_hash);
            let played_at = when_played(&replay).unwrap_or_else(|| file_seconds(when));
            Some(Played {
                played_at,
                file: path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                path: path.display().to_string(),
                player: replay.player.clone(),
                mods: replay.mods.to_string(),
                score: replay.score,
                combo: replay.max_combo,
                bytes: bytes.len() as u64,
                have_map,
            })
        })
        .collect();
    played.sort_by_key(|one| std::cmp::Reverse(one.played_at));
    played.truncate(most);
    played
}

const TICKS_TO_UNIX: i64 = 62_135_596_800;

fn when_played(replay: &dossier_replay::Replay) -> Option<i64> {
    let seconds = replay.timestamp_ticks / 10_000_000 - TICKS_TO_UNIX;
    (seconds > 0).then_some(seconds)
}

fn file_seconds(when: std::time::SystemTime) -> i64 {
    when.duration_since(std::time::UNIX_EPOCH)
        .map(|gap| gap.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {

    #[test]
    fn only_an_osr_inside_the_replay_folder_can_be_dropped() {
        let shelf = scratch("shelf");
        let outside = scratch("outside");
        let good = shelf.join("one.osr");
        std::fs::write(&good, b"x").expect("written");
        std::fs::write(shelf.join("two.mp4"), b"x").expect("written");
        std::fs::write(outside.join("three.osr"), b"x").expect("written");

        assert!(droppable(&shelf, &good).is_ok());
        assert!(droppable(&shelf, &shelf.join("two.mp4")).is_err());
        assert!(droppable(&shelf, &outside.join("three.osr")).is_err());
        assert!(droppable(&shelf, &shelf.join("nothing.osr")).is_err());

        std::fs::remove_dir_all(&shelf).ok();
        std::fs::remove_dir_all(&outside).ok();
    }
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("dossier-library-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a place");
        dir
    }

    #[test]
    fn a_folder_that_is_not_there_says_so_rather_than_counting_zero() {
        let shelf = shelve_songs("/nowhere/at/all");
        assert!(!shelf.exists);
        assert!(
            !shelf.note.is_empty(),
            "it said nothing about being missing"
        );
    }

    #[test]
    fn maps_are_counted_as_archives_and_as_folders() {
        let dir = scratch("songs");
        std::fs::write(dir.join("one.osz"), b"zip").expect("written");
        std::fs::write(dir.join("two.OSZ"), b"zip").expect("written");
        std::fs::create_dir(dir.join("unpacked")).expect("a folder");
        std::fs::write(dir.join("unpacked/a.osu"), b"map").expect("written");

        std::fs::create_dir(dir.join("empty")).expect("a folder");

        let shelf = shelve_songs(&dir.display().to_string());
        assert!(shelf.exists);
        assert_eq!(shelf.items, 3, "{}", shelf.note);
        assert!(shelf.note.contains("2 архива"), "{}", shelf.note);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_skin_is_a_folder_with_pictures_or_an_ini_in_it() {
        let dir = scratch("skins");
        std::fs::create_dir(dir.join("with-ini")).expect("a folder");
        std::fs::write(dir.join("with-ini/skin.ini"), b"[General]").expect("written");
        std::fs::create_dir(dir.join("just-pictures")).expect("a folder");
        std::fs::write(dir.join("just-pictures/cursor.png"), b"pic").expect("written");
        std::fs::create_dir(dir.join("not-a-skin")).expect("a folder");
        std::fs::write(dir.join("not-a-skin/notes.txt"), b"hello").expect("written");

        let shelf = shelve_skins(&dir.display().to_string());
        assert_eq!(shelf.items, 2, "{}", shelf.note);
        assert_eq!(shelf.note, "2 скина");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn replays_are_counted_by_their_ending_whatever_its_case() {
        let dir = scratch("replays");
        std::fs::write(dir.join("a.osr"), b"replay").expect("written");
        std::fs::write(dir.join("b.OSR"), b"replay").expect("written");
        std::fs::write(dir.join("c.txt"), b"not one").expect("written");
        let shelf = shelve_replays(&dir.display().to_string());
        assert_eq!(shelf.items, 2);
        assert_eq!(shelf.note, "2 реплея");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rubbish_in_the_replay_folder_is_stepped_over() {
        let dir = scratch("bad-replays");
        std::fs::write(dir.join("nonsense.osr"), b"not a replay at all").expect("written");
        let said = Settings {
            replays: dir.display().to_string(),
            songs: String::new(),
            ..Settings::default()
        };
        assert!(
            played(&said, 10).is_empty(),
            "it read something out of rubbish"
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Module {
    pub name: String,
    pub what: String,

    pub state: &'static str,
    pub said: String,
    pub fix: String,

    pub picks: Option<&'static str>,
}

const ENGINE: [(&str, &str); 6] = [
    (
        "dossier-replay",
        "Разбор .osr: вся доступная информация из архива",
    ),
    (
        "dossier-beatmap",
        "Разбор .osu и .osz: объекты, тайминг, расположение объектов",
    ),
    (
        "dossier-sim",
        "Симуляция игрового процесса: попытка визуализировать геймплей",
    ),
    (
        "dossier-render",
        "Игровые элементы: скины, курсор, слайдеры, судейство, интерфейс",
    ),
    (
        "dossier-audio",
        "Хитсаунды, песня и выстраивание композиции",
    ),
    ("dossier-produce", "Сцена, кодирование, поиск карт и скинов"),
];

pub fn modules(said: &Settings) -> Vec<Module> {
    let version = env!("CARGO_PKG_VERSION");
    let mut out: Vec<Module> = ENGINE
        .iter()
        .map(|(name, what)| Module {
            name: (*name).to_owned(),
            what: (*what).to_owned(),
            state: "builtin",
            said: format!("встроен · {version}"),
            fix: String::new(),
            picks: None,
        })
        .collect();

    let ffmpeg = crate::check::on_path("ffmpeg");
    out.push(Module {
        name: "ffmpeg".to_owned(),
        what: "Склеивает кадры в видео и достаёт звук из карт".to_owned(),
        state: if ffmpeg.is_some() { "found" } else { "missing" },
        said: ffmpeg.map_or_else(
            || "Не найден в PATH".to_owned(),
            |path| path.display().to_string(),
        ),
        fix: "brew install ffmpeg · apt install ffmpeg · ffmpeg.org/download".to_owned(),
        picks: None,
    });

    let font = dossier_produce::font::find(None).ok().flatten();
    out.push(Module {
        name: "Шрифт".to_owned(),
        what: "Цифры и подписи на кадре".to_owned(),
        state: if font.is_some() { "found" } else { "missing" },
        said: if font.is_some() {
            "найден".to_owned()
        } else {
            "не найден — рисуется без цифр".to_owned()
        },
        fix: "положите .ttf рядом с приложением или назовите его в DOSSIER_FONT".to_owned(),
        picks: None,
    });

    let shelves = look(said);
    for (name, what, shelf, picks) in [
        (
            "Карты",
            "Папка Songs — по ней ищется карта реплея",
            &shelves.songs,
            "songs",
        ),
        (
            "Скины",
            "Чем рисовать, кроме встроенного",
            &shelves.skins,
            "skins",
        ),
        (
            "Реплеи",
            "Что предлагать к рендеру",
            &shelves.replays,
            "replays",
        ),
    ] {
        out.push(Module {
            name: name.to_owned(),
            what: what.to_owned(),
            state: if shelf.exists { "found" } else { "missing" },
            said: if shelf.exists {
                format!("{} · {}", shelf.note, shelf.path)
            } else {
                shelf.note.clone()
            },
            fix: "Укажите папку — «Обзор» рядом с полем".to_owned(),
            picks: Some(picks),
        });
    }

    out.push(Module {
        name: "Плагины".to_owned(),
        what: "Чужие модули: свои движки, свои сцены, свои выходные форматы".to_owned(),
        state: "planned",
        said: "заложено, но ещё не грузится".to_owned(),
        fix: "форма модуля здесь и есть подготовка к этому".to_owned(),
        picks: None,
    });
    out
}

pub fn install_skin(said: &Settings, archive: &Path) -> Result<String, String> {
    if said.skins.is_empty() {
        return Err("Папка скинов не указана: укажите её в настройках".to_owned());
    }
    let shelf = Path::new(&said.skins);
    std::fs::create_dir_all(shelf).map_err(|why| format!("Папка скинов не создалась: {why}"))?;

    let name = archive
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .filter(|stem| !stem.is_empty())
        .ok_or("У файла нет имени")?;
    let into = shelf.join(&name);
    if into.exists() {
        return Err(format!(
            "Скин «{name}» уже стоит. Для начала уберите старый"
        ));
    }

    let bytes = std::fs::read(archive).map_err(|why| format!("Файл не читается: {why}"))?;
    if let Err(why) = dossier_produce::skin::unpack(&bytes, &into) {
        let _ = std::fs::remove_dir_all(&into);
        return Err(format!("Архив не распаковался: {why}"));
    }
    flatten(&into);
    Ok(name)
}

pub fn droppable(shelf: &Path, file: &Path) -> Result<(), String> {
    if file
        .extension()
        .is_none_or(|e| !e.eq_ignore_ascii_case("osr"))
    {
        return Err("Это не файл реплея".to_owned());
    }
    let both = shelf.canonicalize().ok().zip(file.canonicalize().ok());
    match both {
        Some((shelf, file)) if file.starts_with(&shelf) => Ok(()),
        _ => Err("Реплей лежит не в папке реплеев — уберите его сами".to_owned()),
    }
}

pub fn drop_replay(said: &Settings, file: &Path) -> Result<(), String> {
    droppable(Path::new(&said.replays), file)?;
    std::fs::remove_file(file).map_err(|why| format!("Реплей не убрался: {why}"))
}

fn free_name(shelf: &Path, wanted: &str) -> String {
    if !shelf.join(wanted).exists() {
        return wanted.to_owned();
    }
    for n in 2..100 {
        let tried = format!("{wanted} ({n})");
        if !shelf.join(&tried).exists() {
            return tried;
        }
    }
    format!("{wanted} ({})", std::process::id())
}

fn copy_tree(from: &Path, into: &Path) -> Result<(), String> {
    std::fs::create_dir_all(into).map_err(|why| format!("{why}"))?;
    for entry in entries(from) {
        let Some(name) = entry.file_name() else {
            continue;
        };
        let landing = into.join(name);
        if entry.is_dir() {
            copy_tree(&entry, &landing)?;
        } else {
            std::fs::copy(&entry, &landing).map_err(|why| format!("{why}"))?;
        }
    }
    Ok(())
}

fn one_skin(said: &Settings, name: &str) -> Result<PathBuf, String> {
    if said.skins.is_empty() {
        return Err("Папка скинов не указана: укажите её в настройках".to_owned());
    }
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("Такого скина нет".to_owned());
    }
    let folder = Path::new(&said.skins).join(name);
    if folder.is_dir() {
        Ok(folder)
    } else {
        Err(format!("Скина «{name}» нет в папке скинов"))
    }
}

pub fn clone_skin(said: &Settings, name: &str) -> Result<String, String> {
    let from = one_skin(said, name)?;
    let shelf = Path::new(&said.skins);
    let made = free_name(shelf, &format!("{name} копия"));
    let into = shelf.join(&made);
    if let Err(why) = copy_tree(&from, &into) {
        let _ = std::fs::remove_dir_all(&into);
        return Err(format!("Скин не скопировался: {why}"));
    }
    Ok(made)
}

pub fn export_skin(said: &Settings, name: &str, into: &Path) -> Result<String, String> {
    let from = one_skin(said, name)?;
    if !into.is_dir() {
        return Err("Некуда класть: такой папки нет".to_owned());
    }
    let file = into.join(format!("{name}.osk"));
    dossier_produce::skin::pack(&from, &file).map_err(|why| format!("Архив не собрался: {why}"))?;
    Ok(file.display().to_string())
}

pub fn remove_skin(said: &Settings, name: &str) -> Result<(), String> {
    let folder = one_skin(said, name)?;
    std::fs::remove_dir_all(&folder).map_err(|why| format!("Скин не убрался: {why}"))
}

fn flatten(into: &Path) {
    if into.join("skin.ini").exists() {
        return;
    }
    let mut inside = entries(into);
    if inside.len() != 1 || !inside[0].is_dir() {
        return;
    }
    let only = inside.pop().expect("the one directory");
    for item in entries(&only) {
        let Some(name) = item.file_name() else {
            continue;
        };
        let _ = std::fs::rename(&item, into.join(name));
    }
    let _ = std::fs::remove_dir(&only);
}

#[cfg(test)]
mod skin_tests {
    use super::*;

    fn a_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut out));
            for (name, body) in entries {
                zip.start_file(*name, zip::write::SimpleFileOptions::default())
                    .expect("an entry");
                std::io::Write::write_all(&mut zip, body).expect("bytes");
            }
            zip.finish().expect("a zip");
        }
        out
    }

    fn a_shelf(name: &str) -> (tempish::Dir, Settings) {
        let dir = tempish::Dir::new(name);
        let said = Settings {
            skins: dir.path().join("Skins").display().to_string(),
            ..Settings::default()
        };
        (dir, said)
    }

    #[test]
    fn an_osk_becomes_a_folder_named_after_it() {
        let (dir, said) = a_shelf("osk-plain");
        let file = dir.path().join("Nice Skin.osk");
        std::fs::write(&file, a_zip(&[("skin.ini", b"[General]")])).expect("an archive");
        let name = install_skin(&said, &file).expect("it installed");
        assert_eq!(name, "Nice Skin");
        assert!(Path::new(&said.skins).join("Nice Skin/skin.ini").is_file());
    }

    #[test]
    fn an_archive_packed_as_one_folder_is_lifted_out_of_it() {
        let (dir, said) = a_shelf("osk-nested");
        let file = dir.path().join("Deep.osk");
        std::fs::write(
            &file,
            a_zip(&[("Deep/skin.ini", b"[General]"), ("Deep/cursor.png", b"x")]),
        )
        .expect("an archive");
        install_skin(&said, &file).expect("it installed");
        let shelf = Path::new(&said.skins).join("Deep");
        assert!(shelf.join("skin.ini").is_file(), "skin.ini did not come up");
        assert!(shelf.join("cursor.png").is_file());
    }

    #[test]
    fn a_name_already_on_the_shelf_is_left_alone() {
        let (dir, said) = a_shelf("osk-twice");
        let file = dir.path().join("Same.osk");
        std::fs::write(&file, a_zip(&[("skin.ini", b"[General]")])).expect("an archive");
        install_skin(&said, &file).expect("the first one");
        let again = install_skin(&said, &file);
        assert!(again.is_err(), "it overwrote a skin somebody had");
    }
}

#[cfg(test)]
mod tempish {
    use std::path::{Path, PathBuf};

    pub struct Dir(PathBuf);

    impl Dir {
        pub fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("dossier-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("a scratch directory");
            Self(path)
        }

        pub fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
