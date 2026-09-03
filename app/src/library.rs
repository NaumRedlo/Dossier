//! What is on this machine's shelves: maps, skins, replays.
//!
//! Counted rather than trusted. A folder somebody typed into a settings screen
//! is a guess until something has looked inside it, and "no maps here" said
//! before a render is worth more than a job handed back after one.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::settings::Settings;

/// One folder, and what was found in it.
#[derive(Debug, Clone, Serialize)]
pub struct Shelf {
    pub path: String,
    pub exists: bool,
    pub items: usize,
    pub bytes: u64,
    /// What it holds, in the words somebody would use.
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

/// Every shelf at once.
#[derive(Debug, Clone, Serialize)]
pub struct Library {
    pub songs: Shelf,
    pub skins: Shelf,
    pub replays: Shelf,
}

/// A replay this person could ask to have drawn.
///
/// Read from the file's own header rather than from its name: a name is
/// whatever the client wrote, and two of them collide the moment somebody
/// plays the same map twice.
#[derive(Debug, Clone, Serialize)]
pub struct Played {
    pub path: String,
    pub file: String,
    pub player: String,
    pub mods: String,
    pub score: i32,
    pub combo: u16,
    pub bytes: u64,
    /// Whether the map it was played on is on this machine.
    pub have_map: bool,
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

/// Maps: archives as they are downloaded, and folders as osu! unpacks them.
fn shelve_songs(path: &str) -> Shelf {
    let at = Path::new(path);
    if path.is_empty() || !at.is_dir() {
        return Shelf::missing(path, "папки нет — карты качать будет некуда");
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

/// Skins: a folder is one when it has a `skin.ini` or any picture in it.
///
/// Both, because a skin with no `skin.ini` is perfectly ordinary — the file is
/// optional and osu! falls back to its defaults for every key.
fn shelve_skins(path: &str) -> Shelf {
    let at = Path::new(path);
    if path.is_empty() || !at.is_dir() {
        return Shelf::missing(path, "не указана — рисуем своим скином");
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
        return Shelf::missing(path, "не указана — рисовать своё будет нечего");
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

/// The skins on the shelf, by folder name, in the order a list should show them.
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

/// The replays on the shelf, newest first, with the map's presence checked.
///
/// Capped: a folder somebody has been playing out of for a year holds thousands,
/// and a list nobody can scroll is not a list. The cap is stated rather than
/// silent — see the caller.
pub fn played(said: &Settings, most: usize) -> Vec<Played> {
    let at = Path::new(&said.replays);
    if said.replays.is_empty() || !at.is_dir() {
        return Vec::new();
    }
    let mut files: Vec<_> = entries(at)
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("osr")))
        .filter_map(|p| {
            let when = p.metadata().ok()?.modified().ok()?;
            Some((when, p))
        })
        .collect();
    files.sort_by_key(|(when, _)| std::cmp::Reverse(*when));
    files.truncate(most);

    let songs = Path::new(&said.songs);
    files
        .into_iter()
        .filter_map(|(_, path)| {
            let bytes = std::fs::read(&path).ok()?;
            let replay = dossier_replay::Replay::parse(&bytes).ok()?;
            // Whether the map is here decides whether this can be drawn at all,
            // and finding out now costs one search rather than one failed render.
            let have_map = dossier_produce::locate::search_dir(songs, &replay.beatmap_hash)
                .ok()
                .flatten()
                .is_some();
            Some(Played {
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
        .collect()
}

#[cfg(test)]
mod tests {
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
        // An empty folder is not a map, whatever it is called.
        std::fs::create_dir(dir.join("empty")).expect("a folder");

        let shelf = shelve_songs(&dir.display().to_string());
        assert!(shelf.exists);
        assert_eq!(shelf.items, 3, "{}", shelf.note);
        assert!(shelf.note.contains("2 архива"), "{}", shelf.note);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// A skin with no `skin.ini` is perfectly ordinary — the file is optional.
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

    /// A file that is not a replay is skipped rather than breaking the list.
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

/// One piece of what the application is made of, or needs.
///
/// The tab that shows these is not a status page: it is the shelf. What is
/// built in cannot be removed, what is outside can be missing and says how to
/// stop being missing, and what is planned says so rather than pretending.
#[derive(Debug, Clone, Serialize)]
pub struct Module {
    pub name: String,
    pub what: String,
    /// `builtin`, `found`, `missing`, `planned`.
    pub state: &'static str,
    pub said: String,
    pub fix: String,
    /// A folder this module lives in, when picking one is the way to fix it.
    pub picks: Option<&'static str>,
}

const ENGINE: [(&str, &str); 6] = [
    (
        "dossier-replay",
        "разбор .osr: путь курсора и что было нажато",
    ),
    (
        "dossier-beatmap",
        "разбор .osu и .osz: объекты, тайминг, кривые слайдеров",
    ),
    (
        "dossier-sim",
        "реплей против карты: что на самом деле произошло",
    ),
    (
        "dossier-render",
        "кадры: скины, курсор, слайдеры, судейство, интерфейс",
    ),
    ("dossier-audio", "хитсаунды, песня и смесь из них"),
    ("dossier-produce", "сцена, кодирование, поиск карт и скинов"),
];

/// What is on the shelf, and what is missing from it.
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
        what: "склеивает кадры в видео и достаёт звук из карт".to_owned(),
        state: if ffmpeg.is_some() { "found" } else { "missing" },
        said: ffmpeg.map_or_else(
            || "не найден в PATH".to_owned(),
            |path| path.display().to_string(),
        ),
        fix: "brew install ffmpeg · apt install ffmpeg · ffmpeg.org/download".to_owned(),
        picks: None,
    });

    let font = dossier_produce::font::find(None).ok().flatten();
    out.push(Module {
        name: "шрифт".to_owned(),
        what: "цифры и подписи на кадре".to_owned(),
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
            "карты",
            "папка Songs — по ней ищется карта реплея",
            &shelves.songs,
            "songs",
        ),
        (
            "скины",
            "чем рисовать, кроме встроенного",
            &shelves.skins,
            "skins",
        ),
        (
            "реплеи",
            "что предлагать к отрисовке",
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
            fix: "укажите папку — «Обзор» рядом с полем".to_owned(),
            picks: Some(picks),
        });
    }

    out.push(Module {
        name: "плагины".to_owned(),
        what: "чужие модули: свои судьи, свои сцены, свои выходные форматы".to_owned(),
        state: "planned",
        said: "заложено, но ещё не грузится".to_owned(),
        fix: "форма модуля здесь и есть подготовка к этому".to_owned(),
        picks: None,
    });
    out
}

/// Install an `.osk` — an osu! skin archive, which is a zip under another name.
///
/// Under the archive's own name, because that is the name somebody will look
/// for afterwards. A folder already called that is left exactly as it is and
/// said so: overwriting a skin because a download happened to share its name is
/// not a thing to do quietly.
pub fn install_skin(said: &Settings, archive: &Path) -> Result<String, String> {
    if said.skins.is_empty() {
        return Err("папка скинов не указана — назовите её в настройках".to_owned());
    }
    let shelf = Path::new(&said.skins);
    std::fs::create_dir_all(shelf).map_err(|why| format!("папка скинов не создалась: {why}"))?;

    let name = archive
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .filter(|stem| !stem.is_empty())
        .ok_or("у файла нет имени")?;
    let into = shelf.join(&name);
    if into.exists() {
        return Err(format!("скин «{name}» уже стоит — сначала уберите старый"));
    }

    let bytes = std::fs::read(archive).map_err(|why| format!("файл не читается: {why}"))?;
    if let Err(why) = dossier_produce::skin::unpack(&bytes, &into) {
        let _ = std::fs::remove_dir_all(&into);
        return Err(format!("архив не распаковался: {why}"));
    }
    flatten(&into);
    Ok(name)
}

/// Some archives are packed as one folder with everything inside it. osu!
/// treats the archive itself as the skin, so a lone directory is lifted out —
/// otherwise the skin is one level deeper than anything looks.
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

/// A directory that removes itself. `tempfile` for one purpose in one test
/// module is a dependency for a `Drop`.
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
