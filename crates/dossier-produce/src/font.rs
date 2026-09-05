use std::path::{Path, PathBuf};

use dossier_render::Font;

fn candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(said) = std::env::var_os("DOSSIER_FONT") {
        out.push(PathBuf::from(said));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(beside) = exe.parent() {
            out.push(beside.join("TorusNotched-Bold.ttf"));
            out.push(beside.join("assets/fonts/TorusNotched-Bold.ttf"));
        }
    }
    for up in ["", "../", "../../"] {
        out.push(PathBuf::from(format!(
            "{up}assets/fonts/TorusNotched-Bold.ttf"
        )));
    }
    out
}

pub fn find(explicit: Option<&Path>) -> Result<Option<Font>, String> {
    if let Some(path) = explicit {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        return Font::from_bytes(&bytes)
            .map(Some)
            .map_err(|e| format!("{}: {e}", path.display()));
    }
    for candidate in candidates() {
        if let Ok(bytes) = std::fs::read(&candidate) {
            if let Ok(font) = Font::from_bytes(&bytes) {
                return Ok(Some(font));
            }
        }
    }
    Ok(None)
}
