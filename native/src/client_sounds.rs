use std::path::{Path, PathBuf};

const LIBRARY: &str = "osu!gameplay.dll";
const INTERFACE: &str = "osu!ui.dll";
const MOD_ICON: &str = "selection-mod-";
const PICTURE: &[u8] = b"\x89PNG\r\n\x1a\n";
const STAMP: &str = "from.txt";
const RESOURCES: u32 = 0xBEEF_CACE;
const BYTES: u32 = 0x20;
const STREAM: u32 = 0x21;

pub fn folder() -> PathBuf {
    crate::sources::own_root().join("osu-sounds")
}

pub fn icons_folder() -> PathBuf {
    crate::sources::own_root().join("osu-icons")
}

fn u16_at(b: &[u8], at: usize) -> Option<usize> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?) as usize)
}

fn u32_at(b: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?) as usize)
}

fn seven_bit(b: &[u8], mut at: usize) -> Option<(usize, usize)> {
    let (mut value, mut shift) = (0usize, 0u32);
    loop {
        let byte = *b.get(at)?;
        at += 1;
        value |= usize::from(byte & 0x7F).checked_shl(shift)?;
        if byte & 0x80 == 0 {
            return Some((value, at));
        }
        shift += 7;
        if shift > 28 {
            return None;
        }
    }
}

fn offset_of(b: &[u8], rva: usize) -> Option<usize> {
    let pe = u32_at(b, 0x3C)?;
    let count = u16_at(b, pe + 6)?;
    let first = pe + 24 + u16_at(b, pe + 20)?;
    (0..count).find_map(|section| {
        let at = first + section * 40;
        let (virtual_size, address, raw_size, raw) = (u32_at(b, at + 8)?, u32_at(b, at + 12)?, u32_at(b, at + 16)?, u32_at(b, at + 20)?);
        (address <= rva && rva < address + virtual_size.max(raw_size)).then(|| raw + (rva - address))
    })
}

fn blobs(b: &[u8]) -> Vec<(usize, usize)> {
    let found = (|| {
        let pe = u32_at(b, 0x3C)?;
        let optional = pe + 24;
        let directories = optional + if u16_at(b, optional)? == 0x20B { 112 } else { 96 };
        let managed = offset_of(b, u32_at(b, directories + 14 * 8)?)?;
        let (rva, size) = (u32_at(b, managed + 24)?, u32_at(b, managed + 28)?);
        let mut at = offset_of(b, rva)?;
        let end = at + size;
        let mut out = Vec::new();
        while at + 4 <= end {
            let length = u32_at(b, at)?;
            let blob = at + 4;
            if u32_at(b, blob)? as u32 == RESOURCES {
                out.push((blob, length));
            }
            at = (blob + length + 3) & !3;
        }
        Some(out)
    })();
    found.unwrap_or_default()
}

fn items(b: &[u8], at: usize) -> Vec<(String, usize, Option<usize>)> {
    let found = (|| {
        let mut i = at + 8;
        i += 4 + u32_at(b, i)?;
        let (count, kinds) = (u32_at(b, i + 4)?, u32_at(b, i + 8)?);
        i += 12;
        for _ in 0..kinds {
            let (length, after) = seven_bit(b, i)?;
            i = after + length;
        }
        i += (8 - (i - at) % 8) % 8;
        i += 4 * count;
        let offsets: Vec<usize> = (0..count).map(|n| u32_at(b, i + 4 * n)).collect::<Option<_>>()?;
        i += 4 * count;
        let data = at + u32_at(b, i)?;
        let names = i + 4;
        let mut out = Vec::new();
        for offset in offsets {
            let (length, name_at) = seven_bit(b, names + offset)?;
            let units: Vec<u16> = b.get(name_at..name_at + length)?.chunks_exact(2).map(|pair| u16::from_le_bytes([pair[0], pair[1]])).collect();
            let value = data + u32_at(b, name_at + length)?;
            let (kind, body) = seven_bit(b, value)?;
            if kind as u32 == BYTES || kind as u32 == STREAM {
                out.push((String::from_utf16_lossy(&units), body + 4, Some(u32_at(b, body)?)));
            } else {
                out.push((String::from_utf16_lossy(&units), body, None));
            }
        }
        Some(out)
    })();
    found.unwrap_or_default()
}

fn kind_of(sound: &[u8]) -> Option<&'static str> {
    [(&b"RIFF"[..], "wav"), (b"OggS", "ogg"), (b"ID3", "mp3"), (b"\xff\xfb", "mp3")].into_iter().find_map(|(magic, kind)| sound.starts_with(magic).then_some(kind))
}

fn plain(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '@')
}

pub fn sounds_in(library: &[u8]) -> Vec<(String, &'static str, &[u8])> {
    let mut out = Vec::new();
    for (blob, _) in blobs(library) {
        for (name, at, size) in items(library, blob) {
            let Some(sound) = size.and_then(|size| library.get(at..at + size)) else {
                continue;
            };
            if let Some(kind) = kind_of(sound).filter(|_| plain(&name)) {
                out.push((name, kind, sound));
            }
        }
    }
    out
}

fn picture_from(b: &[u8], from: usize, until: usize) -> Option<&[u8]> {
    let room = b.get(from..until.min(b.len()))?;
    let begins = from + room.windows(PICTURE.len()).position(|window| window == PICTURE)?;
    let mut at = begins + PICTURE.len();
    while at + 8 <= b.len() {
        let length = u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?) as usize;
        let last = b.get(at + 4..at + 8)? == b"IEND";
        at += 12 + length;
        if last {
            return b.get(begins..at);
        }
    }
    None
}

pub fn mod_icons_in(library: &[u8]) -> Vec<(String, &'static str, &[u8])> {
    let mut out = Vec::new();
    for (blob, length) in blobs(library) {
        let held = items(library, blob);
        let mut starts: Vec<usize> = held.iter().map(|(_, at, _)| *at).collect();
        starts.sort_unstable();
        for (name, at, size) in &held {
            if !name.starts_with(MOD_ICON) || !plain(name) {
                continue;
            }
            let until = match size {
                Some(size) => at + size,
                None => starts.iter().copied().find(|start| start > at).unwrap_or(blob + length),
            };
            if let Some(picture) = picture_from(library, *at, until) {
                out.push((name.clone(), "png", picture));
            }
        }
    }
    out
}

fn stamp_of(library: &Path) -> Option<String> {
    let meta = std::fs::metadata(library).ok()?;
    let changed = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
    Some(format!("{}\n{}\n{changed}\n", library.display(), meta.len()))
}

type Found<'a> = Vec<(String, &'static str, &'a [u8])>;

pub fn take_from(library: &Path, into: &Path) -> Result<usize, String> {
    take(library, into, sounds_in)
}

fn take(library: &Path, into: &Path, wanted: for<'a> fn(&'a [u8]) -> Found<'a>) -> Result<usize, String> {
    let bytes = std::fs::read(library).map_err(|why| format!("{}: {why}", library.display()))?;
    let sounds = wanted(&bytes);
    if sounds.is_empty() {
        return Err(format!("{}: nothing wanted inside", library.display()));
    }
    let fresh = into.with_extension("part");
    let _ = std::fs::remove_dir_all(&fresh);
    std::fs::create_dir_all(&fresh).map_err(|why| format!("{}: {why}", fresh.display()))?;
    for (name, kind, sound) in &sounds {
        std::fs::write(fresh.join(format!("{name}.{kind}")), sound).map_err(|why| why.to_string())?;
    }
    std::fs::write(fresh.join(STAMP), stamp_of(library).unwrap_or_default()).map_err(|why| why.to_string())?;
    let _ = std::fs::remove_dir_all(into);
    std::fs::rename(&fresh, into).map_err(|why| format!("{}: {why}", into.display()))?;
    Ok(sounds.len())
}

fn named(sources: &[crate::sources::Source], leaf: &str) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = sources.iter().map(|source| source.root.clone()).collect();
    roots.extend(crate::sources::stable_roots());
    let mut seen = std::collections::HashSet::new();
    roots.into_iter().map(|root| root.join(leaf)).filter(|library| library.is_file() && seen.insert(library.clone())).collect()
}

pub fn libraries(sources: &[crate::sources::Source]) -> Vec<PathBuf> {
    named(sources, LIBRARY)
}

fn kept(libraries: &[PathBuf], into: PathBuf, wanted: for<'a> fn(&'a [u8]) -> Found<'a>) -> Option<PathBuf> {
    let held = std::fs::read_to_string(into.join(STAMP)).ok();
    if let Some(held) = &held {
        if libraries.is_empty() || libraries.iter().any(|library| stamp_of(library).as_deref() == Some(held.as_str())) {
            return Some(into);
        }
    }
    for library in libraries {
        if take(library, &into, wanted).is_ok() {
            return Some(into);
        }
    }
    held.map(|_| into)
}

pub fn ready(sources: &[crate::sources::Source]) -> Option<PathBuf> {
    kept(&libraries(sources), folder(), sounds_in)
}

pub fn icons_ready(sources: &[crate::sources::Source]) -> Option<PathBuf> {
    kept(&named(sources, INTERFACE), icons_folder(), mod_icons_in)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seven(mut value: usize) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let byte = (value & 0x7F) as u8;
            value >>= 7;
            out.push(if value > 0 { byte | 0x80 } else { byte });
            if value == 0 {
                return out;
            }
        }
    }

    fn resources(held: &[(&str, u32, &[u8])]) -> Vec<u8> {
        let mut names = Vec::new();
        let mut data = Vec::new();
        let mut offsets = Vec::new();
        for (name, kind, body) in held {
            offsets.push(names.len() as u32);
            let wide: Vec<u8> = name.encode_utf16().flat_map(u16::to_le_bytes).collect();
            names.extend(seven(wide.len()));
            names.extend(wide);
            names.extend((data.len() as u32).to_le_bytes());
            data.extend(seven(*kind as usize));
            data.extend((body.len() as u32).to_le_bytes());
            data.extend(*body);
        }
        let mut out = Vec::new();
        out.extend(RESOURCES.to_le_bytes());
        out.extend(1u32.to_le_bytes());
        out.extend(4u32.to_le_bytes());
        out.extend(*b"skip");
        out.extend(2u32.to_le_bytes());
        out.extend((held.len() as u32).to_le_bytes());
        out.extend(0u32.to_le_bytes());
        while out.len() % 8 != 0 {
            out.push(b'P');
        }
        out.extend(std::iter::repeat_n(0u8, 4 * held.len()));
        for offset in &offsets {
            out.extend(offset.to_le_bytes());
        }
        let data_at = out.len() + 4 + names.len();
        out.extend((data_at as u32).to_le_bytes());
        out.extend(names);
        out.extend(data);
        out
    }

    fn library(blob: &[u8]) -> Vec<u8> {
        const SECTION: usize = 0x200;
        const ADDRESS: u32 = 0x2000;
        let mut b = vec![0u8; SECTION];
        b[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        let pe = 0x80;
        b[pe + 6..pe + 8].copy_from_slice(&1u16.to_le_bytes());
        b[pe + 20..pe + 22].copy_from_slice(&224u16.to_le_bytes());
        let optional = pe + 24;
        b[optional..optional + 2].copy_from_slice(&0x10Bu16.to_le_bytes());
        let managed = optional + 96 + 14 * 8;
        b[managed..managed + 4].copy_from_slice(&ADDRESS.to_le_bytes());
        b[managed + 4..managed + 8].copy_from_slice(&72u32.to_le_bytes());
        let section = optional + 224;
        let mut body = vec![0u8; 72];
        let held = 72u32;
        body[24..28].copy_from_slice(&(ADDRESS + held).to_le_bytes());
        body[28..32].copy_from_slice(&((blob.len() + 4) as u32).to_le_bytes());
        body.extend((blob.len() as u32).to_le_bytes());
        body.extend(blob);
        b[section + 8..section + 12].copy_from_slice(&(body.len() as u32).to_le_bytes());
        b[section + 12..section + 16].copy_from_slice(&ADDRESS.to_le_bytes());
        b[section + 16..section + 20].copy_from_slice(&(body.len() as u32).to_le_bytes());
        b[section + 20..section + 24].copy_from_slice(&(SECTION as u32).to_le_bytes());
        b.extend(body);
        b
    }

    #[test]
    fn the_sounds_of_a_client_library_are_found_by_name_and_kind() {
        let made = library(&resources(&[("combobreak", STREAM, b"ID3 a break"), ("normal-hitnormal", BYTES, b"RIFF....WAVE"), ("menu-background", BYTES, b"\x89PNG...."), ("../outside", BYTES, b"RIFF....")]));
        let found = sounds_in(&made);
        assert_eq!(found.iter().map(|(name, kind, sound)| (name.as_str(), *kind, sound.len())).collect::<Vec<_>>(), vec![("combobreak", "mp3", 11), ("normal-hitnormal", "wav", 12)]);
        assert!(sounds_in(b"not a library at all").is_empty());
        assert!(sounds_in(&made[..made.len() - 20]).len() <= 1, "a cut file must not be read past its end");
    }

    #[test]
    fn the_sounds_are_written_once_and_again_only_when_the_library_changes() {
        let dir = std::env::temp_dir().join(format!("dossier-client-sounds-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let from = dir.join(LIBRARY);
        std::fs::write(&from, library(&resources(&[("combobreak", STREAM, b"ID3 a break")]))).unwrap();
        let into = dir.join("osu-sounds");
        assert_eq!(take_from(&from, &into), Ok(1));
        assert_eq!(std::fs::read(into.join("combobreak.mp3")).unwrap(), b"ID3 a break");
        assert_eq!(std::fs::read_to_string(into.join(STAMP)).ok(), stamp_of(&from));
        assert!(take_from(&dir.join("absent.dll"), &into).is_err());
        assert!(into.join("combobreak.mp3").is_file(), "a failed reading took the sounds away");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_mod_icon_is_found_inside_whatever_the_library_wraps_it_in() {
        let mut picture = PICTURE.to_vec();
        picture.extend(13u32.to_be_bytes());
        picture.extend(*b"IHDR");
        picture.extend([0u8; 13 + 4]);
        picture.extend(0u32.to_be_bytes());
        picture.extend(*b"IEND");
        picture.extend([1, 2, 3, 4]);
        let mut wrapped = b"System.Drawing.Bitmap, a wrapper of some length".to_vec();
        wrapped.extend(&picture);
        wrapped.extend(*b"and a tail");
        let made = library(&resources(&[("selection-mod-hidden", 0x41, &wrapped), ("selection-mod-easy@2x", BYTES, &picture), ("menu-back", 0x41, &wrapped), ("selection-mod-broken", 0x41, b"no picture here")]));
        let found = mod_icons_in(&made);
        assert_eq!(found.iter().map(|(name, kind, bytes)| (name.as_str(), *kind, bytes.len())).collect::<Vec<_>>(), vec![("selection-mod-hidden", "png", picture.len()), ("selection-mod-easy@2x", "png", picture.len())]);
        assert!(found.iter().all(|(_, _, bytes)| bytes.starts_with(PICTURE) && bytes.ends_with(&[1, 2, 3, 4])));
    }

    #[test]
    #[ignore = "needs DOSSIER_CLIENT_LIBRARY, a real osu!gameplay.dll"]
    fn a_real_client_gives_its_whole_kit() {
        let library = std::fs::read(std::env::var("DOSSIER_CLIENT_LIBRARY").unwrap()).unwrap();
        let found = sounds_in(&library);
        let names: Vec<&str> = found.iter().map(|(name, _, _)| name.as_str()).collect();
        for wanted in ["combobreak", "normal-hitnormal", "soft-hitwhistle", "drum-hitclap", "sectionpass", "spinnerbonus"] {
            assert!(names.contains(&wanted), "{wanted} is not among {} sounds", names.len());
        }
    }
}
