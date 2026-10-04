use std::io::Read;
use std::path::{Component, Path, PathBuf};

use crate::library::{self, Map};

const MAP_LIMIT: u64 = 16 * 1024 * 1024;
const MEDIA_LIMIT: u64 = 256 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    Read,
    Map,
    Mode,
    Save,
}

pub fn read_pool(path: &Path, now: i64) -> Result<crate::pools::Pool, crate::pool_share::Refused> {
    let bytes = bounded(path, 256 * 1024).map_err(|_| crate::pool_share::Refused::NotAPool)?;
    crate::pool_share::from_file(&bytes, now)
}

fn bounded(path: &Path, limit: u64) -> Result<Vec<u8>, ()> {
    if !std::fs::metadata(path).map_err(|_| ())?.is_file() {
        return Err(());
    }
    let file = std::fs::File::open(path).map_err(|_| ())?;
    if !file.metadata().map_err(|_| ())?.is_file() {
        return Err(());
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes).map_err(|_| ())?;
    if bytes.len() as u64 > limit { Err(()) } else { Ok(bytes) }
}

fn relative(name: &str) -> Option<PathBuf> {
    let name = name.replace('\\', "/");
    if name.is_empty() || name.contains(':') {
        return None;
    }
    let path = PathBuf::from(name);
    path.components().all(|part| matches!(part, Component::Normal(_) | Component::CurDir)).then_some(path)
}

fn media(root: &Path, stage: &Path, name: &str, extensions: &[&str]) -> Result<(), Refused> {
    let Some(relative) = relative(name) else { return Ok(()) };
    if !relative.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| extensions.iter().any(|allowed| ext.eq_ignore_ascii_case(allowed))) {
        return Ok(());
    }
    let mut source = root.to_path_buf();
    for part in relative.components() {
        let Component::Normal(name) = part else { continue };
        let exact = source.join(name);
        source = if exact.exists() { exact } else {
            let Some(found) = std::fs::read_dir(&source).ok().and_then(|entries| entries.flatten().find(|entry| entry.file_name().to_string_lossy().to_lowercase() == name.to_string_lossy().to_lowercase())) else { return Ok(()) };
            found.path()
        };
    }
    let Ok(source) = source.canonicalize() else { return Ok(()) };
    if !source.starts_with(root) {
        return Ok(());
    }
    let bytes = bounded(&source, MEDIA_LIMIT).map_err(|_| Refused::Save)?;
    let target = stage.join(relative);
    std::fs::create_dir_all(target.parent().ok_or(Refused::Save)?).map_err(|_| Refused::Save)?;
    std::fs::write(target, bytes).map_err(|_| Refused::Save)
}

pub fn import_map(path: &Path, songs: &Path) -> Result<(String, Map), Refused> {
    let bytes = bounded(path, MAP_LIMIT).map_err(|_| Refused::Read)?;
    let text = String::from_utf8_lossy(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes));
    let parsed = dossier_beatmap::Beatmap::parse(&text).map_err(|_| Refused::Map)?;
    if parsed.mode != 0 {
        return Err(Refused::Mode);
    }
    if parsed.metadata.title.trim().is_empty() || parsed.objects.is_empty() {
        return Err(Refused::Map);
    }
    let hash = library::md5_hex(&bytes);
    let folder = songs.join(format!("Dossier-{hash}"));
    let target = folder.join("map.osu");
    if library::describe(&target).is_some_and(|(found, _)| found == hash) {
        let root = path.parent().unwrap_or(Path::new(".")).canonicalize().map_err(|_| Refused::Read)?;
        media(&root, &folder, &parsed.audio_filename, &["mp3", "ogg", "wav", "flac", "m4a", "opus"])?;
        if let Some(background) = &parsed.background {
            media(&root, &folder, background, &["jpg", "jpeg", "png", "bmp", "gif", "webp"])?;
        }
        return library::describe(&target).ok_or(Refused::Map);
    }
    std::fs::create_dir_all(songs).map_err(|_| Refused::Save)?;
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let staging = songs.parent().ok_or(Refused::Save)?.join(".pool-imports");
    std::fs::create_dir_all(&staging).map_err(|_| Refused::Save)?;
    let stage = staging.join(format!("{}-{serial}", std::process::id()));
    std::fs::create_dir(&stage).map_err(|_| Refused::Save)?;
    let result = (|| {
        let root = path.parent().unwrap_or(Path::new(".")).canonicalize().map_err(|_| Refused::Read)?;
        std::fs::write(stage.join("map.osu"), &bytes).map_err(|_| Refused::Save)?;
        media(&root, &stage, &parsed.audio_filename, &["mp3", "ogg", "wav", "flac", "m4a", "opus"])?;
        if let Some(background) = parsed.background {
            media(&root, &stage, &background, &["jpg", "jpeg", "png", "bmp", "gif", "webp"])?;
        }
        std::fs::rename(&stage, &folder).map_err(|_| Refused::Save)?;
        library::describe(&target).ok_or(Refused::Map)
    })();
    let _ = std::fs::remove_dir_all(&stage);
    result
}

pub fn import_archive(path: &Path, songs: &Path) -> Result<Vec<(String, Map)>, Refused> {
    let file = std::fs::File::open(path).map_err(|_| Refused::Read)?;
    if !file.metadata().map_err(|_| Refused::Read)?.is_file() || file.metadata().map_err(|_| Refused::Read)?.len() > MEDIA_LIMIT {
        return Err(Refused::Read);
    }
    let mut archive = zip::ZipArchive::new(file).map_err(|_| Refused::Map)?;
    if archive.len() > 4096 { return Err(Refused::Map); }
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let stage = songs.parent().ok_or(Refused::Save)?.join(".pool-imports").join(format!("archive-{}-{serial}", std::process::id()));
    std::fs::create_dir_all(&stage).map_err(|_| Refused::Save)?;
    struct Staging(PathBuf);
    impl Drop for Staging { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
    let _staging = Staging(stage.clone());
    let mut charts = Vec::new();
    let mut total = 0u64;
    for at in 0..archive.len() {
        let mut entry = archive.by_index(at).map_err(|_| Refused::Map)?;
        if entry.is_dir() { continue; }
        let relative = relative(entry.name()).ok_or(Refused::Map)?;
        if entry.unix_mode().is_some_and(|mode| mode & 0o170000 == 0o120000) { return Err(Refused::Map); }
        let extension = relative.extension().and_then(|extension| extension.to_str()).unwrap_or_default().to_ascii_lowercase();
        if !["osu", "jpg", "jpeg", "png", "bmp", "gif", "webp", "mp3", "ogg", "wav", "flac", "m4a", "opus"].contains(&extension.as_str()) { continue; }
        let limit = if extension == "osu" { MAP_LIMIT } else { MEDIA_LIMIT };
        total = total.checked_add(entry.size()).ok_or(Refused::Map)?;
        if entry.size() > limit || total > 512 * 1024 * 1024 { return Err(Refused::Map); }
        let target = stage.join(relative);
        std::fs::create_dir_all(target.parent().ok_or(Refused::Save)?).map_err(|_| Refused::Save)?;
        let mut out = std::fs::OpenOptions::new().write(true).create_new(true).open(&target).map_err(|_| Refused::Map)?;
        let written = std::io::copy(&mut entry.by_ref().take(limit + 1), &mut out).map_err(|_| Refused::Save)?;
        if written > limit { return Err(Refused::Map); }
        if extension == "osu" { charts.push(target); }
    }
    charts.sort();
    let mut maps = Vec::new();
    let mut foreign = false;
    let mut seen = std::collections::HashSet::new();
    for chart in charts {
        match import_map(&chart, songs) {
            Ok((hash, map)) if seen.insert(hash.clone()) => maps.push((hash, map)),
            Ok(_) | Err(Refused::Map) => {},
            Err(Refused::Mode) => foreign = true,
            Err(why) => return Err(why),
        }
    }
    if maps.is_empty() { return Err(if foreign { Refused::Mode } else { Refused::Map }); }
    maps.sort_by(|a, b| a.1.version.cmp(&b.1.version).then(a.0.cmp(&b.0)));
    Ok(maps)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("dossier-pool-files-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn chart() -> String {
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../crates/dossier-assay/corpus/maps/5114204.osu")).unwrap()
    }

    #[test]
    fn imported_chart_and_referenced_media_survive_the_original_folder() {
        let root = scratch("copy");
        let source = root.join("source");
        std::fs::create_dir_all(source.join("art")).unwrap();
        let chart = chart().lines().filter(|line| !line.starts_with("AudioFilename:") && !line.starts_with("0,0,")).collect::<Vec<_>>().join("\n")
            .replace("[HitObjects]", "[General]\nAudioFilename: song.mp3\n[Events]\n0,0,\"art/bg.jpg\",0,0\n[HitObjects]");
        std::fs::write(source.join("chart.OSU"), &chart).unwrap();
        std::fs::write(source.join("song.mp3"), b"audio").unwrap();
        std::fs::write(source.join("art/bg.jpg"), b"picture").unwrap();
        std::fs::write(source.join("personal.txt"), b"private").unwrap();
        let songs = root.join("Songs");
        let (hash, map) = import_map(&source.join("chart.OSU"), &songs).unwrap();
        assert_eq!(hash, library::md5_hex(chart.as_bytes()));
        assert_eq!(std::fs::read(map.folder().join("song.mp3")).unwrap(), b"audio");
        assert_eq!(std::fs::read(map.background.as_ref().unwrap()).unwrap(), b"picture");
        assert!(!map.folder().join("personal.txt").exists());
        assert_eq!(import_map(&source.join("chart.OSU"), &songs).unwrap().1.file, map.file);
        std::fs::remove_dir_all(source).unwrap();
        assert!(crate::pools::measure(&map, &hash, crate::pools::Mod::Hr).is_ok());
        assert_eq!(library::describe(&map.file).unwrap().0, hash);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reimporting_an_osu_file_recovers_media_with_different_filename_case() {
        let root = scratch("recover-art");
        let source = root.join("source");
        std::fs::create_dir_all(&source).unwrap();
        let chart = chart().lines().filter(|line| !line.starts_with("AudioFilename:") && !line.starts_with("0,0,")).collect::<Vec<_>>().join("\n")
            .replace("[HitObjects]", "[Events]\n0,0,\"art/bg.jpg\",0,0\n[HitObjects]");
        let path = source.join("chart.osu");
        std::fs::write(&path, &chart).unwrap();
        let songs = root.join("Songs");
        let (_, first) = import_map(&path, &songs).unwrap();
        assert!(first.background.is_none());
        std::fs::create_dir_all(source.join("ART")).unwrap();
        std::fs::write(source.join("ART/BG.JPG"), b"picture").unwrap();
        let (_, repaired) = import_map(&path, &songs).unwrap();
        assert_eq!(std::fs::read(repaired.background.unwrap()).unwrap(), b"picture");
        std::fs::remove_dir_all(root).unwrap();
    }

    fn pack(path: &Path, files: &[(&str, &[u8])]) {
        use std::io::Write;
        let mut archive = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for (name, bytes) in files {
            archive.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            archive.write_all(bytes).unwrap();
        }
        archive.finish().unwrap();
    }

    #[test]
    fn an_osz_imports_standard_difficulties_and_media_and_can_be_imported_again() {
        let root = scratch("archive");
        let path = root.join("set.OSZ");
        let standard = chart().lines().filter(|line| !line.starts_with("AudioFilename:") && !line.starts_with("0,0,")).collect::<Vec<_>>().join("\n")
            .replace("[HitObjects]", "[General]\nAudioFilename: song.mp3\n[Events]\n0,0,\"art/bg.jpg\",0,0\n[HitObjects]");
        let second = format!("{standard}\n");
        let foreign = format!("{standard}\n[General]\nMode: 3\n");
        pack(&path, &[("first.osu", standard.as_bytes()), ("second.OSU", second.as_bytes()), ("mania.osu", foreign.as_bytes()), ("art/bg.jpg", b"picture"), ("song.mp3", b"audio"), ("personal.txt", b"private")]);
        let songs = root.join("Songs");
        let maps = import_archive(&path, &songs).unwrap();
        assert_eq!(maps.len(), 2);
        assert_eq!(import_archive(&path, &songs).unwrap(), maps);
        for (hash, map) in &maps {
            assert_eq!(std::fs::read(map.folder().join("song.mp3")).unwrap(), b"audio");
            assert_eq!(std::fs::read(map.background.as_ref().unwrap()).unwrap(), b"picture");
            assert!(!map.folder().join("personal.txt").exists());
            assert!(crate::pools::measure(map, hash, crate::pools::Mod::Dt).is_ok());
        }
        assert_eq!(std::fs::read_dir(root.join(".pool-imports")).unwrap().count(), 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn malformed_empty_foreign_and_escaping_archives_leave_no_import_staging() {
        let root = scratch("archive-refused");
        let path = root.join("set.osz");
        std::fs::write(&path, b"not a zip").unwrap();
        assert_eq!(import_archive(&path, &root.join("Songs")), Err(Refused::Map));
        pack(&path, &[("../escaped.osu", chart().as_bytes())]);
        assert_eq!(import_archive(&path, &root.join("Songs")), Err(Refused::Map));
        assert!(!root.join("escaped.osu").exists());
        pack(&path, &[("picture.jpg", b"picture")]);
        assert_eq!(import_archive(&path, &root.join("Songs")), Err(Refused::Map));
        let foreign = format!("{}\n[General]\nMode: 3\n", chart());
        pack(&path, &[("mania.osu", foreign.as_bytes())]);
        assert_eq!(import_archive(&path, &root.join("Songs")), Err(Refused::Mode));
        assert_eq!(std::fs::read_dir(root.join(".pool-imports")).unwrap().count(), 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_pool_file_is_read_by_the_same_exchange_rules() {
        let root = scratch("pool");
        let path = root.join("example.pool");
        let pool = crate::pools::Pool::new(crate::pools::Frame::Duel, "Test pool", 100);
        std::fs::write(&path, crate::pool_share::to_file(&pool)).unwrap();
        let opened = read_pool(&path, 200).unwrap();
        assert_eq!(opened.name, pool.name);
        assert_ne!(opened.id, pool.id);
        assert_eq!(opened.made_at, 200);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn foreign_modes_bad_files_and_escaping_media_are_rejected() {
        let root = scratch("refused");
        let path = root.join("map.osu");
        std::fs::write(&path, "not a beatmap").unwrap();
        assert_eq!(import_map(&path, &root.join("Songs")), Err(Refused::Map));
        std::fs::write(&path, format!("{}\n[General]\nMode: 3\n", chart())).unwrap();
        assert_eq!(import_map(&path, &root.join("Songs")), Err(Refused::Mode));
        assert!(!root.join("Songs").exists());
        for name in ["../private.jpg", "/private.jpg", "C:\\private.jpg", "art/../../private.jpg"] {
            assert!(relative(name).is_none());
        }
        assert_eq!(relative("art\\bg.jpg"), Some(PathBuf::from("art/bg.jpg")));
        let large = root.join("large.pool");
        std::fs::write(&large, vec![b' '; 256 * 1024 + 1]).unwrap();
        assert_eq!(read_pool(&large, 1), Err(crate::pool_share::Refused::NotAPool));
        std::fs::remove_dir_all(root).unwrap();
    }
}
