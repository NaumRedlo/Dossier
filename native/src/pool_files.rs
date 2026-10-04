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
    let Ok(source) = root.join(&relative).canonicalize() else { return Ok(()) };
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
    if let Some((found, map)) = library::describe(&target).filter(|(found, _)| *found == hash) {
        return Ok((found, map));
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
