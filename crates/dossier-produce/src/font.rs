use std::path::{Path, PathBuf};

use dossier_render::Font;

const CHAIN: &[&[&str]] = &[
    &["VarelaRound-Regular.ttf"],
    &["Commissioner-Regular.ttf"],
    &["MPLUSRounded1c-Regular.ttf"],
    &["JetBrainsMono-Bold.ttf"],
];

fn shelves() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(beside) = exe.parent() {
            out.push(beside.to_path_buf());
            out.push(beside.join("assets/fonts"));
            out.push(beside.join("../Resources/assets/fonts"));
        }
    }
    for up in ["", "../", "../../"] {
        out.push(PathBuf::from(format!("{up}assets/fonts")));
    }
    out
}

fn look_for(names: &[&str]) -> Option<Font> {
    for shelf in shelves() {
        for name in names {
            let path = shelf.join(name);
            if let Ok(bytes) = std::fs::read(&path) {
                if let Ok(font) = Font::from_bytes(&bytes) {
                    return Some(font);
                }
            }
        }
    }
    None
}

fn backed(front: Font) -> Font {
    CHAIN.iter().skip(1).filter_map(|names| look_for(names)).fold(
        front,
        |built, behind| built.behind(&behind),
    )
}

fn chained() -> Option<Font> {
    let mut found = CHAIN.iter().filter_map(|names| look_for(names));
    let front = found.next()?;
    Some(found.fold(front, |built, behind| built.behind(&behind)))
}

pub fn find(explicit: Option<&Path>) -> Result<Option<Font>, String> {
    if let Some(path) = explicit {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        return Font::from_bytes(&bytes)
            .map(|face| Some(backed(face)))
            .map_err(|e| format!("{}: {e}", path.display()));
    }
    if let Some(said) = std::env::var_os("DOSSIER_FONT") {
        if let Ok(bytes) = std::fs::read(PathBuf::from(said)) {
            if let Ok(face) = Font::from_bytes(&bytes) {
                return Ok(Some(backed(face)));
            }
        }
    }
    Ok(chained())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_font_found_in_the_repository_carries_its_fallback() {
        let Some(font) = find(None).expect("looking does not fail") else {
            return;
        };
        assert!(
            font.faces() >= 1,
            "a font with no faces cannot draw anything"
        );
        assert!(font.width("Dossier", 24.0) > 0.0);
        let on_disk = CHAIN.iter().filter(|names| look_for(names).is_some()).count();
        assert_eq!(font.faces(), on_disk, "a face on the shelf did not join the chain");
        if on_disk == CHAIN.len() {
            assert!(font.width("Съешь", 24.0) > 0.0, "no width for Cyrillic");
            assert!(font.width("結界", 24.0) > 0.0, "no width for a CJK title");
        }
    }

    #[test]
    fn each_script_is_drawn_by_the_face_that_carries_it() {
        let Some(font) = find(None).expect("looking does not fail") else {
            return;
        };
        if font.faces() < 3 {
            return;
        }
        let latin = font.width("a", 24.0);
        let cyrillic = font.width("а", 24.0);
        let kana = font.width("あ", 24.0);
        assert!(latin > 0.0 && cyrillic > 0.0 && kana > 0.0);
        assert!(kana > latin, "kana should come from the wider CJK face, not be a tofu box");
    }
}
