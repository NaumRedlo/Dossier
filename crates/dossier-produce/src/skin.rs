//! Reading a skin off the disk and putting it on.
//!
//! Which pictures a render wants is not a property of the skin — it is a
//! property of what is drawn — so the list is assembled here rather than being
//! asked of every caller, and the two that draw (a command line and an
//! application) cannot end up wanting different things.

use std::path::Path;

use dossier_render::elements::{Element, Health, Verdict};
use dossier_render::imported::Sprites;
use dossier_render::Skin;

/// What a render draws, and so what is worth reading out of a skin folder.
///
/// Listed rather than derived from the enum: several of its members are for the
/// skin *exporter* and have no drawing code behind them yet, and reading files
/// nothing will use would be work done for a picture nobody sees.
pub const DRAWN_FROM_SKINS: &[Element] = &[
    Element::HitCircle,
    Element::HitCircleOverlay,
    Element::ApproachCircle,
    Element::ReverseArrow,
    Element::Cursor,
    Element::CursorMiddle,
    Element::CursorTrail,
    Element::Verdict(Verdict::Miss),
    Element::Verdict(Verdict::Fifty),
    Element::Verdict(Verdict::Hundred),
    Element::Verdict(Verdict::Three),
    // The ten combo digits. For an instafade skin these are the note itself,
    // so they are not optional decoration.
    Element::Digit(0),
    Element::Digit(1),
    Element::Digit(2),
    Element::Digit(3),
    Element::Digit(4),
    Element::Digit(5),
    Element::Digit(6),
    Element::Digit(7),
    Element::Digit(8),
    Element::Digit(9),
    // The slider's own furniture, its two ends included: osu! lets a skin draw
    // those differently from a note, and one that does looks half-applied
    // without them — the notes wear the skin and the sliders do not.
    Element::InputOverlayBackground,
    Element::InputOverlayKey,
    Element::FollowPoint,
    Element::Lighting,
    Element::SliderHead,
    Element::SliderHeadOverlay,
    Element::SliderTail,
    Element::SliderTailOverlay,
    Element::SliderBall,
    Element::SliderFollowCircle,
    Element::SliderScorePoint,
    // The spinner. `SpinnerBackground` is read for what its presence says
    // rather than to be drawn — it is how a skin declares which of osu!'s two
    // spinner styles it is drawn in.
    Element::SpinnerApproachCircle,
    Element::SpinnerCircle,
    Element::SpinnerMiddle,
    Element::SpinnerMiddle2,
    Element::SpinnerBackground,
    Element::SpinnerMetre,
    Element::SpinnerBottom,
    Element::SpinnerGlow,
    Element::SpinnerTop,
    // Read for what a blank one says — that the skin wants no read-out — rather
    // than to be drawn. The HUD still writes the figure in its own letters.
    Element::SpinnerRpm,
    Element::SectionPass,
    Element::SectionFail,
];

/// The skin's own HUD lettering: the figures in the corners, and the signs that
/// go with them. Built rather than listed because it is fourteen names of the
/// same shape.
/// The health bar's pieces, including all three of its marks.
fn scorebar_pieces() -> Vec<Element> {
    let mut all = vec![Element::ScoreBarBackground, Element::ScoreBarFill];
    all.extend([Health::Fine, Health::Low, Health::Critical].map(Element::ScoreBarMark));
    all
}

fn hud_glyphs() -> Vec<Element> {
    // Both faces. osu! skins the score and the combo counter apart, and on a
    // skin that names them apart these are two different sets of files under
    // one set of characters.
    ('0'..='9')
        .chain([',', '.', '%', 'x'])
        .flat_map(|c| [Element::Score(c), Element::Combo(c)])
        .collect()
}

/// Everything worth reading out of a skin folder.

/// Every picture a render looks for in a skin folder.
pub fn wanted() -> Vec<Element> {
    let mut all = DRAWN_FROM_SKINS.to_vec();
    all.extend(hud_glyphs());
    all.extend(scorebar_pieces());
    all
}

/// Put the skin in `path` on, over whatever the caller started from.
///
/// `tint_ball` is a third answer rather than a boolean: `None` is nobody having
/// said, which leaves the skin's own `AllowSliderBallTint` standing.
pub fn from_folder(mut skin: Skin, path: &Path, tint_ball: Option<bool>) -> Skin {
    let mut sprites = Sprites::read(path, &wanted());
    // Before the colouring, which is where the tinted pictures are made — and
    // not made at all when the answer is no.
    if let Some(yes) = tint_ball {
        sprites.allow_slider_ball_tint(yes);
    }
    let ini = sprites.ini().clone();
    if !ini.combo_colours.is_empty() {
        skin.combo_colours = ini.combo_colours.clone();
    }
    if let Some(border) = ini.slider_border {
        skin.slider_border = border;
    }
    skin.slider_body = ini.slider_track;
    skin.sprites = Some(std::sync::Arc::new(sprites.tint_for(&skin.combo_colours)));
    skin
}

/// Unpack a skin that arrived as one archive.
///
/// A skin is sent as a zip rather than as its files because it is hundreds of
/// them. Nothing from inside is trusted with a path: an entry that climbs out
/// of the folder is skipped, which is the whole of the defence a worker needs
/// against an archive it did not make.
pub fn unpack(archive: &[u8], into: &Path) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(archive)).map_err(|e| e.to_string())?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|e| e.to_string())?;
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        let path = into.join(name);
        if entry.is_dir() {
            std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut file).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    /// An archive is not trusted with where its files go.
    #[test]
    fn an_entry_that_climbs_out_of_the_folder_is_left_alone() {
        let mut made = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut made));
            let plain = zip::write::SimpleFileOptions::default();
            zip.start_file("cursor.png", plain).expect("a file");
            zip.write_all(b"picture").expect("written");
            zip.start_file("../escaped.png", plain).expect("a file");
            zip.write_all(b"nope").expect("written");
            zip.finish().expect("an archive");
        }
        let dir = std::env::temp_dir().join(format!("dossier-skin-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a place");
        super::unpack(&made, &dir).expect("unpacked");
        assert!(
            dir.join("cursor.png").is_file(),
            "the ordinary file was lost"
        );
        assert!(
            !dir.parent().expect("a parent").join("escaped.png").exists(),
            "an entry climbed out of the folder"
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
