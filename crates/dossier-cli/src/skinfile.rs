use std::path::{Path, PathBuf};

use dossier_render::Skin;
use tiny_skia::Color;

const SKIN_VERSION: &str = "2.7";

const SAMPLE_SETS: [&str; 3] = ["normal", "soft", "drum"];
const SAMPLE_SOUNDS: [&str; 4] = ["normal", "whistle", "finish", "clap"];

fn elements() -> Vec<dossier_render::elements::Element> {
    use dossier_render::elements::Element;
    let mut all = vec![
        Element::HitCircle,
        Element::HitCircleOverlay,
        Element::ApproachCircle,
        Element::ReverseArrow,
        Element::SliderScorePoint,
        Element::Cursor,
        Element::CursorMiddle,
        Element::CursorTrail,
        Element::SpinnerApproachCircle,
    ];
    all.extend((0..=9).map(Element::Digit));
    all.extend(dossier_render::elements::Verdict::ALL.map(Element::Verdict));
    all
}

pub struct Written {
    pub folder: PathBuf,
    pub sounds: usize,
    pub images: usize,
}

pub fn write(
    skin: &Skin,
    name: &str,
    folder: &Path,
    samples: Option<&Path>,
) -> Result<Written, String> {
    std::fs::create_dir_all(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let ini = folder.join("skin.ini");
    std::fs::write(&ini, ini_text(skin, name)).map_err(|e| format!("{}: {e}", ini.display()))?;

    let mut sounds = 0;
    if let Some(from) = samples {
        for set in SAMPLE_SETS {
            for sound in SAMPLE_SOUNDS {
                let file = format!("{set}-hit{sound}.wav");
                let source = from.join(&file);
                if source.is_file() && std::fs::copy(&source, folder.join(&file)).is_ok() {
                    sounds += 1;
                }
            }

            let tick = format!("{set}-slidertick.wav");
            let source = from.join(&tick);
            if source.is_file() && std::fs::copy(&source, folder.join(&tick)).is_ok() {
                sounds += 1;
            }
        }
    }

    let mut images = 0;
    for element in elements() {
        for (suffix, size) in [("", element.size()), ("@2x", element.size() * 2)] {
            let Some(pixmap) = dossier_render::elements::element(skin, element, size) else {
                continue;
            };
            let path = folder.join(format!("{}{suffix}.png", element.stem()));
            match pixmap.encode_png() {
                Ok(png) => {
                    std::fs::write(&path, png).map_err(|e| format!("{}: {e}", path.display()))?;
                    images += 1;
                }
                Err(error) => return Err(format!("{}: {error}", path.display())),
            }
        }
    }

    Ok(Written {
        folder: folder.to_path_buf(),
        sounds,
        images,
    })
}

fn ini_text(skin: &Skin, name: &str) -> String {
    let mut out = String::new();
    out.push_str("// Written by `dossier skin`. The colours are the engine's own,\n");
    out.push_str("// so a render and a play in this skin agree about the palette.\n\n");

    out.push_str("[General]\n");
    out.push_str(&format!("Name: {name}\n"));
    out.push_str("Author: dossier\n");
    out.push_str(&format!("Version: {SKIN_VERSION}\n"));
    out.push('\n');

    out.push_str("[Colours]\n");

    out.push_str("// Combo2 is shown first and Combo1 last — osu!'s own ordering.\n");
    let colours = &skin.combo_colours;
    for (index, colour) in colours.iter().enumerate() {
        let slot = if index + 1 == colours.len() {
            1
        } else {
            index + 2
        };
        out.push_str(&format!("Combo{slot}: {}\n", rgb(*colour)));
    }
    out.push_str(&format!("SliderBorder: {}\n", rgb(skin.slider_border)));
    out.push('\n');

    out.push_str("[Fonts]\n");
    out.push_str(&format!(
        "HitCircleOverlap: {}\n",
        (dossier_render::elements::DIGIT_PADDING * 2.0).round() as i32
    ));
    out.push('\n');

    out
}

fn rgb(colour: Color) -> String {
    let channel = |v: f32| (v * 255.0).round().clamp(0.0, 255.0) as u8;
    format!(
        "{},{},{}",
        channel(colour.red()),
        channel(colour.green()),
        channel(colour.blue())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_combo_cycle_is_written_in_osus_backwards_order() {
        let text = ini_text(&Skin::default(), "dossier");
        let at = |slot: &str| {
            text.lines()
                .find(|l| l.starts_with(slot))
                .unwrap_or_else(|| panic!("no {slot} in {text}"))
        };
        assert_eq!(
            at("Combo2:"),
            "Combo2: 255,192,0",
            "ours first is shown first"
        );
        assert_eq!(at("Combo3:"), "Combo3: 0,202,0");
        assert_eq!(at("Combo4:"), "Combo4: 18,124,255");
        assert_eq!(
            at("Combo1:"),
            "Combo1: 242,24,57",
            "ours last is shown last"
        );
    }

    #[test]
    fn the_version_is_pinned_rather_than_left_to_the_game() {
        let text = ini_text(&Skin::default(), "dossier");
        assert!(text.contains("Version: 2.7"), "{text}");
        assert!(!text.contains("latest"));
    }

    #[test]
    fn a_colour_is_written_as_three_bytes() {
        assert_eq!(rgb(Color::from_rgba8(226, 72, 72, 255)), "226,72,72");
        assert_eq!(rgb(Color::from_rgba8(0, 0, 0, 255)), "0,0,0");
        assert_eq!(rgb(Color::from_rgba8(255, 255, 255, 255)), "255,255,255");
    }
}
