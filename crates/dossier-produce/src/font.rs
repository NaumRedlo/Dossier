//! Finding the font a render draws its numbers with.
//!
//! Without one the play is drawn and the score, the accuracy and the combo are
//! not — a video that looks finished, is not, and that nobody watching would
//! think to report as a setup problem. So it is worth looking in more than one
//! place, and worth saying when none of them had it.

use std::path::{Path, PathBuf};

use dossier_render::Font;

/// Where to look when nobody said. In order.
///
/// `DOSSIER_FONT` first, because somebody who set it meant it. Then beside the
/// program, which is where it ships for anyone who did not build it. Then the
/// source tree, from one, two and three directories up — which is where it is
/// while the thing is being written, and the reason those three lines were in
/// the command line before anything else was.
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

/// The font, or `None` when there is none to be had.
///
/// An explicit path that cannot be read is an error — somebody named a file and
/// it is not there. Nothing found among the fallbacks is not: it is a render
/// without numbers, which is worth saying and not worth stopping for.
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
