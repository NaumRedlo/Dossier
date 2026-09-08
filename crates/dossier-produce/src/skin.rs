use std::path::Path;

use dossier_render::elements::{Element, Health, Verdict};
use dossier_render::imported::Sprites;
use dossier_render::Skin;

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
    Element::SpinnerApproachCircle,
    Element::SpinnerCircle,
    Element::SpinnerMiddle,
    Element::SpinnerMiddle2,
    Element::SpinnerBackground,
    Element::SpinnerMetre,
    Element::SpinnerBottom,
    Element::SpinnerGlow,
    Element::SpinnerTop,
    Element::SpinnerRpm,
    Element::SpinnerSpin,
    Element::SpinnerClear,
    Element::SectionPass,
    Element::SectionFail,
];

fn scorebar_pieces() -> Vec<Element> {
    let mut all = vec![Element::ScoreBarBackground, Element::ScoreBarFill];
    all.extend([Health::Fine, Health::Low, Health::Critical].map(Element::ScoreBarMark));
    all
}

fn hud_glyphs() -> Vec<Element> {
    ('0'..='9')
        .chain([',', '.', '%', 'x'])
        .flat_map(|c| [Element::Score(c), Element::Combo(c)])
        .collect()
}

pub fn wanted() -> Vec<Element> {
    let mut all = DRAWN_FROM_SKINS.to_vec();
    all.extend(hud_glyphs());
    all.extend(scorebar_pieces());
    all
}

pub fn from_folder(mut skin: Skin, path: &Path, tint_ball: Option<bool>) -> Skin {
    let mut sprites = Sprites::read(path, &wanted());

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

pub fn pack(folder: &Path, into: &Path) -> Result<(), String> {
    let file = std::fs::File::create(into).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let how = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mut stack = vec![(folder.to_path_buf(), String::new())];
    while let Some((dir, under)) = stack.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else {
                continue;
            };
            let inside = if under.is_empty() {
                name
            } else {
                format!("{under}/{name}")
            };
            if path.is_dir() {
                stack.push((path, inside));
                continue;
            }
            let bytes = match std::fs::read(&path) {
                Ok(bytes) => bytes,
                Err(_) => continue,
            };
            zip.start_file(&inside, how).map_err(|e| e.to_string())?;
            std::io::Write::write_all(&mut zip, &bytes).map_err(|e| e.to_string())?;
        }
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

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
