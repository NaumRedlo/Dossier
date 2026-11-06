use std::io::Read;
use std::path::Path;

pub const TOO_MANY_FILES: &str = "the archive has too many files";
pub const TOO_LARGE: &str = "the archive unpacks too large";
pub const BAD_PATH: &str = "the archive has a path outside its folder";
pub const LINK: &str = "the archive has a symbolic link";

pub fn key(why: &str) -> Option<&'static str> {
    match why {
        TOO_MANY_FILES => Some("archive-too-many-files"),
        TOO_LARGE => Some("archive-too-large"),
        BAD_PATH => Some("archive-bad-path"),
        LINK => Some("archive-link"),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub files: usize,
    pub bytes: u64,
}

pub const MAP_SET: Limits = Limits { files: 5000, bytes: 2 * 1024 * 1024 * 1024 };
pub const UPDATE: Limits = Limits { files: 20000, bytes: 1024 * 1024 * 1024 };

const KIND: u32 = 0o170000;
const SYMLINK: u32 = 0o120000;

pub fn unpack(archive: &Path, into: &Path, limits: Limits) -> Result<usize, String> {
    let file = std::fs::File::open(archive).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    if zip.len() > limits.files {
        return Err(TOO_MANY_FILES.to_owned());
    }
    std::fs::create_dir_all(into).map_err(|e| e.to_string())?;
    let mut remaining = limits.bytes;
    let mut written = 0;
    for at in 0..zip.len() {
        let mut entry = zip.by_index(at).map_err(|e| e.to_string())?;
        if entry.is_symlink() || entry.unix_mode().is_some_and(|mode| mode & KIND == SYMLINK) {
            return Err(LINK.to_owned());
        }
        let relative = entry.enclosed_name().ok_or(BAD_PATH)?;
        let target = into.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
            continue;
        }
        if entry.size() > remaining {
            return Err(TOO_LARGE.to_owned());
        }
        if let Some(dir) = target.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let runnable = entry.unix_mode().is_some_and(|mode| mode & 0o111 != 0);
        let mut out = std::fs::File::create(&target).map_err(|e| e.to_string())?;
        let copied = std::io::copy(&mut (&mut entry).take(remaining + 1), &mut out).map_err(|e| e.to_string())?;
        remaining = remaining.checked_sub(copied).ok_or(TOO_LARGE)?;
        drop(out);
        settle_mode(&target, runnable)?;
        written += 1;
    }
    Ok(written)
}

#[cfg(unix)]
fn settle_mode(path: &Path, runnable: bool) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mode = if runnable { 0o755 } else { 0o644 };
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).map_err(|e| e.to_string())
}

#[cfg(not(unix))]
fn settle_mode(_: &Path, _: bool) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;

    fn place(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("dossier-unzip-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn archive(root: &Path, entries: &[(&str, &[u8], u32)]) -> PathBuf {
        let path = root.join("a.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        for (name, body, mode) in entries {
            let options = zip::write::SimpleFileOptions::default().unix_permissions(*mode);
            zip.start_file(*name, options).unwrap();
            zip.write_all(body).unwrap();
        }
        zip.finish().unwrap();
        path
    }

    #[test]
    fn files_unpack_with_only_the_run_bit_kept_from_their_modes() {
        let root = place("modes");
        let made = archive(&root, &[("app/run", b"binary", 0o4755), ("app/notes.txt", b"text", 0o666)]);
        assert_eq!(unpack(&made, &root.join("out"), MAP_SET).unwrap(), 2);
        assert_eq!(std::fs::read(root.join("out/app/run")).unwrap(), b"binary");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(root.join("out/app/run")).unwrap().permissions().mode() & 0o7777, 0o755);
            assert_eq!(std::fs::metadata(root.join("out/app/notes.txt")).unwrap().permissions().mode() & 0o7777, 0o644);
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_symbolic_link_refuses_the_whole_archive() {
        let root = place("link");
        let path = root.join("a.zip");
        {
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
            zip.add_symlink("escape", "/etc", zip::write::SimpleFileOptions::default()).unwrap();
            zip.finish().unwrap();
        }
        assert_eq!(unpack(&path, &root.join("out"), MAP_SET).unwrap_err(), LINK);
        assert!(!root.join("out/escape").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_path_out_of_the_folder_is_refused() {
        let root = place("path");
        let made = archive(&root, &[("../outside.txt", b"x", 0o644)]);
        assert_eq!(unpack(&made, &root.join("out"), MAP_SET).unwrap_err(), BAD_PATH);
        assert!(!root.join("outside.txt").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn every_refusal_has_words() {
        for why in [TOO_MANY_FILES, TOO_LARGE, BAD_PATH, LINK] {
            assert!(key(why).is_some(), "{why}");
        }
        assert!(key("the mirror timed out").is_none());
    }

    #[test]
    fn too_many_files_or_too_many_bytes_are_refused() {
        let root = place("limits");
        let made = archive(&root, &[("a", &[0u8; 600], 0o644), ("b", &[0u8; 600], 0o644)]);
        assert_eq!(unpack(&made, &root.join("one"), Limits { files: 1, bytes: 1 << 20 }).unwrap_err(), TOO_MANY_FILES);
        assert_eq!(unpack(&made, &root.join("small"), Limits { files: 10, bytes: 1000 }).unwrap_err(), TOO_LARGE);
        assert_eq!(unpack(&made, &root.join("fits"), Limits { files: 2, bytes: 1200 }).unwrap(), 2);
        let _ = std::fs::remove_dir_all(&root);
    }
}
