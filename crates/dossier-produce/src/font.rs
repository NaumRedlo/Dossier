use std::path::{Path, PathBuf};

use dossier_render::Font;

const FACES: &[&str] = &["Huninn-Regular.ttf"];

const BEHIND: &[&str] = &["JetBrainsMono-Bold.ttf"];

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
    match look_for(BEHIND) {
        Some(behind) => front.behind(&behind),
        None => front,
    }
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
    Ok(look_for(FACES).map(backed))
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
        if look_for(BEHIND).is_some() {
            assert_eq!(font.faces(), 2, "the fallback did not attach");
            assert!(font.width("結界", 24.0) > 0.0, "no width for a CJK title");
        }
    }
}
